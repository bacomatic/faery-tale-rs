use sdl2::pixels::Color;

use std::convert::From;

// Game graphics library

// type alias to be consistent with original code
pub struct RGB4 {
    pub color: u16
}

impl From<(u8, u8, u8)> for RGB4 {
    fn from(c: (u8, u8, u8)) -> RGB4 {
        RGB4 {
            color:
              ((c.0 as u16 & 0xF0) << 4)
            |  (c.1 as u16 & 0xF0)
            | ((c.2 as u16 & 0xF0) >> 4)
        }
    }
}

impl From<[u8; 3]> for RGB4 {
    fn from(ca: [u8; 3]) -> RGB4 {
        RGB4 {color:
              ((ca[0] as u16 & 0xF0) << 4)
            |  (ca[1] as u16 & 0xF0)
            | ((ca[2] as u16 & 0xF0) >> 4)}
    }
}

impl From<u16> for RGB4 {
    fn from(c: u16) -> RGB4 {
        RGB4 { color: c }
    }
}

impl From<&Color> for RGB4 {
    fn from(c: &Color) -> RGB4 {
        RGB4 {
            color:
              ((c.r as u16 & 0xF0) << 4)
            |  (c.g as u16 & 0xF0)
            | ((c.b as u16 & 0xF0) >> 4)
        }
  }
}

impl From<&RGB4> for Color {
    fn from(c: &RGB4) -> Color {
        let rc = (c.color & 0xF00) >> 8;
        let gc = (c.color & 0xF0) >> 4;
        let bc = c.color & 0x0F;

        Color::RGB (
            (rc | (rc << 4)) as u8,
            (gc | (gc << 4)) as u8,
            (bc | (bc << 4)) as u8
        )
    }
}

impl RGB4 {
    pub fn to_color(&self) -> Color {
        let rc = (self.color & 0xF00) >> 8;
        let gc = (self.color & 0xF0) >> 4;
        let bc = self.color & 0x0F;

        Color::RGB (
            (rc | (rc << 4)) as u8,
            (gc | (gc << 4)) as u8,
            (bc | (bc << 4)) as u8
        )
    }

    pub fn r(&self) -> u8 {
        let rc = (self.color & 0xF00) >> 8;
        (rc | (rc << 4)) as u8
    }
    pub fn g(&self) -> u8 {
        let gc = (self.color & 0xF0) >> 4;
        (gc | (gc << 4)) as u8
    }
    pub fn b(&self) -> u8 {
        let bc = self.color & 0x0F;
        (bc | (bc << 4)) as u8
    }
}

pub type Palette4 = [RGB4; 4];
pub type Palette16 = [RGB4; 16];
pub type Palette32 = [RGB4; 32];
