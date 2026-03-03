
use std::any::Any;
use std::collections::HashMap;

use sdl2::event::Event;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::game::audio::AudioSystem;
use crate::game::font_texture::FontTexture;
use crate::game::game_library::GameLibrary;
use crate::game::image_texture::ImageTexture;

/**
 * Result of a scene update. Determines what happens next in the scene loop.
 */
pub enum SceneResult {
    /// Scene is still running, continue calling update().
    Continue,
    /// Scene is finished, transition to the next scene (if any).
    Done,
}

/**
 * Rendering resources available to scenes. Contains references to loaded
 * image textures, font textures, and a name-to-index map for image lookup.
 *
 * The two lifetime parameters decouple the borrow lifetime (`'a`) from
 * the texture creator lifetime (`'tex`). This avoids drop-order conflicts
 * in main.rs where the texture Vec is declared after the texture creator.
 *
 * Created each frame in main.rs and passed to Scene::update().
 */
pub struct SceneResources<'a, 'tex> {
    pub image_textures: &'a mut [ImageTexture<'tex>],
    pub image_name_map: &'a HashMap<String, usize>,
    pub amber_font: &'a FontTexture<'tex>,
    pub topaz_font: &'a FontTexture<'tex>,
    /// Scratch texture (320×200 render target) for page flip animation.
    /// Used by IntroScene to snapshot the old page content before flipping.
    pub scratch: &'a mut Texture<'tex>,
    /// Audio system, for scenes that need to start or stop music.
    pub audio: Option<&'a AudioSystem>,
}

impl<'a, 'tex> SceneResources<'a, 'tex> {
    /// Look up an image texture by its name (as defined in faery.toml).
    pub fn find_image(&self, name: &str) -> Option<&ImageTexture<'tex>> {
        self.image_name_map.get(name).map(|&idx| &self.image_textures[idx])
    }

    /// Look up a mutable image texture by name, for palette re-rasterization.
    pub fn find_image_mut(&mut self, name: &str) -> Option<&mut ImageTexture<'tex>> {
        self.image_name_map.get(name).copied().map(move |idx| &mut self.image_textures[idx])
    }
}

/**
 * A Scene represents a distinct phase of the game (intro, story pages,
 * copy protection, gameplay, etc). Scenes own their internal state and
 * drive rendering, input handling, and transitions.
 *
 * The scene receives the canvas, offscreen texture, delta ticks, game
 * library, and rendering resources as separate parameters to avoid borrow
 * conflicts with SDL2's texture rendering patterns.
 */
pub trait Scene {
    /**
     * Handle an SDL event. Called once per event before update().
     * Return true if the event was consumed, false to pass it along.
     */
    fn handle_event(&mut self, _event: &Event) -> bool {
        false
    }

    /**
     * Called once per frame to update state and render.
     *
     * `canvas` - the SDL2 canvas for rendering (logical 640x480)
     * `play_tex` - the 320x200 offscreen texture for Amiga-resolution rendering
     * `delta_ticks` - elapsed ticks since last frame (1/60s per tick)
     * `game_lib` - reference to all game assets
     * `resources` - image and font textures for rendering
     *
     * Returns SceneResult::Continue to keep running, or SceneResult::Done
     * when the scene is finished.
     */
    fn update(
        &mut self,
        canvas: &mut Canvas<Window>,
        play_tex: &mut Texture,
        delta_ticks: u32,
        game_lib: &GameLibrary,
        resources: &mut SceneResources<'_, '_>,
    ) -> SceneResult;

    /**
     * Called when the scene is about to be replaced. Clean up any resources.
     */
    fn on_exit(&mut self) {}

    /**
     * Downcast support so callers can recover the concrete scene type.
     */
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
