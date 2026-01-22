
// Classes and utilities for working with bitmaps and bitplanes

use std::cell::RefCell;

use serde::Deserialize;

use crate::game::colors::Palette;

#[derive(Deserialize, Debug, Clone)]
pub struct BitMap {
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    pub stride: usize, // bytes per row
    pub planes: Vec<Vec<u8>>,

    // Optimization: cached index buffer
    #[serde(skip)]
    index_buffer: RefCell<Option<Vec<usize>>>
}

impl BitMap {
    /**
     * Create an empty (and invalid) BitMap.
     */
    pub fn new() -> BitMap {
        BitMap {
            width: 0,
            height: 0,
            depth: 0,
            stride: 0,
            planes: Vec::new(),
            index_buffer: RefCell::new(None)
        }
    }

    /**
     * Create a new BitMap from interleaved plane data.
     * Bitplane data is interleaved per row, e.g. for 2 planes:
     * Row 0: P0B0..BN, P1B0..BN
     * Row 1: P0B0..BN, P1B0..BN
     * etc.
     * This is the format used in IFF ILBM BODY chunks.
     *
     * Stride is the number of bytes per row per plane, padded to a WORD boundary. To get to the next row for the same plane,
     * skip depth * stride bytes.
     */
    pub fn with_interleaved_data(data: Vec<u8>, width: usize, height: usize, depth: usize, stride: usize) -> BitMap {
        let mut bitmap = BitMap {
            width: width,
            height: height,
            depth: depth,
            stride: stride,
            planes: Vec::with_capacity(depth),
            index_buffer: RefCell::new(None)
        };

        let plane_size = stride * height;

        // preallocate plane data
        for _ in 0..depth {
            bitmap.planes.push(Vec::with_capacity(plane_size));
        }

        let mut plane_index = 0;
        let mut offset = 0;
        for _ in 0..height*depth {
            // copy row by row to each plane
            let plane = &mut bitmap.planes[plane_index];
            let row_start = offset;
            let row_end = row_start + stride;

            plane.extend_from_slice(&data[row_start..row_end]);
            offset += stride;

            plane_index += 1;
            if plane_index >= depth {
                plane_index = 0;
            }
        }

        bitmap
    }

    /**
     * Create a new BitMap from raw plane data. Pixels are in contiguous
     * plane order. For interleaved use with_interleaved_data.
     */
    pub fn with_data(data: Vec<u8>, width: usize, height: usize, depth: usize, stride: usize) -> BitMap {
        let mut bitmap = BitMap {
            width: width,
            height: height,
            depth: depth,
            stride: stride,
            planes: Vec::with_capacity(depth),
            index_buffer: RefCell::new(None)
        };

        let plane_size = stride * height;
        for pp in 0..depth {
            let start = pp * plane_size;
            let end = start + plane_size;
            let plane_data = data[start..end].to_vec();
            bitmap.planes.push(plane_data);
        }

        bitmap
    }

    pub fn get_size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /**
     * Create a new BitMap with planes preallocated and ready to use.
     * The planes are zero initialized.
     * Size is not really constrained, depth must be 1 to 5. I'm not supporting EHB or HAM modes.
     * plane stride is calculated according to the AmigaOS RASSIZE macro, which pads to the nearest WORD boundary
     */
    pub fn build(width: usize, height: usize, depth: usize) -> Result<BitMap, String> {
        // depth must be 1..5
        if depth < 1 || depth > 5 {
            return Err("BitMap depth must be 1 to 5 inclusive".to_string());
        }

        let mut bitmap = BitMap {
            width: width,
            height: height,
            depth: depth,
            stride: ((width + 15) >> 3) & !1_usize,
            planes: Vec::with_capacity(depth),
            index_buffer: RefCell::new(None)
        };

        let plane_size = bitmap.stride * height;
        for _ in 0..depth {
            let mut new_plane = Vec::new();
            new_plane.resize(plane_size, 0);
            bitmap.planes.push(new_plane);
        }

        Ok(bitmap)
    }

    /**
     * Convert a BitMap into an RGB32 pixel buffer using the provided color palette.
     * If set, the key_color index in the palette will be treated as transparent
     * and set to transparent black in the output buffer.
     *
     * @return tuple containing a u8 vector and the byte stride for the pixel buffer
     */
    pub fn generate_rgb32(&self, colors: &Palette, key_color: Option<usize>) -> Result<(Vec<u8>, usize), String> {
        // start with a clear pixel buffer
        let pixel_count = self.width * self.height;
        let mut pixels: Vec<u8> = Vec::with_capacity(pixel_count * 4);
        pixels.resize(pixel_count * 4, 0);

        self.update_rgb32(&mut pixels, self.width * 4, colors, key_color)?;
        Ok((pixels, self.width * 4))
    }

    pub fn update_rgb32(&self, pixels: &mut Vec<u8>, stride: usize, colors: &Palette, key_color: Option<usize>) -> Result<(), String> {
        let pixel_count = self.width * self.height;
        if pixels.len() < pixel_count * 4 {
            return Err("Provided pixel buffer is too small for BitMap dimensions".to_string());
        }

        // build a LUT for the palette indices to RGBA32 colors
        let mut color_table = colors.to_rgba32_table(self.depth)?;
        // if there's a key color, set that entry to transparent black
        if let Some(key_index) = key_color {
            if key_index < color_table.len() {
                color_table[key_index] = 0x00000000;
            }
        }

        // optimization: reverse iterate over the planes and build an index buffer directly from plane data
        if self.index_buffer.borrow().is_none() {
            // build index buffer
            let mut index_buffer: Vec<usize> = Vec::with_capacity(self.width * self.height);
            for yy in 0..self.height {
                for xx in 0..self.width {
                    let mut pixel_index: usize = 0;
                    for pp in 0..self.depth {
                        let plane = &self.planes[pp];
                        let byte_index = yy * self.stride + (xx >> 3);
                        let bit_index = 7 - (xx & 0x07);
                        let bit = (plane[byte_index] >> bit_index) & 0x01;
                        pixel_index |= (bit as usize) << pp;
                    }
                    index_buffer.push(pixel_index);
                }
            }
            // cache it
            *self.index_buffer.borrow_mut() = Some(index_buffer);
        }

        // now build the pixel buffer from the index buffer and color table
        let index_buffer = self.index_buffer.borrow();
        let indices = index_buffer.as_ref().unwrap();

        // since stride may not match (esp if we're copying into a larger pixmap), we have to write row by row
        for row in 0..self.height {
            let row_start = row * self.width;
            let pixel_row_start = row * stride;
            for col in 0..self.width {
                let color_index = indices[row_start + col];
                let color = color_table[color_index];
                let pixel_offset = pixel_row_start + col * 4;
                pixels[pixel_offset + 0] = ((color >> 24) & 0xFF) as u8; // R
                pixels[pixel_offset + 1] = ((color >> 16) & 0xFF) as u8; // G
                pixels[pixel_offset + 2] = ((color >> 8) & 0xFF) as u8;  // B
                pixels[pixel_offset + 3] = (color & 0xFF) as u8;         // A
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::game::colors::RGB4;

    use super::*;

    fn build_interleaved_test_bitmap() -> BitMap {
        let width = 16;
        let height = 16;
        let depth = 2;
        let stride = 2; // 16 pixels = 2 bytes per row
        let mut data: Vec<u8> = Vec::new();

        // Plane 0: all zeros
        for yy in 0..height {
            for _ in 0..stride {
                data.push(0);
            }
            // Plane 1: checkerboard pattern
            for xx in 0..stride {
                let byte = if (yy + xx * 8) % 2 == 0 { 0xAA } else { 0x55 };
                data.push(byte);
            }
        }

        BitMap::with_interleaved_data(data, width, height, depth, stride)
    }

    /**
     * Build a simple 16x16 2-bit bitmap for testing
     * Plane 0: all zeros
     * Plane 1: checkerboard pattern
     *
     * Resulting pixel indices:
     * 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1
     *
     */
    fn build_test_bitmap() -> BitMap {
        let width = 16;
        let height = 16;
        let depth = 2;
        let stride = 2; // 16 pixels = 2 bytes per row
        let plane_size = stride * height;
        let mut data: Vec<u8> = Vec::new();

        // Plane 0: all zeros
        data.resize(plane_size, 0);
        // Plane 1: checkerboard pattern
        for yy in 0..height {
            for xx in 0..stride {
                let byte = if (yy + xx * 8) % 2 == 0 { 0xAA } else { 0x55 };
                data.push(byte);
            }
        }

        BitMap::with_data(data, width, height, depth, stride)
    }

    #[test]
    fn test_bitmap_creation() {
        let bitmap = BitMap::build(320, 200, 5).unwrap();
        assert_eq!(bitmap.width, 320);
        assert_eq!(bitmap.height, 200);
        assert_eq!(bitmap.depth, 5);
        assert_eq!(bitmap.stride, 40); // (320 + 15) >> 3 = 40
        assert_eq!(bitmap.planes.len(), 5);
        for plane in bitmap.planes {
            assert_eq!(plane.len(), 8000); // 40 * 200
        }
    }

    #[test]
    fn test_bitmap_invalid_depth() {
        let mut result = BitMap::build(320, 200, 6);
        assert!(result.is_err());

        result = BitMap::build(320, 200, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_bitmap_with_data() {
        let bitmap = build_test_bitmap();
        assert_eq!(bitmap.width, 16);
        assert_eq!(bitmap.height, 16);
        assert_eq!(bitmap.depth, 2);
        assert_eq!(bitmap.stride, 2);
        assert_eq!(bitmap.planes.len(), 2);
        let plane_size = bitmap.stride * bitmap.height;
        assert_eq!(bitmap.planes[0].len(), plane_size);
        assert_eq!(bitmap.planes[1].len(), plane_size);
    }

    #[test]
    fn test_generate_rgb32() {
        let bitmap = build_test_bitmap();
        let mut palette = Palette { colors: Vec::new() };
        palette.colors.push(RGB4::from(0x006)); // blue
        palette.colors.push(RGB4::from(0xFFF)); // white
        palette.colors.push(RGB4::from(0x390)); // green
        palette.colors.push(RGB4::from(0x000)); // black

        let (pixels, stride) = bitmap.generate_rgb32(&palette, None).unwrap();
        assert_eq!(stride, 64); // 16 pixels * 4 bytes
        assert_eq!(pixels.len(), 16 * 16 * 4);

        // Check a few pixel values
        // Top-left pixel (0,0) should be color index 2 (green)
        assert_eq!(pixels[0], 0x33); // R
        assert_eq!(pixels[1], 0x99); // G
        assert_eq!(pixels[2], 0x00); // B
        assert_eq!(pixels[3], 0xFF); // A
        // Pixel (1,0) should be color index 0 (blue)
        assert_eq!(pixels[4], 0x00); // R
        assert_eq!(pixels[5], 0x00); // G
        assert_eq!(pixels[6], 0x66); // B
        assert_eq!(pixels[7], 0xFF); // A
        // Pixel (0,1) should be color index 0 (blue)
        assert_eq!(pixels[64], 0x00); // R
        assert_eq!(pixels[65], 0x00); // G
        assert_eq!(pixels[66], 0x66); // B
        assert_eq!(pixels[67], 0xFF); // A
        // Pixel (1,1) should be color index 2 (green)
        assert_eq!(pixels[68], 0x33); // R
        assert_eq!(pixels[69], 0x99); // G
        assert_eq!(pixels[70], 0x00); // B
        assert_eq!(pixels[71], 0xFF); // A
    }

    #[test]
    fn test_generate_rgb32_with_key_color() {
        let bitmap = build_test_bitmap();
        let mut palette = Palette { colors: Vec::new() };
        palette.colors.push(RGB4::from(0x006)); // blue
        palette.colors.push(RGB4::from(0xFFF)); // white
        palette.colors.push(RGB4::from(0x390)); // green
        palette.colors.push(RGB4::from(0x000)); // black
        let key_color_index = 2; // green
        let (pixels, stride) = bitmap.generate_rgb32(&palette, Some(key_color_index)).unwrap();
        assert_eq!(stride, 64); // 16 pixels * 4 bytes
        assert_eq!(pixels.len(), 16 * 16 * 4);

        // Check a few pixel values
        // Top-left pixel (0,0) should be transparent black
        assert_eq!(pixels[0], 0x00); // R
        assert_eq!(pixels[1], 0x00); // G
        assert_eq!(pixels[2], 0x00); // B
        assert_eq!(pixels[3], 0x00); // A
        // Pixel (1,0) should be color index 0 (blue)
        assert_eq!(pixels[4], 0x00); // R
        assert_eq!(pixels[5], 0x00); // G
        assert_eq!(pixels[6], 0x66); // B
        assert_eq!(pixels[7], 0xFF); // A
    }

    #[test]
    fn test_generate_rgb32_interleaved() {
        let bitmap = build_interleaved_test_bitmap();
        let mut palette = Palette { colors: Vec::new() };
        palette.colors.push(RGB4::from(0x006)); // blue
        palette.colors.push(RGB4::from(0xFFF)); // white
        palette.colors.push(RGB4::from(0x390)); // green
        palette.colors.push(RGB4::from(0x000)); // black

        let (pixels, stride) = bitmap.generate_rgb32(&palette, None).unwrap();
        assert_eq!(stride, 64); // 16 pixels * 4 bytes
        assert_eq!(pixels.len(), 16 * 16 * 4);

        // Check a few pixel values
        // Top-left pixel (0,0) should be color index 2 (green)
        assert_eq!(pixels[0], 0x33); // R
        assert_eq!(pixels[1], 0x99); // G
        assert_eq!(pixels[2], 0x00); // B
        assert_eq!(pixels[3], 0xFF); // A
        // Pixel (1,0) should be color index 0 (blue)
        assert_eq!(pixels[4], 0x00); // R
        assert_eq!(pixels[5], 0x00); // G
        assert_eq!(pixels[6], 0x66); // B
        assert_eq!(pixels[7], 0xFF); // A
    }

}
