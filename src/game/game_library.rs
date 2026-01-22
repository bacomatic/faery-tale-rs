
use crate::game::{
    placard::Placard, cursor::CursorAsset, font::{DiskFont, FontAsset}, colors::Palette, iff_image::{IffImage, ImageAsset}
};

use serde::Deserialize;

use std::{
    collections::HashMap,
    error::Error,
    fs,
    path::Path
};

/*
 * GameLibrary contains all the information needed in the game.
 * Largely, this is strings and whatnot that we don't want to hard code.
 *
 * For now, this is implemented as a large TOML file containing data
 * extracted from the original source, with minor tweaks since we're
 * using modern systems with phat resources. Some assets are referenced
 * by path and loaded from files. All assets are loaded at startup.
 */

#[derive(Deserialize, Debug)]
pub struct CopyProtectQuestion {
    pub question: String,
    pub answer: String
}

#[derive(Deserialize, Debug)]
pub struct GameLibrary {
    palettes: HashMap<String, Palette>,
    placards: HashMap<String, Placard>,
    fonts: HashMap<String, FontAsset>,
    images: HashMap<String, ImageAsset>,
    cursors: HashMap<String, CursorAsset>,
    copy_protect_junk: Vec<CopyProtectQuestion>
}

impl GameLibrary {
    // images
    pub fn get_image_count(&self) -> usize {
        self.images.len()
    }

    pub fn get_image_names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for (name, _) in &self.images {
            names.push(name.clone());
        }
        names
    }

    pub fn get_image(&self, index: usize) -> Option<&ImageAsset> {
        if index >= self.images.len() {
            return None;
        }
        let key = self.images.keys().nth(index).unwrap();
        self.images.get(key)
    }

    pub fn find_image(&self, name: &str) -> Option<&ImageAsset> {
        self.images.get(name)
    }

    // color palettes
    pub fn get_palette_count(&self) -> usize {
        self.palettes.len()
    }

    pub fn find_palette(&self, name: &str) -> Option<&Palette> {
        self.palettes.get(name)
    }

    // placards
    pub fn get_placard_count(&self) -> usize {
        self.placards.len()
    }

    pub fn get_placard_names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for (name, _) in &self.placards {
            names.push(name.clone());
        }
        names
    }

    pub fn find_placard(&self, name: &str) -> Option<&Placard> {
        self.placards.get(name)
    }

    // fonts
    pub fn get_font_count(&self) -> usize {
        self.fonts.len()
    }

    pub fn get_font_names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for (name, _) in &self.fonts {
            names.push(name.clone());
        }
        names
    }

    pub fn get_font_sizes(&self, name: &str) -> Option<Vec<usize>> {
        let font = self.fonts.get(name).unwrap();
        Some(font.get_sizes())
    }

    pub fn find_font(&self, name: &str, size: usize) -> Option<&DiskFont> {
        let font = self.fonts.get(name).unwrap();
        font.get_font(size)
    }

    pub fn get_cursor(&self, name: &str) -> Option<&CursorAsset> {
        self.cursors.get(name)
    }

}

pub fn load_game_library(lib_path: &Path) -> Result<GameLibrary, Box<dyn Error>> {
    let config = fs::read_to_string(lib_path)?;
    let mut game_lib = toml::from_str::<GameLibrary>(&config)?;

    // preload all file based assets
    // FIXME: implement file cache to avoid reloading same file multiple times
    for font_asset in game_lib.fonts.values_mut() {
        font_asset.load()?;
    }

    for image_asset in game_lib.images.values_mut() {
        image_asset.image = Some(IffImage::load_from_file(Path::new(&image_asset.path))?);
    }

    Ok(game_lib)
}
