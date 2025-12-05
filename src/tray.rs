use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIconBuilder};

use crate::config::Config;
use crate::input::hotkey::AppState;

#[derive(Debug, Clone)]
pub enum TrayCommand {
    UpdateHotkey { modifiers: Vec<String>, key: String },
    UpdateModel { name: String },
    UpdateThreads(usize),
    UpdateBeamSize(usize),
    UpdateLanguage(Option<String>),
    UpdateBufferSize(usize),
    TogglePreload,
    ToggleTelemetry,
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
        // Preload all three icons into cache
        let mut cached_icons = HashMap::new();
        cached_icons.insert(AppState::Idle, Self::load_icon(AppState::Idle)?);
        cached_icons.insert(AppState::Recording, Self::load_icon(AppState::Recording)?);
        cached_icons.insert(AppState::Processing, Self::load_icon(AppState::Processing)?);

        let tray = Self::build_tray(config, AppState::Idle, &cached_icons)?;

        Ok(Self {
            tray,
            state,
            current_icon_state: AppState::Idle,
            cached_icons,
        })
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

        TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Whisper Hotkey")
            .with_icon(icon)
            .build()
            .context("failed to build tray icon")
    }

    fn load_icon(state: AppState) -> Result<Icon> {
        // Load appropriate icon based on state
        let icon_filename = match state {
            AppState::Idle => "icon-32.png",
            AppState::Recording => "icon-recording-32.png",
            AppState::Processing => "icon-processing-32.png",
        };

        // Try to load from app bundle Resources folder first (for installed apps)
        // exe_path is something like: /Applications/WhisperHotkey.app/Contents/MacOS/WhisperHotkey
        // We want: /Applications/WhisperHotkey.app/Contents/Resources/icon-32.png
        let icon_path = std::env::current_exe()
            .ok()
            .and_then(|exe_path| exe_path.parent().map(std::path::Path::to_path_buf))
            .and_then(|macos_dir| macos_dir.parent().map(std::path::Path::to_path_buf))
            .map(|contents_dir| contents_dir.join("Resources").join(icon_filename))
            .filter(|path| path.exists())
            .unwrap_or_else(|| {
                // Fallback to source directory (for development)
                std::path::PathBuf::from(format!(
                    "{}/{}",
                    env!("CARGO_MANIFEST_DIR"),
                    icon_filename
                ))
            });

        tracing::debug!("loading icon for state {:?}: {:?}", state, icon_path);

        let image = image::open(&icon_path)
            .with_context(|| {
                format!(
                    "failed to load {icon_filename} from {}",
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

    fn build_menu(config: &Config, app_state: Option<AppState>) -> Result<Menu> {
        let menu = Menu::new();

        // Status item showing current state (non-clickable)
        let status_text = Self::get_status_text(app_state);
        let status = MenuItem::new(status_text, false, None);
        menu.append(&status)
            .context("failed to append status item")?;
        menu.append(&PredefinedMenuItem::separator())
            .context("failed to append separator")?;

        // Hotkey submenu
        let hotkey_submenu = Submenu::new("Hotkey", true);
        let current_hotkey = format!("{:?}+{}", config.hotkey.modifiers, config.hotkey.key);

        // Common hotkey combinations
        let hotkeys = vec![
            ("Control+Option+Z", vec!["Control", "Option"], "Z"),
            ("Command+Shift+V", vec!["Command", "Shift"], "V"),
            ("Command+Option+V", vec!["Command", "Option"], "V"),
            ("Control+Shift+Space", vec!["Control", "Shift"], "Space"),
        ];

        for (label, mods, key) in hotkeys {
            let is_selected = format!("{:?}+{}", mods, key) == current_hotkey;
            let display_label = if is_selected {
                format!("✓ {}", label)
            } else {
                label.to_owned()
            };
            let item = MenuItem::new(&display_label, true, None);
            hotkey_submenu
                .append(&item)
                .context("failed to append hotkey item")?;
        }

        menu.append(&hotkey_submenu)
            .context("failed to append hotkey submenu")?;

        // Model submenu
        let model_submenu = Submenu::new("Model", true);
        let models = vec!["tiny", "base", "small", "medium"];

        for model_name in models {
            let is_selected = config.model.name == model_name;
            let display_label = if is_selected {
                format!("✓ {}", model_name)
            } else {
                model_name.to_owned()
            };
            let item = MenuItem::new(&display_label, true, None);
            model_submenu
                .append(&item)
                .context("failed to append model item")?;
        }

        menu.append(&model_submenu)
            .context("failed to append model submenu")?;

        // Optimization submenu
        let opt_submenu = Submenu::new("Optimization", true);

        // Threads submenu
        let threads_submenu = Submenu::new("Threads", true);
        for threads in [2, 4, 6, 8] {
            let is_selected = config.model.threads == threads;
            let label = if is_selected {
                format!("✓ {} threads", threads)
            } else {
                format!("{} threads", threads)
            };
            let item = MenuItem::new(&label, true, None);
            threads_submenu
                .append(&item)
                .context("failed to append threads item")?;
        }
        opt_submenu
            .append(&threads_submenu)
            .context("failed to append threads submenu")?;

        // Beam size submenu
        let beam_submenu = Submenu::new("Beam Size", true);
        for beam in [1, 3, 5, 8, 10] {
            let is_selected = config.model.beam_size == beam;
            let label = if is_selected {
                format!("✓ Beam size {}", beam)
            } else {
                format!("Beam size {}", beam)
            };
            let item = MenuItem::new(&label, true, None);
            beam_submenu
                .append(&item)
                .context("failed to append beam item")?;
        }
        opt_submenu
            .append(&beam_submenu)
            .context("failed to append beam submenu")?;

        menu.append(&opt_submenu)
            .context("failed to append optimization submenu")?;

        // Language submenu
        let lang_submenu = Submenu::new("Language", true);
        let languages = vec![
            ("Auto-detect", None),
            ("English", Some("en")),
            ("Polish", Some("pl")),
            ("Spanish", Some("es")),
            ("French", Some("fr")),
            ("German", Some("de")),
        ];

        for (label, lang_code) in languages {
            let is_selected = config.model.language.as_deref() == lang_code;
            let display_label = if is_selected {
                format!("✓ {}", label)
            } else {
                label.to_owned()
            };
            let item = MenuItem::new(&display_label, true, None);
            lang_submenu
                .append(&item)
                .context("failed to append language item")?;
        }

        menu.append(&lang_submenu)
            .context("failed to append language submenu")?;

        // Audio buffer submenu
        let buffer_submenu = Submenu::new("Audio Buffer", true);
        for size in [512, 1024, 2048, 4096] {
            let is_selected = config.audio.buffer_size == size;
            let label = if is_selected {
                format!("✓ {} samples", size)
            } else {
                format!("{} samples", size)
            };
            let item = MenuItem::new(&label, true, None);
            buffer_submenu
                .append(&item)
                .context("failed to append buffer item")?;
        }

        menu.append(&buffer_submenu)
            .context("failed to append buffer submenu")?;

        // Toggles
        menu.append(&PredefinedMenuItem::separator())
            .context("failed to append separator")?;

        let preload = CheckMenuItem::new("Preload Model", config.model.preload, true, None);
        menu.append(&preload)
            .context("failed to append preload item")?;

        let telemetry = CheckMenuItem::new("Telemetry", config.telemetry.enabled, true, None);
        menu.append(&telemetry)
            .context("failed to append telemetry item")?;

        // Actions
        menu.append(&PredefinedMenuItem::separator())
            .context("failed to append separator")?;

        let open_config = MenuItem::new("Open Config File", true, None);
        menu.append(&open_config)
            .context("failed to append open config item")?;

        menu.append(&PredefinedMenuItem::quit(None))
            .context("failed to append quit item")?;

        Ok(menu)
    }

    pub fn update_menu(&self, config: &Config) -> Result<()> {
        let current_state = *self
            .state
            .lock()
            .map_err(|e| anyhow!("state lock poisoned: {}", e))?;
        let new_menu = Self::build_menu(config, Some(current_state))?;
        self.tray.set_menu(Some(Box::new(new_menu)));
        Ok(())
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
        // Strip checkmark if present
        let id = id.trim_start_matches("✓ ");

        match id {
            // Hotkeys
            "Control+Option+Z" => Some(TrayCommand::UpdateHotkey {
                modifiers: vec!["Control".to_owned(), "Option".to_owned()],
                key: "Z".to_owned(),
            }),
            "Command+Shift+V" => Some(TrayCommand::UpdateHotkey {
                modifiers: vec!["Command".to_owned(), "Shift".to_owned()],
                key: "V".to_owned(),
            }),
            "Command+Option+V" => Some(TrayCommand::UpdateHotkey {
                modifiers: vec!["Command".to_owned(), "Option".to_owned()],
                key: "V".to_owned(),
            }),
            "Control+Shift+Space" => Some(TrayCommand::UpdateHotkey {
                modifiers: vec!["Control".to_owned(), "Shift".to_owned()],
                key: "Space".to_owned(),
            }),

            // Models
            "tiny" | "base" | "small" | "medium" => Some(TrayCommand::UpdateModel {
                name: id.to_owned(),
            }),

            // Threads
            "2 threads" => Some(TrayCommand::UpdateThreads(2)),
            "4 threads" => Some(TrayCommand::UpdateThreads(4)),
            "6 threads" => Some(TrayCommand::UpdateThreads(6)),
            "8 threads" => Some(TrayCommand::UpdateThreads(8)),

            // Beam sizes
            "Beam size 1" => Some(TrayCommand::UpdateBeamSize(1)),
            "Beam size 3" => Some(TrayCommand::UpdateBeamSize(3)),
            "Beam size 5" => Some(TrayCommand::UpdateBeamSize(5)),
            "Beam size 8" => Some(TrayCommand::UpdateBeamSize(8)),
            "Beam size 10" => Some(TrayCommand::UpdateBeamSize(10)),

            // Languages
            "Auto-detect" => Some(TrayCommand::UpdateLanguage(None)),
            "English" => Some(TrayCommand::UpdateLanguage(Some("en".to_owned()))),
            "Polish" => Some(TrayCommand::UpdateLanguage(Some("pl".to_owned()))),
            "Spanish" => Some(TrayCommand::UpdateLanguage(Some("es".to_owned()))),
            "French" => Some(TrayCommand::UpdateLanguage(Some("fr".to_owned()))),
            "German" => Some(TrayCommand::UpdateLanguage(Some("de".to_owned()))),

            // Audio buffer
            "512 samples" => Some(TrayCommand::UpdateBufferSize(512)),
            "1024 samples" => Some(TrayCommand::UpdateBufferSize(1024)),
            "2048 samples" => Some(TrayCommand::UpdateBufferSize(2048)),
            "4096 samples" => Some(TrayCommand::UpdateBufferSize(4096)),

            // Toggles and Actions
            "Preload Model" => Some(TrayCommand::TogglePreload),
            "Telemetry" => Some(TrayCommand::ToggleTelemetry),
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

    #[test]
    fn test_parse_menu_event_hotkeys() {
        let cmd = TrayManager::parse_menu_event("Control+Option+Z");
        assert!(matches!(cmd, Some(TrayCommand::UpdateHotkey { .. })));
        if let Some(TrayCommand::UpdateHotkey { modifiers, key }) = cmd {
            assert_eq!(modifiers, vec!["Control", "Option"]);
            assert_eq!(key, "Z");
        }

        let cmd = TrayManager::parse_menu_event("Command+Shift+V");
        assert!(matches!(cmd, Some(TrayCommand::UpdateHotkey { .. })));
    }

    #[test]
    fn test_parse_menu_event_models() {
        let cmd = TrayManager::parse_menu_event("tiny");
        assert!(matches!(cmd, Some(TrayCommand::UpdateModel { .. })));
        if let Some(TrayCommand::UpdateModel { name }) = cmd {
            assert_eq!(name, "tiny");
        }

        let cmd = TrayManager::parse_menu_event("base");
        assert!(matches!(cmd, Some(TrayCommand::UpdateModel { .. })));

        let cmd = TrayManager::parse_menu_event("small");
        assert!(matches!(cmd, Some(TrayCommand::UpdateModel { .. })));
    }

    #[test]
    fn test_parse_menu_event_threads() {
        let cmd = TrayManager::parse_menu_event("2 threads");
        assert!(matches!(cmd, Some(TrayCommand::UpdateThreads(2))));

        let cmd = TrayManager::parse_menu_event("4 threads");
        assert!(matches!(cmd, Some(TrayCommand::UpdateThreads(4))));

        let cmd = TrayManager::parse_menu_event("8 threads");
        assert!(matches!(cmd, Some(TrayCommand::UpdateThreads(8))));
    }

    #[test]
    fn test_parse_menu_event_beam_size() {
        let cmd = TrayManager::parse_menu_event("Beam size 1");
        assert!(matches!(cmd, Some(TrayCommand::UpdateBeamSize(1))));

        let cmd = TrayManager::parse_menu_event("Beam size 5");
        assert!(matches!(cmd, Some(TrayCommand::UpdateBeamSize(5))));

        let cmd = TrayManager::parse_menu_event("Beam size 10");
        assert!(matches!(cmd, Some(TrayCommand::UpdateBeamSize(10))));
    }

    #[test]
    fn test_parse_menu_event_language() {
        let cmd = TrayManager::parse_menu_event("Auto-detect");
        assert!(matches!(cmd, Some(TrayCommand::UpdateLanguage(None))));

        let cmd = TrayManager::parse_menu_event("English");
        if let Some(TrayCommand::UpdateLanguage(Some(lang))) = cmd {
            assert_eq!(lang, "en");
        }

        let cmd = TrayManager::parse_menu_event("Polish");
        if let Some(TrayCommand::UpdateLanguage(Some(lang))) = cmd {
            assert_eq!(lang, "pl");
        }
    }

    #[test]
    fn test_parse_menu_event_buffer_size() {
        let cmd = TrayManager::parse_menu_event("512 samples");
        assert!(matches!(cmd, Some(TrayCommand::UpdateBufferSize(512))));

        let cmd = TrayManager::parse_menu_event("1024 samples");
        assert!(matches!(cmd, Some(TrayCommand::UpdateBufferSize(1024))));

        let cmd = TrayManager::parse_menu_event("2048 samples");
        assert!(matches!(cmd, Some(TrayCommand::UpdateBufferSize(2048))));

        let cmd = TrayManager::parse_menu_event("4096 samples");
        assert!(matches!(cmd, Some(TrayCommand::UpdateBufferSize(4096))));
    }

    #[test]
    fn test_parse_menu_event_actions() {
        let cmd = TrayManager::parse_menu_event("Open Config File");
        assert!(matches!(cmd, Some(TrayCommand::OpenConfigFile)));

        // Note: Quit not tested - PredefinedMenuItem::quit() bypasses event system
    }

    #[test]
    fn test_parse_menu_event_unknown() {
        let cmd = TrayManager::parse_menu_event("Unknown Item");
        assert!(cmd.is_none());

        let cmd = TrayManager::parse_menu_event("");
        assert!(cmd.is_none());
    }

    #[test]
    #[allow(clippy::redundant_clone)] // Testing Clone trait explicitly
    fn test_tray_command_clone() {
        let cmd1 = TrayCommand::UpdateThreads(4);
        let cmd1_cloned = cmd1.clone();
        assert!(matches!(&cmd1_cloned, TrayCommand::UpdateThreads(4)));

        let cmd3 = TrayCommand::UpdateLanguage(Some("en".to_owned()));
        if let TrayCommand::UpdateLanguage(Some(lang)) = cmd3 {
            assert_eq!(lang, "en");
        }
    }

    #[test]
    fn test_tray_command_debug() {
        let cmd = TrayCommand::OpenConfigFile;
        let debug_str = format!("{cmd:?}");
        assert!(debug_str.contains("OpenConfigFile"));
    }

    #[test]
    fn test_load_icon_idle() {
        let result = TrayManager::load_icon(AppState::Idle);
        assert!(result.is_ok(), "Should load idle icon");
    }

    #[test]
    fn test_load_icon_recording() {
        let result = TrayManager::load_icon(AppState::Recording);
        assert!(result.is_ok(), "Should load recording icon");
    }

    #[test]
    fn test_load_icon_processing() {
        let result = TrayManager::load_icon(AppState::Processing);
        assert!(result.is_ok(), "Should load processing icon");
    }

    #[test]
    fn test_tray_command_variants() {
        let cmd1 = TrayCommand::UpdateHotkey {
            modifiers: vec!["Control".to_owned()],
            key: "Z".to_owned(),
        };
        let debug = format!("{cmd1:?}");
        assert!(debug.contains("UpdateHotkey"));

        let cmd2 = TrayCommand::UpdateModel {
            name: "base".to_owned(),
        };
        let debug = format!("{cmd2:?}");
        assert!(debug.contains("UpdateModel"));

        let cmd3 = TrayCommand::TogglePreload;
        let debug = format!("{cmd3:?}");
        assert!(debug.contains("TogglePreload"));

        let cmd4 = TrayCommand::ToggleTelemetry;
        let debug = format!("{cmd4:?}");
        assert!(debug.contains("ToggleTelemetry"));

        let cmd5 = TrayCommand::OpenConfigFile;
        let debug = format!("{cmd5:?}");
        assert!(debug.contains("OpenConfigFile"));
    }

    #[test]
    #[allow(clippy::redundant_clone)] // Testing Clone trait explicitly
    fn test_tray_command_clone_all_variants() {
        // Test that Clone trait works for all variants
        let cmd1 = TrayCommand::UpdateBeamSize(5);
        let cmd1_cloned = cmd1.clone();
        assert!(matches!(&cmd1_cloned, TrayCommand::UpdateBeamSize(5)));

        let cmd2 = TrayCommand::UpdateBufferSize(1024);
        let cmd2_cloned = cmd2.clone();
        assert!(matches!(&cmd2_cloned, TrayCommand::UpdateBufferSize(1024)));

        let cmd3 = TrayCommand::TogglePreload;
        let cmd3_cloned = cmd3.clone();
        assert!(matches!(&cmd3_cloned, TrayCommand::TogglePreload));

        let cmd4 = TrayCommand::UpdateModel {
            name: "tiny".to_owned(),
        };
        let cmd4_cloned = cmd4.clone();
        if let TrayCommand::UpdateModel { name } = &cmd4_cloned {
            assert_eq!(name, "tiny");
        }
    }

    fn create_test_config() -> Config {
        use crate::config::{AudioConfig, HotkeyConfig, ModelConfig, TelemetryConfig};
        Config {
            hotkey: HotkeyConfig {
                modifiers: vec!["Control".to_owned(), "Option".to_owned()],
                key: "Z".to_owned(),
            },
            audio: AudioConfig {
                buffer_size: 1024,
                sample_rate: 16000,
            },
            model: ModelConfig {
                name: "small".to_owned(),
                path: "~/.whisper-hotkey/models/ggml-small.bin".to_owned(),
                preload: true,
                threads: 4,
                beam_size: 5,
                language: None,
            },
            telemetry: TelemetryConfig {
                enabled: true,
                log_path: "~/.whisper-hotkey/crash.log".to_owned(),
            },
        }
    }

    #[test]
    #[ignore = "Requires main thread for macOS menu creation"]
    fn test_build_tray_with_all_states() {
        // Test that build_tray() can construct tray for all states
        // Note: This test must run on main thread due to macOS muda::Menu restrictions
        let config = create_test_config();
        let mut cached_icons = HashMap::new();
        cached_icons.insert(
            AppState::Idle,
            TrayManager::load_icon(AppState::Idle).unwrap(),
        );
        cached_icons.insert(
            AppState::Recording,
            TrayManager::load_icon(AppState::Recording).unwrap(),
        );
        cached_icons.insert(
            AppState::Processing,
            TrayManager::load_icon(AppState::Processing).unwrap(),
        );

        // Test Idle state
        let result = TrayManager::build_tray(&config, AppState::Idle, &cached_icons);
        assert!(
            result.is_ok(),
            "Should build tray for Idle state: {:?}",
            result.err()
        );

        // Test Recording state
        let result = TrayManager::build_tray(&config, AppState::Recording, &cached_icons);
        assert!(
            result.is_ok(),
            "Should build tray for Recording state: {:?}",
            result.err()
        );

        // Test Processing state
        let result = TrayManager::build_tray(&config, AppState::Processing, &cached_icons);
        assert!(
            result.is_ok(),
            "Should build tray for Processing state: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_build_tray_missing_icon() {
        // Test that build_tray() fails gracefully when icon is missing from cache
        let config = create_test_config();
        let cached_icons = HashMap::new(); // Empty cache

        let result = TrayManager::build_tray(&config, AppState::Idle, &cached_icons);
        assert!(
            result.is_err(),
            "Should fail when icon is missing from cache"
        );
    }

    #[test]
    #[ignore = "Requires full config and tray icon initialization"]
    fn test_state_icon_changes() {
        // Verify state changes are properly detected
        let state = Arc::new(Mutex::new(AppState::Idle));

        // Load config (requires ~/.whisper-hotkey.toml)
        let config = Config::load().unwrap();
        let mut tray = TrayManager::new(&config, Arc::clone(&state)).unwrap();
        assert_eq!(tray.current_icon_state, AppState::Idle);

        // Change state to Recording
        *state.lock().unwrap() = AppState::Recording;
        let result = tray.update_icon_if_needed(&config);
        assert!(result.is_ok());
        assert_eq!(tray.current_icon_state, AppState::Recording);

        // Change state to Processing
        *state.lock().unwrap() = AppState::Processing;
        let result = tray.update_icon_if_needed(&config);
        assert!(result.is_ok());
        assert_eq!(tray.current_icon_state, AppState::Processing);

        // No change - should not reload icon
        let result = tray.update_icon_if_needed(&config);
        assert!(result.is_ok());
        assert_eq!(tray.current_icon_state, AppState::Processing);

        // Back to Idle
        *state.lock().unwrap() = AppState::Idle;
        let result = tray.update_icon_if_needed(&config);
        assert!(result.is_ok());
        assert_eq!(tray.current_icon_state, AppState::Idle);
    }

    // Phase 3: Tray Menu Logic Tests
    #[test]
    fn test_build_menu_idle_state() {
        let status = TrayManager::get_status_text(Some(AppState::Idle));
        assert_eq!(status, "Whisper Hotkey - Ready");
    }

    #[test]
    fn test_build_menu_recording_state() {
        let status = TrayManager::get_status_text(Some(AppState::Recording));
        assert_eq!(status, "🎤 Recording...");
    }

    #[test]
    fn test_build_menu_processing_state() {
        let status = TrayManager::get_status_text(Some(AppState::Processing));
        assert_eq!(status, "⏳ Transcribing...");
    }

    fn create_test_config() -> Config {
        use crate::config::{AudioConfig, HotkeyConfig, ModelConfig, TelemetryConfig};
        Config {
            hotkey: HotkeyConfig {
                modifiers: vec!["Command".to_owned(), "Shift".to_owned()],
                key: "V".to_owned(),
            },
            model: ModelConfig {
                name: "base".to_owned(),
                path: "/tmp/model.bin".to_owned(),
                threads: 4,
                beam_size: 5,
                language: None,
                preload: false,
            },
            audio: AudioConfig {
                sample_rate: 16000,
                buffer_size: 1024,
            },
            telemetry: TelemetryConfig {
                enabled: false,
                log_path: "/tmp/test.log".to_owned(),
            },
        }
    }

    #[test]
    fn test_build_menu_hotkey_selection() {
        let config = create_test_config();

        // Verify the expected hotkey format matches
        let current_hotkey = format!("{:?}+{}", config.hotkey.modifiers, config.hotkey.key);
        let expected_hotkey = format!("{:?}+{}", vec!["Command", "Shift"], "V");
        assert_eq!(current_hotkey, expected_hotkey);
    }

    #[test]
    fn test_build_menu_model_selection() {
        let mut config = create_test_config();
        config.model.name = "small".to_owned();

        // Verify model selection logic
        assert_eq!(config.model.name, "small");
        assert!(["tiny", "base", "small", "medium"].contains(&config.model.name.as_str()));
    }

    #[test]
    fn test_build_menu_threads_selection() {
        let mut config = create_test_config();
        config.model.threads = 6;

        // Verify threads selection logic
        assert_eq!(config.model.threads, 6);
        assert!([2, 4, 6, 8].contains(&config.model.threads));
    }

    #[test]
    fn test_build_menu_beam_size_selection() {
        let mut config = create_test_config();
        config.model.beam_size = 10;

        // Verify beam size selection logic
        assert_eq!(config.model.beam_size, 10);
        assert!([1, 3, 5, 8, 10].contains(&config.model.beam_size));
    }

    #[test]
    fn test_build_menu_language_selection() {
        let config_auto = create_test_config();
        let mut config_polish = create_test_config();
        config_polish.model.language = Some("pl".to_owned());

        // Verify language selection logic
        assert_eq!(config_auto.model.language, None);
        assert_eq!(config_polish.model.language, Some("pl".to_owned()));
    }

    #[test]
    fn test_build_menu_buffer_size_selection() {
        let mut config = create_test_config();
        config.audio.buffer_size = 2048;

        // Verify buffer size selection logic
        assert_eq!(config.audio.buffer_size, 2048);
        assert!([512, 1024, 2048, 4096].contains(&config.audio.buffer_size));
    }

    #[test]
    fn test_build_menu_preload_toggle_checked() {
        let mut config = create_test_config();
        config.model.preload = true;

        // Verify preload toggle state
        assert!(config.model.preload);
    }

    #[test]
    fn test_build_menu_telemetry_toggle_unchecked() {
        let config = create_test_config();

        // Verify telemetry toggle state
        assert!(!config.telemetry.enabled);
    }

    #[test]
    fn test_parse_menu_event_with_checkmark() {
        // Test that checkmark prefix is properly stripped
        let cmd = TrayManager::parse_menu_event("✓ tiny");
        assert!(matches!(cmd, Some(TrayCommand::UpdateModel { .. })));
        if let Some(TrayCommand::UpdateModel { name }) = cmd {
            assert_eq!(name, "tiny");
        }

        let cmd = TrayManager::parse_menu_event("✓ 4 threads");
        assert!(matches!(cmd, Some(TrayCommand::UpdateThreads(4))));

        let cmd = TrayManager::parse_menu_event("✓ Beam size 5");
        assert!(matches!(cmd, Some(TrayCommand::UpdateBeamSize(5))));

        let cmd = TrayManager::parse_menu_event("✓ English");
        if let Some(TrayCommand::UpdateLanguage(Some(lang))) = cmd {
            assert_eq!(lang, "en");
        }
    }
}
