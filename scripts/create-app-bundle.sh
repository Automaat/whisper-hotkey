#!/usr/bin/env bash
set -euo pipefail

# Create macOS .app bundle for whisper-hotkey

APP_NAME="WhisperHotkey"
BUNDLE_DIR="target/release/$APP_NAME.app"
CONTENTS_DIR="$BUNDLE_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

# Only build if binary doesn't exist or is older than source
if [ ! -f "target/release/whisper-hotkey" ] || [ "src/" -nt "target/release/whisper-hotkey" ]; then
    echo "🔨 Building release binary..."
    cargo build --release
else
    echo "✅ Using existing release binary"
fi

echo "📦 Creating .app bundle structure..."
rm -rf "$BUNDLE_DIR"
mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"

echo "📋 Copying binary..."
cp target/release/whisper-hotkey "$MACOS_DIR/$APP_NAME"

echo "🎨 Creating/copying icon..."
if [ ! -f "resources/AppIcon.icns" ]; then
    ./scripts/create-icon.sh
fi
if [ ! -f "resources/AppIcon.icns" ]; then
    echo "❌ Error: Icon file resources/AppIcon.icns not found after attempting to create it."
    exit 1
fi
cp resources/AppIcon.icns "$RESOURCES_DIR/"

echo "📋 Copying Info.plist..."
cat > "$CONTENTS_DIR/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>

    <key>CFBundleExecutable</key>
    <string>$APP_NAME</string>

    <key>CFBundleIdentifier</key>
    <string>com.whisper-hotkey</string>

    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>

    <key>CFBundleName</key>
    <string>$APP_NAME</string>

    <key>CFBundlePackageType</key>
    <string>APPL</string>

    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>

    <key>CFBundleVersion</key>
    <string>1</string>

    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>

    <key>LSUIElement</key>
    <true/>

    <key>CFBundleIconFile</key>
    <string>AppIcon</string>

    <key>NSMicrophoneUsageDescription</key>
    <string>Required to capture voice for transcription</string>

    <key>NSAppleEventsUsageDescription</key>
    <string>Required to insert transcribed text at cursor position</string>
</dict>
</plist>
EOF

echo "🔏 Ad-hoc code signing..."
if ! codesign --force --deep --sign - "$BUNDLE_DIR"; then
    echo "⚠️  codesign failed. The bundle may be invalid or your system may not support ad-hoc signing."
    echo "You can still try to run the app, but macOS may refuse to launch it or show a warning."
fi

echo "✅ .app bundle created: $BUNDLE_DIR"
echo ""
echo "To install:"
echo "  cp -r $BUNDLE_DIR /Applications/"
echo ""
echo "To run:"
echo "  open $BUNDLE_DIR"
