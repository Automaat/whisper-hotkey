#!/usr/bin/env bash
set -euo pipefail

# Create DMG background image with arrow

BG_DIR="resources/dmg"
BG_FILE="$BG_DIR/background.png"
BG_2X="$BG_DIR/background@2x.png"
WIDTH=660
HEIGHT=400

mkdir -p "$BG_DIR"

echo "🎨 Creating DMG background..."

# Create background with Python PIL
python3 <<PYTHON
from PIL import Image, ImageDraw, ImageFont
import os

def create_background(width, height, scale=1):
    # White background (macOS style)
    img = Image.new('RGB', (width, height), color='#ffffff')
    draw = ImageDraw.Draw(img)

    # Arrow parameters (centered between app icon at 180 and Applications at 480)
    arrow_y = int(170 * scale)
    arrow_start_x = int(340 * scale)
    arrow_end_x = int(440 * scale)
    arrow_width = int(3 * scale)
    arrow_color = (128, 128, 128)  # Gray arrow

    # Draw arrow shaft (thicker)
    for i in range(arrow_width):
        offset = i - arrow_width // 2
        draw.line(
            [(arrow_start_x, arrow_y + offset), (arrow_end_x - int(15 * scale), arrow_y + offset)],
            fill=arrow_color,
            width=1
        )

    # Draw arrow head (larger triangle)
    arrow_size = int(15 * scale)
    points = [
        (arrow_end_x, arrow_y),
        (arrow_end_x - arrow_size * 2, arrow_y - arrow_size),
        (arrow_end_x - arrow_size * 2, arrow_y + arrow_size)
    ]
    draw.polygon(points, fill=arrow_color)

    return img

# Create standard resolution
img = create_background($WIDTH, $HEIGHT, scale=1)
os.makedirs('$BG_DIR', exist_ok=True)
img.save('$BG_FILE')

# Create retina resolution
img_2x = create_background($WIDTH * 2, $HEIGHT * 2, scale=2)
img_2x.save('$BG_2X')

print(f"✅ Background created: $BG_FILE")
PYTHON

echo "✅ DMG background created: $BG_FILE"
ls -lh "$BG_FILE" "$BG_2X"
