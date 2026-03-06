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
use crate::game::bitmap::BitMap;
use crate::game::bitblit;
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

    // --- Compass textures ---
    // Pre-composited from hiscreen background + hinor/hivar plane 2 data.
    compass_normal: Option<Texture<'tex>>,
    compass_highlight: Option<Texture<'tex>>,
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

        // ── Stencil textures (inverted alpha for bg-color rendering) ──────
        let mut amber = amber;
        let mut topaz = topaz;
        if let Ok(mut s) = tex_maker.create_texture_static(Some(PixelFormatEnum::RGBA32), amber_bounds.width(), amber_bounds.height()) {
            s.set_blend_mode(sdl2::render::BlendMode::Blend);
            amber.init_stencil(s);
        }
        if let Ok(mut s) = tex_maker.create_texture_static(Some(PixelFormatEnum::RGBA32), topaz_bounds.width(), topaz_bounds.height()) {
            s.set_blend_mode(sdl2::render::BlendMode::Blend);
            topaz.init_stencil(s);
        }

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

        // ── Compass textures ───────────────────────────────────────────────
        // Extract the compass region from hiscreen, combine with hinor/hivar
        // as plane 2, convert to RGBA using the textcolors palette.
        let (compass_normal, compass_highlight) = Self::build_compass_textures(
            tex_maker, game_lib,
        );

        RenderResources {
            _font_backing: font_backing,
            amber,
            topaz,
            _image_backing: image_backing,
            images,
            image_map,
            compass_normal,
            compass_highlight,
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
            compass_normal: self.compass_normal.as_ref(),
            compass_highlight: self.compass_highlight.as_ref(),
        }
    }

    // ── Compass texture builder ───────────────────────────────────────────

    /// Build pre-composited compass textures from hiscreen + hinor/hivar.
    ///
    /// Returns `(normal, highlight)` as `Option<(Texture, Texture)>` — `None`
    /// if compass data or the hiscreen image is unavailable.
    fn build_compass_textures(
        tex_maker: &'tex TextureCreator<WindowContext>,
        game_lib: &GameLibrary,
    ) -> (Option<Texture<'tex>>, Option<Texture<'tex>>) {
        let result = Self::try_build_compass(tex_maker, game_lib);
        match result {
            Some((n, h)) => (Some(n), Some(h)),
            None => (None, None),
        }
    }

    fn try_build_compass(
        tex_maker: &'tex TextureCreator<WindowContext>,
        game_lib: &GameLibrary,
    ) -> Option<(Texture<'tex>, Texture<'tex>)> {
        // Compass position and size within hiscreen.
        const CX: usize = 567;
        const CY: usize = 15;
        const CW: usize = 48;
        const CH: usize = 24;

        let compass_cfg = game_lib.get_compass()?;
        let hiscreen_iff = game_lib.find_image("hiscreen").and_then(|a| a.image.as_ref())?;
        let textcolors = game_lib.find_palette("textcolors")?;

        // Create a BitMap from the full hiscreen image.
        let row_bytes = ((hiscreen_iff.width + 15) / 16) * 2;
        let hiscreen_bm = BitMap::with_interleaved_data(
            hiscreen_iff.pixels.clone(),
            hiscreen_iff.width,
            hiscreen_iff.height,
            hiscreen_iff.bitplanes,
            row_bytes,
        );

        // Extract the compass sub-region (handles non-byte-aligned x=567).
        let compass_base = bitblit::extract_region(&hiscreen_bm, CX, CY, CW, CH);

        // Build "normal" composite: base planes 0,1,3 + hinor as plane 2.
        let mut normal_bm = compass_base.clone();
        let hinor_bm = &compass_cfg.hinor;
        bitblit::set_plane(&mut normal_bm, 2, &hinor_bm.planes[0]);
        normal_bm.invalidate_cache();

        // Build "highlight" composite: base planes 0,1,3 + hivar as plane 2.
        let mut highlight_bm = compass_base;
        let hivar_bm = &compass_cfg.hivar;
        bitblit::set_plane(&mut highlight_bm, 2, &hivar_bm.planes[0]);
        highlight_bm.invalidate_cache();

        // Convert to RGBA pixel buffers.
        let (normal_rgba, _) = normal_bm.generate_rgb32(textcolors, None).ok()?;
        let (highlight_rgba, _) = highlight_bm.generate_rgb32(textcolors, None).ok()?;

        // Create SDL2 textures from the RGBA buffers.
        let mut normal_tex = tex_maker
            .create_texture_static(
                Some(PixelFormatEnum::RGBA32),
                CW as u32,
                CH as u32,
            )
            .ok()?;
        normal_tex.update(None, &normal_rgba, CW * 4).ok()?;

        let mut highlight_tex = tex_maker
            .create_texture_static(
                Some(PixelFormatEnum::RGBA32),
                CW as u32,
                CH as u32,
            )
            .ok()?;
        highlight_tex.update(None, &highlight_rgba, CW * 4).ok()?;

        Some((normal_tex, highlight_tex))
    }
}
