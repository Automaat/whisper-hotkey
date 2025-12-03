/// Model download and management
pub mod download;
/// Whisper model inference engine
pub mod engine;

pub use download::ensure_model_downloaded;
pub use engine::TranscriptionEngine;
