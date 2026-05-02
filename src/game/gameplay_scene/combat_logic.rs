//! Combat tick processing and hit application.
//! See `docs/spec/combat.md` for specification.

use super::*;

impl GameplayScene {
    pub(crate) fn run_combat_tick(&mut self) {
        use crate::game::combat::{weapon_tip, combat_reach, rand256, bitrand};
        use crate::game::actor::ActorState;
        use crate::game::debug_command::GodModeFlags;

        let freeze = self.state.freeze_timer > 0;
        let brave = self.state.brave;
        let tick = self.state.cycle;
        let one_hit_kill = self.state.god_mode.contains(GodModeFlags::ONE_HIT_KILL);
        let insane_reach = self.state.god_mode.contains(GodModeFlags::INSANE_REACH);
        let anix = self.state.anix;

        struct Combatant {
            x: i32,
            y: i32,
            facing: u8,
            weapon: u8,
            fighting: bool,
            active: bool,
        }
        let mut combatants: Vec<Combatant> = Vec::with_capacity(anix);
        for (i, actor) in self.state.actors.iter().take(anix).enumerate() {
            let fighting = matches!(actor.state, ActorState::Fighting(_));
            combatants.push(Combatant {
                x: actor.abs_x as i32,
                y: actor.abs_y as i32,
                facing: actor.facing,
                weapon: if i == 0 { actor.weapon.max(1) } else { actor.weapon },
                fighting,
                active: !matches!(actor.state, ActorState::Dead | ActorState::Dying),
            });
        }

        struct HitRecord {
            attacker: usize,
            target: usize,
            facing: u8,
            damage: i16,
        }
        let mut hits: Vec<HitRecord> = Vec::new();

        for (i, attacker) in combatants.iter().enumerate() {
            if i == 1 { continue; } // skip raft slot
            if !attacker.active || !attacker.fighting { continue; }
            if i > 0 && freeze { continue; } // NPCs frozen

            let mut wt = attacker.weapon;
            if wt & 4 != 0 { continue; } // bow/wand — handled by shoot state machine
            if wt >= 8 { wt = 5; } // cap touch attack
            let wt_dmg = wt as i16 + bitrand(2) as i16;

            let reach = if insane_reach && i == 0 {
                combat_reach(true, brave, tick) * 4
            } else {
                combat_reach(i == 0, brave, tick)
            };

            let (tip_x, tip_y) = weapon_tip(attacker.x, attacker.y, attacker.facing, wt as i16);

            for (j, target) in combatants.iter().enumerate() {
                if j == 1 || j == i { continue; } // skip raft, self
                if !target.active { continue; }

                let xd = (target.x - tip_x).abs();
                let yd = (target.y - tip_y).abs();
                let dist = xd.max(yd);

                // Hit check: hero always hits, NPCs must pass brave dodge
                let hit_roll = i == 0 || rand256() > brave;
                if hit_roll && dist < reach as i32 {
                    let damage = if one_hit_kill && i == 0 {
                        999
                    } else {
                        wt_dmg
                    };
                    hits.push(HitRecord {
                        attacker: i,
                        target: j,
                        facing: attacker.facing,
                        damage,
                    });
                    break; // one hit per swing
                }
            }
        }

        // Apply hits
        for hit in hits {
            self.apply_hit(hit.attacker, hit.target, hit.facing, hit.damage);
        }
    }

    pub(crate) fn apply_hit(&mut self, attacker_idx: usize, target_idx: usize, facing: u8, damage: i16) {
        if target_idx == 0 {
            // NPC hitting hero
            self.state.vitality = (self.state.vitality - damage).max(0);
            self.dlog(format!("enemy hit hero for {}", damage));

            // Pushback: hero pushed 2px in attacker's facing direction
            let (px, py) = push_offset(facing, 2);
            self.state.hero_x = (self.state.hero_x as i32 + px).clamp(0, 32767) as u16;
            self.state.hero_y = (self.state.hero_y as i32 + py).clamp(0, 32767) as u16;

            // checkdead for hero
            if self.state.vitality <= 0 {
                if let Some(player) = self.state.actors.first_mut() {
                    player.state = crate::game::actor::ActorState::Dying;
                }
                self.death_type = 5; // combat death (SPEC §20.1)
                self.dlog("hero killed in combat".to_string());
                // luck -= 5 is applied uniformly at death-init in tick_goodfairy_countdown.
            }
        } else {
            // Hero (or NPC) hitting an NPC
            let attacker_weapon = if attacker_idx == 0 {
                self.state.actors.first().map_or(1, |a| a.weapon)
            } else {
                self.state.actors.get(attacker_idx).map_or(1, |a| a.weapon)
            };

            // Work inside the npc_table borrow, collect results to act on after.
            let mut logs: Vec<String> = Vec::new();
            let mut dead_npc: Option<crate::game::npc::Npc> = None;
            let mut immunity_msg: Option<String> = None;
            let mut dark_knight_speech: Option<String> = None;
            let mut necro_talisman_pos: Option<(i16, i16)> = None;
            let mut witch_lasso_pos: Option<(i16, i16)> = None;

            let bname = self.brother_name().to_string();
            if let Some(ref mut table) = self.npc_table {
                let npc_idx = target_idx.saturating_sub(2);
                if npc_idx < table.npcs.len() {
                    let npc = &mut table.npcs[npc_idx];

                    // Immunity guard per SPEC §10.2 / combat.md#dohit
                    // (fmain2.c:231-235). Immune targets take zero damage
                    // AND skip knockback, follow-through, and checkdead —
                    // the original `dohit` returns immediately before the
                    // damage/SFX/move_figure block.
                    use crate::game::combat::{check_immunity, ImmunityResult};
                    let has_sun_stone = self.state.stuff()[7] != 0;
                    let immunity = check_immunity(npc.race, attacker_weapon, has_sun_stone);

                    let immune = match immunity {
                        ImmunityResult::Vulnerable => false,
                        ImmunityResult::ImmuneSilent => true,
                        ImmunityResult::ImmuneWithMessage => {
                            immunity_msg = Some(crate::game::events::speak(&self.narr, 58, &bname));
                            true
                        }
                    };

                    if !immune {
                        npc.vitality -= damage;
                        if npc.vitality < 0 { npc.vitality = 0; }

                        // Pushback on target: 2px in attacker facing, but
                        // DRAGON and SETFIG races refuse to move (fmain2.c:243).
                        let target_pushable = npc.npc_type != crate::game::npc::NPC_TYPE_DRAGON
                            && (npc.race & 0x80) == 0;
                        let target_moved = if target_pushable {
                            let (px, py) = push_offset(facing, 2);
                            npc.x = (npc.x as i32 + px).clamp(0, 32767) as i16;
                            npc.y = (npc.y as i32 + py).clamp(0, 32767) as i16;
                            true
                        } else {
                            false
                        };

                        // Attacker follow-through: only if the target moved
                        // successfully AND attacker is a real melee source
                        // (i >= 0; arrows/fireballs use i = -1/-2 and don't
                        // recoil). apply_hit is never called for projectiles
                        // so the i>=0 guard is implicit here.
                        if target_moved && attacker_idx == 0 {
                            let (rx, ry) = push_offset(facing, 2);
                            self.state.hero_x = (self.state.hero_x as i32 + rx).clamp(0, 32767) as u16;
                            self.state.hero_y = (self.state.hero_y as i32 + ry).clamp(0, 32767) as u16;
                        }

                        if damage > 0 {
                            logs.push(format!("combat hit npc {} for {}", npc_idx, damage));
                        }
                    }

                    // checkdead — fmain.c:2769-2784 / combat.md#checkdead.
                    // Reference: vitality<1 triggers transition; brave+1 for
                    // any enemy (i!=0); kind-=3 for SETFIG non-witch kills;
                    // speak(42) on dark-knight race 7.
                    if npc.vitality == 0 {
                        use crate::game::npc::{RACE_NECROMANCER, RACE_WOODCUTTER, RACE_WITCH, NpcState};
                        const RACE_DARK_KNIGHT: u8 = 7; // fmain.c:2774

                        if npc.race == RACE_DARK_KNIGHT {
                            dark_knight_speech = Some(crate::game::events::speak(&self.narr, 42, &bname));
                        } else if (npc.race & 0x80) != 0 && npc.race != RACE_WITCH {
                            // SETFIG type (bit 7 set) non-witch: kindness penalty.
                            self.state.kind -= 3;
                        }

                        if npc.race == RACE_NECROMANCER {
                            // SPEC §15.7: transform in-place → Woodcutter; don't despawn.
                            necro_talisman_pos = Some((npc.x, npc.y));
                            npc.race = RACE_WOODCUTTER;
                            npc.vitality = 10;
                            npc.state = NpcState::Still;
                            npc.weapon = 0;
                            logs.push("necromancer slain: transformed to woodcutter, talisman drops".to_string());
                        } else {
                            // SPEC §14.20: Witch drops Golden Lasso on death.
                            if npc.race == RACE_WITCH {
                                witch_lasso_pos = Some((npc.x, npc.y));
                            }
                            // F9.11 — searchable body lifecycle. NPC stays in
                            // `npc_table` with `active=true`, `state=Dead`,
                            // `looted=false` so TAKE → search_body
                            // (`fmain.c:3251-3283`) can consume it. Slot reuse
                            // via `Npc::slot_free()` treats Dead as free.
                            npc.mark_dead();
                        }
                        // fmain.c:2777 — brave += 1 on any enemy kill.
                        // No cap in original; the .min(100) was invented.
                        self.state.brave += 1;
                        if self.state.kind < 0 { self.state.kind = 0; }
                        dead_npc = Some(npc.clone());
                        logs.push(format!("enemy slain, bravery now {}", self.state.brave));
                    }
                }
            }

            // Deferred work outside the npc_table borrow
            if let Some(msg) = immunity_msg {
                self.messages.push_wrapped(msg);
            }
            if let Some(msg) = dark_knight_speech {
                self.messages.push_wrapped(msg);
            }
            for msg in logs {
                self.dlog(msg);
            }
            // F9.11: auto-loot on melee kill removed — the original game does
            // NOT roll treasure or grant a weapon at the moment of death; the
            // body must be TAKEn (`fmain.c:3249-3283 search_body`). Special
            // setfig drops (witch lasso, necromancer transform) below remain
            // because they're triggered by leave_item at death time, not
            // treasure_probs.
            let _ = dead_npc; // kept for future hooks (no auto-loot)
            // SPEC §15.7: drop Talisman (ob_id 139) at necromancer's death coords.
            // Per reference/logic/quests.md#leave_item, `leave_item` places the
            // dropped object at `(abs_x, abs_y + 10)` — i.e. at the actor's
            // feet. Apply the same +10 Y offset the witch-lasso drop below uses.
            if let Some((tx, ty)) = necro_talisman_pos {
                use crate::game::game_state::WorldObject;
                let drop_y = (ty as i32 + 10).clamp(0, u16::MAX as i32) as u16;
                self.state.world_objects.push(WorldObject {
                    ob_id: 139,
                    ob_stat: 1, // ground item
                    region: self.state.region_num,
                    x: tx as u16,
                    y: drop_y,
                    visible: true,
                    goal: 0,
                });
                self.dlog("talisman (ob_id 139) placed at necromancer death coords".to_string());
            }
            // SPEC §14.20: drop Golden Lasso (ob_id 27) at witch's death coords (+10 Y).
            if let Some((wx, wy)) = witch_lasso_pos {
                use crate::game::game_state::WorldObject;
                let drop_y = (wy as i32 + 10).clamp(0, 32767) as u16;
                self.state.world_objects.push(WorldObject {
                    ob_id: 27,
                    ob_stat: 1, // ground item
                    region: self.state.region_num,
                    x: wx as u16,
                    y: drop_y,
                    visible: true,
                    goal: 0,
                });
                self.dlog("witch slain: golden lasso (ob_id 27) placed at death coords".to_string());
            }
        }
    }
}
