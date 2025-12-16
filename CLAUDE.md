# whisper-hotkey

macOS background app for system-wide voice-to-text via hotkey using local Whisper.

**Stack:** Rust 1.84, whisper-rs, global-hotkey, cpal, CoreAudio, CGEvent, tokio, tracing
**Performance:** <50ms audio start, <2s transcription (10s audio), <100ms insertion, <500MB memory

---

## Project Structure

```
src/
├── main.rs                # Event loop, initialization, NSApplication
├── audio/
│   ├── mod.rs
│   └── capture.rs        # cpal audio capture, ring buffer
├── transcription/
│   ├── mod.rs
│   ├── engine.rs         # Whisper model loading, inference, multi-profile
│   └── download.rs       # Model download from HuggingFace
├── input/
│   ├── mod.rs
│   ├── hotkey.rs         # global-hotkey integration, multi-profile
│   └── cgevent.rs        # CGEvent text insertion FFI
├── tray.rs               # Menubar tray icon with state display
├── alias.rs              # Fuzzy text matching for alias replacement
├── recording_cleanup.rs  # Debug recording retention management
├── permissions.rs        # Microphone + Accessibility + Input Monitoring
├── config.rs             # TOML config (~/.whisper-hotkey/config.toml)
└── telemetry.rs          # tracing, performance metrics
```

**Config:** `~/.whisper-hotkey/config.toml`
```toml
# Multi-profile support - each profile has its own hotkey and model
[[profiles]]
name = "Fast"
model_type = "small"
[profiles.hotkey]
modifiers = ["Control", "Option"]
key = "Z"
preload = true
threads = 4
beam_size = 1
language = "en"

[[profiles]]
name = "Accurate"
model_type = "medium"
[profiles.hotkey]
modifiers = ["Command", "Shift"]
key = "V"
threads = 4
beam_size = 5
language = "en"

[audio]
sample_rate = 16000
buffer_size = 1024

[telemetry]
enabled = true
log_path = "~/.whisper-hotkey/crash.log"

[recording]
enabled = true  # Save debug recordings
retention_days = 7
max_count = 100

[aliases]
enabled = true
threshold = 0.85  # Fuzzy match threshold (0.0-1.0)
[aliases.entries]
"my email" = "user@example.com"
"my address" = "123 Main St, City, State 12345"
```

---

## Development Workflow

**Before coding:**
1. Ask clarifying questions until 95% confident
2. Search for existing patterns
3. Create plan for threading/FFI changes
4. Work incrementally (20-50 line changes for concurrent code)

**Use Plan Mode (Shift+Tab twice) for:**
- Threading changes (audio/transcription/main coordination)
- FFI boundaries (CoreAudio, CGEvent)
- Performance optimizations
- Lock-free data structures

**Testing (TDD preferred):**
- Unit: `#[test]` for pure logic
- Integration: `#[test] #[ignore]` for hardware (manual: `cargo test -- --ignored`)
- Before commit: Test hotkey + 5s/10s/30s audio + insertion in 3 apps

---

## Rust Conventions

**Style:** `cargo fmt`, `cargo clippy` (default), snake_case, 4 spaces, 100 char lines

**Error Handling:**
- Application code: `anyhow::Result` with `.context()`
- Library modules: `thiserror` (audio::CaptureError, transcription::ModelError)
- Early returns preferred

```rust
// Library module
#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("failed to initialize CoreAudio device")]
    DeviceInit(#[from] coreaudio::Error),
}

// App code
fn setup_audio() -> Result<AudioCapture> {
    AudioCapture::new().context("failed to setup audio capture")?
}
```

**Unsafe Policy:**
- ONLY for: FFI (CoreAudio, CGEvent), lock-free data structures, profiled hot paths
- ALWAYS document safety invariants
- Minimize unsafe scope

```rust
/// # Safety: `device_ptr` valid for callback duration
unsafe fn audio_callback(device_ptr: *mut AudioDevice) {
    let device = &*device_ptr;
    // Minimal unsafe block, safe code continues...
}
```

**Async Boundaries:**
- Main thread: tokio (event loop, hotkey, tray events)
- Audio thread: cpal callback thread (minimize allocations/locks)
- Transcription: tokio blocking pool (Whisper is CPU-bound, NOT async)
- Communication: `tokio::sync::mpsc` or `Arc<Mutex<T>>`

**Concurrency:**
- Audio thread: Lock-free only (ring buffer, atomics)
- Main/transcription: Standard primitives OK (`Arc<Mutex<T>>`, channels)

**Linter Errors:**

**ALWAYS:**

- Attempt to fix linter errors properly
- Research solutions online if unclear how to fix
- Fix root cause, not symptoms

**NEVER:**

- Use skip/disable directives (e.g., `// eslint-disable`, `# noqa`, `//nolint`)
- Ignore linter warnings
- Work around linter errors

**If stuck:**

- Try fixing the error
- Research online for proper solution
- If still unclear after research, ASK what to do (don't skip/disable)

---

## Simplicity Principles

**❌ NEVER:**
- Over-abstract threading (3 threads sufficient)
- Trait hierarchies for single implementations
- Async for CPU-bound work (Whisper)
- Plugin systems for "future models" (YAGNI)
- Config for every constant (start with 5-10 key settings)
- Generate entire files (incremental 20-50 lines)
- Placeholders (`// ... rest of code ...`)
- Optimize before profiling
- Helper functions for one-time operations

**✅ ALWAYS:**
- Simplest solution first (direct FFI > abstraction layers)
- Three similar lines > premature abstraction
- Profile before optimizing (`cargo flamegraph`, `Instruments.app`)
- Touch only files for current task
- Search codebase for existing patterns
- Complete code (no placeholders)

**Pattern Drift Threats:**
- **Audio:** Don't add buffering layers (keep direct: CoreAudio → ring buffer → queue)
- **Threading:** Don't async CPU-bound work (Whisper = blocking pool, audio = OS thread)
- **Config:** Don't expose every constant (start minimal)
- **Errors:** Don't wrap excessively (use `?`, sparse `.context()`)

**Complexity Check (before implementing):**
1. Can this be simpler?
2. Abstractions needed NOW (not future)?
3. Similar code exists?
4. Minimal change?
5. Profiled first?

If unsure: STOP, ask for approval.

---

## Code Generation Rules

**ALWAYS:** Incremental changes (20-50 lines), complete code, TDD when possible, profile before optimize, document unsafe, early returns

**NEVER:** Entire files at once, >100 lines/response, placeholders, optimize without profiling, async for CPU-bound, unsafe without justification

**Incremental Process (threading/FFI):**
1. Document design
2. Define types/interfaces
3. Implement one thread (minimal)
4. Add synchronization
5. Test in isolation
6. Integrate incrementally
7. Profile

Review/approve each step before next.

---

## Common Commands

```bash
# Setup
mise install

# Build/run (requires Microphone + Accessibility permissions)
cargo build
cargo run

# Test
cargo test                    # Unit only
cargo test -- --ignored       # Hardware (manual)

# Lint/format
cargo clippy
cargo fmt

# Profile
cargo flamegraph --bin whisper-hotkey
instruments -t "Allocations" target/release/whisper-hotkey
RUST_LOG=whisper_hotkey=trace cargo run
```

**Git:** Feature branches (`feat/`, `fix/`), conventional commits, **`-s -S` flags REQUIRED**

---

## macOS-Specific Patterns

**Audio Capture (cpal):**
```rust
let stream = device.build_input_stream(
    &config,
    move |data: &[f32], _: &_| {
        // Minimize allocations/locks in audio thread
        ring_buffer.push_slice(data);  // Lock-free preferred
    },
    error_callback,
    None,
)?;
```

**CGEvent Text Insertion:**
```rust
fn insert_text(text: &str) -> Result<()> {
    let event = unsafe { CGEvent::new_keyboard_event(None, 0, true)? };
    unsafe {
        event.set_string(text);
        event.post(CGEventTapLocation::HID);
    }
    Ok(())
}
```

**NSApplication (required for global-hotkey):**
```rust
#[cfg(target_os = "macos")]
unsafe {
    let app = NSApp();
    app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);
}
```

**Tray Icon (retina support):**
```rust
// Detect display scale for proper icon sizing
let scale = NSScreen::backingScaleFactor(screen);
let icon_size = if scale >= 2.0 { 32 } else { 16 };
```

**Permissions:**
- Microphone (audio capture)
- Accessibility (CGEvent + global-hotkey)
- Input Monitoring (hotkey detection)
- NO sandboxing (incompatible with system APIs)
- Check at startup, provide clear instructions for quarantine removal

---

## Project-Specific Context

**Whisper:**
- GGML format (quantized CPU inference)
- small = default (good speed/accuracy balance)
- Multi-profile support: different models per hotkey
- Preload at startup (2-3s load = critical, NOT per-transcription)
- Synchronous, thread-safe (`Arc`), NO async

**Audio:**
- cpal for cross-platform audio input
- Ring buffer for thread communication (lock-free preferred)
- Minimize allocations in audio callback

**Text Insertion:**
- CGEvent simulates keyboard
- Some apps block (Terminal secure input) - warn user, no fallback

**Tray Icon:**
- Three states: Idle (adaptive), Recording (red), Processing (yellow)
- Displays all profiles with hotkeys
- Retina display detection for proper icon sizing
- Menu rebuilt on state changes (macOS set_icon bug workaround)

**Aliases:**
- Fuzzy matching using Jaro-Winkler similarity
- Case-insensitive matching
- Configurable threshold (default 0.85)
- Best match selection when multiple aliases match

**Recording Cleanup:**
- Debug recordings saved to `~/.whisper-hotkey/debug/`
- Automatic cleanup based on age and count
- Configurable retention policy

**Known Issues:**
- Model must exist at config path (auto-downloaded on first run)
- macOS quarantine blocks permissions (provide `xattr -d` command)
- Reset permissions: `tccutil reset Microphone`, `tccutil reset Accessibility`

**Integration:**
- whisper-rs: Blocking, thread-safe, run in blocking pool
- global-hotkey: Event-based callback, requires NSApplication
- cpal: Audio capture (simpler than CoreAudio FFI)
- tray-icon: Menubar integration
- strsim: Fuzzy matching for aliases

---

## Performance Optimization

**Profile FIRST:**
1. `cargo flamegraph` (CPU bottlenecks)
2. `Instruments.app` (memory allocations)
3. `tracing` spans (latencies)
4. Release build: `cargo build --release`

**Known Opportunities:**
- Ring buffer size: Balance latency/reliability (start 1s = 16000 samples)
- Quantized models (GGML Q4/Q5)
- Async text insertion (don't block transcription)
- Background model preloading during startup

**When NOT to optimize:**
- Targets already met (<50ms audio, <2s transcription, <500MB memory)
- No user-reported issues

Measure, don't guess.

---

## Additional Resources

- **Documentation:** https://automaat.github.io/whisper-hotkey/ (mdBook site)
- **Whisper:** [whisper.cpp](https://github.com/ggerganov/whisper.cpp), [whisper-rs](https://github.com/tazz4843/whisper-rs)
- **Models:** [Whisper models](https://huggingface.co/ggerganov/whisper.cpp)
- **macOS APIs:** [core-graphics-rs](https://github.com/servo/core-graphics-rs), [cocoa-rs](https://github.com/servo/core-foundation-rs)
- **Audio:** [cpal](https://github.com/RustAudio/cpal), [ringbuf](https://docs.rs/ringbuf/)
- **UI:** [tray-icon](https://github.com/tauri-apps/tray-icon), [global-hotkey](https://github.com/tauri-apps/global-hotkey)
- **Async:** [Tokio](https://tokio.rs/tokio/tutorial)

---

## Maintenance

**Update when:** Threading changes, new FFI, performance targets shift, PR mistakes, new macOS APIs
**Keep concise:** <500 lines, whisper-hotkey-specific, no general Rust advice
**Review monthly:** Targets relevant? Update from PR feedback? New anti-patterns?
