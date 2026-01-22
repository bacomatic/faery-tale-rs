
use crate::game::bitmap::BitMap;
use crate::game::iff_image::IffImage;
use crate::game::colors::Palette;

use sdl2::rect::Rect;
use sdl2::render::{Canvas, RenderTarget, Texture};

use std::cell::RefCell;
use std::rc::Weak;

/*
 * Texture that contains an IFF image. The backing texture
 * could be shared with other components so we define bounds
 * that can contain the image.
 */
pub struct ImageTexture<'a> {
    image: Option<&'a IffImage>,
    bitmap: Option<BitMap>,

    // bounds within the provided texture
    texture_bounds: Rect,

    // cached pixel arrays used to generate textures, so we don't have to repeat
    // expensive operations.
    pixels_32: Vec<u8>,
    stride: usize,

    // Shared backing texture
    texture: Weak<RefCell<Texture<'a>>>,
}

impl<'a> ImageTexture<'a> {
    pub fn new(image: &'a IffImage, bounds: &Rect, texture: Weak<RefCell<Texture<'a>>>) -> ImageTexture<'a> {
        let mut it = ImageTexture {
            image: Some(image),
            bitmap: None,
            texture_bounds: *bounds,
            pixels_32: Vec::new(),
            stride: 0,
            texture: texture.clone()
        };

        it.init_bitmap();
        // We need a palette to generate the pixel cache, which is provided later
        it
    }

    /**
     * Create an ImageTexture from a BitMap directly. The BitMap is cloned into the ImageTexture.
     */
    fn from_bitmap(bitmap: &BitMap, bounds: &Rect, texture: Weak<RefCell<Texture<'a>>>) -> ImageTexture<'a> {
        let it = ImageTexture {
            image: None,
            bitmap: Some(bitmap.clone()),
            texture_bounds: *bounds,
            pixels_32: Vec::new(),
            stride: 0,
            texture: texture.clone()
        };
        it
    }

    pub fn get_bounds(&self) -> &Rect {
        &self.texture_bounds
    }

    fn init_bitmap(&mut self) {
        if self.bitmap.is_none() {
            if let Some(ref img) = self.image {
                let bitmap = BitMap::with_interleaved_data(img.pixels.clone(), img.width, img.height, img.bitplanes, ((img.width + 15) / 16) * 2);
                self.bitmap = Some(bitmap);
            } else {
                println!("Error: ImageTexture has no image or bitmap to initialize from");
            }
        }
    }

    pub fn update(&mut self, palette: &Palette, key_color: Option<usize>) {
        // build the pixel cache if needed
        if self.pixels_32.len() == 0 {
            if let Some(ref bitmap) = self.bitmap {
                let result = bitmap.generate_rgb32(palette, key_color);
                if result.is_err() {
                    println!("Error generating RGB32 pixel data for ImageTexture: {}", result.err().unwrap());
                    return;
                }
                let (pixels, stride) = result.unwrap();
                self.pixels_32 = pixels;
                self.stride = stride;
            }
        } else {
            // update existing pixel cache in case palette changed
            if let Some(ref bitmap) = self.bitmap {
                let result = bitmap.update_rgb32(&mut self.pixels_32, self.stride, palette, key_color);
                if result.is_err() {
                    println!("Error updating RGB32 pixel data for ImageTexture: {}", result.err().unwrap());
                    return;
                }
            }
        }

        // we need a mutable borrow of the shared texture to draw the image into it
        if let Some(strong_texture) = self.texture.upgrade() {
            let mut texture = strong_texture.borrow_mut();
            texture.update(
                Some(self.texture_bounds),
                &self.pixels_32,
                self.stride
            ).unwrap();
        } else {
            println!("Error upgrading weak reference to shared texture in ImageTexture");
        }
    }

    pub fn draw<T: RenderTarget>(&self, canvas: &mut Canvas<T>, x: i32, y: i32) {
        // we need a mutable borrow of the shared texture to draw the image into it
        if let Some(strong_texture) = self.texture.upgrade() {
            let texture = strong_texture.borrow();
            let src_rect = self.texture_bounds.clone();

            let (width, height) = self.bitmap.as_ref().unwrap().get_size();
            let dest_rect = Rect::new(
                x,
                y,
                width as u32,
                height as u32
            );
            canvas.copy(&*texture, Some(src_rect), Some(dest_rect)).unwrap();
        } else {
            println!("Error upgrading weak reference to shared texture in ImageTexture");
        }
    }
}
