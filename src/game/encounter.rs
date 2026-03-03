//! Encounter spawning: chart-based enemy selection for random encounters.
//! Ports encounter_chart[] and encounter probability logic from fmain.c.

use crate::game::npc::{Npc, NPC_TYPE_ORC, NPC_TYPE_SKELETON, NPC_TYPE_GHOST, NPC_TYPE_WRAITH, RACE_ENEMY, RACE_UNDEAD, RACE_WRAITH};

/// encounter_chart[11]: enemy type per region/zone index.
/// Index 0-10 maps to different region types; values are NPC type codes.
pub const ENCOUNTER_CHART: [u8; 11] = [
    NPC_TYPE_ORC,       // 0: forest
    NPC_TYPE_ORC,       // 1: plains
    NPC_TYPE_SKELETON,  // 2: mountains
    NPC_TYPE_GHOST,     // 3: swamp
    NPC_TYPE_WRAITH,    // 4: dungeon
    NPC_TYPE_ORC,       // 5: road
    NPC_TYPE_SKELETON,  // 6: ruins
    NPC_TYPE_GHOST,     // 7: graveyard
    NPC_TYPE_WRAITH,    // 8: dark zone
    NPC_TYPE_ORC,       // 9: beach
    NPC_TYPE_SKELETON,  // 10: desert
];

/// Probability of encounter per tile step (0-255, checked against random).
/// Higher = more frequent encounters.
pub const ENCOUNTER_PROBABILITY: u8 = 32;

/// Determine if an encounter should trigger.
/// Uses a simple LCG pseudorandom check against probability.
pub fn should_encounter(tick: u32) -> bool {
    // Deterministic pseudo-random using tick count
    let rng = tick.wrapping_mul(1664525).wrapping_add(1013904223);
    (rng & 0xFF) < ENCOUNTER_PROBABILITY as u32
}

/// Spawn an encounter NPC at the given position for the given region/zone.
pub fn spawn_encounter(region_zone_idx: usize, x: i16, y: i16) -> Npc {
    let npc_type = ENCOUNTER_CHART[region_zone_idx.min(10)];
    let race = match npc_type {
        t if t == NPC_TYPE_WRAITH => RACE_WRAITH,
        t if t == NPC_TYPE_GHOST || t == NPC_TYPE_SKELETON => RACE_UNDEAD,
        _ => RACE_ENEMY,
    };
    Npc {
        npc_type,
        race,
        x: x + 50, // spawn slightly offset from hero
        y: y + 50,
        vitality: 8 + (region_zone_idx as i16 * 2), // harder in later zones
        gold: 5 + region_zone_idx as i16,
        speed: 2 + (region_zone_idx / 4) as u8,
        active: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encounter_chart_length() {
        assert_eq!(ENCOUNTER_CHART.len(), 11);
    }

    #[test]
    fn test_spawn_encounter_wraith() {
        let npc = spawn_encounter(4, 0, 0); // dungeon = wraith
        assert_eq!(npc.npc_type, NPC_TYPE_WRAITH);
        assert_eq!(npc.race, RACE_WRAITH);
    }

    #[test]
    fn test_should_encounter_deterministic() {
        // Same tick always gives same result
        let r1 = should_encounter(12345);
        let r2 = should_encounter(12345);
        assert_eq!(r1, r2);
    }
}
