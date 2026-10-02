#!/usr/bin/env python3
"""T2.2.1 -- master tile atlas: every tile the game can draw, deduplicated.

Inputs are the accepted per-region atlases (T2.2, ``assets/tiles/region_NN/atlas_indexed.png``)
and the world decoded from the ADF (``tools/decode_map_data.load_regions``), segmented into the
shipped maps by ``tools/extract_maps.py`` (T2.5).

A tile is *used* by a region when it appears in a shipped map drawn with that region's tileset:
the whole overworld for regions 0-7 (``src/fsubs.asm:565-604``, ``fmain.c:3557-3562``), and for
regions 8/9 -- which share one map and sector pool (``fmain.c:624-625``) -- the interior spaces
reached with that region loaded (``doorlist[].secs``, ``fmain.c:1926``; stargate
``fmain.c:2632-2637``; quicksand drop ``fmain.c:1784-1789``).

Used tiles are deduplicated by palette-index content. The key also carries the region's colour-31
value (``src/fmain2.c:381-386``), the shadow mask entry + occlusion mode (``fmain.c:2577-2595``)
and the collision record -- feature type + sub-tile mask (``fsubs.asm:607-614``) -- so every
master tile is a complete 1:1 stand-in for an original tile id: art, mask, and walkability.
``master.json`` records each master tile's sources and, per region, a 256-entry
``tile index -> master index`` reference map (``null`` = never drawn).

Usage::

    python tools/extract_master_atlas.py            # -> assets/tiles/master
    python tools/extract_master_atlas.py --out-dir /tmp/master
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import asset_common as ac  # noqa: E402
import extract_maps as em  # noqa: E402

REPO_ROOT = TOOLS_DIR.parent
TILE_W, TILE_H = 16, 32       # fsubs.asm:755-771, fsubs.asm:1701
SRC_COLS = 16                 # T2.2 atlas layout
NUM_TILES = 256               # fmain.c:639
NUM_REGIONS = 10              # file_index[10], fmain.c:615
SECTOR_ROWS, SECTOR_COLS = 8, 16   # fsubs.asm:596-602
MASTER_COLS = 32


def load_palette(palettes: Path) -> tuple[list[tuple[int, ...]], dict[int, dict]]:
    pal = [tuple(e["rgba8"]) for e in sorted(json.loads((palettes / "pagecolors.json").read_text()),
                                             key=lambda e: e["index"])]
    ov = json.loads((palettes / "region_overrides.json").read_text())
    return pal, {"default": ov["default"], "regions": {int(k): v for k, v in ov["regions"].items()},
                 "secret": {int(k): v for k, v in ov.get("conditional_regions", {}).items()}}


def color_31(overrides: dict, region: int) -> dict:
    return overrides["regions"].get(region) or overrides["default"]


def split_tiles(a: np.ndarray) -> np.ndarray:
    rows = a.shape[0] // TILE_H
    return a.reshape(rows, TILE_H, SRC_COLS, TILE_W).transpose(0, 2, 1, 3).reshape(-1, TILE_H, TILE_W)


def region_tiles(tiles: Path, region: int) -> np.ndarray:
    return split_tiles(np.array(Image.open(tiles / f"region_{region:02d}/atlas_indexed.png")))


def region_shadow(tiles: Path, region: int) -> tuple[np.ndarray, list[dict]]:
    """(256, 32, 16) shadow-mask bits and the per-tile mask/mask_mode records of a T2.2 atlas."""
    bits = split_tiles(np.array(Image.open(tiles / f"region_{region:02d}/atlas_shadowmask.png").convert("L")) > 0)
    recs = json.loads((tiles / f"region_{region:02d}/tiles.json").read_text())["tiles"]
    return bits.astype(np.uint8), recs


def build(tiles: Path, palettes: Path, src_dir: Path, game_dir: Path) -> tuple[np.ndarray, np.ndarray, np.ndarray, dict]:
    pal, overrides = load_palette(palettes)
    world = em.World(game_dir, src_dir)
    spaces = em.finish_spaces(world, em.segment(world))
    used_by_region = em.used_tiles(world, spaces)
    masters: list[dict] = []
    pixels: list[np.ndarray] = []
    shadows: list[np.ndarray] = []
    key_to_master: dict[tuple, int] = {}
    region_maps: dict[str, list] = {}
    regions_meta: dict[str, dict] = {}

    for region in range(NUM_REGIONS):
        used = used_by_region[region]
        terra = world.regions[region]["terra"]
        meta = {"name": world.regions[region]["name"],
                "maps": sorted({sp["name"] for sp in spaces if sp["region"] == region}) if region >= 8 else ["overworld"]}
        art = region_tiles(tiles, region)
        shadow_bits, shadow_recs = region_shadow(tiles, region)
        c31 = color_31(overrides, region)
        ref = [None] * NUM_TILES
        for t in used.tolist():
            px = art[t]
            has31 = bool((px == 31).any())
            mask, mode = shadow_recs[t]["mask"], shadow_recs[t]["mask_mode"]
            ftype, smask = terra[t]["feature_type"], terra[t]["subtile_mask"]
            # same art but a different shadow mask / occlusion mode / collision is a different master tile
            key = (px.tobytes(), c31["rgb4"] if has31 else None, mask, mode, ftype, smask)
            m = key_to_master.get(key)
            if m is None:
                m = len(masters)
                key_to_master[key] = m
                pixels.append(px)
                shadows.append(shadow_bits[t])
                masters.append({"index": m, "x": (m % MASTER_COLS) * TILE_W, "y": (m // MASTER_COLS) * TILE_H,
                                "w": TILE_W, "h": TILE_H, "uses_index_31": has31,
                                "color_31": c31 if has31 else None, "mask": mask, "mask_mode": mode,
                                "feature_type": ftype, "subtile_mask": smask, "sources": []})
            masters[m]["sources"].append({"region": region, "tile": t})
            ref[t] = m
        region_maps[str(region)] = ref
        regions_meta[str(region)] = {**meta, "tiles_used": int(len(used)),
                                     "tiles_unused": NUM_TILES - int(len(used)), "color_31": c31}

    # secret_timer variants: a tile drawn (with index 31) in a region whose colour 31 is conditional
    # gets a second master tile -- the same indices verbatim, colour 31 = the timer value in the
    # RGBA atlas (fmain2.c:382-384).
    secret_variants = []
    for m in [m for m in masters if m["uses_index_31"]]:
        cond = next((overrides["secret"][s["region"]] for s in m["sources"] if s["region"] in overrides["secret"]), None)
        if cond is None:
            continue
        v = len(masters)
        pixels.append(pixels[m["index"]])
        shadows.append(shadows[m["index"]])
        masters.append({"index": v, "x": (v % MASTER_COLS) * TILE_W, "y": (v // MASTER_COLS) * TILE_H,
                        "w": TILE_W, "h": TILE_H, "uses_index_31": True,
                        "color_31": {"rgb4": cond["rgb4"], "rgba8": list(cond["rgba8"])},
                        "mask": m["mask"], "mask_mode": m["mask_mode"],
                        "feature_type": m["feature_type"], "subtile_mask": m["subtile_mask"],
                        "variant_of": m["index"], "condition": cond["condition"], "sources": []})
        m["secret_variant"] = v
        secret_variants.append({"tile": m["index"], "variant": v, "sources": m["sources"]})

    rows = -(-len(masters) // MASTER_COLS)
    atlas = np.zeros((rows * TILE_H, MASTER_COLS * TILE_W), dtype=np.uint8)
    shadow = np.zeros_like(atlas)
    for m, (px, sh) in enumerate(zip(pixels, shadows)):
        x, y = (m % MASTER_COLS) * TILE_W, (m // MASTER_COLS) * TILE_H
        atlas[y:y + TILE_H, x:x + TILE_W] = px
        shadow[y:y + TILE_H, x:x + TILE_W] = sh
    rgba = np.array(pal, dtype=np.uint8)[atlas]
    for m in masters:  # exact colour 31 per master tile
        if m["uses_index_31"]:
            sl = (slice(m["y"], m["y"] + TILE_H), slice(m["x"], m["x"] + TILE_W))
            rgba[sl][atlas[sl] == 31] = m["color_31"]["rgba8"]

    meta = {
        "tile_size": [TILE_W, TILE_H],
        "atlas": {"columns": MASTER_COLS, "rows": rows, "width": MASTER_COLS * TILE_W, "height": rows * TILE_H,
                  "padding_tiles": rows * MASTER_COLS - len(masters),
                  "note": "Slots past the last master tile are index 0 padding."},
        "count": len(masters),
        "used_pairs": sum(len(m["sources"]) for m in masters),
        "usage": {
            "rule": "A tile is used by a region when it appears in a shipped map (assets/maps) drawn with that "
                    "region's tileset: the overworld for regions 0-7 (tile = sector_mem[map_mem[sec_num]*128 + "
                    "row*16 + col]); for regions 8 and 9, which share one map and sector pool, the interior "
                    "spaces the game enters with that region loaded (door secs, stargate, quicksand drop). "
                    "Parts of the interior map no entry reaches count for neither.",
            "citations": ["src/fsubs.asm:565-604", "src/fmain.c:3557-3562", "src/fmain.c:615-626",
                          "src/fmain.c:1919-1927", "src/fmain.c:1936-1950", "src/fmain.c:2632-2637",
                          "src/fmain.c:1784-1789"],
        },
        "dedup": {
            "rule": "Identical palette-index tiles collapse into one master tile. The key also holds the "
                    "region's colour-31 value (when the tile uses index 31), the tile's shadow mask entry and "
                    "occlusion mode, and its collision record (feature_type, subtile_mask), so every master "
                    "tile is a complete stand-in for an original tile id: atlas_rgba.png, atlas_shadowmask.png "
                    "and the walkability are exact 1:1 lookups. Art that appears with different masks/modes/"
                    "collision is kept once per combination.",
            "citations": ["src/fmain2.c:381-386", "src/fmain.c:2577-2595", "src/fsubs.asm:607-614"],
        },
        "collision": {
            "fields": "tiles[].feature_type (terra byte 1 high nibble) and tiles[].subtile_mask (terra byte 2)",
            "rule": "px_to_im(x, y) returns feature_type when subtile_mask has bit 0x80 >> (4*col + row) set "
                    "for sub-tile col = (x>>3)&1, row = (y>>3)&3 (8x8 px), else 0. prox blocks movement on 1 "
                    "and on >= 10; proxcheck lets the hero through 8 and 9; 12 passes with the Shard; 15 is "
                    "an openable door (doorfind). Named types: 1 impassable, 2 sink, 3 slow/brush; 13 is "
                    "furniture (beds), 15 doors.",
            "citations": ["src/fsubs.asm:548-614", "src/fsubs.asm:1590-1611", "src/fmain2.c:277-283",
                          "src/fmain.c:1607-1609", "src/fmain.c:684-685", "src/fmain.c:1081-1125"],
        },
        "shadow_mask": {
            "file": "atlas_shadowmask.png",
            "note": "The 16x32 shadow mask (assets/masks/ entry tiles[].mask, from terra byte 0) the game applies "
                    "over sprites behind the tile, or empty when tiles[].mask_mode (terra byte 1 & 15) is 0. "
                    "The mode decides when it is applied (fmain.c:2584-2594); see region tiles.json.shadow_mask.",
            "citations": ["src/fmain.c:2577-2595", "src/fsubs.asm:1047-1083"],
        },
        "palette": {
            "file": "palettes/pagecolors.json",
            "index_31": "verbatim source indices; opaque (no key colour). atlas_rgba.png draws each tile's "
                        "index-31 pixels in its color_31 (palettes/region_overrides.json).",
        },
        "highlight_mask": {"indices": [ac.HIGHLIGHT_LO, ac.HIGHLIGHT_HI], "source": "src/fmain2.c:412-413"},
        "index_31": {
            "tiles": [m["index"] for m in masters if m["uses_index_31"]],
            "note": "Master tiles containing palette index 31, the only colour fade_page changes per region; "
                    "each carries its color_31.",
            "secret_variants": secret_variants,
            "secret_variants_note": "While secret_timer runs (Crystal Orb, magic case 8 fmain.c:3307; decremented "
                                    "fmain.c:1381) region 9 shows colour 31 as 0x00f0, revealing hidden passages. "
                                    "Each affected tile has a second master tile (variant): identical indices, "
                                    "colour 31 = 0x00f0 in atlas_rgba.png. Draw the variant while the timer runs, "
                                    "or swap palette entry 31 when using atlas_indexed.png.",
            "citations": ["src/fmain2.c:381-386", "src/fmain.c:3307", "src/fmain.c:1381"],
        },
        "regions": regions_meta,
        "tiles": masters,
        "region_maps": region_maps,
        "region_maps_note": "region_maps[R][t] is the master tile index for region R's tile t, or null when "
                            "no sector referenced by region R's map contains tile t.",
    }
    return atlas, rgba, shadow, meta


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("--assets-dir", type=Path, default=REPO_ROOT / "assets")
    p.add_argument("--out-dir", type=Path, default=None)
    p.add_argument("--src-dir", type=Path, default=REPO_ROOT / "src")
    p.add_argument("--game-dir", type=Path, default=REPO_ROOT / "src" / "assets")
    args = p.parse_args(argv)
    assets = args.assets_dir
    out = args.out_dir or assets / "tiles" / "master"
    atlas, rgba, shadow, meta = build(assets / "tiles", assets / "palettes", args.src_dir, args.game_dir)
    out.mkdir(parents=True, exist_ok=True)
    pal, _ = load_palette(assets / "palettes")
    ac.write_indexed_png(out / "atlas_indexed.png", atlas, pal, transparent=False)
    ac.write_rgba_png(out / "atlas_rgba.png", rgba)
    ac.write_highlight_mask(out / "atlas_highlightmask.png", atlas)
    ac.write_bit_mask(out / "atlas_shadowmask.png", shadow)
    ac.write_json(out / "master.json", meta)
    print(f"master atlas: {meta['count']} unique tiles from {meta['used_pairs']} used (region, tile) pairs; "
          f"{meta['atlas']['width']}x{meta['atlas']['height']} px")
    for r, m in meta["regions"].items():
        print(f"  region {r}: {m['tiles_used']} used / {m['tiles_unused']} unused  ({m['name']})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
