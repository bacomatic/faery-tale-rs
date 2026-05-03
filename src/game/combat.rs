//! Combat system: melee combat resolution between hero and NPCs.
//! Ports the battle loop from original fmain.c.

use crate::game::game_state::GameState;
use crate::game::npc::{
    Npc, RACE_GHOST, RACE_NECROMANCER, RACE_SPECTRE, RACE_UNDEAD, RACE_WITCH, RACE_WRAITH,
};

/// Maximum concurrent projectiles (missile_list[6] from fmain.c).
pub const MAX_MISSILES: usize = 6;

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
}

/// Fire a missile from origin toward target direction.
/// dir: 0=N, 2=E, 4=S, 6=W (and diagonals).
/// weapon: 4=bow (arrow), 5=wand (fireball).
/// speed: projectile velocity (default 2 for arrows/wands, 5 for dragon fireballs).
/// Returns the index of the missile slot used, or None if full.
pub fn fire_missile(
    missiles: &mut [Missile; MAX_MISSILES],
    x: i32,
    y: i32,
    dir: u8,
    weapon: u8,
    is_friendly: bool,
    speed: i32,
) -> Option<usize> {
    let slot = missiles.iter().position(|m| !m.active)?;
    let (dx, dy) = match dir & 7 {
        0 => (0, -speed),      // N
        1 => (speed, -speed),  // NE
        2 => (speed, 0),       // E
        3 => (speed, speed),   // SE
        4 => (0, speed),       // S
        5 => (-speed, speed),  // SW
        6 => (-speed, 0),      // W
        7 => (-speed, -speed), // NW
        _ => (0, -speed),
    };
    let missile_type = if weapon == 5 {
        MissileType::Fireball
    } else {
        MissileType::Arrow
    };
    missiles[slot] = Missile {
        active: true,
        x,
        y,
        dx,
        dy,
        missile_type,
        is_friendly,
        time_of_flight: 0,
    };
    Some(slot)
}

/// Weapon type damage factors (from original weapon table).
/// Index = weapon slot in stuff[], value = damage multiplier.
/// Fists (slot 0) = 5: original fmain.c caps weapon index >= 8 to 5.
#[deprecated(note = "Use bitrand_damage() instead")]
pub const WEAPON_DAMAGE: &[u8] = &[
    5,  // fists (slot 0, unarmed — original caps touch attack to 5)
    3,  // dagger
    4,  // short sword
    5,  // long sword
    6,  // axe
    7,  // mace
    8,  // halberd
    10, // magic sword
];

/// Armor defense factors.
pub const ARMOR_DEFENSE: &[u8] = &[
    0, // none
    1, // leather
    2, // chain
    3, // plate
    4, // magic armor
];

/// Result of one round of combat.
#[derive(Debug, Clone)]
pub struct CombatResult {
    pub hero_damage: i16,
    pub enemy_damage: i16,
    pub enemy_defeated: bool,
    pub hero_defeated: bool,
}

/// Resolve one round of melee combat between hero and NPC.
/// Weapon slot is index into stuff[]; 0 = fists.
///
/// **Deprecated:** Use `GameplayScene::run_combat_tick()` instead.
#[deprecated(note = "Use GameplayScene::run_combat_tick() instead")]
#[allow(deprecated)]
pub fn resolve_combat(
    state: &mut GameState,
    npc: &mut Npc,
    hero_weapon_slot: usize,
) -> CombatResult {
    // Hero attacks enemy
    let weapon_factor = WEAPON_DAMAGE.get(hero_weapon_slot).copied().unwrap_or(1) as i16;
    let hero_attack = (state.vitality * weapon_factor / 8).max(1);
    npc.vitality -= hero_attack;

    // Enemy attacks hero
    let enemy_attack = (npc.vitality.max(0) * npc.speed as i16 / 8).max(0);
    // Undead can't be harmed by normal weapons (resist)
    let resisted = npc.race == RACE_UNDEAD || npc.race == RACE_WRAITH;
    let actual_hero_damage = if resisted { 0 } else { enemy_attack };
    state.vitality = (state.vitality - actual_hero_damage).max(0);

    let enemy_defeated = npc.vitality <= 0;
    let hero_defeated = state.vitality <= 0;

    if enemy_defeated {
        // F9.11: legacy combat path now uses the searchable-body lifecycle.
        // `mark_dead` keeps `active=true` and flips `state=Dead` so the
        // body persists for `search_body` (`fmain.c:3251-3283`).
        npc.mark_dead();
    }

    CombatResult {
        hero_damage: actual_hero_damage,
        enemy_damage: hero_attack,
        enemy_defeated,
        hero_defeated,
    }
}

/// Award loot from a defeated NPC to the hero.
pub fn award_loot(state: &mut GameState, npc: &Npc) {
    state.gold += npc.gold as i32;
    // BRV farming guard (player-109): brave is not awarded while riding the turtle.
    // When a future commit adds brave += N here, it must be wrapped:
    //   if !(state.on_raft && state.active_carrier == CARRIER_TURTLE) { state.brave += N; }
    // Additional item drops handled by npc-106
}

/// Compute melee hit reach in pixels.
/// Mirrors fmain.c: bv = (brave/20) + 5 for player, capped at 15.
/// INSANE_REACH god-mode multiplies by 4.
pub fn melee_reach(brave: i16, weapon: u8, insane_reach: bool) -> i16 {
    let base = ((brave / 20) + 5).min(15).max(4) as i16;
    // Weapon adds 2px per level, mirroring wt+wt offset in newx/newy.
    let reach = base + (weapon as i16) * 2;
    if insane_reach {
        reach * 4
    } else {
        reach
    }
}

/// Direction-sensitive melee proximity check (ports fmain.c sword proximity loop).
/// Returns true if the target is within reach of the hero's weapon tip.
/// Uses Chebyshev distance (max(|dx|,|dy|) < reach) matching original `yd < bv`.
pub fn in_melee_range(
    hero_x: i16,
    hero_y: i16,
    facing: u8,
    weapon: u8,
    brave: i16,
    target_x: i16,
    target_y: i16,
    insane_reach: bool,
) -> bool {
    let reach = melee_reach(brave, weapon, insane_reach);
    // Project weapon tip ahead in facing direction (mirrors newx/newy offset by wt+wt).
    let offset = reach;
    let (ox, oy): (i16, i16) = match facing & 7 {
        0 => (0, -offset),       // N
        1 => (offset, -offset),  // NE
        2 => (offset, 0),        // E
        3 => (offset, offset),   // SE
        4 => (0, offset),        // S
        5 => (-offset, offset),  // SW
        6 => (-offset, 0),       // W
        7 => (-offset, -offset), // NW
        _ => (0, -offset),
    };
    let tip_x = hero_x as i32 + ox as i32;
    let tip_y = hero_y as i32 + oy as i32;
    let xd = (target_x as i32 - tip_x).abs();
    let yd = (target_y as i32 - tip_y).abs();
    xd.max(yd) < reach as i32
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
pub fn weapon_tip(abs_x: i32, abs_y: i32, facing: u8, wt: i16) -> (i32, i32) {
    let offset = (wt * 2) as i32;
    let (ox, oy): (i32, i32) = match facing & 7 {
        0 => (0, -offset),
        1 => (offset, -offset),
        2 => (offset, 0),
        3 => (offset, offset),
        4 => (0, offset),
        5 => (-offset, offset),
        6 => (-offset, 0),
        7 => (-offset, -offset),
        _ => (0, -offset),
    };
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
    #[allow(deprecated)]
    fn test_combat_reduces_enemy_vitality() {
        let mut state = GameState::new();
        state.vitality = 50;
        let mut orc = make_orc();
        let result = resolve_combat(&mut state, &mut orc, 3); // long sword
        assert!(result.enemy_damage > 0);
        assert!(orc.vitality < 10);
    }

    #[test]
    #[allow(deprecated)]
    fn test_combat_enemy_defeated() {
        let mut state = GameState::new();
        state.vitality = 200; // very strong hero
        let mut orc = make_orc();
        orc.vitality = 1; // near death
        let result = resolve_combat(&mut state, &mut orc, 5); // mace
        assert!(result.enemy_defeated);
        // F9.11: defeated NPC is now flagged Dead (looted=false) but
        // remains active so TAKE → search_body can consume the body
        // (`fmain.c:3251-3283`).
        assert!(orc.active, "body must stay active for TAKE/search_body");
        assert_eq!(orc.state, crate::game::npc::NpcState::Dead);
        assert_eq!(orc.vitality, 0);
        assert!(!orc.looted);
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
        let slot = fire_missile(&mut missiles, 100, 200, 2, 4, true, 2);
        assert!(slot.is_some());
        let m = &missiles[slot.unwrap()];
        assert_eq!(m.missile_type, MissileType::Arrow);
        assert_eq!(m.attacker_code(), -1);
    }

    #[test]
    fn test_fire_fireball_weapon_5() {
        let mut missiles = std::array::from_fn(|_| Missile::default());
        let slot = fire_missile(&mut missiles, 100, 200, 0, 5, false, 2);
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
        let slot = fire_missile(&mut missiles, 0, 0, 2, 4, true, 2);
        assert!(slot.is_some());
        assert!(missiles[slot.unwrap()].active);
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
        let (tx, ty) = weapon_tip(100, 100, 0, 3);
        assert!(ty < 100, "north tip_y={} should be < 100", ty);
        assert!((tx - 100).abs() <= 4, "north tip_x={} too far from 100", tx);
    }

    #[test]
    fn test_weapon_tip_offset_east() {
        let (tx, ty) = weapon_tip(100, 100, 2, 3);
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
