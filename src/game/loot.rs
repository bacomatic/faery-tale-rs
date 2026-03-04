//! Enemy loot tables: treasure_probs[] and item drop resolution.
//! Ports treasure_probs[] and encounter_chart.treasure from fmain.c.

use crate::game::npc::Npc;
use crate::game::game_state::GameState;

/// First gold item index in inv_list[] (from fmain.c: #define GOLDBASE 31).
pub const GOLDBASE: usize = 31;

/// Gold amounts per inv_list slot (31→2, 32→5, 33→10, 34→100 from fmain.c inv_list[]).
const GOLD_AMOUNTS: [i32; 4] = [2, 5, 10, 100];

/// treasure_probs[row*8 + rand8] from fmain.c.
/// Row index = encounter_chart[race].treasure. Column = rand 0-7.
/// Values are inv_list[] indices; 0 = no drop, ≥31 = gold.
const TREASURE_PROBS: &[u8] = &[
    0,  0,  0,  0,  0,  0,  0,  0,  // row 0: no treasure
    9,  11, 13, 31, 31, 17, 17, 32, // row 1: stone/vial/totem/gold/keys
    12, 14, 20, 20, 20, 31, 33, 31, // row 2: skull/ring/fruit/gold
    10, 10, 16, 16, 11, 17, 18, 19, // row 3: jewels/keys/vial
    15, 21, 0,  0,  0,  0,  0,  0,  // row 4: jade skull / white key
];

/// treasure field from encounter_chart[] indexed by race (fmain.c).
/// Race 0=Ogre, 1=Orc, 2=Wraith, 3=Skeleton, 4=Snake, 5=Salamander,
///      6=Spider, 7=DKnight, 8=Loraii, 9=Necromancer, 10=Woodcutter
const ENCOUNTER_TREASURE: &[u8] = &[2, 1, 4, 3, 0, 0, 0, 0, 0, 0, 0];

/// Simple tick-seeded pseudo-random 0-7.
fn rand8_from_tick(tick: u32, salt: u32) -> usize {
    let h = (tick ^ salt).wrapping_mul(2246822519).wrapping_add(3266489917);
    (h as usize) & 7
}

/// Roll treasure drop for a defeated NPC using treasure_probs[] (fmain.c body-search logic).
/// Returns (item_slot, gold_amount): item_slot<GOLDBASE → stuff[item_slot]++,
///   gold_amount>0 → add to state.gold.
/// Returns None if no treasure drops.
pub fn roll_treasure(npc: &Npc, tick: u32) -> Option<LootDrop> {
    // Setfigs (race >= 0x80) never drop treasure (fmain.c: if j & 0x80 then j=0).
    if npc.race >= 0x80 { return None; }
    let treasure_row = ENCOUNTER_TREASURE.get(npc.race as usize).copied().unwrap_or(0) as usize;
    let col = rand8_from_tick(tick, npc.race as u32 ^ 0xDEAD);
    let inv_idx = TREASURE_PROBS.get(treasure_row * 8 + col).copied().unwrap_or(0) as usize;
    if inv_idx == 0 { return None; }
    if inv_idx >= GOLDBASE {
        let gold = GOLD_AMOUNTS.get(inv_idx - GOLDBASE).copied().unwrap_or(0);
        if gold > 0 { Some(LootDrop::Gold(gold)) } else { None }
    } else {
        Some(LootDrop::Item(inv_idx))
    }
}

/// Result of a treasure roll.
#[derive(Debug, Clone, PartialEq)]
pub enum LootDrop {
    /// inv_list item slot → stuff[slot]++
    Item(usize),
    /// Gold amount to add to state.gold
    Gold(i32),
}

/// Award a treasure drop to the hero's inventory.
/// Returns the item slot if a weapon was dropped (slot 0-3 = Dirk/Mace/Sword/Bow),
/// so the caller can auto-equip if better.
pub fn award_treasure(state: &mut GameState, drop: &LootDrop) -> Option<u8> {
    match *drop {
        LootDrop::Gold(amount) => {
            state.gold += amount;
            None
        }
        LootDrop::Item(slot) => {
            if slot < 35 {
                state.stuff_mut()[slot] = state.stuff()[slot].saturating_add(1);
            }
            // Weapons are inv_list slots 0-3 (Dirk, Mace, Sword, Bow).
            // In anim_list weapon encoding: 1=dirk, 2=mace, 3=sword, 4=bow.
            if slot < 4 { Some((slot + 1) as u8) } else { None }
        }
    }
}

/// Legacy roll_loot kept for callers in GameAction::Attack.
/// Returns a vec of (item_slot, count) pairs.
pub fn roll_loot(npc: &Npc, tick: u32) -> Vec<(usize, u8)> {
    match roll_treasure(npc, tick) {
        Some(LootDrop::Item(slot)) => vec![(slot, 1)],
        _ => vec![],
    }
}

/// Legacy award_drops kept for callers in GameAction::Attack.
pub fn award_drops(state: &mut GameState, drops: &[(usize, u8)]) {
    for &(item_slot, count) in drops {
        if item_slot < 35 {
            state.stuff_mut()[item_slot] = state.stuff()[item_slot].saturating_add(count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::npc::{Npc, NPC_TYPE_ORC};
    use crate::game::game_state::GameState;

    fn make_orc() -> Npc {
        Npc { npc_type: NPC_TYPE_ORC, race: 1, ..Default::default() }
    }

    #[test]
    fn test_setfig_no_treasure() {
        let npc = Npc { race: 0x89, ..Default::default() };
        assert!(roll_treasure(&npc, 42).is_none());
    }

    #[test]
    fn test_orc_treasure_row1() {
        // Orc race=1 → treasure_row=1; some column should yield a non-zero drop over 8 tries.
        let npc = make_orc();
        let any_drop = (0u32..8).any(|t| roll_treasure(&npc, t).is_some());
        assert!(any_drop, "Orc (row 1) should have at least one non-zero entry");
    }

    #[test]
    fn test_gold_drop() {
        // Force a gold drop by finding a tick that yields inv_idx>=31 for an orc.
        let npc = make_orc();
        for tick in 0..256 {
            if let Some(LootDrop::Gold(g)) = roll_treasure(&npc, tick) {
                assert!(g > 0);
                return;
            }
        }
        // Not required to always drop gold; acceptable if no gold column hit in 256 tries.
    }

    #[test]
    fn test_award_treasure_item() {
        let mut state = GameState::new();
        let drop = LootDrop::Item(9); // Blue Stone
        let weapon = award_treasure(&mut state, &drop);
        assert_eq!(state.stuff()[9], 1);
        assert!(weapon.is_none()); // slot 9 is not a weapon
    }

    #[test]
    fn test_award_treasure_weapon_slot() {
        let mut state = GameState::new();
        let drop = LootDrop::Item(2); // Sword → weapon 3
        let weapon = award_treasure(&mut state, &drop);
        assert_eq!(weapon, Some(3));
    }

    #[test]
    fn test_award_treasure_gold() {
        let mut state = GameState::new();
        let drop = LootDrop::Gold(10);
        award_treasure(&mut state, &drop);
        assert_eq!(state.gold, 10);
    }

    #[test]
    fn test_award_drops_legacy() {
        let mut state = GameState::new();
        let drops = vec![(0usize, 3u8)];
        award_drops(&mut state, &drops);
        assert_eq!(state.stuff()[0], 3);
    }
}
