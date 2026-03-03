//! Game world data for a single region.
//! Mirrors the sector_mem, map_mem, terra_mem, image_mem arrays from fmain.c.

use crate::game::adf::AdfDisk;
use anyhow::Result;

/// Block counts for each data segment (from fmain.c memory layout).
pub const SECTOR_BLOCKS: u32 = 64;   // 32768 bytes
pub const MAP_BLOCKS: u32 = 8;       // 4096 bytes
pub const TERRA_BLOCKS: u32 = 2;     // 1024 bytes
pub const IMAGE_BLOCKS: u32 = 160;   // 81920 bytes

/// Block offset table for all 8 regions (region 0-7).
/// Values from file_index[] in fmain.c / hdrive.c. Placeholder zeros
/// until exact ADF block numbers are decoded from the original disk image.
/// Index: [region][0=sector, 1=map, 2=terra, 3=image]
pub const REGION_BLOCKS: [[u32; 4]; 9] = [
    [0,   0, 0, 0],    // region 0 — placeholder
    [64,  8, 2, 64],   // region 1 — placeholder
    [128, 16, 4, 224], // region 2 — placeholder
    [192, 24, 6, 384], // region 3 (starting region F3) — placeholder
    [256, 32, 8, 544], // region 4 — placeholder
    [320, 40, 10, 704],// region 5 — placeholder
    [384, 48, 12, 864],// region 6 — placeholder
    [448, 56, 14, 1024],// region 7 — placeholder
    [512, 64, 16, 1184],// region 8 (indoor) — placeholder
];

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

    /// Load world data for the given region from the ADF disk.
    pub fn load(adf: &AdfDisk, region_num: u8) -> Result<Self> {
        let idx = region_num as usize;
        if idx >= REGION_BLOCKS.len() {
            anyhow::bail!("invalid region number: {}", region_num);
        }
        let [sec_blk, map_blk, terra_blk, img_blk] = REGION_BLOCKS[idx];

        let mut sector_mem = Box::new([0u8; 32768]);
        let mut map_mem = Box::new([0u8; 4096]);
        let mut terra_mem = Box::new([0u8; 1024]);
        let mut image_mem = Box::new([0u8; 81920]);

        // Load each segment — gracefully handle if ADF is too small (placeholder offsets)
        if let Ok(slice) = Self::try_load(adf, sec_blk, SECTOR_BLOCKS) {
            sector_mem[..slice.len()].copy_from_slice(slice);
        }
        if let Ok(slice) = Self::try_load(adf, map_blk, MAP_BLOCKS) {
            map_mem[..slice.len()].copy_from_slice(slice);
        }
        if let Ok(slice) = Self::try_load(adf, terra_blk, TERRA_BLOCKS) {
            terra_mem[..slice.len()].copy_from_slice(slice);
        }
        if let Ok(slice) = Self::try_load(adf, img_blk, IMAGE_BLOCKS) {
            image_mem[..slice.len()].copy_from_slice(slice);
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
}
