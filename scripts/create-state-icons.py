#!/usr/bin/env python3
"""Generate state-specific tray icons."""

from PIL import Image, ImageDraw
import sys

def create_recording_icon(base_path, output_path):
    """Add red recording dot to base icon."""
    img = Image.open(base_path).convert("RGBA")
    draw = ImageDraw.Draw(img)

    # Draw red recording dot in bottom-right corner
    dot_size = 8
    x, y = img.width - dot_size - 2, img.height - dot_size - 2
    draw.ellipse([x, y, x + dot_size, y + dot_size], fill=(255, 0, 0, 255))

    img.save(output_path)
    print(f"✓ Created {output_path}")

def create_processing_icon(base_path, output_path):
    """Add spinner arc to base icon."""
    img = Image.open(base_path).convert("RGBA")

    # Create overlay layer for spinner
    overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay)

    # Draw spinner arc in top-right corner
    spinner_size = 12
    x = img.width - spinner_size - 2
    y = 2

    # Draw partial circle (arc) - simulates spinner at one position
    draw.arc(
        [x, y, x + spinner_size, y + spinner_size],
        start=0,
        end=270,
        fill=(100, 150, 255, 255),
        width=2
    )

    # Composite overlay onto base
    img = Image.alpha_composite(img, overlay)
    img.save(output_path)
    print(f"✓ Created {output_path}")

if __name__ == "__main__":
    base_icon = "icon-32.png"

    create_recording_icon(base_icon, "icon-recording-32.png")
    create_processing_icon(base_icon, "icon-processing-32.png")

    print("✓ All state icons created")
