
use std::any::Any;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::game::palette_fader::PaletteFader;
use crate::game::scene::{Scene, SceneResources, SceneResult};
use crate::game::viewport_zoom::ViewportZoom;
use crate::game::game_library::GameLibrary;

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
    TitleFadeOut { fader: PaletteFader },
    /// Zoom in from black, revealing page0.
    ZoomIn {
        zoom: ViewportZoom,
        fader: PaletteFader,
        page_drawn: bool,
    },
    /// Display a story page for a fixed duration.
    ShowPage {
        page_index: usize,
        ticks_remaining: u32,
    },
    /// Animate the page flip between two pages.
    /// For now this is an instant transition; proper strip animation is a
    /// future enhancement requiring two scratch textures.
    FlipPage {
        from_index: usize,
        to_index: usize,
        drawn: bool,
    },
    /// Zoom out from the last page to black.
    ZoomOut { zoom: ViewportZoom, fader: PaletteFader },
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
/// Original uses Delay(350) at 50Hz = 7 seconds.
const PAGE_DISPLAY_TICKS: u32 = 420;

/// How long to hold page0 after zoom in before flipping.
const PAGE0_DISPLAY_TICKS: u32 = 420;

/// How long to hold the last page after display before zooming out.
/// Original: Delay(190) at 50Hz = 3.8s.
const LAST_PAGE_HOLD_TICKS: u32 = 228;

/// How long to hold the title text before proceeding.
/// Original: Delay(50) x2 at 50Hz ≈ 2 seconds.
const TITLE_HOLD_TICKS: u32 = 120;

/// Duration of palette fades in ticks (zoom in/out includes a fade).
/// The zoom itself runs 40 steps x 2 ticks = 80 ticks. The fade runs in parallel.
const FADE_DURATION_TICKS: u32 = 80;

/// Title text is rendered directly to the canvas (640x480 logical) because the
/// original uses a 640-wide hires text viewport. The text y-coordinates are
/// offset by this amount to position them within the visible area.
const TITLE_Y_OFFSET: i32 = 140;

pub struct IntroScene {
    phase: IntroPhase,
    skip_requested: bool,
}

impl IntroScene {
    pub fn new() -> IntroScene {
        IntroScene {
            phase: IntroPhase::TitleText {
                ticks_remaining: TITLE_HOLD_TICKS,
            },
            skip_requested: false,
        }
    }

    /// Advance to the next logical phase.
    fn advance(&mut self, game_lib: &GameLibrary) {
        self.phase = match &self.phase {
            IntroPhase::TitleText { .. } => {
                // Fade to black before starting the story pages
                let text_palette = game_lib.find_palette("textcolors").unwrap();
                let black_palette = game_lib.find_palette("blackcolors").unwrap();
                IntroPhase::TitleFadeOut {
                    fader: PaletteFader::new(text_palette, black_palette, 24),
                }
            }
            IntroPhase::TitleFadeOut { .. } => {
                let black_palette = game_lib.find_palette("blackcolors").unwrap();
                let intro_palette = game_lib.find_palette("introcolors").unwrap();
                IntroPhase::ZoomIn {
                    zoom: ViewportZoom::zoom_in(),
                    fader: PaletteFader::new(black_palette, intro_palette, FADE_DURATION_TICKS),
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
                        drawn: false,
                    }
                } else {
                    // Last page shown, now zoom out
                    let intro_palette = game_lib.find_palette("introcolors").unwrap();
                    let black_palette = game_lib.find_palette("blackcolors").unwrap();
                    IntroPhase::ZoomOut {
                        zoom: ViewportZoom::zoom_out(),
                        fader: PaletteFader::new(intro_palette, black_palette, FADE_DURATION_TICKS),
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
        resources: &SceneResources<'_, '_>,
    ) -> SceneResult {
        let delta = delta_ticks;

        // Skip requested — jump to done
        if self.skip_requested {
            self.phase = IntroPhase::Done;
        }

        match &mut self.phase {
            IntroPhase::TitleText { ticks_remaining } => {
                // Draw dark blue background covering the full canvas
                canvas.set_draw_color(Color::RGB(0, 0, 0x66));
                canvas.clear();

                // Render the titletext placard directly onto the canvas.
                // The placard coordinates are designed for a 640-wide display,
                // which matches our logical canvas width. Y-coordinates are
                // offset to center vertically in the 480-tall window.
                if let Some(placard) = game_lib.find_placard("titletext") {
                    placard.draw_offset(
                        resources.amber_font,
                        canvas,
                        0,
                        TITLE_Y_OFFSET,
                    );
                }

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
                // Brief fade to black
                fader.tick(delta);

                // Draw black screen during fade
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();

                if fader.is_done() {
                    self.advance(game_lib);
                }

                SceneResult::Continue
            }

            IntroPhase::ZoomIn { zoom, fader, page_drawn } => {
                // Draw page0 to play_tex once at the start of the zoom
                if !*page_drawn {
                    let _ = canvas.with_texture_canvas(play_tex, |play_canvas| {
                        // Draw the book background
                        if let Some(page0) = resources.find_image("page0") {
                            page0.draw(play_canvas, 0, 0);
                        } else {
                            // Fallback: dark background
                            play_canvas.set_draw_color(Color::RGB(0x33, 0x22, 0x11));
                            play_canvas.clear();
                        }
                    });
                    *page_drawn = true;
                }

                // Advance both zoom and fade
                let viewport = zoom.tick(delta);
                fader.tick(delta);

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

            IntroPhase::FlipPage { to_index, drawn, .. } => {
                // Instant page transition: draw new page overlays onto play_tex.
                // play_tex already contains the previous page content (page0 bg +
                // previous overlays). We simply overlay the new page's portrait
                // and bio on top, matching the original game's accumulative drawing.
                //
                // TODO: Implement proper strip-based page flip animation using
                // two scratch textures (PageFlip::update).
                if !*drawn {
                    let page_idx = *to_index;
                    let _ = canvas.with_texture_canvas(play_tex, |play_canvas| {
                        draw_page_overlays(page_idx, play_canvas, resources);
                    });
                    *drawn = true;
                }

                // Show the new page briefly then advance
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();

                let screen_dest = Rect::new(0, 40, 640, 400);
                canvas.copy(play_tex, None, Some(screen_dest)).unwrap();

                self.advance(game_lib);
                SceneResult::Continue
            }

            IntroPhase::ZoomOut { zoom, fader } => {
                let viewport = zoom.tick(delta);
                fader.tick(delta);

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
                    self.advance(game_lib);
                }

                SceneResult::Continue
            }

            IntroPhase::Done => SceneResult::Done,
        }
    }
}
