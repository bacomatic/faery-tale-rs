
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
 *
 * Each step has a Delay(2) = ~33ms, so the full zoom takes ~1.3 seconds.
 */

/// The step size used in the original zoom loops.
const ZOOM_STEP: i32 = 4;

/// Maximum half-width for the zoom effect.
const ZOOM_MAX: i32 = 160;

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

/// Manages the animated zoom in/out sequence.
pub struct ViewportZoom {
    /// Current half-width value (0 = closed, 160 = full open).
    current: i32,
    /// Target half-width value.
    target: i32,
    /// Direction: +ZOOM_STEP for zoom in, -ZOOM_STEP for zoom out.
    step: i32,
    /// Tick accumulator for timing. Original uses Delay(2) per step = ~2 ticks at 60Hz.
    tick_accum: u32,
    /// Ticks per zoom step. Original is ~2 ticks (Delay(2) at 50Hz ≈ 2.4 ticks at 60Hz).
    ticks_per_step: u32,
}

impl ViewportZoom {
    /// Create a zoom-in effect (0 -> 160).
    pub fn zoom_in() -> ViewportZoom {
        ViewportZoom {
            current: 0,
            target: ZOOM_MAX,
            step: ZOOM_STEP,
            tick_accum: 0,
            ticks_per_step: 2,
        }
    }

    /// Create a zoom-out effect (156 -> 0).
    pub fn zoom_out() -> ViewportZoom {
        ViewportZoom {
            current: ZOOM_MAX - ZOOM_STEP, // 156, matching the original
            target: 0,
            step: -ZOOM_STEP,
            tick_accum: 0,
            ticks_per_step: 2,
        }
    }

    /// Advance the zoom by `delta` ticks. Returns the current viewport rect.
    pub fn tick(&mut self, delta: u32) -> Rect {
        self.tick_accum += delta;
        while self.tick_accum >= self.ticks_per_step && !self.is_done() {
            self.tick_accum -= self.ticks_per_step;
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
        // SDL2 Rect enforces minimum size of 1, so width/height won't be 0
        // This is fine — the zoom loop starts at step 4, never actually blits a 0-size rect.
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
    fn test_zoom_in_progression() {
        let mut zoom = ViewportZoom::zoom_in();
        assert!(!zoom.is_done());
        assert_eq!(zoom.current, 0);

        // Advance through all 40 steps (each step = 2 ticks)
        for _ in 0..40 {
            zoom.tick(2);
        }
        assert!(zoom.is_done());
        assert_eq!(zoom.current, 160);
    }

    #[test]
    fn test_zoom_out_progression() {
        let mut zoom = ViewportZoom::zoom_out();
        assert!(!zoom.is_done());

        // Advance through all steps
        for _ in 0..40 {
            zoom.tick(2);
        }
        assert!(zoom.is_done());
        assert_eq!(zoom.current, 0);
    }
}
