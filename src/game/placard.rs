
use crate::game::font_texture::FontTexture;
use crate::game::gfx::Palette;

use sdl2::render::Canvas;
use sdl2::render::RenderTarget;

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

pub fn draw_placard_border<'a, T: RenderTarget>(canvas: &mut Canvas<T>, palette: &Palette) {
    const MOD: i32 = 4;
    const XMOD: [i32; 16] = [-MOD,-MOD,-MOD,  0,  0,  0,MOD,MOD,   0,-MOD,   0,MOD,MOD,  0,  0,  0];
    const YMOD: [i32; 16] = [   0,   0,   0,MOD,MOD,MOD,  0,  0,-MOD,   0,-MOD,  0,  0,MOD,MOD,MOD];

    let mut xorg: i32 = 12;
    let mut yorg: i32 = 0;

    for ii in 0..=16 {
        for jj in 0..=15 {
            let dy = yorg + YMOD[jj];
            let dx = xorg + XMOD[jj];

		    for k in 0..4 {
                let color: usize;
                if k > 0 { color = 24 } else { color = 1 };
                canvas.set_draw_color(&palette.colors[color]);

                if ii < 7 {
                    // vertical borders
                        // LEFT (drawn top to bottom)
                    canvas.draw_line((xorg, yorg), (dx, dy)).unwrap();

                        // RIGHT (drawn bottom to top)
                    canvas.draw_line((284-xorg, 124-yorg), (284-dx, 124-dy)).unwrap();
                }
                // Vertical borders
                    // TOP (drawn left to right)
                canvas.draw_line((16+yorg, 12-xorg), (16+dy, 12-dx)).unwrap();

                    // BOTTOM (drawn right to left)
                canvas.draw_line((268-yorg, 112+xorg), (268-dy, 112+dx)).unwrap();
            }

            xorg = dx;
            yorg = dy;
	    }
    }
}
