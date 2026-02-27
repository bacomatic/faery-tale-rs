
use std::any::Any;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::game::placard;
use crate::game::scene::{Scene, SceneResources, SceneResult};
use crate::game::game_library::GameLibrary;

/**
 * Scene that displays a named placard with the swirly border.
 *
 * Used for character start/death text (julian_start, julian_dead, etc.)
 * and other in-game messages that use the placard border animation.
 *
 * The scene clears the 320x200 play texture, draws the placard text
 * using the amber font, draws the decorative border, holds for a
 * specified duration, then completes.
 *
 * In the original game:
 * - placard_text(N) draws the text
 * - placard() draws the animated border
 * - Delay(120) holds for 2.4 seconds (at 50Hz)
 *
 * The border animation is drawn instantly for now. The full progressive
 * animation can be added by using PlacardRenderer as a RenderTask.
 */

/// Default hold duration after border is drawn (ticks at 60Hz).
/// Original: Delay(120) at 50Hz = 2.4s = 144 ticks at 60Hz.
const DEFAULT_HOLD_TICKS: u32 = 144;

enum PlacardPhase {
    /// First frame: draw everything to play_tex.
    Draw,
    /// Hold the placard on screen for a duration.
    Hold { ticks_remaining: u32 },
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
        }
    }

    /// Create a placard scene with a custom hold duration.
    #[allow(dead_code)]
    pub fn with_hold_ticks(mut self, ticks: u32) -> PlacardScene {
        self.hold_ticks = ticks;
        self
    }
}

impl Scene for PlacardScene {
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
        if self.skip_requested {
            self.phase = PlacardPhase::Done;
        }

        match &mut self.phase {
            PlacardPhase::Draw => {
                // Get the palette for border colors
                let palette = game_lib.find_palette(&self.palette_name);

                // Draw everything to play_tex
                let placard_name = self.placard_name.clone();
                // Set the font color to palette index 24 (red in pagecolors).
                // The original game uses SetAPen(rp, 24) in map_message() for
                // placard text, matching the border color.
                if let Some(pal) = palette {
                    if let Some(c) = pal.get_color(24) {
                        resources.amber_font.set_color_mod(c.r(), c.g(), c.b());
                    }
                }

                let _ = canvas.with_texture_canvas(play_tex, |play_canvas| {
                    // Black background
                    play_canvas.set_draw_color(Color::BLACK);
                    play_canvas.clear();

                    // Draw the placard text
                    if let Some(plac) = game_lib.find_placard(&placard_name) {
                        plac.draw(resources.amber_font, play_canvas);
                    }

                    // Draw the decorative border using the palette
                    if let Some(pal) = palette {
                        placard::draw_placard_border(play_canvas, pal);
                    }
                });

                // Reset font color to white for subsequent rendering
                resources.amber_font.set_color_mod(255, 255, 255);

                // Blit to screen
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();
                let screen_dest = Rect::new(0, 40, 640, 400);
                canvas.copy(play_tex, None, Some(screen_dest)).unwrap();

                self.phase = PlacardPhase::Hold {
                    ticks_remaining: self.hold_ticks,
                };
                SceneResult::Continue
            }

            PlacardPhase::Hold { ticks_remaining } => {
                // Just blit the existing play_tex content to screen
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();
                let screen_dest = Rect::new(0, 40, 640, 400);
                canvas.copy(play_tex, None, Some(screen_dest)).unwrap();

                if delta_ticks >= *ticks_remaining {
                    self.phase = PlacardPhase::Done;
                } else {
                    *ticks_remaining -= delta_ticks;
                }

                SceneResult::Continue
            }

            PlacardPhase::Done => SceneResult::Done,
        }
    }
}
