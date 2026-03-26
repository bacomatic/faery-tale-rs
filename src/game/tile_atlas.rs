//! Tile atlas: decodes WorldData image_mem into an indexed tile atlas.
//! 256 tiles (4 groups × 64), each 16×32 px, 4 Amiga bitplanes → RGBA32.

use crate::game::world_data::WorldData;

pub const TILES_PER_GROUP: usize = 64;
pub const TILE_GROUPS: usize = 4;
pub const TOTAL_TILES: usize = TILE_GROUPS * TILES_PER_GROUP; // 256
pub const TILE_W: usize = 16;
pub const TILE_H: usize = 32;
pub const TILE_PIXELS: usize = TILE_W * TILE_H; // 512
pub const NUM_PLANES: usize = 5;
/// 2 bytes per row per bitplane (16 pixels / 8 bits).
const BYTES_PER_ROW: usize = TILE_W / 8; // 2
/// 64 bytes per tile per bitplane (32 rows × 2 bytes).
const BYTES_PER_TILE_PLANE: usize = TILE_H * BYTES_PER_ROW; // 64
/// 4096 bytes: one bitplane of one tile group (64 tiles × 64 bytes = QPLAN_SZ in original).
const BYTES_PER_PLANE_QUARTER: usize = TILES_PER_GROUP * BYTES_PER_TILE_PLANE; // 4096
/// 20480 bytes per tile group (5 planes × 4096 bytes).
const BYTES_PER_GROUP: usize = NUM_PLANES * BYTES_PER_PLANE_QUARTER; // 20480

pub struct TileAtlas {
    /// RGBA32 pixel data: TOTAL_TILES × TILE_PIXELS pixels, row-major.
    pub pixels: Vec<u32>,
}

impl TileAtlas {
    /// Decode all 256 tiles from WorldData.image_mem using the given palette (32 RGBA32 entries).
    ///
    /// image_mem layout (from how WorldData::load() stores groups linearly):
    ///   group G, plane P, tile-in-group L, row R:
    ///   offset = G * BYTES_PER_GROUP + P * BYTES_PER_PLANE_QUARTER + L * BYTES_PER_TILE_PLANE + R * BYTES_PER_ROW
    ///
    /// This matches the ADF on-disk order: group 0 full (5 planes × 8 blocks), then group 1, etc.
    pub fn from_world_data(world: &WorldData, palette: &[u32; 32]) -> Self {
        let mut pixels = vec![0u32; TOTAL_TILES * TILE_PIXELS];
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
                    pixels[dst_base + row * TILE_W + col] = palette[color_idx];
                }
            }
        }
        TileAtlas { pixels }
    }

    /// Rebuild the atlas with a new palette (e.g., on region transition).
    pub fn rebuild(&mut self, world: &WorldData, palette: &[u32; 32]) {
        *self = Self::from_world_data(world, palette);
    }

    /// Returns the RGBA32 pixels for a single tile as a slice.
    pub fn tile_pixels(&self, tile_idx: usize) -> &[u32] {
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
        let palette = [0xFFFFFFFF_u32; 32];
        let atlas = TileAtlas::from_world_data(&world, &palette);
        assert_eq!(atlas.pixels.len(), TOTAL_TILES * TILE_PIXELS);
    }

    #[test]
    fn test_tile_pixels_slice() {
        let world = empty_world();
        let palette = [0u32; 32];
        let atlas = TileAtlas::from_world_data(&world, &palette);
        assert_eq!(atlas.tile_pixels(0).len(), TILE_PIXELS);
        assert_eq!(atlas.tile_pixels(255).len(), TILE_PIXELS);
    }
}
