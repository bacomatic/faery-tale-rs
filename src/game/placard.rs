
use crate::game::font::DiskFont;

use sdl2::render::Canvas;
use sdl2::render::RenderTarget;
use sdl2::render::Texture;

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
    name: String,
    lines: Vec<PlacardLine>
}

impl Placard {
    pub fn print(&self) {
        for line in &self.lines {
            // only use x here
            println!("{0: <1$}{2}", "", line.x/10, line.text);
        }
    }

    pub fn draw<T: RenderTarget>(&self, font: &DiskFont, canvas: &mut Canvas<T>, texture: &mut Texture) {
        for line in &self.lines {
            font.render_string(&line.text, canvas, texture, line.x as i32, line.y as i32);
        }
    }

    pub fn is_named(&self, name: &str) -> bool {
        self.name == name
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }
}
