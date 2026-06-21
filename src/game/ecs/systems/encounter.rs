//! EncounterSystem — triggers random encounters and spawns enemy groups.
//! Port of the danger-check and set_encounter logic from `src/game/encounter.rs`.

use crate::game::actor::Goal;
use crate::game::collision::{actor_collides, proxcheck};
use crate::game::combat::rand4 as combat_rand4;
use crate::game::ecs::components::{AiState, Enemy, Health, Position};
use crate::game::ecs::resources::Resources;
use crate::game::ecs::spawn::spawn_enemy;
use crate::game::npc::{RACE_ENEMY, RACE_SNAKE, RACE_UNDEAD, RACE_WRAITH};
use hecs::World;

/// Direction vector tables from fsubs.asm (8 compass directions).
/// Used by `random_origin` to mirror the original `set_loc()` algorithm.
const XDIR: [i32; 8] = [-2, 0, 2, 3, 2, 0, -2, -3];
const YDIR: [i32; 8] = [-2, -3, -2, 0, 2, 3, 2, 0];

const WORLD_MAX: i32 = 32768;

/// Run the encounter system once per gameplay tick.
pub fn run(world: &mut World, res: &mut Resources) {
    if let Some(encounter_type) = try_trigger_encounter(world, res) {
        let n = spawn_encounter_group(world, res, encounter_type);
        if n > 0 {
            res.region.encounter_type = encounter_type as u16;
            res.region.encounter_number = res.region.encounter_number.wrapping_add(1);
        }
    }
}

/// Apply all encounter trigger gates.
/// Returns Some(encounter_type 0–10) if an encounter should fire, None otherwise.
fn try_trigger_encounter(world: &World, res: &Resources) -> Option<usize> {
    let tick = res.clock.tick_counter;

    // Gate 1: tick cadence.
    if tick & 31 != 0 {
        return None;
    }

    // Gate 2: no active enemies on screen (CollisionSystem sets battleflag).
    if res.region.battleflag {
        return None;
    }

    // Gate 3: enemy cap — at most 3 active enemies already.
    let active_count = count_active_enemies(world);
    if active_count >= 4 {
        return None;
    }

    // Gate 4: peace zone.
    let xtype = res.region.xtype as u8;
    if xtype >= 50 {
        return None;
    }

    // Gate 5: not riding a carrier.
    if res.carrier_entity.is_some() {
        return None;
    }

    // Gate 6: danger roll.
    let threshold = if res.region.region_num <= 7 {
        2 + xtype as u32
    } else {
        5 + xtype as u32
    };
    if rand64(tick) > threshold {
        return None;
    }

    Some(select_encounter_type(xtype, tick))
}

/// Count active (living) enemy entities.
fn count_active_enemies(world: &World) -> usize {
    world
        .query::<(&Enemy, &Health)>()
        .iter()
        .filter(|(_, hp)| !hp.is_dead())
        .count()
}

/// Select the encounter type index, applying zone-specific overrides.
fn select_encounter_type(xtype: u8, tick: u32) -> usize {
    // Forced overrides first.
    if xtype == 49 {
        return 2; // wraith
    }
    if xtype == 8 {
        return 6; // spider
    }

    // Base random selection: 0–3.
    let mut enc = combat_rand4(tick) as usize;

    // Swamp: wraith → snake.
    if xtype == 7 && enc == 2 {
        enc = 4; // snake
    }

    enc
}

/// Spawn up to 4 enemies of `encounter_type` around the hero.
/// Returns the number of enemies actually spawned.
fn spawn_encounter_group(world: &mut World, res: &Resources, encounter_type: usize) -> usize {
    // Race-guard — should already be < 4 from try_trigger_encounter.
    if count_active_enemies(world) >= 4 {
        return 0;
    }

    let tables = &res.encounter_tables;
    let stats = &tables.chart[encounter_type];
    let npc_type = tables.npc_type_map[encounter_type];
    let tick = res.clock.tick_counter;

    let hero_pos = hero_position(world, res);
    let origin = random_origin(hero_pos, tick);
    let mut occupied = occupied_positions(world, res, hero_pos);
    let world_data = res.map.world.as_ref();

    let mut spawned = 0usize;

    for i in 0..4 {
        let seed = tick.wrapping_add(i as u32 * 31);
        if let Some(pos) = find_spawn_position(origin, &occupied, seed, world_data) {
            let weapon =
                tables.weapon_probs[(stats.arms as usize * 4) + (combat_rand4(seed) as usize)];

            let race = match encounter_type {
                2 => RACE_WRAITH,
                3 | 5 => RACE_UNDEAD,
                4 => RACE_SNAKE,
                _ => RACE_ENEMY,
            };

            let entity = spawn_enemy(
                world,
                pos.0,
                pos.1,
                npc_type,
                race,
                stats.hp,
                weapon,
                stats.treasure as i16,
                2, // speed: original fmain.c set_encounter() default
                stats.clever,
                stats.cfile,
            );

            // Initial goal based on weapon class and cleverness (SPEC §11.7).
            let goal = if weapon & 4 != 0 {
                if stats.clever > 0 {
                    Goal::Archer2
                } else {
                    Goal::Archer1
                }
            } else {
                if stats.clever > 0 {
                    Goal::Attack2
                } else {
                    Goal::Attack1
                }
            };
            if let Ok(mut ai) = world.get::<&mut AiState>(entity) {
                ai.goal = goal;
            }

            occupied.push(pos);
            spawned += 1;
        }
    }

    spawned
}

fn hero_position(world: &World, res: &Resources) -> (f32, f32) {
    world
        .get::<&Position>(res.hero_entity)
        .map(|p| (p.x, p.y))
        .unwrap_or((0.0, 0.0))
}

/// Compute encounter origin point 150–213 pixels from the hero in a random direction.
/// Mirrors the original `set_loc()` using 8 compass directions.
fn random_origin(hero: (f32, f32), tick: u32) -> (f32, f32) {
    let dir = rand8(tick) as usize;
    let dist = 150 + rand64(tick);
    let ox = hero.0 as i32 + (XDIR[dir] * dist as i32) / 2;
    let oy = hero.1 as i32 + (YDIR[dir] * dist as i32) / 2;
    (ox as f32, oy as f32)
}

fn occupied_positions(world: &World, _res: &Resources, hero_pos: (f32, f32)) -> Vec<(f32, f32)> {
    let mut out = vec![hero_pos];
    for (_, _hp, pos) in world
        .query::<(&Enemy, &Health, &Position)>()
        .iter()
        .filter(|(_, hp, _)| !hp.is_dead())
    {
        out.push((pos.x, pos.y));
    }
    out
}

/// Try up to 15 scatter positions around `origin` (±31 px).
/// Accept the first position that is on open terrain (proxcheck) and does not
/// collide with any occupied slot and is inside the world bounds.
/// Mirrors set_encounter (fmain.c:2742-2747).
fn find_spawn_position(
    origin: (f32, f32),
    occupied: &[(f32, f32)],
    seed: u32,
    world_data: Option<&crate::game::world_data::WorldData>,
) -> Option<(f32, f32)> {
    const MAX_TRY: u32 = 15;
    const SCATTER: i32 = 31;
    let scatter_range = SCATTER * 2 + 1;

    let others: Vec<(i32, i32)> = occupied
        .iter()
        .map(|&(ox, oy)| (ox as i32, oy as i32))
        .collect();

    for j in 0..MAX_TRY {
        let sub_seed = seed.wrapping_add(j * 7);
        let dx = (rand64(sub_seed) as i32 % scatter_range) - SCATTER;
        let dy = (rand64(sub_seed.wrapping_add(13)) as i32 % scatter_range) - SCATTER;
        let pos = (origin.0 + dx as f32, origin.1 + dy as f32);

        if pos.0 < 0.0 || pos.0 > WORLD_MAX as f32 || pos.1 < 0.0 || pos.1 > WORLD_MAX as f32 {
            continue;
        }

        let px = pos.0 as i32;
        let py = pos.1 as i32;

        // set_encounter (fmain.c:2745): reject blocked terrain before actor check.
        if !proxcheck(world_data, px, py) {
            continue;
        }

        if !actor_collides(px, py, &others) {
            return Some(pos);
        }
    }

    None
}

/// Pseudo-random 0–63 from a tick seed, matching the original `rand64()` range.
fn rand64(tick: u32) -> u32 {
    let h = tick.wrapping_mul(1664525).wrapping_add(1013904223);
    (h >> 10) & 63
}

/// Pseudo-random 0–7 from a tick seed, matching the original `rand8()` range.
fn rand8(tick: u32) -> u32 {
    let h = tick.wrapping_mul(1103515245).wrapping_add(12345);
    (h >> 13) & 7
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ecs::components::{Enemy, Health};
    use crate::game::ecs::resources::EncounterTables;

    mod test_fixtures {
        use crate::game::ecs::components::Hero;
        use crate::game::ecs::resources::Resources;
        use hecs::World;

        pub fn minimal() -> (World, Resources) {
            let mut world = World::new();
            let hero = world.spawn((Hero,));
            let res = Resources::new(hero);
            (world, res)
        }
    }

    #[test]
    fn no_spawn_in_peace_zone() {
        let (world, mut res) = test_fixtures::minimal();
        res.clock.tick_counter = 0;
        res.region.xtype = 50;
        res.carrier_entity = None;
        assert!(try_trigger_encounter(&world, &res).is_none());
    }

    #[test]
    fn no_spawn_on_non_gated_tick() {
        let (world, mut res) = test_fixtures::minimal();
        res.clock.tick_counter = 1;
        res.region.xtype = 10;
        res.carrier_entity = None;
        assert!(try_trigger_encounter(&world, &res).is_none());
    }

    #[test]
    fn no_spawn_while_riding_carrier() {
        let (mut world, mut res) = test_fixtures::minimal();
        res.clock.tick_counter = 0;
        res.region.xtype = 10;
        res.carrier_entity = Some(world.spawn(()));
        assert!(try_trigger_encounter(&world, &res).is_none());
    }

    #[test]
    fn enemy_cap_blocks_spawn() {
        let (mut world, mut res) = test_fixtures::minimal();
        for _ in 0..4 {
            world.spawn((Enemy, Health::new(10)));
        }
        res.clock.tick_counter = 0;
        res.region.xtype = 10;
        res.carrier_entity = None;
        assert!(try_trigger_encounter(&world, &res).is_none());
    }

    #[test]
    fn encounter_tables_default_is_valid() {
        let tables = EncounterTables::default();
        assert_eq!(tables.chart.len(), 11);
        assert_eq!(tables.npc_type_map.len(), 11);
        assert_eq!(tables.weapon_probs.len(), 32);
    }

    // ── find_spawn_position tests ─────────────────────────────────────────────

    #[test]
    fn spawn_finds_position_when_clear() {
        // No world map (terrain always passes), no occupied slots → must find a spot.
        let result = find_spawn_position((500.0, 500.0), &[], 42, None);
        assert!(result.is_some(), "should find a spawn position on clear terrain");
    }

    #[test]
    fn spawn_blocked_by_actors_returns_none() {
        // Fill every possible scatter position with an actor at the origin.
        // With bounding box |dx|<11, |dy|<9, an actor at the origin blocks any
        // candidate within ±10 pixels.  Scatter is ±31 so we can't block all 15
        // tries this way — but we can verify the function respects actor_collides
        // by placing blockers at every position the RNG will produce for seed 0.
        // Easier: use a spread of 0 by placing a blocker exactly at the origin.
        // Since scatter is ±31 the origin itself is reachable; if all 15 tries
        // happen to land within the bounding box the function returns None.
        // Instead test the no-actor-block path above and the actor-block path via
        // a large occupant grid that covers all scatter offsets within ±31.
        let mut blockers: Vec<(f32, f32)> = Vec::new();
        let origin = (500.0f32, 500.0f32);
        // Place blockers at every integer position within ±31 px (62*62 = 3844 positions).
        // actor_collides bounding box: |dx|<11, |dy|<9 — so every candidate within
        // the scatter box will be covered.
        for dx in -31i32..=31 {
            for dy in -31i32..=31 {
                blockers.push((origin.0 + dx as f32, origin.1 + dy as f32));
            }
        }
        let result = find_spawn_position(origin, &blockers, 42, None);
        assert!(result.is_none(), "should not spawn inside densely occupied area");
    }
}
