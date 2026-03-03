//! Amiga Disk Format (ADF) reader.
//!
//! The game/image file is a raw 880 KB ADF disk image (1760 512-byte sectors).
//! We access it as a flat array of 512-byte blocks by index.
//! No filesystem parsing is needed — all data offsets are hardcoded from
//! the original game's hdrive.c block table.

use std::path::Path;
use anyhow::{Context, Result, bail};

/// Number of bytes per ADF block (sector).
pub const BLOCK_SIZE: usize = 512;
/// Total blocks in a standard DD ADF image (880 KB).
pub const TOTAL_BLOCKS: usize = 1760;
/// Total size of the ADF image in bytes.
pub const DISK_SIZE: usize = BLOCK_SIZE * TOTAL_BLOCKS;

pub struct AdfDisk {
    data: Vec<u8>,
}

impl AdfDisk {
    /// Open an ADF disk image from disk.
    pub fn open(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)
            .with_context(|| format!("failed to read ADF image: {}", path.display()))?;
        if data.len() < BLOCK_SIZE {
            bail!("ADF image too small: {} bytes", data.len());
        }
        Ok(AdfDisk { data })
    }

    /// Create an AdfDisk from raw bytes (useful for testing).
    pub fn from_bytes(data: Vec<u8>) -> Self {
        AdfDisk { data }
    }

    /// Returns a slice covering `count` blocks starting at `f_block`.
    /// Panics if the range exceeds the image size.
    pub fn load_blocks(&self, f_block: u32, count: u32) -> &[u8] {
        let start = (f_block as usize) * BLOCK_SIZE;
        let end = start + (count as usize) * BLOCK_SIZE;
        assert!(end <= self.data.len(),
            "ADF block range [{}, {}) exceeds image size {}", f_block, f_block + count, self.data.len() / BLOCK_SIZE);
        &self.data[start..end]
    }

    /// Returns a single block's bytes.
    pub fn block(&self, f_block: u32) -> &[u8] {
        self.load_blocks(f_block, 1)
    }

    /// Total number of available blocks in this image.
    pub fn num_blocks(&self) -> usize {
        self.data.len() / BLOCK_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_adf(blocks: usize) -> AdfDisk {
        let mut data = vec![0u8; blocks * BLOCK_SIZE];
        // Write block index as first byte of each block
        for i in 0..blocks {
            data[i * BLOCK_SIZE] = i as u8;
        }
        AdfDisk::from_bytes(data)
    }

    #[test]
    fn test_load_single_block() {
        let adf = make_adf(4);
        let block = adf.block(2);
        assert_eq!(block.len(), BLOCK_SIZE);
        assert_eq!(block[0], 2); // block index marker
    }

    #[test]
    fn test_load_multiple_blocks() {
        let adf = make_adf(4);
        let slice = adf.load_blocks(1, 2);
        assert_eq!(slice.len(), 2 * BLOCK_SIZE);
        assert_eq!(slice[0], 1);
        assert_eq!(slice[BLOCK_SIZE], 2);
    }

    #[test]
    fn test_num_blocks() {
        let adf = make_adf(10);
        assert_eq!(adf.num_blocks(), 10);
    }

    #[test]
    #[should_panic]
    fn test_out_of_range_panics() {
        let adf = make_adf(2);
        let _ = adf.load_blocks(1, 2); // block 2 doesn't exist
    }
}
