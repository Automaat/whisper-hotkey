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
    #[serde(default = "default_threads")]
    pub threads: usize,
    #[serde(default = "default_beam_size")]
    pub beam_size: usize,
    #[serde(default = "default_language")]
    pub language: Option<String>,
}

fn default_threads() -> usize {
    4 // Optimal for M1/M2 chips
}

fn default_beam_size() -> usize {
    5 // Balance speed/accuracy
}

fn default_language() -> Option<String> {
    None // Auto-detect by default
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
threads = 4        # CPU threads for inference (4 optimal for M1/M2)
beam_size = 5      # Beam search size (higher = more accurate but slower)
# language = "pl"  # Language hint: "en", "pl", "es", etc. Omit for auto-detect

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_expand_path_with_tilde() {
        let home = env::var("HOME").expect("HOME not set");
        let result = Config::expand_path("~/test/path").unwrap();
        assert_eq!(result, PathBuf::from(home).join("test/path"));
    }

    #[test]
    fn test_expand_path_without_tilde() {
        let result = Config::expand_path("/absolute/path").unwrap();
        assert_eq!(result, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_expand_path_relative() {
        let result = Config::expand_path("relative/path").unwrap();
        assert_eq!(result, PathBuf::from("relative/path"));
    }

    #[test]
    fn test_parse_valid_config() {
        let toml = r#"
[hotkey]
modifiers = ["Control", "Option"]
key = "Z"

[audio]
buffer_size = 1024
sample_rate = 16000

[model]
name = "small"
path = "~/.whisper-hotkey/models/ggml-small.bin"
preload = true
threads = 4
beam_size = 5

[telemetry]
enabled = true
log_path = "~/.whisper-hotkey/crash.log"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.hotkey.modifiers, vec!["Control", "Option"]);
        assert_eq!(config.hotkey.key, "Z");
        assert_eq!(config.audio.buffer_size, 1024);
        assert_eq!(config.audio.sample_rate, 16000);
        assert_eq!(config.model.name, "small");
        assert_eq!(config.model.threads, 4);
        assert_eq!(config.model.beam_size, 5);
        assert!(config.telemetry.enabled);
    }

    #[test]
    fn test_parse_invalid_toml() {
        let invalid_toml = r#"
[hotkey
modifiers = ["Control"
"#;
        let result: Result<Config, _> = toml::from_str(invalid_toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_required_field() {
        let toml = r#"
[hotkey]
modifiers = ["Control"]

[audio]
buffer_size = 1024
sample_rate = 16000

[model]
name = "small"
path = "~/models/ggml-small.bin"
preload = true
"#;
        let result: Result<Config, _> = toml::from_str(toml);
        assert!(result.is_err()); // Missing telemetry section
    }

    #[test]
    fn test_parse_different_modifiers() {
        let toml = r#"
[hotkey]
modifiers = ["Command", "Shift"]
key = "V"

[audio]
buffer_size = 2048
sample_rate = 16000

[model]
name = "base"
path = "/usr/local/share/whisper/base.bin"
preload = false
threads = 8
beam_size = 10

[telemetry]
enabled = false
log_path = "/tmp/test.log"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.hotkey.modifiers, vec!["Command", "Shift"]);
        assert_eq!(config.hotkey.key, "V");
        assert_eq!(config.audio.buffer_size, 2048);
        assert!(!config.model.preload);
        assert_eq!(config.model.threads, 8);
        assert_eq!(config.model.beam_size, 10);
        assert!(!config.telemetry.enabled);
    }

    #[test]
    fn test_parse_config_with_default_optimization() {
        let toml = r#"
[hotkey]
modifiers = ["Control"]
key = "M"

[audio]
buffer_size = 512
sample_rate = 16000

[model]
name = "tiny"
path = "/tmp/tiny.bin"
preload = true

[telemetry]
enabled = true
log_path = "/tmp/crash.log"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        // When not specified, should use defaults
        assert_eq!(config.model.threads, 4);
        assert_eq!(config.model.beam_size, 5);
    }

    #[test]
    #[ignore] // Requires filesystem access
    fn test_load_creates_default_if_missing() {
        // This test would require setting up a temp directory and manipulating HOME
        // Skip for now as it's integration-level testing
    }

    #[test]
    #[ignore] // Requires filesystem access
    fn test_load_reads_existing_config() {
        // This test would require creating a temp config file
        // Skip for now as it's integration-level testing
    }
}
