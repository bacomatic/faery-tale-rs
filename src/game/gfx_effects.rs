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

/// State for the teleport colorplay effect.
///
/// Ports `colorplay()` from fmain2.c:425-431: a 32-frame palette storm where
/// palette entries 1..31 are set to random 12-bit RGB4 values each frame via
/// `bitrand(0xfff)`.  Callers apply the returned colors directly to
/// `current_palette[1..32]` using `amiga_color_to_rgba`.
pub struct TeleportEffect {
    pub active: bool,
    frame: u32,
}

const COLORPLAY_FRAMES: u32 = 32;

impl TeleportEffect {
    pub fn new() -> Self { TeleportEffect { active: false, frame: 0 } }

    pub fn start(&mut self) { self.active = true; self.frame = 0; }

    /// Returns 31 random 12-bit RGB4 values (palette slots 1..31) for this frame.
    /// Returns None when the 32-frame effect is complete.
    pub fn tick(&mut self) -> Option<[u16; 31]> {
        if !self.active { return None; }
        if self.frame >= COLORPLAY_FRAMES {
            self.active = false;
            return None;
        }
        let mut colors = [0u16; 31];
        for c in colors.iter_mut() {
            *c = (crate::game::combat::bitrand(0xfff)) as u16;
        }
        self.frame += 1;
        Some(colors)
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
    fn test_teleport_effect_colorplay() {
        let mut t = TeleportEffect::new();
        t.start();
        // 32 frames of random palette values (12-bit each, ≤ 0x0fff).
        for _ in 0..32 {
            let colors = t.tick().expect("active during colorplay");
            assert_eq!(colors.len(), 31);
            for &c in &colors {
                assert!(c <= 0x0fff, "palette value {c:#06x} exceeds 12 bits");
            }
        }
        // After 32 frames: effect ends.
        assert!(t.tick().is_none());
        assert!(!t.active);
    }
}
