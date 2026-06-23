//! ECS component types.
//!
//! Components are plain data structs — no methods beyond `new()` constructors.
//! All direction-typed fields use `Direction`; all position fields use `f32`.

use crate::game::direction::Direction;
use crate::game::actor::{ActorState, Goal, Tactic};
use crate::game::npc::NpcState;

// ── Marker components ────────────────────────────────────────────────────────

/// Marks the single hero entity.
#[derive(Debug, Clone, Copy)]
pub struct Hero;

/// Marks an enemy NPC entity.
#[derive(Debug, Clone, Copy)]
pub struct Enemy;

/// Marks a stationary world NPC (wizard, beggar, princess, etc.).
#[derive(Debug, Clone, Copy)]
pub struct SetFig;

/// Marks a ground item entity.
#[derive(Debug, Clone, Copy)]
pub struct GroundItem;

/// Marks a dead brother's inventory cache in the world.
#[derive(Debug, Clone, Copy)]
pub struct Bones;

/// Marks a missile (arrow, fireball, etc.).
#[derive(Debug, Clone, Copy)]
pub struct Missile;

/// Marks a carrier entity (raft, turtle, swan, dragon).
#[derive(Debug, Clone, Copy)]
pub struct Carrier;

/// Marks the good fairy entity spawned during the hero rescue sequence.
#[derive(Debug, Clone, Copy)]
pub struct GoodFairy;

// ── Shared components (used by multiple entity types) ────────────────────────

/// World-space position in pixels. All entity types share this component.
/// The Amiga coordinate space is preserved: X increases east, Y increases south.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self { Self { x, y } }

    pub fn set(&mut self, x: f32, y: f32) { self.x = x; self.y = y; }

    pub fn distance_to(&self, other: &Position) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Manhattan distance (used by original game for proximity checks).
    pub fn manhattan_to(&self, other: &Position) -> f32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

/// Facing direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facing {
    pub dir: Direction,
}

impl Facing {
    pub fn new(dir: Direction) -> Self { Self { dir } }
}

impl Default for Facing {
    fn default() -> Self { Self { dir: Direction::N } }
}

// ── Hero-only components ─────────────────────────────────────────────────────

/// Which of the three brothers is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrotherKind {
    /// 0 = Julian, 1 = Phillip, 2 = Kevin.
    pub id: u8,
}

impl BrotherKind {
    pub fn julian()  -> Self { Self { id: 0 } }
    pub fn phillip() -> Self { Self { id: 1 } }
    pub fn kevin()   -> Self { Self { id: 2 } }
}

/// Hero character statistics.
#[derive(Debug, Clone, Copy)]
pub struct HeroStats {
    pub vitality: i16,
    pub brave:    i16,
    pub luck:     i16,
    pub kind:     i16,
    pub wealth:   i16,
    pub hunger:   i16,
    pub fatigue:  i16,
    pub gold:     i32,
}

impl HeroStats {
    pub fn is_dead(&self) -> bool { self.vitality <= 0 }
}

/// Active brother's inventory: item counts in 36 slots.
/// Slot 35 is the transient quiver (arrows in flight); only slots 0–34 are saved.
#[derive(Debug, Clone)]
pub struct Inventory {
    pub stuff: [u8; 36],
}

impl Inventory {
    pub fn empty() -> Self { Self { stuff: [0; 36] } }

    /// Starting inventory for a new brother: one Dirk in slot 0 (fmain.c:3501 `stuff[0]=1`).
    pub fn with_dirk() -> Self {
        let mut s = [0u8; 36];
        s[0] = 1;
        Self { stuff: s }
    }
}

impl Default for Inventory {
    fn default() -> Self { Self::empty() }
}

/// Hero physics: continuous velocity and terrain modifier.
#[derive(Debug, Clone, Copy, Default)]
pub struct ActorMotion {
    pub vel_x:   f32,
    pub vel_y:   f32,
    pub environ: i8,
    pub moving:  bool,
}

/// Hero combat/animation state and equipped weapon.
#[derive(Debug, Clone)]
pub struct CombatState {
    pub state:  ActorState,
    pub weapon: u8,
}

impl Default for CombatState {
    fn default() -> Self { Self { state: ActorState::Still, weapon: 0 } }
}

/// Active carrier mount state.
#[derive(Debug, Clone, Copy, Default)]
pub struct CarrierMount {
    pub riding:         i16,
    pub flying:         i16,
    pub swan_vx:        f32,
    pub swan_vy:        f32,
    pub active_carrier: i16,
    pub on_raft:        bool,
    pub raftprox:       i16,
    pub wcarry:         u8,
}

/// Blocked-movement frustration counter (reset on successful move).
#[derive(Debug, Clone, Copy, Default)]
pub struct FrustFlag {
    pub count: u8,
}

/// Safe respawn point set when hero enters a safe zone.
#[derive(Debug, Clone, Copy)]
pub struct SafePoint {
    pub x:      f32,
    pub y:      f32,
    pub region: u8,
}

// ── Enemy components ─────────────────────────────────────────────────────────

/// Enemy type and race codes (determines AI, loot table, immunity).
#[derive(Debug, Clone, Copy)]
pub struct EnemyKind {
    pub npc_type: u8,
    pub race:     u8,
}

/// Enemy AI state: goal, tactic, animation state, cleverness.
#[derive(Debug, Clone)]
pub struct AiState {
    pub goal:            Goal,
    pub tactic:          Tactic,
    pub state:           NpcState,
    pub cleverness:      u8,
    /// Current fight-animation substate (0-8), advanced via TRANS_LIST each tick.
    pub fight_substate:  u8,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            goal:           Goal::None,
            tactic:         Tactic::None,
            state:          NpcState::Still,
            cleverness:     0,
            fight_substate: 0,
        }
    }
}

/// Enemy hit points.
#[derive(Debug, Clone, Copy)]
pub struct Health {
    pub vitality: i16,
}

impl Health {
    pub fn new(vitality: i16) -> Self { Self { vitality } }
    pub fn is_dead(&self) -> bool { self.vitality <= 0 }
}

/// Enemy movement speed.
#[derive(Debug, Clone, Copy)]
pub struct Speed {
    pub speed: u8,
}

/// Enemy weapon and loot state.
#[derive(Debug, Clone, Copy, Default)]
pub struct Loot {
    pub weapon: u8,
    pub gold:   i16,
    pub looted: bool,
}

/// Sprite sheet index for rendering.
#[derive(Debug, Clone, Copy)]
pub struct SpriteRef {
    pub cfile_idx: u8,
}

/// Marks an arena training dummy (immortal, no AI, no loot).
#[derive(Debug, Clone, Copy)]
pub struct ArenaDummy;

// ── WorldObject components (GroundItem and SetFig) ────────────────────────────

/// Ground item / setfig world state.
#[derive(Debug, Clone, Copy)]
pub struct WorldObj {
    pub ob_id:   u8,
    /// 0=taken, 1=ground item, 3=setfig NPC, 5=hidden
    pub ob_stat: u8,
    pub region:  u8,
    pub visible: bool,
    /// ob_list index for setfig dialogue variant.
    pub goal:    u8,
}

// ── Bones components ─────────────────────────────────────────────────────────
// BrotherKind + Inventory + WorldObj on a Bones entity capture everything needed.
// No additional component required.

// ── Missile components ───────────────────────────────────────────────────────

/// Missile velocity and remaining flight time.
#[derive(Debug, Clone, Copy)]
pub struct MissileMotion {
    pub dx:             f32,
    pub dy:             f32,
    pub time_of_flight: u8,
}

/// Missile type and allegiance.
#[derive(Debug, Clone, Copy)]
pub struct MissileKind {
    pub missile_type: crate::game::combat::MissileType,
    pub is_friendly:  bool,
}

// ── Carrier components ───────────────────────────────────────────────────────

/// Carrier vehicle kind (1=raft, 2=turtle, 3=swan, 4=dragon).
#[derive(Debug, Clone, Copy)]
pub struct CarrierKind {
    pub kind: i16,
}

// ── Optional: talk flicker timer ─────────────────────────────────────────────

/// Added to a SetFig entity when spoken to; removed when timer reaches zero.
#[derive(Debug, Clone, Copy)]
pub struct TalkFlicker {
    pub timer: u8,
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::direction::Direction;

    #[test]
    fn position_distance() {
        let a = Position::new(0.0, 0.0);
        let b = Position::new(3.0, 4.0);
        assert!((a.distance_to(&b) - 5.0).abs() < 0.001);
    }

    #[test]
    fn position_manhattan() {
        let a = Position::new(0.0, 0.0);
        let b = Position::new(3.0, 4.0);
        assert_eq!(a.manhattan_to(&b), 7.0);
    }

    #[test]
    fn facing_default_is_north() {
        assert_eq!(Facing::default().dir, Direction::N);
    }

    #[test]
    fn brother_kind_ids() {
        assert_eq!(BrotherKind::julian().id, 0);
        assert_eq!(BrotherKind::phillip().id, 1);
        assert_eq!(BrotherKind::kevin().id, 2);
    }

    #[test]
    fn health_dead() {
        assert!(Health::new(0).is_dead());
        assert!(Health::new(-1).is_dead());
        assert!(!Health::new(1).is_dead());
    }

    #[test]
    fn hero_stats_dead() {
        let mut s = HeroStats { vitality: 1, brave: 0, luck: 0, kind: 0,
                                wealth: 0, hunger: 0, fatigue: 0, gold: 0 };
        assert!(!s.is_dead());
        s.vitality = 0;
        assert!(s.is_dead());
    }

    #[test]
    fn inventory_empty() {
        let inv = Inventory::empty();
        assert!(inv.stuff.iter().all(|&v| v == 0));
    }
}
