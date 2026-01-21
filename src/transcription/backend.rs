use thiserror::Error;

/// Errors that can occur during transcription
#[derive(Debug, Error)]
pub enum TranscriptionError {
    /// Failed to load Whisper model
    #[error("failed to load whisper model from {path}: {source}")]
    ModelLoad {
        /// Path to model file
        path: String,
        /// Underlying error
        source: anyhow::Error,
    },

    /// Failed to create Whisper inference state
    #[error("failed to create whisper state")]
    StateCreation,

    /// Transcription inference failed
    #[error("failed to transcribe audio: {0}")]
    Transcription(#[from] anyhow::Error),

    /// Deepgram API error
    #[error("deepgram API error: {0}")]
    DeepgramApi(String),

    /// Deepgram configuration missing
    #[error("deepgram backend requires API key configuration")]
    DeepgramConfigMissing,

    /// Backend does not support streaming transcription
    #[error("streaming not supported by this backend")]
    StreamingNotSupported,
}

/// Trait for transcription backends (local Whisper or cloud Deepgram)
pub trait TranscriptionBackend: Send + Sync {
    /// Transcribe audio samples (16kHz mono f32) to text
    ///
    /// # Errors
    /// Returns error if transcription fails
    fn transcribe(&self, audio_data: &[f32]) -> Result<String, TranscriptionError>;

    /// Get backend name for telemetry
    #[allow(dead_code)] // TODO: Use for telemetry/logging
    fn backend_name(&self) -> &'static str;

    /// Returns true if this backend supports streaming transcription
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Start a streaming transcription session
    /// Call this when recording starts to open connection early
    ///
    /// # Errors
    /// Returns error if stream cannot be started or backend doesn't support streaming
    fn start_stream(&self) -> Result<(), TranscriptionError> {
        Err(TranscriptionError::StreamingNotSupported)
    }

    /// Send audio chunk to active stream (16kHz mono f32)
    /// Call periodically during recording
    ///
    /// # Errors
    /// Returns error if send fails, no active stream, or backend doesn't support streaming
    fn send_audio_chunk(&self, _audio_data: &[f32]) -> Result<(), TranscriptionError> {
        Err(TranscriptionError::StreamingNotSupported)
    }

    /// Finish streaming and get final transcript
    /// Call when recording stops
    ///
    /// # Errors
    /// Returns error if finishing fails, no active stream, or backend doesn't support streaming
    fn finish_stream(&self) -> Result<String, TranscriptionError> {
        Err(TranscriptionError::StreamingNotSupported)
    }
}
