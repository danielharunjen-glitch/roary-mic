#!/usr/bin/env python3
"""Generate the DMG background image used by Tauri's macOS bundler.

Run: /tmp/pil-venv/bin/python assets/generate-dmg-background.py
or:  python3 -m pip install --user Pillow && python3 assets/generate-dmg-background.py

Produces assets/dmg-background.png (660x400). Tauri composites two icons on top
of this image at the positions configured in src-tauri/tauri.conf.json:
  - app icon at (180, 170)
  - Applications folder symlink at (480, 170)

The image draws an arrow pointing from the app position to the Applications
position and a "first launch: right-click -> Open" note below.
"""
from __future__ import annotations

from PIL import Image, ImageDraw, ImageFont


WIDTH, HEIGHT = 660, 400
APP_X, APP_Y = 180, 170
DEST_X, DEST_Y = 480, 170
ICON_HALF = 64  # Tauri places ~128px icons; arrow stays clear of them.

BG_TOP = (24, 24, 28)
BG_BOTTOM = (40, 40, 48)
ACCENT = (255, 196, 64)         # warm Roary lion gold
TEXT = (235, 235, 240)
SUBTEXT = (170, 170, 180)


def vertical_gradient(width: int, height: int, top: tuple, bottom: tuple) -> Image.Image:
    img = Image.new("RGB", (width, height), top)
    px = img.load()
    for y in range(height):
        t = y / max(1, height - 1)
        r = int(top[0] + (bottom[0] - top[0]) * t)
        g = int(top[1] + (bottom[1] - top[1]) * t)
        b = int(top[2] + (bottom[2] - top[2]) * t)
        for x in range(width):
            px[x, y] = (r, g, b)
    return img


def find_font(size: int, italic: bool = False) -> ImageFont.FreeTypeFont:
    candidates = [
        "/System/Library/Fonts/SFNSItalic.ttf" if italic else "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/Library/Fonts/Arial.ttf",
    ]
    for path in candidates:
        try:
            return ImageFont.truetype(path, size=size)
        except (OSError, IOError):
            continue
    return ImageFont.load_default()


def draw_arrow(d: ImageDraw.ImageDraw, start: tuple, end: tuple, color: tuple, width: int = 6) -> None:
    sx, sy = start
    ex, ey = end
    d.line([(sx, sy), (ex, ey)], fill=color, width=width)
    head = 22
    d.polygon(
        [(ex, ey - head // 2), (ex, ey + head // 2), (ex + head, ey)],
        fill=color,
    )


def main() -> None:
    img = vertical_gradient(WIDTH, HEIGHT, BG_TOP, BG_BOTTOM)
    d = ImageDraw.Draw(img)

    title_font = find_font(34, italic=True)
    subtitle_font = find_font(15)
    note_font = find_font(13)

    d.text(
        (WIDTH // 2, 46),
        "Roary Mic",
        fill=TEXT,
        font=title_font,
        anchor="mm",
    )
    d.text(
        (WIDTH // 2, 78),
        "Drag the app onto Applications",
        fill=SUBTEXT,
        font=subtitle_font,
        anchor="mm",
    )

    # Arrow from app icon area to Applications folder area, clearing the icons.
    arrow_start = (APP_X + ICON_HALF + 12, APP_Y)
    arrow_end = (DEST_X - ICON_HALF - 24, DEST_Y)
    draw_arrow(d, arrow_start, arrow_end, ACCENT, width=5)

    # First-launch note centered below.
    d.text(
        (WIDTH // 2, HEIGHT - 56),
        "First launch only: right-click the installed app -> Open",
        fill=TEXT,
        font=subtitle_font,
        anchor="mm",
    )
    d.text(
        (WIDTH // 2, HEIGHT - 32),
        "(macOS one-time bypass for unsigned apps. Double-click works after.)",
        fill=SUBTEXT,
        font=note_font,
        anchor="mm",
    )

    out = "assets/dmg-background.png"
    img.save(out, "PNG", optimize=True)
    print(f"wrote {out} ({WIDTH}x{HEIGHT})")


if __name__ == "__main__":
    main()
