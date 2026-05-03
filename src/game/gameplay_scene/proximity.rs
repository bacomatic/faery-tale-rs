//! NPC proximity detection and auto-speech triggering.
//! See `docs/spec/npcs-dialogue.md` §13 for specification.

use super::*;

impl GameplayScene {
    pub(crate) fn nearest_fig(&self, constraint: u8, max_dist: i32) -> Option<NearestFig> {
        use crate::game::collision::calc_dist;
        use crate::game::npc::NpcState;
        let hx = self.state.hero_x as i32;
        let hy = self.state.hero_y as i32;

        let mut best: Option<NearestFig> = None;
        let mut best_dist = max_dist;

        // Search enemy NPCs from npc_table
        if let Some(ref table) = self.npc_table {
            for (i, npc) in table.npcs.iter().enumerate() {
                if !npc.active {
                    continue;
                }
                let is_dead = npc.state == NpcState::Dead;
                if constraint == 1 && is_dead {
                    continue;
                }
                let d = calc_dist(hx, hy, npc.x as i32, npc.y as i32);
                if d < best_dist {
                    best_dist = d;
                    best = Some(NearestFig {
                        kind: FigKind::Npc(i),
                        dist: d,
                    });
                }
            }
        }

        // Search world_objects for setfigs (ob_stat=3) and ground items (ob_stat=1/5)
        for (i, obj) in self.state.world_objects.iter().enumerate() {
            if obj.region != self.state.region_num {
                continue;
            }

            if constraint == 1 {
                // Looking for actors: must be visible setfigs only
                if !obj.visible {
                    continue;
                }
                if obj.ob_stat != 3 {
                    continue;
                }
            } else {
                // Looking for items: visible ground items (ob_stat=1) OR hidden items
                // (ob_stat=5) — original Take picks up hidden items without Look first.
                if obj.ob_stat == 3 {
                    continue;
                } // skip setfigs
                if obj.ob_stat == 0 {
                    continue;
                } // skip already-taken items
                if obj.ob_stat != 1 && obj.ob_stat != 5 {
                    continue;
                }
                if !obj.visible && obj.ob_stat != 5 {
                    continue;
                } // skip invisible non-hidden
                if obj.ob_id == 0x1d {
                    continue;
                } // empty chest
            }

            let d = calc_dist(hx, hy, obj.x as i32, obj.y as i32);
            if d < best_dist {
                best_dist = d;
                if obj.ob_stat == 3 {
                    best = Some(NearestFig {
                        kind: FigKind::SetFig {
                            world_idx: i,
                            setfig_type: obj.ob_id,
                        },
                        dist: d,
                    });
                } else {
                    best = Some(NearestFig {
                        kind: FigKind::Item {
                            world_idx: i,
                            ob_id: obj.ob_id,
                        },
                        dist: d,
                    });
                }
            }
        }

        best
    }

    pub(crate) fn update_proximity_speech(&mut self) {
        let fig = match self.nearest_fig(1, PROXIMITY_SPEECH_RANGE) {
            Some(fig) => fig,
            None => {
                self.last_person = None;
                return;
            }
        };

        let person = PersonId::from(&fig.kind);
        if self.last_person == Some(person) {
            return;
        }
        self.last_person = Some(person);

        let bname = self.brother_name().to_string();
        match &fig.kind {
            FigKind::Npc(idx) => {
                if let Some(ref table) = self.npc_table {
                    if let Some(npc) = table.npcs.get(*idx) {
                        use crate::game::npc::{RACE_BEGGAR, RACE_NECROMANCER, RACE_WITCH};
                        const RACE_DREAM_KNIGHT: u8 = 7;
                        let speech_id = match npc.race {
                            RACE_BEGGAR => Some(23),
                            RACE_WITCH => Some(46),
                            RACE_NECROMANCER => Some(43),
                            RACE_DREAM_KNIGHT => Some(41),
                            _ => None,
                        };
                        if let Some(id) = speech_id {
                            self.messages
                                .push(crate::game::events::speak(&self.narr, id, &bname));
                        }
                    }
                }
            }
            FigKind::SetFig { setfig_type, .. } => {
                let speech_id = match *setfig_type {
                    13 => Some(23), // Beggar
                    9 => Some(46),  // Witch
                    4 => {
                        let princess_captive = self
                            .state
                            .world_objects
                            .get(PRINCESS_OB_INDEX)
                            .map(|obj| obj.ob_stat != 0)
                            .unwrap_or(false);
                        if princess_captive {
                            Some(16)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                if let Some(id) = speech_id {
                    self.messages
                        .push(crate::game::events::speak(&self.narr, id, &bname));
                }
            }
            FigKind::Item { .. } => {
                // nearest_fig(1, ...) excludes ground items; unreachable.
            }
        }
    }
}
