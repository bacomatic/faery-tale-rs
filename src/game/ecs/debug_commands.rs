//! Handles [`DebugCommand`] values from the debug console in [`EcsScene`].
//!
//! Called once per tick from `EcsScene::run_tick()` after draining the debug
//! console's command queue.

use hecs::World;

use crate::game::debug_command::{DebugCommand, MagicEffect, StatId};
use crate::game::ecs::components::{HeroStats, Inventory, Position};
use crate::game::ecs::resources::Resources;

/// Dispatch a single debug command against the ECS world and resources.
pub fn handle(cmd: DebugCommand, world: &mut World, res: &mut Resources) {
    match cmd {
        DebugCommand::SetGodMode { flags } => {
            res.brother.god_mode = flags;
        }

        DebugCommand::TeleportCoords { x, y } => {
            if let Ok(mut pos) = world.get::<&mut Position>(res.hero_entity) {
                pos.set(x as f32, y as f32);
            }
        }

        DebugCommand::ToggleMagicEffect { effect } => {
            // Toggle the corresponding sticky flag and ensure the timer is
            // active when sticky is enabled, so the effect is actually visible.
            match effect {
                MagicEffect::Jewel => {
                    res.clock.light_sticky = !res.clock.light_sticky;
                    if res.clock.light_sticky && res.clock.light_timer <= 0 {
                        res.clock.light_timer = 1;
                    }
                }
                MagicEffect::Orb => {
                    res.clock.secret_sticky = !res.clock.secret_sticky;
                    if res.clock.secret_sticky && res.clock.secret_timer <= 0 {
                        res.clock.secret_timer = 1;
                    }
                }
                MagicEffect::Ring => {
                    res.clock.freeze_sticky = !res.clock.freeze_sticky;
                    if res.clock.freeze_sticky && res.clock.freeze_timer <= 0 {
                        res.clock.freeze_timer = 1;
                    }
                }
            }
        }

        DebugCommand::SetStat { stat, value } => {
            if let Ok(mut stats) = world.get::<&mut HeroStats>(res.hero_entity) {
                apply_stat_set(&mut stats, stat, value);
            }
        }

        DebugCommand::AdjustStat { stat, delta } => {
            if let Ok(mut stats) = world.get::<&mut HeroStats>(res.hero_entity) {
                apply_stat_adjust(&mut stats, stat, delta);
            }
        }

        DebugCommand::SetInventory { index, value } => {
            if let Ok(mut inv) = world.get::<&mut Inventory>(res.hero_entity) {
                if let Some(slot) = inv.stuff.get_mut(index as usize) {
                    *slot = value;
                }
            }
        }

        DebugCommand::AdjustInventory { index, delta } => {
            if let Ok(mut inv) = world.get::<&mut Inventory>(res.hero_entity) {
                if let Some(slot) = inv.stuff.get_mut(index as usize) {
                    *slot = (*slot as i16 + delta as i16).clamp(0, 255) as u8;
                }
            }
        }

        DebugCommand::HeroPack => {
            // Full test loadout per DEBUG_SPECIFICATION.md §/pack.
            const PACK: &[(usize, u8)] = &[
                (0, 1),   // Dirk
                (1, 1),   // Mace
                (2, 1),   // Sword
                (3, 1),   // Bow
                (4, 1),   // Magic Wand
                (5, 1),   // Golden Lasso
                (6, 1),   // Sea Shell
                (7, 1),   // Sun Stone
                (8, 255), // Arrows (full quiver)
                (9, 3),   // Blue Stone
                (10, 3),  // Green Jewel
                (11, 3),  // Glass Vial
                (12, 3),  // Crystal Orb
                (13, 3),  // Bird Totem
                (14, 3),  // Gold Ring
                (15, 3),  // Jade Skull
                (16, 5),  // Gold Key
                (17, 5),  // Green Key
                (18, 5),  // Blue Key
                (19, 5),  // Red Key
                (20, 5),  // Grey Key
                (21, 5),  // White Key
                (23, 1),  // Rose
                (24, 255), // Apple
            ];
            if let Ok(mut inv) = world.get::<&mut Inventory>(res.hero_entity) {
                for &(idx, val) in PACK {
                    if let Some(slot) = inv.stuff.get_mut(idx) {
                        *slot = val;
                    }
                }
            }
            // Also heal: vitality to 15 + brave/4, hunger=0, fatigue=0.
            if let Ok(mut stats) = world.get::<&mut HeroStats>(res.hero_entity) {
                let cap = 15i16.saturating_add(stats.brave / 4);
                stats.vitality = cap;
                stats.hunger = 0;
                stats.fatigue = 0;
            }
        }

        // ── Remaining commands not yet implemented ──
        _ => {}
    }
}

fn apply_stat_set(stats: &mut HeroStats, stat: StatId, value: i16) {
    match stat {
        StatId::Vitality => stats.vitality = value,
        StatId::Brave    => stats.brave    = value,
        StatId::Luck     => stats.luck     = value,
        StatId::Kind     => stats.kind     = value,
        StatId::Wealth   => stats.wealth   = value,
        StatId::Hunger   => stats.hunger   = value,
        StatId::Fatigue  => stats.fatigue  = value,
    }
}

fn apply_stat_adjust(stats: &mut HeroStats, stat: StatId, delta: i16) {
    match stat {
        StatId::Vitality => stats.vitality = stats.vitality.saturating_add(delta),
        StatId::Brave    => stats.brave    = stats.brave.saturating_add(delta),
        StatId::Luck     => stats.luck     = stats.luck.saturating_add(delta),
        StatId::Kind     => stats.kind     = stats.kind.saturating_add(delta),
        StatId::Wealth   => stats.wealth   = stats.wealth.saturating_add(delta),
        StatId::Hunger   => stats.hunger   = stats.hunger.saturating_add(delta),
        StatId::Fatigue  => stats.fatigue  = stats.fatigue.saturating_add(delta),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hecs::World;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::spawn_hero;
    use crate::game::ecs::components::{HeroStats, Inventory};
    use crate::game::debug_command::GodModeFlags;

    fn make_world_and_res() -> (World, Resources) {
        let mut world = World::new();
        let stats = HeroStats {
            vitality: 100,
            brave: 50,
            luck: 50,
            kind: 50,
            wealth: 50,
            hunger: 0,
            fatigue: 0,
            gold: 0,
        };
        let hero = spawn_hero(&mut world, 100.0, 200.0, 0, stats, Inventory::empty());
        let mut res = Resources::new(hero);
        res.hero_entity = hero;
        (world, res)
    }

    #[test]
    fn teleport_coords_moves_hero() {
        let (mut world, mut res) = make_world_and_res();
        handle(
            DebugCommand::TeleportCoords { x: 300, y: 400 },
            &mut world,
            &mut res,
        );
        let pos = world.get::<&Position>(res.hero_entity).unwrap();
        assert_eq!(pos.x, 300.0);
        assert_eq!(pos.y, 400.0);
    }

    #[test]
    fn set_god_mode_updates_resources() {
        let (mut world, mut res) = make_world_and_res();
        let flags = GodModeFlags::INVINCIBLE | GodModeFlags::NOCLIP;
        handle(DebugCommand::SetGodMode { flags }, &mut world, &mut res);
        assert_eq!(res.brother.god_mode, flags);
    }

    #[test]
    fn unknown_command_does_not_panic() {
        let (mut world, mut res) = make_world_and_res();
        // InstaKill is not yet wired; should silently do nothing.
        handle(DebugCommand::InstaKill, &mut world, &mut res);
    }

    #[test]
    fn toggle_magic_jewel_sticky_and_timer() {
        let (mut world, mut res) = make_world_and_res();
        handle(DebugCommand::ToggleMagicEffect { effect: MagicEffect::Jewel }, &mut world, &mut res);
        assert!(res.clock.light_sticky);
        assert!(res.clock.light_timer > 0);
    }

    #[test]
    fn toggle_magic_orb_sticky_and_timer() {
        let (mut world, mut res) = make_world_and_res();
        handle(DebugCommand::ToggleMagicEffect { effect: MagicEffect::Orb }, &mut world, &mut res);
        assert!(res.clock.secret_sticky);
        assert!(res.clock.secret_timer > 0);
    }

    #[test]
    fn toggle_magic_ring_sticky_and_timer() {
        let (mut world, mut res) = make_world_and_res();
        handle(DebugCommand::ToggleMagicEffect { effect: MagicEffect::Ring }, &mut world, &mut res);
        assert!(res.clock.freeze_sticky);
        assert!(res.clock.freeze_timer > 0);
    }

    #[test]
    fn toggle_magic_effect_turns_off_sticky() {
        let (mut world, mut res) = make_world_and_res();
        let cmd = DebugCommand::ToggleMagicEffect { effect: MagicEffect::Orb };
        handle(cmd.clone(), &mut world, &mut res);
        assert!(res.clock.secret_sticky);
        handle(cmd, &mut world, &mut res);
        assert!(!res.clock.secret_sticky);
    }

    #[test]
    fn set_stat_vitality() {
        let (mut world, mut res) = make_world_and_res();
        handle(DebugCommand::SetStat { stat: StatId::Vitality, value: 42 }, &mut world, &mut res);
        assert_eq!(world.get::<&HeroStats>(res.hero_entity).unwrap().vitality, 42);
    }

    #[test]
    fn adjust_stat_wealth() {
        let (mut world, mut res) = make_world_and_res();
        handle(DebugCommand::AdjustStat { stat: StatId::Wealth, delta: 50 }, &mut world, &mut res);
        assert_eq!(world.get::<&HeroStats>(res.hero_entity).unwrap().wealth, 100);
    }

    #[test]
    fn set_inventory_slot() {
        let (mut world, mut res) = make_world_and_res();
        handle(DebugCommand::SetInventory { index: 2, value: 7 }, &mut world, &mut res);
        assert_eq!(world.get::<&Inventory>(res.hero_entity).unwrap().stuff[2], 7);
    }

    #[test]
    fn adjust_inventory_slot() {
        let (mut world, mut res) = make_world_and_res();
        handle(DebugCommand::SetInventory { index: 8, value: 10 }, &mut world, &mut res);
        handle(DebugCommand::AdjustInventory { index: 8, delta: 5 }, &mut world, &mut res);
        assert_eq!(world.get::<&Inventory>(res.hero_entity).unwrap().stuff[8], 15);
    }

    #[test]
    fn hero_pack_fills_inventory_and_heals() {
        let (mut world, mut res) = make_world_and_res();
        handle(DebugCommand::HeroPack, &mut world, &mut res);
        let inv = world.get::<&Inventory>(res.hero_entity).unwrap();
        assert_eq!(inv.stuff[0], 1,   "Dirk");
        assert_eq!(inv.stuff[8], 255, "Arrows");
        assert_eq!(inv.stuff[9], 3,   "Blue Stone");
        assert_eq!(inv.stuff[16], 5,  "Gold Key");
        assert_eq!(inv.stuff[24], 255, "Apple");
        let stats = world.get::<&HeroStats>(res.hero_entity).unwrap();
        assert_eq!(stats.hunger, 0);
        assert_eq!(stats.fatigue, 0);
        // vitality should be 15 + brave/4 = 15 + 50/4 = 15 + 12 = 27
        assert_eq!(stats.vitality, 27);
    }
}
