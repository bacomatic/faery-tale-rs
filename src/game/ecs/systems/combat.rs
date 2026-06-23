//! CombatSystem — melee hit detection and damage event emission.
//! Ports the dohit() / attack() battle loop from fmain.c.

use hecs::{Entity, World};
use crate::game::ecs::resources::Resources;
use crate::game::ecs::components::{
    Position, Facing, CombatState, HeroStats, Inventory,
    AiState, EnemyKind, Health,
};
use crate::game::npc::NpcState;
use crate::game::ecs::events::{DamageEvent, SpeechEvent};
use crate::game::actor::ActorState;
use crate::game::combat::{weapon_tip, combat_reach, bitrand};

pub fn run(world: &mut World, res: &mut Resources) {
    if res.clock.freeze_timer > 0 {
        return;
    }

    let hero = res.hero_entity;

    let fighting = world.get::<&CombatState>(hero)
        .map(|cs| matches!(cs.state, ActorState::Fighting(_)))
        .unwrap_or(false);

    if fighting {
        let (hx, hy, hfacing, weapon_code, brave) = {
            let mut q = world.query_one::<(&Position, &Facing, &CombatState, &HeroStats)>(hero);
            match q.get() {
                Ok((pos, face, cs, stats)) => (pos.x, pos.y, face.dir, cs.weapon, stats.brave),
                Err(_) => return,
            }
        };

        // fmain.c:2245 — touch attack (code >= 8) clamps wt to 5 before computing strike distance.
        let wt = if weapon_code >= 8 { 5i16 } else { weapon_code as i16 };
        let (sx_i, sy_i) = weapon_tip(hx as i32, hy as i32, hfacing, wt);
        let (sx, sy) = (sx_i as f32, sy_i as f32);
        let reach = combat_reach(true, brave, res.clock.tick_counter);

        let sun_stone = world.get::<&Inventory>(hero)
            .map(|inv| inv.stuff[7] > 0)
            .unwrap_or(false);

        // Only target living enemies — checkdead() guards "state != DYING && state != DEAD".
        let targets: Vec<(Entity, u8, f32, f32)> = world
            .query::<(Entity, &Health, &EnemyKind, &Position, &AiState)>()
            .iter()
            .filter(|(_, h, _, _, ai)| {
                h.vitality > 0
                    && !matches!(ai.state, NpcState::Dying | NpcState::Dead)
            })
            .map(|(e, _, ek, p, _)| (e, ek.race, p.x, p.y))
            .collect();

        let brother_name = res.brother.active_name.clone();

        for (entity, race, ex, ey) in targets {
            let dist = (sx - ex).abs().max((sy - ey).abs());
            if dist >= reach as f32 {
                continue;
            }

            if race == 0x8a || race == 0x8b {
                continue;
            }
            if race == 9 && weapon_code < 4 {
                res.events.speech.push(SpeechEvent {
                    speech_id: 58,
                    brother_name: brother_name.clone(),
                });
                continue;
            }
            if race == 0x89 && weapon_code < 4 && !sun_stone {
                res.events.speech.push(SpeechEvent {
                    speech_id: 58,
                    brother_name: brother_name.clone(),
                });
                continue;
            }

            // Touch attack (code >= 8): clamp wt to 5 before the bonus (fmain.c:2244).
            let amount = if weapon_code >= 8 {
                5 + bitrand(2) as i16
            } else {
                weapon_code as i16 + bitrand(2) as i16
            };

            res.events.damage.push(DamageEvent {
                target:           entity,
                amount,
                weapon:           weapon_code,
                is_friendly_fire: false,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hecs::World;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::components::{
        Position, Facing, CombatState, HeroStats, Inventory,
        AiState, EnemyKind, Health,
    };
    use crate::game::actor::ActorState;

    fn setup_hero_fighting(weapon: u8, brave: i16, hx: f32, hy: f32) -> (World, Resources) {
        let mut world = World::new();
        let hero = world.spawn((
            Position::new(hx, hy),
            Facing::default(),
            CombatState { state: ActorState::Fighting(0), weapon },
            HeroStats { vitality: 100, brave, luck: 0, kind: 0, wealth: 0, hunger: 0, fatigue: 0, gold: 0 },
            Inventory::empty(),
        ));
        let mut res = Resources::new(hero);
        res.clock.freeze_timer = 0;
        (world, res)
    }

    fn spawn_enemy(world: &mut World, race: u8, ex: f32, ey: f32, vitality: i16) -> Entity {
        world.spawn((
            Position::new(ex, ey),
            EnemyKind { npc_type: 0, race },
            Health { vitality },
            AiState::default(),
        ))
    }

    #[test]
    fn hero_fighting_enemy_in_range_emits_damage_event() {
        // brave=100 → reach=10; enemy at hero position → always in range.
        let (mut world, mut res) = setup_hero_fighting(2, 100, 100.0, 100.0);
        let enemy = spawn_enemy(&mut world, 5, 100.0, 100.0, 20);

        run(&mut world, &mut res);

        assert_eq!(res.events.damage.len(), 1, "expected exactly one DamageEvent");
        assert_eq!(res.events.damage[0].target, enemy);
        assert_eq!(res.events.damage[0].weapon, 2);
        assert!(!res.events.damage[0].is_friendly_fire);
    }

    #[test]
    fn spectre_is_immune_to_all_damage() {
        let (mut world, mut res) = setup_hero_fighting(5, 80, 100.0, 100.0);
        spawn_enemy(&mut world, 0x8a, 100.0, 100.0, 30);

        run(&mut world, &mut res);

        let spectre_events: Vec<_> = res.events.damage.iter()
            .filter(|ev| ev.target != res.hero_entity)
            .collect();
        assert!(spectre_events.is_empty(), "Spectre must not receive any DamageEvent");
        assert!(res.events.speech.is_empty(), "Spectre immunity is silent");
    }

    #[test]
    fn witch_immune_without_sun_stone_vulnerable_with_it() {
        // Without Sun Stone: immune, emits speech.
        {
            let (mut world, mut res) = setup_hero_fighting(1, 80, 100.0, 100.0);
            spawn_enemy(&mut world, 0x89, 100.0, 100.0, 30);

            run(&mut world, &mut res);

            let hits: Vec<_> = res.events.damage.iter()
                .filter(|ev| ev.target != res.hero_entity)
                .collect();
            assert!(hits.is_empty(), "Witch must be immune without Sun Stone");
            assert!(!res.events.speech.is_empty(), "Witch immunity emits speech event");
        }

        // With Sun Stone: vulnerable.
        {
            let (mut world, mut res) = setup_hero_fighting(1, 80, 100.0, 100.0);
            if let Ok(mut inv) = world.get::<&mut Inventory>(res.hero_entity) {
                inv.stuff[7] = 1;
            }
            let witch = spawn_enemy(&mut world, 0x89, 100.0, 100.0, 30);

            run(&mut world, &mut res);

            let hits: Vec<_> = res.events.damage.iter()
                .filter(|ev| ev.target == witch)
                .collect();
            assert!(!hits.is_empty(), "Witch must be vulnerable when hero has Sun Stone");
        }
    }
}
