use sdl2::rect::Point;
use serde::Deserialize;

use crate::game::bitmap::BitMap;

#[derive(Deserialize, Debug, Clone)]
pub struct Hotspot {
    pub x: usize,
    pub y: usize
}

impl From<&Point> for Hotspot {
    fn from(p: &Point) -> Hotspot {
        Hotspot { x: p.x as usize, y: p.y as usize }
    }
}

impl From<&Hotspot> for Point {
    fn from(value: &Hotspot) -> Self {
        Point::new(value.x as i32, value.y as i32)
    }
}

#[derive(Deserialize, Debug)]
pub struct CursorAsset {
    pub hotspot: Hotspot,
    pub bitmap: BitMap
}
