#!/usr/bin/env python3
"""Background tile atlas extractor for The Faery Tale Adventure (T2.2).

Each of the 10 regions (``file_index[]``, ``fmain.c:615-626``) draws its map
from ``image_mem``: 256 background tiles ("chars") in 5 bitplanes. This script
rebuilds ``image_mem`` for every region from the ADF and writes one atlas per
region into ``assets/tiles/region_<NN>/``:

* ``atlas_indexed.png``  -- 8-bit indexed PNG (palette ``pagecolors.json``),
* ``atlas_rgba.png``     -- the same pixels through the region's palette,
* ``atlas_highlightmask.png`` -- 1-bit, set where the index is 16..24,
* ``tiles.json``         -- tile rects, group/block provenance and citations.

Tile geometry, from the tile blitter ``next_image`` (``fsubs.asm:746-774``):
the char number is shifted left 6 (64 bytes per char per plane, ``:750``;
``IPLAN_SZ equ 16384 ; 256 * 64``, ``:661``) and the copy loop runs 16 times
moving one word (16 px) for 2 scanlines each (``:755-771``), so a tile is
**16 px wide by 32 scanlines tall** (``vsc equ 32``, ``fsubs.asm:1701``;
``img_y = map_y>>5``, ``fmain.c:1981``). Plane P of tile T lives at
``image_mem + P*IPLAN_SZ + T*64`` (``fsubs.asm:672-675, 750-761``).

Disk layout, from ``load_new_region`` (``fmain.c:3575-3591``): region R uses
the four 40-block image groups ``file_index[R].image[0..3]``; group g fills
tiles ``g*64 .. g*64+63`` (``imem0 += QPLAN_SZ`` = 4096 = 64 chars * 64 bytes,
``fmain.c:638``). Within a group, plane P is the 8 blocks (4096 bytes) at
``image[g] + 8*P`` (``fmain.c:3579-3587``). ``load_track_range`` reads block
``b`` at byte ``b*512`` (``hdrive.c:131,136``). Hence, in the concatenation of
the four groups, ``offset(T,P,R) = (T/64)*20480 + P*4096 + (T%64)*64 + R*2``.

Palette: ``assets/palettes/pagecolors.json`` (accepted T1.1), with colour 31
replaced per region from ``assets/palettes/region_overrides.json``
(``fade_page``, ``fmain2.c:381-386``: region 4 -> 0x0980, region 9 -> 0x0445,
else 0x0bdf). Index 31 is drawn **opaque** with that colour in both atlases:
tiles are the bottom layer and the game shows colour 31 there (that is the
whole point of the override); colour-31 keying applies only to things drawn on
top of the tiles, so the indexed PNG carries no tRNS (user decision, 2026-09-28).

Usage::

    python tools/extract_tiles.py                # all regions -> assets/tiles
    python tools/extract_tiles.py --region 7
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

import numpy as np

TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import asset_common as ac  # noqa: E402
import extract_masks as em  # noqa: E402
import extract_table as et  # noqa: E402

REPO_ROOT = TOOLS_DIR.parent
BLOCK_SIZE = 512          # hdrive.c:131,136
NUM_PLANES = 5            # fmain.c:640, fmain.c:3579-3587
TILE_W, TILE_H = 16, 32   # fsubs.asm:755-771, fsubs.asm:1701
TILE_BYTES = 64           # bytes per tile per plane, fsubs.asm:661,750
TILES_PER_GROUP = 64      # QPLAN_SZ / 64, fmain.c:638
PLANE_BYTES = 4096        # QPLAN_SZ, fmain.c:638 (8 blocks, fmain.c:3579)
GROUP_BYTES = NUM_PLANES * PLANE_BYTES   # 20480 = 40 blocks
NUM_GROUPS = 4            # file_index[].image[4], ftale.h:105; fmain.c:3576
NUM_TILES = NUM_GROUPS * TILES_PER_GROUP  # 256, fmain.c:639
ATLAS_COLS = 16
FILE_INDEX_FIELDS = ["image_0", "image_1", "image_2", "image_3", "terra1",
                     "terra2", "sector", "region", "setchar"]  # ftale.h:105


# --------------------------------------------------------------------------- #
# Source parsing
# --------------------------------------------------------------------------- #
def load_file_index(src_dir: Path) -> list[dict]:
    """``file_index[10]`` rows (fmain.c:615-626) with the ``/* Fn - name */`` comments."""
    table = et.extract_tables(str(src_dir / "fmain.c"))["file_index"]
    lines = (src_dir / "fmain.c").read_text(errors="replace").splitlines()
    regions = []
    n = table["line"]  # 1-based line of "struct need file_index[10] = {"
    for r, row in enumerate(table["values"]):
        n += 1
        while not re.match(r"\s*\{[^{}]*\}", lines[n - 1]):
            n += 1
        m = re.search(r"/\*\s*F(\d+)\s*-\s*(.*?)\s*\*/", lines[n - 1])
        entry = dict(zip(FILE_INDEX_FIELDS, row))
        entry.update(region=r, label=f"F{m.group(1)}", name=m.group(2),
                     source=f"src/fmain.c:{n}")
        regions.append(entry)
    return regions


def load_palette(pagecolors: Path, overrides: Path, region: int) -> tuple[list, dict, dict | None]:
    """pagecolors rgba8 list with colour 31 swapped per region (fmain2.c:381-386).

    Also returns the region's conditional colour-31 variant, if any: region 9 shows colour 31
    as 0x00f0 while ``secret_timer`` runs (Crystal Orb, fmain.c:3307), which is how hidden
    passages are revealed (fmain2.c:383)."""
    entries = json.loads(pagecolors.read_text())
    pal = [tuple(e["rgba8"]) for e in sorted(entries, key=lambda e: e["index"])]
    ov = json.loads(overrides.read_text())
    c31 = ov["regions"].get(str(region), ov["default"])
    pal[ov["color_index"]] = tuple(c31["rgba8"])
    return pal, c31, ov.get("conditional_regions", {}).get(str(region))


# --------------------------------------------------------------------------- #
# Pixel decoding
# --------------------------------------------------------------------------- #
def decode_group(image: bytes, block: int) -> np.ndarray:
    """One 40-block image group -> (64, 32, 16) uint8 palette indices.

    Group bytes are plane-major: plane P at P*4096, tile t at t*64, row r at r*2
    (fmain.c:3579-3587, fsubs.asm:750-771)."""
    raw = np.frombuffer(image, dtype=np.uint8, count=GROUP_BYTES, offset=block * BLOCK_SIZE)
    bits = np.unpackbits(raw.reshape(NUM_PLANES, TILES_PER_GROUP, TILE_H, TILE_W // 8), axis=3)
    idx = np.zeros((TILES_PER_GROUP, TILE_H, TILE_W), dtype=np.uint8)
    for plane in range(NUM_PLANES):
        idx |= bits[plane] << plane
    return idx


def decode_region(image: bytes, blocks: list[int]) -> np.ndarray:
    """All 256 tiles of a region, (256, 32, 16); group g holds tiles g*64.. (fmain.c:3576-3591)."""
    return np.concatenate([decode_group(image, b) for b in blocks])


def pack_atlas(tiles: np.ndarray) -> np.ndarray:
    """(256, 32, 16) -> (16 rows * 32, 16 cols * 16): tile T at column T % 16, row T // 16."""
    rows = NUM_TILES // ATLAS_COLS
    return tiles.reshape(rows, ATLAS_COLS, TILE_H, TILE_W).transpose(0, 2, 1, 3) \
                .reshape(rows * TILE_H, ATLAS_COLS * TILE_W)


# --------------------------------------------------------------------------- #
# Output
# --------------------------------------------------------------------------- #
def tiles_with_index(atlas: np.ndarray, index: int) -> list[int]:
    rows = NUM_TILES // ATLAS_COLS
    per_tile = (atlas == index).reshape(rows, TILE_H, ATLAS_COLS, TILE_W).any(axis=(1, 3))
    return [int(t) for t in np.flatnonzero(per_tile)]


def tile_masks(terra: list[dict], masks: np.ndarray) -> tuple[list[dict], np.ndarray]:
    """Per tile: the shadow mask entry (terra byte 0) and occlusion mode (byte 1 & 15), and the
    mask pixels the game applies -- entry `maptag` when the mode is non-zero, otherwise none
    (fmain.c:2577-2595). Returns (per-tile records, (256, 32, 16) bits)."""
    recs, bits = [], np.zeros((NUM_TILES, TILE_H, TILE_W), dtype=np.uint8)
    for t, e in enumerate(terra):
        applied = e["mask_mode"] != 0
        recs.append({"mask": e["maptag"] if applied else None, "mask_mode": e["mask_mode"]})
        if applied:
            bits[t] = masks[e["maptag"]]
    return recs, bits


def emit_region(entry: dict, image: bytes, palette: list, c31: dict, secret: dict | None,
                terra: list[dict], masks: np.ndarray, out: Path) -> dict:
    blocks = [entry[f"image_{g}"] for g in range(NUM_GROUPS)]
    atlas = pack_atlas(decode_region(image, blocks))
    mask_recs, mask_bits = tile_masks(terra, masks)
    out.mkdir(parents=True, exist_ok=True)
    ac.write_indexed_png(out / "atlas_indexed.png", atlas, palette, transparent=False)
    ac.write_rgba_png(out / "atlas_rgba.png", np.array(palette, dtype=np.uint8)[atlas])
    ac.write_highlight_mask(out / "atlas_highlightmask.png", atlas)
    ac.write_bit_mask(out / "atlas_shadowmask.png", pack_atlas(mask_bits))
    index31 = {"tiles": tiles_with_index(atlas, 31),
               "note": "Tiles containing palette index 31, the only colour fade_page changes per region "
                       "(and, in region 9, per secret_timer).",
               "source": "src/fmain2.c:381-386"}
    if secret:
        index31["secret_timer"] = {
            "condition": secret["condition"],
            "color_31": {"rgb4": secret["rgb4"], "rgba8": list(secret["rgba8"])},
            "note": "While secret_timer runs (Crystal Orb, magic case 8 fmain.c:3307; decremented "
                    "fmain.c:1381) colour 31 is this value: the hidden passages become visible. "
                    "Swap palette entry 31 on the indexed atlas, or use the variant tiles in the master atlas.",
            "source": "src/fmain2.c:382-384, src/fmain.c:3307, src/fmain.c:1381"}

    meta = {
        "region": entry["region"],
        "label": entry["label"],
        "name": entry["name"],
        "source": entry["source"],
        "tile_size": [TILE_W, TILE_H],
        "tile_size_source": "src/fsubs.asm:750-771, src/fsubs.asm:1701, src/fmain.c:1981",
        "tile_count": NUM_TILES,
        "planes": NUM_PLANES,
        "atlas": {"file": "atlas_indexed.png", "rgba": "atlas_rgba.png",
                  "highlight_mask": "atlas_highlightmask.png", "shadow_mask": "atlas_shadowmask.png",
                  "columns": ATLAS_COLS, "rows": NUM_TILES // ATLAS_COLS,
                  "width": ATLAS_COLS * TILE_W, "height": (NUM_TILES // ATLAS_COLS) * TILE_H},
        "layout": {
            "formula": "offset(T,P,R) = (T/64)*20480 + P*4096 + (T%64)*64 + R*2 "
                       "within the concatenation of the four image groups",
            "source": "src/fmain.c:3575-3591, src/fmain.c:638-640, src/fsubs.asm:661, "
                      "src/fsubs.asm:750-761, src/hdrive.c:131",
        },
        "groups": [{"index": g, "block": b, "byte_offset": b * BLOCK_SIZE,
                    "tiles": [g * TILES_PER_GROUP, (g + 1) * TILES_PER_GROUP - 1]}
                   for g, b in enumerate(blocks)],
        "palette": {
            "file": "palettes/pagecolors.json",
            "color_31": {"rgb4": c31["rgb4"], "rgba8": list(c31["rgba8"]),
                         "source": "src/fmain2.c:381-386"},
            "index_31": "opaque (no key colour) in both atlases: tiles are the background, "
                        "colour-31 keying applies only to things drawn on top of them",
        },
        "highlight_mask": {"indices": [ac.HIGHLIGHT_LO, ac.HIGHLIGHT_HI],
                           "source": "src/fmain2.c:412-413"},
        "index_31": index31,
        "shadow_mask": {
            "note": "atlas_shadowmask.png holds, in the tile's cell, the 16x32 shadow mask (assets/masks/, "
                    "entry = terra byte 0 'maptag') the game applies over sprites standing behind the tile, "
                    "or nothing when the tile's occlusion mode (terra byte 1 & 15) is 0. Set bit = terrain in "
                    "front. The mode decides when the mask is applied (fmain.c:2584-2594): 1 skip when xm==0, "
                    "2 skip when ystop>35, 3 always (except the bridge in sector 48), 4 skip when xm==0 or "
                    "ystop>35, 5 skip when xm==0 and ystop>35, 6 use tile 64's mask when ym!=0, 7 skip when "
                    "ystop>20; a falling actor uses mode 3. Per tile: tiles[].mask / mask_mode.",
            "source": "src/fmain.c:2577-2595, src/fsubs.asm:1047-1083",
            "terra_source": f"assets/world/region_{entry['region']}.json terra.entries",
        },
        "tiles": [{"index": t, "x": (t % ATLAS_COLS) * TILE_W, "y": (t // ATLAS_COLS) * TILE_H,
                   "w": TILE_W, "h": TILE_H, "group": t // TILES_PER_GROUP,
                   "block": blocks[t // TILES_PER_GROUP], **mask_recs[t]} for t in range(NUM_TILES)],
    }
    ac.write_json(out / "tiles.json", meta)
    return meta


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    ac.add_io_args(parser)
    parser.add_argument("--out-dir", type=Path, default=REPO_ROOT / "assets" / "tiles")
    parser.add_argument("--palette-dir", type=Path, default=REPO_ROOT / "assets" / "palettes")
    parser.add_argument("--world-dir", type=Path, default=REPO_ROOT / "assets" / "world",
                        help="T2.5 region JSONs (terra entries -> per-tile shadow mask + mode)")
    parser.add_argument("--region", type=int, default=None, help="region 0-9 (default: all)")
    args = parser.parse_args(argv)

    image = (args.game_dir / "image").read_bytes()
    first, count, _ = em.mask_blocks(args.src_dir)
    masks = em.decode_masks(image, first, count)
    for entry in load_file_index(args.src_dir):
        r = entry["region"]
        if args.region is not None and r != args.region:
            continue
        palette, c31, secret = load_palette(args.palette_dir / "pagecolors.json",
                                            args.palette_dir / "region_overrides.json", r)
        terra = json.loads((args.world_dir / f"region_{r}.json").read_text())["terra"]["entries"]
        emit_region(entry, image, palette, c31, secret, terra, masks, args.out_dir / f"region_{r:02d}")
        print(f"region_{r:02d}: {entry['label']} {entry['name']} "
              f"blocks {[entry[f'image_{g}'] for g in range(NUM_GROUPS)]} colour31 {c31['rgb4']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
