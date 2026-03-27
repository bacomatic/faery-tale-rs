# Tile Sprite-Depth Masking — Design Spec

## Problem

Two related rendering bugs in the foreground tile overlay system:

1. **Direction-dependent clipping**: Foreground tiles always clip sprites regardless of approach direction. The original game only masks sprites when they are positioned behind the tile (based on ground-line calculations), not when they are in front of it.

2. **Rectangular overlay**: Foreground tiles overlay their entire rectangular area (including background-colored pixels) instead of only the foreground shape (e.g., just the tree trunk/canopy). The original uses per-pixel bitmasks from `shadow_mem` to control exactly which pixels mask the sprite.

Both bugs stem from the same root cause: the current fg overlay is a simple boolean "always on top" when the original Amiga uses a per-sprite, per-tile-column masking system with 8 condition types and per-pixel bitmasks.

## Approach

Port the complete `maskit()` sprite-depth system from `fmain.c` lines 3134–3184 and `fsubs.asm` lines 1048–1084. This replaces the current fg_framebuf overlay with faithful per-sprite, per-tile-column masking using ground-line logic and shadow_mem bitmasks.

## Data Loading: shadow_mem

`shadow_mem` is a global pre-computed dataset containing per-tile bitmasks for sprite masking. It is loaded from ADF blocks 896–919 (24 blocks × 512 bytes = 12,288 bytes).

Structure: up to 192 mask tiles, each 64 bytes (32 rows × 2 bytes/row = 16 pixels wide × 32 pixels tall, 1 bit per pixel). Indexed by the `maptag` byte from `terra_mem[tile*4+0]`.

### Changes

- Add `shadow_mem: Vec<u8>` loaded once at game startup. Since it is global (not per-region), it belongs on `MapRenderer` (which already owns the atlas and framebuf) or as a field on `GameplayScene`. The masking pass in `gameplay_scene.rs` needs access to it alongside the atlas, so storing it on `MapRenderer` keeps the data co-located.
- Add ADF block reference to `faery.toml` under a new `[global_assets]` section or equivalent (blocks 896–919 are not per-region).
- Load via `adf.rs` `load_blocks(896, 24)` during initialization, before any region is loaded.

## TileAtlas Changes

Replace `fg_flags: [bool; 256]` with two per-tile arrays:

```rust
pub mask_type: [u8; TOTAL_TILES],  // k value (0–7) from terra_mem[tile*4+1] & 0x0f
pub maptag: [u8; TOTAL_TILES],     // shadow_mem index from terra_mem[tile*4+0]
```

Populated during `TileAtlas::from_world_data()` from the existing `terra_mem` data.

## Rendering Pipeline

### Current (broken)

1. `compose()` → all tiles to `framebuf`; fg-flagged tiles also to `fg_framebuf`
2. `blit_actors()` → sprites drawn on `framebuf`
3. Unconditionally overlay every non-transparent `fg_framebuf` pixel onto `framebuf`

### New (faithful)

1. `compose()` → all tiles to `framebuf` only. Remove `fg_framebuf` writes.
2. `blit_actors()` → sprites drawn on `framebuf`
3. **Per-sprite masking pass** — for each rendered actor/object sprite:
   a. Determine which minimap tiles the sprite's screen bounding box overlaps (16×32 tile grid).
   b. For each overlapping tile column (`xm`) and tile row (`ym`):
      - Look up tile index from minimap → get `mask_type` (k) and `maptag` from TileAtlas.
      - Calculate `ystop = ground - ((ym + ym_base) << 5)` where `ground = sprite_y + 32`.
      - Apply the masking-type switch (see below).
      - If masking applies: read the 16×32 bitmask from `shadow_mem[maptag * 64]`.
      - For each pixel where the bitmask bit is set: overwrite `framebuf` with the tile pixel from `TileAtlas.pixels`.
4. Remove `fg_framebuf` from `MapRenderer` entirely.

## Masking-Type Switch Logic

Ported from `fmain.c` lines 3146–3181:

```
match k {
    0 => skip,                                          // transparent — never mask
    1 => if xm == 0 { skip },                          // right-half only
    2 => if ystop > 35 { skip },                        // ground-level
    3 => if hero_sector == 48 && actor_idx != 1 { skip }, // bridge special
    4 => if xm == 0 || ystop > 35 { skip },             // right + ground
    5 => if xm == 0 && ystop > 35 { skip },             // right OR ground
    6 => if ym != 0 { use maptag from tile 64 instead }, // full-if-above
    7 => if ystop > 20 { skip },                         // near-top
}
```

### Special cases

- **FALL state**: If the actor's state is FALL and the tile index ≤ 220, skip masking entirely; otherwise treat as k=3.
- **Case 6 substitution**: When `ym != 0`, replace the current tile's maptag with tile 64's maptag (`terra_mem[64*4+0]`). This gives tall structures a solid mask for rows above the sprite's ground line.
- **Case 3 (bridge)**: Requires access to `hero_sector` and the actor index in the sprite list. Only masks when `hero_sector == 48` and the actor is index 1.

## Variables

Matching the original's variable semantics:

| Variable | Meaning | Calculation |
|----------|---------|-------------|
| `ground` | Sprite's feet Y position | `sprite_y + 32` |
| `ym_base` | Starting tile row on screen | `sprite_screen_y >> 5` |
| `ym_count` | Number of tile rows sprite spans | `((sprite_screen_y + sprite_h) >> 5) - ym_base` |
| `ystop` | Y distance from ground to tile row | `ground - ((ym + ym_base) << 5)` |
| `xm` | Tile column index within sprite | `0..sprite_width_in_16px_cols` |
| `xbw` | Sprite's left edge in tile columns | `sprite_screen_x >> 4` |

## Bitmask Application

For a given tile at minimap position, when masking applies:

1. Read maptag = `atlas.maptag[tile_idx]`
2. Offset into shadow_mem = `maptag as usize * 64`
3. For each row (0–31) within the 16×32 tile:
   - Read 2 bytes (big-endian u16) from `shadow_mem[offset + row * 2]`
   - Each bit (MSB = leftmost pixel) controls one pixel
   - If bit is 1: write `atlas.pixels[tile_idx * 512 + row * 16 + col]` to `framebuf` at the corresponding screen position

## Error Handling

- **shadow_mem bounds**: `maptag * 64 + 63` must be < 12,288. Log and skip if out of range.
- **Minimap bounds**: Clamp sprite overlap calculations to valid minimap indices (`SCROLL_TILES_W × SCROLL_TILES_H`).
- **Tile 64**: Always exists (256 tiles per region). No special handling needed.
- **All actors**: Apply masking to every actor in the sprite list, not just the hero.
- **World objects**: Dropped items rendered via `blit_obj_to_framebuf` also need the masking pass applied.

## Files Changed

| File | Change |
|------|--------|
| `faery.toml` | Add shadow_mem ADF block reference (blocks 896–919) |
| `src/game/game_library.rs` | Parse the new shadow_mem config |
| `src/game/world_data.rs` or new module | Load shadow_mem bytes from ADF |
| `src/game/tile_atlas.rs` | Replace `fg_flags: [bool]` with `mask_type: [u8]` + `maptag: [u8]` |
| `src/game/map_renderer.rs` | Remove `fg_framebuf`; remove fg writes from `compose()` |
| `src/game/gameplay_scene.rs` | Remove fg_framebuf overlay (lines 2924–2929); add per-sprite masking pass after `blit_actors()` |
| New: `src/game/sprite_mask.rs` (or inline) | Masking-type switch logic and bitmask application |

## Testing

### Unit tests
- Test masking-type switch logic with known inputs: verify each k value (0–7) correctly skips or applies based on `ystop`, `xm`, `ym` values.
- Test bitmask decoding: given known shadow_mem bytes, verify correct pixel positions are identified.

### Integration tests
- Load a real region's terra_mem + shadow_mem, verify all maptag indices stay within shadow_mem bounds.
- Verify tile 64 substitution for case 6 returns valid mask data.

### Visual verification
- Run with `--debug --skip-intro` and walk around:
  - **Trees** (expected k=2): sprite visible when approaching from below, masked when behind.
  - **Buildings/walls** (expected k=1): right-half masking only.
  - **Bridges** (expected k=3): special sector 48 behavior.
  - Confirm no rectangular artifacts — only the tree/building shape masks the sprite.
