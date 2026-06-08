# ECS Migration Plan B: Components and Resources

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Define all ECS component types, the `Resources` struct, entity spawn helpers, and the `Events` queue — all in new files that compile cleanly alongside the existing `GameplayScene`. No game logic is changed yet.

**Architecture:** New files are introduced under `src/game/ecs/`. The existing `GameState`, `GameplayScene`, `Actor`, `Npc`, etc. are untouched. Components mirror the data in existing structs but are independent types. Spawn helpers produce `hecs::Entity` handles from existing data. All tests added in this plan are unit tests on the component types and spawn helpers themselves.

**Prerequisites:** Plan A must be complete (hecs in Cargo.toml, Direction as field type).

**Tech Stack:** Rust 2021, `hecs = "0.11"`, existing test suite (≥712 tests passing).

---

## File map

| File | Change |
|---|---|
| `src/game/ecs/mod.rs` | **Create** — module root, re-exports all public ECS types |
| `src/game/ecs/components.rs` | **Create** — all component structs |
| `src/game/ecs/resources.rs` | **Create** — `Resources` struct and all resource types |
| `src/game/ecs/events.rs` | **Create** — `Events` struct and all event enums |
| `src/game/ecs/spawn.rs` | **Create** — entity spawn helper functions |
| `src/game/mod.rs` | Add `pub mod ecs;` |

---

## Task 1: Create the `ecs` module skeleton

**Files:**
- Create: `src/game/ecs/mod.rs`
- Modify: `src/game/mod.rs`

- [ ] **Step 1: Create `src/game/ecs/mod.rs`**

```rust
//! ECS world types for the faery-tale-rs rearchitecture.
//! See docs/superpowers/plans/2025-01-01-ecs-plan-b-components.md

pub mod components;
pub mod events;
pub mod resources;
pub mod spawn;

pub use components::*;
pub use events::Events;
pub use resources::Resources;
```

- [ ] **Step 2: Add to `src/game/mod.rs`**

Add the line:
```rust
pub mod ecs;
```

- [ ] **Step 3: Create stub files so it compiles**

Create `src/game/ecs/components.rs`:
```rust
// Component types — filled in by Plan B Task 2.
```

Create `src/game/ecs/resources.rs`:
```rust
// Resource types — filled in by Plan B Task 3.
```

Create `src/game/ecs/events.rs`:
```rust
// Event types — filled in by Plan B Task 4.
```

Create `src/game/ecs/spawn.rs`:
```rust
// Spawn helpers — filled in by Plan B Task 5.
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo check 2>&1 | grep "^error"
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/game/ecs/ src/game/mod.rs
git commit -m "chore: scaffold src/game/ecs module"
```

---

## Task 2: Define all component types

**Files:**
- Modify: `src/game/ecs/components.rs`

- [ ] **Step 1: Write tests for component construction**

Replace the stub `src/game/ecs/components.rs` with the full file:

```rust
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
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
pub struct AiState {
    pub goal:       Goal,
    pub tactic:     Tactic,
    pub state:      NpcState,
    pub cleverness: u8,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            goal:       Goal::None,
            tactic:     Tactic::None,
            state:      NpcState::Still,
            cleverness: 0,
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
```

- [ ] **Step 2: Run tests**

```bash
cargo test ecs::components 2>&1 | grep -E "test.*ok|FAILED|^error"
```
Expected: all component tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/game/ecs/components.rs
git commit -m "feat(ecs): define all component types"
```

---

## Task 3: Define `Resources` and all resource types

**Files:**
- Modify: `src/game/ecs/resources.rs`

- [ ] **Step 1: Write `resources.rs`**

Replace the stub with:

```rust
//! Global singleton resources — non-entity game state shared by all systems.

use hecs::Entity;
use crate::game::direction::Direction;
use crate::game::actor::GodModeFlags;
use crate::game::world_data::WorldData;
use crate::game::map_renderer::MapRenderer;
use crate::game::npc::NpcState;
use crate::game::debug_log::DebugLogEntry;
use crate::game::ecs::events::Events;

// ── Clock ────────────────────────────────────────────────────────────────────

/// Day/night cycle, spell timers, tick counter.
#[derive(Debug, Clone, Default)]
pub struct GameClock {
    /// 0–24000 wrapping NTSC counter.
    pub daynight:      u16,
    /// Triangle-wave light level 0–300–0 derived from daynight.
    pub lightlevel:    u16,
    /// Completed full day cycles.
    pub game_days:     u32,
    /// Generic animation counter (incremented each gameplay tick).
    pub cycle:         u32,
    /// Sprite flash counter.
    pub flasher:       u32,
    /// Total gameplay ticks since session start.
    pub tick_counter:  u32,
    /// Green Jewel light spell remaining ticks.
    pub light_timer:   i16,
    /// Secret Totem spell remaining ticks.
    pub secret_timer:  i16,
    /// Gold Ring freeze spell remaining ticks.
    pub freeze_timer:  i16,
    /// Whether light_timer is pinned (cheat/debug).
    pub light_sticky:  bool,
    pub secret_sticky: bool,
    pub freeze_sticky: bool,
}

impl GameClock {
    pub fn is_frozen(&self) -> bool { self.freeze_timer > 0 }
}

// ── Region ───────────────────────────────────────────────────────────────────

/// Current region and in-progress encounter state.
#[derive(Debug, Clone, Default)]
pub struct RegionState {
    pub region_num:      u8,
    /// Pending region transition target (set by DoorSystem/ZoneSystem).
    pub new_region:      u8,
    /// True when any active enemy is within 300px of the hero.
    pub battleflag:      bool,
    pub encounter_type:  u16,
    pub encounter_number: u8,
    pub xtype:           u16,
    pub actor_file:      i16,
    pub set_file:        i16,
    pub princess:        u8,
    pub dayperiod:       u8,
    pub current_mood:    u8,
}

// ── Brother roster ───────────────────────────────────────────────────────────

/// Which brother is active, plus flags and cheats.
#[derive(Debug, Clone, Default)]
pub struct BrotherRoster {
    /// Index of the active brother (0=Julian, 1=Phillip, 2=Kevin).
    pub active_brother: usize,
    /// Raw brother code stored in save (1=Julian, 2=Phillip, 3=Kevin).
    pub brother:        u8,
    pub witchflag:      bool,
    pub safe_flag:      bool,
    pub cheat1:         bool,
    pub god_mode:       GodModeFlags,
    /// Inventories for the two inactive brothers.
    pub inactive_inventories: [[u8; 36]; 3],
}

// ── View / UI ─────────────────────────────────────────────────────────────────

/// View mode and pause state.
#[derive(Debug, Clone, Default)]
pub struct ViewState {
    /// 0=normal, 1=map, 2=message, 3=fade-in, 4=inventory, 98/99=force redraw.
    pub viewstatus: u8,
    pub cmode:      u8,
    pub paused:     bool,
}

// ── Camera ───────────────────────────────────────────────────────────────────

/// Camera scroll position in world pixels.
#[derive(Debug, Clone, Default)]
pub struct CameraState {
    pub map_x: f32,
    pub map_y: f32,
}

// ── Palette ──────────────────────────────────────────────────────────────────

pub type Palette = [u32; 32];

/// Current display palette and pending transition.
#[derive(Debug, Clone)]
pub struct PaletteState {
    pub current_palette:     Palette,
    pub base_colors_palette: Option<Palette>,
    pub dirty:               bool,
    pub transition:          Option<PaletteTransition>,
    pub textcolors:          Palette,
    pub compass_regions:     Vec<(i32, i32, i32, i32)>,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            current_palette:     [0u32; 32],
            base_colors_palette: None,
            dirty:               true,
            transition:          None,
            textcolors:          [0u32; 32],
            compass_regions:     Vec::new(),
        }
    }
}

/// Palette cross-fade in progress.
#[derive(Debug, Clone)]
pub struct PaletteTransition {
    pub from:   Palette,
    pub to:     Palette,
    pub ticks:  u8,
    pub total:  u8,
}

// ── Map data ──────────────────────────────────────────────────────────────────

/// Region terrain data and renderer.  Replaced on each region transition.
#[derive(Default)]
pub struct MapData {
    pub world:    Option<WorldData>,
    pub renderer: Option<MapRenderer>,
}

// ── Sprite sheets ─────────────────────────────────────────────────────────────

use crate::game::sprite::SpriteSheet;

/// Loaded cfile sprite sheets.  Replaced on each region transition.
#[derive(Default)]
pub struct SpriteSheets {
    pub sheets:         Vec<Option<SpriteSheet>>,
    pub object_sprites: Option<SpriteSheet>,
}

// ── Encounter context ─────────────────────────────────────────────────────────

/// Encounter, arena, and death-sequence state.
#[derive(Debug, Clone, Default)]
pub struct EncounterContext {
    pub in_encounter_zone: bool,
    pub arena_mode:        bool,
    pub arena_zone:        (i32, i32, i32, i32),
    pub in_arena_zone:     bool,
    pub arena_encounter_idx: u8,
    pub arena_damage_enabled: bool,
    pub sleeping:          bool,
    pub dying:             bool,
    pub fiery_death:       bool,
    pub death_type:        usize,
    pub goodfairy:         i16,
    pub luck_gate_fired:   bool,
    pub last_zone:         Option<usize>,
}

// ── VFX ───────────────────────────────────────────────────────────────────────

use crate::game::gameplay_scene::WitchEffect;
use crate::game::gameplay_scene::TeleportEffect;

/// Active visual effects.
#[derive(Debug, Clone, Default)]
pub struct VfxState {
    pub witch_effect:    WitchEffect,
    pub teleport_effect: TeleportEffect,
}

// ── Input ─────────────────────────────────────────────────────────────────────

// InputState is already defined in gameplay_scene/mod.rs.
// Plan D will move it here; for now it's re-used from GameplayScene.

// ── Top-level Resources container ─────────────────────────────────────────────

/// All global singleton state, passed to every system.
pub struct Resources {
    pub clock:     GameClock,
    pub region:    RegionState,
    pub brother:   BrotherRoster,
    pub view:      ViewState,
    pub camera:    CameraState,
    pub palette:   PaletteState,
    pub map:       MapData,
    pub sprites:   SpriteSheets,
    pub encounter: EncounterContext,
    pub vfx:       VfxState,
    pub events:    Events,
    pub log:       Vec<DebugLogEntry>,

    /// Stable handle for the hero entity (set at spawn, never changes mid-session).
    pub hero_entity: Entity,
    /// Handle for the active carrier entity, if any.
    pub carrier_entity: Option<Entity>,
}

impl Resources {
    /// Construct with a placeholder hero_entity. Replace with the real handle
    /// immediately after spawning the hero in World.
    pub fn new(placeholder: Entity) -> Self {
        Self {
            clock:          GameClock::default(),
            region:         RegionState::default(),
            brother:        BrotherRoster::default(),
            view:           ViewState::default(),
            camera:         CameraState::default(),
            palette:        PaletteState::default(),
            map:            MapData::default(),
            sprites:        SpriteSheets::default(),
            encounter:      EncounterContext::default(),
            vfx:            VfxState::default(),
            events:         Events::default(),
            log:            Vec::new(),
            hero_entity:    placeholder,
            carrier_entity: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_clock_frozen() {
        let mut c = GameClock::default();
        assert!(!c.is_frozen());
        c.freeze_timer = 1;
        assert!(c.is_frozen());
        c.freeze_timer = 0;
        assert!(!c.is_frozen());
    }

    #[test]
    fn health_check() {
        let h = crate::game::ecs::components::Health::new(10);
        assert!(!h.is_dead());
    }
}
```

- [ ] **Step 2: Fix any import errors**

```bash
cargo check 2>&1 | grep "^error" | head -20
```

The most likely issues are:
- `WitchEffect` / `TeleportEffect` — check their actual module path in `gameplay_scene/mod.rs` and adjust the import accordingly. If they are private, use a placeholder `pub struct WitchEffect; pub struct TeleportEffect;` in `resources.rs` until Plan D moves the real types here.
- `GodModeFlags` — check `src/game/actor.rs` for its definition. Import path is `crate::game::actor::GodModeFlags`.
- `SpriteSheet` — check `src/game/sprite.rs` or `src/game/gameplay_scene/`. Adjust import path to match.
- `DebugLogEntry` — check `src/game/debug_log.rs`. Adjust import path.

Fix each import error until `cargo check` passes.

- [ ] **Step 3: Run tests**

```bash
cargo test ecs::resources 2>&1 | grep -E "test.*ok|FAILED|^error"
```
Expected: both resource tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/game/ecs/resources.rs
git commit -m "feat(ecs): define Resources struct and all resource types"
```

---

## Task 4: Define the `Events` queue

**Files:**
- Modify: `src/game/ecs/events.rs`

- [ ] **Step 1: Write `events.rs`**

```rust
//! Cross-system event queues. Emitted by systems during a tick, consumed
//! by downstream systems or drained at tick start.

/// All event queues for one tick. Cleared at the start of each gameplay tick.
#[derive(Default)]
pub struct Events {
    pub clock:   Vec<ClockEvent>,
    pub damage:  Vec<DamageEvent>,
    pub died:    Vec<EnemyDiedEvent>,
    pub brother: Vec<BrotherDiedEvent>,
    pub sfx:     Vec<SfxEvent>,
    pub message: Vec<MessageEvent>,
    pub speech:  Vec<SpeechEvent>,
    pub zone:    Vec<ZoneEvent>,
    pub region:  Vec<RegionTransitionEvent>,
    pub item:    Vec<ItemEvent>,
}

impl Events {
    /// Drain all queues. Called at the start of each gameplay tick.
    pub fn clear(&mut self) {
        self.clock.clear();
        self.damage.clear();
        self.died.clear();
        self.brother.clear();
        self.sfx.clear();
        self.message.clear();
        self.speech.clear();
        self.zone.clear();
        self.region.clear();
        self.item.clear();
    }
}

// ── Event types ──────────────────────────────────────────────────────────────

/// Emitted by ClockSystem on time-period boundaries.
#[derive(Debug, Clone)]
pub enum ClockEvent {
    /// A new time period (day/night bucket) has begun.
    NewPeriod { period: u8 },
    /// Hunger threshold crossed.
    HungerWarning,
    /// Fatigue threshold crossed.
    FatigueWarning,
}

/// Emitted by CombatSystem or MissileSystem when an entity takes damage.
#[derive(Debug, Clone)]
pub struct DamageEvent {
    pub target:   hecs::Entity,
    pub amount:   i16,
    /// Weapon type code that dealt the damage (0 = unarmed).
    pub weapon:   u8,
    pub is_friendly_fire: bool,
}

/// Emitted by DeathSystem or CombatSystem when an enemy reaches vitality ≤ 0.
#[derive(Debug, Clone)]
pub struct EnemyDiedEvent {
    pub entity: hecs::Entity,
    /// NPC race (for loot table lookup).
    pub race:   u8,
    /// NPC weapon (for body-search logic).
    pub weapon: u8,
    /// Gold carried.
    pub gold:   i16,
    pub x:      f32,
    pub y:      f32,
}

/// Emitted by DeathSystem when the hero's vitality reaches ≤ 0.
#[derive(Debug, Clone)]
pub struct BrotherDiedEvent {
    pub brother_id: u8,
    pub x:          f32,
    pub y:          f32,
    /// Inventory at time of death (for Bones entity).
    pub stuff:      [u8; 36],
}

/// Audio cue request.
#[derive(Debug, Clone)]
pub struct SfxEvent {
    pub sfx_id: u8,
}

/// Scroll-area message to display.
#[derive(Debug, Clone)]
pub struct MessageEvent {
    pub text: String,
}

/// Proximity auto-speech triggered.
#[derive(Debug, Clone)]
pub struct SpeechEvent {
    pub speech_id: usize,
    pub brother_name: String,
}

/// Hero entered or exited a zone.
#[derive(Debug, Clone)]
pub enum ZoneEvent {
    Entered { zone_idx: usize },
    Exited  { zone_idx: usize },
}

/// Region transition requested.
#[derive(Debug, Clone)]
pub struct RegionTransitionEvent {
    pub new_region: u8,
    pub dest_x:     f32,
    pub dest_y:     f32,
}

/// Item interaction (TAKE action resolved).
#[derive(Debug, Clone)]
pub enum ItemEvent {
    /// Hero picks up a ground item.
    TakeItem { entity: hecs::Entity },
    /// Hero searches an enemy body.
    SearchBody { entity: hecs::Entity },
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::Events;

    #[test]
    fn events_clear() {
        let mut ev = Events::default();
        ev.message.push(super::MessageEvent { text: "hello".into() });
        ev.sfx.push(super::SfxEvent { sfx_id: 3 });
        assert_eq!(ev.message.len(), 1);
        ev.clear();
        assert!(ev.message.is_empty());
        assert!(ev.sfx.is_empty());
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test ecs::events 2>&1 | grep -E "test.*ok|FAILED|^error"
```
Expected: events_clear passes.

- [ ] **Step 3: Commit**

```bash
git add src/game/ecs/events.rs
git commit -m "feat(ecs): define Events queue and all event types"
```

---

## Task 5: Write entity spawn helpers

**Files:**
- Modify: `src/game/ecs/spawn.rs`

- [ ] **Step 1: Write `spawn.rs`**

```rust
//! Helper functions to spawn canonical entity bundles into a `hecs::World`.
//!
//! Each function returns the `hecs::Entity` handle of the newly spawned entity.
//! Spawn functions are pure: they do not read or write any `Resources`.

use hecs::World;
use crate::game::direction::Direction;
use crate::game::actor::{ActorState, Goal, Tactic};
use crate::game::npc::NpcState;
use crate::game::ecs::components::*;

/// Spawn the hero entity.  Called once at game start or on brother succession.
///
/// `brother_id`: 0=Julian, 1=Phillip, 2=Kevin.
/// Starting stats come from `GameLibrary.brothers[brother_id]` before calling this.
pub fn spawn_hero(
    world: &mut World,
    x: f32,
    y: f32,
    brother_id: u8,
    stats: HeroStats,
    inventory: Inventory,
) -> hecs::Entity {
    world.spawn((
        Hero,
        Position::new(x, y),
        Facing::new(Direction::S),
        BrotherKind { id: brother_id },
        stats,
        inventory,
        ActorMotion::default(),
        CombatState::default(),
        CarrierMount::default(),
        FrustFlag::default(),
    ))
}

/// Spawn an enemy NPC entity.
pub fn spawn_enemy(
    world: &mut World,
    x: f32,
    y: f32,
    npc_type: u8,
    race: u8,
    vitality: i16,
    weapon: u8,
    gold: i16,
    speed: u8,
    cleverness: u8,
    cfile_idx: u8,
) -> hecs::Entity {
    world.spawn((
        Enemy,
        Position::new(x, y),
        Facing::new(Direction::N),
        EnemyKind { npc_type, race },
        AiState {
            goal: Goal::Attack1,
            tactic: Tactic::Pursue,
            state: NpcState::Still,
            cleverness,
        },
        Health::new(vitality),
        Speed { speed },
        Loot { weapon, gold, looted: false },
        SpriteRef { cfile_idx },
    ))
}

/// Spawn an arena training dummy (immortal, no AI, no loot).
pub fn spawn_arena_dummy(
    world: &mut World,
    x: f32,
    y: f32,
    npc_type: u8,
    vitality: i16,
    cfile_idx: u8,
) -> hecs::Entity {
    world.spawn((
        Enemy,
        ArenaDummy,
        Position::new(x, y),
        Facing::new(Direction::SE),
        EnemyKind { npc_type, race: 0 },
        AiState::default(),
        Health::new(vitality),
        Speed { speed: 0 },
        Loot::default(),
        SpriteRef { cfile_idx },
    ))
}

/// Spawn a stationary world NPC (setfig).
pub fn spawn_setfig(
    world: &mut World,
    x: f32,
    y: f32,
    obj: WorldObj,
    cfile_idx: u8,
) -> hecs::Entity {
    world.spawn((
        SetFig,
        Position::new(x, y),
        Facing::new(Direction::S),
        obj,
        SpriteRef { cfile_idx },
    ))
}

/// Spawn a ground item.
pub fn spawn_ground_item(
    world: &mut World,
    x: f32,
    y: f32,
    obj: WorldObj,
) -> hecs::Entity {
    world.spawn((
        GroundItem,
        Position::new(x, y),
        obj,
    ))
}

/// Spawn a Bones entity when a brother dies.
pub fn spawn_bones(
    world: &mut World,
    x: f32,
    y: f32,
    region: u8,
    brother_id: u8,
    stuff: [u8; 36],
) -> hecs::Entity {
    world.spawn((
        Bones,
        Position::new(x, y),
        BrotherKind { id: brother_id },
        Inventory { stuff },
        WorldObj {
            ob_id:   28, // BONES ob_id
            ob_stat: 1,
            region,
            visible: true,
            goal:    0,
        },
    ))
}

/// Spawn a missile.
pub fn spawn_missile(
    world: &mut World,
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    time_of_flight: u8,
    missile_type: crate::game::combat::MissileType,
    is_friendly: bool,
) -> hecs::Entity {
    world.spawn((
        Missile,
        Position::new(x, y),
        MissileMotion { dx, dy, time_of_flight },
        MissileKind { missile_type, is_friendly },
    ))
}

/// Spawn a carrier entity (raft, turtle, swan, dragon).
pub fn spawn_carrier(
    world: &mut World,
    x: f32,
    y: f32,
    kind: i16,
    cfile_idx: u8,
) -> hecs::Entity {
    world.spawn((
        Carrier,
        Position::new(x, y),
        Facing::new(Direction::S),
        CarrierKind { kind },
        SpriteRef { cfile_idx },
    ))
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hecs::World;
    use crate::game::ecs::components::*;
    use crate::game::combat::MissileType;

    fn make_stats() -> HeroStats {
        HeroStats { vitality: 100, brave: 50, luck: 50, kind: 50,
                    wealth: 50, hunger: 0, fatigue: 0, gold: 0 }
    }

    #[test]
    fn spawn_hero_has_required_components() {
        let mut world = World::new();
        let e = spawn_hero(&mut world, 100.0, 200.0, 0, make_stats(), Inventory::empty());
        assert!(world.get::<&Hero>(e).is_ok());
        assert!(world.get::<&Position>(e).is_ok());
        assert!(world.get::<&HeroStats>(e).is_ok());
        assert!(world.get::<&Inventory>(e).is_ok());
        assert_eq!(world.get::<&Position>(e).unwrap().x, 100.0);
        assert_eq!(world.get::<&BrotherKind>(e).unwrap().id, 0);
    }

    #[test]
    fn spawn_enemy_has_required_components() {
        let mut world = World::new();
        let e = spawn_enemy(&mut world, 50.0, 60.0, 1, 0, 20, 1, 5, 3, 5, 0);
        assert!(world.get::<&Enemy>(e).is_ok());
        assert!(world.get::<&Health>(e).is_ok());
        assert!(world.get::<&AiState>(e).is_ok());
        assert_eq!(world.get::<&Health>(e).unwrap().vitality, 20);
    }

    #[test]
    fn spawn_arena_dummy_has_arena_dummy_marker() {
        let mut world = World::new();
        let e = spawn_arena_dummy(&mut world, 0.0, 0.0, 1, 50, 0);
        assert!(world.get::<&ArenaDummy>(e).is_ok());
        assert!(world.get::<&Enemy>(e).is_ok());
    }

    #[test]
    fn spawn_bones_has_inventory_and_brother() {
        let mut world = World::new();
        let mut stuff = [0u8; 36];
        stuff[0] = 3;
        let e = spawn_bones(&mut world, 10.0, 20.0, 1, 0, stuff);
        assert!(world.get::<&Bones>(e).is_ok());
        assert_eq!(world.get::<&Inventory>(e).unwrap().stuff[0], 3);
        assert_eq!(world.get::<&BrotherKind>(e).unwrap().id, 0);
    }

    #[test]
    fn spawn_missile_position() {
        let mut world = World::new();
        let e = spawn_missile(&mut world, 5.0, 6.0, 1.0, 0.0, 10,
                              MissileType::Arrow, true);
        assert!(world.get::<&Missile>(e).is_ok());
        let pos = world.get::<&Position>(e).unwrap();
        assert_eq!(pos.x, 5.0);
        assert_eq!(pos.y, 6.0);
    }

    #[test]
    fn despawn_removes_entity() {
        let mut world = World::new();
        let e = spawn_ground_item(&mut world, 0.0, 0.0,
            WorldObj { ob_id: 5, ob_stat: 1, region: 0, visible: true, goal: 0 });
        assert!(world.contains(e));
        world.despawn(e).unwrap();
        assert!(!world.contains(e));
    }
}
```

- [ ] **Step 2: Fix import errors**

```bash
cargo check 2>&1 | grep "^error" | head -20
```

The most likely issue is `MissileType` — check `src/game/combat.rs` for its definition. If it's not `pub`, make it pub or adjust the import.

- [ ] **Step 3: Run tests**

```bash
cargo test ecs::spawn 2>&1 | grep -E "test.*ok|FAILED|^error"
```
Expected: all 7 spawn tests pass.

- [ ] **Step 4: Run full suite**

```bash
cargo test 2>&1 | grep "^test result"
```
Expected: 4 `ok` lines. Count should be ≥ 712 + new tests.

- [ ] **Step 5: Commit**

```bash
git add src/game/ecs/spawn.rs
git commit -m "feat(ecs): entity spawn helpers with hecs::World"
```

---

## Completion check

All existing tests pass (≥ 712). New additions:
- `src/game/ecs/components.rs` — component structs + 6 unit tests
- `src/game/ecs/resources.rs` — resource structs + 2 unit tests
- `src/game/ecs/events.rs` — event enums + 1 unit test
- `src/game/ecs/spawn.rs` — spawn helpers + 7 unit tests

The ECS module compiles cleanly alongside the existing `GameplayScene`. Nothing in the existing game logic has been changed.

```bash
cargo test 2>&1 | grep "^test result"
```
