use sdl2::rect::Rect;

/**
 * Viewport zoom effect - port of the original screen_size() function.
 *
 * The original function takes a half-width parameter `x` (0-160) and computes
 * a centered viewport: width = x*2, height = (x*5/8)*2, centered on (160, 100).
 * This creates a growing/shrinking reveal rectangle on the 320x200 screen.
 *
 * The zoom is combined with a palette fade: as the viewport opens, the palette
 * transitions from black to the target palette.
 *
 * Original: for (i = 0; i <= 160; i += 4) screen_size(i)  -> zoom in (40 steps)
 * Original: for (i = 156; i >= 0; i -= 4) screen_size(i)  -> zoom out (40 steps)
 */

/// The step size used in the original zoom loops.
const ZOOM_STEP: i32 = 4;

/// Maximum half-width for the zoom effect.
const ZOOM_MAX: i32 = 160;

/// Reduce a fraction n/d to lowest terms using Euclid's algorithm.
fn reduce_fraction(n: u32, d: u32) -> (u32, u32) {
    fn gcd(a: u32, b: u32) -> u32 {
        if b == 0 {
            a
        } else {
            gcd(b, a % b)
        }
    }
    let g = gcd(n, d);
    (n / g, d / g)
}

/// Compute the viewport rectangle for a given zoom level.
/// `half_width` ranges from 0 (fully closed) to 160 (fully open).
/// Returns the source rect within the 320x200 offscreen texture.
pub fn zoom_rect(half_width: i32) -> Rect {
    let x = half_width.clamp(0, ZOOM_MAX);
    let half_height = (x * 5) / 8;

    let left = 160 - x;
    let top = 100 - half_height;
    let width = (x * 2).max(0) as u32;
    let height = (half_height * 2).max(0) as u32;

    Rect::new(left, top, width, height)
}

/// Number of zoom steps (0..ZOOM_MAX by ZOOM_STEP).
const ZOOM_STEPS: u32 = (ZOOM_MAX / ZOOM_STEP) as u32; // 40

/// Manages the animated zoom in/out sequence.
pub struct ViewportZoom {
    /// Current half-width value (0 = closed, 160 = full open).
    current: i32,
    /// Target half-width value.
    target: i32,
    /// Direction: +ZOOM_STEP for zoom in, -ZOOM_STEP for zoom out.
    step: i32,
    /// Rational tick accumulator (in units of 1/step_denom ticks).
    tick_accum: u32,
    /// Step timing numerator: one step advances every (step_numer/step_denom) real ticks.
    step_numer: u32,
    /// Step timing denominator.
    step_denom: u32,
}

impl ViewportZoom {
    /// Create a zoom-in effect (0 -> 160) with a specified total duration.
    pub fn zoom_in_duration(total_ticks: u32) -> ViewportZoom {
        // Reduce total_ticks/ZOOM_STEPS to lowest terms for the rational accumulator.
        let (n, d) = reduce_fraction(total_ticks, ZOOM_STEPS);
        ViewportZoom {
            current: 0,
            target: ZOOM_MAX,
            step: ZOOM_STEP,
            tick_accum: 0,
            step_numer: n,
            step_denom: d,
        }
    }

    /// Create a zoom-in effect (0 -> 160).
    pub fn zoom_in() -> ViewportZoom {
        ViewportZoom::zoom_in_duration(2 * ZOOM_STEPS) // original: 2 ticks/step
    }

    /// Create a zoom-out effect (156 -> 0) with a specified total duration.
    pub fn zoom_out_duration(total_ticks: u32) -> ViewportZoom {
        let (n, d) = reduce_fraction(total_ticks, ZOOM_STEPS);
        ViewportZoom {
            current: ZOOM_MAX - ZOOM_STEP, // 156, matching the original
            target: 0,
            step: -ZOOM_STEP,
            tick_accum: 0,
            step_numer: n,
            step_denom: d,
        }
    }

    /// Create a zoom-out effect (156 -> 0).
    pub fn zoom_out() -> ViewportZoom {
        ViewportZoom::zoom_out_duration(2 * ZOOM_STEPS) // original: 2 ticks/step
    }

    /// Advance the zoom by `delta` ticks. Returns the current viewport rect.
    pub fn tick(&mut self, delta: u32) -> Rect {
        // Accumulate in units of (1/step_denom) ticks; advance a step per step_numer units.
        self.tick_accum += delta * self.step_denom;
        while self.tick_accum >= self.step_numer && !self.is_done() {
            self.tick_accum -= self.step_numer;
            self.current += self.step;
            // Clamp to bounds
            if self.step > 0 {
                self.current = self.current.min(self.target);
            } else {
                self.current = self.current.max(self.target);
            }
        }
        zoom_rect(self.current)
    }

    /// Returns a value from 0.0 to 1.0 indicating zoom progress.
    /// For zoom-in: 0.0 = closed, 1.0 = fully open.
    /// For zoom-out: 0.0 = fully open, 1.0 = fully closed.
    pub fn progress(&self) -> f32 {
        if self.step > 0 {
            // zoom in
            self.current as f32 / ZOOM_MAX as f32
        } else {
            // zoom out: progress goes 0..1 as we close
            1.0 - (self.current as f32 / (ZOOM_MAX - ZOOM_STEP) as f32)
        }
    }

    /// Returns true when the zoom animation is complete.
    pub fn is_done(&self) -> bool {
        self.current == self.target
    }

    /// Get the current viewport rect without advancing.
    pub fn current_rect(&self) -> Rect {
        zoom_rect(self.current)
    }

    /// Get the current half-width value (0 = closed, 160 = full open).
    /// This is the raw zoom position used by the original's screen_size() function.
    pub fn half_width(&self) -> i32 {
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zoom_rect_closed() {
        let r = zoom_rect(0);
        assert_eq!(r.x(), 160);
        assert_eq!(r.y(), 100);
        assert!(r.width() <= 1);
        assert!(r.height() <= 1);
    }

    #[test]
    fn test_zoom_rect_full() {
        let r = zoom_rect(160);
        assert_eq!(r.x(), 0);
        assert_eq!(r.y(), 0);
        assert_eq!(r.width(), 320);
        assert_eq!(r.height(), 200);
    }

    #[test]
    fn test_zoom_rect_midpoint() {
        let r = zoom_rect(80);
        assert_eq!(r.x(), 80);
        assert_eq!(r.y(), 50);
        assert_eq!(r.width(), 160);
        assert_eq!(r.height(), 100);
    }

    #[test]
    fn test_zoom_in_progression_original() {
        // Original timing: 2 ticks/step × 40 steps = 80 ticks total
        let mut zoom = ViewportZoom::zoom_in();
        assert!(!zoom.is_done());
        assert_eq!(zoom.current, 0);
        for _ in 0..40 {
            zoom.tick(2);
        }
        assert!(zoom.is_done());
        assert_eq!(zoom.current, 160);
    }

    #[test]
    fn test_zoom_in_3s_progression() {
        // 3s = 180 ticks total: 4.5 ticks/step × 40 steps
        let mut zoom = ViewportZoom::zoom_in_duration(180);
        assert!(!zoom.is_done());
        // Feed exactly 180 ticks one at a time
        for _ in 0..180 {
            zoom.tick(1);
        }
        assert!(zoom.is_done());
        assert_eq!(zoom.current, 160);
    }

    #[test]
    fn test_zoom_out_progression() {
        let mut zoom = ViewportZoom::zoom_out();
        assert!(!zoom.is_done());
        for _ in 0..40 {
            zoom.tick(2);
        }
        assert!(zoom.is_done());
        assert_eq!(zoom.current, 0);
    }

    #[test]
    fn test_reduce_fraction() {
        assert_eq!(reduce_fraction(180, 40), (9, 2));
        assert_eq!(reduce_fraction(80, 40), (2, 1));
        assert_eq!(reduce_fraction(60, 40), (3, 2));
    }
}
