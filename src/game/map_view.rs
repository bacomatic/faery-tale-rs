//! Map view utilities: genmini viewport tile computation and rendering data.
//!
//! Ported from _genmini in fsubs.asm and gen_mini() in fmain.c.

use crate::game::world_data::WorldData;

/// Viewport dimensions in tiles.
pub const VIEWPORT_TILES_W: usize = 19;
pub const VIEWPORT_TILES_H: usize = 6;
pub const VIEWPORT_TILES: usize = VIEWPORT_TILES_W * VIEWPORT_TILES_H; // 114

/// Extended tile dimensions for sub-tile-offset scrolling (one extra column + row).
pub const SCROLL_TILES_W: usize = 20;
pub const SCROLL_TILES_H: usize = 7;
pub const SCROLL_TILES: usize = SCROLL_TILES_W * SCROLL_TILES_H; // 140

/// Fill the 19×6 minimap tile index array for the given viewport position.
///
/// Parameters:
///   img_x = map_x >> 4  (viewport top-left X in 16-pixel / tile-column units)
///   img_y = map_y >> 5  (viewport top-left Y in 32-pixel / tile-row units)
///   region_num: current region (0..9), used to compute xreg/yreg offsets
///
/// Algorithm ported directly from _genmini in original/fsubs.asm:
///   for i in 0..19 (tile columns), for j in 0..6 (tile rows):
///     x = (img_x + i) & 0x7fff
///     y = (img_y + j) & 0x7fff
///     xs = clamp((x>>4) - xreg, 0..63) + xreg  — absolute sector col (0..127)
///     ys = clamp((y>>3) - yreg, 0..31)           — region-local sector row
///     sec_num = map_mem[xs + ys * 128]
///     tile = sector_mem[sec_num * 128 + (y & 7) * 16 + (x & 15)]
///
/// xreg = (region & 1) << 6  (0 or 64 — which half of the map horizontally)
/// yreg = (region >> 1) << 5  (0, 32, 64, … — which band of the map vertically)
pub fn genmini(img_x: u16, img_y: u16, region_num: u8, world: &WorldData) -> [u16; VIEWPORT_TILES] {
    // Compute xreg/yreg from region_num (gen_mini() in fmain.c:3684-3690).
    // Indoor regions (>= 8): xr = 0 as in original ("if (lregion > 7) xr = 0").
    let xr: u16 = if region_num <= 7 { (region_num & 1) as u16 } else { 0 };
    let yr: u16 = (region_num >> 1) as u16;
    let xreg = xr << 6; // 0 or 64
    let yreg = yr << 5; // 0, 32, 64, 96, …

    let mut minimap = [0u16; VIEWPORT_TILES];

    // Outer loop: 19 tile columns (each = 16 pixels wide)
    for i in 0..VIEWPORT_TILES_W {
        let x = img_x.wrapping_add(i as u16) & 0x7fff;

        // xs: absolute sector column in map_mem (0..127).
        // Subtract xreg to get region-local column, clamp 0..63, add xreg back.
        let xs_raw = (x >> 4) as i16 - xreg as i16;
        let xs = (xs_raw.max(0).min(63) as u16 + xreg) as usize;

        // Inner loop: 6 tile rows (each = 32 pixels tall)
        for j in 0..VIEWPORT_TILES_H {
            let y = img_y.wrapping_add(j as u16) & 0x7fff;

            // ys: region-local sector row in map_mem (0..31).
            let ys_raw = (y >> 3) as i32 - yreg as i32;
            let ys = ys_raw.max(0).min(31) as usize;

            let sec_num = world.sector_at(xs, ys);
            let lx = (x & 0xF) as usize;  // tile column within sector (0..15)
            let ly = (y & 0x7) as usize;  // tile row within sector (0..7)
            let tile_idx = world.tile_at(sec_num, lx, ly);

            minimap[j * VIEWPORT_TILES_W + i] = tile_idx as u16;
        }
    }

    minimap
}

/// Fill the 20×7 minimap tile index array for sub-tile-offset scrolling.
///
/// Identical algorithm to `genmini` but covers one extra column and one extra row so
/// `map_renderer::compose()` can shift the blit by up to 15 px (X) / 31 px (Y) without
/// leaving an unfilled strip at the right or bottom edge of the framebuf.
pub fn genmini_scrolled(img_x: u16, img_y: u16, region_num: u8, world: &WorldData) -> [u16; SCROLL_TILES] {
    let xr: u16 = if region_num <= 7 { (region_num & 1) as u16 } else { 0 };
    let yr: u16 = (region_num >> 1) as u16;
    let xreg = xr << 6;
    let yreg = yr << 5;

    let mut minimap = [0u16; SCROLL_TILES];

    for i in 0..SCROLL_TILES_W {
        let x = img_x.wrapping_add(i as u16) & 0x7fff;
        let xs_raw = (x >> 4) as i16 - xreg as i16;
        let xs = (xs_raw.max(0).min(63) as u16 + xreg) as usize;

        for j in 0..SCROLL_TILES_H {
            let y = img_y.wrapping_add(j as u16) & 0x7fff;
            let ys_raw = (y >> 3) as i32 - yreg as i32;
            let ys = ys_raw.max(0).min(31) as usize;

            let sec_num = world.sector_at(xs, ys);
            let lx = (x & 0xF) as usize;
            let ly = (y & 0x7) as usize;
            let tile_idx = world.tile_at(sec_num, lx, ly);

            minimap[j * SCROLL_TILES_W + i] = tile_idx as u16;
        }
    }

    minimap
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::world_data::WorldData;

    #[test]
    fn test_genmini_size() {
        let world = WorldData::empty();
        let minimap = genmini(0, 0, 0, &world);
        assert_eq!(minimap.len(), VIEWPORT_TILES);
    }

    #[test]
    fn test_genmini_no_panic_at_edges() {
        let world = WorldData::empty();
        // Max possible coordinates
        let _ = genmini(0xFFFF, 0xFFFF, 0, &world);
        let _ = genmini(0, 0, 0, &world);
    }
}
