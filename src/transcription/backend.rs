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

#[cfg(test)]
mod tests {
    use super::*;

    // Test error Display implementations
    #[test]
    fn test_error_display_model_load() {
        let err = TranscriptionError::ModelLoad {
            path: "/path/to/model.bin".to_owned(),
            source: anyhow::anyhow!("file not found"),
        };
        let msg = format!("{err}");
        assert!(msg.contains("/path/to/model.bin"));
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn test_error_display_state_creation() {
        let err = TranscriptionError::StateCreation;
        assert_eq!(format!("{err}"), "failed to create whisper state");
    }

    #[test]
    fn test_error_display_transcription() {
        let err = TranscriptionError::Transcription(anyhow::anyhow!("inference failed"));
        assert!(format!("{err}").contains("inference failed"));
    }

    #[test]
    fn test_error_display_deepgram_api() {
        let err = TranscriptionError::DeepgramApi("connection timeout".to_owned());
        assert!(format!("{err}").contains("connection timeout"));
    }

    #[test]
    fn test_error_display_deepgram_config_missing() {
        let err = TranscriptionError::DeepgramConfigMissing;
        assert!(format!("{err}").contains("API key"));
    }

    #[test]
    fn test_error_display_streaming_not_supported() {
        let err = TranscriptionError::StreamingNotSupported;
        assert_eq!(format!("{err}"), "streaming not supported by this backend");
    }

    // Mock backend for testing default trait implementations
    struct MockBackend;

    impl TranscriptionBackend for MockBackend {
        fn transcribe(&self, _audio_data: &[f32]) -> Result<String, TranscriptionError> {
            Ok("mock transcription".to_owned())
        }

        fn backend_name(&self) -> &'static str {
            "mock"
        }
    }

    #[test]
    fn test_default_supports_streaming() {
        let backend = MockBackend;
        assert!(!backend.supports_streaming());
    }

    #[test]
    fn test_default_start_stream() {
        let backend = MockBackend;
        let result = backend.start_stream();
        assert!(matches!(
            result,
            Err(TranscriptionError::StreamingNotSupported)
        ));
    }

    #[test]
    fn test_default_send_audio_chunk() {
        let backend = MockBackend;
        let result = backend.send_audio_chunk(&[0.0; 100]);
        assert!(matches!(
            result,
            Err(TranscriptionError::StreamingNotSupported)
        ));
    }

    #[test]
    fn test_default_finish_stream() {
        let backend = MockBackend;
        let result = backend.finish_stream();
        assert!(matches!(
            result,
            Err(TranscriptionError::StreamingNotSupported)
        ));
    }

    #[test]
    fn test_mock_backend_transcribe() {
        let backend = MockBackend;
        let result = backend.transcribe(&[0.0; 100]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "mock transcription");
    }

    #[test]
    fn test_mock_backend_name() {
        let backend = MockBackend;
        assert_eq!(backend.backend_name(), "mock");
    }
}
