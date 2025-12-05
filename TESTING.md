# Whisper Hotkey Testing Guide

## Test Coverage

**Current Status:** 61.69% line coverage (163 passing, 46 ignored)

Run coverage report:
```bash
cargo llvm-cov                                    # Terminal summary
cargo llvm-cov --lcov --output-path coverage.lcov # LCOV format
cargo llvm-cov --html && open target/llvm-cov/html/index.html # HTML
```

**Coverage by Module:**
- `config.rs`: 88.07% ✅
- `input/hotkey.rs`: 72.31% ✅
- `audio/capture.rs`: 70.28% ✅
- `transcription/download.rs`: 60.87%
- `tray.rs`: 58.97%
- `permissions.rs`: 55.83%
- `input/cgevent.rs`: 54.17%
- `telemetry.rs`: 49.40%
- `transcription/engine.rs`: 45.26%
- `main.rs`: 0.00% (binary, not unit testable)

**Expected Coverage Gaps:**

Some modules have lower coverage due to external dependencies that cannot be mocked in unit tests:

- **transcription/engine.rs** (180 lines): Requires actual Whisper model file (~75MB). Tests are `#[ignore]`
- **tray.rs** (200 lines): Requires macOS main thread for Menu/TrayIcon creation. Tests are `#[ignore]`
- **main.rs** (210 lines): Binary initialization and event loop, requires full system integration

These account for ~590 lines (40% of gaps). Target realistic coverage: **65-70%** after accounting for integration-only code.

**Phase Completion:**
- ✅ Phase 1: Low-hanging fruit (47% → 60%)
- ✅ Phase 2: Mock infrastructure (60% → 75%)
- ✅ Phase 3: Tray menu logic (75% → 82%)
- ✅ Phase 4: Testability refactoring (58% → 62%)
  - Extracted pure configuration logic from FFI calls
  - tray.rs: Menu configuration now 100% testable
  - engine.rs: Sampling strategy extracted and tested
  - +17 new unit tests covering previously untestable code

---

## Quick Test

After building, test the full pipeline:

1. **Build**: `cargo build --release`
2. **Run**: `cargo run --release`
3. **Test hotkey**: Press Ctrl+Option+Z (or configured hotkey)
4. **Record**: Hold for 5s, speak clearly
5. **Verify**: Text inserted at cursor

Expected latency: <2.5s total (5s recording + <2s transcription + <100ms insertion)

---

## Unit Tests

```bash
# Fast unit tests (no hardware)
cargo test

# Hardware tests (requires mic + permissions)
cargo test -- --ignored
```

Key tests:

- [config.rs:237](src/config.rs#L237) - Config parsing with default optimization parameters
- [engine.rs:327](src/transcription/engine.rs#L327) - Long 30s recordings
- [engine.rs:347](src/transcription/engine.rs#L347) - Optimization param variations
- [engine.rs:374](src/transcription/engine.rs#L374) - Noise handling

---

## Performance Profiling

### Audio Latency

Enable detailed logging:
```bash
RUST_LOG=whisper_hotkey=trace cargo run --release
```

Key metrics (check logs):
- `start_recording`: latency_us <50μs target
- `stop_recording`: total_ms <50ms target
- `convert_to_16khz_mono`: total_us <20ms target
- `transcription`: inference_ms <2000ms for 10s audio

### CPU Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Profile (run app, trigger hotkey, Ctrl+C)
sudo cargo flamegraph --release

# Open flamegraph.svg
open flamegraph.svg
```

Look for:
- Whisper inference (should dominate)
- Audio conversion overhead (should be minimal)
- Unnecessary allocations in hot paths

### Memory Profiling

#### macOS Instruments (recommended)
```bash
# Build release
cargo build --release

# Profile allocations
instruments -t Allocations target/release/whisper-hotkey

# Profile leaks
instruments -t Leaks target/release/whisper-hotkey
```

**Trigger multiple recordings** (10+) while profiling.

Expected memory:
- Idle: ~1.5GB (model loaded)
- During transcription: +50-100MB temporary
- No leaks after recordings complete

#### Valgrind (Linux alternative)
```bash
valgrind --leak-check=full --show-leak-kinds=all target/release/whisper-hotkey
```

---

## Manual Test Cases

### Edge Cases

#### 1. Silence (0-1s)
- Press hotkey, don't speak, release
- Expected: Empty or minimal text

#### 2. Short audio (1-3s)
- "Hello world"
- Expected: Accurate transcription, <1.5s latency

#### 3. Long recording (30s+)
- Read paragraph continuously
- Expected: Complete transcription, <5s latency, no crashes

#### 4. Noise
- Record with background music/TV
- Expected: Partial transcription or gibberish (no crash)

#### 5. Rapid hotkey cycles
- Press/release 10 times quickly (0.1s each)
- Expected: All cycles complete, no deadlocks

#### 6. Concurrent recordings
- Press hotkey during active transcription
- Expected: Graceful handling (queue or reject)

### App Compatibility

Test text insertion in:
- ✓ TextEdit
- ✓ VS Code
- ✓ Chrome (Gmail, Google Docs)
- ✓ Slack
- ✓ Terminal (may fail - secure input mode)
- ✓ Notes
- ✓ Messages

Check logs for insertion failures: `~/.whisper-hotkey/crash.log`

### Config Changes

Edit `~/.whisper-hotkey.toml`:

#### Optimization tuning
```toml
[model]
threads = 8        # Try 2, 4, 8
beam_size = 1      # Try 1 (fast), 5 (balanced), 10 (accurate)
```

Restart app, test 10s recording:
- threads=8, beam_size=1: Fastest (~1s)
- threads=4, beam_size=5: Balanced (~2s)
- threads=4, beam_size=10: Accurate (~3s)

#### Different models
```toml
[model]
name = "tiny"   # or base, small, medium, large
path = "~/.whisper-hotkey/models/ggml-tiny.bin"
```

Restart, verify auto-download + transcription accuracy.

---

## Performance Targets

| Metric | Target | Command |
|--------|--------|---------|
| Audio start | <50μs | `RUST_LOG=trace` logs |
| Transcription (10s) | <2s | `RUST_LOG=info` logs |
| Text insertion | <100ms | Test manually |
| Idle CPU | <1% | Activity Monitor |
| Idle RAM | ~1.5GB | Activity Monitor |
| No leaks | 0 | Instruments/Valgrind |

Failing targets? Check:
1. Config optimization (threads, beam_size)
2. Model size (try smaller)
3. Flamegraph for bottlenecks

---

## Debugging

### App won't start
```bash
# Check config syntax
cat ~/.whisper-hotkey.toml

# Validate manually
cargo run 2>&1 | grep -i error

# Reset config
rm ~/.whisper-hotkey.toml && cargo run
```

### No transcription output
```bash
# Check logs
tail -f ~/.whisper-hotkey/crash.log

# Verify model
ls -lh ~/.whisper-hotkey/models/

# Test directly
cargo test test_transcribe_short_audio -- --ignored --nocapture
```

### Audio not recording
```bash
# Reset mic permissions
tccutil reset Microphone

# Restart app, grant permission

# Verify device
cargo test test_audio_capture_initialization -- --ignored --nocapture
```

### Memory leak
```bash
# Profile with Instruments
instruments -t Leaks target/release/whisper-hotkey

# Check ringbuf not growing
# Run 100 recordings, verify RSS stable
```

---

## CI/CD

GitHub Actions runs on PR/main:
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test` (unit only, no hardware)
- `cargo build --release`

Hardware tests (require manual run):
```bash
cargo test -- --ignored
```

---

## Reporting Issues

Include:
1. macOS version (`sw_vers`)
2. Chip (M1/M2/Intel)
3. Logs (`~/.whisper-hotkey/crash.log`)
4. Config (`~/.whisper-hotkey.toml`)
5. Steps to reproduce
6. Expected vs actual behavior

Example:
```
**macOS**: 14.0 (M1 Pro)
**Config**: threads=4, beam_size=5, model=small
**Issue**: 30s recording crashes with "mutex poisoned"
**Logs**: [attach crash.log]
**Reproduce**: Hold hotkey for 30s, speak continuously
```
