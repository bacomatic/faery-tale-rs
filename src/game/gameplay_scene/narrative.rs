//! Narrative sequence handling — placard queuing, princess rescue, brother succession.
//! See `docs/spec/intro-narrative.md` for specification.

use super::*;

impl GameplayScene {
    pub(crate) fn tick_narrative_sequence(&mut self) {
        self.narrative_queue.tick_one();
    }

    pub(crate) fn execute_active_narrative_step(&mut self, game_lib: &GameLibrary) {
        let Some(step) = self.narrative_queue.active_step().cloned() else {
            return;
        };

        match step {
            NarrativeStep::WaitTicks { .. } => {}
            NarrativeStep::ShowPlacard {
                key,
                substitution,
                hold_ticks,
            } => {
                if hold_ticks > 0 {
                    return;
                }
                if let Some(placard) = game_lib.find_placard(&key) {
                    for line in placard.text_lines_with_substitution(substitution.as_deref()) {
                        self.messages.push_wrapped(line);
                    }
                    self.dlog(format!("narrative: show_placard {}", key));
                } else {
                    self.dlog(format!("fidelity error: missing placard key {}", key));
                }
                self.narrative_queue.advance_active_step();
            }
            NarrativeStep::ClearInnerRect => {
                self.messages.clear();
                self.dlog("narrative: clear_inner_rect");
                self.narrative_queue.advance_active_step();
            }
            NarrativeStep::ShowRescueHomeText {
                line17,
                hero_name,
                line18,
            } => {
                for key in [line17.as_str(), line18.as_str()] {
                    if key.is_empty() {
                        continue;
                    }
                    if let Some(placard) = game_lib.find_placard(key) {
                        for line in placard.text_lines_with_substitution(Some(&hero_name)) {
                            self.messages.push_wrapped(line);
                        }
                    } else {
                        self.dlog(format!("fidelity error: missing placard key {}", key));
                    }
                }
                self.dlog("narrative: rescue_home_text");
                self.narrative_queue.advance_active_step();
            }
            NarrativeStep::TeleportHero { x, y, region } => {
                if x >= 0 && y >= 0 && x <= u16::MAX as i32 && y <= u16::MAX as i32 {
                    self.state.hero_x = x as u16;
                    self.state.hero_y = y as u16;
                    self.state.region_num = region;
                }
                self.narrative_queue.advance_active_step();
            }
            NarrativeStep::MoveExtent { index, x, y } => {
                if !self.state.move_extent_for_script(index, x, y) {
                    self.dlog(format!(
                        "fidelity blocker: move_extent missing target index={} x={} y={}",
                        index, x, y,
                    ));
                }
                self.narrative_queue.advance_active_step();
            }
            NarrativeStep::SwapObjectId {
                object_index,
                new_id,
            } => {
                if !self
                    .state
                    .swap_world_object_id_for_script(object_index, new_id)
                {
                    self.dlog(format!(
                        "fidelity blocker: swap_object_id missing target index={} new_id={}",
                        object_index, new_id,
                    ));
                }
                self.narrative_queue.advance_active_step();
            }
            NarrativeStep::ApplyRescueRewardsAndFlags => {
                self.execute_princess_rescue();
                self.narrative_queue.advance_active_step();
            }
        }
    }

    fn rescue_placard_key_for_princess_count(&self, princess_count: u8) -> &'static str {
        match princess_count {
            0 => "rescue_katra",
            1 => "rescue_karla",
            _ => "rescue_kandy",
        }
    }

    pub(crate) fn enqueue_princess_rescue_sequence(&mut self) {
        if !self.narrative_queue.is_idle() {
            self.dlog("narrative: rescue enqueue deferred behind active sequence");
        }

        let hero_name = self.brother_name().to_string();
        let rescue_key = self
            .rescue_placard_key_for_princess_count(self.state.princess)
            .to_string();
        let mut steps: Vec<NarrativeStep> = vec![NarrativeStep::ShowPlacard {
            key: rescue_key,
            substitution: Some(hero_name.clone()),
            hold_ticks: 72,
        }];

        steps.push(NarrativeStep::WaitTicks { remaining: 380 });
        steps.push(NarrativeStep::ClearInnerRect);
        steps.push(NarrativeStep::ShowRescueHomeText {
            line17: "princess_home".to_string(),
            hero_name,
            line18: String::new(),
        });
        steps.push(NarrativeStep::TeleportHero {
            x: 5511,
            y: 33780,
            region: 0,
        });
        steps.push(NarrativeStep::MoveExtent {
            index: 0,
            x: 22205,
            y: 21231,
        });
        steps.push(NarrativeStep::SwapObjectId {
            object_index: 2,
            new_id: 4,
        });
        steps.push(NarrativeStep::ApplyRescueRewardsAndFlags);

        self.narrative_queue.enqueue(steps);
    }

    pub(crate) fn enqueue_succession_placards(&mut self, dead_key: &str, start_key: &str) {
        if !self.narrative_queue.is_idle() {
            self.dlog("narrative: succession enqueue deferred behind active sequence");
        }

        let sub = Some(self.brother_name().to_string());
        self.narrative_queue.enqueue(vec![
            NarrativeStep::ShowPlacard {
                key: dead_key.to_string(),
                substitution: sub.clone(),
                hold_ticks: 72,
            },
            NarrativeStep::ShowPlacard {
                key: start_key.to_string(),
                substitution: sub,
                hold_ticks: 72,
            },
        ]);
    }

    pub(crate) fn execute_princess_rescue(&mut self) {
        const ITEM_WRIT: usize = 28; // stuff[28] = Writ

        let bname = self.brother_name().to_string();

        // Award Writ (stuff[28] = 1)
        self.state.stuff_mut()[ITEM_WRIT] = 1;

        // Award 100 gold
        self.state.wealth = self.state.wealth.saturating_add(100);

        // Award +3 of each key type (stuff[16..22] are the 6 key types)
        for i in 16..22 {
            let current = self.state.stuff()[i];
            self.state.stuff_mut()[i] = current.saturating_add(3);
        }

        // Increment princess counter
        self.state.princess = self.state.princess.saturating_add(1);

        // Clear princess captive flag
        if self.state.world_objects.len() > PRINCESS_OB_INDEX {
            self.state.world_objects[PRINCESS_OB_INDEX].ob_stat = 0;
            self.state.world_objects[PRINCESS_OB_INDEX].visible = false;
        }

        // King's post-rescue line (fmain2.c:1599, `speak(18)`): the writ
        // designation speech sourced from faery.toml [narr].speeches[18].
        self.messages
            .push(crate::game::events::speak(&self.narr, 18, &bname));

        self.dlog(format!(
            "Princess rescue complete (count={})",
            self.state.princess
        ));
    }

    #[cfg(test)]
    pub(crate) fn debug_enqueue_sequence_for_test(&mut self, steps: Vec<NarrativeStep>) {
        self.narrative_queue.reset(steps);
    }

    #[cfg(test)]
    pub(crate) fn debug_tick_sequence_only(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.tick_narrative_sequence();
        }
    }

    #[cfg(test)]
    pub(crate) fn debug_tick_and_execute_sequence_only(
        &mut self,
        ticks: u32,
        game_lib: &GameLibrary,
    ) {
        for _ in 0..ticks {
            self.tick_narrative_sequence();
            self.execute_active_narrative_step(game_lib);
        }
    }

    #[cfg(test)]
    pub(crate) fn debug_active_step_index(&self) -> Option<usize> {
        self.narrative_queue.active_step_index()
    }

    #[cfg(test)]
    pub(crate) fn debug_advance_active_sequence_step_for_test(&mut self) {
        self.narrative_queue.advance_active_step();
    }

    #[cfg(test)]
    pub(crate) fn debug_narrative_steps(&self) -> Vec<NarrativeStep> {
        self.narrative_queue.debug_snapshot_steps()
    }

    #[cfg(test)]
    pub(crate) fn debug_trigger_princess_rescue_for_test(&mut self) {
        self.enqueue_princess_rescue_sequence();
    }

    #[cfg(test)]
    pub(crate) fn debug_sequence_placard_keys(&self) -> Vec<String> {
        self.narrative_queue
            .debug_snapshot_steps()
            .iter()
            .filter_map(|step| {
                if let NarrativeStep::ShowPlacard { key, .. } = step {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn debug_run_sequence_to_completion(&mut self, game_lib: &GameLibrary) {
        for _ in 0..10_000 {
            self.tick_narrative_sequence();
            self.execute_active_narrative_step(game_lib);
            if self.narrative_queue.debug_snapshot_steps().is_empty() {
                return;
            }
        }
        panic!("sequence did not complete in test helper");
    }

    #[cfg(test)]
    pub(crate) fn debug_extent_position(&self, index: usize) -> Option<(i32, i32)> {
        self.state.scripted_extent_position(index)
    }

    #[cfg(test)]
    pub(crate) fn debug_drain_logs_for_test(&mut self) -> Vec<String> {
        self.drain_logs()
    }
}
