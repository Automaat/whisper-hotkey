use anyhow::{bail, Result};

/// Check and request microphone permission
///
/// # Errors
/// Currently never returns error (permission check deferred to first audio capture)
#[allow(clippy::unnecessary_wraps)] // Consistent API with other permission checks
pub fn check_microphone_permission() -> Result<()> {
    tracing::info!("checking microphone permission");

    // On first run, macOS will automatically prompt for microphone access
    // when we try to use CoreAudio. For now, we'll just log that we need it.
    tracing::warn!("microphone permission will be requested on first audio capture");

    Ok(())
}

/// Check and request accessibility permission (for text insertion)
///
/// Uses the official macOS Accessibility API (`AXIsProcessTrusted`) to check permission.
/// If denied, triggers the system permission dialog automatically.
///
/// # Errors
/// Returns error if accessibility permission is denied (macOS only)
pub fn check_accessibility_permission() -> Result<()> {
    tracing::info!("checking accessibility permission");

    #[cfg(target_os = "macos")]
    {
        use core_foundation::base::TCFType;
        use core_foundation::boolean::CFBoolean;
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::string::CFString;

        // SAFETY: FFI declarations for Accessibility API
        // These are stable macOS APIs available since 10.9
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrusted() -> bool;
            fn AXIsProcessTrustedWithOptions(
                options: core_foundation::dictionary::CFDictionaryRef,
            ) -> bool;
        }

        // First check if we already have permission
        // SAFETY: AXIsProcessTrusted is a safe macOS API that only reads permission status
        let is_trusted = unsafe { AXIsProcessTrusted() };

        if is_trusted {
            tracing::info!("accessibility permission already granted");
            return Ok(());
        }

        tracing::warn!("accessibility permission not granted, requesting...");

        // Create options dictionary to trigger system prompt
        let key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
        let value = CFBoolean::true_value();
        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

        // Request permission with system dialog
        // SAFETY: AXIsProcessTrustedWithOptions is safe, shows system permission dialog
        let is_trusted_after_prompt =
            unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) };

        if !is_trusted_after_prompt {
            bail!(
                "Accessibility permission denied\n\n\
                Enable in: System Settings → Privacy & Security → Accessibility\n\
                Add and enable this app, then restart.\n"
            );
        }

        tracing::info!("accessibility permission granted");
    }

    Ok(())
}

/// Check Input Monitoring permission (for global hotkeys and text insertion)
///
/// # Errors
/// Returns error if Input Monitoring permission is denied (macOS only)
pub fn check_input_monitoring_permission() -> Result<()> {
    tracing::info!("checking input monitoring permission");

    #[cfg(target_os = "macos")]
    {
        use core_graphics::event::CGEvent;
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

        // Try to create a CGEventSource with HIDSystemState - requires Input Monitoring
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).map_err(|()| {
            anyhow::anyhow!(
                "Input Monitoring permission denied\n\n\
                Enable in: System Settings → Privacy & Security → Input Monitoring\n\
                Add and enable this app, then restart.\n"
            )
        })?;

        // Verify we can actually create events (tests full permission chain)
        CGEvent::new_keyboard_event(source, 0, true).map_err(|()| {
            anyhow::anyhow!(
                "Failed to create CGEvent - Input Monitoring may be restricted\n\n\
                Enable in: System Settings → Privacy & Security → Input Monitoring\n"
            )
        })?;

        tracing::info!("input monitoring permission granted");
    }

    Ok(())
}

/// Request all required permissions
///
/// # Errors
/// Returns error if any permission check fails
pub fn request_all_permissions() -> Result<()> {
    tracing::info!("requesting all permissions");

    check_microphone_permission()?;
    check_accessibility_permission()?;
    check_input_monitoring_permission()?;

    tracing::info!("all permissions checked");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_microphone_permission_always_ok() {
        // Microphone permission always returns Ok (deferred to first audio capture)
        let result = check_microphone_permission();
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_microphone_permission_never_fails() {
        // Call multiple times to ensure consistent behavior
        for _ in 0..3 {
            assert!(check_microphone_permission().is_ok());
        }
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_accessibility_permission_on_non_macos() {
        // On non-macOS, should always succeed
        let result = check_accessibility_permission();
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_input_monitoring_permission_on_non_macos() {
        // On non-macOS, should always succeed
        let result = check_input_monitoring_permission();
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_request_all_permissions_on_non_macos() {
        // On non-macOS, all permissions should succeed
        let result = request_all_permissions();
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires Input Monitoring permission on macOS"]
    fn test_check_input_monitoring_permission() {
        let result = check_input_monitoring_permission();
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires accessibility permissions on macOS"]
    fn test_check_accessibility_permission() {
        let result = check_accessibility_permission();
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires permissions on macOS"]
    fn test_request_all_permissions() {
        let result = request_all_permissions();
        assert!(result.is_ok());
    }
}
