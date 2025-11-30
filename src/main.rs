mod audio;
mod config;
mod input;
mod permissions;
mod telemetry;
mod transcription;

use anyhow::{Context, Result};
use global_hotkey::GlobalHotKeyEvent;
use std::sync::{Arc, Mutex};

#[cfg(target_os = "macos")]
use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicyAccessory};
#[cfg(target_os = "macos")]
use cocoa::base::nil;

#[tokio::main]
async fn main() -> Result<()> {
    // macOS: Initialize NSApplication event loop (required for global-hotkey)
    #[cfg(target_os = "macos")]
    unsafe {
        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);
    }
    // Phase 1: Foundation
    // Load configuration
    let config = config::Config::load().context("failed to load configuration")?;
    println!("✓ Config loaded from ~/.whisper-hotkey.toml");

    // Initialize telemetry
    telemetry::init(config.telemetry.enabled, &config.telemetry.log_path)
        .context("failed to initialize telemetry")?;
    tracing::info!("whisper-hotkey starting");
    println!("✓ Telemetry initialized");

    // Request permissions
    permissions::request_all_permissions().context("permission check failed")?;
    println!("✓ Permissions OK");

    // Phase 4: Whisper model setup
    let model_path =
        config::Config::expand_path(&config.model.path).context("failed to expand model path")?;
    let downloaded = transcription::ensure_model_downloaded(&config.model.name, &model_path)
        .context("failed to download/verify Whisper model")?;
    if downloaded {
        println!("✓ Model downloaded to {}", model_path.display());
        tracing::info!("whisper model downloaded: {}", model_path.display());
    } else {
        println!("✓ Model found at {}", model_path.display());
        tracing::info!("whisper model found: {}", model_path.display());
    }

    // Preload model if configured
    let transcription_engine = if config.model.preload {
        println!("Loading Whisper model (this may take a few seconds)...");
        println!(
            "  Optimization: {} threads, beam_size={}",
            config.model.threads, config.model.beam_size
        );
        if let Some(ref lang) = config.model.language {
            println!("  Language: {} (hint)", lang);
        } else {
            println!("  Language: auto-detect");
        }
        match transcription::TranscriptionEngine::new(
            &model_path,
            config.model.threads,
            config.model.beam_size,
            config.model.language.clone(),
        ) {
            Ok(engine) => {
                println!("✓ Whisper model loaded and ready");
                tracing::info!("whisper model preloaded successfully");
                Some(Arc::new(engine))
            }
            Err(e) => {
                tracing::error!("failed to preload whisper model: {:?}", e);
                println!("⚠ Model preload failed: {}", e);
                println!("  Continuing without transcription (hotkey will still work)");
                None
            }
        }
    } else {
        println!("⚠ Model preload disabled (transcription will be slower)");
        tracing::info!("model preload disabled in config");
        None
    };

    // Phase 3: Audio recording
    let audio_capture =
        audio::AudioCapture::new(&config.audio).context("failed to initialize audio capture")?;
    #[allow(clippy::arc_with_non_send_sync)]
    let audio_capture = Arc::new(Mutex::new(audio_capture));
    println!("✓ Audio capture initialized");
    tracing::info!(
        "audio capture initialized: buffer_size={}, sample_rate={}",
        config.audio.buffer_size,
        config.audio.sample_rate
    );

    // Phase 2: Global hotkey (with Phase 5 transcription integration)
    let hotkey_manager = input::hotkey::HotkeyManager::new(
        &config.hotkey,
        Arc::clone(&audio_capture),
        transcription_engine.clone(),
    )
    .context("failed to register global hotkey")?;
    println!(
        "✓ Hotkey registered: {:?} + {}",
        config.hotkey.modifiers, config.hotkey.key
    );
    tracing::info!(
        "hotkey registered: {:?} + {}",
        config.hotkey.modifiers,
        config.hotkey.key
    );

    // Phase 6: Integration & Polish - Main event loop
    tracing::info!("all components initialized successfully");
    tracing::info!("event loop starting (press Ctrl+C to exit)");
    println!("\nWhisper Hotkey is running. Press the hotkey to record and transcribe.");
    if transcription_engine.is_some() {
        println!("✓ Full pipeline ready: hotkey → audio → transcription → text insertion");
        tracing::info!("full pipeline active");
    } else {
        println!("⚠ Transcription disabled (preload = false in config or model load failed)");
        println!("  Audio recording will work, but no transcription will occur");
        tracing::warn!("transcription disabled, running in degraded mode");
    }
    println!("Press Ctrl+C to exit.\n");

    let receiver = GlobalHotKeyEvent::receiver();
    loop {
        // macOS: Pump the event loop to process global hotkey events
        #[cfg(target_os = "macos")]
        unsafe {
            use cocoa::foundation::{NSAutoreleasePool, NSDate};

            let pool = NSAutoreleasePool::new(nil);
            let app = NSApp();
            let distant_past = NSDate::distantPast(nil);

            loop {
                let event = app.nextEventMatchingMask_untilDate_inMode_dequeue_(
                    u64::MAX,
                    distant_past,
                    cocoa::foundation::NSDefaultRunLoopMode,
                    true,
                );
                if event == nil {
                    break;
                }
                app.sendEvent_(event);
            }

            pool.drain();
        }

        // Poll for hotkey events
        if let Ok(event) = receiver.try_recv() {
            tracing::debug!("hotkey event received: {:?}", event);
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

    tracing::info!("whisper-hotkey shutdown complete");
    Ok(())
}
