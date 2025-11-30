#!/usr/bin/env bash
set -euo pipefail

# Create DMG installer for whisper-hotkey with visual drag-to-Applications interface

APP_NAME="WhisperHotkey"
VERSION="${1:-1.0.0}"
BUNDLE_DIR="target/release/$APP_NAME.app"
DMG_NAME="WhisperHotkey-$VERSION.dmg"
DMG_TEMP="target/dmg-temp"
DMG_PATH="target/release/$DMG_NAME"
BACKGROUND_DIR="resources/dmg"

echo "📦 Creating DMG for $APP_NAME v$VERSION..."

# Check for create-dmg tool
if ! command -v create-dmg &> /dev/null; then
    echo "⚠️  create-dmg not found. Install with: brew install create-dmg"
    echo "📝 Falling back to basic DMG creation..."
    USE_BASIC=true
else
    USE_BASIC=false
fi

# Build app bundle first
if [ ! -d "$BUNDLE_DIR" ]; then
    echo "🔨 Building .app bundle..."
    ./scripts/create-app-bundle.sh
fi

# Clean up previous DMG
rm -rf "$DMG_TEMP" "$DMG_PATH"

if [ "$USE_BASIC" = true ]; then
    # Basic DMG creation (fallback)
    mkdir -p "$DMG_TEMP"

    echo "📋 Copying app bundle..."
    cp -r "$BUNDLE_DIR" "$DMG_TEMP/"

    echo "🔗 Creating Applications symlink..."
    ln -s /Applications "$DMG_TEMP/Applications"

    echo "💿 Creating DMG..."
    # Use hdiutil to create DMG (built into macOS)
    hdiutil create -volname "$APP_NAME" \
        -srcfolder "$DMG_TEMP" \
        -ov -format UDZO \
        "$DMG_PATH"

    # Clean up temp
    rm -rf "$DMG_TEMP"

else
    # Fancy DMG creation with visual installer
    echo "🎨 Creating visual DMG installer..."

    # Create background if it doesn't exist
    if [ ! -f "$BACKGROUND_DIR/background.png" ]; then
        ./scripts/create-dmg-background.sh
    fi

    mkdir -p "$DMG_TEMP"
    cp -r "$BUNDLE_DIR" "$DMG_TEMP/"

    # Create DMG with visual installer
    create-dmg \
        --volname "Whisper Hotkey" \
        --volicon "resources/AppIcon.icns" \
        --background "$BACKGROUND_DIR/background.png" \
        --window-pos 200 120 \
        --window-size 660 420 \
        --icon-size 160 \
        --icon "$APP_NAME.app" 180 190 \
        --hide-extension "$APP_NAME.app" \
        --app-drop-link 480 190 \
        --hdiutil-quiet \
        "$DMG_PATH" \
        "$DMG_TEMP"

    # Clean up
    rm -rf "$DMG_TEMP"
fi

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
