/// SDL2 rendering resources for the game.
///
/// [`RenderResources`] owns every SDL2 texture that the game creates at
/// startup — the shared font atlas, the shared image atlas, and the two
/// off-screen render targets — keeping them decoupled from the raw asset
/// data in [`GameLibrary`].
///
/// # Lifetime
///
/// The single `'tex` lifetime parameter ties all owned textures to the
/// [`sdl2::render::TextureCreator`] that allocated them.  As long as the
/// `TextureCreator` lives, `RenderResources` is valid.
///
/// # Usage
///
/// ```no_run
/// let tex_maker = canvas.texture_creator();
/// let sys_palette = game_lib.find_palette("introcolors").unwrap();
/// let mut rr = RenderResources::build(&tex_maker, &game_lib, sys_palette);
///
/// // each frame:
/// let mut resources = rr.prepare(&mut scratch_tex, audio.as_ref());
/// scene.update(&mut canvas, &mut play_tex, delta, &game_lib, &mut resources);
/// ```

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;

use crate::game::audio::AudioSystem;
use crate::game::colors::Palette;
use crate::game::font_texture::FontTexture;
use crate::game::game_library::GameLibrary;
use crate::game::image_texture::ImageTexture;
use crate::game::scene::SceneResources;

// Atlas dimensions — large enough to hold all game images in a single texture.
const IMAGE_ATLAS_W: u32 = 4096;
const IMAGE_ATLAS_H: u32 = 4096;

pub struct RenderResources<'tex> {
    // --- Font atlas ---
    // The backing texture is kept alive by `Rc`; `FontTexture` holds a `Weak`.
    _font_backing: Rc<RefCell<Texture<'tex>>>,
    pub amber: FontTexture<'tex>,
    pub topaz: FontTexture<'tex>,

    // --- Image atlas ---
    _image_backing: Rc<RefCell<Texture<'tex>>>,
    images: Vec<ImageTexture<'tex>>,
    image_map: HashMap<String, usize>,
}

impl<'tex> RenderResources<'tex> {
    /// Build all SDL2 rendering resources from the loaded game library.
    ///
    /// Fonts and images are uploaded to their respective atlas textures
    /// immediately; the `GameLibrary` reference is not retained.
    pub fn build(
        tex_maker: &'tex TextureCreator<WindowContext>,
        game_lib: &GameLibrary,
        sys_palette: &Palette,
    ) -> Self {
        // ── Font atlas ────────────────────────────────────────────────────
        let amber_font = game_lib.find_font("amber", 9).unwrap();
        let topaz_font = game_lib.find_font("topaz", 8).unwrap();

        let amber_bounds = amber_font.get_font_bounds();
        // leave a small gap between the two sub-atlases
        let mut topaz_bounds = topaz_font.get_font_bounds();
        topaz_bounds.offset(0, amber_bounds.height() as i32 + 4);
        let atlas_bounds = amber_bounds.union(topaz_bounds);

        let mut font_tex = tex_maker
            .create_texture_static(
                Some(PixelFormatEnum::RGBA32),
                atlas_bounds.width(),
                atlas_bounds.height(),
            )
            .unwrap();
        font_tex.set_blend_mode(sdl2::render::BlendMode::Blend);
        let font_backing = Rc::new(RefCell::new(font_tex));

        let amber = FontTexture::new(amber_font, &amber_bounds, Rc::downgrade(&font_backing));
        let topaz = FontTexture::new(topaz_font, &topaz_bounds, Rc::downgrade(&font_backing));

        // ── Image atlas ───────────────────────────────────────────────────
        let image_atlas_rect = Rect::new(0, 0, IMAGE_ATLAS_W, IMAGE_ATLAS_H);
        let image_backing = Rc::new(RefCell::new(
            tex_maker
                .create_texture_static(
                    Some(PixelFormatEnum::RGBA32),
                    IMAGE_ATLAS_W,
                    IMAGE_ATLAS_H,
                )
                .unwrap(),
        ));

        let mut images: Vec<ImageTexture<'tex>> = Vec::new();
        let mut image_map: HashMap<String, usize> = HashMap::new();
        let mut next_x: u32 = 0;
        let mut next_y: u32 = 0;
        let mut row_h: u32 = 0;

        for name in game_lib.get_image_names() {
            let asset = game_lib.find_image(&name).unwrap();
            let iff = match asset.image.as_ref() {
                Some(i) => i,
                None => {
                    println!("Warning: ImageAsset {} has no IffImage data", asset.path);
                    continue;
                }
            };

            // Advance to next row if this image does not fit horizontally.
            if next_x + iff.width as u32 > image_atlas_rect.width() {
                next_x = 0;
                next_y += row_h;
                row_h = 0;
            }
            let slot = Rect::new(next_x as i32, next_y as i32, iff.width as u32, iff.height as u32);
            let mut img_tex = ImageTexture::new(iff, &slot, Rc::downgrade(&image_backing));

            let palette = iff.colormap.as_ref().unwrap_or(sys_palette);
            img_tex.update(palette, iff.transparent_color);

            next_x += iff.width as u32;
            row_h = row_h.max(iff.height as u32);

            image_map.insert(name, images.len());
            images.push(img_tex);
        }

        RenderResources {
            _font_backing: font_backing,
            amber,
            topaz,
            _image_backing: image_backing,
            images,
            image_map,
        }
    }

    // ── Image lookup ──────────────────────────────────────────────────────

    pub fn find_image(&self, name: &str) -> Option<&ImageTexture<'tex>> {
        self.image_map.get(name).map(|&i| &self.images[i])
    }

    pub fn find_image_mut(&mut self, name: &str) -> Option<&mut ImageTexture<'tex>> {
        self.image_map.get(name).copied().map(|i| &mut self.images[i])
    }

    /// Return the pixel dimensions of the image at `index` (for the debug window).
    pub fn image_dimensions(&self, index: usize) -> Option<(u32, u32)> {
        self.images.get(index).map(|t| {
            let b = t.get_bounds();
            (b.width(), b.height())
        })
    }

    // ── Per-frame interface ───────────────────────────────────────────────

    /// Construct a [`SceneResources`] that borrows from `self`.
    ///
    /// `scratch` is the 320×200 render-target used by [`IntroScene`] for
    /// page-flip snapshots; it lives in `main.rs` alongside `play_tex`.
    pub fn prepare<'a>(
        &'a mut self,
        scratch: &'a mut Texture<'tex>,
        audio: Option<&'a AudioSystem>,
    ) -> SceneResources<'a, 'tex> {
        SceneResources {
            image_textures: &mut self.images,
            image_name_map: &self.image_map,
            amber_font: &self.amber,
            topaz_font: &self.topaz,
            scratch,
            audio,
        }
    }
}
