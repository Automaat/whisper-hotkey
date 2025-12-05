use core_graphics::event::{CGEvent, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use thiserror::Error;
use tracing::{debug, error, info};

/// Generate preview of text for logging (pure, testable)
///
/// Truncates text >50 chars with "..." suffix. Respects UTF-8 char boundaries.
#[must_use]
pub fn generate_text_preview(text: &str) -> String {
    if text.len() > 50 {
        // Find char boundary at or before byte 47
        let mut end = 47.min(text.len());
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        if end == 0 {
            return "...".to_owned();
        }
        format!("{}...", &text[..end])
    } else {
        text.to_owned()
    }
}

/// Text insertion errors
#[derive(Debug, Error)]
pub enum TextInsertionError {
    /// Failed to create `CGEvent` source
    #[error("failed to create CGEvent source")]
    EventSourceCreation,

    /// Failed to create keyboard `CGEvent`
    #[error("failed to create keyboard CGEvent")]
    EventCreation,

    /// Text is empty
    #[error("text is empty")]
    EmptyText,
}

/// Inserts text at the current cursor position using `CGEvent` API
///
/// # Errors
/// Returns error if `CGEvent` creation fails or text is empty.
///
/// # Implementation
/// Uses `CGEventKeyboardSetUnicodeString` to simulate keyboard input.
/// Requires Input Monitoring permission (verified at app startup).
///
/// # Known Limitations
/// - `event.post()` does not return errors - if insertion fails silently,
///   check System Settings → Privacy & Security → Input Monitoring
/// - Some apps block `CGEvent` insertion (e.g., Terminal with secure input)
/// - No clipboard fallback (by design)
///
/// # Permissions
/// Input Monitoring permission is verified at startup via
/// `check_input_monitoring_permission()`. If that check passed, this function
/// should work. If insertions fail at runtime, the user may have revoked permission
/// or the target app has secure input enabled.
pub fn insert_text(text: &str) -> Result<(), TextInsertionError> {
    if text.is_empty() {
        error!("attempted to insert empty text");
        return Err(TextInsertionError::EmptyText);
    }

    let preview = generate_text_preview(text);

    info!(
        text_len = text.len(),
        text_preview = %preview,
        "starting text insertion"
    );

    // Create event source (requires Input Monitoring permission)
    debug!("creating CGEventSource with HIDSystemState");
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|()| {
            error!("FAILED: CGEventSource creation - Input Monitoring permission may have been revoked or blocked");
            error!("Check System Settings → Privacy & Security → Input Monitoring");
            TextInsertionError::EventSourceCreation
        })?;
    debug!("✓ CGEventSource created successfully");

    // Create a keyboard event with dummy keycode (will be overridden by string)
    debug!("creating keyboard CGEvent");
    let event = CGEvent::new_keyboard_event(source, 0, true).map_err(|()| {
        error!("FAILED: CGEvent creation - unexpected error after permission check passed");
        TextInsertionError::EventCreation
    })?;
    debug!("✓ keyboard CGEvent created successfully");

    // Set the text to insert
    // Note: set_string_from_utf16_unchecked is not marked unsafe in the core-graphics crate.
    // SAFETY: The UTF-16 slice passed to set_string_from_utf16_unchecked must be valid UTF-16
    // (no unpaired surrogates). This is guaranteed because Rust's encode_utf16() on &str
    // always produces valid UTF-16.
    debug!("encoding text to UTF-16 for insertion");
    let utf16: Vec<u16> = text.encode_utf16().collect();
    event.set_string_from_utf16_unchecked(&utf16);
    debug!(utf16_len = utf16.len(), "✓ text set on CGEvent");

    // Post the event to the HID system
    // NOTE: post() does not return a result. If this fails (e.g., target app has
    // secure input enabled), the failure is silent. Permission was verified at startup.
    debug!("posting CGEvent to HID system");
    event.post(CGEventTapLocation::HID);

    info!(
        text_len = text.len(),
        text_preview = %generate_text_preview(text),
        "✓ CGEvent posted to HID - text should appear at cursor"
    );
    debug!(
        "If text did NOT appear: target app may have secure input enabled or revoked permission"
    );

    Ok(())
}

/// Attempts to insert text, logging errors without panicking
///
/// This is the primary interface for the hotkey manager.
/// Errors are logged to telemetry but do not crash the app.
pub fn insert_text_safe(text: &str) -> bool {
    match insert_text(text) {
        Ok(()) => true,
        Err(e) => {
            error!(error = %e, text_len = text.len(), "text insertion failed");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_text_preview_short() {
        assert_eq!(generate_text_preview("hello"), "hello");
        assert_eq!(generate_text_preview("12345"), "12345");
    }

    #[test]
    fn test_generate_text_preview_exactly_50_chars() {
        let text_50 = "a".repeat(50);
        assert_eq!(generate_text_preview(&text_50), text_50);
    }

    #[test]
    fn test_generate_text_preview_long() {
        let text_100 = "a".repeat(100);
        let preview = generate_text_preview(&text_100);
        assert!(preview.len() <= 50); // 47 or fewer chars + "..."
        assert!(preview.ends_with("..."));
        assert!(preview.len() >= 3); // At least "..."
        assert!(preview.starts_with(&text_100[..preview.len() - 3]));
    }

    #[test]
    fn test_generate_text_preview_empty() {
        assert_eq!(generate_text_preview(""), "");
    }

    #[test]
    fn test_generate_text_preview_exactly_51_chars() {
        let text_51 = "a".repeat(51);
        let preview = generate_text_preview(&text_51);
        assert!(preview.len() <= 50);
        assert!(preview.ends_with("..."));
    }

    #[test]
    fn test_generate_text_preview_unicode() {
        // Short unicode should not be truncated
        let short_unicode = "Hello 👋";
        assert_eq!(generate_text_preview(short_unicode), short_unicode);

        // Long unicode should be truncated
        let long_unicode = "👋".repeat(30); // Each emoji is 4 bytes
        let preview = generate_text_preview(&long_unicode);
        assert!(preview.ends_with("..."));
        assert!(preview.len() <= 54); // Adjusted char boundary + "..."
        assert!(preview.len() < long_unicode.len()); // Should be shorter than original
    }

    #[test]
    fn test_insert_text_empty() {
        let result = insert_text("");
        assert!(result.is_err());
        assert!(matches!(result, Err(TextInsertionError::EmptyText)));
    }

    #[test]
    fn test_insert_text_safe_empty_returns_false() {
        let result = insert_text_safe("");
        assert!(!result);
    }

    #[test]
    #[ignore = "requires Accessibility permissions and active cursor"]
    fn test_insert_text_simple() {
        // Simple ASCII text
        let result = insert_text("hello");
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires Accessibility permissions and active cursor"]
    fn test_insert_text_unicode() {
        // Unicode text (emojis, non-ASCII)
        let result = insert_text("Hello 👋 Świat 🌍");
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires Accessibility permissions and active cursor"]
    fn test_insert_text_multiline() {
        // Multiline text with newlines
        let result = insert_text("Line 1\nLine 2\nLine 3");
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires Accessibility permissions and active cursor"]
    fn test_insert_text_long() {
        // Long text (>1000 characters)
        let long_text = "a".repeat(1500);
        let result = insert_text(&long_text);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires Accessibility permissions and active cursor"]
    fn test_insert_text_special_chars() {
        // Special characters that might need escaping
        let result = insert_text("Hello \"world\" with 'quotes' and <symbols>");
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires Accessibility permissions and active cursor"]
    fn test_insert_text_safe_simple() {
        let result = insert_text_safe("test");
        assert!(result);
    }

    #[test]
    #[ignore = "requires Accessibility permissions and active cursor"]
    fn test_multiple_insertions() {
        // Verify multiple insertions work
        assert!(insert_text("First").is_ok());
        assert!(insert_text(" Second").is_ok());
        assert!(insert_text(" Third").is_ok());
    }

    #[test]
    #[ignore = "requires Accessibility permissions and active cursor"]
    fn test_insert_text_polish_unicode() {
        // Test Polish characters specifically (from project requirements)
        let result = insert_text("Witaj świecie! Zażółć gęślą jaźń.");
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires Accessibility permissions and active cursor"]
    fn test_insert_text_mixed_languages() {
        // Mixed English/Polish/Emoji
        let result = insert_text("Hello / Cześć 👋 / Hola");
        assert!(result.is_ok());
    }

    #[test]
    fn test_insert_text_safe_with_non_empty_text() {
        // This will fail on systems without accessibility permissions
        // but it tests the code path
        let _ = insert_text_safe("test");
    }

    #[test]
    fn test_utf16_encoding_emojis() {
        // Test that emojis encode correctly to UTF-16
        let text = "Hello 👋 World 🌍";
        let utf16: Vec<u16> = text.encode_utf16().collect();

        // Emojis require 2 UTF-16 code units each (surrogate pairs)
        // "Hello " = 6, "👋" = 2, " World " = 7, "🌍" = 2 = 17 total
        assert_eq!(utf16.len(), 17);

        // UTF-16 length should be more than character count
        // (because 2 emojis are encoded as 4 UTF-16 code units)
        assert!(utf16.len() > text.chars().count());
    }

    #[test]
    fn test_utf16_encoding_polish() {
        // Test Polish characters encode correctly
        let text = "Zażółć gęślą jaźń";

        // Polish characters with diacritics should encode correctly
        // Each character is a single UTF-16 code unit
        assert_eq!(text.encode_utf16().count(), text.chars().count());
    }

    #[test]
    fn test_utf16_encoding_newlines_and_tabs() {
        // Test whitespace characters encode correctly
        let text = "Line1\nLine2\tTabbed";
        let utf16: Vec<u16> = text.encode_utf16().collect();

        // Newlines and tabs should be preserved in UTF-16
        assert_eq!(utf16.len(), text.chars().count());

        // Verify newline and tab are present
        let decoded: String = char::decode_utf16(utf16.iter().copied())
            .map(|r| r.unwrap_or('�'))
            .collect();
        assert!(decoded.contains('\n'));
        assert!(decoded.contains('\t'));
    }
}
