# Audio Settings

Configure audio capture for optimal quality and performance.

## Audio Configuration

Edit `~/.whisper-hotkey/config.toml`:

```toml
[audio]
buffer_size = 1024   # Ring buffer size in samples
sample_rate = 16000  # Sample rate in Hz (Whisper requirement)
```

## Buffer Size

### What It Does

`buffer_size` controls the ring buffer used for audio capture:
- Larger buffer = more reliable (less chance of glitches)
- Smaller buffer = lower latency

### Default: `1024`

Good balance for most use cases.

### Recommendations

| Use Case | Buffer Size | Trade-off |
|----------|-------------|-----------|
| Normal use | `1024` | Balanced (default) |
| High CPU load | `2048` | More reliable, +10ms latency |
| Low latency | `512` | Lower latency, may glitch |

### When to Increase

Increase to `2048` if you experience:
- Audio glitches (pops, clicks)
- Dropped samples warnings in logs
- High CPU usage from other apps

```toml
[audio]
buffer_size = 2048
```

### When to Decrease

Decrease to `512` if you need:
- Lowest possible latency
- Fast audio start response
- You have low CPU usage

```toml
[audio]
buffer_size = 512
```

**Warning:** May cause audio glitches on slower Macs.

## Sample Rate

### What It Does

`sample_rate` controls audio capture frequency (samples per second).

### Default: `16000` (16kHz)

This is a **Whisper requirement** - do not change.

### Why 16kHz?

- Whisper models are trained on 16kHz audio
- Lower sample rate = smaller data = faster processing
- 16kHz is sufficient for speech (human voice: ~80Hz-12kHz)

### Can I Change It?

**No.** Changing sample rate will cause transcription errors or crashes.

If you need different sample rate for testing:
- Audio is automatically resampled to 16kHz before Whisper

## Audio Capture Pipeline

### How It Works

1. **macOS CoreAudio** captures at `sample_rate` (16kHz)
2. **Ring buffer** stores samples (size: `buffer_size`)
3. **Main thread** reads samples when hotkey released
4. **Resampling** (if needed) converts to 16kHz mono f32
5. **Whisper** processes 16kHz audio

### Real-Time Thread

Audio capture runs on CoreAudio's real-time thread:
- **High priority** (preempts other tasks)
- **<10ms latency** required (preferably <1ms)
- **No allocations** allowed (crashes if allocate)
- **No locks** allowed (causes glitches)

This is why buffer size matters - must handle bursts of audio data.

## Audio Quality

### Input Source

Whisper Hotkey uses the **default system microphone**.

To change input source:
1. System Settings → Sound → Input
2. Select preferred microphone
3. Restart Whisper Hotkey

### Supported Formats

**Input:**
- 16kHz mono (preferred)
- Any sample rate (auto-resampled to 16kHz)
- Mono or stereo (converted to mono)

**Output to Whisper:**
- Always 16kHz mono f32

### Improving Audio Quality

#### 1. Use Good Microphone

**Built-in Mac microphone:**
- ✅ Sufficient for most use
- ✅ Always available
- ⚠️ Sensitive to keyboard noise
- ⚠️ Less directional

**External USB microphone:**
- ✅ Better quality
- ✅ More directional (less background noise)
- ✅ Better signal-to-noise ratio

**Recommendations:**
- Blue Yeti
- Audio-Technica ATR2100
- Rode NT-USB

#### 2. Microphone Position

**Built-in Mac mic:**
- Speak facing keyboard
- 12-18 inches from mouth
- Avoid covering speaker grilles

**External mic:**
- 6-12 inches from mouth
- Slightly off-axis (reduces plosives)
- Use pop filter if available

#### 3. Environment

**Ideal environment:**
- Quiet room (no background noise)
- Soft surfaces (reduces echo)
- No fans or HVAC noise

**To reduce background noise:**
- Close windows
- Turn off fans
- Mute notifications
- Use noise-canceling mic

#### 4. Speaking Technique

**Best results:**
- Speak clearly at normal pace
- Don't shout or whisper
- Pause briefly at sentence boundaries
- Wait 0.5s after pressing hotkey

## Debugging Audio Issues

### Check Audio Levels

```bash
# Run with debug logging
RUST_LOG=debug cargo run --release
```

Look for audio capture metrics:
```
📼 Captured 3.5s audio (56000 samples)
  Peak amplitude: 0.42
  RMS level: 0.18
```

**Good levels:**
- Peak: 0.3-0.7 (avoid clipping at 1.0)
- RMS: 0.1-0.3 (sufficient signal)

**Too quiet:**
- Peak < 0.1
- Increase microphone volume in System Settings

**Too loud:**
- Peak > 0.9 (clipping)
- Decrease microphone volume

### Check for Glitches

Enable trace logging:

```bash
RUST_LOG=whisper_hotkey=trace cargo run --release
```

Look for warnings:
```
⚠️  Ring buffer overflow (dropped 128 samples)
⚠️  Audio glitch detected (discontinuity)
```

**Solutions:**
- Increase `buffer_size` to `2048`
- Close CPU-intensive apps
- Check Activity Monitor for high CPU usage

### Save Debug Recordings

Recordings are automatically saved to `~/.whisper-hotkey/recordings/`:

```bash
ls -lh ~/.whisper-hotkey/recordings/
```

Play recordings to verify quality:

```bash
afplay ~/.whisper-hotkey/recordings/recording-*.wav
```

## Performance Impact

### Buffer Size Impact

| Buffer Size | Latency | CPU Usage | Reliability |
|-------------|---------|-----------|-------------|
| 512 | ~32ms | Low | May glitch |
| 1024 | ~64ms | Low | Good |
| 2048 | ~128ms | Low | Excellent |
| 4096 | ~256ms | Low | Maximum |

**Note:** Latency is buffer duration, actual perceived latency is lower.

### CPU Usage

Audio capture is very efficient:
- **Idle:** 0% CPU (no audio running)
- **Recording:** <0.5% CPU (on M1/M2)
- **Real-time thread:** <1ms per callback

### Memory Usage

**Ring buffer memory:**
- `buffer_size = 1024`: ~4KB
- `buffer_size = 2048`: ~8KB
- `buffer_size = 4096`: ~16KB

Negligible compared to Whisper model memory (~1.3GB).

## Advanced Configuration

### Custom Audio Backend

By default, uses CoreAudio (macOS native).

Alternative: `cpal` (cross-platform, slightly higher latency).

To switch (requires recompile):
```rust
// In src/audio/capture.rs
// Change from CoreAudio to cpal
```

### Multiple Audio Inputs

Currently only supports default system microphone.

For multiple inputs (future feature), would require:
- Profile-specific input device selection
- Device enumeration at startup

## Troubleshooting

### "No input device available"

**Cause:** Microphone permission not granted

**Solution:**
```bash
# Grant permission
# System Settings → Privacy & Security → Microphone

# Reset permission if needed
tccutil reset Microphone
```

### Audio cutting off early

**Symptom:** Recording stops before releasing hotkey

**Cause:** Buffer overflow

**Solution:**
```toml
[audio]
buffer_size = 2048  # Increase from 1024
```

### Audio sounds distorted

**Symptom:** Crackling or popping in recordings

**Causes:**
1. **Clipping** (too loud)
   - Lower microphone volume
2. **Buffer underrun** (too small buffer)
   - Increase `buffer_size`
3. **CPU overload**
   - Close other apps

### No audio captured

**Symptom:** 0 samples captured

**Check:**
1. Microphone permission granted
2. Correct input device selected (System Settings)
3. Microphone not muted
4. Test mic in other apps (Voice Memos, FaceTime)

## Next Steps

- Choose optimal [Model](./models.md) for your use case
- Tune [Performance Settings](./performance.md)
- See [Common Issues](../troubleshooting/common-issues.md)
