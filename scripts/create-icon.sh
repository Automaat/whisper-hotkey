#!/usr/bin/env bash
set -euo pipefail

# Create app icon for WhisperHotkey from logo.png

ICON_DIR="resources"
LOGO="$ICON_DIR/logo.png"
ICONSET="$ICON_DIR/AppIcon.iconset"
ICNS="$ICON_DIR/AppIcon.icns"

echo "🎨 Creating app icon from logo..."

# Verify logo exists
if [[ ! -f "$LOGO" ]]; then
    echo "❌ Error: logo.png not found at $LOGO"
    exit 1
fi

# Create directories
mkdir -p "$ICONSET"

# Use logo.png as base (resize to 1024x1024 if needed)
echo "📐 Preparing base image..."
sips -z 1024 1024 "$LOGO" --out "$ICONSET/base.png" >/dev/null

# Generate all required sizes from base image
for size in 16 32 128 256 512; do
    size2=$((size * 2))
    sips -z $size $size "$ICONSET/base.png" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
    sips -z $size2 $size2 "$ICONSET/base.png" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done

# Create .icns file
iconutil -c icns "$ICONSET" -o "$ICNS"

# Clean up
rm -rf "$ICONSET"

echo "✅ Icon created: $ICNS"
ls -lh "$ICNS"
