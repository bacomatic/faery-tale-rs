
use crate::game::{
    billboard::Billboard, cursor::CursorAsset, font::{DiskFont, FontAsset}, gfx::Palette, iff_image::{IffImage, ImageAsset}
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
pub struct GameLibrary {
    palettes: HashMap<String, Palette>,
    billboards: HashMap<String, Billboard>,
    fonts: HashMap<String, FontAsset>,
    images: HashMap<String, ImageAsset>,
    cursors: HashMap<String, CursorAsset>
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

    // billboards
    pub fn get_billboard_count(&self) -> usize {
        self.billboards.len()
    }

    pub fn get_billboard_names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for (name, _) in &self.billboards {
            names.push(name.clone());
        }
        names
    }

    pub fn find_billboard(&self, name: &str) -> Option<&Billboard> {
        self.billboards.get(name)
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
