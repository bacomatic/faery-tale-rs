
use crate::game::image_texture::ImageTexture;

use sdl2::rect::Rect;
use sdl2::render::{Canvas, RenderTarget};

/**
 * Page flip animation - port of the original flipscan() function.
 *
 * The original uses three lookup tables (flip1=rate, flip2=width, flip3=delay)
 * across 22 steps. The first 11 steps flip the right half of the page,
 * the last 11 steps flip the left half.
 *
 * The animation composites vertical strips from the old page (pagea) and
 * new page (pageb) to simulate a physical page turning from right to left.
 *
 * The page_det() function defines a curved page edge shape.
 */

// rate: how many pixels to advance per strip
const FLIP1: [i32; 22] = [8,  6, 5, 4, 3, 2, 3, 5, 13, 0, 0,
                           13, 5, 3, 2, 3, 4, 5, 6, 8,  0, 0];
// width: width of each strip in pixels
const FLIP2: [i32; 22] = [7, 5, 4, 3, 2, 1, 1, 1, 1, 0, 0,
                           1, 1, 1, 1, 2, 3, 4, 5, 7, 0, 0];
// delay: extra delay in ticks after this step (original uses Delay(n) at 50Hz)
const FLIP3: [i32; 22] = [12, 9, 6, 3, 0, 0, 0, 0, 0, 0, 0,
                           0,  0, 0, 0, 0, 0, 3, 6, 9, 0, 0];

/// Lookup table for the page edge curve at small column offsets (0..10).
const PAGE_DET_TABLE: [i32; 11] = [9, 9, 8, 7, 6, 5, 5, 5, 4, 4, 4];

/// Port of page_det(col) from fsubs.asm.
/// Returns the vertical offset for the curved page edge at a given column position.
fn page_det(col: i32) -> i32 {
    if col < 11 {
        return PAGE_DET_TABLE[col as usize];
    }
    if col > 136 { return 10; }
    if col > 135 { return 7; }
    if col > 123 { return 6; }
    if col > 98 { return 5; }
    if col > 71 { return 4; }
    3 // default for 11..71
}

/// Manages the animated page flip sequence between two images.
pub struct PageFlip {
    /// Current step index (0..21).
    step: usize,
    /// Tick accumulator for timing between steps.
    tick_accum: u32,
    /// Delay remaining for the current step (from FLIP3).
    step_delay: u32,
    /// Whether this step has been drawn yet.
    step_drawn: bool,
}

impl PageFlip {
    pub fn new() -> PageFlip {
        PageFlip {
            step: 0,
            tick_accum: 0,
            step_delay: 0,
            step_drawn: false,
        }
    }

    /// Returns true when the animation is complete.
    pub fn is_done(&self) -> bool {
        self.step >= 22
    }

    /// Advance the animation by `delta` ticks and draw the current frame.
    ///
    /// `old_page` and `new_page` are the source images. Drawing happens onto
    /// the provided canvas. The images are expected to be 320x200 and are
    /// drawn starting at position (0, 0) in the canvas.
    ///
    /// Returns true if the animation is still running, false when complete.
    pub fn update<T: RenderTarget>(
        &mut self,
        canvas: &mut Canvas<T>,
        old_page: &ImageTexture,
        new_page: &ImageTexture,
        delta: u32,
    ) -> bool {
        if self.is_done() {
            return false;
        }

        // Handle delay between steps
        if self.step_drawn && self.step_delay > 0 {
            if delta >= self.step_delay {
                self.step_delay = 0;
            } else {
                self.step_delay -= delta;
                return true;
            }
        }

        // Time to advance to next step?
        if self.step_drawn {
            self.step += 1;
            if self.is_done() {
                // Draw the final new page
                new_page.draw(canvas, 0, 0);
                return false;
            }
            self.step_drawn = false;
        }

        // Draw the current step
        self.draw_step(canvas, old_page, new_page);
        self.step_drawn = true;

        // Convert original Delay(n) at 50Hz to ticks at 60Hz: multiply by 1.2
        let delay_50hz = FLIP3[self.step] as u32;
        self.step_delay = (delay_50hz as f32 * 1.2) as u32;

        // Consume one tick for the step itself (each step takes ~1 frame in the original)
        self.tick_accum = 0;

        true
    }

    /// Draw a single step of the flip animation.
    fn draw_step<T: RenderTarget>(
        &self,
        canvas: &mut Canvas<T>,
        old_page: &ImageTexture,
        new_page: &ImageTexture,
    ) {
        let i = self.step;
        let rate = FLIP1[i];
        let wide = FLIP2[i];

        if i < 11 {
            // Right half phase: reveal new page on the right, old page strips retreating

            // Draw the right half of the new page (x=160, w=135)
            // But first draw old page as the base for the left half
            old_page.draw_region(canvas, Rect::new(0, 0, 160, 200), 0, 0);
            new_page.draw_region(canvas, Rect::new(160, 0, 135, 200), 160, 0);

            if rate == 0 {
                // No strips to draw, just the base composition
                return;
            }

            // Overlay strips from the old page, creating folding effect
            let mut dcol: i32 = 0;
            let bcol = 161 - wide;
            let mut scol = wide;
            let mut h = 0i32;

            while scol < 136 {
                h = page_det(scol);
                let strip_height = (200 - h - h).max(0);
                if strip_height > 0 && wide > 0 {
                    old_page.draw_region(
                        canvas,
                        Rect::new(bcol + scol, h, wide as u32, strip_height as u32),
                        161 + dcol,
                        h,
                    );
                }
                dcol += wide;
                scol += rate;
            }

            // Draw the spine edge from old page
            if dcol > 0 {
                old_page.draw_region(
                    canvas,
                    Rect::new(296, 7, 1, 186),
                    161 + dcol,
                    h,
                );
            }
        } else {
            // Left half phase: reveal new page on the left, old page retreating
            let bcol = 160;

            if rate == 0 {
                // Just copy the left half of the new page
                new_page.draw_region(canvas, Rect::new(24, 0, 135, 200), 24, 0);
                // Right half is already the new page
                new_page.draw_region(canvas, Rect::new(160, 0, 135, 200), 160, 0);
                return;
            }

            // Draw the left half of old page as base, right half is new page
            old_page.draw_region(canvas, Rect::new(24, 0, 135, 200), 24, 0);
            new_page.draw_region(canvas, Rect::new(160, 0, 135, 200), 160, 0);

            // Overlay strips from new page expanding leftward
            let mut dcol: i32 = 0;
            let mut scol = wide;
            let mut h;

            while scol < 136 {
                h = page_det(scol);
                dcol += wide;
                let strip_height = (200 - h - h).max(0);
                if strip_height > 0 && wide > 0 {
                    new_page.draw_region(
                        canvas,
                        Rect::new(bcol - scol, h, wide as u32, strip_height as u32),
                        bcol - dcol,
                        h,
                    );
                }
                scol += rate;
            }

            // Draw the spine edge from new page
            let h = 7;
            if dcol > 0 {
                new_page.draw_region(
                    canvas,
                    Rect::new(24, h, 1, (200 - h - h) as u32),
                    159 - dcol,
                    h,
                );
            }
        }
    }
}
