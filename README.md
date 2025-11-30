# whisper-hotkey

macOS background app for system-wide voice-to-text via hotkey using local Whisper.

Hold a hotkey, speak, release → text inserted at cursor. Privacy-first (100% local, no cloud).

---

## Quick Start

### Prerequisites

- **macOS** (M1/M2 or Intel)
- **Rust 1.84** (via mise)
- **Permissions**: Microphone + Accessibility

### 1. Install

```bash
# Clone repo
git clone https://github.com/Automaat/whisper-hotkey.git
cd whisper-hotkey

# Install Rust toolchain
mise install

# Build (downloads ~466MB Whisper model on first run)
mise exec -- cargo build --release
```

### 2. Run

```bash
mise exec -- cargo run --release
```

**First run:**
- Creates config: `~/.whisper-hotkey.toml`
- Prompts for **Microphone** permission (System Settings → Privacy & Security)
- Prompts for **Accessibility** permission (for hotkey + text insertion)
- Downloads Whisper model: `~/.whisper-hotkey/models/ggml-small.bin` (~466MB)
- Loads model (takes 2-3s)

### 3. Test

1. Open any text editor (TextEdit, VS Code, Notes, Chrome)
2. Click into a text field
3. **Press and hold** `Ctrl+Option+Z`
4. Speak clearly: "Hello, this is a test"
5. **Release** the hotkey
6. Text appears at cursor in ~2s

**Expected output:**
```
✓ Config loaded from ~/.whisper-hotkey.toml
✓ Telemetry initialized
✓ Permissions OK
✓ Model found at /Users/you/.whisper-hotkey/models/ggml-small.bin
Loading Whisper model (this may take a few seconds)...
  Optimization: 4 threads, beam_size=5
✓ Whisper model loaded and ready
✓ Audio capture initialized
✓ Hotkey registered: ["Control", "Option"] + Z

Whisper Hotkey is running. Press the hotkey to record and transcribe.
✓ Full pipeline ready: hotkey → audio → transcription → text insertion
Press Ctrl+C to exit.
```

---

## Configuration

Edit `~/.whisper-hotkey.toml`:

```toml
[hotkey]
modifiers = ["Control", "Option"]  # or ["Command", "Shift"]
key = "Z"                           # any letter A-Z

[audio]
buffer_size = 1024                  # frames (leave default)
sample_rate = 16000                 # Hz (leave default)

[model]
name = "small"                      # tiny, base, small, medium, large
path = "~/.whisper-hotkey/models/ggml-small.bin"
preload = true                      # load on startup (recommended)
threads = 4                         # CPU threads (try 2/4/8)
beam_size = 5                       # 1=fast, 5=balanced, 10=accurate

[telemetry]
enabled = true                      # local crash logging only
log_path = "~/.whisper-hotkey/crash.log"
```

**After editing**: Restart app (`Ctrl+C`, then `cargo run --release`)

---

## Performance Tuning

### Fast Mode (sacrifice accuracy)
```toml
[model]
threads = 8
beam_size = 1
```

### Accurate Mode (slower)
```toml
[model]
threads = 4
beam_size = 10
```

### Different Models
```toml
[model]
name = "tiny"   # Faster, less accurate (~75MB)
# or
name = "base"   # Good balance (~142MB)
# or
name = "medium" # More accurate, slower (~1.5GB)
```

App auto-downloads model on next run.

---

## Troubleshooting

### "No input device available"
- Grant **Microphone** permission: System Settings → Privacy & Security → Microphone
- Reset: `tccutil reset Microphone`, then restart app

### "Failed to register global hotkey"
- Grant **Accessibility** permission: System Settings → Privacy & Security → Accessibility
- Add Terminal/iTerm to allowed apps

### Text not inserting
- Check **Accessibility** permission (same as above)
- Some apps block insertion (Terminal secure input mode)
- Check logs: `tail -f ~/.whisper-hotkey/crash.log`

### Slow transcription
- Try faster config (threads=8, beam_size=1)
- Use smaller model (tiny or base)
- Check logs for `inference_ms` metric

### Model download fails
- Manual download from [Hugging Face](https://huggingface.co/ggerganov/whisper.cpp)
- Place in: `~/.whisper-hotkey/models/ggml-{name}.bin`

---

## Development

### Run tests
```bash
# Unit tests (no hardware required)
mise exec -- cargo test

# Hardware tests (requires mic + permissions)
mise exec -- cargo test -- --ignored
```

### Profile performance
```bash
# Detailed logs
RUST_LOG=whisper_hotkey=trace cargo run --release

# CPU profiling
sudo cargo flamegraph --release
# Trigger hotkey, Ctrl+C, then: open flamegraph.svg

# Memory profiling (macOS)
instruments -t Allocations target/release/whisper-hotkey
```

See [TESTING.md](TESTING.md) for comprehensive profiling guide.

### Format & lint
```bash
mise exec -- cargo fmt
mise exec -- cargo clippy
```

---

## How It Works

1. **Hotkey pressed** → Clear audio buffer, start recording
2. **Hotkey held** → Accumulate audio samples (16kHz mono)
3. **Hotkey released** → Stop recording, convert audio format
4. **Transcription** → Whisper processes audio (~2s for 10s recording)
5. **Text insertion** → CGEvent inserts text at cursor

**Tech stack:**
- Rust 1.84
- Whisper.cpp (via whisper-rs bindings)
- cpal (audio capture)
- global-hotkey (hotkey detection)
- Core Graphics CGEvent (text insertion)

---

## Privacy

- **100% local**: No cloud, no internet required (except model download)
- **No telemetry**: Only local crash logs (`~/.whisper-hotkey/crash.log`)
- **No storage**: Audio discarded after transcription

---

## Limitations

- **macOS only** (uses Core Graphics, Accessibility APIs)
- **No real-time streaming** (Whisper design limitation)
- **No App Store** (requires Accessibility, no sandbox)
- **Some apps resist text insertion** (Terminal secure input, etc.)

---

## Performance Targets

| Metric | Target | Actual (M1, small model) |
|--------|--------|--------------------------|
| Audio start | <50ms | ~5-10ms |
| Transcription (10s) | <2s | ~1.5-2s |
| Text insertion | <100ms | ~20-50ms |
| Idle CPU | <1% | ~0.5% |
| Idle RAM | ~1.5GB | ~1.3GB |

---

## Roadmap

- [x] Phase 1: Foundation (config, telemetry, permissions)
- [x] Phase 2: Global hotkey
- [x] Phase 3: Audio recording
- [x] Phase 4: Whisper integration
- [x] Phase 5: Text insertion
- [x] Phase 6: Integration & polish
- [x] Phase 7: Optimization & testing
- [ ] Phase 8: Distribution (.app bundle, installer)

See [implem-plan.md](implem-plan.md) for detailed implementation plan.

---

## License

MIT

---

## Contributing

PRs welcome! Please:
- Run `cargo fmt` and `cargo clippy` before submitting
- Add tests for new features
- Update TESTING.md for profiling changes

---

## Support

- **Issues**: https://github.com/Automaat/whisper-hotkey/issues
- **Docs**: See [TESTING.md](TESTING.md) for profiling/debugging
- **Implementation**: See [implem-plan.md](implem-plan.md)
