use anyhow::{Context, Result};
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

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
    #[allow(dead_code)] // Used in Phase 5
    StateCreation,

    /// Transcription inference failed
    #[error("failed to transcribe audio")]
    Transcription(#[from] anyhow::Error),
}

/// Whisper transcription engine
pub struct TranscriptionEngine {
    /// Whisper context (thread-safe)
    #[allow(dead_code)] // Used in transcribe() method (Phase 5)
    ctx: Arc<Mutex<WhisperContext>>,
    /// Number of CPU threads for inference
    threads: i32,
    /// Beam search width
    beam_size: i32,
    /// Language code (None = auto-detect)
    language: Option<String>,
}

impl TranscriptionEngine {
    /// Determines sampling strategy based on beam size (pure, testable)
    const fn get_sampling_strategy(beam_size: i32) -> SamplingStrategy {
        if beam_size > 1 {
            SamplingStrategy::BeamSearch {
                beam_size,
                patience: -1.0,
            }
        } else {
            SamplingStrategy::Greedy { best_of: 1 }
        }
    }

    /// Creates a new `TranscriptionEngine` by loading the model from the given path
    ///
    /// # Errors
    /// Returns error if model file doesn't exist, is invalid, or if `threads`/`beam_size` exceed `i32::MAX`
    pub fn new(
        model_path: &Path,
        threads: usize,
        beam_size: usize,
        language: Option<String>,
    ) -> Result<Self, TranscriptionError> {
        if threads == 0 {
            return Err(TranscriptionError::ModelLoad {
                path: model_path.display().to_string(),
                source: anyhow::anyhow!("threads must be > 0"),
            });
        }
        if beam_size == 0 {
            return Err(TranscriptionError::ModelLoad {
                path: model_path.display().to_string(),
                source: anyhow::anyhow!("beam_size must be > 0"),
            });
        }

        // Validate that threads and beam_size fit in i32 (required by whisper-rs API)
        let threads_i32 = i32::try_from(threads).map_err(|_| TranscriptionError::ModelLoad {
            path: model_path.display().to_string(),
            source: anyhow::anyhow!("threads value too large (max: {})", i32::MAX),
        })?;
        let beam_size_i32 =
            i32::try_from(beam_size).map_err(|_| TranscriptionError::ModelLoad {
                path: model_path.display().to_string(),
                source: anyhow::anyhow!("beam_size value too large (max: {})", i32::MAX),
            })?;

        tracing::info!(
            path = %model_path.display(),
            threads = threads,
            beam_size = beam_size,
            language = ?language,
            "loading whisper model"
        );

        let path_str = model_path
            .to_str()
            .ok_or_else(|| TranscriptionError::ModelLoad {
                path: model_path.display().to_string(),
                source: anyhow::anyhow!("model path contains invalid UTF-8"),
            })?;

        let params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(path_str, params).map_err(|e| {
            TranscriptionError::ModelLoad {
                path: model_path.display().to_string(),
                source: anyhow::anyhow!("{e:?}"),
            }
        })?;

        tracing::info!("whisper model loaded successfully");

        Ok(Self {
            ctx: Arc::new(Mutex::new(ctx)),
            threads: threads_i32,
            beam_size: beam_size_i32,
            language,
        })
    }

    /// Transcribes audio samples (16kHz mono f32) to text with language auto-detection
    ///
    /// # Errors
    /// Returns error if Whisper inference fails or mutex is poisoned
    #[allow(dead_code)] // Used in Phase 5
    pub fn transcribe(&self, audio_data: &[f32]) -> Result<String, TranscriptionError> {
        let _span = tracing::debug_span!("transcription", samples = audio_data.len()).entered();
        tracing::debug!("starting transcription");

        // Create state for this transcription
        let mut state = self
            .ctx
            .lock()
            .map_err(|e| anyhow::anyhow!("mutex poisoned: {e}"))?
            .create_state()
            .map_err(|_| TranscriptionError::StateCreation)?;

        // Configure transcription parameters with optimization settings
        let strategy = Self::get_sampling_strategy(self.beam_size);
        let mut params = FullParams::new(strategy);
        params.set_n_threads(self.threads);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_language(self.language.as_deref()); // Use configured language or auto-detect
        params.set_translate(false);

        // Run transcription
        let start = std::time::Instant::now();
        state
            .full(params, audio_data)
            .context("whisper inference failed")?;
        let inference_duration = start.elapsed();

        // Extract text from all segments
        let mut result = String::new();
        for segment in state.as_iter() {
            result.push_str(&segment.to_string());
        }

        // Trim whitespace
        let result = result.trim().to_owned();

        tracing::info!(
            segments = state.full_n_segments(),
            text_len = result.len(),
            inference_ms = inference_duration.as_millis(),
            "transcription completed"
        );

        Ok(result)
    }
}

// SAFETY: TranscriptionEngine is thread-safe because:
// 1. WhisperContext is wrapped in Arc<Mutex<>>, ensuring exclusive access
// 2. All methods require acquiring the mutex lock before accessing the context
// 3. No shared mutable state exists outside the mutex
// 4. whisper-rs WhisperContext is documented as thread-safe when properly synchronized
#[allow(unsafe_code)]
unsafe impl Send for TranscriptionEngine {}
#[allow(unsafe_code)]
unsafe impl Sync for TranscriptionEngine {}

#[cfg(test)]
#[allow(clippy::print_stderr)] // Test diagnostics
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_test_model_path() -> Option<PathBuf> {
        // Check if a test model exists
        let home = std::env::var("HOME").ok()?;
        let path = PathBuf::from(home)
            .join(".whisper-hotkey")
            .join("models")
            .join("ggml-tiny.bin");

        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    #[test]
    fn test_model_load_nonexistent_path() {
        let nonexistent_path = Path::new("/tmp/nonexistent_model.bin");
        let result = TranscriptionEngine::new(nonexistent_path, 4, 5, None);

        assert!(result.is_err());
        assert!(matches!(result, Err(TranscriptionError::ModelLoad { .. })));
        if let Err(TranscriptionError::ModelLoad { path, .. }) = result {
            assert!(path.contains("nonexistent_model.bin"));
        }
    }

    #[test]
    #[ignore = "requires actual model file"]
    fn test_model_load_success() {
        let Some(model_path) = get_test_model_path() else {
            eprintln!("Skipping test: no model found at ~/.whisper-hotkey/models/ggml-tiny.bin");
            return;
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5, None);
        assert!(engine.is_ok(), "Failed to load model: {:?}", engine.err());
    }

    #[test]
    #[ignore = "requires actual model file"]
    fn test_transcribe_silence() {
        let Some(model_path) = get_test_model_path() else {
            eprintln!("Skipping test: no model found");
            return;
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5, None).unwrap();

        // 1 second of silence (16kHz)
        let silence: Vec<f32> = vec![0.0; 16000];

        let result = engine.transcribe(&silence);
        assert!(result.is_ok());

        // Silence should produce empty or minimal output
        let text = result.unwrap();
        assert!(
            text.is_empty() || text.len() < 50,
            "Expected empty or minimal output for silence, got: '{text}'"
        );
    }

    #[test]
    #[ignore = "requires actual model file"]
    fn test_transcribe_empty_audio() {
        let Some(model_path) = get_test_model_path() else {
            eprintln!("Skipping test: no model found");
            return;
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5, None).unwrap();

        let empty: Vec<f32> = vec![];

        let result = engine.transcribe(&empty);
        // Empty audio might fail or return empty string
        // Both are acceptable behaviors
        if let Ok(text) = result {
            assert!(text.is_empty() || text.len() < 50);
        }
    }

    #[test]
    #[ignore = "requires actual model file"]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn test_transcribe_short_audio() {
        let Some(model_path) = get_test_model_path() else {
            eprintln!("Skipping test: no model found");
            return;
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5, None).unwrap();

        // 0.5 seconds of a simple tone (440Hz sine wave)
        let sample_rate = 16000.0;
        let duration = 0.5;
        let frequency = 440.0;
        let samples = (sample_rate * duration) as usize;

        let audio: Vec<f32> = (0..samples)
            .map(|i| {
                let t = i as f32 / sample_rate;
                (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.5
            })
            .collect();

        let result = engine.transcribe(&audio);
        assert!(result.is_ok());

        // Tone should produce some output (might be empty or gibberish)
        // Just verify it doesn't crash
    }

    #[test]
    #[ignore = "requires actual model file"]
    fn test_multiple_transcriptions() {
        let Some(model_path) = get_test_model_path() else {
            eprintln!("Skipping test: no model found");
            return;
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5, None).unwrap();

        // Run multiple transcriptions to verify state management works
        for _ in 0..3 {
            let silence: Vec<f32> = vec![0.0; 16000];
            let result = engine.transcribe(&silence);
            assert!(result.is_ok());
        }
    }

    #[test]
    #[ignore = "requires actual model file"]
    fn test_transcribe_different_lengths() {
        let Some(model_path) = get_test_model_path() else {
            eprintln!("Skipping test: no model found");
            return;
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5, None).unwrap();

        // Test different audio lengths
        let lengths = vec![8000, 16000, 32000, 48000]; // 0.5s, 1s, 2s, 3s

        for length in lengths {
            let audio: Vec<f32> = vec![0.0; length];
            let result = engine.transcribe(&audio);
            assert!(result.is_ok(), "Failed to transcribe {length} samples");
        }
    }

    #[test]
    #[ignore = "requires actual model file"]
    fn test_long_recording_30s() {
        let Some(model_path) = get_test_model_path() else {
            eprintln!("Skipping test: no model found");
            return;
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5, None).unwrap();

        // 30 seconds of silence (16kHz)
        let audio: Vec<f32> = vec![0.0; 16000 * 30];

        let result = engine.transcribe(&audio);
        assert!(result.is_ok(), "Failed to transcribe 30s audio");
    }

    #[test]
    #[ignore = "requires actual model file"]
    fn test_optimization_params() {
        // NOTE: This test validates that different optimization parameters are accepted
        // without crashing, but does not verify that they actually affect behavior or
        // transcription quality. For performance validation, see manual tests in TESTING.md.
        let Some(model_path) = get_test_model_path() else {
            eprintln!("Skipping test: no model found");
            return;
        };

        // Test with different optimization params
        let engine_default = TranscriptionEngine::new(&model_path, 4, 5, None).unwrap();
        let engine_fast = TranscriptionEngine::new(&model_path, 8, 1, None).unwrap();
        let engine_accurate = TranscriptionEngine::new(&model_path, 4, 10, None).unwrap();

        let silence: Vec<f32> = vec![0.0; 16000];

        // All should work without errors
        assert!(engine_default.transcribe(&silence).is_ok());
        assert!(engine_fast.transcribe(&silence).is_ok());
        assert!(engine_accurate.transcribe(&silence).is_ok());
    }

    #[test]
    #[ignore = "requires actual model file"]
    #[allow(clippy::cast_precision_loss)]
    fn test_transcribe_noise() {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};

        let Some(model_path) = get_test_model_path() else {
            eprintln!("Skipping test: no model found");
            return;
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5, None).unwrap();

        // 2 seconds of random noise (16kHz)
        let hasher = RandomState::new().build_hasher();
        let seed = hasher.finish();

        let mut rng_state = seed;
        let mut noise = Vec::with_capacity(32000);
        for _ in 0..32000 {
            // Simple LCG for deterministic noise
            rng_state = rng_state.wrapping_mul(1_103_515_245).wrapping_add(12345);
            let sample = ((rng_state >> 16) as f32 / 32768.0) - 1.0;
            noise.push(sample * 0.1); // Low amplitude noise
        }

        let result = engine.transcribe(&noise);
        assert!(result.is_ok(), "Failed to transcribe noise");

        // Noise should produce empty or minimal/gibberish output
        let _text = result.unwrap();
        // Just verify it doesn't crash - output is unpredictable for noise
    }

    #[test]
    fn test_engine_is_send_sync() {
        // Verify TranscriptionEngine can be shared across threads
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<TranscriptionEngine>();
        assert_sync::<TranscriptionEngine>();
    }

    #[test]
    fn test_new_with_zero_threads() {
        let path = Path::new("/tmp/dummy.bin");
        let result = TranscriptionEngine::new(path, 0, 5, None);
        assert!(result.is_err());
        assert!(matches!(result, Err(TranscriptionError::ModelLoad { .. })));
        if let Err(TranscriptionError::ModelLoad { source, .. }) = result {
            assert!(source.to_string().contains("threads must be > 0"));
        }
    }

    #[test]
    fn test_new_with_zero_beam_size() {
        let path = Path::new("/tmp/dummy.bin");
        let result = TranscriptionEngine::new(path, 4, 0, None);
        assert!(result.is_err());
        assert!(matches!(result, Err(TranscriptionError::ModelLoad { .. })));
        if let Err(TranscriptionError::ModelLoad { source, .. }) = result {
            assert!(source.to_string().contains("beam_size must be > 0"));
        }
    }

    #[test]
    fn test_new_with_valid_params() {
        let path = Path::new("/tmp/nonexistent_but_valid_params.bin");
        let result = TranscriptionEngine::new(path, 4, 5, Some("en".to_owned()));
        // Will fail because file doesn't exist, but params are validated first
        assert!(result.is_err());
        assert!(matches!(result, Err(TranscriptionError::ModelLoad { .. })));
    }

    #[test]
    fn test_thread_count_edge_cases() {
        let path = Path::new("/tmp/dummy.bin");

        // Test max i32 threads (i32::MAX as usize fits in i32, so validation passes)
        // This tests that valid thread counts fail only on file load, not validation
        let result = TranscriptionEngine::new(path, i32::MAX as usize, 5, None);
        assert!(result.is_err());
        assert!(matches!(result, Err(TranscriptionError::ModelLoad { .. })));

        // Test overflow: usize > i32::MAX
        #[cfg(target_pointer_width = "64")]
        {
            let result = TranscriptionEngine::new(path, (i32::MAX as usize) + 1, 5, None);
            assert!(result.is_err());
            assert!(matches!(result, Err(TranscriptionError::ModelLoad { .. })));
            if let Err(TranscriptionError::ModelLoad { source, .. }) = result {
                assert!(source.to_string().contains("threads value too large"));
            }
        }
    }

    #[test]
    fn test_beam_size_edge_cases() {
        let path = Path::new("/tmp/dummy.bin");

        // Test max i32 beam_size (i32::MAX as usize fits in i32, so validation passes)
        // This tests that valid beam sizes fail only on file load, not validation
        let result = TranscriptionEngine::new(path, 4, i32::MAX as usize, None);
        assert!(result.is_err());
        assert!(matches!(result, Err(TranscriptionError::ModelLoad { .. })));

        // Test overflow: usize > i32::MAX
        #[cfg(target_pointer_width = "64")]
        {
            let result = TranscriptionEngine::new(path, 4, (i32::MAX as usize) + 1, None);
            assert!(result.is_err());
            assert!(matches!(result, Err(TranscriptionError::ModelLoad { .. })));
            if let Err(TranscriptionError::ModelLoad { source, .. }) = result {
                assert!(source.to_string().contains("beam_size value too large"));
            }
        }
    }

    // Phase 4: Sampling strategy tests (pure logic, fully testable)
    #[test]
    fn test_get_sampling_strategy_greedy() {
        // beam_size = 1 should use Greedy strategy
        let strategy = TranscriptionEngine::get_sampling_strategy(1);
        assert!(matches!(strategy, SamplingStrategy::Greedy { best_of: 1 }));
    }

    #[test]
    fn test_get_sampling_strategy_beam_search() {
        // beam_size > 1 should use BeamSearch strategy
        let strategy = TranscriptionEngine::get_sampling_strategy(5);
        assert!(
            matches!(
                strategy,
                SamplingStrategy::BeamSearch {
                    beam_size: 5,
                    patience: -1.0
                }
            ),
            "Expected BeamSearch with beam_size=5, patience=-1.0"
        );
    }

    #[test]
    fn test_get_sampling_strategy_various_beam_sizes() {
        // Test different beam sizes
        for beam in [1, 2, 3, 5, 8, 10] {
            let strategy = TranscriptionEngine::get_sampling_strategy(beam);
            if beam == 1 {
                assert!(matches!(strategy, SamplingStrategy::Greedy { .. }));
            } else {
                assert!(
                    matches!(strategy, SamplingStrategy::BeamSearch { beam_size, .. } if beam_size == beam),
                    "Expected BeamSearch with beam_size={beam}"
                );
            }
        }
    }

    #[test]
    fn test_get_sampling_strategy_large_beam() {
        // Test with large beam size
        let strategy = TranscriptionEngine::get_sampling_strategy(100);
        assert!(
            matches!(
                strategy,
                SamplingStrategy::BeamSearch { beam_size: 100, .. }
            ),
            "Expected BeamSearch with beam_size=100"
        );
    }

    #[test]
    fn test_get_sampling_strategy_min_beam() {
        // Test boundary: beam_size = 1 is Greedy, beam_size = 2 is BeamSearch
        let greedy = TranscriptionEngine::get_sampling_strategy(1);
        assert!(matches!(greedy, SamplingStrategy::Greedy { .. }));

        let beam = TranscriptionEngine::get_sampling_strategy(2);
        assert!(matches!(beam, SamplingStrategy::BeamSearch { .. }));
    }

    #[test]
    fn test_get_sampling_strategy_patience_always_negative_one() {
        // Verify patience is always -1.0 for BeamSearch
        for beam_size in [2, 5, 10, 20] {
            let strategy = TranscriptionEngine::get_sampling_strategy(beam_size);
            assert!(
                matches!(
                    strategy,
                    SamplingStrategy::BeamSearch { patience: -1.0, .. }
                ),
                "Expected BeamSearch with patience=-1.0 for beam_size={beam_size}"
            );
        }
    }
}
