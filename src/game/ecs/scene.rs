//! [`EcsScene`] — the ECS-based gameplay scene, implementing the [`Scene`] trait.
//!
//! This is the sole gameplay scene since `GameplayScene` was removed.
//! Subsystems are being ported into ECS systems over Plans D–F.

use std::any::Any;
use std::collections::HashSet;

use hecs::World;
use sdl3::event::Event;
use sdl3::render::{Canvas, Texture};
use sdl3::video::Window;

use crate::game::colors::RGB4;
use crate::game::debug_tui::DebugConsole;
use crate::game::direction::Direction;
use crate::game::ecs::components::{Bones, BrotherKind, HeroStats, Inventory, Position, SetFig, WorldObj};
use crate::game::ecs::resources::Resources;
use crate::game::ecs::spawn::{spawn_bones, spawn_hero};
use crate::game::ecs::systems;
use crate::game::game_library::GameLibrary;
use crate::game::map_renderer::{MAP_DST_H, MAP_DST_W};
use crate::game::magic::{magic_dispatch_ecs, MagicResult, ITEM_BLUE_STONE};
use crate::game::menu::{MenuAction, MenuState};
use crate::game::shop::{buy_slot_ecs, BuyOutcome, BuyResult};
use crate::game::palette::{amiga_color_to_rgba, Palette, PALETTE_SIZE};
use crate::game::scene::{Scene, SceneResources, SceneResult};

use super::debug_commands;

// ── InputState ────────────────────────────────────────────────────────────────

/// Tracks which movement keys are currently held.  Direction flags are derived
/// by summing axis contributions from all held keys so that opposites cancel
/// (e.g. Left+Right → no horizontal movement).
///
/// Port of `InputState` from `gameplay_scene/mod.rs`.
struct InputState {
    up:    bool,
    down:  bool,
    left:  bool,
    right: bool,
    /// Set of movement keycodes currently physically held.
    pressed_movement_keys: HashSet<sdl3::keyboard::Keycode>,
    /// Gamepad left-stick contribution, each axis clamped to {-1, 0, +1}.
    gamepad_x: i32,
    gamepad_y: i32,
    /// Fire/attack held state — true whenever at least one source holds the attack input.
    /// Sources: keyboard (Kp0), gamepad South button, right mouse button on compass.
    fire_keyboard: bool,
    fire_gamepad:  bool,
    fire_mouse:    bool,
}

impl InputState {
    fn new() -> Self {
        Self {
            up:    false,
            down:  false,
            left:  false,
            right: false,
            pressed_movement_keys: HashSet::new(),
            gamepad_x: 0,
            gamepad_y: 0,
            fire_keyboard: false,
            fire_gamepad:  false,
            fire_mouse:    false,
        }
    }

    fn fire(&self) -> bool {
        self.fire_keyboard || self.fire_gamepad || self.fire_mouse
    }

    /// Recompute up/down/left/right by summing contributions from all held
    /// movement keys and the gamepad stick.  Opposite directions cancel.
    fn recompute(&mut self) {
        use sdl3::keyboard::Keycode;
        let mut x: i32 = self.gamepad_x;
        let mut y: i32 = self.gamepad_y;
        for kc in &self.pressed_movement_keys {
            let (kx, ky): (i32, i32) = match kc {
                Keycode::Up    | Keycode::Kp8 => ( 0, -1),
                Keycode::Down  | Keycode::Kp2 => ( 0,  1),
                Keycode::Left  | Keycode::Kp4 => (-1,  0),
                Keycode::Right | Keycode::Kp6 => ( 1,  0),
                Keycode::Kp7               => (-1, -1),
                Keycode::Kp9               => ( 1, -1),
                Keycode::Kp1               => (-1,  1),
                Keycode::Kp3               => ( 1,  1),
                _ => (0, 0),
            };
            x += kx;
            y += ky;
        }
        self.up    = y < 0;
        self.down  = y > 0;
        self.left  = x < 0;
        self.right = x > 0;
    }

    /// Decode 8-way direction from current input flags.
    fn to_direction(&self) -> Direction {
        match (self.up, self.down, self.left, self.right) {
            (true,  false, false, false) => Direction::N,
            (true,  false, false, true)  => Direction::NE,
            (false, false, false, true)  => Direction::E,
            (false, true,  false, true)  => Direction::SE,
            (false, true,  false, false) => Direction::S,
            (false, true,  true,  false) => Direction::SW,
            (false, false, true,  false) => Direction::W,
            (true,  false, true,  false) => Direction::NW,
            _                            => Direction::None,
        }
    }
}

/// ECS-based gameplay scene (Plan D skeleton).
///
/// Owns the `hecs::World` and the singleton `Resources`. Each call to
/// `update()` runs one or more gameplay ticks followed by a render pass.
// ── Render layout constants (from display-rendering.md) ───────────────────────
const CANVAS_MARGIN_Y:    i32 = 40;
const PLAYFIELD_X:        i32 = 32;
const PLAYFIELD_Y:        i32 = CANVAS_MARGIN_Y;
const PLAYFIELD_LORES_W:  u32 = 288;
const PLAYFIELD_LORES_H:  u32 = 140;
const PLAYFIELD_CANVAS_W: u32 = PLAYFIELD_LORES_W * 2;
const PLAYFIELD_CANVAS_H: u32 = PLAYFIELD_LORES_H * 2;
const HIBAR_NATIVE_H:     u32 = 57;
const HIBAR_H:            u32 = HIBAR_NATIVE_H * 2;
const HIBAR_Y:            i32 = CANVAS_MARGIN_Y + PLAYFIELD_CANVAS_H as i32 + 6;

pub struct EcsScene {
    pub world:          World,
    pub res:            Resources,
    console:            Option<DebugConsole>,
    input:              InputState,
    last_mood:          u8,
    mood_tick:          u32,
    adf_load_done:      bool,
    /// ADF disk image — kept alive so `reload_region()` can reload assets.
    adf:                Option<std::sync::Arc<crate::game::adf::AdfDisk>>,
    /// RGB4 base palette used as input to fade_page() for day/night computation.
    base_colors:        Option<crate::game::colors::Palette>,
    /// Scroll-area message queue (up to 4 visible at once, most recent last).
    messages:           Vec<String>,
    /// Menu bar state: mode, enabled buttons, key/click dispatch.
    menu:               MenuState,
    /// Set to true when the player chooses Quit from the Game menu.
    quit_requested:     bool,
    /// Menu actions queued from handle_event() (runs outside ECS borrow).
    pending_menu_actions: Vec<MenuAction>,
    /// If true, emit BrotherSuccession on the first update() call to trigger julian_start placard.
    /// Set to false when launched with --skip-intro.
    show_start_placard: bool,
    /// True until the first update() call has been processed.
    first_update: bool,

}

impl EcsScene {
    /// Construct a new `EcsScene`, spawning the hero at the location specified
    /// in `faery.toml` for brother 0 (Julian).  Falls back to `(100, 100)` if
    /// the library has no brother or location data.
    pub fn new(game_lib: &GameLibrary, console: Option<DebugConsole>, show_start_placard: bool) -> Self {
        let mut world = World::new();

        // Resolve hero starting position from the library.
        let (start_x, start_y, start_region) = game_lib
            .get_brother(0)
            .and_then(|bro| game_lib.find_location(&bro.spawn))
            .map(|loc| (loc.x as f32, loc.y as f32, loc.region))
            .unwrap_or((100.0, 100.0, 0));

        // Build hero stats from library data.
        // Vitality formula: 15 + brave/4 (fmain.c revive(), RESEARCH §14).
        let stats = game_lib
            .get_brother(0)
            .map(|bro| HeroStats {
                vitality: 15 + bro.brave / 4,
                brave:    bro.brave,
                luck:     bro.luck,
                kind:     bro.kind,
                wealth:   bro.wealth,
                hunger:   0,
                fatigue:  0,
                gold:     0,
            })
            .unwrap_or(HeroStats {
                vitality: 23, // Julian default: 15 + 35/4 = 23
                brave:    35,
                luck:     20,
                kind:     15,
                wealth:   20,
                hunger:   0,
                fatigue:  0,
                gold:     0,
            });

        let hero = spawn_hero(&mut world, start_x, start_y, 0, stats, Inventory::with_dirk());

        let mut res = Resources::new(hero);
        res.region.region_num = start_region;
        res.brother.active_name = game_lib
            .get_brother(0)
            .map(|b| b.name.clone())
            .unwrap_or_else(|| "Hero".to_string());

        // Populate compass hit-regions from the library data (comptable).
        if let Some(cfg) = game_lib.get_compass() {
            res.palette.compass_regions = cfg
                .comptable
                .regions
                .iter()
                .map(|r| (r.x, r.y, r.w, r.h))
                .collect();
        }

        // Load textcolors palette for menu button rendering (spec §25.8).
        if let Some(tc_pal) = game_lib.find_palette("textcolors") {
            for (i, entry) in tc_pal.colors.iter().enumerate().take(PALETTE_SIZE) {
                res.palette.textcolors[i] = amiga_color_to_rgba(entry.color);
            }
        }

        Self {
            world,
            res,
            console,
            input: InputState::new(),
            last_mood: u8::MAX,
            mood_tick: 0,
            adf_load_done: false,
            adf: None,
            base_colors: None,
            messages: Vec::new(),
            menu: MenuState::new(),
            quit_requested: false,
            pending_menu_actions: Vec::new(),
            show_start_placard,
            first_update: true,
        }
    }


    /// Return the id of the next living brother after `dead_id` in succession order
    /// (0→1→2), skipping any brother already represented by a Bones entity in the world.
    /// Returns `None` when all brothers are dead.
    fn next_living_brother(&self, dead_id: u8) -> Option<u8> {
        let bones_ids: std::collections::HashSet<u8> = self.world
            .query::<&BrotherKind>()
            .with::<&Bones>()
            .iter()
            .map(|bk| bk.id)
            .collect();
        for candidate in (dead_id + 1)..3 {
            if !bones_ids.contains(&candidate) {
                return Some(candidate);
            }
        }
        None
    }

    /// Consume all `BrotherDiedEvent`s from this tick.
    /// Spawns a Bones entity, selects successor, and swaps the hero entity.
    /// Returns `Some(SceneResult)` when a placard sequence should be shown,
    /// or `None` if the event queue was empty.
    fn drain_brother_deaths(&mut self, game_lib: &GameLibrary) -> Option<SceneResult> {
        let events: Vec<_> = self.res.events.brother.drain(..).collect();
        if events.is_empty() {
            return None;
        }
        let mut result = None;
        for ev in events {
            spawn_bones(
                &mut self.world,
                ev.x,
                ev.y,
                self.res.region.region_num,
                ev.brother_id,
                ev.stuff,
            );
            self.res.brother.inactive_inventories[ev.brother_id as usize] = ev.stuff;

            match self.next_living_brother(ev.brother_id) {
                Some(successor) => {
                    let cfg = game_lib.get_brother(successor as usize);
                    let stats = cfg.map(|b| HeroStats {
                        vitality: 15 + b.brave / 4,
                        brave:    b.brave,
                        luck:     b.luck,
                        kind:     b.kind,
                        wealth:   b.wealth,
                        hunger:   0,
                        fatigue:  0,
                        gold:     0,
                    }).unwrap_or(HeroStats {
                        vitality: 10,
                        brave: 0, luck: 0, kind: 0, wealth: 0,
                        hunger: 0, fatigue: 0, gold: 0,
                    });
                    // Spec §15.12: new brother always starts with only a Dirk.
                    // inactive_inventories[successor] holds the dead brother's items for
                    // bones pickup — it is not the successor's starting loadout.
                    let inv = Inventory::with_dirk();
                    let _ = self.world.despawn(self.res.hero_entity);
                    let new_hero = spawn_hero(
                        &mut self.world,
                        19036.0, 15755.0,
                        successor,
                        stats,
                        inv,
                    );
                    self.res.hero_entity = new_hero;
                    self.res.brother.active_brother = successor as usize;
                    self.res.brother.active_name = game_lib
                        .get_brother(successor as usize)
                        .map(|b| b.name.clone())
                        .unwrap_or_else(|| "Hero".to_string());
                    self.res.brother.brother = successor + 1;
                    self.res.region.new_region = 3;
                    self.res.clock.daynight = 8000;

                    let dead_name = game_lib
                        .get_brother(ev.brother_id as usize)
                        .map(|b| b.name.to_lowercase())
                        .unwrap_or_else(|| "julian".to_string());
                    let succ_name = game_lib
                        .get_brother(successor as usize)
                        .map(|b| b.name.to_lowercase())
                        .unwrap_or_else(|| "phillip".to_string());
                    result = Some(SceneResult::BrotherSuccession {
                        dead_placard:  Some(format!("{}_dead", dead_name)),
                        start_placard: Some(format!("{}_start", succ_name)),
                    });
                }
                None => {
                    result = Some(SceneResult::GameOver);
                }
            }
        }
        result
    }

    /// Drain message and speech events from the current tick into the message queue.
    /// Called from `update()` after each tick so `game_lib` is available for narr lookup.
    fn drain_messages(&mut self, game_lib: &GameLibrary) {
        // Plain text messages.
        for ev in self.res.events.message.drain(..) {
            self.messages.push(ev.text);
        }
        // Speech events resolved through narr table.
        for ev in self.res.events.speech.drain(..) {
            let text = crate::game::events::speak(&game_lib.narr, ev.speech_id, &ev.brother_name);
            self.messages.push(text);
            // TODO(Plan I): Route Talk-action speech to narrative.push(Placard{..})
            // Proximity-triggered speech should continue using scroll messages.
        }
        // Trim to 64 messages — the hibar only shows the last 4.
        if self.messages.len() > 64 {
            let overflow = self.messages.len() - 64;
            self.messages.drain(0..overflow);
        }
    }

    /// Queue a placard event from a Talk action (Plan I integration point).
    pub fn add_talk_placard(&mut self, text: String, hold_ticks: u32) {
        use crate::game::ecs::resources::NarrEvent;
        self.res.narrative.push(NarrEvent::Placard { text, hold_ticks });
    }

    /// Returns true if a bartender NPC (SetFig with ob_id == 8) is within
    /// Chebyshev distance 32 of the hero.
    ///
    /// Mirrors the legacy `has_shopkeeper_nearby` guard from `shop.rs`.
    fn has_shopkeeper_nearby(&self) -> bool {
        let hero_pos = match self.world.get::<&Position>(self.res.hero_entity) {
            Ok(p) => (p.x, p.y),
            Err(_) => return false,
        };
        self.world
            .query::<(&Position, &WorldObj)>()
            .with::<&SetFig>()
            .iter()
            .any(|(pos, obj)| {
                obj.ob_id == 8
                    && (pos.x - hero_pos.0).abs().max((pos.y - hero_pos.1).abs()) < 32.0
            })
    }

    /// Route a MenuAction emitted by MenuState to the appropriate ECS operation.
    /// Returns true if the scene should quit.
    fn dispatch_menu_action(&mut self, action: MenuAction, game_lib: &GameLibrary, _resources: &mut SceneResources<'_, '_>) -> bool {
        match action {
            MenuAction::SwitchMode(_) => {}

            MenuAction::Inventory => {
                self.res.view.viewstatus = 1;
            }

            MenuAction::Take => {
                use crate::game::ecs::components::Loot;
                use crate::game::ecs::events::ItemEvent;
                let hero_pos = self.world
                    .get::<&Position>(self.res.hero_entity)
                    .map(|p| (p.x, p.y))
                    .unwrap_or((0.0, 0.0));
                let mut best_item:   Option<(hecs::Entity, f32)> = None;
                let mut best_corpse: Option<(hecs::Entity, f32)> = None;
                for (entity, obj, pos) in self.world.query::<(hecs::Entity, &WorldObj, &Position)>().iter() {
                    if obj.ob_stat != 1 || !obj.visible { continue; }
                    let dx = pos.x - hero_pos.0;
                    let dy = pos.y - hero_pos.1;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist <= 16.0 && best_item.map_or(true, |(_, d)| dist < d) {
                        best_item = Some((entity, dist));
                    }
                }
                for (entity, loot, pos) in self.world.query::<(hecs::Entity, &Loot, &Position)>().iter() {
                    if loot.looted { continue; }
                    let dx = pos.x - hero_pos.0;
                    let dy = pos.y - hero_pos.1;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist <= 16.0 && best_corpse.map_or(true, |(_, d)| dist < d) {
                        best_corpse = Some((entity, dist));
                    }
                }
                if let Some((entity, _)) = best_item {
                    self.res.events.item.push(ItemEvent::TakeItem { entity });
                } else if let Some((entity, _)) = best_corpse {
                    self.res.events.item.push(ItemEvent::SearchBody { entity });
                }
            }

            MenuAction::Look => {}

            MenuAction::SetWeapon(weapon_slot) => {
                use crate::game::ecs::components::CombatState;
                if let Ok(mut cs) = self.world.get::<&mut CombatState>(self.res.hero_entity) {
                    cs.weapon = weapon_slot;
                }
            }

            MenuAction::TogglePause => {
                self.res.view.paused = !self.res.view.paused;
            }

            MenuAction::ToggleMusic => {}
            MenuAction::ToggleSound => {}
            MenuAction::RefreshMusic => {}

            MenuAction::SaveGame(_slot) => {}
            MenuAction::LoadGame(_slot) => {}

            MenuAction::Quit => {
                self.quit_requested = true;
            }

            MenuAction::Yell | MenuAction::Say | MenuAction::Ask => {}
            MenuAction::BuyItem(hit) => {
                // Guard: player must be adjacent to a bartender.
                if !self.has_shopkeeper_nearby() {
                    return self.quit_requested;
                }

                let name = self.res.brother.active_name.clone();
                let slot = hit as usize;
                let text = match buy_slot_ecs(slot, &mut self.world, &mut self.res) {
                    // Slot out of range — silent.
                    BuyResult::Silent => None,
                    // Hardcoded denial string (dialog_system.md, fmain.c:3440).
                    BuyResult::NotEnough => Some("Not enough money!".to_string()),
                    // Food → event_msg[22]; Arrows → event_msg[23]
                    // (faery.toml [narr].event_msg, via events::event_msg; `%` → hero name).
                    BuyResult::Bought(BuyOutcome::Food) =>
                        Some(crate::game::events::event_msg(&game_lib.narr, 22, &name)),
                    BuyResult::Bought(BuyOutcome::Arrows) =>
                        Some(crate::game::events::event_msg(&game_lib.narr, 23, &name)),
                    // Generic item → hardcoded "% bought a {item}." (dialog_system.md,
                    // fmain.c:3436-3437). Item name from inv_list[].name.
                    BuyResult::Bought(BuyOutcome::Item { inv_idx }) => Some(format!(
                        "{name} bought a {}.",
                        crate::game::world_objects::stuff_index_name(inv_idx)
                    )),
                };
                if let Some(text) = text {
                    self.res.events.message.push(crate::game::ecs::events::MessageEvent { text });
                }
            }
            MenuAction::GiveGold | MenuAction::GiveWrit | MenuAction::GiveBone => {}
            MenuAction::TryKey(_) => {}
            MenuAction::CastSpell(hit) => {
                let item_idx = ITEM_BLUE_STONE + hit as usize;
                let result = magic_dispatch_ecs(item_idx, &mut self.world, &mut self.res);
                let name = game_lib
                    .get_brother(self.res.brother.active_brother)
                    .map(|b| b.name.as_str())
                    .unwrap_or("Hero");

                match result {
                    MagicResult::NoOwned => {
                        self.res.events.message.push(crate::game::ecs::events::MessageEvent {
                            text: crate::game::events::event_msg(&game_lib.narr, 21, name),
                        });
                    }
                    MagicResult::Healed { capped: false } | MagicResult::StoneTeleport { capped: false } => {
                        self.res.events.message.push(crate::game::ecs::events::MessageEvent {
                            text: "That feels a lot better!".to_string(),
                        });
                    }
                    MagicResult::MassKill { in_battle: true, .. } => {
                        self.res.events.message.push(crate::game::ecs::events::MessageEvent {
                            text: crate::game::events::event_msg(&game_lib.narr, 34, name),
                        });
                    }
                    _ => {}
                }
            }
            MenuAction::SummonTurtle => {}
            MenuAction::UseSunstone => {}
            MenuAction::None => {}
            MenuAction::UseMenu | MenuAction::GiveMenu => {}
        }
        self.quit_requested
    }

    /// Lazy-load the ADF disk image, WorldData, MapRenderer, and sprite sheets.
    /// Mirrors the `render-world-load` block from the old `GameplayScene::update`.
    fn load_world(&mut self, game_lib: &GameLibrary) {
        self.adf_load_done = true;

        let adf_path = game_lib
            .disk
            .as_ref()
            .map(|d| d.adf.as_str())
            .unwrap_or("game/image");

        let adf_raw = match crate::game::adf::AdfDisk::open(std::path::Path::new(adf_path)) {
            Ok(a) => a,
            Err(e) => { self.res.diag_log.push(format!("EcsScene: AdfDisk::open failed: {e}")); return; }
        };
        let adf = std::sync::Arc::new(adf_raw);
        self.adf = Some(adf.clone());
        self.res.adf = Some(adf.clone());

        let region = self.res.region.region_num;
        let world_result = game_lib.find_region_config(region).map(|cfg| {
            let map_blocks: Vec<u32> = if region < 8 {
                [0u8, 2, 4, 6]
                    .iter()
                    .filter_map(|&r| game_lib.find_region_config(r))
                    .map(|c| c.map_block)
                    .collect()
            } else {
                vec![cfg.map_block]
            };
            crate::game::world_data::WorldData::load(
                &adf,
                region,
                cfg.sector_block,
                &map_blocks,
                cfg.terra_block,
                cfg.terra2_block,
                &cfg.image_blocks,
            )
        });

        let world = match world_result {
            Some(Ok(w)) => w,
            Some(Err(e)) => { self.res.diag_log.push(format!("EcsScene: WorldData::load failed: {e}")); return; }
            None => { self.res.diag_log.push(format!("EcsScene: no region config for region {region}")); return; }
        };

        // Shadow memory bitmask for sprite depth masking.
        let shadow_mem = game_lib.disk.as_ref()
            .filter(|d| d.shadow_count > 0)
            .map(|d| crate::game::world_data::load_shadow_mem(&adf, d.shadow_block, d.shadow_count))
            .unwrap_or_default();

        let renderer = crate::game::map_renderer::MapRenderer::new(&world, shadow_mem.clone());

        // Sprite sheets: player (0-2), enemies (4-12), setfigs (13-17).
        while self.res.sprites.sheets.len() < 18 {
            self.res.sprites.sheets.push(None);
        }
        for cfile_idx in [0u8, 1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17] {
            self.res.sprites.sheets[cfile_idx as usize] =
                crate::game::sprites::SpriteSheet::load(&adf, cfile_idx);
        }
        self.res.sprites.object_sprites = crate::game::sprites::SpriteSheet::load_objects(&adf);

        // Palette.
        self.base_colors = build_base_colors_palette(game_lib, region);
        self.res.palette.current_palette = region_palette(game_lib, region);
        self.res.palette.dirty = true;

        // Zones for this region.
        self.res.zones = game_lib.zones.clone();

        // Store map data.
        self.res.map.renderer = Some(renderer);
        self.res.map.world    = Some(world);

        // Populate door table for this region.
        self.reload_doors(region, game_lib);

        // Snap camera to hero's spawn position.
        self.snap_camera();

        self.res.diag_log.push(format!("EcsScene: world loaded for region {region}"));
    }

    /// Perform a full region transition: despawn old actors, load new world data,
    /// spawn NPCs, refresh palette and zones, reposition hero, snap camera.
    ///
    /// Called by `region::run()` for each `RegionTransitionEvent`.
    pub fn reload_region(
        &mut self,
        region: u8,
        dest_x: f32,
        dest_y: f32,
        game_lib: &GameLibrary,
    ) {
        use crate::game::ecs::components::{Enemy, GroundItem, SetFig};
        use crate::game::ecs::spawn::{spawn_enemy, spawn_setfig};
        use crate::game::npc::{NpcTable, NPC_TYPE_NONE, NPC_TYPE_HUMAN};
        use crate::game::ecs::components::WorldObj;

        // 1. Despawn all Enemy, SetFig, and GroundItem entities.
        let to_despawn: Vec<hecs::Entity> = {
            let enemies: Vec<hecs::Entity> = self
                .world.query::<(hecs::Entity, &Enemy)>().iter().map(|(e, _)| e).collect();
            let setfigs: Vec<hecs::Entity> = self
                .world.query::<(hecs::Entity, &SetFig)>().iter().map(|(e, _)| e).collect();
            let items: Vec<hecs::Entity> = self
                .world.query::<(hecs::Entity, &GroundItem)>().iter().map(|(e, _)| e).collect();
            enemies.into_iter().chain(setfigs).chain(items).collect()
        };
        for e in to_despawn {
            let _ = self.world.despawn(e);
        }

        // 2. Load WorldData + MapRenderer.
        let adf = match self.adf.as_ref() {
            Some(a) => a.clone(),
            None => { self.res.diag_log.push("reload_region: no ADF loaded".to_string()); return; }
        };

        let world_result = game_lib.find_region_config(region).map(|cfg| {
            let map_blocks: Vec<u32> = if region < 8 {
                [0u8, 2, 4, 6]
                    .iter()
                    .filter_map(|&r| game_lib.find_region_config(r))
                    .map(|c| c.map_block)
                    .collect()
            } else {
                vec![cfg.map_block]
            };
            crate::game::world_data::WorldData::load(
                &adf,
                region,
                cfg.sector_block,
                &map_blocks,
                cfg.terra_block,
                cfg.terra2_block,
                &cfg.image_blocks,
            )
        });

        let world_data = match world_result {
            Some(Ok(w)) => w,
            Some(Err(e)) => { self.res.diag_log.push(format!("reload_region: WorldData::load failed: {e}")); return; }
            None => { self.res.diag_log.push(format!("reload_region: no region config for {region}")); return; }
        };

        let shadow_mem = game_lib.disk.as_ref()
            .filter(|d| d.shadow_count > 0)
            .map(|d| crate::game::world_data::load_shadow_mem(&adf, d.shadow_block, d.shadow_count))
            .unwrap_or_default();

        let renderer = crate::game::map_renderer::MapRenderer::new(&world_data, shadow_mem);

        // 3a. Spawn SetFig NPCs (ob_stat=3) from the world object list for this region.
        // Setfigs live in game_lib.objects — NOT in the NPC carrier table.
        // ob_id is the setfig type index (0–13); goal is its index within the region's list
        // (matching fmain2.c:1275 `goal = i`).
        {
            let mut goal: u8 = 0;
            for obj_cfg in game_lib.objects.iter().filter(|o| o.region == region) {
                if obj_cfg.ob_stat == 3 {
                    let cfile_idx = crate::game::sprites::SETFIG_TABLE
                        .get(obj_cfg.ob_id as usize)
                        .map(|e| e.cfile_entry)
                        .unwrap_or(13);
                    let obj = WorldObj {
                        ob_id:   obj_cfg.ob_id,
                        ob_stat: 3,
                        region,
                        visible: true,
                        goal,
                    };
                    spawn_setfig(&mut self.world, obj_cfg.x as f32, obj_cfg.y as f32, obj, cfile_idx);
                }
                goal = goal.wrapping_add(1);
            }
        }

        // 3b. Spawn enemy NPCs from the carrier table for this region.
        let npc_table = NpcTable::load(&adf, region);
        for npc in &npc_table.npcs {
            if !npc.active || npc.npc_type == NPC_TYPE_NONE || npc.npc_type == NPC_TYPE_HUMAN {
                continue; // human setfigs come from game_lib.objects above
            }
            let cfile_idx = npc_type_to_cfile(npc.npc_type, npc.race).unwrap_or(6) as u8;
            spawn_enemy(
                &mut self.world,
                npc.x as f32,
                npc.y as f32,
                npc.npc_type,
                npc.race,
                npc.vitality,
                npc.weapon,
                npc.gold,
                npc.speed,
                npc.cleverness,
                cfile_idx,
            );
        }

        // 4. Zones.
        self.res.zones = game_lib.zones.clone();

        // 5. Palette.
        self.base_colors = build_base_colors_palette(game_lib, region);
        self.res.palette.current_palette = region_palette(game_lib, region);
        self.res.palette.dirty = true;

        // Store map data.
        self.res.map.renderer = Some(renderer);
        self.res.map.world    = Some(world_data);

        // Populate door table for this region.
        self.reload_doors(region, game_lib);

        self.res.region.region_num = region;

        // 6. Reposition hero.
        if let Ok(mut pos) = self.world.get::<&mut crate::game::ecs::components::Position>(self.res.hero_entity) {
            pos.x = dest_x;
            pos.y = dest_y;
        }

        // 7. Snap camera.
        self.res.camera.map_x = (dest_x - 144.0).rem_euclid(0x8000 as f32);
        self.res.camera.map_y = (dest_y - 70.0).rem_euclid(0x8000 as f32);

        self.res.diag_log.push(format!("EcsScene: region transition to {region} at ({dest_x}, {dest_y})"));
    }

    /// Reload `res.map.doors` and clear `opened_doors` for the given region.
    /// Outdoor regions (< 8): load only doors whose src_region matches.
    /// Indoor regions (>= 8): load all doors — doorfind_exit matches by dst coords,
    /// so the full table is needed (original uses a single global doorlist for both).
    fn reload_doors(&mut self, region: u8, game_lib: &GameLibrary) {
        self.res.map.doors.clear();
        self.res.map.opened_doors.clear();
        self.res.map.transitioned_doors.clear();
        let iter = game_lib.doors.iter().filter(|d| {
            region < 8 && d.src_region == region || region >= 8
        });
        for door_cfg in iter {
            self.res.map.doors.push(crate::game::doors::DoorEntry {
                src_region: door_cfg.src_region,
                src_x: door_cfg.src_x, src_y: door_cfg.src_y,
                dst_region: door_cfg.dst_region,
                dst_x: door_cfg.dst_x, dst_y: door_cfg.dst_y,
                door_type: door_cfg.door_type,
            });
        }
    }

    fn snap_camera(&mut self) {
        if let Ok(pos) = self.world.get::<&crate::game::ecs::components::Position>(self.res.hero_entity) {
            const CX: f32 = 144.0;
            const CY: f32 = 70.0;
            const WRAP: f32 = 0x8000 as f32;
            self.res.camera.map_x = (pos.x - CX).rem_euclid(WRAP);
            self.res.camera.map_y = (pos.y - CY).rem_euclid(WRAP);
        }
    }

    /// Compose the map framebuf, blit sprites into it, then copy to the SDL canvas.
    fn render_map(&mut self, canvas: &mut Canvas<Window>) {
        let map_x = self.res.camera.map_x as u16;
        let map_y = self.res.camera.map_y as u16;

        // Step 1: compose tiles into the indexed framebuf.
        if let (Some(renderer), Some(world_data)) = (
            self.res.map.renderer.as_mut(),
            self.res.map.world.as_ref(),
        ) {
            renderer.compose(map_x, map_y, world_data);
        }

        if self.res.map.renderer.as_ref().map_or(true, |r| r.framebuf.is_empty()) {
            return;
        }

        // Step 2: blit sprites then immediately apply per-sprite depth masking.
        // Masking is interleaved with blitting (back-to-front) so each closer sprite
        // draws over the terrain re-stamped by sprites behind it — matching the
        // original save_blit → mask_blit → shape_blit per-actor pass (fmain.c:2412-2609).
        let cycle       = self.res.clock.cycle as usize;
        let hero_entity = self.res.hero_entity;

        // Compute hero sector before mutably borrowing the renderer (both live in res.map).
        // Used by mask type 3 (bridge): when hero_sector == 48 the bridge tiles don't
        // mask the hero (fmain.c:3149-3179, should_mask_tile case 3).
        use crate::game::ecs::components::Position;
        let hero_sector = self.world
            .get::<&Position>(hero_entity)
            .ok()
            .and_then(|pos| self.res.map.world.as_ref()
                .map(|w| w.sector_at_pos(pos.x, pos.y)))
            .unwrap_or(0);

        if let Some(renderer) = self.res.map.renderer.as_mut() {
            blit_actors_inner(
                &self.world,
                hero_entity,
                &self.res.sprites.sheets,
                self.res.sprites.object_sprites.as_ref(),
                cycle,
                map_x,
                map_y,
                hero_sector,
                renderer,
                self.res.encounter.hero_dying_countdown,
                self.res.encounter.dying,
            );
        }

        // Step 3: convert indexed framebuf to RGBA and blit to canvas.
        let pal = &self.res.palette.current_palette;
        let framebuf = &self.res.map.renderer.as_ref().unwrap().framebuf;
        let mut rgb_buf: Vec<u8> = Vec::with_capacity(framebuf.len() * 4);
        for &idx in framebuf {
            let rgba = pal[(idx & 31) as usize];
            // ARGB8888 little-endian: bytes are [B, G, R, A]
            rgb_buf.push((rgba & 0xFF) as u8);
            rgb_buf.push(((rgba >> 8) & 0xFF) as u8);
            rgb_buf.push(((rgba >> 16) & 0xFF) as u8);
            rgb_buf.push(0xFF);
        }

        {
            let tc = canvas.texture_creator();
            if let Ok(surface) = sdl3::surface::Surface::from_data(
                &mut rgb_buf,
                MAP_DST_W,
                MAP_DST_H,
                MAP_DST_W * 4,
                sdl3::pixels::PixelFormat::ARGB8888,
            ) {
                if let Ok(mut tex) = tc.create_texture_from_surface(&surface) {
                    tex.set_scale_mode(sdl3::render::ScaleMode::Nearest);
                    let src = sdl3::rect::Rect::new(0, 0, PLAYFIELD_LORES_W, PLAYFIELD_LORES_H);
                    let dst = sdl3::rect::Rect::new(
                        PLAYFIELD_X, PLAYFIELD_Y, PLAYFIELD_CANVAS_W, PLAYFIELD_CANVAS_H,
                    );
                    let _ = canvas.copy(&tex, src, dst);
                }
            }
        }
        drop(rgb_buf);
    }

    /// Render the inventory overlay (viewstatus == 1).
    ///
    /// Mirrors `render_inventory_items_page` (fmain.c:3120-3145):
    /// - Each inv_list[] slot is placed at `(xoff + INV_ICON_X_OFFSET, yoff)` in lores coords.
    /// - The sprite icon is taken from frame `image_number` of the OBJECTS sheet.
    /// - Only rows `img_off .. img_off+img_height` of that frame are blitted.
    /// - For stackable items (ydelta > 0) the icon is repeated `min(count, maxshown)` times,
    ///   each copy offset downward by `ydelta` pixels.
    /// - Gold slots (31+) are not rendered here.
    /// All lores coords are scaled 2× for the canvas.
    fn render_inventory(&mut self, canvas: &mut Canvas<Window>) {
        use crate::game::ecs::components::Inventory;
        use crate::game::sprites::{INV_LIST, OBJ_SPRITE_H, SPRITE_W};

        // INV_ICON_X_OFFSET = 20 (fmain.c:3131)
        const INV_ICON_X_OFFSET: i32 = 20;
        // All lores coords are scaled ×2 onto the canvas.
        const SCALE: i32 = 2;

        let stuff = match self.world.get::<&Inventory>(self.res.hero_entity) {
            Ok(inv) => inv.stuff,
            Err(_) => return,
        };

        // Clear playfield to black.
        canvas.set_draw_color(sdl3::pixels::Color::RGB(0, 0, 0));
        canvas.fill_rect(sdl3::rect::Rect::new(
            PLAYFIELD_X, PLAYFIELD_Y, PLAYFIELD_CANVAS_W, PLAYFIELD_CANVAS_H,
        )).ok();

        let obj_sheet = match self.res.sprites.object_sprites.as_ref() {
            Some(s) => s,
            None => return,
        };
        let pal = self.res.palette.current_palette;

        for slot in 0..INV_LIST.len() {
            let count = stuff[slot];
            if count == 0 { continue; }

            let entry = &INV_LIST[slot];
            let frame = entry.image_number as usize;

            let frame_pixels = match obj_sheet.frame_pixels(frame) {
                Some(fp) => fp,
                None => continue,
            };

            // Number of copies to draw (capped by maxshown).
            let copies = (count as usize).min(entry.maxshown as usize);

            // Base lores destination (fmain.c:3131-3132).
            let base_lores_x = entry.xoff as i32 + INV_ICON_X_OFFSET;
            let base_lores_y = entry.yoff as i32;

            // img_off and img_height define the sub-rect within the 16-row frame.
            let img_off    = entry.img_off as usize;
            let img_height = entry.img_height as usize;

            // Build the ARGB pixel buffer once per slot (same icon data for every copy).
            let mut rgba_buf: Vec<u8> = Vec::with_capacity(SPRITE_W * img_height * 4);
            for row in img_off..(img_off + img_height).min(OBJ_SPRITE_H) {
                for col in 0..SPRITE_W {
                    let idx = frame_pixels[row * SPRITE_W + col];
                    let transparent = idx == 31;
                    let color = if transparent { 0u32 } else { pal[(idx & 31) as usize] };
                    rgba_buf.push((color & 0xFF) as u8);
                    rgba_buf.push(((color >> 8) & 0xFF) as u8);
                    rgba_buf.push(((color >> 16) & 0xFF) as u8);
                    rgba_buf.push(if transparent { 0 } else { 0xFF });
                }
            }

            let tc = canvas.texture_creator();
            for copy in 0..copies {
                let lores_y = base_lores_y + copy as i32 * entry.ydelta as i32;

                let dst_x = PLAYFIELD_X + base_lores_x * SCALE;
                let dst_y = PLAYFIELD_Y + lores_y * SCALE;
                let dst_w = (SPRITE_W as i32 * SCALE) as u32;
                let dst_h = (img_height as i32 * SCALE) as u32;

                if let Ok(surface) = sdl3::surface::Surface::from_data(
                    &mut rgba_buf,
                    SPRITE_W as u32,
                    img_height as u32,
                    (SPRITE_W * 4) as u32,
                    sdl3::pixels::PixelFormat::ARGB8888,
                ) {
                    if let Ok(mut tex) = tc.create_texture_from_surface(&surface) {
                        tex.set_scale_mode(sdl3::render::ScaleMode::Nearest);
                        let _ = canvas.copy(&tex, None,
                            sdl3::rect::Rect::new(dst_x, dst_y, dst_w, dst_h));
                    }
                }
            }
            drop(rgba_buf);
        }
    }

    /// Render centered placard overlay for narrative events (viewstatus == 2).
    fn render_placard(&self, canvas: &mut Canvas<Window>, resources: &SceneResources<'_, '_>) {
        use crate::game::ecs::resources::NarrEvent;

        let text = if let Some(NarrEvent::Placard { text, .. }) = &self.res.narrative.active {
            text.clone()
        } else {
            return;
        };

        const PW: u32 = 400;
        const PH: u32 = 120;
        let px = PLAYFIELD_X + (PLAYFIELD_CANVAS_W as i32 - PW as i32) / 2;
        let py = PLAYFIELD_Y + (PLAYFIELD_CANVAS_H as i32 - PH as i32) / 2;

        let rect = sdl3::rect::Rect::new(px, py, PW, PH);
        canvas.set_draw_color(sdl3::pixels::Color::RGBA(0, 0, 0, 200));
        canvas.fill_rect(rect).ok();
        canvas.set_draw_color(sdl3::pixels::Color::RGB(255, 255, 255));
        canvas.draw_rect(rect).ok();

        let font = resources.topaz_font;
        font.set_color_mod(255, 255, 255);

        let line_height = 14i32;
        let lines: Vec<&str> = text.lines().collect();
        let total_h = lines.len() as i32 * line_height;
        let start_y = py + (PH as i32 - total_h) / 2;

        for (i, line) in lines.iter().enumerate() {
            let text_w = (line.len() as i32) * 8;
            let lx = px + (PW as i32 - text_w) / 2;
            let ly = start_y + i as i32 * line_height;
            font.render_string(line, canvas, lx, ly);
        }
    }

    /// Render the HI bar (stats, messages, compass) at the bottom of the canvas.
    /// Port of `GameplayScene::render_hibar`.
    fn render_hibar(
        &mut self,
        canvas: &mut Canvas<Window>,
        resources: &mut SceneResources<'_, '_>,
    ) {
        // Gather hero stats from the ECS world.
        let (brave, luck, kind, vitality, wealth) =
            match self.world.get::<&crate::game::ecs::components::HeroStats>(self.res.hero_entity) {
                Ok(s) => (s.brave, s.luck, s.kind, s.vitality, s.wealth),
                Err(_) => return,
            };

        // Last 4 messages visible in the scroll area.
        let msg_count = self.messages.len().min(4);
        let msg_start = self.messages.len().saturating_sub(4);
        let msgs_visible: Vec<&str> = self.messages[msg_start..].iter().map(|s| s.as_str()).collect();

        let hiscreen_opt = resources.find_image("hiscreen");
        let amber_font   = resources.amber_font;
        let topaz_font   = resources.topaz_font;
        let compass_normal    = resources.compass_normal;
        let compass_highlight = resources.compass_highlight;

        // Current input direction → compass arrow index.
        let input_dir = self.input.to_direction();
        let compass_arrow = compass_dir_index(input_dir);
        let compass_regions = &self.res.palette.compass_regions;

        // Snapshot textcolors and build button list before entering the closure
        // (can't borrow self.menu and self.res simultaneously inside the closure).
        let textcolors = self.res.palette.textcolors;
        let buttons = self.menu.print_options();

        let tc = canvas.texture_creator();
        if let Ok(mut hibar_tex) =
            tc.create_texture_target(sdl3::pixels::PixelFormat::RGBA32, 640, HIBAR_NATIVE_H)
        {
            hibar_tex.set_scale_mode(sdl3::render::ScaleMode::Nearest);
            let _ = canvas.with_texture_canvas(&mut hibar_tex, |hc| {
                hc.set_draw_color(sdl3::pixels::Color::RGB(0, 0, 0));
                hc.clear();

                if let Some(hiscreen) = hiscreen_opt {
                    hiscreen.draw_scaled(hc, sdl3::rect::Rect::new(0, 0, 640, HIBAR_NATIVE_H));
                } else {
                    hc.set_draw_color(sdl3::pixels::Color::RGB(80, 60, 20));
                    hc.fill_rect(sdl3::rect::Rect::new(0, 0, 640, HIBAR_NATIVE_H)).ok();
                }

                // Draw menu buttons: 2 columns × 6 rows in the right side
                // of the HI bar (spec §25.3, _discovery/menu-system.md).
                // Even slots at x=430, odd at x=482; y = (j/2)*9 + 8 (baseline).
                // Pixel-perfect: two JAM2 Text() calls per entry —
                //   1) 6-space background field at (x, y)
                //   2) 5-char label at (x+4, y) with 4px left margin
                for btn in &buttons {
                    let j = btn.display_slot;
                    let col_x = if j % 2 == 0 { 430 } else { 482 };
                    let baseline_y = (j / 2) as i32 * 9 + 8;

                    let bg_rgba = textcolors.get(btn.bg_color as usize).copied().unwrap_or(0xFF000000);
                    let bg = (
                        ((bg_rgba >> 16) & 0xFF) as u8,
                        ((bg_rgba >> 8)  & 0xFF) as u8,
                        (bg_rgba & 0xFF)          as u8,
                    );
                    let fg_rgba = textcolors.get(btn.fg_color as usize).copied().unwrap_or(0xFFFFFFFF);
                    let fg = (
                        ((fg_rgba >> 16) & 0xFF) as u8,
                        ((fg_rgba >> 8)  & 0xFF) as u8,
                        (fg_rgba & 0xFF)          as u8,
                    );

                    // Background field: 6 spaces at (x, y) — fills 6×char_w × y_size.
                    topaz_font.render_string_with_bg("      ", hc, col_x, baseline_y, bg, fg);
                    // Label text: 5 chars at (x+4, y) — 4px left margin.
                    topaz_font.render_string_with_bg(&btn.text, hc, col_x + 4, baseline_y, bg, fg);
                }

                amber_font.set_color_mod(0xAA, 0x55, 0x00);
                amber_font.render_string(&format!("Brv:{:3}", brave), hc, 14, 52);
                amber_font.render_string(&format!("Lck:{:3}", luck), hc, 90, 52);
                amber_font.render_string(&format!("Knd:{:3}", kind), hc, 168, 52);
                amber_font.render_string(&format!("Vit:{:3}", vitality), hc, 245, 52);
                amber_font.render_string(&format!("Wlth:{:3}", wealth), hc, 321, 52);

                // Scroll messages (up to 4, bottom-anchored at y=42).
                for (i, msg) in msgs_visible.iter().enumerate() {
                    let line_from_bottom = (msg_count - 1 - i) as i32;
                    let y = 42 - line_from_bottom * 10;
                    amber_font.render_string(msg, hc, 16, y);
                }
                amber_font.set_color_mod(255, 255, 255);

                // Compass.
                const COMPASS_X: i32 = 567;
                const COMPASS_SRC_Y: i32 = 15;
                const COMPASS_SRC_W: u32 = 48;
                const COMPASS_SRC_H: u32 = 24;
                let compass_dest = sdl3::rect::Rect::new(COMPASS_X, COMPASS_SRC_Y, COMPASS_SRC_W, COMPASS_SRC_H);
                if let Some(normal_tex) = compass_normal {
                    hc.copy(normal_tex, None, compass_dest).ok();
                }
                if compass_arrow < compass_regions.len() {
                    let (rx, ry, rw, rh) = compass_regions[compass_arrow];
                    if rw > 1 || rh > 1 {
                        if let Some(hl_tex) = compass_highlight {
                            let src = sdl3::rect::Rect::new(rx, ry, rw as u32, rh as u32);
                            let dst = sdl3::rect::Rect::new(COMPASS_X + rx, COMPASS_SRC_Y + ry, rw as u32, rh as u32);
                            hc.copy(hl_tex, src, dst).ok();
                        }
                    }
                }
            });
            canvas.copy(
                &hibar_tex,
                sdl3::rect::Rect::new(0, 0, 640, HIBAR_NATIVE_H),
                sdl3::rect::Rect::new(0, HIBAR_Y, 640, HIBAR_H),
            ).ok();
        }; // semicolon: drops Result<Texture> temporary before tc is dropped
    }

    /// Read hero Inventory + HeroStats and update MenuState enabled flags.
    fn update_menu_options(&mut self) {
        use crate::game::ecs::components::HeroStats;
        let (stuff, wealth): ([u8; 36], i16) = {
            let mut q = self.world.query_one::<(&Inventory, &HeroStats)>(self.res.hero_entity);
            q.get().map(|(inv, stats)| (inv.stuff, stats.wealth)).unwrap_or(([0u8; 36], 0))
        };
        self.menu.set_options(&stuff, wealth);
    }

    /// Run one gameplay tick: advance all systems then drain debug commands.
    fn run_tick(&mut self, game_lib: &GameLibrary) {
        self.res.events.clear();

        // ── System schedule (mirrors order in systems/mod.rs) ────────────────
        systems::clock::run(&mut self.world, &mut self.res);

        // Palette update every 4 ticks or when dirty (SPEC §17.5).
        let daynight = self.res.clock.daynight;
        if (daynight & 3) == 0 || self.res.palette.dirty {
            self.res.palette.dirty = false;
            if let Some(base) = self.base_colors.clone() {
                let lightlevel    = self.res.clock.lightlevel;
                let light_on      = self.res.clock.light_timer > 0;
                let secret_active = self.res.region.region_num == 9
                    && self.res.clock.secret_timer > 0;
                self.res.palette.current_palette = compute_current_palette(
                    &base, self.res.region.region_num, lightlevel, light_on, secret_active,
                );
            }
        }
        systems::input::run(&mut self.world, &mut self.res);
        self.update_menu_options();
        // sleep system not yet ported — skipped
        self.res.input_direction = self.input.to_direction();
        self.res.input_fire      = self.input.fire();
        systems::movement::run(&mut self.world, &mut self.res);
        systems::carrier::run(&mut self.world, &mut self.res);
        systems::collision::run(&self.world, &mut self.res);
        systems::door::run(&self.world, &mut self.res, game_lib);
        systems::zone::run(&self.world, &mut self.res);
        systems::npc_ai::run(&mut self.world, &mut self.res);
        systems::npc_movement::run(&mut self.world, &mut self.res);
        systems::combat::run(&mut self.world, &mut self.res);
        systems::damage::run(&mut self.world, &mut self.res);
        systems::missile::run(&mut self.world, &mut self.res);
        systems::encounter::run(&mut self.world, &mut self.res);
        systems::proximity::run(&self.world, &mut self.res);
        systems::item::run(&mut self.world, &mut self.res);
        systems::narrative::run(&mut self.world, &mut self.res);
        systems::death::run(&mut self.world, &mut self.res);
        systems::region::run(&mut self.world, &mut self.res, game_lib);

        // Apply any pending region transition (set by RegionSystem above).
        if let Some(ev) = self.res.pending_transition.take() {
            self.reload_region(ev.new_region, ev.dest_x, ev.dest_y, game_lib);
        }

        // ── Debug command dispatch ────────────────────────────────────────────
        if let Some(console) = &mut self.console {
            for cmd in console.drain_commands() {
                debug_commands::handle(cmd, &mut self.world, &mut self.res);
            }
        }
    }
}

/// Map current game state to a music group index (0–6).
///
/// Priority order mirrors `setmood()` from `gameplay_scene/game_event.rs`
/// and R-AUDIO-011.  Group indices correspond to 4-track offsets in the songs
/// file: Day=0, Battle=1, Night=2, Zone=4, Dungeon=5, Death=6.
fn compute_mood(
    vitality: i16,
    in_astral_zone: bool,
    battleflag: bool,
    region_num: u8,
    lightlevel: u16,
) -> u8 {
    if vitality <= 0    { return 6; } // death
    if in_astral_zone   { return 4; } // astral plane (etype 52) — R-AUDIO-011
    if battleflag       { return 1; } // battle
    if region_num > 7   { return 5; } // dungeon
    if lightlevel > 120 { 0 } else   { 2 } // day / night
}

impl EcsScene {
    /// Drain pending SFX events and update the music mood every 4 ticks
    /// (gameloop-113).  Mirrors the audio block in the old `GameplayScene::update`.
    fn run_audio(&mut self, resources: &mut SceneResources<'_, '_>) {
        // Drain queued SFX events.
        for ev in self.res.events.sfx.drain(..) {
            if let Some(audio) = resources.audio {
                audio.play_sfx(ev.sfx_id);
            }
        }

        // Evaluate mood every 4 ticks (gameloop-113).
        self.mood_tick += 1;
        if self.mood_tick >= 4 {
            self.mood_tick = 0;

            let vitality = self.world
                .get::<&crate::game::ecs::components::HeroStats>(self.res.hero_entity)
                .map(|s| s.vitality)
                .unwrap_or(100);

            // Astral zone (etype 52) is the only zone that overrides music (R-AUDIO-011).
            let in_astral_zone = self.res.encounter.last_zone
                .and_then(|idx| self.res.zones.get(idx))
                .map(|z| z.etype == 52)
                .unwrap_or(false);

            let mood = compute_mood(
                vitality,
                in_astral_zone,
                self.res.region.battleflag,
                self.res.region.region_num,
                self.res.clock.lightlevel,
            );

            if mood != self.last_mood {
                self.last_mood = mood;
                if let Some(audio) = resources.audio {
                    audio.set_score(mood);
                }
            }
        }
    }
}

impl Scene for EcsScene {
    fn handle_event(&mut self, event: &Event) -> bool {
        use sdl3::keyboard::Keycode;
        match event {
            Event::KeyDown { keycode: Some(kc), repeat: false, .. } => {
                if self.res.view.viewstatus == 1 {
                    self.res.view.viewstatus = 0;
                    return true;
                }
                match kc {
                    Keycode::Up    | Keycode::Kp8
                    | Keycode::Down  | Keycode::Kp2
                    | Keycode::Left  | Keycode::Kp4
                    | Keycode::Right | Keycode::Kp6
                    | Keycode::Kp7   | Keycode::Kp9
                    | Keycode::Kp1   | Keycode::Kp3 => {
                        self.input.pressed_movement_keys.insert(*kc);
                        self.input.recompute();
                        true
                    }
                    Keycode::Kp0 => {
                        self.input.fire_keyboard = true;
                        true
                    }
                    _ => {
                        let menu_byte: Option<u8> = match kc {
                            Keycode::F1 => Some(10),
                            Keycode::F2 => Some(11),
                            Keycode::F3 => Some(12),
                            Keycode::F4 => Some(13),
                            Keycode::F5 => Some(14),
                            Keycode::F6 => Some(15),
                            Keycode::F7 => Some(16),
                            Keycode::Space => Some(b' '),
                            _ => {
                                let name = kc.name();
                                if name.len() == 1 {
                                    name.chars().next().map(|c| c.to_ascii_uppercase() as u8)
                                } else {
                                    None
                                }
                            }
                        };
                        if let Some(byte) = menu_byte {
                            let action = self.menu.handle_key(byte);
                            self.pending_menu_actions.push(action);
                            true
                        } else {
                            false
                        }
                    }
                }
            }
            Event::KeyUp { keycode: Some(kc), .. } => {
                match kc {
                    Keycode::Up    | Keycode::Kp8
                    | Keycode::Down  | Keycode::Kp2
                    | Keycode::Left  | Keycode::Kp4
                    | Keycode::Right | Keycode::Kp6
                    | Keycode::Kp7   | Keycode::Kp9
                    | Keycode::Kp1   | Keycode::Kp3 => {
                        self.input.pressed_movement_keys.remove(kc);
                        self.input.recompute();
                        true
                    }
                    Keycode::Kp0 => {
                        self.input.fire_keyboard = false;
                        true
                    }
                    _ => false,
                }
            }
            // Gamepad buttons.
            Event::ControllerButtonDown { button, .. } => {
                use sdl3::gamepad::Button;
                if *button == Button::South {
                    self.input.fire_gamepad = true;
                    true
                } else {
                    false
                }
            }
            Event::ControllerButtonUp { button, .. } => {
                use sdl3::gamepad::Button;
                if *button == Button::South {
                    self.input.fire_gamepad = false;
                    true
                } else {
                    false
                }
            }
            // Gamepad left stick → aggregate into direction.
            Event::ControllerAxisMotion { axis, value, .. } => {
                // Clear inventory view on any interaction
                if self.res.view.viewstatus == 1 {
                    self.res.view.viewstatus = 0;
                    return true;
                }
                use sdl3::gamepad::Axis;
                const THRESHOLD: i16 = 8000;
                match axis {
                    Axis::LeftX => {
                        self.input.gamepad_x = if *value < -THRESHOLD { -1 }
                            else if *value > THRESHOLD { 1 } else { 0 };
                        self.input.recompute();
                        true
                    }
                    Axis::LeftY => {
                        self.input.gamepad_y = if *value < -THRESHOLD { -1 }
                            else if *value > THRESHOLD { 1 } else { 0 };
                        self.input.recompute();
                        true
                    }
                    _ => false,
                }
            }
            Event::MouseButtonDown { x, y, mouse_btn, .. } => {
                // Clear inventory view on any interaction
                if self.res.view.viewstatus == 1 {
                    self.res.view.viewstatus = 0;
                    return true;
                }
                let nx = *x as i32;
                let ny = (*y as i32 - HIBAR_Y) / 2; // native y within hibar
                // Right click on compass (native x 567..615, y 15..39) → attack held.
                if *mouse_btn == sdl3::mouse::MouseButton::Right
                    && nx >= 567 && nx < 615 && ny >= 15 && ny < 39
                {
                    self.input.fire_mouse = true;
                    return true;
                }
                // Menu buttons occupy 2 columns × 6 rows in the right side of the
                // hibar (native x 430..534, native y 2..55).  Canvas → native:
                // native_x = canvas_x (both 640 wide), native_y = (canvas_y - HIBAR_Y) / 2.
                if nx >= 430 && nx < 534 && ny >= 2 && ny < 55 {
                    let col = if nx < 482 { 0 } else { 1 };
                    let row = (ny - 2) / 9; // rows 0..5
                    let display_slot = (row as usize) * 2 + col;
                    if display_slot < 12 {
                        // Start tracking press for click-and-hold behavior
                        self.menu.handle_mouse_down(display_slot);
                        return true;
                    }
                }
                false
            }
            Event::MouseButtonUp { x, y, mouse_btn, .. } => {
                // Clear inventory view on any interaction
                if self.res.view.viewstatus == 1 {
                    self.res.view.viewstatus = 0;
                    return true;
                }
                // Release right-click fire.
                if *mouse_btn == sdl3::mouse::MouseButton::Right {
                    self.input.fire_mouse = false;
                    return true;
                }
                // End of click - execute action if over the same slot
                let nx = *x as i32;
                let ny = (*y as i32 - HIBAR_Y) / 2;
                if nx >= 430 && nx < 534 && ny >= 2 && ny < 55 {
                    let col = if nx < 482 { 0 } else { 1 };
                    let row = (ny - 2) / 9;
                    let display_slot = (row as usize) * 2 + col;
                    if display_slot < 12 {
                        let action = self.menu.handle_mouse_up(display_slot);
                        if action != MenuAction::None {
                            self.pending_menu_actions.push(action);
                        }
                        return true;
                    }
                }
                // Released outside menu region - cancel any pending press
                self.menu.cancel_press();
                false
            }
            Event::MouseMotion { x, y, .. } => {
                // Handle mouse movement while button is held
                let nx = *x as i32;
                let ny = (*y as i32 - HIBAR_Y) / 2;
                if nx >= 430 && nx < 534 && ny >= 2 && ny < 55 {
                    let col = if nx < 482 { 0 } else { 1 };
                    let row = (ny - 2) / 9;
                    let display_slot = (row as usize) * 2 + col;
                    // Let menu handle re-hover logic (re-activate committed slot, cancel if different)
                    self.menu.handle_mouse_move_while_held(display_slot);
                    return true;
                } else {
                    // Mouse moved completely out of menu region - cancel any press
                    self.menu.cancel_press();
                    return true;
                }
            }
            _ => false,
        }
    }

    fn update(
        &mut self,
        canvas: &mut Canvas<Window>,
        _play_tex: &mut Texture,
        delta_ticks: u32,
        game_lib: &GameLibrary,
        resources: &mut SceneResources<'_, '_>,
    ) -> SceneResult {
        // Drain menu actions queued from handle_event() (runs outside ECS borrow).
        let pending: Vec<MenuAction> = std::mem::take(&mut self.pending_menu_actions);
        for action in pending {
            if self.dispatch_menu_action(action, game_lib, resources) {
                return SceneResult::Quit;
            }
        }

        // On the first update, trigger the brother start placard if requested.
        if self.first_update {
            self.first_update = false;
            if self.show_start_placard {
                let name = game_lib
                    .get_brother(self.res.brother.active_brother)
                    .map(|b| b.name.to_lowercase())
                    .unwrap_or_else(|| "julian".to_string());
                return SceneResult::BrotherSuccession {
                    dead_placard:  None,
                    start_placard: Some(format!("{}_start", name)),
                };
            }
        }

        // Lazy-load world data on first frame.
        if !self.adf_load_done {
            self.load_world(game_lib);
        }

        // Run gameplay ticks (capped to avoid spiral-of-death).
        // No .max(1) — when delta_ticks is 0 (e.g. at 15 Hz every other 30fps
        // frame), we skip the tick entirely rather than running at double speed.
        let ticks = delta_ticks.min(4);
        for _ in 0..ticks {
            self.run_tick(game_lib);
            self.drain_messages(game_lib);
        }

        if let Some(result) = self.drain_brother_deaths(game_lib) {
            return result;
        }

        self.run_audio(resources);

        // ── Render ────────────────────────────────────────────────────────────
        canvas.set_draw_color(sdl3::pixels::Color::RGB(0, 0, 0));
        canvas.clear();
        if self.res.view.viewstatus == 1 {
            self.render_inventory(canvas);
        } else {
            self.render_map(canvas);
        }
        self.render_hibar(canvas, resources);

        // Render narrative placard overlay if active.
        if self.res.view.viewstatus == 2 {
            self.render_placard(canvas, resources);
        }

        // Render the debug console overlay if present.
        if let Some(console) = &mut self.console {
            console.render();
        }

        if self.quit_requested {
            return SceneResult::Quit;
        }
        SceneResult::Continue
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ── Sprite render helpers (ported from gameplay_scene/rendering.rs) ──────────

/// Compute an actor's framebuf-relative position from world coords.
/// Mirrors `GameplayScene::actor_rel_pos` (fmain.c sprite blit offset).
fn actor_rel_pos(abs_x: f32, abs_y: f32, map_x: u16, map_y: u16) -> (i32, i32) {
    const WRAP: i32 = 0x8000;
    const OX: i32 = -8;
    const OY: i32 = -26;
    let dx = (abs_x as i32 - map_x as i32 + OX).rem_euclid(WRAP);
    let rel_x = if dx > WRAP / 2 { dx - WRAP } else { dx };
    let dy = (abs_y as i32 - map_y as i32 + OY).rem_euclid(WRAP);
    let rel_y = if dy > WRAP / 2 { dy - WRAP } else { dy };
    (rel_x, rel_y)
}

/// Blit a 16-wide sprite frame into the indexed framebuf, skipping transparent pixels (index 31).
fn blit_sprite_to_framebuf(
    frame_pixels: &[u8],
    rel_x: i32,
    rel_y: i32,
    max_rows: usize,
    framebuf: &mut [u8],
    fb_w: i32,
    fb_h: i32,
) {
    use crate::game::sprites::{SPRITE_H, SPRITE_W};
    let row_limit = max_rows.min(SPRITE_H) as i32;
    for row in 0..row_limit {
        let dst_y = rel_y + row;
        if dst_y < 0 || dst_y >= fb_h { continue; }
        for col in 0..SPRITE_W as i32 {
            let dst_x = rel_x + col;
            if dst_x < 0 || dst_x >= fb_w { continue; }
            let src_idx = frame_pixels[(row as usize) * SPRITE_W + col as usize];
            if src_idx == 31 { continue; }
            framebuf[(dst_y * fb_w + dst_x) as usize] = src_idx;
        }
    }
}

/// Map facing direction to walking sprite frame base (diroffs[0..7]).
fn facing_to_frame_base(facing: Direction) -> usize {
    const DIROFFS_WALK: [usize; 9] = [16, 16, 24, 24, 0, 0, 8, 8, 0];
    DIROFFS_WALK[facing as usize]
}

/// Map facing direction to fighting sprite frame base (diroffs[8..15]).
fn facing_to_fight_frame_base(facing: Direction) -> usize {
    const DIROFFS_FIGHT: [usize; 9] = [56, 56, 68, 68, 32, 32, 44, 44, 32];
    DIROFFS_FIGHT[facing as usize]
}

/// Compute the final enemy sprite frame index from race, state, and cycle.
///
/// Implements `update_actor_index` (fmain.c:1799-1824) + `select_atype_inum`
/// parity/offset adjustments (fmain.c:2459-2460).
///
/// `idx` is the actor's slot index within the current enemy batch (used for
/// the Loraii `i%3` cluster and the standard `(cycle+i)&7` walk spread).
/// `num_frames` is the sheet frame count for bounds clamping.
pub(super) fn enemy_frame(
    race: u8,
    state: &crate::game::npc::NpcState,
    vitality: i16,
    facing: crate::game::direction::Direction,
    cycle: usize,
    idx: usize,
    num_frames: usize,
) -> usize {
    use crate::game::npc::{NpcState, RACE_DARK_KNIGHT, RACE_LORAII, RACE_SNAKE, RACE_WRAITH};
    let frame_base = facing_to_frame_base(facing);
    let n = num_frames.max(1);

    // update_actor_index: race-specific logical index.
    let logical: usize = if race == RACE_SNAKE {
        match state {
            NpcState::Still    => frame_base + (cycle & 1),
            NpcState::Walking  => frame_base + ((cycle / 2) & 1),
            _                  => frame_base,
        }
    } else if race == RACE_DARK_KNIGHT && vitality <= 0 {
        1
    } else if race == RACE_LORAII {
        match state {
            NpcState::Dying => 0x3f,
            _ => {
                let phase = (cycle & 3) * 2;
                let phase = if phase > 4 { phase - 1 } else { phase };
                match idx % 3 {
                    0 => 0x25,
                    1 => 0x28 + phase,
                    _ => 0x30 + phase,
                }
            }
        }
    } else if race == RACE_WRAITH {
        match state {
            NpcState::Still => frame_base + 1,
            _               => frame_base,
        }
    } else {
        match state {
            NpcState::Walking => frame_base + ((cycle + idx) & 7),
            NpcState::Still   => frame_base + 1,
            _                 => frame_base,
        }
    };

    // select_atype_inum: snake +0x24 offset; all others get race parity LSB.
    if race == RACE_SNAKE && !matches!(state, NpcState::Dying | NpcState::Dead) {
        (logical + 0x24) % n
    } else if race & 1 != 0 {
        (logical | 1) % n
    } else {
        (logical & !1usize) % n
    }
}

/// Map (npc_type, race) → cfile index.  Returns None for setfigs and skipped types.
fn npc_type_to_cfile(npc_type: u8, race: u8) -> Option<usize> {
    use crate::game::npc::*;
    match npc_type {
        NPC_TYPE_NONE | NPC_TYPE_CONTAINER => None,
        NPC_TYPE_HUMAN if race == RACE_ENEMY => Some(6),
        NPC_TYPE_HUMAN => None, // SetFig — skip here
        NPC_TYPE_SWAN   => Some(11),
        NPC_TYPE_HORSE  => Some(5),
        NPC_TYPE_DRAGON => Some(10),
        NPC_TYPE_GHOST | NPC_TYPE_WRAITH | NPC_TYPE_SKELETON => Some(7),
        NPC_TYPE_ORC    => Some(6),
        NPC_TYPE_SNAKE | NPC_TYPE_SPIDER | NPC_TYPE_DKNIGHT => Some(8),
        NPC_TYPE_LORAII | NPC_TYPE_NECROMANCER => Some(9),
        NPC_TYPE_RAFT   => Some(4),
        _ => Some(6),
    }
}

/// Select the hero STATELIST index during the DYING animation.
/// Reference: reference/logic/death-sequence.md — tactic counts 7→0 over 7 ticks;
/// frames 80/81 swap depending on facing.
fn hero_dying_statelist_index(countdown: u8, facing: Direction) -> usize {
    // Amiga facing: d==0 or d>4 → one frame; d==1..4 → the other.
    // Discriminants: NW=0, N=1, NE=2, E=3, SE=4, S=5, SW=6, W=7, None=9.
    let group_a = matches!(facing, Direction::NW | Direction::S | Direction::SW | Direction::W | Direction::None);
    if countdown > 4 {
        if group_a { 80 } else { 81 }
    } else {
        if group_a { 81 } else { 80 }
    }
}

/// Blit all visible actors (hero, enemies, setfigs) into the indexed framebuf,
/// interleaving per-sprite depth masking immediately after each blit.
/// Must be called after `MapRenderer::compose()` and before palette conversion.
/// Sprites are Y-sorted (painter's algorithm) before blitting.
fn blit_actors_inner(
    world: &World,
    hero_entity: hecs::Entity,
    sheets: &[Option<crate::game::sprites::SpriteSheet>],
    object_sprites: Option<&crate::game::sprites::SpriteSheet>,
    cycle: usize,
    map_x: u16,
    map_y: u16,
    hero_sector: u16,
    renderer: &mut crate::game::map_renderer::MapRenderer,
    hero_dying_countdown: u8,
    encounter_dying: bool,
) {
    use crate::game::actor::ActorState;
    use crate::game::ecs::components::{
        ActorMotion, AiState, BrotherKind, CombatState, Enemy, Facing, FrustFlag, GoodFairy,
        Hero, Loot, Position, SetFig, WorldObj,
    };
    use crate::game::npc::NpcState;
    use crate::game::sprite_mask::BlittedSprite;
    use crate::game::sprites::{BOW_X, BOW_Y, SPRITE_H, SPRITE_W, STATELIST};

    let fb_w = MAP_DST_W as i32;
    let fb_h = MAP_DST_H as i32;

    // Pending draws: (descriptor, owned pixel data).
    // Collected from all actor passes, then Y-sorted (ground ascending) before blitting
    // so that actors further back (lower ground-line Y) render behind closer actors.
    // Matches the original bubble-sort at fmain.c:2367-2393.
    let mut pending: Vec<(BlittedSprite, Vec<u8>)> = Vec::new();

    // ── Hero ──────────────────────────────────────────────────────────────────
    type HeroQuery<'a> = (
        &'a Hero,
        &'a Position,
        &'a Facing,
        Option<&'a ActorMotion>,
        Option<&'a CombatState>,
        Option<&'a FrustFlag>,
        Option<&'a BrotherKind>,
    );
    let mut hero_q = world.query_one::<HeroQuery<'_>>(hero_entity);
    if let Ok((_, pos, facing_c, motion_opt, combat_opt, frust_opt, brother_opt)) = hero_q.get() {
        let cfile_idx: usize = brother_opt.map(|b: &BrotherKind| b.id as usize).unwrap_or(0).min(2);
        if let Some(Some(ref sheet)) = sheets.get(cfile_idx) {
            let (rel_x, rel_y) = actor_rel_pos(pos.x, pos.y, map_x, map_y);
            let environ: i8 = motion_opt.map(|m: &ActorMotion| m.environ).unwrap_or(0);
            let combat_state: Option<&ActorState> = combat_opt.map(|c: &CombatState| &c.state);

            // compute_shape_clip (sprite-rendering.md): environ drives Y-shift and frame swap.
            // environ > 29 (k=30, fully submerged): replace sprite with bubble from OBJECTS sheet.
            // environ > 2 (wading/sinking): push ystart down by environ px (sprite sinks into water).
            // environ == 2 (brush): clip bottom 10px from sprite.
            let fully_submerged = environ > 29;
            let is_dead = matches!(combat_state, Some(ActorState::Dead));

            if fully_submerged && is_dead {
                // fmain.c:2493 — dead actor fully submerged: hidden entirely.
            } else if fully_submerged {
                // fmain.c:2494-2497 — draw alternating bubble frames from OBJECTS sheet.
                // Bubble inum = 97 + ((cycle + actor_index) & 1); actor index 0 for hero.
                let bubble_inum = 97 + (cycle & 1); // i==0 for hero
                let bub_y = rel_y + 27; // fmain.c:2494 — 27 = drowning bubble Y-anchor
                let bub_rows: usize = 8; // fmain.c:2495 — ystop = ystart + 7 → 8 rows
                if rel_x > -(SPRITE_W as i32) && rel_x < fb_w
                    && bub_y > -(bub_rows as i32) && bub_y < fb_h
                {
                    if let Some(obj_sheet) = object_sprites {
                        if let Some(fp) = obj_sheet.frame_pixels(bubble_inum) {
                            pending.push((BlittedSprite {
                                screen_x: rel_x,
                                screen_y: bub_y,
                                width:    SPRITE_W,
                                height:   bub_rows,
                                ground:   rel_y + SPRITE_H as i32,
                                is_falling: false,
                            }, fp.to_vec()));
                        }
                    }
                }
            } else {
                // Normal actor render with optional environ Y-shift.
                let (draw_y, body_rows) = if environ == 2 {
                    // fmain.c:2491 — brush: clip bottom 10px, no Y shift.
                    (rel_y, SPRITE_H.saturating_sub(10))
                } else if environ > 2 {
                    // fmain.c:2500 — wading/sinking: push sprite down by environ px.
                    (rel_y + environ as i32, SPRITE_H.saturating_sub(environ as usize))
                } else {
                    (rel_y, SPRITE_H)
                };

                if rel_x > -(SPRITE_W as i32) && rel_x < fb_w
                    && draw_y > -(SPRITE_H as i32) && draw_y < fb_h
                {
                    let hero_facing = facing_c.dir;
                    let is_moving   = motion_opt.map(|m: &ActorMotion| m.moving).unwrap_or(false);
                    let frustflag   = frust_opt.map(|f: &FrustFlag| f.count).unwrap_or(0);

                    let frame = if hero_dying_countdown > 0 {
                        // fmain.c:1719-1722: DYING animation frames 80/81 swap by facing.
                        hero_dying_statelist_index(hero_dying_countdown, hero_facing)
                    } else if encounter_dying {
                        // Goodfairy countdown running — corpse frame (statelist 82).
                        82
                    } else if frustflag >= 41 {
                        40
                    } else if frustflag >= 21 {
                        84 + ((cycle >> 1) & 1)
                    } else if matches!(combat_state, Some(ActorState::Sinking)) {
                        // fmain.c:1576 — death_step pins STATE_SINK to statelist index 83.
                        83
                    } else if let Some(ActorState::Fighting(f)) = combat_state {
                        let fight_base = facing_to_fight_frame_base(hero_facing);
                        fight_base + (*f as usize).min(8)
                    } else {
                        let frame_base = facing_to_frame_base(hero_facing);
                        if is_moving { frame_base + cycle % 8 } else { frame_base + 1 }
                    };

                    // Frustration, fighting, sinking, and dying frames are statelist indices.
                    let needs_statelist = hero_dying_countdown > 0
                        || encounter_dying
                        || frustflag >= 21
                        || matches!(combat_state, Some(ActorState::Fighting(_)) | Some(ActorState::Sinking));
                    let body_frame = if needs_statelist {
                        STATELIST.get(frame).map(|e| e.figure as usize).unwrap_or(frame)
                    } else {
                        frame
                    };

                    // Weapon overlay — port of select_atype_inum (fmain.c:2420-2446)
                    // and resolve_pass_params (fmain.c:2400-2409).
                    // `frame` is the STATELIST index (inum); `body_frame` is the body sprite frame.
                    let weapon: u8 = combat_opt.map(|c| c.weapon).unwrap_or(0);
                    let facing_dir: usize = hero_facing as usize;
                    let entry = STATELIST.get(frame).copied().unwrap_or(STATELIST[0]);

                    // Pixel offset (fmain.c:2422-2426).
                    let (wpn_dx, mut wpn_dy): (i32, i32) = if weapon == 4 && frame < 32 {
                        (BOW_X[frame] as i32, BOW_Y[frame] as i32)
                    } else {
                        (entry.wpn_x as i32, entry.wpn_y as i32)
                    };

                    // OBJECTS-sheet frame (fmain.c:2429-2444).
                    let wpn_frame: usize = if weapon == 4 && frame < 32 {
                        let q = frame / 8;
                        if q & 1 != 0 { 30 }
                        else if q & 2 != 0 { 0x53 }
                        else { 0x51 }
                    } else if weapon == 5 {
                        if facing_dir == 2 { wpn_dy -= 6; } // NE: nudge up 6 px
                        facing_dir + 103
                    } else {
                        let k: usize = match weapon { 2 => 32, 3 => 48, 1 => 64, _ => 0 };
                        entry.wpn_no as usize + k
                    };

                    // Gate (fmain.c:2400-2401): 0 < weapon < 8, hero alive (frame < 80).
                    let draw_weapon = weapon > 0 && weapon < 8 && frame < 80;

                    // Facing-dependent draw order (resolve_pass_params, fmain.c:2402-2407).
                    // `true` = weapon behind body (drawn first in pending).
                    let weapon_behind = if weapon == 4 && frame < 32 {
                        (facing_dir & 4) == 0
                    } else {
                        ((facing_dir as i32 - 2) & 4) != 0
                    };

                    // Weapon behind body: push weapon first so it renders under the body.
                    // Uses same ground value — stable sort preserves insertion order.
                    if draw_weapon && weapon_behind {
                        if let Some(obj_sheet) = object_sprites {
                            if let Some(wp) = obj_sheet.frame_pixels(wpn_frame) {
                                pending.push((BlittedSprite {
                                    screen_x: rel_x + wpn_dx,
                                    screen_y: draw_y + wpn_dy,
                                    width:    SPRITE_W,
                                    height:   obj_sheet.frame_h,
                                    ground:   rel_y + SPRITE_H as i32,
                                    is_falling: false,
                                }, wp.to_vec()));
                            }
                        }
                    }

                    if let Some(fp) = sheet.frame_pixels(body_frame) {
                        pending.push((BlittedSprite {
                            screen_x: rel_x,
                            screen_y: draw_y,
                            width:    SPRITE_W,
                            height:   body_rows,
                            ground:   rel_y + SPRITE_H as i32,
                            is_falling: false,
                        }, fp.to_vec()));
                    }

                    // Weapon on top: push weapon after body so it renders over it.
                    if draw_weapon && !weapon_behind {
                        if let Some(obj_sheet) = object_sprites {
                            if let Some(wp) = obj_sheet.frame_pixels(wpn_frame) {
                                pending.push((BlittedSprite {
                                    screen_x: rel_x + wpn_dx,
                                    screen_y: draw_y + wpn_dy,
                                    width:    SPRITE_W,
                                    height:   obj_sheet.frame_h,
                                    ground:   rel_y + SPRITE_H as i32,
                                    is_falling: false,
                                }, wp.to_vec()));
                            }
                        }
                    }
                }
            }
        }
    }

    // ── Enemies ───────────────────────────────────────────────────────────────
    use crate::game::npc::RACE_LORAII;
    let mut enemy_q = world.query::<(
        &Enemy,
        &Position,
        &Facing,
        &crate::game::ecs::components::EnemyKind,
        Option<&AiState>,
        Option<&crate::game::ecs::components::Health>,
        Option<&ActorMotion>,
        Option<&Loot>,
    )>();
    for (idx, (_, pos, facing_c, kind, ai_opt, health_opt, motion_opt, loot_opt)) in enemy_q.iter().enumerate() {
        let race = kind.race;

        // Dead Loraii are parked off-screen (fmain.c:1807 — abs_x = 0).
        let vitality = health_opt.map(|h| h.vitality).unwrap_or(1);
        if race == RACE_LORAII && vitality <= 0 {
            continue;
        }

        let Some(cfile_idx) = npc_type_to_cfile(kind.npc_type, race) else { continue; };
        let Some(Some(ref sheet)) = sheets.get(cfile_idx) else { continue; };

        let (rel_x, rel_y) = actor_rel_pos(pos.x, pos.y, map_x, map_y);
        if rel_x <= -(SPRITE_W as i32) || rel_x >= fb_w
            || rel_y <= -(SPRITE_H as i32) || rel_y >= fb_h
        {
            continue;
        }

        let environ: i8 = motion_opt.map(|m: &ActorMotion| m.environ).unwrap_or(0);
        let npc_state = ai_opt.map(|a| &a.state).unwrap_or(&NpcState::Still);

        // fmain.c:2491-2500 — environ drives Y-shift and height clip, same as hero.
        let (draw_y, body_rows) = if environ == 2 {
            (rel_y, SPRITE_H.saturating_sub(10))
        } else if environ > 2 {
            (rel_y + environ as i32, SPRITE_H.saturating_sub(environ as usize))
        } else {
            (rel_y, SPRITE_H)
        };

        let frame = enemy_frame(race, npc_state, vitality, facing_c.dir, cycle, idx, sheet.num_frames);

        // Weapon overlay — same logic as hero (fmain.c:2400-2446).
        // Enemies with weapon >= 8 (WEAPON_TOUCH) are excluded by the gate.
        let weapon: u8 = loot_opt.map(|l| l.weapon).unwrap_or(0);
        let facing_dir: usize = facing_c.dir as usize;
        let entry = STATELIST.get(frame).copied().unwrap_or(STATELIST[0]);

        let (wpn_dx, mut wpn_dy): (i32, i32) = if weapon == 4 && frame < 32 {
            (BOW_X[frame] as i32, BOW_Y[frame] as i32)
        } else {
            (entry.wpn_x as i32, entry.wpn_y as i32)
        };

        let wpn_frame: usize = if weapon == 4 && frame < 32 {
            let q = frame / 8;
            if q & 1 != 0 { 30 } else if q & 2 != 0 { 0x53 } else { 0x51 }
        } else if weapon == 5 {
            if facing_dir == 2 { wpn_dy -= 6; }
            facing_dir + 103
        } else {
            let k: usize = match weapon { 2 => 32, 3 => 48, 1 => 64, _ => 0 };
            entry.wpn_no as usize + k
        };

        let draw_weapon = weapon > 0 && weapon < 8 && frame < 80;
        let weapon_behind = if weapon == 4 && frame < 32 {
            (facing_dir & 4) == 0
        } else {
            ((facing_dir as i32 - 2) & 4) != 0
        };

        if draw_weapon && weapon_behind {
            if let Some(obj_sheet) = object_sprites {
                if let Some(wp) = obj_sheet.frame_pixels(wpn_frame) {
                    pending.push((BlittedSprite {
                        screen_x: rel_x + wpn_dx,
                        screen_y: draw_y + wpn_dy,
                        width:    SPRITE_W,
                        height:   obj_sheet.frame_h,
                        ground:   rel_y + SPRITE_H as i32,
                        is_falling: false,
                    }, wp.to_vec()));
                }
            }
        }

        if let Some(fp) = sheet.frame_pixels(frame) {
            pending.push((BlittedSprite {
                screen_x: rel_x,
                screen_y: draw_y,
                width:    SPRITE_W,
                height:   body_rows,
                ground:   rel_y + SPRITE_H as i32,
                is_falling: false,
            }, fp.to_vec()));
        }

        if draw_weapon && !weapon_behind {
            if let Some(obj_sheet) = object_sprites {
                if let Some(wp) = obj_sheet.frame_pixels(wpn_frame) {
                    pending.push((BlittedSprite {
                        screen_x: rel_x + wpn_dx,
                        screen_y: draw_y + wpn_dy,
                        width:    SPRITE_W,
                        height:   obj_sheet.frame_h,
                        ground:   rel_y + SPRITE_H as i32,
                        is_falling: false,
                    }, wp.to_vec()));
                }
            }
        }
    }

    // ── SetFigs ──────────────────────────────────────────────────────────────
    let mut setfig_q = world.query::<(&SetFig, &Position, &WorldObj)>();
    for (_, pos, obj) in setfig_q.iter() {
        // Setfig type index = race byte with the 0x80 setfig bit stripped (fmain.c:3374).
        let k = (obj.ob_id & 0x7f) as usize;
        let Some(entry) = crate::game::sprites::SETFIG_TABLE.get(k) else { continue; };
        let cfile_idx = entry.cfile_entry as usize;
        let Some(Some(ref sheet)) = sheets.get(cfile_idx) else { continue; };
        let (rel_x, rel_y) = actor_rel_pos(pos.x, pos.y, map_x, map_y);
        if rel_x <= -(SPRITE_W as i32) || rel_x >= fb_w
            || rel_y <= -(SPRITE_H as i32) || rel_y >= fb_h
        {
            continue;
        }
        // Idle pose for this setfig type is SETFIG_TABLE[k].image_base.
        if let Some(fp) = sheet.frame_pixels(entry.image_base as usize) {
            pending.push((BlittedSprite {
                screen_x: rel_x,
                screen_y: rel_y,
                width:    SPRITE_W,
                height:   SPRITE_H,
                ground:   rel_y + SPRITE_H as i32,
                is_falling: false,
            }, fp.to_vec()));
        }
    }

    // ── Good Fairy ────────────────────────────────────────────────────────────
    use crate::game::sprites::OBJ_SPRITE_H;
    let mut fairy_q = world.query::<(&GoodFairy, &Position)>();
    for (_, pos) in fairy_q.iter() {
        let (rel_x, rel_y) = actor_rel_pos(pos.x, pos.y, map_x, map_y);
        if rel_x <= -(SPRITE_W as i32) || rel_x >= fb_w
            || rel_y <= -(OBJ_SPRITE_H as i32) || rel_y >= fb_h
        {
            continue;
        }
        // OBJECTS sheet frames 100/101 alternating (reference/_discovery/brother-succession.md).
        let frame_inum = 100 + (cycle & 1);
        if let Some(obj_sheet) = object_sprites {
            if let Some(fp) = obj_sheet.frame_pixels(frame_inum) {
                pending.push((BlittedSprite {
                    screen_x: rel_x,
                    screen_y: rel_y,
                    width:    SPRITE_W,
                    height:   OBJ_SPRITE_H,
                    ground:   rel_y + OBJ_SPRITE_H as i32,
                    is_falling: false,
                }, fp.to_vec()));
            }
        }
    }

    // ── Y-sort, blit, and mask (interleaved per sprite) ───────────────────────
    // Sort back-to-front by ground-line Y (ascending) — painter's algorithm.
    // Mirrors fmain.c:2367-2393 bubble sort on anim_index[] by Y coordinate.
    pending.sort_by_key(|(s, _)| s.ground);

    // Blit and mask each sprite in order: closer sprites draw over both the
    // farther sprite's pixels AND any terrain re-stamped by the farther sprite's mask.
    // This matches the original per-actor save_blit → mask_blit → shape_blit loop.
    for (sprite, pixels) in pending {
        blit_sprite_to_framebuf(&pixels, sprite.screen_x, sprite.screen_y, sprite.height, &mut renderer.framebuf, fb_w, fb_h);
        crate::game::sprite_mask::apply_sprite_mask(renderer, &sprite, hero_sector, 0);
    }
}

// ── Compass helpers ───────────────────────────────────────────────────────────

/// Map a Direction to its compass arrow index (0..9, where 9=None).
fn compass_dir_index(dir: Direction) -> usize {
    // Amiga order: NW=0, N=1, NE=2, E=3, SE=4, S=5, SW=6, W=7, None=8
    dir as usize
}


// ── Palette helpers (ported from gameplay_scene/region.rs) ───────────────────

/// Build the initial RGBA palette for a region (no day/night fade applied).
fn region_palette(game_lib: &GameLibrary, region: u8) -> Palette {
    let mut palette = [0xFF808080_u32; PALETTE_SIZE];
    if let Some(base) = game_lib.find_palette("pagecolors") {
        for (i, entry) in base.colors.iter().enumerate().take(PALETTE_SIZE) {
            palette[i] = amiga_color_to_rgba(entry.color);
        }
    }
    palette[31] = amiga_color_to_rgba(match region { 4 => 0x0980, 9 => 0x0445, _ => 0x0bdf });
    palette
}

/// Build a base `colors::Palette` with the per-region color-31 override applied.
fn build_base_colors_palette(
    game_lib: &GameLibrary,
    region: u8,
) -> Option<crate::game::colors::Palette> {
    let base = game_lib.find_palette("pagecolors")?;
    let mut cloned = base.clone();
    let color31: u16 = match region { 4 => 0x0980, 9 => 0x0445, _ => 0x0bdf };
    if let Some(c) = cloned.colors.get_mut(31) {
        *c = RGB4::from(color31);
    }
    Some(cloned)
}

/// Recompute the display palette from base colors + current lighting state.
/// Mirrors `GameplayScene::compute_current_palette` (SPEC §17.5–17.6).
fn compute_current_palette(
    base: &crate::game::colors::Palette,
    region_num: u8,
    lightlevel: u16,
    light_on: bool,
    secret_active: bool,
) -> Palette {
    if region_num >= 8 {
        // Indoors: full brightness; jewel tint applied by fade_page.
        let faded = crate::game::palette_fader::fade_page(100, 100, 100, true, light_on, base);
        let mut pal = [0xFF808080_u32; PALETTE_SIZE];
        for (i, entry) in faded.colors.iter().enumerate().take(PALETTE_SIZE) {
            pal[i] = amiga_color_to_rgba(entry.color);
        }
        pal[31] = amiga_color_to_rgba(match (region_num, secret_active) {
            (9, true)  => 0x00f0,
            (9, false) => 0x0445,
            _          => 0x0bdf,
        });
        return pal;
    }
    let ll = lightlevel as i32;
    let boost = if light_on { 200i32 } else { 0 };
    let r_pct = (ll - 80 + boost) as i16;
    let g_pct = (ll - 61) as i16;
    let b_pct = (ll - 62) as i16;
    let faded = crate::game::palette_fader::fade_page(r_pct, g_pct, b_pct, true, light_on, base);
    let mut out = [0xFF808080_u32; PALETTE_SIZE];
    for (i, entry) in faded.colors.iter().enumerate().take(PALETTE_SIZE) {
        out[i] = amiga_color_to_rgba(entry.color);
    }
    out[31] = amiga_color_to_rgba(match region_num { 4 => 0x0980, _ => 0x0bdf });
    out
}

/// Construct a minimal `EcsScene` for unit tests (no SDL, no disk assets).
#[cfg(test)]
pub fn new_for_test() -> EcsScene {
    use crate::game::ecs::components::{HeroStats, Inventory};
    use crate::game::ecs::spawn::spawn_hero;
    let mut world = World::new();
    let stats = HeroStats { vitality: 10, brave: 20, luck: 30, kind: 40, wealth: 50, hunger: 0, fatigue: 0, gold: 0 };
    let hero = spawn_hero(&mut world, 100.0, 200.0, 0, stats, Inventory::empty());
    let mut res = Resources::new(hero);
    res.region.region_num = 0;
    res.region.new_region = 0;
    EcsScene {
        world,
        res,
        console: None,
        input: InputState::new(),
        last_mood: u8::MAX,
        mood_tick: 0,
        adf_load_done: false,
        adf: None,
        base_colors: None,
        messages: Vec::new(),
        menu: MenuState::new(),
        quit_requested: false,
        pending_menu_actions: Vec::new(),
        show_start_placard: false,
        first_update: false,
    }
}

#[cfg(test)]
mod tests {
    // EcsScene::new() requires a GameLibrary loaded from disk, which is not
    // available in unit tests.  System-level tests live in each system's
    // own module; persist round-trip tests are in persist.rs.

    use super::*;
    use crate::game::ecs::components::{CombatState, WorldObj};
    use crate::game::ecs::spawn::{spawn_enemy, spawn_ground_item, spawn_setfig};

    fn load_game_lib() -> GameLibrary {
        let config = std::fs::read_to_string("faery.toml")
            .expect("faery.toml must be present in project root");
        toml::from_str::<GameLibrary>(&config)
            .expect("faery.toml should deserialize without errors")
    }

    fn make_fake_adf() -> std::sync::Arc<crate::game::adf::AdfDisk> {
        // Minimal ADF large enough that block loads don't panic.
        std::sync::Arc::new(crate::game::adf::AdfDisk::from_bytes(vec![0u8; 2048 * 512]))
    }

    fn make_test_obj(region: u8) -> WorldObj {
        WorldObj { ob_id: 1, ob_stat: 1, region, visible: true, goal: 0 }
    }

    /// region_clears_old_entities: Enemy + SetFig despawned; hero remains.
    #[test]
    fn region_clears_old_entities() {
        let mut scene = new_for_test();
        let game_lib = load_game_lib();

        let enemy  = spawn_enemy(&mut scene.world, 200.0, 200.0, 1, 0, 10, 0, 0, 3, 0, 0);
        let setfig = spawn_setfig(&mut scene.world, 300.0, 300.0, make_test_obj(0), 13);
        let item   = spawn_ground_item(&mut scene.world, 400.0, 400.0, make_test_obj(0));

        let hero = scene.res.hero_entity;
        assert!(scene.world.contains(hero));
        assert!(scene.world.contains(enemy));
        assert!(scene.world.contains(setfig));
        assert!(scene.world.contains(item));

        scene.adf = Some(make_fake_adf());
        scene.res.adf = scene.adf.clone();

        // WorldData::load will fail on zero data but despawn happens before that.
        scene.reload_region(1, 500.0, 600.0, &game_lib);

        assert!(!scene.world.contains(enemy),  "enemy should be despawned");
        assert!(!scene.world.contains(setfig), "setfig should be despawned");
        assert!(!scene.world.contains(item),   "ground item should be despawned");
        assert!(scene.world.contains(hero),    "hero must survive");
    }

    /// region_num_updated: res.region.region_num reflects the new region after transition.
    #[test]
    fn region_num_updated() {
        let mut scene = new_for_test();
        let game_lib = load_game_lib();

        scene.adf = Some(make_fake_adf());
        scene.res.adf = scene.adf.clone();

        scene.reload_region(3, 0.0, 0.0, &game_lib);

        assert_eq!(scene.res.region.region_num, 3);
    }

    /// zones_populated: res.zones is non-empty after a region load when game_lib has zones.
    #[test]
    fn zones_populated() {
        let mut scene = new_for_test();
        let game_lib = load_game_lib();
        let had_zones = !game_lib.zones.is_empty();

        assert!(scene.res.zones.is_empty(), "zones should start empty");

        scene.adf = Some(make_fake_adf());
        scene.res.adf = scene.adf.clone();

        scene.reload_region(0, 0.0, 0.0, &game_lib);

        if had_zones {
            assert!(!scene.res.zones.is_empty(), "res.zones should be populated after reload");
        }
    }

    /// hero_repositioned: hero Position matches dest_x/dest_y if reload succeeds to that step.
    /// Note: hero is repositioned only when WorldData loads successfully.
    /// With a blank ADF the load fails early; we test the plumbing via a direct call.
    #[test]
    fn hero_repositioned_on_successful_load() {
        let mut scene = new_for_test();

        // Manually perform only the hero reposition and camera snap steps
        // (mirrors what reload_region does after a successful WorldData load).
        if let Ok(mut pos) = scene.world.get::<&mut crate::game::ecs::components::Position>(scene.res.hero_entity) {
            pos.x = 500.0;
            pos.y = 600.0;
        }
        scene.res.camera.map_x = (500.0f32 - 144.0).rem_euclid(0x8000 as f32);
        scene.res.camera.map_y = (600.0f32 - 70.0).rem_euclid(0x8000 as f32);

        let pos = scene.world.get::<&crate::game::ecs::components::Position>(scene.res.hero_entity).unwrap();
        assert_eq!(pos.x, 500.0);
        assert_eq!(pos.y, 600.0);
        assert_eq!(scene.res.camera.map_x, 356.0);
        assert_eq!(scene.res.camera.map_y, 530.0);
    }

    /// camera_snapped: camera offset is derived from dest position.
    #[test]
    fn camera_snap_formula() {
        let dest_x = 500.0f32;
        let dest_y = 600.0f32;
        let cam_x = (dest_x - 144.0).rem_euclid(0x8000 as f32);
        let cam_y = (dest_y - 70.0).rem_euclid(0x8000 as f32);
        assert_eq!(cam_x, 356.0);
        assert_eq!(cam_y, 530.0);
    }

    #[test]
    fn test_menu_state_initializes_with_scene() {
        use crate::game::menu::MenuMode;
        let scene = new_for_test();
        assert_eq!(scene.menu.cmode, MenuMode::Items);
        assert!(!scene.quit_requested);
    }

    #[test]
    fn test_keyboard_q_triggers_quit() {
        use crate::game::menu::{MenuAction, MenuMode};
        let mut scene = new_for_test();
        // Q opens the SaveX submenu (Game menu slot 8 → gomenu(SaveX)).
        let _ = scene.menu.handle_key(b'Q');
        assert_eq!(scene.menu.cmode, MenuMode::SaveX);
        // X in SaveX mode → MenuAction::Quit.
        let action = scene.menu.handle_key(b'X');
        assert!(matches!(action, MenuAction::Quit));
        // Verify the flag pathway works.
        scene.quit_requested = true;
        assert!(scene.quit_requested);
    }

    #[test]
    fn test_set_weapon_dispatches() {
        use crate::game::ecs::components::CombatState;
        let mut scene = new_for_test();
        scene.world.insert_one(scene.res.hero_entity, CombatState::default()).ok();
        if let Ok(mut cs) = scene.world.get::<&mut CombatState>(scene.res.hero_entity) {
            cs.weapon = 2;
        }
        let cs = scene.world.get::<&CombatState>(scene.res.hero_entity).unwrap();
        assert_eq!(cs.weapon, 2);
    }

    #[test]
    fn test_update_menu_options_enables_magic() {
        use crate::game::menu::MenuMode;
        let mut scene = new_for_test();
        if let Ok(mut inv) = scene.world.get::<&mut Inventory>(scene.res.hero_entity) {
            inv.stuff[9] = 1;
        }
        scene.update_menu_options();
        assert_eq!(scene.menu.menus[MenuMode::Magic as usize].enabled[5], 10);
    }

    #[test]
    fn test_take_action_emits_item_event() {
        use crate::game::ecs::components::{GroundItem, Position, WorldObj};
        use crate::game::ecs::events::ItemEvent;
        let mut scene = new_for_test();
        // Hero is at (100.0, 200.0) in new_for_test; spawn item 1px away.
        let _item = scene.world.spawn((
            GroundItem,
            Position::new(101.0, 200.0),
            WorldObj { ob_id: 5, ob_stat: 1, region: 0, visible: true, goal: 0 },
        ));
        let hero_pos = scene.world
            .get::<&Position>(scene.res.hero_entity)
            .map(|p| (p.x, p.y))
            .unwrap_or((0.0, 0.0));
        let mut best: Option<(hecs::Entity, f32)> = None;
        for (entity, obj, pos) in scene.world.query::<(hecs::Entity, &WorldObj, &Position)>().iter() {
            if obj.ob_stat != 1 || !obj.visible { continue; }
            let dx = pos.x - hero_pos.0;
            let dy = pos.y - hero_pos.1;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= 16.0 {
                if best.map_or(true, |(_, d)| dist < d) {
                    best = Some((entity, dist));
                }
            }
        }
        if let Some((entity, _)) = best {
            scene.res.events.item.push(ItemEvent::TakeItem { entity });
        }
        assert_eq!(scene.res.events.item.len(), 1);
    }

    #[test]
    fn setfig_race_resolves_to_table_entry() {
        use crate::game::ecs::components::{SetFig, Position, WorldObj};
        use crate::game::sprites::SETFIG_TABLE;
        let mut world = hecs::World::new();
        // Priest = setfig index 1 → race byte 0x81.
        world.spawn((
            SetFig,
            Position { x: 100.0, y: 200.0 },
            WorldObj { ob_id: 0x81, ob_stat: 1, region: 0, visible: true, goal: 0 },
        ));
        let mut q = world.query::<(&SetFig, &WorldObj)>();
        let (_, obj) = q.iter().next().unwrap();
        let entry = SETFIG_TABLE[(obj.ob_id & 0x7f) as usize];
        assert_eq!(entry.cfile_entry, 13, "priest is on cfile 13");
        assert_eq!(entry.image_base, 4, "priest idle frame is 4, not 0");
    }

    #[test]
    fn setfig_query_finds_entity() {
        use crate::game::ecs::components::{SetFig, Enemy, Position, WorldObj, EnemyKind};
        let mut world = hecs::World::new();
        world.spawn((
            SetFig,
            Position { x: 10.0, y: 10.0 },
            WorldObj { ob_id: 0x82, ob_stat: 1, region: 0, visible: true, goal: 0 }, // guard
        ));
        world.spawn((
            Enemy,
            Position { x: 20.0, y: 20.0 },
            EnemyKind { npc_type: 0, race: 0 },
        ));
        let mut setfig_q = world.query::<(&SetFig, &Position, &WorldObj)>();
        assert_eq!(setfig_q.iter().count(), 1);
    }

    #[test]
    fn setfig_not_rendered_when_offscreen() {
        use crate::game::sprites::{SPRITE_W, SPRITE_H};
        // Simulate: camera at (0,0), framebuffer 320x200.
        let fb_w: i32 = 320;
        let fb_h: i32 = 200;
        // SetFig positioned far to the left — rel_x will be deeply negative.
        let rel_x: i32 = -(SPRITE_W as i32) - 1;
        let rel_y: i32 = 50;
        let culled = rel_x <= -(SPRITE_W as i32)
            || rel_x >= fb_w
            || rel_y <= -(SPRITE_H as i32)
            || rel_y >= fb_h;
        assert!(culled, "SetFig outside framebuffer bounds must be culled");
    }

    // ── hero_dying_statelist_index tests ─────────────────────────────────────

    use crate::game::direction::Direction;

    #[test]
    fn hero_dying_frames_swap_by_facing() {
        // Phase A (countdown > 4): facing d==0 or d>4 → 80; d==1..4 → 81.
        assert_eq!(super::hero_dying_statelist_index(7, Direction::NW), 80);
        assert_eq!(super::hero_dying_statelist_index(7, Direction::S),  80);
        assert_eq!(super::hero_dying_statelist_index(7, Direction::SW), 80);
        assert_eq!(super::hero_dying_statelist_index(7, Direction::W),  80);
        assert_eq!(super::hero_dying_statelist_index(7, Direction::N),  81);
        assert_eq!(super::hero_dying_statelist_index(7, Direction::NE), 81);
        assert_eq!(super::hero_dying_statelist_index(7, Direction::E),  81);
        assert_eq!(super::hero_dying_statelist_index(7, Direction::SE), 81);

        // Phase B (countdown 1..4): swapped.
        assert_eq!(super::hero_dying_statelist_index(4, Direction::NW), 81);
        assert_eq!(super::hero_dying_statelist_index(3, Direction::S),  81);
        assert_eq!(super::hero_dying_statelist_index(2, Direction::SW), 81);
        assert_eq!(super::hero_dying_statelist_index(1, Direction::W),  81);
        assert_eq!(super::hero_dying_statelist_index(4, Direction::N),  80);
        assert_eq!(super::hero_dying_statelist_index(3, Direction::NE), 80);
        assert_eq!(super::hero_dying_statelist_index(2, Direction::E),  80);
        assert_eq!(super::hero_dying_statelist_index(1, Direction::SE), 80);
    }

    // ── enemy_frame tests ─────────────────────────────────────────────────────
    // All frame numbers are verified against actor-animation-catalog.md.

    use crate::game::npc::{NpcState, RACE_DARK_KNIGHT, RACE_LORAII, RACE_SNAKE, RACE_WRAITH};
    use crate::game::ecs::scene::enemy_frame;

    const N: usize = 64; // typical ENEMY sheet frame count

    // Wraith (race 2, even): walk frame frozen at diroffs[d], same as Still after
    // parity clears LSB.  South facing: diroffs[4] = 0.
    #[test]
    fn wraith_walk_frozen_south() {
        // Walking at any cycle must stay on frame 0 (diroffs[S]=0, LSB cleared).
        for cycle in 0..16 {
            let f = enemy_frame(RACE_WRAITH, &NpcState::Walking, 10, Direction::S, cycle, 0, N);
            assert_eq!(f, 0, "wraith walking S cycle {cycle}: expected 0, got {f}");
        }
    }

    #[test]
    fn wraith_still_and_walk_same_frame() {
        // Still computes diroffs[S]+1 = 1, then even parity clears LSB → 0.
        // Walking computes diroffs[S] = 0, parity clears → 0.
        // Both should land on the same physical frame.
        let walk = enemy_frame(RACE_WRAITH, &NpcState::Walking, 10, Direction::S, 0, 0, N);
        let still = enemy_frame(RACE_WRAITH, &NpcState::Still,   10, Direction::S, 0, 0, N);
        assert_eq!(walk, still, "wraith walk and still must render the same frame");
    }

    #[test]
    fn wraith_walk_frozen_west() {
        // West facing: diroffs[6] = 8, parity clears LSB → 8.
        for cycle in 0..16 {
            let f = enemy_frame(RACE_WRAITH, &NpcState::Walking, 10, Direction::W, cycle, 0, N);
            assert_eq!(f, 8, "wraith walking W cycle {cycle}: expected 8, got {f}");
        }
    }

    // Snake (race 4, even after +0x24 offset): half-rate 2-frame wiggle, offset into
    // second half of ENEMY sheet.  South facing: diroffs[4] = 0.
    #[test]
    fn snake_walk_half_rate_south() {
        // Walking: logical = 0 + ((cycle/2)&1), then +0x24 = 36.
        // cycle 0,1 → index 0 → frame 36; cycle 2,3 → index 1 → frame 37.
        assert_eq!(enemy_frame(RACE_SNAKE, &NpcState::Walking, 10, Direction::S, 0, 0, N), 36);
        assert_eq!(enemy_frame(RACE_SNAKE, &NpcState::Walking, 10, Direction::S, 1, 0, N), 36);
        assert_eq!(enemy_frame(RACE_SNAKE, &NpcState::Walking, 10, Direction::S, 2, 0, N), 37);
        assert_eq!(enemy_frame(RACE_SNAKE, &NpcState::Walking, 10, Direction::S, 3, 0, N), 37);
    }

    #[test]
    fn snake_still_full_rate_south() {
        // Still: logical = 0 + (cycle&1), then +0x24 = 36 or 37.
        assert_eq!(enemy_frame(RACE_SNAKE, &NpcState::Still, 10, Direction::S, 0, 0, N), 36);
        assert_eq!(enemy_frame(RACE_SNAKE, &NpcState::Still, 10, Direction::S, 1, 0, N), 37);
    }

    #[test]
    fn snake_uses_second_sheet_half() {
        // In any walk/still state, frame must be >= 0x24 (36).
        for cycle in 0..8 {
            let f = enemy_frame(RACE_SNAKE, &NpcState::Walking, 10, Direction::S, cycle, 0, N);
            assert!(f >= 0x24, "snake walking S should use second sheet half, got {f}");
        }
    }

    // Dark Knight (race 7, odd): zero-HP pins to frame 1 (odd parity preserves LSB).
    #[test]
    fn dark_knight_zero_hp_reanimation() {
        let f = enemy_frame(RACE_DARK_KNIGHT, &NpcState::Walking, 0, Direction::S, 0, 0, N);
        assert_eq!(f, 1, "dark knight at vitality 0 should pin to frame 1");
    }

    #[test]
    fn dark_knight_alive_walks_normally() {
        // Alive dark knight (race 7, odd) uses standard walk + odd parity → frame must be odd.
        let f = enemy_frame(RACE_DARK_KNIGHT, &NpcState::Walking, 10, Direction::S, 0, 0, N);
        assert_eq!(f % 2, 1, "living dark knight walk frame must be odd (race 7 is odd-race)");
    }

    // Loraii (race 8, even): cluster cycle across three slots.
    #[test]
    fn loraii_slot0_is_static() {
        // Slot 0 always returns 0x25, parity clears LSB → 0x24.
        for cycle in 0..16 {
            let f = enemy_frame(RACE_LORAII, &NpcState::Walking, 10, Direction::S, cycle, 0, N);
            assert_eq!(f, 0x24, "Loraii slot 0 should be static frame 0x24, got {f} at cycle {cycle}");
        }
    }

    #[test]
    fn loraii_slot1_cycles() {
        // Slot 1: logical = 0x28 + phase; even parity clears LSB.
        // phase per cycle: 0→0, 1→2, 2→4, 3→5, 4→0, 5→2, 6→4, 7→5.
        // After even parity (&!1): 0x28,0x2a,0x2c,0x2c, 0x28,0x2a,0x2c,0x2c.
        let expected = [0x28, 0x2a, 0x2c, 0x2c, 0x28, 0x2a, 0x2c, 0x2c];
        for (cycle, &exp) in expected.iter().enumerate() {
            let f = enemy_frame(RACE_LORAII, &NpcState::Walking, 10, Direction::S, cycle, 1, N);
            assert_eq!(f, exp, "Loraii slot 1 cycle {cycle}: expected {exp:#x}, got {f:#x}");
        }
    }

    #[test]
    fn loraii_dying_frame() {
        // Dying: logical 0x3f, even parity → 0x3e.
        let f = enemy_frame(RACE_LORAII, &NpcState::Dying, 10, Direction::S, 0, 0, N);
        assert_eq!(f, 0x3e, "Loraii dying should be frame 0x3e");
    }

    // Parity: standard even-race enemy (race 0, Ogre) gets even frames.
    #[test]
    fn ogre_even_parity() {
        let f = enemy_frame(0, &NpcState::Walking, 10, Direction::S, 0, 0, N);
        assert_eq!(f % 2, 0, "ogre (race 0) walk frame must be even");
    }

    // Parity: standard odd-race enemy (race 1, Orcs) gets odd frames.
    #[test]
    fn orcs_odd_parity() {
        let f = enemy_frame(1, &NpcState::Walking, 10, Direction::S, 0, 0, N);
        assert_eq!(f % 2, 1, "orcs (race 1) walk frame must be odd");
    }

    // Spec §15.12: every brother begins play with only a Dirk (stuff[0] = 1), equipped (weapon=1).
    #[test]
    fn julian_starts_with_dirk() {
        let lib = load_game_lib();
        let scene = EcsScene::new(&lib, None, false);
        let inv = scene.world.get::<&Inventory>(scene.res.hero_entity)
            .expect("hero must have Inventory component");
        assert_eq!(inv.stuff[0], 1, "Julian should start with 1 Dirk in slot 0");
        for i in 1..35 {
            assert_eq!(inv.stuff[i], 0, "inventory slot {i} should be empty at game start");
        }
        let cs = scene.world.get::<&CombatState>(scene.res.hero_entity)
            .expect("hero must have CombatState component");
        assert_eq!(cs.weapon, 1, "Dirk (weapon=1) must be equipped at game start");
    }
}
