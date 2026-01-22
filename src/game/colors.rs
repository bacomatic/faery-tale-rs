use sdl2::pixels::Color;
use serde::Deserialize;

use std::convert::From;

// Game graphics library

// type alias to be consistent with original code
#[derive(Deserialize, Debug, Clone, Copy)]
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
        RGB4 { color: c & 0x0FFF } // mask to 12 bits
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

#[derive(Deserialize, Debug)]
pub struct Palette {
    #[serde(deserialize_with = "deserialize_rgb4_vec")]
    pub colors: Vec<RGB4>
}

impl Palette {
    pub fn get_color(&self, index: usize) -> Option<&RGB4> {
        self.colors.get(index)
    }

    /**
     * Create a lookup table converting palette indices to RGBA32 colors, but only
     * to the specified depth.
     */
    pub fn to_rgba32_table(&self, depth: usize) -> Result<Vec<u32>, String> {
        if depth < 1 || depth > 5 {
            return Err("Palette depth must be 1 to 5 inclusive".to_string());
        }

        let mut table: Vec<u32> = Vec::with_capacity(1 << depth);
        let color_count = self.colors.len();
        for i in 0..(1 << depth) {
            if i < color_count {
                let c = &self.colors[i];
                let color: u32 =
                    ((c.r() as u32) << 24) |
                    ((c.g() as u32) << 16) |
                    ((c.b() as u32) << 8)  |
                    (0xFF);
                table.push(color);
            } else {
                table.push(0); // transparent
            }
        }
        Ok(table)
    }
}

fn deserialize_rgb4_vec<'de, D>(deserializer: D) -> Result<Vec<RGB4>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw_colors: Vec<u16> = Vec::deserialize(deserializer)?;
    Ok(raw_colors.into_iter().map(|c| RGB4::from(c)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb4_conversion() {
        let c1 = RGB4::from((0xAB, 0xCD, 0xEF));
        assert_eq!(c1.color, 0x0ACE); // conversion will truncate the lower nibble
    }

    #[test]
    fn test_rgb4_to_color() {
        let c1 = RGB4::from((0xAB, 0xCD, 0xEF));
        let color: Color = c1.to_color();
        assert_eq!(color.r, 0xAA);
        assert_eq!(color.g, 0xCC);
        assert_eq!(color.b, 0xEE);
    }

    #[test]
    fn test_palette_deserialization() {
        let toml_data = r#"
            colors = [0x0ACE, 0xA50, 0x0FFF, 0xABCD]
        "#;

        let palette: Palette = toml::from_str(toml_data).unwrap();
        assert_eq!(palette.colors.len(), 4);
        assert_eq!(palette.colors[0].color, 0x0ACE);
        assert_eq!(palette.colors[1].color, 0x0A50);
        assert_eq!(palette.colors[2].color, 0x0FFF);
        assert_eq!(palette.colors[3].color, 0x0BCD); // top nibble of 0xABCD is truncated

        // make sure colors are in the expected places
        assert_eq!(palette.colors[0].r(), 0xAA);
        assert_eq!(palette.colors[0].g(), 0xCC);
        assert_eq!(palette.colors[0].b(), 0xEE);
    }

    #[test]
    fn test_palette_to_rgba32_table() {
        let toml_data = r#"
            colors = [
                0x000, 0xFFF, 0xE00, 0xA00, 0xD80, 0xEC0, 0x390, 0x021,
                0xEEB, 0xEDA, 0xEEA, 0xCB8, 0xA95, 0x973, 0x840, 0x620,
                0xA52, 0xC74, 0xD96, 0xFCA, 0x449, 0x444, 0xDC9, 0x668,
                0x33F, 0x888, 0xA60, 0xAAF, 0xBBB, 0xCCF, 0xDDD, 0xEEE
            ]
        "#;
        let palette: Palette = toml::from_str(toml_data).unwrap();
        let four_table = palette.to_rgba32_table(2).unwrap();
        assert_eq!(four_table.len(), 4);
        assert_eq!(four_table[0], 0x000000FF);
        assert_eq!(four_table[1], 0xFFFFFFFF);
        assert_eq!(four_table[2], 0xEE0000FF);
        assert_eq!(four_table[3], 0xAA0000FF);

        let eight_table = palette.to_rgba32_table(3).unwrap();
        assert_eq!(eight_table.len(), 8);
        assert_eq!(eight_table[0], 0x000000FF);
        assert_eq!(eight_table[1], 0xFFFFFFFF);
        assert_eq!(eight_table[2], 0xEE0000FF);
        assert_eq!(eight_table[3], 0xAA0000FF);
        assert_eq!(eight_table[4], 0xDD8800FF);
        assert_eq!(eight_table[5], 0xEECC00FF);
        assert_eq!(eight_table[6], 0x339900FF);
        assert_eq!(eight_table[7], 0x002211FF);

        let sixteen_table = palette.to_rgba32_table(4).unwrap();
        assert_eq!(sixteen_table.len(), 16);
        assert_eq!(sixteen_table[0], 0x000000FF);
        assert_eq!(sixteen_table[1], 0xFFFFFFFF);
        assert_eq!(sixteen_table[2], 0xEE0000FF);
        assert_eq!(sixteen_table[3], 0xAA0000FF);
        assert_eq!(sixteen_table[4], 0xDD8800FF);
        assert_eq!(sixteen_table[5], 0xEECC00FF);
        assert_eq!(sixteen_table[6], 0x339900FF);
        assert_eq!(sixteen_table[7], 0x002211FF);
        assert_eq!(sixteen_table[8], 0xEEEEBBFF);
        assert_eq!(sixteen_table[9], 0xEEDDAAFF);
        assert_eq!(sixteen_table[10], 0xEEEEAAFF);
        assert_eq!(sixteen_table[11], 0xCCBB88FF);
        assert_eq!(sixteen_table[12], 0xAA9955FF);
        assert_eq!(sixteen_table[13], 0x997733FF);
        assert_eq!(sixteen_table[14], 0x884400FF);
        assert_eq!(sixteen_table[15], 0x662200FF);

        let thirtytwo_table = palette.to_rgba32_table(5).unwrap();
        assert_eq!(thirtytwo_table.len(), 32);
        assert_eq!(thirtytwo_table[0], 0x000000FF);
        assert_eq!(thirtytwo_table[1], 0xFFFFFFFF);
        assert_eq!(thirtytwo_table[2], 0xEE0000FF);
        assert_eq!(thirtytwo_table[3], 0xAA0000FF);
        assert_eq!(thirtytwo_table[4], 0xDD8800FF);
        assert_eq!(thirtytwo_table[5], 0xEECC00FF);
        assert_eq!(thirtytwo_table[6], 0x339900FF);
        assert_eq!(thirtytwo_table[7], 0x002211FF);
        assert_eq!(thirtytwo_table[8], 0xEEEEBBFF);
        assert_eq!(thirtytwo_table[9], 0xEEDDAAFF);
        assert_eq!(thirtytwo_table[10], 0xEEEEAAFF);
        assert_eq!(thirtytwo_table[11], 0xCCBB88FF);
        assert_eq!(thirtytwo_table[12], 0xAA9955FF);
        assert_eq!(thirtytwo_table[13], 0x997733FF);
        assert_eq!(thirtytwo_table[14], 0x884400FF);
        assert_eq!(thirtytwo_table[15], 0x662200FF);
        assert_eq!(thirtytwo_table[16], 0xAA5522FF);
        assert_eq!(thirtytwo_table[17], 0xCC7744FF);
        assert_eq!(thirtytwo_table[18], 0xDD9966FF);
        assert_eq!(thirtytwo_table[19], 0xFFCCAAFF);
        assert_eq!(thirtytwo_table[20], 0x444499FF);
        assert_eq!(thirtytwo_table[21], 0x444444FF);
        assert_eq!(thirtytwo_table[22], 0xDDCC99FF);
        assert_eq!(thirtytwo_table[23], 0x666688FF);
        assert_eq!(thirtytwo_table[24], 0x3333FFFF);
        assert_eq!(thirtytwo_table[25], 0x888888FF);
        assert_eq!(thirtytwo_table[26], 0xAA6600FF);
        assert_eq!(thirtytwo_table[27], 0xAAAAFFFF);
        assert_eq!(thirtytwo_table[28], 0xBBBBBBFF);
        assert_eq!(thirtytwo_table[29], 0xCCCCFFFF);
        assert_eq!(thirtytwo_table[30], 0xDDDDDDFF);
        assert_eq!(thirtytwo_table[31], 0xEEEEEEFF);
    }

}
