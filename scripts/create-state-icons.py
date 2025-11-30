#!/usr/bin/env python3
"""Generate state-specific tray icons."""

from PIL import Image, ImageDraw
import sys

def create_recording_icon(base_path, output_path):
    """Add red recording dot to base icon."""
    img = Image.open(base_path).convert("RGBA")
    draw = ImageDraw.Draw(img)

    # Draw larger red recording dot in top-right corner (more visible)
    dot_size = 12
    x, y = img.width - dot_size - 1, 1
    # Draw with bright red fill and outline for visibility
    draw.ellipse([x, y, x + dot_size, y + dot_size], fill=(255, 20, 20, 255), outline=(180, 0, 0, 255), width=2)

    img.save(output_path)
    print(f"✓ Created {output_path}")

def create_processing_icon(base_path, output_path):
    """Add spinner arc to base icon."""
    img = Image.open(base_path).convert("RGBA")

    # Create overlay layer for spinner
    overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay)

    # Draw larger spinner arc in top-right corner (more visible)
    spinner_size = 14
    x = img.width - spinner_size - 1
    y = 1

    # Draw partial circle (arc) with bright blue - simulates spinner at one position
    draw.arc(
        [x, y, x + spinner_size, y + spinner_size],
        start=45,
        end=315,
        fill=(30, 144, 255, 255),  # Bright blue
        width=3
    )

    # Add a second arc for more visibility
    draw.arc(
        [x + 2, y + 2, x + spinner_size - 2, y + spinner_size - 2],
        start=45,
        end=315,
        fill=(100, 180, 255, 255),  # Lighter blue
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
