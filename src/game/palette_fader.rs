
use crate::game::colors::{Palette, RGB4};

/// Scale an RGBA32 palette by a lightlevel percentage (0–100).
///
/// This is the day/night equivalent of `fade_page()` applied to the game's
/// RGBA32 atlas palette.  Each channel is multiplied by `pct / 100`.
///
/// Called each tick when `lightlevel` changes to keep the tile atlas
/// reflecting the current time-of-day brightness (gfx-101).
pub fn apply_lightlevel_dim(
    palette: &[u32; crate::game::palette::PALETTE_SIZE],
    pct: i16,
) -> [u32; crate::game::palette::PALETTE_SIZE] {
    let pct = pct.clamp(0, 100) as u32;
    let mut out = [0u32; crate::game::palette::PALETTE_SIZE];
    for (i, &c) in palette.iter().enumerate() {
        let r = (c >> 16 & 0xFF) * pct / 100;
        let g = (c >> 8  & 0xFF) * pct / 100;
        let b = (c        & 0xFF) * pct / 100;
        out[i] = 0xFF000000 | (r << 16) | (g << 8) | b;
    }
    out
}

/**
 * Port of the original game's `fade_page()` function from fmain2.c.
 *
 * Applies a per-channel multiplicative scale to a source palette. Each channel
 * (r, g, b) is specified as a percentage (0–100). When `limit` is true,
 * night-time corrections are applied: minimum floor values, blue tint injection,
 * and vegetation color boosting for palette indices 16–24.
 *
 * The `light_timer` flag simulates the Green Jewel light effect by boosting the red
 * channel of colors where red is less than green.
 */
pub fn fade_page(
    r: i16,
    g: i16,
    b: i16,
    limit: bool,
    light_timer: bool,
    colors: &Palette,
) -> Palette {
    // Clamp channel percentages to valid range
    let (r, g, b, g2) = if limit {
        // Night limits: never fully dark
        let r = r.clamp(10, 100);
        let g = g.clamp(25, 100);
        let b = b.clamp(60, 100);
        let g2 = ((100 - g) / 3) as i32;
        (r, g, b, g2)
    } else {
        let r = r.clamp(0, 100);
        let g = g.clamp(0, 100);
        let b = b.clamp(0, 100);
        (r, g, b, 0i32)
    };

    let mut faded = Vec::with_capacity(colors.colors.len());

    for (i, color) in colors.colors.iter().enumerate() {
        // Extract 4-bit channels, shifted to match original's bit positions:
        // Original: r1 = (colors[i] & 0x0F00) >> 4  → gives 0..0xF0 range
        //           g1 = colors[i] & 0x00F0           → gives 0..0xF0 range
        //           b1 = colors[i] & 0x000F           → gives 0..0xF range
        let mut r1 = ((color.color & 0x0F00) >> 4) as i32;
        let g1_raw = (color.color & 0x00F0) as i32;
        let b1_raw = (color.color & 0x000F) as i32;

        // Green Jewel light effect: boost red to at least green's level
        if light_timer && r1 < g1_raw {
            r1 = g1_raw;
        }

        // Apply multiplicative scale
        // Original: r1 = (r * r1) / 1600  (r is 0–100, r1 is 0–0xF0=240)
        // At 100%: r1 = (100 * 240) / 1600 = 15 → the original 4-bit value
        r1 = (r as i32 * r1) / 1600;
        let mut g1 = (g as i32 * g1_raw) / 1600;
        // Blue channel includes night blue tint injection: g2 * g1 adds moonlight
        let mut b1 = (b as i32 * b1_raw + g2 * g1) / 100;

        if limit {
            // Vegetation blue boost for palette indices 16–24 at partial darkness
            if i >= 16 && i <= 24 && g > 20 {
                if g < 50 {
                    b1 += 2;
                } else if g < 75 {
                    b1 += 1;
                }
            }
            if b1 > 15 {
                b1 = 15;
            }
        }

        // Clamp all channels to 4-bit range
        r1 = r1.clamp(0, 15);
        g1 = g1.clamp(0, 15);
        b1 = b1.clamp(0, 15);

        faded.push(RGB4 {
            color: ((r1 as u16) << 8) | ((g1 as u16) << 4) | (b1 as u16),
        });
    }

    Palette { colors: faded }
}

/// The result of a fade operation. Determines how the fade should be applied
/// to the rendering pipeline.
pub enum FadeResult {
    /// A uniform brightness scale that can be applied via SDL2 `set_color_mod()`.
    /// The three values are the RGB modulation bytes (0–255).
    ColorMod(u8, u8, u8),
    /// A fully computed palette that must be applied via `ImageTexture::update()`.
    /// Used for non-uniform fades (zoom, day/night) where per-color manipulation
    /// is needed.
    PaletteUpdate(Palette),
}

/**
 * Controls a palette fade over time using the original game's `fade_page()`
 * algorithm. Interpolates the per-channel percentages from start to target
 * over a given duration.
 *
 * For uniform fades (all channels equal, no night limits), returns a
 * `FadeResult::ColorMod` that can be applied cheaply via SDL2 color modulation.
 * For non-uniform fades, returns `FadeResult::PaletteUpdate` with the fully
 * computed palette.
 */
pub struct FadeController {
    /// Source palette to fade
    source: Palette,
    /// Starting channel percentages (r, g, b)
    from_rgb: (i16, i16, i16),
    /// Target channel percentages (r, g, b)
    to_rgb: (i16, i16, i16),
    /// Whether night limits apply
    limit: bool,
    /// Whether the Green Jewel light effect is active
    light_timer: bool,
    /// Elapsed ticks since fade started
    elapsed_ticks: u32,
    /// Total fade duration in ticks
    total_ticks: u32,
}

impl FadeController {
    /// Create a fade controller with full control over channel percentages.
    pub fn new(
        source: &Palette,
        from_rgb: (i16, i16, i16),
        to_rgb: (i16, i16, i16),
        limit: bool,
        light_timer: bool,
        duration_ticks: u32,
    ) -> FadeController {
        FadeController {
            source: Palette { colors: source.colors.clone() },
            from_rgb,
            to_rgb,
            limit,
            light_timer,
            elapsed_ticks: 0,
            total_ticks: duration_ticks,
        }
    }

    /// Create a uniform fade from full brightness to black (100 → 0 on all channels).
    /// Equivalent to the original `fade_down()`: 21 steps with Delay(1) each.
    /// At 30Hz, 24 ticks ≈ 0.8s.
    pub fn fade_down(source: &Palette, duration_ticks: u32) -> FadeController {
        FadeController::new(source, (100, 100, 100), (0, 0, 0), false, false, duration_ticks)
    }

    /// Create a uniform fade from black to full brightness (0 → 100 on all channels).
    /// Equivalent to the original `fade_normal()`.
    pub fn fade_normal(source: &Palette, duration_ticks: u32) -> FadeController {
        FadeController::new(source, (0, 0, 0), (100, 100, 100), false, false, duration_ticks)
    }

    /// Compute the fade percentages for a given zoom half-width value.
    /// Replicates the original `screen_size()` formula:
    ///   y = (x * 5) / 8
    ///   fade_page(y*2 - 40, y*2 - 70, y*2 - 100, 0, introcolors)
    ///
    /// At x=0: (-40, -70, -100) → clamped to (0, 0, 0) → black
    /// At x=80: (60, 30, 0) → partial, no blue yet
    /// At x=160: (160, 130, 100) → clamped to (100, 100, 100) → full color
    ///
    /// Returns (r, g, b) percentages. The caller should apply these via `fade_page()`.
    pub fn zoom_percentages(half_width: i32) -> (i16, i16, i16) {
        let y = (half_width * 5) / 8;
        let r = (y * 2 - 40) as i16;
        let g = (y * 2 - 70) as i16;
        let b = (y * 2 - 100) as i16;
        (r, g, b)
    }

    /// Compute a faded palette for a given zoom level directly, without time
    /// interpolation. Used when the zoom position itself drives the fade.
    pub fn zoom_fade(source: &Palette, half_width: i32) -> Palette {
        let (r, g, b) = FadeController::zoom_percentages(half_width);
        fade_page(r, g, b, false, false, source)
    }

    /// Get the interpolation parameter t (0.0 at start, 1.0 at end).
    fn t(&self) -> f32 {
        if self.total_ticks == 0 {
            1.0
        } else {
            self.elapsed_ticks as f32 / self.total_ticks as f32
        }
    }

    /// Get the current interpolated channel percentages.
    fn current_percentages(&self) -> (i16, i16, i16) {
        let t = self.t();
        let r = self.from_rgb.0 as f32 + (self.to_rgb.0 - self.from_rgb.0) as f32 * t;
        let g = self.from_rgb.1 as f32 + (self.to_rgb.1 - self.from_rgb.1) as f32 * t;
        let b = self.from_rgb.2 as f32 + (self.to_rgb.2 - self.from_rgb.2) as f32 * t;
        (r.round() as i16, g.round() as i16, b.round() as i16)
    }

    /// Returns true if this is a uniform fade (all channels equal, no limits).
    /// Uniform fades can use SDL2 color modulation instead of palette regeneration.
    pub fn is_uniform(&self) -> bool {
        !self.limit
            && self.from_rgb.0 == self.from_rgb.1
            && self.from_rgb.0 == self.from_rgb.2
            && self.to_rgb.0 == self.to_rgb.1
            && self.to_rgb.0 == self.to_rgb.2
            && !self.light_timer
    }

    /// Advance the fade by `delta` ticks and return the appropriate fade result.
    ///
    /// For uniform fades, returns `FadeResult::ColorMod` with SDL2 color modulation values.
    /// For non-uniform fades, returns `FadeResult::PaletteUpdate` with a fully computed palette.
    pub fn tick(&mut self, delta: u32) -> FadeResult {
        self.elapsed_ticks = (self.elapsed_ticks + delta).min(self.total_ticks);
        self.current_result()
    }

    /// Get the current fade result without advancing time.
    pub fn current_result(&self) -> FadeResult {
        let (r, g, b) = self.current_percentages();

        if self.is_uniform() {
            // All channels are equal — use SDL2 color modulation
            // Convert percentage (0–100) to byte (0–255)
            let mod_val = ((r as f32 / 100.0) * 255.0).round().clamp(0.0, 255.0) as u8;
            FadeResult::ColorMod(mod_val, mod_val, mod_val)
        } else {
            // Non-uniform — compute full palette
            FadeResult::PaletteUpdate(fade_page(r, g, b, self.limit, self.light_timer, &self.source))
        }
    }

    /// Returns true when the fade is complete.
    pub fn is_done(&self) -> bool {
        self.elapsed_ticks >= self.total_ticks
    }

    /// Reset the fade to the beginning.
    pub fn reset(&mut self) {
        self.elapsed_ticks = 0;
    }

    /// Reverse the fade direction (swap from/to percentages).
    pub fn reverse(&mut self) {
        std::mem::swap(&mut self.from_rgb, &mut self.to_rgb);
        self.elapsed_ticks = 0;
    }
}

/**
 * Interpolates between two palettes over a given duration in ticks (1/60s).
 *
 * The original game fades in 20 steps with Delay(1) each (~0.4s total at 50Hz).
 * At 60 ticks/second, 24 ticks gives a similar ~0.4s fade.
 *
 * Note: This performs a simple linear interpolation between two palettes.
 * For fades that match the original game's multiplicative per-channel scaling
 * with night corrections, use `FadeController` with `fade_page()` instead.
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

    // ---- fade_page tests ----

    #[test]
    fn test_fade_page_all_zero_produces_black() {
        let palette = Palette {
            colors: vec![
                RGB4 { color: 0xFFF },
                RGB4 { color: 0xA52 },
                RGB4 { color: 0x390 },
            ],
        };
        let result = fade_page(0, 0, 0, false, false, &palette);
        for c in &result.colors {
            assert_eq!(c.color, 0x000, "All colors should be black at 0% fade");
        }
    }

    #[test]
    fn test_fade_page_full_reproduces_original() {
        let palette = Palette {
            colors: vec![
                RGB4 { color: 0xFFF },
                RGB4 { color: 0xA52 },
                RGB4 { color: 0x080 },
            ],
        };
        let result = fade_page(100, 100, 100, false, false, &palette);
        assert_eq!(result.colors[0].color, 0xFFF);
        assert_eq!(result.colors[1].color, 0xA52);
        assert_eq!(result.colors[2].color, 0x080);
    }

    #[test]
    fn test_fade_page_night_limits_enforce_floor() {
        // With limit=true and very low percentages, the floor clamps apply:
        // r >= 10, g >= 25, b >= 60
        let palette = Palette {
            colors: vec![RGB4 { color: 0xFFF }],
        };
        let result = fade_page(0, 0, 0, true, false, &palette);
        // At the minimum limits with a white color:
        // r1 = (10 * 0xF0) / 1600 = (10 * 240) / 1600 = 1
        // g1 = (25 * 0xF0) / 1600 = (25 * 240) / 1600 = 3  (3.75 truncated)
        // g2 = (100 - 25) / 3 = 25
        // b1 = (60 * 15 + 25 * 3) / 100 = (900 + 75) / 100 = 9
        let r = (result.colors[0].color & 0xF00) >> 8;
        let g = (result.colors[0].color & 0x0F0) >> 4;
        let b = result.colors[0].color & 0x00F;
        assert!(r >= 1, "Red should be at least 1 with night floor on white");
        assert!(g >= 3, "Green should be at least 3 with night floor on white");
        assert!(b >= 5, "Blue should be boosted with night tint on white");
    }

    #[test]
    fn test_fade_page_vegetation_boost() {
        // Create a palette with 25 entries, indices 16–24 should get blue boost
        let mut colors = vec![RGB4 { color: 0x000 }; 25];
        colors[16] = RGB4 { color: 0x390 }; // vegetation green
        colors[20] = RGB4 { color: 0x4A2 };
        let palette = Palette { colors };

        // At g=40 (between 20 and 50), b1 should get +2
        let result = fade_page(50, 40, 50, true, false, &palette);
        // Without boost, b1 for color 0x390 would be:
        // b1_raw = 0, so b1 = (50*0 + g2*g1)/100 = (g2*g1)/100
        // g2 = (100-40)/3 = 20, g1 = (40 * 0x90) / 1600 = (40*144)/1600 = 3
        // b1 = (20*3)/100 = 0, +2 boost = 2
        let b16 = result.colors[16].color & 0x00F;
        assert!(b16 >= 2, "Vegetation index 16 should get blue boost at partial darkness, got {}", b16);
    }

    #[test]
    fn test_fade_page_light_timer_boosts_red() {
        // Color where red < green: 0x090 (r=0, g=9, b=0)
        let palette = Palette {
            colors: vec![RGB4 { color: 0x090 }],
        };
        // Without light_timer
        let no_light = fade_page(50, 50, 50, false, false, &palette);
        // With light_timer: red should be boosted to green's level before scaling
        let with_light = fade_page(50, 50, 50, false, true, &palette);

        let no_r = (no_light.colors[0].color & 0xF00) >> 8;
        let with_r = (with_light.colors[0].color & 0xF00) >> 8;
        assert!(with_r > no_r, "Light timer should boost red: {} > {}", with_r, no_r);
    }

    #[test]
    fn test_fade_page_zoom_midpoint() {
        // At zoom half_width=80: y=50, percentages = (60, 30, 0)
        // Blue channel should be 0 (clamped from negative)
        let palette = Palette {
            colors: vec![RGB4 { color: 0xFFF }],
        };
        let (r, g, b) = FadeController::zoom_percentages(80);
        assert_eq!(r, 60);
        assert_eq!(g, 30);
        assert_eq!(b, 0);

        let result = fade_page(r, g, b, false, false, &palette);
        let rb = result.colors[0].color & 0x00F;
        assert_eq!(rb, 0, "Blue should be 0 at zoom midpoint");
    }

    // ---- FadeController tests ----

    #[test]
    fn test_fade_controller_uniform_uses_color_mod() {
        let palette = Palette {
            colors: vec![RGB4 { color: 0xFFF }],
        };
        let mut fc = FadeController::fade_down(&palette, 10);
        assert!(fc.is_uniform());

        // At start (100%), should return ColorMod(255, 255, 255)
        match fc.current_result() {
            FadeResult::ColorMod(r, g, b) => {
                assert_eq!(r, 255);
                assert_eq!(g, 255);
                assert_eq!(b, 255);
            }
            _ => panic!("Expected ColorMod for uniform fade at start"),
        }

        // At end (0%), should return ColorMod(0, 0, 0)
        fc.tick(10);
        assert!(fc.is_done());
        match fc.current_result() {
            FadeResult::ColorMod(r, g, b) => {
                assert_eq!(r, 0);
                assert_eq!(g, 0);
                assert_eq!(b, 0);
            }
            _ => panic!("Expected ColorMod for uniform fade at end"),
        }
    }

    #[test]
    fn test_fade_controller_non_uniform_uses_palette() {
        let palette = Palette {
            colors: vec![RGB4 { color: 0xFFF }],
        };
        // Non-uniform: different channel targets
        let mut fc = FadeController::new(&palette, (0, 0, 0), (100, 70, 40), false, false, 10);
        assert!(!fc.is_uniform());

        fc.tick(10);
        match fc.current_result() {
            FadeResult::PaletteUpdate(p) => {
                assert_eq!(p.colors.len(), 1);
                // At (100, 70, 40): r=15, g=10, b=6
                let r = (p.colors[0].color & 0xF00) >> 8;
                let g = (p.colors[0].color & 0x0F0) >> 4;
                let b = p.colors[0].color & 0x00F;
                assert_eq!(r, 15);
                assert_eq!(g, 10);
                assert_eq!(b, 6);
            }
            _ => panic!("Expected PaletteUpdate for non-uniform fade"),
        }
    }

    #[test]
    fn test_fade_controller_zoom_fade() {
        let palette = Palette {
            colors: vec![RGB4 { color: 0xFFF }],
        };
        // Fully open (half_width=160): should reproduce original palette
        let full = FadeController::zoom_fade(&palette, 160);
        assert_eq!(full.colors[0].color, 0xFFF);

        // Fully closed (half_width=0): should be black
        let closed = FadeController::zoom_fade(&palette, 0);
        assert_eq!(closed.colors[0].color, 0x000);
    }

    #[test]
    fn test_fade_controller_reverse() {
        let palette = Palette {
            colors: vec![RGB4 { color: 0xFFF }],
        };
        let mut fc = FadeController::fade_down(&palette, 10);
        fc.tick(10);
        assert!(fc.is_done());

        fc.reverse();
        assert!(!fc.is_done());

        // After reverse, should fade from 0 to 100 (= fade_normal)
        fc.tick(10);
        match fc.current_result() {
            FadeResult::ColorMod(r, _, _) => assert_eq!(r, 255),
            _ => panic!("Expected ColorMod after reverse"),
        }
    }

    // ---- PaletteFader (lerp) tests ----

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
