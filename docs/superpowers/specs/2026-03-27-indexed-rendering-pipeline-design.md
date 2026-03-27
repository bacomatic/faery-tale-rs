# Indexed Rendering Pipeline Design

**Date:** 2026-03-27
**Scope:** Playfield framebuf + intro storybook scene. HI bar and font rendering are unaffected.

---

## Problem

The current pipeline decodes Amiga bitplanes → RGBA32 at load time, storing fully resolved color pixels in `TileAtlas` and `SpriteSheet`. When the palette changes (day/night cycle, jewel effect, region transitions), `TileAtlas::rebuild()` must re-decode all 256 tiles from scratch. This is wasteful: the tile shape data never changes, only the palette does.

---

## Solution

Store tiles and sprites as palette indices (`u8`, values 0–31) throughout the pipeline. Apply the palette exactly once, at the final render step, just before uploading to SDL2. The Amiga hardware worked this way — bitplanes held indices, palette registers held colors.

---

## Data Model

### `TileAtlas`

- `pixels: Vec<u8>` — raw 5-bit palette indices (0–31) decoded from Amiga bitplanes.
- `from_world_data()` drops the `palette` parameter. Decodes bitplanes → `u8` index per pixel only.
- `rebuild()` is deleted. There is nothing to rebuild; tile shape data never changes.
- `tile_pixels()` returns `&[u8]`.

### `SpriteSheet`

- `pixels: Vec<u8>` — palette indices per pixel.
- Index **31** is the transparency sentinel, matching the Amiga's "all planes set = mask" convention. Currently transparent pixels are `0x00000000`; they become `31u8`.
- `frame_pixels()` returns `&[u8]`.

### `MapRenderer`

- `framebuf: Vec<u8>` — 304×192 = ~58 KB (was ~232 KB as `Vec<u32>`). Initialized to 0.
- `fg_framebuf: Vec<u8>` — same size; 0xFF = "no foreground" sentinel (0xFF is outside the 0–31 index range).
- `compose()` is structurally unchanged — copies `u8` tile slices into `framebuf` via `copy_from_slice()`.

### `Palette`

- `Palette = [u32; 32]` — unchanged. RGBA32. The sole bridge between the indexed and RGB worlds.

---

## Palette Ownership

`GameplayScene` owns two palette fields:

- **`base_palette: Palette`** — loaded from `faery.toml` at `init_from_library()` via `game_lib` (`pagecolors` entries + per-region color[31] override). Reloaded on every region change.
- **`current_palette: Palette`** — derived from `base_palette` by applying `fade_page()`. Recomputed whenever `lightlevel`, `light_on` (jewel timer), or `secret_timer_active` (region 9 hidden passages) changes.

The existing `last_lightlevel` / `last_light_on` state machine and atlas rebuild trigger are **deleted**. The `base_colors_palette: Option<colors::Palette>` field introduced in bug-fix task B1 is folded into this ownership model and does not need to exist as a separate field.

The existing `region_palette()` helper becomes the source of `base_palette`.

**Per-region color[31] rules** (unchanged from original):
- Region 4 (desert): `0x0980`
- Region 9 (dungeon, secret active): `0x00f0`
- Region 9 (dungeon, no secret): `0x0445`
- All others: `0x0bdf`

**Indoors (region_num >= 8):** `current_palette = base_palette` (no fade applied; always full brightness).

---

## Final Render Step

In `render_by_viewstatus()`, after sprites are composited into `framebuf`:

```rust
// Indexed framebuf → RGBA32 using current_palette
let mut rgb_buf = vec![0u32; self.framebuf.len()];
for (i, &idx) in self.framebuf.iter().enumerate() {
    rgb_buf[i] = self.current_palette[idx as usize];
}
// Upload rgb_buf to SDL2 surface as before (reinterpret as &[u8], ARGB8888)
```

Index 31 in `framebuf` maps to `current_palette[31]` — the sky/background color — which is correct for any tile pixel that isn't overwritten by a sprite.

This is the **only place RGB pixels exist** in the playfield pipeline.

---

## Sprite Compositing

**`blit_actors_to_framebuf()`:** The transparency check changes from `if pixel == 0x00000000` to `if pixel == 31`. Otherwise identical.

**Foreground masking (issue #104):** After sprite compositing, merge `fg_framebuf` over `framebuf`:

```rust
for (i, &fg) in self.fg_framebuf.iter().enumerate() {
    if fg != 0xFF {
        self.framebuf[i] = fg;
    }
}
```

---

## Intro Storybook Scene

The storybook framebuf becomes `Vec<u8>`. The scene owns a `palette: Palette` loaded at init. The final upload step applies the same indexed → RGBA32 loop before creating the SDL2 surface.

---

## Impact on Existing Bug Fix Plan

This pipeline change must land **before** the Group B and C bug fix tasks, as it changes the foundation they build on:

| Task | Impact |
|------|--------|
| **B1 (#115/#119)** | Atlas rebuild approach is replaced entirely. `fade_page()` now produces `current_palette` only; no atlas involvement. Simpler than originally planned. |
| **C1 (#108)** | Weapon sprite blit works on `u8` framebuf; same logic, different type. |
| **C2 (#102)** | Carrier offset blit works on `u8` framebuf; no change in logic. |
| **C3 (#104)** | `fg_framebuf: Vec<u8>` and 0xFF sentinel — cleaner than the `Vec<u32>` non-zero check. |
| **C4 (#101)** | `secret_timer_active` now triggers `current_palette` recompute only; atlas rebuild removed. |
| **Group A tasks** | Unaffected. |
| **Group D/E tasks** | Unaffected. |

---

## Acceptance Criteria

- `cargo test` passes throughout.
- Day/night cycle changes color without any atlas decode — only `current_palette` recomputes.
- Game starts at full brightness (lightlevel=300, daynight=8000).
- At midnight outdoors, blue cast visible; jewel produces amber cast. Indoors always full brightness.
- Sprite transparency works correctly (no index-31 pixels appear as opaque).
- Foreground tiles (trees, walls) render over sprites.
- Memory for tile atlas reduced from ~512 KB to ~128 KB.
