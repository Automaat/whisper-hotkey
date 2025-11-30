use anyhow::{Context, Result};
use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIconBuilder};

use crate::config::Config;

#[derive(Debug, Clone)]
pub enum TrayCommand {
    UpdateHotkey { modifiers: Vec<String>, key: String },
    UpdateModel { name: String },
    UpdateThreads(usize),
    UpdateBeamSize(usize),
    UpdateLanguage(Option<String>),
    UpdateBufferSize(usize),
    TogglePreload(bool),
    ToggleTelemetry(bool),
    OpenConfigFile,
    Quit,
}

pub struct TrayManager {
    tray: tray_icon::TrayIcon,
}

impl TrayManager {
    pub fn new(config: &Config) -> Result<Self> {
        let icon = Self::load_icon()?;
        let menu = Self::build_menu(config)?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Whisper Hotkey")
            .with_icon(icon)
            .build()
            .context("failed to build tray icon")?;

        Ok(Self { tray })
    }

    fn load_icon() -> Result<Icon> {
        // Load the 32x32 icon for Retina displays (will scale to 16x16 automatically)
        let icon_path = concat!(env!("CARGO_MANIFEST_DIR"), "/icon-32.png");
        let image = image::open(icon_path)
            .context("failed to load icon-32.png")?
            .into_rgba8();

        let (width, height) = image.dimensions();
        let rgba = image.into_raw();

        Icon::from_rgba(rgba, width, height).context("failed to create icon from RGBA data")
    }

    fn build_menu(config: &Config) -> Result<Menu> {
        let menu = Menu::new();

        // Status item (non-clickable)
        let status = MenuItem::new("Whisper Hotkey", false, None);
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
            let item = CheckMenuItem::new(label, is_selected, true, None);
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
            let item = CheckMenuItem::new(model_name, is_selected, true, None);
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
            let label = format!("{} threads", threads);
            let is_selected = config.model.threads == threads;
            let item = CheckMenuItem::new(&label, is_selected, true, None);
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
            let label = format!("Beam size {}", beam);
            let is_selected = config.model.beam_size == beam;
            let item = CheckMenuItem::new(&label, is_selected, true, None);
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
            let item = CheckMenuItem::new(label, is_selected, true, None);
            lang_submenu
                .append(&item)
                .context("failed to append language item")?;
        }

        menu.append(&lang_submenu)
            .context("failed to append language submenu")?;

        // Audio buffer submenu
        let buffer_submenu = Submenu::new("Audio Buffer", true);
        for size in [512, 1024, 2048, 4096] {
            let label = format!("{} samples", size);
            let is_selected = config.audio.buffer_size == size;
            let item = CheckMenuItem::new(&label, is_selected, true, None);
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
        menu.append(&quit)
            .context("failed to append quit item")?;

        Ok(menu)
    }

    pub fn update_menu(&mut self, config: &Config) -> Result<()> {
        let new_menu = Self::build_menu(config)?;
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
        // Hotkeys
        if id == "Control+Option+Z" {
            return Some(TrayCommand::UpdateHotkey {
                modifiers: vec!["Control".to_string(), "Option".to_string()],
                key: "Z".to_string(),
            });
        }
        if id == "Command+Shift+V" {
            return Some(TrayCommand::UpdateHotkey {
                modifiers: vec!["Command".to_string(), "Shift".to_string()],
                key: "V".to_string(),
            });
        }
        if id == "Command+Option+V" {
            return Some(TrayCommand::UpdateHotkey {
                modifiers: vec!["Command".to_string(), "Option".to_string()],
                key: "V".to_string(),
            });
        }
        if id == "Control+Shift+Space" {
            return Some(TrayCommand::UpdateHotkey {
                modifiers: vec!["Control".to_string(), "Shift".to_string()],
                key: "Space".to_string(),
            });
        }

        // Models
        if id == "tiny" {
            return Some(TrayCommand::UpdateModel {
                name: "tiny".to_string(),
            });
        }
        if id == "base" {
            return Some(TrayCommand::UpdateModel {
                name: "base".to_string(),
            });
        }
        if id == "small" {
            return Some(TrayCommand::UpdateModel {
                name: "small".to_string(),
            });
        }
        if id == "medium" {
            return Some(TrayCommand::UpdateModel {
                name: "medium".to_string(),
            });
        }

        // Threads
        if id == "2 threads" {
            return Some(TrayCommand::UpdateThreads(2));
        }
        if id == "4 threads" {
            return Some(TrayCommand::UpdateThreads(4));
        }
        if id == "6 threads" {
            return Some(TrayCommand::UpdateThreads(6));
        }
        if id == "8 threads" {
            return Some(TrayCommand::UpdateThreads(8));
        }

        // Beam sizes
        if id == "Beam size 1" {
            return Some(TrayCommand::UpdateBeamSize(1));
        }
        if id == "Beam size 3" {
            return Some(TrayCommand::UpdateBeamSize(3));
        }
        if id == "Beam size 5" {
            return Some(TrayCommand::UpdateBeamSize(5));
        }
        if id == "Beam size 8" {
            return Some(TrayCommand::UpdateBeamSize(8));
        }
        if id == "Beam size 10" {
            return Some(TrayCommand::UpdateBeamSize(10));
        }

        // Languages
        if id == "Auto-detect" {
            return Some(TrayCommand::UpdateLanguage(None));
        }
        if id == "English" {
            return Some(TrayCommand::UpdateLanguage(Some("en".to_string())));
        }
        if id == "Polish" {
            return Some(TrayCommand::UpdateLanguage(Some("pl".to_string())));
        }
        if id == "Spanish" {
            return Some(TrayCommand::UpdateLanguage(Some("es".to_string())));
        }
        if id == "French" {
            return Some(TrayCommand::UpdateLanguage(Some("fr".to_string())));
        }
        if id == "German" {
            return Some(TrayCommand::UpdateLanguage(Some("de".to_string())));
        }

        // Audio buffer
        if id == "512 samples" {
            return Some(TrayCommand::UpdateBufferSize(512));
        }
        if id == "1024 samples" {
            return Some(TrayCommand::UpdateBufferSize(1024));
        }
        if id == "2048 samples" {
            return Some(TrayCommand::UpdateBufferSize(2048));
        }
        if id == "4096 samples" {
            return Some(TrayCommand::UpdateBufferSize(4096));
        }

        // Toggles
        if id == "Preload Model" {
            return Some(TrayCommand::TogglePreload(true));
        }
        if id == "Telemetry" {
            return Some(TrayCommand::ToggleTelemetry(true));
        }

        // Actions
        if id == "Open Config File" {
            return Some(TrayCommand::OpenConfigFile);
        }
        if id == "Quit" {
            return Some(TrayCommand::Quit);
        }

        None
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
}

