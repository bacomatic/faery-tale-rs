# Maps

Directory: `maps/`. The world as loadable tile maps: the stitched **overworld**, 62 building
**interiors**, 5 **dungeons** and the **astral plane**, each as index-layer PNGs plus `map.json`.
[`maps/README.md`](../maps/README.md) is the full description (layout, how the spaces were cut
out of the shared interior sheet, names, the Python decoder); this page is the field reference.

```
maps/
  maps.json                      index of all 69 maps
  overworld/                     2048 × 1024 tiles — regions 0–7 stitched 2 across × 4 down
  astral_plane/                  212 × 68 tiles, region-8 tileset
  dungeons/<name>/               tombs dragon_cave witchwood_cave spider_pit troll_cave — region-9 tileset
  interiors/<name>/              62 spaces — region-8 tileset
```

## Layers (every map directory)

| File | Format | Pixel value |
|---|---|---|
| `tiles.png` | 8-bit greyscale PNG, `size_tiles` wide/high | original tile id 0–255 in the map's tileset region (cell `t` of `tiles/region_RR/atlas_*.png`) |
| `master.png` | **16-bit** greyscale PNG (big-endian samples) | master tile index into `tiles/master/master.json` `tiles[]`; `65535` = none (does not occur) |
| `preview.png` | RGBA render | non-authoritative; magenta rings = entry landing points, cyan outlines = openable door tiles (type 15) with their `open_list` name; the overworld preview is at 1/4 scale |

One pixel = one map cell = one 16×32-px tile. Row 0 is the top. World pixel of cell `(col, row)`
is `origin_world_px + (col*16, row*32)`.

For the overworld the tileset region of a cell is `(row // 256) * 2 + (col // 1024)`
(`map.json.tileset.rule`, regions laid out `[0 1 / 2 3 / 4 5 / 6 7]`); every other map has one
`tileset.region`. `master.png` needs neither.

## `maps.json`

`{schema_version, tile_size_px: [16, 32], layers, fields, rows[]}` with one row per map:
`{dir, name, kind, size_tiles, tileset_region, entries}` — `kind` ∈ `overworld | interior |
dungeon | astral`; `tileset_region` is a region number or `"per cell"` (overworld); `entries` is a
`", "`-joined list of entry names, `null` for the overworld. `layers` repeats the layer formats.

## `map.json` — interiors, dungeons, astral plane

| Field | Meaning |
|---|---|
| `name`, `source_name`, `kind` | shipped name (decided in review; also the directory), the name derived from the `doorlist` comment, kind |
| `place_names` | `{inside, outside, source}` — the names the game prints for the inside and outside sectors (`src/narr.asm`), `null` when it prints none |
| `size_tiles`, `tile_size_px` | `[cols, rows]`, `[16, 32]` |
| `origin_interior_tile`, `origin_world_px` | where the crop sits on the original 1024×256-tile interior sheet, in tiles and in world pixels |
| `tileset` | `{region, dir, name, source}` |
| `layers`, `preview` | as above |
| `entries[]` | how the space is entered: `{kind: door \| stargate \| quicksand, name, door, door_type, landing_px, landing_tile, outside_px, source}` — `landing_tile` is relative to this map; `landing_px`/`outside_px` are world pixels; `door_type` is the `doorlist` type name (`HWOOD`, `VLOG`, `CAVE`, `STAIR`, …) |
| `blanked_tiles` | cells inside the bounding box that belong to another space (set to tile 0) |
| `walkable_subtiles` | size of the reachable area (8×8-px sub-tiles) used to cut the space |
| `notes[]`, `citations[]` | method notes and source lines |

## `map.json` — overworld

| Field | Meaning |
|---|---|
| `kind` `"overworld"`, `name`, `size_tiles` `[2048, 1024]`, `tile_size_px`, `origin_world_px` `[0, 0]` | |
| `tileset` | `{rule, source, regions: [{region, name, cols: [c0, c1], rows: [r0, r1], source}]}` |
| `doors[]` | every `doorlist` entry (`src/fmain.c:240-325`): `{name, type, secs, interior_region, outside_px, outside_tile, inside_px, source, fix?}` — `outside_px` is the door's overworld position, `inside_px` the landing in the interior sheet; `secs` 1 → region 8, else 9. Three cabin-yard gates carry `fix: {yc2, source_yc2, reason, problem}`: the shipped `inside_px` is corrected (the source points them into the wrong cabin, `reference/PROBLEMS.md` P27) |
| `doors_note` | the above in prose |
| `quicksand` | `{sector, cell_px, cell_tile, note, source}` — sinking here drops the hero into the spider pit |
| `layers`, `preview`, `citations` | as above |

Collision is not stored per map: it is a property of the tile, carried on every master tile
(`feature_type`, `subtile_mask`; see [tiles.md](tiles.md)).
