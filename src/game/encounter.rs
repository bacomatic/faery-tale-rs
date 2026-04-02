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
        ..Default::default()
    }
}

/// Spawn up to 4 enemies into free NPC slots, mirroring fmain.c group encounter logic.
///
/// Applies mixflag blending: each successive NPC alternates the low bit of race
/// (`race = (base_race & 0xFE) | (i & 1)`), creating a mixed enemy group.
/// Spawn positions fan out in 4 cardinal directions from the hero.
///
/// Returns the number of NPCs spawned.
pub fn spawn_encounter_group(
    table: &mut crate::game::npc::NpcTable,
    region_zone_idx: usize,
    hero_x: i16,
    hero_y: i16,
) -> usize {
    const MAX_GROUP: usize = 4;
    const OFFSETS: [(i16, i16); 4] = [(48, 0), (-48, 0), (0, 48), (0, -48)];

    let base = spawn_encounter(region_zone_idx, hero_x, hero_y);
    let base_race = base.race & 0xFE; // clear low bit for mixflag alternation

    let mut spawned = 0;
    for (i, (ox, oy)) in OFFSETS.iter().enumerate() {
        if spawned >= MAX_GROUP {
            break;
        }
        if let Some(slot) = table.npcs.iter_mut().find(|n| !n.active) {
            let mut npc = spawn_encounter(region_zone_idx, hero_x, hero_y);
            npc.x = hero_x.saturating_add(*ox);
            npc.y = hero_y.saturating_add(*oy);
            npc.race = base_race | (i as u8 & 1); // mixflag: alternate even/odd
            *slot = npc;
            spawned += 1;
        }
    }
    spawned
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

    #[test]
    fn test_spawn_encounter_group_fills_slots() {
        use crate::game::npc::NpcTable;
        let mut table = NpcTable { npcs: Default::default() };
        let spawned = spawn_encounter_group(&mut table, 0, 100, 100);
        assert_eq!(spawned, 4, "should fill 4 free slots");
        let active: Vec<_> = table.npcs.iter().filter(|n| n.active).collect();
        assert_eq!(active.len(), 4);
    }

    #[test]
    fn test_spawn_encounter_group_respects_existing() {
        use crate::game::npc::{NpcTable, Npc};
        let mut table = NpcTable { npcs: Default::default() };
        // Pre-fill 14 of the 16 slots
        for i in 0..14 {
            table.npcs[i] = Npc { active: true, ..Default::default() };
        }
        let spawned = spawn_encounter_group(&mut table, 0, 100, 100);
        assert_eq!(spawned, 2, "should only fill the 2 remaining free slots");
    }

    #[test]
    fn test_spawn_encounter_group_mixflag_alternates_race() {
        use crate::game::npc::NpcTable;
        let mut table = NpcTable { npcs: Default::default() };
        spawn_encounter_group(&mut table, 0, 100, 100);
        // With mixflag blending, consecutive enemies alternate race (even/odd LSB).
        let active: Vec<_> = table.npcs.iter().filter(|n| n.active).collect();
        assert!(active.len() >= 2);
        // LSBs of race for first two should differ by 1 (0 and 1, or 2 and 3, etc.)
        let r0 = active[0].race & 1;
        let r1 = active[1].race & 1;
        assert_ne!(r0, r1, "mixflag: consecutive NPCs should alternate race LSB");
    }
}
