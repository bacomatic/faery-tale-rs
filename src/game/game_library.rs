use crate::game::{
    bitmap::BitMap,
    colors::Palette,
    cursor::CursorAsset,
    font::{DiskFont, FontAsset},
    iff_image::{IffImage, ImageAsset},
    placard::Placard,
};

use serde::Deserialize;

use std::{collections::HashMap, error::Error, fs, path::Path};

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
    pub answer: String,
}

/// A named point-of-interest on the world map (village, landmark, etc.).
/// Expandable: add new entries to [[locations]] in faery.toml as POIs are decoded.
#[derive(Deserialize, Debug, Clone)]
pub struct LocationConfig {
    pub name: String,
    pub x: u16,
    pub y: u16,
    pub region: u8,
}

/// Per-brother starting attributes (mirrors blist[] from fmain.c).
#[derive(Deserialize, Debug, Clone)]
pub struct BrotherConfig {
    pub name: String,
    pub brave: i16,
    pub luck: i16,
    pub kind: i16,
    pub wealth: i16,
    /// Location name where this brother spawns at start and on revive.
    pub spawn: String,
}

#[derive(Deserialize, Debug)]
pub struct ItemsConfig {
    pub costs: Vec<i32>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ObjectConfig {
    pub x: u16,
    pub y: u16,
    pub ob_id: u8,
    pub ob_stat: u8,
    /// Which region this object belongs to. 255 = global (ob_listg).
    pub region: u8,
}

#[derive(Deserialize, Debug)]
pub struct DoorConfig {
    pub src_region: u8,
    pub src_x: u16,
    pub src_y: u16,
    pub dst_region: u8,
    pub dst_x: u16,
    pub dst_y: u16,
    #[serde(default)]
    pub door_type: u8,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ZoneConfig {
    pub label: String,
    pub etype: u8,
    pub x1: u16,
    pub y1: u16,
    pub x2: u16,
    pub y2: u16,
    pub v1: u8,
    pub v2: u8,
    pub v3: u8,
}

#[derive(Debug, Deserialize)]
pub struct DiskConfig {
    pub adf: String,
    #[serde(default)]
    pub shadow_block: u32,
    #[serde(default)]
    pub shadow_count: u32,
}

#[derive(Debug, Deserialize)]
pub struct AudioConfig {
    pub instruments: String,
    pub songs: String,
    pub samples: String,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct RegionBlockConfig {
    pub id: u8,
    /// ADF block for sector data (64 blocks = 32768 bytes).
    /// Corresponds to `file_index[n].sector` in the original.
    pub sector_block: u32,
    /// ADF block for region map data (8 blocks = 4096 bytes).
    /// Corresponds to `file_index[n].region` in the original.
    pub map_block: u32,
    /// ADF block for first terra layer (1 block = 512 bytes).
    /// = TERRA_BLOCK (149) + file_index[n].terra1
    pub terra_block: u32,
    /// ADF block for second terra layer (1 block = 512 bytes).
    /// = TERRA_BLOCK (149) + file_index[n].terra2
    #[serde(default)]
    pub terra2_block: u32,
    /// ADF block numbers for each of the 4 tile image groups (40 blocks each, 5 planes × 8).
    /// Corresponds to `file_index[n].image[0..4]` in the original.
    pub image_blocks: Vec<u32>,
}

#[derive(Deserialize, Debug, Default)]
pub struct WorldConfig {
    #[serde(default)]
    pub region: Vec<RegionBlockConfig>,
}

#[derive(Deserialize, Debug, Default)]
pub struct SpritesConfig {
    pub cfile_block_count: u32,
    pub cfile_blocks: Vec<u32>,
}

#[derive(Deserialize, Debug, Default)]
pub struct NpcsConfig {
    pub cfile_start_block: u32,
}

/// A direction sub-region within the 48×24 compass bitmap.
#[derive(Deserialize, Debug, Clone)]
pub struct CompassRegion {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// The comptable: one entry per direction index (0-9).
#[derive(Deserialize, Debug, Clone)]
pub struct CompassTable {
    pub regions: Vec<CompassRegion>,
}

/// Compass rose bitmap data extracted from fsubs.asm.
/// `hinor` = normal (unhighlighted) compass; `hivar` = highlighted variant.
/// Both are single-bitplane images (plane 2 of the text viewport).
#[derive(Deserialize, Debug)]
pub struct CompassConfig {
    pub comptable: CompassTable,
    pub hinor: BitMap,
    pub hivar: BitMap,
}

/// Narrative strings from `narr.asm`, loaded from `faery.toml [narr]`.
#[derive(Deserialize, Debug, Default, Clone)]
pub struct NarrConfig {
    #[serde(default)]
    pub event_msg: Vec<String>,
    #[serde(default)]
    pub speeches: Vec<String>,
    #[serde(default)]
    pub place_msg: Vec<String>,
    #[serde(default)]
    pub inside_msg: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct GameLibrary {
    palettes: HashMap<String, Palette>,
    placards: HashMap<String, Placard>,
    fonts: HashMap<String, FontAsset>,
    images: HashMap<String, ImageAsset>,
    cursors: HashMap<String, CursorAsset>,
    copy_protect_junk: Vec<CopyProtectQuestion>,
    #[serde(default)]
    pub locations: Vec<LocationConfig>,
    #[serde(default)]
    pub brothers: Vec<BrotherConfig>,
    pub items: Option<ItemsConfig>,
    #[serde(default)]
    pub doors: Vec<DoorConfig>,
    #[serde(default)]
    pub zones: Vec<ZoneConfig>,
    pub disk: Option<DiskConfig>,
    pub audio: Option<AudioConfig>,
    pub world: Option<WorldConfig>,
    pub sprites: Option<SpritesConfig>,
    pub npcs: Option<NpcsConfig>,
    pub compass: Option<CompassConfig>,
    #[serde(default)]
    pub narr: NarrConfig,
    #[serde(default)]
    pub objects: Vec<ObjectConfig>,
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

    // copy protection
    pub fn get_copy_protect_questions(&self) -> &[CopyProtectQuestion] {
        &self.copy_protect_junk
    }

    pub fn get_copy_protect_count(&self) -> usize {
        self.copy_protect_junk.len()
    }

    // region block config
    pub fn find_region_config(&self, region_num: u8) -> Option<&RegionBlockConfig> {
        self.world
            .as_ref()?
            .region
            .iter()
            .find(|r| r.id == region_num)
    }

    // locations
    pub fn find_location(&self, name: &str) -> Option<&LocationConfig> {
        self.locations.iter().find(|l| l.name == name)
    }

    // brothers
    pub fn get_brother(&self, index: usize) -> Option<&BrotherConfig> {
        self.brothers.get(index)
    }

    // compass
    pub fn get_compass(&self) -> Option<&CompassConfig> {
        self.compass.as_ref()
    }

    /// Returns all objects for a given region (both global objects with region=255
    /// and region-specific objects).
    pub fn objects_for_region(&self, region: u8) -> Vec<&ObjectConfig> {
        self.objects
            .iter()
            .filter(|o| o.region == region || o.region == 255)
            .collect()
    }
}

pub fn load_game_library(lib_path: &Path) -> Result<GameLibrary, Box<dyn Error>> {
    let config = fs::read_to_string(lib_path)?;
    let mut game_lib = toml::from_str::<GameLibrary>(&config)?;

    // preload all file based assets
    for font_asset in game_lib.fonts.values_mut() {
        font_asset.load()?;
    }

    for image_asset in game_lib.images.values_mut() {
        image_asset.image = Some(IffImage::load_from_file(Path::new(&image_asset.path))?);
    }

    Ok(game_lib)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_library() -> GameLibrary {
        let config =
            fs::read_to_string("faery.toml").expect("faery.toml should exist in the project root");
        toml::from_str::<GameLibrary>(&config)
            .expect("faery.toml should deserialize into GameLibrary without errors")
    }

    /// Ensure faery.toml can be deserialized into GameLibrary without errors.
    /// This catches TOML syntax issues and schema mismatches early.
    #[test]
    fn faery_toml_parses() {
        load_library();
    }

    #[test]
    fn test_objects_for_region_filters_correctly() {
        let lib = load_library();
        let r3 = lib.objects_for_region(3);
        // Region 3 should have ground items (CHEST at 19298,16128 etc.) + globals
        assert!(!r3.is_empty(), "region 3 should have objects");
        let chest = r3
            .iter()
            .find(|o| o.ob_id == 15 && o.x == 19298 && o.y == 16128);
        assert!(
            chest.is_some(),
            "region 3 should have the starting chest at (19298, 16128)"
        );
    }

    #[test]
    fn test_global_objects_included_in_all_regions() {
        let lib = load_library();
        let globals: Vec<_> = lib.objects.iter().filter(|o| o.region == 255).collect();
        assert!(!globals.is_empty(), "should have global objects");
        // Globals should appear in region 3 query
        let r3 = lib.objects_for_region(3);
        let global_count = r3.iter().filter(|o| o.region == 255).count();
        assert_eq!(global_count, globals.len());
    }
}
