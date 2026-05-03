//! Environmental state updates — spectre visibility, terrain damage, goodfairy countdown.
//! See `docs/spec/survival.md` and `docs/spec/death-revival.md`.

use super::*;

impl GameplayScene {
    pub(crate) fn update_spectre_visibility(&mut self) {
        let is_night = self.state.lightlevel < 40;
        for obj in &mut self.state.world_objects {
            if obj.region == 255 && obj.ob_id == 10 && obj.ob_stat == 3 {
                obj.visible = is_night;
            }
        }
    }

    pub(crate) fn update_environ(&mut self) {
        let terrain = if let Some(ref world) = self.map_world {
            collision::px_to_terrain_type(world, self.state.hero_x as i32, self.state.hero_y as i32)
        } else {
            return;
        };

        if self.state.on_raft || self.state.flying != 0 {
            if let Some(player) = self.state.actors.first_mut() {
                player.environ = 0;
            }
            return;
        }

        let cur_environ = self.state.actors.first().map_or(0i8, |a| a.environ);
        let mut k: i8 = cur_environ;

        match terrain {
            0 => {
                k = 0;
            }
            6 => {
                k = -1;
            } // slippery (environ -1)
            7 => {
                k = -2;
            } // ice (environ -2)
            8 => {
                k = -3;
            } // lava (environ -3)
            2 => {
                k = 2;
            } // shallow water/wading
            3 => {
                k = 5;
            } // brush/deep wade
            4 | 5 => {
                let threshold: i8 = if terrain == 4 { 10 } else { 30 };
                if k > threshold {
                    k -= 1;
                } else if k < threshold {
                    k += 1;
                    if k > 15 {
                        // Trigger SINK state
                        if let Some(player) = self.state.actors.first_mut() {
                            if !matches!(player.state, ActorState::Dying | ActorState::Dead) {
                                player.state = ActorState::Sinking;
                            }
                        }
                        // fmain.c:1577 — sinking resets frustflag.
                        self.state.frustflag = 0;
                    }
                }
            }
            _ => {} // types 1, 9-15: no environ change from these
        }

        // Reset SINK state when leaving water
        if k == 0 {
            if let Some(player) = self.state.actors.first_mut() {
                if player.state == ActorState::Sinking {
                    player.state = ActorState::Still;
                }
            }
        }

        if let Some(player) = self.state.actors.first_mut() {
            player.environ = k;
        }
    }

    /// Advance the good-fairy rescue countdown one step (SPEC §20.2, T3-COMBAT-GOODFAIRY).
    ///
    /// **Timeline** (countdown starts at 255, decrements ~30 Hz):
    /// - 255–200: death sequence / song (no action)
    /// - 199–120: luck gate fires **once**; luck < 1 → brother succession immediately
    /// - 119–20: fairy sprite flying; `battleflag` cleared
    /// - 19–2:   resurrection glow (no action)
    /// - ≤ 1:    `revive(FALSE)` — fairy rescues hero; countdown ends
    ///
    /// Also initialises the countdown when `vitality ≤ 0` and `dying` is not yet set.
    /// Extracted from `update()` to allow unit-testing without SDL dependencies.
    pub(crate) fn tick_goodfairy_countdown(&mut self, game_lib: &GameLibrary, delta_ticks: u32) {
        // Start countdown on hero death.
        if !self.dying
            && self.state.vitality <= 0
            && !self.state.god_mode.contains(GodModeFlags::INVINCIBLE)
        {
            self.dying = true;
            self.goodfairy = 255;
            self.luck_gate_fired = false;
            self.last_mood = u8::MAX; // force death music re-evaluation
                                      // fmain.c:1725 — dying state resets frustflag.
            self.state.frustflag = 0;

            // SPEC §20.2: every death costs 5 luck (single deduction, any cause).
            // The luck gate below then reads the post-deduction value to decide
            // fairy rescue vs brother succession. Fairy rescue itself has no
            // additional cost.
            self.state.luck = (self.state.luck - 5).max(0);

            // T1-DEATH-MESSAGE: emit death event message (SPEC §20.1)
            let bname = self.brother_name().to_string();
            let death_msg = crate::game::events::event_msg(&self.narr, self.death_type, &bname);
            if !death_msg.is_empty() {
                self.messages.push_wrapped(death_msg);
            }

            self.dlog("death: goodfairy countdown started (255)");
        }

        if self.dying {
            // Decrement each frame (~30 Hz; 255 frames total ≈ 8.5 s).
            self.goodfairy -= delta_ticks as i16;

            // Luck gate: fires once when countdown crosses below 200 (SPEC §20.2 range 199–120).
            // Fully deterministic: luck cannot change during DEAD state.
            if self.goodfairy <= 199 && !self.luck_gate_fired {
                self.luck_gate_fired = true;
                if self.state.luck < 1 {
                    // Luck depleted — the Good Fairy cannot rescue; next brother takes over.
                    self.dying = false;
                    self.luck_gate_fired = false;
                    // brother-succession.md §revive (fmain.c:2837-2840):
                    //   if (brother > 0 && brother < 3) {
                    //     ob_listg[brother].xc = hero_x; yc = hero_y; ob_stat = 1;
                    //     ob_listg[brother + GHOST_OFFSET].ob_stat = 3;
                    //   }
                    // Only the *current* dying brother's bones/ghost are placed.
                    // Slot scheme in port: world_objects[brother] = bones (1=Julian, 2=Phillip),
                    // world_objects[brother+2] = ghost set-figure (3=Julian, 4=Phillip).
                    // Kevin (brother == 3) leaves no bones — game is over.
                    let dying = self.state.brother as usize;
                    if (1..=2).contains(&dying) {
                        let death_x = self.state.hero_x;
                        let death_y = self.state.hero_y;
                        if let Some(bones) = self.state.world_objects.get_mut(dying) {
                            if bones.ob_id == 28 {
                                bones.x = death_x;
                                bones.y = death_y;
                                bones.ob_stat = 1;
                                bones.visible = true;
                            }
                        }
                        if let Some(ghost) = self.state.world_objects.get_mut(dying + 2) {
                            if ghost.ob_id == 10 || ghost.ob_id == 11 {
                                ghost.ob_stat = 3;
                                ghost.visible = true;
                            }
                        }
                    }
                    if let Some(next) = self.state.next_brother() {
                        if let Some(bro) = game_lib.get_brother(next) {
                            let (sx, sy, sr) = game_lib
                                .find_location(&bro.spawn)
                                .map(|loc| (loc.x, loc.y, loc.region))
                                .unwrap_or((19036, 15755, 3));
                            self.state.activate_brother_from_config(
                                next, bro.brave, bro.luck, bro.kind, bro.wealth, sx, sy, sr,
                            );
                        } else {
                            self.state.activate_brother(next);
                        }
                        let previous_brother = dying as u8;

                        self.update_brother_substitution();
                        let succession_keys = match (previous_brother, self.state.brother) {
                            (1, 2) => Some(("julian_dead", "phillip_start")),
                            (2, 3) => Some(("phillip_dead", "kevin_start")),
                            _ => None,
                        };
                        if let Some((dead_key, start_key)) = succession_keys {
                            self.enqueue_succession_placards(dead_key, start_key);
                        }

                        let bname = self.brother_name().to_string();
                        // Original: event(9) + event(10) for Phillip,
                        //           event(9) + event(11) for Kevin.
                        self.messages
                            .push_wrapped(crate::game::events::event_msg(&self.narr, 9, &bname));
                        let cont_id = match self.state.brother {
                            2 => Some(10),
                            3 => Some(11),
                            _ => None,
                        };
                        if let Some(id) = cont_id {
                            self.messages.push_wrapped(crate::game::events::event_msg(
                                &self.narr, id, &bname,
                            ));
                        }
                        self.last_mood = u8::MAX;
                        self.dlog(format!("brother died, {} continues", &bname));
                    } else {
                        // All brothers dead — game over
                        self.quit_requested = true;
                        self.dlog("All brothers dead — GAME OVER");
                    }
                }
                // luck >= 1: countdown continues toward fairy rescue at goodfairy == 1
            }

            // Fairy sprite phase: clear battleflag when fairy is flying (SPEC §20.2 range 119–20).
            if self.goodfairy <= 119 {
                self.state.battleflag = false;
            }

            // T3-COMBAT-GOODFAIRY: Fairy rescue fires at goodfairy ≤ 1 (not 0).
            // SPEC §20.2: "1 | frame 256 | revive(FALSE) — fairy rescues hero, same character."
            if self.goodfairy <= 1 && self.dying {
                self.dying = false;
                self.luck_gate_fired = false;
                // SPEC §20.2: fairy rescue itself has no luck cost.
                // The 5-luck deduction was already applied once at death-init.
                // revive(FALSE): return to safe position with full HP; stats unchanged.
                self.state.hero_x = self.state.safe_x;
                self.state.hero_y = self.state.safe_y;
                self.state.region_num = self.state.safe_r;
                self.state.vitality = crate::game::magic::heal_cap(self.state.brave);
                self.state.hunger = 0;
                self.state.fatigue = 0;
                self.state.battleflag = false;
                // brother-succession.md §revive (fmain.c:2894): revive(FALSE) calls
                // fade_down() with no event/print. The original emits no scroll text on
                // fairy rescue. Do not invent one.
                let bname = self.brother_name().to_string();
                self.last_mood = u8::MAX; // restart normal music
                self.dlog(format!(
                    "faery revived {}, luck now {}",
                    &bname, self.state.luck
                ));
            }
        }
    }

    /// Check if the hero is in the volcanic/lava region.
    /// Mirrors fmain.c:1554: fiery_death = (map_x > 8802 && map_x < 13562 && map_y > 24744 && map_y < 29544).
    pub(crate) fn update_fiery_death(&mut self) {
        let mx = self.state.hero_x as i32;
        let my = self.state.hero_y as i32;
        self.fiery_death = mx > 8802 && mx < 13562 && my > 24744 && my < 29544;
    }

    /// Apply environ-based damage: drowning at environ==30, lava in fiery_death region.
    /// Port of fmain.c:2131-2147.
    pub(crate) fn apply_environ_damage(&mut self) {
        let environ = self.state.actors.first().map_or(0i8, |a| a.environ);

        // Lava damage (fiery_death region, fmain.c:2133-2140)
        if self.fiery_death {
            // Rose (stuff[23]) grants fire immunity
            if self.state.stuff()[23] > 0 {
                if let Some(player) = self.state.actors.first_mut() {
                    player.environ = 0;
                }
            } else if environ > 15 {
                self.state.vitality = 0;
                self.death_type = 27; // lava death (SPEC §20.1)
            } else if environ > 2 {
                let old_vit = self.state.vitality;
                self.state.vitality = (self.state.vitality - 1).max(0);
                if old_vit > 0 && self.state.vitality == 0 {
                    self.death_type = 27; // lava death
                }
            }
        }

        // Drowning damage (fmain.c:2142-2146): environ==30 && (cycle & 7)==0
        if environ as i32 == 30 && (self.state.cycle & 7) == 0 {
            let old_vit = self.state.vitality;
            self.state.vitality = (self.state.vitality - 1).max(0);
            if old_vit > 0 && self.state.vitality == 0 {
                self.death_type = 6; // drowning death (SPEC §20.1)
            }
        }
    }
}
