//! Region palette loading and smooth transition support.

/// Number of palette entries (Amiga OCS has 32 colors, 4 bitplanes).
pub const PALETTE_SIZE: usize = 32;
/// RGBA32 black.
pub const BLACK: u32 = 0xFF000000;

/// A 32-color RGBA32 palette.
pub type Palette = [u32; PALETTE_SIZE];

/// Convert Amiga 12-bit RGB (R4G4B4) to RGBA32 (0xRRGGBBAA).
/// Input format: 0x0RGB where each nibble is 4 bits.
pub fn amiga_color_to_rgba(color12: u16) -> u32 {
    let r = ((color12 >> 8) & 0xF) as u8;
    let g = ((color12 >> 4) & 0xF) as u8;
    let b = (color12 & 0xF) as u8;
    // Expand 4-bit to 8-bit by duplicating the nibble
    let r8 = (r << 4) | r;
    let g8 = (g << 4) | g;
    let b8 = (b << 4) | b;
    0xFF000000 | ((r8 as u32) << 16) | ((g8 as u32) << 8) | (b8 as u32)
}

/// Palette transition state for smooth fades between regions.
pub struct PaletteTransition {
    /// Source palette (where we're fading from).
    pub from: Palette,
    /// Target palette (where we're fading to).
    pub to: Palette,
    /// Progress: 0 = at from, STEPS = at to.
    pub step: u8,
    /// Total transition steps (original: 8 frames).
    pub steps: u8,
}

impl PaletteTransition {
    pub const STEPS: u8 = 8;

    pub fn new(from: Palette, to: Palette) -> Self {
        PaletteTransition {
            from,
            to,
            step: 0,
            steps: Self::STEPS,
        }
    }

    /// Returns true if the transition is complete.
    pub fn is_done(&self) -> bool {
        self.step >= self.steps
    }

    /// Advance one frame; returns the interpolated palette for this frame.
    pub fn tick(&mut self) -> Palette {
        if self.step >= self.steps {
            return self.to;
        }
        self.step += 1;
        let t = self.step as u32;
        let total = self.steps as u32;
        let mut out = [0u32; PALETTE_SIZE];
        for i in 0..PALETTE_SIZE {
            let fr = self.from[i];
            let to = self.to[i];
            let r = (((fr >> 16 & 0xFF) * (total - t) + (to >> 16 & 0xFF) * t) / total) as u8;
            let g = (((fr >> 8 & 0xFF) * (total - t) + (to >> 8 & 0xFF) * t) / total) as u8;
            let b = (((fr & 0xFF) * (total - t) + (to & 0xFF) * t) / total) as u8;
            out[i] = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amiga_color_conversion() {
        // White: 0xFFF -> RGB (FF,FF,FF)
        assert_eq!(amiga_color_to_rgba(0xFFF), 0xFFFFFFFF);
        // Black: 0x000 -> RGB (00,00,00)
        assert_eq!(amiga_color_to_rgba(0x000), 0xFF000000);
    }

    #[test]
    fn test_transition_completes_in_steps() {
        let from = [0xFF000000u32; PALETTE_SIZE];
        let to = [0xFFFFFFFF_u32; PALETTE_SIZE];
        let mut tr = PaletteTransition::new(from, to);
        for _ in 0..PaletteTransition::STEPS {
            let _ = tr.tick();
        }
        assert!(tr.is_done());
    }

    #[test]
    fn test_final_palette_matches_target() {
        let from = [0xFF000000u32; PALETTE_SIZE];
        let to = [0xFFFFFFFF_u32; PALETTE_SIZE];
        let mut tr = PaletteTransition::new(from, to);
        let mut last = from;
        for _ in 0..PaletteTransition::STEPS {
            last = tr.tick();
        }
        assert_eq!(last, to);
    }
}
