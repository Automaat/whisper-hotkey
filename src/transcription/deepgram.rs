use std::io::Cursor;
use std::sync::Arc;
use tokio::runtime::Runtime;

use super::backend::{TranscriptionBackend, TranscriptionError};

/// Deepgram cloud transcription backend
#[allow(dead_code)] // TODO: Remove when API implementation is complete
pub struct DeepgramBackend {
    /// Deepgram client (shared across requests)
    client: Arc<deepgram::Deepgram>,
    /// Model name (e.g., "whisper-large", "nova-3")
    model: String,
    /// Language code (None = auto-detect)
    language: Option<String>,
    /// Enable smart formatting (punctuation, capitalization)
    smart_format: bool,
    /// Tokio runtime for async operations
    runtime: Arc<Runtime>,
}

impl DeepgramBackend {
    /// Creates new Deepgram backend
    ///
    /// # Errors
    /// Returns error if client creation fails
    pub fn new(
        api_key: &str,
        model: String,
        language: Option<String>,
        smart_format: bool,
        runtime: Arc<Runtime>,
    ) -> Result<Self, TranscriptionError> {
        let client = deepgram::Deepgram::new(api_key)
            .map_err(|e| TranscriptionError::DeepgramApi(e.to_string()))?;

        Ok(Self {
            client: Arc::new(client),
            model,
            language,
            smart_format,
            runtime,
        })
    }

    /// Converts PCM f32 samples to WAV bytes
    ///
    /// # Errors
    /// Returns error if WAV encoding fails
    fn convert_pcm_to_wav(audio_data: &[f32]) -> Result<Vec<u8>, TranscriptionError> {
        let mut cursor = Cursor::new(Vec::new());

        {
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: 16000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };

            let mut writer = hound::WavWriter::new(&mut cursor, spec).map_err(|e| {
                TranscriptionError::AudioConversion(format!("failed to create WAV writer: {e}"))
            })?;

            // Convert f32 [-1.0, 1.0] to i16 samples
            for &sample in audio_data {
                #[allow(clippy::cast_possible_truncation)] // Intentional conversion to i16 range
                let sample_i16 = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
                writer.write_sample(sample_i16).map_err(|e| {
                    TranscriptionError::AudioConversion(format!("failed to write WAV sample: {e}"))
                })?;
            }

            writer.finalize().map_err(|e| {
                TranscriptionError::AudioConversion(format!("failed to finalize WAV: {e}"))
            })?;
        }

        Ok(cursor.into_inner())
    }
}

impl TranscriptionBackend for DeepgramBackend {
    fn transcribe(&self, audio_data: &[f32]) -> Result<String, TranscriptionError> {
        let _span =
            tracing::debug_span!("deepgram_transcription", samples = audio_data.len()).entered();
        tracing::debug!(
            model = %self.model,
            language = ?self.language,
            smart_format = self.smart_format,
            "starting deepgram transcription"
        );

        // Convert PCM to WAV format
        let start = std::time::Instant::now();
        let wav_bytes = Self::convert_pcm_to_wav(audio_data)?;
        let conversion_duration = start.elapsed();
        tracing::debug!(
            wav_size_bytes = wav_bytes.len(),
            conversion_ms = conversion_duration.as_millis(),
            "audio converted to WAV"
        );

        // TODO: Deepgram SDK v0.7 API integration needs completion
        // The deepgram crate API has changed and requires research into the correct usage
        // for prerecorded transcription with audio buffers.
        //
        // Required implementation:
        // 1. Create AudioSource from WAV bytes (use deepgram::common or appropriate module)
        // 2. Build Options with model, language, smart_format
        // 3. Call client.transcription().prerecorded(source, options)
        // 4. Extract transcript from response
        //
        // References:
        // - https://docs.rs/deepgram
        // - https://github.com/deepgram/deepgram-rust-sdk/tree/main/examples
        //
        // For now, return error indicating incomplete implementation
        let _ = wav_bytes; // Suppress unused variable warning
        let _ = conversion_duration;

        Err(TranscriptionError::DeepgramApi(format!(
            "Deepgram backend not fully implemented - API needs completion. \
                Model: {}, Language: {:?}, Smart format: {}",
            self.model, self.language, self.smart_format
        )))
    }

    fn backend_name(&self) -> &'static str {
        "deepgram"
    }
}

// SAFETY: DeepgramBackend is thread-safe because:
// 1. Deepgram client is wrapped in Arc and is documented as thread-safe
// 2. All fields are either Arc (shared), String (owned), or primitive types
// 3. No shared mutable state exists
#[allow(unsafe_code)]
unsafe impl Send for DeepgramBackend {}
#[allow(unsafe_code)]
unsafe impl Sync for DeepgramBackend {}
