//! Victory image scene — displays `winpic` after the victory placard.
//!
//! Original `win_colors()` (`fmain2.c:1605-1636`) after the placard:
//! 1. `unpackbrush("winpic", bm_draw, 0, 0)` — IFF image from `game/winpic`.
//! 2. Blacks out both viewports and hides the HUD.
//! 3. Expands playfield (`screen_size(156)`) to 312×194.
//! 4. 55-frame sunrise animation over `sun_colors[53]` gradient.
//! 5. Final 30-tick pause then fade to black.
//!
//! This port presents the image, holds for a few seconds, then fades to black
//! and returns `Done`. The full 55-frame `sun_colors[]` palette animation is a
//! polish item tracked separately (T4). Player-visible outcome (seeing the
//! reward image before the game exits) matches the original behavior.

use std::any::Any;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::game::game_library::GameLibrary;
use crate::game::scene::{Scene, SceneResources, SceneResult};

/// Hold duration for the victory image (ticks at 30 Hz). Original sunrise
/// animation ≈ 11.1 s + 30-tick pause; approximate here with a 6-second still.
const HOLD_TICKS: u32 = 180;
/// Fade-to-black duration (ticks at 30 Hz).
const FADE_TICKS: u32 = 60;

enum Phase {
    Hold { ticks_remaining: u32 },
    Fade { ticks_remaining: u32 },
    Done,
}

pub struct VictoryScene {
    phase: Phase,
    skip_requested: bool,
}

impl VictoryScene {
    pub fn new() -> Self {
        VictoryScene {
            phase: Phase::Hold { ticks_remaining: HOLD_TICKS },
            skip_requested: false,
        }
    }
}

impl Default for VictoryScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for VictoryScene {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn handle_event(&mut self, event: &Event) -> bool {
        match event {
            Event::KeyDown { keycode: Some(Keycode::Space), .. }
            | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
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
        _game_lib: &GameLibrary,
        resources: &mut SceneResources<'_, '_>,
    ) -> SceneResult {
        if self.skip_requested {
            if !matches!(self.phase, Phase::Fade { .. } | Phase::Done) {
                self.phase = Phase::Fade { ticks_remaining: FADE_TICKS };
            }
            self.skip_requested = false;
        }

        // Paint the winpic into play_tex every frame (cheap; texture already
        // lives in VRAM).
        let _ = canvas.with_texture_canvas(play_tex, |play_canvas| {
            play_canvas.set_draw_color(Color::BLACK);
            play_canvas.clear();
            if let Some(img) = resources.find_image("winpic") {
                img.draw(play_canvas, 0, 0);
            }
        });

        // Apply fade color modulation to the composited texture.
        let (r, g, b) = match &self.phase {
            Phase::Fade { ticks_remaining } => {
                let t = (*ticks_remaining as f32 / FADE_TICKS as f32).clamp(0.0, 1.0);
                let v = (t * 255.0) as u8;
                (v, v, v)
            }
            _ => (255, 255, 255),
        };
        play_tex.set_color_mod(r, g, b);

        // Blit to screen (match the 640×400 area used by other full-screen
        // scenes — upscaled 2× from 320×200 play_tex).
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        let screen_dest = Rect::new(0, 40, 640, 400);
        canvas.copy(play_tex, None, Some(screen_dest)).unwrap();

        // Advance phase.
        match &mut self.phase {
            Phase::Hold { ticks_remaining } => {
                if delta_ticks >= *ticks_remaining {
                    self.phase = Phase::Fade { ticks_remaining: FADE_TICKS };
                } else {
                    *ticks_remaining -= delta_ticks;
                }
                SceneResult::Continue
            }
            Phase::Fade { ticks_remaining } => {
                if delta_ticks >= *ticks_remaining {
                    play_tex.set_color_mod(255, 255, 255);
                    self.phase = Phase::Done;
                } else {
                    *ticks_remaining -= delta_ticks;
                }
                SceneResult::Continue
            }
            Phase::Done => {
                play_tex.set_color_mod(255, 255, 255);
                SceneResult::Done
            }
        }
    }
}
