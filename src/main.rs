mod config;
mod permissions;
mod telemetry;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Phase 1: Foundation
    // Load configuration
    let config = config::Config::load()?;
    println!("✓ Config loaded from ~/.whisper-hotkey.toml");

    // Initialize telemetry
    telemetry::init(config.telemetry.enabled, &config.telemetry.log_path)?;
    tracing::info!("whisper-hotkey starting");
    println!("✓ Telemetry initialized");

    // Request permissions
    permissions::request_all_permissions()?;
    println!("✓ Permissions OK");

    // Main event loop (placeholder for Phase 2+)
    tracing::info!("event loop starting (press Ctrl+C to exit)");
    println!("\nWhisper Hotkey is running. Press Ctrl+C to exit.");

    // Keep running
    tokio::signal::ctrl_c().await?;
    tracing::info!("shutdown signal received");
    println!("\nShutting down...");

    Ok(())
}
