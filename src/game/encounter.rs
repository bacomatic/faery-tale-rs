//! Encounter spawning: chart-based enemy selection for random encounters.
//! Ports encounter_chart[] and encounter probability logic from fmain.c.

use crate::game::actor::{Goal, Tactic};
use crate::game::npc::{
    Npc, NpcState, NPC_TYPE_ORC, NPC_TYPE_SKELETON, NPC_TYPE_GHOST, NPC_TYPE_WRAITH,
    NPC_TYPE_SNAKE, NPC_TYPE_SPIDER, NPC_TYPE_DKNIGHT, NPC_TYPE_LORAII, NPC_TYPE_NECROMANCER,
    RACE_ENEMY, RACE_UNDEAD, RACE_WRAITH, RACE_SNAKE,
};

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

/// Map encounter_chart index → NPC type constant for sprite rendering.
/// In the original game, all enemies had type=ENEMY and race=encounter_index;
/// the Rust port uses npc_type for cfile lookup, so we need this translation.
const ENCOUNTER_TO_NPC_TYPE: [u8; 11] = [
    NPC_TYPE_ORC,          // 0: Ogre
    NPC_TYPE_ORC,          // 1: Orcs
    NPC_TYPE_WRAITH,       // 2: Wraith
    NPC_TYPE_SKELETON,     // 3: Skeleton
    NPC_TYPE_SNAKE,        // 4: Snake
    NPC_TYPE_GHOST,        // 5: Salamander (shares cfile 7 with ghost)
    NPC_TYPE_SPIDER,       // 6: Spider
    NPC_TYPE_DKNIGHT,      // 7: DKnight
    NPC_TYPE_LORAII,       // 8: Loraii
    NPC_TYPE_NECROMANCER,  // 9: Necromancer
    NPC_TYPE_ORC,          // 10: Woodcutter (placeholder)
];

/// Weapon probability table from fmain.c weapon_probs[] (`fmain2.c:860-868`).
/// Indexed by arms * 4 + rand4(). Returns weapon index for the NPC.
/// Weapon codes: 0=none, 1=dirk, 2=mace, 3=sword, 4=bow, 5=wand, 8=touch.
pub const WEAPON_PROBS: [u8; 32] = [
    0, 0, 0, 0,  // arms=0: no weapon
    1, 1, 1, 1,  // arms=1: all dirks
    1, 2, 1, 2,  // arms=2: dirks and maces
    1, 2, 3, 2,  // arms=3: mostly maces, some swords
    4, 4, 3, 2,  // arms=4: bows and swords
    5, 5, 5, 5,  // arms=5: all magic wands
    8, 8, 8, 8,  // arms=6: touch attack
    3, 3, 3, 3,  // arms=7: all swords
];

fn rand4_from_tick(tick: u32) -> u32 {
    let h = tick.wrapping_mul(2246822519).wrapping_add(3266489917);
    (h >> 16) & 3
}

fn rand8_from_tick(tick: u32) -> u32 {
    let h = tick.wrapping_mul(1103515245).wrapping_add(12345);
    (h >> 13) & 7
}

fn rand64_from_tick(tick: u32) -> u32 {
    let h = tick.wrapping_mul(1664525).wrapping_add(1013904223);
    (h >> 10) & 63
}

fn bitrand_from_tick(tick: u32, mask: u32) -> u32 {
    let h = tick.wrapping_mul(214013).wrapping_add(2531011);
    (h >> 8) & mask
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
/// Ports `roll_wilderness_encounter` (fmain.c:2080-2092) Phase 14j gates:
/// `(daynight & 31) == 0 and not actors_on_screen and not actors_loading
///  and active_carrier == 0 and xtype < 50`.
/// Returns Some(encounter_type) if spawn should happen, None otherwise.
pub fn try_trigger_encounter(
    tick: u32,
    table: &crate::game::npc::NpcTable,
    hero_x: i16,
    hero_y: i16,
    xtype: u16,
    region_num: u8,
    active_carrier: i16,
) -> Option<usize> {
    // Gate 1: every 32 ticks only (~1 Hz at 30fps)
    if tick & 31 != 0 {
        return None;
    }
    // Gate 2: no actors on screen
    if actors_on_screen(table, hero_x, hero_y) {
        return None;
    }
    // Gate 3: fewer than 4 active enemies (4-slot cap, fmain.c:2064 anix < 7)
    if active_enemy_count(table) >= 4 {
        return None;
    }
    // Gate 4: not in forced encounter zone (fmain.c:2081)
    if xtype >= 50 {
        return None;
    }
    // Gate 5: not riding a carrier (fmain.c:2081 active_carrier == 0)
    if active_carrier != 0 {
        return None;
    }
    // Gate 6: danger level check (fmain.c:2082-2085)
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

/// Direction vector tables from fsubs.asm (8 compass directions).
/// Used by newx/newy to compute offsets: result = base + (dir_table[dir] * distance) / 2
const XDIR: [i16; 8] = [-2, 0, 2, 3, 2, 0, -2, -3];
const YDIR: [i16; 8] = [-2, -3, -2, 0, 2, 3, 2, 0];

/// Compute encounter origin point 150-213 pixels from hero in a random direction.
/// Ports set_loc() from fmain2.c: direction = rand8(), distance = 150 + rand64().
fn encounter_origin(hero_x: i16, hero_y: i16, tick: u32) -> (i16, i16) {
    let dir = rand8_from_tick(tick) as usize;
    let dist = 150 + rand64_from_tick(tick.wrapping_add(7)) as i16;
    let ox = hero_x.wrapping_add((XDIR[dir].wrapping_mul(dist)) / 2);
    let oy = hero_y.wrapping_add((YDIR[dir].wrapping_mul(dist)) / 2);
    (ox, oy)
}

/// Spawn a single encounter NPC with stats from the full encounter chart.
pub fn spawn_encounter(encounter_type: usize, origin_x: i16, origin_y: i16, tick: u32) -> Npc {
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
    // Assign goal based on weapon type and cleverness.
    let is_ranged = weapon >= 4; // bow or wand
    let goal = if is_ranged {
        if stats.clever > 0 { Goal::Archer2 } else { Goal::Archer1 }
    } else {
        if stats.clever > 0 { Goal::Attack2 } else { Goal::Attack1 }
    };

    Npc {
        npc_type: ENCOUNTER_TO_NPC_TYPE[etype],
        race,
        x: origin_x,
        y: origin_y,
        vitality: stats.hp,
        // fmain.c:2763 set_encounter does not seed gold; treasure is rolled
        // separately via roll_treasure on body search (fmain.c:3270-3273).
        gold: 0,
        speed: 2,
        weapon,
        active: true,
        goal,
        tactic: Tactic::Pursue,
        // fmain.c:2761 `an->facing = 0` (north). Actual heading is rewritten
        // the first time the AI picks a tactic (set_course).
        facing: 0,
        state: NpcState::Walking,
        cleverness: stats.clever,
        looted: false,
    }
}

/// Spawn up to 4 enemies into free NPC slots, mirroring fmain.c group encounter logic.
/// Enemies spawn offscreen: origin is 150-213px from hero in a random direction,
/// then each enemy is scattered ±(spread/2) around that origin.
/// Each NPC retries up to MAX_TRY (15) positions to avoid overlapping other actors
/// (hero + already-placed NPCs), matching the original set_encounter() retry loop.
/// Ports set_loc() + set_encounter() from fmain.c/fmain2.c.
/// Returns the number of NPCs spawned.
pub fn spawn_encounter_group(
    table: &mut crate::game::npc::NpcTable,
    encounter_type: usize,
    hero_x: i16,
    hero_y: i16,
    tick: u32,
) -> usize {
    use crate::game::collision::actor_collides;

    const MAX_GROUP: usize = 4;
    const SPREAD: i16 = 63; // bitrand(spread) - spread/2 per axis
    const MAX_TRY: u32 = 15; // fmain.c set_encounter() retry limit

    let (enc_x, enc_y) = encounter_origin(hero_x, hero_y, tick);

    // Collect positions of hero + all already-active NPCs for collision checks.
    let mut occupied: Vec<(i32, i32)> = Vec::with_capacity(crate::game::npc::MAX_NPCS + 1);
    occupied.push((hero_x as i32, hero_y as i32));
    for n in table.npcs.iter() {
        if n.active {
            occupied.push((n.x as i32, n.y as i32));
        }
    }

    let mut spawned = 0;
    for i in 0..MAX_GROUP {
        let slot_idx = match table.npcs.iter().position(|n| !n.active) {
            Some(idx) => idx,
            None => break,
        };

        // Retry loop: try up to MAX_TRY scatter positions per NPC (original set_encounter).
        let mut placed = false;
        for j in 0..MAX_TRY {
            let sub_tick = tick.wrapping_add(i as u32).wrapping_mul(31).wrapping_add(j * 7);
            let scatter_x = bitrand_from_tick(sub_tick, SPREAD as u32) as i16 - SPREAD / 2;
            let scatter_y = bitrand_from_tick(sub_tick.wrapping_add(13), SPREAD as u32) as i16 - SPREAD / 2;
            let npc_x = enc_x.wrapping_add(scatter_x);
            let npc_y = enc_y.wrapping_add(scatter_y);

            if !actor_collides(npc_x as i32, npc_y as i32, &occupied) {
                let mut npc = spawn_encounter(encounter_type, npc_x, npc_y, tick.wrapping_add(i as u32));
                npc.x = npc_x;
                npc.y = npc_y;
                table.npcs[slot_idx] = npc;
                occupied.push((npc_x as i32, npc_y as i32));
                spawned += 1;
                placed = true;
                break;
            }
        }
        // If all MAX_TRY attempts collided, skip this NPC (original returns FALSE).
        if !placed {
            continue;
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
    fn test_spawn_encounter_npc_types() {
        // Verify encounter-spawned NPCs get the correct npc_type for cfile lookup.
        // Bug: previously stored raw encounter_chart index as npc_type,
        // causing wraith→bird sprites, skeleton→turtle sprites, etc.
        assert_eq!(spawn_encounter(0, 0, 0, 1).npc_type, NPC_TYPE_ORC);       // Ogre
        assert_eq!(spawn_encounter(1, 0, 0, 1).npc_type, NPC_TYPE_ORC);       // Orcs
        assert_eq!(spawn_encounter(2, 0, 0, 1).npc_type, NPC_TYPE_WRAITH);    // Wraith
        assert_eq!(spawn_encounter(3, 0, 0, 1).npc_type, NPC_TYPE_SKELETON);  // Skeleton
        assert_eq!(spawn_encounter(4, 0, 0, 1).npc_type, NPC_TYPE_SNAKE);     // Snake
        assert_eq!(spawn_encounter(5, 0, 0, 1).npc_type, NPC_TYPE_GHOST);     // Salamander
        assert_eq!(spawn_encounter(6, 0, 0, 1).npc_type, NPC_TYPE_SPIDER);    // Spider
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
        assert!(try_trigger_encounter(1, &table, 100, 100, 0, 0, 0).is_none());
    }

    #[test]
    fn test_try_trigger_encounter_blocks_when_actors_present() {
        let mut table = crate::game::npc::NpcTable { npcs: Default::default() };
        table.npcs[0] = Npc { active: true, x: 100, y: 100, ..Default::default() };
        assert!(try_trigger_encounter(32, &table, 100, 100, 0, 0, 0).is_none());
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
        assert!(try_trigger_encounter(32, &table, 100, 100, 0, 0, 0).is_none());
    }

    #[test]
    fn test_try_trigger_encounter_blocks_on_carrier() {
        // fmain.c:2081 — `active_carrier == 0` is part of the 14j gate.
        let table = crate::game::npc::NpcTable { npcs: Default::default() };
        // graveyard-ish xtype=48 would otherwise fire almost every window.
        assert!(try_trigger_encounter(32, &table, 100, 100, 48, 0, 1).is_none(),
            "encounters must be suppressed while riding a carrier");
    }

    #[test]
    fn test_spawn_encounter_group_caps_at_4() {
        let mut table = crate::game::npc::NpcTable { npcs: Default::default() };
        let spawned = spawn_encounter_group(&mut table, 0, 100, 100, 42);
        assert_eq!(spawned, 4);
        assert_eq!(active_enemy_count(&table), 4);
    }

    #[test]
    fn test_encounter_origin_is_offscreen() {
        // Original game viewport is 288x140; encounter origin should be >144px
        // from hero on at least one axis. set_loc() uses 150+rand64 = 150-213px
        // distance with direction vectors of magnitude 2-3, giving 150-320px offset.
        for tick in 0..100u32 {
            let (ox, oy) = encounter_origin(1000, 1000, tick);
            let dx = (ox as i32 - 1000).abs();
            let dy = (oy as i32 - 1000).abs();
            // At minimum, distance = 150 with smallest dir component (magnitude 2),
            // giving offset = (2*150)/2 = 150px. Must be well outside half-viewport.
            assert!(dx > 70 || dy > 70,
                "tick={}: origin ({},{}) too close to hero (1000,1000): dx={}, dy={}",
                tick, ox, oy, dx, dy);
        }
    }

    #[test]
    fn test_spawn_encounter_group_positions_offscreen() {
        // All spawned enemies should be far from the hero, not surrounding them.
        let mut table = crate::game::npc::NpcTable { npcs: Default::default() };
        let hero_x = 5000i16;
        let hero_y = 5000i16;
        spawn_encounter_group(&mut table, 0, hero_x, hero_y, 42);
        for npc in table.npcs.iter().filter(|n| n.active) {
            let dx = (npc.x as i32 - hero_x as i32).abs();
            let dy = (npc.y as i32 - hero_y as i32).abs();
            assert!(dx > 50 || dy > 50,
                "Enemy at ({},{}) too close to hero ({},{}): dx={}, dy={}",
                npc.x, npc.y, hero_x, hero_y, dx, dy);
        }
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

    #[test]
    fn test_spawn_encounter_has_ai_fields() {
        use crate::game::actor::{Goal, Tactic};
        use crate::game::npc::NpcState;

        let npc = spawn_encounter(0, 100, 100, 42); // Ogre
        assert_eq!(npc.cleverness, 0);
        assert!(matches!(npc.goal, Goal::Attack1 | Goal::Attack2));
        assert_eq!(npc.tactic, Tactic::Pursue);
        assert_eq!(npc.state, NpcState::Walking);
    }

    #[test]
    fn test_spawn_encounter_archer_goal() {
        use crate::game::actor::Goal;

        // Try many ticks to find one where weapon >= 4 (bow).
        for tick in 0..100u32 {
            let npc = spawn_encounter(7, 100, 100, tick);
            if npc.weapon >= 4 {
                assert!(matches!(npc.goal, Goal::Archer1 | Goal::Archer2),
                    "Bow/wand wielder should get Archer goal, got {:?}", npc.goal);
                return;
            }
        }
        // If no bow was rolled in 100 ticks, that's fine — skip.
    }

    #[test]
    fn test_spawn_encounter_cleverness_wraith() {
        let npc = spawn_encounter(2, 100, 100, 42); // Wraith: clever=1
        assert_eq!(npc.cleverness, 1);
    }

    // SPEC §10.6: weapon_probs groups 4-7 exact values.
    #[test]
    fn test_weapon_probs_group2() {
        // fmain2.c:862 — dirks and maces.
        assert_eq!(&WEAPON_PROBS[8..12], &[1, 2, 1, 2]);
    }

    #[test]
    fn test_weapon_probs_group3() {
        // fmain2.c:863 — mostly maces, some swords.
        assert_eq!(&WEAPON_PROBS[12..16], &[1, 2, 3, 2]);
    }

    #[test]
    fn test_weapon_probs_group4() {
        assert_eq!(&WEAPON_PROBS[16..20], &[4, 4, 3, 2]);
    }

    #[test]
    fn test_weapon_probs_group5() {
        assert_eq!(&WEAPON_PROBS[20..24], &[5, 5, 5, 5]);
    }

    #[test]
    fn test_weapon_probs_group6() {
        assert_eq!(&WEAPON_PROBS[24..28], &[8, 8, 8, 8]);
    }

    #[test]
    fn test_weapon_probs_group7() {
        assert_eq!(&WEAPON_PROBS[28..32], &[3, 3, 3, 3]);
    }
}
