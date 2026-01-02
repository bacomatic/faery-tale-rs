
// Classes and utilities for working with bitmaps and bitplanes

use std::cell::RefCell;

use serde::Deserialize;

use crate::game::gfx::Palette;

#[derive(Deserialize, Debug)]
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
        for pp in 0..depth {
            let mut plane_data = Vec::with_capacity(plane_size);
            for yy in 0..height {
                let row_start = yy * stride * depth;
                let plane_row_start = row_start + pp;
                for xx in 0..stride {
                    let byte = data[plane_row_start + xx * depth];
                    plane_data.push(byte);
                }
            }
            bitmap.planes.push(plane_data);
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
        for pp in 0..depth {
            let mut new_plane = Vec::new();
            new_plane.resize(plane_size, 0);

            bitmap.planes[pp] = new_plane;
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
        let index_buffer = index_buffer.as_ref().unwrap();
        for i in 0..(self.width * self.height) {
            let color_index = index_buffer[i];
            let color = color_table[color_index];
            let pixel_offset = i * 4;
            pixels[pixel_offset + 0] = ((color >> 24) & 0xFF) as u8; // R
            pixels[pixel_offset + 1] = ((color >> 16) & 0xFF) as u8; // G
            pixels[pixel_offset + 2] = ((color >> 8) & 0xFF) as u8;  // B
            pixels[pixel_offset + 3] = (color & 0xFF) as u8;         // A
        }

        Ok((pixels, self.width * 4))
    }
}
