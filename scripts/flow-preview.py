#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = ["pillow>=10"]
# ///
"""Render contact sheets from the Rust preview's cells, using Flow's 2x4 masks.

Run the ignored Rust preview first (src/flow/preview.rs), then:
    uv run scripts/flow-preview.py /tmp/flow-preview/frames.json

The HTML artifact shows actual terminal glyphs. These PNG sheets use the same
quadrant/braille masks as upstream's desktop renderer, for compact review.
"""
import argparse
import json
import math
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

CLEAR = 0x01000000
QUAD = [0x20, 0x2598, 0x259D, 0x2580, 0x2596, 0x258C, 0x259E, 0x259B,
        0x2597, 0x259A, 0x2590, 0x259C, 0x2584, 0x2599, 0x259F, 0x2588]
BRAILLE = [[1, 2, 4, 64], [8, 16, 32, 128]]
SHADE = {0x2591: 0.3, 0x2592: 0.55, 0x2593: 0.8}
SHAPES = {0x2500: 0b00110000, 0x2502: 0b01010101, 0x2571: 0b01011010,
          0x2572: 0b10100101, 0xB7: 0b00000100, 0x2A: 0b00111100,
          0x25CF: 0b00111100, 0x2663: 0b01111111, 0x2302: 0b11111100,
          0x7E: 0b00100100, 0x76: 0b00001100, 0x5E: 0b00110000}
BACKGROUND = (16, 21, 31)


def coverage(cp):
    if cp in (0x20, 0xA0):
        return 0
    if cp in QUAD:
        q = QUAD.index(cp)
        return ((5 if q & 1 else 0) | (10 if q & 2 else 0)
                | (80 if q & 4 else 0) | (160 if q & 8 else 0))
    if 0x2580 < cp < 0x2588:
        rows = math.floor((cp - 0x2580) / 2 + 0.5)
        return (0xFF << ((4 - rows) * 2)) & 0xFF
    if 0x2588 < cp < 0x2590:
        columns = math.floor((0x2590 - cp) / 4 + 0.5)
        return 0xFF if columns >= 2 else 0x55 if columns == 1 else 0
    if 0x2800 <= cp <= 0x28FF:
        return sum(1 << (x + 2 * y) for x in range(2) for y in range(4)
                   if (cp - 0x2800) & BRAILLE[x][y])
    return SHAPES.get(cp, -1)


def rgb(c):
    return BACKGROUND if c == CLEAR else ((c >> 16) & 255, (c >> 8) & 255, c & 255)


def mix(a, b, k):
    return tuple(math.floor(x + (y - x) * k + 0.5) for x, y in zip(a, b))


def frame_image(frame):
    w, h = frame["columns"], frame["rows"]
    image = Image.new("RGB", (w * 2, h * 4), BACKGROUND)
    for i in range(w * h):
        cp, fg, bg = frame["words"][i * 3:i * 3 + 3]
        mask = coverage(cp)
        for y in range(4):
            for x in range(2):
                if cp in SHADE:
                    color = mix(rgb(bg), rgb(fg), SHADE[cp])
                elif mask >= 0:
                    color = rgb(fg if mask & (1 << (x + 2 * y)) else bg)
                else:
                    color = rgb(fg if y in (1, 2) else bg)
                image.putpixel(((i % w) * 2 + x, (i // w) * 4 + y), color)
    return image


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("frames", type=Path)
    args = parser.parse_args()
    frames = json.loads(args.frames.read_text())
    scenes = list(dict.fromkeys(f["scene"] for f in frames))
    modes = list(dict.fromkeys(f["mode"] for f in frames))
    font = ImageFont.load_default(size=16)
    scale = 4
    width, row_height = 1120, 224
    for mode in modes:
        sheet = Image.new("RGB", (width, 55 + row_height * len(scenes)), BACKGROUND)
        draw = ImageDraw.Draw(sheet)
        draw.text((16, 16), f"Flow Rust: {mode} | 15x4, 24x8, 40x12", font=font, fill="#d9e2f1")
        for j, scene in enumerate(scenes):
            y = 55 + j * row_height
            draw.text((16, y + 4), scene, font=font, fill="#d9e2f1")
            for x, size in [(150, (15, 4)), (340, (24, 8)), (600, (40, 12))]:
                f = next(f for f in frames if f["scene"] == scene and f["mode"] == mode
                         and (f["columns"], f["rows"]) == size)
                image = frame_image(f)
                image = image.resize((image.width * scale, image.height * scale), Image.Resampling.NEAREST)
                sheet.paste(image, (x, y + 24))
                draw.text((x, y + 2), f"{size[0]}x{size[1]} ({f['mean_ms']:.3f} ms)", font=font, fill="#a0aec5")
        path = args.frames.parent / f"{mode}.png"
        sheet.save(path)
        print(path)


if __name__ == "__main__":
    main()
