
use std::any::Any;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::game::palette_fader::{FadeController, FadeResult};
use crate::game::placard::{PlacardRenderer, start_placard_renderer};
use crate::game::scene::{Scene, SceneResources, SceneResult};
use crate::game::game_library::GameLibrary;

/**
 * Scene that displays a named placard with the swirly border.
 *
 * Used for character start/death text (julian_start, julian_dead, etc.)
 * and other in-game messages that use the placard border animation.
 *
 * The scene clears the 320x200 play texture, draws the placard text
 * using the amber font, then progressively draws the decorative border
 * over ~4.5 seconds using PlacardRenderer. After the border completes,
 * holds for a specified duration, then completes.
 *
 * In the original game:
 * - placard_text(N) draws the text
 * - placard() draws the animated border
 * - Delay(120) holds for 2.4 seconds (at 50Hz)
 */

/// Default hold duration after border is drawn (ticks at 30Hz).
/// Original: Delay(120) at 50Hz = 2.4s = 72 ticks at 30Hz.
const DEFAULT_HOLD_TICKS: u32 = 72;

enum PlacardPhase {
    /// First frame: draw text to play_tex, then start border animation.
    Draw,
    /// Progressive border animation drawn to play_tex each frame.
    AnimateBorder { renderer: PlacardRenderer },
    /// Hold the placard on screen for a duration.
    Hold { ticks_remaining: u32 },
    /// Fade to black before completing.
    FadeOut { fader: FadeController },
    /// Scene complete.
    Done,
}

pub struct PlacardScene {
    phase: PlacardPhase,
    /// Name of the placard in GameLibrary (e.g., "julian_start").
    placard_name: String,
    /// Name of the palette for the border colors.
    palette_name: String,
    /// How long to hold the placard after drawing (ticks).
    hold_ticks: u32,
    /// Allow Space to skip.
    skip_requested: bool,
    /// Optional substitution for `%` in placard text (brother name).
    substitution: Option<String>,
}

impl PlacardScene {
    /// Create a new placard scene showing the named placard.
    pub fn new(placard_name: &str, palette_name: &str) -> PlacardScene {
        PlacardScene {
            phase: PlacardPhase::Draw,
            placard_name: placard_name.to_string(),
            palette_name: palette_name.to_string(),
            hold_ticks: DEFAULT_HOLD_TICKS,
            skip_requested: false,
            substitution: None,
        }
    }

    /// Create a placard scene with a custom hold duration.
    pub fn with_hold_ticks(mut self, ticks: u32) -> PlacardScene {
        self.hold_ticks = ticks;
        self
    }

    /// Substitute `%` in all placard lines with the given string. Used for
    /// placards that interpolate the hero's name (e.g. `player_win`,
    /// `rescue_katra`).
    pub fn with_substitution(mut self, sub: impl Into<String>) -> PlacardScene {
        self.substitution = Some(sub.into());
        self
    }
}

impl Scene for PlacardScene {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

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
        if self.skip_requested {
            self.phase = PlacardPhase::Done;
        }

        match &mut self.phase {
            PlacardPhase::Draw => {
                // Get the palette for border colors
                let palette = game_lib.find_palette(&self.palette_name);

                // Horizontal centering: border is 284px wide, play_tex is 320px wide.
                const BORDER_X_OFFSET: i32 = (320 - 284) / 2; // = 18

                // Draw text to play_tex (border will be animated separately)
                let placard_name = self.placard_name.clone();
                let substitution = self.substitution.clone();
                // Set the font color to palette index 24 (red in pagecolors).
                if let Some(pal) = palette {
                    if let Some(c) = pal.get_color(24) {
                        resources.amber_font.set_color_mod(c.r(), c.g(), c.b());
                    }
                }

                let _ = canvas.with_texture_canvas(play_tex, |play_canvas| {
                    // Black background
                    play_canvas.set_draw_color(Color::BLACK);
                    play_canvas.clear();

                    // Draw the placard text shifted right to align with centered border
                    if let Some(plac) = game_lib.find_placard(&placard_name) {
                        match &substitution {
                            Some(sub) => plac.draw_offset_substituted(
                                resources.amber_font, play_canvas, BORDER_X_OFFSET, 0, sub,
                            ),
                            None => plac.draw_offset(
                                resources.amber_font, play_canvas, BORDER_X_OFFSET, 0,
                            ),
                        }
                    }
                });

                // Reset font color to white for subsequent rendering
                resources.amber_font.set_color_mod(255, 255, 255);

                // Blit to screen
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();
                let screen_dest = Rect::new(0, 40, 640, 400);
                canvas.copy(play_tex, None, Some(screen_dest)).unwrap();

                // Start the progressive border animation, centered horizontally
                let renderer = if let Some(pal) = palette {
                    start_placard_renderer(&sdl2::rect::Point::new(BORDER_X_OFFSET, 0), pal)
                } else {
                    // Fallback: skip to hold if no palette
                    self.phase = PlacardPhase::Hold {
                        ticks_remaining: self.hold_ticks,
                    };
                    return SceneResult::Continue;
                };
                self.phase = PlacardPhase::AnimateBorder { renderer };
                SceneResult::Continue
            }

            PlacardPhase::AnimateBorder { renderer } => {
                // Draw more border segments onto the persistent play_tex
                let _ = canvas.with_texture_canvas(play_tex, |play_canvas| {
                    renderer.draw_segments(play_canvas, delta_ticks as i32);
                });

                let done = renderer.is_done();

                // Blit to screen
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();
                let screen_dest = Rect::new(0, 40, 640, 400);
                canvas.copy(play_tex, None, Some(screen_dest)).unwrap();

                if done {
                    self.phase = PlacardPhase::Hold {
                        ticks_remaining: self.hold_ticks,
                    };
                }

                SceneResult::Continue
            }

            PlacardPhase::Hold { ticks_remaining } => {
                // Just blit the existing play_tex content to screen
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();
                let screen_dest = Rect::new(0, 40, 640, 400);
                canvas.copy(play_tex, None, Some(screen_dest)).unwrap();

                if delta_ticks >= *ticks_remaining {
                    // Fade to black before completing
                    let palette = game_lib.find_palette(&self.palette_name).unwrap();
                    self.phase = PlacardPhase::FadeOut {
                        fader: FadeController::fade_down(palette, 60), // 1s fade
                    };
                } else {
                    *ticks_remaining -= delta_ticks;
                }

                SceneResult::Continue
            }

            PlacardPhase::FadeOut { fader } => {
                let result = fader.tick(delta_ticks);
                let (r, g, b) = match result {
                    FadeResult::ColorMod(r, g, b) => (r, g, b),
                    _ => (255, 255, 255),
                };
                play_tex.set_color_mod(r, g, b);

                canvas.set_draw_color(Color::BLACK);
                canvas.clear();
                let screen_dest = Rect::new(0, 40, 640, 400);
                canvas.copy(play_tex, None, Some(screen_dest)).unwrap();

                if fader.is_done() {
                    play_tex.set_color_mod(255, 255, 255);
                    self.phase = PlacardPhase::Done;
                }
                SceneResult::Continue
            }

            PlacardPhase::Done => SceneResult::Done,
        }
    }
}
