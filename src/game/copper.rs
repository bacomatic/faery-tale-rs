//! Copper list simulation for per-scanline palette effects.
//!
//! On the Amiga, the copper coprocessor modifies COLOR registers each scanline
//! to create gradients (sky) and shimmer effects (water). We simulate this
//! in software by adjusting palette entries per scanline group.

/// A simulated copper instruction: at `scanline`, change color register
/// `color_reg` to `color` (12-bit Amiga color: 0x0RGB).
#[derive(Debug, Clone, Copy)]
pub struct CopperInstruction {
    pub scanline: u16,
    pub color_reg: u8,
    pub color: u16,
}

/// A copper list is a sequence of instructions applied top-to-bottom.
pub struct CopperList {
    instructions: Vec<CopperInstruction>,
}

impl CopperList {
    pub fn new() -> Self {
        CopperList { instructions: Vec::new() }
    }

    pub fn add(&mut self, scanline: u16, color_reg: u8, color: u16) {
        self.instructions.push(CopperInstruction { scanline, color_reg, color });
    }

    /// Returns instructions that apply at or before `scanline`, sorted by scanline.
    pub fn instructions_up_to(&self, scanline: u16) -> impl Iterator<Item = &CopperInstruction> {
        self.instructions.iter().filter(move |i| i.scanline <= scanline)
    }

    /// Parse a raw copper list from bytes.
    /// Format: pairs of u16 big-endian: (WAIT_or_MOVE, data)
    /// WAIT: high bit of first word is 0, encodes scanline in bits 8-15
    /// MOVE: high bit is 1, encodes register offset in bits 1-8
    /// This is a simplified parser for palette MOVE instructions only.
    pub fn parse(data: &[u8]) -> Self {
        let mut list = CopperList::new();
        let mut current_scanline = 0u16;
        let mut i = 0;
        while i + 3 < data.len() {
            let word1 = u16::from_be_bytes([data[i], data[i+1]]);
            let word2 = u16::from_be_bytes([data[i+2], data[i+3]]);
            i += 4;
            if word1 == 0xFFFF && word2 == 0xFFFE {
                break; // End of copper list
            }
            let is_wait = (word1 & 0x0001) == 0;
            if is_wait {
                // WAIT instruction: bits 8-15 = vertical position
                current_scanline = (word1 >> 8) & 0xFF;
            } else {
                // MOVE instruction: register offset in word1, value in word2
                // Color registers: 0x180-0x1BE (COLOR00-COLOR31), offset by 0x180
                let reg_offset = word1 & 0x01FE;
                if reg_offset >= 0x180 && reg_offset <= 0x1BE {
                    let color_reg = ((reg_offset - 0x180) / 2) as u8;
                    list.add(current_scanline, color_reg, word2 & 0x0FFF);
                }
            }
        }
        list
    }
}

impl Default for CopperList {
    fn default() -> Self { Self::new() }
}
