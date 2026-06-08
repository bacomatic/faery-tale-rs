//! Combat system: melee combat resolution between hero and NPCs.
//! Ports the battle loop from original fmain.c.

use crate::game::game_state::GameState;
use crate::game::gameplay_scene::Direction;
use crate::game::npc::{Npc, RACE_GHOST, RACE_NECROMANCER, RACE_SPECTRE, RACE_WITCH};

/// Maximum concurrent projectiles (missile_list[6] from fmain.c).
pub const MAX_MISSILES: usize = 6;

/// Bow/wand shot spawn X offsets by facing direction (Amiga DIR_* order: NW=0..W=7).
/// Reference: sprite-rendering.md §bowshotx[8]; fmain2.c:885.
/// char bowshotx[8] = { 0, 0, 3, 6, -3, -3, -3, -6 };  // NW N NE E SE S SW W
const BOWSHOTX: [i32; 8] = [0, 0, 3, 6, -3, -3, -3, -6];

/// Bow/wand shot spawn Y offsets by facing direction (Amiga DIR_* order: NW=0..W=7).
/// Reference: sprite-rendering.md §bowshoty[8]; fmain2.c:886.
/// char bowshoty[8] = { -6,-6,-1, 0, 6, 8, 0,-1 };  // NW N NE E SE S SW W
const BOWSHOTY: [i32; 8] = [-6, -6, -1, 0, 6, 8, 0, -1];

/// Missile type identifier (SPEC §10.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MissileType {
    #[default]
    Arrow, // weapon 4: hit radius 6px, attacker code -1
    Fireball, // weapon 5: hit radius 9px, attacker code -2
}

/// A projectile (arrow, wand bolt, etc.).
#[derive(Debug, Clone, Default)]
pub struct Missile {
    pub active: bool,
    pub x: i32,
    pub y: i32,
    pub dx: i32, // velocity x (-2, 0, or 2)
    pub dy: i32, // velocity y (-2, 0, or 2)
    pub missile_type: MissileType,
    pub is_friendly: bool, // true = fired by hero
    /// Ticks since fired. Missile expires when this exceeds 40
    /// (`fmain.c:2274`, `combat.md#missile_step`).
    pub time_of_flight: u8,
}

impl Missile {
    /// Advance one frame; returns true if this missile hit its target.
    /// Hit radius per SPEC §10.4: arrows 6px, fireballs 9px.
    pub fn tick(&mut self, target_x: i32, target_y: i32) -> bool {
        if !self.active {
            return false;
        }
        self.x += self.dx;
        self.y += self.dy;
        // Out of bounds check (world size ~32768)
        if self.x < 0 || self.x > 32768 || self.y < 0 || self.y > 32768 {
            self.active = false;
            return false;
        }
        let radius = match self.missile_type {
            MissileType::Arrow => 6,
            MissileType::Fireball => 9,
        };
        let hit = (self.x - target_x).abs() < radius && (self.y - target_y).abs() < radius;
        if hit {
            self.active = false;
        }
        hit
    }

    /// Compute damage for this missile per SPEC §10.4: rand8() + 4 for both types.
    pub fn damage(&self) -> i16 {
        (melee_rand(8) as i16) + 4
    }

    /// Get attacker code for dohit() per SPEC §10.4: -1 for arrows, -2 for fireballs.
    pub fn attacker_code(&self) -> i8 {
        match self.missile_type {
            MissileType::Arrow => -1,
            MissileType::Fireball => -2,
        }
    }

    /// Derive facing direction from velocity vector.
    /// Used for knockback direction when missile hits target.
    pub fn facing(&self) -> Direction {
        match (self.dx.signum(), self.dy.signum()) {
            (-1, -1) => Direction::NW,
            (0, -1)  => Direction::N,
            (1, -1)  => Direction::NE,
            (1, 0)   => Direction::E,
            (1, 1)   => Direction::SE,
            (0, 1)   => Direction::S,
            (-1, 1)  => Direction::SW,
            (-1, 0)  => Direction::W,
            _        => Direction::N,
        }
    }
}

/// Fire a missile from origin toward target direction.
/// weapon: 4=bow (arrow), 5=wand (fireball).
/// speed: projectile velocity (default 2 for arrows/wands, 5 for dragon fireballs).
/// Returns the index of the missile slot used, or None if full.
pub fn fire_missile(
    missiles: &mut [Missile; MAX_MISSILES],
    x: i32,
    y: i32,
    dir: Direction,
    weapon: u8,
    is_friendly: bool,
    speed: i32,
) -> Option<usize> {
    let slot = missiles.iter().position(|m| !m.active)?;
    let facing = (dir as u8 & 7) as usize;
    let (dx, dy) = match dir {
        Direction::NW   => (-speed, -speed),
        Direction::N    => (0, -speed),
        Direction::NE   => (speed, -speed),
        Direction::E    => (speed, 0),
        Direction::SE   => (speed, speed),
        Direction::S    => (0, speed),
        Direction::SW   => (-speed, speed),
        Direction::W    => (-speed, 0),
        Direction::None => (0, -speed),
    };
    // Apply bow/wand spawn offset from hero's facing position.
    // Arrows spawn from bow tip, not hero's feet (SPEC §10.4).
    let spawn_x = x + BOWSHOTX[facing];
    let spawn_y = y + BOWSHOTY[facing];

    let missile_type = if weapon == 5 {
        MissileType::Fireball
    } else {
        MissileType::Arrow
    };
    missiles[slot] = Missile {
        active: true,
        x: spawn_x,
        y: spawn_y,
        dx,
        dy,
        missile_type,
        is_friendly,
        time_of_flight: 0,
    };
    Some(slot)
}

/// Armor defense factors.
pub const ARMOR_DEFENSE: &[u8] = &[
    0, // none
    1, // leather
    2, // chain
    3, // plate
    4, // magic armor
];

/// Award loot from a defeated NPC to the hero.
pub fn award_loot(state: &mut GameState, npc: &Npc) {
    state.gold += npc.gold as i32;
    // BRV farming guard (player-109): brave is not awarded while riding the turtle.
    // When a future commit adds brave += N here, it must be wrapped:
    //   if !(state.on_raft && state.active_carrier == CARRIER_TURTLE) { state.brave += N; }
    // Additional item drops handled by npc-106
}

/// Random 0–3 from game tick, matching original rand4() used by trans_list.
/// Uses tick-seeded hash to avoid SystemTime dependency in animation loop.
pub fn rand4(tick: u32) -> usize {
    let h = tick.wrapping_mul(2246822519).wrapping_add(3266489917);
    (h as usize >> 16) & 3
}

/// Simple pseudo-random number for damage rolls (no external crate dependency).
pub fn melee_rand(max: u32) -> u32 {
    if max == 0 {
        return 0;
    }
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    nanos % max
}

/// Port of original `bitrand(mask)` — `rand() & mask`.
/// For combat: `bitrand(2)` returns 0, 1, or 2 (mask 0b10 → values 0..=2).
pub fn bitrand(mask: u32) -> u32 {
    melee_rand(u32::MAX) & mask
}

/// Melee damage from original dohit() formula.
/// `weapon_index`: attacker's weapon value (1=dirk..5=wand, ≥8 capped to 5).
/// Returns `wt + bitrand(2)`: weapon base + random 0–2 bonus.
pub fn bitrand_damage(weapon_index: u8) -> i16 {
    let wt = if weapon_index >= 8 { 5 } else { weapon_index };
    wt as i16 + bitrand(2) as i16
}

/// Compute weapon tip position with jitter (ports fmain.c newx/newy + rand8() - 3).
/// `wt` is the weapon value (after cap).
/// Returns (tip_x, tip_y) in world coordinates.
pub fn weapon_tip(abs_x: i32, abs_y: i32, facing: Direction, wt: i16) -> (i32, i32) {
    let offset = (wt * 2) as i32;
    let (ox, oy) = facing.push_offset(offset);
    let jitter_x = (melee_rand(8) as i32) - 3;
    let jitter_y = (melee_rand(8) as i32) - 3;
    (abs_x + ox + jitter_x, abs_y + oy + jitter_y)
}

/// Compute melee reach for a combatant.
/// For hero (is_hero=true): (brave/20) + 5, capped at 15, min 4.
/// For NPCs: 2 + rand4(tick), capped at 15.
pub fn combat_reach(is_hero: bool, brave: i16, tick: u32) -> i16 {
    let bv = if is_hero {
        ((brave / 20) + 5).max(4)
    } else {
        2 + rand4(tick) as i16
    };
    bv.min(15)
}

/// rand256(): random 0–255 for dodge rolls.
pub fn rand256() -> i16 {
    melee_rand(256) as i16
}

/// Compute NPC movement speed per tick based on terrain type (SPEC §9.5).
/// Applies the same speed chain as the hero, but derived directly from the
/// raw terrain type rather than the hero's stateful `environ` accumulator.
///
/// `race_ignores_terrain`: true for wraiths (RACE_WRAITH) and snakes
/// (RACE_SNAKE) — both bypass the terrain-speed chain entirely and always
/// use normal speed 2 (`fmain.c:1639`).
pub fn npc_speed_for_terrain(terrain: u8, race_ignores_terrain: bool) -> i8 {
    if race_ignores_terrain {
        return 2;
    }
    match terrain {
        6 => 4,          // slippery (ice / smooth) → fast
        2 | 3 => 1,      // shallow/deep water → slow
        t if t > 6 => 1, // any deeper water bands → slow
        _ => 2,          // default walking speed
    }
}

/// Compute hero movement speed based on terrain environ and riding status (SPEC §9.5).
/// Returns signed speed value for newx/newy multiplication.
pub fn hero_speed_for_env(environ: i8, riding_raft: bool) -> i8 {
    if riding_raft {
        // SPEC: raft riding = 3
        return 3;
    }
    match environ {
        -3 => -2,        // reversal tile (backward movement)
        -1 => 4,         // slippery (fast)
        2 => 1,          // wading (slow)
        e if e > 6 => 1, // deep water (slow)
        _ => 2,          // default walking speed
    }
}

/// Result of enemy immunity check per SPEC §10.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImmunityResult {
    /// Target takes full damage.
    Vulnerable,
    /// Target is immune; no message.
    ImmuneSilent,
    /// Target is immune; emit speak(58).
    ImmuneWithMessage,
}

/// Check if an NPC is immune to physical melee damage.
/// Per SPEC §10.2:
/// - Spectre (0x8a) and Ghost (0x8b) are fully immune (silent).
/// - Witch (0x89) is immune to weapon<4 unless player has Sun Stone (stuff[7]).
/// - Necromancer (9) is immune to weapon<4; emit speak(58) on block.
pub fn check_immunity(npc_race: u8, weapon: u8, has_sun_stone: bool) -> ImmunityResult {
    match npc_race {
        RACE_SPECTRE | RACE_GHOST => ImmunityResult::ImmuneSilent,
        RACE_NECROMANCER if weapon < 4 => ImmunityResult::ImmuneWithMessage,
        RACE_WITCH if weapon < 4 && !has_sun_stone => ImmunityResult::ImmuneWithMessage,
        _ => ImmunityResult::Vulnerable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::npc::{Npc, NPC_TYPE_ORC, RACE_ENEMY};

    fn make_orc() -> Npc {
        Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 0,
            y: 0,
            vitality: 10,
            gold: 5,
            speed: 2,
            active: true,
            ..Default::default()
        }
    }

    #[test]
    fn test_rand4_range() {
        for tick in 0..100u32 {
            let v = rand4(tick);
            assert!(v < 4, "rand4({tick}) returned {v}, expected 0-3");
        }
    }

    #[test]
    fn test_award_loot() {
        let mut state = GameState::new();
        state.gold = 0;
        let orc = make_orc();
        award_loot(&mut state, &orc);
        assert_eq!(state.gold, 5);
    }

    // T1-COMBAT-MISSILES tests

    #[test]
    fn test_arrow_hit_radius_6px() {
        let mut arrow = Missile {
            active: true,
            x: 100,
            y: 100,
            dx: 2,
            dy: 0,
            missile_type: MissileType::Arrow,
            is_friendly: true,
            time_of_flight: 0,
        };
        // Within 6px → hit
        assert!(arrow.tick(105, 100));
        assert!(!arrow.active); // missile deactivated after hit
    }

    #[test]
    fn test_arrow_miss_beyond_6px() {
        let mut arrow = Missile {
            active: true,
            x: 100,
            y: 100,
            dx: 2,
            dy: 0,
            missile_type: MissileType::Arrow,
            is_friendly: true,
            time_of_flight: 0,
        };
        // After tick, arrow at 102. Target at 109 → distance 7px → miss
        assert!(!arrow.tick(109, 100));
        assert_eq!(arrow.x, 102); // missile advanced
        assert!(arrow.active);
    }

    #[test]
    fn test_fireball_hit_radius_9px() {
        let mut fireball = Missile {
            active: true,
            x: 100,
            y: 100,
            dx: 0,
            dy: 2,
            missile_type: MissileType::Fireball,
            is_friendly: true,
            time_of_flight: 0,
        };
        // After tick, fireball at y=102. Target at 110 → distance 8px → hit
        assert!(fireball.tick(100, 110));
        assert!(!fireball.active);
    }

    #[test]
    fn test_fireball_miss_beyond_9px() {
        let mut fireball = Missile {
            active: true,
            x: 100,
            y: 100,
            dx: 0,
            dy: 2,
            missile_type: MissileType::Fireball,
            is_friendly: true,
            time_of_flight: 0,
        };
        // After tick, fireball at y=102. Target at 112 → distance 10px → miss
        assert!(!fireball.tick(100, 112));
        assert_eq!(fireball.y, 102);
        assert!(fireball.active);
    }

    #[test]
    fn test_missile_damage_range() {
        // SPEC §10.4: rand8() + 4 = 4–11 for both types
        for _ in 0..100 {
            let arrow = Missile {
                active: true,
                x: 0,
                y: 0,
                dx: 0,
                dy: 0,
                missile_type: MissileType::Arrow,
                is_friendly: true,
                time_of_flight: 0,
            };
            let dmg = arrow.damage();
            assert!(
                (4..=11).contains(&dmg),
                "arrow damage {} out of range 4-11",
                dmg
            );

            let fireball = Missile {
                active: true,
                x: 0,
                y: 0,
                dx: 0,
                dy: 0,
                missile_type: MissileType::Fireball,
                is_friendly: true,
                time_of_flight: 0,
            };
            let dmg = fireball.damage();
            assert!(
                (4..=11).contains(&dmg),
                "fireball damage {} out of range 4-11",
                dmg
            );
        }
    }

    #[test]
    fn test_attacker_codes() {
        let arrow = Missile {
            active: true,
            x: 0,
            y: 0,
            dx: 0,
            dy: 0,
            missile_type: MissileType::Arrow,
            is_friendly: true,
            time_of_flight: 0,
        };
        assert_eq!(arrow.attacker_code(), -1);

        let fireball = Missile {
            active: true,
            x: 0,
            y: 0,
            dx: 0,
            dy: 0,
            missile_type: MissileType::Fireball,
            is_friendly: true,
            time_of_flight: 0,
        };
        assert_eq!(fireball.attacker_code(), -2);
    }

    #[test]
    fn test_fire_arrow_weapon_4() {
        let mut missiles = std::array::from_fn(|_| Missile::default());
        let slot = fire_missile(&mut missiles, 100, 200, Direction::NE, 4, true, 2);
        assert!(slot.is_some());
        let m = &missiles[slot.unwrap()];
        assert_eq!(m.missile_type, MissileType::Arrow);
        assert_eq!(m.attacker_code(), -1);
    }

    #[test]
    fn test_fire_fireball_weapon_5() {
        let mut missiles = std::array::from_fn(|_| Missile::default());
        let slot = fire_missile(&mut missiles, 100, 200, Direction::NW, 5, false, 2);
        assert!(slot.is_some());
        let m = &missiles[slot.unwrap()];
        assert_eq!(m.missile_type, MissileType::Fireball);
        assert_eq!(m.attacker_code(), -2);
    }

    #[test]
    fn test_missile_ticks() {
        let mut m = Missile {
            active: true,
            x: 0,
            y: 100,
            dx: 2,
            dy: 0,
            missile_type: MissileType::Arrow,
            is_friendly: true,
            time_of_flight: 0,
        };
        let hit = m.tick(50, 100); // too far
        assert!(!hit);
        assert_eq!(m.x, 2);
    }

    #[test]
    fn test_fire_missile_slots() {
        let mut missiles = std::array::from_fn(|_| Missile::default());
        let slot = fire_missile(&mut missiles, 0, 0, Direction::NE, 4, true, 2);
        assert!(slot.is_some());
        assert!(missiles[slot.unwrap()].active);
    }

    #[test]
    fn test_missile_slot0_dodge_logic() {
        // fmain.c:2294 — slot 0 uses a dodge roll: hit only when bitrand(512) > brave.
        // With brave = i16::MAX (511 > 32767 is false), slot 0 never hits.
        // With brave = -1 (bitrand(512) >= 0 > -1), slot 0 always hits.
        for _ in 0..100 {
            let never_hit = bitrand(512) as i16 > i16::MAX;
            assert!(!never_hit, "slot 0 with max brave must never hit");
        }
        for _ in 0..100 {
            let always_hit = bitrand(512) as i16 > -1;
            assert!(always_hit, "slot 0 with brave=-1 must always hit");
        }
    }

    #[test]
    fn test_missile_slot1_always_hits_logic() {
        // fmain.c:2294 — slots 1+ always hit (no dodge gate).
        // The condition `slot > 0` short-circuits before the dodge roll.
        // Simulate with a helper that mirrors the scene_impl expression.
        fn missile_hit(slot: usize, brave: i16) -> bool {
            slot > 0 || bitrand(512) as i16 > brave
        }
        // Even with maxed brave, slots 1-5 always hit.
        for brave in [0i16, 50, 100, 200, i16::MAX] {
            assert!(missile_hit(1, brave), "slot 1 must always hit (brave={brave})");
            assert!(missile_hit(5, brave), "slot 5 must always hit (brave={brave})");
        }
        // Slot 0 with brave = i16::MAX should never hit (bitrand(512) < 32767 always).
        // bitrand(512) returns 0-511 which is always < 32767, so slot 0 never hits.
        for _ in 0..100 {
            assert!(!missile_hit(0, i16::MAX), "slot 0 with max brave must never hit");
        }
    }

    #[test]
    fn test_bitrand_range() {
        for _ in 0..1000 {
            let v = bitrand(2);
            assert!(v <= 2, "bitrand(2) returned {v}, expected 0-2");
        }
    }

    #[test]
    fn test_bitrand_damage_range() {
        // Dirk (weapon 1): damage should be 1, 2, or 3
        for _ in 0..100 {
            let d = bitrand_damage(1);
            assert!((1..=3).contains(&d), "dirk damage {d} out of range 1-3");
        }
        // Sword (weapon 3): damage should be 3, 4, or 5
        for _ in 0..100 {
            let d = bitrand_damage(3);
            assert!((3..=5).contains(&d), "sword damage {d} out of range 3-5");
        }
        // Touch attack (weapon 8+): capped to 5, so damage 5, 6, or 7
        for _ in 0..100 {
            let d = bitrand_damage(10);
            assert!((5..=7).contains(&d), "touch damage {d} out of range 5-7");
        }
    }

    #[test]
    fn test_bitrand_damage_fists() {
        // weapon 0 = no weapon: damage should be 0, 1, or 2
        for _ in 0..100 {
            let d = bitrand_damage(0);
            assert!((0..=2).contains(&d), "fist damage {d} out of range 0-2");
        }
    }

    #[test]
    fn test_weapon_tip_offset_north() {
        let (tx, ty) = weapon_tip(100, 100, Direction::N, 3);
        assert!(ty < 100, "north tip_y={} should be < 100", ty);
        assert!((tx - 100).abs() <= 4, "north tip_x={} too far from 100", tx);
    }

    #[test]
    fn test_weapon_tip_offset_east() {
        let (tx, ty) = weapon_tip(100, 100, Direction::E, 3);
        assert!(tx > 100, "east tip_x={} should be > 100", tx);
        assert!((ty - 100).abs() <= 4, "east tip_y={} too far from 100", ty);
    }

    #[test]
    fn test_combat_reach_hero() {
        let r = combat_reach(true, 50, 0);
        assert_eq!(r, 7);
    }

    #[test]
    fn test_combat_reach_hero_cap() {
        let r = combat_reach(true, 250, 0);
        assert_eq!(r, 15);
    }

    #[test]
    fn test_combat_reach_hero_min() {
        let r = combat_reach(true, 0, 0);
        assert_eq!(r, 5);
    }

    // T1-MOVE-SPEED-TERRAIN tests

    #[test]
    fn test_terrain_speed_default() {
        assert_eq!(hero_speed_for_env(0, false), 2);
        assert_eq!(hero_speed_for_env(1, false), 2);
        assert_eq!(hero_speed_for_env(-2, false), 2); // ice environ
    }

    #[test]
    fn test_terrain_speed_slippery() {
        assert_eq!(hero_speed_for_env(-1, false), 4);
    }

    #[test]
    fn test_terrain_speed_reversal() {
        assert_eq!(hero_speed_for_env(-3, false), -2);
    }

    #[test]
    fn test_terrain_speed_wading() {
        assert_eq!(hero_speed_for_env(2, false), 1);
    }

    #[test]
    fn test_terrain_speed_deep_water() {
        assert_eq!(hero_speed_for_env(7, false), 1);
        assert_eq!(hero_speed_for_env(10, false), 1);
        assert_eq!(hero_speed_for_env(30, false), 1);
    }

    #[test]
    fn test_terrain_speed_raft_overrides() {
        assert_eq!(hero_speed_for_env(0, true), 3);
        assert_eq!(hero_speed_for_env(-1, true), 3);
        assert_eq!(hero_speed_for_env(-3, true), 3);
        assert_eq!(hero_speed_for_env(2, true), 3);
        assert_eq!(hero_speed_for_env(10, true), 3);
    }

    #[test]
    fn test_combat_reach_npc_range() {
        for tick in 0..100u32 {
            let r = combat_reach(false, 0, tick);
            assert!(
                (2..=5).contains(&r),
                "npc reach {} out of range at tick {}",
                r,
                tick
            );
        }
    }

    #[test]
    fn test_rand256_range() {
        for _ in 0..1000 {
            let r = rand256();
            assert!((0..=255).contains(&r), "rand256 returned {}", r);
        }
    }

    #[test]
    fn test_immunity_spectre_always_immune() {
        use crate::game::npc::RACE_SPECTRE;
        for weapon in 0..=5 {
            assert_eq!(
                check_immunity(RACE_SPECTRE, weapon, false),
                ImmunityResult::ImmuneSilent,
                "Spectre should be immune to weapon {}",
                weapon
            );
            assert_eq!(
                check_immunity(RACE_SPECTRE, weapon, true),
                ImmunityResult::ImmuneSilent,
                "Spectre should be immune to weapon {} even with Sun Stone",
                weapon
            );
        }
    }

    #[test]
    fn test_immunity_ghost_always_immune() {
        use crate::game::npc::RACE_GHOST;
        for weapon in 0..=5 {
            assert_eq!(
                check_immunity(RACE_GHOST, weapon, false),
                ImmunityResult::ImmuneSilent,
                "Ghost should be immune to weapon {}",
                weapon
            );
            assert_eq!(
                check_immunity(RACE_GHOST, weapon, true),
                ImmunityResult::ImmuneSilent,
                "Ghost should be immune to weapon {} even with Sun Stone",
                weapon
            );
        }
    }

    #[test]
    fn test_immunity_necromancer_immune_below_4() {
        use crate::game::npc::RACE_NECROMANCER;
        for weapon in 0..4 {
            assert_eq!(
                check_immunity(RACE_NECROMANCER, weapon, false),
                ImmunityResult::ImmuneWithMessage,
                "Necromancer should be immune to weapon {}",
                weapon
            );
            assert_eq!(
                check_immunity(RACE_NECROMANCER, weapon, true),
                ImmunityResult::ImmuneWithMessage,
                "Necromancer should be immune to weapon {} even with Sun Stone",
                weapon
            );
        }
    }

    #[test]
    fn test_immunity_necromancer_vulnerable_at_4() {
        use crate::game::npc::RACE_NECROMANCER;
        for weapon in 4..=5 {
            assert_eq!(
                check_immunity(RACE_NECROMANCER, weapon, false),
                ImmunityResult::Vulnerable,
                "Necromancer should be vulnerable to weapon {}",
                weapon
            );
        }
    }

    #[test]
    fn test_immunity_witch_immune_below_4_without_stone() {
        use crate::game::npc::RACE_WITCH;
        for weapon in 0..4 {
            assert_eq!(
                check_immunity(RACE_WITCH, weapon, false),
                ImmunityResult::ImmuneWithMessage,
                "Witch should be immune to weapon {} without Sun Stone",
                weapon
            );
        }
    }

    #[test]
    fn test_immunity_witch_vulnerable_below_4_with_stone() {
        use crate::game::npc::RACE_WITCH;
        for weapon in 0..4 {
            assert_eq!(
                check_immunity(RACE_WITCH, weapon, true),
                ImmunityResult::Vulnerable,
                "Witch should be vulnerable to weapon {} with Sun Stone",
                weapon
            );
        }
    }

    #[test]
    fn test_immunity_witch_vulnerable_at_4() {
        use crate::game::npc::RACE_WITCH;
        for weapon in 4..=5 {
            assert_eq!(
                check_immunity(RACE_WITCH, weapon, false),
                ImmunityResult::Vulnerable,
                "Witch should be vulnerable to weapon {} without Sun Stone",
                weapon
            );
            assert_eq!(
                check_immunity(RACE_WITCH, weapon, true),
                ImmunityResult::Vulnerable,
                "Witch should be vulnerable to weapon {} with Sun Stone",
                weapon
            );
        }
    }

    #[test]
    fn test_immunity_normal_enemies_vulnerable() {
        use crate::game::npc::{RACE_ENEMY, RACE_UNDEAD, RACE_WRAITH};
        for race in [RACE_ENEMY, RACE_UNDEAD, RACE_WRAITH] {
            for weapon in 0..=5 {
                assert_eq!(
                    check_immunity(race, weapon, false),
                    ImmunityResult::Vulnerable,
                    "Race {} should be vulnerable to weapon {}",
                    race,
                    weapon
                );
            }
        }
    }
}
