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
    /// Per-tile foreground flag (terra_mem[tile*4+1] & 0x0F != 0).
    pub fg_flags: [bool; TOTAL_TILES],
}

impl TileAtlas {
    /// Decode all 256 tiles from WorldData.image_mem into palette indices.
    /// No palette needed — indices are resolved at render time.
    pub fn from_world_data(world: &WorldData) -> Self {
        let mut pixels = vec![0u8; TOTAL_TILES * TILE_PIXELS];
        let mut fg_flags = [false; TOTAL_TILES];
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
            // Foreground flag: terra_mem[tile*4+1] lower nibble (mask case selector).
            let terra_off = tile_idx * 4 + 1;
            if terra_off < world.terra_mem.len() {
                fg_flags[tile_idx] = (world.terra_mem[terra_off] & 0x0F) != 0;
            }
        }
        TileAtlas { pixels, fg_flags }
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
        for &idx in &atlas.pixels {
            assert!(idx <= 31, "index {} out of range", idx);
        }
    }
}
