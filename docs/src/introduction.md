# Introduction

![Whisper Hotkey logo](https://raw.githubusercontent.com/Automaat/whisper-hotkey/main/resources/logo.png)

**Whisper Hotkey** is a macOS background app that provides system-wide voice-to-text via hotkey using local Whisper AI.

## What It Does

Hold a hotkey, speak, release → text inserted at cursor. Simple, fast, private.

## Key Features

- **Privacy-First**: 100% local processing, no cloud, no internet required (except initial model download)
- **System-Wide**: Works in any macOS app (TextEdit, VS Code, Chrome, Notes, etc.)
- **Fast**: <50ms audio start, <2s transcription for 10s audio, <100ms text insertion
- **Multi-Profile**: Configure different hotkeys with different Whisper models and settings
- **Alias Matching**: Auto-expand common phrases (e.g., "dot com" → ".com")
- **Efficient**: <1% idle CPU, ~1.5GB RAM with loaded model

## How It Works

1. **Press and hold** configured hotkey (default: `Ctrl+Option+Z`)
2. **Speak** your text clearly
3. **Release** hotkey when done
4. **Text appears** at cursor position in ~2 seconds

## Tech Stack

- **Rust 1.84** - Modern systems programming language
- **Whisper.cpp** - Local AI transcription (via whisper-rs bindings)
- **cpal** - Cross-platform audio capture
- **global-hotkey** - System-wide hotkey detection
- **Core Graphics** - macOS text insertion via CGEvent

## Privacy Guarantee

- **No cloud processing** - Everything runs locally on your Mac
- **No telemetry** - Only local crash logs (`~/.whisper-hotkey/crash.log`)
- **No audio storage** - Audio is discarded immediately after transcription
- **No internet** - After initial model download, works completely offline

## Requirements

- **macOS** (M1/M2/M3 or Intel)
- **Permissions**: Microphone, Accessibility, Input Monitoring
- **Disk Space**: ~500MB for model + ~50MB for app

## Next Steps

Ready to get started? Head to [Installation](./getting-started/installation.md) to install Whisper Hotkey.
