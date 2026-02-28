
use sdl2::rect::Rect;
use sdl2::render::{Canvas, RenderTarget, Texture};

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
 *
 * The animation draws directly to the provided canvas using display-scale
 * coordinates:  dest_x = src_x * scale,  dest_y = y_offset + src_y * scale.
 * This allows rendering onto the 640×480 window canvas from 320×200 source
 * textures without an intermediate offscreen composite.
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

/// Copy a sub-region from `texture` to `canvas` with display scaling.
///
/// `src` is the source rect in native (320×200) coordinates.
/// The destination is scaled by `scale` and offset vertically by `y_offset`.
fn copy_region<T: RenderTarget>(
    canvas: &mut Canvas<T>,
    texture: &Texture,
    src: Rect,
    dst_x: i32,
    dst_y: i32,
    scale: u32,
    y_offset: i32,
) {
    let dst = Rect::new(
        dst_x * scale as i32,
        y_offset + dst_y * scale as i32,
        src.width() * scale,
        src.height() * scale,
    );
    canvas.copy(texture, Some(src), Some(dst)).unwrap();
}

/// Copy the full texture (320×200) to `canvas` with display scaling.
fn copy_full<T: RenderTarget>(
    canvas: &mut Canvas<T>,
    texture: &Texture,
    scale: u32,
    y_offset: i32,
) {
    let dst = Rect::new(0, y_offset, 320 * scale, 200 * scale);
    canvas.copy(texture, None, Some(dst)).unwrap();
}

/// Manages the animated page flip sequence between two textures.
///
/// Source textures are expected to be 320×200.  The animation is drawn
/// directly to the provided canvas using the given display `scale` and
/// vertical `y_offset`.
pub struct PageFlip {
    /// Current step index (0..21).
    step: usize,
    /// Delay remaining for the current step (from FLIP3).
    step_delay: u32,
    /// Whether this step has been drawn (and its delay computed).
    step_drawn: bool,
}

impl PageFlip {
    pub fn new() -> PageFlip {
        PageFlip {
            step: 0,
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
    /// `old_page` and `new_page` are 320×200 source textures.
    /// `scale` and `y_offset` map native coordinates to the canvas:
    ///   dest_x = src_x * scale, dest_y = y_offset + src_y * scale.
    ///
    /// Every call draws the current step to the canvas (required because
    /// the window canvas is cleared each frame).
    ///
    /// Returns true while the animation is running, false when complete.
    pub fn update<T: RenderTarget>(
        &mut self,
        canvas: &mut Canvas<T>,
        old_page: &Texture,
        new_page: &Texture,
        delta: u32,
        scale: u32,
        y_offset: i32,
    ) -> bool {
        if self.is_done() {
            return false;
        }

        // Count down the hold-delay for the current step.
        if self.step_drawn && self.step_delay > 0 {
            if delta >= self.step_delay {
                self.step_delay = 0;
            } else {
                self.step_delay -= delta;
            }
        }

        // When hold-delay expires, advance to the next step.
        if self.step_drawn && self.step_delay == 0 {
            self.step += 1;
            if self.is_done() {
                // Draw the final new page
                copy_full(canvas, new_page, scale, y_offset);
                return false;
            }
            self.step_drawn = false;
        }

        // First time seeing this step — compute its hold-delay.
        // A minimum delay of 2 ticks (~33ms) approximates the CPU/blitter
        // overhead on the original Amiga, where even zero-delay steps took
        // real time to execute flipscan().
        if !self.step_drawn {
            // FLIP3 values are NTSC 60Hz ticks directly — no conversion needed.
            // A minimum of 2 ticks approximates the blitter/CPU overhead on
            // the original Amiga even for zero-delay steps.
            self.step_delay = (FLIP3[self.step] as u32).max(2);
            self.step_drawn = true;
        }

        // Draw the current step (every frame, since the canvas is cleared).
        self.draw_step(canvas, old_page, new_page, scale, y_offset);
        true
    }

    /// Draw a single step of the flip animation.
    fn draw_step<T: RenderTarget>(
        &self,
        canvas: &mut Canvas<T>,
        old_page: &Texture,
        new_page: &Texture,
        scale: u32,
        y_offset: i32,
    ) {
        let i = self.step;
        let rate = FLIP1[i];
        let wide = FLIP2[i];

        if i < 11 {
            // Right half phase: reveal new page on the right, old page strips retreating

            // Draw the left half from old page and right half from new page
            copy_region(canvas, old_page, Rect::new(0, 0, 160, 200), 0, 0, scale, y_offset);
            copy_region(canvas, new_page, Rect::new(160, 0, 160, 200), 160, 0, scale, y_offset);

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
                    copy_region(
                        canvas, old_page,
                        Rect::new(bcol + scol, h, wide as u32, strip_height as u32),
                        161 + dcol, h,
                        scale, y_offset,
                    );
                }
                dcol += wide;
                scol += rate;
            }

            // Draw the spine edge from old page
            if dcol > 0 {
                copy_region(
                    canvas, old_page,
                    Rect::new(296, 7, 1, 186),
                    161 + dcol, h,
                    scale, y_offset,
                );
            }
        } else {
            // Left half phase: reveal new page on the left, old page retreating
            let bcol = 160;

            if rate == 0 {
                // Just copy both halves from new page
                copy_region(canvas, new_page, Rect::new(0, 0, 160, 200), 0, 0, scale, y_offset);
                copy_region(canvas, new_page, Rect::new(160, 0, 160, 200), 160, 0, scale, y_offset);
                return;
            }

            // Draw the left half of old page as base, right half is new page
            copy_region(canvas, old_page, Rect::new(0, 0, 160, 200), 0, 0, scale, y_offset);
            copy_region(canvas, new_page, Rect::new(160, 0, 160, 200), 160, 0, scale, y_offset);

            // Overlay strips from new page expanding leftward
            let mut dcol: i32 = 0;
            let mut scol = wide;
            let mut h;

            while scol < 136 {
                h = page_det(scol);
                dcol += wide;
                let strip_height = (200 - h - h).max(0);
                if strip_height > 0 && wide > 0 {
                    copy_region(
                        canvas, new_page,
                        Rect::new(bcol - scol, h, wide as u32, strip_height as u32),
                        bcol - dcol, h,
                        scale, y_offset,
                    );
                }
                scol += rate;
            }

            // Draw the spine edge from new page
            let h = 7;
            if dcol > 0 {
                copy_region(
                    canvas, new_page,
                    Rect::new(24, h, 1, (200 - h - h) as u32),
                    159 - dcol, h,
                    scale, y_offset,
                );
            }
        }
    }
}
