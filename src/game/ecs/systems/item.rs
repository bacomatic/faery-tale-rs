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
    // Authoritative single-item pickup line (dialog_system.md, fmain.c:3191-3193).
    let name = res.brother.active_name.clone();
    let item_name = crate::game::world_objects::stuff_index_name(ob_id as usize);
    res.events.message.push(MessageEvent {
        text: format!("{name} found a {item_name}."),
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
            let name = res.brother.active_name.clone();
            res.events.message.push(MessageEvent {
                text: format!("{name} found his brother's bones."),
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
        // Mark looted first to prevent double-search.
        if let Ok(mut loot) = world.get::<&mut Loot>(entity) {
            loot.looted = true;
        }

        let name = res.brother.active_name.clone();
        let mut msg = format!("{name} searched the body and found");

        if weapon > 0 {
            // `weapon` is a 1-based code (1=Dirk..4=Bow); inventory slot is code-1
            // (fmain.c:3256-3257). The previous code used stuff[weapon] (off by one).
            let slot = (weapon - 1) as usize;
            let weapon_name = crate::game::world_objects::stuff_index_name(slot);
            msg.push_str(&format!(" a {weapon_name}"));
            if let Ok(mut inv) = world.get::<&mut Inventory>(res.hero_entity) {
                if slot < inv.stuff.len() {
                    inv.stuff[slot] = inv.stuff[slot].saturating_add(1);
                }
            }
            // Bow (code 4): also grant a random arrow bundle (rand8()+2 = 2..9).
            if weapon == 4 {
                let arrows = crate::game::combat::bitrand(7) as u8 + 2;
                if let Ok(mut inv) = world.get::<&mut Inventory>(res.hero_entity) {
                    inv.stuff[8] = inv.stuff[8].saturating_add(arrows);
                }
                msg.push_str(&format!(" and {arrows} Arrows"));
            }
        } else {
            msg.push_str(" nothing");
        }
        msg.push('.');

        // Gold is transferred silently (the original body-search line announces
        // weapon/treasure, not gold).
        if gold > 0 {
            if let Ok(mut stats) = world.get::<&mut HeroStats>(res.hero_entity) {
                stats.wealth = stats.wealth.saturating_add(gold);
            }
        }

        res.events.sfx.push(SfxEvent { sfx_id: 5 });
        res.events.message.push(MessageEvent { text: msg });
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
        res.brother.active_name = "Julian".to_string();
        let item = spawn_ground_item(&mut world, 102.0, 100.0,
            WorldObj { ob_id: 2, ob_stat: 1, region: 0, visible: true, goal: 0 });
        res.events.item.push(ItemEvent::TakeItem { entity: item });
        run(&mut world, &mut res);
        let inv = world.get::<&Inventory>(hero).unwrap();
        assert_eq!(inv.stuff[2], 1, "stuff[2] (Sword slot) should be 1 after pickup");
    }

    #[test]
    fn take_item_emits_found_message() {
        let mut world = World::new();
        let hero = world.spawn((Inventory::empty(),));
        let mut res = Resources::new(hero);
        res.brother.active_name = "Julian".to_string();
        // Glass Vial = slot 11.
        let item = world.spawn((WorldObj { ob_id: 11, ob_stat: 1, region: 0, visible: true, goal: 0 },));
        res.events.item.push(ItemEvent::TakeItem { entity: item });

        run(&mut world, &mut res);

        let msg = res.events.message.last().expect("a message");
        assert_eq!(msg.text, "Julian found a Glass Vial.");
        assert_eq!(world.get::<&Inventory>(hero).unwrap().stuff[11], 1);
    }

    #[test]
    fn take_item_already_taken_is_noop() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.brother.active_name = "Julian".to_string();
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
        res.brother.active_name = "Julian".to_string();
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
        res.brother.active_name = "Julian".to_string();
        let enemy = world.spawn((
            Enemy,
            Position::new(102.0, 100.0),
            WorldObj { ob_id: 0, ob_stat: 0, region: 0, visible: false, goal: 0 },
            Loot { weapon: 1, gold: 0, looted: false },
        ));
        res.events.item.push(ItemEvent::SearchBody { entity: enemy });
        run(&mut world, &mut res);
        let inv = world.get::<&Inventory>(hero).unwrap();
        assert_eq!(inv.stuff[0], 1, "Dirk (slot 0) should be in hero inventory after looting");
        assert_eq!(res.events.message.last().unwrap().text,
            "Julian searched the body and found a Dirk.");
    }

    #[test]
    fn search_body_sword_grants_slot_2_and_message() {
        let mut world = World::new();
        let hero = world.spawn((Inventory::empty(), hero_stats()));
        let mut res = Resources::new(hero);
        res.brother.active_name = "Julian".to_string();
        // Sword = weapon code 3 → inventory slot 2.
        let enemy = world.spawn((Enemy, Position::new(0.0, 0.0),
            WorldObj { ob_id: 0, ob_stat: 0, region: 0, visible: false, goal: 0 },
            Loot { weapon: 3, gold: 0, looted: false }));
        res.events.item.push(ItemEvent::SearchBody { entity: enemy });

        run(&mut world, &mut res);

        assert_eq!(world.get::<&Inventory>(hero).unwrap().stuff[2], 1, "Sword goes to slot 2");
        assert_eq!(res.events.message.last().unwrap().text,
            "Julian searched the body and found a Sword.");
    }

    #[test]
    fn search_body_no_weapon_says_nothing() {
        let mut world = World::new();
        let hero = world.spawn((Inventory::empty(), hero_stats()));
        let mut res = Resources::new(hero);
        res.brother.active_name = "Julian".to_string();
        let enemy = world.spawn((Enemy, Position::new(0.0, 0.0),
            WorldObj { ob_id: 0, ob_stat: 0, region: 0, visible: false, goal: 0 },
            Loot { weapon: 0, gold: 0, looted: false }));
        res.events.item.push(ItemEvent::SearchBody { entity: enemy });

        run(&mut world, &mut res);

        assert_eq!(res.events.message.last().unwrap().text,
            "Julian searched the body and found nothing.");
    }

    #[test]
    fn search_body_bow_grants_arrows() {
        let mut world = World::new();
        let hero = world.spawn((Inventory::empty(), hero_stats()));
        let mut res = Resources::new(hero);
        res.brother.active_name = "Julian".to_string();
        // Bow = weapon code 4 → slot 3; arrows go to slot 8.
        let enemy = world.spawn((Enemy, Position::new(0.0, 0.0),
            WorldObj { ob_id: 0, ob_stat: 0, region: 0, visible: false, goal: 0 },
            Loot { weapon: 4, gold: 0, looted: false }));
        res.events.item.push(ItemEvent::SearchBody { entity: enemy });

        run(&mut world, &mut res);

        assert_eq!(world.get::<&Inventory>(hero).unwrap().stuff[3], 1, "Bow goes to slot 3");
        let arrows = world.get::<&Inventory>(hero).unwrap().stuff[8];
        assert!((2..=9).contains(&arrows), "arrow bundle is 2..9, got {arrows}");
        assert!(res.events.message.last().unwrap().text.contains("Bow and "));
        assert!(res.events.message.last().unwrap().text.ends_with(" Arrows."));
    }

    #[test]
    fn search_bones_emits_brothers_bones_message() {
        let mut world = World::new();
        let hero = world.spawn((Inventory::empty(),));
        let mut res = Resources::new(hero);
        res.brother.active_name = "Phillip".to_string();
        let mut stuff = [0u8; 36];
        stuff[2] = 1; // dead brother carried a Sword
        let bones = world.spawn((Bones, BrotherKind { id: 0 }, Position::new(0.0, 0.0),
            Inventory { stuff },
            WorldObj { ob_id: 0, ob_stat: 1, region: 0, visible: true, goal: 0 }));
        res.events.item.push(ItemEvent::SearchBody { entity: bones });

        run(&mut world, &mut res);

        assert_eq!(world.get::<&Inventory>(hero).unwrap().stuff[2], 1, "brother's Sword merged");
        assert_eq!(res.events.message.last().unwrap().text,
            "Phillip found his brother's bones.");
    }

    #[test]
    fn search_bones_merges_inventory() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.brother.active_name = "Julian".to_string();
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
        res.brother.active_name = "Julian".to_string();
        run(&mut world, &mut res);
    }
}
