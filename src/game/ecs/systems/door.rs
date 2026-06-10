//! DoorSystem — detects hero collision with door rectangles and emits region transitions.
//! Port of door bump/walk-through logic from gameplay_scene (input.rs lines 460–511).
//! See docs/spec/doors-buildings.md.

use hecs::World;
use crate::game::ecs::components::{CarrierMount, Inventory, Position};
use crate::game::ecs::resources::Resources;

pub fn run(world: &World, res: &mut Resources, _game_lib: &crate::game::game_library::GameLibrary) {
    // fmain.c:1859 — no door use while mounted on a carrier (raft/turtle/swan/dragon).
    if let Ok(mount) = world.get::<&CarrierMount>(res.hero_entity) {
        if mount.riding != 0 {
            return;
        }
    }

    // 1. Get hero position
    let hero_pos = match world.get::<&Position>(res.hero_entity) {
        Ok(p) => *p,
        Err(_) => return,
    };
    let hero_x = hero_pos.x as u16;
    let hero_y = hero_pos.y as u16;

    // 2. Choose doorfind_binary() vs doorfind_exit() based on res.region.region_num
    let region_num = res.region.region_num;
    let hit = if region_num < 8 {
        // outdoor: binary search by source coords (doorlist sorted by src_x)
        crate::game::doors::doorfind_binary(&res.map.doors, region_num, hero_x, hero_y)
    } else {
        // indoor: find by destination coords (exit)
        crate::game::doors::doorfind_exit(&res.map.doors, hero_x, hero_y)
    };

    let door = match hit {
        Some(d) => d,
        None => return,
    };

    // fmain.c:1881 — DESERT oasis gates require 5 gold statues (stuff[25] >= 5).
    if door.door_type == crate::game::doors::DESERT {
        let has_statues = world.get::<&Inventory>(res.hero_entity)
            .map(|inv| inv.stuff[25] >= 5)
            .unwrap_or(false);
        if !has_statues {
            return;
        }
    }

    // 2b. Sub-tile position guard for outdoor entry (doorfind "caller responsibility").
    // Ensures transition fires only after the hero has walked past the bump point:
    //   Horizontal (type & 1): hero must be in upper half of 32px cell (y & 0x10 == 0).
    //   Vertical             : hero must be in left portion of 16px cell (x & 15 <= 6).
    // doorfind_exit already applies its own guard for the indoor-exit path.
    if region_num < 8 {
        if door.door_type & 1 != 0 {
            if hero_y & 0x10 != 0 { return; }
        } else if hero_x & 15 > 6 {
            return;
        }
    }

    // 3. Find the index of this door in res.map.doors to deduplicate
    let idx = match res.map.doors.iter().position(|d| {
        d.src_region == door.src_region
            && d.src_x == door.src_x
            && d.src_y == door.src_y
    }) {
        Some(i) => i,
        None => return,
    };

    // 4. Skip if transition already emitted for this door
    if res.map.transitioned_doors.contains(&idx) {
        return;
    }
    res.map.transitioned_doors.insert(idx);

    // 5. Compute spawn position and destination region.
    // Entering (outdoor→indoor): spawn at dst coords, go to dst_region.
    // Exiting  (indoor→outdoor): spawn at src coords, go to src_region.
    let (new_region, spawn_x, spawn_y) = if region_num < 8 {
        let (x, y) = crate::game::doors::entry_spawn(&door);
        (door.dst_region, x, y)
    } else {
        let (x, y) = crate::game::doors::exit_spawn(&door);
        (door.src_region, x, y)
    };

    // 6. Emit region transition event
    res.events.region.push(crate::game::ecs::events::RegionTransitionEvent {
        new_region,
        dest_x: spawn_x as f32,
        dest_y: spawn_y as f32,
    });
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::spawn_hero;
    use crate::game::game_library::load_game_library;
    use crate::game::doors::DoorEntry;
    use super::run;

    fn hero_stats() -> HeroStats {
        HeroStats { vitality: 100, brave: 0, luck: 0, kind: 0,
                    wealth: 0, hunger: 0, fatigue: 0, gold: 0 }
    }

    // A VWOOD (vertical, type=2) door at outdoor src (0x60, 0x60) → indoor dst (0x0bd0, 0x84c0).
    // doorfind matches on xtest==src_x and ytest==src_y, so hero at exactly (0x60, 0x60) hits it.
    fn make_outdoor_door() -> DoorEntry {
        DoorEntry {
            src_region: 0,
            src_x: 0x60,
            src_y: 0x60,
            dst_region: 8,
            dst_x: 0x0bd0,
            dst_y: 0x84c0,
            door_type: crate::game::doors::VWOOD,
        }
    }

    // A VWOOD door for the indoor (exit) path.
    // doorfind_exit matches dst coords: xtest==dst_x and ytest==dst_y with x&15 >= 2.
    // Hero at (0x0bd2, 0x84c0) → xtest=0x0bd0==dst_x, ytest=0x84c0==dst_y, x&15=2 >= 2 → match.
    fn make_indoor_door() -> DoorEntry {
        DoorEntry {
            src_region: 0,
            src_x: 0x60,
            src_y: 0x60,
            dst_region: 8,
            dst_x: 0x0bd0,
            dst_y: 0x84c0,
            door_type: crate::game::doors::VWOOD,
        }
    }

    #[test]
    fn door_system_no_panic_empty_world() {
        let game_lib = load_game_library(std::path::Path::new("faery.toml")).unwrap();
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        run(&world, &mut res, &game_lib);
        assert!(res.events.region.is_empty());
    }

    #[test]
    fn door_system_no_transition_without_doors() {
        let game_lib = load_game_library(std::path::Path::new("faery.toml")).unwrap();
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 5000.0, 5000.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        run(&world, &mut res, &game_lib);
        assert!(res.events.region.is_empty());
    }

    #[test]
    fn test_door_triggers_transition() {
        let game_lib = load_game_library(std::path::Path::new("faery.toml")).unwrap();
        let mut world = World::new();
        // Hero at exactly the door's src position — doorfind grid-aligns and matches.
        let hero = spawn_hero(&mut world, 0x60 as f32, 0x60 as f32, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.region.region_num = 0;
        res.map.doors.push(make_outdoor_door());
        run(&world, &mut res, &game_lib);
        assert_eq!(res.events.region.len(), 1, "expected one RegionTransitionEvent");
        assert_eq!(res.events.region[0].new_region, 8);
    }

    #[test]
    fn test_door_no_retrigger() {
        let game_lib = load_game_library(std::path::Path::new("faery.toml")).unwrap();
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 0x60 as f32, 0x60 as f32, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.region.region_num = 0;
        res.map.doors.push(make_outdoor_door());
        // Pre-mark door index 0 as transition already emitted.
        res.map.transitioned_doors.insert(0usize);
        run(&world, &mut res, &game_lib);
        assert!(res.events.region.is_empty(), "no event for already-transitioned door");
    }

    #[test]
    fn test_bump_open_then_walk_through_fires_transition() {
        // Primary regression: hero bumps door A (opened_doors gets idx=0),
        // then walks through it — transition must still fire.
        let game_lib = load_game_library(std::path::Path::new("faery.toml")).unwrap();
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 0x60 as f32, 0x60 as f32, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.region.region_num = 0;
        res.map.doors.push(make_outdoor_door());
        // Simulate movement system having performed the bump-open tile replacement.
        res.map.opened_doors.insert(0usize);
        // transitioned_doors is empty — hero has not yet walked through.
        run(&world, &mut res, &game_lib);
        assert_eq!(res.events.region.len(), 1, "transition must fire after bump-open when hero walks through");
        assert_eq!(res.events.region[0].new_region, 8);
    }

    #[test]
    fn test_subtile_guard_blocks_transition_at_bump_position() {
        // Hero is in the door's grid cell but at a position where the sub-tile guard says
        // "not yet walked through" — transition must NOT fire.
        // VWOOD (vertical, type=2): guard is "skip if hero_x & 15 > 6".
        // Hero at x=0x67 → x & 15 = 7 > 6 → guard blocks.
        let game_lib = load_game_library(std::path::Path::new("faery.toml")).unwrap();
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 0x67 as f32, 0x60 as f32, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.region.region_num = 0;
        res.map.doors.push(make_outdoor_door()); // VWOOD door at src (0x60, 0x60)
        run(&world, &mut res, &game_lib);
        assert!(res.events.region.is_empty(), "vertical door: transition must not fire when x & 15 > 6");
    }

    #[test]
    fn test_no_doors_no_panic() {
        let game_lib = load_game_library(std::path::Path::new("faery.toml")).unwrap();
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 0x60 as f32, 0x60 as f32, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.region.region_num = 0;
        // res.map.doors is empty by default
        run(&world, &mut res, &game_lib);
        assert!(res.events.region.is_empty());
        assert!(res.events.sfx.is_empty());
    }

    fn make_desert_door() -> DoorEntry {
        DoorEntry {
            src_region: 0,
            src_x: 0x60,
            src_y: 0x60,
            dst_region: 8,
            dst_x: 0x0bd0,
            dst_y: 0x84c0,
            door_type: crate::game::doors::DESERT,
        }
    }

    #[test]
    fn test_door_blocked_while_riding() {
        let game_lib = load_game_library(std::path::Path::new("faery.toml")).unwrap();
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 0x60 as f32, 0x60 as f32, 0, hero_stats(), Inventory::empty());
        // Mount hero on a carrier (riding != 0).
        world.insert_one(hero, CarrierMount { riding: 1, ..Default::default() }).unwrap();
        let mut res = Resources::new(hero);
        res.region.region_num = 0;
        res.map.doors.push(make_outdoor_door());
        run(&world, &mut res, &game_lib);
        assert!(res.events.region.is_empty(), "door transition must be blocked while riding");
    }

    #[test]
    fn test_desert_door_blocked_without_statues() {
        let game_lib = load_game_library(std::path::Path::new("faery.toml")).unwrap();
        let mut world = World::new();
        // Inventory with stuff[25] = 4 (< 5) — not enough gold statues.
        let mut inv = Inventory::empty();
        inv.stuff[25] = 4;
        let hero = spawn_hero(&mut world, 0x60 as f32, 0x60 as f32, 0, hero_stats(), inv);
        let mut res = Resources::new(hero);
        res.region.region_num = 0;
        res.map.doors.push(make_desert_door());
        run(&world, &mut res, &game_lib);
        assert!(res.events.region.is_empty(), "DESERT door must block without 5 gold statues");
    }

    #[test]
    fn test_desert_door_passes_with_statues() {
        let game_lib = load_game_library(std::path::Path::new("faery.toml")).unwrap();
        let mut world = World::new();
        // Inventory with stuff[25] = 5 — enough gold statues.
        let mut inv = Inventory::empty();
        inv.stuff[25] = 5;
        let hero = spawn_hero(&mut world, 0x60 as f32, 0x60 as f32, 0, hero_stats(), inv);
        let mut res = Resources::new(hero);
        res.region.region_num = 0;
        res.map.doors.push(make_desert_door());
        run(&world, &mut res, &game_lib);
        assert_eq!(res.events.region.len(), 1, "DESERT door must allow passage with 5 gold statues");
        assert_eq!(res.events.region[0].new_region, 8);
    }

    #[test]
    fn test_indoor_uses_doorfind_exit() {
        let game_lib = load_game_library(std::path::Path::new("faery.toml")).unwrap();
        let mut world = World::new();
        // Hero at (0x0bd2, 0x84c0): xtest=0x0bd0==dst_x, ytest=0x84c0==dst_y, x&15=2 >= 2 → match.
        let hero = spawn_hero(&mut world, 0x0bd2 as f32, 0x84c0 as f32, 8, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.region.region_num = 8; // indoor
        res.map.doors.push(make_indoor_door());
        run(&world, &mut res, &game_lib);
        assert_eq!(res.events.region.len(), 1, "expected RegionTransitionEvent on indoor exit");
        let door = make_indoor_door();
        let (ex, ey) = crate::game::doors::exit_spawn(&door);
        assert_eq!(res.events.region[0].new_region, door.src_region, "exit goes to src_region");
        assert_eq!(res.events.region[0].dest_x, ex as f32);
        assert_eq!(res.events.region[0].dest_y, ey as f32);
    }
}
