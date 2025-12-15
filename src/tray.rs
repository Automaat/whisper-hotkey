use anyhow::{anyhow, Context, Result};
use cocoa::appkit::NSScreen;
use cocoa::base::id;
use cocoa::foundation::NSArray;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

use crate::config::Config;
use crate::input::hotkey::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayCommand {
    OpenConfigFile,
    // Note: Quit removed - PredefinedMenuItem::quit() bypasses event system entirely
}

pub struct TrayManager {
    tray: tray_icon::TrayIcon,
    state: Arc<Mutex<AppState>>,
    current_icon_state: AppState,
    cached_icons: HashMap<AppState, Icon>,
}

impl TrayManager {
    pub fn new(config: &Config, state: Arc<Mutex<AppState>>) -> Result<Self> {
        // Detect display scale for proper retina support
        let scale = Self::detect_display_scale();

        // Preload all three icons into cache
        let mut cached_icons = HashMap::new();
        cached_icons.insert(AppState::Idle, Self::load_icon(AppState::Idle, scale)?);
        cached_icons.insert(
            AppState::Recording,
            Self::load_icon(AppState::Recording, scale)?,
        );
        cached_icons.insert(
            AppState::Processing,
            Self::load_icon(AppState::Processing, scale)?,
        );

        let tray = Self::build_tray(config, AppState::Idle, &cached_icons)?;

        Ok(Self {
            tray,
            state,
            current_icon_state: AppState::Idle,
            cached_icons,
        })
    }

    /// Detect display scale factor (1.0 for regular, 2.0 for retina)
    ///
    /// # Safety
    /// Uses Cocoa FFI to query `NSScreen` backing scale factor:
    /// - `NSScreen::screens()` returns a retained `NSArray` (non-null by Cocoa contract)
    /// - `objectAtIndex(0)` returns a valid `NSScreen*` for the lifetime of this function
    /// - `backingScaleFactor` is a safe getter with no side effects
    fn detect_display_scale() -> f64 {
        unsafe {
            let screens = NSScreen::screens(cocoa::base::nil);
            // NSScreen::screens() returns an autoreleased NSArray (non-null by Cocoa)
            if screens.is_null() || NSArray::count(screens) == 0 {
                // Fallback to retina if no screens detected (most modern Macs)
                return 2.0;
            }
            let screen: id = screens.objectAtIndex(0);
            NSScreen::backingScaleFactor(screen)
        }
    }

    fn build_tray(
        config: &Config,
        app_state: AppState,
        cached_icons: &HashMap<AppState, Icon>,
    ) -> Result<tray_icon::TrayIcon> {
        let icon = cached_icons
            .get(&app_state)
            .with_context(|| format!("icon for state {:?} not in cache", app_state))?
            .clone();
        let menu = Self::build_menu(config, Some(app_state))?;

        let mut builder = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Whisper Hotkey")
            .with_icon(icon);

        // Only use template mode for idle state (adaptive black/white)
        // Recording/processing states use colored icons
        if app_state == AppState::Idle {
            builder = builder.with_icon_as_template(true);
        }

        builder.build().context("failed to build tray icon")
    }

    fn load_icon(state: AppState, scale: f64) -> Result<Icon> {
        // Load appropriate icon based on state and display scale
        // Use 16px for @1x displays, 32px for @2x (retina) displays
        let size_suffix = if scale >= 2.0 { "32" } else { "16" };

        let icon_filename = match state {
            AppState::Idle => format!("icon-{size_suffix}.png"),
            AppState::Recording => format!("icon-recording-{size_suffix}.png"),
            AppState::Processing => format!("icon-processing-{size_suffix}.png"),
        };

        // Try to load from app bundle Resources folder first (for installed apps)
        // exe_path is something like: /Applications/WhisperHotkey.app/Contents/MacOS/WhisperHotkey
        // We want: /Applications/WhisperHotkey.app/Contents/Resources/icon-16.png or icon-32.png
        let icon_path = std::env::current_exe()
            .ok()
            .and_then(|exe_path| exe_path.parent().map(std::path::Path::to_path_buf))
            .and_then(|macos_dir| macos_dir.parent().map(std::path::Path::to_path_buf))
            .map(|contents_dir| contents_dir.join("Resources").join(&icon_filename))
            .filter(|path| path.exists())
            .unwrap_or_else(|| {
                // Fallback to assets directory (for development)
                std::path::PathBuf::from(format!(
                    "{}/assets/{}",
                    env!("CARGO_MANIFEST_DIR"),
                    icon_filename
                ))
            });

        tracing::debug!("loading icon for state {:?}: {:?}", state, icon_path);

        let image = image::open(&icon_path)
            .with_context(|| {
                format!(
                    "failed to load {} from {}",
                    icon_filename,
                    icon_path.display()
                )
            })?
            .into_rgba8();

        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        tracing::debug!("icon loaded: {}x{}, {} bytes", width, height, rgba.len());

        Icon::from_rgba(rgba, width, height).context("failed to create icon from RGBA data")
    }

    /// Update icon and menu if state changed
    pub fn update_icon_if_needed(&mut self, config: &Config) -> Result<()> {
        let new_state = *self
            .state
            .lock()
            .map_err(|e| anyhow!("state lock poisoned: {}", e))?;
        if new_state != self.current_icon_state {
            tracing::info!(
                "🔄 tray state change: {:?} -> {:?}",
                self.current_icon_state,
                new_state
            );

            // Rebuild entire tray with new state (workaround for macOS set_icon() bug)
            let new_tray = Self::build_tray(config, new_state, &self.cached_icons)?;
            self.tray = new_tray;

            self.current_icon_state = new_state;
            tracing::info!("✓ tray icon rebuilt with state: {:?}", new_state);
        }
        Ok(())
    }

    fn get_status_text(app_state: Option<AppState>) -> &'static str {
        app_state.map_or("Whisper Hotkey", |state| match state {
            AppState::Idle => "Whisper Hotkey - Ready",
            AppState::Recording => "🎤 Recording...",
            AppState::Processing => "⏳ Transcribing...",
        })
    }

    fn format_hotkey(mods: &[String], key: &str) -> String {
        if mods.is_empty() {
            key.to_owned()
        } else {
            format!("{}+{}", mods.join("+"), key)
        }
    }

    pub(crate) fn build_menu(config: &Config, app_state: Option<AppState>) -> Result<Menu> {
        let menu = Menu::new();

        // Status header
        let status = MenuItem::new(Self::get_status_text(app_state), false, None);
        menu.append(&status).context("failed to append status")?;
        menu.append(&PredefinedMenuItem::separator())?;

        // Profile list (read-only)
        for profile in &config.profiles {
            let hotkey_str = Self::format_hotkey(&profile.hotkey.modifiers, &profile.hotkey.key);
            let model_name = profile.model_type.as_str().to_owned();
            let profile_name = profile.name.as_ref().unwrap_or(&model_name);
            let label = format!(
                "{} ({}): {}",
                profile_name,
                hotkey_str,
                profile.model_type.as_str()
            );
            menu.append(&MenuItem::new(&label, false, None))?;
        }

        // Actions
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&MenuItem::with_id(
            "Open Config File",
            "Open Config File",
            true,
            None,
        ))?;
        menu.append(&PredefinedMenuItem::quit(None))?;

        Ok(menu)
    }

    pub fn poll_events() -> Option<TrayCommand> {
        use tray_icon::menu::MenuEvent;

        if let Ok(event) = MenuEvent::receiver().try_recv() {
            let id = event.id.0.as_str();
            tracing::debug!("tray menu event received: id={:?}", id);
            return Self::parse_menu_event(id);
        }

        None
    }

    fn parse_menu_event(id: &str) -> Option<TrayCommand> {
        match id {
            "Open Config File" => Some(TrayCommand::OpenConfigFile),
            // Note: "Quit" not handled here - PredefinedMenuItem::quit() uses native
            // macOS terminate: selector which bypasses event system entirely
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ModelType;

    #[test]
    fn test_parse_menu_event_open_config() {
        let cmd = TrayManager::parse_menu_event("Open Config File");
        assert!(matches!(cmd, Some(TrayCommand::OpenConfigFile)));
    }

    #[test]
    fn test_parse_menu_event_unknown() {
        assert!(TrayManager::parse_menu_event("Unknown Item").is_none());
        assert!(TrayManager::parse_menu_event("").is_none());
    }

    #[test]
    fn test_tray_command_debug() {
        let cmd = TrayCommand::OpenConfigFile;
        let debug_str = format!("{cmd:?}");
        assert!(debug_str.contains("OpenConfigFile"));
    }

    #[test]
    fn test_format_hotkey() {
        assert_eq!(
            TrayManager::format_hotkey(&["Command".to_owned(), "Shift".to_owned()], "V"),
            "Command+Shift+V"
        );
        assert_eq!(TrayManager::format_hotkey(&[], "V"), "V");
    }

    #[test]
    fn test_load_icon_idle() {
        let result = TrayManager::load_icon(AppState::Idle, 2.0);
        assert!(result.is_ok(), "Should load idle icon");
    }

    #[test]
    fn test_load_icon_recording() {
        let result = TrayManager::load_icon(AppState::Recording, 2.0);
        assert!(result.is_ok(), "Should load recording icon");
    }

    #[test]
    fn test_load_icon_processing() {
        let result = TrayManager::load_icon(AppState::Processing, 2.0);
        assert!(result.is_ok(), "Should load processing icon");
    }

    #[test]
    fn test_load_icon_scale_selection() {
        // Test that @1x displays load 16px icons
        let result = TrayManager::load_icon(AppState::Idle, 1.0);
        assert!(result.is_ok(), "Should load 16px icon for @1x display");

        // Test that @2x displays load 32px icons
        let result = TrayManager::load_icon(AppState::Idle, 2.0);
        assert!(result.is_ok(), "Should load 32px icon for @2x display");

        // Test edge case: exactly 2.0
        let result = TrayManager::load_icon(AppState::Recording, 2.0);
        assert!(result.is_ok(), "Should load 32px icon at scale=2.0");

        // Test all states work with both scales
        let result = TrayManager::load_icon(AppState::Processing, 1.0);
        assert!(result.is_ok(), "Should load 16px processing icon");
    }

    fn create_test_config() -> Config {
        use crate::config::{
            AliasesConfig, AudioConfig, HotkeyConfig, ModelConfig, RecordingConfig, TelemetryConfig,
        };
        Config {
            profiles: vec![crate::config::TranscriptionProfile {
                name: None,
                model_type: ModelType::Small,
                hotkey: HotkeyConfig {
                    modifiers: vec!["Control".to_owned(), "Option".to_owned()],
                    key: "Z".to_owned(),
                },
                preload: true,
                threads: 4,
                beam_size: 5,
                language: None,
            }],
            hotkey: HotkeyConfig {
                modifiers: vec!["Control".to_owned(), "Option".to_owned()],
                key: "Z".to_owned(),
            },
            audio: AudioConfig {
                buffer_size: 1024,
                sample_rate: 16000,
            },
            model: ModelConfig {
                model_type: ModelType::Small,
                preload: true,
                threads: 4,
                beam_size: 5,
                language: None,
            },
            telemetry: TelemetryConfig {
                enabled: true,
                log_path: "~/.whisper-hotkey/crash.log".to_owned(),
            },
            recording: RecordingConfig::default(),
            aliases: AliasesConfig::default(),
        }
    }

    #[test]
    #[ignore = "Requires main thread for macOS menu creation"]
    fn test_build_tray_with_all_states() {
        let config = create_test_config();
        let mut cached_icons = HashMap::new();
        cached_icons.insert(
            AppState::Idle,
            TrayManager::load_icon(AppState::Idle, 2.0).unwrap(),
        );
        cached_icons.insert(
            AppState::Recording,
            TrayManager::load_icon(AppState::Recording, 2.0).unwrap(),
        );
        cached_icons.insert(
            AppState::Processing,
            TrayManager::load_icon(AppState::Processing, 2.0).unwrap(),
        );

        let result = TrayManager::build_tray(&config, AppState::Idle, &cached_icons);
        assert!(result.is_ok());

        let result = TrayManager::build_tray(&config, AppState::Recording, &cached_icons);
        assert!(result.is_ok());

        let result = TrayManager::build_tray(&config, AppState::Processing, &cached_icons);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tray_missing_icon() {
        let config = create_test_config();
        let cached_icons = HashMap::new();

        let result = TrayManager::build_tray(&config, AppState::Idle, &cached_icons);
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "Requires full config and tray icon initialization"]
    fn test_state_icon_changes() {
        let state = Arc::new(Mutex::new(AppState::Idle));
        let config = Config::load().unwrap();
        let mut tray = TrayManager::new(&config, Arc::clone(&state)).unwrap();
        assert_eq!(tray.current_icon_state, AppState::Idle);

        *state.lock().unwrap() = AppState::Recording;
        let result = tray.update_icon_if_needed(&config);
        assert!(result.is_ok());
        assert_eq!(tray.current_icon_state, AppState::Recording);

        *state.lock().unwrap() = AppState::Processing;
        let result = tray.update_icon_if_needed(&config);
        assert!(result.is_ok());
        assert_eq!(tray.current_icon_state, AppState::Processing);

        *state.lock().unwrap() = AppState::Idle;
        let result = tray.update_icon_if_needed(&config);
        assert!(result.is_ok());
        assert_eq!(tray.current_icon_state, AppState::Idle);
    }

    #[test]
    fn test_get_status_text() {
        assert_eq!(
            TrayManager::get_status_text(Some(AppState::Idle)),
            "Whisper Hotkey - Ready"
        );
        assert_eq!(
            TrayManager::get_status_text(Some(AppState::Recording)),
            "🎤 Recording..."
        );
        assert_eq!(
            TrayManager::get_status_text(Some(AppState::Processing)),
            "⏳ Transcribing..."
        );
        assert_eq!(TrayManager::get_status_text(None), "Whisper Hotkey");
    }
}
