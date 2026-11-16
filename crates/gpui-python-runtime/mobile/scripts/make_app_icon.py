#!/usr/bin/env python3
"""Render the default GPUI Python app icon (1024x1024 PNG).

Run with Pillow available, e.g.:
    uv run --no-project --with pillow \
        python3 scripts/make_app_icon.py [output-path]

Generated projects can replace the result with real artwork: drop an
`icon-1024.png` next to `main_app.py` and the stage scripts prefer it.
"""

from pathlib import Path
import sys

from PIL import Image, ImageDraw, ImageFont

SIZE = 1024
BACKGROUND = (24, 24, 27)
ACCENT = (0, 122, 204)
TEXT = (255, 255, 255)

# Bundled with gpui-miniapp; relative to the workspace root.
FONT_CANDIDATES = (
    "crates/assets/fonts/ibm-plex-sans/IBMPlexSans-SemiBold.ttf",
    "assets/fonts/ibm-plex-sans/IBMPlexSans-SemiBold.ttf",
)


def find_font(size: int) -> ImageFont.FreeTypeFont:
    here = Path(__file__).resolve()
    for root in here.parents:
        for candidate in FONT_CANDIDATES:
            path = root / candidate
            if path.is_file():
                return ImageFont.truetype(str(path), size=size)
    return ImageFont.load_default(size=size)


def main() -> int:
    output = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("icon-1024.png")
    image = Image.new("RGB", (SIZE, SIZE), BACKGROUND)
    draw = ImageDraw.Draw(image)
    margin = 96
    draw.rounded_rectangle(
        (margin, margin, SIZE - margin, SIZE - margin), radius=180, fill=ACCENT
    )
    font = find_font(430)
    draw.text((SIZE / 2, SIZE / 2), "Py", font=font, fill=TEXT, anchor="mm")
    output.parent.mkdir(parents=True, exist_ok=True)
    image.save(output)
    print(f"wrote {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
