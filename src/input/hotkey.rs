use anyhow::{anyhow, Context, Result};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

use crate::alias;
use crate::audio::AudioCapture;
use crate::config::{AliasesConfig, HotkeyConfig};
use crate::input::cgevent;
use crate::transcription::{ModelManager, TranscriptionEngine};

/// Application state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    /// Waiting for hotkey press
    Idle,
    /// Recording audio (hotkey held)
    Recording,
    /// Transcribing and inserting text
    Processing,
}

/// Lazy loading configuration (model manager + model name)
type LazyLoadConfig = (Arc<Mutex<ModelManager>>, String);

/// Global hotkey manager with state tracking
pub struct HotkeyManager {
    manager: Arc<GlobalHotKeyManager>,
    hotkey: HotKey,
    state: Arc<Mutex<AppState>>,
    audio: Arc<Mutex<AudioCapture>>,
    transcription: Option<Arc<TranscriptionEngine>>,
    recording_enabled: bool,
    aliases: Arc<AliasesConfig>,
    /// For lazy loading: model manager + model name
    lazy_load_config: Option<LazyLoadConfig>,
}

impl HotkeyManager {
    /// Create and register global hotkey from config using shared manager
    ///
    /// # Errors
    /// Returns error if unknown modifiers/keys or registration fails
    pub fn new(
        manager: Arc<GlobalHotKeyManager>,
        config: &HotkeyConfig,
        audio: Arc<Mutex<AudioCapture>>,
        transcription: Option<Arc<TranscriptionEngine>>,
        recording_enabled: bool,
        aliases: Arc<AliasesConfig>,
        lazy_load_config: Option<LazyLoadConfig>,
    ) -> Result<Self> {
        let modifiers = Self::parse_modifiers(&config.modifiers)?;
        let code = Self::parse_key(&config.key)?;

        let hotkey = HotKey::new(Some(modifiers), code);
        manager
            .register(hotkey)
            .context("failed to register hotkey")?;

        info!("registered hotkey: {:?} + {}", config.modifiers, config.key);

        Ok(Self {
            manager,
            hotkey,
            state: Arc::new(Mutex::new(AppState::Idle)),
            audio,
            transcription,
            recording_enabled,
            aliases,
            lazy_load_config,
        })
    }

    /// Get shared state for external monitoring (e.g., UI updates)
    #[must_use]
    pub fn state_shared(&self) -> Arc<Mutex<AppState>> {
        Arc::clone(&self.state)
    }

    /// Handle hotkey press event
    pub fn on_press(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match *state {
            AppState::Idle => {
                info!("🎤 Hotkey pressed - recording started");
                *state = AppState::Recording;
                drop(state);

                // Start audio recording with error recovery
                let recording_result = self
                    .audio
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .start_recording();

                if let Err(e) = recording_result {
                    warn!(error = %e, "❌ Failed to start recording");
                    let mut state = self
                        .state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    *state = AppState::Idle;
                    // Continue running - this is a transient error, user can try again
                }
            }
            AppState::Recording => {
                drop(state);
                debug!("hotkey pressed while recording (ignored)");
            }
            AppState::Processing => {
                drop(state);
                debug!("hotkey pressed while processing (ignored)");
            }
        }
    }

    /// Handle hotkey release event
    pub fn on_release(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match *state {
            AppState::Recording => {
                info!("⏹️  Hotkey released - processing audio");
                *state = AppState::Processing;
                drop(state);

                // Stop audio recording and get samples
                let stop_result = self
                    .audio
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .stop_recording();

                match stop_result {
                    Ok(samples) => {
                        // Duration calculation: usize → f64 for sample_count / sample_rate
                        // Safe: even 1hr audio = 57.6M samples, well within f64 precision
                        #[allow(clippy::cast_precision_loss)]
                        let duration_secs = samples.len() as f64 / 16000.0;
                        info!(
                            sample_count = samples.len(),
                            duration_secs = format!("{:.1}", duration_secs),
                            "📼 Captured {:.1}s audio ({} samples)",
                            duration_secs,
                            samples.len()
                        );

                        if self.recording_enabled {
                            Self::save_debug_wav(&samples);
                        }
                        self.process_transcription(samples);
                    }
                    Err(e) => {
                        warn!(error = %e, "❌ Failed to stop recording: {}", e);
                        let mut state = self
                            .state
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        *state = AppState::Idle;
                        // Continue running - this is a transient error, user can try again
                    }
                }
            }
            AppState::Idle => {
                drop(state);
                debug!("hotkey released while idle (ignored)");
            }
            AppState::Processing => {
                drop(state);
                debug!("hotkey released while processing (ignored)");
            }
        }
    }

    /// Save debug WAV file with error recovery
    fn save_debug_wav(samples: &[f32]) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_owned());
        let debug_path = std::path::PathBuf::from(home)
            .join(".whisper-hotkey")
            .join("debug")
            .join(format!("recording_{timestamp}.wav"));

        if let Err(e) = AudioCapture::save_wav_debug(samples, &debug_path) {
            warn!(error = %e, path = ?debug_path, "failed to save debug WAV");
        } else {
            debug!(path = ?debug_path, "saved debug WAV");
        }
    }

    /// Process transcription and text insertion in background thread
    fn process_transcription(&self, samples: Vec<f32>) {
        let engine = self.transcription.clone();
        let lazy_load_config = self.lazy_load_config.clone();
        let state_arc = Arc::clone(&self.state);
        let aliases = Arc::clone(&self.aliases);

        // Set state to Processing if lazy loading needed (loading + transcription)
        if engine.is_none() && lazy_load_config.is_some() {
            *state_arc
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = AppState::Processing;
        }

        std::thread::spawn(move || {
            // Try lazy loading if needed (in background thread)
            let engine = engine.or_else(|| {
                if let Some((model_mgr, model_name)) = &lazy_load_config {
                    info!("🔄 Lazy loading model: {}", model_name);
                    let result = {
                        let mut mgr = model_mgr
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        mgr.get_or_load(model_name)
                    };
                    match result {
                        Ok(engine) => {
                            info!("✅ Model loaded: {}", model_name);
                            Some(engine)
                        }
                        Err(e) => {
                            warn!(error = %e, model = %model_name, "❌ Failed to lazy load model");
                            None
                        }
                    }
                } else {
                    None
                }
            });

            if let Some(engine) = engine {
                match engine.transcribe(&samples) {
                    Ok(text) => {
                        let text_preview: String = text.chars().take(50).collect();
                        info!(
                            text_len = text.len(),
                            text_preview = %text_preview,
                            "✨ Transcription: \"{}{}\"",
                            text_preview,
                            if text.len() > 50 { "..." } else { "" }
                        );

                        // Apply alias matching
                        let final_text = alias::apply_aliases(&text, &aliases);

                        // Insert text at cursor, only if non-empty
                        if final_text.is_empty() {
                            info!("🔇 No speech detected (silence or noise)");
                        } else if cgevent::insert_text_safe(&final_text) {
                            info!(
                                text_len = final_text.len(),
                                "✅ Inserted {} chars",
                                final_text.len()
                            );
                        } else {
                            warn!(
                                text_len = final_text.len(),
                                text_preview = %text_preview,
                                "❌ Text insertion failed - check permissions"
                            );
                        }
                    }
                    Err(e) => {
                        warn!(
                            error = %e,
                            sample_count = samples.len(),
                            "❌ Transcription failed: {}",
                            e
                        );
                    }
                }
            } else {
                warn!("⚠️  Transcription engine not available");
            }

            // Set state to Idle after processing (always recover)
            *state_arc
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = AppState::Idle;
            info!("✓ Ready for next recording");
        });
    }

    /// Process hotkey events from global event channel
    pub fn handle_event(&self, event: GlobalHotKeyEvent) {
        if event.id != self.hotkey.id() {
            return;
        }

        match event.state {
            global_hotkey::HotKeyState::Pressed => self.on_press(),
            global_hotkey::HotKeyState::Released => self.on_release(),
        }
    }

    /// Get hotkey ID for this manager
    #[must_use]
    pub fn hotkey_id(&self) -> u32 {
        self.hotkey.id()
    }

    fn parse_modifiers(modifiers: &[String]) -> Result<Modifiers> {
        let mut result = Modifiers::empty();
        for modifier in modifiers {
            match modifier.as_str() {
                "Control" | "Ctrl" => result |= Modifiers::CONTROL,
                "Option" | "Alt" => result |= Modifiers::ALT,
                "Command" | "Super" => result |= Modifiers::SUPER,
                "Shift" => result |= Modifiers::SHIFT,
                _ => return Err(anyhow!("unknown modifier: {modifier}")),
            }
        }
        Ok(result)
    }

    fn parse_key(key: &str) -> Result<Code> {
        match key {
            "A" => Ok(Code::KeyA),
            "B" => Ok(Code::KeyB),
            "C" => Ok(Code::KeyC),
            "D" => Ok(Code::KeyD),
            "E" => Ok(Code::KeyE),
            "F" => Ok(Code::KeyF),
            "G" => Ok(Code::KeyG),
            "H" => Ok(Code::KeyH),
            "I" => Ok(Code::KeyI),
            "J" => Ok(Code::KeyJ),
            "K" => Ok(Code::KeyK),
            "L" => Ok(Code::KeyL),
            "M" => Ok(Code::KeyM),
            "N" => Ok(Code::KeyN),
            "O" => Ok(Code::KeyO),
            "P" => Ok(Code::KeyP),
            "Q" => Ok(Code::KeyQ),
            "R" => Ok(Code::KeyR),
            "S" => Ok(Code::KeyS),
            "T" => Ok(Code::KeyT),
            "U" => Ok(Code::KeyU),
            "V" => Ok(Code::KeyV),
            "W" => Ok(Code::KeyW),
            "X" => Ok(Code::KeyX),
            "Y" => Ok(Code::KeyY),
            "Z" => Ok(Code::KeyZ),
            _ => Err(anyhow!("unsupported key: {key}")),
        }
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        if let Err(e) = self.manager.unregister(self.hotkey) {
            tracing::error!("failed to unregister hotkey: {}", e);
        }
    }
}

/// Multi-profile hotkey manager supporting multiple models and hotkeys
pub struct MultiHotkeyManager {
    /// Individual hotkey managers (one per profile)
    managers: Vec<(String, HotkeyManager)>,
    /// Shared audio capture
    #[allow(dead_code)] // Held for lifetime management
    audio: Arc<Mutex<AudioCapture>>,
    /// Model manager for lazy loading
    #[allow(dead_code)] // Held for lifetime management
    model_manager: Arc<Mutex<ModelManager>>,
}

impl MultiHotkeyManager {
    /// Pump macOS `NSApplication` event loop to ensure system is responsive
    /// Required for global hotkey registration to complete without timeout
    #[cfg(target_os = "macos")]
    #[allow(unsafe_code)] // Required for NSApplication event loop FFI
    fn pump_event_loop() {
        use objc2::rc::autoreleasepool;
        use objc2_app_kit::{NSApp, NSEventMask};
        use objc2_foundation::{MainThreadMarker, NSDate, NSDefaultRunLoopMode};

        autoreleasepool(|_| {
            // Safety: This function is only called during initialization on the main thread
            let mtm = unsafe { MainThreadMarker::new_unchecked() };
            let app = NSApp(mtm);
            let distant_past = NSDate::distantPast();

            // Process all pending events
            loop {
                let event = unsafe {
                    app.nextEventMatchingMask_untilDate_inMode_dequeue(
                        NSEventMask(u64::MAX),
                        Some(&distant_past),
                        NSDefaultRunLoopMode,
                        true,
                    )
                };
                if let Some(event) = event {
                    app.sendEvent(&event);
                } else {
                    break;
                }
            }
        });
    }

    #[cfg(not(target_os = "macos"))]
    fn pump_event_loop() {
        // No-op on non-macOS platforms
    }

    /// Create multi-hotkey manager from profiles
    ///
    /// # Errors
    /// Returns error if hotkey registration fails or model preloading fails
    pub fn new(
        profiles: &[crate::config::TranscriptionProfile],
        audio: Arc<Mutex<AudioCapture>>,
        recording_enabled: bool,
        aliases: &Arc<AliasesConfig>,
    ) -> Result<Self> {
        // Create single shared GlobalHotKeyManager for all profiles
        // Pump event loop first to ensure NSApplication is ready
        Self::pump_event_loop();
        let global_manager =
            Arc::new(GlobalHotKeyManager::new().context("failed to create global hotkey manager")?);

        // Create model manager (preloads where profile.preload=true)
        let model_manager = Arc::new(Mutex::new(
            ModelManager::new(profiles).context("failed to initialize model manager")?,
        ));

        let mut managers = Vec::new();

        for profile in profiles {
            let model_name = profile.name().to_owned();

            // Get engine if preloaded, None if lazy
            let (engine, lazy_config) = if profile.preload {
                let arc_engine = {
                    let mut mgr = model_manager
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    mgr.get_or_load(&model_name)
                        .with_context(|| format!("failed to get preloaded model: {model_name}"))?
                };
                (Some(Arc::clone(&arc_engine)), None)
            } else {
                // For lazy loading, pass model manager + model name
                (None, Some((Arc::clone(&model_manager), model_name.clone())))
            };

            // Create hotkey manager for this profile with shared global manager
            let mgr = HotkeyManager::new(
                Arc::clone(&global_manager),
                &profile.hotkey,
                Arc::clone(&audio),
                engine,
                recording_enabled,
                Arc::clone(aliases),
                lazy_config,
            )
            .with_context(|| format!("failed to register hotkey for profile: {model_name}"))?;

            info!(
                profile = %model_name,
                hotkey = format!("{:?}+{}", profile.hotkey.modifiers, profile.hotkey.key),
                preloaded = profile.preload,
                "registered profile"
            );

            managers.push((model_name, mgr));
        }

        Ok(Self {
            managers,
            audio,
            model_manager,
        })
    }

    /// Handle hotkey event by dispatching only to the matching manager
    pub fn handle_event(&self, event: GlobalHotKeyEvent) {
        for (_, mgr) in &self.managers {
            if event.id == mgr.hotkey_id() {
                mgr.handle_event(event);
                break; // Only one manager handles a given event
            }
        }
    }

    /// Get state for specific profile
    #[must_use]
    pub fn profile_state(&self, profile_name: &str) -> Option<Arc<Mutex<AppState>>> {
        self.managers
            .iter()
            .find(|(name, _)| name == profile_name)
            .map(|(_, mgr)| mgr.state_shared())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_modifiers_control() {
        let result = HotkeyManager::parse_modifiers(&["Control".to_owned()]).unwrap();
        assert_eq!(result, Modifiers::CONTROL);
    }

    #[test]
    fn test_parse_modifiers_ctrl_alias() {
        let result = HotkeyManager::parse_modifiers(&["Ctrl".to_owned()]).unwrap();
        assert_eq!(result, Modifiers::CONTROL);
    }

    #[test]
    fn test_parse_modifiers_option() {
        let result = HotkeyManager::parse_modifiers(&["Option".to_owned()]).unwrap();
        assert_eq!(result, Modifiers::ALT);
    }

    #[test]
    fn test_parse_modifiers_alt_alias() {
        let result = HotkeyManager::parse_modifiers(&["Alt".to_owned()]).unwrap();
        assert_eq!(result, Modifiers::ALT);
    }

    #[test]
    fn test_parse_modifiers_command() {
        let result = HotkeyManager::parse_modifiers(&["Command".to_owned()]).unwrap();
        assert_eq!(result, Modifiers::SUPER);
    }

    #[test]
    fn test_parse_modifiers_super_alias() {
        let result = HotkeyManager::parse_modifiers(&["Super".to_owned()]).unwrap();
        assert_eq!(result, Modifiers::SUPER);
    }

    #[test]
    fn test_parse_modifiers_shift() {
        let result = HotkeyManager::parse_modifiers(&["Shift".to_owned()]).unwrap();
        assert_eq!(result, Modifiers::SHIFT);
    }

    #[test]
    fn test_parse_modifiers_multiple() {
        let result =
            HotkeyManager::parse_modifiers(&["Control".to_owned(), "Option".to_owned()]).unwrap();
        assert_eq!(result, Modifiers::CONTROL | Modifiers::ALT);
    }

    #[test]
    fn test_parse_modifiers_all() {
        let result = HotkeyManager::parse_modifiers(&[
            "Control".to_owned(),
            "Option".to_owned(),
            "Command".to_owned(),
            "Shift".to_owned(),
        ])
        .unwrap();
        assert_eq!(
            result,
            Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER | Modifiers::SHIFT
        );
    }

    #[test]
    fn test_parse_modifiers_invalid() {
        let result = HotkeyManager::parse_modifiers(&["Invalid".to_owned()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown modifier"));
    }

    #[test]
    fn test_parse_modifiers_empty() {
        let result = HotkeyManager::parse_modifiers(&[]).unwrap();
        assert_eq!(result, Modifiers::empty());
    }

    #[test]
    fn test_parse_key_a_to_z() {
        assert_eq!(HotkeyManager::parse_key("A").unwrap(), Code::KeyA);
        assert_eq!(HotkeyManager::parse_key("B").unwrap(), Code::KeyB);
        assert_eq!(HotkeyManager::parse_key("M").unwrap(), Code::KeyM);
        assert_eq!(HotkeyManager::parse_key("Z").unwrap(), Code::KeyZ);
    }

    #[test]
    fn test_parse_key_unsupported() {
        let result = HotkeyManager::parse_key("F1");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unsupported key"));
    }

    #[test]
    fn test_parse_key_lowercase() {
        let result = HotkeyManager::parse_key("a");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_key_empty() {
        let result = HotkeyManager::parse_key("");
        assert!(result.is_err());
    }

    #[test]
    fn test_app_state_initial_is_idle() {
        let state = AppState::Idle;
        assert_eq!(state, AppState::Idle);
    }

    #[test]
    fn test_app_state_transitions() {
        let mut state = AppState::Idle;
        assert_eq!(state, AppState::Idle);

        state = AppState::Recording;
        assert_eq!(state, AppState::Recording);

        state = AppState::Processing;
        assert_eq!(state, AppState::Processing);

        state = AppState::Idle;
        assert_eq!(state, AppState::Idle);
    }

    #[test]
    fn test_parse_key_all_letters() {
        // Test all 26 letters
        assert_eq!(HotkeyManager::parse_key("C").unwrap(), Code::KeyC);
        assert_eq!(HotkeyManager::parse_key("D").unwrap(), Code::KeyD);
        assert_eq!(HotkeyManager::parse_key("E").unwrap(), Code::KeyE);
        assert_eq!(HotkeyManager::parse_key("F").unwrap(), Code::KeyF);
        assert_eq!(HotkeyManager::parse_key("G").unwrap(), Code::KeyG);
        assert_eq!(HotkeyManager::parse_key("H").unwrap(), Code::KeyH);
        assert_eq!(HotkeyManager::parse_key("I").unwrap(), Code::KeyI);
        assert_eq!(HotkeyManager::parse_key("J").unwrap(), Code::KeyJ);
        assert_eq!(HotkeyManager::parse_key("K").unwrap(), Code::KeyK);
        assert_eq!(HotkeyManager::parse_key("L").unwrap(), Code::KeyL);
        assert_eq!(HotkeyManager::parse_key("N").unwrap(), Code::KeyN);
        assert_eq!(HotkeyManager::parse_key("O").unwrap(), Code::KeyO);
        assert_eq!(HotkeyManager::parse_key("P").unwrap(), Code::KeyP);
        assert_eq!(HotkeyManager::parse_key("Q").unwrap(), Code::KeyQ);
        assert_eq!(HotkeyManager::parse_key("R").unwrap(), Code::KeyR);
        assert_eq!(HotkeyManager::parse_key("S").unwrap(), Code::KeyS);
        assert_eq!(HotkeyManager::parse_key("T").unwrap(), Code::KeyT);
        assert_eq!(HotkeyManager::parse_key("U").unwrap(), Code::KeyU);
        assert_eq!(HotkeyManager::parse_key("V").unwrap(), Code::KeyV);
        assert_eq!(HotkeyManager::parse_key("W").unwrap(), Code::KeyW);
        assert_eq!(HotkeyManager::parse_key("X").unwrap(), Code::KeyX);
        assert_eq!(HotkeyManager::parse_key("Y").unwrap(), Code::KeyY);
    }

    #[test]
    fn test_parse_modifiers_mixed_case() {
        // Test that only exact case works
        let result = HotkeyManager::parse_modifiers(&["control".to_owned()]);
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "requires audio hardware and global hotkey registration"]
    fn test_hotkey_manager_creation() {
        // Would need to:
        // 1. Mock or create AudioCapture
        // 2. Handle global hotkey registration (may conflict with other tests)
        // Skip for now as it's integration-level testing
    }

    #[test]
    #[ignore = "requires state machine with audio integration"]
    fn test_state_transitions_on_press_release() {
        // Would need to:
        // 1. Mock AudioCapture
        // 2. Test state transitions: Idle → Recording → Processing → Idle
        // Skip for now as it's integration-level testing
    }

    // Phase 2: Mock-based state machine tests
    mod mock_tests {
        use super::*;
        use crate::transcription::engine::TranscriptionError;
        use mockall::mock;
        use mockall::predicate::*;

        // Define traits for mocking
        trait AudioCaptureTrait {
            fn start_recording(&mut self) -> Result<()>;
            fn stop_recording(&mut self) -> Result<Vec<f32>>;
        }

        trait TranscriptionEngineTrait {
            fn transcribe(&self, audio_data: &[f32]) -> Result<String, TranscriptionError>;
        }

        // Mock implementations
        mock! {
            AudioCapture {}
            impl AudioCaptureTrait for AudioCapture {
                fn start_recording(&mut self) -> Result<()>;
                fn stop_recording(&mut self) -> Result<Vec<f32>>;
            }
        }

        mock! {
            TranscriptionEngine {}
            impl TranscriptionEngineTrait for TranscriptionEngine {
                fn transcribe(&self, audio_data: &[f32]) -> Result<String, TranscriptionError>;
            }
        }

        // Helper to create HotkeyManager with mocks for testing
        struct TestHotkeyManager {
            state: Arc<Mutex<AppState>>,
            audio: Arc<Mutex<MockAudioCapture>>,
            transcription: Option<Arc<MockTranscriptionEngine>>,
        }

        impl TestHotkeyManager {
            fn new(
                audio: MockAudioCapture,
                transcription: Option<MockTranscriptionEngine>,
            ) -> Self {
                Self {
                    state: Arc::new(Mutex::new(AppState::Idle)),
                    audio: Arc::new(Mutex::new(audio)),
                    transcription: transcription.map(Arc::new),
                }
            }

            fn on_press(&self) {
                let mut state = self.state.lock().unwrap();
                match *state {
                    AppState::Idle => {
                        *state = AppState::Recording;
                        drop(state);

                        let recording_result = self.audio.lock().unwrap().start_recording();
                        if let Err(_e) = recording_result {
                            let mut state = self.state.lock().unwrap();
                            *state = AppState::Idle;
                        }
                    }
                    AppState::Recording | AppState::Processing => {
                        drop(state);
                    }
                }
            }

            /// Process hotkey release event.
            ///
            /// NOTE: This is a simplified synchronous test harness that processes
            /// transcription inline (unlike the real `HotkeyManager` which spawns a thread).
            /// This doesn't validate threading behavior or race conditions.
            fn on_release(&self) {
                let mut state = self.state.lock().unwrap();
                match *state {
                    AppState::Recording => {
                        *state = AppState::Processing;
                        drop(state);

                        let stop_result = self.audio.lock().unwrap().stop_recording();
                        match stop_result {
                            Ok(samples) => {
                                if let Some(engine) = &self.transcription {
                                    match engine.transcribe(&samples) {
                                        Ok(text) => {
                                            if !text.is_empty() {
                                                // Would call cgevent::insert_text_safe here
                                                // but we don't mock that in these tests
                                            }
                                        }
                                        Err(_e) => {}
                                    }
                                }
                                *self.state.lock().unwrap() = AppState::Idle;
                            }
                            Err(_e) => {
                                *self.state.lock().unwrap() = AppState::Idle;
                            }
                        }
                    }
                    AppState::Idle | AppState::Processing => {
                        drop(state);
                    }
                }
            }

            fn get_state(&self) -> AppState {
                *self.state.lock().unwrap()
            }
        }

        #[test]
        fn test_on_press_from_idle_starts_recording() {
            let mut mock_audio = MockAudioCapture::new();
            mock_audio
                .expect_start_recording()
                .times(1)
                .returning(|| Ok(()));

            let manager = TestHotkeyManager::new(mock_audio, None);
            assert_eq!(manager.get_state(), AppState::Idle);

            manager.on_press();
            assert_eq!(manager.get_state(), AppState::Recording);
        }

        #[test]
        fn test_on_press_from_recording_ignored() {
            let mut mock_audio = MockAudioCapture::new();
            mock_audio
                .expect_start_recording()
                .times(1)
                .returning(|| Ok(()));

            let manager = TestHotkeyManager::new(mock_audio, None);
            manager.on_press(); // First press: Idle → Recording
            assert_eq!(manager.get_state(), AppState::Recording);

            manager.on_press(); // Second press: ignored
            assert_eq!(manager.get_state(), AppState::Recording);
        }

        #[test]
        fn test_on_press_from_processing_ignored() {
            let mock_audio = MockAudioCapture::new();
            let manager = TestHotkeyManager::new(mock_audio, None);
            *manager.state.lock().unwrap() = AppState::Processing;

            manager.on_press();
            assert_eq!(manager.get_state(), AppState::Processing);
        }

        #[test]
        fn test_on_release_from_recording_stops_and_transcribes() {
            let mut mock_audio = MockAudioCapture::new();
            mock_audio
                .expect_start_recording()
                .times(1)
                .returning(|| Ok(()));
            mock_audio
                .expect_stop_recording()
                .times(1)
                .returning(|| Ok(vec![0.1, 0.2, 0.3]));

            let mut mock_transcription = MockTranscriptionEngine::new();
            mock_transcription
                .expect_transcribe()
                .times(1)
                .returning(|_| Ok("test text".to_owned()));

            let manager = TestHotkeyManager::new(mock_audio, Some(mock_transcription));
            manager.on_press(); // Idle → Recording
            manager.on_release(); // Recording → Processing → Idle

            assert_eq!(manager.get_state(), AppState::Idle);
        }

        #[test]
        fn test_on_release_from_idle_ignored() {
            let mock_audio = MockAudioCapture::new();
            let manager = TestHotkeyManager::new(mock_audio, None);
            assert_eq!(manager.get_state(), AppState::Idle);

            manager.on_release();
            assert_eq!(manager.get_state(), AppState::Idle);
        }

        #[test]
        fn test_on_release_with_empty_samples() {
            let mut mock_audio = MockAudioCapture::new();
            mock_audio
                .expect_start_recording()
                .times(1)
                .returning(|| Ok(()));
            mock_audio
                .expect_stop_recording()
                .times(1)
                .returning(|| Ok(vec![])); // Empty samples

            let manager = TestHotkeyManager::new(mock_audio, None);
            manager.on_press();
            manager.on_release();

            assert_eq!(manager.get_state(), AppState::Idle);
        }

        #[test]
        fn test_on_release_with_audio_error() {
            let mut mock_audio = MockAudioCapture::new();
            mock_audio
                .expect_start_recording()
                .times(1)
                .returning(|| Ok(()));
            mock_audio
                .expect_stop_recording()
                .times(1)
                .returning(|| Err(anyhow!("audio error")));

            let manager = TestHotkeyManager::new(mock_audio, None);
            manager.on_press();
            manager.on_release();

            assert_eq!(manager.get_state(), AppState::Idle);
        }

        #[test]
        fn test_process_transcription_success() {
            let mut mock_audio = MockAudioCapture::new();
            mock_audio
                .expect_start_recording()
                .times(1)
                .returning(|| Ok(()));
            mock_audio
                .expect_stop_recording()
                .times(1)
                .returning(|| Ok(vec![0.1, 0.2]));

            let mut mock_transcription = MockTranscriptionEngine::new();
            mock_transcription
                .expect_transcribe()
                .with(eq(&[0.1, 0.2][..]))
                .times(1)
                .returning(|_| Ok("transcribed text".to_owned()));

            let manager = TestHotkeyManager::new(mock_audio, Some(mock_transcription));
            manager.on_press();
            manager.on_release();

            assert_eq!(manager.get_state(), AppState::Idle);
        }

        #[test]
        fn test_process_transcription_failure() {
            let mut mock_audio = MockAudioCapture::new();
            mock_audio
                .expect_start_recording()
                .times(1)
                .returning(|| Ok(()));
            mock_audio
                .expect_stop_recording()
                .times(1)
                .returning(|| Ok(vec![0.1]));

            let mut mock_transcription = MockTranscriptionEngine::new();
            mock_transcription
                .expect_transcribe()
                .times(1)
                .returning(|_| {
                    Err(TranscriptionError::Transcription(anyhow!(
                        "transcription failed"
                    )))
                });

            let manager = TestHotkeyManager::new(mock_audio, Some(mock_transcription));
            manager.on_press();
            manager.on_release();

            assert_eq!(manager.get_state(), AppState::Idle);
        }

        #[test]
        fn test_process_transcription_empty_text() {
            let mut mock_audio = MockAudioCapture::new();
            mock_audio
                .expect_start_recording()
                .times(1)
                .returning(|| Ok(()));
            mock_audio
                .expect_stop_recording()
                .times(1)
                .returning(|| Ok(vec![0.0]));

            let mut mock_transcription = MockTranscriptionEngine::new();
            mock_transcription
                .expect_transcribe()
                .times(1)
                .returning(|_| Ok(String::new()));

            let manager = TestHotkeyManager::new(mock_audio, Some(mock_transcription));
            manager.on_press();
            manager.on_release();

            assert_eq!(manager.get_state(), AppState::Idle);
        }

        #[test]
        fn test_process_transcription_no_engine() {
            let mut mock_audio = MockAudioCapture::new();
            mock_audio
                .expect_start_recording()
                .times(1)
                .returning(|| Ok(()));
            mock_audio
                .expect_stop_recording()
                .times(1)
                .returning(|| Ok(vec![0.1]));

            let manager = TestHotkeyManager::new(mock_audio, None);
            manager.on_press();
            manager.on_release();

            assert_eq!(manager.get_state(), AppState::Idle);
        }

        #[test]
        fn test_on_press_with_start_recording_error() {
            let mut mock_audio = MockAudioCapture::new();
            mock_audio
                .expect_start_recording()
                .times(1)
                .returning(|| Err(anyhow!("failed to start")));

            let manager = TestHotkeyManager::new(mock_audio, None);
            manager.on_press();

            // Should revert to Idle on error
            assert_eq!(manager.get_state(), AppState::Idle);
        }

        #[test]
        fn test_debug_wav_path_formatting() {
            // This test verifies the debug WAV path string formatting logic
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_secs();
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_owned());
            let debug_path = std::path::PathBuf::from(home)
                .join(".whisper-hotkey")
                .join("debug")
                .join(format!("recording_{timestamp}.wav"));

            // Verify path structure
            assert!(debug_path.to_string_lossy().contains(".whisper-hotkey"));
            assert!(debug_path.to_string_lossy().contains("debug"));
            assert!(debug_path
                .to_string_lossy()
                .contains(&format!("recording_{timestamp}.wav")));
        }
    }
}
