
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use std::any::Any;

use crate::game::scene::{Scene, SceneResources, SceneResult};
use crate::game::game_library::GameLibrary;

/**
 * Copy protection scene.
 *
 * After the intro sequence, the original game asks 3 random questions
 * from a pool of 8. The player types an answer for each. If all 3
 * are correct, the game proceeds; otherwise it quits.
 *
 * The scene draws the `copy_junk` placard preamble text on a dark blue
 * background, then presents each question with an input cursor. Text
 * is rendered in the 320x200 play texture using the topaz font.
 *
 * Original palette: copyjunk — color 0 = 0x006 (dark blue), color 1 = 0xFFF (white)
 *
 * Question positions (from original): y = 125 + (question_num * 10)
 */

/// How long to display the preamble text before showing the first question.
const PREAMBLE_HOLD_TICKS: u32 = 90; // ~1.5 seconds

/// Maximum characters in an answer.
const MAX_ANSWER_LEN: usize = 9;

/// Cursor color — orange block cursor matching original game.
const CURSOR_COLOR: Color = Color::RGB(0xFF, 0x88, 0x00);

/// Background color matching the copyjunk palette color 0 (0x006).
const BG_COLOR: Color = Color::RGB(0x00, 0x00, 0x66);

/// Number of questions to ask.
const NUM_QUESTIONS: usize = 3;

/// Brief pause after passing/failing (ticks).
const RESULT_PAUSE_TICKS: u32 = 60;

enum CopyProtectPhase {
    /// Display the copy_junk placard preamble. Wait briefly before asking.
    ShowPreamble {
        ticks_remaining: u32,
        drawn: bool,
    },
    /// Ask a question, accept typed input.
    AskQuestion {
        /// Which question we're on (0, 1, 2).
        question_num: usize,
        /// Index into the copy_protect_junk array.
        question_index: usize,
        /// Whether the question text itself has been drawn to play_tex.
        question_drawn: bool,
    },
    /// Brief pause after all questions answered correctly.
    Passed { ticks_remaining: u32 },
    /// Answer was wrong — brief pause then done.
    Failed { ticks_remaining: u32 },
    /// Scene complete.
    Done { success: bool },
}

pub struct CopyProtectScene {
    phase: CopyProtectPhase,
    /// Indices of questions selected (without replacement) from the pool.
    selected_questions: Vec<usize>,
    /// Current typed input (shared across phases to avoid borrow issues).
    input: String,
    /// Set to true by handle_event when Return is pressed.
    submit_pending: bool,
    /// If true, skip the entire scene and succeed immediately.
    #[allow(dead_code)]
    skip: bool,
}

impl CopyProtectScene {
    /// Create a new copy protection scene.
    /// If `skip` is true, the scene immediately succeeds without
    /// asking any questions (for development convenience).
    pub fn new(skip: bool, question_count: usize) -> CopyProtectScene {
        // Select random questions without replacement
        let mut selected = Vec::new();
        let total = question_count;
        if total > 0 && !skip {
            use std::collections::HashSet;
            let mut used = HashSet::new();
            let mut attempts = 0;
            while selected.len() < NUM_QUESTIONS && selected.len() < total && attempts < 100 {
                let idx = simple_rand(total, attempts);
                if !used.contains(&idx) {
                    used.insert(idx);
                    selected.push(idx);
                }
                attempts += 1;
            }
        }

        CopyProtectScene {
            phase: if skip {
                CopyProtectPhase::Done { success: true }
            } else {
                CopyProtectPhase::ShowPreamble {
                    ticks_remaining: PREAMBLE_HOLD_TICKS,
                    drawn: false,
                }
            },
            selected_questions: selected,
            input: String::new(),
            submit_pending: false,
            skip,
        }
    }

    /// Check if the scene completed successfully (all answers correct).
    pub fn passed(&self) -> bool {
        match &self.phase {
            CopyProtectPhase::Done { success } => *success,
            _ => false,
        }
    }
}

/// Simple pseudo-random for question selection.
fn simple_rand(max: usize, salt: usize) -> usize {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize;
    (seed.wrapping_add(salt.wrapping_mul(7919))) % max
}

/// Convert a Keycode to an uppercase ASCII character, if it's a typeable character.
fn keycode_to_char(keycode: Keycode) -> Option<char> {
    match keycode {
        Keycode::A => Some('A'),
        Keycode::B => Some('B'),
        Keycode::C => Some('C'),
        Keycode::D => Some('D'),
        Keycode::E => Some('E'),
        Keycode::F => Some('F'),
        Keycode::G => Some('G'),
        Keycode::H => Some('H'),
        Keycode::I => Some('I'),
        Keycode::J => Some('J'),
        Keycode::K => Some('K'),
        Keycode::L => Some('L'),
        Keycode::M => Some('M'),
        Keycode::N => Some('N'),
        Keycode::O => Some('O'),
        Keycode::P => Some('P'),
        Keycode::Q => Some('Q'),
        Keycode::R => Some('R'),
        Keycode::S => Some('S'),
        Keycode::T => Some('T'),
        Keycode::U => Some('U'),
        Keycode::V => Some('V'),
        Keycode::W => Some('W'),
        Keycode::X => Some('X'),
        Keycode::Y => Some('Y'),
        Keycode::Z => Some('Z'),
        Keycode::Space => Some(' '),
        _ => None,
    }
}

impl Scene for CopyProtectScene {
    fn as_any(&self) -> &dyn Any { self }

    fn handle_event(&mut self, event: &Event) -> bool {
        match &self.phase {
            CopyProtectPhase::AskQuestion { .. } => {
                match event {
                    Event::KeyDown { keycode: Some(Keycode::Return), .. } => {
                        self.submit_pending = true;
                        true
                    }
                    Event::KeyDown { keycode: Some(Keycode::Backspace), .. } => {
                        self.input.pop();
                        true
                    }
                    Event::KeyDown { keycode: Some(kc), .. } => {
                        if let Some(ch) = keycode_to_char(*kc) {
                            if self.input.len() < MAX_ANSWER_LEN {
                                self.input.push(ch);
                            }
                        }
                        true
                    }
                    _ => false,
                }
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
        match &mut self.phase {
            CopyProtectPhase::ShowPreamble { ticks_remaining, drawn } => {
                if !*drawn {
                    let _ = canvas.with_texture_canvas(play_tex, |play_canvas| {
                        play_canvas.set_draw_color(BG_COLOR);
                        play_canvas.clear();

                        if let Some(placard) = game_lib.find_placard("copy_junk") {
                            placard.draw(resources.topaz_font, play_canvas);
                        }
                    });
                    *drawn = true;
                }

                canvas.set_draw_color(Color::BLACK);
                canvas.clear();
                let screen_dest = Rect::new(0, 40, 640, 400);
                canvas.copy(play_tex, None, Some(screen_dest)).unwrap();

                if delta_ticks >= *ticks_remaining {
                    *ticks_remaining = 0;
                } else {
                    *ticks_remaining -= delta_ticks;
                }

                if *ticks_remaining == 0 {
                    if !self.selected_questions.is_empty() {
                        let qi = self.selected_questions[0];
                        self.input.clear();
                        self.phase = CopyProtectPhase::AskQuestion {
                            question_num: 0,
                            question_index: qi,
                            question_drawn: false,
                        };
                    } else {
                        self.phase = CopyProtectPhase::Done { success: true };
                    }
                }

                SceneResult::Continue
            }

            CopyProtectPhase::AskQuestion {
                question_num,
                question_index,
                question_drawn,
            } => {
                let qnum = *question_num;
                let qidx = *question_index;
                let y_pos = 125 + (qnum as i32 * 10);
                let need_draw_question = !*question_drawn;
                *question_drawn = true;

                let question_text = if let Some(q) = game_lib.get_copy_protect_questions().get(qidx) {
                    q.question.clone()
                } else {
                    "???".to_string()
                };

                let current_input = self.input.clone();

                // The prompt is the question text (already ends with "...?")
                // followed by typed input on the same line
                let prompt_width = resources.topaz_font.string_width(&question_text);
                let input_x = 10 + prompt_width;

                let _ = canvas.with_texture_canvas(play_tex, |play_canvas| {
                    if need_draw_question {
                        resources.topaz_font.render_string(
                            &question_text,
                            play_canvas,
                            10,
                            y_pos,
                        );
                    }

                    // Clear and redraw input area on the same line, after the prompt
                    play_canvas.set_draw_color(BG_COLOR);
                    play_canvas.fill_rect(Rect::new(input_x, y_pos - 8, 200, 10)).unwrap();

                    // Draw typed text
                    if !current_input.is_empty() {
                        resources.topaz_font.render_string(
                            &current_input,
                            play_canvas,
                            input_x,
                            y_pos,
                        );
                    }

                    // Draw solid block cursor after the typed text
                    let input_width = resources.topaz_font.string_width(&current_input);
                    let cursor_x = input_x + input_width;
                    let font = resources.topaz_font.get_font();
                    let cursor_w = font.x_size as u32; // space-width for monospaced font
                    let cursor_h = font.y_size as u32;
                    play_canvas.set_draw_color(CURSOR_COLOR);
                    play_canvas.fill_rect(Rect::new(cursor_x, y_pos - font.baseline as i32, cursor_w, cursor_h)).unwrap();
                });

                canvas.set_draw_color(Color::BLACK);
                canvas.clear();
                let screen_dest = Rect::new(0, 40, 640, 400);
                canvas.copy(play_tex, None, Some(screen_dest)).unwrap();

                // Check for answer submission
                if self.submit_pending {
                    self.submit_pending = false;

                    // Clear the block cursor from the play texture before transitioning
                    let _ = canvas.with_texture_canvas(play_tex, |play_canvas| {
                        let input_width = resources.topaz_font.string_width(&current_input);
                        let cursor_x = input_x + input_width;
                        let font = resources.topaz_font.get_font();
                        let cursor_w = font.x_size as u32;
                        let cursor_h = font.y_size as u32;
                        play_canvas.set_draw_color(BG_COLOR);
                        play_canvas.fill_rect(Rect::new(cursor_x, y_pos - font.baseline as i32, cursor_w, cursor_h)).unwrap();
                    });

                    let correct = if let Some(q) = game_lib.get_copy_protect_questions().get(qidx) {
                        self.input.eq_ignore_ascii_case(&q.answer)
                    } else {
                        false
                    };

                    if correct {
                        let next_qnum = qnum + 1;
                        if next_qnum < NUM_QUESTIONS && next_qnum < self.selected_questions.len() {
                            let next_qi = self.selected_questions[next_qnum];
                            self.input.clear();
                            self.phase = CopyProtectPhase::AskQuestion {
                                question_num: next_qnum,
                                question_index: next_qi,
                                question_drawn: false,
                            };
                        } else {
                            self.phase = CopyProtectPhase::Passed {
                                ticks_remaining: RESULT_PAUSE_TICKS,
                            };
                        }
                    } else {
                        self.phase = CopyProtectPhase::Failed {
                            ticks_remaining: RESULT_PAUSE_TICKS,
                        };
                    }
                }

                SceneResult::Continue
            }

            CopyProtectPhase::Passed { ticks_remaining } => {
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();
                let screen_dest = Rect::new(0, 40, 640, 400);
                canvas.copy(play_tex, None, Some(screen_dest)).unwrap();

                if delta_ticks >= *ticks_remaining {
                    self.phase = CopyProtectPhase::Done { success: true };
                } else {
                    *ticks_remaining -= delta_ticks;
                }
                SceneResult::Continue
            }

            CopyProtectPhase::Failed { ticks_remaining } => {
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();
                let screen_dest = Rect::new(0, 40, 640, 400);
                canvas.copy(play_tex, None, Some(screen_dest)).unwrap();

                if delta_ticks >= *ticks_remaining {
                    self.phase = CopyProtectPhase::Done { success: false };
                } else {
                    *ticks_remaining -= delta_ticks;
                }
                SceneResult::Continue
            }

            CopyProtectPhase::Done { .. } => SceneResult::Done,
        }
    }
}
