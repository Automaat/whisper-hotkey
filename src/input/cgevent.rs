use core_graphics::event::{CGEvent, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use thiserror::Error;
use tracing::{debug, error, info};

#[derive(Debug, Error)]
pub enum TextInsertionError {
    #[error("failed to create CGEvent source")]
    EventSourceCreation,

    #[error("failed to create keyboard CGEvent")]
    EventCreation,

    #[error("text is empty")]
    EmptyText,
}

/// Inserts text at the current cursor position using CGEvent API
///
/// # Errors
/// Returns error if CGEvent creation fails or text is empty.
/// Logs error to telemetry but does NOT fall back to clipboard.
///
/// # Implementation
/// Uses CGEventKeyboardSetUnicodeString to simulate keyboard input.
/// Requires Accessibility permissions (same as global hotkey).
///
/// # Known Limitations
/// - Some apps may block CGEvent insertion (e.g., Terminal with secure input)
/// - No clipboard fallback (by design - errors are logged)
pub fn insert_text(text: &str) -> Result<(), TextInsertionError> {
    if text.is_empty() {
        return Err(TextInsertionError::EmptyText);
    }

    debug!(text_len = text.len(), "inserting text at cursor");

    // Create event source
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| TextInsertionError::EventSourceCreation)?;

    // Create a keyboard event with dummy keycode (will be overridden by string)
    let event = CGEvent::new_keyboard_event(source, 0, true)
        .map_err(|_| TextInsertionError::EventCreation)?;

    // Set the text to insert
    // Note: set_string_from_utf16_unchecked is not marked unsafe in core-graphics API
    // UTF-16 conversion from Rust &str is always valid (no unpaired surrogates)
    let utf16: Vec<u16> = text.encode_utf16().collect();
    event.set_string_from_utf16_unchecked(&utf16);

    // Post the event to the HID system
    event.post(CGEventTapLocation::HID);

    info!(text_len = text.len(), "text inserted successfully");

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
}
