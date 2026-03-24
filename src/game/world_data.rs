//! Game world data for a single region.
//! Mirrors the sector_mem, map_mem, terra_mem, image_mem arrays from fmain.c.

use crate::game::adf::AdfDisk;
use crate::game::palette::{amiga_color_to_rgba, Palette, PALETTE_SIZE};
use anyhow::Result;

/// Block counts for each data segment (from fmain.c memory layout).
pub const SECTOR_BLOCKS: u32 = 64;   // 32768 bytes
pub const MAP_BLOCKS: u32 = 8;       // 4096 bytes
pub const TERRA_BLOCKS: u32 = 1;     // 512 bytes per terra layer
pub const IMAGE_BLOCKS_PER_GROUP: u32 = 40; // 5 planes × 8 blocks per tile group
pub const IMAGE_GROUP_COUNT: u32 = 4;

pub struct WorldData {
    pub sector_mem: Box<[u8; 32768]>,
    pub map_mem: Box<[u8; 4096]>,
    pub terra_mem: Box<[u8; 1024]>,
    pub image_mem: Box<[u8; 81920]>,
    pub region_num: u8,
}

impl WorldData {
    /// Return an empty (zeroed) WorldData for use as a placeholder before real data is loaded.
    pub fn empty() -> Self {
        WorldData {
            sector_mem: Box::new([0u8; 32768]),
            map_mem:    Box::new([0u8; 4096]),
            terra_mem:  Box::new([0u8; 1024]),
            image_mem:  Box::new([0u8; 81920]),
            region_num: 0,
        }
    }

    /// Load world data using explicit ADF block numbers (from faery.toml RegionBlockConfig).
    ///
    /// image_group_blocks: the 4 ADF block offsets for each tile group (file_index[n].image[0..4]).
    /// terra_block / terra2_block: TERRA_BLOCK + terra1/terra2 from file_index.
    /// Each image group is 5 planes × 8 blocks = 40 blocks stored consecutively in the ADF.
    /// They are arranged in image_mem in group-major order (group 0 first, then group 1, etc.)
    /// with each group's 5 planes laid out consecutively (plane 0 then plane 1, etc.).
    pub fn load(
        adf: &AdfDisk,
        region_num: u8,
        sector_block: u32,
        map_block: u32,
        terra_block: u32,
        terra2_block: u32,
        image_group_blocks: &[u32],
    ) -> Result<Self> {
        let mut sector_mem = Box::new([0u8; 32768]);
        let mut map_mem = Box::new([0u8; 4096]);
        let mut terra_mem = Box::new([0u8; 1024]);
        let mut image_mem = Box::new([0u8; 81920]);

        if let Ok(slice) = Self::try_load(adf, sector_block, SECTOR_BLOCKS) {
            sector_mem[..slice.len()].copy_from_slice(slice);
        }
        if let Ok(slice) = Self::try_load(adf, map_block, MAP_BLOCKS) {
            map_mem[..slice.len()].copy_from_slice(slice);
        }
        // Load terra1 into first 512 bytes, terra2 into second 512 bytes.
        if terra_block > 0 {
            if let Ok(slice) = Self::try_load(adf, terra_block, TERRA_BLOCKS) {
                terra_mem[..slice.len().min(512)].copy_from_slice(&slice[..slice.len().min(512)]);
            }
        }
        let t2 = if terra2_block > 0 { terra2_block } else { terra_block + 1 };
        if let Ok(slice) = Self::try_load(adf, t2, TERRA_BLOCKS) {
            terra_mem[512..512 + slice.len().min(512)].copy_from_slice(&slice[..slice.len().min(512)]);
        }

        // Load image groups. Each group = IMAGE_BLOCKS_PER_GROUP (40) consecutive ADF blocks.
        // Groups are packed consecutively in image_mem: group 0 at 0, group 1 at 20480, etc.
        for (gi, &group_block) in image_group_blocks.iter().enumerate().take(IMAGE_GROUP_COUNT as usize) {
            let dest_base = gi * (IMAGE_BLOCKS_PER_GROUP as usize * 512);
            if let Ok(slice) = Self::try_load(adf, group_block, IMAGE_BLOCKS_PER_GROUP) {
                let dest = &mut image_mem[dest_base..dest_base + slice.len()];
                dest.copy_from_slice(slice);
            }
        }

        Ok(WorldData { sector_mem, map_mem, terra_mem, image_mem, region_num })
    }

    fn try_load(adf: &AdfDisk, f_block: u32, count: u32) -> Result<&[u8]> {
        let end_block = f_block + count;
        if end_block as usize > adf.num_blocks() {
            anyhow::bail!("block range [{}, {}) exceeds ADF size", f_block, end_block);
        }
        Ok(adf.load_blocks(f_block, count))
    }

    /// Look up the tile index at map coordinate (mx, my).
    /// mx: 0..128 (sector column), my: 0..32 (sector row)
    pub fn sector_at(&self, mx: usize, my: usize) -> u8 {
        self.map_mem[(my * 128 + mx).min(4095)]
    }

    /// Look up the tile within a sector at local position (lx, ly).
    /// sec_num: sector index from map_mem. lx: 0..16, ly: 0..8
    pub fn tile_at(&self, sec_num: u8, lx: usize, ly: usize) -> u8 {
        let base = (sec_num as usize) * 128;
        self.sector_mem[(base + ly * 16 + lx).min(32767)]
    }

    /// Decode a 32-color Amiga palette from 64 bytes at `palette_block` in the ADF.
    /// Each entry is a big-endian u16 color register (0x0RGB, 12-bit).
    ///
    /// Returns a gray fallback palette if `palette_block` is 0 or out of range.
    pub fn decode_palette(adf: &AdfDisk, palette_block: u32) -> Palette {
        const GRAY: Palette = [0xFF808080_u32; PALETTE_SIZE];
        if palette_block == 0 || palette_block as usize >= adf.num_blocks() {
            return GRAY;
        }
        let data = adf.load_blocks(palette_block, 1);
        std::array::from_fn(|i| {
            let word = u16::from_be_bytes([data[i * 2], data[i * 2 + 1]]);
            amiga_color_to_rgba(word)
        })
    }
}
