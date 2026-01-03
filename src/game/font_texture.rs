
use crate::game::font::DiskFont;

use sdl2::rect::Rect;
use sdl2::render::{Canvas, RenderTarget, Texture};

use std::cell::RefCell;
use std::rc::Weak;

/*
 * Texture that contains the glyphs rendered from a DiskFont. The backing texture
 * could be shared with other components so we define bounds that can contain
 * the glyph map.
 */
pub struct FontTexture<'a> {
    font: DiskFont,

    // cached pixel arrays used to generate textures, so we don't have to repeat
    // expensive operations.
    pixels_32: Vec<u8>,
    // pixels_16 ... implement if/when needed

    // Shared backing texture
    texture: Weak<RefCell<Texture<'a>>>,
    bounds: Rect,
}

impl<'a> FontTexture<'a> {
    pub fn new(font: &DiskFont, bounds: &Rect, texture: Weak<RefCell<Texture<'a>>>) -> FontTexture<'a> {
        let mut ft = FontTexture {
            font: font.clone(),
            bounds: *bounds,
            pixels_32: Vec::new(),
            texture: texture.clone()
        };


        ft.init_texture();

        ft
    }

    pub fn name(&self) -> &String {
        &self.font.name
    }

    pub fn get_bounds(&self) -> &Rect {
        &self.bounds
    }

    pub fn get_font(&self) -> &DiskFont {
        &self.font
    }

    // Draw all the font glyphs into the provided texture within the rect provided
    fn init_texture(&mut self) {
        // build the pixel cache if needed
        if self.pixels_32.len() == 0 {
            for yy in 0 .. self.font.y_size {
                let offset = yy * self.font.modulo;
                for xx in 0 .. self.font.modulo {
                    let px = self.font.char_data[offset + xx];

                    // move to all four bytes
                    self.pixels_32.push(px);
                    self.pixels_32.push(px);
                    self.pixels_32.push(px);
                    self.pixels_32.push(px);
                }
            }
        }

        // we need a mutable borrow of the shared texture to draw the font glyphs into it
        if let Some(strong_texture) = self.texture.upgrade() {
            let mut result = strong_texture.try_borrow_mut();
            match result {
                Err(e) => {
                    println!("Error borrowing font texture for update: {}", e);
                    return;
                },
                Ok(ref mut tex) => {
                    let tex_info = tex.query();
                    // println!("texture info: {:?}", tex_info);
                    assert_eq!(tex_info.format.byte_size_per_pixel(), 4); // Enforce 32 bits per pixel

                    tex.update(self.bounds, self.pixels_32.as_slice(), self.font.modulo * 4).unwrap();
                }
            }
        }
    }

    // render a string to the given canvas
    // this does not handle newlines, it assumes the string will reside on a single line
    pub fn render_string<T: RenderTarget>(&self, s: &str, canvas: &mut Canvas<T>, x: i32, y: i32) {
        if let Some(strong_texture) = self.texture.upgrade() {
            let result = strong_texture.try_borrow();
            match result {
                Err(e) => {
                    println!("Error borrowing font texture for rendering: {}", e);
                    return;
                },
                Ok(ref tex) => {
                    self.render_string_internal(s, canvas, tex, x, y);
                }
            }
        }
    }

    /*
     * From the AmigaOS docs:
     * For each glyph the system renders, it has to do several things:
     *
     *      1. Get the value from the kerning table that corresponds to this glyph and begin the rendering that number of pixels to the right.
     *      2. Find this glyph's bitmap using the CharLoc table and blit the glyph to the rastport.
     *      3. If this is a proportional font, look in the spacing table and figure how many pixels to advance the rastport's horizontal position.
     *         For a monospaced font, the horizontal position advance comes from the TextFont's tf_XSize field.
     */
    fn render_string_internal<T: RenderTarget>(&self, s: &str, canvas: &mut Canvas<T>, texture: &Texture, x: i32, y: i32) {
        let cstr = s.as_bytes();

        // y coordinate is for the baseline of the font, so adjust for that
        let y_adjusted = y - self.font.baseline as i32;

        let mut glyph_rect = Rect::new(x, y_adjusted, 0, self.font.y_size as u32);
        for cc in cstr {
            if *cc >= self.font.lo_char && *cc <= self.font.hi_char {
                let cc_index = (cc - self.font.lo_char) as usize;
                let cc_loc = self.font.char_loc[cc_index];

                let kern: i32 = if self.font.is_proportional() { self.font.char_kern[cc_index] as i32 } else { 0 };
                let space: i32 = if self.font.is_proportional() { self.font.char_space[cc_index] as i32 } else { self.font.x_size as i32 };

                // Don't do anything for spaces, just skip ahead to the next coordinates
                if cc_loc.1 > 0 {
                    // grab glyph width and adjust glyph_rect, making sure to adjust the origin to our shared texture bounds
                    glyph_rect.set_width(cc_loc.1 as u32);
                    let src_rect = Rect::new(self.bounds.x + cc_loc.0 as i32 + kern, self.bounds.y, cc_loc.1 as u32, self.font.y_size as u32);

                    // copy the glyph
                    canvas.copy(texture, Some(src_rect), Some(glyph_rect)).unwrap();
                }

                // advance to the next glyph location
                glyph_rect.set_x(glyph_rect.x() + space);
            }
        }
    }
}
