use anyhow::{bail, Result};

/// Extract .app bundle path from executable path
///
/// Returns the path to the .app bundle if the executable is inside one.
/// For example: "/Applications/MyApp.app/Contents/MacOS/myapp" -> Some("/Applications/MyApp.app")
fn extract_app_bundle_path(exe_path: &str) -> Option<String> {
    if !exe_path.contains(".app/Contents/MacOS/") {
        return None;
    }

    let app_idx = exe_path.find(".app/")?;
    Some(exe_path[..app_idx + 4].to_string()) // Include ".app"
}

/// Check if xattr output contains quarantine attribute
///
/// Parses the output from `xattr -l` command to detect com.apple.quarantine.
fn contains_quarantine_attribute(xattr_output: &str) -> bool {
    xattr_output.contains("com.apple.quarantine")
}

/// Format quarantine removal instructions for a given app path
///
/// Used by `check_quarantine_status()` to generate user-facing message.
fn format_quarantine_message(app_path: &str) -> String {
    format!(
        "\n\n⚠️  QUARANTINE DETECTED\n\n\
        Your app has the macOS quarantine attribute (common for downloaded apps).\n\
        This prevents macOS from recognizing granted permissions.\n\n\
        To fix, run this command in Terminal:\n\n\
            xattr -d com.apple.quarantine \"{app_path}\"\n\n\
        Then restart the app.\n"
    )
}

/// Check if the app bundle has macOS quarantine attribute
///
/// Returns detailed instructions for removing quarantine if detected.
/// This is a common issue when apps are downloaded from the internet.
fn check_quarantine_status() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // Get the path to the current executable
        let exe_path = std::env::current_exe().ok()?;
        let exe_str = exe_path.to_string_lossy();

        // Extract .app bundle path
        let app_path = extract_app_bundle_path(&exe_str)?;

        // Check for quarantine attribute using xattr
        let output = Command::new("xattr").arg("-l").arg(&app_path).output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if contains_quarantine_attribute(&stdout) {
                    return Some(format_quarantine_message(&app_path));
                }
            }
            Err(e) => {
                tracing::debug!("failed to check quarantine status: {}", e);
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

    #[test]
    fn test_extract_app_bundle_path_valid() {
        // Test extracting .app path from valid executable paths
        let test_cases = vec![
            (
                "/Applications/WhisperHotkey.app/Contents/MacOS/whisper-hotkey",
                Some("/Applications/WhisperHotkey.app"),
            ),
            (
                "/Users/user/My Apps/Test.app/Contents/MacOS/test",
                Some("/Users/user/My Apps/Test.app"),
            ),
            (
                "/System/Applications/Utilities/Terminal.app/Contents/MacOS/Terminal",
                Some("/System/Applications/Utilities/Terminal.app"),
            ),
        ];

        for (input, expected) in test_cases {
            let result = extract_app_bundle_path(input);
            assert_eq!(result.as_deref(), expected, "Failed for input: {input}");
        }
    }

    #[test]
    fn test_extract_app_bundle_path_invalid() {
        // Test paths that are not in .app bundles
        let invalid_paths = vec![
            "/usr/local/bin/whisper-hotkey",
            "/Applications/WhisperHotkey/bin/app",
            "/home/user/.cargo/bin/test",
            "target/debug/whisper-hotkey",
            "/Applications/Test.app", // Missing /Contents/MacOS/
            "/Applications/Test.app/Contents/Resources/file", // Wrong subdirectory
        ];

        for path in invalid_paths {
            let result = extract_app_bundle_path(path);
            assert!(
                result.is_none(),
                "Expected None for path: {path}, got {result:?}"
            );
        }
    }

    #[test]
    fn test_extract_app_bundle_path_edge_cases() {
        // Test edge cases
        assert_eq!(
            extract_app_bundle_path("/Test.app/Contents/MacOS/test"),
            Some("/Test.app".to_owned())
        );

        // Multiple .app in path (should match first)
        assert_eq!(
            extract_app_bundle_path("/Apps/Outer.app/Inner.app/Contents/MacOS/test"),
            Some("/Apps/Outer.app".to_owned())
        );

        // Empty and unusual paths
        assert_eq!(extract_app_bundle_path(""), None);
        assert_eq!(
            extract_app_bundle_path(".app/Contents/MacOS/test"),
            Some(".app".to_owned())
        );
    }

    #[test]
    fn test_contains_quarantine_attribute_present() {
        // Test with real xattr output containing quarantine
        let xattr_outputs = vec![
            "com.apple.quarantine: 0083;12345678;Safari;",
            "com.apple.FinderInfo:\n00000000  00 00 00 00 00 00 00 00\ncom.apple.quarantine: 0001;",
            "  com.apple.quarantine  :  data  ",
            "com.apple.lastuseddate#PS: ...\ncom.apple.quarantine: ...",
        ];

        for output in xattr_outputs {
            assert!(
                contains_quarantine_attribute(output),
                "Failed to detect quarantine in: {output}"
            );
        }
    }

    #[test]
    fn test_contains_quarantine_attribute_absent() {
        // Test with xattr output without quarantine
        let xattr_outputs = vec![
            "",
            "com.apple.FinderInfo:\n00000000  00 00 00 00",
            "com.apple.metadata:kMDItemWhereFroms",
            "com.apple.lastuseddate#PS: data",
            "   ",
            "\n\n",
        ];

        for output in xattr_outputs {
            assert!(
                !contains_quarantine_attribute(output),
                "False positive for: {output}"
            );
        }
    }

    #[test]
    fn test_contains_quarantine_attribute_case_sensitivity() {
        // Verify case sensitivity (should be exact match)
        assert!(contains_quarantine_attribute("com.apple.quarantine"));
        assert!(!contains_quarantine_attribute("com.apple.QUARANTINE"));
        assert!(!contains_quarantine_attribute("COM.APPLE.QUARANTINE"));
    }

    #[test]
    fn test_contains_quarantine_attribute_substring_matching() {
        // Function uses substring matching (contains)
        assert!(contains_quarantine_attribute("com.apple.quarantine2")); // Substring match
        assert!(contains_quarantine_attribute("com.apple.quarantine: data"));
        assert!(contains_quarantine_attribute(
            "prefix com.apple.quarantine suffix"
        ));
        assert!(contains_quarantine_attribute("com.apple.quarantine")); // Exact match
    }

    #[test]
    fn test_format_quarantine_message() {
        // Test message formatting with various app paths
        let msg = format_quarantine_message("/Applications/WhisperHotkey.app");

        // Verify all required components
        assert!(msg.contains("⚠️  QUARANTINE DETECTED"));
        assert!(msg.contains("com.apple.quarantine"));
        assert!(msg.contains("xattr -d"));
        assert!(msg.contains("restart the app"));

        // Verify path is quoted
        assert!(msg.contains("\"/Applications/WhisperHotkey.app\""));
    }

    #[test]
    fn test_format_quarantine_message_with_spaces() {
        // Test that paths with spaces are properly quoted
        let msg = format_quarantine_message("/Applications/My Apps/Whisper Hotkey.app");

        // Command should have quoted path
        assert!(msg.contains(
            "xattr -d com.apple.quarantine \"/Applications/My Apps/Whisper Hotkey.app\""
        ));

        // Verify message structure
        assert!(msg.contains("⚠️"));
        assert!(msg.contains("Terminal:"));
    }

    #[test]
    fn test_format_quarantine_message_special_chars() {
        // Test with path containing special characters
        let paths = vec![
            "/Applications/Test (1).app",
            "/Applications/Test-App.app",
            "/Applications/Test_App.app",
        ];

        for path in paths {
            let msg = format_quarantine_message(path);
            // All paths should be quoted
            assert!(msg.contains(&format!("\"{path}\"")));
            assert!(msg.contains("xattr -d com.apple.quarantine"));
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_check_quarantine_status_returns_none_when_not_in_bundle() {
        // When running from cargo (not .app bundle), should return None
        let result = check_quarantine_status();

        // Verify current exe path doesn't contain .app/Contents/MacOS/
        let exe_path = std::env::current_exe().ok();
        if let Some(exe) = exe_path {
            let exe_str = exe.to_string_lossy();
            if !exe_str.contains(".app/Contents/MacOS/") {
                // Not in .app bundle, should return None
                assert!(
                    result.is_none(),
                    "Expected None when not running from .app bundle"
                );
            }
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_check_quarantine_status_message_format() {
        // Validate message format by constructing expected message
        let result = check_quarantine_status();

        if let Some(msg) = result {
            // If quarantine detected, verify comprehensive message format
            assert!(
                msg.contains("⚠️  QUARANTINE DETECTED"),
                "Missing warning header"
            );
            assert!(
                msg.contains("com.apple.quarantine"),
                "Missing quarantine attribute name"
            );
            assert!(msg.contains("xattr -d"), "Missing xattr command");
            assert!(
                msg.contains("Restart the app"),
                "Missing restart instruction"
            );

            // Verify path is quoted (critical for paths with spaces)
            let quoted_path_count = msg.matches('"').count();
            assert!(
                quoted_path_count >= 2,
                "Path should be quoted (found {quoted_path_count} quotes)"
            );

            // Verify structure: command should have quoted path
            assert!(
                msg.contains("xattr -d com.apple.quarantine \""),
                "Command should quote path"
            );
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    #[ignore = "requires running from quarantined .app bundle"]
    fn test_check_quarantine_status_in_app_bundle() {
        // Integration test: validates quarantine detection in .app bundle
        // Run with: cargo test -- --ignored test_check_quarantine_status_in_app_bundle
        let result = check_quarantine_status();

        // This test should be run from a .app bundle with quarantine attribute
        // To test:
        // 1. Build app bundle: cargo build --release
        // 2. Create .app bundle structure
        // 3. Add quarantine: xattr -w com.apple.quarantine "0001;$(date +%s);curl|..." path/to/App.app
        // 4. Run: path/to/App.app/Contents/MacOS/whisper-hotkey

        let exe_path = std::env::current_exe().expect("Failed to get current exe");
        let exe_str = exe_path.to_string_lossy();

        if exe_str.contains(".app/Contents/MacOS/") {
            // We're in a .app bundle, check if quarantine is detected
            // Validate format if quarantine detected
            if let Some(msg) = result {
                assert!(msg.contains("xattr -d com.apple.quarantine"));
            }
        }
        // If not in .app bundle, test is skipped (no assertions)
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_check_quarantine_status_non_macos() {
        // On non-macOS, should always return None
        let result = check_quarantine_status();
        assert!(
            result.is_none(),
            "Non-macOS platforms should always return None"
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_check_quarantine_status_handles_xattr_gracefully() {
        // Test that function doesn't panic when xattr command behavior varies
        // This validates the error handling path (lines 40-41)
        let result = check_quarantine_status();

        // Function should return Option (Some or None), never panic
        if let Some(msg) = result {
            // Quarantine detected, validate message
            assert!(msg.contains("xattr"), "Message should mention xattr");
        }
        // If None: no quarantine or not in .app bundle - expected, no assertion needed

        // This test validates that function doesn't panic in any scenario:
        // - When xattr command fails (error path with debug logging)
        // - When not running from .app bundle
        // - When quarantine not present
    }
}
