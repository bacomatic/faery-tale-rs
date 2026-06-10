//! DoorSystem — detects hero collision with door rectangles and emits region transitions.
//! Port of door bump/walk-through logic from gameplay_scene (input.rs lines 460–511).
//! See docs/spec/doors-buildings.md.

use hecs::World;
use crate::game::ecs::components::Position;
use crate::game::ecs::resources::Resources;

pub fn run(world: &World, res: &mut Resources, _game_lib: &crate::game::game_library::GameLibrary) {
    // 1. Get hero position
    let hero_pos = match world.get::<&Position>(res.hero_entity) {
        Ok(p) => *p,
        Err(_) => return,
    };
    let hero_x = hero_pos.x as u16;
    let hero_y = hero_pos.y as u16;

    // 2. Choose doorfind() vs doorfind_exit() based on res.region.region_num
    let region_num = res.region.region_num;
    let hit = if region_num < 8 {
        // outdoor: find by source coords
        crate::game::doors::doorfind(&res.map.doors, region_num, hero_x, hero_y)
    } else {
        // indoor: find by destination coords (exit)
        crate::game::doors::doorfind_exit(&res.map.doors, hero_x, hero_y)
    };

    let door = match hit {
        Some(d) => d,
        None => return,
    };

    // 3. Find the index of this door in res.map.doors to deduplicate
    let idx = match res.map.doors.iter().position(|d| {
        d.src_region == door.src_region
            && d.src_x == door.src_x
            && d.src_y == door.src_y
    }) {
        Some(i) => i,
        None => return,
    };

    // 4. Skip if already opened
    if res.map.opened_doors.contains(&idx) {
        return;
    }
    res.map.opened_doors.insert(idx);

    // 5. Compute spawn position
    let (spawn_x, spawn_y) = if region_num < 8 {
        crate::game::doors::entry_spawn(&door)
    } else {
        crate::game::doors::exit_spawn(&door)
    };

    // 6. Emit region transition event
    res.events.region.push(crate::game::ecs::events::RegionTransitionEvent {
        new_region: door.dst_region,
        dest_x: spawn_x as f32,
        dest_y: spawn_y as f32,
    });

    // 7. Emit door-open SFX
    res.events.sfx.push(crate::game::ecs::events::SfxEvent { sfx_id: 12 });
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
        // Pre-mark door index 0 as already opened.
        res.map.opened_doors.insert(0usize);
        run(&world, &mut res, &game_lib);
        assert!(res.events.region.is_empty(), "no event for already-opened door");
    }

    #[test]
    fn test_door_emits_sfx() {
        let game_lib = load_game_library(std::path::Path::new("faery.toml")).unwrap();
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 0x60 as f32, 0x60 as f32, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.region.region_num = 0;
        res.map.doors.push(make_outdoor_door());
        run(&world, &mut res, &game_lib);
        assert_eq!(res.events.sfx.len(), 1, "expected one SfxEvent");
        assert_eq!(res.events.sfx[0].sfx_id, 12);
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
        // exit_spawn on a VWOOD door: (src_x + 20, src_y + 16)
        let door = &make_indoor_door();
        let (ex, ey) = crate::game::doors::exit_spawn(door);
        assert_eq!(res.events.region[0].dest_x, ex as f32);
        assert_eq!(res.events.region[0].dest_y, ey as f32);
    }
}
