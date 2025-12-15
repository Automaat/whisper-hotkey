# Performance Issues

Troubleshooting performance problems and optimization.

## Expected Performance

### Target Metrics (M1, base.en)

| Metric | Target | Typical |
|--------|--------|---------|
| Audio start latency | <50ms | 5-10ms |
| Transcription (10s audio) | <2s | 1-1.5s |
| Text insertion | <100ms | 20-50ms |
| Idle CPU | <1% | 0.5% |
| Idle RAM | ~1.5GB | 1.3GB |

If metrics significantly worse, see solutions below.

## Slow Transcription

### Symptom: Takes >5s for 10s Audio

**Check model:**
```bash
tail ~/.whisper-hotkey/crash.log | grep "model_type"
```

**Solutions:**

1. **Use faster model:**
   ```toml
   [[profiles]]
   model_type = "tiny.en"  # Fastest
   # or
   model_type = "base.en"  # Balanced
   ```

2. **Reduce beam_size:**
   ```toml
   [[profiles]]
   beam_size = 1  # From 5 or 10
   ```

3. **Increase threads:**
   ```toml
   [[profiles]]
   threads = 8  # From 4 (if 8-core CPU)
   ```

4. **Close other apps:**
   - Check Activity Monitor
   - Quit CPU-intensive apps

5. **Check CPU usage:**
   ```bash
   # While transcribing, check:
   top -pid $(pgrep whisper-hotkey)
   ```

### Symptom: Transcription Fast But Total Time Slow

**Check logs:**
```bash
RUST_LOG=debug cargo run --release
```

Look for:
- `transcription_ms`: Should be <2000ms
- `insertion_ms`: Should be <100ms
- `total_pipeline_ms`: Sum of components

**If insertion_ms high:**
- Text insertion is slow
- May be app-specific
- Test in TextEdit first

## High Memory Usage

### Symptom: Using >3GB RAM

**Check preloaded models:**

```bash
# View config
cat ~/.whisper-hotkey/config.toml | grep -A 10 "[[profiles]]"
```

**Solutions:**

1. **Disable preload for unused profiles:**
   ```toml
   [[profiles]]
   preload = false  # Lazy load
   ```

2. **Use smaller models:**
   ```toml
   [[profiles]]
   model_type = "base.en"  # 500MB instead of small (1.3GB)
   ```

3. **Reduce number of profiles:**
   - Remove unused profiles from config

4. **Check for memory leaks:**
   ```bash
   # Monitor memory over time
   while true; do
     ps aux | grep whisper-hotkey | awk '{print $6/1024 " MB"}'
     sleep 10
   done
   ```

   If steadily increasing, report bug.

## High CPU Usage (Idle)

### Symptom: Using >2% CPU When Not Transcribing

**Expected:** ~0.5% CPU idle

**Solutions:**

1. **Check for errors in logs:**
   ```bash
   tail -f ~/.whisper-hotkey/crash.log
   ```

2. **Check for multiple instances:**
   ```bash
   ps aux | grep whisper-hotkey
   # Should see only one instance
   # Kill extras if present
   ```

3. **Reduce recording cleanup frequency:**
   ```toml
   [recording]
   cleanup_interval_hours = 24  # From 1
   ```

4. **Disable recording:**
   ```toml
   [recording]
   enabled = false
   ```

## High Latency

### Audio Start Latency >100ms

**Symptom:** Delay between pressing hotkey and recording start

**Check:**
```bash
RUST_LOG=debug cargo run --release
# Look for "start_recording: latency_us"
```

**Solutions:**

1. **Close CPU-intensive apps:**
   - Check Activity Monitor
   - Free up CPU cycles

2. **Reduce buffer size:**
   ```toml
   [audio]
   buffer_size = 512  # From 1024 (may cause glitches)
   ```

3. **Check system load:**
   ```bash
   uptime
   # Load average should be <4 on 8-core CPU
   ```

### Transcription Latency >5s

See "Slow Transcription" above.

### Text Insertion Latency >500ms

**Symptom:** Long delay between transcription and text appearing

**Check logs:**
```bash
RUST_LOG=debug cargo run --release
# Look for "insertion_ms"
```

**Solutions:**

1. **Test in different app:**
   - Some apps slower than others
   - Try TextEdit first

2. **Check CPU usage:**
   - High CPU may delay insertion

3. **Check for CGEvent issues:**
   ```bash
   # If logs show errors
   # May be app-specific limitation
   ```

## Disk Space Issues

### Model Download Fails

**Symptom:** "No space left on device"

**Check space:**
```bash
df -h ~
```

**Models require:**
- tiny: 75MB
- base: 142MB
- small: 466MB
- medium: 1.5GB
- large: 3GB

**Solutions:**

1. **Free disk space:**
   ```bash
   # Delete old recordings
   rm -rf ~/.whisper-hotkey/recordings/*

   # Delete unused models
   ls ~/.whisper-hotkey/models/
   rm ~/.whisper-hotkey/models/ggml-{unused}.bin
   ```

2. **Use smaller model:**
   ```toml
   [[profiles]]
   model_type = "base.en"  # 142MB instead of small (466MB)
   ```

## Battery Drain

### Symptom: MacBook Battery Drains Fast

**Check if transcribing frequently:**
```bash
# View logs
tail -f ~/.whisper-hotkey/crash.log
```

**Solutions:**

1. **Use smaller model:**
   ```toml
   [[profiles]]
   model_type = "tiny.en"  # Less CPU intensive
   ```

2. **Reduce threads:**
   ```toml
   [[profiles]]
   threads = 2  # From 4
   ```

3. **Disable model preload:**
   ```toml
   [[profiles]]
   preload = false  # Load on demand
   ```

4. **Quit when not using:**
   - Don't leave running 24/7
   - Start when needed

## App Startup Slow

### Symptom: Takes >10s to Start

**Check preloaded models:**
```bash
cat ~/.whisper-hotkey/config.toml | grep "preload"
```

**Each preloaded model adds ~2-3s startup time.**

**Solutions:**

1. **Disable preload for rarely used profiles:**
   ```toml
   [[profiles]]
   preload = false
   ```

2. **Reduce number of profiles:**
   - Keep only frequently used profiles

3. **Use smaller models:**
   - base.en loads faster than small/medium

**Expected startup times:**
- 1 profile (base.en): 2-3s
- 2 profiles (base.en + small): 4-6s
- 3 profiles: 6-9s

## Profiling Performance

### CPU Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Profile
sudo cargo flamegraph --release

# Trigger hotkey several times
# Ctrl+C when done
# Opens flamegraph.svg
```

**Analyze:**
- Look for hot spots (functions taking most time)
- Report if unexpected bottlenecks

### Memory Profiling

```bash
# macOS Instruments
cargo build --release
instruments -t Allocations target/release/whisper-hotkey

# Trigger hotkey several times
# Stop recording
# Analyze memory allocations
```

### Detailed Timing

```bash
# Trace logging
RUST_LOG=whisper_hotkey=trace cargo run --release
```

**Look for spans:**
- `start_recording`
- `stop_recording`
- `convert_to_16khz_mono`
- `transcription`
- `insert_text`

Each shows timing in microseconds or milliseconds.

## Hardware-Specific Issues

### Intel Mac Performance

**Expected:** 2-3x slower than Apple Silicon

**Recommendations:**
```toml
[[profiles]]
model_type = "base.en"  # Don't use small/medium
threads = 4
beam_size = 1
```

**If still slow:**
- Use `tiny.en` model
- Close all other apps
- Consider upgrading to Apple Silicon

### Older Mac Performance

**MacBook Pro 2017-2019 (Intel):**

**Recommendations:**
```toml
[[profiles]]
model_type = "tiny.en"
threads = 2
beam_size = 1
```

**Expected performance:**
- Audio: <50ms
- Transcription (10s): 3-5s
- Total: 3-6s

## Optimization Checklist

### For Speed

- [ ] Use `tiny.en` or `base.en` model
- [ ] Set `beam_size = 1`
- [ ] Set `threads = 8` (if 8-core CPU)
- [ ] Specify `language = "en"`
- [ ] Close other apps
- [ ] Use SSD (not HDD)

### For Memory

- [ ] Use `base.en` (not small/medium)
- [ ] Set `preload = false` for unused profiles
- [ ] Single profile only
- [ ] Disable debug recording

### For Battery

- [ ] Use `tiny.en` model
- [ ] Set `threads = 2`
- [ ] Set `preload = false`
- [ ] Quit when not using

## Reporting Performance Issues

Include in bug report:

1. **Hardware:**
   ```bash
   sysctl -n machdep.cpu.brand_string
   sysctl -n hw.memsize | awk '{print $1/1024/1024/1024 " GB"}'
   ```

2. **macOS version:**
   ```bash
   sw_vers
   ```

3. **Config:**
   ```bash
   cat ~/.whisper-hotkey/config.toml
   ```

4. **Performance logs:**
   ```bash
   RUST_LOG=debug cargo run --release 2>&1 | tee perf.log
   # Trigger hotkey
   # Attach perf.log to issue
   ```

5. **Expected vs actual metrics**

**GitHub Issues:** https://github.com/Automaat/whisper-hotkey/issues

## Next Steps

- See [Performance Tuning](../configuration/performance.md) for optimization
- Check [Model Selection](../configuration/models.md) for alternatives
- Return to [Common Issues](./common-issues.md)
