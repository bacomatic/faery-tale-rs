//! Combat system: melee combat resolution between hero and NPCs.
//! Ports the battle loop from original fmain.c.

use crate::game::npc::{Npc, RACE_UNDEAD, RACE_WRAITH};
use crate::game::game_state::GameState;

/// Maximum concurrent projectiles (missile_list[6] from fmain.c).
pub const MAX_MISSILES: usize = 6;

/// A projectile (arrow, wand bolt, etc.).
#[derive(Debug, Clone, Default)]
pub struct Missile {
    pub active: bool,
    pub x: i32,
    pub y: i32,
    pub dx: i32, // velocity x (-2, 0, or 2)
    pub dy: i32, // velocity y (-2, 0, or 2)
    pub damage: i16,
    pub is_friendly: bool, // true = fired by hero
}

impl Missile {
    /// Advance one frame; returns true if this missile hit its target.
    /// target_x, target_y = position to test for hit (16px tolerance).
    pub fn tick(&mut self, target_x: i32, target_y: i32) -> bool {
        if !self.active { return false; }
        self.x += self.dx;
        self.y += self.dy;
        // Out of bounds check (world size ~32768)
        if self.x < 0 || self.x > 32768 || self.y < 0 || self.y > 32768 {
            self.active = false;
            return false;
        }
        let hit = (self.x - target_x).abs() < 16 && (self.y - target_y).abs() < 16;
        if hit { self.active = false; }
        hit
    }
}

/// Fire a missile from origin toward target direction.
/// dir: 0=N, 2=E, 4=S, 6=W (and diagonals).
/// Returns the index of the missile slot used, or None if full.
pub fn fire_missile(
    missiles: &mut [Missile; MAX_MISSILES],
    x: i32, y: i32,
    dir: u8,
    damage: i16,
    is_friendly: bool,
) -> Option<usize> {
    let slot = missiles.iter().position(|m| !m.active)?;
    let (dx, dy) = match dir & 7 {
        0 => (0, -2),  // N
        1 => (2, -2),  // NE
        2 => (2, 0),   // E
        3 => (2, 2),   // SE
        4 => (0, 2),   // S
        5 => (-2, 2),  // SW
        6 => (-2, 0),  // W
        7 => (-2, -2), // NW
        _ => (0, -2),
    };
    missiles[slot] = Missile { active: true, x, y, dx, dy, damage, is_friendly };
    Some(slot)
}

/// Weapon type damage factors (from original weapon table).
/// Index = weapon slot in stuff[], value = damage multiplier.
/// Fists (slot 0) = 5: original fmain.c caps weapon index >= 8 to 5.
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
pub fn resolve_combat(state: &mut GameState, npc: &mut Npc, hero_weapon_slot: usize) -> CombatResult {
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
        npc.active = false;
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
    if insane_reach { reach * 4 } else { reach }
}

/// Direction-sensitive melee proximity check (ports fmain.c sword proximity loop).
/// Returns true if the target is within reach of the hero's weapon tip.
/// Uses Chebyshev distance (max(|dx|,|dy|) < reach) matching original `yd < bv`.
pub fn in_melee_range(
    hero_x: i16, hero_y: i16,
    facing: u8, weapon: u8, brave: i16,
    target_x: i16, target_y: i16,
    insane_reach: bool,
) -> bool {
    let reach = melee_reach(brave, weapon, insane_reach);
    // Project weapon tip ahead in facing direction (mirrors newx/newy offset by wt+wt).
    let offset = reach;
    let (ox, oy): (i16, i16) = match facing & 7 {
        0 => (0, -offset),        // N
        1 => (offset, -offset),   // NE
        2 => (offset, 0),         // E
        3 => (offset, offset),    // SE
        4 => (0, offset),         // S
        5 => (-offset, offset),   // SW
        6 => (-offset, 0),        // W
        7 => (-offset, -offset),  // NW
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
    if max == 0 { return 0; }
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    nanos % max
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::npc::{Npc, NPC_TYPE_ORC, RACE_ENEMY};

    fn make_orc() -> Npc {
        Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 0, y: 0,
            vitality: 10,
            gold: 5,
            speed: 2,
            active: true,
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
    fn test_combat_reduces_enemy_vitality() {
        let mut state = GameState::new();
        state.vitality = 50;
        let mut orc = make_orc();
        let result = resolve_combat(&mut state, &mut orc, 3); // long sword
        assert!(result.enemy_damage > 0);
        assert!(orc.vitality < 10);
    }

    #[test]
    fn test_combat_enemy_defeated() {
        let mut state = GameState::new();
        state.vitality = 200; // very strong hero
        let mut orc = make_orc();
        orc.vitality = 1; // near death
        let result = resolve_combat(&mut state, &mut orc, 5); // mace
        assert!(result.enemy_defeated);
        assert!(!orc.active);
    }

    #[test]
    fn test_award_loot() {
        let mut state = GameState::new();
        state.gold = 0;
        let orc = make_orc();
        award_loot(&mut state, &orc);
        assert_eq!(state.gold, 5);
    }

    #[test]
    fn test_missile_ticks() {
        let mut m = Missile { active: true, x: 0, y: 100, dx: 2, dy: 0, damage: 5, is_friendly: true };
        let hit = m.tick(50, 100); // too far
        assert!(!hit);
        assert_eq!(m.x, 2);
    }

    #[test]
    fn test_fire_missile_slots() {
        let mut missiles = std::array::from_fn(|_| Missile::default());
        let slot = fire_missile(&mut missiles, 0, 0, 2, 5, true);
        assert!(slot.is_some());
        assert!(missiles[slot.unwrap()].active);
    }
}
