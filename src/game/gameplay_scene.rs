//! Main gameplay scene: game loop, input, rendering.
//!
//! # Screen layout
//!
//! The original Amiga game used two Copper-switched viewports stacked vertically:
//! - `vp_page` (LORES, 288×140): the playfield
//! - `vp_text` (HIRES, 640×57): the HI bar (buttons, compass, messages)
//!
//! Both are 2× line-doubled (NTSC 60 Hz non-interlaced → line-doubled to fill 400 lines)
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DayNightPhase {
    Day,   // lightlevel < 60
    Dawn,  // 60-120 (transition)
    Dusk,  // 121-180 (transition)
    Night, // >180
}

impl DayNightPhase {
    pub fn from_lightlevel(level: u16) -> Self {
        match level {
            0..=59    => Self::Day,
            60..=120  => Self::Dawn,
            121..=180 => Self::Dusk,
            _         => Self::Night,
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
    map_renderer: Option<MapRenderer>,
    map_world: Option<crate::game::world_data::WorldData>,
    adf: Option<crate::game::adf::AdfDisk>,
    adf_load_attempted: bool,
    rebinding: RebindingState,
    local_bindings: KeyBindings,
    last_region_num: u8,
    palette_transition: Option<crate::game::palette::PaletteTransition>,
    last_indoor: bool,
    pub in_encounter_zone: bool,
    pub npc_table: Option<crate::game::npc::NpcTable>,
    day_night_phase: DayNightPhase,
    /// Last lightlevel used for atlas dim — triggers rebuild when it changes.
    last_lightlevel: u16,

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
    /// Narrative strings from faery.toml [narr], used by event_msg / speak helpers.
    narr: crate::game::game_library::NarrConfig,
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
            map_renderer: None,
            map_world: None,
            adf: None,
            adf_load_attempted: false,
            rebinding: RebindingState { active: false, waiting_for_action: None },
            local_bindings: KeyBindings::default_bindings(),
            last_region_num: u8::MAX,
            palette_transition: None,
            last_indoor: false,
            in_encounter_zone: false,
            npc_table: None,
            day_night_phase: DayNightPhase::Day,
            last_lightlevel: u16::MAX,

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
            narr: crate::game::game_library::NarrConfig::default(),
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

        let stuff = self.state.stuff().clone();
        let wealth = self.state.wealth;
        self.menu.set_options(&stuff, wealth);
    }

    /// Returns true when it is daytime (lightlevel > 60).
    pub fn is_daytime(state: &GameState) -> bool {
        state.lightlevel > 60
    }

    /// Push a debug/status message to the log buffer (shown in debug window).
    fn dlog(&mut self, msg: impl Into<String>) {
        self.log_buffer.push(msg.into());
    }

    /// Drain buffered debug log lines. Called by the main loop to forward to the debug window.
    pub fn drain_logs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.log_buffer)
    }

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
        let dir = self.current_direction();

        let (dx, dy): (i32, i32) = match dir {
            Direction::N    => ( 0, -1),
            Direction::NE   => ( 1, -1),
            Direction::E    => ( 1,  0),
            Direction::SE   => ( 1,  1),
            Direction::S    => ( 0,  1),
            Direction::SW   => (-1,  1),
            Direction::W    => (-1,  0),
            Direction::NW   => (-1, -1),
            Direction::None => ( 0,  0),
        };

        let prev_x = self.state.hero_x;
        let prev_y = self.state.hero_y;

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


            let new_x = (self.state.hero_x as i32 + dx * speed).clamp(0, 0x7FF0) as u16;
            let new_y = (self.state.hero_y as i32 + dy * speed).clamp(0, 0x3FF0) as u16;

            // Turtle guardrail: turtle rides water but cannot enter hard-block terrain (mountains).
            let turtle_blocked = self.state.on_raft
                && self.state.active_carrier == crate::game::game_state::CARRIER_TURTLE
                && self.map_world.as_ref().map_or(false, |world| {
                    collision::px_to_terrain_type(world, new_x as i32, new_y as i32) == 1
                });

            if !turtle_blocked && (self.state.flying != 0 || self.state.on_raft || collision::proxcheck(self.map_world.as_ref(), new_x as i32, new_y as i32)) {
                self.state.hero_x = new_x;
                self.state.hero_y = new_y;
                if let Some(door) = crate::game::doors::doorfind(self.state.region_num, new_x, new_y) {
                    self.state.region_num = door.dst_region;
                    self.state.hero_x = door.dst_x;
                    self.state.hero_y = door.dst_y;
                    self.dlog(format!("door: region transition to {}", door.dst_region));
                }
                // Track safe spawn point after successful movement.
                if let Some(ref world) = self.map_world {
                    let terrain = collision::px_to_terrain_type(
                        world, self.state.hero_x as i32, self.state.hero_y as i32,
                    );
                    self.state.update_safe_spawn(terrain);
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
        // Rate-limited to one swing every 20 ticks (~1/3 s at 60 Hz), matching
        // fmain.c's per-frame proximity check gated by weapon animation state.
        if self.fight_cooldown > 0 {
            self.fight_cooldown -= 1;
        }
        if self.input.fight && self.fight_cooldown == 0 {
            self.apply_melee_combat();
            self.fight_cooldown = 20;
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

        // Visual fatigue feedback: per-step counter is kept for possible future animation,
        // but forced sleep is now driven by the daynight tick (player-111).
        self.state.fatigue_step(moved);
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
                                self.messages.push(format!("You found a better weapon (type {})!", w));
                            }
                        }
                        self.messages.push(format!("Enemy slain! Bravery: {}", self.state.brave));
                    } else {
                        self.messages.push(format!(
                            "Enemy slain! Bravery: {}", self.state.brave
                        ));
                    }
                } else {
                    self.messages.push(format!("You hit for {}!", damage));
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
            for npc in &mut table.npcs {
                let adjacent = npc.tick(hero_x, hero_y);
                if adjacent && npc.active {
                    self.messages.push(format!("An enemy approaches!"));
                }
            }
        }

        // npc-105: Archer NPCs (Goal::Archer1/Archer2) fire missiles toward hero.
        // Rate-limited: one shot per NPC group every 30 ticks (~0.5s at 60Hz),
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
                self.archer_cooldown = 30;
                break; // one archer fires per cycle
            }
        }
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
                        // SAFETY: reinterpreting Vec<u32> as &[u8] — same memory, valid alignment.
                        let pixels_u8: &[u8] = unsafe {
                            std::slice::from_raw_parts(
                                mr.framebuf.as_ptr() as *const u8,
                                mr.framebuf.len() * 4,
                            )
                        };
                        let mut pixels_copy = pixels_u8.to_vec();
                        let tc = canvas.texture_creator();
                        let surface_result = sdl2::surface::Surface::from_data(
                            &mut pixels_copy,
                            crate::game::map_renderer::MAP_DST_W,
                            crate::game::map_renderer::MAP_DST_H,
                            crate::game::map_renderer::MAP_DST_W * 4,
                            // ARGB8888 = 0xAARRGGBB as u32, matching amiga_color_to_rgba output.
                            // RGBA32 is ABGR8888 on little-endian (0xAABBGGRR), which would swap R/B.
                            sdl2::pixels::PixelFormatEnum::ARGB8888,
                        );
                        if let Ok(surface) = surface_result {
                            if let Ok(tex) = tc.create_texture_from_surface(&surface) {
                                // Clip framebuf to the visible LORES viewport (vp_page.DWidth × DHeight)
                                // then 2× scale to canvas. Framebuf may be wider/taller than visible
                                // area due to tile-grid rounding; copper did the clipping on Amiga.
                                let src = sdl2::rect::Rect::new(
                                    0, 0,
                                    PLAYFIELD_LORES_W,
                                    PLAYFIELD_LORES_H,
                                );
                                let dst = sdl2::rect::Rect::new(
                                    PLAYFIELD_X, PLAYFIELD_Y,
                                    PLAYFIELD_CANVAS_W,
                                    PLAYFIELD_CANVAS_H,
                                );
                                let _ = canvas.copy(&tex, Some(src), Some(dst));
                            }
                        }
                    }
                }

                // HI bar: render all content into a native 640×57 offscreen texture,
                // then blit it 2× vertically to canvas (640×114). This makes fonts,
                // buttons, and the compass scale uniformly without per-element ×2 math.
                {
                    // Collect all render data before the with_texture_canvas closure.
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
                    // Compass highlight reflects current input, not player movement.
                    // Same comptable order as propt: NW=0, N=1, NE=2, E=3, SE=4, S=5, SW=6, W=7.
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
                    // Extract resource references before mutably borrowing canvas.
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

                            // Background: hiscreen IFF at native 1:1 size (640×57).
                            if let Some(hiscreen) = hiscreen_opt {
                                hiscreen.draw_scaled(hc, sdl2::rect::Rect::new(0, 0, 640, HIBAR_NATIVE_H));
                            } else {
                                hc.set_draw_color(sdl2::pixels::Color::RGB(80, 60, 20));
                                hc.fill_rect(sdl2::rect::Rect::new(0, 0, 640, HIBAR_NATIVE_H)).ok();
                            }

                            // Stat line: five separate fields matching fmain2.c case 7 + case 4.
                            // Original: move(14,52) «Brv:», move(90,52) «Lck:», move(168,52) «Knd:»,
                            //           move(245,52) «Vit:», move(321,52) «Wlth:» — all baseline y=52.
                            amber_font.set_color_mod(0xAA, 0x55, 0x00);
                            amber_font.render_string(&format!("Brv:{:3}", brave),     hc, 14,  52);
                            amber_font.render_string(&format!("Lck:{:3}", luck),      hc, 90,  52);
                            amber_font.render_string(&format!("Knd:{:3}", kind),      hc, 168, 52);
                            amber_font.render_string(&format!("Vit:{:3}", vitality),  hc, 245, 52);
                            amber_font.render_string(&format!("Wlth:{:3}", wealth),   hc, 321, 52);

                            // Scrolling messages: fmain2.c print() — TXMIN=16, newest at baseline y=42,
                            // older lines ScrollRaster(0,10) → each prior line is 10px higher.
                            for (i, msg) in msgs_visible.iter().enumerate() {
                                let line_from_bottom = (msg_count - 1 - i) as i32;
                                let y = 42 - line_from_bottom * 10;
                                amber_font.render_string(msg, hc, 16, y);
                            }

                            // Button grid: propt() native formula y = row*9+8 (HIRES px).
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

                            // Compass: native HIRES pixel coords within the 57px band.
                            const COMPASS_X: i32 = 567;
                            const COMPASS_SRC_Y: i32 = 15;
                            const COMPASS_SRC_W: u32 = 48;
                            const COMPASS_SRC_H: u32 = 24;
                            let compass_dest = sdl2::rect::Rect::new(
                                COMPASS_X, COMPASS_SRC_Y, COMPASS_SRC_W, COMPASS_SRC_H,
                            );
                            if let Some(normal_tex) = compass_normal {
                                hc.copy(normal_tex, None, compass_dest).ok();
                            }
                            if input_comptable_dir < compass_regions.len() {
                                let (rx, ry, rw, rh) = compass_regions[input_comptable_dir];
                                if rw > 1 || rh > 1 {
                                    if let Some(highlight_tex) = compass_highlight {
                                        let src = sdl2::rect::Rect::new(rx, ry, rw as u32, rh as u32);
                                        let dst = sdl2::rect::Rect::new(
                                            COMPASS_X + rx,
                                            COMPASS_SRC_Y + ry,
                                            rw as u32,
                                            rh as u32,
                                        );
                                        hc.copy(highlight_tex, src, dst).ok();
                                    }
                                }
                            }
                        });
                        // Blit offscreen HI bar to canvas, stretched 2× vertically.
                        canvas.copy(
                            &hibar_tex,
                            sdl2::rect::Rect::new(0, 0, 640, HIBAR_NATIVE_H),
                            sdl2::rect::Rect::new(0, HIBAR_Y, 640, HIBAR_H),
                        ).ok();
                    }; // semicolon: drops Result<Texture> temporary before tc is dropped
                }

                // Tick visual effects and composite them over the map.
                self.witch_effect.tick();
                if let Some((r, g, b, a)) = self.teleport_effect.tick() {
                    canvas.set_draw_color(sdl2::pixels::Color::RGBA(r, g, b, a));
                    canvas.fill_rect(None).ok();
                }
            }
            // Map view
            1 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 48, 0));
                canvas.clear();
                // "MAP VIEW" — text rendering pending font wiring
            }
            // Message overlay
            2 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(48, 48, 48));
                canvas.clear();
                // "MESSAGE" — text rendering pending font wiring
            }
            // Inventory screen
            4 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(64, 32, 0));
                canvas.clear();
                // "INVENTORY" — text rendering pending font wiring
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
        if let Some(ref adf) = self.adf {
            let world_result = if let Some(cfg) = game_lib.find_region_config(region) {
                crate::game::world_data::WorldData::load(
                    adf, region,
                    cfg.sector_block, cfg.map_block,
                    cfg.terra_block, cfg.terra2_block,
                    &cfg.image_blocks,
                )
            } else {
                Err(anyhow::anyhow!("no region config for region {}", region))
            };
            match world_result {
                Ok(world) => {
                    let palette = Self::region_palette(game_lib, region);
                    self.map_renderer = Some(MapRenderer::new(&world, &palette));
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
                let key_slot = 16 + idx as usize;
                if self.state.stuff()[key_slot] == 0 {
                    self.messages.push("No such key.".to_string());
                } else if crate::game::doors::doorfind(
                    self.state.region_num, self.state.hero_x, self.state.hero_y).is_some()
                {
                    self.state.stuff_mut()[key_slot] -= 1;
                    self.messages.push("Door opened.".to_string());
                } else {
                    self.messages.push("Key didn't fit.".to_string());
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
            MenuAction::SaveGame => {
                match crate::game::persist::save_game(&self.state, 0) {
                    Ok(()) => {
                        if let Err(e) = crate::game::persist::save_transcript(
                            self.messages.transcript(), 0,
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
            MenuAction::LoadGame => {
                // EXPLOIT FIX NEEDED: reset all runtime door state before restoring
                // save, otherwise keys replenish but doors stay unlocked.
                match crate::game::persist::load_game(0) {
                    Ok(new_state) => {
                        *self.state = new_state;
                        // Restore existing transcript so new messages are appended.
                        let existing = crate::game::persist::load_transcript(0);
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
                // TODO: call setmood/audio.set_score when audio integration is ready
            }
            MenuAction::ToggleSound => {
                let on = self.menu.is_sound_on();
                self.messages.push(if on { "Sound on." } else { "Sound off." });
                // TODO: guard effect() calls with is_sound_on() when audio sample playback is added
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
                self.messages.push("Inventory opened");
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
                    self.state.region_num, self.state.hero_x, self.state.hero_y
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
                self.fight_cooldown = 20;
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
                // Item pickup — full implementation requires an object actor scan (npc-002 / loot system).
                self.messages.push("Nothing here to take.");
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
            }
            AdjustInventory { index, delta } => {
                let stuff = self.state.stuff_mut();
                if (index as usize) < stuff.len() {
                    stuff[index as usize] = stuff[index as usize].saturating_add_signed(delta);
                }
            }
            TeleportSafe => {
                self.state.hero_x = self.state.safe_x;
                self.state.hero_y = self.state.safe_y;
            }
            TeleportCoords { x, y } => {
                self.state.hero_x = x;
                self.state.hero_y = y;
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
            }
            SetGameTime { hour, minute } => {
                // Each hour = 1000 daynight ticks; each minute ≈ 1000/60
                let ticks = (hour as u16).saturating_mul(1000)
                    + (minute as u16).saturating_mul(1000) / 60;
                self.state.daynight = ticks % 24000;
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
        }
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

    /// Blit one 16×32 sprite frame (RGBA32) into the map framebuf (sprite-103).
    /// Transparent pixels (alpha == 0) are skipped.
    /// `rel_x` / `rel_y` are the top-left destination in framebuf pixels.
    fn blit_sprite_to_framebuf(
        frame_pixels: &[u32],
        rel_x: i32,
        rel_y: i32,
        framebuf: &mut [u32],
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
                let src_px = frame_pixels[(row as usize) * SPRITE_W + col as usize];
                if src_px >> 24 == 0 { continue; } // transparent
                framebuf[(dst_y * fb_w + dst_x) as usize] = src_px;
            }
        }
    }

    /// Compute rel_x/rel_y for an actor at (abs_x, abs_y) given viewport origin (map_x, map_y).
    /// Matches original fmain.c:2150-2158: rel_x = abs_x - map_x - 8, rel_y = abs_y - map_y - 26.
    fn actor_rel_pos(abs_x: u16, abs_y: u16, map_x: u16, map_y: u16) -> (i32, i32) {
        let rel_x = (abs_x as i32) - (map_x as i32) - 8;
        let rel_y = (abs_y as i32) - (map_y as i32) - 26;
        (rel_x, rel_y)
    }

    /// Blit all visible actors (hero + enemy NPCs) onto the map framebuf (sprite-104).
    /// Called immediately after mr.compose() so actors appear on top of tiles.
    fn blit_actors_to_framebuf(
        sprite_sheets: &[Option<crate::game::sprites::SpriteSheet>],
        state: &GameState,
        npc_table: &Option<crate::game::npc::NpcTable>,
        map_x: u16,
        map_y: u16,
        framebuf: &mut Vec<u32>,
    ) {
        use crate::game::map_renderer::{MAP_DST_W, MAP_DST_H};
        use crate::game::sprites::{SPRITE_H, SPRITE_W};
        let fb_w = MAP_DST_W as i32;
        let fb_h = MAP_DST_H as i32;

        // --- Hero sprite ---
        // cfiles[0]=Julian (brother=1), [1]=Phillip (brother=2), [2]=Kevin (brother=3)
        let hero_cfile = state.brother.saturating_sub(1) as usize;
        if let Some(Some(ref sheet)) = sprite_sheets.get(hero_cfile) {
            let (rel_x, rel_y) = Self::actor_rel_pos(state.hero_x, state.hero_y, map_x, map_y);
            if rel_x > -(SPRITE_W as i32) && rel_x < fb_w && rel_y > -(SPRITE_H as i32) && rel_y < fb_h {
                let hero_facing = state.actors.first().map_or(0u8, |a| a.facing);
                // frames_per_dir: 56 frames / 8 dirs = 7 per dir for standard player
                let frames_per_dir = (sheet.num_frames / 8).max(1);
                let frame = (hero_facing as usize) * frames_per_dir;
                if let Some(fp) = sheet.frame_pixels(frame) {
                    Self::blit_sprite_to_framebuf(fp, rel_x, rel_y, framebuf, fb_w, fb_h);
                }
            }
        }

        // --- Enemy NPCs from npc_table ---
        // NPC npc_type maps to cfile index:
        //   NPC_TYPE_HUMAN=1 → cfile 6 (ogre file is default enemy)
        // Enemy blitting is best-effort: use npc_type as a rough cfile hint.
        // SetFig NPCs (wizard, king, etc.) use a separate placement system.
        let _ = npc_table; // reserved for future enemy sprite lookup
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
        if !tick_events.is_empty() {
            let bname = brother_name(&self.state);
            for ev in tick_events {
                let msg = crate::game::events::event_msg(&self.narr, ev as usize, bname);
                if !msg.is_empty() {
                    self.messages.push(msg);
                }
            }
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
                .unwrap_or("game/Faery Tale Adventure (MicroIllusions).adf");
            match crate::game::adf::AdfDisk::open(std::path::Path::new(adf_path)) {
                Ok(adf) => {
                    let region = self.state.region_num;
                    let world_result = if let Some(cfg) = game_lib.find_region_config(region) {
                        crate::game::world_data::WorldData::load(
                            &adf, region,
                            cfg.sector_block, cfg.map_block,
                            cfg.terra_block, cfg.terra2_block,
                            &cfg.image_blocks,
                        )
                    } else {
                        Err(anyhow::anyhow!("no region config for region {}", region))
                    };
                    match world_result {
                        Ok(world) => {
                            let palette = Self::region_palette(game_lib, region);
                            let renderer = MapRenderer::new(&world, &palette);
                            // npc-101: load NPC table for the starting region
                            self.npc_table = Some(crate::game::npc::NpcTable::load(&adf, region));
                            // sprite-101: load player (cfile 0-2) and setfig (cfile 13-17) sprites
                            let sprite_palette = palette;
                            for cfile_idx in [0u8, 1, 2, 13, 14, 15, 16, 17] {
                                if let Some(sheet) = crate::game::sprites::SpriteSheet::load(
                                    &adf, cfile_idx, &sprite_palette,
                                ) {
                                    self.dlog(format!(
                                        "sprite-load: cfile {} → {} frames",
                                        cfile_idx, sheet.num_frames
                                    ));
                                    self.sprite_sheets[cfile_idx as usize] = Some(sheet);
                                }
                            }
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
            self.dlog(format!("Day/night: {:?}", new_phase));
            let from = self.palette_transition
                .as_ref()
                .map(|pt| pt.to)
                .unwrap_or([crate::game::palette::BLACK; crate::game::palette::PALETTE_SIZE]);
            let to = match new_phase {
                DayNightPhase::Night => [crate::game::palette::BLACK; crate::game::palette::PALETTE_SIZE],
                DayNightPhase::Day   => [0xFFFFFFFF_u32; crate::game::palette::PALETTE_SIZE],
                _                   => from,
            };
            self.palette_transition = Some(crate::game::palette::PaletteTransition::new(from, to));
            self.day_night_phase = new_phase;
        }

        // Fatigue is updated per movement step in apply_player_input (player-111).

        // setmood: check music group every 8 ticks (gameloop-113)
        self.mood_tick += delta_ticks;
        if self.mood_tick >= 8 {
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

        // Region palette transition (world-109)
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
                if let (Some(ref mut mr), Some(ref world)) = (&mut self.map_renderer, &self.map_world) {
                    mr.atlas.rebuild(world, &palette);
                }
            }
        }

        // Day/night continuous dimming: rebuild atlas whenever lightlevel changes (gfx-101).
        // lightlevel is a *darkness* value: 0 = full day (bright), 300 = full night (dark).
        // pct is brightness percentage passed to apply_lightlevel_dim, so it must be inverted.
        let lightlevel = self.state.lightlevel;
        if lightlevel != self.last_lightlevel {
            self.last_lightlevel = lightlevel;
            let pct = (100 - lightlevel as i32 * 100 / 300) as i16;
            self.dlog(format!("daynight: lightlevel={} pct={}%", lightlevel, pct));
            if let (Some(ref mut mr), Some(ref world)) = (&mut self.map_renderer, &self.map_world) {
                let base = self.palette_transition
                    .as_ref()
                    .map(|pt| pt.to)
                    .unwrap_or([0xFFFFFFFF_u32; crate::game::palette::PALETTE_SIZE]);
                let faded = crate::game::palette_fader::apply_lightlevel_dim(&base, pct);
                mr.atlas.rebuild(world, &faded);
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
                    self.messages.push("You are ambushed!");
                }
            }
        }

        // Autosave every 3600 ticks (~60s at 60Hz)
        if self.autosave_enabled && self.state.tick_counter % 3600 == 0 && self.state.tick_counter > 0 {
            if let Err(e) = crate::game::persist::save_game(&self.state, 0) {
                eprintln!("autosave failed: {e}");
            } else if let Err(e) = crate::game::persist::save_transcript(
                self.messages.transcript(), 0,
            ) {
                eprintln!("autosave transcript failed: {e}");
            }
        }

        // Death / revive cycle (gameloop-106)
        if self.state.vitality <= 0 && !self.state.god_mode.contains(GodModeFlags::INVINCIBLE) {
            if self.state.try_respawn() {
                self.messages.push("Lucky! You barely survive...");
                self.dlog("try_respawn: luck-gated respawn succeeded");
            } else if let Some(next) = self.state.next_brother() {
                // Use config-driven brother activation if available
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
                // TODO: trigger brother-transition placard (gameloop-104 handles scene transition)
                self.dlog(format!("Brother died, switching to brother {}", next));
            } else {
                // All brothers dead — game over
                // TODO: return SceneResult::Done to trigger game over scene
                self.dlog("All brothers dead — GAME OVER");
            }
        }

        self.apply_player_input();

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
        self.update_actors(delta_ticks);

        // Camera: center hero in 288×160 viewport (gameloop-110)
        self.map_x = self.state.hero_x.saturating_sub(144);
        self.map_y = self.state.hero_y.saturating_sub(80);
        self.state.map_x = self.map_x;
        self.state.map_y = self.map_y;

        // Compose map viewport when in normal play view (world-105).
        // Pass img_x = map_x >> 4 and img_y = map_y >> 5 (tile-column and tile-row units),
        // matching the original: img_x = map_x >> 4, img_y = map_y >> 5 (fmain.c:2306-2307).
        if self.state.viewstatus == 0 {
            if let (Some(ref mut mr), Some(ref world)) = (&mut self.map_renderer, &self.map_world) {
                let img_x = self.map_x >> 4;
                let img_y = self.map_y >> 5;
                mr.compose(img_x, img_y, self.state.region_num, world);
            }
            // Blit actors on top of the composed tiles (sprite-104).
            // Collect borrow-safe parameters before taking &mut map_renderer.
            let map_x = self.map_x;
            let map_y = self.map_y;
            if let Some(ref mut mr) = self.map_renderer {
                Self::blit_actors_to_framebuf(
                    &self.sprite_sheets,
                    &self.state,
                    &self.npc_table,
                    map_x,
                    map_y,
                    &mut mr.framebuf,
                );
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
