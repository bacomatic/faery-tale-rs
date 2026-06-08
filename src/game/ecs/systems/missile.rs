//! MissileSystem — advances missile positions and checks for hits.
//! Port of the missile tick section from gameplay_scene/scene_impl.rs (Phase 16).
//! See docs/spec/combat.md.

use hecs::World;
use crate::game::ecs::components::{Missile, MissileMotion, MissileKind, Position, Enemy};
use crate::game::ecs::resources::Resources;
use crate::game::ecs::events::DamageEvent;
use crate::game::combat::{MissileType, melee_rand};

/// Maximum missile flight time in ticks (fmain.c: missile dies after 40 ticks).
const MAX_FLIGHT_TICKS: u8 = 40;
/// World boundary (inclusive, matching scene_impl.rs: x > 32768 → deactivate).
const WORLD_MAX: i32 = 32768;

pub fn run(world: &mut World, res: &mut Resources) {
    // Collect all missile entities first to avoid borrow conflicts.
    let missiles: Vec<hecs::Entity> = world
        .query::<(hecs::Entity, &Missile)>()
        .iter()
        .map(|(e, _)| e)
        .collect();

    // Snapshot hero position.
    let hero_pos = world.get::<&Position>(res.hero_entity).ok().map(|p| *p);

    // Snapshot enemy positions for hit detection.
    let enemies: Vec<(hecs::Entity, Position)> = world
        .query::<(hecs::Entity, &Position)>()
        .with::<&Enemy>()
        .iter()
        .map(|(e, p)| (e, *p))
        .collect();

    let mut to_despawn: Vec<hecs::Entity> = Vec::new();

    for entity in missiles {
        // Read current missile state.
        let (motion, kind, pos) = {
            let mut q = world.query_one::<(&MissileMotion, &MissileKind, &Position)>(entity);
            match q.get() {
                Ok((m, k, p)) => (*m, *k, *p),
                Err(_) => continue,
            }
        };

        // Age expiry: missile dies after 40 ticks (fmain.c:2274).
        if motion.time_of_flight > MAX_FLIGHT_TICKS {
            to_despawn.push(entity);
            continue;
        }

        // Advance time_of_flight then position (matches scene_impl.rs order).
        let new_tof = motion.time_of_flight.saturating_add(1);
        let new_x = pos.x + motion.dx;
        let new_y = pos.y + motion.dy;

        // Out-of-bounds check (integer comparison matching scene_impl.rs).
        let ix = new_x as i32;
        let iy = new_y as i32;
        if ix < 0 || ix > WORLD_MAX || iy < 0 || iy > WORLD_MAX {
            to_despawn.push(entity);
            continue;
        }

        // Update position and time_of_flight.
        if let Ok(mut p) = world.get::<&mut Position>(entity) {
            p.x = new_x;
            p.y = new_y;
        }
        if let Ok(mut m) = world.get::<&mut MissileMotion>(entity) {
            m.time_of_flight = new_tof;
        }

        // Hit radius per SPEC §10.4 (Chebyshev distance).
        let radius = match kind.missile_type {
            MissileType::Arrow    => 6,
            MissileType::Fireball => 9,
        };

        if kind.is_friendly {
            // Friendly missile: check hits on enemies.
            for (enemy_ent, enemy_pos) in &enemies {
                let dx = (new_x - enemy_pos.x).abs() as i32;
                let dy = (new_y - enemy_pos.y).abs() as i32;
                if dx.max(dy) < radius {
                    let damage = (melee_rand(8) as i16) + 4;
                    res.events.damage.push(DamageEvent {
                        target: *enemy_ent,
                        amount: damage,
                        weapon: 0,
                        is_friendly_fire: false,
                    });
                    to_despawn.push(entity);
                    break;
                }
            }
        } else if let Some(hpos) = hero_pos {
            // Enemy missile: check hit on hero.
            let dx = (new_x - hpos.x).abs() as i32;
            let dy = (new_y - hpos.y).abs() as i32;
            if dx.max(dy) < radius {
                let damage = (melee_rand(8) as i16) + 4;
                res.events.damage.push(DamageEvent {
                    target: res.hero_entity,
                    amount: damage,
                    weapon: 0,
                    is_friendly_fire: false,
                });
                to_despawn.push(entity);
            }
        }
    }

    // Despawn expired/hit missiles.
    for entity in to_despawn {
        let _ = world.despawn(entity);
    }
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::{HeroStats, Inventory, Position};
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::{spawn_hero, spawn_enemy, spawn_missile};
    use crate::game::combat::MissileType;
    use super::run;

    fn hero_stats() -> HeroStats {
        HeroStats { vitality: 100, brave: 0, luck: 0, kind: 0,
                    wealth: 0, hunger: 0, fatigue: 0, gold: 0 }
    }

    #[test]
    fn missile_advances_position() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 500.0, 500.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        let m = spawn_missile(&mut world, 100.0, 100.0, 3.0, 0.0, 0, MissileType::Arrow, true);
        run(&mut world, &mut res);
        let pos = *world.get::<&Position>(m).unwrap();
        assert_eq!(pos.x, 103.0, "Missile should advance by dx=3");
    }

    #[test]
    fn missile_despawns_after_max_ticks() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 500.0, 500.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        // tof=40: condition is `> 40`, so this missile has one more tick.
        // tof=41 triggers despawn immediately.
        let m = spawn_missile(&mut world, 100.0, 100.0, 1.0, 0.0, 41, MissileType::Arrow, true);
        run(&mut world, &mut res);
        assert!(!world.contains(m), "Missile should be despawned when tof > 40");
    }

    #[test]
    fn missile_despawns_at_boundary() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 500.0, 500.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        // Missile one step away from going out of bounds.
        let m = spawn_missile(&mut world, 32760.0, 100.0, 10.0, 0.0, 0, MissileType::Arrow, true);
        run(&mut world, &mut res);
        assert!(!world.contains(m), "Missile should be despawned when out of bounds");
    }

    #[test]
    fn friendly_missile_hits_enemy() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 500.0, 500.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        // Enemy at 110, 100 — missile at 100, 100 moving right by 4; new_x=104, Chebyshev dist=6, not < 6.
        // Place enemy at 108, 100 so dist = 4 < 6 → hit.
        let enemy = spawn_enemy(&mut world, 108.0, 100.0, 1, 0, 50, 0, 0, 3, 0, 0);
        let m = spawn_missile(&mut world, 100.0, 100.0, 4.0, 0.0, 0, MissileType::Arrow, true);
        run(&mut world, &mut res);
        assert!(!res.events.damage.is_empty(), "Friendly missile should hit enemy");
        assert_eq!(res.events.damage[0].target, enemy);
        assert!(!world.contains(m), "Missile should despawn on hit");
    }

    #[test]
    fn enemy_missile_hits_hero() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 200.0, 200.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        // Missile at 196, 196 moving +1,+1; new pos = 197,197 — Chebyshev dist = 3 < 6 → hit.
        let m = spawn_missile(&mut world, 196.0, 196.0, 1.0, 1.0, 0, MissileType::Arrow, false);
        run(&mut world, &mut res);
        assert!(!res.events.damage.is_empty(), "Enemy missile should hit hero");
        assert_eq!(res.events.damage[0].target, hero);
        assert!(!world.contains(m), "Missile should despawn on hit");
    }
}
