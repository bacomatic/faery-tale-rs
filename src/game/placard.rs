
use crate::game::font_texture::FontTexture;
use crate::game::gfx::Palette;
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
    red_index: usize,   // current red block index
    white_index: usize, // current white block index

    segments: Vec<(Point, Point)>, // precomputed line segments
    colors: [Color; 2], // colors to use for drawing, based on the palette provided when starting
}

// segment offset tables, pulled directly from the original game code
const MOD: i32 = 4;
const XMOD: [i32; 16] = [-MOD,-MOD,-MOD,  0,  0,  0,MOD,MOD,   0,-MOD,   0,MOD,MOD,  0,  0,  0];
const YMOD: [i32; 16] = [   0,   0,   0,MOD,MOD,MOD,  0,  0,-MOD,   0,-MOD,  0,  0,MOD,MOD,MOD];

impl RenderTask for PlacardRenderer {
    fn update<'a>(&mut self, canvas: &mut Canvas<Window>, delta_ticks: i32, _area: Option<sdl2::rect::Rect>) -> bool {
        // loop to catch up if we are behind
        let mut first = self.red_index == 0 && self.white_index == 0;

        for _ in 0..delta_ticks {
            // first, make sure we haven't finished already, or have invalid indices
            if self.red_index >= self.segments.len() && self.white_index >= self.segments.len() {
                return false; // done
            }

            let mut line;

            if self.white_index < self.segments.len() {
                line = &self.segments[self.white_index];
                canvas.set_draw_color(self.colors[0]); // white
                canvas.draw_line(line.0, line.1).unwrap();
                self.white_index += 1;
            }

            // skip red on the first pass
            if first == false {
                line = &self.segments[self.red_index];
                canvas.set_draw_color(self.colors[1]); // red
                canvas.draw_line(line.0, line.1).unwrap();
                self.red_index += 1;
            }
            first = false;
        }

        true
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

    let segments = build_placard_segments(origin);
    PlacardRenderer {
        red_index: 0,
        white_index: 0,
        segments: segments,
        colors: [color1, color2]
    }
}

/**
 * Draw the placard border in one shot, for debugging purposes. Only red will be drawn here.
 */
pub fn draw_placard_border<'a, T: RenderTarget>(canvas: &mut Canvas<T>, palette: &Palette) {
    let segments = build_placard_segments(&Point::new(0, 0));
    let color =
        match palette.get_color(24) {
            Some(c) => c.to_color(),
            None => Color::RGB(255, 0, 0)
        };
    canvas.set_draw_color(color);

    for seg in &segments {
        let _ = canvas.draw_line(seg.0, seg.1).unwrap();
    }
}

fn build_placard_segments(origin: &Point) -> Vec<(Point,Point)> {
    let mut segments: Vec<(Point,Point)> = Vec::new();

    let mut xorg: i32 = origin.x + 12;
    let mut yorg: i32 = origin.y + 0;

    for ii in 0..=16 {
        for jj in 0..=15 {
            let dy = yorg + YMOD[jj];
            let dx = xorg + XMOD[jj];

            if ii < 7 {
                // vertical borders
                    // LEFT (drawn top to bottom)
                segments.push((Point::new(xorg, yorg), Point::new(dx, dy)));

                    // RIGHT (drawn bottom to top)
                segments.push((Point::new(284 - xorg, 124 - yorg), Point::new(284 - dx, 124 - dy)));
            }
             // TOP (drawn left to right)
            segments.push((Point::new(16 + yorg, 12 - xorg), Point::new(16 + dy, 12 - dx)));

            // BOTTOM (drawn right to left)
            segments.push((Point::new(268 - yorg, 112 + xorg), Point::new(268 - dy, 112 + dx)));

            xorg = dx;
            yorg = dy;
        }
    }

    segments
}
