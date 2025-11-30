#!/usr/bin/env bash
set -euo pipefail

# Uninstaller for whisper-hotkey

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Whisper Hotkey Uninstaller"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

PLIST_PATH="$HOME/Library/LaunchAgents/com.whisper-hotkey.plist"
APP_PATH="/Applications/WhisperHotkey.app"
BINARY_PATH="/usr/local/bin/whisper-hotkey"
CONFIG_DIR="$HOME/.whisper-hotkey"

# Stop LaunchAgent
if [ -f "$PLIST_PATH" ]; then
    echo "⏹️  Stopping LaunchAgent..."
    launchctl unload "$PLIST_PATH" 2>/dev/null || true
    rm "$PLIST_PATH"
    echo "✅ Removed: $PLIST_PATH"
fi

# Remove .app bundle
if [ -d "$APP_PATH" ]; then
    echo "🗑️  Removing .app bundle..."
    rm -rf "$APP_PATH"
    echo "✅ Removed: $APP_PATH"
fi

# Remove binary
if [ -f "$BINARY_PATH" ]; then
    echo "🗑️  Removing binary..."
    sudo rm "$BINARY_PATH"
    echo "✅ Removed: $BINARY_PATH"
fi

# Ask about config/data
echo ""
echo "Remove configuration and data? (~/.whisper-hotkey/)"
echo "  This includes: config, models (~466MB), logs"
echo ""
read -p "Remove config/data? [y/N] " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    if [ -d "$CONFIG_DIR" ]; then
        echo "🗑️  Removing config/data..."
        rm -rf "$CONFIG_DIR"
        echo "✅ Removed: $CONFIG_DIR"
    fi
else
    echo "⏭️  Keeping config/data at: $CONFIG_DIR"
fi

echo ""
echo "✅ Uninstall complete!"
echo ""
