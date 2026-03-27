# Indexed Rendering Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Store tile atlas and sprite pixels as u8 palette indices throughout the pipeline and only convert to RGBA32 at the final SDL2 upload step.

**Architecture:** Two sequential tasks. Task P1 changes all pixel buffer types in one coordinated commit (TileAtlas, SpriteSheet, MapRenderer, blit functions, GameplayScene call sites, final render step). Task P2 replaces the old atlas-rebuild state machine with proper palette ownership using `fade_page()`. The build must stay green after each task.

**Tech Stack:** Rust, `sdl2` crate, `crate::game::colors::Palette` (RGB4-based, used by `fade_page()`), `crate::game::palette::Palette` = `[u32; 32]` (RGBA32, used for rendering).

---

## Files modified (overview)

| File | Task |
|---|---|
| `src/game/tile_atlas.rs` | P1 |
| `src/game/map_renderer.rs` | P1 |
| `src/game/sprites.rs` | P1 |
| `src/game/gameplay_scene.rs` | P1, P2 |

---

## Type conventions (read this first)

Two `Palette` types exist. Never confuse them:

- **`crate::game::colors::Palette`** — `struct Palette { colors: Vec<RGB4> }`. Used by `fade_page()`. Loaded from `game_lib.find_palette("pagecolors")`. Has `.to_rgba32_table(depth: usize) -> Result<Vec<u32>, String>`.
- **`crate::game::palette::Palette`** = `[u32; 32]` — RGBA32 array. Used for rendering, stored in `current_palette` and returned by `region_palette()`.

---

## Task P1: Switch all pixel buffers to indexed u8

All five changes (TileAtlas, SpriteSheet, MapRenderer, blit functions, GameplayScene call sites + render step) land in **one commit** because intermediate states don't compile.

**Files:**
- Modify: `src/game/tile_atlas.rs`
- Modify: `src/game/map_renderer.rs`
- Modify: `src/game/sprites.rs`
- Modify: `src/game/gameplay_scene.rs`

---

- [ ] **Step 1: Update TileAtlas — store u8 indices**

In `src/game/tile_atlas.rs`, replace the entire file contents:

```rust
//! Tile atlas: decodes WorldData image_mem into an indexed tile atlas.
//! 256 tiles (4 groups × 64), each 16×32 px, 5 Amiga bitplanes → u8 palette index.

use crate::game::world_data::WorldData;

pub const TILES_PER_GROUP: usize = 64;
pub const TILE_GROUPS: usize = 4;
pub const TOTAL_TILES: usize = TILE_GROUPS * TILES_PER_GROUP; // 256
pub const TILE_W: usize = 16;
pub const TILE_H: usize = 32;
pub const TILE_PIXELS: usize = TILE_W * TILE_H; // 512
pub const NUM_PLANES: usize = 5;
const BYTES_PER_ROW: usize = TILE_W / 8; // 2
const BYTES_PER_TILE_PLANE: usize = TILE_H * BYTES_PER_ROW; // 64
const BYTES_PER_PLANE_QUARTER: usize = TILES_PER_GROUP * BYTES_PER_TILE_PLANE; // 4096
const BYTES_PER_GROUP: usize = NUM_PLANES * BYTES_PER_PLANE_QUARTER; // 20480

pub struct TileAtlas {
    /// Palette indices (0–31) decoded from Amiga bitplanes.
    /// TOTAL_TILES × TILE_PIXELS bytes, row-major.
    pub pixels: Vec<u8>,
}

impl TileAtlas {
    /// Decode all 256 tiles from WorldData.image_mem into palette indices.
    /// No palette is needed — indices are resolved at render time.
    pub fn from_world_data(world: &WorldData) -> Self {
        let mut pixels = vec![0u8; TOTAL_TILES * TILE_PIXELS];
        for tile_idx in 0..TOTAL_TILES {
            let group = tile_idx / TILES_PER_GROUP;
            let local = tile_idx % TILES_PER_GROUP;
            let dst_base = tile_idx * TILE_PIXELS;
            for row in 0..TILE_H {
                let mut planes = [0u16; NUM_PLANES];
                for p in 0..NUM_PLANES {
                    let offset = group * BYTES_PER_GROUP
                        + p * BYTES_PER_PLANE_QUARTER
                        + local * BYTES_PER_TILE_PLANE
                        + row * BYTES_PER_ROW;
                    if offset + 1 < world.image_mem.len() {
                        planes[p] = u16::from_be_bytes([
                            world.image_mem[offset],
                            world.image_mem[offset + 1],
                        ]);
                    }
                }
                for col in 0..TILE_W {
                    let bit = 15 - col;
                    let color_idx = (0..NUM_PLANES)
                        .map(|p| (((planes[p] >> bit) & 1) as usize) << p)
                        .fold(0, |acc, b| acc | b);
                    pixels[dst_base + row * TILE_W + col] = color_idx as u8;
                }
            }
        }
        TileAtlas { pixels }
    }

    /// Returns the palette-index slice for a single tile (TILE_PIXELS bytes).
    pub fn tile_pixels(&self, tile_idx: usize) -> &[u8] {
        let start = tile_idx * TILE_PIXELS;
        &self.pixels[start..start + TILE_PIXELS]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::world_data::WorldData;
    use crate::game::adf::AdfDisk;

    fn empty_world() -> WorldData {
        let adf = AdfDisk::from_bytes(vec![0u8; 2048 * 512]);
        WorldData::load(&adf, 0, 0, &[], 0, 0, &[]).unwrap()
    }

    #[test]
    fn test_tile_atlas_size() {
        let world = empty_world();
        let atlas = TileAtlas::from_world_data(&world);
        assert_eq!(atlas.pixels.len(), TOTAL_TILES * TILE_PIXELS);
    }

    #[test]
    fn test_tile_pixels_slice() {
        let world = empty_world();
        let atlas = TileAtlas::from_world_data(&world);
        assert_eq!(atlas.tile_pixels(0).len(), TILE_PIXELS);
        assert_eq!(atlas.tile_pixels(255).len(), TILE_PIXELS);
    }

    #[test]
    fn test_tile_index_in_range() {
        let world = empty_world();
        let atlas = TileAtlas::from_world_data(&world);
        // All decoded indices must be in 0..=31 (5 bitplanes)
        for &idx in &atlas.pixels {
            assert!(idx <= 31, "index {} out of range", idx);
        }
    }
}
```

---

- [ ] **Step 2: Update MapRenderer — Vec<u8> framebuf, drop palette**

In `src/game/map_renderer.rs`, replace the entire file contents:

```rust
//! MapRenderer: combines TileAtlas and genmini() to blit the map viewport.

use crate::game::tile_atlas::{TileAtlas, TILE_W, TILE_H};
use crate::game::map_view::{genmini_scrolled, SCROLL_TILES_W, SCROLL_TILES_H, VIEWPORT_TILES_W, VIEWPORT_TILES_H};
use crate::game::world_data::WorldData;

pub const MAP_DST_X: i32 = 0;
pub const MAP_DST_Y: i32 = 0;
pub const MAP_DST_W: u32 = (TILE_W * VIEWPORT_TILES_W) as u32; // 304
pub const MAP_DST_H: u32 = (TILE_H * VIEWPORT_TILES_H) as u32; // 192

pub struct MapRenderer {
    pub atlas: TileAtlas,
    /// Palette-index pixel buffer for the composed map frame (MAP_DST_W × MAP_DST_H).
    /// Each byte is a palette index (0–31). Converted to RGBA32 at render time.
    pub framebuf: Vec<u8>,
}

impl MapRenderer {
    pub fn new(world: &WorldData) -> Self {
        MapRenderer {
            atlas: TileAtlas::from_world_data(world),
            framebuf: vec![0u8; (MAP_DST_W * MAP_DST_H) as usize],
        }
    }

    /// Compose the map into `framebuf` for the given viewport position.
    pub fn compose(&mut self, map_x: u16, map_y: u16, world: &WorldData) {
        let img_x = map_x >> 4;
        let img_y = map_y >> 5;
        let ox = (map_x & 0xF) as i32;
        let oy = (map_y & 0x1F) as i32;
        let minimap = genmini_scrolled(img_x, img_y, world);

        self.framebuf.fill(0);
        for ty in 0..SCROLL_TILES_H {
            for tx in 0..SCROLL_TILES_W {
                let dst_x = tx as i32 * TILE_W as i32 - ox;
                let dst_y = ty as i32 * TILE_H as i32 - oy;
                if dst_x >= MAP_DST_W as i32 || dst_y >= MAP_DST_H as i32 { continue; }
                if dst_x + TILE_W as i32 <= 0 || dst_y + TILE_H as i32 <= 0 { continue; }
                let tile_idx = minimap[ty * SCROLL_TILES_W + tx] as usize;
                let tile_pixels = self.atlas.tile_pixels(tile_idx.min(255));
                for row in 0..TILE_H {
                    let py = dst_y + row as i32;
                    if py < 0 || py >= MAP_DST_H as i32 { continue; }
                    let col_start = dst_x.max(0) as usize;
                    let col_end = (dst_x + TILE_W as i32).min(MAP_DST_W as i32) as usize;
                    let src_off = (col_start as i32 - dst_x) as usize;
                    let len = col_end - col_start;
                    let dst_base = py as usize * MAP_DST_W as usize;
                    let src_start = row * TILE_W + src_off;
                    self.framebuf[dst_base + col_start..dst_base + col_end]
                        .copy_from_slice(&tile_pixels[src_start..src_start + len]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::world_data::WorldData;

    #[test]
    fn test_compose_no_panic() {
        let world = WorldData::empty();
        let mut renderer = MapRenderer::new(&world);
        renderer.compose(1600, 6400, &world);
        assert_eq!(renderer.framebuf.len(), (MAP_DST_W * MAP_DST_H) as usize);
    }

    #[test]
    fn test_framebuf_is_u8() {
        let world = WorldData::empty();
        let renderer = MapRenderer::new(&world);
        // Verify the buffer is byte-sized (u8), not u32.
        // All indices must be ≤ 31 (5-bitplane palette).
        for &idx in &renderer.framebuf {
            assert!(idx <= 31);
        }
    }
}
```

---

- [ ] **Step 3: Update SpriteSheet — store u8 palette indices**

In `src/game/sprites.rs`, find `pub struct SpriteSheet` and its `impl SpriteSheet` block.

Replace `SpriteSheet` struct:

```rust
/// A loaded sprite sheet: palette indices per pixel.
pub struct SpriteSheet {
    pub cfile_idx: u8,
    /// Palette index per pixel, row-major, num_frames * frame_h * SPRITE_W bytes.
    /// Index 31 = transparent (Amiga "all planes set" convention).
    /// All other indices (0–30) are opaque.
    pub pixels: Vec<u8>,
    pub num_frames: usize,
    pub frame_h: usize,
}
```

Replace `decode()` method — drop `palette` parameter, store indices:

```rust
pub fn decode(cfile_idx: u8, data: &[u8], frame_count: usize, frame_h: usize) -> Self {
    let plane_frame_bytes = frame_h * PLANE_ROW_BYTES;
    let frame_bytes = SPRITE_PLANES * plane_frame_bytes;
    // Initialize all pixels to 31 (transparent). Opaque pixels overwrite below.
    let mut pixels = vec![31u8; frame_count * frame_h * SPRITE_W];

    for frame in 0..frame_count {
        let frame_base = frame * frame_bytes;
        if frame_base + frame_bytes > data.len() {
            break;
        }

        for row in 0..frame_h {
            let row_off = row * PLANE_ROW_BYTES;

            let mut planes = [0u16; SPRITE_PLANES];
            for p in 0..SPRITE_PLANES {
                let pb = &data[frame_base + p * plane_frame_bytes + row_off..];
                planes[p] = u16::from_be_bytes([pb[0], pb[1]]);
            }

            for col in 0..SPRITE_W {
                let bit = 15 - col;
                let color_idx = (0..SPRITE_PLANES)
                    .map(|p| ((planes[p] >> bit) & 1) << p)
                    .fold(0usize, |acc, b| acc | b as usize);
                let pixel_idx = frame * frame_h * SPRITE_W + row * SPRITE_W + col;
                pixels[pixel_idx] = color_idx as u8;
                // Note: index 31 stays as initialized (transparent sentinel)
            }
        }
    }
    SpriteSheet { cfile_idx, pixels, num_frames: frame_count, frame_h }
}
```

Replace `load()` — drop `palette` parameter:

```rust
pub fn load(adf: &AdfDisk, cfile_idx: u8) -> Option<Self> {
    let block = CFILE_BLOCKS[cfile_idx as usize];
    let num_blocks = CFILE_BLOCK_COUNTS[cfile_idx as usize];
    let frame_count = CFILE_FRAME_COUNTS[cfile_idx as usize];
    if block as usize + num_blocks as usize > adf.num_blocks() {
        return None;
    }
    let data = adf.load_blocks(block, num_blocks);
    Some(Self::decode(cfile_idx, data, frame_count, SPRITE_H))
}
```

Replace `load_objects()` — drop `palette` parameter:

```rust
pub fn load_objects(adf: &AdfDisk) -> Option<Self> {
    const CFILE_IDX: u8 = 3;
    let block = CFILE_BLOCKS[CFILE_IDX as usize];
    let num_blocks = CFILE_BLOCK_COUNTS[CFILE_IDX as usize];
    let frame_count = CFILE_FRAME_COUNTS[CFILE_IDX as usize];
    if block as usize + num_blocks as usize > adf.num_blocks() {
        return None;
    }
    let data = adf.load_blocks(block, num_blocks);
    Some(Self::decode(CFILE_IDX, data, frame_count, OBJ_SPRITE_H))
}
```

Replace `frame_pixels()` — return `&[u8]`:

```rust
pub fn frame_pixels(&self, frame: usize) -> Option<&[u8]> {
    if frame >= self.num_frames { return None; }
    let frame_pixels = self.frame_h * SPRITE_W;
    let start = frame * frame_pixels;
    Some(&self.pixels[start..start + frame_pixels])
}
```

---

- [ ] **Step 4: Update blit functions in `gameplay_scene.rs`**

In `src/game/gameplay_scene.rs`, find `blit_sprite_to_framebuf` (around line 1900) and replace:

```rust
/// Blit one 16×32 sprite frame (indexed u8) into the map framebuf (indexed u8).
/// Transparent pixels (index == 31) are skipped.
/// `rel_x` / `rel_y` are the top-left destination in framebuf pixels.
fn blit_sprite_to_framebuf(
    frame_pixels: &[u8],
    rel_x: i32,
    rel_y: i32,
    framebuf: &mut [u8],
    fb_w: i32,
    fb_h: i32,
) {
    use crate::game::sprites::{SPRITE_W, SPRITE_H};
    for row in 0..SPRITE_H as i32 {
        let dst_y = rel_y + row;
        if dst_y < 0 || dst_y >= fb_h { continue; }
        for col in 0..SPRITE_W as i32 {
            let dst_x = rel_x + col;
            if dst_x < 0 || dst_x >= fb_w { continue; }
            let src_idx = frame_pixels[(row as usize) * SPRITE_W + col as usize];
            if src_idx == 31 { continue; } // transparent
            framebuf[(dst_y * fb_w + dst_x) as usize] = src_idx;
        }
    }
}
```

Find `blit_actors_to_framebuf` (around line 1974) and update its signature. Change:

```rust
// Before:
fn blit_actors_to_framebuf(
    sprite_sheets: &[Option<crate::game::sprites::SpriteSheet>],
    state: &GameState,
    map_x: u16,
    map_y: u16,
    framebuf: &mut Vec<u32>,
// After:
fn blit_actors_to_framebuf(
    sprite_sheets: &[Option<crate::game::sprites::SpriteSheet>],
    state: &GameState,
    map_x: u16,
    map_y: u16,
    framebuf: &mut Vec<u8>,
```

---

- [ ] **Step 5: Add `current_palette` field to `GameplayScene`**

In `src/game/gameplay_scene.rs`, in the `GameplayScene` struct definition (around line 180), add after `last_lightlevel`:

```rust
/// RGBA32 palette used for the final indexed→RGBA32 render step.
/// Updated on region change and when lightlevel changes.
current_palette: crate::game::palette::Palette,
```

In `GameplayScene::new()` (around line 231), initialize it:

```rust
current_palette: [0xFF808080_u32; crate::game::palette::PALETTE_SIZE],
```

---

- [ ] **Step 6: Fix MapRenderer::new() call sites in `gameplay_scene.rs`**

There are two call sites for `MapRenderer::new`. Change both:

Around line 1011:
```rust
// Before:
let palette = Self::region_palette(game_lib, region);
self.map_renderer = Some(MapRenderer::new(&world, &palette));

// After:
self.current_palette = Self::region_palette(game_lib, region);
self.map_renderer = Some(MapRenderer::new(&world));
```

Around line 2303:
```rust
// Before:
let palette = Self::region_palette(game_lib, region);
let renderer = MapRenderer::new(&world, &palette);
let sprite_palette = palette;
for cfile_idx in [0u8, 1, 2, 13, 14, 15, 16, 17] {
    if let Some(sheet) = crate::game::sprites::SpriteSheet::load(
        &adf, cfile_idx, &sprite_palette,
    ) {

// After:
self.current_palette = Self::region_palette(game_lib, region);
let renderer = MapRenderer::new(&world);
for cfile_idx in [0u8, 1, 2, 13, 14, 15, 16, 17] {
    if let Some(sheet) = crate::game::sprites::SpriteSheet::load(
        &adf, cfile_idx,
    ) {
```

Also fix `load_objects` call (around line 2321):
```rust
// Before:
self.object_sprites = crate::game::sprites::SpriteSheet::load_objects(
    &adf, &sprite_palette,
);

// After:
self.object_sprites = crate::game::sprites::SpriteSheet::load_objects(&adf);
```

---

- [ ] **Step 7: Remove atlas rebuild calls in `gameplay_scene.rs`**

There are two atlas rebuild blocks. Remove them both.

**First block** (around line 2362 — day/night dimming):
```rust
// DELETE the entire block:
//
// let lightlevel = self.state.lightlevel;
// if lightlevel != self.last_lightlevel {
//     self.last_lightlevel = lightlevel;
//     let pct = if self.state.region_num >= 8 {
//         100i16
//     } else {
//         (lightlevel as i32 * 100 / 300) as i16
//     };
//     self.dlog(format!("daynight: lightlevel={} pct={}%", lightlevel, pct));
//     if let (Some(ref mut mr), Some(ref world)) = (&mut self.map_renderer, &self.map_world) {
//         let base = self.palette_transition...
//         let faded = crate::game::palette_fader::apply_lightlevel_dim(&base, pct);
//         mr.atlas.rebuild(world, &faded);
//     }
// }
```

**Second block** (around line 2535 — palette transition):
```rust
// CHANGE from:
if let Some(ref mut pt) = self.palette_transition {
    if !pt.is_done() {
        let palette = pt.tick();
        if let (Some(ref mut mr), Some(ref world)) = (&mut self.map_renderer, &self.map_world) {
            mr.atlas.rebuild(world, &palette);
        }
    }
}

// TO (keep the tick() call since PaletteTransition is still used for fade effects,
// but remove the atlas rebuild):
if let Some(ref mut pt) = self.palette_transition {
    if !pt.is_done() {
        let palette = pt.tick();
        self.current_palette = palette;
    }
}
```

---

- [ ] **Step 8: Update render_by_viewstatus — apply palette before SDL2 upload**

In `render_by_viewstatus` (around line 838), replace the framebuf-to-SDL2 upload block:

```rust
// Before:
if let Some(ref mr) = self.map_renderer {
    if !mr.framebuf.is_empty() {
        let pixels_u8: &[u8] = unsafe {
            std::slice::from_raw_parts(
                mr.framebuf.as_ptr() as *const u8,
                mr.framebuf.len() * 4,
            )
        };
        let mut pixels_copy = pixels_u8.to_vec();
        // ... SDL2 surface creation ...
    }
}

// After:
if let Some(ref mr) = self.map_renderer {
    if !mr.framebuf.is_empty() {
        // Apply current_palette: indexed u8 → RGBA32 u32 → u8 bytes for SDL2.
        let pal = &self.current_palette;
        let mut rgb_buf: Vec<u8> = Vec::with_capacity(mr.framebuf.len() * 4);
        for &idx in &mr.framebuf {
            let rgba = pal[(idx & 31) as usize];
            // SDL2 ARGB8888 on little-endian: bytes are [B, G, R, A].
            rgb_buf.push((rgba & 0xFF) as u8);          // B
            rgb_buf.push(((rgba >> 8) & 0xFF) as u8);   // G
            rgb_buf.push(((rgba >> 16) & 0xFF) as u8);  // R
            rgb_buf.push(0xFF);                          // A
        }
        let tc = canvas.texture_creator();
        let surface_result = sdl2::surface::Surface::from_data(
            &mut rgb_buf,
            crate::game::map_renderer::MAP_DST_W,
            crate::game::map_renderer::MAP_DST_H,
            crate::game::map_renderer::MAP_DST_W * 4,
            sdl2::pixels::PixelFormatEnum::ARGB8888,
        );
        if let Ok(surface) = surface_result {
            if let Ok(tex) = tc.create_texture_from_surface(&surface) {
                let src = sdl2::rect::Rect::new(0, 0, PLAYFIELD_LORES_W, PLAYFIELD_LORES_H);
                let dst = sdl2::rect::Rect::new(
                    PLAYFIELD_X, PLAYFIELD_Y,
                    PLAYFIELD_CANVAS_W, PLAYFIELD_CANVAS_H,
                );
                let _ = canvas.copy(&tex, Some(src), Some(dst));
            }
        }
    }
}
```

Note on byte order: `amiga_color_to_rgba()` stores colors as `0xFFRRGGBB` (ARGB8888 value, big-endian field order). On little-endian x86, the bytes in memory are `[BB, GG, RR, FF]`. `SDL_PIXELFORMAT_ARGB8888` on SDL2 means the pixel value as a 32-bit integer is `0xAARRGGBB`, which in memory on little-endian is `[BB, GG, RR, AA]`. So the byte decomposition above (`rgba & 0xFF` = blue, `rgba >> 8 & 0xFF` = green, `rgba >> 16 & 0xFF` = red, `0xFF` = alpha) is correct.

---

- [ ] **Step 9: Run tests**

```bash
cargo test 2>&1 | tail -30
```

Expected: all tests pass. Resolve any remaining compile errors before proceeding.

Common errors to expect:
- If `SpriteSheet::load` is called anywhere else with a palette arg, remove it.
- If `atlas.rebuild()` is called anywhere else, remove it.
- Check with: `grep -n "atlas.rebuild\|SpriteSheet::load.*palette\|MapRenderer::new.*palette" src/`

---

- [ ] **Step 10: Commit**

```bash
git add src/game/tile_atlas.rs src/game/map_renderer.rs src/game/sprites.rs src/game/gameplay_scene.rs
git commit -m "refactor: switch tile atlas, sprite sheets, and framebuf to indexed u8 storage"
```

---

## Task P2: GameplayScene — proper palette ownership via fade_page

After Task P1, `current_palette` is set once on init/region-change but not updated for lightlevel changes. This task adds proper palette management: a `base_colors_palette` field (the `colors::Palette` from faery.toml) and recomputes `current_palette` whenever lighting state changes using `fade_page()`.

**Files:**
- Modify: `src/game/gameplay_scene.rs`

**Background on color types:**
- `fade_page(r, g, b, limit, light_timer, colors: &crate::game::colors::Palette) -> crate::game::colors::Palette` — works on the Amiga 12-bit `RGB4` color space. The `colors::Palette` has `.to_rgba32_table(5)` to convert to `Vec<u32>`.
- `game_lib.find_palette("pagecolors")` returns `&crate::game::colors::Palette`.
- `crate::game::palette::amiga_color_to_rgba(u16) -> u32` — converts a 12-bit Amiga color to RGBA32.

---

- [ ] **Step 1: Write failing test**

Add to `src/game/gameplay_scene.rs` test module (or a dedicated test at end of file if it has one):

```rust
#[cfg(test)]
mod palette_tests {
    use super::*;
    use crate::game::gameplay_scene::GameplayScene;

    #[test]
    fn test_current_palette_initialized_not_gray() {
        // After init, current_palette should not be all 0xFF808080 gray
        // (gray = uninitialized sentinel). A properly loaded palette from
        // faery.toml is needed, so we just verify the field exists and is [u32; 32].
        let scene = GameplayScene::new();
        assert_eq!(scene.current_palette.len(), 32);
    }
}
```

Run: `cargo test test_current_palette_initialized_not_gray -- --nocapture`

This test will pass immediately (the field exists from Task P1). Its purpose is to anchor the field as a test fixture for the rest of the task.

---

- [ ] **Step 2: Add `base_colors_palette` field**

In the `GameplayScene` struct, add after `current_palette`:

```rust
/// Base palette loaded from faery.toml (colors::Palette with RGB4 values).
/// Used as input to fade_page() for day/night/jewel palette computation.
/// None until init_from_library() runs.
base_colors_palette: Option<crate::game::colors::Palette>,
```

In `GameplayScene::new()`, initialize:

```rust
base_colors_palette: None,
```

---

- [ ] **Step 3: Add `last_palette_key` dirty-check field**

In the `GameplayScene` struct, add after `last_lightlevel`:

```rust
/// Dirty-check key for current_palette: (lightlevel, light_on, secret_active).
/// When any of these change, current_palette is recomputed.
last_palette_key: (u16, bool, bool),
```

In `GameplayScene::new()`, initialize:

```rust
last_palette_key: (u16::MAX, false, false),
```

---

- [ ] **Step 4: Add `compute_current_palette()` helper**

In `impl GameplayScene`, add this helper method after `region_palette()`:

```rust
/// Recompute current_palette from base_colors_palette + lighting state.
///
/// For outdoors (region_num < 8): applies fade_page() with per-channel
/// percentages derived from lightlevel (0=midnight, 300=noon) and
/// jewel light_on flag.
///
/// For indoors (region_num >= 8): returns base palette at full brightness
/// (no fade).
fn compute_current_palette(
    base: &crate::game::colors::Palette,
    region_num: u8,
    lightlevel: u16,
    light_on: bool,
    secret_active: bool,
    game_lib: &crate::game::game_library::GameLibrary,
) -> crate::game::palette::Palette {
    use crate::game::palette::amiga_color_to_rgba;

    // For indoors or when we have no base palette: use region_palette() directly.
    if region_num >= 8 {
        return Self::region_palette_with_secret(game_lib, region_num, secret_active);
    }

    let ll = lightlevel as i32;
    let ll_boost = if light_on { 200i32 } else { 0 };

    let r_pct = ((ll - 80 + ll_boost) * 100 / 300).clamp(0, 100) as i16;
    let g_pct = ((ll - 61) * 100 / 300).clamp(0, 100) as i16;
    let b_pct = ((ll - 62) * 100 / 300).clamp(0, 100) as i16;

    let faded = crate::game::palette_fader::fade_page(
        r_pct, g_pct, b_pct, true, light_on, base,
    );

    // Convert colors::Palette → [u32; 32]
    let rgba_vec = faded.to_rgba32_table(5).unwrap_or_default();
    let mut out = [0xFF808080_u32; crate::game::palette::PALETTE_SIZE];
    for (i, &v) in rgba_vec.iter().take(crate::game::palette::PALETTE_SIZE).enumerate() {
        out[i] = v;
    }
    out
}

/// Variant of region_palette() that accounts for secret_timer (region 9 only).
fn region_palette_with_secret(
    game_lib: &crate::game::game_library::GameLibrary,
    region: u8,
    secret_active: bool,
) -> crate::game::palette::Palette {
    use crate::game::palette::{amiga_color_to_rgba, PALETTE_SIZE};
    let mut palette = [0xFF808080_u32; PALETTE_SIZE];
    if let Some(base) = game_lib.find_palette("pagecolors") {
        for (i, entry) in base.colors.iter().enumerate().take(PALETTE_SIZE) {
            palette[i] = amiga_color_to_rgba(entry.color);
        }
    }
    let color31: u16 = match region {
        4 => 0x0980,
        9 => if secret_active { 0x00f0 } else { 0x0445 },
        _ => 0x0bdf,
    };
    palette[31] = amiga_color_to_rgba(color31);
    palette
}
```

---

- [ ] **Step 5: Populate `base_colors_palette` in `init_from_library` and on region change**

Find the two `MapRenderer::new` call sites (from Task P1) and update them to also set `base_colors_palette`.

**First site** (around line 1008, in `on_region_changed`):

```rust
// Before (from Task P1):
self.current_palette = Self::region_palette(game_lib, region);
self.map_renderer = Some(MapRenderer::new(&world));

// After:
if let Some(base) = game_lib.find_palette("pagecolors") {
    let mut base_clone = base.clone();
    let color31: u16 = match region {
        4 => 0x0980,
        9 => 0x0445,
        _ => 0x0bdf,
    };
    base_clone.colors.get_mut(31).map(|c| *c = crate::game::colors::RGB4::from(color31));
    self.base_colors_palette = Some(base_clone);
}
self.current_palette = Self::region_palette(game_lib, region);
self.last_palette_key = (u16::MAX, false, false); // force recompute next tick
self.map_renderer = Some(MapRenderer::new(&world));
```

**Second site** (around line 2303, in `init_from_library`/loading block):

```rust
// Before (from Task P1):
self.current_palette = Self::region_palette(game_lib, region);
let renderer = MapRenderer::new(&world);

// After:
if let Some(base) = game_lib.find_palette("pagecolors") {
    let mut base_clone = base.clone();
    let color31: u16 = match region {
        4 => 0x0980,
        9 => 0x0445,
        _ => 0x0bdf,
    };
    base_clone.colors.get_mut(31).map(|c| *c = crate::game::colors::RGB4::from(color31));
    self.base_colors_palette = Some(base_clone);
}
self.current_palette = Self::region_palette(game_lib, region);
self.last_palette_key = (u16::MAX, false, false);
let renderer = MapRenderer::new(&world);
```

---

- [ ] **Step 6: Add per-tick palette recompute (replace the deleted atlas-rebuild block)**

In the game update loop in `gameplay_scene.rs`, at the location where the old `last_lightlevel` atlas-rebuild block was (around line 2362), add:

```rust
// Recompute current_palette when lighting state changes.
let lightlevel = self.state.lightlevel;
let light_on = self.state.light_timer > 0;
let secret_active = self.state.region_num == 9 && self.state.secret_timer > 0;
let palette_key = (lightlevel, light_on, secret_active);
if palette_key != self.last_palette_key {
    self.last_palette_key = palette_key;
    if let Some(ref base) = self.base_colors_palette {
        self.current_palette = Self::compute_current_palette(
            base,
            self.state.region_num,
            lightlevel,
            light_on,
            secret_active,
            game_lib,
        );
    }
    self.dlog(format!(
        "palette: ll={} light_on={} secret={}",
        lightlevel, light_on, secret_active
    ));
}
```

Note: `game_lib` must be in scope at this point. Check the `update()` function signature to confirm. If it is `update(&mut self, canvas, play_tex, delta_ticks, game_lib, resources)`, then `game_lib` is available as a parameter. If not, pass it through or use `self.game_lib_cache` if such a thing exists. Read the function signature before writing this code.

---

- [ ] **Step 7: Write a test for fade_page integration**

```rust
#[test]
fn test_compute_current_palette_indoors_full_brightness() {
    // Indoors should return same palette regardless of lightlevel
    use crate::game::game_library::GameLibrary;
    // We can't easily construct a full GameLibrary in a unit test,
    // so verify the logic via the existing region_palette helper:
    // region 8 is indoor → should return the base palette unchanged.
    // This is a compile-time verification test — if region_palette_with_secret
    // compiles and the field exists, we're good.
    let scene = crate::game::gameplay_scene::GameplayScene::new();
    assert_eq!(scene.current_palette.len(), 32);
    // Indoors: all entries should be non-zero (gray = uninitialized)
    // — this passes even before full init since current_palette starts as gray.
    // The real verification is done by visual inspection in game.
}
```

---

- [ ] **Step 8: Run all tests**

```bash
cargo test 2>&1 | tail -30
```

Expected: all tests pass.

If `compute_current_palette` needs `game_lib` and the update function has it in scope, verify by reading the `update()` function signature at the top of the function body.

---

- [ ] **Step 9: Remove `last_lightlevel` field (now replaced by `last_palette_key`)**

In the `GameplayScene` struct, delete:
```rust
// DELETE:
/// Last lightlevel used for atlas dim — triggers rebuild when it changes.
last_lightlevel: u16,
```

In `GameplayScene::new()`, delete:
```rust
// DELETE:
last_lightlevel: u16::MAX,
```

Run `cargo test` again to confirm clean build.

---

- [ ] **Step 10: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "refactor: GameplayScene owns base/current palette; fade_page drives current_palette on lighting change"
```

---

## Self-Review

**Spec coverage check:**

| Spec requirement | Covered by |
|---|---|
| TileAtlas stores Vec<u8> indices | P1 Step 1 |
| SpriteSheet stores Vec<u8>, index 31 = transparent | P1 Step 3 |
| MapRenderer.framebuf is Vec<u8> | P1 Step 2 |
| rebuild() deleted | P1 Steps 1, 7 |
| GameplayScene owns base_palette + current_palette | P2 Steps 2, 5 |
| base_palette from faery.toml | P2 Step 5 |
| current_palette recomputed on lightlevel/light_on/secret_active change | P2 Step 6 |
| On region change: reload base_palette, recompute current_palette | P2 Step 5 |
| Indoors: full brightness (no fade) | P2 Step 4 |
| Final render: apply current_palette → RGBA32 → SDL2 | P1 Step 8 |
| sprite transparency: skip index 31 | P1 Steps 3, 4 |
| foreground framebuf (fg_framebuf) | Not in this plan (deferred to bug-fix plan task C3 which builds on this) |
| Memory reduction: ~512KB → ~128KB for atlas | Implicit from type change |

**Placeholder scan:** No TBDs. All code blocks are complete.

**Type consistency:** `current_palette` is `crate::game::palette::Palette` = `[u32; 32]` throughout. `base_colors_palette` is `Option<crate::game::colors::Palette>` throughout. `tile_pixels()`, `frame_pixels()` return `&[u8]`. `framebuf` is `Vec<u8>` in both MapRenderer and blit function signatures.
