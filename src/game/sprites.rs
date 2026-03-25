//! Sprite / shape file loading.
//! Ports the cfiles[18] table, setfig_table[14], and seq_list[] from fmain.c.

use crate::game::adf::AdfDisk;

/// Number of shape files (cfiles).
pub const CFILE_COUNT: usize = 18;
/// Pixels per sprite frame: 16 wide × 32 tall, 5 bitplanes (mask is computed at runtime).
pub const SPRITE_W: usize = 16;
pub const SPRITE_H: usize = 32;
pub const SPRITE_PLANES: usize = 5;
/// Bytes per bitplane row: SPRITE_W / 8 = 2 bytes
pub const PLANE_ROW_BYTES: usize = 2;
/// Bytes per plane per frame (all rows of one plane): SPRITE_H * PLANE_ROW_BYTES
pub const PLANE_FRAME_BYTES: usize = SPRITE_H * PLANE_ROW_BYTES; // 64
/// Bytes of shape data per frame (5 planes, no mask): SPRITE_PLANES * PLANE_FRAME_BYTES
pub const SHAPE_FRAME_BYTES: usize = SPRITE_PLANES * PLANE_FRAME_BYTES; // 320

/// ADF block start for each of the 18 shape files (cfiles[].file_id from fmain2.c).
pub const CFILE_BLOCKS: [u32; CFILE_COUNT] = [
    1376, // 0  julian
    1418, // 1  phillip
    1460, // 2  kevin
    1312, // 3  objects
    1348, // 4  raft
    1351, // 5  turtle/carrier
     960, // 6  ogre
    1080, // 7  ghost
    1000, // 8  dknight/spiders
    1040, // 9  necromancer/farmer/loraii
    1160, // 10 dragon
    1120, // 11 bird
    1376, // 12 snake/salamander (reuses julian block)
     936, // 13 wizard/priest
     931, // 14 royal set (guard/princess/king/noble/sorceress)
     941, // 15 bartender
     946, // 16 witch/spectre/ghost
     951, // 17 ranger/beggar
];

/// Number of ADF blocks per shape file (cfiles[].numblocks from fmain2.c).
pub const CFILE_BLOCK_COUNTS: [u32; CFILE_COUNT] = [
    42, // 0  julian
    42, // 1  phillip
    42, // 2  kevin
    36, // 3  objects
     3, // 4  raft
    20, // 5  turtle/carrier
    40, // 6  ogre
    40, // 7  ghost
    40, // 8  dknight/spiders
    40, // 9  necromancer
    12, // 10 dragon
    40, // 11 bird
    40, // 12 snake/salamander
     5, // 13 wizard/priest
     5, // 14 royal set
     5, // 15 bartender
     5, // 16 witch
     5, // 17 ranger/beggar
];

/// Animation frame count per shape file (cfiles[].count from fmain2.c).
///
/// The mask is NOT stored on disk — it is computed at runtime by `make_mask()`
/// (fsubs.asm:1614) and written into the `shape_mem` buffer beyond the shape data.
/// Any extra bytes in the ADF allocation are block-alignment padding (512-byte blocks).
///
/// Key observations from the original data (see commit 83511a3):
/// - Players (0-2):  count=67, numblocks=42 → 67×320=21440 shape bytes (64 padding)
/// - Enemies (6-9,12): count=64, numblocks=40 → 64×320=20480 shape bytes (no padding)
/// - Setfig (13-17): count=8, numblocks=5 → 8×320=2560 shape bytes (no padding)
pub const CFILE_FRAME_COUNTS: [usize; CFILE_COUNT] = [
    67,  // 0  julian
    67,  // 1  phillip
    67,  // 2  kevin
    116, // 3  objects (height=16, decoded separately)
     2,  // 4  raft
    16,  // 5  turtle/carrier
    64,  // 6  ogre
    64,  // 7  ghost
    64,  // 8  dknight/spiders
    64,  // 9  necromancer
     5,  // 10 dragon
     8,  // 11 bird
    64,  // 12 snake/salamander
     8,  // 13 wizard/priest
     8,  // 14 royal set
     8,  // 15 bartender
     8,  // 16 witch
     8,  // 17 ranger/beggar
];

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
    /// Transparent pixels are 0x00000000; opaque pixels have alpha=0xFF (high byte).
    pub pixels: Vec<u32>,
    pub num_frames: usize,
}

impl SpriteSheet {
    /// Decode a sprite sheet from ADF block data.
    ///
    /// On-disk layout (fmain2.c read_shapes / fsubs.asm make_mask):
    ///
    ///   Shape section: frame_count frames × SHAPE_FRAME_BYTES (320) bytes each.
    ///   Within each frame, planes are PLANE-MAJOR (not row-interleaved):
    ///     Frame F: [plane0 64B][plane1 64B][plane2 64B][plane3 64B][plane4 64B]
    ///   → plane P row R of frame F = data[F*320 + P*64 + R*2 .. +2]
    ///
    ///   The mask is NOT stored on disk. It is computed by make_mask() in fsubs.asm:
    ///     mask_bit = NOT(plane0 AND plane1 AND plane2 AND plane3 AND plane4)
    ///   Color index 31 (all planes set) is transparent; all other indices are opaque.
    ///   ("assumes color 31 = transparent" — fsubs.asm:1617)
    ///
    /// `frame_count` must be the cfiles[].count value (not derived from data.len()).
    pub fn decode(cfile_idx: u8, data: &[u8], palette: &[u32; 32], frame_count: usize) -> Self {
        let mut pixels = vec![0u32; frame_count * SPRITE_H * SPRITE_W];

        for frame in 0..frame_count {
            let frame_base = frame * SHAPE_FRAME_BYTES;
            if frame_base + SHAPE_FRAME_BYTES > data.len() {
                break; // shape data truncated
            }

            for row in 0..SPRITE_H {
                let row_off = row * PLANE_ROW_BYTES;

                let mut planes = [0u16; SPRITE_PLANES];
                for p in 0..SPRITE_PLANES {
                    let pb = &data[frame_base + p * PLANE_FRAME_BYTES + row_off..];
                    planes[p] = u16::from_be_bytes([pb[0], pb[1]]);
                }

                for col in 0..SPRITE_W {
                    let bit = 15 - col;
                    let color_idx = (0..SPRITE_PLANES)
                        .map(|p| ((planes[p] >> bit) & 1) << p)
                        .fold(0usize, |acc, b| acc | b as usize);
                    if color_idx == 31 { continue; } // transparent (color 31 = all planes set)
                    let pixel_idx = frame * SPRITE_H * SPRITE_W + row * SPRITE_W + col;
                    pixels[pixel_idx] = palette[color_idx];
                }
            }
        }
        SpriteSheet { cfile_idx, pixels, num_frames: frame_count }
    }

    /// Load and decode a sprite sheet from the ADF for a given cfile index.
    /// Returns None if the ADF doesn't have enough blocks.
    pub fn load(adf: &AdfDisk, cfile_idx: u8, palette: &[u32; 32]) -> Option<Self> {
        let block = CFILE_BLOCKS[cfile_idx as usize];
        let num_blocks = CFILE_BLOCK_COUNTS[cfile_idx as usize];
        let frame_count = CFILE_FRAME_COUNTS[cfile_idx as usize];
        if block as usize + num_blocks as usize > adf.num_blocks() {
            return None;
        }
        let data = adf.load_blocks(block, num_blocks);
        Some(Self::decode(cfile_idx, data, palette, frame_count))
    }

    /// Return the RGBA32 pixel slice for a single frame (SPRITE_H * SPRITE_W pixels).
    /// Returns None for out-of-range frame indices.
    pub fn frame_pixels(&self, frame: usize) -> Option<&[u32]> {
        if frame >= self.num_frames { return None; }
        let start = frame * SPRITE_H * SPRITE_W;
        Some(&self.pixels[start..start + SPRITE_H * SPRITE_W])
    }
}
