//! Per-frame actor update loop — NPC AI ticks, animation, position.
//! See `docs/spec/ai-encounters.md` for specification.

use super::*;

impl GameplayScene {
    pub(crate) fn update_actors(&mut self, _delta: u32) {
        use crate::game::npc::NpcState;
        use crate::game::npc_ai::tick_npc;

        // SPEC §13.2: decrement TALKING flicker timers; drop expired entries.
        // fmain.c:1557 — when `tactic` reaches 0, NPC returns to STILL.
        self.talk_flicker.retain(|_, t| {
            *t = t.saturating_sub(1);
            *t > 0
        });

        let hero_x = self.state.hero_x as i32;
        let hero_y = self.state.hero_y as i32;
        let hero_dead = self.state.vitality <= 0;
        let xtype = self.state.xtype;
        let tick = self.state.tick_counter;
        // SPEC §19.3: freeze_timer > 0 → hostile enemies (race < 7) skip all AI.
        let freeze = self.state.freeze_timer > 0;

        if let Some(ref mut table) = self.npc_table {
            // Snapshot NPC positions for Follow/Evade targeting.
            let positions: Vec<(i32, i32)> = table
                .npcs
                .iter()
                .map(|n| (n.x as i32, n.y as i32))
                .collect();

            // Determine leader: first active hostile NPC.
            let leader_idx = table.npcs.iter().position(|n| {
                n.active
                    && n.state != NpcState::Dead
                    && matches!(
                        n.goal,
                        Goal::Attack1 | Goal::Attack2 | Goal::Archer1 | Goal::Archer2
                    )
            });

            // 1. AI decision pass.
            // SPEC-GAP: `turtle_eggs` global counter (fmain.c) isn't plumbed yet;
            // pass false so snakes fall through to normal AI rather than unconditionally
            // marching to the nest (ref ai-system.md:84).
            let turtle_eggs = false;
            for (npc_idx, npc) in table.npcs.iter_mut().enumerate() {
                if !npc.active || npc.state == NpcState::Dead {
                    continue;
                }
                tick_npc(
                    npc,
                    npc_idx,
                    hero_x,
                    hero_y,
                    hero_dead,
                    leader_idx,
                    &positions,
                    tick,
                    xtype,
                    turtle_eggs,
                    freeze,
                );
            }

            // 2. Movement execution pass (sequential — later NPCs see earlier updates).
            // Track any successful NPC move to apply the global frustflag reset
            // (frustration.md "Reset asymmetry": fmain.c:1650 fires for every actor,
            // not just the hero, so any NPC's successful walk zeroes the hero's counter).
            let mut any_npc_moved = false;
            for i in 0..table.npcs.len() {
                if !table.npcs[i].active {
                    continue;
                }
                if table.npcs[i].state == NpcState::Dead {
                    continue;
                }
                if table.npcs[i].state != NpcState::Walking {
                    continue;
                }
                // SPEC §9.5 / §19.3: when frozen, all non-hero actors skip
                // movement entirely (fmain.c:1473 `goto statc`). AI may still
                // select tactics for non-hostile NPCs, but none of them move.
                if freeze {
                    continue;
                }
                // Build collision list: hero + all other active, alive NPCs.
                let mut others: Vec<(i32, i32)> =
                    Vec::with_capacity(crate::game::npc::MAX_NPCS + 1);
                others.push((hero_x, hero_y));
                for (j, other) in table.npcs.iter().enumerate() {
                    if j == i {
                        continue;
                    }
                    if !other.active {
                        continue;
                    }
                    if other.state == NpcState::Dead {
                        continue;
                    }
                    others.push((other.x as i32, other.y as i32));
                }
                let old_x = table.npcs[i].x;
                let old_y = table.npcs[i].y;
                table.npcs[i].tick_with_actors(self.map_world.as_ref(), &others);
                if table.npcs[i].x != old_x || table.npcs[i].y != old_y {
                    any_npc_moved = true;
                }
            }
            // fmain.c:1650 (NPC branch): any NPC's successful walk step resets
            // the global frustflag — same code path, no actor-index guard.
            if any_npc_moved {
                self.state.frustflag = 0;
            }

            // 3. Battleflag: true if any active NPC within 300px.
            let any_nearby = table.npcs.iter().any(|n| {
                n.active
                    && n.state != NpcState::Dead
                    && (n.x as i32 - hero_x).abs() < 300
                    && (n.y as i32 - hero_y).abs() < 300
            });
            self.state.battleflag = any_nearby;

            // 4. Sync NPC positions → Actor array for rendering.
            let anix = self.state.anix;
            let mut actor_idx = 1; // Skip actor 0 (player).
            for npc in &table.npcs {
                if !npc.active {
                    continue;
                }
                if actor_idx >= anix {
                    break;
                }
                let actor = &mut self.state.actors[actor_idx];
                actor.abs_x = npc.x as u16;
                actor.abs_y = npc.y as u16;
                actor.facing = npc.facing;
                actor.moving = npc.state == NpcState::Walking;
                actor.state = match npc.state {
                    NpcState::Walking => crate::game::actor::ActorState::Walking,
                    NpcState::Fighting => crate::game::actor::ActorState::Fighting(0),
                    NpcState::Shooting => crate::game::actor::ActorState::Shooting(0),
                    NpcState::Dying => crate::game::actor::ActorState::Dying,
                    NpcState::Dead => crate::game::actor::ActorState::Dead,
                    NpcState::Sinking => crate::game::actor::ActorState::Sinking,
                    NpcState::Still => crate::game::actor::ActorState::Still,
                };
                actor_idx += 1;
            }
        }

        // 5. Dragon fireball firing (SPEC §21.5: 25% per frame, speed 5, always south-facing).
        if let Some(ref mut table) = self.npc_table {
            use crate::game::combat::fire_missile;
            use crate::game::npc::NPC_TYPE_DRAGON;
            let hero_x = self.state.hero_x as i32;
            let hero_y = self.state.hero_y as i32;
            let tick = self.state.tick_counter;

            for npc in &mut table.npcs {
                if !npc.active || npc.state == NpcState::Dead {
                    continue;
                }
                if npc.npc_type != NPC_TYPE_DRAGON {
                    continue;
                }

                // Dragon always faces south (SPEC §21.5).
                npc.facing = 4;
                npc.state = NpcState::Still;

                // 25% per-frame firing chance (SPEC §21.5: rand4() == 0).
                let r = (tick.wrapping_mul(2654435761).wrapping_add(npc.x as u32)) & 3;
                if r == 0 {
                    let dir = facing_toward(npc.x as i32, npc.y as i32, hero_x, hero_y);
                    fire_missile(
                        &mut self.missiles,
                        npc.x as i32,
                        npc.y as i32,
                        dir,
                        5,
                        false,
                        5,
                    ); // weapon 5 = fireball, speed 5
                }
            }
        }

        // 6. Archer missile firing (from NPC Shooting state).
        if self.archer_cooldown > 0 {
            self.archer_cooldown -= 1;
        } else if let Some(ref table) = self.npc_table {
            let hero_x = self.state.hero_x as i32;
            let hero_y = self.state.hero_y as i32;
            for npc in &table.npcs {
                if !npc.active || npc.state == NpcState::Dead {
                    continue;
                }
                if npc.state != NpcState::Shooting {
                    continue;
                }
                let ax = npc.x as i32;
                let ay = npc.y as i32;
                if (hero_x - ax).abs().max((hero_y - ay).abs()) > 150 {
                    continue;
                }
                let dir = facing_toward(ax, ay, hero_x, hero_y);
                use crate::game::combat::fire_missile;
                fire_missile(&mut self.missiles, ax, ay, dir, 4, false, 2); // NPCs fire arrows (weapon 4) at speed 2
                self.archer_cooldown = 15;
                break;
            }
        }
    }
}
