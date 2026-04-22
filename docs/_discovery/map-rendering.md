# Discovery: Map Rendering & Tile-Based Scrolling

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace the full map rendering system — tile grid layout, full-screen draw, incremental scroll, blitter shifting, minimap generation, coordinate conversion, and memory layout.

## 1. Memory Layout

### Tile Image Memory (`image_mem`)
- **Allocation**: `fmain.c:917` — `AllocMem(IMAGE_SZ, MEMF_CHIP)` where `IMAGE_SZ = IPLAN_SZ * 5 = 81920` bytes (`fmain.c:640`).
- **IPLAN_SZ**: 16384 bytes = 256 tiles × 64 bytes/tile/plane (`fmain.c:639`, `fsubs.asm:661`).
- **Structure**: 5 consecutive planes of 256 tiles each. Each tile is 16×32 pixels = 16 pixels × 32 scanlines = 2 bytes × 32 = 64 bytes per plane. Plane offsets: `d1=IPLAN_SZ`, `d2=IPLAN_SZ*2`, `d3=IPLAN_SZ*3`, `d4=IPLAN_SZ*4` (`fsubs.asm:672-675`).
- **QPLAN_SZ**: 4096 bytes = 64 tiles × 64 bytes/tile/plane. Image memory is loaded in 4 blocks of QPLAN_SZ per plane (`fmain.c:638`, `fmain.c:3591`).

### Sector Memory (`sector_mem`, `map_mem`)
- **Allocation**: `fmain.c:919` — `AllocMem(SECTOR_SZ, MEMF_CHIP)` where `SECTOR_SZ = (128*256) + 4096 = 36864` bytes.
- **sector_mem**: 256 sectors × 128 bytes each = 32768 bytes. Each sector is a 16×8 grid of tile indices (one byte per tile).
- **map_mem**: `sector_mem + SECTOR_OFF` where `SECTOR_OFF = 128*256 = 32768` (`fmain.c:920`). The region map is 4096 bytes, organized as a 128×32 grid of sector indices (one byte each).

### Terrain Data (`terra_mem`)
- **Allocation**: `fmain.c:928` — `AllocMem(1024, MEMF_CHIP)`.
- **Structure**: 256 entries × 4 bytes each, loaded in two halves of 512 bytes (`fmain.c:3567-3572`). Each 4-byte entry: byte 0 = tile mask index, byte 1 = terrain characteristics (low nibble used for occlusion priority), byte 3 = color for bigdraw minimap (`terrain.c:60-63`, `terrain.c:42-44`).

### Bitplane Memory (`planes`)
- **Variable**: `PLANEPTR *planes` (`fmain.c:675`). Points to a `BitMap->Planes` array of 5 plane pointers.
- **Plane dimensions**: 320×200 pixels = 40 bytes/scanline × 200 scanlines = 8000 bytes per plane (`fmain.c:12,15`).

### Minimap Buffer (`minimap`)
- **Declaration**: `extern short minimap[114]` (`fmain.c:630`). Array of 114 words = 19 columns × 6 rows. Stored column-major (all 6 rows of column 0 first, then column 1, etc.).
- **Contents**: Each entry is a tile index (0–255) resolved from sector data by `_genmini`.

## 2. Tile Grid Layout: 19×6

The visible playfield is a 19-column × 6-row grid of tiles.

- **Evidence in `_map_draw`** (`fsubs.asm:664-726`): The outer loop calls `next_strip` 19 times with d5 = 0, 2, 4, …, 36 (word offsets for 19 columns). Each `next_strip` reads 6 tiles from the minimap.
- **Evidence in `_row_draw`** (`fsubs.asm:822-899`): Calls `next_char` 19 times, advancing 2 bytes each iteration (19 columns).
- **Evidence in `_genmini`** (`fsubs.asm:1136-1207`): Outer loop `d6` counts 0–18 (19 iterations = columns), inner loop `d7` counts 0–5 (6 iterations = rows).
- **Comment**: `fmain.c:629` — `/* playing map is 6 * 19 = 114 */`.

### Tile Dimensions
- Each tile is **16 pixels wide × 32 pixels tall** (16 bits = 1 word, 32 scanlines).
- **Evidence**: `next_image` (`fsubs.asm:731-752`) loops `d6=#15` (16 iterations), each writing 2 scanlines and advancing d5 by 40 (bytes/scanline) twice. Total: 32 scanlines.
- `no_image` (`fsubs.asm:754-756`) skips by adding 1280 to d5, which = 32 scanlines × 40 bytes/scanline.

### Visible Area
- Width: 19 tiles × 16 pixels = 304 pixels (of 320-pixel-wide bitplanes; +16 pixels for scroll margin).
- Height: 6 tiles × 32 pixels = 192 scanlines. `fsubs.asm:1708`: `scan6 = 6 * vsc = 6 * 32 = 192`.

## 3. `_map_draw` — Full-Screen Map Render

**Location**: `fsubs.asm:664-756`

### Algorithm
1. Load `_image_mem` (tile image source), save on stack.
2. Load `_minimap` address into a6 (tile index source).
3. Set `d1–d4` to plane offsets (IPLAN_SZ, 2×, 3×, 4×).
4. Load 5 plane base addresses from `_planes` into `a0–a4`.
5. Iterate 19 columns: d5 = 0, 2, 4, …, 36 (column byte offset), call `next_strip`.
6. `next_strip` (`fsubs.asm:728-735`): Reads 6 words from minimap (a6 auto-increments), calls `next_image` for each.
7. `next_image` (`fsubs.asm:737-756`):
   - Shifts tile index left 6 bits (× 64) to get byte offset into `image_mem`.
   - Computes source: `a5 = image_mem + tile_index * 64`.
   - Inner loop (16 iterations, 2 scanlines each = 32 total): copies one word from each of 5 planes of `image_mem` to the corresponding bitplane at offset d5, advancing d5 by 40 each scanline.

### Key Insight
The minimap buffer acts as indirection — `_genmini` resolves world coordinates to tile indices, then `_map_draw` blindly renders those 114 tiles.

## 4. `_strip_draw` — Single Column Redraw

**Location**: `fsubs.asm:758-810`

### Purpose
Redraws a single vertical strip (one column of 6 tiles) after a horizontal scroll.

### Algorithm
1. Takes one parameter: the column byte offset (0 = leftmost, 36 = rightmost).
2. Sets a6 = `_minimap + offset * 6` (column-major: 6 words = 12 bytes per column; offset is divided by 2 to get column index, multiplied by 12 for byte index. In code: adds offset 6 times to the base).
3. Calls `next_strip` once to draw that column's 6 tiles.

### Call Sites (`fmain.c:2363`)
```c
if (dif_x == 1) strip_draw(36); else if (dif_x == -1) strip_draw(0);
```
- `dif_x == 1` (view moved left): new tiles appear at right edge → `strip_draw(36)` = column 18.
- `dif_x == -1` (view moved right): new tiles at left edge → `strip_draw(0)` = column 0.

## 5. `_row_draw` — Single Row Redraw

**Location**: `fsubs.asm:812-899`

### Purpose
Redraws a single horizontal row (19 tiles) after a vertical scroll.

### Algorithm
1. Takes one parameter: the row byte offset into the minimap (0 = top row, 10 = bottom row).
2. Sets a6 = `_minimap + row_offset` (byte offset; since minimap is column-major words, row offset is a word index within each column).
3. Computes bitplane Y offset: `d0 = row_offset * 640`. Since each tile row = 32 scanlines × 40 bytes = 1280 bytes, and the row parameter is in 2-byte units (0, 2, 4, 6, 8, 10 for rows 0–5), `offset * 640 = (offset/2) * 1280`.
4. Calls `next_char` 19 times (`fsubs.asm:858-895`), advancing d0 by 2 (next column) and a6 by 12 (next column in minimap) each iteration.
5. `next_char` (`fsubs.asm:897-901`): copies d0 to d5, reads tile index from (a6), calls `next_image`, advances a6 by 12.

### Call Sites (`fmain.c:2364`)
```c
if (dif_y == 1) row_draw(10); else if (dif_y == -1) row_draw(0);
```
- `dif_y == 1` (view moved up): new tiles at bottom → `row_draw(10)` = row 5.
- `dif_y == -1` (view moved down): new tiles at top → `row_draw(0)` = row 0.

## 6. `_scrollmap` — Blitter-Based 8-Direction Bitplane Shifting

**Location**: `fsubs.asm:1736-1797`

### Purpose
Uses the Amiga blitter to shift all 5 bitplanes by one tile in any of 8 directions, leaving a gap that `strip_draw` / `row_draw` fills.

### Algorithm
1. Calls `OwnBlitter()` to acquire exclusive blitter access.
2. Loops 5 times (one per bitplane), counting down d2 from 4 to 0.
3. For each plane:
   a. Gets plane pointer from `_planes[d2]`.
   b. Waits for blitter ready (`DMACONR` bit 6 / `WaitBlit`).
   c. Computes ascending/descending from direction: `d1 = ~(direction >> 1) & 2`. Directions 0–3 → descending (bit 1 set in BLTCON1); directions 4–7 → ascending.
   d. Sets `BLTCON0 = $09F0` (A→D copy, all 4 DMA channels enabled, minterm D=A).
   e. Sets masks to $FFFF (no masking).
   f. Indexes `scroll_table` at `direction * 8` to get: source offset, dest offset, modulus, blitsize.
   g. Computes A pointer = plane_base + source_offset, D pointer = plane_base + dest_offset.
   h. Writes BLTAMOD, BLTDMOD (same modulus), BLTSIZE (triggers the blit).
4. Calls `DisownBlitter()`.

### `BLTCON0 = $09F0` (`fsubs.asm:1675`)
- Bits 15–12: shift = 0 (no sub-pixel shift).
- Bit 11: USEA = 1, Bit 10: USEB = 0, Bit 9: USEC = 0, Bit 8: USED = 1.
- Bits 7–0: minterm $F0 = D = A (straight copy).

### Ascending vs Descending (`fsubs.asm:1759-1764`)
```asm
move.w  d0,d1           ; d1 = direction
lsr.w   #1,d1           ; /2
not.w   d1              ; complement
and.w   #$0002,d1       ; isolate bit 1
move.w  d1,BLTCON1(a0)  ; ascending (0) or descending (2)
```
- Directions 0–3 (right/down): descending — blitter works from end to start to avoid overwrite.
- Directions 4–7 (left/up): ascending — blitter works from start to end.

### `scroll_table` — Blitter Parameters

**Location**: `fsubs.asm:1710-1735`

**Constants** (`fsubs.asm:1698-1708`):
| Name | Value | Meaning |
|------|-------|---------|
| `vmod` | 4 | Modulus for vertical/diagonal (skip 2 words = 1 tile column) |
| `hmod` | 2 | Modulus for horizontal (skip 1 word) |
| `vsc` | 32 | Scanlines per tile |
| `sbytes` | 40 | Bytes per scanline |
| `vdelta` | 1280 | Bytes per tile row (32 × 40) |
| `len5` | 6400 | 5 tile rows in bytes |
| `len6` | 7680 | 6 tile rows in bytes |
| `scan5` | 160 | 5 tile rows in scanlines |
| `scan6` | 192 | 6 tile rows in scanlines |

**Table entries** (4 words each: Aoffset, Doffset, modulus, blitsize):

| Dir | Scroll | Aoff | Doff | Mod | Height×Width | Notes |
|-----|--------|------|------|-----|-------------|-------|
| 0 | Right | len6−6 | len6+2−6 | vmod=4 | 192×18w | Desc; D is 2 bytes right of A; 18 of 20 words copied |
| 1 | Down-Right | len5−6 | len5+1282−6 | vmod=4 | 160×18w | Desc; D is 1 row+1 word ahead |
| 2 | Down | len5−4 | len5+vdelta−4 | hmod=2 | 160×19w | Desc; D is 1 row below; full 19-word width |
| 3 | Down-Left | len5+2−6 | len5+vdelta−6 | vmod=4 | 160×18w | Desc; D is 1 row below, 1 word left |
| 4 | Left | 2 | 0 | vmod=4 | 192×18w | Asc; A starts 1 word right, copies left |
| 5 | Up-Left | 1282 | 0 | vmod=4 | 160×18w | Asc; A starts 1 row+1 word ahead |
| 6 | Up | vdelta | 0 | hmod=2 | 160×19w | Asc; A starts 1 row below |
| 7 | Up-Right | vdelta | 2 | vmod=4 | 160×18w | Asc; A starts 1 row below, D starts 1 word right |

### BLTSIZE Encoding
`BLTSIZE = (height_in_scanlines × 64) + width_in_words`:
- Vertical-only (dirs 2,6): 160×19 = `(160*64)+19 = 10259`
- Horizontal (dirs 0,4): 192×18 = `(192*64)+18 = 12306`
- Diagonal: 160×18 = `(160*64)+18 = 10258`

Vertical scrolls copy full width (19 words) with hmod=2; horizontal/diagonal copy 18 of 20 words with vmod=4, leaving a 1-tile-wide gap.

### Direction-to-Scrollmap Mapping (`fmain.c:1999-2228`)

| dif_x | dif_y | scrollmap() | Comment |
|-------|-------|-------------|---------|
| -1 | 0 | scrollmap(0) | scroll right |
| -1 | -1 | scrollmap(1) | scroll down-right |
| 0 | -1 | scrollmap(2) | scroll down |
| 1 | -1 | scrollmap(3) | scroll down-left |
| 1 | 0 | scrollmap(4) | scroll left |
| 1 | 1 | scrollmap(5) | scroll up-left |
| 0 | 1 | scrollmap(6) | scroll up |
| -1 | 1 | scrollmap(7) | scroll up-right |

After the blitter shift, the exposed edge(s) are filled:
- `fmain.c:2363`: `strip_draw(36)` (right edge) or `strip_draw(0)` (left edge) for X movement.
- `fmain.c:2364`: `row_draw(10)` (bottom edge) or `row_draw(0)` (top edge) for Y movement.

## 7. `_genmini` — Minimap Tile Resolution

**Location**: `fsubs.asm:1136-1207`

### Purpose
Fills the 19×6 `minimap[]` array with tile indices by resolving world coordinates through the two-level map hierarchy (region map → sector → tile).

### Parameters
- `d1` = img_x (tile X coordinate, passed from C).
- `d0` = img_y (tile Y coordinate).

### Algorithm (pseudocode)
```
for i in 0..18:                       ; 19 columns
    x = (img_x + i) & 0x7FF
    xs = (x >> 4) - xreg              ; sector X in region map
    if xs out of [0..63]: clamp/wrap
    xs += xreg

    for j in 0..5:                    ; 6 rows
        y = (img_y + j) & 0x7FFF
        ys = (y >> 3) - yreg          ; sector Y in region map
        if ys < 0: ys = 0
        if ys > 31: ys = 31

        sec_num = map_mem[xs + (ys << 7)]      ; region map lookup
        sec_offset = (sec_num << 7) + ((y & 7) << 4) + (x & 15)
        *minimap++ = sector_mem[sec_offset]     ; tile index
    
    img_x++
```

### Key Details
- **Region map** addressing: xs + (ys × 128). The region map is 128 columns × 32 rows of sector indices (`fsubs.asm:1180-1183`).
- **Sector** addressing: sec_num × 128 + (y_within_sector × 16) + x_within_sector. Each sector is 16 tiles wide × 8 tiles tall = 128 bytes (`fsubs.asm:1185-1192`).
- **Hero sector** computation: also computed at the end (`fsubs.asm:1199-1210`) for `_hero_sector`.

## 8. `gen_mini` — C Wrapper for Region/Coordinate Management

**Location**: `fmain.c:2959-2993`

### Purpose
Manages region transitions and calls `genmini()` to fill the minimap.

### Algorithm
1. If `region_num < 8` (outdoor region):
   a. Check for MAP_FLUX boundary conditions (`fmain.c:2964-2968`).
   b. Compute `lregion` from center-of-screen sector coordinates: `xs = (map_x + 151) >> 8`, `ys = (map_y + 64) >> 8`, then `lregion = (xs>>6)&1 + ((ys>>5)&3)*2` (`fmain.c:2971-2975`).
   c. If `lregion != region_num`, trigger `load_all()` to load new region data.
2. If disk I/O complete or MAP_STABLE: update `xreg = (lregion & 1) << 6`, `yreg = (lregion >> 1) << 5` (`fmain.c:2986-2987`).
3. Call `genmini(img_x, img_y)` (`fmain.c:2990`).

### xreg / yreg
- **Declaration**: `unsigned short xreg, yreg` — "where the region is" (`fmain.c:606`).
- **xreg**: 0 or 64 (region X offset in the 128-wide map).
- **yreg**: 0, 32, 64, or 96 (region Y offset in the region map; `yreg = (lregion>>1) << 5`, and lregion>>1 ranges 0–3).

Note: yreg values: lregion 0-1 → yr=0 → yreg=0; lregion 2-3 → yr=1 → yreg=32; lregion 4-5 → yr=2 → yreg=64; lregion 6-7 → yr=3 → yreg=96. But the region map is only 32 rows, and yreg is subtracted from sector coordinates in genmini. So the region map is subdivided into 4 vertical bands of 32/4 = 8 rows? Actually: the fullmap is 128×32 sector indices per region. Each "region" is a 64×32 or similar subdivision, with xreg/yreg as the offset within the full map array. With **8 outdoor regions** (0-7) organized as 2 columns × 4 rows, and xreg=0/64, yreg=0/32, the sum `xreg + yreg<<7` locates the region's sector entries in the map.

Wait, yreg ranges 0-96 but the map is 128×32. Let me re-check: `lregion = xr + yr*2` where `xr=(xs>>6)&1` (0 or 1) and `yr=(ys>>5)&3` (0,1,2,3). So lregion 0-7. `yreg = (lregion>>1) << 5`. lregion>>1 for lregions 0-7: 0,0,1,1,2,2,3,3. So yreg = 0,0,32,32,64,64,96,96. But the region map is only 32 rows... This means each region loads its OWN 128×32 map, and xreg/yreg are offsets within that region's coordinate space.

Actually — each region loads fresh data into `map_mem` via `load_all()`. The 4096-byte map_mem can hold 128×32 = 4096 bytes. With `yreg` up to 96, the `(ys - yreg)` value maps the world's Y sector coordinate into the region's local 32-row space.

## 9. World Coordinates → Tile Coordinates

### Coordinate Hierarchy

| Level | Variable | Bits | Range | Description |
|-------|----------|------|-------|-------------|
| Pixel | `map_x`, `map_y` | 16 | 0–32767 | Absolute pixel position of viewport top-left |
| Tile | `img_x`, `img_y` | — | — | Tile index position of viewport top-left |
| Sector | xs, ys | — | 0–63 / 0–31 | Sector position in region map |
| Region | `region_num` | — | 0–9 | Which region (0–7 outdoor, 8–9 indoor) |

### Conversions
- **Pixel → Tile X**: `img_x = map_x >> 4` (`fmain.c:1980`). Each tile is 16 pixels wide.
- **Pixel → Tile Y**: `img_y = map_y >> 5` (`fmain.c:1981`). Each tile is 32 pixels tall.
- **Tile → Sector X**: `secx = (tile_x >> 4) - xreg` (`fsubs.asm:1153-1156`). Each sector is 16 tiles wide.
- **Tile → Sector Y**: `secy = (tile_y >> 3) - yreg` (`fsubs.asm:1164-1167`). Each sector is 8 tiles tall.
- **Map center → Sector (for region detection)**: `xs = (map_x + 151) >> 8`, `ys = (map_y + 64) >> 8` (`fmain.c:2971-2972`). The +151 and +64 center the calculation on the screen middle (screen is ~304 pixels wide / 2 ≈ 152, and ~128 pixels tall / 2 = 64).
- **MAXCOORD**: `16 * 16 * 128 = 32768` (`fmain.c:632`). Maximum coordinate value; world wraps at 15 bits.

### Scrolling Increments
- `dif_x = img_x - fp_drawing->isv_x` (`fmain.c:1982`): difference in tile X between current and last drawn frame.
- `dif_y = img_y - fp_drawing->isv_y` (`fmain.c:1983`): difference in tile Y.
- If |dif_x| > 1 or |dif_y| > 1 or diagonal with |dif_x|>1, fall back to full `map_draw()` (`fmain.c:2230`).

### `map_adjust` — Smooth Viewport Tracking (`fsubs.asm:1357-1420`)
- Called with hero position; computes `map_x = hero_x - 144`, `map_y = hero_y - 70` (centering hero roughly in viewport).
- Applies dead-zone clamping: only adjusts map_x/map_y by 1 pixel if hero displacement from center is moderate (±20/±10 pixels), or snaps if large (±70/±24–44 pixels).

## 10. `_mapxy` — Map Coordinate to Sector Memory Address

**Location**: `fsubs.asm:1081-1135`

### Purpose
Given a tile X and tile Y coordinate, returns a pointer into `sector_mem` for the terrain byte at that location.

### Algorithm
```
mapxy(imx, imy):
    secx = (imx >> 4) - xreg           ; sector column (0-63), wrapping via bit checks
    if bit 6 set: if bit 5 set → secx=0, else secx=63
    secy = (imy >> 3) - yreg            ; sector row (0-31), wrapping similarly
    if bit 5 set: if bit 4 set → secy=0, else secy=31
    sec_num = map_mem[(secy << 7) + secx + xreg]   ; look up sector index
    offset = (sec_num << 7) + ((imy & 7) << 4) + (imx & 15)
    return &sector_mem[offset]
```

### Parameters and Return
- Input: `d0` = imx (tile X), `d1` = imy (tile Y) (`fsubs.asm:1082-1083`).
- Output: `d0` = pointer to the tile byte in sector_mem (`fsubs.asm:1133`).

### Wrapping Logic (`fsubs.asm:1091-1107`)
Uses a clever bit-test pattern: if bit 6 (for X) or bit 5 (for Y) is set, the coordinate is out of the 0–63 or 0–31 range. Then bit 5 (for X) or bit 4 (for Y) distinguishes underflow (clamp to max) from overflow (clamp to 0).

## 11. `_maskit` — Terrain Occlusion Masking

**Location**: `fsubs.asm:904-939`

### Purpose
Copies a tile's shadow mask from `shadow_mem` into the shape mask buffer (`bmask_mem`), enabling terrain features (trees, buildings) to occlude sprites.

### Parameters (`fsubs.asm:916-919`)
- `d3` (32(sp)) = x offset (in tile units)
- `d4` (36(sp)) = y offset (in tile units)
- `d2` (40(sp)) = modulus (width of mask buffer in words)
- `d0` (44(sp)) = tile index

### Algorithm
1. Source: `shadow_mem + tile_index * 64` (each tile mask is 64 bytes, same planar format as image data).
2. Destination: `bmask_mem + 2 + (y_offset * modulus * 2 * 32) + (x_offset * 2)`. The mask buffer has `modulus` words per row, 32 rows per tile.
3. Copies 32 words (8 iterations × 4 words per iteration) from shadow to mask buffer.

### Usage Context (`fmain.c:2595`)
```c
maskit(xm, ym, blitwide, terra_mem[cm]);
```
Where `terra_mem[cm]` is the tile mask index for the terrain tile (byte 0 of the 4-byte terrain entry).

## 12. `_bigdraw` — Overview Minimap Rendering

**Location**: `fsubs.asm:941-1079`

### Purpose
Renders a zoomed-out overview map (18×9 sectors, each sector shown as a 16×16 pixel block) into the bitplanes. Used by the magic map spell.

### Parameters
- `d0` (60(sp)) = map_x (pixel coordinate)
- `d1` (64(sp)) = map_y (pixel coordinate)

### Algorithm
1. Compute viewport in sector space: `secx = (map_x >> 8) - 9 - xreg`, `secy = (map_y >> 8) - 4 - yreg`.
2. Clamp with wrapping: values wrap through ±128, then clamp to `[0, 64-18]` for X and `[0, 32-9]` for Y (`fsubs.asm:955-980`).
3. Store results in `_secx`, `_secy` globals.
4. Double loop: 9 rows × 18 columns. For each cell:
   a. Look up sector index: `map_mem[(secy+j)*128 + secx+i+xreg]`.
   b. Compute sector data address: `sector_mem + sector_index * 128`.
   c. Call `plotsect` to render one sector.
5. Offset advances: +2 bytes per column (1 word), +(40*15+4) bytes per row between rows.

### `plotsect` subroutine (`fsubs.asm:1008-1062`)
Renders one sector (16×8 terrain tiles) as a 16×16 pixel block:
- Inner loop: 8 rows × 16 columns of terrain bytes.
- For each byte: looks up color from `terra_mem[tile_index * 4 + 3]` (the `big_colors` field).
- Shifts color bits into 5 bitplane data words using `roxr`/`roxl`.
- Each sector row produces one word per plane, written twice (double-height: each sector row = 2 scanlines).

### Call Site (`fmain.c:3313`)
```c
bigdraw(map_x, map_y);
```
Called when the player uses the Map magic spell (stuff[9]).

## Cross-Cutting Findings

- **Terrain occlusion during sprite rendering** (`fmain.c:2578-2595`): The minimap tile indices are used by the sprite rendering loop to determine occlusion priority via `terra_mem[cm+1] & 15`. This means the minimap isn't just for tile drawing — it's also the spatial index for terrain-sprite interaction.
- **Witch effect position** (`fmain.c:2367-2368`): `witchx = abs_x - (map_x & 0xfff0)` and `witchy = abs_y - 15 - (map_y & 0xffe0)`. The mask operations align to tile boundaries (16 and 32 pixel multiples), confirming the 16×32 tile grid.
- **double-buffering interaction**: `pagechange()` (`fmain.c:2993-3007`) swaps `fp_drawing` and `fp_viewing`. The `isv_x`/`isv_y` fields track which tile position each page was last drawn at, so scroll deltas are computed per-page.
- **`map_adjust`** (`fsubs.asm:1357-1420`): Smooth viewport tracking with dead-zones. The viewport tries to keep the hero centered at pixel offset (144, 70) from top-left, but only moves the map by 1 pixel per frame within a ±20/±10 dead zone, or snaps when the hero jumps far.

## Unresolved

None — all questions from the prompt are answered with direct code citations.

## Refinement Log
- 2026-04-06: Initial comprehensive discovery pass covering all 10 questions.
