//! ItemSystem — processes item pickup, body search, and inventory mutations.
//! Port of handle_take_item() and search_body() from gameplay_scene/items.rs.
//! See docs/spec/inventory-items.md.

use hecs::World;
use crate::game::ecs::components::{Bones, HeroStats, Inventory, Loot, WorldObj};
use crate::game::ecs::resources::Resources;
use crate::game::ecs::events::{ItemEvent, MessageEvent, SfxEvent};

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
    let ob_id = match world.get::<&WorldObj>(entity) {
        Ok(obj) if obj.ob_stat == 1 => obj.ob_id,
        _ => return, // already taken or invalid
    };

    if let Ok(mut obj) = world.get::<&mut WorldObj>(entity) {
        obj.ob_stat = 0;
        obj.visible = false;
    }

    if let Ok(mut inv) = world.get::<&mut Inventory>(res.hero_entity) {
        let slot = ob_id as usize;
        if slot < inv.stuff.len() {
            inv.stuff[slot] = inv.stuff[slot].saturating_add(1);
        }
    }

    // Quest state updates (Plan V)
    match ob_id {
        22 => {
            // Talisman (win condition)
            res.quest.talisman_obtained = true;
        }
        7 => {
            // Sun Stone (Witch vulnerability)
            res.quest.sun_stone_obtained = true;
        }
        5 => {
            // Golden Lasso (swan flight)
            res.quest.golden_lasso_obtained = true;
        }
        _ => {
            // Non-quest item, no quest state update
        }
    }

    res.events.sfx.push(SfxEvent { sfx_id: 5 });
    res.events.message.push(MessageEvent {
        text: "Taken.".to_string(),
    });
}

fn handle_search(world: &mut World, res: &mut Resources, entity: hecs::Entity) {
    // Case 1: Bones entity — merge entire inventory into hero's.
    let is_bones = world.get::<&Bones>(entity).is_ok();
    if is_bones {
        let bones_stuff: Option<[u8; 36]> = world
            .get::<&Inventory>(entity)
            .ok()
            .map(|inv| inv.stuff);

        if let Some(bones_stuff) = bones_stuff {
            if let Ok(mut hero_inv) = world.get::<&mut Inventory>(res.hero_entity) {
                for (slot, &count) in bones_stuff.iter().enumerate() {
                    if count > 0 && slot < hero_inv.stuff.len() {
                        hero_inv.stuff[slot] = hero_inv.stuff[slot].saturating_add(count);
                    }
                }
            }
            res.events.message.push(MessageEvent {
                text: "You search the remains.".to_string(),
            });
        }

        if let Ok(mut obj) = world.get::<&mut WorldObj>(entity) {
            obj.ob_stat = 0;
            obj.visible = false;
        }
        return;
    }

    // Case 2: Dead enemy with Loot component.
    let loot_data: Option<(u8, i16)> = world.get::<&Loot>(entity).ok().and_then(|loot| {
        if loot.looted { None } else { Some((loot.weapon, loot.gold)) }
    });

    if let Some((weapon, gold)) = loot_data {
        if let Ok(mut loot) = world.get::<&mut Loot>(entity) {
            loot.looted = true;
        }
        if weapon > 0 {
            let slot = weapon as usize;
            if let Ok(mut inv) = world.get::<&mut Inventory>(res.hero_entity) {
                if slot < inv.stuff.len() {
                    inv.stuff[slot] = inv.stuff[slot].saturating_add(1);
                }
            }
        }
        if gold > 0 {
            if let Ok(mut stats) = world.get::<&mut HeroStats>(res.hero_entity) {
                stats.wealth = stats.wealth.saturating_add(gold);
            }
        }
        res.events.sfx.push(SfxEvent { sfx_id: 5 });
        res.events.message.push(MessageEvent {
            text: "Searched.".to_string(),
        });
    }

    if let Ok(mut obj) = world.get::<&mut WorldObj>(entity) {
        obj.ob_stat = 0;
    }
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
    fn take_item_increments_inventory() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        let item = spawn_ground_item(&mut world, 102.0, 100.0,
            WorldObj { ob_id: 2, ob_stat: 1, region: 0, visible: true, goal: 0 });
        res.events.item.push(ItemEvent::TakeItem { entity: item });
        run(&mut world, &mut res);
        let inv = world.get::<&Inventory>(hero).unwrap();
        assert_eq!(inv.stuff[2], 1, "stuff[2] (Sword slot) should be 1 after pickup");
    }

    #[test]
    fn take_item_already_taken_is_noop() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        let item = spawn_ground_item(&mut world, 102.0, 100.0,
            WorldObj { ob_id: 2, ob_stat: 0, region: 0, visible: false, goal: 0 });
        res.events.item.push(ItemEvent::TakeItem { entity: item });
        run(&mut world, &mut res);
        let inv = world.get::<&Inventory>(hero).unwrap();
        assert_eq!(inv.stuff[2], 0, "Inventory should be unchanged for already-taken item");
        assert_eq!(res.events.message.len(), 0, "No message for already-taken item");
    }

    #[test]
    fn search_body_transfers_gold() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        let enemy = world.spawn((
            Enemy,
            Position::new(102.0, 100.0),
            WorldObj { ob_id: 0, ob_stat: 0, region: 0, visible: false, goal: 0 },
            Loot { weapon: 0, gold: 10, looted: false },
        ));
        res.events.item.push(ItemEvent::SearchBody { entity: enemy });
        run(&mut world, &mut res);
        let stats = world.get::<&HeroStats>(hero).unwrap();
        assert_eq!(stats.wealth, 10, "Hero wealth should increase by looted gold");
        let loot = world.get::<&Loot>(enemy).unwrap();
        assert!(loot.looted, "Loot should be marked as looted after search");
    }

    #[test]
    fn search_body_transfers_weapon() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        let enemy = world.spawn((
            Enemy,
            Position::new(102.0, 100.0),
            WorldObj { ob_id: 0, ob_stat: 0, region: 0, visible: false, goal: 0 },
            Loot { weapon: 1, gold: 0, looted: false },
        ));
        res.events.item.push(ItemEvent::SearchBody { entity: enemy });
        run(&mut world, &mut res);
        let inv = world.get::<&Inventory>(hero).unwrap();
        assert_eq!(inv.stuff[1], 1, "Mace (slot 1) should be in hero inventory after looting");
    }

    #[test]
    fn search_bones_merges_inventory() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        let mut bones_stuff = [0u8; 36];
        bones_stuff[3] = 1; // Bow
        let bones = spawn_bones(&mut world, 105.0, 100.0, 0, 0, bones_stuff);
        res.events.item.push(ItemEvent::SearchBody { entity: bones });
        run(&mut world, &mut res);
        let inv = world.get::<&Inventory>(hero).unwrap();
        assert_eq!(inv.stuff[3], 1, "Bow (slot 3) should be merged into hero inventory from Bones");
    }

    #[test]
    fn no_events_no_panic() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        run(&mut world, &mut res);
    }
}
