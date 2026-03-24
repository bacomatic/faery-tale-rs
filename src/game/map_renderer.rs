//! MapRenderer: combines TileAtlas and genmini() to blit the map viewport.

use crate::game::tile_atlas::{TileAtlas, TILE_W, TILE_H};
use crate::game::map_view::{genmini, VIEWPORT_TILES_W, VIEWPORT_TILES_H};
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
    /// img_x = map_x >> 4  (viewport top-left X in tile-column units)
    /// img_y = map_y >> 5  (viewport top-left Y in tile-row units)
    /// region_num: current region (used to compute map sector offsets)
    pub fn compose(&mut self, img_x: u16, img_y: u16, region_num: u8, world: &WorldData) {
        let minimap = genmini(img_x, img_y, region_num, world);
        for ty in 0..VIEWPORT_TILES_H {
            for tx in 0..VIEWPORT_TILES_W {
                let tile_idx = minimap[ty * VIEWPORT_TILES_W + tx] as usize;
                let tile_pixels = self.atlas.tile_pixels(tile_idx.min(255));
                let dst_x = tx * TILE_W;
                let dst_y = ty * TILE_H;
                for row in 0..TILE_H {
                    let dst_start = (dst_y + row) * MAP_DST_W as usize + dst_x;
                    let src_start = row * TILE_W;
                    self.framebuf[dst_start..dst_start + TILE_W]
                        .copy_from_slice(&tile_pixels[src_start..src_start + TILE_W]);
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
        renderer.compose(100, 200, 3, &world);
        assert_eq!(renderer.framebuf.len(), (MAP_DST_W * MAP_DST_H) as usize);
    }
}
