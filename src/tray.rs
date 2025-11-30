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
    Quit,
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

        let icon = cached_icons
            .get(&AppState::Idle)
            .context("idle icon not in cache")?
            .clone();
        let menu = Self::build_menu(config, Some(AppState::Idle))?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Whisper Hotkey")
            .with_icon(icon)
            .build()
            .context("failed to build tray icon")?;

        Ok(Self {
            tray,
            state,
            current_icon_state: AppState::Idle,
            cached_icons,
        })
    }

    fn load_icon(state: AppState) -> Result<Icon> {
        // Load appropriate icon based on state
        let icon_filename = match state {
            AppState::Idle => "icon-32.png",
            AppState::Recording => "icon-recording-32.png",
            AppState::Processing => "icon-processing-32.png",
        };

        let icon_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), icon_filename);
        tracing::debug!("loading icon for state {:?}: {}", state, icon_path);

        let image = image::open(&icon_path)
            .with_context(|| format!("failed to load {}", icon_filename))?
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

            // Rebuild menu with new state (reliable feedback)
            let new_menu = Self::build_menu(config, Some(new_state))?;
            self.tray.set_menu(Some(Box::new(new_menu)));

            // Try icon update (has known macOS bug but keep trying) - use cached icon
            let icon = self
                .cached_icons
                .get(&new_state)
                .context("icon not in cache")?
                .clone();
            self.tray.set_icon(Some(icon)).ok();

            self.current_icon_state = new_state;
            tracing::info!("✓ tray menu updated with state: {:?}", new_state);
        }
        Ok(())
    }

    fn build_menu(config: &Config, app_state: Option<AppState>) -> Result<Menu> {
        let menu = Menu::new();

        // Status item showing current state (non-clickable)
        let status_text = if let Some(state) = app_state {
            match state {
                AppState::Idle => "Whisper Hotkey - Ready",
                AppState::Recording => "🎤 Recording...",
                AppState::Processing => "⏳ Transcribing...",
            }
        } else {
            "Whisper Hotkey"
        };
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
                label.to_string()
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
                model_name.to_string()
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
                label.to_string()
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

        let quit = MenuItem::new("Quit", true, None);
        menu.append(&quit).context("failed to append quit item")?;

        Ok(menu)
    }

    pub fn update_menu(&mut self, config: &Config) -> Result<()> {
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
                modifiers: vec!["Control".to_string(), "Option".to_string()],
                key: "Z".to_string(),
            }),
            "Command+Shift+V" => Some(TrayCommand::UpdateHotkey {
                modifiers: vec!["Command".to_string(), "Shift".to_string()],
                key: "V".to_string(),
            }),
            "Command+Option+V" => Some(TrayCommand::UpdateHotkey {
                modifiers: vec!["Command".to_string(), "Option".to_string()],
                key: "V".to_string(),
            }),
            "Control+Shift+Space" => Some(TrayCommand::UpdateHotkey {
                modifiers: vec!["Control".to_string(), "Shift".to_string()],
                key: "Space".to_string(),
            }),

            // Models
            "tiny" | "base" | "small" | "medium" => Some(TrayCommand::UpdateModel {
                name: id.to_string(),
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
            "English" => Some(TrayCommand::UpdateLanguage(Some("en".to_string()))),
            "Polish" => Some(TrayCommand::UpdateLanguage(Some("pl".to_string()))),
            "Spanish" => Some(TrayCommand::UpdateLanguage(Some("es".to_string()))),
            "French" => Some(TrayCommand::UpdateLanguage(Some("fr".to_string()))),
            "German" => Some(TrayCommand::UpdateLanguage(Some("de".to_string()))),

            // Audio buffer
            "512 samples" => Some(TrayCommand::UpdateBufferSize(512)),
            "1024 samples" => Some(TrayCommand::UpdateBufferSize(1024)),
            "2048 samples" => Some(TrayCommand::UpdateBufferSize(2048)),
            "4096 samples" => Some(TrayCommand::UpdateBufferSize(4096)),

            // Toggles and Actions
            "Preload Model" => Some(TrayCommand::TogglePreload),
            "Telemetry" => Some(TrayCommand::ToggleTelemetry),
            "Open Config File" => Some(TrayCommand::OpenConfigFile),
            "Quit" => Some(TrayCommand::Quit),

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
    }

    #[test]
    fn test_parse_menu_event_actions() {
        let cmd = TrayManager::parse_menu_event("Open Config File");
        assert!(matches!(cmd, Some(TrayCommand::OpenConfigFile)));

        let cmd = TrayManager::parse_menu_event("Quit");
        assert!(matches!(cmd, Some(TrayCommand::Quit)));
    }

    #[test]
    fn test_parse_menu_event_unknown() {
        let cmd = TrayManager::parse_menu_event("Unknown Item");
        assert!(cmd.is_none());

        let cmd = TrayManager::parse_menu_event("");
        assert!(cmd.is_none());
    }

    #[test]
    fn test_tray_command_clone() {
        let cmd1 = TrayCommand::UpdateThreads(4);
        let cmd2 = cmd1.clone();
        assert!(matches!(cmd2, TrayCommand::UpdateThreads(4)));

        let cmd3 = TrayCommand::UpdateLanguage(Some("en".to_string()));
        let cmd4 = cmd3.clone();
        if let TrayCommand::UpdateLanguage(Some(lang)) = cmd4 {
            assert_eq!(lang, "en");
        }
    }

    #[test]
    fn test_tray_command_debug() {
        let cmd = TrayCommand::Quit;
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Quit"));
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
    #[ignore] // Requires full config and tray icon initialization
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
}
