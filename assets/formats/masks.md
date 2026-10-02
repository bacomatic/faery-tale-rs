# Shadow (occlusion) masks

Directory: `masks/`. The game's one stored mask set, `shadow_mem`: **192 entries × 64 bytes**
(`SHADOW_SZ`, `src/fmain.c:642`), loaded from ADF blocks 896–919 (`src/fmain.c:1222`,
`src/mtrack.c:48-50`). Each entry is a 16×32 1-bit image, one big-endian 16-bit word per row,
MSB = leftmost pixel.

```
masks/
  mask_000.png … mask_191.png   16×32, 1-bit greyscale, set bit = white/opaque, clear = transparent
  masks_sheet.png               all 192 in a 16-column sheet (256×384), entry i at column i % 16, row i // 16
  masks.json
```

**Meaning of a set bit: terrain is in front.** When a sprite stands behind a tile that has a
mask, the sprite pixels under set bits are hidden (`_maskit`, `src/fsubs.asm:1047-1083`, blit
`D = A AND NOT C`, `src/fsubs.asm:1920-1935`). Which entry a tile uses and when it applies come
from the tile's terra record: entry = terra byte 0, mode = terra byte 1 & 15
(`src/fmain.c:2577-2595`). Both are on every tile in `tiles/*/tiles.json` (`mask`, `mask_mode`)
and `tiles/master/master.json`, and each atlas ships the composed `atlas_shadowmask.png`
([tiles.md](tiles.md)).

Occlusion modes (`src/fmain.c:2584-2594`, comment `:689-691`):

| mode | mask applied |
|---|---|
| 0 | never |
| 1 | unless the sprite is in the left tile column of its blit (`xm == 0`) |
| 2 | only when the sprite's feet are within the top 35 px of the tile row (`ystop <= 35`) |
| 3 | always (source comment: "unless flying"); at hero sector 48 (the bridge) only for actor 1 |
| 4 | mode 1 and mode 2 both |
| 5 | unless both of the mode-1 and mode-2 exemptions hold |
| 6 | always; when the tile is not in the sprite's top blit row (`ym != 0`) the mask of terra record 64 is used instead ("full if above") |
| 7 | only when `ystop <= 20` |

A falling actor (`FALL`) is masked with mode 3 everywhere except the first 220 terra entries.

`masks.json`:

| Field | Meaning |
|---|---|
| `entry_count` 192, `entry_bytes` 64, `width` 16, `height` 32, `bits_per_pixel` 1, `row_stride_bytes` 2, `bit_order` | geometry |
| `png`, `set_bit_means` | the PNG convention and the semantics above |
| `sheet` | `{file, columns, note}` |
| `disk` | `{image, first_block, block_count, block_size, total_bytes, total_bytes_define, load, files: [{name, block_start, block_count, source}]}` |
| `indexing` | how the game picks an entry (`entry_source`, `entry_address`, `applied_when`, `rows_copied`) |
| `usage` | `{applied_count, applied_entries[], blank_entries[], with_pixels_never_applied[], source}` — cross-check against the shipped maps: entries 0–170 are referenced, 160 of them with a non-zero mode; 171–191 are all-zero; 11 entries have pixels but no tile ever applies them |
| `sources` | citation per fact |
