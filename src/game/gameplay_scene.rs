//! Main gameplay scene: game loop, input, rendering.
//!
//! # Screen layout
//!
//! The original Amiga game used two Copper-switched viewports stacked vertically:
//! - `vp_page` (LORES, 288×140): the playfield
//! - `vp_text` (HIRES, 640×57): the HI bar (buttons, compass, messages)
//!
//! Both are 2× line-doubled (NTSC 30 Hz interlaced → line-doubled to fill 400 lines)
//! and centered in the SDL 640×480 logical canvas with 40px top/bottom margins:
//!
//! ```text
//!  y=  0.. 39  black margin (40px)
//!  y= 40..319  playfield   (576×280)  x=32..607 (DxOffset=16 LORES px × 2)
//!  y=320..325  gap         (6px)      3 LORES rows × 2
//!  y=326..439  HI bar      (640×114)  x=0..639  (57 HIRES rows × 2)
//!  y=440..479  black margin (40px)
//! ```
//!
//! See `RESEARCH.md § Screen Layout: Amiga Mixed-Resolution Viewports` for full details.
use crate::game::magic::{use_magic, ITEM_LANTERN, ITEM_ORB, ITEM_RING, ITEM_SKULL, ITEM_STONE_RING, ITEM_TOTEM, ITEM_VIAL};
use crate::game::map_renderer::MapRenderer;
use crate::game::message_queue::MessageQueue;
use std::any::Any;

/// Map an SDL Keycode to the corresponding menu key byte used by LETTER_LIST.
/// Numpad movement keys (Kp8/7/9/1/3) are excluded; only top-row Num8 maps to b'8'.
fn keycode_to_menukey(keycode: Keycode) -> Option<u8> {
    match keycode {
        Keycode::Space  => Some(b' '),
        Keycode::I      => Some(b'I'),
        Keycode::T      => Some(b'T'),
        Keycode::Slash  => Some(b'?'),
        Keycode::U      => Some(b'U'),
        Keycode::G      => Some(b'G'),
        Keycode::Y      => Some(b'Y'),
        Keycode::S      => Some(b'S'),
        Keycode::A      => Some(b'A'),
        Keycode::M      => Some(b'M'),
        Keycode::F      => Some(b'F'),
        Keycode::Q      => Some(b'Q'),
        Keycode::L      => Some(b'L'),
        Keycode::O      => Some(b'O'),
        Keycode::R      => Some(b'R'),
        Keycode::Num8   => Some(b'8'),  // top-row 8 only; Kp8 = MoveUp
        Keycode::C      => Some(b'C'),
        Keycode::W      => Some(b'W'),
        Keycode::B      => Some(b'B'),
        Keycode::E      => Some(b'E'),
        Keycode::V      => Some(b'V'),
        Keycode::X      => Some(b'X'),
        Keycode::Num1   => Some(b'1'),
        Keycode::Num2   => Some(b'2'),
        Keycode::Num3   => Some(b'3'),
        Keycode::Num4   => Some(b'4'),
        Keycode::Num5   => Some(b'5'),
        Keycode::Num6   => Some(b'6'),
        Keycode::Num7   => Some(b'7'),
        Keycode::K      => Some(b'K'),
        _ => None,
    }
}

/// Return the 8-way facing direction (0=N..7=NW) from (sx,sy) toward (tx,ty).
/// Mirrors fmain.c directional logic used when setting ms->direction.
fn facing_toward(sx: i32, sy: i32, tx: i32, ty: i32) -> u8 {
    let dx = tx - sx;
    let dy = ty - sy;
    let ax = dx.abs();
    let ay = dy.abs();
    if ax <= ay / 2 {
        if dy > 0 { 4 } else { 0 }   // S or N
    } else if ay <= ax / 2 {
        if dx > 0 { 2 } else { 6 }   // E or W
    } else {
        match (dx > 0, dy > 0) {
            (true,  true)  => 3, // SE
            (true,  false) => 1, // NE
            (false, true)  => 5, // SW
            (false, false) => 7, // NW
        }
    }
}

/// Return the name of the active brother (Julian=1, Phillip=2, Kevin=3).
fn brother_name(state: &crate::game::game_state::GameState) -> &'static str {
    match state.brother {
        1 => "Julian",
        2 => "Phillip",
        _ => "Kevin",
    }
}

/// Canvas layout — original 640×200 game area line-doubled to 640×400,
/// centered in 640×480 logical canvas with 40px margins top and bottom.
///
/// Playfield (vp_page, LORES 288×140 px): DxOffset=16 LORES × 2 = 32px left margin;
/// 2× line-doubled to canvas rect (32, 40, MAP_DST_W*2, MAP_DST_H*2).
///
/// Gap: 3 original LORES rows × 2 = 6px at canvas y=320–325.
///
/// HI bar (vp_text, HIRES 640×57 px): also 2× line-doubled → 640×114;
/// canvas rect (0, 326, 640, 114). Internal coords (buttons, compass) scale ×2 vertically.
const CANVAS_MARGIN_Y: i32 = 40;
const PLAYFIELD_X: i32 = 32;              // vp_page DxOffset=16 LORES px × 2
const PLAYFIELD_Y: i32 = CANVAS_MARGIN_Y; // = 40
/// Visible LORES playfield dimensions — vp_page.DWidth/DHeight from fmain.c.
/// The framebuf (MAP_DST_W×MAP_DST_H) is larger; only this sub-rect is shown.
const PLAYFIELD_LORES_W: u32 = 288;    // vp_page.DWidth
const PLAYFIELD_LORES_H: u32 = 140;    // vp_page.DHeight
const PLAYFIELD_CANVAS_W: u32 = PLAYFIELD_LORES_W * 2; // 576
const PLAYFIELD_CANVAS_H: u32 = PLAYFIELD_LORES_H * 2; // 280
const HIBAR_NATIVE_H: u32 = 57;        // vp_text source height (HIRES rows)
const HIBAR_H: u32 = HIBAR_NATIVE_H * 2; // 114 — 2× line-doubled on canvas
const HIBAR_Y: i32 = CANVAS_MARGIN_Y + PLAYFIELD_CANVAS_H as i32 + 6; // 40 + 280 + 6 = 326

/// Day/night phase derived from lightlevel triangle wave (0–300).
/// lightlevel is a *brightness* value: 0 = midnight (dark), 300 = noon (bright).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DayNightPhase {
    Night, // lightlevel 0–59   (darkest)
    Dawn,  // 60–149  (brightening)
    Day,   // 150–299 (bright)
    Dusk,  // unused; reserved for symmetrical dusk transition
}

impl DayNightPhase {
    pub fn from_lightlevel(level: u16) -> Self {
        match level {
            0..=59  => Self::Night,
            60..=149 => Self::Dawn,
            _        => Self::Day,
        }
    }
}

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::game::actor::{ActorKind, ActorState, Goal};
use crate::game::collision;
use crate::game::debug_command::{BrotherId, DebugCommand, GodModeFlags, MagicEffect, StatId};
use crate::game::gfx_effects::{TeleportEffect, WitchEffect};
use crate::game::game_library::GameLibrary;
use crate::game::game_state::GameState;
use crate::game::key_bindings::{GameAction, KeyBindings};
use crate::game::scene::{Scene, SceneResources, SceneResult};

/// State for the key rebinding mode (F2 to enter, Escape to exit).
pub struct RebindingState {
    pub active: bool,
    pub waiting_for_action: Option<GameAction>,
}

/// 8-way movement direction decoded from input state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    N, NE, E, SE, S, SW, W, NW, None,
}

/// Tracks which movement/action keys are currently held down.
struct InputState {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    fight: bool,
    /// True while the player is holding a compass arrow (mouse-down); cleared on mouse-up.
    compass_held: bool,
}

impl Default for InputState {
    fn default() -> Self {
        InputState { up: false, down: false, left: false, right: false, fight: false, compass_held: false }
    }
}

pub struct GameplayScene {
    pub state: Box<GameState>,
    pub messages: MessageQueue,
    tick_accum: u32,
    autosave_enabled: bool,
    input: InputState,
    map_x: u16,
    map_y: u16,
    last_mood: u8,
    mood_tick: u32,
    music_stop_pending: bool,
    map_renderer: Option<MapRenderer>,
    map_world: Option<crate::game::world_data::WorldData>,
    adf: Option<crate::game::adf::AdfDisk>,
    shadow_mem: Vec<u8>,
    adf_load_attempted: bool,
    rebinding: RebindingState,
    local_bindings: KeyBindings,
    last_region_num: u8,
    palette_transition: Option<crate::game::palette::PaletteTransition>,
    last_indoor: bool,
    pub in_encounter_zone: bool,
    pub npc_table: Option<crate::game::npc::NpcTable>,
    day_night_phase: DayNightPhase,
    /// RGBA32 palette for the final indexed→RGBA32 render step.
    current_palette: crate::game::palette::Palette,
    /// Base palette loaded from faery.toml (colors::Palette with RGB4 values).
    /// Used as input to fade_page() for day/night/jewel palette computation.
    /// None until init_from_library() runs.
    base_colors_palette: Option<crate::game::colors::Palette>,
    /// Dirty-check key for current_palette: (lightlevel, light_on, secret_active).
    /// When any of these change, current_palette is recomputed.
    last_palette_key: (u16, bool, bool),

    witch_effect: WitchEffect,
    teleport_effect: TeleportEffect,
    pub missiles: [crate::game::combat::Missile; crate::game::combat::MAX_MISSILES],
    /// Frames remaining before next melee swing can land (rate-limits continuous fight).
    fight_cooldown: u32,
    /// Frames remaining before an archer NPC can fire again.
    archer_cooldown: u32,
    /// Debug log lines buffered for the debug window. Drained each frame by main loop.
    log_buffer: Vec<String>,
    /// Set to true when the player requests to quit the game.
    quit_requested: bool,
    /// Game is paused (Space key toggles).
    paused: bool,
    /// Compass direction sub-regions from comptable (for highlight overlay).
    compass_regions: Vec<(i32, i32, i32, i32)>,
    menu: crate::game::menu::MenuState,
    textcolors: crate::game::palette::Palette,
    /// Loaded sprite sheets indexed by cfile_idx (None = not yet loaded).
    sprite_sheets: Vec<Option<crate::game::sprites::SpriteSheet>>,
    /// Objects sprite sheet (cfile 3, 16×16 frames) — used for inventory screen.
    object_sprites: Option<crate::game::sprites::SpriteSheet>,
    /// Narrative strings from faery.toml [narr], used by event_msg / speak helpers.
    narr: crate::game::game_library::NarrConfig,
    /// Door table from faery.toml [[doors]], used for region transition checks.
    doors: Vec<crate::game::doors::DoorEntry>,
    /// Indices into `doors` of doors that have been opened (bump phase complete).
    /// Player must make a second movement attempt to cross the threshold and teleport.
    /// Cleared on every region transition.
    opened_doors: std::collections::HashSet<usize>,
    /// Index of the door whose "It's locked." message was last shown.
    /// Prevents the message from repeating every frame while the player holds a direction key.
    /// Reset when the player successfully moves or is no longer blocked by a door tile.
    bumped_door: Option<usize>,
    /// Zone configs from faery.toml, used for event zone entry detection.
    zones: Vec<crate::game::game_library::ZoneConfig>,
    /// Index of the zone the hero was in last frame (None = no zone).
    last_zone: Option<usize>,
    /// Hero is in forced sleep (events 12/24).
    sleeping: bool,
    /// Hero is submerged in water.
    submerged: bool,
    /// Ticks while submerged (for drowning damage cadence).
    drowning_timer: u32,
    /// Death countdown active (goodfairy timer running).
    dying: bool,
    /// Goodfairy countdown: starts at 255, decrements every other tick (~30Hz).
    /// When reaches 0, luck check determines faery revive or next brother.
    goodfairy: i16,
}

impl GameplayScene {
    pub fn new() -> Self {
        GameplayScene {
            state: Box::new(GameState::new()),
            messages: MessageQueue::new(),
            tick_accum: 0,
            autosave_enabled: true,
            input: InputState::default(),
            map_x: 0,
            map_y: 0,
            last_mood: u8::MAX,
            mood_tick: 0,
            music_stop_pending: false,
            map_renderer: None,
            map_world: None,
            adf: None,
            shadow_mem: Vec::new(),
            adf_load_attempted: false,
            rebinding: RebindingState { active: false, waiting_for_action: None },
            local_bindings: KeyBindings::default_bindings(),
            last_region_num: u8::MAX,
            palette_transition: None,
            last_indoor: false,
            in_encounter_zone: false,
            npc_table: None,
            day_night_phase: DayNightPhase::Day,
            current_palette: [0xFF808080_u32; crate::game::palette::PALETTE_SIZE],
            base_colors_palette: None,
            last_palette_key: (u16::MAX, false, false),

            witch_effect: WitchEffect::new(),
            teleport_effect: TeleportEffect::new(),
            missiles: std::array::from_fn(|_| crate::game::combat::Missile::default()),
            fight_cooldown: 0,
            archer_cooldown: 0,
            log_buffer: Vec::new(),
            quit_requested: false,
            paused: false,
            compass_regions: Vec::new(),
            menu: crate::game::menu::MenuState::new(),
            textcolors: [0u32; 32],
            sprite_sheets: (0..crate::game::sprites::CFILE_COUNT).map(|_| None).collect(),
            object_sprites: None,
            narr: crate::game::game_library::NarrConfig::default(),
            doors: Vec::new(),
            opened_doors: std::collections::HashSet::new(),
            bumped_door: None,
            zones: Vec::new(),
            last_zone: None,
            sleeping: false,
            submerged: false,
            drowning_timer: 0,
            dying: false,
            goodfairy: 0,
        }
    }

    /// Apply config-driven brother stats and spawn location from the game library.
    /// Must be called once after construction so that the first brother (Julian)
    /// gets the correct stats from faery.toml instead of hard-coded defaults.
    pub fn init_from_library(&mut self, game_lib: &GameLibrary) {
        if let Some(bro) = game_lib.get_brother(0) {
            let (sx, sy, sr) = game_lib.find_location(&bro.spawn)
                .map(|loc| (loc.x, loc.y, loc.region))
                .unwrap_or((self.state.hero_x, self.state.hero_y, self.state.region_num));
            self.state.init_first_brother(
                bro.brave, bro.luck, bro.kind, bro.wealth, sx, sy, sr,
            );
        }

        if let Some(compass) = game_lib.get_compass() {
            self.compass_regions = compass.comptable.regions.iter()
                .map(|r| (r.x, r.y, r.w, r.h))
                .collect();
        }

        if let Some(pal) = game_lib.find_palette("textcolors") {
            for (i, color) in pal.colors.iter().enumerate().take(32) {
                self.textcolors[i] = ((color.r() as u32) << 16)
                    | ((color.g() as u32) << 8)
                    | (color.b() as u32);
            }
        }

        self.narr = game_lib.narr.clone();
        self.doors = game_lib.doors.iter().map(|d| crate::game::doors::DoorEntry {
            src_region: d.src_region,
            src_x:      d.src_x,
            src_y:      d.src_y,
            dst_region: d.dst_region,
            dst_x:      d.dst_x,
            dst_y:      d.dst_y,
            door_type:  d.door_type,
        }).collect();
        self.zones = game_lib.zones.clone();

        let stuff = self.state.stuff().clone();
        let wealth = self.state.wealth;
        self.menu.set_options(&stuff, wealth);
    }

    /// Returns true when it is daytime (lightlevel >= 40).
    /// Original: ob_listg[5] lantern activates when lightlevel < 40 (fmain.c:2375-2378).
    pub fn is_daytime(state: &GameState) -> bool {
        state.lightlevel >= 40
    }

    /// Push a debug/status message to the log buffer (shown in debug window).
    fn dlog(&mut self, msg: impl Into<String>) {
        self.log_buffer.push(msg.into());
    }

    /// Drain buffered debug log lines. Called by the main loop to forward to the debug window.
    pub fn drain_logs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.log_buffer)
    }

    /// Whether the witch screen-warp effect is active.
    pub fn is_witch_active(&self) -> bool { self.witch_effect.active }

    /// Whether the teleport flash/fade effect is active.
    pub fn is_teleport_active(&self) -> bool { self.teleport_effect.active }

    /// Whether a palette crossfade (region transition) is in progress.
    pub fn is_palette_xfade_active(&self) -> bool { self.palette_transition.is_some() }

    /// Enable or disable echoing every new message to stdout (--echo-transcript flag).
    pub fn set_echo_transcript(&mut self, echo: bool) {
        self.messages.set_echo(echo);
    }

    /// Decode 8-way direction from current input flags.
    fn current_direction(&self) -> Direction {
        match (self.input.up, self.input.down, self.input.left, self.input.right) {
            (true,  false, false, false) => Direction::N,
            (true,  false, false, true)  => Direction::NE,
            (false, false, false, true)  => Direction::E,
            (false, true,  false, true)  => Direction::SE,
            (false, true,  false, false) => Direction::S,
            (false, true,  true,  false) => Direction::SW,
            (false, false, true,  false) => Direction::W,
            (true,  false, true,  false) => Direction::NW,
            _                            => Direction::None,
        }
    }

    /// Apply player input: move hero and update actor facing/state.
    fn apply_player_input(&mut self) {
        if self.sleeping { return; }
        let dir = self.current_direction();

        // Per-direction base deltas from original xdir/ydir tables (fsubs.asm:1277-1278).
        // Applied as: delta = base * speed / 2  →  cardinal=3px, diagonal=2px at speed=2.
        let (base_dx, base_dy): (i32, i32) = match dir {
            Direction::N    => ( 0, -3),
            Direction::NE   => ( 2, -2),
            Direction::E    => ( 3,  0),
            Direction::SE   => ( 2,  2),
            Direction::S    => ( 0,  3),
            Direction::SW   => (-2,  2),
            Direction::W    => (-3,  0),
            Direction::NW   => (-2, -2),
            Direction::None => ( 0,  0),
        };

        let prev_x = self.state.hero_x;
        let prev_y = self.state.hero_y;

        // Stagger when starving (hunger > 120, 1-in-4 chance)
        let dir = if self.state.hunger > 120 && dir != Direction::None && (self.state.cycle & 3) == 0 {
            let r = (self.state.cycle >> 2) & 1;
            let f = if r == 0 {
                (self.state.facing + 1) & 7
            } else {
                (self.state.facing + 7) & 7
            };
            let facing_to_dir = |f: u8| match f {
                0 => Direction::N,  1 => Direction::NE, 2 => Direction::E,  3 => Direction::SE,
                4 => Direction::S,  5 => Direction::SW, 6 => Direction::W,  7 => Direction::NW,
                _ => Direction::None,
            };
            self.state.facing = f;
            facing_to_dir(f)
        } else {
            dir
        };

        if dir != Direction::None {
            // Speed: flying=4px, on_raft=2px (water passable), water terrain (type 2-5)=1px, default=2px.
            let speed: i32 = if self.state.flying != 0 {
                4
            } else if self.state.on_raft {
                2
            } else if let Some(ref world) = self.map_world {
                let terrain = collision::px_to_terrain_type(
                    world,
                    self.state.hero_x as i32,
                    self.state.hero_y as i32,
                );
                if (2..=5).contains(&terrain) { 1 } else { 2 }
            } else {
                2
            };


            let dx = base_dx * speed / 2;
            let dy = base_dy * speed / 2;
            // Outdoor world wraps at MAXCOORD = 0x8000 = 32768 (USHORT arithmetic).
            // Indoor maps (region >= 8) use y coordinates in the 0x8000–0x9FFF range;
            // wrapping would collapse them to 0–0x1FFF and break doorfind_exit matching.
            let new_x = (self.state.hero_x as i32 + dx).rem_euclid(0x8000) as u16;
            let new_y = if self.state.region_num < 8 {
                (self.state.hero_y as i32 + dy).rem_euclid(0x8000) as u16
            } else {
                (self.state.hero_y as i32 + dy) as u16
            };

            // Turtle guardrail: turtle rides water but cannot enter hard-block terrain (mountains).
            let turtle_blocked = self.state.on_raft
                && self.state.active_carrier == crate::game::game_state::CARRIER_TURTLE
                && self.map_world.as_ref().map_or(false, |world| {
                    collision::px_to_terrain_type(world, new_x as i32, new_y as i32) == 1
                });

            if !turtle_blocked && (self.state.flying != 0 || self.state.on_raft || collision::proxcheck(self.map_world.as_ref(), new_x as i32, new_y as i32)) {
                self.state.hero_x = new_x;
                self.state.hero_y = new_y;
                // Successful move — hero is no longer blocked by a door, reset dedup flag.
                self.bumped_door = None;
                if self.state.region_num >= 8 {
                    // Indoor (region >= 8): exit check — match on grid-aligned dst coords.
                    // Mirrors fmain.c indoor branch: xtest = hero_x & 0xFFF0, ytest = hero_y & 0xFFE0.
                    if let Some(door) = crate::game::doors::doorfind_exit(&self.doors, new_x, new_y) {
                        let (ex, ey) = crate::game::doors::exit_spawn(&door);
                        let outdoor_region = Self::outdoor_region_from_pos(ex, ey);
                        self.state.region_num = outdoor_region;
                        self.state.hero_x = ex;
                        self.state.hero_y = ey;
                        self.dlog(format!("door: indoor exit to region {} ({}, {})", outdoor_region, ex, ey));
                    }
                } else if let Some(door) = crate::game::doors::doorfind(&self.doors, self.state.region_num, new_x, new_y) {
                    // Outdoor (region < 8): walk-on entry check — match on src coords.
                    // Sub-tile position guard mirrors fmain.c Phase-2 nodoor conditions:
                    //   Horizontal (type & 1): skip if hero_y & 0x10 != 0 (lower half — not through yet)
                    //   Vertical             : skip if hero_x & 15 > 6   (right portion — not through yet)
                    let in_doorway = if door.door_type & 1 != 0 {
                        new_y & 0x10 == 0  // horizontal: upper half
                    } else {
                        new_x & 15 <= 6    // vertical: left portion
                    };
                    // DESERT doors (oasis) require 5 gold statues; original silently blocks if < 5.
                    use crate::game::doors::{key_req, KeyReq};
                    let allow = in_doorway && match key_req(door.door_type) {
                        KeyReq::GoldStatues => self.state.stuff()[25] >= 5,
                        _ => true, // walk-on path: door was already opened by bump; NOKEY always allowed
                    };
                    if allow {
                        let (ix, iy) = crate::game::doors::entry_spawn(&door);
                        self.state.region_num = door.dst_region;
                        self.state.hero_x = ix;
                        self.state.hero_y = iy;
                        self.dlog(format!("door: region transition to {}", door.dst_region));
                    }
                }
                // Outdoor region transition: recompute region from position after every move.
                // Mirrors gen_mini() in fmain.c — region switches when the hero crosses a
                // sector-grid boundary, not via an explicit trigger.  Only runs for outdoor
                // regions; door transitions to F9/F10 (>= 8) are handled above and must not
                // be overridden.
                if self.state.region_num < 8 {
                    let pos_region = Self::outdoor_region_from_pos(
                        self.state.hero_x, self.state.hero_y,
                    );
                    if pos_region != self.state.region_num {
                        self.dlog(format!(
                            "outdoor region transition: {} -> {} at ({}, {})",
                            self.state.region_num, pos_region,
                            self.state.hero_x, self.state.hero_y,
                        ));
                        self.state.region_num = pos_region;
                    }
                }
                // Track safe spawn point after successful movement.
                if let Some(ref world) = self.map_world {
                    let terrain = collision::px_to_terrain_type(
                        world, self.state.hero_x as i32, self.state.hero_y as i32,
                    );
                    self.state.update_safe_spawn(terrain);
                }
            } else if !turtle_blocked {
                // Check if movement was blocked by a door tile (terrain type 15).
                // Mirrors fmain.c: proxcheck returns 15 → doorfind(xtest, ytest, 0).
                //
                // Two-phase door model (matches original behaviour):
                //   Phase 1 — Bump:      show "It opened." / "It's locked.", record in opened_doors.
                //   Phase 2 — Walk-through: next movement attempt sees opened_doors entry → teleport.
                //
                // This mirrors fmain.c where doorfind() changes sector_mem tiles (making the
                // tile passable) and the actual xfer() teleport fires on the next frame's door scan.
                let right_t = self.map_world.as_ref().map_or(0, |w|
                    collision::px_to_terrain_type(w, new_x as i32 + 4, new_y as i32 + 2));
                let left_t  = self.map_world.as_ref().map_or(0, |w|
                    collision::px_to_terrain_type(w, new_x as i32 - 4, new_y as i32 + 2));
                let door_tile = right_t == 15 || left_t == 15;
                // probe_x: the probe point that found terrain-15 (used for tile-origin alignment).
                let probe_x = if right_t == 15 { new_x as i32 + 4 } else { new_x as i32 - 4 };
                let probe_y = new_y as i32 + 2;
                if door_tile && self.state.region_num < 8 {
                    // Indoor exit is handled by the walk-on branch above (mirrors fmain.c: door
                    // scan runs on hero_x/hero_y after every successful move).
                    use crate::game::doors::{doorfind_nearest_by_bump_radius, key_req, KeyReq,
                                             apply_door_tile_replacement};
                    let region = self.state.region_num;
                    let nearest = doorfind_nearest_by_bump_radius(
                        &self.doors, region, new_x, new_y);
                    if let Some((idx, door)) = nearest {
                        if self.opened_doors.contains(&idx) {
                            // Phase 2 — door was opened; let the hero cross the threshold.
                            // Mirrors fmain.c every-frame door scan sub-tile position check:
                            //   Horizontal (type & 1): teleport only when hero_y & 0x10 == 0
                            //     (upper half of tile — hero walks from lower half into upper).
                            //   Vertical: teleport only when hero_x & 15 <= 6
                            //     (within left portion of tile — hero walks in from right).
                            // Use new_y/new_x (proposed blocked position) as the equivalent
                            // of the original's post-move hero_y/hero_x.
                            let sub_tile_ok = if door.door_type & 1 != 0 {
                                new_y & 0x10 == 0  // horizontal: upper half
                            } else {
                                new_x & 15 <= 6    // vertical: left portion
                            };
                            if sub_tile_ok {
                                let (ix, iy) = crate::game::doors::entry_spawn(&door);
                                self.state.region_num = door.dst_region;
                                self.state.hero_x = ix;
                                self.state.hero_y = iy;
                                self.opened_doors.remove(&idx);
                                self.bumped_door = None;
                                self.dlog(format!("door: walk-through to region {}", door.dst_region));
                            }
                        } else {
                            // Phase 1 — attempt to open the door.
                            match key_req(door.door_type) {
                                KeyReq::NoKey => {
                                    // Freely-opening doors (wood, city gates, caves, stairs).
                                    if let Some(ref mut world) = self.map_world {
                                        apply_door_tile_replacement(world, door.door_type, probe_x, probe_y);
                                    }
                                    self.messages.push("It opened.");
                                    self.opened_doors.insert(idx);
                                    self.bumped_door = None;
                                    self.dlog(format!("door: opened idx={idx}"));
                                }
                                KeyReq::Key(_) | KeyReq::Talisman => {
                                    // Locked: show message once per approach (mirrors fmain.c bumped flag).
                                    if self.bumped_door != Some(idx) {
                                        self.messages.push("It's locked.");
                                        self.bumped_door = Some(idx);
                                    }
                                }
                                KeyReq::GoldStatues => {
                                    // DESERT/oasis: silently blocks if < 5 gold statues
                                    // (original fmain.c: `if (d->type == DESERT && stuff[STATBASE] < 5) break;`)
                                    if self.state.stuff()[25] >= 5 {
                                        if let Some(ref mut world) = self.map_world {
                                            apply_door_tile_replacement(world, door.door_type, probe_x, probe_y);
                                        }
                                        self.messages.push("It opened.");
                                        self.opened_doors.insert(idx);
                                        self.bumped_door = None;
                                        self.dlog(format!("door: oasis opened idx={idx}"));
                                    }
                                }
                            }
                        }
                    }
                    // No doorlist entry in range: silently block.
                } else {
                    // Not a door block — reset the locked-message dedup.
                    self.bumped_door = None;
                }
            }

            let facing: u8 = match dir {
                Direction::N  => 0,
                Direction::NE => 1,
                Direction::E  => 2,
                Direction::SE => 3,
                Direction::S  => 4,
                Direction::SW => 5,
                Direction::W  => 6,
                Direction::NW => 7,
                Direction::None => 0,
            };

            let moved = self.state.hero_x != prev_x || self.state.hero_y != prev_y;
            if let Some(player) = self.state.actors.first_mut() {
                player.facing = facing;
                player.moving = moved;
                if self.input.fight {
                    player.state = ActorState::Fighting(0);
                } else {
                    player.state = ActorState::Walking;
                }
            }
            self.state.facing = facing;
        } else {
            if let Some(player) = self.state.actors.first_mut() {
                player.moving = false;
                if self.input.fight {
                    player.state = ActorState::Fighting(0);
                } else {
                    player.state = ActorState::Still;
                }
            }
        }

        // Actual movement result (computed after the branch above).
        let moved = self.state.hero_x != prev_x || self.state.hero_y != prev_y;

        // Melee combat when fight is held (npc-103).
        // Rate-limited to one swing every 10 ticks (~1/3 s at 30 Hz), matching
        // fmain.c's per-frame proximity check gated by weapon animation state.
        if self.fight_cooldown > 0 {
            self.fight_cooldown -= 1;
        }
        if self.input.fight && self.fight_cooldown == 0 {
            self.apply_melee_combat();
            self.fight_cooldown = 10;
        }

        // Raft proximity detection (player-107).
        // Mirrors fmain.c: raftprox=1 within 16px, raftprox=2 within 9px of raft actor.
        // Auto-boards when hero is adjacent to a raft NPC; auto-disembarks on dry land.
        {
            let hx = self.state.hero_x as i32;
            let hy = self.state.hero_y as i32;
            let raft_close = self.npc_table.as_ref().map_or(false, |t| {
                t.npcs.iter().any(|n| {
                    n.active
                        && n.npc_type == crate::game::npc::NPC_TYPE_RAFT
                        && (n.x as i32 - hx).abs() < 16
                        && (n.y as i32 - hy).abs() < 16
                })
            });
            let raft_aboard = self.npc_table.as_ref().map_or(false, |t| {
                t.npcs.iter().any(|n| {
                    n.active
                        && n.npc_type == crate::game::npc::NPC_TYPE_RAFT
                        && (n.x as i32 - hx).abs() < 9
                        && (n.y as i32 - hy).abs() < 9
                })
            });
            if raft_aboard {
                self.state.raftprox = 2;
                self.state.active_carrier = crate::game::game_state::CARRIER_RAFT;
                self.state.on_raft = true;
            } else if raft_close {
                self.state.raftprox = 1;
            } else {
                self.state.raftprox = 0;
                // Auto-disembark from raft when hero reaches dry land (player-107).
                if self.state.on_raft
                    && self.state.active_carrier == crate::game::game_state::CARRIER_RAFT
                {
                    let on_land = self.map_world.as_ref().map_or(false, |world| {
                        collision::px_to_terrain_type(
                            world,
                            self.state.hero_x as i32,
                            self.state.hero_y as i32,
                        ) < 2
                    });
                    if on_land {
                        self.state.leave_raft();
                    }
                }
            }
        }

        // Water submersion check (#105)
        if !self.state.on_raft && self.state.flying == 0 {
            let terrain = if let Some(ref world) = self.map_world {
                collision::px_to_terrain_type(
                    world, self.state.hero_x as i32, self.state.hero_y as i32,
                )
            } else { 0 };
            self.submerged = terrain == 2;
        } else {
            self.submerged = false;
        }

        // Per-step fatigue: +1 when moving, -1 resting. Returns true on forced sleep.
        if self.state.fatigue_step(moved) {
            // Forced sleep: "just couldn't stay awake any longer!" (event 12)
            let bname = brother_name(&self.state);
            let msg = crate::game::events::event_msg(&self.narr, 12, bname);
            if !msg.is_empty() { self.messages.push(msg); }
            self.sleeping = true;
        }
    }

    /// Return the nearest active NPC within `range` world units (Chebyshev), or None.
    /// Mirrors original nearest_fig() / calc_dist() from fmain.c:4167-4272.
    fn nearest_npc_in_range(&self, range: i32) -> Option<&crate::game::npc::Npc> {
        let hx = self.state.hero_x as i32;
        let hy = self.state.hero_y as i32;
        self.npc_table.as_ref()?.npcs.iter()
            .filter(|n| n.active)
            .filter(|n| {
                let dx = (n.x as i32 - hx).abs();
                let dy = (n.y as i32 - hy).abs();
                dx.max(dy) <= range
            })
            .min_by_key(|n| {
                let dx = (n.x as i32 - hx).abs();
                let dy = (n.y as i32 - hy).abs();
                dx.max(dy)
            })
    }

    /// Helper: buy one unit of item_idx from a nearby shopkeeper (npc-107).
    /// Mirrors fmain.c BUY case: check race==0x88, wealth>j, stuff[i]++.
    fn do_buy(
        state: &mut GameState,
        npc_table: &Option<crate::game::npc::NpcTable>,
        item_idx: usize,
        item_name: &str,
        messages: &mut crate::game::message_queue::MessageQueue,
    ) {
        let hero_x = state.hero_x as i16;
        let hero_y = state.hero_y as i16;
        let near_shop = npc_table.as_ref().map_or(false, |t| {
            crate::game::shop::has_shopkeeper_nearby(&t.npcs, hero_x, hero_y)
        });
        if near_shop {
            match crate::game::shop::buy_item(state, item_idx) {
                Ok(cost) => {
                    messages.push(format!("Bought {} for {} gold.", item_name, cost));
                }
                Err(reason) => {
                    messages.push(format!("Cannot buy {}: {}", item_name, reason));
                }
            }
        } else {
            messages.push("No shopkeeper nearby.");
        }
    }

    /// Apply one melee swing against nearby enemy NPCs (npc-103).
    /// Ports fmain.c sword proximity loop + dohit + checkdead.
    fn apply_melee_combat(&mut self) {
        use crate::game::combat::{in_melee_range, melee_rand};
        use crate::game::debug_command::GodModeFlags;

        // Hero weapon value from actor[0] (default 1 = fists).
        let arms = self.state.actors.first().map_or(1u8, |a| a.weapon.max(1));
        let brave = self.state.brave;
        let facing = self.state.facing;
        let hero_x = self.state.hero_x as i16;
        let hero_y = self.state.hero_y as i16;
        let one_hit_kill = self.state.god_mode.contains(GodModeFlags::ONE_HIT_KILL);
        let insane_reach = self.state.god_mode.contains(GodModeFlags::INSANE_REACH);

        let mut hit_any = false;
        if let Some(ref mut table) = self.npc_table {
            for npc in table.npcs.iter_mut().filter(|n| n.active) {
                if !in_melee_range(hero_x, hero_y, facing, arms, brave,
                                   npc.x, npc.y, insane_reach) {
                    continue;
                }
                // damage = rand() % (arms + 1), min 1 (from task spec / dohit wt).
                let damage: i16 = if one_hit_kill {
                    npc.vitality
                } else {
                    (melee_rand(arms as u32 + 1) as i16).max(1)
                };
                npc.vitality -= damage;
                if npc.vitality < 0 { npc.vitality = 0; }
                // checkdead: vitality <= 0 → mark dead, award brave (fmain.c checkdead).
                if npc.vitality == 0 {
                    npc.active = false;
                    // brave++ on enemy kill (original: if i != 0 { brave++; }).
                    self.state.brave = (self.state.brave + 1).min(100);
                    // npc-106: roll treasure_probs[] drop on kill.
                    let npc_snap = npc.clone();
                    let tick = self.state.tick_counter;
                    if let Some(drop) = crate::game::loot::roll_treasure(&npc_snap, tick) {
                        let weapon_slot = crate::game::loot::award_treasure(&mut self.state, &drop);
                        // Auto-equip dropped weapon if it's better than current (fmain.c body search).
                        if let Some(w) = weapon_slot {
                            let cur = self.state.actors.first().map_or(0, |a| a.weapon);
                            if w > cur {
                                if let Some(player) = self.state.actors.first_mut() {
                                    player.weapon = w;
                                }
                                self.dlog(format!("found better weapon type {}", w));
                            }
                        }
                        self.dlog(format!("enemy slain, bravery now {}", self.state.brave));
                    } else {
                        self.dlog(format!(
                            "enemy slain, bravery now {}", self.state.brave
                        ));
                    }
                } else {
                    self.dlog(format!("combat hit for {}", damage));
                }
                hit_any = true;
                break; // one hit per swing (fmain.c breaks after first hit)
            }
        }
        let _ = hit_any; // no "miss" message — matches original silent miss
    }

    /// Advance all active actors by one frame.
    /// Actor 0 is always the player; actors 1..anix are NPCs with goal-based AI.
    fn update_actors(&mut self, _delta: u32) {
        let hero_x = self.state.hero_x as i32;
        let hero_y = self.state.hero_y as i32;
        // Skip actor 0 (player); apply goal-based movement to NPC actors.
        let anix = self.state.anix;
        for actor in self.state.actors[1..anix.max(1)].iter_mut() {
            if !actor.is_active() {
                continue;
            }
            let ax = actor.abs_x as i32;
            let ay = actor.abs_y as i32;
            let dx = hero_x - ax;
            let dy = hero_y - ay;
            let (vx, vy): (i16, i16) = match actor.goal {
                // Hostile: move toward hero (ATTACK1/ATTACK2/ARCHER1/ARCHER2/GUARD)
                Goal::Attack1 | Goal::Attack2 | Goal::Archer1 | Goal::Archer2 | Goal::Guard => {
                    if dx.abs() > dy.abs() {
                        (dx.signum() as i16, 0)
                    } else {
                        (0, dy.signum() as i16)
                    }
                }
                // Flee: move directly away from hero
                Goal::Flee => {
                    if dx.abs() > dy.abs() {
                        (-(dx.signum() as i16), 0)
                    } else {
                        (0, -(dy.signum() as i16))
                    }
                }
                // Follower/Leader: follow hero but stop when adjacent
                Goal::Follower | Goal::Leader => {
                    if dx.abs() > 32 || dy.abs() > 32 {
                        (dx.signum() as i16, dy.signum() as i16)
                    } else {
                        (0, 0)
                    }
                }
                // Stand, Wait, User, None: stationary
                Goal::Stand | Goal::User | Goal::None => (0, 0),
            };
            actor.vel_x = vx;
            actor.vel_y = vy;
            actor.abs_x = actor.abs_x.wrapping_add_signed(vx);
            actor.abs_y = actor.abs_y.wrapping_add_signed(vy);
            actor.moving = vx != 0 || vy != 0;
        }
        if let Some(ref mut table) = self.npc_table {
            let hero_x = self.state.hero_x as i16;
            let hero_y = self.state.hero_y as i16;
            let mut any_approach = false;
            for npc in &mut table.npcs {
                let adjacent = npc.tick(hero_x, hero_y);
                if adjacent && npc.active {
                    any_approach = true;
                }
            }
            if any_approach {
                self.dlog("enemy approaches".to_string());
            }
        }

        // npc-105: Archer NPCs (Goal::Archer1/Archer2) fire missiles toward hero.
        // Rate-limited: one shot per NPC group every 30 ticks (~1s at 30Hz),
        // mirroring fmain.c state >= SHOOT1 with ms->speed = 3.
        if self.archer_cooldown > 0 {
            self.archer_cooldown -= 1;
        } else {
            let hero_x = self.state.hero_x as i32;
            let hero_y = self.state.hero_y as i32;
            let anix = self.state.anix;
            for actor in self.state.actors[1..anix.max(1)].iter() {
                if !actor.is_active() { continue; }
                if !matches!(actor.goal, Goal::Archer1 | Goal::Archer2) { continue; }
                let ax = actor.abs_x as i32;
                let ay = actor.abs_y as i32;
                // Fire only when hero is within 150px (Chebyshev distance).
                if (hero_x - ax).abs().max((hero_y - ay).abs()) > 150 { continue; }
                let dir = facing_toward(ax, ay, hero_x, hero_y);
                use crate::game::combat::fire_missile;
                fire_missile(&mut self.missiles, ax, ay, dir, 3, false);
                self.archer_cooldown = 15;
                break; // one archer fires per cycle
            }
        }
    }

    /// Render the HI bar (stats, messages, buttons, compass) into the canvas at HIBAR_Y.
    /// Called for both normal play (viewstatus 0) and inventory screen (viewstatus 4).
    fn render_hibar(&mut self, canvas: &mut Canvas<Window>, resources: &mut SceneResources<'_, '_>) {
        let brave    = self.state.brave;
        let luck     = self.state.luck;
        let kind     = self.state.kind;
        let vitality = self.state.vitality;
        let wealth   = self.state.wealth;
        let buttons = self.menu.print_options();
        let msg_count = self.messages.len().min(4);
        let msgs: Vec<&str> = self.messages.iter().collect();
        let msg_start = msgs.len().saturating_sub(4);
        let msgs_visible: Vec<&str> = msgs[msg_start..].to_vec();
        let textcolors = &self.textcolors;
        let compass_regions = &self.compass_regions;
        let input_comptable_dir: usize = match self.current_direction() {
            Direction::NW   => 0,
            Direction::N    => 1,
            Direction::NE   => 2,
            Direction::E    => 3,
            Direction::SE   => 4,
            Direction::S    => 5,
            Direction::SW   => 6,
            Direction::W    => 7,
            Direction::None => 9,
        };
        let hiscreen_opt = resources.find_image("hiscreen");
        let amber_font = resources.amber_font;
        let topaz_font = resources.topaz_font;
        let compass_normal = resources.compass_normal;
        let compass_highlight = resources.compass_highlight;

        let tc = canvas.texture_creator();
        if let Ok(mut hibar_tex) = tc.create_texture_target(
            sdl2::pixels::PixelFormatEnum::RGBA32, 640, HIBAR_NATIVE_H,
        ) {
            let _ = canvas.with_texture_canvas(&mut hibar_tex, |hc| {
                hc.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
                hc.clear();

                if let Some(hiscreen) = hiscreen_opt {
                    hiscreen.draw_scaled(hc, sdl2::rect::Rect::new(0, 0, 640, HIBAR_NATIVE_H));
                } else {
                    hc.set_draw_color(sdl2::pixels::Color::RGB(80, 60, 20));
                    hc.fill_rect(sdl2::rect::Rect::new(0, 0, 640, HIBAR_NATIVE_H)).ok();
                }

                amber_font.set_color_mod(0xAA, 0x55, 0x00);
                amber_font.render_string(&format!("Brv:{:3}", brave),     hc, 14,  52);
                amber_font.render_string(&format!("Lck:{:3}", luck),      hc, 90,  52);
                amber_font.render_string(&format!("Knd:{:3}", kind),      hc, 168, 52);
                amber_font.render_string(&format!("Vit:{:3}", vitality),  hc, 245, 52);
                amber_font.render_string(&format!("Wlth:{:3}", wealth),   hc, 321, 52);

                for (i, msg) in msgs_visible.iter().enumerate() {
                    let line_from_bottom = (msg_count - 1 - i) as i32;
                    let y = 42 - line_from_bottom * 10;
                    amber_font.render_string(msg, hc, 16, y);
                }

                for btn in &buttons {
                    let col = btn.display_slot & 1;
                    let row = btn.display_slot / 2;
                    let btn_x = if col == 0 { 430i32 } else { 482i32 };
                    let btn_y = (row as i32) * 9 + 8;
                    let bg_rgba = textcolors[btn.bg_color as usize];
                    let bg = (((bg_rgba >> 16) & 0xFF) as u8, ((bg_rgba >> 8) & 0xFF) as u8, (bg_rgba & 0xFF) as u8);
                    let fg_rgba = textcolors[btn.fg_color as usize];
                    let fg = (((fg_rgba >> 16) & 0xFF) as u8, ((fg_rgba >> 8) & 0xFF) as u8, (fg_rgba & 0xFF) as u8);
                    topaz_font.render_string_with_bg("      ", hc, btn_x, btn_y, bg, fg);
                    topaz_font.set_color_mod(fg.0, fg.1, fg.2);
                    topaz_font.render_string(&btn.text, hc, btn_x + 4, btn_y);
                    topaz_font.set_color_mod(255, 255, 255);
                }

                const COMPASS_X: i32 = 567;
                const COMPASS_SRC_Y: i32 = 15;
                const COMPASS_SRC_W: u32 = 48;
                const COMPASS_SRC_H: u32 = 24;
                let compass_dest = sdl2::rect::Rect::new(COMPASS_X, COMPASS_SRC_Y, COMPASS_SRC_W, COMPASS_SRC_H);
                if let Some(normal_tex) = compass_normal {
                    hc.copy(normal_tex, None, compass_dest).ok();
                }
                if input_comptable_dir < compass_regions.len() {
                    let (rx, ry, rw, rh) = compass_regions[input_comptable_dir];
                    if rw > 1 || rh > 1 {
                        if let Some(highlight_tex) = compass_highlight {
                            let src = sdl2::rect::Rect::new(rx, ry, rw as u32, rh as u32);
                            let dst = sdl2::rect::Rect::new(COMPASS_X + rx, COMPASS_SRC_Y + ry, rw as u32, rh as u32);
                            hc.copy(highlight_tex, src, dst).ok();
                        }
                    }
                }
            });
            canvas.copy(
                &hibar_tex,
                sdl2::rect::Rect::new(0, 0, 640, HIBAR_NATIVE_H),
                sdl2::rect::Rect::new(0, HIBAR_Y, 640, HIBAR_H),
            ).ok();
        }; // semicolon: drops Result<Texture> temporary before tc is dropped
    }

    /// Clear and color the canvas according to the current viewstatus mode.
    fn render_by_viewstatus(&mut self, canvas: &mut Canvas<Window>, resources: &mut SceneResources<'_, '_>) {
        match self.state.viewstatus {
            // Normal play or forced redraw
            0 | 98 | 99 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
                canvas.clear();
                // Blit composed map framebuf to canvas (world-105).
                if let Some(ref mr) = self.map_renderer {
                    if !mr.framebuf.is_empty() {
                        // Apply current_palette: indexed u8 → RGBA32 bytes for SDL2.
                        let pal = &self.current_palette;
                        let mut rgb_buf: Vec<u8> = Vec::with_capacity(mr.framebuf.len() * 4);
                        for &idx in &mr.framebuf {
                            let rgba = pal[(idx & 31) as usize];
                            // ARGB8888 on little-endian: memory bytes are [B, G, R, A]
                            rgb_buf.push((rgba & 0xFF) as u8);
                            rgb_buf.push(((rgba >> 8) & 0xFF) as u8);
                            rgb_buf.push(((rgba >> 16) & 0xFF) as u8);
                            rgb_buf.push(0xFF);
                        }
                        let tc = canvas.texture_creator();
                        let surface_result = sdl2::surface::Surface::from_data(
                            &mut rgb_buf,
                            crate::game::map_renderer::MAP_DST_W,
                            crate::game::map_renderer::MAP_DST_H,
                            crate::game::map_renderer::MAP_DST_W * 4,
                            sdl2::pixels::PixelFormatEnum::ARGB8888,
                        );
                        if let Ok(surface) = surface_result {
                            if let Ok(tex) = tc.create_texture_from_surface(&surface) {
                                let src = sdl2::rect::Rect::new(0, 0, PLAYFIELD_LORES_W, PLAYFIELD_LORES_H);
                                let dst = sdl2::rect::Rect::new(
                                    PLAYFIELD_X, PLAYFIELD_Y,
                                    PLAYFIELD_CANVAS_W, PLAYFIELD_CANVAS_H,
                                );
                                let _ = canvas.copy(&tex, Some(src), Some(dst));
                            }
                        }
                    }
                }

                self.render_hibar(canvas, resources);

                // Tick visual effects and composite them over the map.
                self.witch_effect.tick();
                if let Some((r, g, b, a)) = self.teleport_effect.tick() {
                    canvas.set_draw_color(sdl2::pixels::Color::RGBA(r, g, b, a));
                    canvas.fill_rect(None).ok();
                }
            }
            // Map view (bird totem)
            1 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
                canvas.clear();

                if let Some(ref world) = self.map_world {
                    let buf = crate::game::map_view::bigdraw(
                        self.state.hero_x, self.state.hero_y, world,
                    );
                    let mut pixels_u8: Vec<u8> = Vec::with_capacity(buf.len() * 4);
                    for &px in &buf {
                        pixels_u8.push((px & 0xFF) as u8);
                        pixels_u8.push(((px >> 8) & 0xFF) as u8);
                        pixels_u8.push(((px >> 16) & 0xFF) as u8);
                        pixels_u8.push(0xFF);
                    }
                    let tc = canvas.texture_creator();
                    let surface_result = sdl2::surface::Surface::from_data(
                        &mut pixels_u8,
                        crate::game::map_view::BIGDRAW_COLS as u32,
                        crate::game::map_view::BIGDRAW_ROWS as u32,
                        (crate::game::map_view::BIGDRAW_COLS * 4) as u32,
                        sdl2::pixels::PixelFormatEnum::ARGB8888,
                    );
                    if let Ok(surface) = surface_result {
                        if let Ok(tex) = tc.create_texture_from_surface(&surface) {
                            let dst = sdl2::rect::Rect::new(32, 40, 576, 144);
                            let _ = canvas.copy(&tex, None, Some(dst));
                        }
                    }
                }

                // Hero position marker (center of the map view)
                canvas.set_draw_color(sdl2::pixels::Color::RGB(255, 255, 255));
                let hero_px = 32 + 576 / 2;
                let hero_py = 40 + 144 / 2;
                let _ = canvas.draw_line(
                    sdl2::rect::Point::new(hero_px - 4, hero_py),
                    sdl2::rect::Point::new(hero_px + 4, hero_py),
                );
                let _ = canvas.draw_line(
                    sdl2::rect::Point::new(hero_px, hero_py - 4),
                    sdl2::rect::Point::new(hero_px, hero_py + 4),
                );

                self.render_hibar(canvas, resources);
            }
            // Message overlay
            2 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(48, 48, 48));
                canvas.clear();
                // "MESSAGE" — text rendering pending font wiring
            }
            // Inventory screen (viewstatus=4): black play area with item sprites, normal HI bar.
            // Original: do_option() ITEMS hit=5 — clears playfield to black, blits item sprites
            // from seq_list[OBJECTS] using inv_list[] layout, then stillscreen() + viewstatus=4.
            4 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
                canvas.clear();

                // Build a 320×200 lores canvas with item sprites at their inv_list positions.
                // Items use the objects sprite sheet (cfile 3, 16×16 frames).
                if let Some(ref obj_sheet) = self.object_sprites {
                    use crate::game::sprites::{INV_LIST, OBJ_SPRITE_H, SPRITE_W};
                    const LORES_W: usize = 320;
                    const LORES_H: usize = 200;
                    // Index 31 = transparent background.
                    let mut inv_indices = vec![31u8; LORES_W * LORES_H];
                    let stuff = *self.state.stuff();

                    for (j, item) in INV_LIST.iter().enumerate() {
                        let count = stuff[j] as usize;
                        if count == 0 { continue; }
                        let num = count.min(item.maxshown as usize);
                        let frame = item.image_number as usize;
                        if let Some(frame_pix) = obj_sheet.frame_pixels(frame) {
                            let mut dst_y = item.yoff as i32;
                            for _ in 0..num {
                                let dst_x = item.xoff as i32 + 20;
                                for row in 0..item.img_height as usize {
                                    let src_row = item.img_off as usize + row;
                                    if src_row >= OBJ_SPRITE_H { break; }
                                    let py = dst_y + row as i32;
                                    if py < 0 || py >= LORES_H as i32 { continue; }
                                    for col in 0..SPRITE_W {
                                        let px = dst_x + col as i32;
                                        if px < 0 || px >= LORES_W as i32 { continue; }
                                        let src_idx = frame_pix[src_row * SPRITE_W + col];
                                        if src_idx != 31 {
                                            inv_indices[py as usize * LORES_W + px as usize] = src_idx;
                                        }
                                    }
                                }
                                dst_y += item.ydelta as i32;
                            }
                        }
                    }

                    // Apply palette: indexed u8 → RGBA32 bytes for SDL2.
                    let pal = &self.current_palette;
                    let mut rgb_buf: Vec<u8> = Vec::with_capacity(LORES_W * LORES_H * 4);
                    for &idx in &inv_indices {
                        let rgba = if idx == 31 {
                            0u32 // transparent background → black
                        } else {
                            pal[(idx & 31) as usize]
                        };
                        rgb_buf.push((rgba & 0xFF) as u8);
                        rgb_buf.push(((rgba >> 8) & 0xFF) as u8);
                        rgb_buf.push(((rgba >> 16) & 0xFF) as u8);
                        rgb_buf.push(0xFF);
                    }
                    // Blit the lores inventory canvas to the playfield rect (clip x=16, scale 2×).
                    let tc = canvas.texture_creator();
                    if let Ok(surface) = sdl2::surface::Surface::from_data(
                        &mut rgb_buf,
                        LORES_W as u32, LORES_H as u32,
                        LORES_W as u32 * 4,
                        sdl2::pixels::PixelFormatEnum::ARGB8888,
                    ) {
                        if let Ok(tex) = tc.create_texture_from_surface(&surface) {
                            let src = sdl2::rect::Rect::new(
                                16, 0, PLAYFIELD_LORES_W, PLAYFIELD_LORES_H,
                            );
                            let dst = sdl2::rect::Rect::new(
                                PLAYFIELD_X, PLAYFIELD_Y,
                                PLAYFIELD_CANVAS_W, PLAYFIELD_CANVAS_H,
                            );
                            let _ = canvas.copy(&tex, Some(src), Some(dst));
                        }
                    }; // semicolon: drops Result<Surface> temporary before rgb_buf is dropped
                }

                self.render_hibar(canvas, resources);
            }
            _ => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
                canvas.clear();
            }
        }
    }

    /// Called when the hero transitions to a new region.
    /// Reloads world data and NPC table for the new region (npc-101, world-110).
    fn on_region_changed(&mut self, region: u8, game_lib: &GameLibrary) {
        self.log_buffer.push(format!("on_region_changed: region changed to {}", region));
        // Reset door interaction state: all opened doors and the locked-message dedup.
        self.opened_doors.clear();
        self.bumped_door = None;
        if let Some(ref adf) = self.adf {
            let world_result = if let Some(cfg) = game_lib.find_region_config(region) {
                let map_blocks: Vec<u32> = if region < 8 {
                    Self::outdoor_map_blocks(game_lib)
                } else {
                    vec![cfg.map_block]
                };
                crate::game::world_data::WorldData::load(
                    adf, region,
                    cfg.sector_block, &map_blocks,
                    cfg.terra_block, cfg.terra2_block,
                    &cfg.image_blocks,
                )
            } else {
                Err(anyhow::anyhow!("no region config for region {}", region))
            };
            match world_result {
                Ok(world) => {
                    self.base_colors_palette = Self::build_base_colors_palette(game_lib, region);
                    self.current_palette = Self::region_palette(game_lib, region);
                    self.last_palette_key = (u16::MAX, false, false); // force recompute next tick
                    self.map_renderer = Some(MapRenderer::new(&world, self.shadow_mem.clone()));
                    self.map_world = Some(world);
                    self.log_buffer.push(format!("on_region_changed: world reloaded for region {}", region));
                }
                Err(e) => self.log_buffer.push(format!("on_region_changed: WorldData::load failed: {e}")),
            }
            self.npc_table = Some(crate::game::npc::NpcTable::load(adf, region));
            self.log_buffer.push(format!("on_region_changed: NPC table loaded for region {}", region));
        }
    }

    /// Dispatch a MenuAction returned by MenuState::handle_key / handle_click.
    fn dispatch_menu_action(&mut self, action: crate::game::menu::MenuAction) {
        use crate::game::menu::MenuAction;
        match action {
            MenuAction::Inventory    => self.do_option(GameAction::Inventory),
            // EXPLOIT GUARD: original bug allows repeated Take while paused (T key).
            // handle_key() already blocks non-Space keys when paused, but verify any
            // direct GameAction::Take path (key_bindings) also checks paused state.
            MenuAction::Take         => self.do_option(GameAction::Take),
            MenuAction::Look         => self.do_option(GameAction::LookAround),
            MenuAction::Yell         => self.do_option(GameAction::Yell),
            MenuAction::Say          => self.do_option(GameAction::Speak),
            MenuAction::Ask          => self.do_option(GameAction::Ask),
            MenuAction::CastSpell(n) => {
                let a = match n {
                    0 => GameAction::CastSpell1,
                    1 => GameAction::CastSpell2,
                    2 => GameAction::CastSpell3,
                    3 => GameAction::CastSpell4,
                    4 => GameAction::CastSpell5,
                    5 => GameAction::CastSpell6,
                    _ => GameAction::CastSpell7,
                };
                self.do_option(a);
            }
            MenuAction::BuyItem(n) => {
                let a = match n {
                    0 => GameAction::BuyFood,
                    1 => GameAction::BuyArrow,
                    2 => GameAction::BuyVial,
                    3 => GameAction::BuyMace,
                    4 => GameAction::BuySword,
                    5 => GameAction::BuyBow,
                    _ => GameAction::BuyTotem,
                };
                self.do_option(a);
            }
            MenuAction::SetWeapon(slot) => {
                use crate::game::menu::MenuMode;
                if let Some(player) = self.state.actors.first_mut() {
                    player.weapon = slot + 1;
                }
                let name = match slot {
                    0 => "Dirk",
                    1 => "Mace",
                    2 => "Sword",
                    3 => "Bow",
                    4 => "Wand",
                    5 => "Lasso",
                    _ => "Shell",
                };
                self.messages.push(format!("{} readied.", name));
                self.menu.gomenu(MenuMode::Items);
            }
            MenuAction::TryKey(idx) => {
                use crate::game::menu::MenuMode;
                use crate::game::doors::{doorfind_nearest_by_bump_radius, key_req, KeyReq,
                                         apply_door_tile_replacement};
                // idx: 0=GOLD, 1=GREEN, 2=KBLUE, 3=RED, 4=GREY, 5=WHITE → stuff[16+idx]
                let key_slot_stuff = 16 + idx as usize;
                if self.state.stuff()[key_slot_stuff] == 0 {
                    self.messages.push("No such key.".to_string());
                } else {
                    // Use the bump-radius search (same 32×64px window as the bump path),
                    // mirroring fmain.c which probes 9 directional positions × 16px around hero.
                    let region = self.state.region_num;
                    let nearest = doorfind_nearest_by_bump_radius(
                        &self.doors, region, self.state.hero_x, self.state.hero_y);
                    if let Some((door_idx, door)) = nearest {
                        let req = key_req(door.door_type);
                        let key_matches = matches!(req, KeyReq::Key(slot) if slot as usize == idx as usize);
                        if key_matches {
                            // Consume key, apply tile replacement, open door
                            // (Phase 1 only — player must walk through).
                            self.state.stuff_mut()[key_slot_stuff] -= 1;
                            if let Some(ref mut world) = self.map_world {
                                // Use hero position as probe; key is used while standing at door.
                                apply_door_tile_replacement(
                                    world, door.door_type,
                                    self.state.hero_x as i32, self.state.hero_y as i32,
                                );
                            }
                            self.messages.push("It opened.".to_string());
                            self.opened_doors.insert(door_idx);
                            self.bumped_door = None;
                            self.dlog(format!("door: key {} opened door idx={}", idx, door_idx));
                        } else {
                            self.messages.push("Key didn't fit.".to_string());
                        }
                    } else {
                        self.messages.push("Key didn't fit.".to_string());
                    }
                }
                self.menu.gomenu(MenuMode::Items);
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            MenuAction::GiveGold => {
                use crate::game::menu::MenuMode;
                let hero_x = self.state.hero_x as i16;
                let hero_y = self.state.hero_y as i16;
                let npc_nearby = self.npc_table.as_ref().map_or(false, |t| {
                    t.npcs.iter().any(|n| {
                        n.active
                            && (n.x - hero_x).abs() < 32
                            && (n.y - hero_y).abs() < 32
                    })
                });
                if !npc_nearby {
                    self.messages.push("There is no one nearby.".to_string());
                } else if self.state.wealth <= 2 {
                    self.messages.push("Not enough gold.".to_string());
                } else {
                    self.state.wealth -= 2;
                    self.messages.push("You gave gold.".to_string());
                }
                self.menu.gomenu(MenuMode::Items);
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            MenuAction::GiveWrit => {
                use crate::game::menu::MenuMode;
                let hero_x = self.state.hero_x as i16;
                let hero_y = self.state.hero_y as i16;
                let npc_nearby = self.npc_table.as_ref().map_or(false, |t| {
                    t.npcs.iter().any(|n| {
                        n.active
                            && (n.x - hero_x).abs() < 32
                            && (n.y - hero_y).abs() < 32
                    })
                });
                if !npc_nearby {
                    self.messages.push("There is no one nearby.".to_string());
                } else if self.state.stuff()[28] == 0 {
                    self.messages.push("You don't have one.".to_string());
                } else {
                    self.state.stuff_mut()[28] -= 1;
                    self.messages.push("You gave the writ.".to_string());
                }
                self.menu.gomenu(MenuMode::Items);
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            MenuAction::GiveBone => {
                use crate::game::menu::MenuMode;
                let hero_x = self.state.hero_x as i16;
                let hero_y = self.state.hero_y as i16;
                let npc_nearby = self.npc_table.as_ref().map_or(false, |t| {
                    t.npcs.iter().any(|n| {
                        n.active
                            && (n.x - hero_x).abs() < 32
                            && (n.y - hero_y).abs() < 32
                    })
                });
                if !npc_nearby {
                    self.messages.push("There is no one nearby.".to_string());
                } else if self.state.stuff()[29] == 0 {
                    self.messages.push("You don't have one.".to_string());
                } else {
                    self.state.stuff_mut()[29] -= 1;
                    self.messages.push("You gave the bone.".to_string());
                }
                self.menu.gomenu(MenuMode::Items);
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            MenuAction::SaveGame(slot) => {
                match crate::game::persist::save_game(&self.state, slot) {
                    Ok(()) => {
                        if let Err(e) = crate::game::persist::save_transcript(
                            self.messages.transcript(), slot,
                        ) {
                            eprintln!("save transcript failed: {e}");
                        }
                        self.messages.push("Game saved.");
                    }
                    Err(e) => {
                        eprintln!("save failed: {e}");
                        self.messages.push("Save failed!");
                    }
                }
            }
            MenuAction::LoadGame(slot) => {
                // EXPLOIT FIX NEEDED: reset all runtime door state before restoring
                // save, otherwise keys replenish but doors stay unlocked.
                match crate::game::persist::load_game(slot) {
                    Ok(new_state) => {
                        *self.state = new_state;
                        // Restore existing transcript so new messages are appended.
                        let existing = crate::game::persist::load_transcript(slot);
                        self.messages.set_transcript(existing);
                        self.messages.push("Game loaded.");
                    }
                    Err(e) => {
                        self.messages.push(format!("Load failed: {}", e));
                    }
                }
            }
            MenuAction::Quit     => self.do_option(GameAction::Quit),
            MenuAction::TogglePause => {
                // MenuState already toggled the bit; sync paused field.
                self.paused = self.menu.is_paused();
                if self.paused {
                    self.messages.push("Game paused. Press Space to continue.");
                }
            }
            MenuAction::ToggleMusic => {
                let on = self.menu.is_music_on();
                self.messages.push(if on { "Music on." } else { "Music off." });
                self.last_mood = u8::MAX; // force re-evaluation next tick
                self.music_stop_pending = !on;
            }
            MenuAction::ToggleSound => {
                let on = self.menu.is_sound_on();
                self.messages.push(if on { "Sound on." } else { "Sound off." });
            }
            MenuAction::RefreshMusic  => {}
            MenuAction::SummonTurtle  => self.do_option(GameAction::SummonTurtle),
            MenuAction::UseSunstone   => self.do_option(GameAction::UseSpecial),
            MenuAction::SwitchMode(_) => {}
            MenuAction::UseMenu | MenuAction::GiveMenu => {}
            MenuAction::None          => {}
        }
    }

    /// Dispatch a game menu/command action.
    fn do_option(&mut self, action: GameAction) {
        self.dlog(format!("do_option: {:?}", action));
        match action {
            GameAction::BuyFood => {
                let hero_x = self.state.hero_x as i16;
                let hero_y = self.state.hero_y as i16;
                let near_shop = self.npc_table.as_ref().map_or(false, |t| {
                    crate::game::shop::has_shopkeeper_nearby(&t.npcs, hero_x, hero_y)
                });
                if near_shop {
                    match crate::game::shop::buy_item(&mut self.state, 0) {
                        Ok(cost) => {
                            self.messages.push(format!("Bought food for {} gold.", cost));
                        }
                        Err(reason) => {
                            self.messages.push(format!("Cannot buy: {}", reason));
                        }
                    }
                } else if self.state.eat_food() {
                    let bname = brother_name(&self.state);
                    self.messages.push(crate::game::events::event_msg(&self.narr, 37, bname));
                    self.dlog(format!("eat_food: consumed food, hunger={}", self.state.hunger));
                } else {
                    self.messages.push("No food.");
                    self.dlog("eat_food: no food in pack");
                }
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            // Shop BUY menu items (npc-107): mirrors fmain.c BUY case / jtrans[] table.
            // label5 = "Food ArrowVial Mace SwordBow  Totem" — 7 items, hits 5-11.
            GameAction::BuyArrow => {
                Self::do_buy(&mut self.state, &self.npc_table, 1, "arrows", &mut self.messages);
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            GameAction::BuyVial => {
                // ITEM_VIAL = 11 in stuff[] (magic healing potion).
                Self::do_buy(&mut self.state, &self.npc_table, 11, "vial", &mut self.messages);
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            GameAction::BuyMace => {
                // Mace → weapon slot 8 (dagger/mace, cheapest weapon).
                Self::do_buy(&mut self.state, &self.npc_table, 8, "mace", &mut self.messages);
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            GameAction::BuySword => {
                // Sword → weapon slot 10 (long sword).
                Self::do_buy(&mut self.state, &self.npc_table, 10, "sword", &mut self.messages);
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            GameAction::BuyBow => {
                // Bow → weapon slot 9 (short sword / bow).
                Self::do_buy(&mut self.state, &self.npc_table, 9, "bow", &mut self.messages);
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            GameAction::BuyTotem => {
                // ITEM_TOTEM = 13 in stuff[].
                Self::do_buy(&mut self.state, &self.npc_table, 13, "totem", &mut self.messages);
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            GameAction::Inventory => {
                self.dlog(format!("Inventory: {}", self.state.inventory_summary()));
                self.state.viewstatus = 4;
                self.dlog("inventory opened".to_string());
            }
            GameAction::Rebind => {
                self.rebinding.active = !self.rebinding.active;
                self.dlog(format!("Rebinding mode: {}", self.rebinding.active));
            }
            GameAction::Board => {
                if self.state.board_raft() {
                    self.messages.push("You board the raft.");
                } else {
                    self.messages.push("Nothing to board here.");
                }
            }
            GameAction::Sleep => {
                let at_door = crate::game::doors::doorfind(
                    &self.doors, self.state.region_num, self.state.hero_x, self.state.hero_y
                ).is_some();
                if at_door {
                    self.messages.push("Cannot sleep here.");
                } else {
                    self.state.fatigue = 0;
                    self.state.hunger = (self.state.hunger + 50)
                        .min(crate::game::game_state::MAX_HUNGER);
                    let bname = brother_name(&self.state);
                    self.messages.push(crate::game::events::event_msg(&self.narr, 26, bname));
                    self.dlog("Player slept: fatigue reset");
                }
            }
            GameAction::GetItem => {
                self.messages.push("Nothing here to take.");
                self.dlog("GetItem: stub");
            }
            GameAction::DropItem => {
                self.messages.push("Dropped item.");
                self.dlog("DropItem: stub");
            }
            GameAction::LookAround => {
                let region = self.state.region_num;
                let msg = format!("Region {}. Vitality: {}. Gold: {}.",
                    region, self.state.vitality, self.state.gold);
                self.messages.push(msg);
            }
            GameAction::Talk => {
                // Talk is the same as Ask/Speak: range 50, nearest NPC (fmain.c:4167).
                self.do_option(GameAction::Speak);
            }
            GameAction::Attack => {
                // Find nearest active NPC and initiate combat
                let mut attacked = false;
                if let Some(ref mut table) = self.npc_table {
                    for npc in table.npcs.iter_mut().filter(|n| n.active) {
                        let dx = (npc.x - self.state.hero_x as i16).abs();
                        let dy = (npc.y - self.state.hero_y as i16).abs();
                        if dx < 32 && dy < 32 {
                            let result = crate::game::combat::resolve_combat(&mut self.state, npc, 0);
                            if result.enemy_defeated {
                                crate::game::combat::award_loot(&mut self.state, npc);
                                let drops = crate::game::loot::roll_loot(npc, self.state.tick_counter);
                                crate::game::loot::award_drops(&mut self.state, &drops);
                                if !drops.is_empty() {
                                    self.messages.push(format!("{} items dropped!", drops.len()));
                                }
                                // Turtle egg rescue: killing a snake near eggs awards a Sea Shell (player-108).
                                if self.state.check_turtle_eggs(npc.race == crate::game::npc::RACE_SNAKE) {
                                    self.messages.push("The turtle rewards you with a Sea Shell!");
                                    self.dlog("check_turtle_eggs: shell awarded for snake kill");
                                }
                                self.messages.push("Enemy defeated!");
                                let wealth = self.state.wealth;
                                self.menu.set_options(self.state.stuff(), wealth);
                            } else {
                                self.messages.push(format!("You hit for {}!", result.enemy_damage));
                            }
                            attacked = true;
                            break;
                        }
                    }
                }
                if !attacked {
                    self.messages.push("Nothing to attack.");
                }
            }
            // Fight (joystick fire / Space key): melee swing using direction-sensitive
            // proximity check (npc-103, mirrors fmain.c keyfight + dohit path).
            GameAction::Fight => {
                use crate::game::game_state::{ITEM_BOW, ITEM_ARROWS};
                let has_bow = self.state.stuff()[ITEM_BOW] > 0;
                let has_arrows = self.state.stuff()[ITEM_ARROWS] > 0;
                if has_bow && has_arrows {
                    // Bow equipped: fire arrow instead of melee swing (fmain.c weapon==4 → SHOOT1).
                    use crate::game::combat::fire_missile;
                    fire_missile(
                        &mut self.missiles,
                        self.state.hero_x as i32,
                        self.state.hero_y as i32,
                        self.state.facing,
                        5,
                        true,
                    );
                    self.state.stuff_mut()[ITEM_ARROWS] -= 1;
                    self.messages.push("You shoot an arrow!");
                    let wealth = self.state.wealth;
                    self.menu.set_options(self.state.stuff(), wealth);
                } else {
                    self.apply_melee_combat();
                }
                self.fight_cooldown = 10;
            }
            GameAction::UseItem => {
                self.messages.push("Nothing to use.");
                self.dlog("UseItem: stub");
            }
            // MAGIC menu items 5..=11 (stuff[9..=15], MAGICBASE=9 in fmain.c).
            GameAction::CastSpell1 => {
                match use_magic(&mut self.state, ITEM_STONE_RING) {
                    Ok(msg) => self.messages.push(msg),
                    Err(e)  => self.messages.push(e),
                }
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            GameAction::CastSpell2 => {
                match use_magic(&mut self.state, ITEM_LANTERN) {
                    Ok(msg) => self.messages.push(msg),
                    Err(e)  => self.messages.push(e),
                }
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            GameAction::CastSpell3 => {
                match use_magic(&mut self.state, ITEM_VIAL) {
                    Ok(msg) => self.messages.push(msg),
                    Err(e)  => self.messages.push(e),
                }
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            GameAction::CastSpell4 => {
                match use_magic(&mut self.state, ITEM_ORB) {
                    Ok(msg) => self.messages.push(msg),
                    Err(e)  => self.messages.push(e),
                }
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            GameAction::CastSpell5 => {
                match use_magic(&mut self.state, ITEM_TOTEM) {
                    Ok(msg) => self.messages.push(msg),
                    Err(e)  => self.messages.push(e),
                }
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            GameAction::CastSpell6 => {
                match use_magic(&mut self.state, ITEM_RING) {
                    Ok(msg) => self.messages.push(msg),
                    Err(e)  => self.messages.push(e),
                }
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            GameAction::CastSpell7 => {
                match use_magic(&mut self.state, ITEM_SKULL) {
                    Ok(msg) => self.messages.push(msg),
                    Err(e)  => self.messages.push(e),
                }
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            GameAction::Shoot => {
                use crate::game::game_state::ITEM_ARROWS;
                if self.state.stuff()[ITEM_ARROWS] == 0 {
                    self.messages.push("No Arrows!");
                } else {
                    use crate::game::combat::fire_missile;
                    fire_missile(
                        &mut self.missiles,
                        self.state.hero_x as i32,
                        self.state.hero_y as i32,
                        self.state.facing,
                        5, // base arrow damage
                        true,
                    );
                    self.state.stuff_mut()[ITEM_ARROWS] -= 1;
                    self.messages.push("You shoot an arrow!");
                    let wealth = self.state.wealth;
                    self.menu.set_options(self.state.stuff(), wealth);
                }
            }
            GameAction::SummonTurtle => {
                if self.state.summon_turtle() {
                    self.messages.push("You summon the turtle!");
                } else {
                    self.messages.push("You have no shells to summon a turtle.");
                }
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            GameAction::Look => {
                // Describe terrain at hero position (original: event 38 = item visible, event 20 = nothing special).
                let terrain_name = if let Some(ref world) = self.map_world {
                    match collision::px_to_terrain_type(world, self.state.hero_x as i32, self.state.hero_y as i32) {
                        0 => "open ground",
                        1 => "hard rock",
                        2 => "shallow water",
                        3 => "deep water",
                        4 => "swamp",
                        5 => "water",
                        6 => "trees",
                        7 => "rough terrain",
                        _  => "unknown terrain",
                    }
                } else {
                    "open ground"
                };
                self.messages.push(format!("You see: {}.", terrain_name));
            }
            GameAction::Take => {
                const PICKUP_RANGE: u16 = 24;
                if let Some(_item_id) = self.state.pickup_world_object(
                    self.state.region_num, self.state.hero_x, self.state.hero_y, PICKUP_RANGE,
                ) {
                    let bname = brother_name(&self.state);
                    let msg = crate::game::events::event_msg(&self.narr, 37, bname);
                    if !msg.is_empty() { self.messages.push(msg); }
                    let wealth = self.state.wealth;
                    self.menu.set_options(self.state.stuff(), wealth);
                } else {
                    self.messages.push("Nothing here to take.");
                }
            }
            GameAction::Give => {
                // Give 2 gold to a nearby beggar (race 0x8d), raising kindness.
                // Mirrors fmain.c GIVE case: hit==5 && wealth>2, kind++.
                use crate::game::npc::RACE_BEGGAR;
                let hero_x = self.state.hero_x as i16;
                let hero_y = self.state.hero_y as i16;
                let near_beggar = self.npc_table.as_ref().map_or(false, |t| {
                    t.npcs.iter().any(|n| {
                        n.active && n.race == RACE_BEGGAR
                            && (n.x - hero_x).abs() < 32
                            && (n.y - hero_y).abs() < 32
                    })
                });
                if near_beggar && self.state.wealth > 2 {
                    self.state.wealth -= 2;
                    // kind++ chance (mirrors: if rand64() > kind { kind++; })
                    if self.state.kind < 100 {
                        self.state.kind += 1;
                    }
                    self.messages.push("You give gold to the beggar. They thank you.");
                    self.dlog(format!("give to beggar: wealth={}, kind={}", self.state.wealth, self.state.kind));
                } else if near_beggar {
                    self.messages.push("You have no gold to spare.");
                } else {
                    self.messages.push("Nothing to give to.");
                }
            }
            GameAction::Yell => {
                // Yell range = 100. If NPC within 35 → "No need to shout, son!" (speech 8).
                // Otherwise yell the missing brother's name (original yell behavior).
                let bname = brother_name(&self.state);
                let yell_dist = self.nearest_npc_in_range(100)
                    .map(|n| {
                        let dx = (n.x as i32 - self.state.hero_x as i32).abs();
                        let dy = (n.y as i32 - self.state.hero_y as i32).abs();
                        dx.max(dy)
                    });
                if yell_dist.map_or(false, |d| d < 35) {
                    self.messages.push(crate::game::events::speak(&self.narr, 8, bname));
                } else {
                    let next_brother = match self.state.brother {
                        1 => "Phillip",
                        2 => "Kevin",
                        _ => "Julian",
                    };
                    self.messages.push(format!("{}!", next_brother));
                }
            }
            GameAction::Speak | GameAction::Ask => {
                // Talk range = 50. Check nearest NPC within range (npc-002, fmain.c:4167).
                let bname = brother_name(&self.state);
                let hero_x = self.state.hero_x as i16;
                let hero_y = self.state.hero_y as i16;
                let near_shop = self.npc_table.as_ref().map_or(false, |t| {
                    crate::game::shop::has_shopkeeper_nearby(&t.npcs, hero_x, hero_y)
                });
                if near_shop {
                    // Show buy menu: list items available for purchase with prices.
                    // Mirrors fmain.c BUY menu (label5 = "Food ArrowVial Mace SwordBow  Totem").
                    let items = [
                        (0,  "Food"),
                        (1,  "Arrows"),
                        (11, "Vial"),
                        (8,  "Mace"),
                        (10, "Sword"),
                        (9,  "Bow"),
                        (13, "Totem"),
                    ];
                    let mut menu = String::from("Shopkeeper: What do you need?\n");
                    for (idx, name) in &items {
                        let cost = crate::game::shop::ITEM_COSTS.get(*idx).copied().unwrap_or(0);
                        if cost > 0 {
                            menu.push_str(&format!("  {} - {} gold\n", name, cost));
                        }
                    }
                    menu.push_str(&format!("  (Your gold: {})", self.state.gold));
                    self.messages.push(menu);
                } else if let Some(npc) = self.nearest_npc_in_range(50) {
                    // NPC in range — show race-appropriate speech (fmain.c:4195-4230).
                    use crate::game::npc::*;
                    let speech_id: usize = match npc.race {
                        RACE_NORMAL   => 3,  // skeleton clattering (generic)
                        RACE_UNDEAD   => 2,  // wraith "Doom!"
                        RACE_WRAITH   => 2,  // wraith "Doom!"
                        RACE_ENEMY    => 1,  // orc "Human must die!"
                        RACE_SNAKE    => 4,  // snake (waste of time)
                        RACE_SHOPKEEPER => 12, // tavern keeper greeting
                        RACE_BEGGAR   => 23, // beggar "Alms!"
                        _             => 6,  // "There was no reply."
                    };
                    self.messages.push(crate::game::events::speak(&self.narr, speech_id, bname));
                } else {
                    self.messages.push("There is no one here to talk to.");
                }
            }
            GameAction::Quit => {
                self.quit_requested = true;
            }
            GameAction::Pause => {
                self.menu.toggle_pause();
                self.paused = self.menu.is_paused();
                if self.paused {
                    self.messages.push("Game paused. Press Space to continue.");
                }
            }
            _ => {}
        }
    }
    pub fn handle_game_event(&mut self, event: crate::game::game_event::GameEvent) {
        use crate::game::game_event::GameEvent;
        match event {
            GameEvent::Message { text } => {
                self.messages.push(text);
            }
            _ => {}
        }
    }

    /// Select music group 0-6 based on current game state (mirrors original setmood()).
    fn setmood(&self) -> u8 {
        let s = &self.state;
        if s.vitality <= 0 { return 6; }
        if s.hero_x >= 0x2400 && s.hero_x <= 0x3100 && s.hero_y >= 0x8200 && s.hero_y <= 0x8a00 { return 4; }
        if s.battleflag { return 1; }
        if s.region_num > 7 { return 5; }
        if s.dayperiod == 1 || s.dayperiod == 2 { return 0; }  // day music during morning/midday
        2
    }

    pub fn apply_command(&mut self, cmd: DebugCommand) {
        use DebugCommand::*;
        match cmd {
            SetStat { stat, value } => {
                let field = Self::stat_field_mut(&mut self.state, stat);
                *field = value;
            }
            AdjustStat { stat, delta } => {
                let field = Self::stat_field_mut(&mut self.state, stat);
                *field = field.saturating_add(delta);
            }
            SetInventory { index, value } => {
                let stuff = self.state.stuff_mut();
                if (index as usize) < stuff.len() {
                    stuff[index as usize] = value;
                }
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            AdjustInventory { index, delta } => {
                let stuff = self.state.stuff_mut();
                if (index as usize) < stuff.len() {
                    stuff[index as usize] = stuff[index as usize].saturating_add_signed(delta);
                }
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            TeleportSafe => {
                self.state.hero_x = self.state.safe_x;
                self.state.hero_y = self.state.safe_y;
                self.snap_camera_to_hero();
            }
            TeleportCoords { x, y } => {
                self.state.hero_x = x;
                self.state.hero_y = y;
                self.snap_camera_to_hero();
            }
            TeleportStoneRing { index } => {
                self.dlog(format!("debug command not yet wired: TeleportStoneRing {{ index: {} }}", index));
            }
            ToggleMagicEffect { effect } => match effect {
                MagicEffect::Light => self.state.light_sticky = !self.state.light_sticky,
                MagicEffect::Secret => self.state.secret_sticky = !self.state.secret_sticky,
                MagicEffect::Freeze => self.state.freeze_sticky = !self.state.freeze_sticky,
            },
            SetGodMode { flags } => {
                self.state.god_mode = flags;
            }
            SetDayPhase { phase } => {
                self.state.daynight = phase;
                let raw = self.state.daynight / 40;
                self.state.lightlevel = if raw >= 300 { 600 - raw } else { raw };
                self.state.dayperiod = ((self.state.daynight / 6000) as u8).min(3);
            }
            SetGameTime { hour, minute } => {
                // Each hour = 1000 daynight ticks; each minute ≈ 1000/60
                let ticks = (hour as u16).saturating_mul(1000)
                    + (minute as u16).saturating_mul(1000) / 60;
                self.state.daynight = ticks % 24000;
                let raw = self.state.daynight / 40;
                self.state.lightlevel = if raw >= 300 { 600 - raw } else { raw };
                self.state.dayperiod = ((self.state.daynight / 6000) as u8).min(3);
            }
            HoldTimeOfDay { hold } => {
                self.state.freeze_sticky = hold;
            }
            ToggleAutosave { enable } => {
                self.autosave_enabled = enable;
            }
            TriggerWitchEffect => {
                self.witch_effect.start();
            }
            TriggerTeleportEffect => {
                self.teleport_effect.start();
            }
            TriggerPaletteTransition { to_black } => {
                self.dlog(format!("TriggerPaletteTransition: to_black={}", to_black));
            }
            InstaKill => {
                let mut killed = 0usize;
                for actor in self.state.actors.iter_mut().skip(1) {
                    if matches!(actor.kind, ActorKind::Enemy | ActorKind::Dragon)
                        && !matches!(actor.state, ActorState::Dead | ActorState::Dying)
                    {
                        actor.vitality = 0;
                        actor.state = ActorState::Dying;
                        killed += 1;
                    }
                }
                self.dlog(format!("InstaKill: killed {} enemies", killed));
            }
            HeroPack => {
                // Fill a sensible selection: full weapon set, all magic, all keys, arrows
                let stuff = self.state.stuff_mut();
                // Weapons: dirk(0), mace(1), sword(2), bow(3), magic wand(4), golden lasso(5)
                for i in 0..=5 { stuff[i] = 1; }
                // Arrows: slot 8
                stuff[8] = 99;
                // Magic items: slots 9-15
                for i in 9..=15 { stuff[i] = 1; }
                // Keys: slots 16-21
                for i in 16..=21 { stuff[i] = 1; }
                self.dlog("HeroPack: weapons, magic, and keys filled".to_string());
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            SummonSwan => {
                self.dlog("SummonSwan: not yet wired".to_string());
            }
            RestartAsBrother { brother } => {
                let b = match brother {
                    BrotherId::Julian => 1u8,
                    BrotherId::Phillip => 2,
                    BrotherId::Kevin => 3,
                };
                self.state.brother = b;
                self.dlog(format!("RestartAsBrother: switched to brother {}", b));
            }
            QueryTerrain => {
                let x = self.state.hero_x as i32;
                let y = self.state.hero_y as i32;
                let lines: Vec<String> = match &self.map_world {
                    None => vec![
                        format!("terrain: hero=({}, {})", x, y),
                        "terrain: map_world not loaded".to_string(),
                    ],
                    Some(world) => {
                        let terra_head = format!("terrain: terra_mem[0..16] = {:02x?}", &world.terra_mem[..16]);
                        let probes: Vec<String> = [
                            ("right_foot", x + 4, y + 2),
                            ("left_foot",  x - 4, y + 2),
                        ].iter().map(|&(label, px, py)| {
                            let p = collision::terrain_probe(world, px, py);
                            format!(
                                "terrain: {}  pos=({},{})  d4=0x{:02x}  imx={} imy={}  xs={} ys={}  map[{}]=sec{}  sec_off={}  tile_idx={}  terra=[{:02x},{:02x},{:02x},{:02x}]  tiles&d4=0x{:02x}  type={}",
                                label, p.x, p.y, p.d4, p.imx, p.imy,
                                p.xs, p.ys, p.map_offset, p.sec_num,
                                p.sector_offset, p.tile_idx,
                                p.terra_bytes[0], p.terra_bytes[1],
                                p.terra_bytes[2], p.terra_bytes[3],
                                p.tiles_and_d4, p.terrain_type,
                            )
                        }).collect();
                        std::iter::once(format!("terrain: hero=({}, {})", x, y))
                            .chain(std::iter::once(terra_head))
                            .chain(probes)
                            .collect()
                    }
                };
                for line in lines { self.dlog(line); }
            }
            QueryActors => {
                let count = self.state.actors.len();
                let lines: Vec<String> = std::iter::once(format!("Actors: {} total", count))
                    .chain(self.state.actors.iter().enumerate().map(|(i, actor)| {
                        format!(
                            "  [{:2}] {:?} race={} vit={} @({},{}) {:?}",
                            i, actor.kind, actor.race, actor.vitality,
                            actor.abs_x, actor.abs_y, actor.state
                        )
                    }))
                    .collect();
                for line in lines {
                    self.dlog(line);
                }
            }
            QuerySongs => {
                self.dlog("QuerySongs: song library info is in main loop; use /songs".to_string());
            }
            DumpAdfBlock { block, count } => {
                match &self.adf {
                    None => self.dlog("DumpAdfBlock: ADF not loaded".to_string()),
                    Some(adf) => {
                        let total = adf.num_blocks() as u32;
                        let end = block + count;
                        if end > total {
                            self.dlog(format!(
                                "DumpAdfBlock: range [{}, {}) exceeds ADF size ({} blocks)",
                                block, end, total
                            ));
                        } else {
                            let data = adf.load_blocks(block, count).to_vec();
                            self.dlog(format!(
                                "ADF block(s) {}..{} ({} bytes):",
                                block, end, data.len()
                            ));
                            for (row_i, chunk) in data.chunks(16).enumerate() {
                                let offset = block as usize * 512 + row_i * 16;
                                let hex: String = chunk
                                    .iter()
                                    .map(|b| format!("{:02X}", b))
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                let ascii: String = chunk
                                    .iter()
                                    .map(|&b| if b >= 0x20 && b < 0x7F { b as char } else { '.' })
                                    .collect();
                                self.dlog(format!("{:06X}: {}  {}", offset, hex, ascii));
                            }
                        }
                    }
                }
            }
            SpawnEncounterRandom => {
                let zone_idx = self.state.region_num as usize;
                let hero_x = self.state.hero_x as i16;
                let hero_y = self.state.hero_y as i16;
                if let Some(ref mut table) = self.npc_table {
                    let spawned = crate::game::encounter::spawn_encounter_group(
                        table, zone_idx, hero_x, hero_y,
                    );
                    self.dlog(format!("forced encounter: {} enemies", spawned));
                } else {
                    self.dlog("forced encounter: no npc_table loaded".to_string());
                }
            }
            SpawnEncounterType(npc_type) => {
                use crate::game::npc::*;
                let zone_idx = self.state.region_num as usize;
                let hero_x = self.state.hero_x as i16;
                let hero_y = self.state.hero_y as i16;
                let requested_type = npc_type;
                if let Some(ref mut table) = self.npc_table {
                    if let Some(slot) = table.npcs.iter_mut().find(|n| !n.active) {
                        let mut npc = crate::game::encounter::spawn_encounter(
                            zone_idx, hero_x + 48, hero_y,
                        );
                        npc.npc_type = requested_type;
                        npc.race = match requested_type {
                            NPC_TYPE_WRAITH   => RACE_WRAITH,
                            NPC_TYPE_GHOST | NPC_TYPE_SKELETON => RACE_UNDEAD,
                            _                 => RACE_ENEMY,
                        };
                        *slot = npc;
                        self.dlog(format!("spawned enemy type={}", requested_type));
                    } else {
                        self.dlog("spawn enemy: no free NPC slots".to_string());
                    }
                } else {
                    self.dlog("spawn enemy: no npc_table loaded".to_string());
                }
            }
            ClearEncounters => {
                if let Some(ref mut table) = self.npc_table {
                    let n = table.active_count();
                    for npc in table.npcs.iter_mut() {
                        npc.active = false;
                    }
                    self.dlog(format!("cleared {} NPCs", n));
                } else {
                    self.dlog("clear encounters: no npc_table loaded".to_string());
                }
            }
            ScatterItems { count, item_id } => {
                use crate::game::sprites::INV_LIST;
                use crate::game::game_state::WorldObject;
                const TALISMAN_IDX: usize = 22;

                if count == 0 {
                    self.dlog("scattered 0 items".to_string());
                    return;
                }

                let region = self.state.region_num;
                let hero_x = self.state.hero_x as i32;
                let hero_y = self.state.hero_y as i32;
                let mut dropped = 0usize;

                if let Some(id) = item_id {
                    // Drop `count` copies of one specific item in a ring.
                    let radius = if count == 1 { 16.0f32 } else { 80.0f32 };
                    for i in 0..count {
                        let angle = if count == 1 {
                            0.0f32
                        } else {
                            2.0 * std::f32::consts::PI * (i as f32) / (count as f32)
                        };
                        let x = (hero_x + (radius * angle.cos()) as i32).clamp(0, 0x7FFF) as u16;
                        let y = (hero_y + (radius * angle.sin()) as i32).clamp(0, 0x7FFF) as u16;
                        self.state.world_objects.push(WorldObject {
                            item_id: id as u8,
                            region,
                            x, y,
                            visible: true,
                        });
                        dropped += 1;
                    }
                    self.dlog(format!("scattered {} x item {} ({})", dropped, id,
                        if id == TALISMAN_IDX { "TALISMAN — end-of-game item" } else { "" }
                    ));
                } else {
                    // Drop `count` items from the safe pool (no talisman), in a ring.
                    let safe_pool: Vec<usize> = (0..INV_LIST.len())
                        .filter(|&i| i != TALISMAN_IDX)
                        .collect();
                    let n = count.min(safe_pool.len() * 4); // allow cycling
                    for i in 0..n {
                        let item_id = safe_pool[i % safe_pool.len()];
                        let angle = 2.0 * std::f32::consts::PI * (i as f32) / (n as f32);
                        let x = (hero_x + (80.0f32 * angle.cos()) as i32).clamp(0, 0x7FFF) as u16;
                        let y = (hero_y + (80.0f32 * angle.sin()) as i32).clamp(0, 0x7FFF) as u16;
                        self.state.world_objects.push(WorldObject {
                            item_id: item_id as u8,
                            region,
                            x, y,
                            visible: true,
                        });
                        dropped += 1;
                    }
                    self.dlog(format!("scattered {} items", dropped));
                }
            }
        }
    }

    /// Collect the four y-band map_block values for the full overworld map (regions 0,2,4,6).
    /// All outdoor region pairs share a map file per y-band (F1/F2 share 160, F3/F4 share 168…).
    fn outdoor_map_blocks(game_lib: &crate::game::game_library::GameLibrary) -> Vec<u32> {
        [0u8, 2, 4, 6]
            .iter()
            .filter_map(|&r| game_lib.find_region_config(r))
            .map(|cfg| cfg.map_block)
            .collect()
    }

    /// Compute the outdoor region_num (0–7) from hero world-coordinates.
    ///
    /// Mirrors `gen_mini()` in `fmain.c`:
    ///   xs  = (hero_x + 7) >> 8          // sector column of viewport centre
    ///   ys  = (hero_y - 26) >> 8         // sector row of viewport centre
    ///   xr  = (xs >> 6) & 1              // 0 = west half, 1 = east half
    ///   yr  = (ys >> 5) & 3              // north→south band 0–3
    ///   region_num = xr + yr * 2
    fn outdoor_region_from_pos(hero_x: u16, hero_y: u16) -> u8 {
        let xs = (hero_x as u32 + 7) >> 8;
        let ys = (hero_y as u32).saturating_sub(26) >> 8;
        let xr = (xs >> 6) & 1;
        let yr = (ys >> 5) & 3;
        (xr + yr * 2) as u8
    }

    /// Build the RGBA32 playfield palette for the given region.
    ///
    /// The base palette is `pagecolors[]` from fmain2.c — hardcoded in faery.toml.
    /// Only color index 31 varies by region (from `fade_page()` in fmain2.c:526-535):
    ///   - region 4 (desert):        0x0980
    ///   - region 9 (dungeons/caves): 0x0445
    ///   - all other regions:         0x0bdf  (already the default in pagecolors)
    fn region_palette(game_lib: &GameLibrary, region: u8) -> crate::game::palette::Palette {
        use crate::game::palette::{amiga_color_to_rgba, PALETTE_SIZE};
        let mut palette = [0xFF808080_u32; PALETTE_SIZE];
        if let Some(base) = game_lib.find_palette("pagecolors") {
            for (i, entry) in base.colors.iter().enumerate().take(PALETTE_SIZE) {
                palette[i] = amiga_color_to_rgba(entry.color);
            }
        }
        let color31: u16 = match region {
            4 => 0x0980, // F5 — desert area
            9 => 0x0445, // F10 — dungeons/caves (0x00f0 when secret_timer active)
            _ => 0x0bdf, // all other regions (already the default in pagecolors[31])
        };
        palette[31] = amiga_color_to_rgba(color31);
        palette
    }

    /// Build a base `colors::Palette` for a region from faery.toml pagecolors,
    /// with per-region color 31 override applied.
    fn build_base_colors_palette(
        game_lib: &GameLibrary,
        region: u8,
    ) -> Option<crate::game::colors::Palette> {
        let base = game_lib.find_palette("pagecolors")?;
        let mut cloned = base.clone();
        let color31: u16 = match region {
            4 => 0x0980,
            9 => 0x0445,
            _ => 0x0bdf,
        };
        if let Some(c) = cloned.colors.get_mut(31) {
            *c = crate::game::colors::RGB4::from(color31);
        }
        Some(cloned)
    }

    /// Recompute current_palette from base_colors_palette + lighting state.
    ///
    /// For outdoors (region < 8): applies fade_page() with per-channel percentages
    /// derived from lightlevel (0=midnight, 300=noon) and jewel light_on flag.
    /// For indoors (region >= 8): returns base palette at full brightness.
    fn compute_current_palette(
        base: &crate::game::colors::Palette,
        region_num: u8,
        lightlevel: u16,
        light_on: bool,
        secret_active: bool,
    ) -> crate::game::palette::Palette {
        use crate::game::palette::{amiga_color_to_rgba, PALETTE_SIZE};

        // Indoors: full brightness, no fade.
        if region_num >= 8 {
            let mut pal = [0xFF808080_u32; PALETTE_SIZE];
            for (i, entry) in base.colors.iter().enumerate().take(PALETTE_SIZE) {
                pal[i] = amiga_color_to_rgba(entry.color);
            }
            // Region 9 secret_timer swaps color 31.
            if region_num == 9 && secret_active {
                pal[31] = amiga_color_to_rgba(0x00f0);
            }
            return pal;
        }

        let ll = lightlevel as i32;
        let ll_boost = if light_on { 200i32 } else { 0 };
        let r_pct = ((ll - 80 + ll_boost) * 100 / 300).clamp(0, 100) as i16;
        let g_pct = ((ll - 61) * 100 / 300).clamp(0, 100) as i16;
        let b_pct = ((ll - 62) * 100 / 300).clamp(0, 100) as i16;

        let faded = crate::game::palette_fader::fade_page(
            r_pct, g_pct, b_pct, true, light_on, base,
        );

        // Convert colors::Palette (RGB4) → [u32; 32] (ARGB8888).
        let mut out = [0xFF808080_u32; PALETTE_SIZE];
        for (i, entry) in faded.colors.iter().enumerate().take(PALETTE_SIZE) {
            out[i] = amiga_color_to_rgba(entry.color);
        }
        out
    }

    fn stat_field_mut(state: &mut GameState, stat: StatId) -> &mut i16 {
        match stat {
            StatId::Vitality => &mut state.vitality,
            StatId::Brave => &mut state.brave,
            StatId::Luck => &mut state.luck,
            StatId::Kind => &mut state.kind,
            StatId::Wealth => &mut state.wealth,
            StatId::Hunger => &mut state.hunger,
            StatId::Fatigue => &mut state.fatigue,
        }
    }

    /// Blit one 16×32 sprite frame (indexed u8) into the map framebuf (sprite-103).
    /// Transparent pixels (index == 31) are skipped.
    /// `rel_x` / `rel_y` are the top-left destination in framebuf pixels.
    fn blit_sprite_to_framebuf(
        frame_pixels: &[u8],
        rel_x: i32,
        rel_y: i32,
        framebuf: &mut [u8],
        fb_w: i32,
        fb_h: i32,
    ) {
        use crate::game::sprites::{SPRITE_W, SPRITE_H};
        for row in 0..SPRITE_H as i32 {
            let dst_y = rel_y + row;
            if dst_y < 0 || dst_y >= fb_h { continue; }
            for col in 0..SPRITE_W as i32 {
                let dst_x = rel_x + col;
                if dst_x < 0 || dst_x >= fb_w { continue; }
                let src_idx = frame_pixels[(row as usize) * SPRITE_W + col as usize];
                if src_idx == 31 { continue; } // transparent
                framebuf[(dst_y * fb_w + dst_x) as usize] = src_idx;
            }
        }
    }

    /// Blit an object sprite (16×obj_h) into the framebuf.
    fn blit_obj_to_framebuf(
        frame_pixels: &[u8],
        rel_x: i32,
        rel_y: i32,
        obj_h: usize,
        framebuf: &mut [u8],
        fb_w: i32,
        fb_h: i32,
    ) {
        use crate::game::sprites::SPRITE_W;
        for row in 0..obj_h as i32 {
            let dst_y = rel_y + row;
            if dst_y < 0 || dst_y >= fb_h { continue; }
            for col in 0..SPRITE_W as i32 {
                let dst_x = rel_x + col;
                if dst_x < 0 || dst_x >= fb_w { continue; }
                let src_idx = frame_pixels[(row as usize) * SPRITE_W + col as usize];
                if src_idx == 31 { continue; }
                framebuf[(dst_y * fb_w + dst_x) as usize] = src_idx;
            }
        }
    }

    /// Compute rel_x/rel_y for an actor at (abs_x, abs_y) given viewport origin (map_x, map_y).
    /// Matches original fmain.c:2150-2158: rel_x = abs_x - map_x - 8, rel_y = abs_y - map_y - 26.
    /// Camera follow from fsubs.asm:1360–1423.
    /// Dead zone (±20 px X / ±10 px Y): camera still, player moves in window.
    /// Creep zone (20–70 px X / 10–24/44 px Y): camera advances 1 px/tick toward player.
    /// Beyond threshold: camera tracks 1:1 with player, keeping player pinned at the edge.
    /// Immediately center the camera on the hero (used after teleports).
    fn snap_camera_to_hero(&mut self) {
        const CX: i32 = 144;
        const CY: i32 = 70;
        const WRAP: i32 = 0x8000;
        self.map_x = (self.state.hero_x as i32 - CX).rem_euclid(WRAP) as u16;
        self.map_y = (self.state.hero_y as i32 - CY).rem_euclid(WRAP) as u16;
    }

    fn map_adjust(hero_x: u16, hero_y: u16, map_x: u16, map_y: u16) -> (u16, u16) {
        const CX: i32 = 144;
        const CY: i32 = 70;
        const WRAP: i32 = 0x8000;

        // Ideal camera origin, wrapped into [0, WRAP).
        let ideal_x = (hero_x as i32 - CX).rem_euclid(WRAP);
        // Shortest-path signed delta in (-WRAP/2, WRAP/2].
        let dx = { let d = (ideal_x - map_x as i32).rem_euclid(WRAP); if d > WRAP/2 { d - WRAP } else { d } };
        let new_map_x = (if dx > 70       { ideal_x - 70 }
                         else if dx < -70 { ideal_x + 70 }
                         else if dx > 20  { map_x as i32 + 1 }
                         else if dx < -20 { map_x as i32 - 1 }
                         else             { map_x as i32 }).rem_euclid(WRAP);

        let ideal_y = (hero_y as i32 - CY).rem_euclid(WRAP);
        let dy = { let d = (ideal_y - map_y as i32).rem_euclid(WRAP); if d > WRAP/2 { d - WRAP } else { d } };
        let new_map_y = (if dy > 44       { ideal_y - 44 }
                         else if dy < -24 { ideal_y + 24 }
                         else if dy > 10  { map_y as i32 + 1 }
                         else if dy < -10 { map_y as i32 - 1 }
                         else             { map_y as i32 }).rem_euclid(WRAP);

        (new_map_x as u16, new_map_y as u16)
    }

    fn actor_rel_pos(abs_x: u16, abs_y: u16, map_x: u16, map_y: u16) -> (i32, i32) {
        Self::actor_rel_pos_offset(abs_x, abs_y, map_x, map_y, -8, -26)
    }

    /// Raft/Carrier/Dragon use (-16, -16) offsets (fmain.c:2152-2155).
    fn carrier_rel_pos(abs_x: u16, abs_y: u16, map_x: u16, map_y: u16) -> (i32, i32) {
        Self::actor_rel_pos_offset(abs_x, abs_y, map_x, map_y, -16, -16)
    }

    fn actor_rel_pos_offset(abs_x: u16, abs_y: u16, map_x: u16, map_y: u16, ox: i32, oy: i32) -> (i32, i32) {
        const WRAP: i32 = 0x8000;
        let dx = (abs_x as i32 - map_x as i32 + ox).rem_euclid(WRAP);
        let rel_x = if dx > WRAP / 2 { dx - WRAP } else { dx };
        let dy = (abs_y as i32 - map_y as i32 + oy).rem_euclid(WRAP);
        let rel_y = if dy > WRAP / 2 { dy - WRAP } else { dy };
        (rel_x, rel_y)
    }

    /// Map a facing direction (0=N…7=NW) to the sprite sheet frame base.
    /// Mirrors the diroffs[] group mapping from fmain.c:
    ///   southwalk=0-7, westwalk=8-15, northwalk=16-23, eastwalk=24-31.
    fn facing_to_frame_base(facing: u8) -> usize {
        match facing {
            0 => 16, // N  → northwalk
            1 => 24, // NE → eastwalk
            2 => 24, // E  → eastwalk
            3 => 0,  // SE → southwalk
            4 => 0,  // S  → southwalk
            5 => 8,  // SW → westwalk
            6 => 8,  // W  → westwalk
            _ => 16, // NW → northwalk
        }
    }

    /// Map (npc_type, race) → cfile index for enemy sprite rendering.
    /// Returns None for SetFig humans (rendered in a separate pass) and skipped types.
    /// cfile 7 covers ghost/wraith/skeleton per RESEARCH.md sprite assignments.
    fn npc_type_to_cfile(npc_type: u8, race: u8) -> Option<usize> {
        use crate::game::npc::*;
        match npc_type {
            NPC_TYPE_NONE | NPC_TYPE_CONTAINER => None,
            NPC_TYPE_HUMAN if race == RACE_ENEMY => Some(6),
            NPC_TYPE_HUMAN => None,  // SetFig — handled in setfig pass
            NPC_TYPE_SWAN     => Some(11),
            NPC_TYPE_HORSE    => Some(5),
            NPC_TYPE_DRAGON   => Some(10),
            NPC_TYPE_GHOST    => Some(7),
            NPC_TYPE_ORC      => Some(6),
            NPC_TYPE_WRAITH   => Some(7),
            NPC_TYPE_SKELETON => Some(7),
            NPC_TYPE_RAFT     => Some(4),
            _                 => Some(6), // unknown enemy types default to ogre sheet
        }
    }

    /// Map (npc_type, race) → SETFIG_TABLE index for named NPC rendering.
    /// Returns None if the NPC is not a SetFig.
    /// SETFIG_TABLE indices: 0=wizard, 8=bartender, 13=beggar (see sprites.rs).
    fn npc_to_setfig_idx(npc_type: u8, race: u8) -> Option<usize> {
        use crate::game::npc::*;
        if npc_type != NPC_TYPE_HUMAN { return None; }
        match race {
            RACE_SHOPKEEPER => Some(8),   // bartender
            RACE_BEGGAR     => Some(13),  // beggar
            RACE_NORMAL     => Some(0),   // wizard (default named NPC)
            _               => None,
        }
    }

    /// Blit all visible actors (hero + enemy NPCs) onto the map framebuf (sprite-104).
    /// Called immediately after mr.compose() so actors appear on top of tiles.
    fn blit_actors_to_framebuf(
        sprite_sheets: &[Option<crate::game::sprites::SpriteSheet>],
        obj_sprites: &Option<crate::game::sprites::SpriteSheet>,
        state: &GameState,
        npc_table: &Option<crate::game::npc::NpcTable>,
        map_x: u16,
        map_y: u16,
        framebuf: &mut Vec<u8>,
        hero_submerged: bool,
    ) {
        use crate::game::map_renderer::{MAP_DST_W, MAP_DST_H};
        use crate::game::sprites::{SPRITE_H, SPRITE_W};
        let fb_w = MAP_DST_W as i32;
        let fb_h = MAP_DST_H as i32;

        // --- Hero sprite ---
        // cfiles[0]=Julian (brother=1), [1]=Phillip (brother=2), [2]=Kevin (brother=3)
        let hero_cfile = state.brother.saturating_sub(1) as usize;
        if let Some(Some(ref sheet)) = sprite_sheets.get(hero_cfile) {
            let (rel_x, mut rel_y) = Self::actor_rel_pos(state.hero_x, state.hero_y, map_x, map_y);
            if hero_submerged { rel_y += 8; }
            if rel_x > -(SPRITE_W as i32) && rel_x < fb_w && rel_y > -(SPRITE_H as i32) && rel_y < fb_h {
                let hero_facing = state.actors.first().map_or(0u8, |a| a.facing);
                let is_moving = state.actors.first().map_or(false, |a| a.moving);
                // Sprite sheet layout (from fmain.c statelist[] and diroffs[]):
                //   southwalk=0-7, westwalk=8-15, northwalk=16-23, eastwalk=24-31
                // Original diroffs[] groups: NW+N→north, NE+E→east, SE+S→south, SW+W→west.
                // Rust facing: 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW.
                let frame_base = Self::facing_to_frame_base(hero_facing);
                // Walking: cycle through 8 frames; still: fmain.c uses diroffs[d]+1.
                let anim_offset = if is_moving { (state.cycle as usize) % 8 } else { 1 };
                let frame = frame_base + anim_offset;
                if let Some(fp) = sheet.frame_pixels(frame) {
                    Self::blit_sprite_to_framebuf(fp, rel_x, rel_y, framebuf, fb_w, fb_h);
                }

                // Weapon overlay (fmain.c passmode weapon blit).
                let weapon_type = state.actors.first().map_or(0u8, |a| a.weapon);
                if weapon_type > 0 && weapon_type <= 5 {
                    if let Some(ref obj_sheet) = obj_sprites {
                        use crate::game::sprites::{STATELIST, OBJ_SPRITE_H};
                        if let Some(entry) = STATELIST.get(frame) {
                            let (wpn_x, wpn_y, wpn_frame) = if weapon_type == 5 {
                                // Wand: facing + 103
                                let wand_y = if hero_facing == 2 { entry.wpn_y - 6 } else { entry.wpn_y };
                                (entry.wpn_x, wand_y, hero_facing as usize + 103)
                            } else {
                                // Hand weapons: dirk(1)=+64, mace(2)=+32, sword(3)=+48, bow(4)=+0
                                let k: usize = match weapon_type {
                                    1 => 64,
                                    2 => 32,
                                    3 => 48,
                                    _ => 0,
                                };
                                (entry.wpn_x, entry.wpn_y, entry.wpn_no as usize + k)
                            };
                            let wx = rel_x + wpn_x as i32;
                            let wy = rel_y + wpn_y as i32;
                            if let Some(wfp) = obj_sheet.frame_pixels(wpn_frame) {
                                Self::blit_obj_to_framebuf(wfp, wx, wy, OBJ_SPRITE_H, framebuf, fb_w, fb_h);
                            }
                        }
                    }
                }
            }
        }

        // --- Enemy NPCs from npc_table ---
        if let Some(ref table) = npc_table {
            for npc in table.npcs.iter().filter(|n| n.active) {
                let Some(cfile_idx) = Self::npc_type_to_cfile(npc.npc_type, npc.race) else { continue };
                let Some(Some(ref sheet)) = sprite_sheets.get(cfile_idx) else { continue };

                let (rel_x, rel_y) = Self::actor_rel_pos(npc.x as u16, npc.y as u16, map_x, map_y);

                // Compute facing from NPC position relative to hero (NPCs always chase hero).
                let dx = state.hero_x as i32 - npc.x as i32;
                let dy = state.hero_y as i32 - npc.y as i32;
                let npc_facing = if dx.abs() >= dy.abs() {
                    if dx > 0 { 2u8 } else { 6u8 }  // E or W toward hero
                } else {
                    if dy > 0 { 4u8 } else { 0u8 }  // S or N toward hero
                };

                let frame_base = Self::facing_to_frame_base(npc_facing);
                // Wrap with sheet.num_frames to handle short sheets (e.g. dragon=5).
                let frame = ((frame_base % sheet.num_frames) + (state.cycle as usize % 8)) % sheet.num_frames;

                if let Some(fp) = sheet.frame_pixels(frame) {
                    Self::blit_sprite_to_framebuf(fp, rel_x, rel_y, framebuf, fb_w, fb_h);
                }
            }
        }

        // --- SetFig NPCs (named NPCs: shopkeepers, beggars, etc.) ---
        if let Some(ref table) = npc_table {
            use crate::game::sprites::SETFIG_TABLE;
            for npc in table.npcs.iter().filter(|n| n.active) {
                let Some(setfig_idx) = Self::npc_to_setfig_idx(npc.npc_type, npc.race) else { continue };
                let entry = SETFIG_TABLE[setfig_idx];
                let cfile_idx = entry.cfile_entry as usize;
                let Some(Some(ref sheet)) = sprite_sheets.get(cfile_idx) else { continue };

                let (rel_x, rel_y) = Self::actor_rel_pos(npc.x as u16, npc.y as u16, map_x, map_y);

                // SetFigs are stationary; use image_base as the static frame.
                let frame = (entry.image_base as usize) % sheet.num_frames;
                if let Some(fp) = sheet.frame_pixels(frame) {
                    Self::blit_sprite_to_framebuf(fp, rel_x, rel_y, framebuf, fb_w, fb_h);
                }
            }
        }
    }

    /// Hit-test canvas position (mx, my) against the 8 compass arrow regions.
    /// If a region is found, updates directional input flags and returns true.
    /// If outside all hitboxes, clears directional flags and returns false.
    /// Does NOT touch compass_held — caller manages that.
    fn apply_compass_input_from_canvas(&mut self, mx: i32, my: i32) -> bool {
        const COMPASS_X_MIN: i32 = 567;
        const COMPASS_X_MAX: i32 = 567 + 48;
        let compass_y_min = HIBAR_Y + 30; // COMPASS_SRC_Y(15) × 2
        let compass_y_max = HIBAR_Y + 78; // (COMPASS_SRC_Y+COMPASS_SRC_H)(39) × 2
        if mx >= COMPASS_X_MIN && mx < COMPASS_X_MAX
            && my >= compass_y_min && my < compass_y_max
        {
            let nx = mx - COMPASS_X_MIN;
            let ny = (my - compass_y_min) / 2; // scale back to native 24px height
            for (idx, &(rx, ry, rw, rh)) in self.compass_regions[..8.min(self.compass_regions.len())].iter().enumerate() {
                if rw > 0 && rh > 0
                    && nx >= rx && nx < rx + rw
                    && ny >= ry && ny < ry + rh
                {
                    // comptable: NW=0,N=1,NE=2,E=3,SE=4,S=5,SW=6,W=7
                    self.input.up    = matches!(idx, 0 | 1 | 2);
                    self.input.down  = matches!(idx, 4 | 5 | 6);
                    self.input.left  = matches!(idx, 0 | 6 | 7);
                    self.input.right = matches!(idx, 2 | 3 | 4);
                    return true;
                }
            }
        }
        // Outside all hitboxes — stop movement while held
        self.input.up    = false;
        self.input.down  = false;
        self.input.left  = false;
        self.input.right = false;
        false
    }
}

impl Scene for GameplayScene {
    fn handle_event(&mut self, event: &Event) -> bool {
        // If rebinding mode is active and waiting for a key, capture the next keypress.
        if self.rebinding.active {
            if let Event::KeyDown { keycode: Some(kc), repeat: false, .. } = event {
                if *kc == Keycode::Escape {
                    self.rebinding.active = false;
                    self.rebinding.waiting_for_action = None;
                    self.dlog("Rebinding mode: false");
                    return true;
                }
                if let Some(action) = self.rebinding.waiting_for_action.take() {
                    self.local_bindings.set_binding(action, vec![*kc]);
                    self.dlog(format!("Rebound {:?} to {:?}", action, kc));
                    return true;
                }
            }
        }
        match event {
            Event::KeyDown { keycode: Some(kc), keymod, repeat: false, .. } => {
                // ALT+F4 → immediate quit (OS convention, takes priority over everything).
                use sdl2::keyboard::Mod;
                let alt_held = keymod.intersects(Mod::LALTMOD | Mod::RALTMOD);
                if alt_held && *kc == Keycode::F4 {
                    self.do_option(GameAction::Quit);
                    return true;
                }
                // ESC: close inventory (viewstatus 4) or map view (viewstatus 1) if open;
                // otherwise do nothing (no quit on ESC — use ALT+F4 instead).
                if *kc == Keycode::Escape {
                    if self.state.viewstatus == 4 || self.state.viewstatus == 1 {
                        self.state.viewstatus = 0;
                    }
                    return true;
                }
                match *kc {
                // Movement keys: arrow keys + numpad (no WASD — those are commands)
                Keycode::Up    | Keycode::Kp8 => { self.input.up = true; true }
                Keycode::Down  | Keycode::Kp2 => { self.input.down = true; true }
                Keycode::Left  | Keycode::Kp4 => { self.input.left = true; true }
                Keycode::Right | Keycode::Kp6 => { self.input.right = true; true }
                // Diagonal movement (numpad only)
                Keycode::Kp7 => { self.input.up = true; self.input.left = true; true }
                Keycode::Kp9 => { self.input.up = true; self.input.right = true; true }
                Keycode::Kp1 => { self.input.down = true; self.input.left = true; true }
                Keycode::Kp3 => { self.input.down = true; self.input.right = true; true }
                // Fight: numpad 0 (original)
                Keycode::Kp0 => { self.input.fight = true; true }
                // All letter_list keys → route through MenuState
                _ => {
                    if let Some(menu_key) = keycode_to_menukey(*kc) {
                        let action = self.menu.handle_key(menu_key);
                        self.dispatch_menu_action(action);
                        true
                    } else {
                        false
                    }
                }
                }
            },
            Event::KeyUp { keycode: Some(kc), .. } => match *kc {
                Keycode::Up    | Keycode::Kp8 => { self.input.up = false; true }
                Keycode::Down  | Keycode::Kp2 => { self.input.down = false; true }
                Keycode::Left  | Keycode::Kp4 => { self.input.left = false; true }
                Keycode::Right | Keycode::Kp6 => { self.input.right = false; true }
                Keycode::Kp7 => { self.input.up = false; self.input.left = false; true }
                Keycode::Kp9 => { self.input.up = false; self.input.right = false; true }
                Keycode::Kp1 => { self.input.down = false; self.input.left = false; true }
                Keycode::Kp3 => { self.input.down = false; self.input.right = false; true }
                Keycode::Kp0 => { self.input.fight = false; true }
                _ => false,
            },
            // Controller axis motion: map left stick to movement input
            Event::ControllerAxisMotion { axis, value, .. } => {
                use sdl2::controller::Axis;
                const THRESHOLD: i16 = 8000;
                match axis {
                    Axis::LeftX => {
                        self.input.left  = *value < -THRESHOLD;
                        self.input.right = *value >  THRESHOLD;
                        true
                    }
                    Axis::LeftY => {
                        self.input.up   = *value < -THRESHOLD;
                        self.input.down = *value >  THRESHOLD;
                        true
                    }
                    _ => false,
                }
            }
            // Controller button press: map to game actions via ControllerBindings
            Event::ControllerButtonDown { button, .. } => {
                use sdl2::controller::Button;
                match button {
                    Button::DPadUp    => { self.input.up    = true; true }
                    Button::DPadDown  => { self.input.down  = true; true }
                    Button::DPadLeft  => { self.input.left  = true; true }
                    Button::DPadRight => { self.input.right = true; true }
                    Button::A         => { self.do_option(GameAction::Fight);     true }
                    Button::X         => { self.do_option(GameAction::Inventory); true }
                    Button::Y         => { self.do_option(GameAction::Look);      true }
                    Button::B         => { self.do_option(GameAction::UseItem);   true }
                    Button::LeftShoulder  => { self.do_option(GameAction::CastSpell1); true }
                    Button::RightShoulder => { self.do_option(GameAction::CastSpell2); true }
                    Button::Start     => { self.do_option(GameAction::Pause);     true }
                    Button::Back      => { self.do_option(GameAction::Map);       true }
                    _ => false,
                }
            }
            // Controller button release: clear movement inputs
            Event::ControllerButtonUp { button, .. } => {
                use sdl2::controller::Button;
                match button {
                    Button::DPadUp    => { self.input.up    = false; true }
                    Button::DPadDown  => { self.input.down  = false; true }
                    Button::DPadLeft  => { self.input.left  = false; true }
                    Button::DPadRight => { self.input.right = false; true }
                    _ => false,
                }
            }
            // Mouse click: close overlay views, or dispatch through MenuState button grid
            Event::MouseButtonDown { x, y, mouse_btn: sdl2::mouse::MouseButton::Left, .. } => {
                // Any click dismisses inventory or map view.
                if self.state.viewstatus == 4 || self.state.viewstatus == 1 {
                    self.state.viewstatus = 0;
                    return true;
                }
                // HIBAR_Y=326, HIBAR_H=114 (2× line-doubled). Button click detection:
                // convert canvas y → native 57px space, then apply propt row pitch (9px).
                const BTN_X_LEFT: i32 = 430;
                const BTN_X_RIGHT: i32 = 482;
                const BTN_X_END: i32 = 530;
                let mx = *x;
                let my = *y;
                if mx >= BTN_X_LEFT && mx <= BTN_X_END
                    && my >= HIBAR_Y && my < HIBAR_Y + HIBAR_H as i32
                {
                    let col = if mx < BTN_X_RIGHT { 0usize } else { 1usize };
                    // Native y within the 57px band; divide by propt row pitch (9) to get row.
                    let native_y = (my - HIBAR_Y) / 2;
                    let row = (native_y / 9) as usize;
                    let slot = row * 2 + col;
                    if slot < 12 {
                        let action = self.menu.handle_click(slot);
                        self.dispatch_menu_action(action);
                        return true;
                    }
                }

                // Compass click: activate direction under pointer and begin tracking.
                if self.apply_compass_input_from_canvas(mx, my) {
                    self.input.compass_held = true;
                    return true;
                }

                false
            }
            // Compass drag: while mouse is held inside compass, follow pointer direction.
            Event::MouseMotion { x, y, .. } => {
                if self.input.compass_held {
                    self.apply_compass_input_from_canvas(*x, *y);
                    true
                } else {
                    false
                }
            }
            Event::MouseButtonUp { mouse_btn: sdl2::mouse::MouseButton::Left, .. } => {
                if self.input.compass_held {
                    self.input.up    = false;
                    self.input.down  = false;
                    self.input.left  = false;
                    self.input.right = false;
                    self.input.compass_held = false;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn update(
        &mut self,
        canvas: &mut Canvas<Window>,
        _play_tex: &mut Texture,
        delta_ticks: u32,
        game_lib: &GameLibrary,
        resources: &mut SceneResources<'_, '_>,
    ) -> SceneResult {
        self.tick_accum += delta_ticks;

        // When paused, skip game logic but keep rendering.
        if self.menu.is_paused() {
            self.render_by_viewstatus(canvas, resources);
            return SceneResult::Continue;
        }

        let tick_events = self.state.tick(delta_ticks);
        self.state.cycle = self.state.cycle.wrapping_add(delta_ticks);
        if !tick_events.is_empty() {
            let bname = brother_name(&self.state);
            for ev in tick_events {
                let msg = crate::game::events::event_msg(&self.narr, ev as usize, bname);
                if !msg.is_empty() {
                    self.messages.push(msg);
                }
                if ev == 12 || ev == 24 {
                    self.sleeping = true;
                }
            }
        }

        // Sleep loop: advance time quickly, reduce fatigue, wake when rested
        if self.sleeping {
            self.state.daynight = ((self.state.daynight as u32 + 63) % 24000) as u16;
            self.state.fatigue = self.state.fatigue.saturating_sub(1);
            let raw = self.state.daynight / 40;
            self.state.lightlevel = if raw >= 300 { 600u16.saturating_sub(raw) } else { raw };
            let can_wake_time = self.state.daynight >= 9000 && self.state.daynight < 10000;
            if self.state.fatigue == 0 || (self.state.fatigue < 30 && can_wake_time) {
                self.sleeping = false;
            }
            self.render_by_viewstatus(canvas, resources);
            return SceneResult::Continue;
        }

        // Drowning damage (#105): 1 vitality per ~1s while submerged
        if self.submerged {
            self.drowning_timer = self.drowning_timer.wrapping_add(delta_ticks);
            if self.drowning_timer % 30 == 0 {
                self.state.vitality = (self.state.vitality - 1).max(0);
            }
        } else {
            self.drowning_timer = 0;
        }

        // Lazy-load ADF + world data on first tick (render-world-load).
        // ADF path comes from faery.toml [disk].adf; falls back to the default filename.
        // Errors are logged to stderr; missing ADF is gracefully handled.
        if !self.adf_load_attempted {
            self.adf_load_attempted = true;
            let adf_path = game_lib
                .disk
                .as_ref()
                .map(|d| d.adf.as_str())
                .unwrap_or("game/image");
            match crate::game::adf::AdfDisk::open(std::path::Path::new(adf_path)) {
                Ok(adf) => {
                    let region = self.state.region_num;
                    let world_result = if let Some(cfg) = game_lib.find_region_config(region) {
                        let map_blocks: Vec<u32> = if region < 8 {
                            Self::outdoor_map_blocks(game_lib)
                        } else {
                            vec![cfg.map_block]
                        };
                        crate::game::world_data::WorldData::load(
                            &adf, region,
                            cfg.sector_block, &map_blocks,
                            cfg.terra_block, cfg.terra2_block,
                            &cfg.image_blocks,
                        )
                    } else {
                        Err(anyhow::anyhow!("no region config for region {}", region))
                    };
                    match world_result {
                        Ok(world) => {
                            self.base_colors_palette = Self::build_base_colors_palette(game_lib, region);
                            self.current_palette = Self::region_palette(game_lib, region);
                            self.last_palette_key = (u16::MAX, false, false); // force recompute next tick
                            // Load global shadow_mem bitmask table (sprite-depth masking).
                            let shadow_mem = if let Some(ref disk) = game_lib.disk {
                                if disk.shadow_count > 0 {
                                    crate::game::world_data::load_shadow_mem(&adf, disk.shadow_block, disk.shadow_count)
                                } else {
                                    Vec::new()
                                }
                            } else {
                                Vec::new()
                            };
                            self.shadow_mem = shadow_mem;
                            let renderer = MapRenderer::new(&world, self.shadow_mem.clone());
                            // npc-101: load NPC table for the starting region
                            self.npc_table = Some(crate::game::npc::NpcTable::load(&adf, region));
                            // sprite-101: load player (cfile 0-2), enemies (cfile 4-12), and setfig (cfile 13-17) sprites
                            for cfile_idx in [0u8, 1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17] {
                                if let Some(sheet) = crate::game::sprites::SpriteSheet::load(
                                    &adf, cfile_idx,
                                ) {
                                    self.dlog(format!(
                                        "sprite-load: cfile {} → {} frames",
                                        cfile_idx, sheet.num_frames
                                    ));
                                    self.sprite_sheets[cfile_idx as usize] = Some(sheet);
                                }
                            }
                            // Load objects sprite sheet (cfile 3, 16×16) for inventory screen.
                            self.object_sprites = crate::game::sprites::SpriteSheet::load_objects(
                                &adf,
                            );
                            self.map_world = Some(world);
                            self.map_renderer = Some(renderer);
                            self.adf = Some(adf);
                            self.dlog(format!("render-world-load: world loaded for region {}", region));
                        }
                        Err(e) => self.dlog(format!("render-world-load: WorldData::load failed: {e}")),
                    }
                }
                Err(e) => self.dlog(format!("render-world-load: AdfDisk::open failed (ADF may not be present): {e}")),
            }
        }


        let new_phase = DayNightPhase::from_lightlevel(self.state.lightlevel);
        if new_phase != self.day_night_phase {
            self.dlog(format!("Day/night phase: {:?}", new_phase));
            self.day_night_phase = new_phase;
        }

        // Recompute current_palette when lighting state changes.
        {
            let lightlevel = self.state.lightlevel;
            let light_on = self.state.light_timer > 0;
            let secret_active = self.state.region_num == 9 && self.state.secret_timer > 0;
            let palette_key = (lightlevel, light_on, secret_active);
            if palette_key != self.last_palette_key {
                self.last_palette_key = palette_key;
                if let Some(ref base) = self.base_colors_palette {
                    self.current_palette = Self::compute_current_palette(
                        base,
                        self.state.region_num,
                        lightlevel,
                        light_on,
                        secret_active,
                    );
                }
            }
        }

        // Fatigue is updated per movement step in apply_player_input (player-111).

        // Handle pending music stop from ToggleMusic OFF
        if self.music_stop_pending {
            self.music_stop_pending = false;
            if let Some(audio) = resources.audio {
                audio.stop_score();
            }
        }

        // setmood: check music group every 4 ticks (gameloop-113)
        self.mood_tick += delta_ticks;
        if self.mood_tick >= 4 {
            self.mood_tick = 0;
            let mood = self.setmood();
            if mood != self.last_mood {
                self.last_mood = mood;
                self.dlog(format!("setmood: switching to group {}", mood));
                if let Some(audio) = resources.audio {
                    if self.menu.is_music_on() {
                        audio.set_score(mood);
                    }
                }
            }
        }


        // Indoor/outdoor mode detection (world-108)
        let indoor = self.state.region_num > 7;
        if indoor != self.last_indoor {
            if indoor {
                self.dlog(format!("{:?}", crate::game::game_event::GameEvent::EnterIndoor { door_index: self.state.region_num }));
            } else {
                self.dlog(format!("{:?}", crate::game::game_event::GameEvent::ExitIndoor));
            }
            self.last_indoor = indoor;
        }

        // Encounter zone check (world-111)
        self.in_encounter_zone = crate::game::zones::in_encounter_zone(
            self.state.region_num, self.state.hero_x, self.state.hero_y);

        // Event zone entry check (#107)
        {
            let hx = self.state.hero_x;
            let hy = self.state.hero_y;
            let region = self.state.region_num;
            let current_zone = self.zones.iter().position(|z|
                z.region == region
                    && hx >= z.x1 && hx <= z.x2
                    && hy >= z.y1 && hy <= z.y2
            );
            if current_zone != self.last_zone {
                if let Some(idx) = current_zone {
                    let event_id = self.zones[idx].event_id as usize;
                    let bname = brother_name(&self.state);
                    let msg = crate::game::events::event_msg(&self.narr, event_id, bname);
                    if !msg.is_empty() { self.messages.push(msg); }
                }
                self.last_zone = current_zone;
            }
        }

        // Encounter spawning (npc-104): trigger random encounter when in encounter zone.
        if self.in_encounter_zone && crate::game::encounter::should_encounter(self.state.tick_counter) {
            if let Some(ref mut table) = self.npc_table {
                if let Some(slot) = table.npcs.iter_mut().find(|n| !n.active) {
                    let zone_idx = self.state.region_num as usize;
                    *slot = crate::game::encounter::spawn_encounter(
                        zone_idx,
                        self.state.hero_x as i16,
                        self.state.hero_y as i16,
                    );
                    self.dlog("ambush triggered".to_string());
                }
            }
        }

        // Autosave every 1800 ticks (~60s at 30Hz)
        if self.autosave_enabled && self.state.tick_counter % 1800 == 0 && self.state.tick_counter > 0 {
            if let Err(e) = crate::game::persist::save_game(&self.state, 0) {
                eprintln!("autosave failed: {e}");
            } else if let Err(e) = crate::game::persist::save_transcript(
                self.messages.transcript(), 0,
            ) {
                eprintln!("autosave transcript failed: {e}");
            }
        }

        // Death / revive cycle (gameloop-106)
        // Trigger: vitality drops to 0 → start goodfairy countdown.
        // During countdown: no input, decrement goodfairy every other tick (~30Hz).
        // At 0: luck check → faery revival or next brother / game over.
        if !self.dying && self.state.vitality <= 0
            && !self.state.god_mode.contains(GodModeFlags::INVINCIBLE)
        {
            self.dying = true;
            self.goodfairy = 255;
            self.last_mood = u8::MAX; // force death music re-evaluation
            self.dlog("death: goodfairy countdown started (255)");
        }

        if self.dying {
            // Decrement every other tick (~30Hz countdown, ~8.5s total)
            self.goodfairy -= delta_ticks as i16;
            if self.goodfairy <= 0 {
                self.dying = false;
                if self.state.luck >= 10 {
                    // Faery resurrection: teleport to safe spawn, drain luck
                    self.state.luck -= 10;
                    self.state.hero_x = self.state.safe_x;
                    self.state.hero_y = self.state.safe_y;
                    self.state.region_num = self.state.safe_r;
                    self.state.vitality = crate::game::magic::heal_cap(self.state.brave);
                    let bname = brother_name(&self.state);
                    self.messages.push(format!("A faery saved {}!", bname));
                    self.last_mood = u8::MAX; // restart normal music
                    self.dlog(format!("faery revived {}, luck now {}", bname, self.state.luck));
                } else if let Some(next) = self.state.next_brother() {
                    if let Some(bro) = game_lib.get_brother(next) {
                        let (sx, sy, sr) = game_lib.find_location(&bro.spawn)
                            .map(|loc| (loc.x, loc.y, loc.region))
                            .unwrap_or((19036, 15755, 3));
                        self.state.activate_brother_from_config(
                            next, bro.brave, bro.luck, bro.kind, bro.wealth, sx, sy, sr,
                        );
                    } else {
                        self.state.activate_brother(next);
                    }
                    let bname = brother_name(&self.state);
                    self.messages.push(format!("{} takes up the quest!", bname));
                    self.last_mood = u8::MAX;
                    self.dlog(format!("brother died, {} continues", bname));
                } else {
                    // All brothers dead — game over
                    self.quit_requested = true;
                    self.dlog("All brothers dead — GAME OVER");
                }
            }
        }

        // Run one simulation step per 30 Hz tick (NTSC interlaced frame rate).
        for _ in 0..delta_ticks {
            if !self.dying {
                self.apply_player_input();
            }

            // Tick missiles (npc-105): advance each active missile, check hits.
            {
                let hero_x = self.state.hero_x as i32;
                let hero_y = self.state.hero_y as i32;
                // Snapshot NPC positions to avoid simultaneous mutable borrow conflicts.
                let npc_positions: Vec<(usize, i32, i32)> = self.npc_table.as_ref().map_or(vec![], |t| {
                    t.npcs.iter().enumerate()
                        .filter(|(_, n)| n.active)
                        .map(|(i, n)| (i, n.x as i32, n.y as i32))
                        .collect()
                });
                let mut hero_missile_damage: i16 = 0;
                let mut npc_hits: Vec<(usize, i16)> = vec![];
                for missile in self.missiles.iter_mut() {
                    if !missile.active { continue; }
                    missile.x += missile.dx;
                    missile.y += missile.dy;
                    if missile.x < 0 || missile.x > 32768 || missile.y < 0 || missile.y > 32768 {
                        missile.active = false;
                        continue;
                    }
                    if missile.is_friendly {
                        for &(npc_idx, nx, ny) in &npc_positions {
                            if (missile.x - nx).abs() < 16 && (missile.y - ny).abs() < 16 {
                                missile.active = false;
                                npc_hits.push((npc_idx, missile.damage));
                                break;
                            }
                        }
                    } else if (missile.x - hero_x).abs() < 16 && (missile.y - hero_y).abs() < 16 {
                        missile.active = false;
                        hero_missile_damage += missile.damage;
                    }
                }
                if let Some(ref mut table) = self.npc_table {
                    for (npc_idx, dmg) in npc_hits {
                        table.npcs[npc_idx].vitality -= dmg;
                        if table.npcs[npc_idx].vitality <= 0 {
                            table.npcs[npc_idx].active = false;
                        }
                    }
                }
                self.state.vitality -= hero_missile_damage;
            }
            let shells = self.state.return_eggs_to_nest(self.state.hero_x, self.state.hero_y, 0);
            if shells > 0 {
                self.messages.push(format!("The turtle rewards you with {} shell(s)!", shells));
            }
            self.update_actors(1);

            let (new_map_x, new_map_y) = Self::map_adjust(
                self.state.hero_x, self.state.hero_y,
                self.map_x, self.map_y,
            );
            self.map_x = new_map_x;
            self.map_y = new_map_y;
            self.state.map_x = self.map_x;
            self.state.map_y = self.map_y;
        }

        // Region transition check (world-109): must run after movement so that on_region_changed()
        // loads the new world data before compose() runs — otherwise compose() sees the new
        // region_num (wrong xreg/yreg) with the old map_world, producing a one-frame glitch.
        let region = self.state.region_num;
        if region != self.last_region_num {
            self.on_region_changed(region, game_lib);
            self.dlog(format!("region_num changed: {} -> {} ({:?})", self.last_region_num, region,
                crate::game::game_event::GameEvent::RegionTransition { region }));
            // Cave instrument swap: region 9 uses new_wave[10] = 0x0307 (audio-105).
            if let Some(audio) = resources.audio {
                audio.set_cave_mode(region == 9);
            }
            let from = self.palette_transition
                .as_ref()
                .map(|pt| pt.to)
                .unwrap_or([crate::game::palette::BLACK; crate::game::palette::PALETTE_SIZE]);
            let to = Self::region_palette(game_lib, region);
            self.palette_transition = Some(crate::game::palette::PaletteTransition::new(from, to));
            self.last_region_num = region;
        }
        if let Some(ref mut pt) = self.palette_transition {
            if !pt.is_done() {
                let palette = pt.tick();
                self.current_palette = palette;
            }
        }

        // Compose map viewport when in normal play view (world-105).
        // Pass pixel-precise map_x/map_y so compose() can apply the sub-tile offset.
        if self.state.viewstatus == 0 {
            if let (Some(ref mut mr), Some(ref world)) = (&mut self.map_renderer, &self.map_world) {
                mr.compose(self.map_x, self.map_y, world);
            }
            // Blit actors on top of the composed tiles (sprite-104).
            // Collect borrow-safe parameters before taking &mut map_renderer.
            let map_x = self.map_x;
            let map_y = self.map_y;
            if let Some(ref mut mr) = self.map_renderer {
                Self::blit_actors_to_framebuf(
                    &self.sprite_sheets,
                    &self.object_sprites,
                    &self.state,
                    &self.npc_table,
                    map_x,
                    map_y,
                    &mut mr.framebuf,
                    self.submerged,
                );
                // Render world objects on the ground
                if let Some(ref obj_sheet) = self.object_sprites {
                    use crate::game::sprites::{SPRITE_W, OBJ_SPRITE_H};
                    let fb_w = crate::game::map_renderer::MAP_DST_W as i32;
                            let fb_h = crate::game::map_renderer::MAP_DST_H as i32;
                    for obj in &self.state.world_objects {
                        if !obj.visible || obj.region != self.state.region_num { continue; }
                        let frame = obj.item_id as usize;
                        if let Some(pix) = obj_sheet.frame_pixels(frame) {
                            let rel_x = obj.x as i32 - map_x as i32 - (SPRITE_W as i32 / 2);
                            let rel_y = obj.y as i32 - map_y as i32 - (OBJ_SPRITE_H as i32 / 2);
                            Self::blit_obj_to_framebuf(pix, rel_x, rel_y, OBJ_SPRITE_H, &mut mr.framebuf, fb_w, fb_h);
                        }
                    }
                }
                // Sprite-depth masking: apply per-sprite tile masking.
                {
                    use crate::game::sprite_mask::{apply_sprite_mask, BlittedSprite};
                    use crate::game::sprites::{SPRITE_W, SPRITE_H, OBJ_SPRITE_H};

                    // Hero sprite masking
                    let (hero_rx, mut hero_ry) = Self::actor_rel_pos(
                        self.state.hero_x, self.state.hero_y, map_x, map_y,
                    );
                    if self.submerged { hero_ry += 8; }
                    let hero_sprite = BlittedSprite {
                        screen_x: hero_rx,
                        screen_y: hero_ry,
                        width: SPRITE_W,
                        height: SPRITE_H,
                        ground: hero_ry + SPRITE_H as i32,
                        is_falling: false, // TODO: wire to actor FALL state when actor states are implemented
                    };
                    apply_sprite_mask(mr, &hero_sprite, self.state.hero_sector, 0);

                    // World object masking
                    for obj in &self.state.world_objects {
                        if !obj.visible || obj.region != self.state.region_num { continue; }
                        let rel_x = obj.x as i32 - map_x as i32 - (SPRITE_W as i32 / 2);
                        let rel_y = obj.y as i32 - map_y as i32 - (OBJ_SPRITE_H as i32 / 2);
                        let obj_sprite = BlittedSprite {
                            screen_x: rel_x,
                            screen_y: rel_y,
                            width: SPRITE_W,
                            height: OBJ_SPRITE_H,
                            ground: rel_y + OBJ_SPRITE_H as i32,
                            is_falling: false,
                        };
                        apply_sprite_mask(mr, &obj_sprite, self.state.hero_sector, 0);
                    }
                }
            }
        }

        self.render_by_viewstatus(canvas, resources);
        if self.quit_requested {
            SceneResult::Quit
        } else {
            SceneResult::Continue
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_facing_to_frame_base() {
        // Rust facing: 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW
        assert_eq!(GameplayScene::facing_to_frame_base(0), 16); // N  → northwalk
        assert_eq!(GameplayScene::facing_to_frame_base(1), 24); // NE → eastwalk
        assert_eq!(GameplayScene::facing_to_frame_base(2), 24); // E  → eastwalk
        assert_eq!(GameplayScene::facing_to_frame_base(3), 0);  // SE → southwalk
        assert_eq!(GameplayScene::facing_to_frame_base(4), 0);  // S  → southwalk
        assert_eq!(GameplayScene::facing_to_frame_base(5), 8);  // SW → westwalk
        assert_eq!(GameplayScene::facing_to_frame_base(6), 8);  // W  → westwalk
        assert_eq!(GameplayScene::facing_to_frame_base(7), 16); // NW → northwalk
    }

    #[test]
    fn test_npc_type_to_cfile() {
        use crate::game::npc::*;
        // Enemy humans → ogre sheet
        assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_HUMAN, RACE_ENEMY), Some(6));
        // Named humans → None (SetFig pass)
        assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_HUMAN, RACE_NORMAL), None);
        assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_HUMAN, RACE_SHOPKEEPER), None);
        // Enemy types
        assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_ORC,      RACE_ENEMY),  Some(6));
        assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_GHOST,    RACE_UNDEAD), Some(7));
        assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_SKELETON, RACE_UNDEAD), Some(7));
        assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_WRAITH,   RACE_WRAITH), Some(7));
        assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_DRAGON,   RACE_ENEMY),  Some(10));
        assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_SWAN,     RACE_NORMAL), Some(11));
        assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_HORSE,    RACE_NORMAL), Some(5));
        assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_RAFT,     RACE_NORMAL), Some(4));
        // Inactive / container → None
        assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_NONE,      RACE_NORMAL), None);
        assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_CONTAINER, RACE_NORMAL), None);
        // Unknown type → default ogre sheet
        assert_eq!(GameplayScene::npc_type_to_cfile(99, RACE_ENEMY), Some(6));
        // Beggar → SetFig pass (not enemy)
        assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_HUMAN, RACE_BEGGAR), None);
    }

    #[test]
    fn test_enemy_npc_render_pass_writes_pixels() {
        use crate::game::sprites::{SpriteSheet, SPRITE_W, SPRITE_H};
        use crate::game::npc::{Npc, NpcTable, NPC_TYPE_ORC, RACE_ENEMY};
        use crate::game::game_state::GameState;
        use crate::game::map_renderer::{MAP_DST_W, MAP_DST_H};

        // Build a minimal mock sprite sheet for cfile 6 (ogre).
        // Pixel value 0 is non-transparent (only 31 is transparent).
        let frames = 64;
        let mock_sheet = SpriteSheet {
            cfile_idx: 6,
            pixels: vec![0u8; SPRITE_W * SPRITE_H * frames],
            num_frames: frames,
            frame_h: SPRITE_H,
        };

        // 18-element vec; only slot 6 is Some.
        let mut sheets: Vec<Option<SpriteSheet>> = (0..18).map(|_| None).collect();
        sheets[6] = Some(mock_sheet);

        let mut state = GameState::new();
        // Hero at viewport center (map_x=0, map_y=0), hero at (8, 26) so rel=(0,0)
        state.hero_x = 8;
        state.hero_y = 26;

        // Place an ORC near the hero but offset so it appears in viewport
        let mut table = NpcTable { npcs: Default::default() };
        table.npcs[0] = Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 80,  // rel_x = 80 - 0 - 8 = 72, well within 304px viewport
            y: 80,  // rel_y = 80 - 0 - 26 = 54
            vitality: 10,
            gold: 5,
            speed: 2,
            active: true,
        };

        let mut framebuf = vec![31u8; (MAP_DST_W * MAP_DST_H) as usize]; // all transparent
        GameplayScene::blit_actors_to_framebuf(
            &sheets, &None, &state, &Some(table), 0, 0, &mut framebuf, false,
        );

        // At least some pixels in the ORC's blit area should have been overwritten to 0
        let orc_area_start = (54 * MAP_DST_W as usize) + 72;
        let has_written = framebuf[orc_area_start..orc_area_start + SPRITE_W]
            .iter()
            .any(|&p| p == 0);
        assert!(has_written, "expected ORC pixels to be written to framebuf");
    }

    #[test]
    fn test_setfig_render_pass_writes_pixels() {
        use crate::game::sprites::{SpriteSheet, SPRITE_W, SPRITE_H};
        use crate::game::npc::{Npc, NpcTable, NPC_TYPE_HUMAN, RACE_SHOPKEEPER};
        use crate::game::game_state::GameState;
        use crate::game::map_renderer::{MAP_DST_W, MAP_DST_H};

        // Bartender uses cfile 15 (SETFIG_TABLE[8]).
        let mock_sheet = SpriteSheet {
            cfile_idx: 15,
            pixels: vec![0u8; SPRITE_W * SPRITE_H * 8],
            num_frames: 8,
            frame_h: SPRITE_H,
        };
        let mut sheets: Vec<Option<SpriteSheet>> = (0..18).map(|_| None).collect();
        sheets[15] = Some(mock_sheet);

        let mut state = GameState::new();
        state.hero_x = 8;
        state.hero_y = 26;

        let mut table = NpcTable { npcs: Default::default() };
        table.npcs[0] = Npc {
            npc_type: NPC_TYPE_HUMAN,
            race: RACE_SHOPKEEPER,
            x: 80, y: 80,
            vitality: 10, gold: 0, speed: 0,
            active: true,
        };

        let mut framebuf = vec![31u8; (MAP_DST_W * MAP_DST_H) as usize];
        GameplayScene::blit_actors_to_framebuf(
            &sheets, &None, &state, &Some(table), 0, 0, &mut framebuf, false,
        );

        let setfig_area_start = (54 * MAP_DST_W as usize) + 72;
        let has_written = framebuf[setfig_area_start..setfig_area_start + SPRITE_W]
            .iter()
            .any(|&p| p == 0);
        assert!(has_written, "expected SetFig pixels to be written to framebuf");
    }

    #[test]
    fn test_scatter_items_adds_world_objects() {
        use crate::game::game_state::{GameState, WorldObject};
        use crate::game::sprites::INV_LIST;

        let mut state = GameState::new();
        state.hero_x = 1000;
        state.hero_y = 1000;
        state.region_num = 3;

        const TALISMAN_IDX: usize = 22;
        let count = 5usize;
        let safe_pool: Vec<usize> = (0..INV_LIST.len()).filter(|&i| i != TALISMAN_IDX).collect();
        let n = count.min(safe_pool.len());
        for i in 0..n {
            let item_id = safe_pool[i % safe_pool.len()];
            let angle = 2.0f32 * std::f32::consts::PI * (i as f32) / (n as f32);
            let x = (state.hero_x as i32 + (80.0f32 * angle.cos()) as i32).clamp(0, 0x7FFF) as u16;
            let y = (state.hero_y as i32 + (80.0f32 * angle.sin()) as i32).clamp(0, 0x7FFF) as u16;
            state.world_objects.push(WorldObject {
                item_id: item_id as u8,
                region: state.region_num,
                x, y,
                visible: true,
            });
        }
        assert_eq!(state.world_objects.len(), 5);
        assert!(state.world_objects.iter().all(|o| o.item_id != TALISMAN_IDX as u8));
    }
}
