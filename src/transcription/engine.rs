use anyhow::{Context, Result};
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[derive(Debug, Error)]
pub enum TranscriptionError {
    #[error("failed to load whisper model from {path}: {source}")]
    ModelLoad { path: String, source: anyhow::Error },

    #[error("failed to create whisper state")]
    #[allow(dead_code)] // Used in Phase 5
    StateCreation,

    #[error("failed to transcribe audio")]
    Transcription(#[from] anyhow::Error),
}

pub struct TranscriptionEngine {
    #[allow(dead_code)] // Used in transcribe() method (Phase 5)
    ctx: Arc<Mutex<WhisperContext>>,
    threads: usize,
    beam_size: usize,
}

impl TranscriptionEngine {
    /// Creates a new TranscriptionEngine by loading the model from the given path
    pub fn new(
        model_path: &Path,
        threads: usize,
        beam_size: usize,
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

        tracing::info!(
            path = %model_path.display(),
            threads = threads,
            beam_size = beam_size,
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
                source: anyhow::anyhow!("{:?}", e),
            }
        })?;

        tracing::info!("whisper model loaded successfully");

        Ok(Self {
            ctx: Arc::new(Mutex::new(ctx)),
            threads,
            beam_size,
        })
    }

    /// Transcribes audio samples (16kHz mono f32) to text with language auto-detection
    #[allow(dead_code)] // Used in Phase 5
    pub fn transcribe(&self, audio_data: &[f32]) -> Result<String, TranscriptionError> {
        let _span = tracing::debug_span!("transcription", samples = audio_data.len()).entered();
        tracing::debug!("starting transcription");

        let ctx = self
            .ctx
            .lock()
            .map_err(|e| anyhow::anyhow!("mutex poisoned: {}", e))?;

        // Create state for this transcription
        let mut state = ctx
            .create_state()
            .map_err(|_| TranscriptionError::StateCreation)?;

        // Configure transcription parameters with optimization settings
        let strategy = if self.beam_size > 1 {
            SamplingStrategy::BeamSearch {
                beam_size: self.beam_size as i32,
                patience: -1.0,
            }
        } else {
            SamplingStrategy::Greedy { best_of: 1 }
        };

        let mut params = FullParams::new(strategy);
        params.set_n_threads(self.threads as i32);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_language(None); // Auto-detect language
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
        let result = result.trim().to_string();

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
unsafe impl Send for TranscriptionEngine {}
unsafe impl Sync for TranscriptionEngine {}

#[cfg(test)]
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
        let result = TranscriptionEngine::new(nonexistent_path, 4, 5);

        assert!(result.is_err());
        match result {
            Err(TranscriptionError::ModelLoad { path, .. }) => {
                assert!(path.contains("nonexistent_model.bin"));
            }
            _ => panic!("Expected ModelLoad error"),
        }
    }

    #[test]
    #[ignore] // Requires actual model file
    fn test_model_load_success() {
        let model_path = match get_test_model_path() {
            Some(path) => path,
            None => {
                eprintln!(
                    "Skipping test: no model found at ~/.whisper-hotkey/models/ggml-tiny.bin"
                );
                return;
            }
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5);
        assert!(engine.is_ok(), "Failed to load model: {:?}", engine.err());
    }

    #[test]
    #[ignore] // Requires actual model file
    fn test_transcribe_silence() {
        let model_path = match get_test_model_path() {
            Some(path) => path,
            None => {
                eprintln!("Skipping test: no model found");
                return;
            }
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5).unwrap();

        // 1 second of silence (16kHz)
        let silence: Vec<f32> = vec![0.0; 16000];

        let result = engine.transcribe(&silence);
        assert!(result.is_ok());

        // Silence should produce empty or minimal output
        let text = result.unwrap();
        assert!(
            text.is_empty() || text.len() < 50,
            "Expected empty or minimal output for silence, got: '{}'",
            text
        );
    }

    #[test]
    #[ignore] // Requires actual model file
    fn test_transcribe_empty_audio() {
        let model_path = match get_test_model_path() {
            Some(path) => path,
            None => {
                eprintln!("Skipping test: no model found");
                return;
            }
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5).unwrap();

        let empty: Vec<f32> = vec![];

        let result = engine.transcribe(&empty);
        // Empty audio might fail or return empty string
        // Both are acceptable behaviors
        if let Ok(text) = result {
            assert!(text.is_empty() || text.len() < 50);
        }
    }

    #[test]
    #[ignore] // Requires actual model file
    fn test_transcribe_short_audio() {
        let model_path = match get_test_model_path() {
            Some(path) => path,
            None => {
                eprintln!("Skipping test: no model found");
                return;
            }
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5).unwrap();

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
    #[ignore] // Requires actual model file
    fn test_multiple_transcriptions() {
        let model_path = match get_test_model_path() {
            Some(path) => path,
            None => {
                eprintln!("Skipping test: no model found");
                return;
            }
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5).unwrap();

        // Run multiple transcriptions to verify state management works
        for _ in 0..3 {
            let silence: Vec<f32> = vec![0.0; 16000];
            let result = engine.transcribe(&silence);
            assert!(result.is_ok());
        }
    }

    #[test]
    #[ignore] // Requires actual model file
    fn test_transcribe_different_lengths() {
        let model_path = match get_test_model_path() {
            Some(path) => path,
            None => {
                eprintln!("Skipping test: no model found");
                return;
            }
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5).unwrap();

        // Test different audio lengths
        let lengths = vec![8000, 16000, 32000, 48000]; // 0.5s, 1s, 2s, 3s

        for length in lengths {
            let audio: Vec<f32> = vec![0.0; length];
            let result = engine.transcribe(&audio);
            assert!(result.is_ok(), "Failed to transcribe {} samples", length);
        }
    }

    #[test]
    #[ignore] // Requires actual model file
    fn test_long_recording_30s() {
        let model_path = match get_test_model_path() {
            Some(path) => path,
            None => {
                eprintln!("Skipping test: no model found");
                return;
            }
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5).unwrap();

        // 30 seconds of silence (16kHz)
        let audio: Vec<f32> = vec![0.0; 16000 * 30];

        let result = engine.transcribe(&audio);
        assert!(result.is_ok(), "Failed to transcribe 30s audio");
    }

    #[test]
    #[ignore] // Requires actual model file
    fn test_optimization_params() {
        // NOTE: This test validates that different optimization parameters are accepted
        // without crashing, but does not verify that they actually affect behavior or
        // transcription quality. For performance validation, see manual tests in TESTING.md.
        let model_path = match get_test_model_path() {
            Some(path) => path,
            None => {
                eprintln!("Skipping test: no model found");
                return;
            }
        };

        // Test with different optimization params
        let engine_default = TranscriptionEngine::new(&model_path, 4, 5).unwrap();
        let engine_fast = TranscriptionEngine::new(&model_path, 8, 1).unwrap();
        let engine_accurate = TranscriptionEngine::new(&model_path, 4, 10).unwrap();

        let silence: Vec<f32> = vec![0.0; 16000];

        // All should work without errors
        assert!(engine_default.transcribe(&silence).is_ok());
        assert!(engine_fast.transcribe(&silence).is_ok());
        assert!(engine_accurate.transcribe(&silence).is_ok());
    }

    #[test]
    #[ignore] // Requires actual model file
    fn test_transcribe_noise() {
        let model_path = match get_test_model_path() {
            Some(path) => path,
            None => {
                eprintln!("Skipping test: no model found");
                return;
            }
        };

        let engine = TranscriptionEngine::new(&model_path, 4, 5).unwrap();

        // 2 seconds of random noise (16kHz)
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        let hasher = RandomState::new().build_hasher();
        let seed = hasher.finish();

        let mut rng_state = seed;
        let mut noise = Vec::with_capacity(32000);
        for _ in 0..32000 {
            // Simple LCG for deterministic noise
            rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
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
}
