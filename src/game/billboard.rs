
use crate::game::font_texture::FontTexture;

use sdl2::render::Canvas;
use sdl2::render::RenderTarget;

use serde::Deserialize;

/*
 * A page of text, possibly with a fancy swirly border.
 */

#[derive(Deserialize, Debug)]
pub struct BillboardLine {
    x: usize,
    y: usize,
    text: String
}

#[derive(Deserialize, Debug)]
pub struct Billboard {
    #[serde(default)]
    placard: bool,  // whether to draw a border around the text
    lines: Vec<BillboardLine>
}

impl Billboard {
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
