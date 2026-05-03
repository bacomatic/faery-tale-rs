//! MapRenderer: combines TileAtlas and genmini() to blit the map viewport.

use crate::game::map_view::{
    genmini_scrolled, SCROLL_TILES, SCROLL_TILES_H, SCROLL_TILES_W, VIEWPORT_TILES_H,
    VIEWPORT_TILES_W,
};
use crate::game::tile_atlas::{TileAtlas, TILE_H, TILE_W};
use crate::game::world_data::WorldData;

pub const MAP_DST_X: i32 = 0;
pub const MAP_DST_Y: i32 = 0;
pub const MAP_DST_W: u32 = (TILE_W * VIEWPORT_TILES_W) as u32; // 304
pub const MAP_DST_H: u32 = (TILE_H * VIEWPORT_TILES_H) as u32; // 192

pub struct MapRenderer {
    pub atlas: TileAtlas,
    /// Palette-index pixel buffer (MAP_DST_W × MAP_DST_H bytes).
    pub framebuf: Vec<u8>,
    /// Global shadow_mem bitmask table (12,288 bytes).
    pub shadow_mem: Vec<u8>,
    /// Minimap tile indices from last compose() call (20×7 grid, row-major).
    pub last_minimap: [u16; SCROLL_TILES],
    /// Sub-tile pixel offsets from last compose().
    pub last_ox: i32,
    pub last_oy: i32,
}

impl MapRenderer {
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

    /// Compose the map into `framebuf` for the given viewport position.
    pub fn compose(&mut self, map_x: u16, map_y: u16, world: &WorldData) {
        let img_x = map_x >> 4;
        let img_y = map_y >> 5;
        let ox = (map_x & 0xF) as i32;
        let oy = (map_y & 0x1F) as i32;
        let minimap = genmini_scrolled(img_x, img_y, world);

        self.framebuf.fill(0);
        self.last_minimap = minimap;
        self.last_ox = ox;
        self.last_oy = oy;
        for ty in 0..SCROLL_TILES_H {
            for tx in 0..SCROLL_TILES_W {
                let dst_x = tx as i32 * TILE_W as i32 - ox;
                let dst_y = ty as i32 * TILE_H as i32 - oy;
                if dst_x >= MAP_DST_W as i32 || dst_y >= MAP_DST_H as i32 {
                    continue;
                }
                if dst_x + TILE_W as i32 <= 0 || dst_y + TILE_H as i32 <= 0 {
                    continue;
                }
                let tile_idx = minimap[ty * SCROLL_TILES_W + tx] as usize;
                let clamped = tile_idx.min(255);
                let tile_pixels = self.atlas.tile_pixels(clamped);
                for row in 0..TILE_H {
                    let py = dst_y + row as i32;
                    if py < 0 || py >= MAP_DST_H as i32 {
                        continue;
                    }
                    let col_start = dst_x.max(0) as usize;
                    let col_end = (dst_x + TILE_W as i32).min(MAP_DST_W as i32) as usize;
                    let src_off = (col_start as i32 - dst_x) as usize;
                    let len = col_end - col_start;
                    let dst_base = py as usize * MAP_DST_W as usize;
                    let src_start = row * TILE_W + src_off;
                    // Background: all tiles
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
        let mut renderer = MapRenderer::new(&world, Vec::new());
        renderer.compose(1600, 6400, &world);
        assert_eq!(renderer.framebuf.len(), (MAP_DST_W * MAP_DST_H) as usize);
    }
}
