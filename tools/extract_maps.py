#!/usr/bin/env python3
"""T2.5 -- world maps: the overworld, every interior space, the dungeons and the astral plane.

The game stores the world as ten 32x64 sector maps over two shared sector pools
(``src/fmain.c:615-626``, ``fsubs.asm:565-604``). Regions 0-7 tile one 128x128-sector
overworld (``fmain.c:2983-2987``); regions 8 and 9 are the *same* map and sector pool shown
with two different tilesets, chosen when a door is entered (``doorlist[].secs``,
``fmain.c:1926``). This tool unpacks that into separate maps a port can load directly:

* ``overworld/``        2048x1024 tiles, regions 0-7 stitched (region = (row//256)*2 + col//1024)
* ``interiors/<name>/`` one map per building interior, cropped to the space a visitor can reach
* ``dungeons/<name>/``  the cave systems (region 9 tileset)
* ``astral_plane/``     the astral plane (reached through the doom-tower stargate)

Interior spaces are found by walking: from every door's landing point (``fmain.c:1919-1924``,
``1944-1948``), the stargate and the quicksand drop (``fmain.c:1784-1789``), flood-fill the
8x8-px sub-tiles the hero may stand on (``px_to_im`` ``fsubs.asm:548-614``; ``prox`` blocks
type 1 and types >= 10, ``fsubs.asm:1596-1609``; type 12 opens with the Shard ``fmain.c:1609``;
type 15 is an openable door ``fmain.c:1607``). Entries that reach the same tiles are one space.
A space's map is the bounding box of its reachable tiles plus their neighbouring wall tiles and a
one-tile margin; tiles in that box that belong to a *different* space are blanked (tile 0).

Every map ships two index layers (see ``assets/maps/README.md``): ``tiles.png`` (8-bit, the
original tile id in the map's tileset region) and ``master.png`` (16-bit, the master-atlas tile
index, ``assets/tiles/master/master.json``). ``preview.png`` is a non-authoritative render with
the entry points marked. Decode the layers with ``tools/decode_map_layers.py``.

Usage::

    python tools/extract_maps.py                 # -> assets/maps (needs assets/tiles/master)
    python tools/extract_maps.py --out-dir /tmp/maps
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from collections import deque
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import asset_common as ac  # noqa: E402
import decode_map_data as dm  # noqa: E402

REPO_ROOT = TOOLS_DIR.parent
TILE_W, TILE_H = 16, 32            # fsubs.asm:755-771, 1701
SUB = 8                            # px_to_im tests 8x8 sub-tiles (fsubs.asm:548-560)
MAP_ROWS, MAP_COLS = 256, 1024     # one region: 32x64 sectors of 8x16 tiles
INDOOR_YREG = 128                  # regions 8/9: xreg 0, yreg 128 (fmain.c:2983-2987)
INDOOR_Y0 = INDOOR_YREG * 256      # world y of interior map row 0
OUTDOOR_REGIONS = range(8)
BLANK_TILE = 0                     # sector 0 is tile 0 everywhere (impassable black)
NO_MASTER = 0xFFFF

# prox (fsubs.asm:1596-1609): px_to_im == 1 blocks, >= 10 blocks; proxcheck lets the hero through
# 8/9 (fmain2.c:282); 12 is passed with the Shard (fmain.c:1609); 15 is an openable door
# (fmain.c:1607 -> doorfind). Everything else is walkable terrain (sink/slow/...).
BLOCKING_TYPES = frozenset({1, 10, 11, 13, 14})
QUICKSAND_SECTOR = 181             # fmain.c:1784: sinking in hero_sector 181 drops into region 9

DOOR_RE = re.compile(r"\{\s*0x([0-9a-f]+),\s*0x([0-9a-f]+),\s*0x([0-9a-f]+),\s*0x([0-9a-f]+),\s*"
                     r"(\w+)\s*,\s*(\d)\s*\}\s*,?\s*/\*\s*(.*?)\s*\*/", re.I)
XFER9_RE = re.compile(r"new_region\s*=\s*9;\s*xfer\((0x[0-9a-f]+|\d+),\s*(0x[0-9a-f]+|\d+),\s*FALSE\)", re.I)
MARK = (255, 0, 255, 255)          # entry marker colour; asserted not to be a palette colour
DOOR_MARK = (0, 255, 255, 255)     # openable-door tile outline in previews (type 15: doorfind)
OPEN_RE = re.compile(r"\{\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+),\s*(\w+)\s*\}\s*,?\s*/\*\s*(.*?)\s*\*/")

# Shipped names (review feedback 2026-10-01). Keys are the names derived from the doorlist comments;
# every map.json keeps that comment as source_name and the narr.asm place names as place_names.
RENAMES = {
    "main castle": "marheim castle",
    "maze caves": "witchwood cave",
    "doom tower": "citadel of doom",
    "unreachable castle": "forbidden keep",      # narr.asm place name; reached by swan
    "village 1": "tambry tavern",
}
RENAME_PREFIX = {"village": "tambry", "city": "marheim"}
ENTRY_LABELS = {                                 # doorlist comment -> label shown for the entry
    "stargate backwards": "portal to astral plane",
    "stargate forwards": "portal from citadel of doom",
}

# Source bug, fixed in the shipped maps (user decision 2026-09-30, reference/PROBLEMS.md P27): three
# cabin-yard gates carry the middle cabin row's inside y (0x9a40) although their cabins sit in another
# row, so they open into cabins 7/8/6's yards. Corrected to their own cabin's row (yard = cabin - 0x60).
DOOR_FIXES = {
    "cabin yard #4": {"yc2": 0x9c40, "reason": "cabin #4 is at 0x9ca0 (fmain.c:303)"},
    "cabin yard #5": {"yc2": 0x9840, "reason": "cabin #5 is at 0x98a0 (fmain.c:316)"},
    "cabin yard #9": {"yc2": 0x9c40, "reason": "cabin #9 is at 0x9ca0 (fmain.c:306)"},
}


# --------------------------------------------------------------------------- #
# Source tables
# --------------------------------------------------------------------------- #
def parse_doors(src_dir: Path) -> list[dict]:
    """doorlist[86] (fmain.c:233-326) with the numeric door type and the source line."""
    lines = (src_dir / "fmain.c").read_text(errors="replace").split("\n")
    defines = dict(re.findall(r"#define\s+(\w+)\s+(\d+)", "\n".join(lines[:240])))
    start = next(i for i, l in enumerate(lines) if "doorlist[DOORCOUNT] = {" in l)
    doors = []
    for i in range(start + 1, len(lines)):
        if lines[i].startswith("};"):
            break
        m = DOOR_RE.search(lines[i])
        if m:
            x1, y1, x2, y2, dtype, secs, name = m.groups()
            d = {"line": i + 1, "name": name, "type": dtype, "type_value": int(defines[dtype]),
                 "xc1": int(x1, 16), "yc1": int(y1, 16), "xc2": int(x2, 16), "yc2": int(y2, 16),
                 "secs": int(secs)}
            if name in DOOR_FIXES:
                fix = DOOR_FIXES[name]
                d["fix"] = {"source_yc2": d["yc2"], "yc2": fix["yc2"], "reason": fix["reason"],
                            "problem": "reference/PROBLEMS.md P27"}
                d["yc2"] = fix["yc2"]
            doors.append(d)
    assert all(any(d["name"] == n for d in doors) for n in DOOR_FIXES)
    return doors


def parse_open_list(src_dir: Path) -> list[dict]:
    """open_list[17] (fmain.c:1053-1077): openable door tiles by (tile id, image-group block)."""
    text = (src_dir / "fmain.c").read_text(errors="replace")
    body = text[text.index("open_list[17] = {"):]
    body = body[:body.index("\n};")]
    return [{"tile": int(t), "block": int(b), "key": key, "name": name}
            for t, b, _, _, _, key, name in OPEN_RE.findall(body)]


def quicksand_xfer(src_dir: Path) -> tuple[int, int, int]:
    """(x, y, line) of the region-9 xfer in the sinking code (fmain.c:1784-1789)."""
    text = (src_dir / "fmain.c").read_text(errors="replace")
    m = XFER9_RE.search(text)
    return int(m.group(1), 0), int(m.group(2), 0), text[:m.start()].count("\n") + 1


def entry_points(doors: list[dict], quicksand: tuple[int, int, int], outdoor_sector_px) -> list[dict]:
    """Every way into the interior map: where the hero lands and which region is loaded.

    - Door entered from outside (fmain.c:1919-1927): lands at xc2 (+24,+16 for CAVE; +16,+0 for
      odd 'horizontal' types; -1,+16 otherwise); region 8 if secs == 1 else 9.
    - Door whose outside coordinate is itself indoor (the stargate pair, fmain.c:254-255): only
      matched from inside on xc2 and lands at xc1 (fmain.c:1944-1948); the region is recomputed
      from the coordinates (xfer flag TRUE, fmain.c:2632-2637) -> 8 for every interior column.
    - Sinking to depth 30 in sector 181 (fmain.c:1784-1789): xfer into region 9.
    """
    out = []
    for d in doors:
        t = d["type_value"]
        if d["yc1"] < INDOOR_Y0:
            if t == 18:
                x, y = d["xc2"] + 24, d["yc2"] + 16
            elif t & 1:
                x, y = d["xc2"] + 16, d["yc2"]
            else:
                x, y = d["xc2"] - 1, d["yc2"] + 16
            e = {"name": d["name"], "kind": "door", "region": 8 if d["secs"] == 1 else 9,
                 "x": x, "y": y, "outside": [d["xc1"], d["yc1"]], "door_type": d["type"],
                 "source": f"src/fmain.c:{d['line']}, src/fmain.c:1919-1927"}
            if "fix" in d:
                e["fix"] = d["fix"]
            out.append(e)
        else:
            if t == 18:
                x, y = d["xc1"] - 4, d["yc1"] + 16
            elif t & 1:
                x, y = d["xc1"] + 16, d["yc1"] + 34
            else:
                x, y = d["xc1"] + 20, d["yc1"] + 16
            out.append({"name": d["name"], "kind": "stargate", "region": 8, "x": x, "y": y,
                        "outside": [d["xc2"], d["yc2"]], "door_type": d["type"],
                        "source": f"src/fmain.c:{d['line']}, src/fmain.c:1944-1950, src/fmain.c:2632-2637"})
    x, y, line = quicksand
    out.append({"name": "quicksand drop", "kind": "quicksand", "region": 9, "x": x, "y": y,
                "outside": outdoor_sector_px(QUICKSAND_SECTOR), "door_type": None,
                "source": f"src/fmain.c:{line - 3}-{line + 1}"})
    return out


# --------------------------------------------------------------------------- #
# World model
# --------------------------------------------------------------------------- #
class World:
    def __init__(self, game_dir: Path, src_dir: Path):
        self.regions = dm.load_regions(str(game_dir), str(src_dir))
        self.quicksand = quicksand_xfer(src_dir)
        self.interior = np.array(self.regions[8]["tiles"], dtype=np.uint8)
        assert self.regions[9]["tiles"] == self.regions[8]["tiles"]  # fmain.c:624-625
        self.overworld = np.zeros((MAP_ROWS * 4, MAP_COLS * 2), dtype=np.uint8)
        for r in OUTDOOR_REGIONS:
            y0, x0 = (r // 2) * MAP_ROWS, (r % 2) * MAP_COLS
            self.overworld[y0:y0 + MAP_ROWS, x0:x0 + MAP_COLS] = self.regions[r]["tiles"]
        self.doors = parse_doors(src_dir)
        self.open_list = parse_open_list(src_dir)
        self.entries = entry_points(self.doors, self.quicksand, self.outdoor_sector_px)
        self.quicksand_px = self.outdoor_sector_px(QUICKSAND_SECTOR)

    def outdoor_sector_px(self, sector: int) -> list[int]:
        """World pixel of the top-left of the only overworld cell holding `sector`."""
        hits = [((c + reg["xreg"]) * 256, (row + reg["yreg"]) * 256)
                for reg in self.regions[:8] for row in range(32) for c in range(64)
                if reg["grid"][row][c] == sector]
        assert len(hits) == 1, hits
        return list(hits[0])

    def door_tiles(self, region: int) -> dict[int, str]:
        """tile id -> open_list name for the openable doors a region's tileset contains
        (doorfind matches door_id against the tile and map_id against the tile's image block,
        fmain.c:1094-1098)."""
        groups = self.regions[region]["file_index"]["image"]
        return {o["tile"]: o["name"] for o in self.open_list if groups[o["tile"] >> 6] == o["block"]}

    def place_names(self, interior_col_row, outside_px) -> dict:
        """The game's own names (narr.asm:86-223) for the interior sector and the outside sector."""
        col, row = interior_col_row
        inside = dm.lookup_place_name(self.regions[8]["grid"][row][col], True)
        outside = None
        if outside_px is not None and outside_px[1] < INDOOR_Y0:
            oc, orow = outside_px[0] >> 8, outside_px[1] >> 8
            reg = next(g for g in self.regions[:8]
                       if g["xreg"] <= oc < g["xreg"] + 64 and g["yreg"] <= orow < g["yreg"] + 32)
            outside = dm.lookup_place_name(reg["grid"][orow - reg["yreg"]][oc - reg["xreg"]], False)
        return {"inside": inside, "outside": outside, "source": "src/narr.asm:86-223"}

    def walkable(self, region: int) -> np.ndarray:
        """(1024, 2048) bool: sub-tile the hero may occupy, per px_to_im + prox rules."""
        terra = self.regions[region]["terra"]
        lut = np.ones((256, 4, 2), dtype=bool)     # [tile, sub_row, sub_col]
        for e in terra:
            if e["feature_type"] in BLOCKING_TYPES:
                for col in range(2):
                    for row in range(4):
                        if e["subtile_mask"] & dm.subtile_bit(col, row):
                            lut[e["tile"], row, col] = False
        w = lut[self.interior]                       # (256, 1024, 4, 2)
        return w.transpose(0, 2, 1, 3).reshape(MAP_ROWS * 4, MAP_COLS * 2)


def flood(walk: np.ndarray, label: np.ndarray, cx: int, cy: int, k: int) -> int:
    h, w = walk.shape
    q = deque([(cx, cy)])
    label[cy, cx] = k
    n = 1
    while q:
        x, y = q.popleft()
        for nx, ny in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
            if 0 <= nx < w and 0 <= ny < h and walk[ny, nx] and not label[ny, nx]:
                label[ny, nx] = k
                q.append((nx, ny))
                n += 1
    return n


def segment(world: World) -> list[dict]:
    """Group the entry points into connected walkable spaces of the interior map."""
    walk = {r: world.walkable(r) for r in (8, 9)}
    label = {r: np.zeros(walk[r].shape, dtype=np.int32) for r in (8, 9)}
    spaces: dict[tuple, dict] = {}
    k = 0
    for e in world.entries:
        r = e["region"]
        x, y = e["x"], e["y"]
        while not walk[r][(y - INDOOR_Y0) // SUB, x // SUB]:   # xfer: while (proxcheck) hero_y++
            y += 1
        cx, cy = x // SUB, (y - INDOOR_Y0) // SUB
        if not label[r][cy, cx]:
            k += 1
            flood(walk[r], label[r], cx, cy, k)
        key = (r, int(label[r][cy, cx]))
        sp = spaces.setdefault(key, {"region": r, "entries": []})
        sp["entries"].append({**e, "landing": [x, y]})
    out = []
    for (r, lab), sp in spaces.items():
        sub = label[r] == lab
        reach = np.zeros((MAP_ROWS, MAP_COLS), dtype=bool)
        reach[sub.reshape(MAP_ROWS, 4, MAP_COLS, 2).any(axis=(1, 3))] = True
        owned = reach.copy()
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                owned |= np.roll(np.roll(reach, dy, axis=0), dx, axis=1)
        ys, xs = np.nonzero(owned)
        sp.update(reach=reach, owned=owned, walkable_subtiles=int(sub.sum()),
                  bbox=(int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max())))
        out.append(sp)
    return out


def space_name(sp: dict) -> tuple[str, str]:
    names = [e["name"] for e in sorted(sp["entries"], key=lambda e: e["source"])]
    if "doom tower" in names:
        name = "doom tower"
    elif any(e["kind"] == "stargate" for e in sp["entries"]):
        name = "astral plane"
    elif "spider exit" in names:
        name = "spider pit"
    elif "maze cave 1" in names and "maze cave 2" in names:
        name = "maze caves"
    else:
        # a cabin's own door names the space, not a yard door the table points at it
        first = next((n for n in names if " yard" not in n), names[0])
        name = re.sub(r" yard|\.a$|\.b$|DB$", "", first).replace("#", "").strip()
    kind = "astral" if name == "astral plane" else "dungeon" if sp["region"] == 9 else "interior"
    sp["source_name"] = name
    head, _, tail = name.partition(" ")
    name = RENAMES.get(name) or (f"{RENAME_PREFIX[head]} {tail}" if head in RENAME_PREFIX else name)
    return name, kind


def attached(nonblank: np.ndarray, seed: np.ndarray) -> np.ndarray:
    """Non-blank tiles 8-connected (through non-blank tiles) to the seed set."""
    keep = np.zeros_like(nonblank)
    q = deque()
    for y, x in zip(*np.nonzero(seed & nonblank)):
        keep[y, x] = True
        q.append((y, x))
    h, w = nonblank.shape
    while q:
        y, x = q.popleft()
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                ny, nx = y + dy, x + dx
                if 0 <= ny < h and 0 <= nx < w and nonblank[ny, nx] and not keep[ny, nx]:
                    keep[ny, nx] = True
                    q.append((ny, nx))
    return keep


def finish_spaces(world: World, spaces: list[dict]) -> list[dict]:
    """Name each space, crop it and blank tiles that belong to another space."""
    for sp in spaces:
        sp["name"], sp["kind"] = space_name(sp)
    for sp in spaces:
        x0, y0, x1, y1 = sp["bbox"]
        x0, y0, x1, y1 = max(x0 - 1, 0), max(y0 - 1, 0), min(x1 + 1, MAP_COLS - 1), min(y1 + 1, MAP_ROWS - 1)
        foreign = np.zeros((MAP_ROWS, MAP_COLS), dtype=bool)
        for other in spaces:
            if other is not sp:
                foreign |= other["owned"]
        foreign &= ~sp["owned"]
        sp["crop"] = (x0, y0, x1 + 1, y1 + 1)
        sp["foreign"] = foreign[y0:y1 + 1, x0:x1 + 1]
        # fragments of other structures that survive the ownership test (e.g. a wall stub two tiles
        # from a neighbour's floor) are not 8-connected to this space's own tiles: blank them too
        crop = world.interior[y0:y1 + 1, x0:x1 + 1]
        nonblank = (crop != BLANK_TILE) & ~sp["foreign"]
        sp["detached"] = nonblank & ~attached(nonblank, sp["owned"][y0:y1 + 1, x0:x1 + 1])
        sp["foreign"] |= sp["detached"]
    spaces.sort(key=lambda s: (s["kind"], s["name"]))
    names = [s["name"] for s in spaces]
    assert len(set(names)) == len(names), f"duplicate space names: {names}"
    return spaces


def slug(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", name.lower().replace("'", "")).strip("_")


def used_tiles(world: World, spaces: list[dict]) -> dict[int, np.ndarray]:
    """Tile ids each region's tileset actually draws in the shipped maps (master atlas input)."""
    used = {r: np.unique(world.regions[r]["tiles"]) for r in OUTDOOR_REGIONS}
    for r in (8, 9):
        ids = set()
        for sp in spaces:
            if sp["region"] == r:
                x0, y0, x1, y1 = sp["crop"]
                crop = world.interior[y0:y1, x0:x1].copy()
                crop[sp["foreign"]] = BLANK_TILE
                ids.update(np.unique(crop).tolist())
        used[r] = np.array(sorted(ids), dtype=np.uint8)
    return used


# --------------------------------------------------------------------------- #
# Output
# --------------------------------------------------------------------------- #
def load_master(master_dir: Path) -> np.ndarray:
    """(10, 256) master index per (region, tile); NO_MASTER where the tile is never drawn."""
    m = json.loads((master_dir / "master.json").read_text())
    ref = np.full((10, 256), NO_MASTER, dtype=np.uint16)
    for r, lst in m["region_maps"].items():
        for t, v in enumerate(lst):
            if v is not None:
                ref[int(r), t] = v
    return ref


def region_atlases(tiles_dir: Path) -> dict[int, np.ndarray]:
    out = {}
    for r in range(10):
        a = np.array(Image.open(tiles_dir / f"region_{r:02d}/atlas_rgba.png").convert("RGBA"))
        out[r] = a.reshape(16, TILE_H, 16, TILE_W, 4).transpose(0, 2, 1, 3, 4).reshape(256, TILE_H, TILE_W, 4)
    return out


def shrink(atlas: np.ndarray, scale: int) -> np.ndarray:
    if scale == 1:
        return atlas
    th, tw = TILE_H // scale, TILE_W // scale
    return np.array([np.array(Image.fromarray(t).resize((tw, th), Image.BOX)) for t in atlas])


def render(tiles: np.ndarray, atlas: np.ndarray) -> np.ndarray:
    """Composite a tile grid from one region's (256, th, tw, 4) RGBA tile array."""
    h, w = tiles.shape
    _, th, tw, _ = atlas.shape
    return atlas[tiles].transpose(0, 2, 1, 3, 4).reshape(h * th, w * tw, 4)


def mark_doors(img: Image.Image, tiles: np.ndarray, door_tiles: dict[int, str]) -> int:
    """Outline every openable-door tile (type 15) and label it with its open_list name."""
    d = ImageDraw.Draw(img)
    n = 0
    for y, x in zip(*np.nonzero(np.isin(tiles, list(door_tiles)))):
        px, py = int(x) * TILE_W, int(y) * TILE_H
        d.rectangle([px - 1, py - 1, px + TILE_W, py + TILE_H], outline=DOOR_MARK, width=2)
        d.text((px + TILE_W + 3, py + 2), door_tiles[int(tiles[y, x])], fill=DOOR_MARK)
        n += 1
    return n


def mark(img: Image.Image, points: list[tuple[int, int, str]], scale: int = 1) -> None:
    d = ImageDraw.Draw(img)
    r = max(16 // scale, 4)
    for x, y, text in points:
        x, y = x // scale, y // scale
        d.ellipse([x - r, y - r, x + r, y + r], outline=MARK, width=max(4 // scale, 1))
        d.line([x - r * 2, y, x + r * 2, y], fill=MARK, width=1)
        d.line([x, y - r * 2, x, y + r * 2], fill=MARK, width=1)
        if text:
            d.text((x + r + 2, y - r - 10), text, fill=MARK)


def write_map(out: Path, meta: dict, tiles: np.ndarray, master: np.ndarray, preview: Image.Image) -> None:
    out.mkdir(parents=True, exist_ok=True)
    ac.write_gray_png(out / "tiles.png", tiles, bit_depth=8)
    ac.write_gray_png(out / "master.png", master, bit_depth=16)
    preview.save(out / "preview.png", optimize=True)
    ac.write_json(out / "map.json", meta)


LAYERS = {
    "tiles": {"file": "tiles.png", "png": "8-bit greyscale", "value": "tile id 0-255 in the map's tileset "
              "region (assets/tiles/region_<RR>/, tile t at column t % 16, row t // 16)"},
    "master": {"file": "master.png", "png": "16-bit greyscale (big-endian samples)",
               "value": "master tile index into assets/tiles/master/master.json tiles[] "
                        f"(atlas cell x = tiles[i].x, y = tiles[i].y); {NO_MASTER} = none"},
}


def build(world: World, spaces: list[dict], ref: np.ndarray, atlases: dict, out_dir: Path) -> dict:
    # fields/rows: the review app renders this shape as a table
    index = {"schema_version": 1, "tile_size_px": [TILE_W, TILE_H], "layers": LAYERS,
             "fields": ["dir", "name", "kind", "size_tiles", "tileset_region", "entries"], "rows": []}

    # overworld
    region_of_cell = (np.arange(MAP_ROWS * 4)[:, None] // MAP_ROWS) * 2 + (np.arange(MAP_COLS * 2)[None, :] // MAP_COLS)
    master = ref[region_of_cell, world.overworld]
    scale = 4
    th, tw = TILE_H // scale, TILE_W // scale
    canvas = np.zeros((MAP_ROWS * 4 * th, MAP_COLS * 2 * tw, 4), dtype=np.uint8)
    for r in OUTDOOR_REGIONS:
        y0, x0 = (r // 2) * MAP_ROWS, (r % 2) * MAP_COLS
        canvas[y0 * th:(y0 + MAP_ROWS) * th, x0 * tw:(x0 + MAP_COLS) * tw] = render(
            world.overworld[y0:y0 + MAP_ROWS, x0:x0 + MAP_COLS], shrink(atlases[r], scale))
    prev = Image.fromarray(canvas, "RGBA")
    doors = world.doors
    pts = [(d["xc1"], d["yc1"], "") for d in doors if d["yc1"] < INDOOR_Y0]
    qx, qy = world.quicksand_px
    pts.append((qx + 128, qy + 128, ""))
    mark(prev, pts, scale)
    meta = {
        "name": "overworld", "kind": "overworld",
        "size_tiles": [MAP_COLS * 2, MAP_ROWS * 4], "tile_size_px": [TILE_W, TILE_H],
        "origin_world_px": [0, 0],
        "tileset": {"rule": "region = (row // 256) * 2 + (col // 1024); tiles.png values index that "
                            "region's tileset (assets/tiles/region_0<region>/)",
                    "regions": [{"region": r, "name": world.regions[r]["name"], "rows": [(r // 2) * MAP_ROWS, (r // 2 + 1) * MAP_ROWS - 1],
                                 "cols": [(r % 2) * MAP_COLS, (r % 2 + 1) * MAP_COLS - 1], "source": world.regions[r]["source"]}
                                for r in OUTDOOR_REGIONS],
                    "source": "src/fmain.c:2983-2987 (xreg/yreg per region), src/fsubs.asm:565-604"},
        "layers": LAYERS,
        "preview": {"file": "preview.png", "scale": f"1/{scale} (tile = {TILE_W // scale}x{TILE_H // scale} px)",
                    "markers": "magenta: outside position of every doorlist entry (xc1, yc1) and the "
                               "quicksand sector 181 cell"},
        "doors": [{"name": d["name"], "type": d["type"], "outside_px": [d["xc1"], d["yc1"]],
                   "outside_tile": [d["xc1"] // TILE_W, d["yc1"] // TILE_H],
                   "inside_px": [d["xc2"], d["yc2"]], "secs": d["secs"],
                   "interior_region": 8 if d["secs"] == 1 else 9, "source": f"src/fmain.c:{d['line']}",
                   **({"fix": d["fix"]} if "fix" in d else {})}
                  for d in doors if d["yc1"] < INDOOR_Y0],
        "doors_note": "inside_px is the doorlist xc2/yc2; entries with a 'fix' ship the corrected yc2 "
                      "(source bug, reference/PROBLEMS.md P27) and keep the original as fix.source_yc2.",
        "quicksand": {"sector": QUICKSAND_SECTOR, "cell_px": [qx, qy], "cell_tile": [qx // TILE_W, qy // TILE_H],
                      "note": "sinking to depth 30 here transfers the hero into the spider pit",
                      "source": "src/fmain.c:1784-1789"},
        "citations": ["src/fmain.c:615-626", "src/fmain.c:2983-2987", "src/fsubs.asm:565-604",
                      "src/fmain.c:233-326", "src/fmain.c:1892-1934"],
    }
    write_map(out_dir / "overworld", meta, world.overworld, master, prev)
    index["rows"].append({"dir": "overworld", "name": "overworld", "kind": "overworld",
                          "size_tiles": meta["size_tiles"], "tileset_region": "per cell", "entries": None})

    for sp in spaces:
        x0, y0, x1, y1 = sp["crop"]
        tiles = world.interior[y0:y1, x0:x1].copy()
        tiles[sp["foreign"]] = BLANK_TILE
        r = sp["region"]
        master = ref[r, tiles]
        prev = Image.fromarray(render(tiles, atlases[r]), "RGBA")
        n_doors = mark_doors(prev, tiles, world.door_tiles(r))
        pts = [(e["landing"][0] - x0 * TILE_W, e["landing"][1] - INDOOR_Y0 - y0 * TILE_H, ENTRY_LABELS.get(e["name"], e["name"]))
               for e in sp["entries"]]
        mark(prev, pts)
        sub = "" if sp["kind"] == "astral" else f"{sp['kind']}s/"
        d = f"{sub}{slug(sp['name'])}"
        first = sp["entries"][0]
        meta = {
            "name": sp["name"], "kind": sp["kind"],
            "source_name": sp["source_name"],
            "place_names": world.place_names((first["landing"][0] // 256, (first["landing"][1] >> 8) - INDOOR_YREG),
                                             first["outside"]),
            "size_tiles": [x1 - x0, y1 - y0], "tile_size_px": [TILE_W, TILE_H],
            "origin_interior_tile": [x0, y0],
            "origin_world_px": [x0 * TILE_W, INDOOR_Y0 + y0 * TILE_H],
            "tileset": {"region": r, "name": world.regions[r]["name"], "dir": f"assets/tiles/region_{r:02d}/",
                        "source": sp["entries"][0]["source"]},
            "layers": LAYERS,
            "preview": {"file": "preview.png", "scale": "1/1",
                        "markers": "magenta rings: entry landing points; cyan outlines: openable door tiles "
                                   f"(type 15, open_list name; {n_doors} here)"},
            "entries": [{"name": ENTRY_LABELS.get(e["name"], e["name"]), "door": e["name"], "kind": e["kind"], "door_type": e["door_type"],
                         "landing_px": e["landing"],
                         "landing_tile": [e["landing"][0] // TILE_W - x0, (e["landing"][1] - INDOOR_Y0) // TILE_H - y0],
                         "outside_px": e["outside"], "source": e["source"],
                         **({"fix": e["fix"]} if "fix" in e else {})} for e in sp["entries"]],
            "walkable_subtiles": sp["walkable_subtiles"],
            "blanked_tiles": int(sp["foreign"].sum()),
            "notes": ["Reachable area from the listed entries by the px_to_im/prox rules; cropped to those "
                      "tiles, their neighbours and a 1-tile margin. blanked_tiles cells belonged to another "
                      "space or were fragments not connected to this one, and were set to tile 0.",
                      "name is the shipped name (review decision); source_name derives from the doorlist "
                      "comment, place_names are what the game prints (narr.asm).",
                      "Tile ids 0-127 differ between the region 8 and 9 tilesets (terra1 / image group 1); "
                      "128-255 are shared (fmain.c:624-625)."],
            "citations": ["src/fmain.c:624-625", "src/fmain.c:1892-1955", "src/fmain.c:2625-2645",
                          "src/fsubs.asm:548-614", "src/fsubs.asm:1590-1611", "src/fmain.c:1607-1609"],
        }
        write_map(out_dir / d, meta, tiles, master, prev)
        index["rows"].append({"dir": d, "name": sp["name"], "kind": sp["kind"], "size_tiles": meta["size_tiles"],
                              "tileset_region": r, "entries": ", ".join(ENTRY_LABELS.get(e["name"], e["name"]) for e in sp["entries"])})
    return index


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ac.add_io_args(p)
    p.add_argument("--assets-dir", type=Path, default=REPO_ROOT / "assets")
    p.add_argument("--out-dir", type=Path, default=None)
    args = p.parse_args(argv)
    out = args.out_dir or args.assets_dir / "maps"
    master_dir = args.assets_dir / "tiles" / "master"
    if not (master_dir / "master.json").is_file():
        print(f"missing {master_dir}/master.json -- run tools/extract_master_atlas.py first", file=sys.stderr)
        return 1

    world = World(args.game_dir, args.src_dir)
    spaces = finish_spaces(world, segment(world))
    ref = load_master(master_dir)
    atlases = region_atlases(args.assets_dir / "tiles")
    pal = {tuple(e["rgba8"]) for e in json.loads((args.assets_dir / "palettes" / "pagecolors.json").read_text())}
    assert MARK not in pal and DOOR_MARK not in pal, "marker colour clashes with the palette"
    index = build(world, spaces, ref, atlases, out)
    ac.write_json(out / "maps.json", index)
    kinds = {}
    for m in index["rows"]:
        kinds[m["kind"]] = kinds.get(m["kind"], 0) + 1
    print(f"{len(index['rows'])} maps under {out}: " + ", ".join(f"{v} {k}" for k, v in sorted(kinds.items())))
    for m in index["rows"]:
        print(f"  {m['dir']:40s} {m['size_tiles'][0]:4d}x{m['size_tiles'][1]:<4d} region {m['tileset_region']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
