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
use crate::game::day_phase::DayPhase;
use crate::game::direction::Direction;
use crate::game::ecs::components::{
    ActorMotion, AiState, CarrierMount, CombatState, EnemyKind, Facing, Health, HeroStats, Loot,
    Position,
};
use crate::game::ecs::resources::{NarrEvent, NarrativeQueue, Resources};
use crate::game::npc::{Npc, NpcState};

// Re-export the command / log types the spec places in bridge.rs. They
// actually live in sibling modules for history reasons; the re-export is
// the single import surface the spec describes.
#[allow(unused_imports)]
pub use crate::game::debug_command::{
    BrotherId, DebugCommand, GodModeFlags, MagicEffect, StatId, DEFAULT_TICK_RATE_HZ,
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

    // ── Narrative queue (debug preview) ───────────────────────────────
    pub narrative_pending_count: u32,
    pub narrative_active: bool,
    pub narrative_timer: u32,
    pub narrative_preview: Vec<String>,

    // ── Actor Watch (DBG-LAYOUT-06) ────────────────────────────────────
    /// Raft (slot 1) world coords when active+visible, otherwise `None`.
    pub raft_xy: Option<(u16, u16)>,
    /// Count of active projectiles (missile list), spec §Actor Watch.
    pub missile_count: u8,
    /// Count of visible ground-item actors (slots 7..=19).
    pub item_count: u8,
}

/// Hero-specific extras for the top-row debug panels.
#[derive(Debug, Clone, Default)]
pub struct HeroExtras {
    /// `15 + brave/4` — current cap for hero HP.
    pub max_vitality: i16,
    /// Weapon slot currently equipped on the hero.
    pub hero_weapon: u8,
    /// Human-readable name of the hero's weapon (Dirk/Mace/Sword/Bow/Wand/…).
    pub hero_weapon_name: String,
    /// Hero ActorState encoded (see `actor_state_u8`).
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
            facing: a.facing as u8,
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
            facing: n.facing as u8,
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

// ── ECS → DebugSnapshot conversion helpers ──────────────────────────────────

/// Build a vector of `ActorSnapshot` values from the ECS world.
///
/// Slot 0 is always the hero.  Slots 1.. are active enemies.  The total is
/// capped at `max_actors` (the DEBUG_SPEC limit is 20).
pub fn build_ecs_actor_snapshots(
    world: &hecs::World,
    hero_entity: hecs::Entity,
    max_actors: usize,
) -> Vec<ActorSnapshot> {
    let mut actors = Vec::with_capacity(max_actors.min(20));

    // Hero as slot 0.
    let hero_ok = (
        world.get::<&Position>(hero_entity).ok(),
        world.get::<&Facing>(hero_entity).ok(),
        world.get::<&HeroStats>(hero_entity).ok(),
        world.get::<&CombatState>(hero_entity).ok(),
        world.get::<&ActorMotion>(hero_entity).ok(),
    );
    if let (Some(pos), Some(facing), Some(stats), Some(combat), Some(motion)) = hero_ok {
        actors.push(ActorSnapshot {
            slot: 0,
            actor_type: 0, // PLAYER
            state: actor_state_u8(&combat.state),
            facing: facing.dir as u8,
            abs_x: pos.x as u16,
            abs_y: pos.y as u16,
            vitality: stats.vitality.clamp(i8::MIN as i16, i8::MAX as i16) as i8,
            weapon: combat.weapon,
            race: crate::game::npc::RACE_NORMAL,
            goal: goal_u8(&Goal::User),
            tactic: tactic_u8(&Tactic::None),
            environ: motion.environ,
            visible: true,
        });
    }

    // Enemies fill the remaining slots.
    let mut slot = 1;
    for (pos, facing, kind, ai, health, loot) in world
        .query::<(&Position, &Facing, &EnemyKind, &AiState, &Health, &Loot)>()
        .iter()
    {
        if actors.len() >= max_actors {
            break;
        }
        actors.push(ActorSnapshot {
            slot: slot as u8,
            actor_type: 1, // ENEMY
            state: npc_state_u8(&ai.state),
            facing: facing.dir as u8,
            abs_x: pos.x as u16,
            abs_y: pos.y as u16,
            vitality: health.vitality.clamp(i8::MIN as i16, i8::MAX as i16) as i8,
            weapon: loot.weapon,
            race: kind.race,
            goal: goal_u8(&ai.goal),
            tactic: tactic_u8(&ai.tactic),
            environ: 0,
            visible: true,
        });
        slot += 1;
    }

    actors
}

/// Extract hero-specific extras for the debug top-row panels.
pub fn build_ecs_hero_extras(
    world: &hecs::World,
    hero_entity: hecs::Entity,
    res: &Resources,
) -> HeroExtras {
    let mut extras = HeroExtras::default();

    let hero_ok = (
        world.get::<&HeroStats>(hero_entity).ok(),
        world.get::<&CombatState>(hero_entity).ok(),
        world.get::<&Facing>(hero_entity).ok(),
        world.get::<&ActorMotion>(hero_entity).ok(),
        world.get::<&CarrierMount>(hero_entity).ok(),
    );
    if let (Some(stats), Some(combat), Some(facing), Some(motion), Some(carrier)) = hero_ok {
        extras.max_vitality = 15 + (stats.brave / 4);
        extras.hero_weapon = combat.weapon;
        extras.hero_weapon_name = weapon_short_name(combat.weapon).to_string();
        extras.hero_state_u8 = actor_state_u8(&combat.state);
        extras.hero_state_name = actor_state_name(extras.hero_state_u8).to_string();
        extras.hero_facing = facing.dir as u8;
        extras.hero_environ = motion.environ;
        extras.active_carrier = carrier.active_carrier;
        extras.active_carrier_name = carrier_name(carrier.active_carrier).to_string();
    }

    extras.jewel_timer = res.clock.light_timer.max(0) as u16;
    extras.totem_timer = res.clock.secret_timer.max(0) as u16;
    extras.freeze_timer = res.clock.freeze_timer.max(0) as u16;

    extras
}

/// Build a short text preview of the next `count` pending narrative events.
pub fn build_ecs_narrative_preview(queue: &NarrativeQueue, count: usize) -> Vec<String> {
    queue
        .pending
        .iter()
        .take(count)
        .map(|event| match event {
            NarrEvent::Placard { text, .. } => format!("PLACARD: {}", text),
            NarrEvent::WaitTicks(t) => format!("WAIT: {} ticks", t),
            NarrEvent::TeleportHero { x, y, region } => {
                format!("TELEPORT: ({},{}) region {}", x, y, region)
            }
            NarrEvent::SwapObjectId { object_index, new_id } => {
                format!("SWAP: object {} -> {}", object_index, new_id)
            }
            NarrEvent::ApplyRewards => "REWARDS".to_string(),
        })
        .collect()
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
/// Uses Amiga DIR_* order: DIR_NW=0, DIR_N=1, DIR_NE=2, DIR_E=3,
/// DIR_SE=4, DIR_S=5, DIR_SW=6, DIR_W=7.
pub fn facing_name(facing: u8) -> &'static str {
    match Direction::from(facing) {
        Direction::NW   => "NW",
        Direction::N    => "N",
        Direction::NE   => "NE",
        Direction::E    => "E",
        Direction::SE   => "SE",
        Direction::S    => "S",
        Direction::SW   => "SW",
        Direction::W    => "W",
        Direction::None => "?",
    }
}

/// Human-readable environ label (SPEC §9.5).
pub fn environ_label(env: i8) -> &'static str {
    match env {
        -3 => "reverse",
        -2 => "ice",
        -1 => "slippery",
        0 => "normal",
        2 => "brush",
        5 => "shallow water",
        e if e > 6 => "deep water",
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

/// Short race label. Maps RACE_* constants to display names.
/// Uses RACE_* constants (not NPC_TYPE_* which have different values).
pub fn race_label(race: u8) -> String {
    use crate::game::npc::{
        RACE_BEGGAR, RACE_ENEMY, RACE_GHOST, RACE_NECROMANCER, RACE_NORMAL, RACE_SNAKE,
        RACE_SPECTRE, RACE_UNDEAD, RACE_WITCH, RACE_WOODCUTTER, RACE_WRAITH,
    };
    match race {
        x if x == RACE_NORMAL => "Normal".into(),
        x if x == RACE_UNDEAD => "Undead".into(),
        x if x == RACE_WRAITH => "Wraith".into(),
        x if x == RACE_ENEMY => "Enemy".into(),
        x if x == RACE_SNAKE => "Snake".into(),
        x if x == RACE_WOODCUTTER => "Woodctr".into(),
        x if x == RACE_GHOST => "Ghost".into(),
        x if x == RACE_NECROMANCER => "Necro".into(),
        x if x == RACE_WITCH => "Witch".into(),
        x if x == RACE_SPECTRE => "Spectre".into(),
        x if x == RACE_BEGGAR => "Beggar".into(),
        // Shopkeeper and other high-byte races
        0x88 => "Shop".into(),
        0x89 => "Witch".into(),
        0x8a => "Spectre".into(),
        0x8b => "Ghost".into(),
        0x8d => "Beggar".into(),
        _ => format!("r:0x{:02X}", race),
    }
}

/// Atomic stderr logging — prevents interleaved output when multiple threads write.
/// Use this instead of `eprintln!` for high-frequency debug logging.
#[allow(dead_code)]
pub fn debug_log_atomic(msg: impl AsRef<str>) {
    use std::io::{self, Write};
    let msg = msg.as_ref();
    let stderr = io::stderr();
    let mut lock = stderr.lock();
    let _ = writeln!(lock, "{}", msg);
    // Lock released when `lock` goes out of scope
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::actor::{ActorState, Goal, Tactic};
    use crate::game::direction::Direction;
    use crate::game::ecs::components::{
        ActorMotion, AiState, CarrierMount, CombatState, EnemyKind, Facing, Health, HeroStats,
        Loot, Position,
    };
    use crate::game::ecs::resources::{NarrativeQueue, NarrEvent, Resources};
    use crate::game::npc::{NpcState, RACE_ENEMY, RACE_NORMAL};

    use super::{
        actor_state_u8, build_ecs_actor_snapshots, build_ecs_hero_extras,
        build_ecs_narrative_preview, carrier_name, weapon_short_name,
    };

    fn hero_stats() -> HeroStats {
        HeroStats {
            vitality: 80,
            brave: 16,
            luck: 8,
            kind: 12,
            wealth: 5,
            hunger: 0,
            fatigue: 0,
            gold: 0,
        }
    }

    #[test]
    fn test_build_ecs_actor_snapshots_includes_hero() {
        let mut world = World::new();
        let hero = world.spawn((
            Position::new(100.0, 200.0),
            Facing::new(Direction::S),
            hero_stats(),
            CombatState { state: ActorState::Still, weapon: 1 },
            ActorMotion::default(),
            CarrierMount::default(),
        ));

        let actors = build_ecs_actor_snapshots(&world, hero, 10);
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0].slot, 0);
        assert_eq!(actors[0].actor_type, 0);
        assert_eq!(actors[0].state, 0);
        assert_eq!(actors[0].facing, 5);
        assert_eq!(actors[0].abs_x, 100);
        assert_eq!(actors[0].abs_y, 200);
        assert_eq!(actors[0].vitality, 80);
        assert_eq!(actors[0].weapon, 1);
        assert_eq!(actors[0].race, RACE_NORMAL);
        assert_eq!(actors[0].goal, 0);
        assert_eq!(actors[0].tactic, 255);
        assert_eq!(actors[0].environ, 0);
    }

    #[test]
    fn test_build_ecs_actor_snapshots_includes_enemies() {
        let mut world = World::new();
        let hero = world.spawn((
            Position::new(0.0, 0.0),
            Facing::new(Direction::N),
            hero_stats(),
            CombatState::default(),
            ActorMotion::default(),
            CarrierMount::default(),
        ));
        for i in 0..3 {
            world.spawn((
                Position::new((i * 50) as f32, 100.0),
                Facing::new(Direction::E),
                EnemyKind { npc_type: 1, race: RACE_ENEMY },
                AiState {
                    state: NpcState::Walking,
                    goal: Goal::Attack1,
                    tactic: Tactic::Pursue,
                    cleverness: 0,
                },
                Health::new(30),
                Loot { weapon: 2, gold: 0, looted: false },
            ));
        }

        let actors = build_ecs_actor_snapshots(&world, hero, 10);
        assert_eq!(actors.len(), 4);
        assert_eq!(actors[0].slot, 0);
        for i in 1..=3 {
            assert_eq!(actors[i].slot, i as u8);
            assert_eq!(actors[i].actor_type, 1);
            assert_eq!(actors[i].vitality, 30);
            assert_eq!(actors[i].weapon, 2);
            assert_eq!(actors[i].race, RACE_ENEMY);
            assert_eq!(actors[i].goal, 1); // Attack1
            assert_eq!(actors[i].tactic, 0); // Pursue
            assert_eq!(actors[i].abs_y, 100);
        }
        assert_eq!(actors[1].abs_x, 0);
        assert_eq!(actors[2].abs_x, 50);
        assert_eq!(actors[3].abs_x, 100);
    }

    #[test]
    fn test_build_ecs_actor_snapshots_respects_max_limit() {
        let mut world = World::new();
        let hero = world.spawn((
            Position::new(0.0, 0.0),
            Facing::new(Direction::N),
            hero_stats(),
            CombatState::default(),
            ActorMotion::default(),
            CarrierMount::default(),
        ));
        for i in 0..15 {
            world.spawn((
                Position::new((i * 10) as f32, 100.0),
                Facing::new(Direction::E),
                EnemyKind { npc_type: 1, race: RACE_ENEMY },
                AiState::default(),
                Health::new(10),
                Loot::default(),
            ));
        }

        let actors = build_ecs_actor_snapshots(&world, hero, 5);
        assert_eq!(actors.len(), 5);
    }

    #[test]
    fn test_build_ecs_hero_extras() {
        let mut world = World::new();
        let hero = world.spawn((
            hero_stats(),
            CombatState { state: ActorState::Fighting(10), weapon: 2 },
            Facing::new(Direction::W),
            ActorMotion { environ: 3, ..Default::default() },
            CarrierMount { active_carrier: 1, ..Default::default() },
        ));
        let mut res = Resources::new(hero);
        res.clock.light_timer = 100;
        res.clock.secret_timer = 200;
        res.clock.freeze_timer = 300;

        let extras = build_ecs_hero_extras(&world, hero, &res);
        assert_eq!(extras.max_vitality, 15 + (16 / 4));
        assert_eq!(extras.hero_weapon, 2);
        assert_eq!(extras.hero_weapon_name, weapon_short_name(2));
        assert_eq!(extras.hero_state_u8, actor_state_u8(&ActorState::Fighting(10)));
        assert_eq!(extras.hero_state_name, "FIGHT");
        assert_eq!(extras.hero_facing, 7);
        assert_eq!(extras.hero_environ, 3);
        assert_eq!(extras.active_carrier, 1);
        assert_eq!(extras.active_carrier_name, carrier_name(1));
        assert_eq!(extras.jewel_timer, 100);
        assert_eq!(extras.totem_timer, 200);
        assert_eq!(extras.freeze_timer, 300);
    }

    #[test]
    fn test_build_ecs_narrative_preview() {
        let mut queue = NarrativeQueue::new();
        queue.push(NarrEvent::Placard {
            text: "Hello".to_string(),
            hold_ticks: 10,
        });
        queue.push(NarrEvent::WaitTicks(5));
        queue.push(NarrEvent::TeleportHero { x: 7.0, y: 8.0, region: 9 });
        queue.push(NarrEvent::ApplyRewards);

        let preview = build_ecs_narrative_preview(&queue, 3);
        assert_eq!(preview.len(), 3);
        assert_eq!(preview[0], "PLACARD: Hello");
        assert_eq!(preview[1], "WAIT: 5 ticks");
        assert_eq!(preview[2], "TELEPORT: (7,8) region 9");
    }
}
