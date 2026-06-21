//! Helper functions to spawn canonical entity bundles into a `hecs::World`.
//!
//! Each function returns the `hecs::Entity` handle of the newly spawned entity.
//! Spawn functions are pure: they do not read or write any `Resources`.

use hecs::World;
use crate::game::direction::Direction;
use crate::game::actor::{Goal, Tactic};
use crate::game::npc::NpcState;
use crate::game::ecs::components::*;

/// Spawn the hero entity. Called once at game start or on brother succession.
///
/// `brother_id`: 0=Julian, 1=Phillip, 2=Kevin.
/// Starting stats come from `GameLibrary.brothers[brother_id]` before calling this.
pub fn spawn_hero(
    world: &mut World,
    x: f32,
    y: f32,
    brother_id: u8,
    stats: HeroStats,
    inventory: Inventory,
) -> hecs::Entity {
    world.spawn((
        Hero,
        Position::new(x, y),
        Facing::new(Direction::S),
        BrotherKind { id: brother_id },
        stats,
        inventory,
        ActorMotion::default(),
        CombatState::default(),
        CarrierMount::default(),
        FrustFlag::default(),
    ))
}

/// Spawn an enemy NPC entity.
pub fn spawn_enemy(
    world: &mut World,
    x: f32,
    y: f32,
    npc_type: u8,
    race: u8,
    vitality: i16,
    weapon: u8,
    gold: i16,
    speed: u8,
    cleverness: u8,
    cfile_idx: u8,
) -> hecs::Entity {
    world.spawn((
        Enemy,
        Position::new(x, y),
        Facing::new(Direction::N),
        EnemyKind { npc_type, race },
        AiState {
            goal: Goal::Attack1,
            tactic: Tactic::Pursue,
            state: NpcState::Still,
            cleverness,
        },
        Health::new(vitality),
        Speed { speed },
        Loot { weapon, gold, looted: false },
        SpriteRef { cfile_idx },
        ActorMotion::default(),
    ))
}

/// Spawn an arena training dummy (immortal, no AI, no loot).
pub fn spawn_arena_dummy(
    world: &mut World,
    x: f32,
    y: f32,
    npc_type: u8,
    vitality: i16,
    cfile_idx: u8,
) -> hecs::Entity {
    world.spawn((
        Enemy,
        ArenaDummy,
        Position::new(x, y),
        Facing::new(Direction::SE),
        EnemyKind { npc_type, race: 0 },
        AiState::default(),
        Health::new(vitality),
        Speed { speed: 0 },
        Loot::default(),
        SpriteRef { cfile_idx },
    ))
}

/// Spawn a stationary world NPC (setfig).
pub fn spawn_setfig(
    world: &mut World,
    x: f32,
    y: f32,
    obj: WorldObj,
    cfile_idx: u8,
) -> hecs::Entity {
    world.spawn((
        SetFig,
        Position::new(x, y),
        Facing::new(Direction::S),
        obj,
        SpriteRef { cfile_idx },
    ))
}

/// Spawn a ground item.
pub fn spawn_ground_item(
    world: &mut World,
    x: f32,
    y: f32,
    obj: WorldObj,
) -> hecs::Entity {
    world.spawn((
        GroundItem,
        Position::new(x, y),
        obj,
    ))
}

/// Spawn a Bones entity when a brother dies.
pub fn spawn_bones(
    world: &mut World,
    x: f32,
    y: f32,
    region: u8,
    brother_id: u8,
    stuff: [u8; 36],
) -> hecs::Entity {
    world.spawn((
        Bones,
        Position::new(x, y),
        BrotherKind { id: brother_id },
        Inventory { stuff },
        WorldObj {
            ob_id:   28, // BONES ob_id
            ob_stat: 1,
            region,
            visible: true,
            goal:    0,
        },
    ))
}

/// Spawn a missile.
pub fn spawn_missile(
    world: &mut World,
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    time_of_flight: u8,
    missile_type: crate::game::combat::MissileType,
    is_friendly: bool,
) -> hecs::Entity {
    world.spawn((
        Missile,
        Position::new(x, y),
        MissileMotion { dx, dy, time_of_flight },
        MissileKind { missile_type, is_friendly },
    ))
}

/// Spawn a carrier entity (raft, turtle, swan, dragon).
pub fn spawn_carrier(
    world: &mut World,
    x: f32,
    y: f32,
    kind: i16,
    cfile_idx: u8,
) -> hecs::Entity {
    world.spawn((
        Carrier,
        Position::new(x, y),
        Facing::new(Direction::S),
        CarrierKind { kind },
        SpriteRef { cfile_idx },
    ))
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hecs::World;
    use crate::game::combat::MissileType;

    fn make_stats() -> HeroStats {
        HeroStats { vitality: 100, brave: 50, luck: 50, kind: 50,
                    wealth: 50, hunger: 0, fatigue: 0, gold: 0 }
    }

    #[test]
    fn spawn_hero_has_required_components() {
        let mut world = World::new();
        let e = spawn_hero(&mut world, 100.0, 200.0, 0, make_stats(), Inventory::empty());
        assert!(world.get::<&Hero>(e).is_ok());
        assert!(world.get::<&Position>(e).is_ok());
        assert!(world.get::<&HeroStats>(e).is_ok());
        assert!(world.get::<&Inventory>(e).is_ok());
        assert_eq!(world.get::<&Position>(e).unwrap().x, 100.0);
        assert_eq!(world.get::<&BrotherKind>(e).unwrap().id, 0);
    }

    #[test]
    fn spawn_enemy_has_required_components() {
        let mut world = World::new();
        let e = spawn_enemy(&mut world, 50.0, 60.0, 1, 0, 20, 1, 5, 3, 5, 0);
        assert!(world.get::<&Enemy>(e).is_ok());
        assert!(world.get::<&Health>(e).is_ok());
        assert!(world.get::<&AiState>(e).is_ok());
        assert_eq!(world.get::<&Health>(e).unwrap().vitality, 20);
    }

    #[test]
    fn spawn_arena_dummy_has_arena_dummy_marker() {
        let mut world = World::new();
        let e = spawn_arena_dummy(&mut world, 0.0, 0.0, 1, 50, 0);
        assert!(world.get::<&ArenaDummy>(e).is_ok());
        assert!(world.get::<&Enemy>(e).is_ok());
    }

    #[test]
    fn spawn_bones_has_inventory_and_brother() {
        let mut world = World::new();
        let mut stuff = [0u8; 36];
        stuff[0] = 3;
        let e = spawn_bones(&mut world, 10.0, 20.0, 1, 0, stuff);
        assert!(world.get::<&Bones>(e).is_ok());
        assert_eq!(world.get::<&Inventory>(e).unwrap().stuff[0], 3);
        assert_eq!(world.get::<&BrotherKind>(e).unwrap().id, 0);
    }

    #[test]
    fn spawn_missile_position() {
        let mut world = World::new();
        let e = spawn_missile(&mut world, 5.0, 6.0, 1.0, 0.0, 10,
                              MissileType::Arrow, true);
        assert!(world.get::<&Missile>(e).is_ok());
        let pos = world.get::<&Position>(e).unwrap();
        assert_eq!(pos.x, 5.0);
        assert_eq!(pos.y, 6.0);
    }

    #[test]
    fn despawn_removes_entity() {
        let mut world = World::new();
        let e = spawn_ground_item(&mut world, 0.0, 0.0,
            WorldObj { ob_id: 5, ob_stat: 1, region: 0, visible: true, goal: 0 });
        assert!(world.contains(e));
        world.despawn(e).unwrap();
        assert!(!world.contains(e));
    }
}
