//! Combat system: melee combat resolution between hero and NPCs.
//! Ports the battle loop from original fmain.c.

use crate::game::npc::{Npc, RACE_UNDEAD, RACE_WRAITH};
use crate::game::game_state::GameState;

/// Weapon type damage factors (from original weapon table).
/// Index = weapon slot in stuff[], value = damage multiplier.
pub const WEAPON_DAMAGE: &[u8] = &[
    1,  // fists (slot 0, placeholder)
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
    // Additional item drops handled by npc-106
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::npc::{Npc, NPC_TYPE_ORC, RACE_ENEMY};
    use crate::game::game_state::GameState;

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
}
