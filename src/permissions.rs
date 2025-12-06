use anyhow::{bail, Result};

/// Check if the app bundle has macOS quarantine attribute
///
/// Returns detailed instructions for removing quarantine if detected.
/// This is a common issue when apps are downloaded from the internet.
fn check_quarantine_status() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // Get the path to the current executable
        let exe_path = std::env::current_exe().ok();

        if let Some(exe) = exe_path {
            // Check if we're running from a .app bundle
            let exe_str = exe.to_string_lossy();
            if exe_str.contains(".app/Contents/MacOS/") {
                // Extract the .app bundle path
                if let Some(app_idx) = exe_str.find(".app/") {
                    let app_path = &exe_str[..app_idx + 4]; // Include ".app"

                    // Check for quarantine attribute using xattr
                    let output = Command::new("xattr").arg("-l").arg(app_path).output();

                    if let Ok(output) = output {
                        let stdout = String::from_utf8_lossy(&output.stdout);

                        if stdout.contains("com.apple.quarantine") {
                            let message = format!(
                                "\n\n⚠️  QUARANTINE DETECTED\n\n\
                                Your app has the macOS quarantine attribute (common for downloaded apps).\n\
                                This prevents macOS from recognizing granted permissions.\n\n\
                                To fix, run this command in Terminal:\n\n\
                                    xattr -d com.apple.quarantine {app_path}\n\n\
                                Then restart the app.\n"
                            );
                            return Some(message);
                        }
                    }
                }
            }
        }
    }

    None
}

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
/// If denied, shows system dialog directing user to System Settings. App must be relaunched
/// after granting permission.
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
        #[allow(unsafe_code)]
        let is_trusted = unsafe { AXIsProcessTrusted() };

        if is_trusted {
            tracing::info!("accessibility permission already granted");
            return Ok(());
        }

        tracing::warn!("accessibility permission not granted, showing system dialog...");

        // Create options dictionary to trigger system prompt
        let key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
        let value = CFBoolean::true_value();
        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

        // Show system dialog directing user to System Settings
        // SAFETY: AXIsProcessTrustedWithOptions is safe, shows system permission dialog
        // Note: This returns immediately with current status (false), before user can grant permission
        #[allow(unsafe_code)]
        let _ = unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) };

        // Check for quarantine attribute that might be blocking permissions
        let quarantine_msg = check_quarantine_status().unwrap_or_default();

        // Always exit with instructions after showing dialog
        // User must grant permission in System Settings and relaunch the app
        bail!(
            "Accessibility permission required\n\n\
            A system dialog has been shown. Please:\n\
            1. Open System Settings → Privacy & Security → Accessibility\n\
            2. Enable this app\n\
            3. Restart the app{quarantine_msg}\n"
        );
    }

    #[cfg(not(target_os = "macos"))]
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

        // Check for quarantine attribute that might be blocking permissions
        let quarantine_msg = check_quarantine_status().unwrap_or_default();

        // Try to create a CGEventSource with HIDSystemState - requires Input Monitoring
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).map_err(|()| {
            anyhow::anyhow!(
                "Input Monitoring permission denied\n\n\
                Enable in: System Settings → Privacy & Security → Input Monitoring\n\
                Add and enable this app, then restart.{quarantine_msg}\n"
            )
        })?;

        // Verify we can actually create events (tests full permission chain)
        CGEvent::new_keyboard_event(source, 0, true).map_err(|()| {
            anyhow::anyhow!(
                "Failed to create CGEvent - Input Monitoring may be restricted\n\n\
                Enable in: System Settings → Privacy & Security → Input Monitoring{quarantine_msg}\n"
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
    #[cfg(not(target_os = "macos"))]
    fn test_check_accessibility_permission_non_macos() {
        // On non-macOS platforms, function should always succeed
        let result = check_accessibility_permission();
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_accessibility_permission_function_signature() {
        // Compile-time check that function has correct signature
        // Actual permission testing requires manual testing with #[ignore] test
        let _: fn() -> Result<()> = check_accessibility_permission;
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_accessibility_permission_denied_error_message() {
        // Test that function returns appropriate error when permission denied
        // This test is expected to fail in CI where permissions aren't granted
        let result = check_accessibility_permission();

        // If permission is denied, verify error message contains expected guidance
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(error_msg.contains("Accessibility permission required"));
            assert!(error_msg.contains("System Settings"));
        }
        // If permission is granted (e.g., on developer machine), that's also fine
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_input_monitoring_permission_denied_error_message() {
        // Test that function returns appropriate error when permission denied
        let result = check_input_monitoring_permission();

        // If permission is denied, verify error message contains expected guidance
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(error_msg.contains("Input Monitoring") || error_msg.contains("CGEvent"));
            assert!(error_msg.contains("System Settings"));
        }
        // If permission is granted (e.g., on developer machine), that's also fine
    }

    #[test]
    fn test_microphone_permission_always_succeeds() {
        // Microphone permission is deferred to first audio capture
        // This function should always succeed
        let result = check_microphone_permission();
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires permissions on macOS"]
    fn test_request_all_permissions() {
        let result = request_all_permissions();
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_request_all_permissions_fails_without_permissions() {
        // In CI without permissions, request_all_permissions should fail
        // at accessibility check (first permission that requires prompt)
        let result = request_all_permissions();

        // Will fail on accessibility check in CI
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(error_msg.contains("Accessibility permission required"));
        }
        // If all permissions granted (dev machine), that's also fine
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_accessibility_permission_error_includes_instructions() {
        let result = check_accessibility_permission();

        if let Err(e) = result {
            let error_msg = e.to_string();
            // Verify comprehensive error message
            assert!(error_msg.contains("Privacy & Security"));
            assert!(error_msg.contains("Accessibility"));
            assert!(error_msg.contains("Restart the app"));
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_input_monitoring_with_event_source_creation() {
        // Test both CGEventSource and CGEvent creation paths
        let result = check_input_monitoring_permission();

        if let Err(e) = result {
            let error_msg = e.to_string();
            // Should mention either Input Monitoring or CGEvent
            assert!(
                error_msg.contains("Input Monitoring") || error_msg.contains("CGEvent"),
                "Error message should mention permission issue: {error_msg}"
            );
        }
    }
}
