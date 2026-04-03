# Combat Loop Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the broken dual-path combat system with a single unified combat tick matching fmain.c, fix input routing so all attack triggers work identically, and implement press-to-aim/release-to-fire bow behavior.

**Architecture:** A new `run_combat_tick()` on `GameplayScene` iterates all combatants each frame (hero + NPCs) in one pass, mirroring fmain.c:2680–2730. Input is unified through the `input.fight` flag. The bow uses a 2-state machine (SHOOT1 on press, fire on release). `apply_melee_combat()` and `fight_cooldown` are deleted.

**Tech Stack:** Rust, SDL2 (existing), no new dependencies.

**Spec:** `docs/superpowers/specs/2026-04-02-combat-loop-rewrite-design.md`

---

### Task 1: Unify fight input routing

All attack triggers must set/clear `self.input.fight` instead of calling combat directly.

**Files:**
- Modify: `src/game/gameplay_scene.rs` (event handling ~lines 3494–3505, do_option ~lines 2121–2140)

- [ ] **Step 1: Change controller button handling to set input.fight**

Replace the `ControllerButtonDown` handler to set `input.fight` when the action is `GameAction::Fight`, and add fight-clear logic to `ControllerButtonUp`:

In `src/game/gameplay_scene.rs`, change the `Event::ControllerButtonDown` arm (~line 3494):

```rust
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
```

Change the `Event::ControllerButtonUp` arm (~line 3502):

```rust
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
```

- [ ] **Step 2: Change do_option(GameAction::Fight) to set input.fight**

Replace the `GameAction::Fight` arm in `do_option()` (~line 2121) to just set the flag:

```rust
            GameAction::Fight => {
                self.input.fight = true;
            }
```

This covers any remaining code paths that dispatch `GameAction::Fight` through the action system (e.g., remapped keyboard bindings).

- [ ] **Step 3: Remove fight_cooldown field**

In the `GameplayScene` struct definition (~line 286), remove:

```rust
    /// Frames remaining before next melee swing can land (rate-limits continuous fight).
    fight_cooldown: u32,
```

And remove its initialization (search for `fight_cooldown: 0` or `fight_cooldown:` in the constructor/Default impl). Also remove any remaining references to `self.fight_cooldown` in the codebase (the ones in `apply_player_input()` and `do_option` — these will be cleaned up in Task 3 when `apply_melee_combat()` is replaced).

- [ ] **Step 4: Build and fix compile errors**

Run: `cargo build 2>&1 | head -60`
Expected: May have compile errors from removed `fight_cooldown` references. Fix any remaining references by deleting the lines that read/write `fight_cooldown`. The `apply_melee_combat()` method and its calls will be removed in Task 3, so for now just comment out or stub the calls if needed to get a clean compile.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "combat: unify fight input routing through input.fight flag

Controller and do_option(Fight) now set input.fight instead of
calling apply_melee_combat() directly. fight_cooldown removed."
```

---

### Task 2: Implement run_combat_tick()

Port the fmain.c combat loop (lines 2680–2730) as a unified per-frame combat tick.

**Files:**
- Modify: `src/game/combat.rs` (add helper functions)
- Modify: `src/game/gameplay_scene.rs` (add `run_combat_tick()` method)

**Reference:** `original/fmain.c:2680–2730`, `RESEARCH.md` Combat System section.

- [ ] **Step 1: Add weapon_tip_offset helper to combat.rs**

Add a public helper that computes the weapon tip position with jitter, matching `newx/newy` in fmain.c:

```rust
/// Compute weapon tip position with jitter (ports fmain.c newx/newy + rand8() - 3).
/// `wt` is the weapon value (after cap).
/// Returns (tip_x, tip_y) in world coordinates.
pub fn weapon_tip(abs_x: i32, abs_y: i32, facing: u8, wt: i16) -> (i32, i32) {
    let offset = (wt * 2) as i32;
    let (ox, oy): (i32, i32) = match facing & 7 {
        0 => (0, -offset),
        1 => (offset, -offset),
        2 => (offset, 0),
        3 => (offset, offset),
        4 => (0, offset),
        5 => (-offset, offset),
        6 => (-offset, 0),
        7 => (-offset, -offset),
        _ => (0, -offset),
    };
    // rand8() - 3: jitter ±3 pixels (original uses rand8 which returns 0-7, so -3 gives -3..+4)
    let jitter_x = (melee_rand(8) as i32) - 3;
    let jitter_y = (melee_rand(8) as i32) - 3;
    (abs_x + ox + jitter_x, abs_y + oy + jitter_y)
}

/// Compute melee reach for a combatant.
/// For hero (is_hero=true): (brave/20) + 5, capped at 15, min 4.
/// For NPCs: 2 + rand4(tick), capped at 15.
pub fn combat_reach(is_hero: bool, brave: i16, tick: u32) -> i16 {
    let bv = if is_hero {
        ((brave / 20) + 5).max(4)
    } else {
        2 + rand4(tick) as i16
    };
    bv.min(15)
}

/// rand256(): random 0–255 for dodge rolls.
pub fn rand256() -> i16 {
    melee_rand(256) as i16
}

/// rand8(): random 0–7 for jitter.
pub fn rand8() -> u32 {
    melee_rand(8)
}
```

- [ ] **Step 2: Implement run_combat_tick() on GameplayScene**

Add the following method to `GameplayScene` in `src/game/gameplay_scene.rs`:

```rust
    /// Unified combat tick — runs every frame for all combatants.
    /// Ports fmain.c:2680–2730 sword proximity loop.
    fn run_combat_tick(&mut self) {
        use crate::game::combat::{weapon_tip, combat_reach, rand256, bitrand, bitrand_damage};
        use crate::game::actor::ActorState;
        use crate::game::debug_command::GodModeFlags;

        let freeze = self.state.freeze_timer > 0;
        let brave = self.state.brave;
        let tick = self.state.cycle;
        let one_hit_kill = self.state.god_mode.contains(GodModeFlags::ONE_HIT_KILL);
        let insane_reach = self.state.god_mode.contains(GodModeFlags::INSANE_REACH);
        let anix = self.state.anix;

        // Snapshot combatant data: (abs_x, abs_y, facing, weapon, is_fighting, is_hero, active)
        // Actor 0 = hero, actor 1 = raft (skip), actors 2..anix = NPCs.
        struct Combatant {
            x: i32,
            y: i32,
            facing: u8,
            weapon: u8,
            fighting: bool,
            is_hero: bool,
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
                is_hero: i == 0,
                active: !matches!(actor.state, ActorState::Dead | ActorState::Dying),
            });
        }

        // Collect hits to apply after the loop (avoids borrow conflicts).
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
            if i > 0 && freeze { break; } // NPCs frozen

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
                if hit_roll && dist < reach as i32 && !freeze {
                    let damage = if one_hit_kill && i == 0 {
                        // God mode: instant kill
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
                } else if dist < (reach as i32 + 2) && wt != 5 {
                    // TODO: near-miss sound effect(1, 150 + rand256())
                }
            }
        }

        // Apply hits
        for hit in hits {
            self.apply_hit(hit.attacker, hit.target, hit.facing, hit.damage);
        }
    }
```

- [ ] **Step 3: Wire run_combat_tick() into the main game loop**

In the main tick loop (~line 3870), add the call after `update_actors()`:

```rust
            self.update_actors(1);
            self.run_combat_tick();
```

- [ ] **Step 4: Build and verify**

Run: `cargo build 2>&1 | head -60`
Expected: Clean compile (apply_hit doesn't exist yet — add a stub).

Add a temporary stub if needed:

```rust
    fn apply_hit(&mut self, _attacker: usize, _target: usize, _facing: u8, _damage: i16) {
        // Stub — implemented in Task 3
    }
```

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "combat: add unified run_combat_tick() per-frame loop

Ports fmain.c:2680-2730 sword proximity loop. All combatants
(hero + NPCs) swing each frame. Wired into main tick after
update_actors()."
```

---

### Task 3: Implement apply_hit() and remove apply_melee_combat()

Port `dohit()` from fmain2.c and delete the old combat resolution path.

**Files:**
- Modify: `src/game/gameplay_scene.rs` (replace stub, delete old method)
- Modify: `src/game/npc.rs` (add missing race constants if needed)

**Reference:** `original/fmain2.c:317–356`, `RESEARCH.md` Combat System section.

- [ ] **Step 1: Add missing race constants**

Check `src/game/npc.rs` for the Necromancer and Witch race values. The original uses race 9 (Necromancer) and 0x89 (Witch) for the weapon >= 4 immunity guard. Add if missing:

```rust
pub const RACE_NECROMANCER: u8 = 9;
pub const RACE_WITCH: u8 = 0x89;
```

- [ ] **Step 2: Implement apply_hit()**

Replace the stub in `src/game/gameplay_scene.rs`:

```rust
    /// Apply one melee hit from attacker to target.
    /// Ports fmain2.c dohit(i, j, fc, wt).
    fn apply_hit(&mut self, attacker_idx: usize, target_idx: usize, facing: u8, damage: i16) {
        use crate::game::npc::{RACE_NECROMANCER, RACE_WITCH};

        // Determine if target is hero (idx 0) or NPC
        if target_idx == 0 {
            // NPC hitting hero
            self.state.vitality = (self.state.vitality - damage).max(0);
            self.dlog(format!("enemy hit hero for {}", damage));

            // Pushback: hero pushed 2px in attacker's facing direction
            let (px, py) = push_offset(facing, 2);
            self.state.hero_x = (self.state.hero_x as i32 + px as i32).clamp(0, 32767) as u16;
            self.state.hero_y = (self.state.hero_y as i32 + py as i32).clamp(0, 32767) as u16;

            // checkdead for hero
            if self.state.vitality <= 0 {
                if let Some(player) = self.state.actors.first_mut() {
                    player.state = crate::game::actor::ActorState::Dying;
                }
                self.state.luck = (self.state.luck - 5).max(0);
                self.dlog("hero killed in combat".to_string());
            }
        } else {
            // Hero (or NPC) hitting an NPC
            if let Some(ref mut table) = self.npc_table {
                // Map actor index to NPC table index: actors 2..anix correspond to npcs 0..
                let npc_idx = target_idx.saturating_sub(2);
                if npc_idx < table.npcs.len() {
                    let npc = &mut table.npcs[npc_idx];

                    // Immunity guard: Necromancer/Witch immune unless weapon >= 4
                    let attacker_weapon = if attacker_idx == 0 {
                        self.state.actors.first().map_or(1, |a| a.weapon)
                    } else {
                        // NPC-on-NPC: use attacker's weapon from actor
                        self.state.actors.get(attacker_idx).map_or(1, |a| a.weapon)
                    };
                    let actual_damage = if (npc.race == RACE_NECROMANCER || npc.race == RACE_WITCH)
                        && attacker_weapon < 4
                    {
                        0 // immune
                    } else {
                        damage
                    };

                    npc.vitality -= actual_damage;
                    if npc.vitality < 0 { npc.vitality = 0; }

                    // Pushback on target: 2px in attacker facing
                    let (px, py) = push_offset(facing, 2);
                    npc.x = (npc.x as i32 + px as i32).clamp(0, 32767) as i16;
                    npc.y = (npc.y as i32 + py as i32).clamp(0, 32767) as i16;

                    // If hero is attacker, hero also pushes forward 2px (recoil)
                    if attacker_idx == 0 {
                        let (rx, ry) = push_offset(facing, 2);
                        self.state.hero_x = (self.state.hero_x as i32 + rx as i32).clamp(0, 32767) as u16;
                        self.state.hero_y = (self.state.hero_y as i32 + ry as i32).clamp(0, 32767) as u16;
                    }

                    if actual_damage > 0 {
                        self.dlog(format!("combat hit npc {} for {}", npc_idx, actual_damage));
                    }

                    // checkdead
                    if npc.vitality == 0 {
                        npc.active = false;
                        self.state.brave = (self.state.brave + 1).min(100);

                        let npc_snap = npc.clone();
                        let tick = self.state.tick_counter;
                        if let Some(drop) = crate::game::loot::roll_treasure(&npc_snap, tick) {
                            let weapon_slot = crate::game::loot::award_treasure(&mut self.state, &drop);
                            if let Some(w) = weapon_slot {
                                let cur = self.state.actors.first().map_or(0, |a| a.weapon);
                                if w > cur {
                                    if let Some(player) = self.state.actors.first_mut() {
                                        player.weapon = w;
                                    }
                                    self.dlog(format!("found better weapon type {}", w));
                                }
                            }
                        }
                        self.dlog(format!("enemy slain, bravery now {}", self.state.brave));
                    }
                }
            }
        }
    }
```

Also add the push_offset helper as a free function near the combat methods:

```rust
/// Compute pixel offset for pushback in a facing direction.
fn push_offset(facing: u8, distance: i32) -> (i32, i32) {
    match facing & 7 {
        0 => (0, -distance),
        1 => (distance, -distance),
        2 => (distance, 0),
        3 => (distance, distance),
        4 => (0, distance),
        5 => (-distance, distance),
        6 => (-distance, 0),
        7 => (-distance, -distance),
        _ => (0, 0),
    }
}
```

- [ ] **Step 3: Delete apply_melee_combat()**

Remove the entire `apply_melee_combat()` method from `gameplay_scene.rs` (~lines 1206–1310). This includes:
- The hero swing logic
- The enemy counterattack block
- All references to `fight_cooldown` within it

Also remove any remaining calls to `apply_melee_combat()` — there should be none left after Task 1 changed `do_option(Fight)` and the `apply_player_input()` fight branch no longer calls it.

- [ ] **Step 4: Clean up apply_player_input() fight branch**

The fight branch in `apply_player_input()` (~lines 525–560) currently calls `apply_melee_combat()` and manages `fight_cooldown`. Remove those lines. The branch should now only:
1. Update hero facing from directional input
2. Set `ActorState::Fighting(next_state)` on hero actor
3. Set `player.moving = false`
4. Return early (skip movement)

```rust
        if self.input.fight {
            let facing = match dir {
                Direction::N  => 0u8, Direction::NE => 1, Direction::E  => 2, Direction::SE => 3,
                Direction::S  => 4,   Direction::SW => 5, Direction::W  => 6, Direction::NW => 7,
                Direction::None => self.state.facing,
            };
            self.state.facing = facing;

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
            return;
        }
```

- [ ] **Step 5: Build and run tests**

Run: `cargo build 2>&1 | head -60`
Then: `cargo test 2>&1 | tail -20`
Expected: Clean compile, all existing tests pass.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "combat: implement apply_hit(), delete apply_melee_combat()

Ports dohit() from fmain2.c with pushback, immunity guards,
and checkdead. Removes the old dual-path combat resolution."
```

---

### Task 4: Bow state machine (press-to-aim, release-to-fire)

Implement the original SHOOT1/SHOOT3 two-state machine for bow combat.

**Files:**
- Modify: `src/game/gameplay_scene.rs` (apply_player_input fight branch, bow release logic)
- Modify: `src/game/game_state.rs` (ITEM_BOW, ITEM_ARROWS constants — verify they exist)

**Reference:** `original/fmain.c:1584–1622, 1907–1930`

- [ ] **Step 1: Add bow detection to the fight branch in apply_player_input()**

Modify the fight branch (~line 525) to check if the hero has a bow equipped. If weapon == 4 (bow) and arrows > 0, set `ActorState::Shooting(0)` instead of `Fighting`:

```rust
        if self.input.fight {
            use crate::game::game_state::{ITEM_BOW, ITEM_ARROWS};

            let facing = match dir {
                Direction::N  => 0u8, Direction::NE => 1, Direction::E  => 2, Direction::SE => 3,
                Direction::S  => 4,   Direction::SW => 5, Direction::W  => 6, Direction::NW => 7,
                Direction::None => self.state.facing,
            };
            self.state.facing = facing;

            let hero_weapon = self.state.actors.first().map_or(1, |a| a.weapon);
            let has_bow = hero_weapon == 4;
            let has_arrows = self.state.stuff()[ITEM_ARROWS] > 0;

            if has_bow && has_arrows {
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
            }
            return;
        }
```

- [ ] **Step 2: Add bow release-to-fire logic**

After the fight branch check, add logic to detect when `input.fight` just went false while hero is in Shooting state (the release frame). Place this right after the `if self.input.fight { ... return; }` block:

```rust
        // Bow release-to-fire: SHOOT1 → SHOOT3 transition on button release.
        // Arrow fires on the frame input.fight goes false while in Shooting state.
        if let Some(player) = self.state.actors.first() {
            if matches!(player.state, ActorState::Shooting(_)) {
                // Button released while aiming — fire!
                use crate::game::game_state::ITEM_ARROWS;
                use crate::game::combat::fire_missile;

                if self.state.stuff()[ITEM_ARROWS] > 0 {
                    fire_missile(
                        &mut self.missiles,
                        self.state.hero_x as i32,
                        self.state.hero_y as i32,
                        self.state.facing,
                        5, // arrow damage
                        true,
                    );
                    self.state.stuff_mut()[ITEM_ARROWS] -= 1;
                    self.messages.push("You shoot an arrow!");
                    let wealth = self.state.wealth;
                    self.menu.set_options(self.state.stuff(), wealth);
                }

                // Return to Still after firing
                if let Some(player) = self.state.actors.first_mut() {
                    player.state = ActorState::Still;
                    player.moving = false;
                }
            }
        }
```

- [ ] **Step 3: Build and verify**

Run: `cargo build 2>&1 | head -60`
Expected: Clean compile.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "combat: bow press-to-aim, release-to-fire state machine

SHOOT1 on press (aiming, no movement), arrow fires on release.
Matches original fmain.c SHOOT1/SHOOT3 two-state cycle."
```

---

### Task 5: Tests

Add unit tests for the new combat tick behavior.

**Files:**
- Modify: `src/game/combat.rs` (tests for new helpers)
- Modify: `src/game/gameplay_scene.rs` (integration tests if test infrastructure exists)

- [ ] **Step 1: Add tests for new combat.rs helpers**

Add to the `#[cfg(test)] mod tests` block in `src/game/combat.rs`:

```rust
    #[test]
    fn test_weapon_tip_offset_north() {
        // Facing north (0), wt=3: offset should be (0, -6) plus jitter
        let (tx, ty) = weapon_tip(100, 100, 0, 3);
        // tip_y should be less than 100 (north = negative Y)
        assert!(ty < 100, "north tip_y={} should be < 100", ty);
        // tip_x should be near 100 (jitter ±3)
        assert!((tx - 100).abs() <= 4, "north tip_x={} too far from 100", tx);
    }

    #[test]
    fn test_weapon_tip_offset_east() {
        let (tx, ty) = weapon_tip(100, 100, 2, 3);
        assert!(tx > 100, "east tip_x={} should be > 100", tx);
        assert!((ty - 100).abs() <= 4, "east tip_y={} too far from 100", ty);
    }

    #[test]
    fn test_combat_reach_hero() {
        // brave=50: (50/20)+5 = 7
        let r = combat_reach(true, 50, 0);
        assert_eq!(r, 7);
    }

    #[test]
    fn test_combat_reach_hero_cap() {
        // brave=250: (250/20)+5 = 17, capped to 15
        let r = combat_reach(true, 250, 0);
        assert_eq!(r, 15);
    }

    #[test]
    fn test_combat_reach_hero_min() {
        // brave=0: (0/20)+5 = 5, min 4 (5 > 4 so stays 5)
        let r = combat_reach(true, 0, 0);
        assert_eq!(r, 5);
    }

    #[test]
    fn test_combat_reach_npc_range() {
        // NPC reach: 2 + rand4(tick), should be 2..=5 then capped at 15
        for tick in 0..100u32 {
            let r = combat_reach(false, 0, tick);
            assert!((2..=5).contains(&r), "npc reach {} out of range", r);
        }
    }

    #[test]
    fn test_rand256_range() {
        for _ in 0..1000 {
            let r = rand256();
            assert!((0..=255).contains(&r), "rand256 returned {}", r);
        }
    }
```

- [ ] **Step 2: Add pushback helper test**

In `src/game/gameplay_scene.rs`, add a test (in any existing `#[cfg(test)]` block, or create one):

```rust
#[cfg(test)]
mod combat_tests {
    use super::push_offset;

    #[test]
    fn test_push_offset_directions() {
        assert_eq!(push_offset(0, 2), (0, -2));  // N
        assert_eq!(push_offset(2, 2), (2, 0));   // E
        assert_eq!(push_offset(4, 2), (0, 2));   // S
        assert_eq!(push_offset(6, 2), (-2, 0));  // W
        assert_eq!(push_offset(1, 2), (2, -2));  // NE
        assert_eq!(push_offset(3, 2), (2, 2));   // SE
        assert_eq!(push_offset(5, 2), (-2, 2));  // SW
        assert_eq!(push_offset(7, 2), (-2, -2)); // NW
    }
}
```

- [ ] **Step 3: Run all tests**

Run: `cargo test 2>&1 | tail -30`
Expected: All tests pass, including the new ones.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "combat: add unit tests for combat tick helpers

Tests for weapon_tip, combat_reach, rand256, push_offset."
```

---

### Task 6: Manual verification and cleanup

Verify the full combat flow works end-to-end.

**Files:**
- Possibly modify: `src/game/gameplay_scene.rs` (any remaining cleanup)

- [ ] **Step 1: Run the game with debug mode**

Run: `cargo run -- --debug --skip-intro`

Test scenarios:
1. Press numpad 0 (or controller A): hero should stop, play fight animation, swing every frame
2. Release: hero returns to normal movement
3. With enemies nearby: dlog should show per-frame "combat hit" messages
4. With bow+arrows equipped: hold attack → aiming pose, release → arrow fires
5. Rapidly tap attack with bow: one arrow per tap

- [ ] **Step 2: Clean up any remaining deprecated references**

Search for any remaining references to `apply_melee_combat` or `fight_cooldown`:

Run: `grep -rn "apply_melee_combat\|fight_cooldown" src/`
Expected: No matches (all removed).

- [ ] **Step 3: Run full test suite one final time**

Run: `cargo test 2>&1 | tail -10`
Expected: All tests pass.

- [ ] **Step 4: Final commit if any cleanup was needed**

```bash
git add -A && git commit -m "combat: final cleanup after combat loop rewrite"
```
