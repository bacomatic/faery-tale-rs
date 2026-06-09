//! MovementSystem — applies hero directional input to Position with collision.
//! Port of apply_player_input() from gameplay_scene/input.rs.
//! See docs/spec/movement-input.md.

use hecs::World;
use crate::game::direction::Direction;
use crate::game::ecs::components::{ActorMotion, Facing, Position};
use crate::game::ecs::resources::Resources;

pub fn run(world: &mut World, res: &mut Resources) {
    if res.clock.is_frozen() { return; }

    let dir = res.input_direction;

    // Clear moving flag when no input.
    if dir == Direction::None {
        if let Ok(mut motion) = world.get::<&mut ActorMotion>(res.hero_entity) {
            motion.moving = false;
        }
        return;
    }

    // xdir[]/ydir[] from fsubs.asm:1276 — cardinal magnitude 3, diagonal magnitude 2.
    // For open ground (environ=0) speed=2: step = (xdir[dir] * 2) >> 1 = xdir[dir].
    // Cardinal: ±3 px/tick. Diagonal: ±2 px/tick per axis (fsubs.asm:1276, movement.md).
    let (dx, dy) = dir.walk_step_open();

    let (old_x, old_y) = match world.get::<&Position>(res.hero_entity) {
        Ok(p) => (p.x, p.y),
        Err(_) => return,
    };

    let new_x = old_x + dx as f32;
    let new_y = old_y + dy as f32;

    let can_move = crate::game::collision::proxcheck(
        res.map.world.as_ref(),
        new_x as i32,
        new_y as i32,
    );

    if can_move {
        if let Ok(mut pos) = world.get::<&mut Position>(res.hero_entity) {
            pos.x = new_x;
            pos.y = new_y;
        }
        if let Ok(mut facing) = world.get::<&mut Facing>(res.hero_entity) {
            facing.dir = dir;
        }
        if let Ok(mut motion) = world.get::<&mut ActorMotion>(res.hero_entity) {
            motion.moving = true;
        }
        // Basic camera follow: keep hero centred at (144, 70) in the viewport.
        res.camera.map_x = (new_x - 144.0).max(0.0);
        res.camera.map_y = (new_y - 70.0).max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::direction::Direction;
    use crate::game::ecs::components::{Facing, Position};
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::spawn_hero;
    use crate::game::ecs::components::{HeroStats, Inventory};

    fn make_world_and_res() -> (World, Resources) {
        let mut world = World::new();
        let stats = HeroStats {
            vitality: 100, brave: 50, luck: 50, kind: 50,
            wealth: 50, hunger: 0, fatigue: 0, gold: 0,
        };
        let hero = spawn_hero(&mut world, 200.0, 200.0, 0, stats, Inventory::empty());
        let res = Resources::new(hero);
        (world, res)
    }

    #[test]
    fn hero_moves_north() {
        let (mut world, mut res) = make_world_and_res();
        res.input_direction = Direction::N;
        super::run(&mut world, &mut res);
        let pos = world.get::<&Position>(res.hero_entity).unwrap();
        assert_eq!(pos.y, 197.0, "cardinal N: 3px north");
        assert_eq!(pos.x, 200.0);
    }

    #[test]
    fn hero_moves_east() {
        let (mut world, mut res) = make_world_and_res();
        res.input_direction = Direction::E;
        super::run(&mut world, &mut res);
        let pos = world.get::<&Position>(res.hero_entity).unwrap();
        assert_eq!(pos.x, 203.0, "cardinal E: 3px east");
        assert_eq!(pos.y, 200.0);
    }

    #[test]
    fn hero_moves_diagonal_nw() {
        let (mut world, mut res) = make_world_and_res();
        res.input_direction = Direction::NW;
        super::run(&mut world, &mut res);
        let pos = world.get::<&Position>(res.hero_entity).unwrap();
        assert_eq!(pos.x, 198.0, "diagonal NW: 2px west");
        assert_eq!(pos.y, 198.0, "diagonal NW: 2px north");
    }

    #[test]
    fn moving_flag_set_on_move() {
        use crate::game::ecs::components::ActorMotion;
        let (mut world, mut res) = make_world_and_res();
        res.input_direction = Direction::N;
        super::run(&mut world, &mut res);
        let motion = world.get::<&ActorMotion>(res.hero_entity).unwrap();
        assert!(motion.moving, "moving flag should be true after successful move");
    }

    #[test]
    fn moving_flag_cleared_on_no_input() {
        use crate::game::ecs::components::ActorMotion;
        let (mut world, mut res) = make_world_and_res();
        // First set moving true by moving
        res.input_direction = Direction::N;
        super::run(&mut world, &mut res);
        // Then stop
        res.input_direction = Direction::None;
        super::run(&mut world, &mut res);
        let motion = world.get::<&ActorMotion>(res.hero_entity).unwrap();
        assert!(!motion.moving, "moving flag should be false when no input");
    }

    #[test]
    fn facing_updated_on_move() {
        let (mut world, mut res) = make_world_and_res();
        res.input_direction = Direction::S;
        super::run(&mut world, &mut res);
        let facing = world.get::<&Facing>(res.hero_entity).unwrap();
        assert_eq!(facing.dir, Direction::S);
    }

    #[test]
    fn no_move_when_frozen() {
        let (mut world, mut res) = make_world_and_res();
        res.clock.freeze_timer = 10;
        res.input_direction = Direction::N;
        super::run(&mut world, &mut res);
        let pos = world.get::<&Position>(res.hero_entity).unwrap();
        assert_eq!(pos.y, 200.0, "frozen clock should suppress movement");
    }

    #[test]
    fn no_move_when_direction_none() {
        let (mut world, mut res) = make_world_and_res();
        res.input_direction = Direction::None;
        super::run(&mut world, &mut res);
        let pos = world.get::<&Position>(res.hero_entity).unwrap();
        assert_eq!(pos.x, 200.0);
        assert_eq!(pos.y, 200.0);
    }

    #[test]
    fn movement_stub_compiles() {}
}
