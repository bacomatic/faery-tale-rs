//! Encounter spawning: chart-based enemy selection for random encounters.
//! Ports encounter_chart[] and encounter probability logic from fmain.c.

use crate::game::npc::{Npc, NPC_TYPE_ORC, NPC_TYPE_SKELETON, NPC_TYPE_GHOST, NPC_TYPE_WRAITH, RACE_ENEMY, RACE_UNDEAD, RACE_WRAITH, RACE_SNAKE};

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

/// Per-enemy-type stats from fmain.c encounter_chart[].
#[derive(Debug, Clone, Copy)]
pub struct EnemyTypeStats {
    pub hp: i16,
    pub arms: u8,
    pub clever: u8,
    pub treasure: u8,
    pub cfile: u8,
}

/// Full encounter chart from fmain.c (11 enemy types).
pub const ENCOUNTER_CHART_FULL: [EnemyTypeStats; 11] = [
    EnemyTypeStats { hp: 18, arms: 2, clever: 0, treasure: 2, cfile: 6 },  // 0: Ogre
    EnemyTypeStats { hp: 12, arms: 4, clever: 1, treasure: 1, cfile: 6 },  // 1: Orcs
    EnemyTypeStats { hp: 16, arms: 6, clever: 1, treasure: 4, cfile: 7 },  // 2: Wraith
    EnemyTypeStats { hp: 8,  arms: 3, clever: 0, treasure: 3, cfile: 7 },  // 3: Skeleton
    EnemyTypeStats { hp: 16, arms: 6, clever: 1, treasure: 0, cfile: 8 },  // 4: Snake
    EnemyTypeStats { hp: 9,  arms: 3, clever: 0, treasure: 0, cfile: 7 },  // 5: Salamander
    EnemyTypeStats { hp: 10, arms: 6, clever: 1, treasure: 0, cfile: 8 },  // 6: Spider
    EnemyTypeStats { hp: 40, arms: 7, clever: 1, treasure: 0, cfile: 8 },  // 7: DKnight
    EnemyTypeStats { hp: 12, arms: 6, clever: 1, treasure: 0, cfile: 9 },  // 8: Loraii
    EnemyTypeStats { hp: 50, arms: 5, clever: 0, treasure: 0, cfile: 9 },  // 9: Necromancer
    EnemyTypeStats { hp: 4,  arms: 0, clever: 0, treasure: 0, cfile: 9 },  // 10: Woodcutter
];

/// Weapon probability table from fmain.c weapon_probs[].
/// Indexed by arms * 4 + rand4(). Returns weapon index for the NPC.
pub const WEAPON_PROBS: [u8; 32] = [
    0, 0, 0, 0,  // arms=0: no weapon
    1, 1, 1, 1,  // arms=1: dirk
    1, 1, 2, 1,  // arms=2: mostly dirk, some mace
    2, 1, 2, 2,  // arms=3: mostly mace
    2, 2, 3, 2,  // arms=4: mace with sword chance
    3, 3, 2, 3,  // arms=5: mostly sword
    3, 3, 4, 3,  // arms=6: sword with bow chance
    4, 3, 4, 4,  // arms=7: bow-heavy
];

fn rand4_from_tick(tick: u32) -> u32 {
    let h = tick.wrapping_mul(2246822519).wrapping_add(3266489917);
    (h >> 16) & 3
}

fn rand64_from_tick(tick: u32) -> u32 {
    let h = tick.wrapping_mul(1664525).wrapping_add(1013904223);
    (h >> 10) & 63
}

/// Check if any active enemy NPC is within visibility range (~300px).
pub fn actors_on_screen(
    table: &crate::game::npc::NpcTable,
    hero_x: i16,
    hero_y: i16,
) -> bool {
    table.npcs.iter().any(|n| {
        n.active
            && (n.x as i32 - hero_x as i32).abs() < 300
            && (n.y as i32 - hero_y as i32).abs() < 300
    })
}

/// Count active enemies in the NPC table.
pub fn active_enemy_count(table: &crate::game::npc::NpcTable) -> usize {
    table.npcs.iter().filter(|n| n.active).count()
}

/// Determine encounter type, with zone overrides.
pub fn pick_encounter_type(xtype: u16, tick: u32) -> usize {
    let base = rand4_from_tick(tick) as usize;
    match xtype {
        7 if base == 2 => 4,   // swamp: wraith→snake
        8 => 6,                // spider zone: always spider
        49 => 2,               // forced wraith
        _ => base,
    }
}

/// Check if a random encounter should trigger this tick.
/// Ports fmain.c:2451-2472 gating conditions.
/// Returns Some(encounter_type) if spawn should happen, None otherwise.
pub fn try_trigger_encounter(
    tick: u32,
    table: &crate::game::npc::NpcTable,
    hero_x: i16,
    hero_y: i16,
    xtype: u16,
    region_num: u8,
) -> Option<usize> {
    // Gate 1: every 32 ticks only (~1 Hz at 30fps)
    if tick & 31 != 0 {
        return None;
    }
    // Gate 2: no actors on screen
    if actors_on_screen(table, hero_x, hero_y) {
        return None;
    }
    // Gate 3: fewer than 4 active enemies
    if active_enemy_count(table) >= 4 {
        return None;
    }
    // Gate 4: not in forced encounter zone
    if xtype >= 50 {
        return None;
    }
    // Gate 5: danger level check
    let danger = if region_num > 7 {
        5 + xtype as u32
    } else {
        2 + xtype as u32
    };
    let roll = rand64_from_tick(tick);
    if roll > danger {
        return None;
    }
    Some(pick_encounter_type(xtype, tick))
}

/// Spawn a single encounter NPC with stats from the full encounter chart.
pub fn spawn_encounter(encounter_type: usize, hero_x: i16, hero_y: i16, tick: u32) -> Npc {
    let etype = encounter_type.min(10);
    let stats = &ENCOUNTER_CHART_FULL[etype];
    let wp_idx = (stats.arms as usize * 4 + rand4_from_tick(tick.wrapping_add(etype as u32)) as usize).min(31);
    let weapon = WEAPON_PROBS[wp_idx];
    let race = match etype {
        2 => RACE_WRAITH,
        3 | 5 => RACE_UNDEAD,
        4 => RACE_SNAKE,
        _ => RACE_ENEMY,
    };
    Npc {
        npc_type: etype as u8,
        race,
        x: hero_x.saturating_add(50),
        y: hero_y.saturating_add(50),
        vitality: stats.hp,
        gold: stats.treasure as i16 * 5,
        speed: 2,
        weapon,
        active: true,
        ..Default::default()
    }
}

/// Spawn up to 4 enemies into free NPC slots, mirroring fmain.c group encounter logic.
/// Spawn positions fan out in 4 cardinal directions from the hero.
/// Returns the number of NPCs spawned.
pub fn spawn_encounter_group(
    table: &mut crate::game::npc::NpcTable,
    encounter_type: usize,
    hero_x: i16,
    hero_y: i16,
    tick: u32,
) -> usize {
    const MAX_GROUP: usize = 4;
    const OFFSETS: [(i16, i16); 4] = [(48, 0), (-48, 0), (0, 48), (0, -48)];

    let mut spawned = 0;
    for (i, (ox, oy)) in OFFSETS.iter().enumerate() {
        if spawned >= MAX_GROUP {
            break;
        }
        if let Some(slot) = table.npcs.iter_mut().find(|n| !n.active) {
            let mut npc = spawn_encounter(encounter_type, hero_x, hero_y, tick.wrapping_add(i as u32));
            npc.x = hero_x.saturating_add(*ox);
            npc.y = hero_y.saturating_add(*oy);
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
    fn test_encounter_chart_full_length() {
        assert_eq!(ENCOUNTER_CHART_FULL.len(), 11);
    }

    #[test]
    fn test_spawn_encounter_uses_chart_hp() {
        let npc = spawn_encounter(0, 100, 100, 42);
        assert_eq!(npc.vitality, 18); // Ogre HP
    }

    #[test]
    fn test_spawn_encounter_wraith_race() {
        let npc = spawn_encounter(2, 0, 0, 42);
        assert_eq!(npc.race, RACE_WRAITH);
    }

    #[test]
    fn test_spawn_encounter_has_weapon() {
        let npc = spawn_encounter(0, 0, 0, 42);
        assert!(npc.weapon <= 5, "weapon {} out of range", npc.weapon);
    }

    #[test]
    fn test_actors_on_screen_empty() {
        let table = crate::game::npc::NpcTable { npcs: Default::default() };
        assert!(!actors_on_screen(&table, 100, 100));
    }

    #[test]
    fn test_actors_on_screen_nearby() {
        let mut table = crate::game::npc::NpcTable { npcs: Default::default() };
        table.npcs[0] = Npc { active: true, x: 150, y: 150, ..Default::default() };
        assert!(actors_on_screen(&table, 100, 100));
    }

    #[test]
    fn test_actors_on_screen_far_away() {
        let mut table = crate::game::npc::NpcTable { npcs: Default::default() };
        table.npcs[0] = Npc { active: true, x: 1000, y: 1000, ..Default::default() };
        assert!(!actors_on_screen(&table, 100, 100));
    }

    #[test]
    fn test_try_trigger_encounter_respects_tick_gate() {
        let table = crate::game::npc::NpcTable { npcs: Default::default() };
        assert!(try_trigger_encounter(1, &table, 100, 100, 0, 0).is_none());
    }

    #[test]
    fn test_try_trigger_encounter_blocks_when_actors_present() {
        let mut table = crate::game::npc::NpcTable { npcs: Default::default() };
        table.npcs[0] = Npc { active: true, x: 100, y: 100, ..Default::default() };
        assert!(try_trigger_encounter(32, &table, 100, 100, 0, 0).is_none());
    }

    #[test]
    fn test_active_enemy_count() {
        let mut table = crate::game::npc::NpcTable { npcs: Default::default() };
        assert_eq!(active_enemy_count(&table), 0);
        table.npcs[0] = Npc { active: true, ..Default::default() };
        table.npcs[3] = Npc { active: true, ..Default::default() };
        assert_eq!(active_enemy_count(&table), 2);
    }

    #[test]
    fn test_try_trigger_encounter_blocks_at_4_enemies() {
        let mut table = crate::game::npc::NpcTable { npcs: Default::default() };
        for i in 0..4 {
            table.npcs[i] = Npc { active: true, x: 5000, y: 5000, ..Default::default() };
        }
        assert!(try_trigger_encounter(32, &table, 100, 100, 0, 0).is_none());
    }

    #[test]
    fn test_spawn_encounter_group_caps_at_4() {
        let mut table = crate::game::npc::NpcTable { npcs: Default::default() };
        let spawned = spawn_encounter_group(&mut table, 0, 100, 100, 42);
        assert_eq!(spawned, 4);
        assert_eq!(active_enemy_count(&table), 4);
    }

    #[test]
    fn test_pick_encounter_type_swamp_override() {
        let mut found = false;
        for tick in 0..1000u32 {
            if rand4_from_tick(tick) == 2 {
                let etype = pick_encounter_type(7, tick);
                assert_eq!(etype, 4, "swamp should override type 2 to 4 (snake)");
                found = true;
                break;
            }
        }
        assert!(found, "should find a tick that produces base=2");
    }
}
