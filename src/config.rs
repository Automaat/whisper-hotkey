use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[allow(dead_code)] // Used in Phase 2+
    pub hotkey: HotkeyConfig,
    #[allow(dead_code)] // Used in Phase 3+
    pub audio: AudioConfig,
    #[allow(dead_code)] // Used in Phase 4+
    pub model: ModelConfig,
    pub telemetry: TelemetryConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HotkeyConfig {
    #[allow(dead_code)] // Used in Phase 2
    pub modifiers: Vec<String>,
    #[allow(dead_code)] // Used in Phase 2
    pub key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AudioConfig {
    #[allow(dead_code)] // Used in Phase 3
    pub buffer_size: usize,
    #[allow(dead_code)] // Used in Phase 3
    pub sample_rate: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModelConfig {
    #[allow(dead_code)] // Used in Phase 4
    pub name: String,
    #[allow(dead_code)] // Used in Phase 4
    pub path: String,
    #[allow(dead_code)] // Used in Phase 4
    pub preload: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub log_path: String,
}

impl Config {
    /// Load config from ~/.whisper-hotkey.toml
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            Self::create_default(&config_path).context("failed to create default config")?;
        }

        let contents = fs::read_to_string(&config_path).context("failed to read config file")?;

        let config: Config = toml::from_str(&contents).context("failed to parse config TOML")?;

        Ok(config)
    }

    fn config_path() -> Result<PathBuf> {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(".whisper-hotkey.toml"))
    }

    fn create_default(path: &PathBuf) -> Result<()> {
        let default_config = r#"[hotkey]
modifiers = ["Control", "Option"]
key = "Z"

[audio]
buffer_size = 1024
sample_rate = 16000

[model]
name = "small"
path = "~/.whisper-hotkey/models/ggml-small.bin"
preload = true

[telemetry]
enabled = true
log_path = "~/.whisper-hotkey/crash.log"
"#;
        fs::write(path, default_config).context("failed to write default config")?;
        Ok(())
    }

    /// Expand ~ in paths to home directory
    #[allow(dead_code)] // Used in Phase 3+
    pub fn expand_path(path: &str) -> Result<PathBuf> {
        if let Some(stripped) = path.strip_prefix("~/") {
            let home = std::env::var("HOME").context("HOME environment variable not set")?;
            Ok(PathBuf::from(home).join(stripped))
        } else {
            Ok(PathBuf::from(path))
        }
    }
}
