# Phase 3: Audio Recording - Implementation Plan

## Overview
Implement audio capture with lock-free ring buffer for whisper-hotkey macOS app.

## Requirements (from implem-plan.md)
- Set up audio library (CoreAudio FFI or cpal)
- Read buffer_size from config
- Lock-free ring buffer implementation
- Audio accumulator (Vec<f32>)
- Sample rate conversion to 16kHz mono
- **Validation:** Audio captured, save WAV for debug

## Current State Analysis

### Existing Patterns
- **Config**: Already has AudioConfig (buffer_size, sample_rate) ✓
- **State machine**: HotkeyManager has Idle → Recording → Processing with placeholders for audio (lines 61, 79)
- **Error handling**: anyhow::Result + .context() pattern
- **Module structure**: Clean separation (input/, config.rs, telemetry.rs)

### Integration Points
1. `main.rs`: Need to initialize audio capture, pass to hotkey manager
2. `input/hotkey.rs`: Call audio.start_recording() on press, audio.stop_recording() on release
3. New module `audio/`: capture.rs, buffer.rs

## Design Decision: Audio Library

### Option 1: cpal (RECOMMENDED for MVP)
**Pros:**
- Simple high-level API (StreamConfig, build_input_stream)
- Already commented in Cargo.toml
- Meets <50ms target even with +10-20ms overhead (30ms << 50ms)
- Less FFI complexity (aligns with "simplest solution first")

**Cons:**
- +10-20ms latency vs raw CoreAudio
- Abstraction layer

**Verdict**: Start with cpal for Phase 3 MVP. Can optimize to coreaudio-rs in Phase 7 if profiling shows need.

### Option 2: coreaudio-rs (Future optimization)
**Pros:**
- Lower latency (~5-10ms)
- More control

**Cons:**
- Complex FFI (AudioUnit callbacks, unsafe blocks)
- Violates incremental approach for MVP
- macOS-specific (no Windows/Linux future)

## Architecture

### Threading Model
```
Main Thread (tokio)
├── HotkeyManager (state machine)
└── AudioCapture (start/stop control)
    └── Audio Callback Thread (cpal/OS)
        └── Lock-free Ring Buffer (SPSC)
            └── Main Thread reads accumulated samples
```

### Data Flow
1. Hotkey press → `AudioCapture::start_recording()`
2. Audio callback thread → Push samples to ring buffer (lock-free)
3. Hotkey release → `AudioCapture::stop_recording()` → Read all samples from ring buffer → Convert to 16kHz mono → Return Vec<f32>

### Lock-Free Ring Buffer
Use `ringbuf` crate (0.4.8):
- **Type**: HeapRb<f32> with SharedRb (thread-safe SPSC)
- **Size**: buffer_size from config (1024 default) * 4 (safety margin for jitter)
- **Producer**: Audio callback thread (lock-free push)
- **Consumer**: Main thread on stop_recording() (lock-free pop all)

## File Structure
```
src/
├── audio/
│   ├── mod.rs           # pub use capture::AudioCapture
│   ├── capture.rs       # AudioCapture struct, cpal integration
│   └── buffer.rs        # OPTIONAL: wrapper if needed (start with ringbuf directly)
```

## Implementation Steps (Incremental)

### Step 1: Dependencies (5 lines)
Add to Cargo.toml:
```toml
cpal = "0.15"
ringbuf = "0.4"
hound = "3"  # WAV file writing for debug validation
```

### Step 2: Audio Module Structure (20 lines)
- Create src/audio/mod.rs
- Create src/audio/capture.rs with skeleton:
  ```rust
  pub struct AudioCapture {
      stream: Option<cpal::Stream>,
      ring_buffer: Arc<SharedRb<...>>,
      is_recording: Arc<AtomicBool>,
  }

  impl AudioCapture {
      pub fn new(config: &AudioConfig) -> Result<Self>
      pub fn start_recording(&mut self) -> Result<()>
      pub fn stop_recording(&mut self) -> Result<Vec<f32>>
  }
  ```

### Step 3: Ring Buffer Setup (15 lines)
- Initialize HeapRb in AudioCapture::new()
- Split into producer/consumer
- Producer moved into audio callback (Arc clone)

### Step 4: cpal Input Stream (30 lines)
- Get default input device
- Get config (sample_rate from config or device default)
- Build input stream with callback:
  ```rust
  stream = device.build_input_stream(
      &stream_config,
      move |data: &[f32], _: &_| {
          if is_recording.load(Ordering::Relaxed) {
              producer.push_slice(data); // Lock-free
          }
      },
      err_callback,
      None,
  )?;
  ```

### Step 5: Start/Stop Methods (20 lines)
- `start_recording()`: Set is_recording flag, clear ring buffer
- `stop_recording()`: Clear flag, drain ring buffer into Vec<f32>, return

### Step 6: Sample Rate Conversion (25 lines)
- In `stop_recording()`, after draining buffer
- If captured_rate != 16000, use simple linear interpolation
- Convert stereo → mono (average channels if needed)

### Step 7: Integration with HotkeyManager (15 lines)
- Pass Arc<Mutex<AudioCapture>> to HotkeyManager::new()
- In `on_press()`: audio.lock().unwrap().start_recording()?
- In `on_release()`: let samples = audio.lock().unwrap().stop_recording()?

### Step 8: WAV Debug Output (15 lines)
- In main.rs or audio/capture.rs, add helper:
  ```rust
  fn save_wav_debug(samples: &[f32], path: &str) -> Result<()> {
      // Use hound crate
  }
  ```
- Call after stop_recording() for validation

### Step 9: Main.rs Integration (10 lines)
- Create AudioCapture in main()
- Wrap in Arc<Mutex<>>
- Pass to HotkeyManager

**Total**: ~155 lines across 6 files

## Validation Plan
1. Run app, press hotkey for 2 seconds, release
2. Check WAV file created in ~/.whisper-hotkey/debug/recording_TIMESTAMP.wav
3. Verify: 16kHz sample rate, mono, ~2 seconds duration
4. Test edge cases:
   - Quick press/release (<100ms)
   - Long recording (10s, 30s)
   - Rapid press/release cycles

## Error Handling
- Device not found: Return Err with context
- Stream build fails: Return Err (likely permissions issue)
- Buffer overflow (samples dropped): Log warning via tracing, continue

## Performance Targets (Phase 3)
- Audio capture start: <30ms (cpal overhead)
- Ring buffer overhead: <1ms per callback
- Total recording → Vec<f32>: <50ms for 10s audio

## Dependencies
```toml
cpal = "0.15"
ringbuf = "0.4"
hound = "3"  # Debug WAV output
```

## Deferred to Later Phases
- Voice Activity Detection (Phase 7)
- Streaming to Whisper (Phase 4)
- Buffer size tuning (Phase 7 optimization)

## User Decisions ✓

1. **Audio library**: cpal for MVP (coreaudio-rs optimization deferred to Phase 7)
2. **Debug WAV location**: ~/.whisper-hotkey/debug/
3. **Sample rate mismatch**: Use device default + resample (never fail)

## References
- [cpal](https://github.com/RustAudio/cpal)
- [ringbuf](https://docs.rs/ringbuf/0.4.8/ringbuf/)
- [coreaudio-rs](https://github.com/RustAudio/coreaudio-rs) (future optimization)
- [hound](https://docs.rs/hound/) (WAV encoding)

## Risk Mitigation
- **Ring buffer overflow**: Size = buffer_size * 4, log warnings
- **Audio glitches**: Accept for MVP, profile in Phase 7
- **Permission denial**: Already handled in Phase 1 (permissions.rs)
