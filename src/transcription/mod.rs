/// Backend trait and error types
pub mod backend;
/// Deepgram cloud transcription backend
pub mod deepgram;
/// Model download and management
pub mod download;
/// Whisper model inference engine
pub mod engine;

pub use backend::TranscriptionBackend;
pub use download::ensure_model_downloaded;
pub use engine::ModelManager;
