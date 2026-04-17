
use crate::game::font_texture::FontTexture;
use crate::game::colors::Palette;
use crate::game::render_task::RenderTask;

use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::render::Canvas;
use sdl2::render::RenderTarget;

use sdl2::video::Window;
use serde::Deserialize;

/*
 * A page of text, possibly with a fancy swirly border.
 */

#[derive(Deserialize, Debug)]
pub struct PlacardLine {
    x: usize,
    y: usize,
    text: String
}

#[derive(Deserialize, Debug)]
pub struct Placard {
    #[serde(default)]
    lines: Vec<PlacardLine>
}

impl Placard {
    pub fn print(&self) {
        for line in &self.lines {
            // only use x here
            println!("{0: <1$}{2}", "", line.x/10, line.text);
        }
    }

    pub fn draw<'a, T: RenderTarget>(&self, font: &FontTexture<'a>, canvas: &mut Canvas<T>) {
        for line in &self.lines {
            font.render_string(&line.text, canvas, line.x as i32, line.y as i32);
        }
    }

    /// Draw the placard text with a pixel offset applied to all line positions.
    /// Useful for positioning the text within different screen regions (e.g.,
    /// title text centered vertically on a 640x480 canvas).
    pub fn draw_offset<'a, T: RenderTarget>(
        &self,
        font: &FontTexture<'a>,
        canvas: &mut Canvas<T>,
        x_offset: i32,
        y_offset: i32,
    ) {
        for line in &self.lines {
            font.render_string(
                &line.text,
                canvas,
                line.x as i32 + x_offset,
                line.y as i32 + y_offset,
            );
        }
    }

    /// Like `draw_offset`, but substitutes `%` in every line with the given
    /// name. This mirrors the original `name()` call emitted inline between
    /// `placard_text()` calls — e.g. for the victory and princess-rescue
    /// placards ("Having defeated the villanous Necromancer and recovered the
    /// Talisman, [name] returned to Marheim…").
    pub fn draw_offset_substituted<'a, T: RenderTarget>(
        &self,
        font: &FontTexture<'a>,
        canvas: &mut Canvas<T>,
        x_offset: i32,
        y_offset: i32,
        substitution: &str,
    ) {
        for line in &self.lines {
            let text = line.text.replace('%', substitution);
            font.render_string(
                &text,
                canvas,
                line.x as i32 + x_offset,
                line.y as i32 + y_offset,
            );
        }
    }

    /// Draw the placard text at 2× glyph height (title screen).
    /// Simulates Amiga CRT line-doubling: each glyph is rendered at normal
    /// width but 2× height, with Y positions also doubled so line spacing
    /// scales correctly.
    pub fn draw_line_doubled<'a, T: RenderTarget>(
        &self,
        font: &FontTexture<'a>,
        canvas: &mut Canvas<T>,
        x_offset: i32,
        y_offset: i32,
    ) {
        for line in &self.lines {
            font.render_string_hires(
                &line.text,
                canvas,
                line.x as i32 + x_offset,
                (line.y as i32) * 2 + y_offset,
            );
        }
    }
}

/**
 * This struct holds the renderer state for placards, since we need to draw them over time.
 * Each tick will draw one segment of the border at each side. In total there are 16 segments
 * per block, and up to 17 blocks per side (horizontal). It should take about 17*16 = 272
 * ticks to draw the full border, or around 4.5 seconds.
 *
 * This is slightly different from the original game, that didn't have any timing delays but instead
 * repeatedly drew the same border segments multiple times to create a delay effect, we don't need to
 * do that.
 */
#[derive(Debug)]
pub struct PlacardRenderer {
    block_index: usize, // outer loop: current block index 0..17
    segment_index: usize, // inner loop: current segment index 0..16

    xorg: i32, // current x origin for drawing (border-local)
    yorg: i32, // current y origin for drawing (border-local)

    /// Horizontal pixel offset applied to all drawn x coordinates.
    /// Set to (canvas_width - 284) / 2 for horizontal centering.
    x_offset: i32,

    colors: [Color; 2], // colors to use for drawing, based on the palette provided when starting
}

// segment offset tables, pulled directly from the original game code
const MOD: i32 = 4;
const XMOD: [i32; 16] = [-MOD,-MOD,-MOD,  0,  0,  0,MOD,MOD,   0,-MOD,   0,MOD,MOD,  0,  0,  0];
const YMOD: [i32; 16] = [   0,   0,   0,MOD,MOD,MOD,  0,  0,-MOD,   0,-MOD,  0,  0,MOD,MOD,MOD];

impl PlacardRenderer {
    /// Returns true when the border animation is complete.
    pub fn is_done(&self) -> bool {
        self.block_index >= 17
    }

    /// Draw border segments for this frame onto any canvas.
    /// Advances the animation by `delta_ticks` worth of segments.
    /// Returns true if still animating, false if complete.
    pub fn draw_segments<T: RenderTarget>(&mut self, canvas: &mut Canvas<T>, delta_ticks: i32) -> bool {
        // loop to catch up if we are behind
        let count = delta_ticks * 3; // multiple iterations per frame, otherwise it's too slow
        let x_min = self.x_offset;
        let x_max = self.x_offset + 284;
        let y_min = 0i32;
        let y_max = 124i32;
        let cx = |x: i32| x.clamp(x_min, x_max);
        let cy = |y: i32| y.clamp(y_min, y_max);

        for _ in 0..count {
            // check if we're finished
            if self.block_index >= 17 {
                return false; // done
            }

            let dx = self.xorg + XMOD[self.segment_index];
            let dy = self.yorg + YMOD[self.segment_index];

            canvas.set_draw_color(self.colors[1]); // border color
            if self.block_index < 7 {
                    // Left
                canvas.draw_line(
                    Point::new(cx(self.xorg + self.x_offset), cy(self.yorg)),
                    Point::new(cx(dx + self.x_offset), cy(dy))
                ).unwrap();
                    // Right
                canvas.draw_line(
                    Point::new(cx(284 - self.xorg + self.x_offset), cy(124 - self.yorg)),
                    Point::new(cx(284 - dx + self.x_offset), cy(124 - dy))
                ).unwrap();
            }
                // Top
            canvas.draw_line(
                Point::new(cx(16 + self.yorg + self.x_offset), cy(12 - self.xorg)),
                Point::new(cx(16 + dy + self.x_offset), cy(12 - dx))
            ).unwrap();
                // Bottom
            canvas.draw_line(
                Point::new(cx(268 - self.yorg + self.x_offset), cy(112 + self.xorg)),
                Point::new(cx(268 - dy + self.x_offset), cy(112 + dx))
            ).unwrap();

            self.xorg = dx;
            self.yorg = dy;
            self.segment_index += 1;

            if self.segment_index >= 16 {
                self.segment_index = 0;
                self.block_index += 1;
            }
        }

        // if we're not finished, draw one last line segment in white
        // this will be redrawn in the next frame
        if self.block_index < 17 {
            let dx = self.xorg + XMOD[self.segment_index];
            let dy = self.yorg + YMOD[self.segment_index];

            canvas.set_draw_color(self.colors[0]); // white
            if self.block_index < 7 {
                    // Left
                canvas.draw_line(
                    Point::new(cx(self.xorg + self.x_offset), cy(self.yorg)),
                    Point::new(cx(dx + self.x_offset), cy(dy))
                ).unwrap();
                    // Right
                canvas.draw_line(
                    Point::new(cx(284 - self.xorg + self.x_offset), cy(124 - self.yorg)),
                    Point::new(cx(284 - dx + self.x_offset), cy(124 - dy))
                ).unwrap();
            }
                // Top
            canvas.draw_line(
                Point::new(cx(16 + self.yorg + self.x_offset), cy(12 - self.xorg)),
                Point::new(cx(16 + dy + self.x_offset), cy(12 - dx))
            ).unwrap();
                // Bottom
            canvas.draw_line(
                Point::new(cx(268 - self.yorg + self.x_offset), cy(112 + self.xorg)),
                Point::new(cx(268 - dy + self.x_offset), cy(112 + dx))
            ).unwrap();
        }

        true
    }
}

impl RenderTask for PlacardRenderer {
    fn update(&mut self, canvas: &mut Canvas<Window>, delta_ticks: i32, _area: Option<sdl2::rect::Rect>) -> bool {
        self.draw_segments(canvas, delta_ticks)
    }
}

pub fn start_placard_renderer(
    origin: &Point,
    palette: &Palette,
) -> PlacardRenderer {
    // pick colors from the palette
    let color1 = match palette.get_color(1) {
        Some(c) => c.to_color(),
        None => Color::RGB(255, 255, 255)
    };
    let color2 = match palette.get_color(24) {
        Some(c) => c.to_color(),
        None => Color::RGB(255, 0, 0)
    };

    PlacardRenderer {
        block_index: 0,
        segment_index: 0,
        xorg: 12, // border-local starting position
        yorg: 0,
        x_offset: origin.x,
        colors: [color1, color2]
    }
}

/**
 * Draw the placard border in one shot, for debugging purposes. Only red will be drawn here.
 */
pub fn draw_placard_border<'a, T: RenderTarget>(canvas: &mut Canvas<T>, palette: &Palette) {
    let color =
        match palette.get_color(24) {
            Some(c) => c.to_color(),
            None => Color::RGB(255, 0, 0)
        };
    canvas.set_draw_color(color);

    let mut xorg: i32 = 12;
    let mut yorg: i32 = 0;

    for ii in 0..=16 {
        for jj in 0..=15 {
            let dy = yorg + YMOD[jj];
            let dx = xorg + XMOD[jj];

            if ii < 7 {
                // vertical borders
                    // LEFT (drawn top to bottom)
                canvas.draw_line(
                    Point::new(xorg, yorg),
                    Point::new(dx, dy)
                ).unwrap();

                    // RIGHT (drawn bottom to top)
                canvas.draw_line(
                    Point::new(284 - xorg, 124 - yorg),
                    Point::new(284 - dx, 124 - dy)
                ).unwrap();
            }
            // TOP (drawn left to right)
            canvas.draw_line(
                Point::new(16 + yorg, 12 - xorg),
                Point::new(16 + dy, 12 - dx)
            ).unwrap();

            // BOTTOM (drawn right to left)
            canvas.draw_line(
                Point::new(268 - yorg, 112 + xorg),
                Point::new(268 - dy, 112 + dx)
            ).unwrap();

            xorg = dx;
            yorg = dy;
        }
    }
}
