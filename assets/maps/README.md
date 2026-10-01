# assets/maps — the game world as loadable maps

Produced by `tools/extract_maps.py` (T2.5) from the ADF (`src/assets/image`) and the door tables in
`src/fmain.c`. Decode the layers with `tools/decode_map_layers.py` (see bottom).

## Layout

```
maps.json                      index of every map (fields/rows table)
overworld/                     2048 x 1024 tiles: regions 0-7 (F1-F8) stitched 2 across x 4 down
astral_plane/                  212 x 68, region 8 tileset, reached through the doom-tower stargate
dungeons/<name>/               5 cave systems, region 9 tileset
interiors/<name>/              62 building interiors, region 8 tileset
```

Each map directory contains:

| file | what |
|---|---|
| `map.json` | name, kind, size, origin, tileset, entries (doors / stargate / quicksand), citations |
| `tiles.png` | **8-bit greyscale** PNG, pixel value = original tile id 0–255 in the map's tileset region |
| `master.png` | **16-bit greyscale** PNG (big-endian samples), pixel value = master tile index into `assets/tiles/master/master.json` `tiles[]`; `65535` = none (never occurs in shipped maps) |
| `preview.png` | non-authoritative render from `assets/tiles/region_<RR>/atlas_rgba.png`; magenta rings mark entry landing points (overworld: door outside positions and the quicksand cell, at 1/4 scale) |

One pixel in a layer = one map cell; image width/height = `map.json.size_tiles`. A tile is 16×32 px in
game (`fsubs.asm:755-771`, `1701`). Rows run top to bottom, columns left to right.

## Tilesets

Two coordinate systems are provided for every cell, pick either:

* **Original:** `tiles.png` value `t` in region `R` is the tile at column `t % 16`, row `t // 16` of
  `assets/tiles/region_<RR>/atlas_*.png`. For interiors/dungeons/astral `R` is `map.json.tileset.region`
  (8 = buildings, 9 = caves — the two indoor tilesets share tiles 128–255 and differ in 0–127,
  `fmain.c:624-625`). For the overworld `R = (row // 256) * 2 + col // 1024` (`map.json.tileset.rule`).
* **Master:** `master.png` value `i` is `master.json.tiles[i]`, a rectangle in
  `assets/tiles/master/atlas_*.png`. Each master tile carries its shadow mask/mode **and** its collision
  record (`feature_type`, `subtile_mask`), so a port needs no per-region lookup at all.

## Collision

Walkability is a property of the tile, not stored per map. Per tile: `feature_type` (terra byte 1
high nibble) applies to the 8×8-px sub-tiles whose bit is set in `subtile_mask` (terra byte 2); bit =
`0x80 >> (4*col + row)` for sub-tile `col = (x>>3)&1`, `row = (y>>3)&3` (`fsubs.asm:548-614`). Movement
is blocked on type 1 and on types ≥ 10 (`fsubs.asm:1596-1609`), except that the hero passes 8/9
(`fmain2.c:282`), 12 with the Shard (`fmain.c:1609`) and opens 15 (doors, `fmain.c:1607`, `1081-1125`).
Named types: 1 impassable, 2 sink, 3 slow/brush (`fmain.c:684-685`); 13 is furniture (beds), 10 appears
on one passage-corner tile. Available on every master tile, or per region in the terra tables
(`tools/decode_map_data.load_regions`).

## How the spaces were cut

The interior map (regions 8/9, `yreg` 128 → world y ≥ 32768) packs every building, cave and the astral
plane into one 1024×256-tile sheet. Each shipped space is what a visitor can reach: from every entry's
landing point (door landing `fmain.c:1919-1924`; stargate `fmain.c:1944-1948`, region from
`fmain.c:2632-2637`; quicksand drop `fmain.c:1784-1789`, from overworld sector 181) a flood fill over
walkable sub-tiles; entries that reach the same tiles form one space. The map is the bounding box of
the reached tiles, their neighbouring (wall) tiles and a one-tile margin; cells inside that box that
belong to a different space are set to tile 0 (`map.json.blanked_tiles`). The tileset is the region the
game loads on entry (`doorlist[].secs`, `fmain.c:1926`). One deliberate deviation from the source: the
three cabin-yard gates that `doorlist` points into the wrong cabin (`reference/PROBLEMS.md` P27) are
corrected to their own cabins; the affected entries carry `fix.source_yc2` with the original value. `map.json.origin_interior_tile` /
`origin_world_px` place the crop back on the original sheet; `entries[].landing_tile` is relative to
the map.

## Reading the layers

```python
import sys; sys.path.insert(0, "tools")
from decode_map_layers import load_map
m = load_map("assets/maps/interiors/main_castle")
m["tiles"]     # numpy uint8  (rows, cols)
m["master"]    # numpy uint16 (rows, cols)
m["region"]    # numpy uint8  tileset region per cell
m["meta"]      # map.json
```

```
python tools/decode_map_layers.py assets/maps/dungeons/tombs             # summary + entries
python tools/decode_map_layers.py assets/maps/overworld --cell 1189 984  # {"region":7,"tile":41,"master":23}
python tools/decode_map_layers.py assets/maps/astral_plane --dump json   # layers as JSON rows
python tools/decode_map_layers.py assets/maps/interiors/crypt --dump csv --layer master
```

Without Python: any PNG reader that preserves 8/16-bit greyscale samples (Pillow `Image.open(...)`
gives modes `L` / `I;16`; `pngjs`, `stb_image` 16-bit, ImageMagick `-depth 16` all work).
