# Testing

Test strategy and coverage for Whisper Hotkey.

## Test Coverage

**Current:** 62.03% line coverage (163 passing, 46 ignored)

```bash
# View coverage report
cargo llvm-cov
cargo llvm-cov --html && open target/llvm-cov/html/index.html
```

## Running Tests

### Unit Tests (Fast)

```bash
# All unit tests (no hardware required)
cargo test

# Specific module
cargo test config::
cargo test transcription::

# With output
cargo test -- --nocapture
```

### Integration Tests (Hardware Required)

```bash
# All hardware tests (requires mic + permissions)
cargo test -- --ignored

# Specific hardware test
cargo test test_audio_capture_real -- --ignored
```

## Test Types

### Unit Tests

**What:** Pure logic, no external dependencies

**Location:** Same file as code (`#[cfg(test)]` modules)

**Example:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let config = Config::default();
        assert_eq!(config.profiles.len(), 1);
    }
}
```

### Integration Tests

**What:** Tests requiring hardware (mic, accessibility)

**Location:** Same file, marked with `#[ignore]`

**Example:**
```rust
#[test]
#[ignore]  // Requires microphone permission
fn test_audio_capture_real() {
    let capture = AudioCapture::new().unwrap();
    // ...
}
```

**Run with:**
```bash
cargo test -- --ignored
```

## Coverage by Module

| Module | Coverage | Status |
|--------|----------|--------|
| config.rs | 88.07% | ✅ Excellent |
| input/hotkey.rs | 72.31% | ✅ Good |
| audio/capture.rs | 70.28% | ✅ Good |
| transcription/download.rs | 60.87% | ⚠️ Acceptable |
| tray.rs | 58.97% | ⚠️ Acceptable |
| permissions.rs | 55.83% | ⚠️ Acceptable |
| input/cgevent.rs | 54.17% | ⚠️ Acceptable |
| telemetry.rs | 49.40% | ⚠️ Acceptable |
| transcription/engine.rs | 45.26% | ⚠️ Limited (requires model) |
| main.rs | 0.00% | N/A (binary) |

### Why Some Modules Have Lower Coverage

**transcription/engine.rs** (45.26%):
- Requires actual Whisper model file (~75MB)
- Tests marked `#[ignore]` (integration tests)

**tray.rs** (58.97%):
- Requires macOS main thread for Menu/TrayIcon
- Tests marked `#[ignore]` (integration tests)

**main.rs** (0.00%):
- Binary initialization and event loop
- Requires full system integration

## Manual Testing

### Quick Pipeline Test

After building, test full pipeline:

1. **Build:** `cargo build --release`
2. **Run:** `cargo run --release`
3. **Test hotkey:** Press Ctrl+Option+Z
4. **Record:** Hold for 5s, speak clearly
5. **Verify:** Text inserted at cursor

**Expected:** <2.5s total (5s recording + <2s transcription + <100ms insertion)

### Multi-Profile Test

```bash
# Edit config to add second profile
nano ~/.whisper-hotkey/config.toml

# Add:
[[profiles]]
name = "test2"
model_type = "tiny.en"
modifiers = ["Command", "Shift"]
key = "T"
preload = true

# Restart and test both hotkeys
```

### Alias Matching Test

```bash
# Edit config
nano ~/.whisper-hotkey/config.toml

# Add:
[aliases]
enabled = true
threshold = 0.8

[aliases.entries]
"period" = "."
"dot com" = ".com"

# Restart, speak "hello world period"
# Should insert: "hello world."
```

## Performance Testing

### Audio Latency

```bash
RUST_LOG=debug cargo run --release
```

**Look for:** `start_recording: latency_us`

**Target:** <50ms
**Typical:** 5-10ms

### Transcription Speed

```bash
RUST_LOG=debug cargo run --release
```

**Look for:** `transcription_ms`

**Target (base.en, 10s audio, M1):** <2000ms
**Typical:** 1000-1500ms

### Memory Usage

```bash
cargo run --release &
sleep 5
ps aux | grep whisper-hotkey | awk '{print $6/1024 " MB"}'
```

**Target:** <1.5GB
**Typical:** ~1.3GB (with small model preloaded)

## Profiling

### CPU Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Profile
sudo cargo flamegraph --release

# Trigger hotkey, speak, release
# Ctrl+C when done
# Opens flamegraph.svg
```

### Memory Profiling (macOS)

```bash
# Build release
cargo build --release

# Profile with Instruments
instruments -t Allocations target/release/whisper-hotkey

# Trigger hotkey multiple times
# Stop recording
# Analyze allocations
```

### Detailed Logging

```bash
# Trace level (all operations)
RUST_LOG=whisper_hotkey=trace cargo run --release

# Module-specific
RUST_LOG=whisper_hotkey::transcription=trace cargo run --release

# JSON output
RUST_LOG=debug cargo run --release 2>&1 | tee test.log
```

## Test Automation

### Pre-commit Checks

```bash
# Format check
cargo fmt --all -- --check

# Lint check
cargo clippy --all-targets --all-features -- -D warnings

# Unit tests
cargo test

# All checks (via mise)
mise run check
```

### CI Tests (GitHub Actions)

See `.github/workflows/test.yml`:
- Runs on every PR
- Tests on macOS (Intel + Apple Silicon)
- Runs: `cargo fmt`, `cargo clippy`, `cargo test`

## Debugging Tests

### Show Test Output

```bash
cargo test -- --nocapture
```

### Run Single Test

```bash
cargo test test_config_default -- --nocapture
```

### Debug Test with LLDB

```bash
# Build tests
cargo test --no-run

# Find test binary
ls target/debug/deps/whisper_hotkey-*

# Debug
lldb target/debug/deps/whisper_hotkey-xxxxx
(lldb) b config::tests::test_config_default
(lldb) run
```

### Test with Logging

```bash
RUST_LOG=trace cargo test -- --nocapture
```

## Writing Tests

### Unit Test Template

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_feature() {
        // Arrange
        let input = "test";

        // Act
        let result = process(input);

        // Assert
        assert_eq!(result, "expected");
    }
}
```

### Integration Test Template

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]  // Requires hardware/permissions
    fn test_hardware_feature() {
        // Arrange
        let device = Device::new().unwrap();

        // Act
        let result = device.capture();

        // Assert
        assert!(result.is_ok());
    }
}
```

## Known Test Limitations

### Cannot Mock

- CoreAudio (requires real audio device)
- CGEvent (requires Accessibility permission)
- Whisper model (requires model file)
- macOS Menu/TrayIcon (requires main thread)

### Workarounds

1. **Extract pure logic** from FFI code
2. **Use integration tests** for FFI code
3. **Mark as `#[ignore]`** for manual testing
4. **Document** in test comments

## Next Steps

- Read [Contributing](./contributing.md) guidelines
- See [Project Structure](./structure.md)
- Learn about [Building](./building.md)
