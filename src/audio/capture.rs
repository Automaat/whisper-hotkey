use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapCons, HeapRb,
};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::config::AudioConfig;

/// Trait for controlling audio stream lifecycle
trait StreamControl {
    /// Resume audio stream (activate microphone)
    fn play(&self) -> Result<()>;
    /// Pause audio stream (deactivate microphone)
    fn pause(&self) -> Result<()>;
}

/// CPAL stream wrapper implementing `StreamControl`
struct CpalStreamControl {
    stream: cpal::Stream,
}

impl StreamControl for CpalStreamControl {
    fn play(&self) -> Result<()> {
        self.stream.play().context("failed to resume audio stream")
    }

    fn pause(&self) -> Result<()> {
        self.stream.pause().context("failed to pause audio stream")
    }
}

/// Audio capture using CoreAudio/CPAL
pub struct AudioCapture {
    /// Stream controller (kept alive to prevent stream drop)
    #[allow(dead_code)] // Kept alive to prevent stream drop
    stream_control: Option<Box<dyn StreamControl>>,
    /// Ring buffer consumer for reading captured samples
    ring_buffer_consumer: HeapCons<f32>,
    /// Recording state flag
    is_recording: Arc<AtomicBool>,
    /// Device sample rate in Hz
    device_sample_rate: u32,
    /// Number of audio channels
    device_channels: u16,
}

impl AudioCapture {
    /// Creates a new audio capture instance
    ///
    /// # Errors
    /// Returns error if default audio device is unavailable or stream creation fails
    pub fn new(_config: &AudioConfig) -> Result<Self> {
        info!("initializing audio capture");

        // Get default input device
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("no input device available")?;

        let device_name = device.name().unwrap_or_else(|_| "unknown".to_owned());
        info!("using input device: {}", device_name);

        // Get device config (use device default, will resample to 16kHz later)
        let supported_config = device
            .default_input_config()
            .context("failed to get default input config")?;

        let device_sample_rate = supported_config.sample_rate().0;
        let device_channels = supported_config.channels();

        info!(
            "device config: {} Hz, {} channels",
            device_sample_rate, device_channels
        );

        // Create ring buffer sized for max recording duration (30s at device sample rate)
        // This ensures no samples are dropped during recording
        let max_recording_secs = 30;
        let ring_buffer_capacity =
            (device_sample_rate as usize) * (device_channels as usize) * max_recording_secs;
        info!(
            "ring buffer capacity: {} samples ({} seconds at {} Hz)",
            ring_buffer_capacity, max_recording_secs, device_sample_rate
        );
        let ring_buffer = HeapRb::<f32>::new(ring_buffer_capacity);
        let (ring_buffer_producer, ring_buffer_consumer) = ring_buffer.split();

        let is_recording = Arc::new(AtomicBool::new(false));

        // Build input stream with callback
        let is_recording_clone = Arc::clone(&is_recording);
        let mut producer = ring_buffer_producer;

        let stream_config = supported_config.into();
        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if is_recording_clone.load(Ordering::Relaxed) {
                        // Lock-free push to ring buffer
                        let pushed = producer.push_slice(data);
                        if pushed < data.len() {
                            warn!("ring buffer full, dropped {} samples", data.len() - pushed);
                        }
                    }
                },
                move |err| {
                    warn!("audio stream error: {}", err);
                },
                None,
            )
            .context("failed to build input stream")?;

        // Wrap stream in controller
        let stream_control = CpalStreamControl { stream };

        // Start the stream and immediately pause it (mic inactive until hotkey pressed)
        stream_control.play()?;
        stream_control.pause()?;
        info!("audio stream initialized (paused)");

        Ok(Self {
            stream_control: Some(Box::new(stream_control)),
            ring_buffer_consumer,
            is_recording,
            device_sample_rate,
            device_channels,
        })
    }

    /// Starts recording audio
    ///
    /// # Errors
    /// Returns error if ring buffer flush fails
    #[allow(clippy::unnecessary_wraps)] // Consistent API, may add fallible ops later
    pub fn start_recording(&mut self) -> Result<()> {
        let _span = tracing::debug_span!("start_recording").entered();
        let start = std::time::Instant::now();
        debug!("starting recording");

        // Clear ring buffer
        self.ring_buffer_consumer.clear();

        // Set recording flag BEFORE resuming stream to avoid race condition
        self.is_recording.store(true, Ordering::Relaxed);

        // Resume audio stream (activate microphone)
        if let Some(stream_control) = &self.stream_control {
            stream_control.play()?;
        }

        let duration = start.elapsed();
        info!(latency_us = duration.as_micros(), "recording started");
        Ok(())
    }

    /// Stops recording and returns captured samples (16kHz mono f32)
    ///
    /// # Errors
    /// Returns error if sample conversion fails
    #[allow(clippy::unnecessary_wraps)] // Consistent API, may add fallible ops later
    pub fn stop_recording(&mut self) -> Result<Vec<f32>> {
        let _span = tracing::debug_span!("stop_recording").entered();
        let start_total = std::time::Instant::now();
        debug!("stopping recording");

        // Clear recording flag
        self.is_recording.store(false, Ordering::Relaxed);

        // Pause audio stream (deactivate microphone)
        if let Some(stream_control) = &self.stream_control {
            stream_control.pause()?;
        }

        // Drain ring buffer into Vec
        let start_drain = std::time::Instant::now();
        let mut samples = Vec::new();
        while let Some(sample) = self.ring_buffer_consumer.try_pop() {
            samples.push(sample);
        }
        let drain_duration = start_drain.elapsed();

        info!(
            samples = samples.len(),
            drain_us = drain_duration.as_micros(),
            "ring buffer drained"
        );

        // Convert to 16kHz mono
        let samples_16khz_mono = self.convert_to_16khz_mono(&samples);

        let total_duration = start_total.elapsed();
        info!(
            total_ms = total_duration.as_millis(),
            "stop_recording complete"
        );

        Ok(samples_16khz_mono)
    }

    fn convert_to_16khz_mono(&self, samples: &[f32]) -> Vec<f32> {
        let _span = tracing::debug_span!("convert_to_16khz_mono").entered();
        let start_total = std::time::Instant::now();
        let target_sample_rate = 16000;

        // Convert stereo to mono if needed
        let start_downmix = std::time::Instant::now();
        let mono_samples = if self.device_channels == 1 {
            samples.to_vec()
        } else {
            // Average channels (simple downmix)
            let channels_f64 = f64::from(self.device_channels);
            samples
                .chunks(self.device_channels as usize)
                .map(|frame| {
                    let sum_f64: f64 = frame.iter().map(|&s| f64::from(s)).sum();
                    // f64 → f32: audio samples are stored as f32, precision sufficient
                    #[allow(clippy::cast_possible_truncation)]
                    {
                        (sum_f64 / channels_f64) as f32
                    }
                })
                .collect()
        };
        let downmix_duration = start_downmix.elapsed();

        if self.device_channels > 1 {
            debug!(
                channels = self.device_channels,
                downmix_us = downmix_duration.as_micros(),
                "stereo to mono conversion"
            );
        }

        // Resample if needed
        if self.device_sample_rate == target_sample_rate {
            return mono_samples;
        }

        // Simple linear interpolation resampling
        // Algorithm requires f64 ↔ usize conversions for fractional index calculations
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let resampled = {
            let start_resample = std::time::Instant::now();
            let ratio = f64::from(self.device_sample_rate) / f64::from(target_sample_rate);

            // Calculate output length - ratio is always positive for valid sample rates
            let output_len_f64 = (mono_samples.len() as f64) / ratio;
            let output_len = if output_len_f64.is_finite() && output_len_f64 >= 0.0 {
                output_len_f64.ceil() as usize
            } else {
                mono_samples.len()
            };

            let mut resampled = Vec::with_capacity(output_len);
            for i in 0..output_len {
                // Calculate source index with linear interpolation
                let src_idx_f64 = (i as f64) * ratio;

                // Floor gives integer part, safe because src_idx >= 0
                let src_idx_floor = if src_idx_f64 >= 0.0 && src_idx_f64 < (usize::MAX as f64) {
                    src_idx_f64.floor() as usize
                } else {
                    0
                };

                let src_idx_ceil = (src_idx_floor + 1).min(mono_samples.len().saturating_sub(1));
                let fract = src_idx_f64 - src_idx_f64.floor();

                let sample = if src_idx_floor < mono_samples.len() {
                    let s1 = f64::from(mono_samples[src_idx_floor]);
                    let s2 = f64::from(mono_samples[src_idx_ceil]);
                    // Use mul_add for better precision
                    let interpolated = s1.mul_add(1.0 - fract, s2 * fract);
                    interpolated as f32
                } else {
                    0.0_f32
                };

                resampled.push(sample);
            }

            let resample_duration = start_resample.elapsed();
            info!(
                device_rate = self.device_sample_rate,
                target_rate = target_sample_rate,
                input_samples = mono_samples.len(),
                output_samples = resampled.len(),
                resample_us = resample_duration.as_micros(),
                "resampling completed"
            );

            resampled
        };

        let total_duration = start_total.elapsed();
        debug!(
            total_us = total_duration.as_micros(),
            "audio conversion complete"
        );

        resampled
    }

    /// Save samples to WAV file for debugging
    ///
    /// # Errors
    /// Returns error if directory creation or file write fails
    pub fn save_wav_debug(samples: &[f32], path: &Path) -> Result<()> {
        debug!("saving WAV debug file: {:?}", path);

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("failed to create debug directory")?;
        }

        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = WavWriter::create(path, spec).context("failed to create WAV file")?;

        for &sample in samples {
            writer
                .write_sample(sample)
                .context("failed to write sample")?;
        }

        writer.finalize().context("failed to finalize WAV file")?;

        info!(
            "saved WAV debug file: {:?} ({} samples)",
            path,
            samples.len()
        );
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // Test assertions with known exact values
mod tests {
    use super::*;

    // Mock StreamControl for testing
    struct MockStreamControl {
        play_count: Arc<AtomicBool>,
        pause_count: Arc<AtomicBool>,
    }

    impl StreamControl for MockStreamControl {
        fn play(&self) -> Result<()> {
            self.play_count.store(true, Ordering::Relaxed);
            Ok(())
        }

        fn pause(&self) -> Result<()> {
            self.pause_count.store(true, Ordering::Relaxed);
            Ok(())
        }
    }

    // Mock AudioCapture for testing conversion logic
    fn mock_audio_capture(sample_rate: u32, channels: u16) -> AudioCapture {
        AudioCapture {
            stream_control: None,
            ring_buffer_consumer: HeapRb::<f32>::new(1024).split().1,
            is_recording: Arc::new(AtomicBool::new(false)),
            device_sample_rate: sample_rate,
            device_channels: channels,
        }
    }

    #[test]
    fn test_stereo_to_mono_conversion() {
        let capture = mock_audio_capture(16000, 2);

        // Stereo samples: [L1, R1, L2, R2, L3, R3]
        let stereo_samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

        let result = capture.convert_to_16khz_mono(&stereo_samples);

        // Expected: [(1.0+2.0)/2, (3.0+4.0)/2, (5.0+6.0)/2]
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], 1.5);
        assert_eq!(result[1], 3.5);
        assert_eq!(result[2], 5.5);
    }

    #[test]
    fn test_mono_passthrough_no_resampling() {
        let capture = mock_audio_capture(16000, 1);

        let mono_samples = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        let result = capture.convert_to_16khz_mono(&mono_samples);

        // Should pass through unchanged
        assert_eq!(result, mono_samples);
    }

    #[test]
    fn test_downsampling_48khz_to_16khz() {
        let capture = mock_audio_capture(48000, 1);

        // 48kHz -> 16kHz is 3:1 ratio
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

        let result = capture.convert_to_16khz_mono(&samples);

        // 9 samples at 48kHz -> 3 samples at 16kHz
        assert_eq!(result.len(), 3);

        // Linear interpolation should give values between input samples
        for &sample in &result {
            assert!((1.0..=9.0).contains(&sample));
        }
    }

    #[test]
    fn test_upsampling_8khz_to_16khz() {
        let capture = mock_audio_capture(8000, 1);

        // 8kHz -> 16kHz is 1:2 ratio
        let samples = vec![1.0, 2.0, 3.0, 4.0];

        let result = capture.convert_to_16khz_mono(&samples);

        // 4 samples at 8kHz -> 8 samples at 16kHz
        assert_eq!(result.len(), 8);

        // Interpolated values should be between min and max
        for &sample in &result {
            assert!((1.0..=4.0).contains(&sample));
        }
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn test_stereo_downsampling() {
        let capture = mock_audio_capture(44100, 2);

        // Create 10 stereo frames (20 samples)
        let mut samples = Vec::new();
        for i in 0..10 {
            samples.push(i as f32);
            samples.push((i + 1) as f32);
        }

        let result = capture.convert_to_16khz_mono(&samples);

        // 44.1kHz -> 16kHz is ~2.76:1, 10 frames -> ~4 samples
        assert!(result.len() >= 3 && result.len() <= 5);

        // Mono conversion should average channels
        for &sample in &result {
            assert!((0.0..=11.0).contains(&sample));
        }
    }

    #[test]
    fn test_empty_samples() {
        let capture = mock_audio_capture(44100, 2);

        let empty: Vec<f32> = vec![];
        let result = capture.convert_to_16khz_mono(&empty);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_single_sample_mono() {
        let capture = mock_audio_capture(16000, 1);

        let samples = vec![42.0];
        let result = capture.convert_to_16khz_mono(&samples);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 42.0);
    }

    #[test]
    fn test_multichannel_conversion() {
        let capture = mock_audio_capture(16000, 4); // 4 channels

        // 4-channel samples: [C1, C2, C3, C4, C1, C2, C3, C4]
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

        let result = capture.convert_to_16khz_mono(&samples);

        // Expected: [(1+2+3+4)/4, (5+6+7+8)/4] = [2.5, 6.5]
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], 2.5);
        assert_eq!(result[1], 6.5);
    }

    #[test]
    fn test_resampling_preserves_bounds() {
        let capture = mock_audio_capture(22050, 1);

        // All samples in range [-1.0, 1.0]
        let samples = vec![-1.0, -0.5, 0.0, 0.5, 1.0];

        let result = capture.convert_to_16khz_mono(&samples);

        // Linear interpolation should keep values in same range
        for &sample in &result {
            assert!((-1.0..=1.0).contains(&sample));
        }
    }

    #[test]
    fn test_wav_debug_spec() {
        use std::env;
        use std::fs;

        let samples = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let temp_dir = env::temp_dir();
        let wav_path = temp_dir.join("test_audio.wav");

        // Clean up if exists
        let _ = fs::remove_file(&wav_path);

        let result = AudioCapture::save_wav_debug(&samples, &wav_path);
        assert!(result.is_ok());

        // Verify file exists
        assert!(wav_path.exists());

        // Read back and verify spec
        let reader = hound::WavReader::open(&wav_path).unwrap();
        let spec = reader.spec();

        assert_eq!(spec.channels, 1);
        assert_eq!(spec.sample_rate, 16000);
        assert_eq!(spec.bits_per_sample, 32);
        assert_eq!(spec.sample_format, hound::SampleFormat::Float);

        // Verify sample count
        let sample_count = reader.len() as usize;
        assert_eq!(sample_count, samples.len());

        // Clean up
        let _ = fs::remove_file(wav_path);
    }

    #[test]
    fn test_save_wav_debug_empty_samples() {
        use std::env;
        use std::fs;

        let samples: Vec<f32> = vec![];
        let temp_dir = env::temp_dir();
        let wav_path = temp_dir.join("test_empty.wav");

        let _ = fs::remove_file(&wav_path);

        let result = AudioCapture::save_wav_debug(&samples, &wav_path);
        assert!(result.is_ok());
        assert!(wav_path.exists());

        let _ = fs::remove_file(wav_path);
    }

    #[test]
    fn test_save_wav_debug_creates_parent_dir() {
        use std::env;
        use std::fs;

        let samples = vec![0.1, 0.2];
        let temp_dir = env::temp_dir();
        let nested_path = temp_dir.join("test_audio_nested").join("debug.wav");

        // Ensure parent doesn't exist
        let _ = fs::remove_dir_all(temp_dir.join("test_audio_nested"));

        let result = AudioCapture::save_wav_debug(&samples, &nested_path);
        assert!(result.is_ok());
        assert!(nested_path.exists());

        // Clean up
        let _ = fs::remove_dir_all(temp_dir.join("test_audio_nested"));
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn test_upsampling_maintains_sample_count_ratio() {
        let capture = mock_audio_capture(8000, 1);

        // 10 samples at 8kHz
        let samples = vec![0.0; 10];

        let result = capture.convert_to_16khz_mono(&samples);

        // Should be approximately 20 samples at 16kHz (2x ratio)
        let len_f32 = result.len() as f32;
        assert!((len_f32 - 20.0).abs() < 2.0);
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn test_downsampling_maintains_sample_count_ratio() {
        let capture = mock_audio_capture(32000, 1);

        // 20 samples at 32kHz
        let samples = vec![0.0; 20];

        let result = capture.convert_to_16khz_mono(&samples);

        // Should be approximately 10 samples at 16kHz (0.5x ratio)
        let len_f32 = result.len() as f32;
        assert!((len_f32 - 10.0).abs() < 2.0);
    }

    // Integration tests (require audio hardware, run with: cargo test -- --ignored)

    #[test]
    #[ignore = "requires audio hardware"]
    fn test_audio_capture_initialization() {
        let config = AudioConfig {
            buffer_size: 1024,
            sample_rate: 16000,
        };

        let result = AudioCapture::new(&config);
        assert!(
            result.is_ok(),
            "Audio capture should initialize with valid config"
        );

        let capture = result.unwrap();
        assert!(capture.device_sample_rate > 0);
        assert!(capture.device_channels > 0);
    }

    #[test]
    #[ignore = "requires audio hardware"]
    fn test_start_stop_recording() {
        let config = AudioConfig {
            buffer_size: 1024,
            sample_rate: 16000,
        };

        let mut capture = AudioCapture::new(&config).unwrap();

        // Start recording
        let start_result = capture.start_recording();
        assert!(start_result.is_ok());
        assert!(capture.is_recording.load(Ordering::Relaxed));

        // Wait a bit to capture some audio
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Stop recording
        let stop_result = capture.stop_recording();
        assert!(stop_result.is_ok());
        assert!(!capture.is_recording.load(Ordering::Relaxed));

        let _samples = stop_result.unwrap();
        // Should have captured some samples (depends on system)
        // In quiet environment might be 0, so just verify it doesn't error
    }

    #[test]
    #[ignore = "requires audio hardware"]
    fn test_multiple_recording_cycles() {
        let config = AudioConfig {
            buffer_size: 1024,
            sample_rate: 16000,
        };

        let mut capture = AudioCapture::new(&config).unwrap();

        for _ in 0..3 {
            capture.start_recording().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(50));
            let _samples = capture.stop_recording().unwrap();
            // Just verify no errors, sample count depends on audio input
        }
    }

    #[test]
    #[ignore = "requires audio hardware"]
    fn test_ring_buffer_clearing() {
        let config = AudioConfig {
            buffer_size: 1024,
            sample_rate: 16000,
        };

        let mut capture = AudioCapture::new(&config).unwrap();

        // First recording
        capture.start_recording().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let samples1 = capture.stop_recording().unwrap();

        // Second recording should start fresh (buffer cleared)
        capture.start_recording().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let samples2 = capture.stop_recording().unwrap();

        // Both should have captured data independently
        // (actual lengths depend on audio input)
        // Just verify no errors during recording cycles
        let _ = (samples1, samples2);
    }

    #[test]
    fn test_stream_control_pause_resume() {
        // Create mock stream control to verify play/pause calls
        let play_called = Arc::new(AtomicBool::new(false));
        let pause_called = Arc::new(AtomicBool::new(false));
        let mock_stream = MockStreamControl {
            play_count: Arc::clone(&play_called),
            pause_count: Arc::clone(&pause_called),
        };

        let ring_buffer = HeapRb::<f32>::new(1024);
        let (_, consumer) = ring_buffer.split();

        let mut capture = AudioCapture {
            stream_control: Some(Box::new(mock_stream)),
            ring_buffer_consumer: consumer,
            is_recording: Arc::new(AtomicBool::new(false)),
            device_sample_rate: 16000,
            device_channels: 1,
        };

        // Start recording should call play()
        capture.start_recording().unwrap();
        assert!(play_called.load(Ordering::Relaxed));
        assert!(capture.is_recording.load(Ordering::Relaxed));

        // Stop recording should call pause()
        let _ = capture.stop_recording().unwrap();
        assert!(pause_called.load(Ordering::Relaxed));
        assert!(!capture.is_recording.load(Ordering::Relaxed));
    }

    #[test]
    #[ignore = "requires audio hardware"]
    fn test_stream_pause_resume() {
        let config = AudioConfig {
            buffer_size: 1024,
            sample_rate: 16000,
        };

        let mut capture = AudioCapture::new(&config).unwrap();

        // Stream should be paused initially after new()
        // Start recording should resume stream
        capture.start_recording().unwrap();
        assert!(capture.is_recording.load(Ordering::Relaxed));

        std::thread::sleep(std::time::Duration::from_millis(100));

        // Stop recording should pause stream
        let samples = capture.stop_recording().unwrap();
        assert!(!capture.is_recording.load(Ordering::Relaxed));

        // Should have captured some samples (even if silent/noise)
        // In a real test environment this verifies stream was active
        let _ = samples;
    }
}
