//! Item pickup, body searching, and loot tracking.
//! See `docs/spec/inventory-items.md` for specification.

use super::*;

impl GameplayScene {
    /// Handle taking a specific world item. Ports fmain.c:3880-4000.
    /// Returns true if the item was successfully taken.
    pub(crate) fn handle_take_item(&mut self, world_idx: usize, ob_id: u8, bname: &str) -> bool {
        use crate::game::world_objects::{ob_id_to_stuff_index, stuff_index_name};

        match ob_id {
            // FOOTSTOOL, TURTLE — can't take
            31 | 102 => {
                return false;
            }
            // MONEY — +50 gold
            13 => {
                self.state.gold += 50;
                self.messages.push(format!("{} found 50 gold pieces.", bname));
                self.state.mark_object_taken(world_idx);
                return true;
            }
            // SCRAP OF PAPER (ob_id 20): event 17, then 18 or 19 by region
            20 => {
                let msg17 = crate::game::events::event_msg(&self.narr, 17, bname);
                if !msg17.is_empty() { self.messages.push(msg17); }
                let region_event = if self.state.region_num > 7 { 19 } else { 18 };
                let msg = crate::game::events::event_msg(&self.narr, region_event, bname);
                if !msg.is_empty() { self.messages.push(msg); }
                self.state.mark_object_taken(world_idx);
                return true;
            }
            // FRUIT (ob_id 148): auto-eat if hungry, else store per SPEC §14.5
            148 => {
                let ate = self.state.pickup_fruit();
                if ate {
                    self.dlog(format!("ate fruit, hunger now {}", self.state.hunger));
                } else {
                    let msg = crate::game::events::event_msg(&self.narr, 36, bname);
                    if !msg.is_empty() { self.messages.push(msg); }
                }
                self.state.mark_object_taken(world_idx);
                return true;
            }
            // BROTHER'S BONES (ob_id 28): merge dead brother's inventory + retire ghosts.
            // Port of pickup_brother_bones (fmain.c:3173-3178); see
            // reference/logic/brother-succession.md §pickup_brother_bones.
            28 => {
                // announce_treasure("his brother's bones.") → "{name} found his brother's bones."
                self.messages.push(format!("{} found his brother's bones.", bname));
                // Both ghost set-figures retire regardless of which bones were picked up
                // (ob_listg[3].ob_stat = 0, ob_listg[4].ob_stat = 0; fmain.c:3174).
                for ghost_idx in [3usize, 4usize] {
                    if let Some(g) = self.state.world_objects.get_mut(ghost_idx) {
                        if g.ob_id == 10 || g.ob_id == 11 {
                            g.ob_stat = 0;
                            g.visible = false;
                        }
                    }
                }
                // Merge 31 pre-gold slots: stuff[k] += julstuff[k] (x == 1) else philstuff[k].
                // `x` in ref == anim_list[nearest].vitality & 0x7f (1=Julian, 2=Phillip).
                // Port slot scheme: world_objects[1] = Julian's bones, [2] = Phillip's bones.
                // Gold slots (GOLDBASE=31 .. ARROWBASE=35) are intentionally not merged.
                let donor_snapshot: [u8; 36] = if world_idx == 1 {
                    self.state.julstuff
                } else {
                    self.state.philstuff
                };
                let stuff = self.state.stuff_mut();
                for k in 0..31usize {
                    stuff[k] = stuff[k].saturating_add(donor_snapshot[k]);
                }
                self.state.mark_object_taken(world_idx);
                return true;
            }
            // URN (14), CHEST (15), SACKS (16) — containers with random loot
            14 | 15 | 16 => {
                let container_name = match ob_id {
                    14 => "a brass urn",
                    15 => "a chest",
                    16 => "some sacks",
                    _ => "a container",
                };

                // rand4() determines loot: 0=nothing, 1=one item, 2=two items, 3=three of same
                // Original uses print/print_cont for multi-part messages on the HI bar.
                // We combine announce_container prefix with loot suffix into one push.
                let prefix = format!("{} found {} containing ", bname, container_name);
                let roll = (self.state.tick_counter & 3) as u8;
                match roll {
                    0 => {
                        self.messages.push_wrapped(format!("{}nothing.", prefix));
                    }
                    1 => {
                        // One random item from inv_list[rand8()+8]
                        let item_idx = ((self.state.tick_counter >> 2) & 7) as usize + 8;
                        let item_idx = if item_idx == 8 { 35usize } else { item_idx }; // 8→ARROWBASE(35)
                        if item_idx < 36 {
                            self.state.pickup_item(item_idx);
                        }
                        let name = if item_idx < 31 { stuff_index_name(item_idx) } else { "quiver of arrows" };
                        self.messages.push_wrapped(format!("{}a {}.", prefix, name));
                    }
                    2 => {
                        // Two different random items
                        // Special: first item i==8 → 100 gold (SPEC §14.10).
                        let raw1 = ((self.state.tick_counter >> 2) & 7) as usize + 8;
                        let (item1, gold_special) = if raw1 == 8 {
                            (34usize, true) // GOLDBASE+3 = inv_list[34] = "100 Gold Pieces"
                        } else {
                            (raw1, false)
                        };
                        if gold_special {
                            self.state.gold += 100;
                        }
                        let mut item2 = ((self.state.tick_counter >> 5) & 7) as usize + 8;
                        if item2 == raw1 { item2 = ((item2 + 1) & 7) + 8; }
                        let item2 = if item2 == 8 { 35 } else { item2 };
                        if !gold_special && item1 < 31 { self.state.pickup_item(item1); }
                        // inventory.md#take_command (fmain.c:3229): stuff[k] += 1
                        // unconditionally — including when k == 35 (ARROWBASE),
                        // which is folded to stuff[8] * 10 at the epilogue.
                        self.state.pickup_item(item2);
                        let n1 = if item1 < 31 { stuff_index_name(item1) } else if item1 == 34 { "100 Gold Pieces" } else { "quiver of arrows" };
                        let n2 = if item2 < 31 { stuff_index_name(item2) } else { "quiver of arrows" };
                        self.messages.push_wrapped(format!("{}{} and a {}.", prefix, n1, n2));
                    }
                    3 | _ => {
                        // Three of the same item
                        let item = ((self.state.tick_counter >> 2) & 7) as usize + 8;
                        if item == 8 {
                            // Special: 3 random keys
                            self.messages.push_wrapped(format!("{}3 keys.", prefix));
                            for shift in [4, 7, 10] {
                                let mut key_idx = ((self.state.tick_counter >> shift) & 7) as usize + 16; // KEYBASE
                                if key_idx == 22 { key_idx = 16; }
                                if key_idx == 23 { key_idx = 20; }
                                self.state.pickup_item(key_idx);
                            }
                        } else {
                            let name = if item < 31 { stuff_index_name(item) } else { "quiver of arrows" };
                            self.messages.push_wrapped(format!("{}3 {}s.", prefix, name));
                            if item < 35 {
                                self.state.pickup_item(item);
                                self.state.pickup_item(item);
                                self.state.pickup_item(item);
                            }
                        }
                    }
                }

                // Original fmain2.c:1548-1551: chest → replace with open sprite (0x1d);
                // urn/sacks → set ob_stat = flag (hidden).
                if ob_id == 15 {
                    if let Some(obj) = self.state.world_objects.get_mut(world_idx) {
                        obj.ob_id = 0x1d; // open/empty chest sprite
                    }
                } else {
                    self.state.mark_object_taken(world_idx);
                }
                return true;
            }
            _ => {}
        }

        // Standard itrans pickup
        if let Some(stuff_idx) = ob_id_to_stuff_index(ob_id) {
            if self.state.pickup_item(stuff_idx) {
                // stuff[] indices >= 31 are gold/arrow accumulators — they
                // have no entry in NAMES. The only itrans target in that
                // range is QUIVER (ob_id 11 → stuff[35] = ARROWBASE), which
                // the container loot path also names "quiver of arrows".
                let name = if stuff_idx < 31 {
                    stuff_index_name(stuff_idx)
                } else {
                    "quiver of arrows"
                };
                // Original `announce_treasure(name)` composes "% found a <name>."
                // event_msg[37] is the apple-eaten message, NOT a generic pickup
                // string — using it here was a copy/paste error.
                self.messages.push(format!("{} found a {}.", bname, name));
                self.state.mark_object_taken(world_idx);
                return true;
            }
        }

        false
    }

    /// Port of `search_body` from `fmain.c:3251-3283` (referenced by
    /// `inventory.md#search_body`). Run when the TAKE dispatcher's
    /// `nearest_fig(0,30)` returns an NPC instead of a ground item.
    ///
    /// Lifecycle invariants (see `Npc::mark_dead`):
    /// - Live targets emit `event_msg[35]` "No time for that now!" and
    ///   leave `looted` untouched.
    /// - Dead+un-looted targets emit a composed scroll line and flip
    ///   `looted=true` regardless of whether anything dropped.
    /// - Dead+already-looted is a silent no-op (the original re-runs but
    ///   adds nothing because `weapon=0` and the second roll usually
    ///   returns nothing — we make it strictly silent for clarity).
    ///
    /// Strings come exclusively from the hardcoded literal registry in
    /// `reference/logic/dialog_system.md` (see "TAKE — body search
    /// composition") and `event_msg[35]`. No new prose.
    pub(crate) fn search_body(&mut self, npc_idx: usize, bname: &str) {
        use crate::game::npc::NpcState;
        use crate::game::world_objects::stuff_index_name;
        use crate::game::loot::{GOLDBASE};

        // Snapshot what we need from the body without holding the borrow
        // across mutations to `self.state`.
        let (npc_active, npc_state, npc_vitality, npc_race, npc_weapon, npc_looted, npc_x, npc_y) = {
            let table = match self.npc_table.as_ref() {
                Some(t) => t,
                None => return,
            };
            let n = match table.npcs.get(npc_idx) {
                Some(n) => n,
                None => return,
            };
            (n.active, n.state.clone(), n.vitality, n.race, n.weapon, n.looted, n.x, n.y)
        };
        if !npc_active { return; }

        // Alive guard. fmain.c:3252 checks `vitality && !frzcount`; this
        // port's freeze model is global (`state.freeze_timer`) rather than
        // per-actor, so a frozen-alive NPC is searchable while
        // freeze_timer > 0 — exactly the original semantics translated to
        // the global freeze (`SPEC §19.3`).
        if npc_state != NpcState::Dead && npc_vitality != 0 && self.state.freeze_timer == 0 {
            let msg = crate::game::events::event_msg(&self.narr, 35, bname);
            if !msg.is_empty() {
                self.messages.push(msg);
            }
            return;
        }

        // Already-looted body: silent no-op. Original re-prompts a search
        // but with `weapon=0` it produces only "found nothing." which we
        // suppress to keep TAKE on a previously-looted body quiet (matches
        // typical play experience; see SPEC §14.x note on loot idempotence).
        if npc_looted { return; }

        // --- Composed scroll line (fmain.c:3253-3283) ---
        // Prefix: "% searched the body and found"
        let mut line = format!("{} searched the body and found", bname);

        // Weapon phase. NPC `weapon` field encodes 1=Dirk, 2=Mace,
        // 3=Sword, 4=Bow, 5=Magic Wand. stuff[] slots are weapon-1.
        let mut got_weapon = false;
        if (1..=5).contains(&npc_weapon) {
            let slot = (npc_weapon - 1) as usize;
            self.state.stuff_mut()[slot] = self.state.stuff()[slot].saturating_add(1);
            let name = stuff_index_name(slot);
            line.push_str(&format!(" a {}", name));
            got_weapon = true;

            // Auto-equip if strictly better than current. fmain.c:3261:
            //   if(an->weapon > anim_list[0].weapon) anim_list[0].weapon = an->weapon
            let cur = self.state.actors.first().map(|a| a.weapon).unwrap_or(0);
            if npc_weapon > cur {
                if let Some(player) = self.state.actors.first_mut() {
                    player.weapon = npc_weapon;
                }
            }
        }

        // Bow short-circuit (fmain.c:3263-3268): if weapon was a bow,
        // grant rand8()+2 quivers via the ARROWBASE accumulator (stuff[35])
        // and end the scroll line WITHOUT rolling treasure. The TAKE
        // epilogue folds stuff[35] into stuff[8]*10 just like for chests.
        if npc_weapon == 4 {
            // Tick-seeded rand 0-7 (matches the seeding used by
            // loot.rs::rand8_from_tick — keep determinism while no
            // PRNG is plumbed yet).
            let h = (self.state.tick_counter ^ npc_x as u32 ^ npc_y as u32)
                .wrapping_mul(2246822519).wrapping_add(3266489917);
            let n = ((h as usize) & 7) + 2;
            self.state.stuff_mut()[35] =
                self.state.stuff()[35].saturating_add(n as u8);
            line.push_str(&format!(" and {} Arrows.", n));
            self.messages.push_wrapped(line);
            self.mark_npc_looted(npc_idx);
            return;
        }

        // Treasure phase — skipped entirely if `race & 0x80` (setfig
        // body, fmain.c:3270 `if(j & 0x80) j = 0`). Mirrors `roll_treasure`'s
        // setfig guard but inlined here so we can credit `wealth` (i16)
        // for gold, per `fmain.c:3279 wealth += GOLD_AMOUNTS[j-GOLDBASE]`.
        let mut got_treasure = false;
        if npc_race < 0x80 {
            // Reuse the existing treasure roller — it returns the same
            // inv_idx-derived (LootDrop::Item, LootDrop::Gold) we need.
            // Build a temporary npc shape to feed it; we already have the
            // race and the call only reads `race`, `tick`.
            let temp_npc = crate::game::npc::Npc {
                race: npc_race,
                ..Default::default()
            };
            if let Some(drop) = crate::game::loot::roll_treasure(&temp_npc, self.state.tick_counter) {
                use crate::game::loot::LootDrop;
                match drop {
                    LootDrop::Item(slot) => {
                        // stuff[slot]++ for j < GOLDBASE (always true here
                        // because gold inv_idxs become LootDrop::Gold).
                        let slot_idx = slot.min(35);
                        self.state.stuff_mut()[slot_idx] =
                            self.state.stuff()[slot_idx].saturating_add(1);
                        let name = if slot < 31 {
                            stuff_index_name(slot)
                        } else {
                            "an unknown thing"
                        };
                        let connector = if got_weapon { " and a " } else { " a " };
                        line.push_str(&format!("{}{}", connector, name));
                        got_treasure = true;
                        let _ = GOLDBASE;
                    }
                    LootDrop::Gold(amount) => {
                        // Body-search credits `wealth` (i16), per
                        // fmain.c:3279. We do NOT touch state.gold here —
                        // the gold/wealth divergence in award_treasure is
                        // out of scope for F9.11.
                        self.state.wealth = self.state.wealth.saturating_add(amount as i16);
                        // The original's gold path likewise extends the
                        // scroll with " and " + count + " Gold Pieces" —
                        // but the documented dialog_system.md registry
                        // notes only the non-gold treasure entry (j <
                        // GOLDBASE). Gold credit is silent in the scroll
                        // here; the wealth bar update is the visible
                        // feedback. (See SPEC §23.6 — wealth lines are
                        // not in the hardcoded-literal allowlist.)
                        got_treasure = true;
                    }
                }
            }
        }

        // No weapon, no treasure → "nothing".
        if !got_weapon && !got_treasure {
            line.push_str(" nothing");
        }
        // Close with a period (fmain.c:3283 `print_cont(".");`).
        line.push('.');
        self.messages.push_wrapped(line);

        self.mark_npc_looted(npc_idx);
    }

    /// Flip the NPC's `looted` flag so subsequent TAKE attempts are no-ops.
    pub(crate) fn mark_npc_looted(&mut self, npc_idx: usize) {
        if let Some(ref mut table) = self.npc_table {
            if let Some(n) = table.npcs.get_mut(npc_idx) {
                n.looted = true;
            }
        }
    }
}
