use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use ringbuf::{traits::*, HeapCons, HeapRb};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::config::AudioConfig;

pub struct AudioCapture {
    stream: Option<cpal::Stream>,
    ring_buffer_consumer: HeapCons<f32>,
    is_recording: Arc<AtomicBool>,
    device_sample_rate: u32,
    device_channels: u16,
}

impl AudioCapture {
    pub fn new(config: &AudioConfig) -> Result<Self> {
        info!("initializing audio capture");

        // Get default input device
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("no input device available")?;

        let device_name = device.name().unwrap_or_else(|_| "unknown".to_string());
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

        // Create ring buffer (buffer_size * 4 for safety margin)
        let ring_buffer_capacity = config.buffer_size * 4;
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

        // Start the stream (but not recording yet)
        stream.play().context("failed to start audio stream")?;
        info!("audio stream started");

        Ok(Self {
            stream: Some(stream),
            ring_buffer_consumer,
            is_recording,
            device_sample_rate,
            device_channels,
        })
    }

    pub fn start_recording(&mut self) -> Result<()> {
        debug!("starting recording");

        // Clear ring buffer
        self.ring_buffer_consumer.clear();

        // Set recording flag
        self.is_recording.store(true, Ordering::Relaxed);

        info!("recording started");
        Ok(())
    }

    pub fn stop_recording(&mut self) -> Result<Vec<f32>> {
        debug!("stopping recording");

        // Clear recording flag
        self.is_recording.store(false, Ordering::Relaxed);

        // Drain ring buffer into Vec
        let mut samples = Vec::new();
        while let Some(sample) = self.ring_buffer_consumer.try_pop() {
            samples.push(sample);
        }

        info!("recorded {} samples", samples.len());

        // Convert to 16kHz mono
        let samples_16khz_mono = self.convert_to_16khz_mono(&samples)?;

        Ok(samples_16khz_mono)
    }

    fn convert_to_16khz_mono(&self, samples: &[f32]) -> Result<Vec<f32>> {
        let target_sample_rate = 16000;

        // Convert stereo to mono if needed
        let mono_samples = if self.device_channels == 1 {
            samples.to_vec()
        } else {
            // Average channels (simple downmix)
            samples
                .chunks(self.device_channels as usize)
                .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
                .collect()
        };

        // Resample if needed
        if self.device_sample_rate == target_sample_rate {
            return Ok(mono_samples);
        }

        // Simple linear interpolation resampling
        let ratio = self.device_sample_rate as f64 / target_sample_rate as f64;
        let output_len = (mono_samples.len() as f64 / ratio).ceil() as usize;

        let mut resampled = Vec::with_capacity(output_len);
        for i in 0..output_len {
            let src_idx = i as f64 * ratio;
            let src_idx_floor = src_idx.floor() as usize;
            let src_idx_ceil = (src_idx_floor + 1).min(mono_samples.len() - 1);
            let fract = src_idx - src_idx_floor as f64;

            let sample = if src_idx_floor < mono_samples.len() {
                let s1 = mono_samples[src_idx_floor];
                let s2 = mono_samples[src_idx_ceil];
                s1 + (s2 - s1) * fract as f32
            } else {
                0.0
            };

            resampled.push(sample);
        }

        info!(
            "resampled {} Hz → 16000 Hz ({} → {} samples)",
            self.device_sample_rate,
            mono_samples.len(),
            resampled.len()
        );

        Ok(resampled)
    }

    /// Save samples to WAV file for debugging
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
