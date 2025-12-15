# Project Structure

Understanding the Whisper Hotkey codebase architecture.

## Repository Structure

```
whisper-hotkey/
├── src/
│   ├── main.rs              # Application entry point
│   ├── config.rs            # Configuration management
│   ├── permissions.rs       # macOS permission checks
│   ├── telemetry.rs         # Logging and crash reporting
│   ├── tray.rs              # System tray menu
│   ├── audio/
│   │   ├── mod.rs           # Audio module exports
│   │   ├── capture.rs       # CoreAudio FFI, recording
│   │   └── vad.rs           # Voice activity detection
│   ├── transcription/
│   │   ├── mod.rs           # Transcription module exports
│   │   ├── engine.rs        # Whisper model inference
│   │   ├── queue.rs         # Async transcription queue
│   │   └── download.rs      # Model download from Hugging Face
│   └── input/
│       ├── mod.rs           # Input module exports
│       ├── hotkey.rs        # global-hotkey integration
│       └── cgevent.rs       # CGEvent text insertion FFI
├── scripts/
│   ├── install.sh           # Installation script
│   ├── uninstall.sh         # Uninstallation script
│   ├── setup-launchagent.sh # Auto-start configuration
│   └── create-release.sh    # Release automation
├── docs/
│   └── (documentation files)
├── resources/
│   └── logo.png             # App logo
├── Cargo.toml               # Rust dependencies
├── .mise.toml               # Development tools (mise)
└── README.md                # Project documentation
```

## Module Architecture

### Entry Point: `main.rs`

**Purpose:** Application initialization and event loop

**Responsibilities:**
- Parse command-line arguments
- Load configuration
- Check permissions
- Initialize telemetry
- Create transcription profiles
- Register hotkeys
- Run tokio event loop

**Key functions:**
- `main()` - Entry point
- `initialize_profiles()` - Load models and register hotkeys
- Event loop - Handles hotkey events and transcription

### Configuration: `config.rs`

**Purpose:** Configuration file parsing and management

**Key types:**
- `Config` - Top-level configuration
- `TranscriptionProfile` - Profile with hotkey + model config
- `HotkeyConfig` - Hotkey settings
- `AudioConfig` - Audio capture settings
- `ModelConfig` - Whisper model settings
- `TelemetryConfig` - Logging settings
- `RecordingConfig` - Debug recording settings
- `AliasesConfig` - Alias matching settings

**Key functions:**
- `load()` - Load config from file
- `save()` - Save config to file
- `default()` - Generate default configuration

**Location:** `~/.whisper-hotkey/config.toml`

### Permissions: `permissions.rs`

**Purpose:** Check macOS permissions (Microphone, Accessibility)

**Key functions:**
- `check_permissions()` - Verify all required permissions
- `check_microphone_permission()` - Microphone access
- `check_accessibility_permission()` - Accessibility access
- `detect_quarantine()` - Detect macOS quarantine attribute

**Permission types:**
- `kTCCServiceMicrophone` - Audio recording
- `kTCCServiceAccessibility` - Hotkeys + text insertion
- `kTCCServiceListenEvent` - Input monitoring

### Telemetry: `telemetry.rs`

**Purpose:** Local crash logging (no cloud telemetry)

**Key functions:**
- `init()` - Initialize tracing subscriber
- `setup_file_appender()` - Configure log file

**Log levels:**
- `INFO` - Normal operations
- `DEBUG` - Detailed timing info
- `TRACE` - Low-level operations

**Log file:** `~/.whisper-hotkey/crash.log`

### System Tray: `tray.rs`

**Purpose:** System tray menu with profile status

**Key components:**
- `TrayState` - Current profile status
- `TrayMenu` - Menu configuration
- Menu items for each profile
- Status display (Ready/Recording/Transcribing)

**Features:**
- Show active profile
- Display status for each profile
- Quit menu item

## Audio Module

### Location: `src/audio/`

### `capture.rs`

**Purpose:** Low-level audio capture via CoreAudio

**Key types:**
- `AudioCapture` - Audio capture instance
- `AudioCallback` - Real-time audio callback

**Key functions:**
- `new()` - Initialize audio device
- `start_recording()` - Begin capture (<50ms latency target)
- `stop_recording()` - Stop and retrieve audio
- `convert_to_16khz_mono()` - Convert format for Whisper

**Real-time constraints:**
- NO heap allocations in callback
- NO locks in callback
- NO syscalls in callback
- <10ms callback duration (prefer <1ms)

**Lock-free communication:**
- Ring buffer for audio samples
- Atomic flags for state management

### `vad.rs`

**Purpose:** Voice activity detection (future feature)

**Status:** Planned, not yet implemented

## Transcription Module

### Location: `src/transcription/`

### `engine.rs`

**Purpose:** Whisper model loading and inference

**Key types:**
- `WhisperEngine` - Model wrapper
- `TranscriptionParams` - Inference parameters

**Key functions:**
- `new()` - Load model from disk (2-3s)
- `transcribe()` - Run inference (synchronous, CPU-bound)
- `transcribe_with_params()` - Custom parameters

**Thread safety:**
- `Send + Sync` - Safe to share across threads
- Wrapped in `Arc<>` for shared ownership
- Inference is synchronous (runs in tokio blocking pool)

**Performance:**
- base.en: ~1s for 10s audio (M1, beam_size=1)
- small: ~2s for 10s audio (M1, beam_size=1)

### `queue.rs`

**Purpose:** Async transcription queue (future feature)

**Status:** Planned for batch processing

### `download.rs`

**Purpose:** Download Whisper models from Hugging Face

**Key functions:**
- `download_model()` - Fetch model file
- `get_model_url()` - Construct Hugging Face URL

**Download location:** `~/.whisper-hotkey/models/ggml-{name}.bin`

**Models hosted at:**
```
https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{name}.bin
```

## Input Module

### Location: `src/input/`

### `hotkey.rs`

**Purpose:** Global hotkey registration via global-hotkey crate

**Key types:**
- `HotkeyManager` - Hotkey registration manager
- `HotkeyEvent` - Hotkey press/release events

**Key functions:**
- `register()` - Register global hotkey
- `unregister()` - Unregister hotkey
- Event channel - Async hotkey events

**Supported modifiers:**
- Control, Option, Command, Shift

**Supported keys:**
- A-Z, 0-9, F1-F12, Space, Tab, Return, Escape

### `cgevent.rs`

**Purpose:** Text insertion via Core Graphics CGEvent

**Key functions:**
- `insert_text()` - Insert text at cursor
- `create_text_event()` - Create CGEvent for text

**Unsafe FFI:**
- Calls Core Graphics C API
- Requires Accessibility permission

**Limitations:**
- Some apps block CGEvent (Terminal secure input, password fields)
- No fallback mechanism

## Threading Model

### Main Thread (tokio runtime)

**Responsibilities:**
- Event loop
- Hotkey monitoring
- Configuration management
- Transcription coordination

**Async operations:**
- Hotkey events (channel)
- Model loading (blocking pool)
- Transcription (blocking pool)

### Audio Thread (CoreAudio real-time thread)

**Responsibilities:**
- Audio capture callback
- Ring buffer writes

**Constraints:**
- OS-managed real-time thread
- <10ms callback duration
- NO allocations, locks, syscalls

### Transcription Thread (tokio blocking pool)

**Responsibilities:**
- Whisper model inference
- CPU-bound work

**Rationale:**
- Whisper is synchronous (not async)
- CPU-intensive (don't block main thread)
- Use tokio::task::spawn_blocking

## Communication Channels

### Hotkey → Main Thread

**Method:** `tokio::sync::mpsc` channel

**Events:**
- Hotkey pressed
- Hotkey released

### Audio Thread → Main Thread

**Method:** Lock-free ring buffer + atomic flags

**Data:** Audio samples (f32)

### Main Thread → Transcription Thread

**Method:** `tokio::task::spawn_blocking`

**Data:** Audio samples + transcription params

### Transcription Thread → Main Thread

**Method:** Function return (blocking)

**Data:** Transcribed text

## Error Handling

### Application Code

**Uses:** `anyhow::Result` with context

```rust
use anyhow::{Context, Result};

fn setup_audio() -> Result<AudioCapture> {
    AudioCapture::new()
        .context("failed to setup audio capture")?
}
```

### Library Modules

**Uses:** `thiserror` for custom errors

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("failed to initialize CoreAudio device")]
    DeviceInit(#[from] coreaudio::Error),
}
```

### FFI Code

**Safety invariants documented:**

```rust
/// # Safety: `device_ptr` valid for callback duration
unsafe fn audio_callback(device_ptr: *mut AudioDevice) {
    let device = &*device_ptr;
    // ...
}
```

## Dependencies

### Core Dependencies

**Cargo.toml highlights:**
- `whisper-rs` - Whisper model bindings
- `tokio` - Async runtime
- `global-hotkey` - System-wide hotkeys
- `coreaudio-rs` - macOS audio capture
- `cpal` - Cross-platform audio (fallback)
- `serde` + `toml` - Configuration parsing
- `anyhow` - Error handling
- `thiserror` - Custom errors
- `tracing` - Logging
- `tao` + `tray-icon` - System tray

### Build Dependencies

**Development tools (.mise.toml):**
- `rust 1.91` - Compiler
- `shellcheck` - Shell script linting
- `actionlint` - GitHub Actions linting
- `mdbook` - Documentation

## File Locations

### User Data

- Config: `~/.whisper-hotkey/config.toml`
- Models: `~/.whisper-hotkey/models/ggml-{name}.bin`
- Logs: `~/.whisper-hotkey/crash.log`
- Recordings: `~/.whisper-hotkey/recordings/` (debug only)

### System

- Binary: `/usr/local/bin/whisper-hotkey` (install.sh)
- App bundle: `/Applications/WhisperHotkey.app` (install.sh app)
- LaunchAgent: `~/Library/LaunchAgents/com.whisper-hotkey.plist`

## Next Steps

- Learn how to [Build from Source](./building.md)
- Understand [Testing](./testing.md) strategy
- Read [Contributing](./contributing.md) guidelines
