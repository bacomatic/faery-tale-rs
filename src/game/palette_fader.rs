
use crate::game::colors::{Palette, RGB4};

/**
 * Interpolates between two palettes over a given duration in ticks (1/60s).
 *
 * The original game fades in 20 steps with Delay(1) each (~0.4s total at 50Hz).
 * At 60 ticks/second, 24 ticks gives a similar ~0.4s fade.
 */
pub struct PaletteFader {
    from: Vec<RGB4>,
    to: Vec<RGB4>,
    elapsed_ticks: u32,
    total_ticks: u32,
}

impl PaletteFader {
    /// Create a new fader that interpolates from `from` to `to` over `duration_ticks`.
    pub fn new(from: &Palette, to: &Palette, duration_ticks: u32) -> PaletteFader {
        PaletteFader {
            from: from.colors.clone(),
            to: to.colors.clone(),
            elapsed_ticks: 0,
            total_ticks: duration_ticks,
        }
    }

    /// Advance the fader by `delta` ticks. Returns the interpolated palette.
    pub fn tick(&mut self, delta: u32) -> Palette {
        self.elapsed_ticks = (self.elapsed_ticks + delta).min(self.total_ticks);
        self.current_palette()
    }

    /// Get the current interpolated palette without advancing time.
    pub fn current_palette(&self) -> Palette {
        let t = if self.total_ticks == 0 {
            1.0f32
        } else {
            self.elapsed_ticks as f32 / self.total_ticks as f32
        };

        let len = self.from.len().max(self.to.len());
        let mut colors = Vec::with_capacity(len);

        for i in 0..len {
            let from_c = self.from.get(i).copied().unwrap_or(RGB4 { color: 0 });
            let to_c = self.to.get(i).copied().unwrap_or(RGB4 { color: 0 });
            colors.push(lerp_rgb4(&from_c, &to_c, t));
        }

        Palette { colors }
    }

    /// Returns true when the fade is complete.
    pub fn is_done(&self) -> bool {
        self.elapsed_ticks >= self.total_ticks
    }

    /// Reset the fader to the beginning.
    pub fn reset(&mut self) {
        self.elapsed_ticks = 0;
    }

    /// Reverse the fade direction (swap from/to).
    pub fn reverse(&mut self) {
        std::mem::swap(&mut self.from, &mut self.to);
        self.elapsed_ticks = 0;
    }
}

/// Linearly interpolate between two RGB4 colors at parameter t (0.0 = from, 1.0 = to).
fn lerp_rgb4(from: &RGB4, to: &RGB4, t: f32) -> RGB4 {
    let fr = ((from.color & 0xF00) >> 8) as f32;
    let fg = ((from.color & 0x0F0) >> 4) as f32;
    let fb = (from.color & 0x00F) as f32;

    let tr = ((to.color & 0xF00) >> 8) as f32;
    let tg = ((to.color & 0x0F0) >> 4) as f32;
    let tb = (to.color & 0x00F) as f32;

    let r = (fr + (tr - fr) * t).round() as u16;
    let g = (fg + (tg - fg) * t).round() as u16;
    let b = (fb + (tb - fb) * t).round() as u16;

    RGB4 { color: (r << 8) | (g << 4) | b }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp_rgb4_endpoints() {
        let black = RGB4 { color: 0x000 };
        let white = RGB4 { color: 0xFFF };

        let result_start = lerp_rgb4(&black, &white, 0.0);
        assert_eq!(result_start.color, 0x000);

        let result_end = lerp_rgb4(&black, &white, 1.0);
        assert_eq!(result_end.color, 0xFFF);
    }

    #[test]
    fn test_lerp_rgb4_midpoint() {
        let black = RGB4 { color: 0x000 };
        let white = RGB4 { color: 0xFFF };

        let result_mid = lerp_rgb4(&black, &white, 0.5);
        // 15 * 0.5 = 7.5, rounds to 8
        assert_eq!(result_mid.color, 0x888);
    }

    #[test]
    fn test_palette_fader_basic() {
        let from = Palette { colors: vec![RGB4 { color: 0x000 }, RGB4 { color: 0xFFF }] };
        let to = Palette { colors: vec![RGB4 { color: 0xFFF }, RGB4 { color: 0x000 }] };

        let mut fader = PaletteFader::new(&from, &to, 10);
        assert!(!fader.is_done());

        // At start, should be equal to 'from'
        let p0 = fader.current_palette();
        assert_eq!(p0.colors[0].color, 0x000);
        assert_eq!(p0.colors[1].color, 0xFFF);

        // Advance to end
        let p_end = fader.tick(10);
        assert!(fader.is_done());
        assert_eq!(p_end.colors[0].color, 0xFFF);
        assert_eq!(p_end.colors[1].color, 0x000);
    }

    #[test]
    fn test_palette_fader_reverse() {
        let from = Palette { colors: vec![RGB4 { color: 0x000 }] };
        let to = Palette { colors: vec![RGB4 { color: 0xFFF }] };

        let mut fader = PaletteFader::new(&from, &to, 10);
        fader.tick(10);
        assert!(fader.is_done());

        fader.reverse();
        assert!(!fader.is_done());

        let p = fader.tick(10);
        // reversed: from=FFF, to=000
        assert_eq!(p.colors[0].color, 0x000);
    }
}
