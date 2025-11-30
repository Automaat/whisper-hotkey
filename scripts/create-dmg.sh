#!/usr/bin/env bash
set -euo pipefail

# Create DMG installer for whisper-hotkey

APP_NAME="WhisperHotkey"
VERSION="${1:-1.0.0}"
BUNDLE_DIR="target/release/$APP_NAME.app"
DMG_NAME="WhisperHotkey-$VERSION.dmg"
DMG_TEMP="target/dmg-temp"
DMG_PATH="target/release/$DMG_NAME"

echo "📦 Creating DMG for $APP_NAME v$VERSION..."

# Build app bundle first
if [ ! -d "$BUNDLE_DIR" ]; then
    echo "🔨 Building .app bundle..."
    ./scripts/create-app-bundle.sh
fi

# Clean up previous DMG
rm -rf "$DMG_TEMP" "$DMG_PATH"
mkdir -p "$DMG_TEMP"

echo "📋 Copying app bundle..."
cp -r "$BUNDLE_DIR" "$DMG_TEMP/"

echo "🔗 Creating Applications symlink..."
ln -s /Applications "$DMG_TEMP/Applications"

echo "📝 Creating README..."
cat > "$DMG_TEMP/README.txt" <<EOF
WhisperHotkey - Voice-to-Text for macOS

INSTALLATION:
1. Drag WhisperHotkey.app to Applications folder
2. Open WhisperHotkey from Applications
3. Grant permissions:
   - Microphone (System Settings → Privacy & Security)
   - Accessibility (System Settings → Privacy & Security)
4. First run downloads Whisper model (~466MB)

USAGE:
- Default hotkey: Ctrl+Option+Z
- Press and hold, speak, release
- Text appears at cursor

CONFIG:
- Location: ~/.whisper-hotkey/config.toml
- Change hotkey, model, performance settings

AUTO-START:
Run in Terminal:
  /Applications/WhisperHotkey.app/Contents/MacOS/WhisperHotkey

Or setup LaunchAgent (requires git clone):
  git clone https://github.com/Automaat/whisper-hotkey.git
  cd whisper-hotkey
  ./scripts/setup-launchagent.sh

SUPPORT:
https://github.com/Automaat/whisper-hotkey

LICENSE: MIT
EOF

echo "💿 Creating DMG..."
# Use hdiutil to create DMG (built into macOS)
hdiutil create -volname "$APP_NAME" \
    -srcfolder "$DMG_TEMP" \
    -ov -format UDZO \
    "$DMG_PATH"

# Clean up temp
rm -rf "$DMG_TEMP"

echo ""
echo "✅ DMG created: $DMG_PATH"
echo ""
echo "File info:"
ls -lh "$DMG_PATH"
echo ""
echo "To test:"
echo "  open $DMG_PATH"
echo ""
echo "To create GitHub release:"
echo "  gh release create v$VERSION $DMG_PATH --title \"v$VERSION\" --notes \"See CHANGELOG\""
