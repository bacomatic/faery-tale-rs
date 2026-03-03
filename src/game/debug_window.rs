
use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::VideoSubsystem;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use crate::game::colors::Palette;
use crate::game::debug_command::DebugCommand;
use crate::game::font::DiskFont;
use crate::game::font_texture::FontTexture;
use crate::game::game_clock::DayPhase;
use crate::game::placard::{self, Placard};
use crate::game::render_task::RenderTask;
use crate::game::settings::GameSettings;

const DEBUG_WINDOW_WIDTH: u32 = 660;
const DEBUG_WINDOW_HEIGHT: u32 = 520;
const TAB_BAR_HEIGHT: i32 = 18;

// ── Tab definitions ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DebugTab {
    Info,
    Placards,
    Images,
    Tilemap,
    Map,
    Songs,
    Player,
    Actors,
}

const ALL_TABS: [DebugTab; 8] = [
    DebugTab::Info,
    DebugTab::Placards,
    DebugTab::Images,
    DebugTab::Tilemap,
    DebugTab::Map,
    DebugTab::Songs,
    DebugTab::Player,
    DebugTab::Actors,
];

impl DebugTab {
    fn label(&self) -> &'static str {
        match self {
            DebugTab::Info => "Info",
            DebugTab::Placards => "Placards",
            DebugTab::Images => "Images",
            DebugTab::Tilemap => "Tilemap",
            DebugTab::Map => "Map",
            DebugTab::Songs => "Songs",
            DebugTab::Player => "Player",
            DebugTab::Actors => "Actors",
        }
    }

    fn key_hint(&self) -> &'static str {
        match self {
            DebugTab::Info => "1",
            DebugTab::Placards => "2",
            DebugTab::Images => "3",
            DebugTab::Tilemap => "4",
            DebugTab::Map => "5",
            DebugTab::Songs => "6",
            DebugTab::Player => "7",
            DebugTab::Actors => "8",
        }
    }
}

// ── State snapshot passed from main loop ─────────────────────────────

/// Copied hero scalar stats for display in the debug window.
#[derive(Debug, Clone, Default)]
pub struct HeroStats {
    pub vitality: i16,
    pub max_vitality: i16,
    pub brave: i16,
    pub luck: i16,
    pub kind: i16,
    pub wealth: i16,
    pub hunger: i16,
    pub fatigue: i16,
    pub brother: u8,
    pub riding: i16,
    pub flying: i16,
    pub hero_x: u16,
    pub hero_y: u16,
    pub hero_sector: u16,
    pub hero_place: u16,
    pub region_num: u8,
}

/// Snapshot of gameplay timers for display in the debug window.
#[derive(Debug, Clone, Default)]
pub struct TimerSnapshot {
    pub light_timer: i16,
    pub secret_timer: i16,
    pub freeze_timer: i16,
    pub daynight: u16,
    pub lightlevel: u16,
}

/// Compact actor info for display; avoids borrowing the full Actor.
#[derive(Debug, Clone, Default)]
pub struct ActorInfo {
    pub kind: String,
    pub state: String,
    pub abs_x: u16,
    pub abs_y: u16,
    pub vitality: i16,
    pub race: u8,
    pub weapon: u8,
}

/// Info snapshot passed into the debug window each frame.
/// Uses owned/copied values to avoid borrow conflicts in the main loop.
pub struct DebugState<'a> {
    // Timing (copied from GameClock)
    pub game_day: u32,
    pub game_hour: u32,
    pub game_minute: u32,
    pub day_phase: DayPhase,
    pub game_ticks: u64,
    pub mono_ticks: u64,
    pub paused: bool,
    pub fps: f64,

    // Scene
    pub scene_name: Option<&'a str>,

    // Placard tab data
    pub placard_names: &'a [String],
    pub current_placard: Option<&'a Placard>,
    pub sys_palette: &'a Palette,

    // Image tab data (metadata only; textures can't cross SDL2 renderers)
    pub image_names: &'a [String],
    pub image_dimensions: Option<(u32, u32)>,

    // Songs tab data
    pub song_group_count: usize,
    pub current_song_group: Option<usize>,

    // Gameplay snapshot fields (None before gameplay begins)
    pub hero_stats: Option<HeroStats>,
    pub inventory: Option<[u8; 35]>,
    pub actors: Option<Vec<ActorInfo>>,
    pub timers: Option<TimerSnapshot>,
    pub safe_pos: Option<(u16, u16, u8)>,
    pub god_mode_flags: u8,
    pub time_held: bool,
    pub autosave_enabled: bool,
}

// ── Per-tab state ────────────────────────────────────────────────────

struct PlacardTabState {
    current_index: usize,
    renderer: Option<placard::PlacardRenderer>,
    needs_redraw: bool,
}

struct ImageTabState {
    current_index: usize,
}

struct SongTabState {
    /// Highlighted (but not necessarily playing) group index (0-based).
    highlighted_group: usize,
}

/// Human-readable names for the 35 inventory slots (stuff[0..35]).
/// Slot assignments deduced from the original fmain.c source.
const ITEM_NAMES: [&str; 35] = [
    "Weapon",     // 0  - currently equipped weapon (1=dirk initially)
    "Item 1",     // 1
    "Item 2",     // 2
    "Item 3",     // 3
    "Item 4",     // 4
    "Raft",       // 5
    "Item 6",     // 6
    "Item 7",     // 7
    "Arrows",     // 8  - consumed when bow fires
    "Item 9",     // 9
    "Item 10",    // 10
    "Item 11",    // 11
    "Item 12",    // 12
    "Item 13",    // 13
    "Item 14",    // 14
    "Item 15",    // 15
    "Key(Gold)",  // 16 - KEYBASE
    "Key(Green)", // 17
    "Key(Blue)",  // 18
    "Key(Red)",   // 19
    "Key(Grey)",  // 20
    "Key(White)", // 21
    "Item 22",    // 22
    "Item 23",    // 23
    "Food",       // 24 - decremented by hunger
    "Item 25",    // 25 - STATBASE
    "Item 26",    // 26
    "Item 27",    // 27
    "Item 28",    // 28
    "Item 29",    // 29
    "Item 30",    // 30
    "Gold",       // 31 - GOLDBASE
    "Item 32",    // 32
    "Item 33",    // 33
    "Item 34",    // 34
];

/// Stat IDs in order matching the 7 stat display lines.
const STAT_LINE_IDS: [crate::game::debug_command::StatId; 7] = [
    crate::game::debug_command::StatId::Vitality,
    crate::game::debug_command::StatId::Brave,
    crate::game::debug_command::StatId::Luck,
    crate::game::debug_command::StatId::Kind,
    crate::game::debug_command::StatId::Wealth,
    crate::game::debug_command::StatId::Hunger,
    crate::game::debug_command::StatId::Fatigue,
];

struct PlayerTabState {
    inv_scroll: usize,
    /// Window-space y of the first stat line (set during render, used for click detection).
    stat_y_start_win: i32,
    /// Window-space y of the first inventory slot (set during render).
    inv_y_start_win: i32,
    /// Line height in pixels (set during render).
    line_h: i32,
}

struct ActorsTabState {
    scroll: usize,
}

// ── DebugWindow ──────────────────────────────────────────────────────

/// A separate SDL2 window dedicated to debug/diagnostic output,
/// fully isolated from the main game canvas.
pub struct DebugWindow<'a> {
    canvas: Canvas<Window>,
    window_id: u32,

    // Own font resources so the debug window is self-contained
    font_texture: Rc<RefCell<sdl2::render::Texture<'a>>>,
    font_text: Rc<RefCell<FontTexture<'a>>>,

    // Tab state
    active_tab: DebugTab,

    // FPS tracking
    frame_count: u64,
    last_fps_time: Instant,
    current_fps: f64,

    // Extra log lines that game code can push
    log_lines: Vec<String>,
    max_log_lines: usize,

    // Per-tab state
    placard_tab: PlacardTabState,
    image_tab: ImageTabState,
    song_tab: SongTabState,
    player_tab: PlayerTabState,
    actors_tab: ActorsTabState,

    /// Song group requested by the user from the Songs tab.
    /// Consumed by the main loop via `take_song_request()`.
    song_group_requested: Option<usize>,
    /// Stop-music request from the Songs tab.
    stop_requested: bool,

    /// Queued commands to apply to game state; consumed via `drain_commands()`.
    pending_commands: Vec<DebugCommand>,

    // Offscreen texture for placard rendering (320×200)
    placard_texture: sdl2::render::Texture<'a>,
}

impl<'a> DebugWindow<'a> {
    /// Create the debug window positioned to the right of the game window.
    pub fn new(
        video: &VideoSubsystem,
        font: &DiskFont,
        settings: &GameSettings,
        game_window_pos: Option<(i32, i32)>,
        game_window_size: (u32, u32),
    ) -> Result<DebugWindow<'a>, String> {
        // Use saved size, or fall back to defaults
        let (win_w, win_h) = settings.debug_window_size
            .unwrap_or((DEBUG_WINDOW_WIDTH, DEBUG_WINDOW_HEIGHT));

        // Use saved position, or position to the right of the game window
        let (dx, dy) = settings.debug_window_position.unwrap_or_else(|| {
            match game_window_pos {
                Some((gx, gy)) => (gx + game_window_size.0 as i32 + 10, gy),
                None => (0, 0),
            }
        });

        let mut wb = video.window("Debug", win_w, win_h);
        wb.position(dx, dy).resizable();
        let window = wb.build().map_err(|e| e.to_string())?;
        let window_id = window.id();

        let mut canvas = window
            .into_canvas()
            .accelerated()
            .build()
            .map_err(|e| e.to_string())?;
        canvas.set_draw_color(Color::RGB(30, 30, 30));
        canvas.clear();
        canvas.present();

        // Build a private font texture for the debug window
        let font_bounds = font.get_font_bounds();
        let tex_creator = canvas.texture_creator();

        // SAFETY: we transmute the texture lifetime so it can live alongside the canvas.
        let tex = tex_creator
            .create_texture_static(
                Some(PixelFormatEnum::RGBA32),
                font_bounds.width(),
                font_bounds.height(),
            )
            .map_err(|e| e.to_string())?;
        let tex: sdl2::render::Texture<'a> = unsafe { std::mem::transmute(tex) };

        let font_texture = Rc::new(RefCell::new(tex));
        let font_text = Rc::new(RefCell::new(FontTexture::new(
            font,
            &font_bounds,
            Rc::downgrade(&font_texture),
        )));

        // Create offscreen texture for placard rendering (target texture)
        let placard_tex = tex_creator
            .create_texture_target(Some(PixelFormatEnum::RGBA32), 320, 200)
            .map_err(|e| e.to_string())?;
        let placard_tex: sdl2::render::Texture<'a> = unsafe { std::mem::transmute(placard_tex) };

        Ok(DebugWindow {
            canvas,
            window_id,
            font_texture,
            font_text,
            active_tab: DebugTab::Info,
            frame_count: 0,
            last_fps_time: Instant::now(),
            current_fps: 0.0,
            log_lines: Vec::new(),
            max_log_lines: 24,
            placard_tab: PlacardTabState {
                current_index: 0,
                renderer: None,
                needs_redraw: true,
            },
            image_tab: ImageTabState { current_index: 0 },
            song_tab: SongTabState { highlighted_group: 0 },
            player_tab: PlayerTabState {
                inv_scroll: 0,
                stat_y_start_win: 0,
                inv_y_start_win: 0,
                line_h: 0,
            },
            actors_tab: ActorsTabState { scroll: 0 },
            song_group_requested: None,
            stop_requested: false,
            pending_commands: Vec::new(),
            placard_texture: placard_tex,
        })
    }

    /// Returns the SDL window id so the main loop can route events.
    pub fn window_id(&self) -> u32 {
        self.window_id
    }

    /// Returns the placard index currently selected in the debug window's Placards tab.
    pub fn placard_index(&self) -> usize {
        self.placard_tab.current_index
    }

    /// Returns the image index currently selected in the debug window's Images tab.
    pub fn image_index(&self) -> usize {
        self.image_tab.current_index
    }

    /// Consumes and returns a pending play-group request (group index 0–6), if any.
    pub fn take_song_request(&mut self) -> Option<usize> {
        self.song_group_requested.take()
    }

    /// Consumes and returns a pending stop-music request.
    pub fn take_stop_request(&mut self) -> bool {
        let v = self.stop_requested;
        self.stop_requested = false;
        v
    }

    /// Drains and returns all pending debug commands queued since last call.
    pub fn drain_commands(&mut self) -> Vec<DebugCommand> {
        self.pending_commands.drain(..).collect()
    }

    /// Push a log message that will be shown in the debug window.
    pub fn log(&mut self, msg: String) {
        self.log_lines.push(msg);
        if self.log_lines.len() > self.max_log_lines {
            self.log_lines.remove(0);
        }
    }

    /// Handle window events directed at the debug window.
    /// Returns true if the event was consumed.
    pub fn handle_event(&mut self, event: &Event, settings: &mut GameSettings) -> bool {
        match event {
            Event::Window { window_id, win_event, .. } if *window_id == self.window_id => {
                use sdl2::event::WindowEvent;
                match win_event {
                    WindowEvent::Moved(x, y) => {
                        settings.set_debug_window_position((*x, *y));
                    }
                    WindowEvent::Resized(w, h) => {
                        settings.set_debug_window_size((*w as u32, *h as u32));
                    }
                    _ => {}
                }
                true
            }

            // Keyboard: only consume if the debug window has focus
            Event::KeyDown {
                window_id,
                scancode: Some(sc),
                repeat: false,
                ..
            } if *window_id == self.window_id => {
                match sc {
                    // Tab switching with F1..F6
                    Scancode::F1 => { self.active_tab = DebugTab::Info; true }
                    Scancode::F2 => {
                        self.active_tab = DebugTab::Placards;
                        self.placard_tab.needs_redraw = true;
                        true
                    }
                    Scancode::F3 => { self.active_tab = DebugTab::Images; true }
                    Scancode::F4 => { self.active_tab = DebugTab::Tilemap; true }
                    Scancode::F5 => { self.active_tab = DebugTab::Map; true }
                    Scancode::F6 => { self.active_tab = DebugTab::Songs; true }
                    Scancode::F7 => { self.active_tab = DebugTab::Player; true }
                    Scancode::F8 => { self.active_tab = DebugTab::Actors; true }

                    // Left/Right to cycle items in content tabs
                    Scancode::Left | Scancode::Right => {
                        let fwd = *sc == Scancode::Right;
                        match self.active_tab {
                            DebugTab::Placards => {
                                self.placard_tab.needs_redraw = true;
                                if fwd {
                                    self.placard_tab.current_index =
                                        self.placard_tab.current_index.wrapping_add(1);
                                } else {
                                    self.placard_tab.current_index =
                                        self.placard_tab.current_index.wrapping_sub(1);
                                }
                                self.placard_tab.renderer = None; // restart border anim
                            }
                            DebugTab::Images => {
                                if fwd {
                                    self.image_tab.current_index =
                                        self.image_tab.current_index.wrapping_add(1);
                                } else {
                                    self.image_tab.current_index =
                                        self.image_tab.current_index.wrapping_sub(1);
                                }
                            }
                            _ => {}
                        }
                        true
                    }

                    // Number keys 1-7 in Songs tab: select/play a group (0-6)
                    Scancode::Num1 | Scancode::Num2 | Scancode::Num3 | Scancode::Num4 |
                    Scancode::Num5 | Scancode::Num6 | Scancode::Num7
                    if self.active_tab == DebugTab::Songs => {
                        let group = match sc {
                            Scancode::Num1 => 0,
                            Scancode::Num2 => 1,
                            Scancode::Num3 => 2,
                            Scancode::Num4 => 3,
                            Scancode::Num5 => 4,
                            Scancode::Num6 => 5,
                            Scancode::Num7 => 6,
                            _ => unreachable!(),
                        };
                        self.song_tab.highlighted_group = group;
                        self.song_group_requested = Some(group);
                        true
                    }

                    // Key 0 or S in Songs tab: stop music
                    Scancode::Num0 | Scancode::S
                    if self.active_tab == DebugTab::Songs => {
                        self.stop_requested = true;
                        true
                    }

                    // Up/Down to scroll inventory in Player tab and actor list in Actors tab
                    Scancode::Up | Scancode::Down => {
                        let down = *sc == Scancode::Down;
                        match self.active_tab {
                            DebugTab::Player => {
                                if down {
                                    if self.player_tab.inv_scroll + 1 < 35 {
                                        self.player_tab.inv_scroll += 1;
                                    }
                                } else if self.player_tab.inv_scroll > 0 {
                                    self.player_tab.inv_scroll -= 1;
                                }
                            }
                            DebugTab::Actors => {
                                if down {
                                    self.actors_tab.scroll = self.actors_tab.scroll.saturating_add(1);
                                } else {
                                    self.actors_tab.scroll = self.actors_tab.scroll.saturating_sub(1);
                                }
                            }
                            _ => {}
                        }
                        true
                    }

                    _ => true, // consume all other keys when debug window focused
                }
            }

            // Mouse clicks in Player tab: adjust stats (left half) or inventory slots
            Event::MouseButtonDown {
                window_id,
                x: _,
                y,
                mouse_btn,
                ..
            } if *window_id == self.window_id && self.active_tab == DebugTab::Player => {
                use sdl2::mouse::MouseButton;
                let lh = self.player_tab.line_h;
                if lh > 0 {
                    let stat_y = self.player_tab.stat_y_start_win;
                    let inv_y  = self.player_tab.inv_y_start_win;
                    let delta_i16 = if *mouse_btn == MouseButton::Left { 1i16 } else { -1i16 };
                    let delta_i8  = if *mouse_btn == MouseButton::Left { 1i8  } else { -1i8  };
                    if *y >= stat_y && *y < inv_y {
                        let line = ((*y - stat_y) / lh) as usize;
                        if line < STAT_LINE_IDS.len() {
                            self.pending_commands.push(
                                DebugCommand::AdjustStat { stat: STAT_LINE_IDS[line], delta: delta_i16 }
                            );
                        }
                    } else if *y >= inv_y {
                        let slot = ((*y - inv_y) / lh) as usize + self.player_tab.inv_scroll;
                        if slot < 35 {
                            self.pending_commands.push(
                                DebugCommand::AdjustInventory { index: slot as u8, delta: delta_i8 }
                            );
                        }
                    }
                }
                true
            }

            _ => false,
        }
    }

    /// Render one frame of the debug window.
    pub fn render(&mut self, state: &DebugState) {
        // Update FPS counter
        self.frame_count += 1;
        let elapsed = self.last_fps_time.elapsed().as_secs_f64();
        if elapsed >= 1.0 {
            self.current_fps = self.frame_count as f64 / elapsed;
            self.frame_count = 0;
            self.last_fps_time = Instant::now();
        }

        // Use actual window size so content scales when the window is resized.
        let (win_w, win_h) = self.canvas.window().size();

        // Clear background
        self.canvas.set_draw_color(Color::RGB(30, 30, 30));
        self.canvas.clear();
        self.canvas
            .set_viewport(Rect::new(0, 0, win_w, win_h));

        // Draw tab bar
        self.draw_tab_bar(win_w);

        // Set the content viewport below the tab bar
        let content_y = TAB_BAR_HEIGHT + 2;
        let content_h = win_h as i32 - content_y;
        self.canvas
            .set_viewport(Rect::new(0, content_y, win_w, content_h as u32));

        // Render the active tab
        match self.active_tab {
            DebugTab::Info => self.render_info_tab(state, win_w, win_h),
            DebugTab::Placards => self.render_placard_tab(state, win_w),
            DebugTab::Images => self.render_image_tab(state),
            DebugTab::Tilemap => self.render_stub_tab("Character / NPC Tilemap", "Not yet implemented"),
            DebugTab::Map => self.render_stub_tab("Map View", "Not yet implemented"),
            DebugTab::Songs => self.render_songs_tab(state, win_w, win_h),
            DebugTab::Player => self.render_player_tab(state, win_h),
            DebugTab::Actors => self.render_actors_tab(state, win_h),
        }

        self.canvas.present();
    }

    // ── Tab bar ──────────────────────────────────────────────────────

    fn draw_tab_bar(&mut self, win_w: u32) {
        self.canvas
            .set_viewport(Rect::new(0, 0, win_w, TAB_BAR_HEIGHT as u32));

        // Background
        self.canvas.set_draw_color(Color::RGB(50, 50, 50));
        self.canvas
            .fill_rect(Rect::new(0, 0, win_w, TAB_BAR_HEIGHT as u32))
            .ok();

        let font_ref = self.font_text.borrow();
        let char_w = font_ref.get_font().x_size as i32;
        let mut x: i32 = 4;

        for tab in &ALL_TABS {
            let is_active = *tab == self.active_tab;
            let label = format!("F{} {}", tab.key_hint(), tab.label());
            let label_px = label.len() as i32 * char_w;

            if is_active {
                self.canvas.set_draw_color(Color::RGB(80, 80, 120));
                self.canvas
                    .fill_rect(Rect::new(x - 2, 0, (label_px + 6) as u32, TAB_BAR_HEIGHT as u32))
                    .ok();
            }

            {
                let mut tex = self.font_texture.borrow_mut();
                if is_active {
                    tex.set_color_mod(255, 255, 255);
                } else {
                    tex.set_color_mod(140, 140, 140);
                }
            }

            font_ref.render_string(&label, &mut self.canvas, x, TAB_BAR_HEIGHT - 4);
            x += label_px + 10;
        }

        // Bottom separator
        self.canvas.set_draw_color(Color::RGB(100, 100, 100));
        self.canvas
            .draw_line(
                sdl2::rect::Point::new(0, TAB_BAR_HEIGHT - 1),
                sdl2::rect::Point::new(win_w as i32, TAB_BAR_HEIGHT - 1),
            )
            .ok();
    }

    // ── Info tab (original debug view) ───────────────────────────────

    fn render_info_tab(&mut self, state: &DebugState, _win_w: u32, win_h: u32) {
        let font_ref = self.font_text.borrow();
        let line_height: i32 = font_ref.get_font().y_size as i32 + 2;
        let left: i32 = 10;
        let mut y: i32 = 10;

        set_font_color(&self.font_texture, 0, 220, 80);
        font_ref.render_string("=== DEBUG ===", &mut self.canvas, left, y);
        y += line_height + 4;
        draw_separator(&mut self.canvas, y - 4);

        set_font_color(&self.font_texture, 180, 220, 255);

        let fps = format!("FPS: {:.1}  (debug)", self.current_fps);
        font_ref.render_string(&fps, &mut self.canvas, left, y);
        y += line_height;

        let gfps = format!("Game FPS: {:.1}", state.fps);
        font_ref.render_string(&gfps, &mut self.canvas, left, y);
        y += line_height;

        let clock = format!("Clock: Day {} {:02}:{:02}", state.game_day, state.game_hour, state.game_minute);
        font_ref.render_string(&clock, &mut self.canvas, left, y);
        y += line_height;

        let phase = format!("Phase: {:?}", state.day_phase);
        font_ref.render_string(&phase, &mut self.canvas, left, y);
        y += line_height;

        let gticks = format!("Game ticks: {}", state.game_ticks);
        font_ref.render_string(&gticks, &mut self.canvas, left, y);
        y += line_height;

        let mticks = format!("Mono ticks: {}", state.mono_ticks);
        font_ref.render_string(&mticks, &mut self.canvas, left, y);
        y += line_height;

        let pstr = if state.paused { "PAUSED" } else { "Running" };
        font_ref.render_string(&format!("State: {}", pstr), &mut self.canvas, left, y);
        y += line_height + 4;

        draw_separator(&mut self.canvas, y - 4);
        set_font_color(&self.font_texture, 255, 200, 100);

        let scene = match state.scene_name {
            Some(n) => format!("Scene: {}", n),
            None => "Scene: (none)".into(),
        };
        font_ref.render_string(&scene, &mut self.canvas, left, y);
        y += line_height + 4;

        draw_separator(&mut self.canvas, y - 4);
        set_font_color(&self.font_texture, 160, 160, 160);
        font_ref.render_string("-- Log --", &mut self.canvas, left, y);
        y += line_height;

        for line in &self.log_lines {
            font_ref.render_string(line, &mut self.canvas, left, y);
            y += line_height;
            if y > win_h as i32 - TAB_BAR_HEIGHT - line_height {
                break;
            }
        }
    }

    // ── Placard tab ──────────────────────────────────────────────────

    fn render_placard_tab(&mut self, state: &DebugState, win_w: u32) {
        let num_placards = state.placard_names.len();
        if num_placards == 0 {
            self.render_stub_tab("Placards", "No placards loaded");
            return;
        }

        // Wrap index
        self.placard_tab.current_index = self.placard_tab.current_index % num_placards;
        let idx = self.placard_tab.current_index;
        let name = &state.placard_names[idx];

        // Restart renderer if needed
        if self.placard_tab.needs_redraw {
            self.placard_tab.renderer = Some(placard::start_placard_renderer(
                &sdl2::rect::Point::new(0, 0),
                state.sys_palette,
            ));
            let _ = self.canvas.with_texture_canvas(&mut self.placard_texture, |tc| {
                tc.set_draw_color(Color::BLACK);
                tc.clear();
            });
            self.placard_tab.needs_redraw = false;
        }

        // Advance the placard border renderer into the offscreen texture
        {
            if let Some(ref mut r) = self.placard_tab.renderer {
                let _ = self.canvas.with_texture_canvas(&mut self.placard_texture, |tc| {
                    tc.set_viewport(Rect::new(16, 0, 288, 200));
                    r.update(tc, 1, None);
                });
            }
        }

        // Draw the placard text into the offscreen texture
        let placard = find_placard_by_name(state.placard_names, idx, state);
        if let Some(placard) = placard {
            let font_ref = self.font_text.borrow();
            let _ = self.canvas.with_texture_canvas(&mut self.placard_texture, |tc| {
                tc.set_viewport(Rect::new(16, 0, 288, 200));
                set_font_color(&self.font_texture, 220, 200, 160);
                placard.draw(&font_ref, tc);
            });
        }

        // Blit the offscreen texture into the debug window canvas, scaled
        let dest_w = win_w;
        let dest_h = (200.0 * (win_w as f64 / 320.0)) as u32;
        let dest_y = 30;
        self.canvas
            .copy(
                &self.placard_texture,
                None,
                Some(Rect::new(0, dest_y, dest_w, dest_h)),
            )
            .ok();

        // Label and navigation hint
        let font_ref = self.font_text.borrow();
        let left = 10;

        set_font_color(&self.font_texture, 255, 220, 100);
        let hdr = format!("Placard: {} ({}/{})", name, idx + 1, num_placards);
        font_ref.render_string(&hdr, &mut self.canvas, left, 6);

        set_font_color(&self.font_texture, 140, 140, 140);
        font_ref.render_string(
            "Left/Right to browse",
            &mut self.canvas,
            left,
            dest_y + dest_h as i32 + 8,
        );
    }

    // ── Image tab ────────────────────────────────────────────────────

    fn render_image_tab(&mut self, state: &DebugState) {
        let num_images = state.image_names.len();
        if num_images == 0 {
            self.render_stub_tab("Images", "No images loaded");
            return;
        }

        // Wrap index
        self.image_tab.current_index = self.image_tab.current_index % num_images;
        let idx = self.image_tab.current_index;
        let name = &state.image_names[idx];

        let font_ref = self.font_text.borrow();
        let line_height = font_ref.get_font().y_size as i32 + 2;
        let left = 10;
        let mut y = 6;

        set_font_color(&self.font_texture, 180, 220, 255);
        let hdr = format!("Image: {} ({}/{})", name, idx + 1, num_images);
        font_ref.render_string(&hdr, &mut self.canvas, left, y);
        y += line_height + 4;

        draw_separator(&mut self.canvas, y - 4);

        // Show image dimensions
        if let Some((w, h)) = state.image_dimensions {
            set_font_color(&self.font_texture, 220, 220, 220);
            let dim = format!("Dimensions: {}x{}", w, h);
            font_ref.render_string(&dim, &mut self.canvas, left, y);
            y += line_height;
        }

        y += line_height;
        set_font_color(&self.font_texture, 140, 140, 140);
        font_ref.render_string("Left/Right to browse", &mut self.canvas, left, y);
    }

    // ── Songs tab ────────────────────────────────────────────────────

    fn render_songs_tab(&mut self, state: &DebugState, win_w: u32, win_h: u32) {
        let font_ref = self.font_text.borrow();
        let line_height = font_ref.get_font().y_size as i32 + 2;
        let left = 10;
        let mut y = 6;

        set_font_color(&self.font_texture, 180, 220, 255);
        font_ref.render_string("Songs", &mut self.canvas, left, y);
        y += line_height + 4;

        draw_separator(&mut self.canvas, y - 4);

        let n = state.song_group_count;
        if n == 0 {
            set_font_color(&self.font_texture, 140, 140, 140);
            font_ref.render_string("No songs loaded", &mut self.canvas, left, y);
            return;
        }

        // Show current playback status
        let status = match state.current_song_group {
            Some(g) => format!("Playing: Group {} ({})", g + 1, song_group_label(g)),
            None    => "Stopped".to_string(),
        };
        set_font_color(&self.font_texture, 100, 220, 100);
        font_ref.render_string(&status, &mut self.canvas, left, y);
        y += line_height + 6;

        draw_separator(&mut self.canvas, y - 4);

        // List all groups; highlight the currently-playing one and the
        // highlighted (cursor) one.
        let highlighted = self.song_tab.highlighted_group;
        for g in 0..n {
            let is_playing  = state.current_song_group == Some(g);
            let is_cursor   = highlighted == g;

            // Background highlight for selected row
            if is_cursor || is_playing {
                let bg = if is_playing {
                    Color::RGB(40, 80, 40)
                } else {
                    Color::RGB(50, 50, 80)
                };
                self.canvas.set_draw_color(bg);
                self.canvas
                    .fill_rect(Rect::new(left - 2, y - 1, win_w.saturating_sub(14), line_height as u32 + 2))
                    .ok();
            }

            // Row text color
            if is_playing {
                set_font_color(&self.font_texture, 80, 255, 80);
            } else if is_cursor {
                set_font_color(&self.font_texture, 255, 255, 160);
            } else {
                set_font_color(&self.font_texture, 200, 200, 200);
            }

            let marker = if is_playing { ">" } else { " " };
            let row = format!("{} [{}] Group {} – {}", marker, g + 1, g + 1, song_group_label(g));
            font_ref.render_string(&row, &mut self.canvas, left, y);
            y += line_height;

            if y > win_h as i32 - TAB_BAR_HEIGHT - line_height {
                break;
            }
        }

        y += line_height;
        set_font_color(&self.font_texture, 140, 140, 140);
        font_ref.render_string(
            "1-7: play group  0/S: stop",
            &mut self.canvas,
            left,
            y,
        );
    }

    // ── Player tab (F7) ──────────────────────────────────────────────

    fn render_player_tab(&mut self, state: &DebugState, win_h: u32) {
        let font_ref = self.font_text.borrow();
        let line_h = font_ref.get_font().y_size as i32 + 2;
        let left = 10;
        let mut y = 6;
        const CONTENT_Y: i32 = TAB_BAR_HEIGHT + 2;

        set_font_color(&self.font_texture, 180, 220, 255);
        font_ref.render_string("Player", &mut self.canvas, left, y);
        y += line_h + 4;
        draw_separator(&mut self.canvas, y - 4);

        let Some(ref stats) = state.hero_stats else {
            set_font_color(&self.font_texture, 140, 140, 140);
            font_ref.render_string("Not in gameplay", &mut self.canvas, left, y);
            return;
        };
        let inventory = state.inventory.unwrap_or([0u8; 35]);

        // ── Stats panel ──
        set_font_color(&self.font_texture, 255, 220, 100);
        font_ref.render_string("Stats  (click to +1 / right-click to -1):", &mut self.canvas, left, y);
        y += line_h + 2;

        // Store window-space y of the first stat line for click detection.
        self.player_tab.stat_y_start_win = CONTENT_Y + y;
        self.player_tab.line_h = line_h;

        let stat_lines: [(&str, String); 7] = [
            ("VIT", format!("{}/{}", stats.vitality, stats.max_vitality)),
            ("BRV", format!("{}", stats.brave)),
            ("LCK", format!("{}", stats.luck)),
            ("KND", format!("{}", stats.kind)),
            ("WLT", format!("{}", stats.wealth)),
            ("HGR", format!("{}", stats.hunger)),
            ("FTG", format!("{}", stats.fatigue)),
        ];
        set_font_color(&self.font_texture, 200, 255, 200);
        for (label, value) in &stat_lines {
            let row = format!("  {:3}: {}", label, value);
            font_ref.render_string(&row, &mut self.canvas, left, y);
            y += line_h;
        }

        // Non-adjustable info lines
        set_font_color(&self.font_texture, 180, 180, 255);
        let bro_name = match stats.brother { 0 => "Julian", 1 => "Phillip", _ => "Kevin" };
        let info1 = format!("  BRO: {}  RGN: {}", bro_name, stats.region_num);
        font_ref.render_string(&info1, &mut self.canvas, left, y);
        y += line_h;
        let info2 = format!("  POS: ({}, {})  SEC: {}  PLACE: {}",
            stats.hero_x, stats.hero_y, stats.hero_sector, stats.hero_place);
        font_ref.render_string(&info2, &mut self.canvas, left, y);
        y += line_h + 4;

        draw_separator(&mut self.canvas, y - 4);

        // ── Inventory panel ──
        set_font_color(&self.font_texture, 255, 220, 100);
        font_ref.render_string("Inventory  (click +1, right-click -1, Up/Down scroll):", &mut self.canvas, left, y);
        y += line_h + 2;

        // Store window-space y of inventory start for click detection.
        self.player_tab.inv_y_start_win = CONTENT_Y + y;

        set_font_color(&self.font_texture, 210, 210, 210);
        let scroll = self.player_tab.inv_scroll;
        for i in scroll..35 {
            let count = inventory[i];
            let row = format!("  [{:2}] {:12}: {}", i, ITEM_NAMES[i], count);
            font_ref.render_string(&row, &mut self.canvas, left, y);
            y += line_h;
            if y > win_h as i32 - line_h {
                break;
            }
        }
    }

    // ── Actors tab (F8) ──────────────────────────────────────────────

    fn render_actors_tab(&mut self, state: &DebugState, win_h: u32) {
        let font_ref = self.font_text.borrow();
        let line_h = font_ref.get_font().y_size as i32 + 2;
        let left = 10;
        let mut y = 6;

        set_font_color(&self.font_texture, 180, 220, 255);
        font_ref.render_string("Actors", &mut self.canvas, left, y);
        y += line_h + 4;
        draw_separator(&mut self.canvas, y - 4);

        let Some(ref actors) = state.actors else {
            set_font_color(&self.font_texture, 140, 140, 140);
            font_ref.render_string("Not in gameplay", &mut self.canvas, left, y);
            return;
        };

        if actors.is_empty() {
            set_font_color(&self.font_texture, 140, 140, 140);
            font_ref.render_string("No active actors", &mut self.canvas, left, y);
            return;
        }

        let scroll = self.actors_tab.scroll.min(actors.len().saturating_sub(1));
        self.actors_tab.scroll = scroll;

        for (slot, actor) in actors.iter().enumerate().skip(scroll) {
            // Color coding by actor kind
            match actor.kind.as_str() {
                "Player"  => set_font_color(&self.font_texture, 80, 255, 80),
                "Enemy"   => set_font_color(&self.font_texture, 255, 80, 80),
                "SetFig"  => set_font_color(&self.font_texture, 255, 255, 80),
                "Object"  => set_font_color(&self.font_texture, 160, 160, 160),
                "Carrier" => set_font_color(&self.font_texture, 80, 255, 255),
                "Dragon"  => set_font_color(&self.font_texture, 255, 80, 255),
                _         => set_font_color(&self.font_texture, 220, 220, 220),
            }
            let row = format!(
                "[{:2}] {:8}  {:10}  ({:5},{:5})  VIT:{:4}  Race:{:2}  Wpn:{:2}",
                slot,
                actor.kind, actor.state,
                actor.abs_x, actor.abs_y,
                actor.vitality, actor.race, actor.weapon,
            );
            font_ref.render_string(&row, &mut self.canvas, left, y);
            y += line_h;
            if y > win_h as i32 - line_h * 2 {
                break;
            }
        }

        set_font_color(&self.font_texture, 140, 140, 140);
        font_ref.render_string("Up/Down to scroll", &mut self.canvas, left, win_h as i32 - line_h - 4);
    }

    // ── Stub tab (for unimplemented tabs) ────────────────────────────

    fn render_stub_tab(&mut self, title: &str, message: &str) {        let font_ref = self.font_text.borrow();
        let line_height = font_ref.get_font().y_size as i32 + 2;
        let left = 10;
        let mut y = 14;

        set_font_color(&self.font_texture, 255, 200, 100);
        font_ref.render_string(title, &mut self.canvas, left, y);
        y += line_height + 4;

        draw_separator(&mut self.canvas, y - 4);

        set_font_color(&self.font_texture, 140, 140, 140);
        font_ref.render_string(message, &mut self.canvas, left, y);
    }

    #[allow(dead_code)]
    fn draw_separator(&mut self, y: i32) {
        draw_separator(&mut self.canvas, y);
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn find_placard_by_name<'a>(
    names: &[String],
    index: usize,
    state: &DebugState<'a>,
) -> Option<&'a Placard> {
    if index < names.len() {
        state.current_placard
    } else {
        None
    }
}

fn set_font_color(font_texture: &Rc<RefCell<sdl2::render::Texture>>, r: u8, g: u8, b: u8) {
    let mut tex = font_texture.borrow_mut();
    tex.set_color_mod(r, g, b);
}

fn draw_separator(canvas: &mut Canvas<Window>, y: i32) {
    canvas.set_draw_color(Color::RGB(80, 80, 80));
    // Use the current viewport width (output_size tracks the active viewport).
    let (vp_w, _) = canvas.output_size().unwrap_or((660, 520));
    canvas
        .draw_line(
            sdl2::rect::Point::new(5, y),
            sdl2::rect::Point::new(vp_w as i32 - 5, y),
        )
        .ok();
}

/// Human-readable label for a song group index.
///
/// Group 3 is the intro music; the rest are identified by their track-slot
/// offset so they can be cross-referenced with the original source.
fn song_group_label(group: usize) -> &'static str {
    match group {
        0 => "Outdoor daytime",
        1 => "Battle",
        2 => "Outdoor nighttime",
        3 => "Intro sequence",
        4 => "Palace zone",
        5 => "Indoor / dungeon",
        6 => "Death / game over",
        _ => "Unknown",
    }
}
