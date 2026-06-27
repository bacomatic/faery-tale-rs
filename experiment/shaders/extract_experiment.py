#!/usr/bin/env python3
"""
extract_experiment.py — build the day/night RGBA experiment bundle.

Pipeline:
  ADF (src/assets/image)
    -> indexed hero frames (cfile julian) + F8 terrain tiles (256, 16x16)
    -> daynight_lut.json  (the concrete RGB table: per light level x 32 entries)
    -> frames/  full_bright base + per-level baked RGBA bank + highlight masks

Nothing here is "rendered" with effects baked creatively — every output pixel is
the original indexed pixel run through fade_page.py (the verbatim port of the
original fade_page()). compare.py proves that bit-exactly.

Decode logic is reused/ported from tools/extract_sprites.py (sprites) and the
tile-atlas offset formula documented in assets/plan.md item 2.
"""

import argparse
import json
import os

from PIL import Image

import fade_page as fp

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_IMAGE = os.path.join(HERE, "../../src/assets/image")
BLOCK_SIZE = 512
NUM_PLANES = 5

# Hero sprite: cfile julian (CFILES[0], fmain2.c:643-665): 16x32, 67 frames, blk 1376.
HERO = {"name": "julian", "file_id": 1376, "width_words": 1, "height": 32, "count": 67}
# Default representative poses (all valid frame indices < 67); --all-frames overrides.
HERO_DEFAULT_FRAMES = [0, 8, 16, 24, 32, 44, 56, 64]

# Terrain: region F8 "forest and wilderness" (file_index image groups), region_num 7.
# Vegetation-heavy => exercises palette indices 16-24 (the contested veg boost).
TERRAIN = {"name": "f8_forest", "region_num": 7, "groups": [320, 280, 240, 200],
           "tiles": 256, "tile_w": 16, "tile_h": 16}
GROUP_BYTES = 20480   # 40 blocks per group (5 planes x 8 blocks); plane stride 4096

TRANSPARENT_INDEX = 31


# ---------------------------------------------------------------------------
# Indexed decoders (return list[list[int]] of palette indices 0..31)
# ---------------------------------------------------------------------------

def load_adf(path):
    with open(path, "rb") as f:
        return f.read()


def decode_hero_indices(data, frame_idx):
    """Decode one julian frame to a height x width grid of palette indices.
    Planar-per-frame, 5 planes, MSB-first (tools/extract_sprites.py:72-114)."""
    ww, h = HERO["width_words"], HERO["height"]
    width_px = ww * 16
    bpp_frame = ww * h * 2                      # bytes per plane per frame
    bytes_per_frame = NUM_PLANES * bpp_frame
    base = HERO["file_id"] * BLOCK_SIZE + frame_idx * bytes_per_frame
    grid = []
    for y in range(h):
        row = []
        for x in range(width_px):
            word_idx = x // 16
            bit_pos = 15 - (x % 16)
            pix = 0
            for plane in range(NUM_PLANES):
                off = base + plane * bpp_frame + (y * ww * 2 + word_idx * 2)
                word = (data[off] << 8) | data[off + 1]
                pix |= ((word >> bit_pos) & 1) << plane
            row.append(pix)
        grid.append(row)
    return grid


def build_terrain_mem(data):
    """Concatenate the 4 F8 image groups into one image_mem (matches the
    (T//64)*20480 term in the tile offset formula)."""
    mem = bytearray()
    for gb in TERRAIN["groups"]:
        start = gb * BLOCK_SIZE
        mem += data[start:start + GROUP_BYTES]
    return mem


def decode_tile_indices(mem, T):
    """Decode tile T to a 16x16 grid of palette indices.
    offset(T,P,R) = (T//64)*20480 + P*4096 + (T%64)*64 + R*2  (assets/plan.md item 2)."""
    h = TERRAIN["tile_h"]
    grid = []
    for R in range(h):
        row = []
        for x in range(16):
            bit_pos = 15 - x
            pix = 0
            for P in range(NUM_PLANES):
                off = (T // 64) * 20480 + P * 4096 + (T % 64) * 64 + R * 2
                word = (mem[off] << 8) | mem[off + 1]
                pix |= ((word >> bit_pos) & 1) << P
            row.append(pix)
        grid.append(row)
    return grid


# ---------------------------------------------------------------------------
# Baking
# ---------------------------------------------------------------------------

def palette_rgba(amiga_pal):
    """12-bit palette -> list[32] of (r,g,b,a) with idx31 transparent."""
    return [fp.rgb4_to_rgba8(amiga_pal[i], i) for i in range(32)]


def bake_image(grid, rgba_pal):
    h, w = len(grid), len(grid[0])
    img = Image.new("RGBA", (w, h))
    img.putdata([rgba_pal[grid[y][x]] for y in range(h) for x in range(w)])
    return img


def highlight_mask_image(grid):
    """1-bit highlight mask: R=255 where index in 16..24, else 0. Alpha follows
    sprite transparency so the mask lines up with the base RGBA."""
    h, w = len(grid), len(grid[0])
    img = Image.new("RGBA", (w, h))
    data = []
    for y in range(h):
        for x in range(w):
            idx = grid[y][x]
            veg = 255 if fp.VEG_LO <= idx <= fp.VEG_HI else 0
            a = 0 if idx == TRANSPARENT_INDEX else 255
            data.append((veg, veg, veg, a))
    img.putdata(data)
    return img


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser(description="Build day/night RGBA experiment bundle")
    ap.add_argument("--image", default=DEFAULT_IMAGE, help="Path to ADF (game/image)")
    ap.add_argument("--out", default=os.path.join(HERE, "frames"), help="Output frames dir")
    ap.add_argument("--lut", default=os.path.join(HERE, "daynight_lut.json"))
    ap.add_argument("--all-frames", action="store_true", help="Bake all 67 hero frames")
    args = ap.parse_args()

    data = load_adf(args.image)
    print(f"Loaded ADF: {len(data):,} bytes ({len(data)//BLOCK_SIZE} blocks)")

    region_num = TERRAIN["region_num"]
    levels = fp.CANONICAL_LEVELS

    # --- LUT: the concrete RGB table -------------------------------------
    lut = {"source": "fade_page() src/fmain2.c:377-419 via fade_page.py",
           "region_num": region_num, "levels": {}}
    palettes = {}
    for lv in levels:
        amiga = fp.fade_palette_at(lv, region_num=region_num)
        palettes[lv] = amiga
        entries = []
        for i in range(32):
            r, g, b, _ = fp.rgb4_to_rgba8(amiga[i], i)
            entries.append({"index": i, "rgb4": f"0x{amiga[i]:03X}",
                            "rgba8": [r, g, b, 255 if i != 31 else 0]})
        lut["levels"][str(lv)] = entries
    with open(args.lut, "w") as f:
        json.dump(lut, f, indent=2)
    print(f"Wrote LUT -> {os.path.relpath(args.lut, HERE)} "
          f"({len(levels)} levels x 32 entries)")

    rgba_pals = {lv: palette_rgba(palettes[lv]) for lv in levels}
    full_pal = palette_rgba(fp.fade_palette_at(fp.FULL_BRIGHT_LEVEL, region_num=region_num))

    # --- Hero ------------------------------------------------------------
    hero_frames = range(HERO["count"]) if args.all_frames else HERO_DEFAULT_FRAMES
    _bake_subject("hero", args.out, hero_frames,
                  lambda i: decode_hero_indices(data, i), levels, rgba_pals, full_pal,
                  fmt="frame_{:03d}.png")

    # --- Terrain ---------------------------------------------------------
    mem = build_terrain_mem(data)
    _bake_subject("terrain", args.out, range(TERRAIN["tiles"]),
                  lambda t: decode_tile_indices(mem, t), levels, rgba_pals, full_pal,
                  fmt="tile_{:03d}.png")

    print("Done.")


def _bake_subject(name, out, ids, decode, levels, rgba_pals, full_pal, fmt):
    root = os.path.join(out, name)
    dirs = {"full_bright": os.path.join(root, "full_bright"),
            "highlight_mask": os.path.join(root, "highlight_mask")}
    for lv in levels:
        dirs[lv] = os.path.join(root, f"L{lv}")
    for d in dirs.values():
        os.makedirs(d, exist_ok=True)

    n = 0
    for i in ids:
        grid = decode(i)
        fn = fmt.format(i)
        bake_image(grid, full_pal).save(os.path.join(dirs["full_bright"], fn))
        highlight_mask_image(grid).save(os.path.join(dirs["highlight_mask"], fn))
        for lv in levels:
            bake_image(grid, rgba_pals[lv]).save(os.path.join(dirs[lv], fn))
        n += 1
    print(f"  {name}: {n} subjects x ({len(levels)} levels + full_bright + highlight_mask) "
          f"-> {os.path.relpath(root, HERE)}/")


if __name__ == "__main__":
    main()
