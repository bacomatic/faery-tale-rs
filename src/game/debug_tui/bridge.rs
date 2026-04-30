//! Debug TUI bridge: the data interface between the game loop and the TUI.
//!
//! Per `DEBUG_SPECIFICATION.md` §Architecture, this module defines the
//! snapshot/command types that flow between the game and the debug console.
//! Label helpers used by both panels and command-dump output live here too.
//!
//! This module is **always compiled**, even when the `debug-tui` feature is
//! disabled: `main.rs` constructs `DebugSnapshot` values as part of its
//! normal game-loop plumbing, and the stub console still consumes them.
//! The ratatui-dependent rendering lives in `view.rs` / `commands.rs` which
//! are feature-gated.

use crate::game::actor::{Actor, ActorKind, ActorState, Goal, Tactic};
use crate::game::game_state::DayPhase;
use crate::game::npc::{Npc, NpcState};

// Re-export the command / log types the spec places in bridge.rs. They
// actually live in sibling modules for history reasons; the re-export is
// the single import surface the spec describes.
#[allow(unused_imports)]
pub use crate::game::debug_command::{
    BrotherId, DebugCommand, GodModeFlags, MagicEffect, StatId,
};
#[allow(unused_imports)]
pub use crate::game::debug_log::{DebugLogEntry, LogCategory};

// ── Status snapshot ──────────────────────────────────────────────────────────

/// Lightweight game-state snapshot for the status header.
/// Built by main.rs each frame when the console is active.
#[derive(Debug, Clone, Default)]
pub struct DebugSnapshot {
    pub fps: f64,
    pub tps: f64,
    pub game_day: u32,
    pub game_hour: u32,
    pub game_minute: u32,
    pub day_phase: DayPhase,
    pub daynight: u16,
    pub lightlevel: u16,
    pub game_ticks: u64,
    pub paused: bool,
    pub is_paused: bool,
    pub scene_name: Option<String>,
    pub hero_x: u16,
    pub hero_y: u16,
    pub brother: u8,
    pub region_num: u8,
    pub vitality: i16,
    pub hunger: i16,
    pub fatigue: i16,
    pub god_mode_flags: u8,
    pub time_held: bool,
    pub song_group_count: usize,
    pub current_song_group: Option<usize>,
    pub cave_mode: bool,

    // Geography
    pub current_zone_idx: Option<usize>,
    pub current_zone_label: Option<String>,

    // VFX state
    pub vfx_jewel_active: bool,
    pub vfx_light_sticky: bool,
    pub vfx_secret_active: bool,
    pub vfx_witch_active: bool,
    pub vfx_teleport_active: bool,
    pub vfx_palette_xfade: bool,

    // Time-of-day period derived from day_phase.
    pub time_period: String,

    // Quest state (for `/quest` command — DEBUG_SPEC §DebugSnapshot Data Model).
    pub princess_captive: bool,
    pub princess_rescues: u16,
    pub statues_collected: u8,
    pub has_writ: bool,
    pub has_talisman: bool,

    // Encounter state (for `/enc` commands).
    pub encounter_number: u8,
    pub encounter_type: u8,
    pub active_enemy_count: u8,

    // Full inventory array for `/inventory` command.
    pub stuff: Vec<u8>,
    // cheat1 flag (DEBUG_SPEC §Mutation /cheat).
    pub cheat1: bool,
    // Wealth (gold) — used by /give/take and /inventory dumps.
    pub wealth: u16,
    // Brave — used to compute heal cap `15 + brave/4` for /heal.
    pub brave: u16,

    // Actor slots (for `/actors` command and `/watch` feature). Up to 20 active slots.
    pub actors: Vec<ActorSnapshot>,

    // ── Hero top-row extras (DBG-LAYOUT-01) ────────────────────────────
    /// `15 + brave/4` — current cap for hero HP.
    pub max_vitality: i16,
    pub luck: i16,
    pub kind: i16,
    /// Weapon slot currently equipped on the hero (`actors[0].weapon`).
    pub hero_weapon: u8,
    /// Human-readable name of the hero's weapon (Dirk/Mace/Sword/Bow/Wand/…).
    pub hero_weapon_name: String,
    /// Hero ActorState encoded (see actor_state_u8).
    pub hero_state_u8: u8,
    /// Human-readable hero state (WALKING, FIGHT, …).
    pub hero_state_name: String,
    /// Hero facing direction 0..=7 (0=N, clockwise).
    pub hero_facing: u8,
    /// Hero environ value (−3..=2); see SPEC §9.5.
    pub hero_environ: i8,
    /// Carrier index currently ridden (0 none / 1 raft / 2 turtle / 3 swan / 4 dragon).
    pub active_carrier: i16,
    /// Active carrier human-readable label.
    pub active_carrier_name: String,
    /// Light-timer tick count (Green Jewel spell).
    pub jewel_timer: u16,
    /// Totem (Bird Totem / map) active indicator tick count.
    pub totem_timer: u16,
    /// Freeze (Gold Ring) timer tick count.
    pub freeze_timer: u16,

    // ── Actor Watch (DBG-LAYOUT-06) ────────────────────────────────────
    /// Raft (slot 1) world coords when active+visible, otherwise `None`.
    pub raft_xy: Option<(u16, u16)>,
    /// Count of active projectiles (missile list), spec §Actor Watch.
    pub missile_count: u8,
    /// Count of visible ground-item actors (slots 7..=19).
    pub item_count: u8,
}

/// Per-actor snapshot for the Actor Watch panel and `/actors` dump.
/// See DEBUG_SPECIFICATION.md §DebugSnapshot Data Model for field semantics.
#[derive(Debug, Clone, Default)]
pub struct ActorSnapshot {
    pub slot: u8,
    pub actor_type: u8,
    pub state: u8,
    pub facing: u8,
    pub abs_x: u16,
    pub abs_y: u16,
    pub vitality: i8,
    pub weapon: u8,
    pub race: u8,
    pub goal: u8,
    pub tactic: u8,
    pub environ: i8,
    pub visible: bool,
}

impl ActorSnapshot {
    pub fn from_actor(slot: u8, a: &Actor) -> Self {
        Self {
            slot,
            actor_type: actor_kind_u8(&a.kind),
            state: actor_state_u8(&a.state),
            facing: a.facing,
            abs_x: a.abs_x,
            abs_y: a.abs_y,
            vitality: a.vitality.clamp(i8::MIN as i16, i8::MAX as i16) as i8,
            weapon: a.weapon,
            race: a.race,
            goal: goal_u8(&a.goal),
            tactic: tactic_u8(&a.tactic),
            environ: a.environ,
            visible: true,
        }
    }

    pub fn from_npc(idx: usize, n: &Npc) -> Self {
        Self {
            slot: idx as u8,
            actor_type: 7, // NPC (npc_table entry, distinct from combat Actor kinds 0-6)
            state: npc_state_u8(&n.state),
            facing: n.facing,
            abs_x: n.x as u16,
            abs_y: n.y as u16,
            vitality: n.vitality.clamp(i8::MIN as i16, i8::MAX as i16) as i8,
            weapon: n.weapon,
            race: n.race,
            goal: goal_u8(&n.goal),
            tactic: tactic_u8(&n.tactic),
            environ: 0,
            visible: n.active,
        }
    }
}

fn actor_kind_u8(k: &ActorKind) -> u8 {
    match k {
        ActorKind::Player => 0,
        ActorKind::Enemy => 1,
        ActorKind::Object => 2,
        ActorKind::Raft => 3,
        ActorKind::SetFig => 4,
        ActorKind::Carrier => 5,
        ActorKind::Dragon => 6,
    }
}

pub fn actor_state_u8(s: &ActorState) -> u8 {
    match s {
        ActorState::Still => 0,
        ActorState::Walking => 1,
        ActorState::Fighting(_) => 2,
        ActorState::Dying => 3,
        ActorState::Dead => 4,
        ActorState::Shooting(_) => 5,
        ActorState::Sinking => 6,
        ActorState::Falling => 7,
        ActorState::Sleeping => 8,
    }
}

fn npc_state_u8(s: &NpcState) -> u8 {
    match s {
        NpcState::Still => 0,
        NpcState::Walking => 1,
        NpcState::Fighting => 2,
        NpcState::Dying => 3,
        NpcState::Dead => 4,
        NpcState::Shooting => 5,
        NpcState::Sinking => 6,
    }
}

fn goal_u8(g: &Goal) -> u8 {
    match g {
        Goal::User => 0,
        Goal::Attack1 => 1,
        Goal::Attack2 => 2,
        Goal::Archer1 => 3,
        Goal::Archer2 => 4,
        Goal::Flee => 5,
        Goal::Follower => 6,
        Goal::Leader => 7,
        Goal::Stand => 8,
        Goal::Guard => 9,
        Goal::Confused => 10,
        Goal::None => 255,
    }
}

fn tactic_u8(t: &Tactic) -> u8 {
    match t {
        Tactic::Pursue => 0,
        Tactic::Shoot => 1,
        Tactic::Random => 2,
        Tactic::BumbleSeek => 3,
        Tactic::Backup => 4,
        Tactic::Follow => 5,
        Tactic::Evade => 6,
        Tactic::EggSeek => 7,
        Tactic::Frust => 8,
        Tactic::None => 255,
    }
}

/// Human-readable label for a `DayPhase` variant — used to populate
/// `DebugSnapshot::time_period` (spec §DebugSnapshot Data Model).
pub fn day_phase_label(phase: DayPhase) -> String {
    match phase {
        DayPhase::Midnight => "Night".to_string(),
        DayPhase::Morning => "Morning".to_string(),
        DayPhase::Midday => "Midday".to_string(),
        DayPhase::Evening => "Evening".to_string(),
    }
}

/// Hero weapon slot → short display name (DBG-LAYOUT-01).
pub fn weapon_short_name(weapon: u8) -> &'static str {
    match weapon {
        0 => "—",
        1 => "Dirk",
        2 => "Mace",
        3 => "Sword",
        4 => "Bow",
        5 => "Wand",
        _ => "?",
    }
}

/// ActorState discriminant → compact display name (DBG-LAYOUT-01).
pub fn actor_state_name(state: u8) -> &'static str {
    match state {
        0 => "STILL",
        1 => "WALK",
        2 => "FIGHT",
        3 => "DYING",
        4 => "DEAD",
        5 => "SHOOT",
        6 => "SINK",
        7 => "FALL",
        8 => "SLEEP",
        _ => "?",
    }
}

/// Facing direction 0..=7 → 8-point compass label.
pub fn facing_name(facing: u8) -> &'static str {
    match facing & 7 {
        0 => "N",
        1 => "NE",
        2 => "E",
        3 => "SE",
        4 => "S",
        5 => "SW",
        6 => "W",
        7 => "NW",
        _ => "?",
    }
}

/// Human-readable environ label (SPEC §9.5).
pub fn environ_label(env: i8) -> &'static str {
    match env {
        -3 => "reverse",
        -2 => "swamp",
        -1 => "slippery",
        0 => "normal",
        1 => "wading",
        2 => "fire",
        _ => "?",
    }
}

/// Carrier slot → human-readable label.
pub fn carrier_name(carrier: i16) -> &'static str {
    match carrier {
        0 => "none",
        1 => "raft",
        2 => "turtle",
        3 => "swan",
        4 => "dragon",
        _ => "?",
    }
}

/// Short label for `ActorKind` codes (matches `actor_kind_u8`).
pub fn actor_kind_name(kind: u8) -> &'static str {
    match kind {
        0 => "PLAYER",
        1 => "ENEMY",
        2 => "OBJECT",
        3 => "RAFT",
        4 => "SETFIG",
        5 => "CARRIER",
        6 => "DRAGON",
        7 => "NPC",
        _ => "?",
    }
}

/// Short label for `Goal` codes (matches `goal_u8`).
pub fn goal_name(g: u8) -> &'static str {
    match g {
        0 => "USER",
        1 => "ATK1",
        2 => "ATK2",
        3 => "ARC1",
        4 => "ARC2",
        5 => "FLEE",
        6 => "FOLL",
        7 => "LEAD",
        8 => "STAND",
        9 => "GUARD",
        10 => "CONF",
        255 => "—",
        _ => "?",
    }
}

/// Short label for `Tactic` codes (matches `tactic_u8`).
pub fn tactic_name(t: u8) -> &'static str {
    match t {
        0 => "PURSUE",
        1 => "SHOOT",
        2 => "RANDOM",
        3 => "BUMBLE",
        4 => "BACKUP",
        5 => "FOLLOW",
        6 => "EVADE",
        7 => "EGG",
        8 => "FRUST",
        255 => "—",
        _ => "?",
    }
}

/// Short race/NPC-type label. Tries NPC type byte first, then known race constants; otherwise
/// falls back to a hex representation so the panel remains informative.
pub fn race_label(race: u8) -> String {
    use crate::game::npc::{
        NPC_TYPE_DKNIGHT, NPC_TYPE_DRAGON, NPC_TYPE_GHOST, NPC_TYPE_HORSE, NPC_TYPE_HUMAN,
        NPC_TYPE_LORAII, NPC_TYPE_NECROMANCER, NPC_TYPE_ORC, NPC_TYPE_RAFT, NPC_TYPE_SKELETON,
        NPC_TYPE_SNAKE, NPC_TYPE_SPIDER, NPC_TYPE_SWAN, NPC_TYPE_WRAITH, RACE_BEGGAR, RACE_GHOST,
        RACE_NECROMANCER, RACE_SHOPKEEPER, RACE_SPECTRE, RACE_WITCH, RACE_WOODCUTTER,
    };
    match race {
        0 => "Normal".into(),
        x if x == NPC_TYPE_HUMAN => "Human".into(),
        x if x == NPC_TYPE_SWAN => "Swan".into(),
        x if x == NPC_TYPE_HORSE => "Horse".into(),
        x if x == NPC_TYPE_DRAGON => "Dragon".into(),
        x if x == NPC_TYPE_GHOST => "Ghost".into(),
        x if x == NPC_TYPE_ORC => "Orc".into(),
        x if x == NPC_TYPE_WRAITH => "Wraith".into(),
        x if x == NPC_TYPE_SKELETON => "Skel".into(),
        x if x == NPC_TYPE_RAFT => "Raft".into(),
        x if x == NPC_TYPE_SNAKE => "Snake".into(),
        x if x == NPC_TYPE_SPIDER => "Spider".into(),
        x if x == NPC_TYPE_DKNIGHT => "DKnight".into(),
        x if x == NPC_TYPE_LORAII => "Loraii".into(),
        x if x == NPC_TYPE_NECROMANCER => "Necro".into(),
        x if x == RACE_WOODCUTTER => "Woodctr".into(),
        x if x == RACE_SHOPKEEPER => "Shop".into(),
        x if x == RACE_BEGGAR => "Beggar".into(),
        x if x == RACE_WITCH => "Witch".into(),
        x if x == RACE_SPECTRE => "Spectre".into(),
        x if x == RACE_GHOST => "Ghost".into(),
        x if x == RACE_NECROMANCER => "Necro".into(),
        _ => format!("r:0x{:02X}", race),
    }
}
