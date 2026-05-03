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

mod narrative;
mod proximity;
mod carriers;
mod game_event;
mod combat_logic;
mod region;
mod items;
mod environ;
mod rendering;
mod npc_interaction;
mod actors;
mod input;
mod menu_actions;
mod debug_commands;
mod scene_impl;

#[cfg(test)]
mod tests;

/// Attack animation transition table from fmain.c:132-140.
/// Each entry has 4 possible next states, selected by rand4().
/// States 0-8 represent weapon swing positions.
const FIGHT_TRANS_LIST: [[u8; 4]; 9] = [
    [1, 8, 0, 1], // 0: arm down, weapon low
    [2, 0, 1, 0], // 1: arm down, weapon diagonal down
    [3, 1, 2, 8], // 2: arm swing1, weapon horizontal
    [4, 2, 3, 7], // 3: arm swing2, weapon raised
    [5, 3, 4, 6], // 4: arm swing2, weapon diag up
    [6, 4, 5, 5], // 5: arm swing2, weapon high
    [8, 5, 6, 4], // 6: arm high, weapon up
    [8, 6, 7, 3], // 7: arm high, weapon horizontal
    [0, 6, 8, 2], // 8: arm middle, weapon raise fwd
];

/// Proximity radius (pixels) for auto-speech checks (SPEC §13.4).
/// Per `reference/logic/game-loop.md#sort_sprites` (`fmain.c:2370`): speech
/// proximity radius is 50 px.
const PROXIMITY_SPEECH_RANGE: i32 = 50;
/// Princess world object index (ob_list8[9]) used for captive flag checks.
const PRINCESS_OB_INDEX: usize = 9;

/// Advance the fight animation state using trans_list random transitions.
/// `state`: current fight state (0-8). `tick`: game cycle for randomness.
fn advance_fight_state(state: u8, tick: u32) -> u8 {
    let idx = (state as usize).min(8);
    let col = crate::game::combat::rand4(tick);
    FIGHT_TRANS_LIST[idx][col]
}

/// Compute pixel offset for pushback in a facing direction.
fn push_offset(facing: u8, distance: i32) -> (i32, i32) {
    match facing & 7 {
        0 => (0, -distance),
        1 => (distance, -distance),
        2 => (distance, 0),
        3 => (distance, distance),
        4 => (0, distance),
        5 => (-distance, distance),
        6 => (-distance, 0),
        7 => (-distance, -distance),
        _ => (0, 0),
    }
}

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
        // F1-F7: magic spell shortcuts (fmain.c:537-547, key codes 10-16)
        Keycode::F1     => Some(10),
        Keycode::F2     => Some(11),
        Keycode::F3     => Some(12),
        Keycode::F4     => Some(13),
        Keycode::F5     => Some(14),
        Keycode::F6     => Some(15),
        Keycode::F7     => Some(16),
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

fn default_brother_names() -> Vec<String> {
    vec!["Julian".to_string(), "Phillip".to_string(), "Kevin".to_string()]
}

fn compass_dir_for_facing(facing: u8) -> usize {
    match facing {
        0..=7 => ((facing as usize) + 1) % 8,
        _ => 9,
    }
}

/// Pick the compass highlight segment (comptable index 0..=7) or 9 for
/// "no highlight" per SPECIFICATION §25.7.
///
/// Per RESEARCH.md §4.5 / §4.6, the highlight is driven by the resolved input
/// direction this frame (`oldir`), not by persistent `facing`. When input is
/// idle (`Direction::None`), index 9 is returned, which is a null comptable
/// region — the base `_hinor` bitmap renders with no `_hivar` overlay.
fn compass_dir_for_input(dir: Direction) -> usize {
    match dir {
        Direction::NW => 0,
        Direction::N  => 1,
        Direction::NE => 2,
        Direction::E  => 3,
        Direction::SE => 4,
        Direction::S  => 5,
        Direction::SW => 6,
        Direction::W  => 7,
        Direction::None => 9,
    }
}

/// Find the next owned weapon slot in the given direction.
/// `current` is the 1-based weapon value (1=Dirk..5=Wand, matching actor.weapon).
/// `direction` is +1 (next) or -1 (prev).
/// `stuff` is the player's inventory array.
/// Returns `Some(new_weapon_value)` if a different weapon is found, `None` otherwise.
fn cycle_weapon_slot(current: u8, direction: i8, stuff: &[u8; 36]) -> Option<u8> {
    let weapon_count: i8 = 5; // weapons 1..=5, stuff indices 0..=4
    let cur_0 = (current as i8 - 1).max(0); // convert to 0-based index
    for offset in 1..weapon_count {
        let idx_0 = (cur_0 + direction * offset).rem_euclid(weapon_count) as usize;
        if stuff[idx_0] > 0 {
            return Some((idx_0 as u8) + 1); // return 1-based weapon value
        }
    }
    None
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
use crate::game::key_bindings::{ControllerBindings, ControllerMode, GameAction, KeyBindings};
use crate::game::narrative_sequence::{NarrativeQueue, NarrativeStep};
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

/// Cursor state for controller-driven HI bar menu navigation.
#[derive(Debug, Clone, Default)]
struct MenuCursor {
    col: usize,   // 0 or 1 (HI bar is 2 columns)
    row: usize,   // 0–5 (6 rows)
    active: bool,  // true when in menu mode
}

impl MenuCursor {
    fn navigate_up(&mut self) {
        self.row = if self.row == 0 { 5 } else { self.row - 1 };
    }

    fn navigate_down(&mut self) {
        self.row = if self.row == 5 { 0 } else { self.row + 1 };
    }

    fn navigate_left(&mut self) {
        self.col = if self.col == 0 { 1 } else { 0 };
    }

    fn navigate_right(&mut self) {
        self.col = if self.col == 1 { 0 } else { 1 };
    }

    /// Returns the display slot index for MenuState::handle_click().
    fn slot(&self) -> usize {
        self.row * 2 + self.col
    }
}

pub struct GameplayScene {
    pub state: Box<GameState>,
    pub messages: MessageQueue,
    tick_accum: u32,
    input: InputState,
    map_x: u16,
    map_y: u16,
    last_mood: u8,
    mood_tick: u32,
    pending_music_toggle: Option<bool>,
    pending_sound_toggle: Option<bool>,
    map_renderer: Option<MapRenderer>,
    map_world: Option<crate::game::world_data::WorldData>,
    adf: Option<crate::game::adf::AdfDisk>,
    shadow_mem: Vec<u8>,
    adf_load_attempted: bool,
    rebinding: RebindingState,
    local_bindings: KeyBindings,
    controller_mode: ControllerMode,
    controller_bindings: ControllerBindings,
    menu_cursor: MenuCursor,
    last_region_num: u8,
    palette_transition: Option<crate::game::palette::PaletteTransition>,
    last_indoor: bool,
    pub in_encounter_zone: bool,
    pub npc_table: Option<crate::game::npc::NpcTable>,
    /// RGBA32 palette for the final indexed→RGBA32 render step.
    current_palette: crate::game::palette::Palette,
    /// Base palette loaded from faery.toml (colors::Palette with RGB4 values).
    /// Used as input to fade_page() for day/night/jewel palette computation.
    /// None until init_from_library() runs.
    base_colors_palette: Option<crate::game::colors::Palette>,
    /// Forces a palette recompute on the next cadence tick (set on region load/transition).
    palette_dirty: bool,

    witch_effect: WitchEffect,
    teleport_effect: TeleportEffect,
    pub missiles: [crate::game::combat::Missile; crate::game::combat::MAX_MISSILES],
    /// Frames remaining before an archer NPC can fire again.
    archer_cooldown: u32,
    /// Debug log lines buffered for the debug window. Drained each frame by main loop.
    log_buffer: Vec<String>,
    /// Categorized debug log entries buffered for the debug window. Drained each
    /// frame by the main loop and forwarded into `DebugConsole::log_entry`.
    /// Parallel to `log_buffer` during the DBG-LOG-04 migration.
    pub pending_log: Vec<crate::game::debug_log::DebugLogEntry>,
    /// Set to true when the player requests to quit the game.
    quit_requested: bool,
    /// Set to true when the Talisman win condition fires (`stuff[22]` set
    /// after an item pickup). Drives the Gameplay→VictoryScene transition
    /// in `main.rs`. Mirrors `quitflag = TRUE; viewstatus = 2` from
    /// `fmain.c:3244-3247`.
    victory_triggered: bool,
    /// Deterministic gameplay-owned scripted sequence runner.
    narrative_queue: NarrativeQueue,
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
    /// Brother display names (datanames[brother-1]).
    brother_names: Vec<String>,
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
    /// Last nearest person for proximity auto-speech (SPEC §13.4).
    last_person: Option<PersonId>,
    /// Trigger princess rescue sequence on next frame.
    trigger_princess_rescue: bool,
    /// Hero is in forced sleep (events 12/24).
    sleeping: bool,
    /// True when hero is in the volcanic region (lava damage active).
    /// Mirrors fiery_death global from fmain.c:1554.
    fiery_death: bool,
    /// Death countdown active (goodfairy timer running).
    dying: bool,
    /// Goodfairy countdown: u8 semantic, held as i16 for arithmetic convenience.
    /// Initialised to 255 when hero dies.  Counts down each frame (30 Hz) toward 1.
    /// Timeline (SPEC §20.2):
    ///   255–200 (~56 frames): death sequence / song
    ///   199–120 (~80 frames): luck gate fires once (luck < 1 → brother succession)
    ///   119–20  (~100 frames): fairy sprite flying; battleflag cleared
    ///   19–2    (~18 frames): resurrection glow
    ///   1       (frame 256): revive(FALSE) — fairy rescues hero
    goodfairy: i16,
    /// True once the luck gate (goodfairy crossing below 200) has fired this death cycle.
    /// Prevents the gate from re-firing on subsequent frames.
    luck_gate_fired: bool,
    /// Death type for event message (5=combat, 6=drowning, 27=lava, 0=starvation).
    death_type: usize,
    /// SPEC §13.2: per-world-object TALKING flicker timer (15 ticks).
    /// Keyed by `world_idx` into `state.world_objects`. While > 0, the
    /// SetFig sprite's frame index gets `+ bitrand(1)` on render
    /// (`fmain.c:1556` — `dex += rand2()`). Decremented each tick.
    talk_flicker: std::collections::HashMap<usize, u8>,
}

/// What kind of figure was found by nearest_fig.
enum FigKind {
    /// An enemy NPC from npc_table, with its index. Includes Dead bodies
    /// (they remain in npc_table until ClearEncounters); search_body decides
    /// what to do with them.
    Npc(usize),
    /// A setfig from world_objects, with its index and setfig type (ob_id).
    SetFig { world_idx: usize, setfig_type: u8 },
    /// A pickable ground item from world_objects (`ob_stat != 3`). Returned
    /// only by `nearest_fig(constraint=0)`.
    Item { world_idx: usize, ob_id: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PersonId {
    Npc(usize),
    SetFig(usize),
}

impl From<&FigKind> for PersonId {
    fn from(kind: &FigKind) -> Self {
        match kind {
            FigKind::Npc(idx) => PersonId::Npc(*idx),
            FigKind::SetFig { world_idx, .. } => PersonId::SetFig(*world_idx),
            FigKind::Item { world_idx, .. } => PersonId::SetFig(*world_idx),
        }
    }
}

/// Result of nearest_fig search.
pub(crate) struct NearestFig {
    kind: FigKind,
    dist: i32,
}

impl GameplayScene {
    pub fn new() -> Self {
        GameplayScene {
            state: Box::new(GameState::new()),
            messages: MessageQueue::new(),
            tick_accum: 0,
            input: InputState::default(),
            map_x: 0,
            map_y: 0,
            last_mood: u8::MAX,
            pending_music_toggle: None,
            pending_sound_toggle: None,
            mood_tick: 0,
            map_renderer: None,
            map_world: None,
            adf: None,
            shadow_mem: Vec::new(),
            adf_load_attempted: false,
            rebinding: RebindingState { active: false, waiting_for_action: None },
            local_bindings: KeyBindings::default_bindings(),
            controller_mode: ControllerMode::Gameplay,
            controller_bindings: ControllerBindings::default_bindings(),
            menu_cursor: MenuCursor::default(),
            last_region_num: u8::MAX,
            palette_transition: None,
            last_indoor: false,
            in_encounter_zone: false,
            npc_table: None,
            current_palette: [0xFF808080_u32; crate::game::palette::PALETTE_SIZE],
            base_colors_palette: None,
            palette_dirty: true,

            witch_effect: WitchEffect::new(),
            teleport_effect: TeleportEffect::new(),
            missiles: std::array::from_fn(|_| crate::game::combat::Missile::default()),
            archer_cooldown: 0,
            log_buffer: Vec::new(),
            pending_log: Vec::new(),
            quit_requested: false,
            victory_triggered: false,
            narrative_queue: NarrativeQueue::default(),
            paused: false,
            compass_regions: Vec::new(),
            menu: crate::game::menu::MenuState::new(),
            textcolors: [0u32; 32],
            sprite_sheets: (0..crate::game::sprites::CFILE_COUNT).map(|_| None).collect(),
            object_sprites: None,
            narr: crate::game::game_library::NarrConfig::default(),
            brother_names: default_brother_names(),
            doors: Vec::new(),
            opened_doors: std::collections::HashSet::new(),
            bumped_door: None,
            zones: Vec::new(),
            last_zone: None,
            last_person: None,
            trigger_princess_rescue: false,
            sleeping: false,
            fiery_death: false,
            dying: false,
            goodfairy: 0,
            luck_gate_fired: false,
            death_type: 0,
            talk_flicker: std::collections::HashMap::new(),
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
        let mut names: Vec<String> = game_lib.brothers.iter().map(|b| b.name.clone()).collect();
        if names.is_empty() {
            names = default_brother_names();
        } else if names.len() < 3 {
            let defaults = default_brother_names();
            for idx in names.len()..3 {
                names.push(defaults[idx].clone());
            }
        }
        self.brother_names = names;
        self.update_brother_substitution();

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

        // Push startup event message (original: revive() calls event(9) +
        // print_cont(".") for Julian, event(10/11) for later brothers).
        let bname = self.brother_name().to_string();
        let mut msg9 = crate::game::events::event_msg(&self.narr, 9, &bname);
        match self.state.brother {
            1 => { msg9.push('.'); self.messages.push_wrapped(msg9); }
            2 => { self.messages.push_wrapped(msg9);
                    self.messages.push_wrapped(
                        crate::game::events::event_msg(&self.narr, 10, &bname)); }
            _ => { self.messages.push_wrapped(msg9);
                    self.messages.push_wrapped(
                        crate::game::events::event_msg(&self.narr, 11, &bname)); }
        }

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

    /// True once the Talisman win condition has fired. `main.rs` observes this
    /// on `SceneResult::Done` to branch into the victory sequence rather than
    /// restarting gameplay.
    pub fn is_victory(&self) -> bool { self.victory_triggered }

    /// Current hero's display name ("Julian", "Phillip", "Kevin"). Used by
    /// external scenes (e.g. the victory placard) that need `%`-substitution.
    pub fn hero_name(&self) -> &str { self.brother_name() }

    fn brother_name(&self) -> &str {
        let idx = self.state.brother.saturating_sub(1) as usize;
        self.brother_names
            .get(idx)
            .map(|s| s.as_str())
            .unwrap_or("Kevin")
    }

    fn update_brother_substitution(&mut self) {
        let name = self.brother_name().to_string();
        self.messages.set_substitution(name);
    }

    /// Current zone index and label for the debug console.
    pub fn current_zone_info(&self) -> (Option<usize>, Option<String>) {
        let label = self.last_zone
            .and_then(|i| self.zones.get(i).map(|z| z.label.clone()));
        (self.last_zone, label)
    }

    /// Enable or disable echoing every new message to stdout (--echo-transcript flag).
    pub fn set_echo_transcript(&mut self, echo: bool) {
        self.messages.set_echo(echo);
    }

    /// T3-CARRY-TURTLE-AUTO: Autonomous turtle movement when unmounted (SPEC §21.3).
    ///
    /// Runs EVERY tick (`fmain.c:1520-1542`). The turtle:
    ///
    /// 1. Probes 4 directions in priority order from current `facing`:
    ///    `d`, `(d+1)&7`, `(d-1)&7`, `(d-2)&7`. Each probe steps **3 pixels**
    ///    and commits only when BOTH probe points return terrain type **5**
    ///    (very deep water). Types 2–4 and all land are impassable.
    /// 2. **Does not persist facing** on success or failure — the autonomous
    ///    handler exits via `goto raise` which bypasses the `facing = d` write
    ///    at `newloc:` (`fmain.c:1545, 1633`). Facing is instead updated every
    ///    16 ticks by the CARRIER AI path via `set_course(SC_AIM)` aimed at
    ///    the hero — producing slow hero-seeking drift.

    /// SPEC §17.5: Returns `true` when `day_fade()` should update the palette.
    ///
    /// Fires every 4 ticks (`daynight & 3 == 0`) or during a screen rebuild
    /// (`viewstatus > 97`), matching the original Amiga cadence exactly.
    #[inline]
    pub(crate) fn should_update_palette(daynight: u16, viewstatus: u8) -> bool {
        (daynight & 3) == 0 || viewstatus > 97
    }

}
