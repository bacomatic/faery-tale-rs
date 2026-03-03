//! Map view utilities: genmini viewport tile computation and rendering data.

use crate::game::world_data::WorldData;

/// Viewport dimensions in tiles.
pub const VIEWPORT_TILES_W: usize = 19;
pub const VIEWPORT_TILES_H: usize = 6;
pub const VIEWPORT_TILES: usize = VIEWPORT_TILES_W * VIEWPORT_TILES_H; // 114

/// Region/scroll register offsets (set per region, default 0).
pub struct ScrollRegs {
    pub xreg: u16,
    pub yreg: u16,
}

impl Default for ScrollRegs {
    fn default() -> Self { ScrollRegs { xreg: 0, yreg: 0 } }
}

/// Port of gen_mini() from fsubs.asm / fmain.c.
///
/// Given hero position (img_x, img_y) and world data, fills a 19×6 array
/// of tile indices for the current viewport.
///
/// Original algorithm:
///   secx = (img_x >> 4).wrapping_sub(xreg)   — sector X (clamped 0..127)
///   secy = (img_y >> 5) - yreg                 — sector Y (clamped 0..31)
///   for each viewport tile (tx, ty):
///     sx = secx + tx - VIEWPORT_TILES_W/2  (clamped to sector bounds)
///     sy = secy + ty - VIEWPORT_TILES_H/2  (clamped to sector bounds)
///     sec_num = map_mem[sx + sy*128]
///     tile_idx = sector_mem[sec_num*128 + (img_y & 7)*16 + (img_x & 15)]
///   But per-tile pixel offset uses the SAME img_x/img_y for sub-tile scrolling.
pub fn genmini(img_x: u16, img_y: u16, world: &WorldData, regs: &ScrollRegs) -> [u16; VIEWPORT_TILES] {
    let mut minimap = [0u16; VIEWPORT_TILES];

    let secx = (img_x >> 4).wrapping_sub(regs.xreg) as usize;
    let secy = ((img_y >> 5) as usize).saturating_sub(regs.yreg as usize);

    let sub_x = (img_x & 0xF) as usize; // 0..15 pixel offset within tile
    let sub_y = (img_y & 0x7) as usize; // 0..7 pixel offset within tile

    for ty in 0..VIEWPORT_TILES_H {
        for tx in 0..VIEWPORT_TILES_W {
            // Compute sector coordinates for this viewport tile
            let sx = (secx + tx).saturating_sub(VIEWPORT_TILES_W / 2).min(127);
            let sy = (secy + ty).saturating_sub(VIEWPORT_TILES_H / 2).min(31);

            let sec_num = world.sector_at(sx, sy);
            let tile_idx = world.tile_at(sec_num, sub_x, sub_y);
            minimap[ty * VIEWPORT_TILES_W + tx] = tile_idx as u16;
        }
    }
    minimap
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::world_data::WorldData;
    use crate::game::adf::AdfDisk;

    #[test]
    fn test_genmini_size() {
        let adf = AdfDisk::from_bytes(vec![0u8; 2048 * 512]);
        let world = WorldData::load(&adf, 0).unwrap();
        let regs = ScrollRegs::default();
        let minimap = genmini(0, 0, &world, &regs);
        assert_eq!(minimap.len(), VIEWPORT_TILES);
    }

    #[test]
    fn test_genmini_no_panic_at_edges() {
        let adf = AdfDisk::from_bytes(vec![0u8; 2048 * 512]);
        let world = WorldData::load(&adf, 0).unwrap();
        let regs = ScrollRegs::default();
        // Max possible coordinates
        let _ = genmini(0xFFFF, 0xFFFF, &world, &regs);
        let _ = genmini(0, 0, &world, &regs);
    }
}
