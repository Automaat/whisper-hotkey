use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::path::PathBuf;

/// Initialize telemetry logging
pub fn init(enabled: bool, log_path: &str) -> Result<()> {
    if !enabled {
        // Basic stdout logging only
        tracing_subscriber::fmt().with_target(false).init();
        return Ok(());
    }

    let expanded_path = expand_log_path(log_path)?;

    // Create parent directory if needed
    if let Some(parent) = expanded_path.parent() {
        fs::create_dir_all(parent).context("failed to create log directory")?;
    }

    // Set up file appender
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&expanded_path)
        .context("failed to open log file")?;

    tracing_subscriber::fmt()
        .with_writer(file)
        .with_target(false)
        .with_ansi(false)
        .init();

    tracing::info!("telemetry initialized: {}", expanded_path.display());

    Ok(())
}

fn expand_log_path(path: &str) -> Result<PathBuf> {
    if let Some(stripped) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(stripped))
    } else {
        Ok(PathBuf::from(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_expand_log_path_with_tilde() {
        let home = env::var("HOME").expect("HOME not set");
        let result = expand_log_path("~/logs/crash.log").unwrap();
        assert_eq!(result, PathBuf::from(home).join("logs/crash.log"));
    }

    #[test]
    fn test_expand_log_path_without_tilde() {
        let result = expand_log_path("/var/log/app.log").unwrap();
        assert_eq!(result, PathBuf::from("/var/log/app.log"));
    }

    #[test]
    fn test_expand_log_path_relative() {
        let result = expand_log_path("logs/app.log").unwrap();
        assert_eq!(result, PathBuf::from("logs/app.log"));
    }

    #[test]
    #[ignore] // Requires filesystem access and global tracing subscriber initialization
    fn test_init_with_telemetry_enabled() {
        // Would need to:
        // 1. Set up temp directory for log file
        // 2. Handle global tracing subscriber (can only init once per process)
        // 3. Verify file creation
        // Skip for now as it's integration-level testing
    }

    #[test]
    #[ignore] // Requires global tracing subscriber initialization
    fn test_init_with_telemetry_disabled() {
        // Would need to handle global tracing subscriber (can only init once per process)
        // Skip for now as it's integration-level testing
    }

    #[test]
    #[ignore] // Requires filesystem access
    fn test_init_creates_parent_directory() {
        // Would need to:
        // 1. Set up temp directory structure
        // 2. Verify parent directory creation
        // Skip for now as it's integration-level testing
    }
}
