//! Scene trait implementation — event handling and per-frame update loop.

use super::*;

impl Scene for GameplayScene {
    fn handle_event(&mut self, event: &Event) -> bool {
        // If rebinding mode is active and waiting for a key, capture the next keypress.
        if self.rebinding.active {
            if let Event::KeyDown { keycode: Some(kc), repeat: false, .. } = event {
                if *kc == Keycode::Escape {
                    self.rebinding.active = false;
                    self.rebinding.waiting_for_action = None;
                    self.dlog("Rebinding mode: false");
                    return true;
                }
                if let Some(action) = self.rebinding.waiting_for_action.take() {
                    self.local_bindings.set_binding(action, vec![*kc]);
                    self.dlog(format!("Rebound {:?} to {:?}", action, kc));
                    return true;
                }
            }
        }
        match event {
            Event::KeyDown { keycode: Some(kc), keymod, repeat: false, .. } => {
                // ALT+F4 → immediate quit (OS convention, takes priority over everything).
                use sdl2::keyboard::Mod;
                let alt_held = keymod.intersects(Mod::LALTMOD | Mod::RALTMOD);
                if alt_held && *kc == Keycode::F4 {
                    self.do_option(GameAction::Quit);
                    return true;
                }
                // ESC: close inventory (viewstatus 4) or map view (viewstatus 1) if open;
                // otherwise do nothing (no quit on ESC — use ALT+F4 instead).
                if *kc == Keycode::Escape {
                    if self.state.viewstatus == 4 || self.state.viewstatus == 1 {
                        self.state.viewstatus = 0;
                    }
                    return true;
                }
                // SPEC §25.9: cheat1 debug keys. Intercepts arrows (teleport override),
                // F9/F10, and a handful of letter/punct keys when cheat1 is enabled.
                if self.state.cheat1 && self.handle_cheat1_key(*kc) {
                    return true;
                }
                match *kc {
                // Movement keys: arrow keys + numpad (no WASD — those are commands)
                Keycode::Up    | Keycode::Kp8 => { self.input.up = true; true }
                Keycode::Down  | Keycode::Kp2 => { self.input.down = true; true }
                Keycode::Left  | Keycode::Kp4 => { self.input.left = true; true }
                Keycode::Right | Keycode::Kp6 => { self.input.right = true; true }
                // Diagonal movement (numpad only)
                Keycode::Kp7 => { self.input.up = true; self.input.left = true; true }
                Keycode::Kp9 => { self.input.up = true; self.input.right = true; true }
                Keycode::Kp1 => { self.input.down = true; self.input.left = true; true }
                Keycode::Kp3 => { self.input.down = true; self.input.right = true; true }
                // Fight: numpad 0 and top-row 0 (keytrans: scancode $0F and $0A both → '0' = KEY_FIGHT_DOWN=48)
                Keycode::Kp0 | Keycode::Num0 => { self.input.fight = true; true }
                // All letter_list keys → route through MenuState
                _ => {
                    if let Some(menu_key) = keycode_to_menukey(*kc) {
                        let action = self.menu.handle_key(menu_key);
                        self.dispatch_menu_action(action);
                        true
                    } else {
                        false
                    }
                }
                }
            },
            Event::KeyUp { keycode: Some(kc), .. } => match *kc {
                Keycode::Up    | Keycode::Kp8 => { self.input.up = false; true }
                Keycode::Down  | Keycode::Kp2 => { self.input.down = false; true }
                Keycode::Left  | Keycode::Kp4 => { self.input.left = false; true }
                Keycode::Right | Keycode::Kp6 => { self.input.right = false; true }
                Keycode::Kp7 => { self.input.up = false; self.input.left = false; true }
                Keycode::Kp9 => { self.input.up = false; self.input.right = false; true }
                Keycode::Kp1 => { self.input.down = false; self.input.left = false; true }
                Keycode::Kp3 => { self.input.down = false; self.input.right = false; true }
                Keycode::Kp0 | Keycode::Num0 => { self.input.fight = false; true }
                _ => false,
            },
            // Controller axis motion: map left stick to movement input
            Event::ControllerAxisMotion { axis, value, .. } => {
                use sdl2::controller::Axis;
                const THRESHOLD: i16 = 8000;
                match axis {
                    Axis::LeftX => {
                        self.input.left  = *value < -THRESHOLD;
                        self.input.right = *value >  THRESHOLD;
                        true
                    }
                    Axis::LeftY => {
                        self.input.up   = *value < -THRESHOLD;
                        self.input.down = *value >  THRESHOLD;
                        true
                    }
                    _ => false,
                }
            }
            Event::ControllerButtonDown { button, .. } => {
                if let Some(action) = self.controller_bindings.action_for_button(
                    self.controller_mode, *button
                ) {
                    if action == GameAction::Fight {
                        self.input.fight = true;
                    } else {
                        self.do_option(action);
                    }
                }
                true
            }
            Event::ControllerButtonUp { button, .. } => {
                if let Some(action) = self.controller_bindings.action_for_button(
                    self.controller_mode, *button
                ) {
                    if action == GameAction::Fight {
                        self.input.fight = false;
                    }
                }
                true
            }
            // Mouse click: close overlay views, or dispatch through MenuState button grid
            Event::MouseButtonDown { x, y, mouse_btn: sdl2::mouse::MouseButton::Left, .. } => {
                // Any click dismisses inventory or map view.
                if self.state.viewstatus == 4 || self.state.viewstatus == 1 {
                    self.state.viewstatus = 0;
                    return true;
                }
                // HIBAR_Y=326, HIBAR_H=114 (2× line-doubled). Button click detection:
                // convert canvas y → native 57px space, then apply propt row pitch (9px).
                const BTN_X_LEFT: i32 = 430;
                const BTN_X_RIGHT: i32 = 482;
                const BTN_X_END: i32 = 530;
                let mx = *x;
                let my = *y;
                if mx >= BTN_X_LEFT && mx <= BTN_X_END
                    && my >= HIBAR_Y && my < HIBAR_Y + HIBAR_H as i32
                {
                    let col = if mx < BTN_X_RIGHT { 0usize } else { 1usize };
                    // Native y within the 57px band; divide by propt row pitch (9) to get row.
                    let native_y = (my - HIBAR_Y) / 2;
                    let row = (native_y / 9) as usize;
                    let slot = row * 2 + col;
                    if slot < 12 {
                        let action = self.menu.handle_click(slot);
                        self.dispatch_menu_action(action);
                        return true;
                    }
                }

                // Compass click: activate direction under pointer and begin tracking.
                if self.apply_compass_input_from_canvas(mx, my) {
                    self.input.compass_held = true;
                    return true;
                }

                false
            }
            // Compass drag: while mouse is held inside compass, follow pointer direction.
            Event::MouseMotion { x, y, .. } => {
                if self.input.compass_held {
                    self.apply_compass_input_from_canvas(*x, *y);
                    true
                } else {
                    false
                }
            }
            Event::MouseButtonUp { mouse_btn: sdl2::mouse::MouseButton::Left, .. } => {
                if self.input.compass_held {
                    self.input.up    = false;
                    self.input.down  = false;
                    self.input.left  = false;
                    self.input.right = false;
                    self.input.compass_held = false;
                    true
                } else {
                    false
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
        self.tick_accum += delta_ticks;

        // Apply pending audio toggles (SPEC §25.5 GAME).
        if let Some(on) = self.pending_music_toggle.take() {
            if let Some(audio) = resources.audio {
                audio.set_music_enabled(on);
                if on {
                    let mood = self.setmood();
                    audio.set_score(mood);
                }
            }
        }
        if let Some(on) = self.pending_sound_toggle.take() {
            if let Some(audio) = resources.audio {
                audio.set_sfx_enabled(on);
            }
        }

        // When paused, skip game logic but keep rendering.
        if self.menu.is_paused() {
            self.render_by_viewstatus(canvas, resources);
            return SceneResult::Continue;
        }

        let tick_events = self.state.tick(delta_ticks);
        self.state.cycle = self.state.cycle.wrapping_add(delta_ticks);
        if !tick_events.is_empty() {
            let bname = self.brother_name().to_string();
            for ev in tick_events {
                let msg = crate::game::events::event_msg(&self.narr, ev as usize, &bname);
                if !msg.is_empty() {
                    self.messages.push(msg);
                }
                if ev == 12 || ev == 24 {
                    self.sleeping = true;
                }
            }
        }

        // Sleep loop: advance time quickly, reduce fatigue, wake when rested
        if self.sleeping {
            let should_wake = self.state.sleep_advance_daynight();
            if should_wake {
                self.sleeping = false;
            }
            self.render_by_viewstatus(canvas, resources);
            return SceneResult::Continue;
        }

        // Lazy-load ADF + world data on first tick (render-world-load).
        // ADF path comes from faery.toml [disk].adf; falls back to the default filename.
        // Errors are logged to stderr; missing ADF is gracefully handled.
        if !self.adf_load_attempted {
            self.adf_load_attempted = true;
            let adf_path = game_lib
                .disk
                .as_ref()
                .map(|d| d.adf.as_str())
                .unwrap_or("game/image");
            match crate::game::adf::AdfDisk::open(std::path::Path::new(adf_path)) {
                Ok(adf) => {
                    let region = self.state.region_num;
                    let world_result = if let Some(cfg) = game_lib.find_region_config(region) {
                        let map_blocks: Vec<u32> = if region < 8 {
                            Self::outdoor_map_blocks(game_lib)
                        } else {
                            vec![cfg.map_block]
                        };
                        crate::game::world_data::WorldData::load(
                            &adf, region,
                            cfg.sector_block, &map_blocks,
                            cfg.terra_block, cfg.terra2_block,
                            &cfg.image_blocks,
                        )
                    } else {
                        Err(anyhow::anyhow!("no region config for region {}", region))
                    };
                    match world_result {
                        Ok(world) => {
                            self.base_colors_palette = Self::build_base_colors_palette(game_lib, region);
                            self.current_palette = Self::region_palette(game_lib, region);
                            self.palette_dirty = true; // force recompute next cadence tick
                            // Load global shadow_mem bitmask table (sprite-depth masking).
                            let shadow_mem = if let Some(ref disk) = game_lib.disk {
                                if disk.shadow_count > 0 {
                                    crate::game::world_data::load_shadow_mem(&adf, disk.shadow_block, disk.shadow_count)
                                } else {
                                    Vec::new()
                                }
                            } else {
                                Vec::new()
                            };
                            self.shadow_mem = shadow_mem;
                            let renderer = MapRenderer::new(&world, self.shadow_mem.clone());
                            // npc-101: load NPC table for the starting region
                            self.npc_table = Some(crate::game::npc::NpcTable::load(&adf, region));
                            self.state.populate_world_objects(game_lib);
                            // sprite-101: load player (cfile 0-2), enemies (cfile 4-12), and setfig (cfile 13-17) sprites
                            for cfile_idx in [0u8, 1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17] {
                                if let Some(sheet) = crate::game::sprites::SpriteSheet::load(
                                    &adf, cfile_idx,
                                ) {
                                    self.dlog(format!(
                                        "sprite-load: cfile {} → {} frames",
                                        cfile_idx, sheet.num_frames
                                    ));
                                    self.sprite_sheets[cfile_idx as usize] = Some(sheet);
                                }
                            }
                            // Load objects sprite sheet (cfile 3, 16×16) for inventory screen.
                            self.object_sprites = crate::game::sprites::SpriteSheet::load_objects(
                                &adf,
                            );
                            self.map_world = Some(world);
                            self.map_renderer = Some(renderer);
                            self.adf = Some(adf);
                            self.dlog(format!("render-world-load: world loaded for region {}", region));
                        }
                        Err(e) => self.dlog(format!("render-world-load: WorldData::load failed: {e}")),
                    }
                }
                Err(e) => self.dlog(format!("render-world-load: AdfDisk::open failed (ADF may not be present): {e}")),
            }
        }


        // SPEC §17.5: day_fade() — update palette every 4 ticks (daynight & 3 == 0) or
        // during screen rebuild (viewstatus > 97), or when palette_dirty is set (region change).
        if Self::should_update_palette(self.state.daynight, self.state.viewstatus) || self.palette_dirty {
            self.palette_dirty = false;
            if let Some(ref base) = self.base_colors_palette {
                let lightlevel = self.state.lightlevel;
                let light_on = self.state.light_timer > 0;
                let secret_active = self.state.region_num == 9 && self.state.secret_timer > 0;
                self.current_palette = Self::compute_current_palette(
                    base,
                    self.state.region_num,
                    lightlevel,
                    light_on,
                    secret_active,
                );
            }
        }

        // colorplay() — Reference: fmain2.c:425-431.
        // When active (triggered by TriggerTeleportEffect), override palette entries 1..31
        // with random 12-bit RGB4 values every tick for 32 frames.  This runs after the
        // normal palette update so that the storm always takes precedence.
        if let Some(storm) = self.teleport_effect.tick() {
            use crate::game::palette::amiga_color_to_rgba;
            for (i, &c) in storm.iter().enumerate() {
                self.current_palette[i + 1] = amiga_color_to_rgba(c);
            }
        }

        // SPEC §17.4: Spectre night visibility toggle (ob_listg[5])
        // When lightlevel < 40 (deep night): visible, otherwise hidden.
        self.update_spectre_visibility();

        // Fatigue is updated per movement step in apply_player_input (player-111).


        // setmood: check music group every 4 ticks (gameloop-113)
        self.mood_tick += delta_ticks;
        if self.mood_tick >= 4 {
            self.mood_tick = 0;
            let mood = self.setmood();
            if mood != self.last_mood {
                self.last_mood = mood;
                self.dlog(format!("setmood: switching to group {}", mood));
                if let Some(audio) = resources.audio {
                    // set_score now handles the music_enabled check internally
                    audio.set_score(mood);
                }
            }
        }


        // Indoor/outdoor mode detection (world-108)
        let indoor = self.state.region_num > 7;
        if indoor != self.last_indoor {
            if indoor {
                self.dlog(format!("{:?}", crate::game::game_event::GameEvent::EnterIndoor { door_index: self.state.region_num }));
            } else {
                self.dlog(format!("{:?}", crate::game::game_event::GameEvent::ExitIndoor));
            }
            self.last_indoor = indoor;
        }

        // Encounter zone check (world-111)
        self.in_encounter_zone = crate::game::zones::in_encounter_zone(
            &self.zones, self.state.hero_x, self.state.hero_y);

        // Event zone entry check (#107)
        {
            let hx = self.state.hero_x;
            let hy = self.state.hero_y;
            let current_zone = crate::game::zones::find_zone(&self.zones, hx, hy);
            if current_zone != self.last_zone {
                // Update xtype from zone etype when zone changes
                if let Some(zone_idx) = current_zone {
                    if zone_idx < self.zones.len() {
                        self.state.xtype = self.zones[zone_idx].etype as u16;
                    }
                }

                // SPEC §15.6: Princess rescue trigger (xtype == 83)
                if let Some(zone_idx) = current_zone {
                    if zone_idx < self.zones.len() && self.zones[zone_idx].etype == 83 {
                        // Check if princess is captive (ob_list8[9].ob_stat != 0)
                        // ob_list8 is region 8, but the princess object should be global
                        // Looking at the spec, ob_list8[9] is a specific world object.
                        // We need to find the princess object in world_objects.
                        if self.state.world_objects.len() > PRINCESS_OB_INDEX {
                            let princess_captive = self.state.world_objects[PRINCESS_OB_INDEX].ob_stat != 0;
                            if princess_captive {
                                self.trigger_princess_rescue = true;
                            }
                        }
                    }
                }

                self.last_zone = current_zone;
            }
        }

        // SPEC §15.6: Princess rescue sequence
        if self.trigger_princess_rescue {
            self.trigger_princess_rescue = false;
            self.enqueue_princess_rescue_sequence();
        }

        // Encounter spawning (npc-104): trigger random encounter when in encounter zone.
        // SPEC §19.3: suppressed while freeze_timer > 0.
        if self.in_encounter_zone && self.state.freeze_timer == 0 {
            let trigger = self.npc_table.as_ref().and_then(|table| {
                crate::game::encounter::try_trigger_encounter(
                    self.state.tick_counter,
                    table,
                    self.state.hero_x as i16,
                    self.state.hero_y as i16,
                    self.state.xtype,
                    self.state.region_num,
                    self.state.active_carrier,
                )
            });
            if let Some(encounter_type) = trigger {
                if let Some(ref mut table) = self.npc_table {
                    crate::game::encounter::spawn_encounter_group(
                        table,
                        encounter_type,
                        self.state.hero_x as i16,
                        self.state.hero_y as i16,
                        self.state.tick_counter,
                    );
                }
            }
        }

        // Death / revive cycle (SPEC §20.2, gameloop-106)
        self.tick_goodfairy_countdown(game_lib, delta_ticks);

        // Run one simulation step per 30 Hz tick (NTSC interlaced frame rate).
        for _ in 0..delta_ticks {
            self.tick_narrative_sequence();
            self.execute_active_narrative_step(game_lib);

            // Phase 6 — fiery-death zone flag (fmain.c:1384-1385); must precede Phase 7
            // so that resolve_player_state can read the correct fiery_death value when
            // deciding whether the swan can be dismounted (fmain.c:1418).
            self.update_fiery_death();
            // Phase 7 — player state resolution (fmain.c:1387-1459)
            if !self.dying {
                self.apply_player_input();
            }

            self.update_environ();
            self.apply_environ_damage();

            // Phase 9 — actor processing loop (fmain.c:1476-1826)
            self.update_actors(1);
            self.update_turtle_autonomous();
            self.update_proximity_speech();

            // Phase 15 — melee hit detection (fmain.c:2262-2296)
            self.run_combat_tick();

            // Phase 16 — missile tick (fmain.c:2298-2340): runs after Phase 9 actors have
            // moved so hit-detection uses up-to-date NPC positions, and after Phase 15 melee.
            {
                let hero_x = self.state.hero_x as i32;
                let hero_y = self.state.hero_y as i32;
                // Snapshot NPC positions to avoid simultaneous mutable borrow conflicts.
                let npc_positions: Vec<(usize, i32, i32)> = self.npc_table.as_ref().map_or(vec![], |t| {
                    t.npcs.iter().enumerate()
                        .filter(|(_, n)| n.active && n.state != crate::game::npc::NpcState::Dead)
                        .map(|(i, n)| (i, n.x as i32, n.y as i32))
                        .collect()
                });
                let mut hero_missile_damage: i16 = 0;
                let mut npc_hits: Vec<(usize, i16)> = vec![];
                for missile in self.missiles.iter_mut() {
                    if !missile.active { continue; }
                    // Age expiry: original fmain.c:2274 / combat.md#missile_step —
                    // missile dies after 40 ticks of flight.
                    if missile.time_of_flight > 40 {
                        missile.active = false;
                        continue;
                    }
                    missile.time_of_flight = missile.time_of_flight.saturating_add(1);
                    missile.x += missile.dx;
                    missile.y += missile.dy;
                    if missile.x < 0 || missile.x > 32768 || missile.y < 0 || missile.y > 32768 {
                        missile.active = false;
                        continue;
                    }
                    // Use correct hit radius per SPEC §10.4
                    let radius = match missile.missile_type {
                        crate::game::combat::MissileType::Arrow => 6,
                        crate::game::combat::MissileType::Fireball => 9,
                    };
                    if missile.is_friendly {
                        for &(npc_idx, nx, ny) in &npc_positions {
                            if (missile.x - nx).abs() < radius && (missile.y - ny).abs() < radius {
                                missile.active = false;
                                npc_hits.push((npc_idx, missile.damage()));
                                break;
                            }
                        }
                    } else if (missile.x - hero_x).abs() < radius && (missile.y - hero_y).abs() < radius {
                        missile.active = false;
                        hero_missile_damage += missile.damage();
                    }
                }
                if let Some(ref mut table) = self.npc_table {
                    for (npc_idx, dmg) in npc_hits {
                        table.npcs[npc_idx].vitality -= dmg;
                        if table.npcs[npc_idx].vitality <= 0 {
                            // F9.11: missile kill leaves a searchable body.
                            // SPEC §10.4 / `fmain.c:2334`. The original sets
                            // `vitality = 0` and runs the same checkdead
                            // path; here we route through `mark_dead` to
                            // keep `active=true` for TAKE → search_body.
                            table.npcs[npc_idx].mark_dead();
                        }
                    }
                }
                self.state.vitality -= hero_missile_damage;
            }

            let (new_map_x, new_map_y) = Self::map_adjust(
                self.state.hero_x, self.state.hero_y,
                self.map_x, self.map_y,
            );
            self.map_x = new_map_x;
            self.map_y = new_map_y;
            self.state.map_x = self.map_x;
            self.state.map_y = self.map_y;
        }

        // Region transition check (world-109): must run after movement so that on_region_changed()
        // loads the new world data before compose() runs — otherwise compose() sees the new
        // region_num (wrong xreg/yreg) with the old map_world, producing a one-frame glitch.
        let region = self.state.region_num;
        if region != self.last_region_num {
            self.on_region_changed(region, game_lib);
            self.dlog(format!("region_num changed: {} -> {} ({:?})", self.last_region_num, region,
                crate::game::game_event::GameEvent::RegionTransition { region }));
            // Cave instrument swap: region 9 uses new_wave[10] = 0x0307 (audio-105).
            if let Some(audio) = resources.audio {
                audio.set_cave_mode(region == 9);
            }
            let from = self.palette_transition
                .as_ref()
                .map(|pt| pt.to)
                .unwrap_or([crate::game::palette::BLACK; crate::game::palette::PALETTE_SIZE]);
            let to = Self::region_palette(game_lib, region);
            self.palette_transition = Some(crate::game::palette::PaletteTransition::new(from, to));
            self.last_region_num = region;
        }
        if let Some(ref mut pt) = self.palette_transition {
            if !pt.is_done() {
                let palette = pt.tick();
                self.current_palette = palette;
            }
        }

        // Compose map viewport when in normal play view (world-105).
        // Pass pixel-precise map_x/map_y so compose() can apply the sub-tile offset.
        if self.state.viewstatus == 0 {
            if let (Some(ref mut mr), Some(ref world)) = (&mut self.map_renderer, &self.map_world) {
                mr.compose(self.map_x, self.map_y, world);
            }
            // Blit actors on top of the composed tiles (sprite-104).
            // Collect borrow-safe parameters before taking &mut map_renderer.
            let map_x = self.map_x;
            let map_y = self.map_y;
            if let Some(ref mut mr) = self.map_renderer {
                // --- Unified Y-sorted render pass (fmain2.c:set_objects) ---
                // Build render list for ALL visible entities, sort by Y, render in order.
                use crate::game::map_renderer::{MAP_DST_W, MAP_DST_H};
                use crate::game::sprite_mask::{apply_sprite_mask, BlittedSprite};
                use crate::game::sprites::{SPRITE_W, SPRITE_H, OBJ_SPRITE_H, SETFIG_TABLE, STATELIST};

                let fb_w = MAP_DST_W as i32;
                let fb_h = MAP_DST_H as i32;

                #[derive(Clone, Copy)]
                enum RenderKind {
                    Hero,
                    Enemy(usize),
                    WorldObj(usize),
                    SetFig(usize),
                }
                struct RenderEntry {
                    abs_y: u16,
                    /// abs_y with render-order biases from fmain.c:2378-2391:
                    ///   dead actor or raft slot → -32 (draw before live actors at same Y)
                    ///   environ > 25 (sinking)  → +32 (draw after others at same Y)
                    ///   carrier while hero riding → -32 (carrier behind mounted hero)
                    sort_y: i32,
                    kind: RenderKind,
                }

                let mut entries: Vec<RenderEntry> = Vec::new();

                // Hero
                {
                    let hero_actor = self.state.actors.first();
                    let dead = hero_actor.map_or(false, |a| matches!(a.state, crate::game::actor::ActorState::Dead));
                    let environ = hero_actor.map_or(0i8, |a| a.environ);
                    let sort_y = self.state.hero_y as i32
                        + if dead { -32 } else { 0 }
                        + if environ > 25 { 32 } else { 0 };
                    entries.push(RenderEntry { abs_y: self.state.hero_y, sort_y, kind: RenderKind::Hero });
                }

                // Enemy NPCs (skip setfig-type entries from NpcTable)
                if let Some(ref table) = self.npc_table {
                    use crate::game::npc::{NpcState, NPC_TYPE_RAFT, NPC_TYPE_SWAN, NPC_TYPE_HORSE, NPC_TYPE_DRAGON};
                    let hero_riding = self.state.riding != 0;
                    for (i, npc) in table.npcs.iter().enumerate() {
                        if !npc.active { continue; }
                        if Self::npc_to_setfig_idx(npc.npc_type, npc.race).is_some() { continue; }
                        let dead = npc.state == NpcState::Dead;
                        let is_raft = npc.npc_type == NPC_TYPE_RAFT;
                        let is_carrier = matches!(npc.npc_type, NPC_TYPE_SWAN | NPC_TYPE_HORSE | NPC_TYPE_DRAGON | NPC_TYPE_RAFT);
                        let sort_y = npc.y as i32
                            + if dead || is_raft { -32 } else { 0 }
                            + if is_carrier && hero_riding { -32 } else { 0 };
                        entries.push(RenderEntry { abs_y: npc.y as u16, sort_y, kind: RenderKind::Enemy(i) });
                    }
                }

                // World objects and setfigs from world_objects list
                for (i, obj) in self.state.world_objects.iter().enumerate() {
                    if !obj.visible || obj.region != self.state.region_num { continue; }
                    if obj.ob_stat == 3 {
                        entries.push(RenderEntry { abs_y: obj.y, sort_y: obj.y as i32, kind: RenderKind::SetFig(i) });
                    } else {
                        entries.push(RenderEntry { abs_y: obj.y, sort_y: obj.y as i32, kind: RenderKind::WorldObj(i) });
                    }
                }

                // Sort ascending by adjusted Y (fmain.c:2378-2391)
                entries.sort_by_key(|e| e.sort_y);

                // Collect BlittedSprite info for masking pass
                let mut blitted: Vec<BlittedSprite> = Vec::new();

                for entry in &entries {
                    match entry.kind {
                        RenderKind::Hero => {
                            // Hero blit (unchanged from blit_actors_to_framebuf)
                            let hero_cfile = self.state.brother.saturating_sub(1) as usize;
                            if let Some(Some(ref sheet)) = self.sprite_sheets.get(hero_cfile) {
                                let (rel_x, mut rel_y) = Self::actor_rel_pos(
                                    self.state.hero_x, self.state.hero_y, map_x, map_y,
                                );
                                let environ = self.state.actors.first().map_or(0i8, |a| a.environ);
                                // Environ rendering (fmain.c:3026-3040, passmode==0):
                                //   environ==2:  ystop -= 10 (clip bottom 10 rows, no Y shift)
                                //   environ>29:  fully submerged (splash sprite)
                                //   environ>2:   ystart += environ (shift down, clip bottom)
                                let body_rows: usize = if environ > 29 {
                                    // Fully submerged — render splash sprite instead of body.
                                    // fmain.c:3026-3029: ob_id 97 (still) / 98 (moving), from cfiles[3].
                                    use crate::game::sprites::OBJ_SPRITE_H;
                                    if let Some(ref obj_sheet) = self.object_sprites {
                                        let frame = 97 + (self.state.cycle & 1) as usize;
                                        if let Some(pix) = obj_sheet.frame_pixels(frame) {
                                            let splash_y = rel_y + (SPRITE_H as i32 - OBJ_SPRITE_H as i32);
                                            if rel_x > -(SPRITE_W as i32) && rel_x < fb_w
                                                && splash_y > -(OBJ_SPRITE_H as i32) && splash_y < fb_h
                                            {
                                                let sprite_info = BlittedSprite {
                                                    screen_x: rel_x,
                                                    screen_y: splash_y,
                                                    width: SPRITE_W,
                                                    height: OBJ_SPRITE_H,
                                                    ground: splash_y + OBJ_SPRITE_H as i32,
                                                    is_falling: false,
                                                };
                                                Self::blit_obj_to_framebuf(pix, rel_x, splash_y, OBJ_SPRITE_H, &mut mr.framebuf, fb_w, fb_h);
                                                apply_sprite_mask(mr, &sprite_info, self.state.hero_sector, 0);
                                                blitted.push(sprite_info);
                                            }
                                        }
                                    }
                                    continue;
                                } else if environ == 2 {
                                    // Shallow water: clip bottom 10 rows, no Y shift
                                    SPRITE_H.saturating_sub(10)
                                } else if environ > 2 {
                                    rel_y += environ as i32;
                                    SPRITE_H.saturating_sub(environ as usize)
                                } else {
                                    SPRITE_H
                                };
                                if rel_x > -(SPRITE_W as i32) && rel_x < fb_w
                                    && rel_y > -(SPRITE_H as i32) && rel_y < fb_h
                                {
                                    let hero_facing = self.state.actors.first().map_or(0u8, |a| a.facing);
                                    let is_moving = self.state.actors.first().map_or(false, |a| a.moving);
                                    let hero_state = self.state.actors.first().map(|a| &a.state);
                                    let frame = if let Some(ActorState::Fighting(fight_state)) = hero_state {
                                        let fight_base = Self::facing_to_fight_frame_base(hero_facing);
                                        fight_base + (*fight_state as usize).min(8)
                                    } else {
                                        let frame_base = Self::facing_to_frame_base(hero_facing);
                                        if is_moving { frame_base + (self.state.cycle as usize) % 8 } else { frame_base + 1 }
                                    };
                                    // Body sprite frame: for fighting, use STATELIST figure field; for walking/still, frame is already correct
                                    let body_frame = if let Some(ActorState::Fighting(_)) = hero_state {
                                        STATELIST[frame].figure as usize
                                    } else {
                                        frame
                                    };

                                    // Weapon draw order (fmain.c:2907-2916 passmode):
                                    // Original facing: 0=NW,1=N,2=NE,3=E,4=SE,5=S,6=SW,7=W
                                    // Rust facing:     0=N, 1=NE,2=E, 3=SE,4=S, 5=SW,6=W, 7=NW
                                    // (orig_facing - 2) & 4 → behind for orig 0,1,6,7 = NW,N,SW,W
                                    // Mapped to Rust: N(0), SW(5), W(6), NW(7).
                                    let weapon_behind = matches!(hero_facing, 0 | 5 | 6 | 7);

                                    // Build BlittedSprite for masking
                                    let sprite_info = BlittedSprite {
                                        screen_x: rel_x,
                                        screen_y: rel_y,
                                        width: SPRITE_W,
                                        height: SPRITE_H,
                                        ground: rel_y + SPRITE_H as i32,
                                        is_falling: false,
                                    };

                                    // Prepare weapon blit parameters
                                    let weapon_type = self.state.actors.first().map_or(0u8, |a| a.weapon);
                                    let wpn_blit = if let Some(ref obj_sheet) = self.object_sprites {
                                        Self::compute_weapon_blit(frame, hero_facing, weapon_type, obj_sheet, rel_x, rel_y)
                                    } else { None };

                                    // Draw weapon BEHIND body when facing N/SW/W/NW
                                    if weapon_behind {
                                        if let Some((wfp, wx, wy, wh)) = wpn_blit {
                                            Self::blit_obj_to_framebuf(wfp, wx, wy, wh, &mut mr.framebuf, fb_w, fb_h);
                                        }
                                    }

                                    if let Some(fp) = sheet.frame_pixels(body_frame) {
                                        Self::blit_sprite_to_framebuf(fp, rel_x, rel_y, body_rows, &mut mr.framebuf, fb_w, fb_h);
                                    }

                                    // Draw weapon IN FRONT when facing NE/E/SE/S
                                    if !weapon_behind {
                                        if let Some((wfp, wx, wy, wh)) = wpn_blit {
                                            Self::blit_obj_to_framebuf(wfp, wx, wy, wh, &mut mr.framebuf, fb_w, fb_h);
                                        }
                                    }

                                    // Mask AFTER blit: restore foreground terrain over the body.
                                    // should_apply_terrain_mask bypasses (fmain.c:2563-2566):
                                    //   hero on swan (riding==11) and hero in fiery_death zone skip masking.
                                    let hero_skip_mask = self.state.riding == 11 || self.fiery_death;
                                    if !hero_skip_mask {
                                        apply_sprite_mask(mr, &sprite_info, self.state.hero_sector, 0);
                                    }

                                    // Mask the weapon separately (original uses two-pass masking:
                                    // one for body, one for weapon, each with its own bounding box
                                    // but sharing the body's ground line — fmain.c:2921-3184).
                                    if !hero_skip_mask {
                                        if let Some((_, wx, wy, wh)) = wpn_blit {
                                            let wpn_info = BlittedSprite {
                                                screen_x: wx,
                                                screen_y: wy,
                                                width: SPRITE_W,
                                                height: wh,
                                                ground: sprite_info.ground,
                                                is_falling: false,
                                            };
                                            apply_sprite_mask(mr, &wpn_info, self.state.hero_sector, 0);
                                        }
                                    }

                                    blitted.push(sprite_info);
                                }
                            }
                        }
                        RenderKind::Enemy(idx) => {
                            if let Some(ref table) = self.npc_table {
                                let npc = &table.npcs[idx];
                                let (cfile_idx, override_frame) =
                                    if let Some((ovr_cfile, ovr_frame)) = Self::swan_grounded_override(npc, &self.state) {
                                        (ovr_cfile, Some(ovr_frame))
                                    } else {
                                        let Some(c) = Self::npc_type_to_cfile(npc.npc_type, npc.race) else { continue };
                                        (c, None)
                                    };
                                let Some(Some(ref sheet)) = self.sprite_sheets.get(cfile_idx) else { continue };

                                let (rel_x, rel_y) = Self::actor_rel_pos(npc.x as u16, npc.y as u16, map_x, map_y);

                                let frame = override_frame
                                    .map(|f| f.min(sheet.num_frames.saturating_sub(1)))
                                    .unwrap_or_else(|| Self::npc_animation_frame(npc, idx, self.state.cycle, sheet.num_frames));

                                let sprite_info = BlittedSprite {
                                    screen_x: rel_x,
                                    screen_y: rel_y,
                                    width: SPRITE_W,
                                    height: SPRITE_H,
                                    ground: rel_y + SPRITE_H as i32,
                                    is_falling: false,
                                };

                                // Weapon draw order: same (facing - 2) & 4 rule as hero.
                                let weapon_behind = matches!(npc.facing, 0 | 5 | 6 | 7);
                                let wpn_blit = if npc.weapon > 0 && npc.weapon < 8 {
                                    if let Some(ref obj_sheet) = self.object_sprites {
                                        Self::compute_weapon_blit(frame, npc.facing, npc.weapon, obj_sheet, rel_x, rel_y)
                                    } else { None }
                                } else { None };

                                if weapon_behind {
                                    if let Some((wfp, wx, wy, wh)) = wpn_blit {
                                        Self::blit_obj_to_framebuf(wfp, wx, wy, wh, &mut mr.framebuf, fb_w, fb_h);
                                    }
                                }
                                if let Some(fp) = sheet.frame_pixels(frame) {
                                    Self::blit_sprite_to_framebuf(fp, rel_x, rel_y, SPRITE_H, &mut mr.framebuf, fb_w, fb_h);
                                }
                                if !weapon_behind {
                                    if let Some((wfp, wx, wy, wh)) = wpn_blit {
                                        Self::blit_obj_to_framebuf(wfp, wx, wy, wh, &mut mr.framebuf, fb_w, fb_h);
                                    }
                                }

                                // should_apply_terrain_mask bypasses (fmain.c:2563-2569):
                                //   CARRIER types never occlude; race 0x85/0x87 are transparent setfigs.
                                use crate::game::npc::{NPC_TYPE_SWAN, NPC_TYPE_HORSE, NPC_TYPE_DRAGON, NPC_TYPE_RAFT,
                                                       RACE_NOMASK_A, RACE_NOMASK_B};
                                let skip_mask = matches!(npc.npc_type, NPC_TYPE_SWAN | NPC_TYPE_HORSE | NPC_TYPE_DRAGON | NPC_TYPE_RAFT)
                                    || npc.race == RACE_NOMASK_A || npc.race == RACE_NOMASK_B;
                                if !skip_mask {
                                    apply_sprite_mask(mr, &sprite_info, self.state.hero_sector, 0);
                                }

                                blitted.push(sprite_info);
                            }
                        }
                        RenderKind::WorldObj(idx) => {
                            let obj = &self.state.world_objects[idx];
                            if let Some(ref obj_sheet) = self.object_sprites {
                                // Resolve sprite display attrs from ob_id.
                                // Many cfile-3 frames are shared between two items
                                // (top half rows 0..7, bottom half rows 8..15) — INV_LIST
                                // describes each item's exact (frame, img_off, img_height)
                                // sub-rectangle. Items lacking an INV_LIST entry
                                // (containers, money, fruit, etc.) use the full 16-row
                                // frame at ob_id.
                                use crate::game::sprites::INV_LIST;
                                use crate::game::world_objects::ob_id_to_stuff_index;
                                let (frame, img_off, img_height) = ob_id_to_stuff_index(obj.ob_id)
                                    .and_then(|s| INV_LIST.get(s))
                                    .map(|it| (
                                        it.image_number as usize,
                                        it.img_off as usize,
                                        it.img_height as usize,
                                    ))
                                    .unwrap_or((obj.ob_id as usize, 0, OBJ_SPRITE_H));
                                if let Some(pix) = obj_sheet.frame_pixels(frame) {
                                    // Use actor_rel_pos_offset so indoor coords (bit 15 set)
                                    // wrap correctly against the indoor map_y origin.
                                    let (rel_x, rel_y) = Self::actor_rel_pos_offset(
                                        obj.x, obj.y, map_x, map_y,
                                        -(SPRITE_W as i32 / 2),
                                        -(img_height as i32 / 2),
                                    );

                                    // Slice into the frame at img_off so blit_obj
                                    // reads only the item's sub-rectangle.
                                    let row_start = img_off * SPRITE_W;
                                    let row_end = row_start + img_height * SPRITE_W;
                                    let pix_clip = if row_end <= pix.len() {
                                        &pix[row_start..row_end]
                                    } else {
                                        pix
                                    };

                                    // Mask BEFORE blit
                                    let sprite_info = BlittedSprite {
                                        screen_x: rel_x,
                                        screen_y: rel_y,
                                        width: SPRITE_W,
                                        height: img_height,
                                        ground: rel_y + img_height as i32,
                                        is_falling: false,
                                    };
                                    Self::blit_obj_to_framebuf(pix_clip, rel_x, rel_y, img_height, &mut mr.framebuf, fb_w, fb_h);

                                    // should_apply_terrain_mask bypass: OBJECTS frames 100-101
                                    // are bubble/spell-effect sprites (fmain.c:2568).
                                    if frame < 100 || frame > 101 {
                                        apply_sprite_mask(mr, &sprite_info, self.state.hero_sector, 0);
                                    }

                                    blitted.push(sprite_info);
                                }
                            }
                        }
                        RenderKind::SetFig(idx) => {
                            let obj = &self.state.world_objects[idx];
                            let setfig_idx = obj.ob_id as usize;
                            if setfig_idx < SETFIG_TABLE.len() {
                                let sf_entry = SETFIG_TABLE[setfig_idx];
                                let cfile_idx = sf_entry.cfile_entry as usize;
                                if let Some(Some(ref sheet)) = self.sprite_sheets.get(cfile_idx) {
                                    // SPEC §13.2 / R-NPC-020: while TALKING (flicker
                                    // timer > 0), add rand2() (0 or 1) to the frame
                                    // index — fmain.c:1556 `dex += rand2()`. This
                                    // produces a random per-tick jitter between two
                                    // adjacent sprite frames only for SetFigs with
                                    // can_talk=true (guarded at entry in handle_setfig_talk).
                                    let jitter = if self.talk_flicker.contains_key(&idx) {
                                        crate::game::combat::bitrand(1) as usize
                                    } else {
                                        0
                                    };
                                    let frame = (sf_entry.image_base as usize + jitter) % sheet.num_frames;
                                    if let Some(fp) = sheet.frame_pixels(frame) {
                                        // Original does ystart = yc - map_y - 8; ystart -= 18 (total: -26).
                                        // actor_rel_pos already applies a Y offset of -26, matching that total,
                                        // so no further adjustment is needed here.
                                        let (rel_x, rel_y) = Self::actor_rel_pos(obj.x, obj.y, map_x, map_y);

                                        // Mask BEFORE blit
                                        let sprite_info = BlittedSprite {
                                            screen_x: rel_x,
                                            screen_y: rel_y,
                                            width: SPRITE_W,
                                            height: SPRITE_H,
                                            ground: rel_y + SPRITE_H as i32,
                                            is_falling: false,
                                        };
                                        Self::blit_sprite_to_framebuf(fp, rel_x, rel_y, SPRITE_H, &mut mr.framebuf, fb_w, fb_h);

                                        // Mask AFTER blit: restore foreground terrain over the sprite
                                        apply_sprite_mask(mr, &sprite_info, self.state.hero_sector, 0);

                                        blitted.push(sprite_info);
                                    }
                                }
                            }
                        }
                    }
                }

                // Per-sprite masking is done after each blit (mask restores foreground terrain)
            }
        }

        self.render_by_viewstatus(canvas, resources);
        if self.quit_requested {
            SceneResult::Quit
        } else if self.victory_triggered {
            // Talisman picked up → transition to victory sequence via main.rs.
            SceneResult::Done
        } else {
            SceneResult::Continue
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
