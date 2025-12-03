use anyhow::{anyhow, Context, Result};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

use crate::audio::AudioCapture;
use crate::config::HotkeyConfig;
use crate::input::cgevent;
use crate::transcription::TranscriptionEngine;

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

/// Global hotkey manager with state tracking
pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    hotkey: HotKey,
    state: Arc<Mutex<AppState>>,
    audio: Arc<Mutex<AudioCapture>>,
    transcription: Option<Arc<TranscriptionEngine>>,
}

impl HotkeyManager {
    /// Create and register global hotkey from config
    ///
    /// # Errors
    /// Returns error if hotkey manager creation fails, unknown modifiers/keys, or registration fails
    pub fn new(
        config: &HotkeyConfig,
        audio: Arc<Mutex<AudioCapture>>,
        transcription: Option<Arc<TranscriptionEngine>>,
    ) -> Result<Self> {
        let manager = GlobalHotKeyManager::new().context("failed to create hotkey manager")?;

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
        })
    }

    /// Get shared state for external monitoring (e.g., UI updates)
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

                        // Save WAV debug file with error recovery
                        let timestamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                            .as_secs();
                        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_owned());
                        let debug_path = std::path::PathBuf::from(home)
                            .join(".whisper-hotkey")
                            .join("debug")
                            .join(format!("recording_{timestamp}.wav"));

                        if let Err(e) = AudioCapture::save_wav_debug(&samples, &debug_path) {
                            warn!(error = %e, path = ?debug_path, "failed to save debug WAV");
                        } else {
                            debug!(path = ?debug_path, "saved debug WAV");
                        }

                        // Phase 5 & 6: Transcription + Text Insertion (background thread with error recovery)
                        if let Some(engine) = &self.transcription {
                            let engine = Arc::clone(engine);
                            let state_arc = Arc::clone(&self.state);

                            std::thread::spawn(move || {
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

                                        // Insert text at cursor, only if non-empty
                                        if text.is_empty() {
                                            info!("🔇 No speech detected (silence or noise)");
                                        } else {
                                            if cgevent::insert_text_safe(&text) {
                                                info!(
                                                    text_len = text.len(),
                                                    "✅ Inserted {} chars",
                                                    text.len()
                                                );
                                            } else {
                                                warn!(
                                                    text_len = text.len(),
                                                    text_preview = %text_preview,
                                                    "❌ Text insertion failed - check permissions"
                                                );
                                            }
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

                                // Set state to Idle after processing (always recover)
                                let mut state = state_arc
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                                *state = AppState::Idle;
                                info!("✓ Ready for next recording");
                            });
                        } else {
                            warn!("⚠️  Transcription engine not available");
                            let mut state = self
                                .state
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            *state = AppState::Idle;
                            info!("✓ Ready for next recording (transcription disabled)");
                        }
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
}
