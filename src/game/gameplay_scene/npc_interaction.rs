//! NPC conversation dispatch and shop purchase handling.
//! See `docs/spec/npcs-dialogue.md` for specification.

use super::*;

impl GameplayScene {
    /// Handle dialogue with the nearest NPC/setfig. Ports fmain.c:4188-4261.
    pub(crate) fn handle_setfig_talk(&mut self, fig: &NearestFig, bname: &str) {
        match &fig.kind {
            FigKind::Npc(idx) => {
                // Enemy NPC — per `reference/logic/npc-dialogue.md` talk_dispatch
                // (`fmain.c:3422`): ENEMY type fires `speak(an.race)`; races 0..9
                // map 1:1 to speech indices 0..9.  Setfig-race NPCs
                // (RACE_SHOPKEEPER/BEGGAR/WITCH/SPECTRE/GHOST) retain their
                // setfig-switch speech indices from `talk_dispatch` so that any
                // port-specific spawning through `npc_table` still yields the
                // correct line.
                if let Some(ref table) = self.npc_table {
                    if let Some(npc) = table.npcs.get(*idx) {
                        use crate::game::npc::*;
                        let speech_id: usize = match npc.race {
                            RACE_SHOPKEEPER => 12,
                            RACE_BEGGAR => 23,
                            RACE_WITCH => 46,
                            RACE_SPECTRE => 47,
                            RACE_GHOST => 49,
                            r if r < 10 => r as usize,
                            _ => 6,
                        };
                        self.messages
                            .push(crate::game::events::speak(&self.narr, speech_id, bname));
                    }
                }
            }
            FigKind::SetFig {
                world_idx,
                setfig_type,
            } => {
                let k = *setfig_type as usize;
                let sf_goal = self
                    .state
                    .world_objects
                    .get(*world_idx)
                    .map_or(0u8, |o| o.goal) as usize;
                // SPEC §13.2 / R-NPC-020: enter TALKING flicker state for 15
                // ticks if this SetFig has can_talk = true
                // (Wizard, Priest, King, Ranger, Beggar — fmain.c:3376-3377).
                if k < crate::game::sprites::SETFIG_TABLE.len()
                    && crate::game::sprites::SETFIG_TABLE[k].can_talk
                {
                    self.talk_flicker.insert(*world_idx, 15);
                }
                // Per-setfig dialogue (fmain.c:4188-4261).
                match k {
                    0 => {
                        // Wizard (SPEC §13.1): kind < 10 → speak(35), else speak(27 + goal).
                        if self.state.kind < 10 {
                            self.messages
                                .push(crate::game::events::speak(&self.narr, 35, bname));
                        } else {
                            self.messages.push(crate::game::events::speak(
                                &self.narr,
                                27 + sf_goal,
                                bname,
                            ));
                        }
                    }
                    1 => {
                        // Priest (SPEC §13.1): kind < 10 → speak(40), else speak(36 + daynight%3) + heal.
                        if self.state.kind < 10 {
                            self.messages
                                .push(crate::game::events::speak(&self.narr, 40, bname));
                        } else {
                            let day_mod = (self.state.daynight % 3) as usize;
                            self.messages.push(crate::game::events::speak(
                                &self.narr,
                                36 + day_mod,
                                bname,
                            ));
                            // Heal to 15 + brave/4 (fmain.c:4222).
                            self.state.vitality = 15 + self.state.brave / 4;
                        }
                    }
                    2 | 3 => {
                        // Guard: speak(15).
                        self.messages
                            .push(crate::game::events::speak(&self.narr, 15, bname));
                    }
                    4 => {
                        // Princess (fmain.c:3397): speak(16) only if princess still captive
                        // (ob_list8[9].ob_stat != 0).
                        let princess_captive = self
                            .state
                            .world_objects
                            .get(PRINCESS_OB_INDEX)
                            .map(|obj| obj.ob_stat != 0)
                            .unwrap_or(false);
                        if princess_captive {
                            self.messages
                                .push(crate::game::events::speak(&self.narr, 16, bname));
                        }
                    }
                    5 => {
                        // King (fmain.c:3398): speak(17) only if princess still captive
                        // (ob_list8[9].ob_stat != 0).
                        let princess_captive = self
                            .state
                            .world_objects
                            .get(PRINCESS_OB_INDEX)
                            .map(|obj| obj.ob_stat != 0)
                            .unwrap_or(false);
                        if princess_captive {
                            self.messages
                                .push(crate::game::events::speak(&self.narr, 17, bname));
                        }
                    }
                    6 => {
                        // Noble: speak(20).
                        self.messages
                            .push(crate::game::events::speak(&self.narr, 20, bname));
                    }
                    7 => {
                        // Sorceress: luck boost (fmain.c:4241-4247).
                        if self.state.luck < 64 {
                            self.state.luck += 5;
                        }
                        self.messages
                            .push(crate::game::events::speak(&self.narr, 45, bname));
                    }
                    8 => {
                        // Bartender: fatigue < 5 → speak(13), dayperiod > 7 → speak(12), else speak(14).
                        let speech = if self.state.fatigue < 5 {
                            13
                        } else if self.state.dayperiod > 7 {
                            12
                        } else {
                            14
                        };
                        self.messages
                            .push(crate::game::events::speak(&self.narr, speech, bname));
                    }
                    9 => {
                        // Witch: speak(46).
                        self.messages
                            .push(crate::game::events::speak(&self.narr, 46, bname));
                    }
                    10 => {
                        // Spectre: speak(47).
                        self.messages
                            .push(crate::game::events::speak(&self.narr, 47, bname));
                    }
                    11 => {
                        // Ghost: speak(49).
                        self.messages
                            .push(crate::game::events::speak(&self.narr, 49, bname));
                    }
                    12 => {
                        // Ranger (SPEC §13.1): region 2 → speak(22), else speak(53 + goal).
                        if self.state.region_num == 2 {
                            self.messages
                                .push(crate::game::events::speak(&self.narr, 22, bname));
                        } else {
                            self.messages.push(crate::game::events::speak(
                                &self.narr,
                                53 + sf_goal,
                                bname,
                            ));
                        }
                    }
                    13 => {
                        // Beggar TALK (SPEC §13.1): always speak(23) "Alms! Alms for the poor!"
                        self.messages
                            .push(crate::game::events::speak(&self.narr, 23, bname));
                    }
                    _ => {
                        self.messages
                            .push(crate::game::events::speak(&self.narr, 6, bname));
                    }
                }
            }
            FigKind::Item { .. } => {
                // nearest_fig(1, ...) excludes ground items; unreachable.
            }
        }
    }

    /// BUY menu dispatch (`buy_dispatch`, `fmain.c:3424-3442`).
    ///
    /// `slot` is the menu hit minus 5 (0..=6 ⇒ Food, Arrow, Vial, Mace,
    /// Sword, Bow, Totem).  Mirrors the original's silent-break when
    /// nearest person is not a bartender (`race != 0x88`,
    /// `fmain.c:3426`), the `"Not enough money!"` denial
    /// (`fmain.c:3440`, dialog_system.md:341), and the three per-branch
    /// side-effects / narrations at `fmain.c:3433-3437`.
    pub(crate) fn do_buy_slot(&mut self, slot: usize) {
        use crate::game::npc::RACE_SHOPKEEPER;
        use crate::game::shop::{buy_slot, BuyOutcome, BuyResult};

        // fmain.c:3425-3426 — silent break unless `nearest_person` is a
        // bartender (race 0x88).  In Tambry the bartender is loaded from
        // faery.toml as a SetFig (world_object, ob_stat=3, ob_id=8 =
        // SETFIG_TABLE bartender index), NOT from the per-region NPC
        // table — `nearest_fig` already searches both, so use it here
        // to match the original's `nearest_person` semantics.
        let near_shop = match self.nearest_fig(1, 50) {
            Some(NearestFig {
                kind: FigKind::SetFig { setfig_type, .. },
                ..
            }) => setfig_type == 8,
            Some(NearestFig {
                kind: FigKind::Npc(idx),
                ..
            }) => self
                .npc_table
                .as_ref()
                .and_then(|t| t.npcs.get(idx))
                .map_or(false, |n| n.race == RACE_SHOPKEEPER),
            _ => false,
        };
        if !near_shop {
            return;
        }

        match buy_slot(&mut self.state, slot) {
            BuyResult::Silent => {}
            BuyResult::NotEnough => {
                // dialog_system.md:341 — hard-coded denial literal.
                self.messages.push("Not enough money!");
            }
            BuyResult::Bought(BuyOutcome::Food) => {
                // fmain.c:3433 — event(22) + eat(50).  eat(50) additionally
                // fires event(13) "% was feeling quite full." whenever the
                // meal takes hunger below zero (shops.md, fmain2.c:1704-1708).
                let bname = self.brother_name().to_string();
                self.messages
                    .push(crate::game::events::event_msg(&self.narr, 22, &bname));
                let hunger_before = self.state.hunger;
                self.state.eat_amount(50);
                if hunger_before < 50 {
                    self.messages
                        .push(crate::game::events::event_msg(&self.narr, 13, &bname));
                }
                self.dlog(format!(
                    "BUY food: wealth={}, hunger {}→{}",
                    self.state.wealth, hunger_before, self.state.hunger
                ));
            }
            BuyResult::Bought(BuyOutcome::Arrows) => {
                // fmain.c:3434 — event(23) "% bought some arrows." (10-bundle).
                let bname = self.brother_name().to_string();
                self.messages
                    .push(crate::game::events::event_msg(&self.narr, 23, &bname));
                self.dlog(format!(
                    "BUY arrows: wealth={}, arrows={}",
                    self.state.wealth,
                    self.state.stuff()[8]
                ));
            }
            BuyResult::Bought(BuyOutcome::Item { inv_idx }) => {
                // fmain.c:3436-3437 — extract("% bought a ") + inv_list[i].name
                // + ".".  Literal authorised by dialog_system.md:340.
                let bname = self.brother_name().to_string();
                let item_name = crate::game::world_objects::stuff_index_name(inv_idx);
                self.messages
                    .push(format!("{} bought a {}.", bname, item_name));
                self.dlog(format!(
                    "BUY stuff[{}]++: wealth={}",
                    inv_idx, self.state.wealth
                ));
            }
        }
    }
}
