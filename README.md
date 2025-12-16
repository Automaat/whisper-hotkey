# whisper-hotkey

[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg)](https://automaat.github.io/whisper-hotkey/)

![Whisper Hotkey logo](resources/logo.png)

macOS background app for system-wide voice-to-text via hotkey using local Whisper.

Hold a hotkey, speak, release → text inserted at cursor. Privacy-first (100% local, no cloud).

**Key Features:**
- 🎯 **Multi-profile support** - Different models per hotkey (e.g., fast for notes, accurate for emails)
- 🔄 **Smart aliases** - Fuzzy text replacement (e.g., "my email" → "user@example.com")
- 📊 **Menubar tray** - Visual feedback (idle/recording/processing) and quick config access
- 🔒 **100% local** - No cloud, no internet (except initial model download)
- ⚡ **Fast** - <50ms audio start, ~2s transcription (10s audio)

---

## Quick Start

### Option 1: Homebrew (Recommended)

**Easiest installation - no code signing issues:**

```bash
brew install Automaat/whisper-hotkey/whisper-hotkey
```

**First launch:**
1. Open WhisperHotkey from Applications
2. Grant permissions when prompted:
   - Accessibility
   - Input Monitoring
   - Microphone
3. App downloads Whisper model (~466MB)

**Usage:**
- Default hotkey: `Ctrl+Option+Z`
- Press and hold → speak → release
- Text appears at cursor

**Configuration:** `~/.whisper-hotkey/config.toml`

**Update:** `brew upgrade whisper-hotkey`

---

### Option 2: Download DMG

**For users without Homebrew:**

1. **Download**: [Latest release](https://github.com/Automaat/whisper-hotkey/releases/latest) → `WhisperHotkey-*.dmg`
2. **Install**: Open DMG, drag `WhisperHotkey.app` to `Applications`
3. **Remove quarantine** (if needed):
   ```bash
   xattr -d com.apple.quarantine /Applications/WhisperHotkey.app
   ```
4. **Run**: Open from Applications
5. **Permissions**: Grant Microphone + Accessibility + Input Monitoring when prompted
6. **First run**: Downloads Whisper model (~466MB)

---

### Option 3: Build from Source

**Prerequisites:**

- **macOS** (M1/M2 or Intel)
- **Rust/Cargo** (install: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- **mise** (optional, recommended: `curl https://mise.run | sh`)
- **Permissions**: Microphone + Accessibility

#### 3a: Automated Installer

```bash
# Clone repo
git clone https://github.com/Automaat/whisper-hotkey.git
cd whisper-hotkey

# Run installer (binary mode - installs to /usr/local/bin)
./scripts/install.sh

# OR .app bundle mode (installs to /Applications)
./scripts/install.sh app
```

The installer will:

- Build release binary
- Install to `/usr/local/bin` or `/Applications`
- Create config: `~/.whisper-hotkey/config.toml`
- Optionally setup auto-start at login (LaunchAgent)

**To uninstall:**

```bash
./scripts/uninstall.sh
```

#### 3b: Manual Build & Run

```bash
# Clone repo
git clone https://github.com/Automaat/whisper-hotkey.git
cd whisper-hotkey

# Install Rust toolchain (if using mise)
mise install

# Build (downloads ~466MB Whisper model on first run)
mise exec -- cargo build --release
# OR without mise:
cargo build --release

# Run
mise exec -- cargo run --release
# OR:
./target/release/whisper-hotkey
```

**First run:**
- Creates config: `~/.whisper-hotkey/config.toml`
- Prompts for **Microphone** permission (System Settings → Privacy & Security)
- Prompts for **Accessibility** permission (for hotkey + text insertion)
- Downloads Whisper model: `~/.whisper-hotkey/models/ggml-small.bin` (~466MB)
- Loads model (takes 2-3s)

**Console output:** Real-time logs show activity:
```
🎤 Hotkey pressed - recording started
⏹️  Hotkey released - processing audio
📼 Captured 3.5s audio (56000 samples)
✨ Transcription: "Hello, this is a test"
✅ Inserted 22 chars
✓ Ready for next recording
```

### 3. Test

1. Open any text editor (TextEdit, VS Code, Notes, Chrome)
2. Click into a text field
3. **Press and hold** `Ctrl+Option+Z`
4. Speak clearly: "Hello, this is a test"
5. **Release** the hotkey
6. Text appears at cursor in ~2s

**Expected output:**
```
✓ Config loaded from ~/.whisper-hotkey/config.toml
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

Edit `~/.whisper-hotkey/config.toml`:

```toml
# Multi-profile support - define multiple hotkeys with different models
[[profiles]]
name = "Fast"                       # optional profile name
model_type = "small"                # tiny, base, small, medium, large, tiny.en, base.en, small.en, medium.en
[profiles.hotkey]
modifiers = ["Control", "Option"]
key = "Z"
preload = true                      # load on startup (recommended)
threads = 4                         # CPU threads (try 2/4/8)
beam_size = 1                       # 1=fast, 5=balanced, 10=accurate
language = "en"                     # optional language hint

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
buffer_size = 1024                  # frames (leave default)
sample_rate = 16000                 # Hz (leave default)

[telemetry]
enabled = true                      # local crash logging only
log_path = "~/.whisper-hotkey/crash.log"

[recording]
enabled = true                      # save debug recordings
retention_days = 7                  # auto-delete after N days
max_count = 100                     # keep max N recordings
cleanup_interval_hours = 24         # cleanup frequency

[aliases]
enabled = true                      # fuzzy text replacement
threshold = 0.85                    # match threshold (0.0-1.0)
[aliases.entries]
"my email" = "user@example.com"
"my address" = "123 Main St, City, State 12345"
"github" = "https://github.com/username"
```

**After editing**: Restart app (`Ctrl+C`, then `cargo run --release`)

**Note:** You can define multiple profiles with different models and hotkeys. The tray icon menu shows all active profiles.

---

## Auto-Start at Login

To run whisper-hotkey automatically when you log in:

```bash
# Setup LaunchAgent (starts now and at every login)
./scripts/setup-launchagent.sh
```

**Manage the service:**

```bash
# Stop service
launchctl unload ~/Library/LaunchAgents/com.whisper-hotkey.plist

# Start service
launchctl load ~/Library/LaunchAgents/com.whisper-hotkey.plist

# Restart service
launchctl kickstart -k gui/$(id -u)/com.whisper-hotkey

# Check status
launchctl list | grep whisper-hotkey

# View logs
tail -f ~/.whisper-hotkey/stdout.log
tail -f ~/.whisper-hotkey/stderr.log
```

**To disable auto-start:**

```bash
launchctl unload ~/Library/LaunchAgents/com.whisper-hotkey.plist
rm ~/Library/LaunchAgents/com.whisper-hotkey.plist
```

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
model_type = "tiny"   # Faster, less accurate (~75MB)
# or
model_type = "base"   # Good balance (~142MB)
# or
model_type = "medium" # More accurate, slower (~1.5GB)
```

App auto-downloads model on next run.

---

## Troubleshooting

### Permissions not working after granting them (macOS Quarantine)

**Symptoms:** You've granted Microphone and Accessibility permissions, but the app still can't access them.

**Cause:** macOS quarantine attribute (applied to downloaded apps) prevents the system from recognizing granted permissions.

**Solution:**

1. Open Terminal
2. Run this command (replace path if needed):
   ```bash
   xattr -d com.apple.quarantine /Applications/WhisperHotkey.app
   ```
3. Restart the app

**Note:** The app will detect quarantine on startup and show this command automatically.

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

## Creating Releases

**For maintainers:**

```bash
# Auto-increment minor version (0.0.0 → 0.1.0)
./scripts/create-release.sh

# Specific version
./scripts/create-release.sh 0.1.0

# Or manually via GitHub CLI
gh workflow run release.yml              # Auto-increment
gh workflow run release.yml -f version=0.1.0  # Specific version
```

The release workflow:

1. Creates and pushes git tag (e.g., `v0.1.0`)
2. Builds release binary with optimizations
3. Creates .app bundle and DMG
4. Generates SHA256 checksum
5. Publishes GitHub release with artifacts

**Monitor release:**

```bash
gh run watch
# OR
gh run list --workflow=release.yml
```

Releases appear at: [GitHub Releases](https://github.com/Automaat/whisper-hotkey/releases)

---

## Development

### Run tests
```bash
# Unit tests (no hardware required)
mise exec -- cargo test

# Hardware tests (requires mic + permissions)
mise exec -- cargo test -- --ignored
```

### Logging levels

```bash
# Default: info level (shows hotkey events, transcription results)
cargo run --release

# Debug: detailed timing information
RUST_LOG=debug cargo run --release

# Trace: everything including low-level operations
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
- tray-icon (menubar integration)

---

## Advanced Features

### Multi-Profile Support

Define multiple transcription profiles with different models and hotkeys:

- **Fast profile** (Ctrl+Option+Z): Use `small` model for quick notes
- **Accurate profile** (Cmd+Shift+V): Use `medium` model for emails

Each profile can have different:
- Model type (tiny → large)
- Hotkey combination
- Inference settings (threads, beam_size)
- Language hints

The tray icon menu displays all active profiles with their hotkeys and models.

### Smart Aliases

Replace transcribed text with predefined values using fuzzy matching:

```toml
[aliases]
enabled = true
threshold = 0.85  # 0.0 (loose) to 1.0 (exact)
[aliases.entries]
"my email" = "john.doe@company.com"
"office address" = "123 Main St, Suite 400, San Francisco, CA 94105"
"github profile" = "https://github.com/username"
```

**How it works:**
- Case-insensitive fuzzy matching (Jaro-Winkler algorithm)
- Say "my email" → automatically replaced with your configured email
- Handles pronunciation variations (e.g., "office address" vs "office adress")
- Best match wins if multiple aliases are close

**Use cases:**
- Email addresses / phone numbers
- Physical addresses
- URLs / code snippets
- Company names / product names

### Menubar Tray Icon

Visual feedback and quick access:

- **Adaptive icon** (idle): Black on light mode, white on dark mode
- **Red icon** (recording): Shows when hotkey is pressed
- **Yellow icon** (processing): Shows during transcription
- **Menu**: Lists all profiles, "Open Config File", "Quit"
- **Retina support**: Automatically uses high-DPI icons

### Debug Recording Retention

Optionally save audio recordings for debugging:

```toml
[recording]
enabled = true           # Save recordings to ~/.whisper-hotkey/debug/
retention_days = 7       # Auto-delete after N days
max_count = 100          # Keep max N most recent recordings
cleanup_interval_hours = 24  # Cleanup frequency
```

Recordings named: `recording_{timestamp}.wav`

**Use cases:**
- Debug transcription accuracy issues
- Compare different model performance
- Report bugs with audio samples

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
- [x] Phase 8: Distribution (.app bundle, installer)

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
