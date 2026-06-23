//! DeathSystem — hero death countdown, good-fairy rescue, and brother succession.
//!
//! State machine:
//!   vitality <= 0 → hero_dying_countdown=7 (dying animation)
//!   hero_dying_countdown 7→0 → dying=true, goodfairy=330
//!   goodfairy 330→1 → countdown; spawn/move fairy at goodfairy 119..20
//!   goodfairy==0 → do_revival(): fairy rescue or brother succession
//!
//! Fairy sprite remains visible until goodfairy reaches 0. Measured timing at
//! 15 Hz: ~211 ticks pre-fairy, ~100 ticks fairy visible; total ~330 ticks (~22s).
//!
//! See docs/spec/death-revival.md §20 and reference/logic/combat.md §checkdead.

use hecs::World;
use crate::game::ecs::components::{BrotherKind, GoodFairy, HeroStats, Inventory, Position};
use crate::game::ecs::events::BrotherDiedEvent;
use crate::game::ecs::resources::Resources;

pub fn run(world: &mut World, res: &mut Resources) {
    let vitality = match world.get::<&HeroStats>(res.hero_entity) {
        Ok(s) => s.vitality,
        Err(_) => return,
    };

    if vitality > 0 { return; }

    // Single entry point: vitality <= 0 → start death sequence.
    if !res.encounter.dying && res.encounter.hero_dying_countdown == 0 {
        if let Ok(mut stats) = world.get::<&mut HeroStats>(res.hero_entity) {
            stats.luck = stats.luck.saturating_sub(5);
        }
        res.encounter.hero_dying_countdown = 7;
        return;
    }

    // Phase 0: 7-tick dying animation (fmain.c:1718-1728; tactic 7→0).
    if res.encounter.hero_dying_countdown > 0 {
        res.encounter.hero_dying_countdown -= 1;
        if res.encounter.hero_dying_countdown == 0 {
            res.encounter.dying = true;
            res.encounter.goodfairy = 330;
        }
        return;
    }

    let goodfairy = res.encounter.goodfairy;

    if goodfairy > 1 {
        // Spawn fairy at the start of the flying phase (spec §20.2: goodfairy 119..1).
        // Fairy remains on screen until the fade-to-black covers her.
        if goodfairy == 119 {
            let hero_pos = world.get::<&Position>(res.hero_entity)
                .map(|p| *p).unwrap_or(Position::new(0.0, 0.0));
            let fx = hero_pos.x + goodfairy as f32 * 2.0 - 20.0;
            let entity = world.spawn((GoodFairy, Position::new(fx, hero_pos.y)));
            res.encounter.fairy_entity = Some(entity);
        }

        // Update fairy position while flying (goodfairy 119..20).
        // At goodfairy < 20 she has arrived; stop updating so she stays at hero_x + 20.
        if goodfairy >= 20 {
            if let Some(fe) = res.encounter.fairy_entity {
                if let Ok(hero_pos) = world.get::<&Position>(res.hero_entity).map(|p| *p) {
                    if let Ok(mut fp) = world.get::<&mut Position>(fe) {
                        fp.x = hero_pos.x + goodfairy as f32 * 2.0 - 20.0;
                        fp.y = hero_pos.y;
                    }
                }
            }
        }

        res.encounter.goodfairy -= 1;
        return;
    }

    if goodfairy == 1 {
        res.encounter.goodfairy = 0;
        if let Some(fe) = res.encounter.fairy_entity.take() {
            let _ = world.despawn(fe);
        }
        do_revival(world, res);
    }
}

fn do_revival(world: &mut World, res: &mut Resources) {
    let luck = world.get::<&HeroStats>(res.hero_entity)
        .map(|s| s.luck).unwrap_or(0);

    res.encounter.dying = false;
    res.encounter.luck_gate_fired = false;

    if luck >= 1 {
        // Fairy rescue: restore hero stats and warp to last safe position.
        if let Ok(mut stats) = world.get::<&mut HeroStats>(res.hero_entity) {
            stats.vitality = 15 + stats.brave / 4;
            stats.hunger   = 0;
            stats.fatigue  = 0;
        }
        let (sx, sy) = res.encounter.safe_pos;
        let safe_r   = res.encounter.safe_r;
        res.pending_transition = Some(crate::game::ecs::events::RegionTransitionEvent {
            new_region: safe_r,
            dest_x:     sx,
            dest_y:     sy,
        });
        res.clock.daynight = 8000;
        res.palette.dirty = true;
    } else {
        // Brother succession: emit event; drain_brother_deaths() handles it.
        let pos = world.get::<&Position>(res.hero_entity).map(|p| *p)
            .unwrap_or(Position::new(0.0, 0.0));
        let inv = world.get::<&Inventory>(res.hero_entity).map(|i| i.stuff)
            .unwrap_or([0; 36]);
        let bid = world.get::<&BrotherKind>(res.hero_entity).map(|b| b.id)
            .unwrap_or(0);
        res.events.brother.push(BrotherDiedEvent {
            brother_id: bid,
            x: pos.x,
            y: pos.y,
            stuff: inv,
        });
    }
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
    fn dead_hero_starts_goodfairy_after_dying_animation() {
        // damage.rs sets hero_dying_countdown=7 when vitality first drops to 0.
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, dead_hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.encounter.hero_dying_countdown = 7;

        // 7 ticks of dying animation; goodfairy does not start yet.
        for _ in 0..7 {
            run(&mut world, &mut res);
            assert!(res.events.brother.is_empty(), "No event during dying animation");
        }
        // After 7 ticks the countdown reaches 0 and goodfairy starts.
        assert!(res.encounter.dying);
        assert_eq!(res.encounter.goodfairy, 330);
    }

    #[test]
    fn dead_hero_emits_event_at_goodfairy_1() {
        // goodfairy reaches 1 → do_revival() fires immediately (luck==0 → brother succession).
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, dead_hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.encounter.dying = true;
        res.encounter.goodfairy = 1;
        run(&mut world, &mut res);
        assert!(!res.events.brother.is_empty(),
            "Should emit BrotherDiedEvent at goodfairy==1 with luck < 1");
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

    #[test]
    fn fairy_rescue_with_luck() {
        let mut world = World::new();
        let mut stats = dead_hero_stats();
        stats.luck = 5; // luck >= 1 → fairy rescue
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, stats, Inventory::empty());
        let mut res = Resources::new(hero);
        res.encounter.dying = true;
        res.encounter.goodfairy = 1;
        run(&mut world, &mut res);
        assert!(res.events.brother.is_empty(), "Fairy rescue should not emit BrotherDiedEvent");
        assert!(!res.encounter.dying);
        assert!(res.pending_transition.is_some(), "Fairy rescue should trigger region transition");
    }

    #[test]
    fn fairy_despawns_at_goodfairy_1() {
        let mut world = World::new();
        let mut stats = dead_hero_stats();
        stats.luck = 5;
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, stats, Inventory::empty());
        let mut res = Resources::new(hero);
        res.encounter.dying = true;
        let fe = world.spawn((GoodFairy, Position::new(50.0, 100.0)));
        res.encounter.fairy_entity = Some(fe);
        res.encounter.goodfairy = 1;
        run(&mut world, &mut res);
        assert!(res.encounter.fairy_entity.is_none(), "Fairy entity handle should be cleared");
        assert!(world.get::<&GoodFairy>(fe).is_err(), "Fairy entity should be despawned from world");
    }

    #[test]
    fn fairy_sprite_lifecycle_matches_reference() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, dead_hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.encounter.dying = true;

        // Spec §20.2: fairy spawns when goodfairy reaches 119, stays until fade covers her.
        res.encounter.goodfairy = 120;
        run(&mut world, &mut res);
        assert!(res.encounter.fairy_entity.is_none(), "Fairy should not appear until goodfairy reaches 119");
        assert_eq!(res.encounter.goodfairy, 119);

        res.encounter.goodfairy = 119;
        run(&mut world, &mut res);
        let fe = res.encounter.fairy_entity.expect("Fairy should spawn at goodfairy 119");
        {
            let pos = world.get::<&Position>(fe).unwrap();
            assert_eq!(pos.x, 100.0 + 119.0 * 2.0 - 20.0);
        }
        assert_eq!(res.encounter.goodfairy, 118);

        // At goodfairy 20 she has arrived: position locked at hero_x + 20.
        res.encounter.goodfairy = 20;
        run(&mut world, &mut res);
        {
            let fe = res.encounter.fairy_entity.unwrap();
            let pos = world.get::<&Position>(fe).unwrap();
            assert_eq!(pos.x, 100.0 + 20.0 * 2.0 - 20.0); // hero_x + 20
        }
        assert_eq!(res.encounter.goodfairy, 19);

        // Fairy remains with position frozen through goodfairy 19..1 (no updates, no despawn).
        res.encounter.goodfairy = 19;
        run(&mut world, &mut res);
        {
            let fe = res.encounter.fairy_entity.unwrap();
            let pos = world.get::<&Position>(fe).unwrap();
            assert_eq!(pos.x, 100.0 + 20.0 * 2.0 - 20.0, "Position should not change after fairy arrives");
        }
        assert_eq!(res.encounter.goodfairy, 18);

        res.encounter.goodfairy = 2;
        run(&mut world, &mut res);
        assert!(res.encounter.fairy_entity.is_some(), "Fairy should still be present at goodfairy 2");
        assert_eq!(res.encounter.goodfairy, 1);
    }
}
