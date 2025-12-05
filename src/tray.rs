use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIconBuilder};

use crate::config::Config;
use crate::input::hotkey::AppState;

/// Menu configuration data (pure, testable)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuConfig {
    pub status_text: &'static str,
    pub hotkeys: Vec<HotkeyOption>,
    pub models: Vec<ModelOption>,
    pub threads: Vec<ThreadOption>,
    pub beam_sizes: Vec<BeamSizeOption>,
    pub languages: Vec<LanguageOption>,
    pub buffer_sizes: Vec<BufferSizeOption>,
    pub preload_enabled: bool,
    pub telemetry_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyOption {
    pub label: String,
    pub modifiers: Vec<String>,
    pub key: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelOption {
    pub name: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadOption {
    pub count: usize,
    pub label: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeamSizeOption {
    pub size: usize,
    pub label: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageOption {
    pub display_name: String,
    pub code: Option<String>,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferSizeOption {
    pub size: usize,
    pub label: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    fn format_hotkey_display(mods: &[String], key: &str) -> String {
        format!("{:?}+{}", mods, key)
    }

    fn is_hotkey_selected(config: &Config, mods: &[&str], key: &str) -> bool {
        let current = Self::format_hotkey_display(&config.hotkey.modifiers, &config.hotkey.key);
        let candidate = format!("{:?}+{}", mods, key);
        current == candidate
    }

    fn format_label_with_checkmark(label: &str, is_selected: bool) -> String {
        if is_selected {
            format!("✓ {}", label)
        } else {
            label.to_owned()
        }
    }

    /// Build menu configuration from app config (pure, testable)
    fn build_menu_config(config: &Config, app_state: Option<AppState>) -> MenuConfig {
        // Hotkey options
        let hotkey_definitions = [
            ("Control+Option+Z", vec!["Control", "Option"], "Z"),
            ("Command+Shift+V", vec!["Command", "Shift"], "V"),
            ("Command+Option+V", vec!["Command", "Option"], "V"),
            ("Control+Shift+Space", vec!["Control", "Shift"], "Space"),
        ];

        let hotkeys = hotkey_definitions
            .iter()
            .map(|(label, mods, key)| HotkeyOption {
                label: (*label).to_owned(),
                modifiers: mods.iter().map(|s| (*s).to_owned()).collect(),
                key: (*key).to_owned(),
                selected: Self::is_hotkey_selected(config, mods, key),
            })
            .collect();

        // Model options
        let model_names = ["tiny", "base", "small", "medium"];
        let models = model_names
            .iter()
            .map(|name| ModelOption {
                name: (*name).to_owned(),
                selected: config.model.name == *name,
            })
            .collect();

        // Thread options
        let thread_counts = [2, 4, 6, 8];
        let threads = thread_counts
            .iter()
            .map(|&count| ThreadOption {
                count,
                label: format!("{} threads", count),
                selected: config.model.threads == count,
            })
            .collect();

        // Beam size options
        let beam_sizes_list = [1, 3, 5, 8, 10];
        let beam_sizes = beam_sizes_list
            .iter()
            .map(|&size| BeamSizeOption {
                size,
                label: format!("Beam size {}", size),
                selected: config.model.beam_size == size,
            })
            .collect();

        // Language options
        let language_definitions = [
            ("Auto-detect", None),
            ("English", Some("en")),
            ("Polish", Some("pl")),
            ("Spanish", Some("es")),
            ("French", Some("fr")),
            ("German", Some("de")),
        ];

        let languages = language_definitions
            .iter()
            .map(|(display, code)| LanguageOption {
                display_name: (*display).to_owned(),
                code: code.map(std::borrow::ToOwned::to_owned),
                selected: config.model.language.as_deref() == *code,
            })
            .collect();

        // Buffer size options
        let buffer_sizes_list = [512, 1024, 2048, 4096];
        let buffer_sizes = buffer_sizes_list
            .iter()
            .map(|&size| BufferSizeOption {
                size,
                label: format!("{} samples", size),
                selected: config.audio.buffer_size == size,
            })
            .collect();

        MenuConfig {
            status_text: Self::get_status_text(app_state),
            hotkeys,
            models,
            threads,
            beam_sizes,
            languages,
            buffer_sizes,
            preload_enabled: config.model.preload,
            telemetry_enabled: config.telemetry.enabled,
        }
    }

    pub(crate) fn build_menu(config: &Config, app_state: Option<AppState>) -> Result<Menu> {
        // Build pure configuration (testable logic)
        let menu_config = Self::build_menu_config(config, app_state);

        // Create menu using configuration (FFI calls)
        let menu = Menu::new();

        // Status
        let status = MenuItem::new(menu_config.status_text, false, None);
        menu.append(&status).context("failed to append status")?;
        menu.append(&PredefinedMenuItem::separator())?;

        // Hotkeys
        let hotkey_submenu = Submenu::new("Hotkey", true);
        for hotkey in &menu_config.hotkeys {
            let label = Self::format_label_with_checkmark(&hotkey.label, hotkey.selected);
            hotkey_submenu.append(&MenuItem::new(&label, true, None))?;
        }
        menu.append(&hotkey_submenu)?;

        // Models
        let model_submenu = Submenu::new("Model", true);
        for model in &menu_config.models {
            let label = Self::format_label_with_checkmark(&model.name, model.selected);
            model_submenu.append(&MenuItem::new(&label, true, None))?;
        }
        menu.append(&model_submenu)?;

        // Optimization submenu
        let opt_submenu = Submenu::new("Optimization", true);

        // Threads
        let threads_submenu = Submenu::new("Threads", true);
        for thread in &menu_config.threads {
            let label = Self::format_label_with_checkmark(&thread.label, thread.selected);
            threads_submenu.append(&MenuItem::new(&label, true, None))?;
        }
        opt_submenu.append(&threads_submenu)?;

        // Beam sizes
        let beam_submenu = Submenu::new("Beam Size", true);
        for beam in &menu_config.beam_sizes {
            let label = Self::format_label_with_checkmark(&beam.label, beam.selected);
            beam_submenu.append(&MenuItem::new(&label, true, None))?;
        }
        opt_submenu.append(&beam_submenu)?;

        menu.append(&opt_submenu)?;

        // Languages
        let lang_submenu = Submenu::new("Language", true);
        for lang in &menu_config.languages {
            let label = Self::format_label_with_checkmark(&lang.display_name, lang.selected);
            lang_submenu.append(&MenuItem::new(&label, true, None))?;
        }
        menu.append(&lang_submenu)?;

        // Buffer sizes
        let buffer_submenu = Submenu::new("Audio Buffer", true);
        for buffer in &menu_config.buffer_sizes {
            let label = Self::format_label_with_checkmark(&buffer.label, buffer.selected);
            buffer_submenu.append(&MenuItem::new(&label, true, None))?;
        }
        menu.append(&buffer_submenu)?;

        // Toggles
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&CheckMenuItem::new(
            "Preload Model",
            menu_config.preload_enabled,
            true,
            None,
        ))?;
        menu.append(&CheckMenuItem::new(
            "Telemetry",
            menu_config.telemetry_enabled,
            true,
            None,
        ))?;

        // Actions
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&MenuItem::new("Open Config File", true, None))?;
        menu.append(&PredefinedMenuItem::quit(None))?;

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
        use crate::config::{
            AudioConfig, HotkeyConfig, ModelConfig, RecordingConfig, TelemetryConfig,
        };
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
            recording: RecordingConfig::default(),
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

    #[test]
    fn test_build_menu_none_state() {
        let status = TrayManager::get_status_text(None);
        assert_eq!(status, "Whisper Hotkey");
    }

    fn create_menu_test_config() -> Config {
        use crate::config::{
            AudioConfig, HotkeyConfig, ModelConfig, RecordingConfig, TelemetryConfig,
        };
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
            recording: RecordingConfig::default(),
        }
    }

    #[test]
    #[ignore = "requires main thread for Menu creation on macOS"]
    fn test_build_menu_hotkey_selection() {
        let config = create_menu_test_config(); // Command+Shift+V
        let menu = TrayManager::build_menu(&config, Some(AppState::Idle)).unwrap();

        // Find Hotkey submenu
        let menu_items = menu.items();
        let hotkey_submenu = menu_items
            .iter()
            .find_map(|item| item.as_submenu().filter(|s| s.text() == "Hotkey"))
            .expect("Hotkey submenu not found");

        // Verify selected hotkey has checkmark
        let hotkey_items = hotkey_submenu.items();
        let has_selected = hotkey_items
            .iter()
            .filter_map(|item| item.as_menuitem())
            .any(|mi| mi.text() == "✓ Command+Shift+V");
        assert!(has_selected, "Selected hotkey should have checkmark");

        // Verify all 4 hotkey options exist
        let expected_hotkeys = [
            "Control+Option+Z",
            "Command+Shift+V",
            "Command+Option+V",
            "Control+Shift+Space",
        ];
        for hotkey in &expected_hotkeys {
            let found = hotkey_items
                .iter()
                .filter_map(|item| item.as_menuitem())
                .any(|mi| mi.text() == *hotkey || mi.text() == format!("✓ {}", hotkey));
            assert!(found, "Hotkey {} not found in menu", hotkey);
        }
    }

    #[test]
    #[ignore = "requires main thread for Menu creation on macOS"]
    fn test_build_menu_model_selection() {
        let mut config = create_menu_test_config();
        config.model.name = "small".to_owned();
        let menu = TrayManager::build_menu(&config, Some(AppState::Idle)).unwrap();

        // Find Model submenu
        let menu_items = menu.items();
        let model_submenu = menu_items
            .iter()
            .find_map(|item| item.as_submenu().filter(|s| s.text() == "Model"))
            .expect("Model submenu not found");

        // Verify selected model has checkmark
        let model_items = model_submenu.items();
        let has_selected = model_items
            .iter()
            .filter_map(|item| item.as_menuitem())
            .any(|mi| mi.text() == "✓ small");
        assert!(has_selected, "Selected model should have checkmark");

        // Verify all 4 model options exist
        let expected_models = ["tiny", "base", "small", "medium"];
        for model in &expected_models {
            let found = model_items
                .iter()
                .filter_map(|item| item.as_menuitem())
                .any(|mi| mi.text() == *model || mi.text() == format!("✓ {}", model));
            assert!(found, "Model {} not found in menu", model);
        }
    }

    #[test]
    #[ignore = "requires main thread for Menu creation on macOS"]
    fn test_build_menu_threads_selection() {
        let mut config = create_menu_test_config();
        config.model.threads = 6;
        let menu = TrayManager::build_menu(&config, Some(AppState::Idle)).unwrap();

        // Find Optimization submenu
        let menu_items = menu.items();
        let opt_submenu = menu_items
            .iter()
            .find_map(|item| item.as_submenu().filter(|s| s.text() == "Optimization"))
            .expect("Optimization submenu not found");

        // Find Threads submenu inside Optimization
        let opt_items = opt_submenu.items();
        let threads_submenu = opt_items
            .iter()
            .find_map(|item| item.as_submenu().filter(|s| s.text() == "Threads"))
            .expect("Threads submenu not found");

        // Verify selected threads has checkmark
        let thread_items = threads_submenu.items();
        let has_selected = thread_items
            .iter()
            .filter_map(|item| item.as_menuitem())
            .any(|mi| mi.text() == "✓ 6 threads");
        assert!(has_selected, "Selected threads should have checkmark");

        // Verify all 4 thread options exist
        let expected_threads = [2, 4, 6, 8];
        for threads in &expected_threads {
            let found = thread_items
                .iter()
                .filter_map(|item| item.as_menuitem())
                .any(|mi| {
                    mi.text() == format!("{} threads", threads)
                        || mi.text() == format!("✓ {} threads", threads)
                });
            assert!(found, "Threads {} not found in menu", threads);
        }
    }

    #[test]
    #[ignore = "requires main thread for Menu creation on macOS"]
    fn test_build_menu_beam_size_selection() {
        let mut config = create_menu_test_config();
        config.model.beam_size = 10;
        let menu = TrayManager::build_menu(&config, Some(AppState::Idle)).unwrap();

        // Find Optimization submenu
        let menu_items = menu.items();
        let opt_submenu = menu_items
            .iter()
            .find_map(|item| item.as_submenu().filter(|s| s.text() == "Optimization"))
            .expect("Optimization submenu not found");

        // Find Beam Size submenu inside Optimization
        let opt_items = opt_submenu.items();
        let beam_submenu = opt_items
            .iter()
            .find_map(|item| item.as_submenu().filter(|s| s.text() == "Beam Size"))
            .expect("Beam Size submenu not found");

        // Verify selected beam size has checkmark
        let beam_items = beam_submenu.items();
        let has_selected = beam_items
            .iter()
            .filter_map(|item| item.as_menuitem())
            .any(|mi| mi.text() == "✓ Beam size 10");
        assert!(has_selected, "Selected beam size should have checkmark");

        // Verify all 5 beam size options exist
        let expected_beams = [1, 3, 5, 8, 10];
        for beam in &expected_beams {
            let found = beam_items
                .iter()
                .filter_map(|item| item.as_menuitem())
                .any(|mi| {
                    mi.text() == format!("Beam size {}", beam)
                        || mi.text() == format!("✓ Beam size {}", beam)
                });
            assert!(found, "Beam size {} not found in menu", beam);
        }
    }

    #[test]
    #[ignore = "requires main thread for Menu creation on macOS"]
    fn test_build_menu_language_selection() {
        // Test Auto-detect (None)
        let config_auto = create_menu_test_config();
        let menu_auto = TrayManager::build_menu(&config_auto, Some(AppState::Idle)).unwrap();
        let menu_auto_items = menu_auto.items();
        let lang_submenu_auto = menu_auto_items
            .iter()
            .find_map(|item| item.as_submenu().filter(|s| s.text() == "Language"))
            .expect("Language submenu not found");
        let lang_auto_items = lang_submenu_auto.items();
        let has_auto = lang_auto_items
            .iter()
            .filter_map(|item| item.as_menuitem())
            .any(|mi| mi.text() == "✓ Auto-detect");
        assert!(has_auto, "Auto-detect should be selected");

        // Test Polish (pl)
        let mut config_polish = create_menu_test_config();
        config_polish.model.language = Some("pl".to_owned());
        let menu_polish = TrayManager::build_menu(&config_polish, Some(AppState::Idle)).unwrap();
        let menu_polish_items = menu_polish.items();
        let lang_submenu_pl = menu_polish_items
            .iter()
            .find_map(|item| item.as_submenu().filter(|s| s.text() == "Language"))
            .expect("Language submenu not found");
        let lang_pl_items = lang_submenu_pl.items();
        let has_polish = lang_pl_items
            .iter()
            .filter_map(|item| item.as_menuitem())
            .any(|mi| mi.text() == "✓ Polish");
        assert!(has_polish, "Polish should be selected");

        // Verify all 6 language options exist
        let expected_langs = [
            "Auto-detect",
            "English",
            "Polish",
            "Spanish",
            "French",
            "German",
        ];
        for lang in &expected_langs {
            let found = lang_auto_items
                .iter()
                .filter_map(|item| item.as_menuitem())
                .any(|mi| mi.text() == *lang || mi.text() == format!("✓ {}", lang));
            assert!(found, "Language {} not found in menu", lang);
        }
    }

    #[test]
    #[ignore = "requires main thread for Menu creation on macOS"]
    fn test_build_menu_buffer_size_selection() {
        let mut config = create_menu_test_config();
        config.audio.buffer_size = 2048;
        let menu = TrayManager::build_menu(&config, Some(AppState::Idle)).unwrap();

        // Find Audio Buffer submenu
        let menu_items = menu.items();
        let buffer_submenu = menu_items
            .iter()
            .find_map(|item| item.as_submenu().filter(|s| s.text() == "Audio Buffer"))
            .expect("Audio Buffer submenu not found");

        // Verify selected buffer size has checkmark
        let buffer_items = buffer_submenu.items();
        let has_selected = buffer_items
            .iter()
            .filter_map(|item| item.as_menuitem())
            .any(|mi| mi.text() == "✓ 2048 samples");
        assert!(has_selected, "Selected buffer size should have checkmark");

        // Verify all 4 buffer size options exist
        let expected_sizes = [512, 1024, 2048, 4096];
        for size in &expected_sizes {
            let found = buffer_items
                .iter()
                .filter_map(|item| item.as_menuitem())
                .any(|mi| {
                    mi.text() == format!("{} samples", size)
                        || mi.text() == format!("✓ {} samples", size)
                });
            assert!(found, "Buffer size {} not found in menu", size);
        }
    }

    #[test]
    #[ignore = "requires main thread for Menu creation on macOS"]
    fn test_build_menu_preload_toggle_checked() {
        let mut config = create_menu_test_config();
        config.model.preload = true;
        let menu = TrayManager::build_menu(&config, Some(AppState::Idle)).unwrap();

        // Find Preload Model CheckMenuItem
        let menu_items = menu.items();
        let preload_item = menu_items
            .iter()
            .find_map(|item| {
                item.as_check_menuitem()
                    .filter(|check| check.text() == "Preload Model")
            })
            .expect("Preload Model CheckMenuItem not found");

        assert!(preload_item.is_checked(), "Preload should be checked");
    }

    #[test]
    #[ignore = "requires main thread for Menu creation on macOS"]
    fn test_build_menu_telemetry_toggle_unchecked() {
        let config = create_menu_test_config();
        let menu = TrayManager::build_menu(&config, Some(AppState::Idle)).unwrap();

        // Find Telemetry CheckMenuItem
        let menu_items = menu.items();
        let telemetry_item = menu_items
            .iter()
            .find_map(|item| {
                item.as_check_menuitem()
                    .filter(|check| check.text() == "Telemetry")
            })
            .expect("Telemetry CheckMenuItem not found");

        assert!(
            !telemetry_item.is_checked(),
            "Telemetry should be unchecked"
        );
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

    // Helper function tests
    #[test]
    fn test_format_label_with_checkmark() {
        assert_eq!(
            TrayManager::format_label_with_checkmark("test", true),
            "✓ test"
        );
        assert_eq!(
            TrayManager::format_label_with_checkmark("test", false),
            "test"
        );
    }

    #[test]
    fn test_format_hotkey_display() {
        let mods = vec!["Command".to_owned(), "Shift".to_owned()];
        assert_eq!(
            TrayManager::format_hotkey_display(&mods, "V"),
            "[\"Command\", \"Shift\"]+V"
        );
    }

    #[test]
    fn test_is_hotkey_selected() {
        let config = create_menu_test_config(); // Command+Shift+V
        assert!(TrayManager::is_hotkey_selected(
            &config,
            &["Command", "Shift"],
            "V"
        ));
        assert!(!TrayManager::is_hotkey_selected(
            &config,
            &["Control", "Option"],
            "Z"
        ));
    }

    // Comprehensive parse_menu_event tests
    #[test]
    fn test_parse_menu_event_all_hotkeys() {
        // Test all 4 hotkey options
        let cmd = TrayManager::parse_menu_event("Control+Option+Z");
        assert!(matches!(cmd, Some(TrayCommand::UpdateHotkey { .. })));
        if let Some(TrayCommand::UpdateHotkey { modifiers, key }) = cmd {
            assert_eq!(modifiers, vec!["Control", "Option"]);
            assert_eq!(key, "Z");
        }

        let cmd = TrayManager::parse_menu_event("Command+Shift+V");
        assert!(matches!(cmd, Some(TrayCommand::UpdateHotkey { .. })));
        if let Some(TrayCommand::UpdateHotkey { modifiers, key }) = cmd {
            assert_eq!(modifiers, vec!["Command", "Shift"]);
            assert_eq!(key, "V");
        }

        let cmd = TrayManager::parse_menu_event("Command+Option+V");
        assert!(matches!(cmd, Some(TrayCommand::UpdateHotkey { .. })));
        if let Some(TrayCommand::UpdateHotkey { modifiers, key }) = cmd {
            assert_eq!(modifiers, vec!["Command", "Option"]);
            assert_eq!(key, "V");
        }

        let cmd = TrayManager::parse_menu_event("Control+Shift+Space");
        assert!(matches!(cmd, Some(TrayCommand::UpdateHotkey { .. })));
        if let Some(TrayCommand::UpdateHotkey { modifiers, key }) = cmd {
            assert_eq!(modifiers, vec!["Control", "Shift"]);
            assert_eq!(key, "Space");
        }
    }

    #[test]
    fn test_parse_menu_event_all_models() {
        // Test all 4 model options
        for model in &["tiny", "base", "small", "medium"] {
            let cmd = TrayManager::parse_menu_event(model);
            assert!(matches!(cmd, Some(TrayCommand::UpdateModel { .. })));
            if let Some(TrayCommand::UpdateModel { name }) = cmd {
                assert_eq!(name, *model);
            }
        }
    }

    #[test]
    fn test_parse_menu_event_all_threads() {
        // Test all 4 thread options
        for threads in &[2, 4, 6, 8] {
            let input = format!("{} threads", threads);
            let cmd = TrayManager::parse_menu_event(&input);
            assert_eq!(cmd, Some(TrayCommand::UpdateThreads(*threads)));
        }
    }

    #[test]
    fn test_parse_menu_event_all_beam_sizes() {
        // Test all 5 beam size options
        for beam in &[1, 3, 5, 8, 10] {
            let input = format!("Beam size {}", beam);
            let cmd = TrayManager::parse_menu_event(&input);
            assert_eq!(cmd, Some(TrayCommand::UpdateBeamSize(*beam)));
        }
    }

    #[test]
    fn test_parse_menu_event_all_languages() {
        // Test Auto-detect
        let cmd = TrayManager::parse_menu_event("Auto-detect");
        assert_eq!(cmd, Some(TrayCommand::UpdateLanguage(None)));

        // Test all 5 language options
        let languages = [
            ("English", "en"),
            ("Polish", "pl"),
            ("Spanish", "es"),
            ("French", "fr"),
            ("German", "de"),
        ];
        for (display, code) in &languages {
            let cmd = TrayManager::parse_menu_event(display);
            assert!(matches!(cmd, Some(TrayCommand::UpdateLanguage(Some(_)))));
            if let Some(TrayCommand::UpdateLanguage(Some(lang))) = cmd {
                assert_eq!(lang, *code);
            }
        }
    }

    #[test]
    fn test_parse_menu_event_all_buffer_sizes() {
        // Test all 4 buffer size options
        for size in &[512, 1024, 2048, 4096] {
            let input = format!("{} samples", size);
            let cmd = TrayManager::parse_menu_event(&input);
            assert_eq!(cmd, Some(TrayCommand::UpdateBufferSize(*size)));
        }
    }

    #[test]
    fn test_parse_menu_event_toggles_and_actions() {
        assert_eq!(
            TrayManager::parse_menu_event("Preload Model"),
            Some(TrayCommand::TogglePreload)
        );
        assert_eq!(
            TrayManager::parse_menu_event("Telemetry"),
            Some(TrayCommand::ToggleTelemetry)
        );
        assert_eq!(
            TrayManager::parse_menu_event("Open Config File"),
            Some(TrayCommand::OpenConfigFile)
        );
    }

    #[test]
    fn test_parse_menu_event_unknown_input() {
        assert_eq!(TrayManager::parse_menu_event("Unknown Option"), None);
        assert_eq!(TrayManager::parse_menu_event(""), None);
        assert_eq!(TrayManager::parse_menu_event("Quit"), None);
    }

    #[test]
    fn test_parse_menu_event_checkmark_stripping() {
        // Verify checkmark is stripped for all command types
        assert!(matches!(
            TrayManager::parse_menu_event("✓ Control+Option+Z"),
            Some(TrayCommand::UpdateHotkey { .. })
        ));
        assert!(matches!(
            TrayManager::parse_menu_event("✓ base"),
            Some(TrayCommand::UpdateModel { .. })
        ));
        assert_eq!(
            TrayManager::parse_menu_event("✓ 8 threads"),
            Some(TrayCommand::UpdateThreads(8))
        );
        assert_eq!(
            TrayManager::parse_menu_event("✓ Beam size 10"),
            Some(TrayCommand::UpdateBeamSize(10))
        );
        assert_eq!(
            TrayManager::parse_menu_event("✓ Auto-detect"),
            Some(TrayCommand::UpdateLanguage(None))
        );
        assert_eq!(
            TrayManager::parse_menu_event("✓ 4096 samples"),
            Some(TrayCommand::UpdateBufferSize(4096))
        );
    }

    // Phase 4: Menu configuration tests (pure logic, fully testable)
    #[test]
    fn test_build_menu_config_hotkeys() {
        let config = create_menu_test_config(); // Command+Shift+V
        let menu_config = TrayManager::build_menu_config(&config, Some(AppState::Idle));

        assert_eq!(menu_config.hotkeys.len(), 4);

        // Check selected hotkey
        let selected = menu_config.hotkeys.iter().find(|h| h.selected).unwrap();
        assert_eq!(selected.modifiers, vec!["Command", "Shift"]);
        assert_eq!(selected.key, "V");

        // Check all hotkeys present
        assert!(menu_config
            .hotkeys
            .iter()
            .any(|h| h.label == "Control+Option+Z"));
        assert!(menu_config
            .hotkeys
            .iter()
            .any(|h| h.label == "Control+Shift+Space"));
    }

    #[test]
    fn test_build_menu_config_models() {
        let mut config = create_menu_test_config();
        config.model.name = "tiny".to_owned();
        let menu_config = TrayManager::build_menu_config(&config, None);

        assert_eq!(menu_config.models.len(), 4);

        let selected = menu_config.models.iter().find(|m| m.selected).unwrap();
        assert_eq!(selected.name, "tiny");

        // Check all models present
        let model_names: Vec<_> = menu_config.models.iter().map(|m| m.name.as_str()).collect();
        assert!(model_names.contains(&"tiny"));
        assert!(model_names.contains(&"base"));
        assert!(model_names.contains(&"small"));
        assert!(model_names.contains(&"medium"));
    }

    #[test]
    fn test_build_menu_config_threads() {
        let mut config = create_menu_test_config();
        config.model.threads = 8;
        let menu_config = TrayManager::build_menu_config(&config, None);

        assert_eq!(menu_config.threads.len(), 4);

        let selected = menu_config.threads.iter().find(|t| t.selected).unwrap();
        assert_eq!(selected.count, 8);
        assert_eq!(selected.label, "8 threads");

        // Check all thread counts present
        let counts: Vec<_> = menu_config.threads.iter().map(|t| t.count).collect();
        assert_eq!(counts, vec![2, 4, 6, 8]);
    }

    #[test]
    fn test_build_menu_config_beam_sizes() {
        let mut config = create_menu_test_config();
        config.model.beam_size = 10;
        let menu_config = TrayManager::build_menu_config(&config, None);

        assert_eq!(menu_config.beam_sizes.len(), 5);

        let selected = menu_config.beam_sizes.iter().find(|b| b.selected).unwrap();
        assert_eq!(selected.size, 10);
        assert_eq!(selected.label, "Beam size 10");

        // Check all beam sizes present
        let sizes: Vec<_> = menu_config.beam_sizes.iter().map(|b| b.size).collect();
        assert_eq!(sizes, vec![1, 3, 5, 8, 10]);
    }

    #[test]
    fn test_build_menu_config_languages() {
        let mut config = create_menu_test_config();
        config.model.language = Some("pl".to_owned());
        let menu_config = TrayManager::build_menu_config(&config, None);

        assert_eq!(menu_config.languages.len(), 6);

        let selected = menu_config.languages.iter().find(|l| l.selected).unwrap();
        assert_eq!(selected.display_name, "Polish");
        assert_eq!(selected.code, Some("pl".to_owned()));

        // Check Auto-detect option
        let auto = menu_config
            .languages
            .iter()
            .find(|l| l.code.is_none())
            .unwrap();
        assert_eq!(auto.display_name, "Auto-detect");
        assert!(!auto.selected);
    }

    #[test]
    fn test_build_menu_config_language_auto_detect() {
        let config = create_menu_test_config(); // language = None
        let menu_config = TrayManager::build_menu_config(&config, None);

        let selected = menu_config.languages.iter().find(|l| l.selected).unwrap();
        assert_eq!(selected.display_name, "Auto-detect");
        assert_eq!(selected.code, None);
    }

    #[test]
    fn test_build_menu_config_buffer_sizes() {
        let mut config = create_menu_test_config();
        config.audio.buffer_size = 2048;
        let menu_config = TrayManager::build_menu_config(&config, None);

        assert_eq!(menu_config.buffer_sizes.len(), 4);

        let selected = menu_config
            .buffer_sizes
            .iter()
            .find(|b| b.selected)
            .unwrap();
        assert_eq!(selected.size, 2048);
        assert_eq!(selected.label, "2048 samples");

        // Check all sizes present
        let sizes: Vec<_> = menu_config.buffer_sizes.iter().map(|b| b.size).collect();
        assert_eq!(sizes, vec![512, 1024, 2048, 4096]);
    }

    #[test]
    fn test_build_menu_config_toggles() {
        let mut config = create_menu_test_config();
        config.model.preload = false;
        config.telemetry.enabled = true;
        let menu_config = TrayManager::build_menu_config(&config, None);

        assert!(!menu_config.preload_enabled);
        assert!(menu_config.telemetry_enabled);
    }

    #[test]
    fn test_build_menu_config_status_text() {
        let config = create_menu_test_config();

        let idle = TrayManager::build_menu_config(&config, Some(AppState::Idle));
        assert_eq!(idle.status_text, "Whisper Hotkey - Ready");

        let recording = TrayManager::build_menu_config(&config, Some(AppState::Recording));
        assert_eq!(recording.status_text, "🎤 Recording...");

        let processing = TrayManager::build_menu_config(&config, Some(AppState::Processing));
        assert_eq!(processing.status_text, "⏳ Transcribing...");

        let none = TrayManager::build_menu_config(&config, None);
        assert_eq!(none.status_text, "Whisper Hotkey");
    }

    #[test]
    fn test_menu_config_selection_logic() {
        // Test that only one item is selected per category
        let config = create_menu_test_config();
        let menu_config = TrayManager::build_menu_config(&config, None);

        // Hotkeys: exactly one selected
        assert_eq!(menu_config.hotkeys.iter().filter(|h| h.selected).count(), 1);

        // Models: exactly one selected
        assert_eq!(menu_config.models.iter().filter(|m| m.selected).count(), 1);

        // Threads: exactly one selected
        assert_eq!(menu_config.threads.iter().filter(|t| t.selected).count(), 1);

        // Beam sizes: exactly one selected
        assert_eq!(
            menu_config.beam_sizes.iter().filter(|b| b.selected).count(),
            1
        );

        // Languages: exactly one selected
        assert_eq!(
            menu_config.languages.iter().filter(|l| l.selected).count(),
            1
        );

        // Buffer sizes: exactly one selected
        assert_eq!(
            menu_config
                .buffer_sizes
                .iter()
                .filter(|b| b.selected)
                .count(),
            1
        );
    }

    #[test]
    fn test_menu_config_with_different_configs() {
        // Test with minimal config
        let mut config = create_menu_test_config();
        config.model.threads = 2;
        config.model.beam_size = 1;
        config.audio.buffer_size = 512;

        let menu_config = TrayManager::build_menu_config(&config, None);

        assert_eq!(
            menu_config
                .threads
                .iter()
                .find(|t| t.selected)
                .unwrap()
                .count,
            2
        );
        assert_eq!(
            menu_config
                .beam_sizes
                .iter()
                .find(|b| b.selected)
                .unwrap()
                .size,
            1
        );
        assert_eq!(
            menu_config
                .buffer_sizes
                .iter()
                .find(|b| b.selected)
                .unwrap()
                .size,
            512
        );
    }
}
