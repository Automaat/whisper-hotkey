# Performance Tuning

Optimize Whisper Hotkey for your speed/accuracy requirements and hardware.

## Performance Metrics

### Target Performance

| Metric | Target | Typical (M1, base.en) |
|--------|--------|----------------------|
| Audio start | <50ms | ~5-10ms |
| Transcription (10s) | <2s | ~1-1.5s |
| Text insertion | <100ms | ~20-50ms |
| Idle CPU | <1% | ~0.5% |
| Idle RAM | ~1.5GB | ~1.3GB |

### Measuring Performance

Enable debug logging to see metrics:

```bash
RUST_LOG=debug cargo run --release
```

Look for performance spans:

```
📼 Captured 3.5s audio (56000 samples)
  capture_latency_ms: 8.2
  transcription_ms: 1450.3
  insertion_ms: 32.1
✨ Transcription: "Hello world"
  total_pipeline_ms: 1490.6
```

## Performance Factors

### 1. Model Selection

**Biggest impact on speed:**

| Model | Speed (10s audio) | Accuracy |
|-------|-------------------|----------|
| tiny.en | ~0.5s | ⭐⭐ |
| **base.en** | ~1s | ⭐⭐⭐ |
| small.en | ~2s | ⭐⭐⭐⭐ |
| medium | ~6s | ⭐⭐⭐⭐⭐ |

**Recommendation:** Use `base.en` for balanced performance.

```toml
[[profiles]]
model_type = "base.en"
```

### 2. Thread Count

**Controls CPU usage during transcription:**

```toml
[[profiles]]
threads = 4  # or 2, 8, etc.
```

**Recommendations:**

| CPU | Threads | Notes |
|-----|---------|-------|
| M1/M2/M3 (8 cores) | `4` | Balanced (default) |
| M1/M2/M3 (8 cores) | `8` | Max speed (+20-30%) |
| Intel (4 cores) | `4` | Max available |
| Intel (2 cores) | `2` | Max available |

**Diminishing returns:**
- 1→2 threads: ~80% faster
- 2→4 threads: ~60% faster
- 4→8 threads: ~20% faster
- 8→16 threads: ~5% faster

**Trade-off:**
- More threads = faster transcription
- More threads = higher CPU usage
- More threads = less responsive UI during transcription

### 3. Beam Size

**Controls accuracy vs speed trade-off:**

```toml
[[profiles]]
beam_size = 1  # or 5, 10
```

**Performance impact:**

| beam_size | Speed | Accuracy |
|-----------|-------|----------|
| `1` | 1x (fast) | Good |
| `5` | 5x slower | Better |
| `10` | 10x slower | Best |

**Examples (base.en, 10s audio, M1):**
- `beam_size = 1`: ~1s
- `beam_size = 5`: ~5s
- `beam_size = 10`: ~10s

**Recommendation:** Use `1` unless accuracy is critical.

### 4. Language Setting

**Auto-detect adds overhead:**

```toml
[[profiles]]
language = "en"  # Specify language (recommended)
# OR
language = null  # Auto-detect (+200ms overhead)
```

**Performance impact:**
- Specified language: 0ms overhead
- Auto-detect: +200ms overhead

**Recommendation:** Always specify language if known.

### 5. Model Preloading

**Load model at startup vs on-demand:**

```toml
[[profiles]]
preload = true   # Load at startup (recommended)
# OR
preload = false  # Load on first use
```

**Impact:**
- `preload = true`: 2-3s startup delay, instant first transcription
- `preload = false`: Fast startup, 2-3s delay on first transcription

**Recommendation:** Preload models you use frequently.

## Optimization Strategies

### Strategy 1: Maximum Speed

**For quick dictation:**

```toml
[[profiles]]
name = "fast"
model_type = "tiny.en"
modifiers = ["Control", "Option"]
key = "Z"
preload = true
threads = 8
beam_size = 1
language = "en"
```

**Performance:**
- ~0.5s for 10s audio
- Lower accuracy (acceptable for casual use)

### Strategy 2: Balanced (Default)

**For everyday use:**

```toml
[[profiles]]
name = "balanced"
model_type = "base.en"
modifiers = ["Control", "Option"]
key = "Z"
preload = true
threads = 4
beam_size = 1
language = "en"
```

**Performance:**
- ~1s for 10s audio
- Good accuracy

### Strategy 3: Maximum Accuracy

**For important documents:**

```toml
[[profiles]]
name = "accurate"
model_type = "small.en"
modifiers = ["Command", "Shift"]
key = "V"
preload = true
threads = 4
beam_size = 5
language = "en"
```

**Performance:**
- ~10s for 10s audio
- Excellent accuracy

### Strategy 4: Multi-Profile

**Switch between fast and accurate:**

```toml
# Daily use - fast
[[profiles]]
name = "fast"
model_type = "base.en"
modifiers = ["Control", "Option"]
key = "Z"
preload = true
threads = 4
beam_size = 1
language = "en"

# Important docs - accurate
[[profiles]]
name = "accurate"
model_type = "small.en"
modifiers = ["Command", "Shift"]
key = "V"
preload = true
threads = 4
beam_size = 5
language = "en"
```

## Hardware Considerations

### Apple Silicon (M1/M2/M3)

**Advantages:**
- High single-core performance
- Efficient power usage
- Large unified memory

**Recommended settings:**
```toml
threads = 4  # or 8 for max speed
beam_size = 1
```

**Performance (base.en, 10s audio):**
- M1: ~1-1.5s
- M2: ~0.8-1.2s
- M3: ~0.7-1s

### Intel Mac

**Considerations:**
- Lower single-core performance
- More heat generation

**Recommended settings:**
```toml
threads = 4  # Don't exceed core count
beam_size = 1
model_type = "base.en"  # Avoid medium/large
```

**Performance (base.en, 10s audio):**
- Intel i5 (4 cores): ~2-3s
- Intel i7 (4-8 cores): ~1.5-2.5s
- Intel i9 (8+ cores): ~1-2s

### Memory Constraints

**If RAM limited (<8GB):**

1. **Use smaller models:**
   ```toml
   model_type = "base.en"  # Not small/medium
   ```

2. **Disable preload:**
   ```toml
   preload = false  # Lazy load
   ```

3. **Single profile:**
   - Don't use multiple profiles
   - Reduces memory by ~1-2GB per profile

**Memory usage by model:**
- tiny.en: ~350MB
- base.en: ~500MB
- small.en: ~1.3GB
- medium: ~3.5GB

## Profiling

### CPU Profiling

```bash
# Install cargo-flamegraph
cargo install flamegraph

# Profile transcription
sudo cargo flamegraph --release

# Trigger hotkey, speak, release, then Ctrl+C
# Opens flamegraph.svg showing CPU hotspots
```

### Memory Profiling

**macOS Instruments:**

```bash
# Build release binary
cargo build --release

# Profile with Instruments
instruments -t Allocations target/release/whisper-hotkey

# Trigger hotkey, analyze memory usage
```

### Detailed Logging

**Trace level (all operations):**

```bash
RUST_LOG=whisper_hotkey=trace cargo run --release
```

**Module-specific:**

```bash
RUST_LOG=whisper_hotkey::transcription=trace cargo run --release
```

**JSON output (for analysis):**

```bash
RUST_LOG=debug cargo run --release 2>&1 | tee performance.log
```

## Performance Debugging

### Slow Transcription

**Symptom:** Transcription takes >5s for 10s audio (base.en)

**Check:**

1. **CPU usage:**
   - Activity Monitor → Check CPU %
   - Close CPU-intensive apps

2. **Model:**
   ```bash
   ls -lh ~/.whisper-hotkey/models/
   # Verify correct model is loaded
   ```

3. **Settings:**
   ```toml
   # Check these aren't too high:
   beam_size = 1  # Not 5 or 10
   threads = 4    # Appropriate for CPU
   ```

4. **Debug logs:**
   ```bash
   RUST_LOG=debug cargo run --release
   # Look for "transcription_ms" metric
   ```

### High Memory Usage

**Symptom:** App uses >3GB RAM

**Causes:**

1. **Multiple preloaded models:**
   ```bash
   # Disable preload for unused profiles
   preload = false
   ```

2. **Large model:**
   ```toml
   # Use smaller model
   model_type = "base.en"  # Instead of medium/large
   ```

3. **Memory leak (report bug):**
   ```bash
   # Profile with Instruments
   instruments -t Allocations target/release/whisper-hotkey
   ```

### High CPU Usage (Idle)

**Symptom:** App uses >1% CPU when not transcribing

**Expected:** ~0.5% CPU idle (hotkey monitoring)

**If higher:**

1. **Check logs for errors:**
   ```bash
   tail -f ~/.whisper-hotkey/crash.log
   ```

2. **Multiple instances running:**
   ```bash
   ps aux | grep whisper-hotkey
   # Kill duplicates if any
   ```

3. **Recording cleanup:**
   ```toml
   # Reduce cleanup frequency
   cleanup_interval_hours = 24  # From 1
   ```

### Audio Latency

**Symptom:** Delay between pressing hotkey and recording start

**Target:** <50ms

**Typical:** ~5-10ms

**If higher:**

1. **Check buffer size:**
   ```toml
   [audio]
   buffer_size = 1024  # Not too large
   ```

2. **Check CPU load:**
   - Close other apps
   - Check Activity Monitor

3. **Debug logs:**
   ```bash
   RUST_LOG=debug cargo run --release
   # Look for "capture_latency_ms"
   ```

## Benchmarking

### Transcription Speed Test

```bash
# Run with trace logging
RUST_LOG=trace cargo run --release

# Perform 5 transcriptions of different lengths
# Record transcription_ms for each

# Calculate average speed
```

### Audio Capture Latency

```bash
RUST_LOG=debug cargo run --release

# Press hotkey, note timestamp
# Look for "Hotkey pressed" log with timestamp
# Calculate latency from press to log
```

### Memory Footprint

```bash
# Start app
cargo run --release &

# Wait for startup
sleep 5

# Check memory
ps aux | grep whisper-hotkey | awk '{print $6/1024 " MB"}'
```

## Tips for Best Performance

1. **Use English-only models** (`.en`) for English transcription
2. **Specify language explicitly** (`language = "en"`)
3. **Keep beam_size = 1** unless accuracy is critical
4. **Match threads to CPU cores** (4 for most Macs)
5. **Preload frequently used models**
6. **Close CPU-intensive apps** during transcription
7. **Use SSD** (not HDD) for model storage
8. **Keep macOS updated** (better hardware drivers)

## Expected Performance by Hardware

### M3 (2024)

```
Model: base.en, threads=4, beam_size=1
- 5s audio: ~0.5s
- 10s audio: ~0.8s
- 30s audio: ~2.5s
```

### M2 (2022)

```
Model: base.en, threads=4, beam_size=1
- 5s audio: ~0.6s
- 10s audio: ~1s
- 30s audio: ~3s
```

### M1 (2020)

```
Model: base.en, threads=4, beam_size=1
- 5s audio: ~0.7s
- 10s audio: ~1.2s
- 30s audio: ~3.5s
```

### Intel i7 (2019)

```
Model: base.en, threads=4, beam_size=1
- 5s audio: ~1.5s
- 10s audio: ~2.5s
- 30s audio: ~7s
```

## Next Steps

- Choose optimal [Model](./models.md) for your needs
- Configure [Audio Settings](./audio.md) for quality
- See [Configuration Reference](./reference.md) for all options
