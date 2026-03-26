//! Map view utilities: genmini viewport tile computation and rendering data.
//!
//! Ported from _genmini in fsubs.asm and gen_mini() in fmain.c.
//!
//! With the full overworld map loaded as a flat 128×128 sector array, no xreg/yreg
//! region offsets are needed — tile coordinates map directly to absolute sector indices.

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
///   img_x = map_x >> 4  (viewport top-left X in tile-column units, 0..2047)
///   img_y = map_y >> 5  (viewport top-left Y in tile-row units, 0..1023)
///
/// Tile coordinates wrap at the world boundary (x: & 0x7ff = mod 2048, y: & 0x3ff = mod 1024)
/// and map directly to absolute sector indices in the flat 128×128 map_mem.
pub fn genmini(img_x: u16, img_y: u16, world: &WorldData) -> [u16; VIEWPORT_TILES] {
    let mut minimap = [0u16; VIEWPORT_TILES];

    for i in 0..VIEWPORT_TILES_W {
        let x = img_x.wrapping_add(i as u16) & 0x7ff; // tile col 0..2047, wraps at world edge
        let xs = (x >> 4) as usize;                    // sector col 0..127

        for j in 0..VIEWPORT_TILES_H {
            let y = img_y.wrapping_add(j as u16) & 0x3ff; // tile row 0..1023, wraps at world edge
            let ys = (y >> 3) as usize;                    // sector row 0..127

            let sec_num = world.sector_at(xs, ys);
            let lx = (x & 0xF) as usize; // tile column within sector (0..15)
            let ly = (y & 0x7) as usize; // tile row within sector (0..7)
            minimap[j * VIEWPORT_TILES_W + i] = world.tile_at(sec_num, lx, ly) as u16;
        }
    }

    minimap
}

/// Fill the 20×7 minimap tile index array for sub-tile-offset scrolling.
///
/// Identical algorithm to `genmini` but covers one extra column and one extra row so
/// `map_renderer::compose()` can shift the blit by up to 15 px (X) / 31 px (Y) without
/// leaving an unfilled strip at the right or bottom edge of the framebuf.
pub fn genmini_scrolled(img_x: u16, img_y: u16, world: &WorldData) -> [u16; SCROLL_TILES] {
    let mut minimap = [0u16; SCROLL_TILES];

    for i in 0..SCROLL_TILES_W {
        let x = img_x.wrapping_add(i as u16) & 0x7ff;
        let xs = (x >> 4) as usize;

        for j in 0..SCROLL_TILES_H {
            let y = img_y.wrapping_add(j as u16) & 0x3ff;
            let ys = (y >> 3) as usize;

            let sec_num = world.sector_at(xs, ys);
            let lx = (x & 0xF) as usize;
            let ly = (y & 0x7) as usize;
            minimap[j * SCROLL_TILES_W + i] = world.tile_at(sec_num, lx, ly) as u16;
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
        let minimap = genmini(0, 0, &world);
        assert_eq!(minimap.len(), VIEWPORT_TILES);
    }

    #[test]
    fn test_genmini_no_panic_at_edges() {
        let world = WorldData::empty();
        let _ = genmini(0xFFFF, 0xFFFF, &world);
        let _ = genmini(0, 0, &world);
    }
}
