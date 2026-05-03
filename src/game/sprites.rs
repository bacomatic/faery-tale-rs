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
    960,  // 6  ogre
    1080, // 7  ghost
    1000, // 8  dknight/spiders
    1040, // 9  necromancer/farmer/loraii
    1160, // 10 dragon
    1120, // 11 bird
    1376, // 12 snake/salamander (reuses julian block)
    936,  // 13 wizard/priest
    931,  // 14 royal set (guard/princess/king/noble/sorceress)
    941,  // 15 bartender
    946,  // 16 witch/spectre/ghost
    951,  // 17 ranger/beggar
];

/// Number of ADF blocks per shape file (cfiles[].numblocks from fmain2.c).
pub const CFILE_BLOCK_COUNTS: [u32; CFILE_COUNT] = [
    42, // 0  julian
    42, // 1  phillip
    42, // 2  kevin
    36, // 3  objects
    3,  // 4  raft
    20, // 5  turtle/carrier
    40, // 6  ogre
    40, // 7  ghost
    40, // 8  dknight/spiders
    40, // 9  necromancer
    12, // 10 dragon
    40, // 11 bird
    40, // 12 snake/salamander
    5,  // 13 wizard/priest
    5,  // 14 royal set
    5,  // 15 bartender
    5,  // 16 witch
    5,  // 17 ranger/beggar
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
    2,   // 4  raft
    16,  // 5  turtle/carrier
    64,  // 6  ogre
    64,  // 7  ghost
    64,  // 8  dknight/spiders
    64,  // 9  necromancer
    5,   // 10 dragon
    8,   // 11 bird
    64,  // 12 snake/salamander
    8,   // 13 wizard/priest
    8,   // 14 royal set
    8,   // 15 bartender
    8,   // 16 witch
    8,   // 17 ranger/beggar
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
    SetfigEntry {
        cfile_entry: 13,
        image_base: 0,
        can_talk: true,
    }, // 0 wizard
    SetfigEntry {
        cfile_entry: 13,
        image_base: 4,
        can_talk: true,
    }, // 1 priest
    SetfigEntry {
        cfile_entry: 14,
        image_base: 0,
        can_talk: false,
    }, // 2 guard
    SetfigEntry {
        cfile_entry: 14,
        image_base: 1,
        can_talk: false,
    }, // 3 guard (back)
    SetfigEntry {
        cfile_entry: 14,
        image_base: 2,
        can_talk: false,
    }, // 4 princess
    SetfigEntry {
        cfile_entry: 14,
        image_base: 4,
        can_talk: true,
    }, // 5 king
    SetfigEntry {
        cfile_entry: 14,
        image_base: 6,
        can_talk: false,
    }, // 6 noble
    SetfigEntry {
        cfile_entry: 14,
        image_base: 7,
        can_talk: false,
    }, // 7 sorceress
    SetfigEntry {
        cfile_entry: 15,
        image_base: 0,
        can_talk: false,
    }, // 8 bartender
    SetfigEntry {
        cfile_entry: 16,
        image_base: 0,
        can_talk: false,
    }, // 9 witch
    SetfigEntry {
        cfile_entry: 16,
        image_base: 6,
        can_talk: false,
    }, // 10 spectre
    SetfigEntry {
        cfile_entry: 16,
        image_base: 7,
        can_talk: false,
    }, // 11 ghost
    SetfigEntry {
        cfile_entry: 17,
        image_base: 0,
        can_talk: true,
    }, // 12 ranger
    SetfigEntry {
        cfile_entry: 17,
        image_base: 4,
        can_talk: true,
    }, // 13 beggar
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

/// Frame height for objects sprite sheet (cfiles[3]: height=16, vs SPRITE_H=32 for others).
pub const OBJ_SPRITE_H: usize = 16;
/// Bytes per bitplane per frame for objects (OBJ_SPRITE_H × PLANE_ROW_BYTES).
pub const OBJ_PLANE_FRAME_BYTES: usize = OBJ_SPRITE_H * PLANE_ROW_BYTES; // 32
/// Total bytes per objects frame (5 planes × OBJ_PLANE_FRAME_BYTES).
pub const OBJ_SHAPE_FRAME_BYTES: usize = SPRITE_PLANES * OBJ_PLANE_FRAME_BYTES; // 160

/// An item entry from inv_list[] (fmain.c:428). Describes how to render an inventory item.
#[derive(Clone, Copy)]
pub struct InvItem {
    /// Sprite frame index in the OBJECTS sequence (seq_list[OBJECTS]).
    pub image_number: u8,
    /// X offset on the lores inventory canvas (dest x = xoff + 20).
    pub xoff: u8,
    /// Y offset on the lores inventory canvas.
    pub yoff: u8,
    /// Y increment for stacked items (each additional item is drawn ydelta pixels lower).
    pub ydelta: u8,
    /// Row within the sprite frame to start blitting from.
    pub img_off: u8,
    /// Number of rows to blit from the sprite frame.
    pub img_height: u8,
    /// Maximum number of items to display on-screen.
    pub maxshown: u8,
}

/// inv_list[] from fmain.c:428 — direct port.
/// Indexed by stuff[] slot (0..GOLDBASE=31). Gold piles (31–34) are excluded from display.
pub const INV_LIST: [InvItem; 31] = [
    InvItem {
        image_number: 12,
        xoff: 10,
        yoff: 0,
        ydelta: 0,
        img_off: 0,
        img_height: 8,
        maxshown: 1,
    }, // 0  Dirk
    InvItem {
        image_number: 9,
        xoff: 10,
        yoff: 10,
        ydelta: 0,
        img_off: 0,
        img_height: 8,
        maxshown: 1,
    }, // 1  Mace
    InvItem {
        image_number: 8,
        xoff: 10,
        yoff: 20,
        ydelta: 0,
        img_off: 0,
        img_height: 8,
        maxshown: 1,
    }, // 2  Sword
    InvItem {
        image_number: 10,
        xoff: 10,
        yoff: 30,
        ydelta: 0,
        img_off: 0,
        img_height: 8,
        maxshown: 1,
    }, // 3  Bow
    InvItem {
        image_number: 17,
        xoff: 10,
        yoff: 40,
        ydelta: 0,
        img_off: 8,
        img_height: 8,
        maxshown: 1,
    }, // 4  Magic Wand
    InvItem {
        image_number: 27,
        xoff: 10,
        yoff: 50,
        ydelta: 0,
        img_off: 0,
        img_height: 8,
        maxshown: 1,
    }, // 5  Golden Lasso
    InvItem {
        image_number: 23,
        xoff: 10,
        yoff: 60,
        ydelta: 0,
        img_off: 8,
        img_height: 8,
        maxshown: 1,
    }, // 6  Sea Shell
    InvItem {
        image_number: 27,
        xoff: 10,
        yoff: 70,
        ydelta: 0,
        img_off: 8,
        img_height: 8,
        maxshown: 1,
    }, // 7  Sun Stone
    InvItem {
        image_number: 3,
        xoff: 30,
        yoff: 0,
        ydelta: 3,
        img_off: 7,
        img_height: 1,
        maxshown: 45,
    }, // 8  Arrows
    InvItem {
        image_number: 18,
        xoff: 50,
        yoff: 0,
        ydelta: 9,
        img_off: 0,
        img_height: 8,
        maxshown: 15,
    }, // 9  Blue Stone
    InvItem {
        image_number: 19,
        xoff: 65,
        yoff: 0,
        ydelta: 6,
        img_off: 0,
        img_height: 5,
        maxshown: 23,
    }, // 10 Green Jewel
    InvItem {
        image_number: 22,
        xoff: 80,
        yoff: 0,
        ydelta: 8,
        img_off: 0,
        img_height: 7,
        maxshown: 17,
    }, // 11 Glass Vial
    InvItem {
        image_number: 21,
        xoff: 95,
        yoff: 0,
        ydelta: 7,
        img_off: 0,
        img_height: 6,
        maxshown: 20,
    }, // 12 Crystal Orb
    InvItem {
        image_number: 23,
        xoff: 110,
        yoff: 0,
        ydelta: 10,
        img_off: 0,
        img_height: 9,
        maxshown: 14,
    }, // 13 Bird Totem
    InvItem {
        image_number: 17,
        xoff: 125,
        yoff: 0,
        ydelta: 6,
        img_off: 0,
        img_height: 5,
        maxshown: 23,
    }, // 14 Gold Ring
    InvItem {
        image_number: 24,
        xoff: 140,
        yoff: 0,
        ydelta: 10,
        img_off: 0,
        img_height: 9,
        maxshown: 14,
    }, // 15 Jade Skull
    InvItem {
        image_number: 25,
        xoff: 160,
        yoff: 0,
        ydelta: 5,
        img_off: 0,
        img_height: 5,
        maxshown: 25,
    }, // 16 Gold Key
    InvItem {
        image_number: 25,
        xoff: 172,
        yoff: 0,
        ydelta: 5,
        img_off: 8,
        img_height: 5,
        maxshown: 25,
    }, // 17 Green Key
    InvItem {
        image_number: 114,
        xoff: 184,
        yoff: 0,
        ydelta: 5,
        img_off: 0,
        img_height: 5,
        maxshown: 25,
    }, // 18 Blue Key
    InvItem {
        image_number: 114,
        xoff: 196,
        yoff: 0,
        ydelta: 5,
        img_off: 8,
        img_height: 5,
        maxshown: 25,
    }, // 19 Red Key
    InvItem {
        image_number: 26,
        xoff: 208,
        yoff: 0,
        ydelta: 5,
        img_off: 0,
        img_height: 5,
        maxshown: 25,
    }, // 20 Grey Key
    InvItem {
        image_number: 26,
        xoff: 220,
        yoff: 0,
        ydelta: 5,
        img_off: 8,
        img_height: 5,
        maxshown: 25,
    }, // 21 White Key
    InvItem {
        image_number: 11,
        xoff: 0,
        yoff: 80,
        ydelta: 0,
        img_off: 8,
        img_height: 8,
        maxshown: 1,
    }, // 22 Talisman
    InvItem {
        image_number: 19,
        xoff: 0,
        yoff: 90,
        ydelta: 0,
        img_off: 8,
        img_height: 8,
        maxshown: 1,
    }, // 23 Rose
    InvItem {
        image_number: 20,
        xoff: 0,
        yoff: 100,
        ydelta: 0,
        img_off: 8,
        img_height: 8,
        maxshown: 1,
    }, // 24 Fruit
    InvItem {
        image_number: 21,
        xoff: 232,
        yoff: 0,
        ydelta: 10,
        img_off: 8,
        img_height: 8,
        maxshown: 5,
    }, // 25 Gold Statue
    InvItem {
        image_number: 22,
        xoff: 0,
        yoff: 110,
        ydelta: 0,
        img_off: 8,
        img_height: 8,
        maxshown: 1,
    }, // 26 Book
    InvItem {
        image_number: 8,
        xoff: 14,
        yoff: 80,
        ydelta: 0,
        img_off: 8,
        img_height: 8,
        maxshown: 1,
    }, // 27 Herb
    InvItem {
        image_number: 9,
        xoff: 14,
        yoff: 90,
        ydelta: 0,
        img_off: 8,
        img_height: 8,
        maxshown: 1,
    }, // 28 Writ
    InvItem {
        image_number: 10,
        xoff: 14,
        yoff: 100,
        ydelta: 0,
        img_off: 8,
        img_height: 8,
        maxshown: 1,
    }, // 29 Bone
    InvItem {
        image_number: 12,
        xoff: 14,
        yoff: 110,
        ydelta: 0,
        img_off: 8,
        img_height: 8,
        maxshown: 1,
    }, // 30 Shard
];

/// Map a short item name or numeric string to an INV_LIST index (0–30).
/// Numeric strings parse to the index directly (0–30 valid; 31+ returns None).
/// Name matching is case-insensitive: user input is checked for table entry containment ("arrows" matches "arrow").
/// Talisman (index 22) is intentionally included — callers guard against it.
pub fn item_name_to_id(name: &str) -> Option<usize> {
    const TABLE: &[(&str, usize)] = &[
        ("dirk", 0),
        ("mace", 1),
        ("sword", 2),
        ("bow", 3),
        ("wand", 4),
        ("lasso", 5),
        ("shell", 6),
        ("sunstone", 7),
        ("sun stone", 7),
        ("arrow", 8),
        ("blue stone", 9),
        ("bluestone", 9),
        ("jewel", 10),
        ("vial", 11),
        ("orb", 12),
        ("totem", 13),
        ("gold ring", 14),
        ("gold key", 16),
        ("green key", 17),
        ("blue key", 18),
        ("red key", 19),
        ("grey key", 20),
        ("gray key", 20),
        ("white key", 21),
        ("ring", 14), // after "gold ring" so specific match wins
        ("key", 16),  // generic key → gold key (first key)
        ("talisman", 22),
        ("rose", 23),
        ("fruit", 24),
        ("statue", 25),
        ("book", 26),
        ("herb", 27),
        ("writ", 28),
        ("bone", 29),
        ("shard", 30),
    ];

    let lower = name.to_ascii_lowercase();
    if lower.is_empty() {
        return None;
    }

    // Numeric index
    if let Ok(n) = lower.parse::<usize>() {
        return if n < INV_LIST.len() { Some(n) } else { None };
    }

    // Exact match first, then substring
    TABLE
        .iter()
        .find(|(entry, _)| *entry == lower.as_str())
        .or_else(|| TABLE.iter().find(|(entry, _)| lower.contains(*entry)))
        .map(|(_, id)| *id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_name_to_id_exact() {
        assert_eq!(item_name_to_id("sword"), Some(2));
        assert_eq!(item_name_to_id("bow"), Some(3));
        assert_eq!(item_name_to_id("talisman"), Some(22));
        assert_eq!(item_name_to_id("shard"), Some(30));
    }

    #[test]
    fn test_item_name_to_id_numeric() {
        assert_eq!(item_name_to_id("0"), Some(0));
        assert_eq!(item_name_to_id("22"), Some(22));
        assert_eq!(item_name_to_id("30"), Some(30));
        assert_eq!(item_name_to_id("31"), None); // out of range
    }

    #[test]
    fn test_item_name_to_id_aliases() {
        assert_eq!(item_name_to_id("wand"), Some(4));
        assert_eq!(item_name_to_id("lasso"), Some(5));
        assert_eq!(item_name_to_id("shell"), Some(6));
        assert_eq!(item_name_to_id("arrows"), Some(8));
        assert_eq!(item_name_to_id("key"), Some(16)); // first key match
    }

    #[test]
    fn test_item_name_to_id_unknown() {
        assert_eq!(item_name_to_id("fireball"), None);
        assert_eq!(item_name_to_id("orc"), None);
        assert_eq!(item_name_to_id(""), None);
    }
}

/// A loaded sprite sheet: palette-index pixel data.
pub struct SpriteSheet {
    pub cfile_idx: u8,
    /// Palette index per pixel, row-major, num_frames * frame_h * SPRITE_W bytes.
    /// Index 31 = transparent (Amiga "all planes set" convention).
    pub pixels: Vec<u8>,
    pub num_frames: usize,
    /// Height of each frame in pixels (SPRITE_H for characters, OBJ_SPRITE_H for objects).
    pub frame_h: usize,
}

impl SpriteSheet {
    /// Decode a sprite sheet from ADF block data.
    ///
    /// On-disk layout (fmain2.c read_shapes / fsubs.asm make_mask):
    ///
    ///   Shape section: frame_count frames × frame_bytes bytes each.
    ///   Within each frame, planes are PLANE-MAJOR (not row-interleaved):
    ///     Frame F: [plane0][plane1][plane2][plane3][plane4]
    ///   → plane P row R of frame F = data[F*frame_bytes + P*(frame_h*PLANE_ROW_BYTES) + R*PLANE_ROW_BYTES]
    ///
    ///   The mask is NOT stored on disk. It is computed by make_mask() in fsubs.asm:
    ///     mask_bit = NOT(plane0 AND plane1 AND plane2 AND plane3 AND plane4)
    ///   Color index 31 (all planes set) is transparent; all other indices are opaque.
    ///   ("assumes color 31 = transparent" — fsubs.asm:1617)
    ///
    /// `frame_count` must be the cfiles[].count value (not derived from data.len()).
    /// `frame_h` is the sprite height in pixels (SPRITE_H for characters, OBJ_SPRITE_H for objects).
    pub fn decode(cfile_idx: u8, data: &[u8], frame_count: usize, frame_h: usize) -> Self {
        let plane_frame_bytes = frame_h * PLANE_ROW_BYTES;
        let frame_bytes = SPRITE_PLANES * plane_frame_bytes;
        // Initialize to 31 (transparent). Opaque pixels will overwrite.
        let mut pixels = vec![31u8; frame_count * frame_h * SPRITE_W];

        for frame in 0..frame_count {
            let frame_base = frame * frame_bytes;
            if frame_base + frame_bytes > data.len() {
                break; // shape data truncated
            }

            for row in 0..frame_h {
                let row_off = row * PLANE_ROW_BYTES;

                let mut planes = [0u16; SPRITE_PLANES];
                for p in 0..SPRITE_PLANES {
                    let pb = &data[frame_base + p * plane_frame_bytes + row_off..];
                    planes[p] = u16::from_be_bytes([pb[0], pb[1]]);
                }

                for col in 0..SPRITE_W {
                    let bit = 15 - col;
                    let color_idx = (0..SPRITE_PLANES)
                        .map(|p| ((planes[p] >> bit) & 1) << p)
                        .fold(0usize, |acc, b| acc | b as usize);
                    let pixel_idx = frame * frame_h * SPRITE_W + row * SPRITE_W + col;
                    pixels[pixel_idx] = color_idx as u8;
                    // index 31 stays as initialized (transparent)
                }
            }
        }
        SpriteSheet {
            cfile_idx,
            pixels,
            num_frames: frame_count,
            frame_h,
        }
    }

    /// Load and decode a character/enemy sprite sheet from the ADF for a given cfile index.
    /// Returns None if the ADF doesn't have enough blocks.
    pub fn load(adf: &AdfDisk, cfile_idx: u8) -> Option<Self> {
        let block = CFILE_BLOCKS[cfile_idx as usize];
        let num_blocks = CFILE_BLOCK_COUNTS[cfile_idx as usize];
        let frame_count = CFILE_FRAME_COUNTS[cfile_idx as usize];
        if block as usize + num_blocks as usize > adf.num_blocks() {
            return None;
        }
        let data = adf.load_blocks(block, num_blocks);
        Some(Self::decode(cfile_idx, data, frame_count, SPRITE_H))
    }

    /// Load and decode the objects sprite sheet (cfile 3, height=16 not 32).
    /// Returns None if the ADF doesn't have enough blocks.
    pub fn load_objects(adf: &AdfDisk) -> Option<Self> {
        const CFILE_IDX: u8 = 3;
        let block = CFILE_BLOCKS[CFILE_IDX as usize];
        let num_blocks = CFILE_BLOCK_COUNTS[CFILE_IDX as usize];
        let frame_count = CFILE_FRAME_COUNTS[CFILE_IDX as usize];
        if block as usize + num_blocks as usize > adf.num_blocks() {
            return None;
        }
        let data = adf.load_blocks(block, num_blocks);
        Some(Self::decode(CFILE_IDX, data, frame_count, OBJ_SPRITE_H))
    }

    /// Return the palette-index slice for a single frame (frame_h * SPRITE_W bytes).
    /// Returns None for out-of-range frame indices.
    pub fn frame_pixels(&self, frame: usize) -> Option<&[u8]> {
        if frame >= self.num_frames {
            return None;
        }
        let frame_pixels = self.frame_h * SPRITE_W;
        let start = frame * frame_pixels;
        Some(&self.pixels[start..start + frame_pixels])
    }
}

/// One entry of statelist[87]: per-animation-index weapon sprite offsets.
/// Ported verbatim from original/fmain.c statelist[].
#[derive(Debug, Clone, Copy)]
pub struct StatEntry {
    pub figure: u8,
    pub wpn_no: u8,
    pub wpn_x: i8,
    pub wpn_y: i8,
}

/// STATELIST — 87 entries ported from fmain.c statelist[].
/// Index = animation state index.
pub const STATELIST: [StatEntry; 87] = [
    // 0–7: southwalk
    StatEntry {
        figure: 0,
        wpn_no: 11,
        wpn_x: -2,
        wpn_y: 11,
    },
    StatEntry {
        figure: 1,
        wpn_no: 11,
        wpn_x: -3,
        wpn_y: 11,
    },
    StatEntry {
        figure: 2,
        wpn_no: 11,
        wpn_x: -3,
        wpn_y: 10,
    },
    StatEntry {
        figure: 3,
        wpn_no: 11,
        wpn_x: -3,
        wpn_y: 9,
    },
    StatEntry {
        figure: 4,
        wpn_no: 11,
        wpn_x: -3,
        wpn_y: 10,
    },
    StatEntry {
        figure: 5,
        wpn_no: 11,
        wpn_x: -3,
        wpn_y: 11,
    },
    StatEntry {
        figure: 6,
        wpn_no: 11,
        wpn_x: -2,
        wpn_y: 11,
    },
    StatEntry {
        figure: 7,
        wpn_no: 11,
        wpn_x: -1,
        wpn_y: 11,
    },
    // 8–15: westwalk
    StatEntry {
        figure: 8,
        wpn_no: 9,
        wpn_x: -12,
        wpn_y: 11,
    },
    StatEntry {
        figure: 9,
        wpn_no: 9,
        wpn_x: -11,
        wpn_y: 12,
    },
    StatEntry {
        figure: 10,
        wpn_no: 9,
        wpn_x: -8,
        wpn_y: 13,
    },
    StatEntry {
        figure: 11,
        wpn_no: 9,
        wpn_x: -4,
        wpn_y: 13,
    },
    StatEntry {
        figure: 12,
        wpn_no: 9,
        wpn_x: 0,
        wpn_y: 13,
    },
    StatEntry {
        figure: 13,
        wpn_no: 9,
        wpn_x: -4,
        wpn_y: 13,
    },
    StatEntry {
        figure: 14,
        wpn_no: 9,
        wpn_x: -8,
        wpn_y: 13,
    },
    StatEntry {
        figure: 15,
        wpn_no: 9,
        wpn_x: -11,
        wpn_y: 12,
    },
    // 16–23: northwalk
    StatEntry {
        figure: 16,
        wpn_no: 14,
        wpn_x: -1,
        wpn_y: 1,
    },
    StatEntry {
        figure: 17,
        wpn_no: 14,
        wpn_x: -1,
        wpn_y: 2,
    },
    StatEntry {
        figure: 18,
        wpn_no: 14,
        wpn_x: -1,
        wpn_y: 3,
    },
    StatEntry {
        figure: 19,
        wpn_no: 14,
        wpn_x: -1,
        wpn_y: 4,
    },
    StatEntry {
        figure: 20,
        wpn_no: 14,
        wpn_x: -1,
        wpn_y: 3,
    },
    StatEntry {
        figure: 21,
        wpn_no: 14,
        wpn_x: -1,
        wpn_y: 2,
    },
    StatEntry {
        figure: 22,
        wpn_no: 14,
        wpn_x: -1,
        wpn_y: 1,
    },
    StatEntry {
        figure: 23,
        wpn_no: 14,
        wpn_x: -1,
        wpn_y: 1,
    },
    // 24–31: eastwalk
    StatEntry {
        figure: 24,
        wpn_no: 10,
        wpn_x: 5,
        wpn_y: 12,
    },
    StatEntry {
        figure: 25,
        wpn_no: 10,
        wpn_x: 3,
        wpn_y: 12,
    },
    StatEntry {
        figure: 26,
        wpn_no: 10,
        wpn_x: 2,
        wpn_y: 12,
    },
    StatEntry {
        figure: 27,
        wpn_no: 10,
        wpn_x: 3,
        wpn_y: 12,
    },
    StatEntry {
        figure: 28,
        wpn_no: 10,
        wpn_x: 5,
        wpn_y: 12,
    },
    StatEntry {
        figure: 29,
        wpn_no: 10,
        wpn_x: 6,
        wpn_y: 12,
    },
    StatEntry {
        figure: 30,
        wpn_no: 10,
        wpn_x: 6,
        wpn_y: 11,
    },
    StatEntry {
        figure: 31,
        wpn_no: 10,
        wpn_x: 6,
        wpn_y: 12,
    },
    // 32–43: south fight
    StatEntry {
        figure: 32,
        wpn_no: 11,
        wpn_x: -2,
        wpn_y: 12,
    },
    StatEntry {
        figure: 32,
        wpn_no: 10,
        wpn_x: 0,
        wpn_y: 12,
    },
    StatEntry {
        figure: 33,
        wpn_no: 0,
        wpn_x: 2,
        wpn_y: 10,
    },
    StatEntry {
        figure: 34,
        wpn_no: 1,
        wpn_x: 4,
        wpn_y: 6,
    },
    StatEntry {
        figure: 34,
        wpn_no: 2,
        wpn_x: 1,
        wpn_y: 4,
    },
    StatEntry {
        figure: 34,
        wpn_no: 3,
        wpn_x: 0,
        wpn_y: 4,
    },
    StatEntry {
        figure: 36,
        wpn_no: 4,
        wpn_x: -5,
        wpn_y: 0,
    },
    StatEntry {
        figure: 36,
        wpn_no: 5,
        wpn_x: -10,
        wpn_y: 1,
    },
    StatEntry {
        figure: 35,
        wpn_no: 12,
        wpn_x: -5,
        wpn_y: 5,
    },
    StatEntry {
        figure: 36,
        wpn_no: 0,
        wpn_x: 0,
        wpn_y: 6,
    },
    StatEntry {
        figure: 38,
        wpn_no: 85,
        wpn_x: -6,
        wpn_y: 5,
    },
    StatEntry {
        figure: 37,
        wpn_no: 81,
        wpn_x: -6,
        wpn_y: 5,
    },
    // 44–55: west fight
    StatEntry {
        figure: 40,
        wpn_no: 9,
        wpn_x: -7,
        wpn_y: 12,
    },
    StatEntry {
        figure: 40,
        wpn_no: 8,
        wpn_x: -9,
        wpn_y: 9,
    },
    StatEntry {
        figure: 41,
        wpn_no: 7,
        wpn_x: -10,
        wpn_y: 5,
    },
    StatEntry {
        figure: 42,
        wpn_no: 7,
        wpn_x: -12,
        wpn_y: 4,
    },
    StatEntry {
        figure: 42,
        wpn_no: 6,
        wpn_x: -12,
        wpn_y: 3,
    },
    StatEntry {
        figure: 42,
        wpn_no: 5,
        wpn_x: -12,
        wpn_y: 3,
    },
    StatEntry {
        figure: 44,
        wpn_no: 5,
        wpn_x: -8,
        wpn_y: 3,
    },
    StatEntry {
        figure: 44,
        wpn_no: 14,
        wpn_x: -7,
        wpn_y: 6,
    },
    StatEntry {
        figure: 43,
        wpn_no: 13,
        wpn_x: -7,
        wpn_y: 8,
    },
    StatEntry {
        figure: 42,
        wpn_no: 5,
        wpn_x: -12,
        wpn_y: 3,
    },
    StatEntry {
        figure: 46,
        wpn_no: 86,
        wpn_x: -3,
        wpn_y: 0,
    },
    StatEntry {
        figure: 45,
        wpn_no: 82,
        wpn_x: -3,
        wpn_y: 0,
    },
    // 56–67: north fight
    StatEntry {
        figure: 48,
        wpn_no: 14,
        wpn_x: -3,
        wpn_y: 0,
    },
    StatEntry {
        figure: 48,
        wpn_no: 6,
        wpn_x: -3,
        wpn_y: -1,
    },
    StatEntry {
        figure: 49,
        wpn_no: 5,
        wpn_x: -2,
        wpn_y: -3,
    },
    StatEntry {
        figure: 50,
        wpn_no: 5,
        wpn_x: -3,
        wpn_y: -4,
    },
    StatEntry {
        figure: 50,
        wpn_no: 4,
        wpn_x: 0,
        wpn_y: 0,
    },
    StatEntry {
        figure: 50,
        wpn_no: 3,
        wpn_x: 3,
        wpn_y: 0,
    },
    StatEntry {
        figure: 52,
        wpn_no: 4,
        wpn_x: 6,
        wpn_y: 1,
    },
    StatEntry {
        figure: 52,
        wpn_no: 15,
        wpn_x: 7,
        wpn_y: 3,
    },
    StatEntry {
        figure: 51,
        wpn_no: 14,
        wpn_x: 1,
        wpn_y: 6,
    },
    StatEntry {
        figure: 50,
        wpn_no: 4,
        wpn_x: 0,
        wpn_y: 0,
    },
    StatEntry {
        figure: 54,
        wpn_no: 87,
        wpn_x: 3,
        wpn_y: 0,
    },
    StatEntry {
        figure: 53,
        wpn_no: 83,
        wpn_x: 3,
        wpn_y: 0,
    },
    // 68–79: east fight
    StatEntry {
        figure: 56,
        wpn_no: 10,
        wpn_x: 5,
        wpn_y: 11,
    },
    StatEntry {
        figure: 56,
        wpn_no: 0,
        wpn_x: 6,
        wpn_y: 9,
    },
    StatEntry {
        figure: 57,
        wpn_no: 1,
        wpn_x: 10,
        wpn_y: 6,
    },
    StatEntry {
        figure: 58,
        wpn_no: 1,
        wpn_x: 10,
        wpn_y: 5,
    },
    StatEntry {
        figure: 58,
        wpn_no: 2,
        wpn_x: 7,
        wpn_y: 3,
    },
    StatEntry {
        figure: 58,
        wpn_no: 3,
        wpn_x: 6,
        wpn_y: 3,
    },
    StatEntry {
        figure: 60,
        wpn_no: 4,
        wpn_x: 1,
        wpn_y: 0,
    },
    StatEntry {
        figure: 60,
        wpn_no: 3,
        wpn_x: 3,
        wpn_y: 2,
    },
    StatEntry {
        figure: 59,
        wpn_no: 15,
        wpn_x: 4,
        wpn_y: 1,
    },
    StatEntry {
        figure: 58,
        wpn_no: 4,
        wpn_x: 5,
        wpn_y: 1,
    },
    StatEntry {
        figure: 62,
        wpn_no: 84,
        wpn_x: 3,
        wpn_y: 0,
    },
    StatEntry {
        figure: 61,
        wpn_no: 80,
        wpn_x: 3,
        wpn_y: 0,
    },
    // 80–82: death sequence
    StatEntry {
        figure: 47,
        wpn_no: 0,
        wpn_x: 5,
        wpn_y: 11,
    },
    StatEntry {
        figure: 63,
        wpn_no: 0,
        wpn_x: 6,
        wpn_y: 9,
    },
    StatEntry {
        figure: 39,
        wpn_no: 0,
        wpn_x: 6,
        wpn_y: 9,
    },
    // 83: sinking sequence
    StatEntry {
        figure: 55,
        wpn_no: 10,
        wpn_x: 5,
        wpn_y: 11,
    },
    // 84–85: oscillations
    StatEntry {
        figure: 64,
        wpn_no: 10,
        wpn_x: 5,
        wpn_y: 11,
    },
    StatEntry {
        figure: 65,
        wpn_no: 10,
        wpn_x: 5,
        wpn_y: 11,
    },
    // 86: asleep
    StatEntry {
        figure: 66,
        wpn_no: 10,
        wpn_x: 5,
        wpn_y: 11,
    },
];

// ---------------------------------------------------------------------------
// Bow-overlay offset tables — fmain2.c:877-882, indexed by the body-pass
// `inum` (= 0..31, the actor's walk-cycle frame). Used only on the bow
// weapon-overlay pass when `inum < 32` (`fmain.c:2422-2423`).
// See reference/logic/sprite-rendering.md "select_atype_inum" and "bow_x[32]".
// ---------------------------------------------------------------------------

/// `bow_x[32]` — per-frame X pixel offset added to `xstart` on the bow
/// weapon-overlay pass when `inum < 32`.
pub const BOW_X: [i8; 32] = [
    1, 2, 3, 4, 3, 2, 1, 0, //  0..7   south-walk
    3, 2, 0, -2, -3, -2, 0, 2, //  8..15  west-walk
    -3, -3, -3, -3, -3, -3, -3, -2, // 16..23  north-walk
    0, 1, 1, 1, 0, -2, -3, -2, // 24..31  east-walk
];

/// `bow_y[32]` — per-frame Y pixel offset added to `ystart` on the bow
/// weapon-overlay pass when `inum < 32`.
pub const BOW_Y: [i8; 32] = [
    8, 8, 8, 7, 8, 8, 8, 8, //  0..7   south-walk
    11, 12, 13, 13, 13, 13, 13, 12, //  8..15  west-walk
    8, 7, 6, 5, 6, 7, 8, 9, // 16..23  north-walk
    12, 12, 12, 12, 12, 12, 11, 12, // 24..31  east-walk
];

// ---------------------------------------------------------------------------
// Per-frame OBJECTS half-height set — fmain.c:2477-2479.
// The OBJECTS sheet stores all frames as 16-scanline rows, but a fixed list
// of `inum` values render as 8-scanline strips packed two-per-row. The high
// bit (`inum & 0x80`) also forces 8-scanline height *and* shifts the source
// data Y-offset by +8 (bit-7 dual role, fmain.c:2479,2524). The bit must be
// stripped from the effective `inum` before addressing the sheet.
// See reference/logic/sprite-rendering.md "compute_sprite_size".
// ---------------------------------------------------------------------------

/// Half-height bit-7 flag for OBJECTS frames (`fmain.c:2479,2524`).
pub const INUM_BIT7_HALF_HEIGHT: u8 = 0x80;

/// Return the effective scanline height of an OBJECTS-sheet frame.
///
/// 8 scanlines for the half-height set (`{0x1b, 8..=12, 25, 26, 0x11..=0x17}`)
/// or any frame with bit 7 set; 16 scanlines otherwise. Mirrors
/// `compute_sprite_size` (`fmain.c:2477-2479`).
pub const fn obj_frame_height(inum: u8) -> u8 {
    if inum & INUM_BIT7_HALF_HEIGHT != 0 {
        return 8;
    }
    let bare = inum;
    match bare {
        0x1b => 8,
        8..=12 => 8,
        25 | 26 => 8,
        // 0x11..=0x17  (note: spec says `inum > 0x10 && inum < 0x18`)
        0x11..=0x17 => 8,
        _ => 16,
    }
}

/// Return the source-data Y-offset (in scanlines) into the addressed
/// OBJECTS frame. For bit-7 frames this is +8 (the lower-half row); for
/// all other frames it is 0. Mirrors `compute_shape_clip` (`fmain.c:2524`).
pub const fn obj_frame_y_offset(inum: u8) -> u8 {
    if inum & INUM_BIT7_HALF_HEIGHT != 0 {
        8
    } else {
        0
    }
}

/// Strip the bit-7 half-height flag from an `inum`, returning the
/// effective frame index used to address the OBJECTS sheet.
pub const fn obj_frame_index(inum: u8) -> u8 {
    inum & !INUM_BIT7_HALF_HEIGHT
}

#[cfg(test)]
mod sprite_metadata_tests {
    use super::*;

    #[test]
    fn bow_tables_are_32_entries() {
        assert_eq!(BOW_X.len(), 32);
        assert_eq!(BOW_Y.len(), 32);
    }

    #[test]
    fn obj_frame_height_full_height_default() {
        // A representative frame outside the half-height set.
        assert_eq!(obj_frame_height(0), 16);
        assert_eq!(obj_frame_height(7), 16);
        assert_eq!(obj_frame_height(13), 16);
        assert_eq!(obj_frame_height(24), 16);
        assert_eq!(obj_frame_height(0x1c), 16);
        assert_eq!(obj_frame_height(0x18), 16);
    }

    #[test]
    fn obj_frame_height_half_height_inum_list() {
        // {0x1b, 8..=12, 25, 26, 0x11..=0x17}
        assert_eq!(obj_frame_height(0x1b), 8);
        for f in 8u8..=12 {
            assert_eq!(obj_frame_height(f), 8, "inum={}", f);
        }
        assert_eq!(obj_frame_height(25), 8);
        assert_eq!(obj_frame_height(26), 8);
        for f in 0x11u8..=0x17 {
            assert_eq!(obj_frame_height(f), 8, "inum={:#x}", f);
        }
    }

    #[test]
    fn obj_frame_height_bit7_forces_half_height() {
        assert_eq!(obj_frame_height(0x80), 8);
        assert_eq!(obj_frame_height(0x80 | 30), 8);
        assert_eq!(obj_frame_height(0x80 | 0x53), 8);
    }

    #[test]
    fn obj_frame_y_offset_only_when_bit7_set() {
        assert_eq!(obj_frame_y_offset(0), 0);
        assert_eq!(obj_frame_y_offset(0x1b), 0);
        assert_eq!(obj_frame_y_offset(0x80), 8);
        assert_eq!(obj_frame_y_offset(0xff), 8);
    }

    #[test]
    fn obj_frame_index_strips_bit7() {
        assert_eq!(obj_frame_index(0x80 | 30), 30);
        assert_eq!(obj_frame_index(0x53), 0x53);
    }
}
