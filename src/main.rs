//! whisper-hotkey - macOS background app for system-wide voice-to-text
//!
//! Provides global hotkey-triggered voice transcription using local Whisper models.
//! Key features:
//! - Global hotkey registration (configurable)
//! - Real-time audio capture via `CoreAudio`
//! - Local Whisper transcription (privacy-focused)
//! - Automatic text insertion via `CGEvent`
//! - Menubar tray icon for configuration

// Allow unsafe code for macOS FFI requirements (NSApp, event loop)
#![allow(unsafe_code)]
// Allow println for user-facing binary output
#![allow(clippy::print_stdout, clippy::uninlined_format_args)]
// Allow items after statements for helper functions in main
#![allow(clippy::items_after_statements)]
// Allow long main function (event loop with config handling)
#![allow(clippy::too_many_lines)]

mod audio;
mod config;
mod input;
mod permissions;
mod recording_cleanup;
mod telemetry;
mod transcription;
mod tray;

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

    // Cleanup old recordings
    match recording_cleanup::cleanup_old_recordings(&config.recording) {
        Ok(deleted) => {
            if deleted > 0 {
                tracing::debug!("startup cleanup: deleted {} old recordings", deleted);
            }
        }
        Err(e) => {
            tracing::warn!("failed to cleanup old recordings: {}", e);
        }
    }

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
        config.recording.enabled,
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

    // Menubar tray icon
    let app_state = hotkey_manager.state_shared();
    let mut tray_manager =
        tray::TrayManager::new(&config, app_state).context("failed to create tray icon")?;
    println!("✓ Menubar icon created");
    tracing::info!("menubar tray icon initialized");

    // Phase 6: Integration & Polish - Main event loop
    tracing::info!("all components initialized successfully");
    tracing::info!("event loop starting (press Ctrl+C to exit)");
    println!("\nWhisper Hotkey is running. Check menubar for config options.");
    if transcription_engine.is_some() {
        println!("✓ Full pipeline ready: hotkey → audio → transcription → text insertion");
        tracing::info!("full pipeline active");
    } else {
        println!("⚠ Transcription disabled (preload = false in config or model load failed)");
        println!("  Audio recording will work, but no transcription will occur");
        tracing::warn!("transcription disabled, running in degraded mode");
    }
    println!("Press Ctrl+C to exit or use menubar Quit option.\n");

    let receiver = GlobalHotKeyEvent::receiver();
    let mut config = config; // Make config mutable for updates

    // Spawn periodic cleanup task if enabled
    if config.recording.cleanup_interval_hours > 0 {
        let recording_config = config.recording.clone();
        tokio::spawn(async move {
            let interval_secs = u64::from(recording_config.cleanup_interval_hours) * 3600;
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
            interval.tick().await; // Skip first immediate tick

            loop {
                interval.tick().await;
                tracing::debug!("running periodic recording cleanup");
                match recording_cleanup::cleanup_old_recordings(&recording_config) {
                    Ok(deleted) => {
                        if deleted > 0 {
                            tracing::debug!("periodic cleanup: deleted {} old recordings", deleted);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("periodic cleanup failed: {}", e);
                    }
                }
            }
        });
        tracing::debug!(
            "periodic cleanup task spawned (interval: {} hours)",
            config.recording.cleanup_interval_hours
        );
    }

    // Helper to save config, log, and update menu after config changes
    fn save_and_update(
        config: &config::Config,
        tray_manager: &tray::TrayManager,
        success_msg: &str,
        requires_restart: bool,
    ) -> Result<()> {
        config.save().context("failed to save config")?;
        let msg = if requires_restart {
            format!("{} (restart required)", success_msg)
        } else {
            success_msg.to_owned()
        };
        println!("✓ {}", msg);
        tracing::info!("{}", success_msg.to_lowercase());
        tray_manager.update_menu(config)?;
        Ok(())
    }

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

        // Update tray menu/icon based on app state
        if let Err(e) = tray_manager.update_icon_if_needed(&config) {
            tracing::warn!(error = %e, "failed to update tray");
        }

        // Poll for tray menu events
        if let Some(tray_cmd) = tray::TrayManager::poll_events() {
            tracing::debug!("tray command received: {:?}", tray_cmd);
            match tray_cmd {
                tray::TrayCommand::UpdateHotkey { modifiers, key } => {
                    config.hotkey.modifiers = modifiers;
                    config.hotkey.key = key;
                    if let Err(e) = save_and_update(&config, &tray_manager, "Hotkey updated", true)
                    {
                        tracing::error!("failed to update config: {:?}", e);
                        println!("⚠ Failed to save config: {}", e);
                    }
                }
                tray::TrayCommand::UpdateModel { name } => {
                    config.model.name = name;
                    if let Err(e) = save_and_update(&config, &tray_manager, "Model updated", true) {
                        tracing::error!("failed to update config: {:?}", e);
                        println!("⚠ Failed to save config: {}", e);
                    }
                }
                tray::TrayCommand::UpdateThreads(threads) => {
                    config.model.threads = threads;
                    if let Err(e) = save_and_update(
                        &config,
                        &tray_manager,
                        &format!("Threads updated to {}", threads),
                        false,
                    ) {
                        tracing::error!("failed to update config: {:?}", e);
                        println!("⚠ Failed to save config: {}", e);
                    }
                }
                tray::TrayCommand::UpdateBeamSize(beam) => {
                    config.model.beam_size = beam;
                    if let Err(e) = save_and_update(
                        &config,
                        &tray_manager,
                        &format!("Beam size updated to {}", beam),
                        false,
                    ) {
                        tracing::error!("failed to update config: {:?}", e);
                        println!("⚠ Failed to save config: {}", e);
                    }
                }
                tray::TrayCommand::UpdateLanguage(lang) => {
                    let msg = lang.as_ref().map_or_else(
                        || "Language set to auto-detect".to_owned(),
                        |l| format!("Language updated to {l}"),
                    );
                    config.model.language = lang;
                    if let Err(e) = save_and_update(&config, &tray_manager, &msg, false) {
                        tracing::error!("failed to update config: {:?}", e);
                        println!("⚠ Failed to save config: {}", e);
                    }
                }
                tray::TrayCommand::UpdateBufferSize(size) => {
                    config.audio.buffer_size = size;
                    if let Err(e) = save_and_update(
                        &config,
                        &tray_manager,
                        &format!("Buffer size updated to {}", size),
                        true,
                    ) {
                        tracing::error!("failed to update config: {:?}", e);
                        println!("⚠ Failed to save config: {}", e);
                    }
                }
                tray::TrayCommand::TogglePreload => {
                    config.model.preload = !config.model.preload;
                    let msg = format!(
                        "Preload {}",
                        if config.model.preload {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    );
                    if let Err(e) = save_and_update(&config, &tray_manager, &msg, true) {
                        tracing::error!("failed to update config: {:?}", e);
                        println!("⚠ Failed to save config: {}", e);
                    }
                }
                tray::TrayCommand::ToggleTelemetry => {
                    config.telemetry.enabled = !config.telemetry.enabled;
                    let msg = format!(
                        "Telemetry {}",
                        if config.telemetry.enabled {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    );
                    if let Err(e) = save_and_update(&config, &tray_manager, &msg, true) {
                        tracing::error!("failed to update config: {:?}", e);
                        println!("⚠ Failed to save config: {}", e);
                    }
                }
                tray::TrayCommand::OpenConfigFile => {
                    if let Ok(path) = config::Config::get_config_path() {
                        #[cfg(target_os = "macos")]
                        {
                            let _ = std::process::Command::new("open").arg(&path).spawn();
                            tracing::info!("opened config file: {:?}", path);
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            println!("Config file location: {:?}", path);
                            tracing::info!("config file location: {:?}", path);
                        }
                    }
                } // Note: Quit case removed - PredefinedMenuItem::quit() calls native
                  // macOS terminate: selector which bypasses event system entirely
            }
        }

        // Check for shutdown signal
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("shutdown signal received");
                println!("\nShutting down...");
                break;
            }
            () = tokio::time::sleep(tokio::time::Duration::from_millis(10)) => {
                // Poll interval (10ms to avoid busy-waiting)
            }
        }
    }

    tracing::info!("whisper-hotkey shutdown complete");
    Ok(())
}
