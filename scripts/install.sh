#!/usr/bin/env bash
set -euo pipefail

# Installer for whisper-hotkey macOS voice-to-text app

INSTALL_MODE="${1:-user}"  # user or app

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Whisper Hotkey Installer"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check prerequisites
echo "🔍 Checking prerequisites..."

if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo not found"
    echo "Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

if ! command -v mise &> /dev/null; then
    echo "⚠️  mise not found (optional)"
    echo "Install with: curl https://mise.run | sh"
    echo "Continuing without mise..."
fi

echo "✅ Prerequisites OK"
echo ""

# Build
echo "🔨 Building whisper-hotkey (release)..."
if command -v mise &> /dev/null && [ -f ".mise.toml" ]; then
    mise exec -- cargo build --release
else
    cargo build --release
fi
echo "✅ Build complete"
echo ""

# Install based on mode
if [ "$INSTALL_MODE" = "app" ]; then
    echo "📦 Creating .app bundle..."
    ./scripts/create-app-bundle.sh

    echo ""
    echo "📋 Installing to /Applications..."
    APP_NAME="WhisperHotkey"
    BUNDLE_DIR="target/release/$APP_NAME.app"

    if [ -d "/Applications/$APP_NAME.app" ]; then
        echo "⚠️  Removing existing installation..."
        rm -rf "/Applications/$APP_NAME.app"
    fi

    cp -r "$BUNDLE_DIR" /Applications/
    echo "✅ Installed: /Applications/$APP_NAME.app"

    BINARY_PATH="/Applications/$APP_NAME.app/Contents/MacOS/$APP_NAME"
else
    echo "📋 Installing binary to /usr/local/bin..."
    sudo cp target/release/whisper-hotkey /usr/local/bin/
    sudo chmod 755 /usr/local/bin/whisper-hotkey
    echo "✅ Installed: /usr/local/bin/whisper-hotkey"

    BINARY_PATH="/usr/local/bin/whisper-hotkey"
fi

echo ""

# Setup config
CONFIG_DIR="$HOME/.whisper-hotkey"
CONFIG_FILE="$CONFIG_DIR/config.toml"

echo "⚙️  Setting up configuration..."
mkdir -p "$CONFIG_DIR"

if [ ! -f "$CONFIG_FILE" ]; then
    echo "Creating default config: $CONFIG_FILE"
    cat > "$CONFIG_FILE" <<'EOF'
[hotkey]
modifiers = ["Control", "Option"]
key = "Z"

[audio]
buffer_size = 1024
sample_rate = 16000

[model]
name = "small"
path = "~/.whisper-hotkey/models/ggml-small.bin"
preload = true
threads = 4
beam_size = 5

[telemetry]
enabled = true
log_path = "~/.whisper-hotkey/crash.log"
EOF
    echo "✅ Created: $CONFIG_FILE"
else
    echo "✅ Using existing: $CONFIG_FILE"
fi

echo ""

# LaunchAgent setup
echo "🚀 Setup auto-start at login?"
echo "   This will install a LaunchAgent to start whisper-hotkey automatically."
echo ""
read -p "Install LaunchAgent? [y/N] " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    ./scripts/setup-launchagent.sh
    AUTO_START="✅ Enabled"
else
    echo "⏭️  Skipped LaunchAgent setup"
    AUTO_START="❌ Disabled (run scripts/setup-launchagent.sh to enable)"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Installation Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📍 Binary:     $BINARY_PATH"
echo "⚙️  Config:     $CONFIG_FILE"
echo "🚀 Auto-start: $AUTO_START"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Next Steps"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "1. Grant permissions:"
echo "   System Settings → Privacy & Security → Microphone"
echo "   System Settings → Privacy & Security → Accessibility"
echo ""
echo "2. Run the app:"
if [ "$INSTALL_MODE" = "app" ]; then
    echo "   open /Applications/$APP_NAME.app"
else
    echo "   whisper-hotkey"
fi
echo ""
echo "   On first run, it will:"
echo "   • Download Whisper model (~466MB)"
echo "   • Prompt for permissions"
echo ""
echo "3. Test voice transcription:"
echo "   • Open any text editor"
echo "   • Press and hold: Ctrl+Option+Z"
echo "   • Speak clearly"
echo "   • Release hotkey"
echo "   • Text appears at cursor"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "For help: https://github.com/Automaat/whisper-hotkey"
echo ""
