//! Menu action dispatch and option execution.
//! See `docs/spec/ui-menus.md` for specification.

use super::*;

impl GameplayScene {
    ///
    /// Dispatch a MenuAction returned by MenuState::handle_key / handle_click.
    pub(crate) fn dispatch_menu_action(&mut self, action: crate::game::menu::MenuAction) {
        use crate::game::menu::MenuAction;
        match action {
            MenuAction::Inventory    => self.do_option(GameAction::Inventory),
            // EXPLOIT GUARD: original bug allows repeated Take while paused (T key).
            // handle_key() already blocks non-Space keys when paused, but verify any
            // direct GameAction::Take path (key_bindings) also checks paused state.
            MenuAction::Take         => self.do_option(GameAction::Take),
            MenuAction::Look         => self.do_option(GameAction::Look),
            MenuAction::Yell         => self.do_option(GameAction::Yell),
            MenuAction::Say          => self.do_option(GameAction::Speak),
            MenuAction::Ask          => self.do_option(GameAction::Ask),
            MenuAction::CastSpell(n) => {
                let a = match n {
                    0 => GameAction::CastSpell1,
                    1 => GameAction::CastSpell2,
                    2 => GameAction::CastSpell3,
                    3 => GameAction::CastSpell4,
                    4 => GameAction::CastSpell5,
                    5 => GameAction::CastSpell6,
                    _ => GameAction::CastSpell7,
                };
                self.do_option(a);
            }
            MenuAction::BuyItem(n) => {
                let a = match n {
                    0 => GameAction::BuyFood,
                    1 => GameAction::BuyArrow,
                    2 => GameAction::BuyVial,
                    3 => GameAction::BuyMace,
                    4 => GameAction::BuySword,
                    5 => GameAction::BuyBow,
                    _ => GameAction::BuyTotem,
                };
                self.do_option(a);
            }
            MenuAction::SetWeapon(slot) => {
                use crate::game::menu::MenuMode;
                // inventory.md#use_dispatch (fmain.c:3449-3455): hitgo gates
                // the equip — when the player owns the weapon (`stuff[slot] > 0`),
                // `anim_list[0].weapon = hit + 1` runs silently. When unowned,
                // `extract("% doesn't have one.")` fires and the equip is
                // skipped. The hardcoded literal is enumerated in
                // dialog_system.md "Hardcoded scroll messages — complete
                // reference" (fmain.c:3451).
                let owned = (slot as usize) < 5
                    && self.state.stuff()[slot as usize] > 0;
                if owned {
                    if let Some(player) = self.state.actors.first_mut() {
                        player.weapon = slot + 1;
                    }
                } else {
                    let bname = self.brother_name().to_string();
                    self.messages.push(format!("{} doesn't have one.", bname));
                }
                self.menu.gomenu(MenuMode::Items);
            }
            MenuAction::TryKey(idx) => {
                use crate::game::menu::MenuMode;
                use crate::game::doors::{doorfind_nearest_by_bump_radius, key_req, KeyReq,
                                         apply_door_tile_replacement};
                use crate::game::world_objects::stuff_index_name;
                // idx: 0=GOLD, 1=GREEN, 2=KBLUE, 3=RED, 4=GREY, 5=WHITE → stuff[16+idx]
                let key_slot_stuff = 16 + idx as usize;
                // fmain.c:3474 — clear "It's locked." suppression latch so a follow-up bump
                // will re-speak the lock message.
                self.bumped_door = None;
                // fmain.c:3475 — silent return to the items menu when the player has zero of
                // this key. Mirrors `if (stuff[hit+KEYBASE]==0) goto menu0;` with no message.
                if self.state.stuff()[key_slot_stuff] != 0 {
                    // fmain.c:3477-3481 — sweep 9 directions (8 compass + self) at 16 px and
                    // try doorfind on each. The Rust port lacks per-tile open_list lookup;
                    // the bump-radius search is the architectural equivalent (see F6.4).
                    let region = self.state.region_num;
                    let nearest = doorfind_nearest_by_bump_radius(
                        &self.doors, region, self.state.hero_x, self.state.hero_y);
                    let opened = if let Some((door_idx, door)) = nearest {
                        let req = key_req(door.door_type);
                        let key_matches = matches!(req, KeyReq::Key(slot) if slot as usize == idx as usize);
                        if key_matches {
                            // fmain.c:3480 — key consumed only on successful match.
                            self.state.stuff_mut()[key_slot_stuff] -= 1;
                            if let Some(ref mut world) = self.map_world {
                                apply_door_tile_replacement(
                                    world, door.door_type,
                                    self.state.hero_x as i32, self.state.hero_y as i32,
                                );
                            }
                            self.messages.push("It opened.".to_string());
                            self.opened_doors.insert(door_idx);
                            self.dlog(format!("door: key {} opened door idx={}", idx, door_idx));
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if !opened {
                        // fmain.c:3483-3485 — "% tried a <keyname> but it didn't fit."
                        // Assembled from extract("% tried a ") + inv_list[KEYBASE+hit].name
                        // + " but it didn't" + print("fit.") fragments.
                        let bname = self.brother_name().to_string();
                        let kname = stuff_index_name(key_slot_stuff);
                        self.messages.push(format!(
                            "{} tried a {} but it didn't fit.", bname, kname,
                        ));
                    }
                }
                self.menu.gomenu(MenuMode::Items);
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            MenuAction::GiveGold => {
                // Port of give_item_to_npc hit==5 branch (fmain.c:3493-3500 /
                // reference/logic/quests.md#give_item_to_npc). Requires a
                // nearby actor cached by the last TALK/GIVE proximity scan;
                // here we fall back to `nearest_fig(1, 50)`. When `wealth > 2`
                // spend 2 gold, probabilistically bump kindness, and speak the
                // correct line based on target race (beggar 0x8d → speak(24+goal),
                // others → speak(50)). If `wealth <= 2` or no target is in
                // range, the original function silently returns — no
                // scroll-text is emitted.
                use crate::game::menu::MenuMode;
                use crate::game::npc::RACE_BEGGAR;
                let bname = self.brother_name().to_string();
                if let Some(fig) = self.nearest_fig(1, 50) {
                    if self.state.wealth > 2 {
                        self.state.wealth -= 2;
                        // kind++ chance: mirrors `if (rand64() > kind) kind++;`
                        // (fmain.c:3496). Use the same tick-driven hash used
                        // elsewhere in the port for rand64().
                        let roll = {
                            let tick = self.state.tick_counter;
                            let h = tick.wrapping_mul(1664525).wrapping_add(1013904223);
                            ((h >> 10) & 63) as i16
                        };
                        if roll > self.state.kind && self.state.kind < i16::MAX {
                            self.state.kind += 1;
                        }
                        // Dispatch on target race / setfig_type.
                        let (is_beggar, sf_goal) = match &fig.kind {
                            FigKind::SetFig { world_idx, setfig_type } => {
                                let goal = self.state.world_objects
                                    .get(*world_idx)
                                    .map_or(0u8, |o| o.goal) as usize;
                                (*setfig_type == 13, goal)
                            }
                            FigKind::Npc(idx) => {
                                let race = self.npc_table.as_ref()
                                    .and_then(|t| t.npcs.get(*idx))
                                    .map_or(0u8, |n| n.race);
                                (race == RACE_BEGGAR, 0usize)
                            }
                            FigKind::Item { .. } => (false, 0usize),
                        };
                        if is_beggar {
                            // speak(24 + goal): goal==3 overflows to speak(27)
                            // per the original bug (reference/logic/quests.md).
                            self.messages.push(crate::game::events::speak(&self.narr, 24 + sf_goal, &bname));
                        } else {
                            self.messages.push(crate::game::events::speak(&self.narr, 50, &bname));
                        }
                    }
                }
                self.menu.gomenu(MenuMode::Items);
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            MenuAction::GiveWrit => {
                // Per reference/logic/quests.md#give_item_to_npc, GIVE slot 7
                // (Writ) is a dead slot in `give_item_to_npc`: the function
                // has no `hit == 7` branch, so selecting it is a silent
                // no-op that simply falls through to `gomenu(CMODE_ITEMS)`.
                // (Writ is consumed only via the passive priest-TALK check at
                // fmain.c:3383-3388, not through GIVE.)
                use crate::game::menu::MenuMode;
                self.menu.gomenu(MenuMode::Items);
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            MenuAction::GiveBone => {
                // Port of give_item_to_npc hit==8 branch (fmain.c:3501-3506 /
                // reference/logic/quests.md#give_item_to_npc). Requires a
                // nearby actor and stuff[29] (Bone) nonzero. For non-spectre
                // targets speak(21) "no use for it" with no consumption; for
                // the Spectre (race 0x8a / setfig_type 10) speak(48), consume
                // the bone, and drop a Crystal Shard (ob_id 140) at the
                // spectre's feet via leave_item semantics (y + 10).
                use crate::game::menu::MenuMode;
                use crate::game::npc::RACE_SPECTRE;
                let bname = self.brother_name().to_string();
                if self.state.stuff()[29] != 0 {
                    if let Some(fig) = self.nearest_fig(1, 50) {
                        let (is_spectre, drop_pos) = match &fig.kind {
                            FigKind::SetFig { world_idx, setfig_type } => {
                                let pos = self.state.world_objects
                                    .get(*world_idx)
                                    .map(|o| (o.x, o.y));
                                (*setfig_type == 10, pos)
                            }
                            FigKind::Npc(idx) => {
                                let info = self.npc_table.as_ref()
                                    .and_then(|t| t.npcs.get(*idx))
                                    .map(|n| (n.race, n.x as u16, n.y as u16));
                                match info {
                                    Some((r, x, y)) => (r == RACE_SPECTRE, Some((x, y))),
                                    None => (false, None),
                                }
                            }
                            FigKind::Item { .. } => (false, None),
                        };
                        if is_spectre {
                            self.messages.push(crate::game::events::speak(&self.narr, 48, &bname));
                            self.state.stuff_mut()[29] = 0;
                            if let Some((sx, sy)) = drop_pos {
                                use crate::game::game_state::WorldObject;
                                let drop_y = (sy as i32 + 10).clamp(0, u16::MAX as i32) as u16;
                                self.state.world_objects.push(WorldObject {
                                    ob_id: 140, // Crystal Shard
                                    ob_stat: 1,
                                    region: self.state.region_num,
                                    x: sx,
                                    y: drop_y,
                                    visible: true,
                                    goal: 0,
                                });
                            }
                        } else {
                            self.messages.push(crate::game::events::speak(&self.narr, 21, &bname));
                        }
                    }
                }
                self.menu.gomenu(MenuMode::Items);
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            MenuAction::SaveGame(slot) => {
                match crate::game::persist::save_game(&self.state, slot) {
                    Ok(()) => {
                        if let Err(e) = crate::game::persist::save_transcript(
                            self.messages.transcript(), slot,
                        ) {
                            eprintln!("save transcript failed: {e}");
                        }
                        // Original emits no scroll text on save success (fmain2.c:1531 guard).
                    }
                    Err(e) => {
                        eprintln!("save failed: {e}");
                        // fmain2.c:1532 — "ERROR: Couldn't save game." (dialog_system.md §save/load)
                        self.messages.push("ERROR: Couldn't save game.");
                    }
                }
            }
            MenuAction::LoadGame(slot) => {
                // EXPLOIT FIX NEEDED: reset all runtime door state before restoring
                // save, otherwise keys replenish but doors stay unlocked.
                match crate::game::persist::load_game(slot) {
                    Ok(new_state) => {
                        *self.state = new_state;
                        // Restore existing transcript so new messages are appended.
                        let existing = crate::game::persist::load_transcript(slot);
                        self.messages.set_transcript(existing);
                        // fmain2.c:1546 — three blank print("") calls clear the scroll area.
                        self.messages.push("");
                        self.messages.push("");
                        self.messages.push("");
                        // Post-load: rebuild menu states from inventory (SPEC §24.5)
                        let wealth = self.state.wealth;
                        self.menu.set_options(self.state.stuff(), wealth);

                        // Post-load runtime state reset (fmain2.c:1541-1548):
                        // Force on_region_changed() on the next tick regardless of whether the
                        // loaded region matches the current one — this reloads world data, NPC
                        // tables, and recalculates the map renderer for the restored position.
                        self.last_region_num = u8::MAX;
                        // Snap camera to the loaded hero position (map_x/map_y are local
                        // to GameplayScene and would otherwise keep the pre-load viewport).
                        self.snap_camera_to_hero();
                        // Clear transient game-loop flags that live outside GameState.
                        self.sleeping = false;
                        self.paused = false;
                        // Reset door state: keys replenish on load so doors must start locked.
                        self.opened_doors.clear();
                        self.bumped_door = None;
                        self.last_person = None;
                        // Clear any in-flight visual effects and missiles.
                        self.witch_effect = WitchEffect::new();
                        self.teleport_effect = TeleportEffect::new();
                        self.missiles = std::array::from_fn(|_| crate::game::combat::Missile::default());
                        // Un-pause the MenuState if it was paused (gomenu(GAME) equivalent,
                        // fmain.c:3471 — cmode is overwritten after savegame returns).
                        if self.menu.is_paused() {
                            self.menu.toggle_pause();
                        }
                        self.menu.gomenu(crate::game::menu::MenuMode::Items);
                    }
                    Err(e) => {
                        eprintln!("load failed: {e}");
                        // fmain2.c:1533 — "ERROR: Couldn't load game." (dialog_system.md §save/load)
                        self.messages.push("ERROR: Couldn't load game.");
                    }
                }
            }
            MenuAction::Quit     => self.do_option(GameAction::Quit),
            MenuAction::TogglePause => {
                // MenuState already toggled the bit; sync paused field.
                self.paused = self.menu.is_paused();
                if self.paused {
                    self.messages.push("Game paused. Press Space to continue.");
                }
            }
            MenuAction::ToggleMusic => {
                let on = self.menu.is_music_on();
                self.messages.push(if on { "Music on." } else { "Music off." });
                self.last_mood = u8::MAX; // force re-evaluation next tick
                self.pending_music_toggle = Some(on);
            }
            MenuAction::ToggleSound => {
                let on = self.menu.is_sound_on();
                self.messages.push(if on { "Sound on." } else { "Sound off." });
                self.pending_sound_toggle = Some(on);
            }
            MenuAction::RefreshMusic  => {}
            MenuAction::SummonTurtle  => self.do_option(GameAction::SummonTurtle),
            MenuAction::UseSunstone   => self.do_option(GameAction::UseSpecial),
            MenuAction::SwitchMode(_) => {}
            MenuAction::UseMenu | MenuAction::GiveMenu => {}
            MenuAction::None          => {}
        }
    }

    /// Dispatch a game menu/command action.
    pub(crate) fn do_option(&mut self, action: GameAction) {
        self.dlog(format!("do_option: {:?}", action));
        match action {
            // Shop BUY menu — seven-slot dispatch (fmain.c:3424-3442,
            // TABLE:jtrans at fmain2.c:850).  label5 = "Food Arrow Vial
            // Mace Sword Bow  Totem" (fmain.c:500); slots 0..=6 map to
            // BUY `hit` values 5..=11.  Per reference/logic/shops.md:
            //   0 Food  → eat(50) + event(22)
            //   1 Arrow → stuff[8] += 10 + event(23)
            //   2 Vial  → stuff[11]++
            //   3 Mace  → stuff[1]++
            //   4 Sword → stuff[2]++
            //   5 Bow   → stuff[3]++
            //   6 Totem → stuff[13]++
            GameAction::BuyFood  => { self.do_buy_slot(0); let w = self.state.wealth; self.menu.set_options(self.state.stuff(), w); }
            GameAction::BuyArrow => { self.do_buy_slot(1); let w = self.state.wealth; self.menu.set_options(self.state.stuff(), w); }
            GameAction::BuyVial  => { self.do_buy_slot(2); let w = self.state.wealth; self.menu.set_options(self.state.stuff(), w); }
            GameAction::BuyMace  => { self.do_buy_slot(3); let w = self.state.wealth; self.menu.set_options(self.state.stuff(), w); }
            GameAction::BuySword => { self.do_buy_slot(4); let w = self.state.wealth; self.menu.set_options(self.state.stuff(), w); }
            GameAction::BuyBow   => { self.do_buy_slot(5); let w = self.state.wealth; self.menu.set_options(self.state.stuff(), w); }
            GameAction::BuyTotem => { self.do_buy_slot(6); let w = self.state.wealth; self.menu.set_options(self.state.stuff(), w); }
            GameAction::Inventory => {
                self.dlog(format!("Inventory: {}", self.state.inventory_summary()));
                self.state.viewstatus = 4;
                self.dlog("inventory opened".to_string());
            }
            GameAction::Rebind => {
                self.rebinding.active = !self.rebinding.active;
                self.dlog(format!("Rebinding mode: {}", self.rebinding.active));
            }
            GameAction::Board => {
                // Ref `carrier-transport.md#raft_tick` (`fmain.c:1562-1573`):
                // raft boarding is automatic when the hero is within 9 px of
                // the raft on terrain 3-5. There is no explicit BOARD command
                // in the original; this action is a debug convenience. The
                // original emits no scroll-area text on board/leave — any
                // string here would violate the two-source rule (SPEC §23.6).
                let _ = self.state.board_raft();
            }
            GameAction::Sleep => {
                let at_door = crate::game::doors::doorfind(
                    &self.doors, self.state.region_num, self.state.hero_x, self.state.hero_y
                ).is_some();
                if at_door {
                    // Cannot sleep at a door (in original, silently ignored)
                } else {
                    self.state.fatigue = 0;
                    self.state.hunger = (self.state.hunger + 50)
                        .min(crate::game::game_state::MAX_HUNGER);
                    let bname = self.brother_name().to_string();
                    self.messages.push(crate::game::events::event_msg(&self.narr, 26, &bname));
                    self.dlog("Player slept: fatigue reset");
                }
            }
            GameAction::GetItem => {
                // fmain2.c:467 (prq case 10) — literal "Take What?" when
                // TAKE fires with nothing in range. See dialog_system.md:273.
                self.messages.push("Take What?");
                self.dlog("GetItem: no nearby item");
            }
            GameAction::DropItem => {
                // No DROP command exists in the original game — the ITEMS
                // submenu labels are "List Take Look Use  Give" (fmain.c:497,
                // inventory.md §Notes "No DROP command"). Silent no-op.
                self.dlog("DropItem: no-op (no DROP in original)");
            }
            GameAction::Talk => {
                // Talk is the same as Ask/Speak: range 50, nearest NPC (fmain.c:4167).
                self.do_option(GameAction::Speak);
            }
            GameAction::Attack => {
                // Legacy menu-driven attack path — real combat runs through
                // `run_combat_tick` via `input.fight`. This branch predates
                // the proximity/swing state machine and is retained only so
                // the menu action doesn't panic. Scroll-area strings here
                // were invented (not in `faery.toml [narr]` nor
                // `dialog_system.md`) and have been removed.
                //
                // F9.11: auto-loot removed from this path too — bodies must
                // be TAKEn via `search_body`. Turtle-egg shell rescue stays
                // because it is a quest hook, not treasure.
                if let Some(ref mut table) = self.npc_table {
                    for npc in table.npcs.iter_mut().filter(|n| n.active && n.state != crate::game::npc::NpcState::Dead) {
                        let dx = (npc.x - self.state.hero_x as i16).abs();
                        let dy = (npc.y - self.state.hero_y as i16).abs();
                        if dx < 32 && dy < 32 {
                            #[allow(deprecated)]
                            let result = crate::game::combat::resolve_combat(&mut self.state, npc, 0);
                            if result.enemy_defeated {
                                // Turtle egg rescue: killing a snake near eggs awards a Sea Shell (player-108).
                                if self.state.check_turtle_eggs(npc.race == crate::game::npc::RACE_SNAKE) {
                                    self.dlog("check_turtle_eggs: shell awarded for snake kill");
                                }
                                let wealth = self.state.wealth;
                                self.menu.set_options(self.state.stuff(), wealth);
                            }
                            break;
                        }
                    }
                }
            }
            GameAction::Fight => {
                self.input.fight = true;
            }
            GameAction::UseItem => {
                // Top-level USE entry — individual weapon / shell / sun-stone
                // slots dispatch via MenuAction::SetWeapon etc. Silent no-op
                // here matches `use_dispatch` fall-through for unhandled hits
                // (inventory.md §use_dispatch, fmain.c:3466 `gomenu(ITEMS)`).
                self.dlog("UseItem: no-op (slot-specific handlers dispatch via menu)");
            }
            // MAGIC menu items 5..=11 (stuff[9..=15], MAGICBASE=9 in fmain.c).
            GameAction::CastSpell1 => {
                self.try_cast_spell(ITEM_STONE_RING);
            }
            GameAction::CastSpell2 => {
                self.try_cast_spell(ITEM_LANTERN);
            }
            GameAction::CastSpell3 => {
                self.try_cast_spell(ITEM_VIAL);
            }
            GameAction::CastSpell4 => {
                self.try_cast_spell(ITEM_ORB);
            }
            GameAction::CastSpell5 => {
                self.try_cast_spell(ITEM_TOTEM);
            }
            GameAction::CastSpell6 => {
                self.try_cast_spell(ITEM_RING);
            }
            GameAction::CastSpell7 => {
                self.try_cast_spell(ITEM_SKULL);
            }
            GameAction::Shoot => {
                use crate::game::game_state::ITEM_ARROWS;
                let weapon = self.state.actors.first().map_or(4, |a| a.weapon);
                let is_bow = weapon == 4;

                if is_bow && self.state.stuff()[ITEM_ARROWS] == 0 {
                    self.messages.push("No Arrows!");
                } else {
                    use crate::game::combat::fire_missile;
                    fire_missile(
                        &mut self.missiles,
                        self.state.hero_x as i32,
                        self.state.hero_y as i32,
                        self.state.facing,
                        weapon,
                        true,
                        2, // Standard hero projectile speed
                    );
                    if is_bow {
                        self.state.stuff_mut()[ITEM_ARROWS] -= 1;
                    }
                    // No scroll-area message: original fmain.c emits no text on
                    // arrow/fireball fire. Ref: combat.md#missile_step.
                    let wealth = self.state.wealth;
                    self.menu.set_options(self.state.stuff(), wealth);
                }
            }
            GameAction::SummonTurtle => {
                // Ref `carrier-transport.md#use_sea_shell` (`fmain.c:3457-3461`):
                // swamp-box veto is silently inert ("'break' out of the case
                // without calling get_turtle"); a successful summon also emits
                // no event text. Any scroll-area string here violates the
                // two-source rule (SPEC §23.6). Inventory-empty is already
                // filtered upstream by the menu `hitgo` gate, but preserve
                // the silent no-op path here for safety.
                if !self.state.is_turtle_summon_blocked() {
                    let _ = self.state.summon_turtle();
                }
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            GameAction::Look => {
                // SPEC §14.19 / inventory.md#look_command (fmain.c:3286-3295):
                // Scan OBJECTS within range 40. Only *hidden* objects (race==0,
                // equivalent to ob_stat==5) count toward the "found" latch and
                // are promoted to ob_stat=1 via change_object(i, 1).
                // Feedback: event(38) if any reveal this tick, else event(20).
                use crate::game::collision::calc_dist;
                const LOOK_RANGE: i32 = 40;
                let hx = self.state.hero_x as i32;
                let hy = self.state.hero_y as i32;
                let region = self.state.region_num;
                let mut flag = false;
                for obj in self.state.world_objects.iter_mut() {
                    if obj.region != region { continue; }
                    if obj.ob_stat == 3 { continue; } // setfigs are not OBJECTS
                    if obj.ob_id == 0x1d { continue; } // empty chest
                    if calc_dist(hx, hy, obj.x as i32, obj.y as i32) >= LOOK_RANGE {
                        continue;
                    }
                    if obj.ob_stat == 5 {
                        // race==0 case only: promote hidden → visible and latch.
                        obj.ob_stat = 1;
                        obj.visible = true;
                        flag = true;
                    }
                }
                let bname = self.brother_name().to_string();
                let idx = if flag { 38 } else { 20 };
                let msg = crate::game::events::event_msg(&self.narr, idx, &bname);
                if !msg.is_empty() {
                    self.messages.push(msg);
                }
            }
            GameAction::Take => {
                // F9.11 — TAKE dispatch (`fmain.c:3147-3287`,
                // `inventory.md#take_command`). Use the unified `nearest_fig`
                // search at constraint=0 (takeables: ground items + dead
                // bodies) and dispatch on what was found:
                //   - FigKind::Item  → handle_take_item (existing object pickup)
                //   - FigKind::Npc   → search_body (loot from corpse)
                //   - FigKind::SetFig → no-op (setfigs aren't TAKE targets)
                //   - None           → "Take What?" literal
                const TAKE_RANGE: i32 = 30; // fmain2.c nearest_fig(0, 30) — see SPECIFICATION.md §menu-table
                // inventory.md#take_command (fmain.c:3151): stuff[35] (ARROWBASE)
                // is the per-TAKE quiver accumulator; it must be cleared on
                // entry, then folded into stuff[8] * 10 at the epilogue.
                self.state.stuff_mut()[35] = 0;
                let bname = self.brother_name().to_string();
                let mut taken = false;
                match self.nearest_fig(0, TAKE_RANGE) {
                    Some(NearestFig { kind: FigKind::Item { world_idx, ob_id }, .. }) => {
                        taken = self.handle_take_item(world_idx, ob_id, &bname);
                    }
                    Some(NearestFig { kind: FigKind::Npc(npc_idx), .. }) => {
                        // search_body always "consumes" the action even if
                        // the body had no weapon and no treasure — the scroll
                        // line "% searched the body and found nothing." is
                        // the original's silent ack, and we must still run
                        // the epilogue (refresh menu, no quiver fold-back).
                        self.search_body(npc_idx, &bname);
                        taken = true;
                    }
                    Some(NearestFig { kind: FigKind::SetFig { .. }, .. }) => {
                        // Setfigs are TALK targets, not TAKE — original
                        // `take_command` falls through with no message
                        // (`fmain.c:3155 if(an->type != OBJECTS) goto sb`,
                        // and setfigs would still match anim_list != OBJECTS,
                        // but body-search gates on `vitality==0` first and
                        // setfigs are alive → silent fallthrough).
                    }
                    None => {
                        // fmain2.c:467 (prq case 10) — "Take What?" literal.
                        // See dialog_system.md hardcoded-scroll registry.
                        self.messages.push("Take What?");
                    }
                }
                if taken {
                    // Epilogue fold-back (fmain.c:3250):
                    //   stuff[8] = stuff[8] + stuff[35] * 10
                    let quivers = self.state.stuff()[35] as u16;
                    if quivers > 0 {
                        let arrows = (self.state.stuff()[8] as u16)
                            .saturating_add(quivers.saturating_mul(10));
                        self.state.stuff_mut()[8] = arrows.min(255) as u8;
                        self.state.stuff_mut()[35] = 0;
                    }
                    let wealth = self.state.wealth;
                    self.menu.set_options(self.state.stuff(), wealth);
                    // Win condition — fmain.c:3244-3247:
                    //   if (stuff[22]) { quitflag = TRUE; viewstatus = 2;
                    //                    map_message(); SetFont(rp,afont); win_colors(); }
                    // Talisman pickup sets stuff[22]; game exits via VictoryScene.
                    if self.state.stuff()[22] != 0 && !self.victory_triggered {
                        self.state.quitflag = true;
                        self.state.viewstatus = 2;
                        self.victory_triggered = true;
                    }
                }
            }
            GameAction::Give => {
                // Give 2 gold to a nearby beggar setfig (ob_id=13, ob_stat=3), raising kindness.
                // T2-NPC-BEGGAR-GOAL: beggar speaks speak(24 + goal) on receipt (SPEC §13.5).
                // Overflow bug at goal==3 → speak(27) is preserved naturally (24+3=27).
                let bname = self.brother_name().to_string();
                let hero_x = self.state.hero_x as i16;
                let hero_y = self.state.hero_y as i16;
                let beggar_world_idx = self.state.world_objects.iter().enumerate().find(|(_, o)| {
                    o.ob_stat == 3 && o.ob_id == 13 && o.visible
                        && o.region == self.state.region_num
                        && ((o.x as i16 - hero_x).abs() < 50)
                        && ((o.y as i16 - hero_y).abs() < 50)
                }).map(|(i, _)| i);
                let near_beggar = beggar_world_idx.is_some();
                if near_beggar && self.state.wealth > 2 {
                    self.state.wealth -= 2;
                    // kind++ chance (mirrors: if rand64() > kind { kind++; })
                    if self.state.kind < 100 {
                        self.state.kind += 1;
                    }
                    let goal = beggar_world_idx
                        .and_then(|i| self.state.world_objects.get(i))
                        .map_or(0usize, |o| o.goal as usize);
                    // speak(24 + goal): goal==3 overflows to speak(27) per original bug.
                    self.messages.push(crate::game::events::speak(&self.narr, 24 + goal, &bname));
                    self.dlog(format!("give to beggar goal={}: wealth={}, kind={}", goal, self.state.wealth, self.state.kind));
                } else if near_beggar {
                    // No gold to spare (silently ignored in original)
                } else {
                    // Nothing to give to (silently ignored in original)
                }
            }
            GameAction::Yell => {
                // Yell: nearest_fig(1, 100). If NPC within 35 → speak(8) "No need to shout!"
                // Otherwise dispatch to the normal TALK switch (fmain.c:3367-3423).
                // If no target is in yell range, the original handler silently
                // returns (`fmain.c:3369`); no brother-name shout is emitted.
                let bname = self.brother_name().to_string();
                if let Some(fig) = self.nearest_fig(1, 100) {
                    if fig.dist < 35 {
                        self.messages.push(crate::game::events::speak(&self.narr, 8, &bname));
                    } else {
                        self.handle_setfig_talk(&fig, &bname);
                    }
                }
            }
            GameAction::Speak | GameAction::Ask => {
                // Talk: nearest_fig(1, 50).  Per reference/logic/shops.md
                // the bartender has no separate "What do you need?" menu:
                // TALK resolves through `handle_setfig_talk` case 8
                // (`fmain.c:3406-3408`, `bartender_speech`) which selects
                // `speak(13/12/14)` from fatigue / dayperiod.  The BUY
                // menu itself is the commercial surface.  Fallback: turtle
                // carrier shell dialogue (SPEC §13.7).
                let bname = self.brother_name().to_string();
                if let Some(fig) = self.nearest_fig(1, 50) {
                    self.handle_setfig_talk(&fig, &bname);
                } else if self.state.active_carrier == crate::game::game_state::CARRIER_TURTLE {
                    // T2-NPC-TURTLE-DIALOG: turtle carrier shell dialogue (SPEC §13.7).
                    // No shell → speak(56) "Thank you for saving my eggs!" and award shell.
                    // Has shell → speak(57) "Hop on my back for a ride".
                    let speech = if self.state.stuff()[crate::game::game_state::ITEM_SHELL] == 0 {
                        self.state.stuff_mut()[crate::game::game_state::ITEM_SHELL] = 1;
                        56
                    } else {
                        57
                    };
                    self.messages.push(crate::game::events::speak(&self.narr, speech, &bname));
                }
                // Else: no target within 50 px and no turtle carrier — the
                // original `talk_dispatch` silently returns (`fmain.c:3369`).
            }
            GameAction::Quit => {
                self.quit_requested = true;
            }
            GameAction::Pause => {
                self.menu.toggle_pause();
                self.paused = self.menu.is_paused();
                if self.paused {
                    self.messages.push("Game paused. Press Space to continue.");
                }
            }
            GameAction::ToggleMenuMode => {
                self.toggle_menu_mode();
            }
            GameAction::MenuUp => {
                self.menu_cursor.navigate_up();
            }
            GameAction::MenuDown => {
                self.menu_cursor.navigate_down();
            }
            GameAction::MenuLeft => {
                self.menu_cursor.navigate_left();
            }
            GameAction::MenuRight => {
                self.menu_cursor.navigate_right();
            }
            GameAction::MenuConfirm => {
                let slot = self.menu_cursor.slot();
                let action = self.menu.handle_click(slot);
                self.dispatch_menu_action(action);
            }
            GameAction::MenuCancel => {
                self.menu_cursor.active = false;
                self.controller_mode = ControllerMode::Gameplay;
            }
            GameAction::UseCrystalVial => {
                self.do_option(GameAction::CastSpell3); // ITEM_VIAL = stuff[11], spell slot 3
            }
            GameAction::UseOrb => {
                self.do_option(GameAction::CastSpell4); // ITEM_ORB = stuff[12], spell slot 4
            }
            GameAction::UseTotem => {
                self.do_option(GameAction::CastSpell5); // ITEM_TOTEM = stuff[13], spell slot 5
            }
            GameAction::UseSkull => {
                self.do_option(GameAction::CastSpell7); // ITEM_SKULL = stuff[15], spell slot 7
            }
            GameAction::WeaponPrev => {
                let current_weapon = self.state.actors.first()
                    .map(|a| a.weapon).unwrap_or(1);
                if let Some(new_weapon) = cycle_weapon_slot(current_weapon, -1, self.state.stuff()) {
                    if let Some(player) = self.state.actors.first_mut() {
                        player.weapon = new_weapon;
                    }
                    // inventory.md#use_dispatch: equip is silent; no scroll text.
                    let wealth = self.state.wealth;
                    self.menu.set_options(self.state.stuff(), wealth);
                }
            }
            GameAction::WeaponNext => {
                let current_weapon = self.state.actors.first()
                    .map(|a| a.weapon).unwrap_or(1);
                if let Some(new_weapon) = cycle_weapon_slot(current_weapon, 1, self.state.stuff()) {
                    if let Some(player) = self.state.actors.first_mut() {
                        player.weapon = new_weapon;
                    }
                    // inventory.md#use_dispatch: equip is silent; no scroll text.
                    let wealth = self.state.wealth;
                    self.menu.set_options(self.state.stuff(), wealth);
                }
            }
            _ => {}
        }
        let wealth = self.state.wealth;
        self.menu.set_options(self.state.stuff(), wealth);
    }

    pub(crate) fn toggle_menu_mode(&mut self) {
        self.menu_cursor.active = !self.menu_cursor.active;
        self.controller_mode = if self.menu_cursor.active {
            ControllerMode::Menu
        } else {
            ControllerMode::Gameplay
        };
    }
}
