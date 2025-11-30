use anyhow::{bail, Result};

/// Check and request microphone permission
pub fn check_microphone_permission() -> Result<()> {
    tracing::info!("checking microphone permission");

    // On first run, macOS will automatically prompt for microphone access
    // when we try to use CoreAudio. For now, we'll just log that we need it.
    tracing::warn!("microphone permission will be requested on first audio capture");

    Ok(())
}

/// Check and request accessibility permission (for text insertion)
pub fn check_accessibility_permission() -> Result<()> {
    tracing::info!("checking accessibility permission");

    #[cfg(target_os = "macos")]
    {
        // Try to create a CGEventSource to test accessibility access
        let source = core_graphics::event_source::CGEventSource::new(
            core_graphics::event_source::CGEventSourceStateID::CombinedSessionState,
        );

        if source.is_err() {
            bail!("accessibility permission denied - enable in System Settings > Privacy & Security > Accessibility");
        }

        tracing::info!("accessibility permission granted");
    }

    Ok(())
}

/// Check Input Monitoring permission (for global hotkeys)
pub fn check_input_monitoring_permission() -> Result<()> {
    tracing::info!("checking input monitoring permission");

    #[cfg(target_os = "macos")]
    {
        // macOS requires Input Monitoring permission for global hotkeys
        // There's no direct API to check this, so we warn the user
        tracing::warn!("input monitoring permission required for global hotkeys");
        tracing::warn!("if hotkeys don't work, enable in System Settings > Privacy & Security > Input Monitoring");

        println!("⚠️  Input Monitoring permission required:");
        println!("   If hotkeys don't work, go to:");
        println!("   System Settings → Privacy & Security → Input Monitoring");
        println!("   Add and enable your terminal app (Terminal/iTerm2/WezTerm/etc)\n");
    }

    Ok(())
}

/// Request all required permissions
pub fn request_all_permissions() -> Result<()> {
    tracing::info!("requesting all permissions");

    check_microphone_permission()?;
    check_accessibility_permission()?;
    check_input_monitoring_permission()?;

    tracing::info!("all permissions checked");
    Ok(())
}
