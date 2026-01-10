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

mod alias;
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
use objc2_app_kit::{NSApp, NSApplicationActivationPolicy};
#[cfg(target_os = "macos")]
use objc2_foundation::MainThreadMarker;

#[tokio::main]
async fn main() -> Result<()> {
    // macOS: Initialize NSApplication event loop (required for global-hotkey)
    #[cfg(target_os = "macos")]
    {
        // Safety: main() runs on the main thread
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        let app = NSApp(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
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
    match tokio::task::spawn_blocking({
        let recording_config = config.recording.clone();
        move || recording_cleanup::cleanup_old_recordings(&recording_config)
    })
    .await
    {
        Ok(Ok(deleted)) => {
            if deleted > 0 {
                tracing::debug!("startup cleanup: deleted {} old recordings", deleted);
            }
        }
        Ok(Err(e)) => {
            tracing::warn!("failed to cleanup old recordings: {}", e);
        }
        Err(e) => {
            tracing::warn!("startup cleanup task panicked: {}", e);
        }
    }

    // Request permissions
    permissions::request_all_permissions().context("permission check failed")?;
    println!("✓ Permissions OK");

    // Phase 4: Whisper model setup - Download models for all profiles
    println!(
        "Checking models for {} profile(s)...",
        config.profiles.len()
    );
    for profile in &config.profiles {
        let model_path = config::Config::expand_path(&profile.model_path())
            .context("failed to expand model path")?;
        let downloaded =
            transcription::ensure_model_downloaded(profile.model_type.model_name(), &model_path)
                .with_context(|| {
                    format!(
                        "failed to download/verify model for profile {}",
                        profile.name()
                    )
                })?;
        if downloaded {
            println!(
                "  ✓ {} downloaded to {}",
                profile.name(),
                model_path.display()
            );
            tracing::info!(
                profile = %profile.name(),
                path = %model_path.display(),
                "model downloaded"
            );
        } else {
            println!("  ✓ {} found at {}", profile.name(), model_path.display());
            tracing::info!(
                profile = %profile.name(),
                path = %model_path.display(),
                "model found"
            );
        }
    }
    println!("✓ All models ready");

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
    // Clone necessary: config.aliases needed in Arc, but config borrowed later by tray manager
    let multi_hotkey_manager = input::hotkey::MultiHotkeyManager::new(
        &config.profiles,
        Arc::clone(&audio_capture),
        config.recording.enabled,
        &Arc::new(config.aliases.clone()),
    )
    .context("failed to register global hotkeys")?;
    println!("✓ {} profile(s) registered", config.profiles.len());
    tracing::info!(profiles = config.profiles.len(), "all profiles registered");

    // Menubar tray icon (use first profile's state for icon updates)
    if config.profiles.is_empty() {
        anyhow::bail!("no profiles configured (at least one profile required)");
    }
    let app_state = multi_hotkey_manager
        .profile_state(config.profiles[0].name())
        .context("failed to get state for first profile (profile may be misconfigured)")?;
    let mut tray_manager =
        tray::TrayManager::new(&config, app_state).context("failed to create tray icon")?;
    println!("✓ Menubar icon created");
    tracing::info!("menubar tray icon initialized");

    // Phase 6: Integration & Polish - Main event loop
    tracing::info!("all components initialized successfully");
    tracing::info!("event loop starting (press Ctrl+C to exit)");
    println!(
        "\nWhisper Hotkey is running with {} profile(s). Check menubar for config options.",
        config.profiles.len()
    );
    for profile in &config.profiles {
        println!(
            "  • {}: {:?}+{} {}",
            profile.name(),
            profile.hotkey.modifiers,
            profile.hotkey.key,
            if profile.preload {
                "(preloaded)"
            } else {
                "(lazy load)"
            }
        );
    }
    println!("✓ Full pipeline ready: hotkey → audio → transcription → text insertion");
    println!("Press Ctrl+C to exit or use menubar Quit option.\n");

    let receiver = GlobalHotKeyEvent::receiver();

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
                match tokio::task::spawn_blocking({
                    let recording_config = recording_config.clone();
                    move || recording_cleanup::cleanup_old_recordings(&recording_config)
                })
                .await
                {
                    Ok(Ok(deleted)) => {
                        if deleted > 0 {
                            tracing::debug!("periodic cleanup: deleted {} old recordings", deleted);
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("periodic cleanup failed: {}", e);
                    }
                    Err(e) => {
                        tracing::warn!("cleanup task panicked: {}", e);
                    }
                }
            }
        });
        tracing::debug!(
            "periodic cleanup task spawned (interval: {} hours)",
            config.recording.cleanup_interval_hours
        );
    }

    loop {
        // macOS: Pump the event loop to process global hotkey events
        #[cfg(target_os = "macos")]
        {
            use objc2::rc::autoreleasepool;
            use objc2_app_kit::NSEventMask;
            use objc2_foundation::{NSDate, NSDefaultRunLoopMode};

            autoreleasepool(|_| {
                // Safety: Event loop runs on main thread
                let mtm = unsafe { MainThreadMarker::new_unchecked() };
                let app = NSApp(mtm);
                let distant_past = NSDate::distantPast();

                loop {
                    let event = unsafe {
                        app.nextEventMatchingMask_untilDate_inMode_dequeue(
                            NSEventMask(u64::MAX),
                            Some(&distant_past),
                            NSDefaultRunLoopMode,
                            true,
                        )
                    };
                    if let Some(event) = event {
                        app.sendEvent(&event);
                    } else {
                        break;
                    }
                }
            });
        }

        // Poll for hotkey events
        if let Ok(event) = receiver.try_recv() {
            tracing::debug!("hotkey event received: {:?}", event);
            multi_hotkey_manager.handle_event(event);
        }

        // Update tray menu/icon based on app state
        if let Err(e) = tray_manager.update_icon_if_needed(&config) {
            tracing::warn!(error = %e, "failed to update tray");
        }

        // Poll for tray menu events
        if let Some(tray_cmd) = tray::TrayManager::poll_events() {
            tracing::debug!("tray command received: {:?}", tray_cmd);
            match tray_cmd {
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
