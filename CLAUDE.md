# whisper-hotkey

macOS background app for system-wide voice-to-text via hotkey using local Whisper.

**Stack:** Rust 1.84, whisper-rs, global-hotkey, cpal, CoreAudio, CGEvent, tokio, tracing
**Performance:** <50ms audio start, <2s transcription (10s audio), <100ms insertion, <500MB memory

---

## Project Structure

```
src/
├── main.rs              # Event loop, initialization
├── audio/
│   ├── capture.rs      # CoreAudio FFI, ring buffer, real-time thread
│   └── vad.rs          # Voice activity detection
├── transcription/
│   ├── engine.rs       # Whisper model loading, inference
│   └── queue.rs        # Async queue for audio chunks
├── input/
│   ├── hotkey.rs       # global-hotkey integration
│   └── cgevent.rs      # CGEvent text insertion FFI
├── config.rs           # TOML config (~/.whisper-hotkey.toml)
└── telemetry.rs        # tracing, performance metrics
```

**Config:** `~/.whisper-hotkey.toml`
```toml
[hotkey]
key = "V"
modifiers = ["Command", "Shift"]

[audio]
sample_rate = 16000
chunk_duration_ms = 10000

[model]
path = "/usr/local/share/whisper/ggml-base.en.bin"
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
- Main thread: tokio (event loop, hotkey)
- Audio thread: OS real-time thread (NOT tokio, NO allocations/locks/syscalls)
- Transcription: tokio blocking pool (Whisper is CPU-bound, NOT async)
- Communication: `crossbeam::channel` or `tokio::sync::mpsc`

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

**CoreAudio (real-time thread constraints):**
```rust
audio_unit.set_input_callback(move |args| {
    // CRITICAL: NO heap allocations, NO locks, NO syscalls
    let samples = args.data.as_slice::<f32>();
    ring_buffer.push_slice(samples);  // Lock-free only
    Ok(())
})?;
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

**Permissions:**
- Accessibility (CGEvent + hotkey), Microphone (CoreAudio)
- NO sandboxing (incompatible)
- Check at startup, clear error messages

---

## Project-Specific Context

**Whisper:**
- GGML format (quantized CPU inference)
- base.en = sweet spot (speed/accuracy)
- Preload at startup (2-3s load = critical, NOT per-transcription)
- Synchronous, thread-safe (`Arc`), NO async

**Audio:**
- CoreAudio callback: OS real-time thread, <10ms (prefer <1ms)
- Violations = audio glitches
- Ring buffer for cross-thread (lock-free)

**Text Insertion:**
- CGEvent simulates keyboard
- Some apps block (Terminal secure input) - warn user, no fallback

**Known Issues:**
- Model must exist at config path (validate at startup)
- App crashes if permissions denied (graceful handling needed)
- Reset permissions: `tccutil reset Microphone`

**Integration:**
- whisper-rs: Blocking, thread-safe, run in blocking pool
- global-hotkey: Event-based callback
- cpal: Fallback if CoreAudio too complex (+10-20ms latency trade-off)

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

- [whisper.cpp](https://github.com/ggerganov/whisper.cpp), [whisper-rs](https://github.com/tazz4843/whisper-rs)
- [core-graphics-rs](https://github.com/servo/core-graphics-rs), [coreaudio-rs](https://github.com/RustAudio/coreaudio-rs)
- [ringbuf](https://docs.rs/ringbuf/), [Tokio](https://tokio.rs/tokio/tutorial)
- [Whisper models](https://huggingface.co/ggerganov/whisper.cpp)

---

## Maintenance

**Update when:** Threading changes, new FFI, performance targets shift, PR mistakes, new macOS APIs
**Keep concise:** <500 lines, whisper-hotkey-specific, no general Rust advice
**Review monthly:** Targets relevant? Update from PR feedback? New anti-patterns?
