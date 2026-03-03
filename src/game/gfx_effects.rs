//! Full-screen graphical effects: witch (scanline warp) and teleport (flash/fade).

/// State for the witch screen-warp effect.
/// Applies sinusoidal horizontal displacement to scanlines of the play texture.
pub struct WitchEffect {
    pub active: bool,
    frame: u32,
    /// Duration in frames (original: runs while witchflag is set)
    pub amplitude_px: f32,
}

impl WitchEffect {
    pub fn new() -> Self {
        WitchEffect { active: false, frame: 0, amplitude_px: 6.0 }
    }

    pub fn start(&mut self) { self.active = true; self.frame = 0; }
    pub fn stop(&mut self)  { self.active = false; }

    /// Advance one frame and return per-scanline x-offsets (pixels) for 200 scanlines.
    /// Caller applies these offsets when blitting the play texture to canvas.
    pub fn tick(&mut self) -> Option<Vec<i32>> {
        if !self.active { return None; }
        let offsets = (0..200).map(|y| {
            let phase = (self.frame as f32 * 0.15) + (y as f32 * 0.12);
            (phase.sin() * self.amplitude_px) as i32
        }).collect();
        self.frame += 1;
        Some(offsets)
    }
}

impl Default for WitchEffect { fn default() -> Self { Self::new() } }

/// State for the teleport flash-and-fade effect.
/// Frame 0-4: flash white; frame 5-34: fade from black.
pub struct TeleportEffect {
    pub active: bool,
    frame: u32,
}

const TELEPORT_FLASH_FRAMES: u32 = 5;
const TELEPORT_FADE_FRAMES: u32 = 30;
const TELEPORT_TOTAL_FRAMES: u32 = TELEPORT_FLASH_FRAMES + TELEPORT_FADE_FRAMES;

impl TeleportEffect {
    pub fn new() -> Self { TeleportEffect { active: false, frame: 0 } }

    pub fn start(&mut self) { self.active = true; self.frame = 0; }

    /// Returns (overlay_color, overlay_alpha) for this frame.
    /// Caller should draw a filled rect over the play texture with this color+alpha.
    /// Returns None when the effect is complete.
    pub fn tick(&mut self) -> Option<(u8, u8, u8, u8)> {
        if !self.active { return None; }
        let result = if self.frame < TELEPORT_FLASH_FRAMES {
            Some((255u8, 255u8, 255u8, 220u8)) // white flash
        } else if self.frame < TELEPORT_TOTAL_FRAMES {
            let fade_progress = (self.frame - TELEPORT_FLASH_FRAMES) as f32 / TELEPORT_FADE_FRAMES as f32;
            let alpha = ((1.0 - fade_progress) * 255.0) as u8;
            Some((0u8, 0u8, 0u8, alpha)) // fade from black
        } else {
            self.active = false;
            None
        };
        self.frame += 1;
        result
    }
}

impl Default for TeleportEffect { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_witch_effect_offsets() {
        let mut w = WitchEffect::new();
        w.start();
        let offsets = w.tick().expect("should produce offsets");
        assert_eq!(offsets.len(), 200);
    }

    #[test]
    fn test_teleport_effect_phases() {
        let mut t = TeleportEffect::new();
        t.start();
        // First 5 frames: white flash
        for _ in 0..5 {
            let (r, g, b, _) = t.tick().expect("active");
            assert_eq!((r, g, b), (255, 255, 255));
        }
        // Frames 5-34: black fade
        for _ in 0..30 {
            let (r, g, b, _) = t.tick().expect("active");
            assert_eq!((r, g, b), (0, 0, 0));
        }
        // After 35 frames: done
        assert!(t.tick().is_none());
    }
}
