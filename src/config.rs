use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Whisper model type variants
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    /// Tiny model (multilingual)
    Tiny,
    /// Tiny model (English-only)
    TinyEn,
    /// Base model (multilingual)
    Base,
    /// Base model (English-only)
    BaseEn,
    /// Small model (multilingual)
    Small,
    /// Small model (English-only)
    SmallEn,
    /// Medium model (multilingual)
    Medium,
    /// Medium model (English-only)
    MediumEn,
    /// Large model (multilingual)
    Large,
    /// Large model v1 (multilingual)
    LargeV1,
    /// Large model v2 (multilingual)
    LargeV2,
    /// Large model v3 (multilingual)
    LargeV3,
}

impl ModelType {
    /// Get model name as string (e.g., "base.en")
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tiny => "tiny",
            Self::TinyEn => "tiny.en",
            Self::Base => "base",
            Self::BaseEn => "base.en",
            Self::Small => "small",
            Self::SmallEn => "small.en",
            Self::Medium => "medium",
            Self::MediumEn => "medium.en",
            Self::Large => "large",
            Self::LargeV1 => "large-v1",
            Self::LargeV2 => "large-v2",
            Self::LargeV3 => "large-v3",
        }
    }

    /// Get default path for model
    #[must_use]
    pub fn default_path(self) -> String {
        format!("~/.whisper-hotkey/models/ggml-{}.bin", self.as_str())
    }

    /// Get all available model type variants
    #[must_use]
    pub fn variants() -> Vec<Self> {
        vec![
            Self::Tiny,
            Self::TinyEn,
            Self::Base,
            Self::BaseEn,
            Self::Small,
            Self::SmallEn,
            Self::Medium,
            Self::MediumEn,
            Self::Large,
            Self::LargeV1,
            Self::LargeV2,
            Self::LargeV3,
        ]
    }
}

/// Application configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// Hotkey configuration
    #[allow(dead_code)] // Used in Phase 2+
    pub hotkey: HotkeyConfig,
    /// Audio capture configuration
    #[allow(dead_code)] // Used in Phase 3+
    pub audio: AudioConfig,
    /// Whisper model configuration
    #[allow(dead_code)] // Used in Phase 4+
    pub model: ModelConfig,
    /// Telemetry configuration
    pub telemetry: TelemetryConfig,
    /// Recording configuration
    #[serde(default)]
    pub recording: RecordingConfig,
}

/// Hotkey configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HotkeyConfig {
    /// Modifier keys (e.g., `["Command", "Shift"]`)
    #[allow(dead_code)] // Used in Phase 2
    pub modifiers: Vec<String>,
    /// Main key (e.g., "V")
    #[allow(dead_code)] // Used in Phase 2
    pub key: String,
}

/// Audio capture configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AudioConfig {
    /// Ring buffer size in samples
    #[allow(dead_code)] // Used in Phase 3
    pub buffer_size: usize,
    /// Sample rate in Hz
    #[allow(dead_code)] // Used in Phase 3
    pub sample_rate: u32,
}

/// Whisper model configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModelConfig {
    /// Model type (if set, auto-constructs name and path)
    #[serde(default)]
    pub model_type: Option<ModelType>,
    /// Model name (e.g., "base.en") - deprecated, use `model_type` instead
    #[serde(default)]
    #[allow(dead_code)] // Used in Phase 4
    pub name: Option<String>,
    /// Path to model file - deprecated, use `model_type` instead
    #[serde(default)]
    #[allow(dead_code)] // Used in Phase 4
    pub path: Option<String>,
    /// Preload model at startup
    #[allow(dead_code)] // Used in Phase 4
    pub preload: bool,
    /// Number of CPU threads for inference
    #[serde(default = "default_threads")]
    pub threads: usize,
    /// Beam search width (higher = slower but more accurate)
    #[serde(default = "default_beam_size")]
    pub beam_size: usize,
    /// Language code (None = auto-detect)
    #[serde(default = "default_language")]
    pub language: Option<String>,
}

const fn default_threads() -> usize {
    4 // Optimal for M1/M2 chips
}

const fn default_beam_size() -> usize {
    5 // Balance speed/accuracy
}

const fn default_language() -> Option<String> {
    None // Auto-detect by default
}

impl ModelConfig {
    /// Get effective model name (from `model_type` if set, else from name field)
    #[must_use]
    pub fn effective_name(&self) -> String {
        self.model_type
            .map(|t| t.as_str().to_owned())
            .or_else(|| self.name.clone())
            .unwrap_or_default()
    }

    /// Get effective model path (from `model_type` if set, else from path field)
    #[must_use]
    pub fn effective_path(&self) -> String {
        self.model_type
            .map(ModelType::default_path)
            .or_else(|| self.path.clone())
            .unwrap_or_default()
    }
}

/// Telemetry and crash logging configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TelemetryConfig {
    /// Enable crash logging
    pub enabled: bool,
    /// Path to log file
    pub log_path: String,
}

/// Recording configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RecordingConfig {
    /// Enable debug recordings
    #[serde(default = "default_recording_enabled")]
    pub enabled: bool,
    /// Delete recordings older than N days (0 = keep all)
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
    /// Keep only N most recent recordings (0 = unlimited)
    #[serde(default = "default_max_count")]
    pub max_count: usize,
    /// Hours between cleanup runs (0 = startup only)
    #[serde(default = "default_cleanup_interval_hours")]
    pub cleanup_interval_hours: u32,
}

const fn default_recording_enabled() -> bool {
    true
}

const fn default_retention_days() -> u32 {
    7
}

const fn default_max_count() -> usize {
    100
}

const fn default_cleanup_interval_hours() -> u32 {
    1
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            enabled: default_recording_enabled(),
            retention_days: default_retention_days(),
            max_count: default_max_count(),
            cleanup_interval_hours: default_cleanup_interval_hours(),
        }
    }
}

impl Config {
    /// Load config from ~/.whisper-hotkey/config.toml
    ///
    /// Automatically migrates from old path (~/.whisper-hotkey.toml) if found.
    /// Creates default config if none exists.
    ///
    /// # Errors
    /// Returns error if config is invalid TOML or path expansion fails
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        // Migrate from old path if needed
        if !config_path.exists() {
            let old_path = Self::old_config_path()?;
            if old_path.exists() {
                // Ensure new directory exists before migration
                if let Some(parent) = config_path.parent() {
                    fs::create_dir_all(parent).context("failed to create config directory")?;
                }
                fs::copy(&old_path, &config_path)
                    .context("failed to migrate config from old location")?;
                fs::remove_file(&old_path)
                    .context("failed to remove old config file after migration")?;
                tracing::info!(
                    "migrated config from {} to {} and removed old config file",
                    old_path.display(),
                    config_path.display()
                );
            }
        }

        if !config_path.exists() {
            Self::create_default(&config_path).context("failed to create default config")?;
        }

        let contents = fs::read_to_string(&config_path).context("failed to read config file")?;

        let config: Self = toml::from_str(&contents).context("failed to parse config TOML")?;

        Ok(config)
    }

    fn config_path() -> Result<PathBuf> {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(".whisper-hotkey/config.toml"))
    }

    fn old_config_path() -> Result<PathBuf> {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(".whisper-hotkey.toml"))
    }

    fn create_default(path: &PathBuf) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("failed to create config directory")?;
        }

        let default_config = r#"[hotkey]
modifiers = ["Control", "Option"]
key = "Z"

[audio]
buffer_size = 1024
sample_rate = 16000

[model]
model_type = "Small"
preload = true
threads = 4        # CPU threads for inference (4 optimal for M1/M2)
beam_size = 5      # Beam search size (higher = more accurate but slower)
# language = "pl"  # Language hint: "en", "pl", "es", etc. Omit for auto-detect

[telemetry]
enabled = true
log_path = "~/.whisper-hotkey/crash.log"

[recording]
enabled = true                  # Save debug recordings to ~/.whisper-hotkey/debug/
retention_days = 7              # Delete recordings older than N days (0 = keep all)
max_count = 100                 # Keep only N most recent recordings (0 = unlimited)
cleanup_interval_hours = 1      # Hours between cleanup runs (0 = startup only)
"#;
        fs::write(path, default_config).context("failed to write default config")?;
        Ok(())
    }

    /// Save config to ~/.whisper-hotkey/config.toml
    ///
    /// # Errors
    /// Returns error if TOML serialization fails or file write fails
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context("failed to create config directory")?;
        }

        let contents =
            toml::to_string_pretty(self).context("failed to serialize config to TOML")?;
        fs::write(&config_path, contents).context("failed to write config file")?;
        Ok(())
    }

    /// Get config file path for external opening
    ///
    /// # Errors
    /// Returns error if HOME environment variable is not set
    pub fn get_config_path() -> Result<PathBuf> {
        Self::config_path()
    }

    /// Expand ~ in paths to home directory
    ///
    /// # Errors
    /// Returns error if HOME environment variable is not set
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
    use std::sync::Mutex;

    // Shared mutex for all tests that modify HOME
    static HOME_TEST_LOCK: Mutex<()> = Mutex::new(());

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
        assert_eq!(config.model.name, Some("small".to_owned()));
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
    fn test_model_config_effective_methods() {
        // Test with model_type set (should use model_type)
        let toml = r#"
[hotkey]
modifiers = ["Control"]
key = "Z"

[audio]
buffer_size = 1024
sample_rate = 16000

[model]
model_type = "Small"
preload = true
threads = 4
beam_size = 5

[telemetry]
enabled = true
log_path = "~/.whisper-hotkey/crash.log"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.model.effective_name(), "small");
        assert_eq!(
            config.model.effective_path(),
            "~/.whisper-hotkey/models/ggml-small.bin"
        );

        // Test with name/path fields (fallback when model_type is None)
        let toml = r#"
[hotkey]
modifiers = ["Control"]
key = "Z"

[audio]
buffer_size = 1024
sample_rate = 16000

[model]
name = "custom-model"
path = "/custom/path/model.bin"
preload = true
threads = 4
beam_size = 5

[telemetry]
enabled = true
log_path = "~/.whisper-hotkey/crash.log"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.model.effective_name(), "custom-model");
        assert_eq!(config.model.effective_path(), "/custom/path/model.bin");

        // Test with neither set (should return empty string)
        let toml = r#"
[hotkey]
modifiers = ["Control"]
key = "Z"

[audio]
buffer_size = 1024
sample_rate = 16000

[model]
preload = true
threads = 4
beam_size = 5

[telemetry]
enabled = true
log_path = "~/.whisper-hotkey/crash.log"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.model.effective_name(), "");
        assert_eq!(config.model.effective_path(), "");
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
    #[ignore = "requires filesystem access"]
    fn test_load_creates_default_if_missing() {
        // This test would require setting up a temp directory and manipulating HOME
        // Skip for now as it's integration-level testing
    }

    #[test]
    #[ignore = "requires filesystem access"]
    fn test_load_reads_existing_config() {
        // This test would require creating a temp config file
        // Skip for now as it's integration-level testing
    }

    #[test]
    #[ignore = "requires filesystem access"]
    fn test_config_migration_from_old_path() {
        use std::env;

        // Create unique temp directory for this test
        let temp_base = env::temp_dir();
        let test_home = temp_base.join(format!(
            "whisper_test_migration_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));
        fs::create_dir_all(&test_home).unwrap();

        // Save original HOME and set to temp directory
        let original_home = env::var("HOME").ok();
        env::set_var("HOME", test_home.to_str().unwrap());

        // Create old config at ~/.whisper-hotkey.toml
        let old_config_path = test_home.join(".whisper-hotkey.toml");
        let old_config_content = r#"[hotkey]
modifiers = ["Command", "Shift"]
key = "V"

[audio]
buffer_size = 2048
sample_rate = 16000

[model]
name = "base"
path = "/test/model.bin"
preload = true

[telemetry]
enabled = false
log_path = "/test/log.txt"
"#;
        fs::write(&old_config_path, old_config_content).unwrap();

        // Verify old config exists
        assert!(old_config_path.exists());

        // Load config (should trigger migration)
        let config = Config::load().unwrap();

        // Verify new config exists at ~/.whisper-hotkey/config.toml
        let new_config_path = test_home.join(".whisper-hotkey/config.toml");
        assert!(new_config_path.exists());

        // Verify old config was removed
        assert!(!old_config_path.exists());

        // Verify config content matches
        assert_eq!(config.hotkey.modifiers, vec!["Command", "Shift"]);
        assert_eq!(config.hotkey.key, "V");
        assert_eq!(config.audio.buffer_size, 2048);
        assert_eq!(config.model.name, Some("base".to_owned()));
        assert!(!config.telemetry.enabled);

        // Restore original HOME
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }

        // Cleanup test directory
        let _ = fs::remove_dir_all(&test_home);
    }

    #[test]
    fn test_config_serialize() {
        let config = Config {
            hotkey: HotkeyConfig {
                modifiers: vec!["Control".to_owned(), "Option".to_owned()],
                key: "Z".to_owned(),
            },
            audio: AudioConfig {
                buffer_size: 1024,
                sample_rate: 16000,
            },
            model: ModelConfig {
                model_type: Some(ModelType::Small),
                name: None,
                path: None,
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
        };

        let serialized = toml::to_string(&config).unwrap();
        assert!(serialized.contains("modifiers"));
        assert!(serialized.contains("Control"));
        assert!(serialized.contains("buffer_size"));
        assert!(serialized.contains("Small"));
    }

    #[test]
    fn test_config_roundtrip() {
        let original = Config {
            hotkey: HotkeyConfig {
                modifiers: vec!["Command".to_owned()],
                key: "V".to_owned(),
            },
            audio: AudioConfig {
                buffer_size: 2048,
                sample_rate: 16000,
            },
            model: ModelConfig {
                model_type: None,
                name: Some("base".to_owned()),
                path: Some("/tmp/model.bin".to_owned()),
                preload: false,
                threads: 8,
                beam_size: 10,
                language: Some("pl".to_owned()),
            },
            telemetry: TelemetryConfig {
                enabled: false,
                log_path: "/tmp/log.txt".to_owned(),
            },
            recording: RecordingConfig::default(),
        };

        let serialized = toml::to_string(&original).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.hotkey.modifiers, original.hotkey.modifiers);
        assert_eq!(deserialized.hotkey.key, original.hotkey.key);
        assert_eq!(deserialized.audio.buffer_size, original.audio.buffer_size);
        assert_eq!(deserialized.model.name, original.model.name);
        assert_eq!(deserialized.model.threads, original.model.threads);
        assert_eq!(deserialized.model.language, original.model.language);
        assert_eq!(deserialized.telemetry.enabled, original.telemetry.enabled);
    }

    #[test]
    fn test_config_path() {
        let path = Config::config_path().unwrap();
        assert!(path
            .to_string_lossy()
            .contains(".whisper-hotkey/config.toml"));
    }

    #[test]
    fn test_get_config_path() {
        let path = Config::get_config_path().unwrap();
        assert!(path
            .to_string_lossy()
            .contains(".whisper-hotkey/config.toml"));
    }

    #[test]
    fn test_old_config_path() {
        let path = Config::old_config_path().unwrap();
        assert!(path.to_string_lossy().contains(".whisper-hotkey.toml"));
        assert!(!path.to_string_lossy().contains(".whisper-hotkey/"));
    }

    #[test]
    fn test_create_default_creates_parent_directory() {
        use std::env;

        // Use temp directory to avoid interfering with actual config
        let temp_base = env::temp_dir();
        let test_dir = temp_base.join(format!(
            "whisper_test_create_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));

        // Create a nested path that doesn't exist
        let config_path = test_dir.join("nested").join("config.toml");

        // Ensure parent doesn't exist yet
        assert!(!config_path.parent().unwrap().exists());

        // Create default config (should create parent directory)
        Config::create_default(&config_path).unwrap();

        // Verify parent directory was created
        assert!(config_path.parent().unwrap().exists());

        // Verify config file was created
        assert!(config_path.exists());

        // Verify it's valid TOML
        let contents = fs::read_to_string(&config_path).unwrap();
        let _: Config = toml::from_str(&contents).unwrap();

        // Cleanup
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_load_with_migration() {
        // Use shared mutex to prevent parallel test execution from interfering with HOME
        let _guard = HOME_TEST_LOCK.lock().unwrap();

        // Create unique temp directory
        let temp_base = env::temp_dir();
        let test_home = temp_base.join(format!(
            "whisper_test_load_migration_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));
        fs::create_dir_all(&test_home).unwrap();

        // Save and set HOME
        let original_home = env::var("HOME").ok();
        env::set_var("HOME", test_home.to_str().unwrap());

        // Create old config
        let old_config_path = test_home.join(".whisper-hotkey.toml");
        let old_config = r#"[hotkey]
modifiers = ["Command"]
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
log_path = "/tmp/test.log"
"#;
        fs::write(&old_config_path, old_config).unwrap();

        // Load should trigger migration
        let config = Config::load().unwrap();

        // Verify migration occurred
        let new_path = test_home.join(".whisper-hotkey/config.toml");
        assert!(new_path.exists());
        assert!(!old_config_path.exists());

        // Verify config loaded correctly
        assert_eq!(config.hotkey.key, "M");
        assert_eq!(config.audio.buffer_size, 512);

        // Restore HOME
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }

        // Cleanup
        let _ = fs::remove_dir_all(&test_home);
    }

    #[test]
    fn test_load_creates_default_when_no_config_exists() {
        let _guard = HOME_TEST_LOCK.lock().unwrap();

        let temp_base = env::temp_dir();
        let test_home = temp_base.join(format!(
            "whisper_test_no_config_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));
        fs::create_dir_all(&test_home).unwrap();

        let original_home = env::var("HOME").ok();
        env::set_var("HOME", test_home.to_str().unwrap());

        // No config exists - should create default
        let config = Config::load().unwrap();

        // Verify new config was created at correct path
        let config_path = test_home.join(".whisper-hotkey/config.toml");
        assert!(config_path.exists());

        // Verify default values
        assert_eq!(config.hotkey.modifiers, vec!["Control", "Option"]);
        assert_eq!(config.hotkey.key, "Z");
        assert_eq!(config.audio.buffer_size, 1024);
        assert_eq!(config.audio.sample_rate, 16000);
        assert_eq!(config.model.model_type, Some(ModelType::Small));
        assert!(config.telemetry.enabled);

        // Restore HOME
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }

        let _ = fs::remove_dir_all(&test_home);
    }

    #[test]
    fn test_load_when_new_config_already_exists() {
        let _guard = HOME_TEST_LOCK.lock().unwrap();

        let temp_base = env::temp_dir();
        let test_home = temp_base.join(format!(
            "whisper_test_existing_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));
        fs::create_dir_all(&test_home).unwrap();

        let original_home = env::var("HOME").ok();
        env::set_var("HOME", test_home.to_str().unwrap());

        // Create new config at correct path
        let new_config_path = test_home.join(".whisper-hotkey/config.toml");
        fs::create_dir_all(new_config_path.parent().unwrap()).unwrap();
        let existing_config = r#"[hotkey]
modifiers = ["Control", "Shift"]
key = "X"

[audio]
buffer_size = 4096
sample_rate = 16000

[model]
name = "large"
path = "/custom/path.bin"
preload = false

[telemetry]
enabled = false
log_path = "/custom/log.txt"
"#;
        fs::write(&new_config_path, existing_config).unwrap();

        // Create old config too - should not migrate since new exists
        let old_config_path = test_home.join(".whisper-hotkey.toml");
        fs::write(&old_config_path, "ignored").unwrap();

        // Load - should use new config, ignore old
        let config = Config::load().unwrap();

        // Verify loaded from new config, not defaults
        assert_eq!(config.hotkey.key, "X");
        assert_eq!(config.audio.buffer_size, 4096);
        assert_eq!(config.model.name, Some("large".to_owned()));
        assert!(!config.telemetry.enabled);

        // Old config should still exist (not migrated)
        assert!(old_config_path.exists());

        // Restore HOME
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }

        let _ = fs::remove_dir_all(&test_home);
    }

    #[test]
    fn test_save_creates_config_file() {
        let _guard = HOME_TEST_LOCK.lock().unwrap();

        let temp_base = env::temp_dir();
        let test_dir = temp_base.join(format!(
            "whisper_test_save_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));
        fs::create_dir_all(&test_dir).unwrap();

        let original_home = env::var("HOME").ok();
        env::set_var("HOME", test_dir.to_str().unwrap());

        let config = Config {
            hotkey: HotkeyConfig {
                modifiers: vec!["Command".to_owned()],
                key: "T".to_owned(),
            },
            audio: AudioConfig {
                buffer_size: 2048,
                sample_rate: 16000,
            },
            model: ModelConfig {
                model_type: None,
                name: Some("base".to_owned()),
                path: Some("/test/base.bin".to_owned()),
                preload: true,
                threads: 4,
                beam_size: 5,
                language: Some("en".to_owned()),
            },
            telemetry: TelemetryConfig {
                enabled: true,
                log_path: "/test/log.txt".to_owned(),
            },
            recording: RecordingConfig::default(),
        };

        config.save().unwrap();

        let config_path = test_dir.join(".whisper-hotkey/config.toml");
        assert!(config_path.exists());

        let contents = fs::read_to_string(&config_path).unwrap();
        let loaded: Config = toml::from_str(&contents).unwrap();
        assert_eq!(loaded.hotkey.key, "T");
        assert_eq!(loaded.audio.buffer_size, 2048);

        // Restore HOME
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_parse_config_with_language() {
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
language = "en"

[telemetry]
enabled = true
log_path = "/tmp/crash.log"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.model.language, Some("en".to_owned()));
    }

    #[test]
    fn test_recording_config_defaults() {
        let config = RecordingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.retention_days, 7);
        assert_eq!(config.max_count, 100);
        assert_eq!(config.cleanup_interval_hours, 1);
    }

    #[test]
    fn test_parse_config_with_recording() {
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

[recording]
enabled = true
retention_days = 7
max_count = 100
cleanup_interval_hours = 1
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.recording.enabled);
        assert_eq!(config.recording.retention_days, 7);
        assert_eq!(config.recording.max_count, 100);
        assert_eq!(config.recording.cleanup_interval_hours, 1);
    }

    #[test]
    fn test_parse_config_without_recording() {
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
        // Should use defaults when section missing
        assert!(config.recording.enabled);
        assert_eq!(config.recording.retention_days, 7);
        assert_eq!(config.recording.max_count, 100);
        assert_eq!(config.recording.cleanup_interval_hours, 1);
    }

    #[test]
    fn test_parse_config_with_custom_recording() {
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

[recording]
enabled = false
retention_days = 30
max_count = 500
cleanup_interval_hours = 24
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(!config.recording.enabled);
        assert_eq!(config.recording.retention_days, 30);
        assert_eq!(config.recording.max_count, 500);
        assert_eq!(config.recording.cleanup_interval_hours, 24);
    }

    #[test]
    fn test_parse_config_with_partial_recording() {
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

[recording]
retention_days = 14
"#;
        let config: Config = toml::from_str(toml).unwrap();
        // Should use defaults for unspecified fields
        assert!(config.recording.enabled);
        assert_eq!(config.recording.retention_days, 14);
        assert_eq!(config.recording.max_count, 100);
        assert_eq!(config.recording.cleanup_interval_hours, 1);
    }

    #[test]
    fn test_recording_config_zero_values() {
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

[recording]
enabled = true
retention_days = 0
max_count = 0
cleanup_interval_hours = 0
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.recording.enabled);
        assert_eq!(config.recording.retention_days, 0);
        assert_eq!(config.recording.max_count, 0);
        assert_eq!(config.recording.cleanup_interval_hours, 0);
    }
}
