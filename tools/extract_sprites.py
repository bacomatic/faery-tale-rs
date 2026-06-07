#!/usr/bin/env python3
"""
Sprite extractor for Faery Tale Adventure (Amiga)
Reads raw bitplane data from game/image and outputs PNG files.

Format: 5 Amiga bitplanes, planar layout (all frames per plane sequentially).
        Width is in 16-px words; height in scanlines.
        Color 31 (all bits set) is transparent (alpha=0).
        Palette from pagecolors[] in fmain2.c:367-371.
"""

import sys
import struct
import os
from PIL import Image

GAME_IMAGE = os.path.join(os.path.dirname(__file__), "../game/image")
BLOCK_SIZE = 512
NUM_PLANES = 5

# pagecolors[] from fmain2.c:367-371 — Amiga 4-bit RGB (0x0RGB), expanded to 8-bit
_RAW = [
    0x0000, 0x0FFF, 0x0E96, 0x0B63, 0x0631, 0x07BF, 0x0333, 0x0DB8,
    0x0223, 0x0445, 0x0889, 0x0BBC, 0x0521, 0x0941, 0x0F82, 0x0FC7,
    0x0040, 0x0070, 0x00B0, 0x06F6, 0x0005, 0x0009, 0x000D, 0x037F,
    0x0C00, 0x0F50, 0x0FA0, 0x0FF6, 0x0EB6, 0x0EA5, 0x000F, 0x0BDF,
]

def _expand4(v):
    """Expand 4-bit channel to 8-bit: 0xF -> 0xFF, 0x0 -> 0x00."""
    return (v << 4) | v

PALETTE = [
    (_expand4((c >> 8) & 0xF), _expand4((c >> 4) & 0xF), _expand4(c & 0xF))
    for c in _RAW
]
TRANSPARENT_INDEX = 31  # color 31 = all planes set = transparent

# cfiles[] metadata from fmain2.c:643-665
# (index, name, width_words, height, count, file_id)
CFILES = [
    (0,  "julian",          1, 32,  67, 1376),
    (1,  "phillip",         1, 32,  67, 1418),
    (2,  "kevin",           1, 32,  67, 1460),
    (3,  "objects",         1, 16, 116, 1312),
    (4,  "raft",            2, 32,   2, 1348),
    (5,  "turtle",          2, 32,  16, 1351),
    (6,  "ogre",            1, 32,  64,  960),
    (7,  "ghost",           1, 32,  64, 1080),
    (8,  "dark_knight",     1, 32,  64, 1000),
    (9,  "necromancer",     1, 32,  64, 1040),
    (10, "dragon",          3, 40,   5, 1160),
    (11, "bird",            4, 64,   8, 1120),
    (12, "snake",           1, 32,  64, 1376),
    (13, "wizard_priest",   1, 32,   8,  936),
    (14, "royal",           1, 32,   8,  931),
    (15, "bartender",       1, 32,   8,  941),
    (16, "witch",           1, 32,   8,  946),
    (17, "ranger_beggar",   1, 32,   8,  951),
]


def read_sprite_data(image_data, file_id, width_words, height, count):
    """Read raw 5-plane sprite data from game/image for one cfiles entry."""
    bytes_per_plane_per_frame = width_words * height * 2  # = height * width_words * 2
    total_per_plane = bytes_per_plane_per_frame * count
    total = total_per_plane * NUM_PLANES
    offset = file_id * BLOCK_SIZE
    return image_data[offset: offset + total]


def decode_frame(raw, frame_idx, width_words, height, bytes_per_plane_per_frame, total_per_plane):
    """
    Decode one frame from raw 5-plane data into an RGBA PIL Image.

    Layout: [plane0: all frames contiguous] [plane1: ...] ... [plane4: ...]
    For frame N, plane P: raw[P*total_per_plane + N*bytes_per_plane_per_frame ...]
    Each frame's plane data is `height` rows of `width_words` words (2 bytes each).
    """
    width_px = width_words * 16
    pixels = []

    for y in range(height):
        for x in range(width_px):
            word_idx = x // 16
            bit_pos = 15 - (x % 16)  # MSB first

            pixel = 0
            for plane in range(NUM_PLANES):
                plane_base = plane * total_per_plane
                frame_base = frame_idx * bytes_per_plane_per_frame
                row_offset = y * width_words * 2 + word_idx * 2
                byte_offset = plane_base + frame_base + row_offset

                if byte_offset + 1 < len(raw):
                    word = (raw[byte_offset] << 8) | raw[byte_offset + 1]
                    bit = (word >> bit_pos) & 1
                    pixel |= (bit << plane)

            # Alpha: transparent if color index == 31
            r, g, b = PALETTE[pixel]
            a = 0 if pixel == TRANSPARENT_INDEX else 255
            pixels.append((r, g, b, a))

    img = Image.new("RGBA", (width_px, height))
    img.putdata(pixels)
    return img


def extract_sheet(image_data, entry, out_dir, frames=None):
    """
    Extract all (or specified) frames from one cfiles entry.
    frames: list of frame indices to extract, or None for all.
    """
    idx, name, width_words, height, count, file_id = entry
    raw = read_sprite_data(image_data, file_id, width_words, height, count)

    bytes_per_plane_per_frame = width_words * height * 2
    total_per_plane = bytes_per_plane_per_frame * count

    sheet_dir = os.path.join(out_dir, name)
    os.makedirs(sheet_dir, exist_ok=True)

    to_extract = frames if frames is not None else range(count)
    extracted = []
    for frame_idx in to_extract:
        if frame_idx >= count:
            print(f"  WARNING: frame {frame_idx} out of range (count={count}), skipping")
            continue
        img = decode_frame(raw, frame_idx, width_words, height,
                           bytes_per_plane_per_frame, total_per_plane)
        path = os.path.join(sheet_dir, f"frame_{frame_idx:03d}.png")
        img.save(path)
        extracted.append(path)

    return extracted


def make_spritesheet(image_data, entry, out_dir, frames=None, cols=16, scale=1):
    """
    Also produce a combined sprite sheet PNG for easy visual review.
    """
    idx, name, width_words, height, count, file_id = entry
    raw = read_sprite_data(image_data, file_id, width_words, height, count)

    bytes_per_plane_per_frame = width_words * height * 2
    total_per_plane = bytes_per_plane_per_frame * count

    to_extract = list(frames) if frames is not None else list(range(count))
    if not to_extract:
        return None

    width_px = width_words * 16
    rows = (len(to_extract) + cols - 1) // cols
    sheet = Image.new("RGBA", (width_px * cols * scale, height * rows * scale), (0, 0, 0, 0))

    for i, frame_idx in enumerate(to_extract):
        if frame_idx >= count:
            continue
        img = decode_frame(raw, frame_idx, width_words, height,
                           bytes_per_plane_per_frame, total_per_plane)
        if scale > 1:
            img = img.resize((width_px * scale, height * scale), Image.NEAREST)
        col = i % cols
        row = i // cols
        sheet.paste(img, (col * width_px * scale, row * height * scale))

    path = os.path.join(out_dir, f"{name}_sheet.png")
    sheet.save(path)
    return path


# Unknown OBJECTS frames from reference/data/sprites/objects.md
UNKNOWN_OBJECTS = [
    0, 1, 2, 4, 5, 6, 7, 24, 28, 29,
    80, 82, 84, 85, 86, 87, 89, 90, 91, 92,
    93, 94, 95, 96, 99, 111, 112, 113, 115,
]

CFILES_BY_NAME = {e[1]: e for e in CFILES}
CFILES_BY_IDX  = {e[0]: e for e in CFILES}


def main():
    import argparse
    parser = argparse.ArgumentParser(
        description="Extract Faery Tale Adventure sprites from game/image")
    parser.add_argument("--image", default=GAME_IMAGE,
                        help="Path to game/image binary")
    parser.add_argument("--out", default="sprite_output",
                        help="Output directory")
    parser.add_argument("--sheet", choices=[e[1] for e in CFILES] + ["all"],
                        default="objects",
                        help="Which sprite sheet to extract (default: objects)")
    parser.add_argument("--unknown-only", action="store_true",
                        help="For objects sheet: extract only the 30 unknown frames")
    parser.add_argument("--frames", type=str, default=None,
                        help="Comma-separated frame indices to extract (overrides --unknown-only)")
    parser.add_argument("--spritesheet", action="store_true",
                        help="Also produce a combined sprite sheet PNG")
    args = parser.parse_args()

    with open(args.image, "rb") as f:
        image_data = f.read()
    print(f"Loaded game/image: {len(image_data):,} bytes ({len(image_data)//512} blocks)")

    if args.sheet == "all":
        entries = CFILES
    else:
        entries = [CFILES_BY_NAME[args.sheet]]

    for entry in entries:
        idx, name, width_words, height, count, file_id = entry
        print(f"\n--- {name} (cfiles[{idx}]): {width_words*16}×{height}px, {count} frames, block {file_id} ---")

        frames = None
        if args.frames:
            frames = [int(x.strip()) for x in args.frames.split(",")]
        elif args.unknown_only and name == "objects":
            frames = UNKNOWN_OBJECTS

        extracted = extract_sheet(image_data, entry, args.out, frames=frames)
        print(f"  Extracted {len(extracted)} frames → {args.out}/{name}/")

        if args.spritesheet:
            path = make_spritesheet(image_data, entry, args.out,
                                    frames=frames, cols=16)
            if path:
                print(f"  Sprite sheet → {path}")


if __name__ == "__main__":
    main()
