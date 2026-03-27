# Tile Sprite-Depth Masking Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the original Amiga `maskit()` sprite-depth system so foreground tiles conditionally mask sprites based on ground-line position and per-pixel bitmasks from `shadow_mem`, replacing the current broken boolean fg overlay.

**Architecture:** Load the global 12 KB `shadow_mem` bitmask table from ADF blocks 896–919. Replace the boolean `fg_flags` per-tile with `mask_type` (0–7) and `maptag` (shadow_mem index). Remove `fg_framebuf` entirely. After blitting sprites, run a per-sprite masking pass that checks each overlapping tile's mask type against the sprite's ground-line position, then applies the shadow_mem bitmask to selectively re-stamp tile pixels over the sprite.

**Tech Stack:** Rust, SDL2, ADF disk image format (raw 512-byte blocks)

---

## File Structure

| File | Responsibility | Action |
|------|---------------|--------|
| `src/game/sprite_mask.rs` | New module: masking-type switch logic, bitmask application, per-sprite masking pass | Create |
| `src/game/tile_atlas.rs` | Tile decode + per-tile metadata | Modify: replace `fg_flags: [bool]` with `mask_type: [u8]` + `maptag: [u8]` |
| `src/game/map_renderer.rs` | Map tile composition | Modify: remove `fg_framebuf`, store `shadow_mem`, expose `last_minimap` |
| `src/game/gameplay_scene.rs` | Rendering pipeline | Modify: remove fg overlay loop, call sprite masking pass |
| `src/game/world_data.rs` | Data loading from ADF | Modify: add `load_shadow_mem()` |
| `src/game/game_library.rs` | Config parsing | Modify: add `shadow_block` to config |
| `faery.toml` | Asset configuration | Modify: add `shadow_block` field |
| `src/game/mod.rs` | Module declarations | Modify: add `pub mod sprite_mask;` |

---

### Task 1: Add shadow_mem loading infrastructure

Load the 12,288-byte `shadow_mem` dataset from ADF blocks 896–919. This is a global asset (not per-region).

**Files:**
- Modify: `faery.toml`
- Modify: `src/game/game_library.rs`
- Modify: `src/game/world_data.rs`

- [ ] **Step 1: Add shadow_block to faery.toml**

Add the shadow memory block reference to the `[disk]` section:

```toml
[disk]
adf = "game/image"
shadow_block = 896
shadow_count = 24
```

- [ ] **Step 2: Parse shadow_block in game_library.rs**

In the `DiskConfig` struct (or equivalent struct that holds `[disk]` fields), add the two new fields. Find the struct that deserializes the `[disk]` section and add:

```rust
#[serde(default)]
pub shadow_block: u32,
#[serde(default)]
pub shadow_count: u32,
```

- [ ] **Step 3: Add load_shadow_mem to world_data.rs**

Add a standalone function (not on WorldData, since shadow_mem is global):

```rust
/// Load the global shadow_mem bitmask table (12,288 bytes) from ADF.
/// Each of the 256 possible maptag values indexes into this table at
/// maptag * 64 bytes (32 rows × 2 bytes/row, 1 bit per pixel, 16px wide).
pub fn load_shadow_mem(adf: &AdfDisk, block: u32, count: u32) -> Vec<u8> {
    let data = adf.load_blocks(block, count);
    data.to_vec()
}
```

- [ ] **Step 4: Run tests to verify nothing breaks**

Run: `cargo test`
Expected: All existing tests pass (no behavioral changes yet).

- [ ] **Step 5: Commit**

```
feat: add shadow_mem loading infrastructure for sprite-depth masking

Load the 12,288-byte global shadow_mem bitmask table from ADF blocks
896-919. This data is used by the maskit() sprite-depth system for
per-pixel foreground tile masking.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```

---

### Task 2: Replace fg_flags with mask_type and maptag in TileAtlas

Store the per-tile masking metadata needed by the sprite-depth system.

**Files:**
- Modify: `src/game/tile_atlas.rs`

- [ ] **Step 1: Write test for new tile metadata fields**

Add to the existing test module in `tile_atlas.rs`:

```rust
#[test]
fn test_tile_mask_metadata() {
    // Build a minimal terra_mem with known values.
    // tile 0: maptag=5, mask_type=2 (ground-level), terra=0x25 (upper=2, lower=5... wait)
    // terra_mem layout: [tile*4+0]=maptag, [tile*4+1]=terrain byte
    // terrain byte: upper nibble = collision, lower nibble = mask_type
    // So for mask_type=2: terrain byte lower nibble = 2 → 0x02 (or 0x72 with collision=7)
    let mut terra = vec![0u8; 1024];
    // tile 0: maptag=5, mask_type=2
    terra[0] = 5;       // maptag
    terra[1] = 0x72;    // collision=7, mask_type=2
    // tile 1: maptag=0, mask_type=0 (transparent/no mask)
    terra[4] = 0;
    terra[5] = 0x30;    // collision=3, mask_type=0
    // tile 2: maptag=10, mask_type=6 (full-if-above)
    terra[8] = 10;
    terra[9] = 0x16;    // collision=1, mask_type=6

    let image = vec![0u8; 81920];
    let world = WorldData::new_for_test(&terra, &image);
    let atlas = TileAtlas::from_world_data(&world);

    assert_eq!(atlas.mask_type[0], 2);
    assert_eq!(atlas.maptag[0], 5);
    assert_eq!(atlas.mask_type[1], 0);
    assert_eq!(atlas.maptag[1], 0);
    assert_eq!(atlas.mask_type[2], 6);
    assert_eq!(atlas.maptag[2], 10);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_tile_mask_metadata`
Expected: FAIL — `mask_type` and `maptag` fields don't exist yet.

- [ ] **Step 3: Replace fg_flags with mask_type and maptag**

In `tile_atlas.rs`, change the struct definition:

```rust
pub struct TileAtlas {
    pub pixels: Vec<u8>,
    /// Per-tile sprite masking type (0-7) from terra_mem[tile*4+1] & 0x0f.
    /// 0 = no masking, 1-7 = various depth-sort conditions.
    pub mask_type: [u8; TOTAL_TILES],
    /// Per-tile shadow_mem index from terra_mem[tile*4+0].
    /// Used by maskit() to look up the 16×32 per-pixel bitmask.
    pub maptag: [u8; TOTAL_TILES],
}
```

In `from_world_data()`, replace the `fg_flags` initialization and population:

Change the locals at the top of the function from:
```rust
let mut fg_flags = [false; TOTAL_TILES];
```
to:
```rust
let mut mask_type = [0u8; TOTAL_TILES];
let mut maptag = [0u8; TOTAL_TILES];
```

Change the terra_mem extraction block (after the bitplane decode loop) from:
```rust
let terra_off = tile_idx * 4 + 1;
if terra_off < world.terra_mem.len() {
    fg_flags[tile_idx] = (world.terra_mem[terra_off] & 0x0F) != 0;
}
```
to:
```rust
let terra_base = tile_idx * 4;
if terra_base + 1 < world.terra_mem.len() {
    maptag[tile_idx] = world.terra_mem[terra_base];
    mask_type[tile_idx] = world.terra_mem[terra_base + 1] & 0x0F;
}
```

Change the struct return from:
```rust
TileAtlas { pixels, fg_flags }
```
to:
```rust
TileAtlas { pixels, mask_type, maptag }
```

- [ ] **Step 4: Add WorldData::new_for_test helper if it doesn't exist**

Check if `WorldData` has a test constructor. If not, add one in `world_data.rs`:

```rust
#[cfg(test)]
impl WorldData {
    pub fn new_for_test(terra: &[u8], image: &[u8]) -> Self {
        let mut terra_mem = Box::new([0u8; 1024]);
        let len = terra.len().min(1024);
        terra_mem[..len].copy_from_slice(&terra[..len]);
        let mut image_mem = Box::new([0u8; 81920]);
        let ilen = image.len().min(81920);
        image_mem[..ilen].copy_from_slice(&image[..ilen]);
        WorldData {
            sector_mem: Box::new([0u8; 32768]),
            map_mem: Box::new([0u8; 16384]),
            terra_mem,
            image_mem,
            region_num: 0,
        }
    }
}
```

- [ ] **Step 5: Fix compile errors from fg_flags removal**

Search for all references to `fg_flags` in the codebase. There will be references in:
- `map_renderer.rs` line 51 (`self.atlas.fg_flags[clamped]`) — change to: `let k = self.atlas.mask_type[clamped];`
- `map_renderer.rs` lines 65-68 (the `if is_fg { ... }` block) — remove entirely (handled in Task 3)
- Any test files referencing `fg_flags` — update accordingly

- [ ] **Step 6: Run tests**

Run: `cargo test`
Expected: All tests pass including `test_tile_mask_metadata`.

- [ ] **Step 7: Commit**

```
refactor: replace fg_flags with mask_type and maptag in TileAtlas

Store per-tile masking type (0-7) and shadow_mem index instead of a
boolean foreground flag. This is the metadata needed for the faithful
maskit() sprite-depth system.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```

---

### Task 3: Remove fg_framebuf from MapRenderer

Strip out the broken foreground overlay buffer. The masking will be done per-sprite instead.

**Files:**
- Modify: `src/game/map_renderer.rs`
- Modify: `src/game/gameplay_scene.rs`

- [ ] **Step 1: Remove fg_framebuf from MapRenderer struct**

In `map_renderer.rs`, remove the `fg_framebuf` field from the struct:

```rust
pub struct MapRenderer {
    pub atlas: TileAtlas,
    pub framebuf: Vec<u8>,
}
```

Remove `fg_framebuf` from `new()`:

```rust
pub fn new(world: &WorldData) -> Self {
    let buf_size = (MAP_DST_W * MAP_DST_H) as usize;
    MapRenderer {
        atlas: TileAtlas::from_world_data(world),
        framebuf: vec![0u8; buf_size],
    }
}
```

In `compose()`, remove:
- The `self.fg_framebuf.fill(0xFF);` line
- The `let is_fg = ...;` line (or the variable if it was renamed in Task 2)
- The `if is_fg { ... }` block that copies to fg_framebuf (if not already removed in Task 2 step 5)

- [ ] **Step 2: Add shadow_mem and last_minimap to MapRenderer**

The masking pass needs access to the minimap tile indices and shadow_mem. Store them on MapRenderer:

```rust
pub struct MapRenderer {
    pub atlas: TileAtlas,
    pub framebuf: Vec<u8>,
    /// Global shadow_mem bitmask table (12,288 bytes).
    pub shadow_mem: Vec<u8>,
    /// Minimap tile indices from last compose() call (20×7 grid, row-major).
    pub last_minimap: [u16; SCROLL_TILES],
    /// Sub-tile pixel offsets from last compose() (for screen→tile mapping).
    pub last_ox: i32,
    pub last_oy: i32,
}
```

Update `new()` to accept shadow_mem and initialize the new fields:

```rust
pub fn new(world: &WorldData, shadow_mem: Vec<u8>) -> Self {
    let buf_size = (MAP_DST_W * MAP_DST_H) as usize;
    MapRenderer {
        atlas: TileAtlas::from_world_data(world),
        framebuf: vec![0u8; buf_size],
        shadow_mem,
        last_minimap: [0u16; SCROLL_TILES],
        last_ox: 0,
        last_oy: 0,
    }
}
```

In `compose()`, save the minimap and offsets after computing them:

```rust
pub fn compose(&mut self, map_x: u16, map_y: u16, world: &WorldData) {
    let img_x = map_x >> 4;
    let img_y = map_y >> 5;
    let ox = (map_x & 0xF) as i32;
    let oy = (map_y & 0x1F) as i32;
    let minimap = genmini_scrolled(img_x, img_y, world);

    self.last_minimap = minimap;
    self.last_ox = ox;
    self.last_oy = oy;

    self.framebuf.fill(0);
    // ... rest of tile blitting loop (bg only, no fg) ...
```

- [ ] **Step 3: Remove fg overlay from gameplay_scene.rs**

In `gameplay_scene.rs`, remove lines 2924-2929 (the fg_framebuf overlay loop):

```rust
// DELETE this entire block:
// Foreground tile layer: overlay fg pixels on top of sprites.
for (i, &fg_px) in mr.fg_framebuf.iter().enumerate() {
    if fg_px != 0xFF {
        mr.framebuf[i] = fg_px;
    }
}
```

- [ ] **Step 4: Update MapRenderer::new() call sites**

Find where `MapRenderer::new(world)` is called in `gameplay_scene.rs` and pass `shadow_mem`. The shadow_mem should be loaded once and stored on `GameplayScene`. Add a field:

```rust
// In GameplayScene struct:
shadow_mem: Vec<u8>,
```

Load it during scene initialization (wherever the ADF is available). Then pass it when creating MapRenderer:

```rust
MapRenderer::new(world, self.shadow_mem.clone())
```

Look for the call site by searching for `MapRenderer::new`. Update it to pass the shadow data.

- [ ] **Step 5: Run tests**

Run: `cargo test`
Expected: All tests pass. The game will temporarily have NO foreground masking (sprites always in front of tiles). This is expected — Task 4 adds the correct masking.

- [ ] **Step 6: Commit**

```
refactor: remove fg_framebuf, add shadow_mem and minimap to MapRenderer

Remove the broken boolean foreground overlay. Store shadow_mem bitmask
data and last-composed minimap on MapRenderer for use by the upcoming
per-sprite masking pass.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```

---

### Task 4: Implement sprite_mask module with masking logic

The core of the fix: the `maskit()` port that checks each tile's mask type against the sprite's ground-line position and applies per-pixel bitmasks.

**Files:**
- Create: `src/game/sprite_mask.rs`
- Modify: `src/game/mod.rs`

- [ ] **Step 1: Write tests for should_mask_tile()**

Create `src/game/sprite_mask.rs` with tests first:

```rust
//! Sprite-depth masking: per-tile, per-sprite-column ground-line masking
//! ported from fmain.c lines 3134-3184 and fsubs.asm maskit().

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_type_0_never_masks() {
        assert!(!should_mask_tile(0, 0, 0, 0, false, false));
        assert!(!should_mask_tile(0, 1, 30, 0, false, false));
    }

    #[test]
    fn test_mask_type_1_right_half_only() {
        // xm == 0 → skip (left column not masked)
        assert!(!should_mask_tile(1, 0, 0, 0, false, false));
        // xm > 0 → mask applies
        assert!(should_mask_tile(1, 1, 0, 0, false, false));
        assert!(should_mask_tile(1, 2, 0, 0, false, false));
    }

    #[test]
    fn test_mask_type_2_ground_level() {
        // ystop > 35 → skip
        assert!(!should_mask_tile(2, 0, 36, 0, false, false));
        assert!(!should_mask_tile(2, 0, 100, 0, false, false));
        // ystop <= 35 → mask
        assert!(should_mask_tile(2, 0, 35, 0, false, false));
        assert!(should_mask_tile(2, 0, 0, 0, false, false));
        assert!(should_mask_tile(2, 0, -10, 0, false, false));
    }

    #[test]
    fn test_mask_type_3_bridge() {
        // hero_sector == 48 && actor is not index 1 → skip
        assert!(!should_mask_tile(3, 0, 0, 0, true, false));
        // hero_sector != 48 → mask
        assert!(should_mask_tile(3, 0, 0, 0, false, false));
        // hero_sector == 48 but actor IS index 1 → mask
        assert!(should_mask_tile(3, 0, 0, 0, true, true));
    }

    #[test]
    fn test_mask_type_4_right_and_ground() {
        // xm == 0 OR ystop > 35 → skip
        assert!(!should_mask_tile(4, 0, 0, 0, false, false));    // xm==0
        assert!(!should_mask_tile(4, 1, 36, 0, false, false));   // ystop>35
        assert!(!should_mask_tile(4, 0, 50, 0, false, false));   // both
        // xm > 0 AND ystop <= 35 → mask
        assert!(should_mask_tile(4, 1, 35, 0, false, false));
        assert!(should_mask_tile(4, 2, 0, 0, false, false));
    }

    #[test]
    fn test_mask_type_5_right_or_ground() {
        // xm == 0 AND ystop > 35 → skip
        assert!(!should_mask_tile(5, 0, 36, 0, false, false));
        // xm > 0 → mask even if ystop > 35
        assert!(should_mask_tile(5, 1, 50, 0, false, false));
        // ystop <= 35 → mask even if xm == 0
        assert!(should_mask_tile(5, 0, 35, 0, false, false));
        // both conditions fail → mask
        assert!(should_mask_tile(5, 1, 20, 0, false, false));
    }

    #[test]
    fn test_mask_type_7_near_top() {
        // ystop > 20 → skip
        assert!(!should_mask_tile(7, 0, 21, 0, false, false));
        // ystop <= 20 → mask
        assert!(should_mask_tile(7, 0, 20, 0, false, false));
        assert!(should_mask_tile(7, 0, 0, 0, false, false));
    }

    #[test]
    fn test_shadow_bit_at() {
        // shadow_mem format: 32 rows × 2 bytes/row (big-endian u16), MSB = leftmost pixel
        let mut shadow = vec![0u8; 64];
        // Row 0: set bit 15 (pixel 0) and bit 0 (pixel 15)
        shadow[0] = 0x80; // bit 15 set
        shadow[1] = 0x01; // bit 0 set
        // Row 1: all bits set
        shadow[2] = 0xFF;
        shadow[3] = 0xFF;

        assert!(shadow_bit_at(&shadow, 0, 0));    // row 0, col 0 (bit 15)
        assert!(!shadow_bit_at(&shadow, 0, 1));   // row 0, col 1 (bit 14)
        assert!(shadow_bit_at(&shadow, 0, 15));   // row 0, col 15 (bit 0)
        assert!(!shadow_bit_at(&shadow, 0, 7));   // row 0, col 7

        // Row 1: all set
        for col in 0..16 {
            assert!(shadow_bit_at(&shadow, 1, col));
        }

        // Row 2: all clear
        for col in 0..16 {
            assert!(!shadow_bit_at(&shadow, 2, col));
        }
    }
}
```

- [ ] **Step 2: Add module declaration to mod.rs**

In `src/game/mod.rs`, add:

```rust
pub mod sprite_mask;
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test sprite_mask`
Expected: FAIL — functions don't exist yet.

- [ ] **Step 4: Implement should_mask_tile() and shadow_bit_at()**

Add above the test module in `sprite_mask.rs`:

```rust
use crate::game::tile_atlas::{TileAtlas, TILE_W, TILE_H, TILE_PIXELS, TOTAL_TILES};
use crate::game::map_renderer::{MapRenderer, MAP_DST_W, MAP_DST_H};
use crate::game::map_view::{SCROLL_TILES_W, SCROLL_TILES_H};

/// Check whether a tile with masking type `k` should mask a sprite at the given position.
///
/// Returns true if the mask should be applied (sprite goes BEHIND the tile).
/// Returns false if the mask should be skipped (sprite stays in front).
///
/// Ported from fmain.c lines 3149-3179.
///
/// Arguments:
/// - `k`: mask type 0-7 from terra_mem[tile*4+1] & 0x0f
/// - `xm`: tile column index within sprite (0 = leftmost 16px column)
/// - `ystop`: ground - ((ym + ym_base) << 5), signed distance from ground to tile row
/// - `_ym`: tile row index (0 = ground row) — used only for case 6 caller logic
/// - `is_bridge_sector`: true if hero_sector == 48
/// - `is_actor_1`: true if this is actor index 1 (raft)
pub fn should_mask_tile(k: u8, xm: u8, ystop: i32, _ym: u8, is_bridge_sector: bool, is_actor_1: bool) -> bool {
    match k {
        0 => false,
        1 => xm != 0,
        2 => ystop <= 35,
        3 => {
            if is_bridge_sector && !is_actor_1 { false } else { true }
        }
        4 => xm != 0 && ystop <= 35,
        5 => !(xm == 0 && ystop > 35),
        6 => true, // case 6 always masks; caller substitutes maptag when ym != 0
        7 => ystop <= 20,
        _ => false,
    }
}

/// Read one bit from a 64-byte shadow_mem tile mask.
/// Layout: 32 rows × 2 bytes/row (big-endian u16). MSB = leftmost pixel (col 0).
/// Returns true if the bit is set (pixel should be masked/overwritten with tile).
pub fn shadow_bit_at(shadow_tile: &[u8], row: usize, col: usize) -> bool {
    if row >= 32 || col >= 16 || shadow_tile.len() < 64 {
        return false;
    }
    let word = u16::from_be_bytes([shadow_tile[row * 2], shadow_tile[row * 2 + 1]]);
    let bit = 15 - col;
    (word >> bit) & 1 != 0
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test sprite_mask`
Expected: All 8 tests pass.

- [ ] **Step 6: Commit**

```
feat: add sprite_mask module with should_mask_tile() and shadow_bit_at()

Port the maskit() condition logic from fmain.c. The 8 mask types (0-7)
determine when a tile should mask a sprite based on ground-line position,
tile column, and special bridge/sector conditions.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```

---

### Task 5: Implement the per-sprite masking pass

Wire the masking logic into the rendering pipeline. After sprites are blitted, iterate over each sprite's bounding box, check overlapping tiles, and apply shadow_mem bitmasks.

**Files:**
- Modify: `src/game/sprite_mask.rs`
- Modify: `src/game/gameplay_scene.rs`

- [ ] **Step 1: Implement apply_sprite_mask() in sprite_mask.rs**

Add the main masking function:

```rust
/// Describes a sprite that was blitted to the framebuf and needs masking.
pub struct BlittedSprite {
    /// Screen X of sprite's top-left corner in framebuf coordinates.
    pub screen_x: i32,
    /// Screen Y of sprite's top-left corner in framebuf coordinates.
    pub screen_y: i32,
    /// Sprite width in pixels.
    pub width: usize,
    /// Sprite height in pixels.
    pub height: usize,
    /// Ground line Y in framebuf coordinates (sprite feet position).
    /// For characters: screen_y + 32. For objects: screen_y + obj_height.
    pub ground: i32,
    /// True if the actor is in FALL state (fmain.c state==22).
    /// When falling and tile index <= 220, masking is skipped entirely;
    /// otherwise mask type is forced to 3 (bridge).
    pub is_falling: bool,
}

/// Apply sprite-depth masking for one sprite against the tile map.
///
/// For each 16×32 tile that overlaps the sprite's bounding box, checks
/// the tile's mask_type against the sprite's ground-line position. If
/// masking applies, reads the shadow_mem bitmask and re-stamps tile
/// pixels over the sprite area in the framebuf.
///
/// This replaces the old fg_framebuf overlay with per-sprite, per-tile
/// conditional masking faithful to fmain.c lines 3134-3184.
pub fn apply_sprite_mask(
    mr: &mut MapRenderer,
    sprite: &BlittedSprite,
    hero_sector: u16,
    _actor_idx: usize,
) {
    let fb_w = MAP_DST_W as i32;
    let fb_h = MAP_DST_H as i32;
    let ox = mr.last_ox;
    let oy = mr.last_oy;

    let is_bridge_sector = hero_sector == 48;
    let is_actor_1 = _actor_idx == 1;

    // Determine which tile columns and rows the sprite overlaps.
    // Tile column = (screen_x + ox) / 16, tile row = (screen_y + oy) / 32.
    // These are indices into the SCROLL_TILES_W × SCROLL_TILES_H minimap.
    let sprite_left = sprite.screen_x;
    let sprite_right = sprite.screen_x + sprite.width as i32 - 1;
    let sprite_top = sprite.screen_y;
    let sprite_bottom = sprite.screen_y + sprite.height as i32 - 1;

    // Convert framebuf pixel coords to tile grid coords.
    // The tile at minimap[ty][tx] covers framebuf pixels:
    //   x: tx * TILE_W - ox .. tx * TILE_W - ox + TILE_W - 1
    //   y: ty * TILE_H - oy .. ty * TILE_H - oy + TILE_H - 1
    let tx_start = ((sprite_left + ox) as usize) / TILE_W;
    let tx_end = ((sprite_right + ox).max(0) as usize) / TILE_W;
    let ty_start = ((sprite_top + oy) as usize) / TILE_H;
    let ty_end = ((sprite_bottom + oy).max(0) as usize) / TILE_H;

    // ym_base and ground in the original's coordinate space.
    let ground = sprite.ground;
    // ym1 = ystart1 >> 5 in original (ystart1 = clipped sprite top in screen pixels)
    let ym_base = ((sprite_top + oy).max(0) >> 5) as u8;

    for tx in tx_start..=tx_end {
        if tx >= SCROLL_TILES_W { continue; }
        let xm = (tx as i32 - ((sprite_left + ox) as i32 / TILE_W as i32)).max(0) as u8;

        for ty in ty_start..=ty_end {
            if ty >= SCROLL_TILES_H { continue; }

            let tile_idx = mr.last_minimap[ty * SCROLL_TILES_W + tx] as usize;
            if tile_idx >= TOTAL_TILES { continue; }

            let k = mr.atlas.mask_type[tile_idx];
            if k == 0 { continue; } // fast path: no masking

            let ym = ty as u8 - ym_base.min(ty as u8);
            let ystop = ground - ((ym as i32 + ym_base as i32) << 5);

            // FALL state handling (fmain.c lines 3143-3147):
            // When falling and tile index <= 220, skip masking; otherwise force k=3.
            let k = if sprite.is_falling {
                if tile_idx <= 220 { continue; } else { 3u8 }
            } else {
                k
            };

            // Case 6: substitute tile 64's maptag for rows above ground
            let maptag = if k == 6 && ym != 0 {
                let tile64 = 64usize.min(TOTAL_TILES - 1);
                mr.atlas.maptag[tile64]
            } else {
                mr.atlas.maptag[tile_idx]
            };

            if !should_mask_tile(k, xm, ystop, ym, is_bridge_sector, is_actor_1) {
                continue;
            }

            // Apply shadow_mem bitmask: re-stamp tile pixels where mask bit is set.
            let shadow_offset = maptag as usize * 64;
            if shadow_offset + 63 >= mr.shadow_mem.len() { continue; }
            let shadow_tile = &mr.shadow_mem[shadow_offset..shadow_offset + 64];
            let tile_pixels = mr.atlas.tile_pixels(tile_idx);

            // Tile's screen position
            let tile_screen_x = tx as i32 * TILE_W as i32 - ox;
            let tile_screen_y = ty as i32 * TILE_H as i32 - oy;

            for row in 0..TILE_H {
                let py = tile_screen_y + row as i32;
                if py < 0 || py >= fb_h { continue; }
                // Only process rows that overlap the sprite
                if py < sprite_top || py > sprite_bottom { continue; }

                for col in 0..TILE_W {
                    let px = tile_screen_x + col as i32;
                    if px < 0 || px >= fb_w { continue; }
                    // Only process columns that overlap the sprite
                    if px < sprite_left || px > sprite_right { continue; }

                    if shadow_bit_at(shadow_tile, row, col) {
                        let fb_idx = (py * fb_w + px) as usize;
                        let tile_px = tile_pixels[row * TILE_W + col];
                        mr.framebuf[fb_idx] = tile_px;
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Wire masking into gameplay_scene.rs rendering pipeline**

In `gameplay_scene.rs`, after `blit_actors_to_framebuf()` and the world objects loop (and where the old fg overlay was), add the masking pass. Replace the deleted fg overlay block with:

```rust
// Sprite-depth masking: apply per-sprite tile masking.
{
    use crate::game::sprite_mask::{apply_sprite_mask, BlittedSprite};
    use crate::game::sprites::{SPRITE_W, SPRITE_H, OBJ_SPRITE_H};

    // Hero sprite masking
    let (hero_rx, mut hero_ry) = Self::actor_rel_pos(
        self.state.hero_x, self.state.hero_y, map_x, map_y,
    );
    if self.submerged { hero_ry += 8; }
    let hero_sprite = BlittedSprite {
        screen_x: hero_rx,
        screen_y: hero_ry,
        width: SPRITE_W,
        height: SPRITE_H,
        ground: hero_ry + SPRITE_H as i32,
        is_falling: false, // TODO: wire to actor FALL state when actor states are implemented
    };
    apply_sprite_mask(mr, &hero_sprite, self.state.hero_sector, 0);

    // World object masking
    for obj in &self.state.world_objects {
        if !obj.visible || obj.region != self.state.region_num { continue; }
        let rel_x = obj.x as i32 - map_x as i32 - (SPRITE_W as i32 / 2);
        let rel_y = obj.y as i32 - map_y as i32 - (OBJ_SPRITE_H as i32 / 2);
        let obj_sprite = BlittedSprite {
            screen_x: rel_x,
            screen_y: rel_y,
            width: SPRITE_W,
            height: OBJ_SPRITE_H,
            ground: rel_y + OBJ_SPRITE_H as i32,
            is_falling: false,
        };
        apply_sprite_mask(mr, &obj_sprite, self.state.hero_sector, 0);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 4: Build and verify compilation**

Run: `cargo build`
Expected: Clean build with no errors.

- [ ] **Step 5: Commit**

```
feat: implement per-sprite tile-depth masking (maskit port)

Replace the broken fg_framebuf overlay with faithful per-sprite,
per-tile-column masking from fmain.c. Each tile's mask_type (0-7)
is checked against the sprite's ground-line position, and shadow_mem
bitmasks control per-pixel masking. Trees now only occlude sprites
when the sprite is behind them, and only the tree shape (not the
full rectangle) masks the sprite.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```

---

### Task 6: Wire shadow_mem loading into GameplayScene initialization

Connect the shadow_mem loading from Task 1 to the actual game startup path.

**Files:**
- Modify: `src/game/gameplay_scene.rs`

- [ ] **Step 1: Find where GameplayScene loads the ADF and creates MapRenderer**

Search for `MapRenderer::new` in `gameplay_scene.rs` to find the call site. Also search for where `self.map_world` is set and the ADF disk is available. The shadow_mem needs to be loaded from the ADF using the config from `game_library`.

- [ ] **Step 2: Add shadow_mem field to GameplayScene**

Add near the other fields:

```rust
shadow_mem: Vec<u8>,
```

- [ ] **Step 3: Load shadow_mem during initialization**

In the GameplayScene constructor or initialization method, after the ADF is loaded and game_library is available:

```rust
let shadow_mem = if lib.disk.shadow_count > 0 {
    crate::game::world_data::load_shadow_mem(&adf, lib.disk.shadow_block, lib.disk.shadow_count)
} else {
    vec![0u8; 12288] // fallback: no masking data
};
```

Store it: `shadow_mem,` in the struct initialization.

- [ ] **Step 4: Pass shadow_mem when creating MapRenderer**

Update the `MapRenderer::new(world)` call to `MapRenderer::new(world, self.shadow_mem.clone())`.

- [ ] **Step 5: Run tests and build**

Run: `cargo test && cargo build`
Expected: All tests pass, clean build.

- [ ] **Step 6: Commit**

```
feat: wire shadow_mem loading into GameplayScene initialization

Load the global shadow_mem bitmask table from ADF blocks 896-919
during game startup and pass it to MapRenderer for use by the
sprite-depth masking system.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```

---

### Task 7: Visual verification and edge-case fixes

Test the full system visually and fix any coordinate/bounds issues discovered.

**Files:**
- Possibly modify: `src/game/sprite_mask.rs`, `src/game/gameplay_scene.rs`

- [ ] **Step 1: Run the game and test tree masking**

Run: `cargo run -- --debug --skip-intro`

Walk the hero near trees (starting region 3, plains). Verify:
- Approaching from the south (walking up toward tree): sprite should be visible, tree does NOT clip sprite.
- Walking behind the tree (north side): tree canopy should mask the sprite.
- The tree shape (not a rectangle) masks the sprite — transparent pixels of the tile should not overwrite the sprite.

- [ ] **Step 2: Test building/wall masking**

Walk near buildings or walls. Verify:
- Walls with mask_type 1 (right-half) only mask the right 16px column of the sprite.
- No rectangular artifacts.

- [ ] **Step 3: Test edge cases**

- Walk to viewport edges — ensure no panics from out-of-bounds minimap/framebuf access.
- Transition between regions — verify shadow_mem persists correctly.
- Drop items near foreground tiles — verify world objects are also masked.

- [ ] **Step 4: Fix any coordinate issues found**

The most likely issues:
- Off-by-one in tile column/row calculation (tx_start, ty_end)
- ground line calculation (original uses `ystart + 32` where ystart is pre-offset; our `screen_y + SPRITE_H` may need adjustment for the sub-tile offset `(map_y & 31)` that the original adds at fmain.c line 3006)
- Shadow_mem bounds with maptag values near 192 (max index)

If the ground-line offset needs the sub-tile adjustment, update `BlittedSprite.ground` in the hero masking call:

```rust
let sub_tile_oy = (map_y & 0x1F) as i32;
ground: hero_ry + SPRITE_H as i32 + sub_tile_oy,
```

- [ ] **Step 5: Commit any fixes**

```
fix: adjust sprite-depth masking coordinates for edge cases

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 7: Final commit if additional fixes were needed**

```
test: verify sprite-depth masking system complete

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```
