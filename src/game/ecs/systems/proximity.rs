//! ProximitySystem — triggers auto-speech when hero approaches SetFig NPCs or enemy NPCs.
//! Port of update_proximity_speech() from gameplay_scene/proximity.rs.
//! See docs/spec/npcs-dialogue.md.

use hecs::{Entity, World};
use crate::game::collision::calc_dist;
use crate::game::ecs::components::{Enemy, EnemyKind, Position, SetFig, WorldObj};
use crate::game::ecs::events::SpeechEvent;
use crate::game::ecs::resources::Resources;
use crate::game::npc::{RACE_BEGGAR, RACE_NECROMANCER, RACE_WITCH};

/// Proximity range for auto-speech trigger (same unit as calc_dist).
const SPEECH_RANGE: i32 = 50;

const RACE_DREAM_KNIGHT: u8 = 7;

pub fn run(world: &World, res: &mut Resources) {
    let hero_pos = match world.get::<&Position>(res.hero_entity) {
        Ok(p) => *p,
        Err(_) => return,
    };
    let hx = hero_pos.x as i32;
    let hy = hero_pos.y as i32;

    // Find nearest eligible figure within SPEECH_RANGE using calc_dist.
    let mut best_dist = SPEECH_RANGE;
    let mut best: Option<(Entity, Option<SpeechEvent>)> = None;

    // Check enemy NPCs.
    for (entity, pos, kind) in world.query::<(Entity, &Position, &EnemyKind)>().with::<&Enemy>().iter() {
        let d = calc_dist(hx, hy, pos.x as i32, pos.y as i32);
        if d < best_dist {
            let speech_id = match kind.race {
                RACE_BEGGAR       => Some(23),
                RACE_WITCH        => Some(46),
                RACE_NECROMANCER  => Some(43),
                RACE_DREAM_KNIGHT => Some(41),
                _                 => None,
            };
            best_dist = d;
            best = Some((entity, speech_id.map(|id| SpeechEvent {
                speech_id: id,
                brother_name: String::new(), // filled by narrative system
            })));
        }
    }

    // Check SetFig world objects.
    for (entity, pos, obj) in world.query::<(Entity, &Position, &WorldObj)>().with::<&SetFig>().iter() {
        if !obj.visible {
            continue;
        }
        let d = calc_dist(hx, hy, pos.x as i32, pos.y as i32);
        if d < best_dist {
            let speech_id = match obj.ob_id {
                13 => Some(23), // Beggar
                9  => Some(46), // Witch
                4  => Some(16), // Princess (visible ↔ captive)
                _  => None,
            };
            best_dist = d;
            best = Some((entity, speech_id.map(|id| SpeechEvent {
                speech_id: id,
                brother_name: String::new(), // filled by narrative system
            })));
        }
    }

    match best {
        None => {
            res.last_speech_entity = None;
        }
        Some((entity, speech_ev)) => {
            if res.last_speech_entity == Some(entity) {
                return; // same figure as last tick — no repeat
            }
            res.last_speech_entity = Some(entity);
            if let Some(ev) = speech_ev {
                res.events.speech.push(ev);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::*;
    use super::run;

    fn hero_stats() -> HeroStats {
        HeroStats { vitality: 100, brave: 0, luck: 0, kind: 0,
                    wealth: 0, hunger: 0, fatigue: 0, gold: 0 }
    }

    fn make_obj(ob_id: u8, visible: bool) -> WorldObj {
        WorldObj { ob_id, ob_stat: 3, region: 0, visible, goal: ob_id }
    }

    #[test]
    fn nearby_setfig_beggar_triggers_speech() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        spawn_setfig(&mut world, 110.0, 100.0, make_obj(13, true), 0);
        run(&world, &mut res);
        assert!(!res.events.speech.is_empty(), "Nearby beggar setfig should trigger speech");
        assert_eq!(res.events.speech[0].speech_id, 23);
    }

    #[test]
    fn far_setfig_no_speech() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 0.0, 0.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        spawn_setfig(&mut world, 500.0, 500.0, make_obj(13, true), 0);
        run(&world, &mut res);
        assert!(res.events.speech.is_empty(), "Distant setfig should not trigger speech");
    }

    #[test]
    fn invisible_setfig_no_speech() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        spawn_setfig(&mut world, 101.0, 100.0, make_obj(13, false), 0);
        run(&world, &mut res);
        assert!(res.events.speech.is_empty(), "Invisible setfig should not trigger speech");
    }

    #[test]
    fn setfig_unknown_type_no_speech() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        // ob_id=1 has no speech mapping
        spawn_setfig(&mut world, 110.0, 100.0, make_obj(1, true), 0);
        run(&world, &mut res);
        // Entity is found (last_speech_entity set) but no SpeechEvent emitted
        assert!(res.events.speech.is_empty(), "Setfig with no speech mapping should not emit event");
        assert!(res.last_speech_entity.is_some(), "last_speech_entity should be set to prevent re-trigger");
    }

    #[test]
    fn dedup_same_entity_no_repeat() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        spawn_setfig(&mut world, 110.0, 100.0, make_obj(13, true), 0);
        run(&world, &mut res);
        assert_eq!(res.events.speech.len(), 1);
        res.events.speech.clear();
        run(&world, &mut res);
        assert!(res.events.speech.is_empty(), "Same entity must not trigger speech twice");
    }

    #[test]
    fn princess_visible_triggers_speech_16() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        spawn_setfig(&mut world, 110.0, 100.0, make_obj(4, true), 0);
        run(&world, &mut res);
        assert!(!res.events.speech.is_empty());
        assert_eq!(res.events.speech[0].speech_id, 16);
    }
}
