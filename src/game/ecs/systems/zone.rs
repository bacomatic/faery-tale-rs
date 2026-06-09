//! ZoneSystem — detects when the hero enters or exits encounter zones.
//! Port of zone check logic from gameplay_scene (scene_impl.rs lines 492–531).
//! See docs/spec/ai-encounters.md.

use hecs::World;
use crate::game::ecs::components::Position;
use crate::game::ecs::resources::Resources;
use crate::game::ecs::events::ZoneEvent;
use crate::game::game_library::ZoneConfig;
use crate::game::zones;

pub fn run(world: &World, res: &mut Resources) {
    let hero_pos = match world.get::<&Position>(res.hero_entity) {
        Ok(p) => *p,
        Err(_) => return,
    };

    // Zone rectangles populated on region load by EcsScene::load_world / reload_region.
    let zones_list: &[ZoneConfig] = &res.zones;

    let hx = hero_pos.x as u16;
    let hy = hero_pos.y as u16;

    // Update encounter zone flag.
    res.encounter.in_encounter_zone = zones::in_encounter_zone(zones_list, hx, hy);

    // Zone entry/exit detection — emit events when zone changes.
    let current_zone = zones::find_zone(zones_list, hx, hy);
    if current_zone != res.encounter.last_zone {
        // Emit exit event for previous zone.
        if let Some(prev_idx) = res.encounter.last_zone {
            res.events.zone.push(ZoneEvent::Exited { zone_idx: prev_idx });
        }
        // Emit entry event for new zone and update xtype.
        if let Some(zone_idx) = current_zone {
            if zone_idx < zones_list.len() {
                res.region.xtype = zones_list[zone_idx].etype as u16;
            }
            res.events.zone.push(ZoneEvent::Entered { zone_idx });
        }
        res.encounter.last_zone = current_zone;
    }
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::spawn_hero;
    use super::run;

    fn hero_stats() -> HeroStats {
        HeroStats { vitality: 100, brave: 0, luck: 0, kind: 0,
                    wealth: 0, hunger: 0, fatigue: 0, gold: 0 }
    }

    #[test]
    fn zone_system_no_panic_empty_world() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        run(&world, &mut res);
        assert!(res.events.zone.is_empty());
    }

    #[test]
    fn zone_system_no_encounter_when_no_zones() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 5000.0, 5000.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        run(&world, &mut res);
        assert!(!res.encounter.in_encounter_zone);
        assert_eq!(res.encounter.last_zone, None);
    }
}
