//! Tile atlas: decodes WorldData image_mem into an indexed tile atlas.
//! 256 tiles (4 groups × 64), each 16×32 px, 4 Amiga bitplanes → RGBA32.

use crate::game::world_data::WorldData;

pub const TILES_PER_GROUP: usize = 64;
pub const TILE_GROUPS: usize = 4;
pub const TOTAL_TILES: usize = TILE_GROUPS * TILES_PER_GROUP; // 256
pub const TILE_W: usize = 16;
pub const TILE_H: usize = 32;
pub const TILE_PIXELS: usize = TILE_W * TILE_H;
/// Each row: 4 bitplanes × 2 bytes = 8 bytes per row. 32 rows = 256 bytes/tile.
pub const TILE_BYTES: usize = TILE_H * (TILE_W / 8) * 4;

pub struct TileAtlas {
    /// RGBA32 pixel data: TOTAL_TILES × TILE_PIXELS pixels, row-major.
    pub pixels: Vec<u32>,
}

impl TileAtlas {
    /// Decode all 256 tiles from WorldData.image_mem using the given palette (32 RGBA32 entries).
    pub fn from_world_data(world: &WorldData, palette: &[u32; 32]) -> Self {
        let mut pixels = vec![0u32; TOTAL_TILES * TILE_PIXELS];
        for tile_idx in 0..TOTAL_TILES {
            let src_offset = tile_idx * TILE_BYTES;
            if src_offset + TILE_BYTES > world.image_mem.len() { break; }
            let tile_data = &world.image_mem[src_offset..src_offset + TILE_BYTES];
            let dst_base = tile_idx * TILE_PIXELS;
            for row in 0..TILE_H {
                // 4 bitplanes, 2 bytes each per row
                let row_offset = row * (TILE_W / 8) * 4;
                let mut planes = [0u16; 4];
                for p in 0..4 {
                    let b = &tile_data[row_offset + p * 2..];
                    planes[p] = u16::from_be_bytes([b[0], b[1]]);
                }
                for col in 0..TILE_W {
                    let bit = 15 - col;
                    let color_idx = (0..4)
                        .map(|p| ((planes[p] >> bit) & 1) << p)
                        .fold(0usize, |acc, b| acc | b as usize);
                    pixels[dst_base + row * TILE_W + col] = palette[color_idx & 31];
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

    #[test]
    fn test_tile_atlas_size() {
        let adf = AdfDisk::from_bytes(vec![0u8; 2048 * 512]);
        let world = WorldData::load(&adf, 0).unwrap();
        let palette = [0xFFFFFFFF_u32; 32];
        let atlas = TileAtlas::from_world_data(&world, &palette);
        assert_eq!(atlas.pixels.len(), TOTAL_TILES * TILE_PIXELS);
    }

    #[test]
    fn test_tile_pixels_slice() {
        let adf = AdfDisk::from_bytes(vec![0u8; 2048 * 512]);
        let world = WorldData::load(&adf, 0).unwrap();
        let palette = [0u32; 32];
        let atlas = TileAtlas::from_world_data(&world, &palette);
        assert_eq!(atlas.tile_pixels(0).len(), TILE_PIXELS);
        assert_eq!(atlas.tile_pixels(255).len(), TILE_PIXELS);
    }
}
