//! DamageSystem — drains the DamageEvent queue and applies vitality reduction.
//! Triggers enemy death when vitality reaches zero.

use hecs::World;
use crate::game::ecs::resources::Resources;
use crate::game::ecs::components::{Health, AiState, EnemyKind, Position, HeroStats, Loot};
use crate::game::ecs::events::{DamageEvent, EnemyDiedEvent};
use crate::game::npc::NpcState;

/// Drain `res.events.damage`, reduce target vitality, and trigger death when
/// vitality reaches zero or below.
pub fn run(world: &mut World, res: &mut Resources) {
    let events: Vec<DamageEvent> = res.events.damage.drain(..).collect();
    for ev in events {
        apply_damage(world, res, ev);
    }
}

fn apply_damage(world: &mut World, res: &mut Resources, ev: DamageEvent) {
    if ev.target == res.hero_entity {
        if let Ok(mut stats) = world.get::<&mut HeroStats>(ev.target) {
            stats.vitality = (stats.vitality - ev.amount).max(0);
        }
        return;
    }

    let vitality_after = match world.get::<&mut Health>(ev.target) {
        Ok(mut h) => {
            h.vitality = (h.vitality - ev.amount).max(0);
            h.vitality
        }
        Err(_) => return,
    };

    // checkdead() guard (fmain.c:2769): only trigger death once — skip if already DYING or DEAD.
    if vitality_after <= 0 {
        let already_dying = world.get::<&AiState>(ev.target)
            .map(|ai| matches!(ai.state, NpcState::Dying | NpcState::Dead))
            .unwrap_or(false);
        if !already_dying {
            trigger_death(world, res, ev.target, ev.weapon);
        }
    }
}

fn trigger_death(world: &mut World, res: &mut Resources, entity: hecs::Entity, weapon: u8) {
    let race = world.get::<&EnemyKind>(entity).map(|ek| ek.race).unwrap_or(0);
    let (x, y) = world.get::<&Position>(entity).map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0));
    let gold = world.get::<&Loot>(entity).map(|l| l.gold).unwrap_or(0);

    if let Ok(mut ai) = world.get::<&mut AiState>(entity) {
        ai.state = NpcState::Dying;
    }

    // fmain.c:2777 — each enemy kill grants brave++ to the hero.
    if let Ok(mut stats) = world.get::<&mut HeroStats>(res.hero_entity) {
        stats.brave += 1;
    }

    res.events.died.push(EnemyDiedEvent {
        entity,
        race,
        weapon,
        gold,
        x,
        y,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use hecs::World;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::components::{Health, AiState, EnemyKind, Position, HeroStats};
    use crate::game::ecs::events::DamageEvent;
    use crate::game::npc::NpcState;

    #[test]
    fn damage_exceeds_vitality_triggers_death_event() {
        let mut world = World::new();
        let hero = world.spawn((
            HeroStats { vitality: 100, brave: 0, luck: 0, kind: 0, wealth: 0, hunger: 0, fatigue: 0, gold: 0 },
            Position::new(0.0, 0.0),
        ));
        let mut res = Resources::new(hero);

        let enemy = world.spawn((
            Health { vitality: 5 },
            EnemyKind { npc_type: 0, race: 3 },
            AiState::default(),
            Position::new(10.0, 10.0),
        ));

        res.events.damage.push(DamageEvent {
            target:           enemy,
            amount:           10,
            weapon:           2,
            is_friendly_fire: false,
        });

        run(&mut world, &mut res);

        let health = world.get::<&Health>(enemy).unwrap();
        assert_eq!(health.vitality, 0);

        assert_eq!(res.events.died.len(), 1);
        assert_eq!(res.events.died[0].entity, enemy);
        assert_eq!(res.events.died[0].race, 3);

        let ai = world.get::<&AiState>(enemy).unwrap();
        assert!(matches!(ai.state, NpcState::Dying));

        // fmain.c:2777 — hero brave increments on each enemy kill.
        let stats = world.get::<&HeroStats>(hero).unwrap();
        assert_eq!(stats.brave, 1, "brave should increment by 1 on enemy kill");
    }
}
