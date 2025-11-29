use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::path::PathBuf;

/// Initialize telemetry logging
pub fn init(enabled: bool, log_path: &str) -> Result<()> {
    if !enabled {
        // Basic stdout logging only
        tracing_subscriber::fmt()
            .with_target(false)
            .init();
        return Ok(());
    }

    let expanded_path = expand_log_path(log_path)?;

    // Create parent directory if needed
    if let Some(parent) = expanded_path.parent() {
        fs::create_dir_all(parent)
            .context("failed to create log directory")?;
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
    if path.starts_with("~/") {
        let home = std::env::var("HOME")
            .context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(&path[2..]))
    } else {
        Ok(PathBuf::from(path))
    }
}
