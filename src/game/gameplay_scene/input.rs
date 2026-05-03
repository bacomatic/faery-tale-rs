//! Player input processing, direction mapping, spell casting, cheat key handling.
//! See `docs/spec/movement-input.md` for specification.

use super::*;

impl GameplayScene {
    pub(crate) fn try_cast_spell(&mut self, item_idx: usize) {
        use crate::game::magic::MagicResult;

        let bname = self.brother_name().to_string();

        // fmain.c:3303 — slot empty ⇒ event(21) "% does not have that item."
        if self.state.stuff()[item_idx] == 0 {
            let msg = crate::game::events::event_msg(&self.narr, 21, &bname);
            self.messages.push(msg);
            return;
        }

        // fmain.c:3304 — extent dampener (astral zone v3==9) ⇒ speak(59).
        let in_necro_arena =
            crate::game::zones::find_zone(&self.zones, self.state.hero_x, self.state.hero_y)
                .and_then(|idx| self.zones.get(idx))
                .map_or(false, |z| z.v3 == 9);
        if in_necro_arena {
            let msg = crate::game::events::speak(&self.narr, 59, &bname);
            self.messages.push(msg);
            return;
        }

        match use_magic(&mut self.state, item_idx) {
            MagicResult::NoOwned => {
                let msg = crate::game::events::event_msg(&self.narr, 21, &bname);
                self.messages.push(msg);
            }
            // fmain.c: precondition miss (wrong sector / region>7 / riding>1)
            // is a silent early-return that preserves the charge.
            MagicResult::Suppressed => {}
            // Green Jewel / Crystal Orb / Gold Ring / Bird Totem: no text.
            MagicResult::Applied => {}
            MagicResult::Healed { capped } | MagicResult::StoneTeleport { capped } => {
                // fmain.c:3351-3352 — `"That feels a lot better!"` is
                // printed only when the heal did NOT clamp at the cap.
                // Source: dialog_system.md:339 (hardcoded literal).
                if !capped {
                    self.messages.push("That feels a lot better!");
                }
            }
            MagicResult::MassKill { in_battle, .. } => {
                // fmain.c:3362 — `if (battleflag) event(34);` — "They're all dead!" he cried.
                if in_battle {
                    let msg = crate::game::events::event_msg(&self.narr, 34, &bname);
                    self.messages.push(msg);
                }
            }
        }

        let wealth = self.state.wealth;
        self.menu.set_options(self.state.stuff(), wealth);
    }

    /// Decode 8-way direction from current input flags.
    pub(crate) fn current_direction(&self) -> Direction {
        match (
            self.input.up,
            self.input.down,
            self.input.left,
            self.input.right,
        ) {
            (true, false, false, false) => Direction::N,
            (true, false, false, true) => Direction::NE,
            (false, false, false, true) => Direction::E,
            (false, true, false, true) => Direction::SE,
            (false, true, false, false) => Direction::S,
            (false, true, true, false) => Direction::SW,
            (false, false, true, false) => Direction::W,
            (true, false, true, false) => Direction::NW,
            _ => Direction::None,
        }
    }

    /// Apply player input: move hero and update actor facing/state.
    pub(crate) fn apply_player_input(&mut self) {
        if self.sleeping {
            return;
        }
        let dir = self.current_direction();

        // Swan dismount — fire button while flying.
        // Ref: reference/logic/carrier-transport.md#swan_dismount
        // (fmain.c:1417-1428). Takes precedence over the fight branch
        // because on the swan the fire button means "dismount", not
        // "attack". `fiery_death` vetos landing over lava (event 32);
        // ±15 velocity gate vetos mid-flight dismount (event 33).
        if self.input.fight && self.state.flying != 0 {
            let bname = self.brother_name().to_string();
            if self.fiery_death {
                // fmain.c:1418 — event(32) "Ground is too hot for swan to land."
                self.messages
                    .push(crate::game::events::event_msg(&self.narr, 32, &bname));
            } else if !self.state.can_dismount_swan() {
                // fmain.c:1427 — event(33) "Flying too fast to dismount."
                self.messages
                    .push(crate::game::events::event_msg(&self.narr, 33, &bname));
            } else {
                // fmain.c:1420-1424 — proxcheck both head (y-14) and feet
                // (y-4) to verify landing spot is clear.
                let hx = self.state.hero_x as i32;
                let hy = self.state.hero_y as i32;
                let land_y = hy - 14;
                let head_clear = collision::proxcheck(self.map_world.as_ref(), hx, land_y);
                let feet_clear = collision::proxcheck(self.map_world.as_ref(), hx, land_y + 10);
                if head_clear && feet_clear {
                    // Commit dismount: clear flight state and land the hero
                    // 14 px above current position. active_carrier stays set
                    // to CARRIER_SWAN so the swan itself remains spawned in
                    // slot 3 (fmain.c: swan "stays at its last position"
                    // after dismount — the extent-driven loader handles
                    // despawn on zone change, not dismount).
                    self.state.stop_swan_flight();
                    self.state.hero_y = land_y as u16;
                    if let Some(player) = self.state.actors.first_mut() {
                        player.abs_y = land_y as u16;
                    }
                }
            }
            self.input.fight = false;
            return;
        }

        // Exclusive fight branch — matches fmain.c where fighting is a separate
        // branch above walking. Movement is suppressed; only facing updates.
        if self.input.fight {
            use crate::game::game_state::ITEM_ARROWS;

            let facing = match dir {
                Direction::N => 0u8,
                Direction::NE => 1,
                Direction::E => 2,
                Direction::SE => 3,
                Direction::S => 4,
                Direction::SW => 5,
                Direction::W => 6,
                Direction::NW => 7,
                Direction::None => self.state.facing,
            };
            self.state.facing = facing;

            let hero_weapon = self.state.actors.first().map_or(1, |a| a.weapon);
            let has_bow = hero_weapon == 4;
            let has_wand = hero_weapon == 5;
            let has_arrows = self.state.stuff()[ITEM_ARROWS] > 0;

            if (has_bow && has_arrows) || has_wand {
                // SHOOT1: aiming. Stay in Shooting state while button held.
                if let Some(player) = self.state.actors.first_mut() {
                    player.facing = facing;
                    player.moving = false;
                    player.state = ActorState::Shooting(0);
                }
            } else {
                // Melee fighting
                let fight_state = match self.state.actors.first() {
                    Some(actor) => match actor.state {
                        ActorState::Fighting(s) => s,
                        _ => 0,
                    },
                    _ => 0,
                };
                let next_state = advance_fight_state(fight_state, self.state.cycle);

                if let Some(player) = self.state.actors.first_mut() {
                    player.facing = facing;
                    player.moving = false;
                    player.state = ActorState::Fighting(next_state);
                }
                // fmain.c:1715 — active melee resets frustflag.
                self.state.frustflag = 0;
            }
            return;
        }

        // Bow/Wand release-to-fire: missile fires on the frame input.fight goes false
        // while hero is in Shooting state (SHOOT1 → SHOOT3 transition).
        if let Some(player) = self.state.actors.first() {
            if matches!(player.state, ActorState::Shooting(_)) {
                use crate::game::combat::fire_missile;
                use crate::game::game_state::ITEM_ARROWS;
                let weapon = player.weapon;

                let can_fire = if weapon == 4 {
                    // Bow requires arrows
                    self.state.stuff()[ITEM_ARROWS] > 0
                } else if weapon == 5 {
                    // Wand has unlimited shots
                    true
                } else {
                    false
                };

                if can_fire {
                    fire_missile(
                        &mut self.missiles,
                        self.state.hero_x as i32,
                        self.state.hero_y as i32,
                        self.state.facing,
                        weapon,
                        true,
                        2, // Standard hero projectile speed
                    );
                    if weapon == 4 {
                        self.state.stuff_mut()[ITEM_ARROWS] -= 1;
                    }
                    // No scroll-area message on fire: original fmain.c emits
                    // no text for bow/wand release. Ref: combat.md#missile_step.
                    let wealth = self.state.wealth;
                    self.menu.set_options(self.state.stuff(), wealth);
                    // fmain.c:1707 — successful shot resets frustflag.
                    self.state.frustflag = 0;
                }

                if let Some(player) = self.state.actors.first_mut() {
                    player.state = ActorState::Still;
                    player.moving = false;
                }
                return;
            }
        }

        // Per-direction base deltas from original xdir/ydir tables (fsubs.asm:1277-1278).
        // Applied as: delta = base * speed / 2  →  cardinal=3px, diagonal=2px at speed=2.
        let (base_dx, base_dy): (i32, i32) = match dir {
            Direction::N => (0, -3),
            Direction::NE => (2, -2),
            Direction::E => (3, 0),
            Direction::SE => (2, 2),
            Direction::S => (0, 3),
            Direction::SW => (-2, 2),
            Direction::W => (-3, 0),
            Direction::NW => (-2, -2),
            Direction::None => (0, 0),
        };

        let prev_x = self.state.hero_x;
        let prev_y = self.state.hero_y;

        // Stagger when starving (hunger > 120, 1-in-4 chance)
        let dir =
            if self.state.hunger > 120 && dir != Direction::None && (self.state.cycle & 3) == 0 {
                let r = (self.state.cycle >> 2) & 1;
                let f = if r == 0 {
                    (self.state.facing + 1) & 7
                } else {
                    (self.state.facing + 7) & 7
                };
                let facing_to_dir = |f: u8| match f {
                    0 => Direction::N,
                    1 => Direction::NE,
                    2 => Direction::E,
                    3 => Direction::SE,
                    4 => Direction::S,
                    5 => Direction::SW,
                    6 => Direction::W,
                    7 => Direction::NW,
                    _ => Direction::None,
                };
                self.state.facing = f;
                facing_to_dir(f)
            } else {
                dir
            };

        if dir != Direction::None {
            // Speed calculation per SPEC §9.5: terrain-modulated via environ.
            // For swan flight (flying != 0), use inertial physics instead of direct movement.
            let speed: i32 = if self.state.flying != 0 {
                0 // speed not used for swan flight (uses velocity instead)
            } else if self.state.on_raft
                && self.state.active_carrier == crate::game::game_state::CARRIER_TURTLE
            {
                // SPEC §21.3: turtle riding forces speed to 3.
                3
            } else {
                use crate::game::combat::hero_speed_for_env;
                let environ = self.state.actors.first().map_or(0i8, |a| a.environ);
                hero_speed_for_env(environ, self.state.on_raft) as i32
            };

            let (dx, dy, facing): (i32, i32, u8) = if self.state.flying != 0 {
                // Swan flight: apply velocity impulse from directional input.
                // xdir/ydir from collision module match the base_dx/base_dy values.
                let (xdir, ydir): (i16, i16) = match dir {
                    Direction::N => (0, -3),
                    Direction::NE => (2, -2),
                    Direction::E => (3, 0),
                    Direction::SE => (2, 2),
                    Direction::S => (0, 3),
                    Direction::SW => (-2, 2),
                    Direction::W => (-3, 0),
                    Direction::NW => (-2, -2),
                    Direction::None => (0, 0),
                };
                self.state.apply_swan_velocity_impulse(xdir, ydir);

                // Position is determined by velocity, not input direction.
                let (new_x, new_y) = self.state.compute_swan_position();
                let dx =
                    (new_x as i32 - self.state.hero_x as i32 + 0x8000).rem_euclid(0x8000) - 0x8000;
                let dy =
                    (new_y as i32 - self.state.hero_y as i32 + 0x8000).rem_euclid(0x8000) - 0x8000;

                // Facing is derived from velocity per SPEC §21.4: set_course(0, -nvx, -nvy, 6).
                // This means facing toward the direction of motion (reversed velocity vector).
                let face_dir = if self.state.swan_vx == 0 && self.state.swan_vy == 0 {
                    self.state.facing // keep current facing when stationary
                } else {
                    // Compute facing from reversed velocity (-vx, -vy).
                    let nvx = -self.state.swan_vx;
                    let nvy = -self.state.swan_vy;
                    // Find closest cardinal/diagonal direction.
                    let angle = (nvy as f32).atan2(nvx as f32);
                    let octant = ((angle / std::f32::consts::PI * 4.0 + 4.5) as i32).rem_euclid(8);
                    // Map octant to facing (0=N, 1=NE, 2=E, etc.)
                    // East=0°, North=90°, West=180°, South=270° in standard coords
                    // But our facing: 0=N, 2=E, 4=S, 6=W
                    // octant 0 = East (2), 2 = North (0), 4 = West (6), 6 = South (4)
                    match octant {
                        0 => 2, // E
                        1 => 1, // NE
                        2 => 0, // N
                        3 => 7, // NW
                        4 => 6, // W
                        5 => 5, // SW
                        6 => 4, // S
                        7 => 3, // SE
                        _ => self.state.facing,
                    }
                };
                (dx, dy, face_dir)
            } else {
                // Normal walking/riding.
                let dx = base_dx * speed / 2;
                let dy = base_dy * speed / 2;

                let facing: u8 = match dir {
                    Direction::N => 0,
                    Direction::NE => 1,
                    Direction::E => 2,
                    Direction::SE => 3,
                    Direction::S => 4,
                    Direction::SW => 5,
                    Direction::W => 6,
                    Direction::NW => 7,
                    Direction::None => 0,
                };
                (dx, dy, facing)
            };

            // Outdoor world wraps at MAXCOORD = 0x8000 = 32768 (USHORT arithmetic).
            // Indoor maps (region >= 8) use y coordinates in the 0x8000–0x9FFF range;
            // wrapping would collapse them to 0–0x1FFF and break doorfind_exit matching.
            let new_x = (self.state.hero_x as i32 + dx).rem_euclid(0x8000) as u16;
            let new_y = if self.state.region_num < 8 {
                (self.state.hero_y as i32 + dy).rem_euclid(0x8000) as u16
            } else {
                (self.state.hero_y as i32 + dy) as u16
            };

            // Turtle guardrail: turtle rides water but cannot enter hard-block terrain (mountains).
            let turtle_blocked = self.state.on_raft
                && self.state.active_carrier == crate::game::game_state::CARRIER_TURTLE
                && self.map_world.as_ref().map_or(false, |world| {
                    collision::px_to_terrain_type(world, new_x as i32, new_y as i32) == 1
                });

            let mut final_x = new_x;
            let mut final_y = new_y;
            let mut final_facing = facing;
            // Gather live NPC positions for actor collision (mirrors original proxcheck actor loop).
            let npc_positions: Vec<(i32, i32)> = if self.state.flying == 0 && !self.state.on_raft {
                self.npc_table
                    .as_ref()
                    .map(|t| {
                        t.npcs
                            .iter()
                            .filter(|n| n.active && n.state != crate::game::npc::NpcState::Dead)
                            .map(|n| (n.x as i32, n.y as i32))
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

            let has_crystal = self.state.stuff()[30] != 0;
            let mut can_move = !turtle_blocked
                && (self.state.flying != 0
                    || self.state.on_raft
                    || (collision::hero_proxcheck(
                        self.map_world.as_ref(),
                        new_x as i32,
                        new_y as i32,
                        has_crystal,
                    ) && !collision::actor_collides(
                        new_x as i32,
                        new_y as i32,
                        &npc_positions,
                    )));

            // Direction deviation (wall-sliding): fmain.c:1615-1625 walk_step.
            // Ref applies the +1 CW / -2 CCW deviation for ALL 8 directions, not just
            // diagonals; cardinal walls must slide the same way (see movement.md#walk_step).
            // Skip deviation when blocked by a door tile (terrain 15) — the player must
            // bump the door to open it, not slide around it.
            let blocked_by_door = !can_move
                && self.map_world.as_ref().map_or(false, |w| {
                    let rt = collision::px_to_terrain_type(w, new_x as i32 + 4, new_y as i32 + 2);
                    let lt = collision::px_to_terrain_type(w, new_x as i32 - 4, new_y as i32 + 2);
                    rt == 15 || lt == 15
                });
            if !can_move
                && !turtle_blocked
                && !blocked_by_door
                && self.state.flying == 0
                && !self.state.on_raft
            {
                // checkdev1: try (facing + 1) & 7
                let dev1 = (facing + 1) & 7;
                let dev1_x = collision::newx(self.state.hero_x, dev1, speed);
                let dev1_y = collision::newy(self.state.hero_y, dev1, speed);
                // Deviation probes use hero lava/pit bypass but NOT crystal bypass (fmain.c:1615).
                if collision::hero_proxcheck(
                    self.map_world.as_ref(),
                    dev1_x as i32,
                    dev1_y as i32,
                    false,
                ) && !collision::actor_collides(dev1_x as i32, dev1_y as i32, &npc_positions)
                {
                    final_x = dev1_x;
                    final_y = dev1_y;
                    final_facing = dev1;
                    can_move = true;
                } else {
                    // checkdev2: try (dev1 - 2) & 7 = (facing - 1) & 7
                    let dev2 = (dev1.wrapping_sub(2)) & 7;
                    let dev2_x = collision::newx(self.state.hero_x, dev2, speed);
                    let dev2_y = collision::newy(self.state.hero_y, dev2, speed);
                    if collision::hero_proxcheck(
                        self.map_world.as_ref(),
                        dev2_x as i32,
                        dev2_y as i32,
                        false,
                    ) && !collision::actor_collides(dev2_x as i32, dev2_y as i32, &npc_positions)
                    {
                        final_x = dev2_x;
                        final_y = dev2_y;
                        final_facing = dev2;
                        can_move = true;
                    }
                }
            }

            // Frustflag update: only while the player is attempting movement.
            // Mirrors walk_step, which is only entered when a direction is active.
            // fmain.c:1650 — successful step resets frustflag (global; no actor-index guard).
            // fmain.c:1654-1656 — all three probes fail → hero-only increment.
            // NPC-walk resets are handled in update_actors after the movement pass.
            if dir != Direction::None {
                if can_move {
                    self.state.frustflag = 0;
                } else if !turtle_blocked {
                    self.state.frustflag = self.state.frustflag.saturating_add(1);
                }
            }

            if can_move {
                self.state.hero_x = final_x;
                self.state.hero_y = final_y;
                // Successful move — hero is no longer blocked by a door, reset dedup flag.
                self.bumped_door = None;
                if self.state.region_num >= 8 {
                    // Indoor (region >= 8): exit check — match on grid-aligned dst coords.
                    // Mirrors fmain.c indoor branch: xtest = hero_x & 0xFFF0, ytest = hero_y & 0xFFE0.
                    // SPEC §21.7 (T1-CARRY-DOOR-BLOCK): All riding values block door entry (and exit).
                    if self.state.riding == 0 {
                        if let Some(door) =
                            crate::game::doors::doorfind_exit(&self.doors, final_x, final_y)
                        {
                            let (ex, ey) = crate::game::doors::exit_spawn(&door);
                            let outdoor_region = Self::outdoor_region_from_pos(ex, ey);
                            self.state.region_num = outdoor_region;
                            self.state.hero_x = ex;
                            self.state.hero_y = ey;
                            self.dlog(format!(
                                "door: indoor exit to region {} ({}, {})",
                                outdoor_region, ex, ey
                            ));
                        }
                    }
                } else if let Some(door) = crate::game::doors::doorfind(
                    &self.doors,
                    self.state.region_num,
                    final_x,
                    final_y,
                ) {
                    // Outdoor (region < 8): walk-on entry check — match on src coords.
                    // Sub-tile position guard mirrors fmain.c Phase-2 nodoor conditions:
                    //   Horizontal (type & 1): skip if hero_y & 0x10 != 0 (lower half — not through yet)
                    //   Vertical             : skip if hero_x & 15 > 6   (right portion — not through yet)
                    let in_doorway = if door.door_type & 1 != 0 {
                        final_y & 0x10 == 0 // horizontal: upper half
                    } else {
                        final_x & 15 <= 6 // vertical: left portion
                    };
                    // SPEC §21.7 (T1-CARRY-DOOR-BLOCK): All riding values block door entry.
                    let not_riding = self.state.riding == 0;
                    // DESERT doors (oasis) require 5 gold statues; original silently blocks if < 5.
                    use crate::game::doors::{key_req, KeyReq};
                    let allow = in_doorway
                        && not_riding
                        && match key_req(door.door_type) {
                            KeyReq::GoldStatues => self.state.stuff()[25] >= 5,
                            _ => true, // walk-on path: door was already opened by bump; NOKEY always allowed
                        };
                    if allow {
                        let (ix, iy) = crate::game::doors::entry_spawn(&door);
                        self.state.region_num = door.dst_region;
                        self.state.hero_x = ix;
                        self.state.hero_y = iy;
                        self.dlog(format!("door: region transition to {}", door.dst_region));
                    }
                }
                // Outdoor region transition: recompute region from position after every move.
                // Mirrors gen_mini() in fmain.c — region switches when the hero crosses a
                // sector-grid boundary, not via an explicit trigger.  Only runs for outdoor
                // regions; door transitions to F9/F10 (>= 8) are handled above and must not
                // be overridden.
                if self.state.region_num < 8 {
                    let pos_region =
                        Self::outdoor_region_from_pos(self.state.hero_x, self.state.hero_y);
                    if pos_region != self.state.region_num {
                        self.dlog(format!(
                            "outdoor region transition: {} -> {} at ({}, {})",
                            self.state.region_num, pos_region, self.state.hero_x, self.state.hero_y,
                        ));
                        self.state.region_num = pos_region;
                    }
                }
                // Track safe spawn point after successful movement.
                if let Some(ref world) = self.map_world {
                    let terrain = collision::px_to_terrain_type(
                        world,
                        self.state.hero_x as i32,
                        self.state.hero_y as i32,
                    );
                    self.state.update_safe_spawn(terrain);
                }
            } else if !turtle_blocked {
                // Check if movement was blocked by a door tile (terrain type 15).
                // Mirrors fmain.c: proxcheck returns 15 → doorfind(xtest, ytest, 0).
                //
                // Two-phase door model (matches original behaviour):
                //   Phase 1 — Bump:      show "It opened." / "It's locked.", record in opened_doors.
                //   Phase 2 — Walk-through: next movement attempt sees opened_doors entry → teleport.
                //
                // This mirrors fmain.c where doorfind() changes sector_mem tiles (making the
                // tile passable) and the actual xfer() teleport fires on the next frame's door scan.
                let right_t = self.map_world.as_ref().map_or(0, |w| {
                    collision::px_to_terrain_type(w, new_x as i32 + 4, new_y as i32 + 2)
                });
                let left_t = self.map_world.as_ref().map_or(0, |w| {
                    collision::px_to_terrain_type(w, new_x as i32 - 4, new_y as i32 + 2)
                });
                let door_tile = right_t == 15 || left_t == 15;
                // probe_x: the probe point that found terrain-15 (used for tile-origin alignment).
                let probe_x = if right_t == 15 {
                    new_x as i32 + 4
                } else {
                    new_x as i32 - 4
                };
                let probe_y = new_y as i32 + 2;
                if door_tile && self.state.region_num < 8 {
                    // Indoor exit is handled by the walk-on branch above (mirrors fmain.c: door
                    // scan runs on hero_x/hero_y after every successful move).
                    use crate::game::doors::{
                        apply_door_tile_replacement, doorfind_nearest_by_bump_radius, key_req,
                        KeyReq,
                    };
                    let region = self.state.region_num;
                    let nearest =
                        doorfind_nearest_by_bump_radius(&self.doors, region, new_x, new_y);
                    if let Some((idx, door)) = nearest {
                        if self.opened_doors.contains(&idx) {
                            // Phase 2 — door was opened; let the hero cross the threshold.
                            // Mirrors fmain.c every-frame door scan sub-tile position check:
                            //   Horizontal (type & 1): teleport only when hero_y & 0x10 == 0
                            //     (upper half of tile — hero walks from lower half into upper).
                            //   Vertical: teleport only when hero_x & 15 <= 6
                            //     (within left portion of tile — hero walks in from right).
                            // Use new_y/new_x (proposed blocked position) as the equivalent
                            // of the original's post-move hero_y/hero_x.
                            let sub_tile_ok = if door.door_type & 1 != 0 {
                                new_y & 0x10 == 0 // horizontal: upper half
                            } else {
                                new_x & 15 <= 6 // vertical: left portion
                            };
                            // SPEC §21.7 (T1-CARRY-DOOR-BLOCK): All riding values block door entry.
                            let not_riding = self.state.riding == 0;
                            if sub_tile_ok && not_riding {
                                let (ix, iy) = crate::game::doors::entry_spawn(&door);
                                self.state.region_num = door.dst_region;
                                self.state.hero_x = ix;
                                self.state.hero_y = iy;
                                self.opened_doors.remove(&idx);
                                self.bumped_door = None;
                                self.dlog(format!(
                                    "door: walk-through to region {}",
                                    door.dst_region
                                ));
                            }
                        } else {
                            // Phase 1 — attempt to open the door.
                            match key_req(door.door_type) {
                                KeyReq::NoKey => {
                                    // Freely-opening doors (wood, city gates, caves, stairs).
                                    if let Some(ref mut world) = self.map_world {
                                        apply_door_tile_replacement(
                                            world,
                                            door.door_type,
                                            probe_x,
                                            probe_y,
                                        );
                                    }
                                    self.messages.push("It opened.");
                                    self.opened_doors.insert(idx);
                                    self.bumped_door = None;
                                    self.dlog(format!("door: opened idx={idx}"));
                                }
                                KeyReq::Key(_) | KeyReq::Talisman => {
                                    // Locked: show message once per approach (mirrors fmain.c bumped flag).
                                    if self.bumped_door != Some(idx) {
                                        self.messages.push("It's locked.");
                                        self.bumped_door = Some(idx);
                                    }
                                }
                                KeyReq::GoldStatues => {
                                    // DESERT/oasis: silently blocks if < 5 gold statues
                                    // (original fmain.c: `if (d->type == DESERT && stuff[STATBASE] < 5) break;`)
                                    if self.state.stuff()[25] >= 5 {
                                        if let Some(ref mut world) = self.map_world {
                                            apply_door_tile_replacement(
                                                world,
                                                door.door_type,
                                                probe_x,
                                                probe_y,
                                            );
                                        }
                                        self.messages.push("It opened.");
                                        self.opened_doors.insert(idx);
                                        self.bumped_door = None;
                                        self.dlog(format!("door: oasis opened idx={idx}"));
                                    }
                                }
                            }
                        }
                    }
                    // No doorlist entry in range: silently block.
                } else {
                    // Not a door block — reset the locked-message dedup.
                    self.bumped_door = None;
                }
            }

            let facing = final_facing;

            let moved = self.state.hero_x != prev_x || self.state.hero_y != prev_y;
            if let Some(player) = self.state.actors.first_mut() {
                player.facing = facing;
                player.moving = moved;
                player.state = ActorState::Walking;
            }
            self.state.facing = facing;
        } else {
            if let Some(player) = self.state.actors.first_mut() {
                player.moving = false;
                player.state = ActorState::Still;
            }
        }

        // Carrier proximity detection (raft + swan — player-107).
        // Ref: reference/logic/carrier-transport.md#compute_raftprox
        // (fmain.c:1455-1464). The original runs a single proxcheck against
        // `anim_list[wcarry]` with `wcarry = 3` when `active_carrier != 0`
        // else `1`, producing raftprox=2 within 9 px, 1 within 16 px, 0
        // otherwise. In the port, raft lives in the NPC table with
        // NPC_TYPE_RAFT and swan lives in the NPC table with NPC_TYPE_SWAN;
        // only one carrier is "active" at a time, so we pick the actor type
        // by active_carrier (or by flying==1 swan latch). Turtle is summoned
        // directly via summon_turtle() and bypasses this block.
        {
            use crate::game::npc::{NPC_TYPE_RAFT, NPC_TYPE_SWAN};
            let hx = self.state.hero_x as i32;
            let hy = self.state.hero_y as i32;

            let find_nearest = |kind: u8, range: i32| -> bool {
                self.npc_table.as_ref().map_or(false, |t| {
                    t.npcs.iter().any(|n| {
                        n.active
                            && n.state != crate::game::npc::NpcState::Dead
                            && n.npc_type == kind
                            && (n.x as i32 - hx).abs() < range
                            && (n.y as i32 - hy).abs() < range
                    })
                })
            };

            // While flying or already latched to the swan extent, keep the
            // swan as the active probe target so dismount (F14.4) isn't
            // stolen by a raft that happens to be nearby.
            let swan_latched = self.state.flying != 0
                || self.state.active_carrier == crate::game::game_state::CARRIER_SWAN;
            // Otherwise, if a swan NPC is within range, prefer swan over
            // raft — this is what the ref's `wcarry = 3 when active_carrier
            // != 0` choice achieves once the carrier-extent loader (F14.5)
            // latches `active_carrier` on zone entry.
            let swan_nearby = find_nearest(NPC_TYPE_SWAN, 16);
            let use_swan = swan_latched || swan_nearby;

            // Get current terrain for raft gating (SPEC §21.2).
            let terrain = self.map_world.as_ref().map_or(0, |world| {
                collision::px_to_terrain_type(
                    world,
                    self.state.hero_x as i32,
                    self.state.hero_y as i32,
                )
            });

            if use_swan {
                // Swan branch — ref carrier-transport.md#carrier_tick
                // (fmain.c:1497-1509): mount when raftprox != 0 && wcarry
                // == 3 && stuff[5] != 0 (Golden Lasso). Swan mount is
                // eligible at "close" (16 px), not just "adjacent" (9 px).
                let within_16 = find_nearest(NPC_TYPE_SWAN, 16);
                let within_9 = find_nearest(NPC_TYPE_SWAN, 9);

                if within_16 {
                    self.state.raftprox = if within_9 { 2 } else { 1 };
                    self.state.active_carrier = crate::game::game_state::CARRIER_SWAN;
                    self.state.wcarry = 3;
                    // Auto-mount when lasso is carried and hero is close
                    // enough (fmain.c:1498). Idempotent — already-flying
                    // swans re-latch per-frame in the original.
                    if self.state.flying == 0 && self.state.has_lasso() {
                        let _ = self.state.start_swan_flight();
                    }
                } else {
                    self.state.raftprox = 0;
                    // Out of range and not flying — release the swan latch
                    // so the raft branch can engage next frame if needed.
                    if self.state.flying == 0 {
                        self.state.active_carrier = 0;
                        self.state.wcarry = 0;
                    }
                }
            } else {
                // Raft branch — unchanged behaviour from prior F14.1 fix.
                let within_16 = find_nearest(NPC_TYPE_RAFT, 16);
                let within_9 = find_nearest(NPC_TYPE_RAFT, 9);
                let raft_aboard = within_9 && self.state.can_board_raft(terrain);

                if raft_aboard {
                    self.state.raftprox = 2;
                    self.state.active_carrier = crate::game::game_state::CARRIER_RAFT;
                    self.state.on_raft = true;
                    self.state.wcarry = 1; // SPEC §21.2: raft is in actor slot 1
                } else if within_16 {
                    self.state.raftprox = 1;
                } else {
                    self.state.raftprox = 0;
                    // Auto-disembark from raft when hero reaches dry land (player-107).
                    if self.state.on_raft
                        && self.state.active_carrier == crate::game::game_state::CARRIER_RAFT
                    {
                        let on_land = terrain < 2;
                        if on_land {
                            self.state.leave_raft();
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn apply_compass_input_from_canvas(&mut self, mx: i32, my: i32) -> bool {
        const COMPASS_X_MIN: i32 = 567;
        const COMPASS_X_MAX: i32 = 567 + 48;
        let compass_y_min = HIBAR_Y + 30; // COMPASS_SRC_Y(15) × 2
        let compass_y_max = HIBAR_Y + 78; // (COMPASS_SRC_Y+COMPASS_SRC_H)(39) × 2
        if mx >= COMPASS_X_MIN && mx < COMPASS_X_MAX && my >= compass_y_min && my < compass_y_max {
            let nx = mx - COMPASS_X_MIN;
            let ny = (my - compass_y_min) / 2; // scale back to native 24px height
            for (idx, &(rx, ry, rw, rh)) in self.compass_regions
                [..8.min(self.compass_regions.len())]
                .iter()
                .enumerate()
            {
                if rw > 0 && rh > 0 && nx >= rx && nx < rx + rw && ny >= ry && ny < ry + rh {
                    // comptable: NW=0,N=1,NE=2,E=3,SE=4,S=5,SW=6,W=7
                    self.input.up = matches!(idx, 0 | 1 | 2);
                    self.input.down = matches!(idx, 4 | 5 | 6);
                    self.input.left = matches!(idx, 0 | 6 | 7);
                    self.input.right = matches!(idx, 2 | 3 | 4);
                    return true;
                }
            }
        }
        // Outside all hitboxes — stop movement while held
        self.input.up = false;
        self.input.down = false;
        self.input.left = false;
        self.input.right = false;
        false
    }

    /// SPEC §25.9: handle a cheat1-gated debug key. Returns true if the key was consumed.
    ///
    /// | Key     | Effect                                                                    |
    /// |---------|---------------------------------------------------------------------------|
    /// | B       | Grant Golden Lasso (`stuff[5]=1`) and summon a grounded swan |
    /// | .       | Add 3 to random `stuff[]` entry (range 0..=30)                            |
    /// | R       | (stub) logs — fairy rescue is invoked automatically on death              |
    /// | =       | (stub) logs — `prq(2)` brother-bio page overlay not yet wired             |
    /// | F9      | Advance `daynight` by 1000                                                |
    /// | F10     | (stub) logs — `prq(3)` brother-bio page overlay not yet wired             |
    /// | ↑ / ↓   | Teleport hero ±150 in Y                                                   |
    /// | ← / →   | Teleport hero ±280 in X                                                   |
    pub(crate) fn handle_cheat1_key(&mut self, kc: Keycode) -> bool {
        use sdl2::keyboard::Keycode as K;
        match kc {
            K::B => {
                self.state.stuff_mut()[5] = 1;
                // Also summon a grounded swan near the hero so the lasso is testable.
                self.apply_command(DebugCommand::SummonSwan);
                self.dlog("cheat1(B): granted Golden Lasso (stuff[5]=1) and summoned swan");
                true
            }
            K::Period => {
                use std::time::SystemTime;
                let nanos = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos();
                let idx = (nanos % 31) as usize;
                let new_val = self.state.stuff()[idx].saturating_add(3);
                self.state.stuff_mut()[idx] = new_val;
                self.dlog(format!("cheat1(.): stuff[{}] += 3 -> {}", idx, new_val));
                true
            }
            K::R => {
                self.dlog("cheat1(R): rescue() not wired; fairy rescue fires on death");
                true
            }
            K::Equals => {
                self.dlog("cheat1(=): prq(2) overlay not yet implemented");
                true
            }
            K::F9 => {
                self.state.daynight = self.state.daynight.wrapping_add(1000);
                self.dlog(format!(
                    "cheat1(F9): daynight += 1000 -> {}",
                    self.state.daynight
                ));
                true
            }
            K::F10 => {
                self.dlog("cheat1(F10): prq(3) overlay not yet implemented");
                true
            }
            K::Up => {
                self.state.hero_y = self.state.hero_y.saturating_sub(150);
                self.dlog(format!("cheat1(↑): hero_y -= 150 -> {}", self.state.hero_y));
                true
            }
            K::Down => {
                self.state.hero_y = self.state.hero_y.saturating_add(150);
                self.dlog(format!("cheat1(↓): hero_y += 150 -> {}", self.state.hero_y));
                true
            }
            K::Left => {
                self.state.hero_x = self.state.hero_x.saturating_sub(280);
                self.dlog(format!("cheat1(←): hero_x -= 280 -> {}", self.state.hero_x));
                true
            }
            K::Right => {
                self.state.hero_x = self.state.hero_x.saturating_add(280);
                self.dlog(format!("cheat1(→): hero_x += 280 -> {}", self.state.hero_x));
                true
            }
            _ => false,
        }
    }
}
