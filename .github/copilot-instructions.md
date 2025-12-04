# Code Review Instructions

## Project Context

**Stack:**
- Language: Rust 1.84 (edition 2021, MSRV 1.75)
- Async Runtime: tokio
- Audio: cpal, CoreAudio FFI
- ML: whisper-rs (Whisper.cpp bindings)
- Platform: macOS only (Core Graphics, Accessibility APIs)
- Logging: tracing + tracing-subscriber

**Purpose:**
macOS voice-to-text app - hold hotkey → speak → release → text inserted at cursor. 100% local, privacy-first.

**Core Modules:**
- `main.rs`: Event loop, initialization, shutdown
- `audio/`: CoreAudio capture, ring buffer, real-time thread
- `transcription/`: Whisper model loading, inference, async queue
- `input/`: Global hotkey, CGEvent text insertion (FFI)
- `config.rs`: TOML config (~/.whisper-hotkey.toml)
- `telemetry.rs`: tracing, performance metrics

**Conventions:**
- Error Handling: `anyhow::Result` + `.context()` in app code, `thiserror` in library modules
- Unsafe: ONLY for FFI (CoreAudio, CGEvent), lock-free, profiled hot paths - always document safety
- Async: tokio runtime, async for I/O, sync for compute (Whisper)
- Testing: Unit tests + hardware tests (`#[ignore]`, run with `--ignored`)
- Formatting: rustfmt (4 spaces, 100 char lines, snake_case)

**Performance Targets:**
- Audio start: <50ms (actual: ~5-10ms)
- Transcription (10s audio): <2s (actual: ~1.5-2s)
- Text insertion: <100ms (actual: ~20-50ms)
- Idle CPU: <1%
- Idle RAM: ~1.5GB

**Critical Areas (Extra Scrutiny):**
- FFI boundaries (CoreAudio, CGEvent) - safety invariants
- Real-time audio thread (no locks, no allocations, no blocking)
- Threading coordination (audio capture → transcription → text insertion)
- Error handling (user-facing errors must be actionable)
- Performance regressions (profile before/after)

---

## Review Before CI Completes

You review PRs immediately, before CI finishes. Do NOT flag issues that CI will catch.

**CI Already Checks:**
- Code formatting (cargo fmt)
- Linting (cargo clippy - all + pedantic + nursery, see Cargo.toml)
- Compilation (cargo build)
- Tests (cargo test)
- Shell scripts (shellcheck)
- GitHub Actions (actionlint)

---

## Review Priority Levels

### 🔴 CRITICAL (Must Block PR)

**Safety Violations** (95%+ confidence)
- [ ] Unsafe code without safety documentation
- [ ] FFI pointer validity not guaranteed
- [ ] Data races (Send/Sync violated)
- [ ] Uninitialized memory access
- [ ] Use-after-free in FFI callbacks
- [ ] Raw pointers dereferenced without null check

**Correctness Issues** (90%+ confidence)
- [ ] Audio buffer synchronization bugs (lost samples, overruns)
- [ ] Hotkey registration race conditions
- [ ] Text insertion failures (CGEvent errors ignored)
- [ ] Model loading failures not handled
- [ ] Panics in production code (violates `panic = "deny"`)
- [ ] `.unwrap()` or `.expect()` outside tests (violates Clippy deny)

**Performance Regressions** (85%+ confidence)
- [ ] Blocking calls in real-time audio thread
- [ ] Allocations in hot paths (audio callback, transcription loop)
- [ ] Locks in latency-critical paths
- [ ] O(n²) algorithms in audio processing
- [ ] Unnecessary clones of large data

### 🟡 HIGH (Request Changes)

**Threading Issues** (80%+ confidence)
- [ ] Shared state without proper synchronization
- [ ] Channel unbounded (memory leak risk)
- [ ] Tokio blocking tasks not spawned with `spawn_blocking`
- [ ] Real-time thread priority not set (audio)
- [ ] Deadlock potential (lock ordering)

**Error Handling** (85%+ confidence)
- [ ] anyhow in library modules (use thiserror)
- [ ] Error context missing (`.context("what failed")`)
- [ ] User-facing errors not actionable ("failed to initialize" vs "grant Microphone permission")
- [ ] Panics converted to errors (bad, propagate instead)
- [ ] FFI errors ignored

**Testing** (80%+ confidence)
- [ ] New features without unit tests
- [ ] Audio/transcription logic without tests
- [ ] Hardware tests not marked `#[ignore]`
- [ ] Tests use `.unwrap()` excessively (use `?` or `.expect()` with context)

**API Design** (75%+ confidence)
- [ ] Public API without docs
- [ ] Breaking changes without migration path
- [ ] `pub` on internal types
- [ ] Inconsistent naming (not snake_case)

### 🟢 MEDIUM (Suggest/Comment)

**Performance** (70%+ confidence)
- [ ] Unnecessary heap allocations
- [ ] String allocations in loops
- [ ] Missing `#[inline]` on hot paths
- [ ] Vec reallocations (use `with_capacity`)
- [ ] Synchronous I/O in async context

**Code Quality** (65%+ confidence)
- [ ] Cognitive complexity >15 (clippy threshold)
- [ ] Function args >7 (clippy threshold)
- [ ] Long functions (>100 lines)
- [ ] Nested match/if (use early returns)
- [ ] Magic numbers (use named constants)

**Documentation** (60%+ confidence)
- [ ] Complex algorithms without explanation
- [ ] Unsafe blocks without safety docs
- [ ] FFI without usage examples
- [ ] Performance-critical code without metrics

### ⚪ LOW (Optional/Skip)

Don't comment on:
- Formatting (rustfmt handles)
- Clippy warnings (CI handles with strict config)
- Style preferences
- Micro-optimizations without profiling data

---

## Security & Safety Deep Dive

### Unsafe Code Requirements
- [ ] Safety invariants documented (/// # Safety: ...)
- [ ] Minimal unsafe scope (only FFI call, not entire function)
- [ ] FFI pointers validated (not null, properly aligned)
- [ ] Lifetimes correct (callback pointers valid for duration)
- [ ] Memory layout matches C types (repr(C) for FFI structs)

### FFI Boundaries (CoreAudio, CGEvent)
- [ ] Raw pointers checked before dereference
- [ ] C strings (CString) handled correctly (no interior nulls)
- [ ] Callbacks don't panic (catch_unwind if necessary)
- [ ] Reference counting correct (retain/release balance)
- [ ] Error codes checked (not ignored)

### Threading Safety
- [ ] Send/Sync bounds correct for concurrent types
- [ ] Arc/Mutex used appropriately (no unnecessary locking)
- [ ] Real-time thread constraints (no alloc, no locks, no syscalls)
- [ ] Channel capacity reasonable (prevent unbounded growth)
- [ ] Join handles awaited (no thread leaks)

### Data Integrity
- [ ] Audio samples not dropped (buffer overruns handled)
- [ ] Config file parsing robust (invalid TOML handled)
- [ ] Model file corruption detected (checksum/validation)
- [ ] Graceful shutdown (cleanup resources)

---

## Code Quality Standards

### Naming
- Functions/Variables: `snake_case`
- Types/Traits: `PascalCase`
- Constants: `UPPER_SNAKE_CASE`
- Modules: `snake_case`
- Meaningful names (avoid `x`, `tmp`, `data` unless iterator/temporary)

### Error Handling (Strict - No Unwrap)
- **Application code:** `anyhow::Result<T>` with `.context("description")`
- **Library modules:** `thiserror` derive for custom errors
- **NEVER:** `.unwrap()`, `.expect()` in production (Clippy denies)
- **Tests:** `.unwrap()`, `.expect()` OK (allowed in clippy.toml)
- **Early returns:** Prefer `?` operator over nested matches

```rust
// Library module
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("failed to initialize audio device")]
    DeviceInit(#[from] cpal::BuildStreamError),
    #[error("audio buffer overrun: {0} samples dropped")]
    BufferOverrun(usize),
}

// App code
fn setup_audio() -> anyhow::Result<AudioCapture> {
    let capture = AudioCapture::new()
        .context("failed to initialize audio capture")?;
    Ok(capture)
}
```

### Unsafe Policy (Minimize)
- **ONLY for:** FFI (CoreAudio, CGEvent), lock-free data structures, profiled hot paths
- **ALWAYS:** Document safety invariants with `/// # Safety:`
- **Minimize scope:** Wrap FFI in safe API, unsafe only for actual FFI call

```rust
/// # Safety: `device_ptr` must be valid for callback duration
unsafe extern "C" fn audio_callback(device_ptr: *mut c_void) -> i32 {
    let device = &*(device_ptr as *const AudioDevice);
    // Rest is safe code
    device.process_audio();
    0
}
```

### Testing Requirements
- **Unit tests:** Pure logic, no hardware
- **Hardware tests:** `#[test] #[ignore]` for mic/permissions
- **Coverage:** All public APIs tested
- **Edge cases:** Error paths, empty input, large input
- **Performance:** Benchmark critical paths if changed

### Documentation
- [ ] Public APIs have docs (missing_docs = "warn")
- [ ] Unsafe blocks have safety docs
- [ ] Complex algorithms explained
- [ ] Performance characteristics noted (O(n), allocations, etc.)
- [ ] README updated if behavior changes

---

## Rust-Specific Guidelines

### Modern Rust (2021 Edition)
- Use `?` operator for error propagation
- `async/await` for I/O, sync for compute
- `std::sync::Arc` for shared ownership
- `tokio::sync::mpsc` for async channels
- Prefer iterators over loops

### Async/Tokio Patterns
- **I/O operations:** async (file, network, timers)
- **CPU-bound:** `spawn_blocking` for blocking work
- **Channels:** bounded channels (prevent unbounded memory)
- **Shutdown:** graceful via `tokio::signal::ctrl_c()`

### Clippy Compliance (Strict)
Project uses:
- `all = "deny"` (over 700 lints)
- `pedantic = "warn"` (code quality)
- `nursery = "warn"` (experimental)
- `unwrap_used = "deny"` (no unwrap in production)
- `expect_used = "deny"` (no expect in production)
- `panic = "deny"` (no explicit panic)
- `exit = "deny"` (no process::exit, use Result)

Exceptions in clippy.toml:
- Tests: allow unwrap, expect, asserts
- Cognitive complexity: ≤15
- Too many args: ≤7

---

## Performance Guidelines

### Real-Time Audio Thread (Critical)
- **NO:** Allocations, locks, syscalls, blocking I/O
- **YES:** Lock-free queues, pre-allocated buffers, simple math
- **Latency:** Every operation <1ms
- **Priority:** Set thread priority to real-time

### Hot Paths
- [ ] Transcription loop (minimize allocations)
- [ ] Audio callback (lock-free, no panic)
- [ ] Text insertion (minimize CGEvent overhead)

### Profiling Requirements
Before claiming performance improvement:
- [ ] Flamegraph before/after (`cargo flamegraph`)
- [ ] Metrics logged (tracing spans)
- [ ] Real-world test (10s audio, multiple apps)

---

## macOS-Specific Guidelines

### FFI to CoreAudio
- [ ] AudioObjectGetPropertyData error checked
- [ ] AudioUnitSetProperty validated
- [ ] Device lifecycle managed (start/stop)
- [ ] Buffer sizes match constraints

### FFI to CGEvent
- [ ] CGEventCreateKeyboardEvent null check
- [ ] CGEventPost error handling
- [ ] Text encoding correct (UTF-16 for Unicode)
- [ ] Event tap permissions checked

### Permissions
- [ ] Microphone permission checked before audio
- [ ] Accessibility permission checked before hotkey
- [ ] User-facing error if permission denied
- [ ] Retry mechanism if permission granted after launch

---

## Architecture Patterns

**Follow these patterns:**
- Pure functions where possible (testable, no side effects)
- Builder pattern for complex initialization
- Type state pattern for state machines (audio states)
- Channel-based actor model for concurrency
- Error types per module (AudioError, TranscriptionError, etc.)

**Avoid these anti-patterns:**
- Global state (use dependency injection)
- God structs (split responsibilities)
- Panics (use Result)
- Blocking in async context (use spawn_blocking)
- Allocations in real-time thread

---

## Review Examples

### ✅ Good: Safe FFI with Documentation
```rust
/// # Safety: `device_ptr` must be valid for callback duration
unsafe extern "C" fn audio_callback(device_ptr: *mut c_void) -> OSStatus {
    if device_ptr.is_null() {
        return kAudioHardwareIllegalOperationError;
    }
    let device = &*(device_ptr as *const AudioDevice);
    device.process_samples()
}
```

### ❌ Bad: Unsafe Without Documentation
```rust
unsafe extern "C" fn audio_callback(device_ptr: *mut c_void) -> OSStatus {
    let device = &*(device_ptr as *const AudioDevice);  // No null check!
    device.process_samples()
}
```

---

### ✅ Good: Error Handling (No Unwrap)
```rust
fn load_model(path: &Path) -> anyhow::Result<WhisperModel> {
    let model = WhisperContext::new(path)
        .context("failed to load Whisper model")?;
    Ok(model)
}
```

### ❌ Bad: Unwrap in Production
```rust
fn load_model(path: &Path) -> WhisperModel {
    WhisperContext::new(path).unwrap()  // DENIED by Clippy
}
```

---

### ✅ Good: Async Blocking Work
```rust
async fn transcribe(audio: Vec<f32>) -> anyhow::Result<String> {
    tokio::task::spawn_blocking(move || {
        // CPU-bound Whisper inference
        whisper_model.transcribe(&audio)
    })
    .await?
    .context("transcription failed")
}
```

### ❌ Bad: Blocking in Async
```rust
async fn transcribe(audio: Vec<f32>) -> anyhow::Result<String> {
    // Blocks entire tokio runtime!
    whisper_model.transcribe(&audio)
}
```

---

### ✅ Good: Real-Time Thread (No Alloc)
```rust
fn audio_callback(samples: &[f32]) {
    // Pre-allocated ring buffer, lock-free push
    RING_BUFFER.push_slice(samples);
}
```

### ❌ Bad: Allocation in Callback
```rust
fn audio_callback(samples: &[f32]) {
    // Allocates! Causes audio glitches
    let mut vec = Vec::new();
    vec.extend_from_slice(samples);
}
```

---

## Maintainer Priorities

**What matters most:**
1. **Safety:** No UB, documented unsafe, correct FFI usage
2. **Performance:** Meet targets (<50ms audio, <2s transcription)
3. **Reliability:** No panics, graceful errors, resource cleanup
4. **User experience:** Clear errors, actionable messages, smooth operation

**Trade-offs we accept:**
- Code verbosity for safety (explicit error handling, safety docs)
- Some memory usage for performance (pre-allocated buffers)
- Conservative defaults for reliability (smaller model, lower beam size)

---

## Confidence Threshold

Only flag issues you're **80% or more confident** about.

If uncertain:
- Phrase as question: "Could this cause audio glitches?"
- Suggest profiling: "Profile this with flamegraph to verify"
- Don't block PR on speculation

---

## Review Tone

- **Constructive:** Explain WHY, not just WHAT
- **Specific:** Point to exact file:line
- **Actionable:** Suggest fix or alternative
- **Educational:** Explain safety/performance implications

**Example:**
❌ "This is unsafe"
✅ "In audio/capture.rs:142, dereferencing raw pointer without null check. If CoreAudio passes null (can happen on device disconnect), this is UB. Add: `if device_ptr.is_null() { return error; }` before dereference."

---

## Out of Scope

Do NOT review:
- [ ] Code formatting (rustfmt handles)
- [ ] Clippy warnings (CI handles with strict config)
- [ ] Style preferences (4 spaces, 100 chars enforced)
- [ ] Micro-optimizations (need profiling data)

---

## Special Cases

**When PR is:**
- **FFI changes:** Require safety docs, null checks, error handling
- **Audio thread:** Verify no allocations, locks, blocking
- **Performance:** Require before/after metrics (flamegraph, logs)
- **Threading:** Check for races, deadlocks, proper synchronization
- **Hotfix:** Focus on correctness + safety only

---

## Checklist Summary

Before approving PR, verify:
- [ ] No unsafe without safety docs
- [ ] No .unwrap() or .expect() in production code
- [ ] FFI pointers validated (null checks, lifetimes)
- [ ] No blocking in async or real-time threads
- [ ] Errors use anyhow (app) or thiserror (library)
- [ ] Tests exist for new code
- [ ] Performance targets met (if changed)
- [ ] User-facing errors actionable
- [ ] README/docs updated if behavior changed

---

## Additional Context

**See also:**
- [README.md](../README.md) - Usage, config, troubleshooting
- [CLAUDE.md](../CLAUDE.md) - Development workflow, testing, conventions
- [TESTING.md](../TESTING.md) - Profiling, performance testing
- [Cargo.toml](../Cargo.toml) - Clippy config (all + pedantic + nursery)
- [clippy.toml](../clippy.toml) - Thresholds, exceptions

**For questions:** Open issue for FFI, threading, or performance changes before implementing
