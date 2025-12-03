//! Whisper Hotkey - macOS voice-to-text app
//!
//! This library exports core modules for testing and potential future reuse.

/// Audio capture and processing
pub mod audio;
/// Configuration management
pub mod config;
/// Input handling (hotkeys, text insertion)
pub mod input;
/// macOS permission checks
pub mod permissions;
/// Telemetry and crash logging
pub mod telemetry;
/// Whisper transcription engine
pub mod transcription;
