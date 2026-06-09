//! Global singleton resources — non-entity game state shared by all systems.

use hecs::Entity;
use crate::game::debug_command::GodModeFlags;
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
    pub region_num:       u8,
    /// Pending region transition target (set by DoorSystem/ZoneSystem).
    pub new_region:       u8,
    /// True when any active enemy is within 300px of the hero.
    pub battleflag:       bool,
    pub encounter_type:   u16,
    pub encounter_number: u8,
    pub xtype:            u16,
    pub actor_file:       i16,
    pub set_file:         i16,
    pub princess:         u8,
    pub dayperiod:        u8,
    pub current_mood:     u8,
}

// ── Brother roster ───────────────────────────────────────────────────────────

/// Which brother is active, plus flags and cheats.
#[derive(Debug, Clone)]
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

impl Default for BrotherRoster {
    fn default() -> Self {
        Self {
            active_brother:       0,
            brother:              0,
            witchflag:            false,
            safe_flag:            false,
            cheat1:               false,
            god_mode:             GodModeFlags::empty(),
            inactive_inventories: [[0u8; 36]; 3],
        }
    }
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

/// Region terrain data and renderer. Replaced on each region transition.
/// The real WorldData and MapRenderer types are imported from their modules.
#[derive(Default)]
pub struct MapData {
    pub world:    Option<crate::game::world_data::WorldData>,
    pub renderer: Option<crate::game::map_renderer::MapRenderer>,
}

// ── Sprite sheets ─────────────────────────────────────────────────────────────

/// Loaded cfile sprite sheets. Replaced on each region transition.
#[derive(Default)]
pub struct SpriteSheets {
    pub sheets:         Vec<Option<crate::game::sprites::SpriteSheet>>,
    pub object_sprites: Option<crate::game::sprites::SpriteSheet>,
}

// ── Encounter context ─────────────────────────────────────────────────────────

/// Encounter, arena, and death-sequence state.
#[derive(Debug, Clone, Default)]
pub struct EncounterContext {
    pub in_encounter_zone:    bool,
    pub arena_mode:           bool,
    pub arena_zone:           (i32, i32, i32, i32),
    pub in_arena_zone:        bool,
    pub arena_encounter_idx:  u8,
    pub arena_damage_enabled: bool,
    pub sleeping:             bool,
    pub dying:                bool,
    pub fiery_death:          bool,
    pub death_type:           usize,
    pub goodfairy:            i16,
    pub luck_gate_fired:      bool,
    pub last_zone:            Option<usize>,
}

// ── VFX ───────────────────────────────────────────────────────────────────────

// TODO(Plan D): move WitchEffect and TeleportEffect here from gfx_effects module.
// For now we use placeholder types so resources.rs compiles independently.

/// Placeholder — real type will be moved from gfx_effects in Plan D.
#[derive(Debug, Clone, Default)]
pub struct WitchEffectPlaceholder;

/// Placeholder — real type will be moved from gfx_effects in Plan D.
#[derive(Debug, Clone, Default)]
pub struct TeleportEffectPlaceholder;

/// Active visual effects.
#[derive(Debug, Clone, Default)]
pub struct VfxState {
    pub witch_effect:    WitchEffectPlaceholder,
    pub teleport_effect: TeleportEffectPlaceholder,
}

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

    /// Current hero movement direction, derived from InputState each tick.
    pub input_direction: crate::game::direction::Direction,

    /// Stable handle for the hero entity (set at spawn, never changes mid-session).
    pub hero_entity: Entity,
    /// Handle for the active carrier entity, if any.
    pub carrier_entity: Option<Entity>,
    /// Last entity that triggered proximity auto-speech (dedup — same NPC only speaks once per approach).
    pub last_speech_entity: Option<Entity>,

    /// ADF disk image — kept alive so region transitions can reload world data.
    pub adf: Option<std::sync::Arc<crate::game::adf::AdfDisk>>,
    /// Zone rectangles for the current region (populated on region load).
    pub zones: Vec<crate::game::game_library::ZoneConfig>,
    /// Pending region transition — set by RegionSystem, consumed by EcsScene.
    pub pending_transition: Option<crate::game::ecs::events::RegionTransitionEvent>,
}

impl Resources {
    /// Construct with a placeholder hero_entity. Replace with the real handle
    /// immediately after spawning the hero in World.
    pub fn new(placeholder: Entity) -> Self {
        // daynight initialized to 8000 (morning) per fmain.c:2905 (revive()).
        let mut clock = GameClock::default();
        clock.daynight = 8000;
        Self {
            clock,
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
            input_direction:     crate::game::direction::Direction::None,
            hero_entity:         placeholder,
            carrier_entity:      None,
            last_speech_entity:  None,
            adf:                 None,
            zones:               Vec::new(),
            pending_transition:  None,
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
    fn resources_daynight_initializes_to_morning() {
        let mut world = hecs::World::new();
        let hero = world.spawn((crate::game::ecs::components::Hero,));
        let res = Resources::new(hero);
        // fmain.c:2905: daynight initialized to 8000 (morning) during revive().
        assert_eq!(res.clock.daynight, 8000, "game should start at morning (daynight=8000)");
    }

    #[test]
    fn health_check() {
        let h = crate::game::ecs::components::Health::new(10);
        assert!(!h.is_dead());
    }
}
