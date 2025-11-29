mod config;
mod input;
mod permissions;
mod telemetry;

use anyhow::Result;
use global_hotkey::GlobalHotKeyEvent;

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

    // Phase 2: Global hotkey
    let hotkey_manager = input::hotkey::HotkeyManager::new(&config.hotkey)?;
    println!(
        "✓ Hotkey registered: {:?} + {}",
        config.hotkey.modifiers, config.hotkey.key
    );

    // Main event loop
    tracing::info!("event loop starting (press Ctrl+C to exit)");
    println!("\nWhisper Hotkey is running. Press the hotkey to test state transitions.");
    println!("Press Ctrl+C to exit.\n");

    let receiver = GlobalHotKeyEvent::receiver();
    loop {
        // Poll for hotkey events
        if let Ok(event) = receiver.try_recv() {
            hotkey_manager.handle_event(event);
        }

        // Check for shutdown signal
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("shutdown signal received");
                println!("\nShutting down...");
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(10)) => {
                // Poll interval (10ms to avoid busy-waiting)
            }
        }
    }

    Ok(())
}
