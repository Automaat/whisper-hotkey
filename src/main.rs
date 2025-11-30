mod audio;
mod config;
mod input;
mod permissions;
mod telemetry;
mod transcription;

use anyhow::Result;
use global_hotkey::GlobalHotKeyEvent;
use std::sync::{Arc, Mutex};

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

    // Phase 4: Whisper model setup
    let model_path = config::Config::expand_path(&config.model.path)?;
    let downloaded = transcription::ensure_model_downloaded(&config.model.name, &model_path)?;
    if downloaded {
        println!("✓ Model downloaded to {}", model_path.display());
    } else {
        println!("✓ Model found at {}", model_path.display());
    }

    // Preload model if configured
    let transcription_engine = if config.model.preload {
        println!("Loading Whisper model (this may take a few seconds)...");
        let engine = transcription::TranscriptionEngine::new(&model_path)?;
        println!("✓ Whisper model loaded and ready");
        Some(Arc::new(engine))
    } else {
        println!("⚠ Model preload disabled (transcription will be slower)");
        None
    };

    // Phase 3: Audio recording
    let audio_capture = audio::AudioCapture::new(&config.audio)?;
    #[allow(clippy::arc_with_non_send_sync)]
    let audio_capture = Arc::new(Mutex::new(audio_capture));
    println!("✓ Audio capture initialized");

    // Phase 2: Global hotkey (with Phase 5 transcription integration)
    let hotkey_manager = input::hotkey::HotkeyManager::new(
        &config.hotkey,
        Arc::clone(&audio_capture),
        transcription_engine.clone(),
    )?;
    println!(
        "✓ Hotkey registered: {:?} + {}",
        config.hotkey.modifiers, config.hotkey.key
    );

    // Main event loop
    tracing::info!("event loop starting (press Ctrl+C to exit)");
    println!("\nWhisper Hotkey is running. Press the hotkey to record and transcribe.");
    if transcription_engine.is_some() {
        println!("✓ Full pipeline ready: hotkey → audio → transcription → text insertion");
    } else {
        println!("⚠ Transcription disabled (preload = false in config)");
    }
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
