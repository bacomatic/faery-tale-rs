
use crate::game::font_texture::FontTexture;
use crate::game::placard::Placard;

use sdl2::render::Canvas;
use sdl2::render::RenderTarget;

use serde::Deserialize;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/*
 * GameLibrary contains all the information needed in the game.
 * Largely, this is strings and whatnot that we don't want to hard code.
 *
 * For now, this is implemented as a large JSON file containing data
 * extracted from the original source, with minor tweaks since we're
 * using modern systems with phat resources.
 */


#[derive(Deserialize, Debug)]
pub struct GameLibrary {
    placards: Vec<Placard>
}

impl GameLibrary {
    pub fn get_placard_count(&self) -> usize {
        self.placards.len()
    }

    pub fn print_placard_n(&self, index: usize) {
        if index < self.placards.len() {
            self.placards[index].print();
        }
    }

    pub fn print_placard(&self, name: &str) {
        let pi = self.find_placard(name);
        if pi.is_some() {
            self.print_placard_n(pi.unwrap());
        } else {
            println!("No placard named {name}");
        }
    }

    pub fn draw_placard_n<'a, T: RenderTarget>(&self, index: usize, font: &FontTexture<'a>, canvas: &mut Canvas<T>) {
        if index < self.placards.len() {
            self.placards[index].draw(font, canvas);
        }
    }

    pub fn draw_placard<'a, T: RenderTarget>(&self, name: &str, font: &FontTexture<'a>, canvas: &mut Canvas<T>) {
        let pi = self.find_placard(name);
        if pi.is_some() {
            self.draw_placard_n(pi.unwrap(), font, canvas);
        } else {
            println!("No placard named {name}");
        }
    }

    fn find_placard(&self, name: &str) -> Option<usize> {
        for pp in 0 .. self.placards.len() {
            if self.placards[pp].is_named(name) {
                return Some(pp)
            }
        }
        None
    }
}

pub fn load_game_library(lib_path: &Path) -> Result<GameLibrary, Box<dyn Error>> {
    let fp = File::open(lib_path)?;

    let game_lib = serde_json::from_reader(BufReader::new(fp))?;

    Ok(game_lib)
}
