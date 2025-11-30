#!/usr/bin/env bash
set -euo pipefail

# Setup LaunchAgent for whisper-hotkey to start at login

PLIST_NAME="com.whisper-hotkey.plist"
LAUNCHAGENTS_DIR="$HOME/Library/LaunchAgents"
PLIST_TEMPLATE="$(cd "$(dirname "$0")" && pwd)/$PLIST_NAME"
PLIST_DEST="$LAUNCHAGENTS_DIR/$PLIST_NAME"

# Determine binary path
if [ -f "target/release/whisper-hotkey" ]; then
    BINARY_PATH="$(pwd)/target/release/whisper-hotkey"
elif [ -f "/Applications/WhisperHotkey.app/Contents/MacOS/WhisperHotkey" ]; then
    BINARY_PATH="/Applications/WhisperHotkey.app/Contents/MacOS/WhisperHotkey"
elif [ -f "/usr/local/bin/whisper-hotkey" ]; then
    BINARY_PATH="/usr/local/bin/whisper-hotkey"
else
    echo "❌ Error: whisper-hotkey binary not found"
    echo "Build with: cargo build --release"
    echo "Or install to: /usr/local/bin/whisper-hotkey or /Applications/WhisperHotkey.app"
    exit 1
fi

echo "📍 Binary path: $BINARY_PATH"

# Create LaunchAgents directory if needed
mkdir -p "$LAUNCHAGENTS_DIR"

# Create log directory
mkdir -p "$HOME/.whisper-hotkey"

# Copy plist and replace placeholders
sed -e "s|BINARY_PATH_PLACEHOLDER|$BINARY_PATH|g" \
    -e "s|HOME_PLACEHOLDER|$HOME|g" \
    "$PLIST_TEMPLATE" > "$PLIST_DEST"

echo "✅ Created: $PLIST_DEST"

# Unload existing if running
if launchctl list | grep -q com.whisper-hotkey; then
    echo "⏹️  Stopping existing service..."
    launchctl unload "$PLIST_DEST" 2>/dev/null || true
fi

# Load new LaunchAgent
echo "▶️  Starting service..."
launchctl load "$PLIST_DEST"

echo ""
echo "✅ LaunchAgent installed successfully!"
echo ""
echo "Service will:"
echo "  • Start now"
echo "  • Start automatically at login"
echo "  • Restart if it crashes"
echo ""
echo "Logs:"
echo "  stdout: $HOME/.whisper-hotkey/stdout.log"
echo "  stderr: $HOME/.whisper-hotkey/stderr.log"
echo ""
echo "Manage service:"
echo "  Stop:    launchctl unload $PLIST_DEST"
echo "  Start:   launchctl load $PLIST_DEST"
echo "  Restart: launchctl kickstart -k gui/\$(id -u)/com.whisper-hotkey"
echo "  Status:  launchctl list | grep whisper-hotkey"
