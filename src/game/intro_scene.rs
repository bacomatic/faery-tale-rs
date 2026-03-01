
use std::any::Any;
use std::sync::Arc;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::game::page_flip::PageFlip;
use crate::game::palette_fader::{FadeController, FadeResult};
use crate::game::scene::{Scene, SceneResources, SceneResult};
use crate::game::viewport_zoom::ViewportZoom;
use crate::game::game_library::GameLibrary;
use crate::game::songs::Track;

/**
 * The intro scene plays the complete opening sequence:
 *
 * 1. TitleText - display legal/title text on dark blue background, wait ~2s
 * 2. TitleFadeOut - brief fade to black
 * 3. ZoomIn - viewport zoom revealing page0 (book background)
 * 4. ShowPage(0) - display page0 for ~7s
 * 5. FlipPage(0->1) - transition to page 1 (page0 + Julian portrait + bio)
 * 6. ShowPage(1) - display page 1 for ~7s
 * 7. FlipPage(1->2) - transition to page 2 (Phillip)
 * 8. ShowPage(2) - display page 2 for ~7s
 * 9. FlipPage(2->3) - transition to page 3 (Kevin)
 * 10. ShowPage(3) - display page 3 for ~3.8s
 * 11. ZoomOut - viewport zoom from page 3 to black
 * 12. Done
 *
 * The user can press Space at any time to skip the intro.
 *
 * Page compositing follows the original game's approach:
 * - page0 (320x200) is the book background, drawn once
 * - Each subsequent page overlays a portrait at (4, 24) and bio text at
 *   a page-specific position, accumulating onto the previous page content
 */

/// Internal phase of the intro sequence.
enum IntroPhase {
    /// Display title/legal text, hold for a delay.
    TitleText { ticks_remaining: u32 },
    /// Fade to black and set up for story pages.
    TitleFadeOut { fader: FadeController },
    /// Zoom in from black, revealing page0.
    ZoomIn {
        zoom: ViewportZoom,
        page_drawn: bool,
    },
    /// Display a story page for a fixed duration.
    ShowPage {
        page_index: usize,
        ticks_remaining: u32,
    },
    /// Animate the page flip between two pages.
    /// `scratch` holds a snapshot of the old page; `play_tex` holds the new
    /// page.  PageFlip draws strips from both directly to the window canvas.
    FlipPage {
        from_index: usize,
        to_index: usize,
        flipper: PageFlip,
        initialized: bool,
    },
    /// Zoom out from the last page to black.
    ZoomOut { zoom: ViewportZoom },
    /// Sequence complete.
    Done,
}

/// Original page compositing data.
/// Each page is the book background (page0) with portrait and bio overlays.
/// Positions are taken directly from the original fmain.c `copypage` calls.
struct PageOverlay {
    portrait: &'static str,
    bio: &'static str,
    bio_x: i32,
    bio_y: i32,
}

const PAGE_OVERLAYS: [PageOverlay; 3] = [
    PageOverlay { portrait: "p1a", bio: "p1b", bio_x: 168, bio_y: 29 }, // Julian  (21 bytes * 8)
    PageOverlay { portrait: "p2a", bio: "p2b", bio_x: 160, bio_y: 29 }, // Phillip (20 bytes * 8)
    PageOverlay { portrait: "p3a", bio: "p3b", bio_x: 160, bio_y: 33 }, // Kevin   (20 bytes * 8)
];

/// Portrait position: same for all pages (from original: unpackbrush(br1, &pageb, 4, 24))
/// The original x=4 is a byte offset → 4 * 8 = 32 pixels.
const PORTRAIT_X: i32 = 32;
const PORTRAIT_Y: i32 = 24;

/// How long to display each story page before flipping (ticks at 60Hz).
/// 8s = 480 ticks.
const PAGE_DISPLAY_TICKS: u32 = 480;

/// How long to hold page0 after zoom in before flipping.
const PAGE0_DISPLAY_TICKS: u32 = 480;

/// How long to hold the last page after display before zooming out.
const LAST_PAGE_HOLD_TICKS: u32 = 480;

/// How long to hold the title text before proceeding.
/// 3s = 180 ticks at 60Hz.
const TITLE_HOLD_TICKS: u32 = 180;

/// Duration of the title fade-out in ticks.
/// 1.5s = 90 ticks at 60Hz.
const TITLE_FADE_TICKS: u32 = 90;

/// Total duration of zoom in/out in ticks.
/// 3s = 180 ticks at 60Hz.
const ZOOM_DURATION_TICKS: u32 = 180;

/// Minimum ticks per page-flip step for ~5s total flip.
/// 22 steps × 14 ticks/step ≈ 308 ticks ≈ 5.1s.
const FLIP_MIN_STEP_TICKS: u32 = 14;

/// Title text is rendered directly to the canvas (640x480 logical) because the
/// original uses a 640-wide hires text viewport. The text y-coordinates are
/// offset by this amount to position them within the visible area.
const TITLE_Y_OFFSET: i32 = 140;

pub struct IntroScene {
    phase: IntroPhase,
    skip_requested: bool,
    /// Intro music tracks (tracks 12-15), to be started when the scene begins
    /// its visual sequence.  Mirrors the original: playscore() is called after
    /// the title text delay but before the zoom-in.
    intro_tracks: Option<[Arc<Track>; 4]>,
    /// True once play_score() has been called (avoids calling it again on skip).
    music_started: bool,
}

impl IntroScene {
    pub fn new(intro_tracks: Option<[Arc<Track>; 4]>) -> IntroScene {
        IntroScene {
            phase: IntroPhase::TitleText {
                ticks_remaining: TITLE_HOLD_TICKS,
            },
            skip_requested: false,
            intro_tracks,
            music_started: false,
        }
    }

    /// Advance to the next logical phase.
    fn advance(&mut self, game_lib: &GameLibrary) {
        self.phase = match &self.phase {
            IntroPhase::TitleText { .. } => {
                // Fade to black before starting the story pages
                let text_palette = game_lib.find_palette("textcolors").unwrap();
                IntroPhase::TitleFadeOut {
                    fader: FadeController::fade_down(text_palette, TITLE_FADE_TICKS),
                }
            }
            IntroPhase::TitleFadeOut { .. } => {
                IntroPhase::ZoomIn {
                    zoom: ViewportZoom::zoom_in_duration(ZOOM_DURATION_TICKS),
                    page_drawn: false,
                }
            }
            IntroPhase::ZoomIn { .. } => {
                IntroPhase::ShowPage {
                    page_index: 0,
                    ticks_remaining: PAGE0_DISPLAY_TICKS,
                }
            }
            IntroPhase::ShowPage { page_index, .. } => {
                let pi = *page_index;
                if pi < 3 {
                    IntroPhase::FlipPage {
                        from_index: pi,
                        to_index: pi + 1,
                        flipper: PageFlip::with_min_step(FLIP_MIN_STEP_TICKS),
                        initialized: false,
                    }
                } else {
                    // Last page shown, now zoom out
                    IntroPhase::ZoomOut {
                        zoom: ViewportZoom::zoom_out_duration(ZOOM_DURATION_TICKS),
                    }
                }
            }
            IntroPhase::FlipPage { to_index, .. } => {
                let hold = if *to_index == 3 {
                    LAST_PAGE_HOLD_TICKS
                } else {
                    PAGE_DISPLAY_TICKS
                };
                IntroPhase::ShowPage {
                    page_index: *to_index,
                    ticks_remaining: hold,
                }
            }
            IntroPhase::ZoomOut { .. } => IntroPhase::Done,
            IntroPhase::Done => IntroPhase::Done,
        };
    }

}

/// Draw a page's overlay images (portrait + bio) onto a canvas.
/// This modifies the content already on the canvas, overlaying new images
/// on top. This matches the original game behavior where pages accumulate.
///
/// Free function to avoid borrow conflicts when called inside a
/// `with_texture_canvas` closure that also matches `&mut self.phase`.
fn draw_page_overlays<T: sdl2::render::RenderTarget>(
    page_index: usize,
    canvas: &mut Canvas<T>,
    resources: &SceneResources<'_, '_>,
) {
    if page_index == 0 {
        // Page 0 is just the book background, no overlays
        return;
    }

    // Pages 1-3 are indexed 0-2 in PAGE_OVERLAYS
    let overlay = &PAGE_OVERLAYS[page_index - 1];

    if let Some(portrait) = resources.find_image(overlay.portrait) {
        portrait.draw(canvas, PORTRAIT_X, PORTRAIT_Y);
    }

    if let Some(bio) = resources.find_image(overlay.bio) {
        bio.draw(canvas, overlay.bio_x, overlay.bio_y);
    }
}

impl Scene for IntroScene {
    fn as_any(&self) -> &dyn Any { self }

    fn handle_event(&mut self, event: &Event) -> bool {
        match event {
            Event::KeyDown { keycode: Some(Keycode::Space), .. } => {
                self.skip_requested = true;
                true
            }
            _ => false,
        }
    }

    fn update(
        &mut self,
        canvas: &mut Canvas<Window>,
        play_tex: &mut Texture,
        delta_ticks: u32,
        game_lib: &GameLibrary,
        resources: &mut SceneResources<'_, '_>,
    ) -> SceneResult {
        let delta = delta_ticks;

        // Skip requested — jump to done
        if self.skip_requested {
            self.phase = IntroPhase::Done;
        }

        match &mut self.phase {
            IntroPhase::TitleText { ticks_remaining } => {
                // Draw black background, white text using the Amber font.
                // Render title to play_tex so TitleFadeOut can fade it via color_mod.
                let _ = canvas.with_texture_canvas(play_tex, |play_canvas| {
                    play_canvas.set_draw_color(Color::BLACK);
                    play_canvas.clear();
                });

                canvas.set_draw_color(Color::BLACK);
                canvas.clear();

                // Render white title text directly onto the canvas.
                // Set color_mod to white, draw, then reset to amber.
                resources.amber_font.set_color_mod(255, 255, 255);
                if let Some(placard) = game_lib.find_placard("titletext") {
                    placard.draw_offset(
                        resources.amber_font,
                        canvas,
                        0,
                        TITLE_Y_OFFSET,
                    );
                }
                resources.amber_font.set_color_mod(0xFF, 0xAA, 0x00);

                if delta >= *ticks_remaining {
                    *ticks_remaining = 0;
                } else {
                    *ticks_remaining -= delta;
                }

                if *ticks_remaining == 0 {
                    self.advance(game_lib);
                }

                SceneResult::Continue
            }

            IntroPhase::TitleFadeOut { fader } => {
                // Start music on the first frame of TitleFadeOut.
                if !self.music_started {
                    if let (Some(tracks), Some(audio)) = (self.intro_tracks.take(), resources.audio) {
                        audio.play_score(tracks);
                    }
                    self.music_started = true;
                }

                // Render white title text to play_tex (black background) on the first
                // frame so we can fade it out via color_mod.
                let result = fader.tick(delta);

                // Apply color_mod to play_tex for the fade.
                let (r, g, b) = match result {
                    FadeResult::ColorMod(r, g, b) => (r, g, b),
                    _ => (255, 255, 255),
                };
                play_tex.set_color_mod(r, g, b);

                // Draw black canvas, then blit fading title from play_tex.
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();

                // Render white text onto play_tex each frame at full brightness,
                // then let color_mod dim it.
                let _ = canvas.with_texture_canvas(play_tex, |play_canvas| {
                    play_canvas.set_draw_color(Color::BLACK);
                    play_canvas.clear();
                });
                // Draw title text to canvas with current fade modulation.
                // We use draw_color_mod on the font itself to simulate the fade,
                // interpolating from white (255,255,255) to black (0,0,0).
                resources.amber_font.set_color_mod(r, g, b);
                if let Some(placard) = game_lib.find_placard("titletext") {
                    placard.draw_offset(
                        resources.amber_font,
                        canvas,
                        0,
                        TITLE_Y_OFFSET,
                    );
                }
                resources.amber_font.set_color_mod(0xFF, 0xAA, 0x00);

                if fader.is_done() {
                    play_tex.set_color_mod(255, 255, 255);
                    self.advance(game_lib);
                }

                SceneResult::Continue
            }

            IntroPhase::ZoomIn { zoom, page_drawn } => {
                // Get the intro palette for the zoom fade
                let intro_palette = game_lib.find_palette("introcolors").unwrap();

                // Compute the zoom-position-dependent faded palette and
                // re-rasterize all relevant images with it. This replicates
                // the original's screen_size() → fade_page(y*2-40, y*2-70, y*2-100, 0, introcolors)
                let hw = zoom.half_width();
                let faded_palette = FadeController::zoom_fade(intro_palette, hw);

                // Re-rasterize the page images with the faded palette
                // We need to update all images that use the intro palette
                let intro_images = ["page0", "p1a", "p1b", "p2a", "p2b", "p3a", "p3b"];
                for name in &intro_images {
                    if let Some(img) = resources.find_image_mut(name) {
                        img.update(&faded_palette, None);
                    }
                }

                // Draw page0 to play_tex (re-drawn each frame since palette changes)
                let _ = canvas.with_texture_canvas(play_tex, |play_canvas| {
                    if let Some(page0) = resources.find_image("page0") {
                        page0.draw(play_canvas, 0, 0);
                    } else {
                        play_canvas.set_draw_color(Color::RGB(0x33, 0x22, 0x11));
                        play_canvas.clear();
                    }
                });
                *page_drawn = true;

                // Advance zoom
                let viewport = zoom.tick(delta);

                // Clear the screen canvas
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();

                // Blit the growing viewport sub-rect from play_tex to the screen
                if viewport.width() > 1 && viewport.height() > 1 {
                    let screen_dest = Rect::new(
                        viewport.x() * 2,
                        40 + viewport.y() * 2,
                        viewport.width() * 2,
                        viewport.height() * 2,
                    );
                    canvas.copy(play_tex, Some(viewport), Some(screen_dest)).unwrap();
                }

                if zoom.is_done() {
                    // Restore full-brightness palette for the ShowPage phase
                    for name in &intro_images {
                        if let Some(img) = resources.find_image_mut(name) {
                            img.update(intro_palette, None);
                        }
                    }
                    // Redraw page0 at full brightness
                    let _ = canvas.with_texture_canvas(play_tex, |play_canvas| {
                        if let Some(page0) = resources.find_image("page0") {
                            page0.draw(play_canvas, 0, 0);
                        }
                    });
                    self.advance(game_lib);
                }

                SceneResult::Continue
            }

            IntroPhase::ShowPage { ticks_remaining, .. } => {
                // The page content is already on play_tex (drawn by ZoomIn or FlipPage).
                // Just blit it to the screen and count down.
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();

                let screen_dest = Rect::new(0, 40, 640, 400);
                canvas.copy(play_tex, None, Some(screen_dest)).unwrap();

                if delta >= *ticks_remaining {
                    *ticks_remaining = 0;
                } else {
                    *ticks_remaining -= delta;
                }

                if *ticks_remaining == 0 {
                    self.advance(game_lib);
                }

                SceneResult::Continue
            }

            IntroPhase::FlipPage { to_index, flipper, initialized, .. } => {
                // On the first frame, snapshot the current play_tex (old page)
                // into the scratch texture, then draw the new page's overlays
                // onto play_tex so it becomes the new page.
                if !*initialized {
                    // 1. Snapshot play_tex → scratch (old page)
                    let _ = canvas.with_texture_canvas(resources.scratch, |scratch_canvas| {
                        scratch_canvas.copy(&*play_tex, None, None).unwrap();
                    });

                    // 2. Draw new page overlays onto play_tex (new page)
                    let page_idx = *to_index;
                    let _ = canvas.with_texture_canvas(play_tex, |play_canvas| {
                        draw_page_overlays(page_idx, play_canvas, resources);
                    });

                    *initialized = true;
                }

                // Draw the flip animation: strips from scratch (old) and
                // play_tex (new) are composited directly onto the window canvas.
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();

                // Reborrow as immutable for canvas.copy() inside PageFlip
                let old_tex: &Texture = &*resources.scratch;
                let new_tex: &Texture = &*play_tex;
                let still_running = flipper.update(canvas, old_tex, new_tex, delta, 2, 40);

                if !still_running {
                    self.advance(game_lib);
                }

                SceneResult::Continue
            }

            IntroPhase::ZoomOut { zoom } => {
                // Compute zoom-position-dependent fade and re-rasterize
                let intro_palette = game_lib.find_palette("introcolors").unwrap();
                let hw = zoom.half_width();
                let faded_palette = FadeController::zoom_fade(intro_palette, hw);

                // Re-rasterize the page images with the faded palette
                let intro_images = ["page0", "p1a", "p1b", "p2a", "p2b", "p3a", "p3b"];
                for name in &intro_images {
                    if let Some(img) = resources.find_image_mut(name) {
                        img.update(&faded_palette, None);
                    }
                }

                // Redraw the current page content with faded palette
                let _ = canvas.with_texture_canvas(play_tex, |play_canvas| {
                    // Redraw page0 background
                    if let Some(page0) = resources.find_image("page0") {
                        page0.draw(play_canvas, 0, 0);
                    }
                    // Redraw all accumulated page overlays (pages 1-3)
                    for pi in 1..=3 {
                        draw_page_overlays(pi, play_canvas, resources);
                    }
                });

                let viewport = zoom.tick(delta);

                // Draw black and show shrinking viewport
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();

                if viewport.width() > 1 && viewport.height() > 1 {
                    let screen_dest = Rect::new(
                        viewport.x() * 2,
                        40 + viewport.y() * 2,
                        viewport.width() * 2,
                        viewport.height() * 2,
                    );
                    canvas.copy(play_tex, Some(viewport), Some(screen_dest)).unwrap();
                }

                if zoom.is_done() {
                    // Restore full-brightness palette
                    for name in &intro_images {
                        if let Some(img) = resources.find_image_mut(name) {
                            img.update(intro_palette, None);
                        }
                    }
                    self.advance(game_lib);
                }

                SceneResult::Continue
            }

            IntroPhase::Done => SceneResult::Done,
        }
    }
}
