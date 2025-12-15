# Installation

Choose your preferred installation method:

## Option 1: Homebrew (Recommended)

**Easiest installation - no code signing issues:**

```bash
brew install Automaat/whisper-hotkey/whisper-hotkey
```

**Update:**

```bash
brew upgrade whisper-hotkey
```

**Uninstall:**

```bash
brew uninstall whisper-hotkey
```

## Option 2: Download DMG

**For users without Homebrew:**

1. **Download**: [Latest release](https://github.com/Automaat/whisper-hotkey/releases/latest) → `WhisperHotkey-*.dmg`
2. **Install**: Open DMG, drag `WhisperHotkey.app` to `Applications`
3. **Remove quarantine** (if needed):
   ```bash
   xattr -d com.apple.quarantine /Applications/WhisperHotkey.app
   ```
4. **Run**: Open from Applications

## Option 3: Build from Source

### Prerequisites

- **Rust/Cargo**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **mise** (optional): `curl https://mise.run | sh`

### Using Automated Installer

```bash
# Clone repository
git clone https://github.com/Automaat/whisper-hotkey.git
cd whisper-hotkey

# Binary mode (installs to /usr/local/bin)
./scripts/install.sh

# OR .app bundle mode (installs to /Applications)
./scripts/install.sh app
```

The installer will:
- Build release binary
- Install to `/usr/local/bin` or `/Applications`
- Create config: `~/.whisper-hotkey/config.toml`
- Optionally setup auto-start at login (LaunchAgent)

**Uninstall:**

```bash
./scripts/uninstall.sh
```

### Manual Build

```bash
# Clone repository
git clone https://github.com/Automaat/whisper-hotkey.git
cd whisper-hotkey

# Install Rust toolchain (if using mise)
mise install

# Build
mise exec -- cargo build --release
# OR without mise:
cargo build --release

# Run
mise exec -- cargo run --release
# OR:
./target/release/whisper-hotkey
```

## First Run

On first launch:
1. App creates config at `~/.whisper-hotkey/config.toml`
2. Prompts for permissions (see [macOS Permissions](./permissions.md))
3. Downloads Whisper model to `~/.whisper-hotkey/models/` (~466MB for small model)
4. Loads model (takes 2-3 seconds)
5. Ready to use!

## Auto-Start at Login

To run Whisper Hotkey automatically when you log in:

```bash
cd whisper-hotkey
./scripts/setup-launchagent.sh
```

**Manage the service:**

```bash
# Stop
launchctl unload ~/Library/LaunchAgents/com.whisper-hotkey.plist

# Start
launchctl load ~/Library/LaunchAgents/com.whisper-hotkey.plist

# Restart
launchctl kickstart -k gui/$(id -u)/com.whisper-hotkey

# Check status
launchctl list | grep whisper-hotkey

# View logs
tail -f ~/.whisper-hotkey/stdout.log
tail -f ~/.whisper-hotkey/stderr.log
```

**Disable auto-start:**

```bash
launchctl unload ~/Library/LaunchAgents/com.whisper-hotkey.plist
rm ~/Library/LaunchAgents/com.whisper-hotkey.plist
```

## Next Steps

After installation, see [Quick Start](./quick-start.md) to test your setup.
