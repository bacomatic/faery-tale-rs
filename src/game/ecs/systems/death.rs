//! DeathSystem — detects hero death and emits BrotherDiedEvent.
//! Successor spawning and Bones entity creation are handled by ItemSystem
//! consuming BrotherDiedEvent.
//! Port of dying/goodfairy logic from gameplay_scene/scene_impl.rs.
//! See docs/spec/death-revival.md.

use hecs::World;
use crate::game::ecs::components::{HeroStats, Position, Inventory, BrotherKind};
use crate::game::ecs::resources::Resources;
use crate::game::ecs::events::BrotherDiedEvent;

pub fn run(world: &mut World, res: &mut Resources) {
    let (vitality, luck) = match world.get::<&HeroStats>(res.hero_entity) {
        Ok(s) => (s.vitality, s.luck),
        Err(_) => return,
    };

    if vitality > 0 { return; }

    // Already in dying sequence — goodfairy countdown.
    if res.encounter.dying {
        if res.encounter.goodfairy > 0 {
            res.encounter.goodfairy -= 1;
            return; // still counting down
        }
        // Goodfairy countdown expired — hero truly dead.
        res.encounter.dying = false;
    } else {
        // First death tick — check luck gate (SPEC §14.3).
        if !res.encounter.luck_gate_fired && luck > 0 {
            let roll = (res.clock.tick_counter.wrapping_mul(2654435761)) as i16;
            if roll.abs() % 100 < luck {
                // Luck saves the hero — restore 1 vitality.
                if let Ok(mut stats) = world.get::<&mut HeroStats>(res.hero_entity) {
                    stats.vitality = 1;
                }
                res.encounter.luck_gate_fired = true;
                return;
            }
        }
        // Start goodfairy countdown (SPEC §14.4: 60 ticks).
        res.encounter.dying = true;
        res.encounter.goodfairy = 60;
        return;
    }

    // Hero is confirmed dead — emit event.
    let pos = world.get::<&Position>(res.hero_entity).map(|p| *p).unwrap_or(Position::new(0.0, 0.0));
    let inv = world.get::<&Inventory>(res.hero_entity).map(|i| i.stuff).unwrap_or([0; 36]);
    let bid = world.get::<&BrotherKind>(res.hero_entity).map(|b| b.id).unwrap_or(0);

    res.events.brother.push(BrotherDiedEvent {
        brother_id: bid,
        x: pos.x,
        y: pos.y,
        stuff: inv,
    });

    res.encounter.luck_gate_fired = false;
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::*;
    use super::run;

    fn dead_hero_stats() -> HeroStats {
        HeroStats { vitality: 0, brave: 0, luck: 0, kind: 0,
                    wealth: 0, hunger: 0, fatigue: 0, gold: 0 }
    }

    fn alive_hero_stats() -> HeroStats {
        HeroStats { vitality: 10, brave: 0, luck: 0, kind: 0,
                    wealth: 0, hunger: 0, fatigue: 0, gold: 0 }
    }

    #[test]
    fn dead_hero_starts_goodfairy_countdown() {
        // First call with dead hero starts the 60-tick countdown, does NOT emit event yet.
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, dead_hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        run(&mut world, &mut res);
        assert!(res.encounter.dying, "Should start goodfairy countdown");
        assert_eq!(res.encounter.goodfairy, 60);
        assert!(res.events.brother.is_empty(), "Should not emit event yet");
    }

    #[test]
    fn dead_hero_emits_event_after_countdown() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, dead_hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        // Fast-forward: already in dying, goodfairy at 0
        res.encounter.dying = true;
        res.encounter.goodfairy = 0;
        run(&mut world, &mut res);
        assert!(!res.events.brother.is_empty(),
            "Should emit BrotherDiedEvent after countdown expires");
    }

    #[test]
    fn alive_hero_emits_no_death_event() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, alive_hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        run(&mut world, &mut res);
        assert!(res.events.brother.is_empty());
        assert!(!res.encounter.dying);
    }
}
