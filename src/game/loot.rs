//! Enemy loot tables: treasure_probs[] and item drop resolution.
//! Ports treasure_probs[] from fmain.c.

use crate::game::npc::{NPC_TYPE_ORC, NPC_TYPE_SKELETON, NPC_TYPE_GHOST, NPC_TYPE_WRAITH, NPC_TYPE_CONTAINER};
use crate::game::npc::Npc;
use crate::game::game_state::GameState;

/// Loot drop: (item_slot, count, probability 0-255).
pub struct DropEntry {
    pub item_slot: usize,
    pub count: u8,
    pub probability: u8, // 0-255; checked against tick-based rng
}

static ORC_LOOT: [DropEntry; 2] = [
    DropEntry { item_slot: 0, count: 1, probability: 100 }, // food
    DropEntry { item_slot: 3, count: 1, probability: 30 },  // key
];

static SKELETON_LOOT: [DropEntry; 1] = [
    DropEntry { item_slot: 9, count: 1, probability: 50 },  // short sword
];

static GHOST_WRAITH_LOOT: [DropEntry; 1] = [
    DropEntry { item_slot: 25, count: 1, probability: 20 }, // ring
];

static CONTAINER_LOOT: [DropEntry; 2] = [
    DropEntry { item_slot: 4, count: 2, probability: 255 }, // 2 potions
    DropEntry { item_slot: 1, count: 3, probability: 255 }, // 3 torches
];

/// Loot table per NPC type (treasure_probs[] from fmain.c).
pub fn loot_table(npc_type: u8) -> &'static [DropEntry] {
    if npc_type >= NPC_TYPE_CONTAINER {
        return &CONTAINER_LOOT;
    }
    match npc_type {
        t if t == NPC_TYPE_ORC => &ORC_LOOT,
        t if t == NPC_TYPE_SKELETON => &SKELETON_LOOT,
        t if t == NPC_TYPE_GHOST || t == NPC_TYPE_WRAITH => &GHOST_WRAITH_LOOT,
        _ => &[],
    }
}

/// Roll for loot drops using tick-based rng.
/// Returns a vec of (item_slot, count) pairs for items that dropped.
pub fn roll_loot(npc: &Npc, tick: u32) -> Vec<(usize, u8)> {
    let table = loot_table(npc.npc_type);
    table.iter().filter_map(|entry| {
        let rng: u64 = (tick as u64).wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        if (rng & 0xFF) < entry.probability as u64 {
            Some((entry.item_slot, entry.count))
        } else {
            None
        }
    }).collect()
}

/// Award loot drops to the hero's inventory.
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
    use crate::game::npc::NPC_TYPE_ORC;
    use crate::game::game_state::GameState;

    #[test]
    fn test_loot_table_orc() {
        let orc = Npc {
            npc_type: NPC_TYPE_ORC,
            ..Default::default()
        };
        let table = loot_table(orc.npc_type);
        assert!(!table.is_empty());
    }

    #[test]
    fn test_award_drops() {
        let mut state = GameState::new();
        let drops = vec![(0usize, 3u8)]; // 3 food
        award_drops(&mut state, &drops);
        assert_eq!(state.stuff()[0], 3);
    }
}
