# Whisper Hotkey Voice-to-Text App - Implementation Plan

## Overview
macOS background app using local Whisper for system-wide voice-to-text via hotkey. Hold to record, release to transcribe and insert at cursor. Runs persistently, preloads model on startup.

## Technology Stack

### Core
- **Language:** Rust (performance, memory safety)
- **Whisper:** whisper.cpp via whisper-rs bindings
- **Model:** small multilingual (466MB, default), user-configurable (tiny/base/medium/large)
- **Audio:** Raw CoreAudio FFI (optimal latency)
- **Hotkey:** global-hotkey crate
- **Text Insertion:** Core Graphics (CGEvent API)

### Key Dependencies
```toml
whisper-rs = "0.15"
global-hotkey = "0.6"
coreaudio-rs = "0.11"  # Raw CoreAudio FFI
core-graphics = "0.23"
cocoa = "0.25"
tokio = { version = "1", features = ["full"] }
toml = "0.8"           # Config file parsing
serde = { version = "1", features = ["derive"] }
tracing = "0.1"        # Logging/telemetry
tracing-subscriber = "0.3"
reqwest = { version = "0.11", features = ["blocking"] }  # Model auto-download
```

## Architecture

### Components
```
Main Event Loop (Main Thread)
├── Global Hotkey Manager (press/release events)
├── Audio Recording Pipeline (CoreAudio → Ring Buffer)
├── Whisper Transcription Engine (background thread)
└── Text Insertion Service (CGEvent)
```

### Threading Model
- **Main:** Event loop, hotkey, text insertion
- **Audio:** Real-time CoreAudio callback (lock-free ring buffer)
- **Transcription:** Whisper inference (CPU-intensive)

### Data Flow
1. Hotkey press → Start audio recording
2. Accumulate audio samples in buffer
3. Hotkey release → Stop recording, send to Whisper
4. Whisper processes complete buffer → text result
5. CGEvent inserts text at cursor position

## Project Structure
```
whisper-hotkey/
├── Cargo.toml
├── .mise.toml              # rust = "1.84"
├── Info.plist              # Permission declarations
├── src/
│   ├── main.rs            # Event loop, initialization
│   ├── config.rs          # TOML config parser
│   ├── audio/
│   │   ├── mod.rs         # Module exports
│   │   ├── capture.rs     # Raw CoreAudio FFI, ring buffer
│   │   └── buffer.rs      # Lock-free ring buffer
│   ├── transcription/
│   │   ├── mod.rs         # Module exports
│   │   ├── engine.rs      # Whisper model loading, inference
│   │   └── download.rs    # Auto-download models
│   ├── input/
│   │   ├── mod.rs         # Module exports
│   │   ├── hotkey.rs      # global-hotkey integration
│   │   └── cgevent.rs     # CGEvent text insertion FFI
│   └── telemetry.rs       # tracing, crash logging
└── scripts/
    └── setup-launchagent.sh
```

## Configuration

**Config file:** `~/.whisper-hotkey.toml`

```toml
[hotkey]
modifiers = ["Control", "Option"]
key = "Z"

[audio]
buffer_size = 1024  # frames (configurable for testing)
sample_rate = 16000

[model]
# Auto-download on first run if not found
# Options: tiny, base, small (default), medium, large
name = "small"  # Multilingual - auto-detects English/Polish/etc
path = "~/.whisper-hotkey/models/ggml-small.bin"
preload = true  # Load on startup

[telemetry]
enabled = true
log_path = "~/.whisper-hotkey/crash.log"
```

## Technical Solutions

### Global Hotkey Capture
- Use global-hotkey crate on main thread
- Configurable via ~/.whisper-hotkey.toml
- Default: Ctrl+Option+Z (free on macOS, no system conflicts)
- Track press/release state
- **Permission:** Accessibility access

### Audio Recording
- Raw CoreAudio FFI (coreaudio-rs) for optimal latency
- Lock-free ring buffer for thread-safe transfer
- Buffer size: 1024 frames default (configurable)
- Accumulate samples while hotkey held
- Convert to 16kHz mono for Whisper
- **Permission:** Microphone access

### Whisper Processing
- **Reality:** NOT real-time streaming (Whisper design limitation)
- **Preload model on startup** (background app paradigm)
- Process complete buffer on hotkey release
- Metal acceleration (automatic on Apple Silicon)
- Target: <2s for 10s audio on M1

### Text Insertion
- Primary: CGEvent with CGEventKeyboardSetUnicodeString
- **No clipboard fallback** - log error if insertion fails
- Handle Unicode correctly
- **Permission:** Accessibility access (same as hotkey)

### Crash Telemetry
- Log crashes and errors to ~/.whisper-hotkey/crash.log
- Track transcription failures
- Configurable via telemetry.enabled in config

### Permissions
- No sandboxing (required for accessibility)
- No App Store distribution
- Info.plist declarations for mic + accessibility
- Manual installation + permission prompts

## Implementation Phases

### Phase 1: Foundation
- Initialize Rust project with Cargo
- Set up mise config (.mise.toml)
- Create Info.plist
- Implement config.rs (TOML parser for ~/.whisper-hotkey.toml)
- Implement telemetry/logger.rs (crash logging)
- Permission request helpers
- Main event loop structure
- **Validation:** App starts, loads config, requests permissions

### Phase 2: Global Hotkey
- Integrate global-hotkey crate
- Read hotkey from config
- Implement press/release handling
- State machine (idle → recording → processing)
- **Validation:** Hotkey triggers state transitions

### Phase 3: Audio Recording
- Set up raw CoreAudio FFI (coreaudio-rs)
- Read buffer_size from config
- Lock-free ring buffer implementation
- Audio accumulator (Vec<f32>)
- Sample rate conversion to 16kHz mono
- **Validation:** Audio captured, save WAV for debug

### Phase 4: Whisper Integration
- Implement auto-download for ggml-small.bin (reqwest)
- Integrate whisper-rs
- **Model preloading on startup** (background thread)
- Transcription function with language auto-detection
- **Validation:** Model auto-downloads, loads on startup, transcription <2s for 10s audio

### Phase 5: Text Insertion
- CGEvent-based insertion
- Test across apps (TextEdit, VS Code, Chrome, Slack)
- Log errors when insertion fails (no clipboard fallback)
- Unicode handling
- **Validation:** Text inserted correctly, errors logged

### Phase 6: Integration & Polish
- Wire all components end-to-end
- Error recovery with telemetry logging
- Test config changes (different hotkeys, buffer sizes)
- **Validation:** Full workflow works, config changes apply

### Phase 7: Optimization & Testing
- Profile audio latency
- Optimize Whisper params (threads, beam size)
- Test long recordings (30s+)
- Edge cases (silence, noise, empty)
- Memory leak testing

### Phase 8: Distribution
- Create .app bundle
- Code signing (optional)
- LaunchAgent auto-setup (start at login)
- Installer script
- Documentation

## Performance Targets
- Audio capture start: <50ms
- Transcription: <2s for 10s audio (M1, small model)
- Text insertion: <100ms
- **Total latency:** <2.5s for 10s recording

**Resource Usage:**
- Idle CPU: <1%
- RAM: ~1.5GB (1GB model + buffers)

## Known Limitations
1. No true real-time streaming (Whisper constraint)
2. Model preload adds ~1-2s to startup time
3. No Mac App Store distribution
4. Some apps may resist text insertion (errors logged, no fallback)
5. Multilingual model slightly slower than .en models for English-only use

## Critical Implementation Files
Once implemented:
1. `src/main.rs` - Event loop orchestration, model preloading
2. `src/config.rs` - TOML parser for ~/.whisper-hotkey.toml
3. `src/audio/capture.rs` - Raw CoreAudio FFI (performance-critical, real-time constraints)
4. `src/audio/buffer.rs` - Lock-free ring buffer
5. `src/transcription/engine.rs` - Whisper integration, model preload, language auto-detection
6. `src/transcription/download.rs` - Model auto-download (reqwest)
7. `src/input/hotkey.rs` - Global hotkey management
8. `src/input/cgevent.rs` - CGEvent text insertion FFI, error logging
9. `src/telemetry.rs` - Crash/error logging
10. `Cargo.toml` - Dependencies, build config

## Setup Commands
```bash
mise install
mise run setup          # cargo build + download model
mise run dev            # cargo run
mise run release        # cargo build --release
```

## Future Enhancements (Post-MVP)
- Visual feedback (menu bar icon)
- Hotkey customization UI
- Voice commands vs dictation mode
- Punctuation auto-insertion
- Long-form chunking (>30s)
- Voice activity detection (VAD)
- Per-hotkey language hints (override auto-detection)

---

## Resolved Design Decisions

**1. Audio library:** Raw CoreAudio FFI (not cpal)

- **Rationale:** Optimal latency, avoid abstraction overhead

**2. Default model:** small multilingual (466MB)

- **Rationale:** Best balance of speed/accuracy/memory, auto-detects English/Polish
- **User-configurable:** Can switch to tiny/base/medium/large via config

**3. VAD (Voice Activity Detection):** Skip for MVP

- **Rationale:** Simplicity, user controls recording via hotkey press/release

**4. Telemetry:** Local crash.log only (no Sentry)

- **Rationale:** Privacy, simplicity, no external dependencies

**5. Model distribution:** Auto-download on first run

- **Rationale:** Better UX, one less setup step for users

**6. Launch at login:** LaunchAgent auto-setup

- **Rationale:** Background app paradigm, reduce friction

**7. Default hotkey:** Ctrl+Option+Z

- **Rationale:** Free on macOS (no system conflicts), easily customizable
