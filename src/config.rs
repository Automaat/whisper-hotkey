use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Whisper model type variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

// Custom serde to serialize as "base.en" instead of "BaseEn"
impl Serialize for ModelType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ModelType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
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

    /// Parse model type from string (e.g., "base.en" -> `BaseEn`)
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "tiny" => Ok(Self::Tiny),
            "tiny.en" => Ok(Self::TinyEn),
            "base" => Ok(Self::Base),
            "base.en" => Ok(Self::BaseEn),
            "small" => Ok(Self::Small),
            "small.en" => Ok(Self::SmallEn),
            "medium" => Ok(Self::Medium),
            "medium.en" => Ok(Self::MediumEn),
            "large" => Ok(Self::Large),
            "large-v1" => Ok(Self::LargeV1),
            "large-v2" => Ok(Self::LargeV2),
            "large-v3" => Ok(Self::LargeV3),
            _ => Err(format!("unknown model type: {s}")),
        }
    }

    /// Get model name for `HuggingFace` download (same as `as_str`)
    #[must_use]
    pub const fn model_name(self) -> &'static str {
        self.as_str()
    }

    /// Get default path for model (as string with ~ for home)
    #[must_use]
    pub fn model_path(self) -> String {
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

#[allow(clippy::derivable_impls)] // We want Small as default, not Tiny (first variant)
impl Default for ModelType {
    fn default() -> Self {
        Self::Small // Small is a good balance of speed/accuracy
    }
}

// Helper functions for skip_serializing_if
fn is_default_hotkey(val: &HotkeyConfig) -> bool {
    val.modifiers.len() == 2
        && val.modifiers[0] == "Control"
        && val.modifiers[1] == "Option"
        && val.key == "Z"
}

fn is_default_audio(val: &AudioConfig) -> bool {
    val.buffer_size == AudioConfig::default().buffer_size
        && val.sample_rate == AudioConfig::default().sample_rate
}

fn is_default_model(val: &ModelConfig) -> bool {
    val.model_type == ModelType::Small
        && val.preload
        && val.threads == 4
        && val.beam_size == 1
        && val.language.as_deref() == Some("en")
}

fn is_default_telemetry(val: &TelemetryConfig) -> bool {
    val.enabled && val.log_path == "~/.whisper-hotkey/crash.log"
}

fn is_default_recording(val: &RecordingConfig) -> bool {
    let default = RecordingConfig::default();
    val.enabled == default.enabled
        && val.retention_days == default.retention_days
        && val.max_count == default.max_count
        && val.cleanup_interval_hours == default.cleanup_interval_hours
}

#[allow(clippy::float_cmp)]
fn is_default_aliases(val: &AliasesConfig) -> bool {
    val.enabled && val.threshold == 0.8 && val.entries.is_empty()
}

fn is_default_profiles(val: &[TranscriptionProfile]) -> bool {
    if val.len() != 1 {
        return false;
    }
    let profile = &val[0];
    profile.name.is_none()
        && profile.model_type == ModelType::BaseEn
        && is_default_hotkey(&profile.hotkey)
        && profile.preload
        && profile.threads == 4
        && profile.beam_size == 1
        && profile.language.as_deref() == Some("en")
}

/// Transcription profile combining hotkey and model configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TranscriptionProfile {
    /// Optional explicit profile name (auto-generated if multiple profiles share same `model_type`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Model type (e.g., "base.en", "small", "tiny")
    pub model_type: ModelType,
    /// Hotkey configuration (inlined)
    #[serde(flatten)]
    pub hotkey: HotkeyConfig,
    /// Preload model at startup
    #[serde(default = "default_preload")]
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

impl TranscriptionProfile {
    /// Get profile name (explicit name or derived from model type)
    #[must_use]
    pub fn name(&self) -> &str {
        self.name
            .as_deref()
            .unwrap_or_else(|| self.model_type.as_str())
    }

    /// Get model path for this profile
    #[must_use]
    pub fn model_path(&self) -> String {
        self.model_type.model_path()
    }
}

/// Application configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// Transcription profiles (each with hotkey + model config)
    #[serde(
        default = "default_profiles",
        skip_serializing_if = "is_default_profiles"
    )]
    pub profiles: Vec<TranscriptionProfile>,
    /// Hotkey configuration
    #[serde(default, skip_serializing_if = "is_default_hotkey")]
    #[allow(dead_code)] // Used in Phase 2+
    pub hotkey: HotkeyConfig,
    /// Audio capture configuration
    #[serde(default, skip_serializing_if = "is_default_audio")]
    #[allow(dead_code)] // Used in Phase 3+
    pub audio: AudioConfig,
    /// Whisper model configuration
    #[serde(default, skip_serializing_if = "is_default_model")]
    #[allow(dead_code)] // Used in Phase 4+
    pub model: ModelConfig,
    /// Telemetry configuration
    #[serde(default, skip_serializing_if = "is_default_telemetry")]
    pub telemetry: TelemetryConfig,
    /// Recording configuration
    #[serde(default, skip_serializing_if = "is_default_recording")]
    pub recording: RecordingConfig,
    /// Aliases configuration
    #[serde(default, skip_serializing_if = "is_default_aliases")]
    pub aliases: AliasesConfig,
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

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            modifiers: vec!["Control".to_owned(), "Option".to_owned()],
            key: "Z".to_owned(),
        }
    }
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

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            buffer_size: 1024,
            sample_rate: 16000,
        }
    }
}

/// Whisper model configuration
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// Model type (e.g., "base.en", "small", "tiny")
    pub model_type: ModelType,
    /// Preload model at startup
    pub preload: bool,
    /// Number of CPU threads for inference
    pub threads: usize,
    /// Beam search width (higher = slower but more accurate)
    pub beam_size: usize,
    /// Language code (None = auto-detect)
    pub language: Option<String>,
}

// Helper struct for deserializing old config format
#[derive(Deserialize)]
struct ModelConfigHelper {
    #[serde(default)]
    model_type: Option<ModelType>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    #[allow(dead_code)] // Needed for deserialization but not used (path is ignored)
    path: Option<String>,
    #[serde(default = "default_preload")]
    preload: bool,
    #[serde(default = "default_threads")]
    threads: usize,
    #[serde(default = "default_beam_size")]
    beam_size: usize,
    #[serde(default = "default_language")]
    language: Option<String>,
}

const fn default_preload() -> bool {
    true
}

const fn default_threads() -> usize {
    4 // Optimal for M1/M2 chips
}

const fn default_beam_size() -> usize {
    1 // Greedy decoding (fast)
}

#[allow(clippy::unnecessary_wraps)]
fn default_language() -> Option<String> {
    Some("en".to_owned()) // English by default (skips auto-detect overhead)
}

fn default_profiles() -> Vec<TranscriptionProfile> {
    vec![TranscriptionProfile {
        name: None,
        model_type: ModelType::BaseEn,
        hotkey: HotkeyConfig::default(),
        preload: default_preload(),
        threads: default_threads(),
        beam_size: default_beam_size(),
        language: default_language(),
    }]
}

impl<'de> Deserialize<'de> for ModelConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = ModelConfigHelper::deserialize(deserializer)?;

        // Migrate from old format if model_type is not set
        let model_type = if let Some(mt) = helper.model_type {
            mt
        } else if let Some(name) = helper.name {
            // Try to parse name into ModelType
            ModelType::from_str(&name).unwrap_or_default()
        } else {
            ModelType::default()
        };

        Ok(Self {
            model_type,
            preload: helper.preload,
            threads: helper.threads,
            beam_size: helper.beam_size,
            language: helper.language,
        })
    }
}

impl Serialize for ModelConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ModelConfig", 5)?;
        state.serialize_field("model_type", &self.model_type)?;
        state.serialize_field("preload", &self.preload)?;
        state.serialize_field("threads", &self.threads)?;
        state.serialize_field("beam_size", &self.beam_size)?;
        state.serialize_field("language", &self.language)?;
        state.end()
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_type: ModelType::default(),
            preload: default_preload(),
            threads: default_threads(),
            beam_size: default_beam_size(),
            language: default_language(),
        }
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

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_path: "~/.whisper-hotkey/crash.log".to_owned(),
        }
    }
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

/// Aliases configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AliasesConfig {
    /// Enable alias matching
    #[serde(default = "default_aliases_enabled")]
    pub enabled: bool,
    /// Minimum similarity threshold (0.0-1.0)
    #[serde(default = "default_aliases_threshold")]
    pub threshold: f64,
    /// Alias mappings (trigger phrase -> output text)
    #[serde(default)]
    pub entries: HashMap<String, String>,
}

const fn default_aliases_enabled() -> bool {
    true
}

const fn default_aliases_threshold() -> f64 {
    0.8
}

impl Default for AliasesConfig {
    fn default() -> Self {
        Self {
            enabled: default_aliases_enabled(),
            threshold: default_aliases_threshold(),
            entries: HashMap::new(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            profiles: default_profiles(),
            hotkey: HotkeyConfig::default(),
            audio: AudioConfig::default(),
            model: ModelConfig::default(),
            telemetry: TelemetryConfig::default(),
            recording: RecordingConfig::default(),
            aliases: AliasesConfig::default(),
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

        let mut contents =
            fs::read_to_string(&config_path).context("failed to read config file")?;

        // Migrate to sparse format if config has content (create backup first)
        // Skip if backup already exists (already migrated) or file is empty
        let backup_path = config_path.with_extension("toml.bak");
        if !contents.trim().is_empty() && !backup_path.exists() {
            Self::migrate_to_sparse(&config_path)
                .context("failed to migrate config to sparse format")?;
            // Re-read contents after migration
            contents = fs::read_to_string(&config_path).context("failed to read config file")?;
        }

        let mut config: Self = toml::from_str(&contents).context("failed to parse config TOML")?;

        // Migrate from old [hotkey]/[model] format to [[profiles]]
        // Check if profiles is empty/default AND old sections exist (non-default values)
        let needs_migration = (config.profiles.is_empty() || is_default_profiles(&config.profiles))
            && (!is_default_hotkey(&config.hotkey) || !is_default_model(&config.model));

        if needs_migration {
            tracing::info!("migrating config from old [hotkey]/[model] format to [[profiles]]");
            config.migrate_to_profiles();
            config.save().context("failed to save migrated config")?;
        }

        // Check if [model] section had old fields (name/path) and save migrated version
        let had_old_fields = contents
            .split("[model]")
            .nth(1)
            .and_then(|model_section| model_section.split('[').next())
            .is_some_and(|model_only| {
                model_only.contains("name =") || model_only.contains("path =")
            });
        if had_old_fields {
            tracing::info!("migrating config: removing deprecated 'name' and 'path' fields");
            config.save().context("failed to save migrated config")?;
        }

        // Ensure unique profile names (auto-generate for duplicates)
        config.ensure_unique_names();

        // Ensure at least one profile exists
        if config.profiles.is_empty() {
            anyhow::bail!(
                "config must contain at least one profile - add a [[profiles]] section to {}",
                Self::config_path()?.display()
            );
        }

        // Validate hotkey conflicts
        config.validate_hotkeys()?;

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

        // Create empty config file - all defaults come from code
        fs::write(path, "").context("failed to write default config")?;
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

    /// Migrate existing config to sparse format (removes default values)
    /// Creates backup as config.toml.bak before migration
    ///
    /// # Errors
    /// Returns error if backup creation or save fails
    fn migrate_to_sparse(config_path: &PathBuf) -> Result<()> {
        // Read current config contents
        let contents = fs::read_to_string(config_path).context("failed to read config file")?;

        // Skip if already empty (already sparse)
        if contents.trim().is_empty() {
            return Ok(());
        }

        // Create backup
        let backup_path = config_path.with_extension("toml.bak");
        fs::copy(config_path, &backup_path).context("failed to create config backup")?;
        tracing::info!("created config backup at {}", backup_path.display());

        // Load config (parses with defaults)
        let config: Self = toml::from_str(&contents).context("failed to parse config TOML")?;

        // Save (will skip default values due to skip_serializing_if)
        let sparse_contents =
            toml::to_string_pretty(&config).context("failed to serialize config to TOML")?;
        fs::write(config_path, sparse_contents).context("failed to write migrated config")?;

        tracing::info!("migrated config to sparse format");
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

    /// Migrate from old [hotkey]/[model] format to [[profiles]]
    /// Converts single hotkey + model config into a single profile
    fn migrate_to_profiles(&mut self) {
        // Only migrate if profiles is empty or using defaults
        if !self.profiles.is_empty() && !is_default_profiles(&self.profiles) {
            return;
        }

        // Create single profile from old hotkey + model
        self.profiles = vec![TranscriptionProfile {
            name: None,
            model_type: self.model.model_type,
            hotkey: self.hotkey.clone(),
            preload: self.model.preload,
            threads: self.model.threads,
            beam_size: self.model.beam_size,
            language: self.model.language.clone(),
        }];
    }

    /// Ensure unique profile names by auto-generating suffixes for duplicates
    fn ensure_unique_names(&mut self) {
        use std::collections::HashMap;

        // Count occurrences of each derived name
        let mut name_counts: HashMap<String, usize> = HashMap::new();
        for profile in &self.profiles {
            let derived_name = profile.model_type.as_str().to_owned();
            *name_counts.entry(derived_name).or_insert(0) += 1;
        }

        // Auto-generate unique names for duplicates
        let mut name_counters: HashMap<String, usize> = HashMap::new();
        for profile in &mut self.profiles {
            if profile.name.is_none() {
                let derived_name = profile.model_type.as_str().to_owned();
                if *name_counts.get(&derived_name).unwrap_or(&0) > 1 {
                    // Multiple profiles with same model_type, generate unique name
                    let counter = name_counters.entry(derived_name.clone()).or_insert(0);
                    *counter += 1;
                    profile.name = Some(format!("{derived_name}-{counter}"));
                }
            }
        }
    }

    /// Validate no duplicate hotkeys across profiles
    ///
    /// # Errors
    /// Returns error if duplicate hotkeys are found
    fn validate_hotkeys(&self) -> Result<()> {
        use std::collections::HashSet;

        let mut seen = HashSet::new();
        for profile in &self.profiles {
            // Sort modifiers for consistent signature (order-independent)
            let mut sorted_mods = profile.hotkey.modifiers.clone();
            sorted_mods.sort();
            let hotkey_sig = format!("{:?}+{}", sorted_mods, profile.hotkey.key);

            if !seen.insert(hotkey_sig.clone()) {
                anyhow::bail!(
                    "duplicate hotkey detected: {} (profiles: {})",
                    hotkey_sig,
                    self.profiles
                        .iter()
                        .filter(|p| {
                            let mut sorted_mods = p.hotkey.modifiers.clone();
                            sorted_mods.sort();
                            format!("{:?}+{}", sorted_mods, p.hotkey.key) == hotkey_sig
                        })
                        .map(TranscriptionProfile::name)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }

        Ok(())
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
model_type = "small"
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
        assert_eq!(config.model.model_type, ModelType::Small);
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
model_type = "small"
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
model_type = "base"
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
    fn test_model_config_migration() {
        // Test with model_type (new format)
        let toml = r#"
[hotkey]
modifiers = ["Control"]
key = "Z"

[audio]
buffer_size = 1024
sample_rate = 16000

[model]
model_type = "small"
preload = true
threads = 4
beam_size = 5

[telemetry]
enabled = true
log_path = "~/.whisper-hotkey/crash.log"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.model.model_type, ModelType::Small);
        assert_eq!(
            config.model.model_type.model_path(),
            "~/.whisper-hotkey/models/ggml-small.bin"
        );

        // Test migration from old format (name/path fields)
        let toml = r#"
[hotkey]
modifiers = ["Control"]
key = "Z"

[audio]
buffer_size = 1024
sample_rate = 16000

[model]
name = "base.en"
path = "/custom/path/model.bin"
preload = true
threads = 4
beam_size = 5

[telemetry]
enabled = true
log_path = "~/.whisper-hotkey/crash.log"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        // Old name field should be migrated to model_type
        assert_eq!(config.model.model_type, ModelType::BaseEn);
        // Path is ignored, model_type determines the path
        assert_eq!(
            config.model.model_type.model_path(),
            "~/.whisper-hotkey/models/ggml-base.en.bin"
        );

        // Test with neither model_type nor name set (should use default)
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
        assert_eq!(config.model.model_type, ModelType::Small); // Default
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
        assert_eq!(config.model.beam_size, 1);
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
        assert_eq!(config.model.model_type, ModelType::Base);
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
        // Test with all default values - should serialize to nearly empty
        let default_config = Config::default();
        let serialized = toml::to_string(&default_config).unwrap();
        // Default values should not appear in serialization
        assert!(!serialized.contains("modifiers"));
        assert!(!serialized.contains("buffer_size"));
        assert!(!serialized.contains("small"));

        // Test with non-default values - should serialize them
        let config = Config {
            profiles: default_profiles(),
            hotkey: HotkeyConfig {
                modifiers: vec!["Command".to_owned()],
                key: "V".to_owned(),
            },
            audio: AudioConfig {
                buffer_size: 2048,
                sample_rate: 16000,
            },
            model: ModelConfig {
                model_type: ModelType::Base,
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
        };

        let serialized = toml::to_string(&config).unwrap();
        // Non-default values should appear
        assert!(serialized.contains("modifiers"));
        assert!(serialized.contains("Command"));
        assert!(serialized.contains("buffer_size"));
        assert!(serialized.contains("2048"));
        assert!(serialized.contains("base")); // model_type serializes as lowercase
        assert!(serialized.contains("beam_size"));
        assert!(serialized.contains('5'));
    }

    #[test]
    fn test_config_roundtrip() {
        let original = Config {
            profiles: default_profiles(),
            hotkey: HotkeyConfig {
                modifiers: vec!["Command".to_owned()],
                key: "V".to_owned(),
            },
            audio: AudioConfig {
                buffer_size: 2048,
                sample_rate: 16000,
            },
            model: ModelConfig {
                model_type: ModelType::Base,
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
            aliases: AliasesConfig::default(),
        };

        let serialized = toml::to_string(&original).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.hotkey.modifiers, original.hotkey.modifiers);
        assert_eq!(deserialized.hotkey.key, original.hotkey.key);
        assert_eq!(deserialized.audio.buffer_size, original.audio.buffer_size);
        assert_eq!(deserialized.model.model_type, original.model.model_type);
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
        assert_eq!(config.model.model_type, ModelType::Small);
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
        assert_eq!(config.model.model_type, ModelType::Large);
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
            profiles: default_profiles(),
            hotkey: HotkeyConfig {
                modifiers: vec!["Command".to_owned()],
                key: "T".to_owned(),
            },
            audio: AudioConfig {
                buffer_size: 2048,
                sample_rate: 16000,
            },
            model: ModelConfig {
                model_type: ModelType::Base,
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
            aliases: AliasesConfig::default(),
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

    #[test]
    fn test_unknown_fields_stripped_during_serialization() {
        // Config with unknown/deprecated fields
        let toml = r#"
[hotkey]
modifiers = ["Control"]
key = "M"
unknown_hotkey_field = "should be ignored"

[audio]
buffer_size = 512
sample_rate = 16000
deprecated_field = 123

[model]
model_type = "tiny"
preload = true
old_field = "removed"

[telemetry]
enabled = true
log_path = "/tmp/crash.log"

[unknown_section]
foo = "bar"

[recording]
enabled = false
unknown_param = "test"
"#;
        // Deserialize (unknown fields ignored)
        let config: Config = toml::from_str(toml).unwrap();

        // Serialize back
        let serialized = toml::to_string(&config).unwrap();

        // Verify unknown fields not present
        assert!(!serialized.contains("unknown_hotkey_field"));
        assert!(!serialized.contains("deprecated_field"));
        assert!(!serialized.contains("old_field"));
        assert!(!serialized.contains("unknown_section"));
        assert!(!serialized.contains("unknown_param"));

        // Verify known fields are preserved
        assert_eq!(config.hotkey.modifiers, vec!["Control"]);
        assert_eq!(config.hotkey.key, "M");
        assert_eq!(config.audio.buffer_size, 512);
        assert!(!config.recording.enabled);
    }

    #[test]
    fn test_migrate_to_sparse() {
        let _guard = HOME_TEST_LOCK.lock().unwrap();

        let temp_base = env::temp_dir();
        let test_home = temp_base.join(format!(
            "whisper_test_sparse_migration_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));
        fs::create_dir_all(&test_home).unwrap();

        let original_home = env::var("HOME").ok();
        env::set_var("HOME", test_home.to_str().unwrap());

        // Create config with all fields explicitly set (including defaults)
        let config_path = test_home.join(".whisper-hotkey/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        let full_config = r#"[hotkey]
modifiers = ["Control", "Option"]
key = "Z"

[audio]
buffer_size = 1024
sample_rate = 16000

[model]
model_type = "small"
preload = true
threads = 4
beam_size = 1
language = "en"

[telemetry]
enabled = true
log_path = "~/.whisper-hotkey/crash.log"

[recording]
enabled = true
retention_days = 7
max_count = 100
cleanup_interval_hours = 1
"#;
        fs::write(&config_path, full_config).unwrap();

        // Verify backup doesn't exist yet
        let backup_path = config_path.with_extension("toml.bak");
        assert!(!backup_path.exists());

        // Load config (should trigger sparse migration)
        let config = Config::load().unwrap();

        // Verify backup exists with original content
        assert!(backup_path.exists());
        let backup_contents = fs::read_to_string(&backup_path).unwrap();
        assert_eq!(backup_contents.trim(), full_config.trim());

        // Verify migrated file omits default values (sparse format)
        let migrated_contents = fs::read_to_string(&config_path).unwrap();
        // All fields are defaults, so should be empty or minimal
        assert!(
            migrated_contents.trim().is_empty()
                || migrated_contents.trim().len() < full_config.len()
        );

        // Verify config loads correctly (defaults applied)
        assert_eq!(config.hotkey.modifiers, vec!["Control", "Option"]);
        assert_eq!(config.hotkey.key, "Z");
        assert_eq!(config.audio.buffer_size, 1024);
        assert_eq!(config.model.model_type, ModelType::Small);
        assert_eq!(config.model.threads, 4);

        // Second load should not re-migrate (idempotent)
        let backup_modified = fs::metadata(&backup_path).unwrap().modified().unwrap();
        let config2 = Config::load().unwrap();
        let backup_modified2 = fs::metadata(&backup_path).unwrap().modified().unwrap();
        assert_eq!(backup_modified, backup_modified2); // Backup not recreated
        assert_eq!(config2.hotkey.key, config.hotkey.key);

        // Restore HOME
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }

        let _ = fs::remove_dir_all(&test_home);
    }

    #[test]
    fn test_transcription_profile_name_explicit() {
        let profile = TranscriptionProfile {
            name: Some("custom-name".to_owned()),
            model_type: ModelType::BaseEn,
            hotkey: HotkeyConfig::default(),
            preload: true,
            threads: 4,
            beam_size: 1,
            language: Some("en".to_owned()),
        };
        assert_eq!(profile.name(), "custom-name");
    }

    #[test]
    fn test_transcription_profile_name_derived() {
        let profile = TranscriptionProfile {
            name: None,
            model_type: ModelType::Small,
            hotkey: HotkeyConfig::default(),
            preload: true,
            threads: 4,
            beam_size: 1,
            language: Some("en".to_owned()),
        };
        assert_eq!(profile.name(), "small");
    }

    #[test]
    fn test_transcription_profile_model_path() {
        let profile = TranscriptionProfile {
            name: None,
            model_type: ModelType::BaseEn,
            hotkey: HotkeyConfig::default(),
            preload: true,
            threads: 4,
            beam_size: 1,
            language: Some("en".to_owned()),
        };
        let path = profile.model_path();
        assert!(path.contains("base.en"));
        assert!(path.contains(".whisper-hotkey/models"));
    }

    #[test]
    fn test_is_default_profiles_true() {
        let profiles = default_profiles();
        assert!(is_default_profiles(&profiles));
    }

    #[test]
    fn test_is_default_profiles_false_multiple() {
        let profiles = vec![
            TranscriptionProfile {
                name: None,
                model_type: ModelType::BaseEn,
                hotkey: HotkeyConfig::default(),
                preload: true,
                threads: 4,
                beam_size: 1,
                language: Some("en".to_owned()),
            },
            TranscriptionProfile {
                name: None,
                model_type: ModelType::Small,
                hotkey: HotkeyConfig {
                    modifiers: vec!["Shift".to_owned()],
                    key: "X".to_owned(),
                },
                preload: true,
                threads: 4,
                beam_size: 1,
                language: Some("en".to_owned()),
            },
        ];
        assert!(!is_default_profiles(&profiles));
    }

    #[test]
    fn test_is_default_profiles_false_custom() {
        let profiles = vec![TranscriptionProfile {
            name: Some("custom".to_owned()),
            model_type: ModelType::BaseEn,
            hotkey: HotkeyConfig::default(),
            preload: true,
            threads: 4,
            beam_size: 1,
            language: Some("en".to_owned()),
        }];
        assert!(!is_default_profiles(&profiles));
    }

    #[test]
    fn test_config_migrate_to_profiles() {
        let mut config = Config {
            profiles: vec![],
            hotkey: HotkeyConfig {
                modifiers: vec!["Command".to_owned(), "Shift".to_owned()],
                key: "V".to_owned(),
            },
            audio: AudioConfig::default(),
            model: ModelConfig {
                model_type: ModelType::Small,
                preload: false,
                threads: 8,
                beam_size: 5,
                language: Some("es".to_owned()),
            },
            telemetry: TelemetryConfig::default(),
            recording: RecordingConfig::default(),
            aliases: AliasesConfig::default(),
        };

        config.migrate_to_profiles();

        assert_eq!(config.profiles.len(), 1);
        assert_eq!(config.profiles[0].model_type, ModelType::Small);
        assert_eq!(config.profiles[0].threads, 8);
        assert_eq!(config.profiles[0].beam_size, 5);
        assert_eq!(config.profiles[0].language, Some("es".to_owned()));
        assert_eq!(config.profiles[0].hotkey.key, "V");
        assert_eq!(config.profiles[0].hotkey.modifiers.len(), 2);
    }

    #[test]
    fn test_config_migrate_to_profiles_noop_if_profiles_exist() {
        let mut config = Config {
            profiles: vec![TranscriptionProfile {
                name: Some("existing".to_owned()),
                model_type: ModelType::Tiny,
                hotkey: HotkeyConfig {
                    modifiers: vec!["Alt".to_owned()],
                    key: "Q".to_owned(),
                },
                preload: true,
                threads: 2,
                beam_size: 3,
                language: Some("fr".to_owned()),
            }],
            hotkey: HotkeyConfig {
                modifiers: vec!["Command".to_owned()],
                key: "V".to_owned(),
            },
            audio: AudioConfig::default(),
            model: ModelConfig {
                model_type: ModelType::Small,
                preload: false,
                threads: 8,
                beam_size: 5,
                language: Some("es".to_owned()),
            },
            telemetry: TelemetryConfig::default(),
            recording: RecordingConfig::default(),
            aliases: AliasesConfig::default(),
        };

        config.migrate_to_profiles();

        // Should not change existing profiles
        assert_eq!(config.profiles.len(), 1);
        assert_eq!(config.profiles[0].name, Some("existing".to_owned()));
        assert_eq!(config.profiles[0].model_type, ModelType::Tiny);
    }

    #[test]
    fn test_config_ensure_unique_names_single_profile() {
        let mut config = Config {
            profiles: vec![TranscriptionProfile {
                name: None,
                model_type: ModelType::BaseEn,
                hotkey: HotkeyConfig::default(),
                preload: true,
                threads: 4,
                beam_size: 1,
                language: Some("en".to_owned()),
            }],
            hotkey: HotkeyConfig::default(),
            audio: AudioConfig::default(),
            model: ModelConfig::default(),
            telemetry: TelemetryConfig::default(),
            recording: RecordingConfig::default(),
            aliases: AliasesConfig::default(),
        };

        config.ensure_unique_names();

        // Single profile should keep derived name (no suffix)
        assert_eq!(config.profiles[0].name, None);
    }

    #[test]
    fn test_config_ensure_unique_names_duplicates() {
        let mut config = Config {
            profiles: vec![
                TranscriptionProfile {
                    name: None,
                    model_type: ModelType::BaseEn,
                    hotkey: HotkeyConfig {
                        modifiers: vec!["Command".to_owned()],
                        key: "A".to_owned(),
                    },
                    preload: true,
                    threads: 4,
                    beam_size: 1,
                    language: Some("en".to_owned()),
                },
                TranscriptionProfile {
                    name: None,
                    model_type: ModelType::BaseEn,
                    hotkey: HotkeyConfig {
                        modifiers: vec!["Command".to_owned()],
                        key: "B".to_owned(),
                    },
                    preload: true,
                    threads: 4,
                    beam_size: 1,
                    language: Some("en".to_owned()),
                },
                TranscriptionProfile {
                    name: None,
                    model_type: ModelType::Small,
                    hotkey: HotkeyConfig {
                        modifiers: vec!["Command".to_owned()],
                        key: "C".to_owned(),
                    },
                    preload: true,
                    threads: 4,
                    beam_size: 1,
                    language: Some("en".to_owned()),
                },
            ],
            hotkey: HotkeyConfig::default(),
            audio: AudioConfig::default(),
            model: ModelConfig::default(),
            telemetry: TelemetryConfig::default(),
            recording: RecordingConfig::default(),
            aliases: AliasesConfig::default(),
        };

        config.ensure_unique_names();

        // Two base.en profiles should get unique names
        assert_eq!(config.profiles[0].name, Some("base.en-1".to_owned()));
        assert_eq!(config.profiles[1].name, Some("base.en-2".to_owned()));
        // Single small profile should keep derived name
        assert_eq!(config.profiles[2].name, None);
    }

    #[test]
    fn test_config_ensure_unique_names_preserves_explicit() {
        let mut config = Config {
            profiles: vec![
                TranscriptionProfile {
                    name: Some("custom-1".to_owned()),
                    model_type: ModelType::BaseEn,
                    hotkey: HotkeyConfig {
                        modifiers: vec!["Command".to_owned()],
                        key: "A".to_owned(),
                    },
                    preload: true,
                    threads: 4,
                    beam_size: 1,
                    language: Some("en".to_owned()),
                },
                TranscriptionProfile {
                    name: None,
                    model_type: ModelType::BaseEn,
                    hotkey: HotkeyConfig {
                        modifiers: vec!["Command".to_owned()],
                        key: "B".to_owned(),
                    },
                    preload: true,
                    threads: 4,
                    beam_size: 1,
                    language: Some("en".to_owned()),
                },
            ],
            hotkey: HotkeyConfig::default(),
            audio: AudioConfig::default(),
            model: ModelConfig::default(),
            telemetry: TelemetryConfig::default(),
            recording: RecordingConfig::default(),
            aliases: AliasesConfig::default(),
        };

        config.ensure_unique_names();

        // Explicit name should be preserved
        assert_eq!(config.profiles[0].name, Some("custom-1".to_owned()));
        // Second profile gets unique name (base.en-1, not base.en-2 since first has explicit name)
        assert_eq!(config.profiles[1].name, Some("base.en-1".to_owned()));
    }

    #[test]
    fn test_config_validate_hotkeys_no_duplicates() {
        let config = Config {
            profiles: vec![
                TranscriptionProfile {
                    name: None,
                    model_type: ModelType::BaseEn,
                    hotkey: HotkeyConfig {
                        modifiers: vec!["Command".to_owned(), "Shift".to_owned()],
                        key: "A".to_owned(),
                    },
                    preload: true,
                    threads: 4,
                    beam_size: 1,
                    language: Some("en".to_owned()),
                },
                TranscriptionProfile {
                    name: None,
                    model_type: ModelType::Small,
                    hotkey: HotkeyConfig {
                        modifiers: vec!["Command".to_owned(), "Option".to_owned()],
                        key: "B".to_owned(),
                    },
                    preload: true,
                    threads: 4,
                    beam_size: 1,
                    language: Some("en".to_owned()),
                },
            ],
            hotkey: HotkeyConfig::default(),
            audio: AudioConfig::default(),
            model: ModelConfig::default(),
            telemetry: TelemetryConfig::default(),
            recording: RecordingConfig::default(),
            aliases: AliasesConfig::default(),
        };

        assert!(config.validate_hotkeys().is_ok());
    }

    #[test]
    fn test_config_validate_hotkeys_duplicate_exact() {
        let config = Config {
            profiles: vec![
                TranscriptionProfile {
                    name: Some("profile-1".to_owned()),
                    model_type: ModelType::BaseEn,
                    hotkey: HotkeyConfig {
                        modifiers: vec!["Command".to_owned(), "Shift".to_owned()],
                        key: "A".to_owned(),
                    },
                    preload: true,
                    threads: 4,
                    beam_size: 1,
                    language: Some("en".to_owned()),
                },
                TranscriptionProfile {
                    name: Some("profile-2".to_owned()),
                    model_type: ModelType::Small,
                    hotkey: HotkeyConfig {
                        modifiers: vec!["Command".to_owned(), "Shift".to_owned()],
                        key: "A".to_owned(),
                    },
                    preload: true,
                    threads: 4,
                    beam_size: 1,
                    language: Some("en".to_owned()),
                },
            ],
            hotkey: HotkeyConfig::default(),
            audio: AudioConfig::default(),
            model: ModelConfig::default(),
            telemetry: TelemetryConfig::default(),
            recording: RecordingConfig::default(),
            aliases: AliasesConfig::default(),
        };

        let result = config.validate_hotkeys();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("duplicate hotkey"));
        assert!(err.contains("profile-1") || err.contains("profile-2"));
    }

    #[test]
    fn test_config_validate_hotkeys_duplicate_order_independent() {
        let config = Config {
            profiles: vec![
                TranscriptionProfile {
                    name: Some("profile-1".to_owned()),
                    model_type: ModelType::BaseEn,
                    hotkey: HotkeyConfig {
                        modifiers: vec!["Command".to_owned(), "Shift".to_owned()],
                        key: "A".to_owned(),
                    },
                    preload: true,
                    threads: 4,
                    beam_size: 1,
                    language: Some("en".to_owned()),
                },
                TranscriptionProfile {
                    name: Some("profile-2".to_owned()),
                    model_type: ModelType::Small,
                    hotkey: HotkeyConfig {
                        modifiers: vec!["Shift".to_owned(), "Command".to_owned()],
                        key: "A".to_owned(),
                    },
                    preload: true,
                    threads: 4,
                    beam_size: 1,
                    language: Some("en".to_owned()),
                },
            ],
            hotkey: HotkeyConfig::default(),
            audio: AudioConfig::default(),
            model: ModelConfig::default(),
            telemetry: TelemetryConfig::default(),
            recording: RecordingConfig::default(),
            aliases: AliasesConfig::default(),
        };

        let result = config.validate_hotkeys();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("duplicate hotkey"));
    }

    #[test]
    fn test_default_profiles_creates_single_profile() {
        let profiles = default_profiles();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].model_type, ModelType::BaseEn);
        assert_eq!(profiles[0].name, None);
        assert!(profiles[0].preload);
        assert_eq!(profiles[0].threads, 4);
        assert_eq!(profiles[0].beam_size, 1);
        assert_eq!(profiles[0].language, Some("en".to_owned()));
    }

    #[test]
    fn test_parse_config_with_profiles() {
        let toml = r#"
[[profiles]]
model_type = "small"
modifiers = ["Command", "Shift"]
key = "A"
preload = true
threads = 8
beam_size = 5
language = "en"

[[profiles]]
name = "spanish-tiny"
model_type = "tiny"
modifiers = ["Command", "Option"]
key = "S"
preload = false
threads = 4
beam_size = 1
language = "es"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.profiles.len(), 2);
        assert_eq!(config.profiles[0].model_type, ModelType::Small);
        assert_eq!(config.profiles[0].threads, 8);
        assert_eq!(config.profiles[1].name, Some("spanish-tiny".to_owned()));
        assert_eq!(config.profiles[1].model_type, ModelType::Tiny);
        assert_eq!(config.profiles[1].language, Some("es".to_owned()));
    }

    #[test]
    fn test_config_default_has_one_profile() {
        let config = Config::default();
        assert_eq!(config.profiles.len(), 1);
        assert_eq!(config.profiles[0].model_type, ModelType::BaseEn);
    }
}
