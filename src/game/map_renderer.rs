//! MapRenderer: combines TileAtlas and genmini() to blit the map viewport.

use crate::game::tile_atlas::{TileAtlas, TILE_W, TILE_H};
use crate::game::map_view::{genmini_scrolled, SCROLL_TILES_W, SCROLL_TILES_H, VIEWPORT_TILES_W, VIEWPORT_TILES_H};
use crate::game::world_data::WorldData;

/// Destination screen rect for the map viewport.
pub const MAP_DST_X: i32 = 0;
pub const MAP_DST_Y: i32 = 0;
pub const MAP_DST_W: u32 = (TILE_W * VIEWPORT_TILES_W) as u32; // 304
pub const MAP_DST_H: u32 = (TILE_H * VIEWPORT_TILES_H) as u32; // 192

pub struct MapRenderer {
    pub atlas: TileAtlas,
    /// RGBA32 pixel buffer for the composed map frame (MAP_DST_W × MAP_DST_H).
    pub framebuf: Vec<u32>,
}

impl MapRenderer {
    pub fn new(world: &WorldData, palette: &[u32; 32]) -> Self {
        MapRenderer {
            atlas: TileAtlas::from_world_data(world, palette),
            framebuf: vec![0u32; (MAP_DST_W * MAP_DST_H) as usize],
        }
    }

    /// Compose the map into `framebuf` for the given viewport position.
    ///
    /// map_x / map_y: pixel-precise viewport origin in world coordinates.
    /// The sub-tile offsets (map_x & 0xF, map_y & 0x1F) are applied so tiles scroll
    /// smoothly rather than snapping by one full tile per boundary crossing.
    pub fn compose(&mut self, map_x: u16, map_y: u16, region_num: u8, world: &WorldData) {
        let img_x = map_x >> 4;
        let img_y = map_y >> 5;
        let ox = (map_x & 0xF) as i32;   // sub-tile X offset (0–15)
        let oy = (map_y & 0x1F) as i32;  // sub-tile Y offset (0–31)
        let minimap = genmini_scrolled(img_x, img_y, region_num, world);

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
        let palette = [0xFF000000_u32; 32];
        let mut renderer = MapRenderer::new(&world, &palette);
        renderer.compose(1600, 6400, 3, &world); // map_x=1600 → img_x=100, map_y=6400 → img_y=200
        assert_eq!(renderer.framebuf.len(), (MAP_DST_W * MAP_DST_H) as usize);
    }
}
