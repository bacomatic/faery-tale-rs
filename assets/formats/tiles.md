# Background tiles

Directory: `tiles/`. Tiles are **16×32 px**, 5 bitplanes, `pagecolors` palette
(`src/fsubs.asm:750-771`, `1701`; `src/fmain.c:1981`). Each of the 10 regions
(`file_index`, `src/fmain.c:616-625`) loads 4 image groups of 64 tiles = 256 tiles; region
`r`'s tile `t` is cell `t` of its atlas (16 columns × 16 rows → 256×512 px).

```
tiles/
  region_00 … region_09/   atlas_indexed.png  atlas_rgba.png  atlas_highlightmask.png  atlas_shadowmask.png  tiles.json
  master/                  atlas_indexed.png  atlas_rgba.png  atlas_highlightmask.png  atlas_shadowmask.png  master.json
```

| Region | Label | Name | colour 31 |
|---|---|---|---|
| 0–3 | F1–F4 | snowy region, witch wood, swampy region, plains and rocks | `0x0bdf` |
| 4 | F5 | desert area | `0x0980` |
| 5–7 | F6–F8 | bay / city / farms, volcanic, forest and wilderness | `0x0bdf` |
| 8 | F9 | inside of buildings | `0x0bdf` |
| 9 | F10 | dungeons and caves | `0x0445` (`0x00f0` while the Crystal Orb runs) |

Regions 1/3 and 5/7 load identical image blocks; 8 and 9 share tiles 128–255 and differ in 0–127.

## Per-region atlas — `region_NN/`

| File | Format |
|---|---|
| `atlas_indexed.png` | 8-bit indexed PNG, verbatim source indices, **no `tRNS`** — background tiles have no key colour; index 31 shows the region's colour 31 |
| `atlas_rgba.png` | the same tiles as RGBA with `pagecolors` and the region's colour 31 (full brightness, lightlevel ≥ 180) |
| `atlas_highlightmask.png` | 1-bit, set where index ∈ 16–24 ([palettes.md](palettes.md)) |
| `atlas_shadowmask.png` | 1-bit; in each tile's cell the 16×32 shadow mask (`masks/`, entry = `tiles[].mask`) the game blits over sprites standing behind the tile, or empty when `tiles[].mask_mode` is 0 ([masks.md](masks.md)) |

`tiles.json`:

| Field | Meaning |
|---|---|
| `region`, `label`, `name`, `source` | identity (`file_index` row) |
| `tile_size` `[16, 32]`, `tile_count` 256, `planes` 5 | geometry |
| `atlas` | `{file, rgba, highlight_mask, shadow_mask, columns, rows, width, height}` |
| `groups[]` | `{index, block, byte_offset, tiles: [first, last]}` — the 4 disk image groups (block = ADF block of the group's first plane) |
| `layout` | the bitplane offset formula used to decode (`offset(T,P,R)`), with sources |
| `palette` | `{file, color_31: {rgb4, rgba8, source}, index_31: <note>}` |
| `highlight_mask` | `{indices: [16, 24], source}` |
| `index_31` | `{tiles: [...], note, source}` — tile ids that contain index 31 (the only colour that differs per region) |
| `shadow_mask` | `{note, source, terra_source}` |
| `tiles[]` | per tile: `{index, group, block, x, y, w, h, mask, mask_mode}` — `x,y,w,h` is the atlas rectangle; `mask` is the `masks/` entry (terra byte 0) or `null`; `mask_mode` the occlusion mode (terra byte 1 & 15, 0 = never applied) |

## Master atlas — `master/`

Every tile that some shipped map actually draws, deduplicated across regions: **973** master
tiles (971 unique + 2 secret-passage variants) in a 32-column atlas (512×992; slots past the last
tile are padding). The dedup key is the pixel indices **plus** colour-31 value (when used), shadow
mask entry + mode, and the collision record, so one master tile is a complete stand-in for a
tile wherever it appears, and a map rendered from `master.png` needs no per-region lookup.

`master.json`:

| Field | Meaning |
|---|---|
| `count`, `tile_size`, `atlas` | `atlas` = `{columns, rows, width, height, padding_tiles, note}` |
| `tiles[]` | `{index, x, y, w, h, sources: [{region, tile}, ...], uses_index_31, color_31, mask, mask_mode, feature_type, subtile_mask}` — `color_31` is `{index, rgb4, rgba8}` or `null`; `feature_type`/`subtile_mask` are the collision record (below) |
| `region_maps` | `{"0": [256], …, "9": [256]}` — region tile id → master index, `null` where that region never draws the tile |
| `regions` | per region `{name, color_31, maps: [...], tiles_used, tiles_unused}` |
| `index_31` | `{tiles: [...], secret_variants: [{tile, variant, sources}], secret_variants_note, …}` — master 971/972 are cave tiles 114/115 of region 9 with colour 31 = `0x00f0`; draw the `variant` while `secret_timer` runs |
| `dedup`, `usage`, `collision`, `shadow_mask`, `palette`, `highlight_mask` | rule text + citations for each aspect |
| `used_pairs` | number of (region, tile) pairs that map to a master tile |

`master/atlas_*.png` have the same four flavours as a region atlas; `atlas_rgba.png` draws each
tile's index-31 pixels in **its own** `color_31`.

### Collision record (on master tiles)

Walkability is a property of the tile (terra record), stored once per master tile:

- `feature_type` — terra byte 1 high nibble; `1` impassable, `2` sink, `3` slow/brush
  (`src/fmain.c:684-685`), `13` furniture (beds), `15` openable door, `10` passage corner;
  `12` passable with the Shard, `8`/`9` passable for the hero only.
- `subtile_mask` — terra byte 2; bit `0x80 >> (4*col + row)` set means the feature applies to the
  8×8-px sub-tile at `col = (x>>3) & 1`, `row = (y>>3) & 3` (`src/fsubs.asm:548-614`).
- Movement is blocked where the applying type is 1 or ≥ 10 (`src/fsubs.asm:1596-1609`), with
  the hero exceptions above (`src/fmain2.c:282`, `src/fmain.c:1607-1609`).

[maps.md](maps.md) explains how a map cell resolves to a master tile.
