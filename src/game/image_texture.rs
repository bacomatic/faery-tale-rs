use crate::game::bitmap::BitMap;
use crate::game::colors::Palette;
use crate::game::iff_image::IffImage;

use sdl2::rect::Rect;
use sdl2::render::{Canvas, RenderTarget, Texture};

use std::cell::RefCell;
use std::rc::Weak;

/// An image view inside a shared SDL2 texture atlas.
///
/// `ImageTexture` converts an [`IffImage`] into a planar [`BitMap`] at
/// construction time and from that point on is independent of the source
/// asset — the `GameLibrary` lifetime does **not** propagate here.
///
/// The `'tex` lifetime tracks the [`sdl2::render::TextureCreator`] that
/// allocated the backing atlas texture.
pub struct ImageTexture<'tex> {
    bitmap: BitMap,

    // Location of this image within the shared atlas texture.
    texture_bounds: Rect,

    // Cached RGBA32 pixel buffer; populated on first `update()` call.
    pixels_32: Vec<u8>,
    stride: usize,

    // Weak reference to the shared backing texture (owned by the atlas).
    texture: Weak<RefCell<Texture<'tex>>>,
}

impl<'tex> ImageTexture<'tex> {
    /// Build an `ImageTexture` from an `IffImage`.
    ///
    /// The planar pixel data is decoded into a [`BitMap`] immediately;
    /// after this call the `image` reference is no longer needed.
    pub fn new(
        image: &IffImage,
        bounds: &Rect,
        texture: Weak<RefCell<Texture<'tex>>>,
    ) -> ImageTexture<'tex> {
        let row_bytes = ((image.width + 15) / 16) * 2;
        let bitmap = BitMap::with_interleaved_data(
            image.pixels.clone(),
            image.width,
            image.height,
            image.bitplanes,
            row_bytes,
        );
        ImageTexture {
            bitmap,
            texture_bounds: *bounds,
            pixels_32: Vec::new(),
            stride: 0,
            texture,
        }
    }

    pub fn get_bounds(&self) -> &Rect {
        &self.texture_bounds
    }

    pub fn update(&mut self, palette: &Palette, key_color: Option<usize>) {
        // build the pixel cache if needed
        if self.pixels_32.is_empty() {
            let result = self.bitmap.generate_rgb32(palette, key_color);
            if result.is_err() {
                println!(
                    "Error generating RGB32 pixel data for ImageTexture: {}",
                    result.err().unwrap()
                );
                return;
            }
            let (pixels, stride) = result.unwrap();
            self.pixels_32 = pixels;
            self.stride = stride;
        } else {
            // update existing pixel cache in case palette changed
            let result =
                self.bitmap
                    .update_rgb32(&mut self.pixels_32, self.stride, palette, key_color);
            if result.is_err() {
                println!(
                    "Error updating RGB32 pixel data for ImageTexture: {}",
                    result.err().unwrap()
                );
                return;
            }
        }

        if let Some(strong_texture) = self.texture.upgrade() {
            let mut texture = strong_texture.borrow_mut();
            texture
                .update(Some(self.texture_bounds), &self.pixels_32, self.stride)
                .unwrap();
        } else {
            println!("Error upgrading weak reference to shared texture in ImageTexture");
        }
    }

    pub fn draw<T: RenderTarget>(&self, canvas: &mut Canvas<T>, x: i32, y: i32) {
        if let Some(strong_texture) = self.texture.upgrade() {
            let texture = strong_texture.borrow();
            let src_rect = self.texture_bounds;
            let (width, height) = self.bitmap.get_size();
            let dest_rect = Rect::new(x, y, width as u32, height as u32);
            canvas
                .copy(&*texture, Some(src_rect), Some(dest_rect))
                .unwrap();
        } else {
            println!("Error upgrading weak reference to shared texture in ImageTexture");
        }
    }

    /// Draw the image scaled to fill `dst` (no aspect-ratio enforcement).
    pub fn draw_scaled<T: RenderTarget>(&self, canvas: &mut Canvas<T>, dst: Rect) {
        if let Some(strong_texture) = self.texture.upgrade() {
            let texture = strong_texture.borrow();
            canvas
                .copy(&*texture, Some(self.texture_bounds), Some(dst))
                .unwrap();
        } else {
            println!(
                "Error upgrading weak reference to shared texture in ImageTexture::draw_scaled"
            );
        }
    }

    /// Draw a sub-region of the image to the canvas at the specified position.
    /// `region` is in image-local coordinates (relative to the image's own top-left).
    pub fn draw_region<T: RenderTarget>(
        &self,
        canvas: &mut Canvas<T>,
        region: Rect,
        x: i32,
        y: i32,
    ) {
        if let Some(strong_texture) = self.texture.upgrade() {
            let texture = strong_texture.borrow();
            let src_rect = Rect::new(
                self.texture_bounds.x() + region.x(),
                self.texture_bounds.y() + region.y(),
                region.width(),
                region.height(),
            );
            let dest_rect = Rect::new(x, y, region.width(), region.height());
            canvas
                .copy(&*texture, Some(src_rect), Some(dest_rect))
                .unwrap();
        } else {
            println!(
                "Error upgrading weak reference to shared texture in ImageTexture::draw_region"
            );
        }
    }
}
