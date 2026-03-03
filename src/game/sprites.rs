//! Sprite / shape file loading.
//! Ports the cfiles[18] table, setfig_table[14], and seq_list[] from fmain.c.

use crate::game::adf::AdfDisk;

/// Number of shape files (cfiles).
pub const CFILE_COUNT: usize = 18;
/// Pixels per sprite frame: 16 wide × 32 tall, 5 bitplanes + 1 mask.
pub const SPRITE_W: usize = 16;
pub const SPRITE_H: usize = 32;
pub const SPRITE_PLANES: usize = 5;
/// Bytes per bitplane row: SPRITE_W / 8 = 2 bytes
pub const PLANE_ROW_BYTES: usize = 2;
/// Bytes per sprite frame: SPRITE_H * PLANE_ROW_BYTES * (SPRITE_PLANES + 1 mask)
pub const SPRITE_FRAME_BYTES: usize = SPRITE_H * PLANE_ROW_BYTES * 6;

/// ADF block start for each of the 18 shape files.
/// Placeholder values — exact block numbers require ADF analysis.
pub const CFILE_BLOCKS: [u32; CFILE_COUNT] = [
    600, 608, 616, 624, 632, 640, 648, 656,  // 0..7
    664, 672, 680, 688, 696, 704, 712, 720,  // 8..15
    728, 736,                                 // 16..17
];
/// Number of blocks per shape file (typically 8 × 512 = 4096 bytes = ~8 frames).
pub const CFILE_BLOCK_COUNT: u32 = 8;

/// Entry in setfig_table: maps NPC type → cfile index, animation base, talk flag.
#[derive(Debug, Clone, Copy)]
pub struct SetfigEntry {
    pub cfile_entry: u8,
    pub image_base: u8,
    pub can_talk: bool,
}

/// setfig_table[14] from fmain.c — direct port.
pub const SETFIG_TABLE: [SetfigEntry; 14] = [
    SetfigEntry { cfile_entry: 13, image_base: 0, can_talk: true  }, // 0 wizard
    SetfigEntry { cfile_entry: 13, image_base: 4, can_talk: true  }, // 1 priest
    SetfigEntry { cfile_entry: 14, image_base: 0, can_talk: false }, // 2 guard
    SetfigEntry { cfile_entry: 14, image_base: 1, can_talk: false }, // 3 guard (back)
    SetfigEntry { cfile_entry: 14, image_base: 2, can_talk: false }, // 4 princess
    SetfigEntry { cfile_entry: 14, image_base: 4, can_talk: true  }, // 5 king
    SetfigEntry { cfile_entry: 14, image_base: 6, can_talk: false }, // 6 noble
    SetfigEntry { cfile_entry: 14, image_base: 7, can_talk: false }, // 7 sorceress
    SetfigEntry { cfile_entry: 15, image_base: 0, can_talk: false }, // 8 bartender
    SetfigEntry { cfile_entry: 16, image_base: 0, can_talk: false }, // 9 witch
    SetfigEntry { cfile_entry: 16, image_base: 6, can_talk: false }, // 10 spectre
    SetfigEntry { cfile_entry: 16, image_base: 7, can_talk: false }, // 11 ghost
    SetfigEntry { cfile_entry: 17, image_base: 0, can_talk: true  }, // 12 ranger
    SetfigEntry { cfile_entry: 17, image_base: 4, can_talk: true  }, // 13 beggar
];

/// Sequence list slot names (from fmain.c seq_list[7]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeqSlot {
    Phil = 0,
    Objects = 1,
    Raft = 2,
    Enemy = 3,
    Setfig = 4,
    Carrier = 5,
    Dragon = 6,
}

/// A loaded sprite sheet: raw pixel data as RGBA32.
pub struct SpriteSheet {
    pub cfile_idx: u8,
    /// RGBA32 pixel data, row-major, num_frames * SPRITE_H * SPRITE_W pixels.
    pub pixels: Vec<u32>,
    pub num_frames: usize,
}

impl SpriteSheet {
    /// Decode a sprite sheet from ADF block data.
    /// Each frame is SPRITE_FRAME_BYTES bytes: 5 bitplanes + 1 mask, interleaved by row.
    pub fn decode(cfile_idx: u8, data: &[u8], palette: &[u32; 32]) -> Self {
        let num_frames = data.len() / SPRITE_FRAME_BYTES;
        let mut pixels = vec![0u32; num_frames * SPRITE_H * SPRITE_W];

        for frame in 0..num_frames {
            let frame_data = &data[frame * SPRITE_FRAME_BYTES..];
            for row in 0..SPRITE_H {
                // Each row: 6 planes × 2 bytes (mask is plane 5)
                let plane_offset = row * PLANE_ROW_BYTES * 6;
                let mut planes = [0u16; 6];
                for p in 0..6 {
                    let b = &frame_data[plane_offset + p * PLANE_ROW_BYTES..];
                    planes[p] = u16::from_be_bytes([b[0], b[1]]);
                }
                let mask = planes[5];
                for col in 0..SPRITE_W {
                    let bit = 15 - col;
                    if (mask >> bit) & 1 == 0 { continue; } // transparent
                    let color_idx = (0..SPRITE_PLANES)
                        .map(|p| ((planes[p] >> bit) & 1) << p)
                        .fold(0usize, |acc, b| acc | b as usize);
                    let pixel_idx = frame * SPRITE_H * SPRITE_W + row * SPRITE_W + col;
                    pixels[pixel_idx] = palette[color_idx & 31];
                }
            }
        }
        SpriteSheet { cfile_idx, pixels, num_frames }
    }

    /// Load and decode a sprite sheet from the ADF for a given cfile index.
    /// Returns None if the ADF doesn't have enough blocks.
    pub fn load(adf: &AdfDisk, cfile_idx: u8, palette: &[u32; 32]) -> Option<Self> {
        let block = CFILE_BLOCKS[cfile_idx as usize];
        if block as usize + CFILE_BLOCK_COUNT as usize > adf.num_blocks() {
            return None;
        }
        let data = adf.load_blocks(block, CFILE_BLOCK_COUNT);
        Some(Self::decode(cfile_idx, data, palette))
    }
}
