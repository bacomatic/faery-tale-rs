//! ItemSystem — processes item pickup, body search, and inventory mutations.
//! Port of handle_take_item() and search_body() from gameplay_scene/items.rs.
//! See docs/spec/inventory-items.md.
//!
//! TODO(Plan D): Full implementation requires world_objects, loot tables,
//! and NPC body state to be migrated into Resources/ECS components. Currently
//! processes ItemEvents and marks items as taken.

use hecs::World;
use crate::game::ecs::components::WorldObj;
use crate::game::ecs::resources::Resources;
use crate::game::ecs::events::{ItemEvent, MessageEvent};

pub fn run(world: &mut World, res: &mut Resources) {
    let item_events: Vec<ItemEvent> = std::mem::take(&mut res.events.item);

    for event in item_events {
        match event {
            ItemEvent::TakeItem { entity } => {
                handle_take(world, res, entity);
            }
            ItemEvent::SearchBody { entity } => {
                handle_search(world, res, entity);
            }
        }
    }
}

fn handle_take(world: &mut World, res: &mut Resources, entity: hecs::Entity) {
    // TODO(Plan D): Full item pickup logic — add item to inventory, check weight,
    // handle quest items (talisman pieces, keys), emit appropriate scroll message
    // from faery.toml event_msg table. See gameplay_scene/items.rs handle_take_item().
    if let Ok(mut obj) = world.get::<&mut WorldObj>(entity) {
        obj.ob_stat = 0; // 0 = taken
        obj.visible = false;
    }
    res.events.message.push(MessageEvent {
        text: "Taken.".to_string(),
    });
}

fn handle_search(world: &mut World, res: &mut Resources, entity: hecs::Entity) {
    // TODO(Plan D): Body search loot logic — transfer gold + weapon from Bones/enemy
    // to hero inventory, emit loot description message.
    // See gameplay_scene/items.rs search_body().
    if let Ok(mut obj) = world.get::<&mut WorldObj>(entity) {
        obj.ob_stat = 0; // mark as looted
    }
    let _ = res;
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::*;
    use crate::game::ecs::events::ItemEvent;
    use super::run;

    fn hero_stats() -> HeroStats {
        HeroStats { vitality: 100, brave: 0, luck: 0, kind: 0,
                    wealth: 0, hunger: 0, fatigue: 0, gold: 0 }
    }

    #[test]
    fn take_item_marks_invisible() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        let item = spawn_ground_item(&mut world, 105.0, 100.0,
            WorldObj { ob_id: 5, ob_stat: 1, region: 0, visible: true, goal: 0 });
        res.events.item.push(ItemEvent::TakeItem { entity: item });
        run(&mut world, &mut res);
        let obj = world.get::<&WorldObj>(item).unwrap();
        assert!(!obj.visible, "Item should be marked invisible after take");
        assert_eq!(obj.ob_stat, 0, "Item ob_stat should be 0 (taken)");
    }

    #[test]
    fn take_item_emits_message() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        let item = spawn_ground_item(&mut world, 105.0, 100.0,
            WorldObj { ob_id: 5, ob_stat: 1, region: 0, visible: true, goal: 0 });
        res.events.item.push(ItemEvent::TakeItem { entity: item });
        run(&mut world, &mut res);
        assert_eq!(res.events.message.len(), 1);
    }

    #[test]
    fn search_body_marks_looted() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        let bones = spawn_bones(&mut world, 110.0, 100.0, 0, 0, [0u8; 36]);
        res.events.item.push(ItemEvent::SearchBody { entity: bones });
        run(&mut world, &mut res);
        let obj = world.get::<&WorldObj>(bones).unwrap();
        assert_eq!(obj.ob_stat, 0, "Bones ob_stat should be 0 after search");
    }

    #[test]
    fn no_events_no_panic() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        run(&mut world, &mut res);
    }
}
