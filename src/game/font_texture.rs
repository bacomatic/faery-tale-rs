use crate::game::font::DiskFont;

use sdl2::rect::Rect;
use sdl2::render::{Canvas, RenderTarget, Texture};

use std::cell::RefCell;
use std::rc::Weak;

/*
 * Texture that contains the glyphs rendered from a DiskFont. The backing texture
 * could be shared with other components so we define bounds that can contain
 * the glyph map.
 *
 * A second stencil texture (owned, not shared) holds inverted-alpha pixel data:
 * glyph pixels are transparent, background pixels are opaque white. This enables
 * background-color rendering via two texture passes with color mod — matching
 * Amiga pen-B behavior with no rectangle drawing.
 */
pub struct FontTexture<'a> {
    font: DiskFont,

    // cached pixel arrays used to generate textures, so we don't have to repeat
    // expensive operations.
    pixels_32: Vec<u8>,
    // pixels_16 ... implement if/when needed

    // Shared backing texture (glyph atlas: glyph pixels opaque, bg transparent)
    texture: Weak<RefCell<Texture<'a>>>,
    bounds: Rect,

    // Stencil texture (inverted alpha: glyph pixels transparent, bg opaque white).
    // Wrapped in RefCell so set_color_mod can be called via &self.
    stencil: Option<RefCell<Texture<'a>>>,
}

impl<'a> FontTexture<'a> {
    pub fn new(
        font: &DiskFont,
        bounds: &Rect,
        texture: Weak<RefCell<Texture<'a>>>,
    ) -> FontTexture<'a> {
        let mut ft = FontTexture {
            font: font.clone(),
            bounds: *bounds,
            pixels_32: Vec::new(),
            texture: texture.clone(),
            stencil: None,
        };

        ft.init_texture();

        ft
    }

    /// Install a stencil texture for background-color rendering.
    ///
    /// The caller (render_resources) creates a same-size texture and passes it here.
    /// This method builds inverted-alpha pixel data from `pixels_32` and uploads it.
    /// Must be called after `new()` (which populates `pixels_32`).
    pub fn init_stencil(&mut self, mut stencil_tex: Texture<'a>) {
        // Build stencil pixels: R=G=B=0xFF, A=255-original_alpha.
        // In pixels_32, every pixel is stored as (px, px, px, px) where px is the
        // char_data byte. Glyph pixels have px=0xFF (opaque), bg pixels px=0 (transparent).
        // Invert: glyph → alpha=0 (transparent), bg → alpha=0xFF (opaque).
        let mut stencil_pixels: Vec<u8> = Vec::with_capacity(self.pixels_32.len());
        let mut i = 0;
        while i < self.pixels_32.len() {
            let alpha = self.pixels_32[i + 3]; // original alpha channel
            stencil_pixels.push(0xFF); // R
            stencil_pixels.push(0xFF); // G
            stencil_pixels.push(0xFF); // B
            stencil_pixels.push(255 - alpha); // A inverted
            i += 4;
        }
        stencil_tex.set_blend_mode(sdl2::render::BlendMode::Blend);
        // Stencil is a standalone texture — upload starts at (0,0), not the atlas offset.
        let stencil_rect = Rect::new(0, 0, self.bounds.width(), self.bounds.height());
        stencil_tex
            .update(stencil_rect, &stencil_pixels, self.font.modulo * 4)
            .unwrap();
        self.stencil = Some(RefCell::new(stencil_tex));
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

    /// Set the color modulation on the backing font texture.
    /// This tints all rendered glyphs by multiplying each pixel's RGB by (r/255, g/255, b/255).
    /// Call with (255, 255, 255) to reset to normal white rendering.
    pub fn set_color_mod(&self, r: u8, g: u8, b: u8) {
        if let Some(strong_texture) = self.texture.upgrade() {
            let mut tex = strong_texture.borrow_mut();
            tex.set_color_mod(r, g, b);
        }
    }

    // Draw all the font glyphs into the provided texture within the rect provided
    fn init_texture(&mut self) {
        // build the pixel cache if needed
        if self.pixels_32.len() == 0 {
            for yy in 0..self.font.y_size {
                let offset = yy * self.font.modulo;
                for xx in 0..self.font.modulo {
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
                }
                Ok(ref mut tex) => {
                    let tex_info = tex.query();
                    // println!("texture info: {:?}", tex_info);
                    assert_eq!(tex_info.format.byte_size_per_pixel(), 4); // Enforce 32 bits per pixel

                    tex.update(self.bounds, self.pixels_32.as_slice(), self.font.modulo * 4)
                        .unwrap();
                }
            }
        }
    }

    /// Calculate the pixel width of a rendered string.
    pub fn string_width(&self, s: &str) -> i32 {
        let cstr = s.as_bytes();
        let mut width: i32 = 0;
        for cc in cstr {
            if *cc >= self.font.lo_char && *cc <= self.font.hi_char {
                let cc_index = (cc - self.font.lo_char) as usize;
                let space: i32 = if self.font.is_proportional() {
                    self.font.char_space[cc_index] as i32
                } else {
                    self.font.x_size as i32
                };
                width += space;
            }
        }
        width
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
                }
                Ok(ref tex) => {
                    self.render_string_internal(s, canvas, tex, x, y);
                }
            }
        }
    }

    /// Render a string with a solid background color, matching Amiga JAM2 mode.
    ///
    /// Amiga `Text()` in JAM2 fills the entire character cell rectangle
    /// (full `space` width × `tf_YSize` height) with pen B, then draws glyph
    /// pixels in pen A.  This produces solid background rows above and below
    /// the glyph strokes (the empty rows within tf_YSize) and fills the full
    /// advance width—including spaces and inter-character gaps.
    ///
    /// Implementation: one filled SDL rect covering the whole string extent,
    /// then glyph rendering on top.
    pub fn render_string_with_bg<T: RenderTarget>(
        &self,
        s: &str,
        canvas: &mut Canvas<T>,
        x: i32,
        y: i32,
        bg: (u8, u8, u8),
        fg: (u8, u8, u8),
    ) {
        // Pass 1: filled rectangle for the full string extent (JAM2 background).
        let total_w = self.string_width(s);
        if total_w > 0 {
            let y_top = y - self.font.baseline as i32;
            let bg_rect = Rect::new(x, y_top, total_w as u32, self.font.y_size as u32);
            canvas.set_draw_color(sdl2::pixels::Color::RGB(bg.0, bg.1, bg.2));
            canvas.fill_rect(bg_rect).unwrap();
        }
        // Pass 2: glyph texture with fg color mod draws glyph pixels on top.
        if let Some(strong_texture) = self.texture.upgrade() {
            strong_texture.borrow_mut().set_color_mod(fg.0, fg.1, fg.2);
            if let Ok(ref tex) = strong_texture.try_borrow() {
                self.render_string_with_texture(s, canvas, tex, self.bounds, x, y);
            }
            // Reset to white so the caller doesn't have to.
            strong_texture.borrow_mut().set_color_mod(255, 255, 255);
        }
    }

    /// Render a string using an arbitrary texture (shared glyph or stencil).
    /// `src_origin` is the top-left offset into the texture where glyph data starts.
    fn render_string_with_texture<T: RenderTarget>(
        &self,
        s: &str,
        canvas: &mut Canvas<T>,
        texture: &Texture,
        src_origin: Rect,
        x: i32,
        y: i32,
    ) {
        let cstr = s.as_bytes();
        let y_adjusted = y - self.font.baseline as i32;
        let mut glyph_rect = Rect::new(x, y_adjusted, 0, self.font.y_size as u32);
        for cc in cstr {
            if *cc >= self.font.lo_char && *cc <= self.font.hi_char {
                let cc_index = (cc - self.font.lo_char) as usize;
                let cc_loc = self.font.char_loc[cc_index];
                let kern: i32 = if self.font.is_proportional() {
                    self.font.char_kern[cc_index] as i32
                } else {
                    0
                };
                let space: i32 = if self.font.is_proportional() {
                    self.font.char_space[cc_index] as i32
                } else {
                    self.font.x_size as i32
                };
                if cc_loc.1 > 0 {
                    glyph_rect.set_width(cc_loc.1 as u32);
                    let src_rect = Rect::new(
                        src_origin.x + cc_loc.0 as i32 + kern,
                        src_origin.y,
                        cc_loc.1 as u32,
                        self.font.y_size as u32,
                    );
                    canvas
                        .copy(texture, Some(src_rect), Some(glyph_rect))
                        .unwrap();
                }
                glyph_rect.set_x(glyph_rect.x() + space);
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
    fn render_string_internal<T: RenderTarget>(
        &self,
        s: &str,
        canvas: &mut Canvas<T>,
        texture: &Texture,
        x: i32,
        y: i32,
    ) {
        let cstr = s.as_bytes();

        // y coordinate is for the baseline of the font, so adjust for that
        let y_adjusted = y - self.font.baseline as i32;

        let mut glyph_rect = Rect::new(x, y_adjusted, 0, self.font.y_size as u32);
        for cc in cstr {
            if *cc >= self.font.lo_char && *cc <= self.font.hi_char {
                let cc_index = (cc - self.font.lo_char) as usize;
                let cc_loc = self.font.char_loc[cc_index];

                let kern: i32 = if self.font.is_proportional() {
                    self.font.char_kern[cc_index] as i32
                } else {
                    0
                };
                let space: i32 = if self.font.is_proportional() {
                    self.font.char_space[cc_index] as i32
                } else {
                    self.font.x_size as i32
                };

                // Don't do anything for spaces, just skip ahead to the next coordinates
                if cc_loc.1 > 0 {
                    // grab glyph width and adjust glyph_rect, making sure to adjust the origin to our shared texture bounds
                    glyph_rect.set_width(cc_loc.1 as u32);
                    let src_rect = Rect::new(
                        self.bounds.x + cc_loc.0 as i32 + kern,
                        self.bounds.y,
                        cc_loc.1 as u32,
                        self.font.y_size as u32,
                    );

                    // copy the glyph
                    canvas
                        .copy(texture, Some(src_rect), Some(glyph_rect))
                        .unwrap();
                }

                // advance to the next glyph location
                glyph_rect.set_x(glyph_rect.x() + space);
            }
        }
    }

    /// Render a string with glyphs stretched to 2× height (title screen style).
    pub fn render_string_hires<T: RenderTarget>(
        &self,
        s: &str,
        canvas: &mut Canvas<T>,
        x: i32,
        y: i32,
    ) {
        if let Some(strong_texture) = self.texture.upgrade() {
            let result = strong_texture.try_borrow();
            match result {
                Err(_) => return,
                Ok(ref tex) => {
                    self.render_string_hires_internal(s, canvas, tex, x, y);
                }
            }
        }
    }

    fn render_string_hires_internal<T: RenderTarget>(
        &self,
        s: &str,
        canvas: &mut Canvas<T>,
        texture: &Texture,
        x: i32,
        y: i32,
    ) {
        let cstr = s.as_bytes();
        let y_adjusted = y - self.font.baseline as i32;
        let dst_h = (self.font.y_size * 2) as u32;
        let mut dst_rect = Rect::new(x, y_adjusted, 0, dst_h);
        for cc in cstr {
            if *cc >= self.font.lo_char && *cc <= self.font.hi_char {
                let cc_index = (cc - self.font.lo_char) as usize;
                let cc_loc = self.font.char_loc[cc_index];
                let kern: i32 = if self.font.is_proportional() {
                    self.font.char_kern[cc_index] as i32
                } else {
                    0
                };
                let space: i32 = if self.font.is_proportional() {
                    self.font.char_space[cc_index] as i32
                } else {
                    self.font.x_size as i32
                };
                if cc_loc.1 > 0 {
                    dst_rect.set_width(cc_loc.1 as u32);
                    let src_rect = Rect::new(
                        self.bounds.x + cc_loc.0 as i32 + kern,
                        self.bounds.y,
                        cc_loc.1 as u32,
                        self.font.y_size as u32,
                    );
                    canvas
                        .copy(texture, Some(src_rect), Some(dst_rect))
                        .unwrap();
                }
                dst_rect.set_x(dst_rect.x() + space);
            }
        }
    }
}
