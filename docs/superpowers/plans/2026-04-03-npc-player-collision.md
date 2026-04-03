# NPC-Player Actor Collision Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add actor-vs-actor collision to prevent NPCs from walking onto the player (and vice versa), matching the original game's `proxcheck()` actor collision loop.

**Architecture:** Add an `actor_collides()` function to `collision.rs` with the original's AABB (±11px X, ±9px Y). Modify `Npc::tick()` to accept a slice of other actor positions and check actor collision alongside terrain. Update the gameplay scene's movement loops (both NPC and player) to pass live actor positions.

**Tech Stack:** Rust, existing collision/NPC/actor modules

**Bug:** [#160](https://github.com/bacomatic/faery-tale-rs/issues/160)
**Spec:** `docs/superpowers/specs/2026-04-03-bug-npc-player-collision.md`

---

### Task 1: Add `actor_collides()` to collision.rs

**Files:**
- Modify: `src/game/collision.rs`

- [ ] **Step 1: Write failing tests for actor collision**

Add these tests at the bottom of `collision.rs` (inside the existing `#[cfg(test)] mod tests` block, or create one if absent):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actor_collides_overlapping() {
        // Two actors at same position — should collide.
        let others = vec![(100i32, 100i32)];
        assert!(actor_collides(100, 100, &others));
    }

    #[test]
    fn test_actor_collides_within_bbox() {
        // Within ±11 X / ±9 Y — should collide.
        let others = vec![(100i32, 100i32)];
        assert!(actor_collides(110, 108, &others)); // dx=10, dy=8 — inside
    }

    #[test]
    fn test_actor_collides_at_boundary() {
        // At exactly 11 X / 9 Y — should NOT collide (original uses strict <).
        let others = vec![(100i32, 100i32)];
        assert!(!actor_collides(111, 100, &others)); // dx=11 — outside
        assert!(!actor_collides(100, 109, &others)); // dy=9 — outside
    }

    #[test]
    fn test_actor_collides_negative_direction() {
        let others = vec![(100i32, 100i32)];
        assert!(actor_collides(90, 92, &others));  // dx=-10, dy=-8 — inside
        assert!(!actor_collides(89, 100, &others)); // dx=-11 — outside
        assert!(!actor_collides(100, 91, &others)); // dy=-9 — outside
    }

    #[test]
    fn test_actor_collides_empty_list() {
        assert!(!actor_collides(100, 100, &[]));
    }

    #[test]
    fn test_actor_collides_multiple_actors() {
        let others = vec![(50i32, 50i32), (200, 200)];
        assert!(actor_collides(55, 55, &others));   // near first
        assert!(!actor_collides(100, 100, &others)); // near neither
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test actor_collides -- --nocapture 2>&1 | head -30`
Expected: compile error — `actor_collides` not defined.

- [ ] **Step 3: Implement `actor_collides()`**

Add this function in `src/game/collision.rs` right after the existing `proxcheck()` function (before `calc_dist`):

```rust
/// Check if position (x, y) collides with any actor in the `others` list.
/// Uses the original game's bounding box: |dx| < 11, |dy| < 9.
/// Mirrors fmain2.c proxcheck() actor-vs-actor loop (lines 395–427).
pub fn actor_collides(x: i32, y: i32, others: &[(i32, i32)]) -> bool {
    for &(ox, oy) in others {
        let dx = x - ox;
        let dy = y - oy;
        if dx > -11 && dx < 11 && dy > -9 && dy < 9 {
            return true;
        }
    }
    false
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test actor_collides -- --nocapture 2>&1 | head -40`
Expected: all 6 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/game/collision.rs
git commit -m "feat(collision): add actor_collides() for actor-vs-actor AABB check

Ports the actor collision loop from fmain2.c proxcheck() (lines 395–427).
Uses the original bounding box: |dx| < 11, |dy| < 9 (pixel distance).

Closes: #160"
```

---

### Task 2: Add actor-aware movement to `Npc::tick()`

**Files:**
- Modify: `src/game/npc.rs`

**Context:** Currently `Npc::tick()` only checks terrain via `proxcheck()`. We need to also check `actor_collides()` against supplied actor positions. The existing `tick()` method stays for backward compat (tests use it with `None` world). We add an `other_actors` parameter.

- [ ] **Step 1: Write failing tests for actor-blocked NPC movement**

Add these tests to the existing `#[cfg(test)] mod tests` block in `src/game/npc.rs`:

```rust
    #[test]
    fn test_npc_tick_blocked_by_actor() {
        use crate::game::actor::Tactic;
        let mut npc = Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 1000,
            y: 1000,
            vitality: 10,
            active: true,
            facing: 2,  // East
            state: NpcState::Walking,
            ..Default::default()
        };
        // Place an actor directly east, blocking the primary + wall-slide directions.
        let others = vec![(1003, 1000), (1002, 998), (1002, 1002)];
        let old_x = npc.x;
        let old_y = npc.y;
        npc.tick_with_actors(None, false, &others);
        // All three directions blocked → should not move, should be frustrated.
        assert_eq!(npc.x, old_x);
        assert_eq!(npc.y, old_y);
        assert_eq!(npc.state, NpcState::Still);
        assert_eq!(npc.tactic, Tactic::Frust);
    }

    #[test]
    fn test_npc_tick_not_blocked_by_distant_actor() {
        let mut npc = Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 1000,
            y: 1000,
            vitality: 10,
            active: true,
            facing: 2,  // East
            state: NpcState::Walking,
            ..Default::default()
        };
        // Place actor far away — should not block.
        let others = vec![(2000, 2000)];
        let old_x = npc.x;
        npc.tick_with_actors(None, false, &others);
        assert!(npc.x > old_x, "NPC should move east — actor is far away");
    }

    #[test]
    fn test_npc_tick_empty_actors_same_as_no_actors() {
        let mut npc1 = Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 1000,
            y: 1000,
            vitality: 10,
            active: true,
            facing: 2,
            state: NpcState::Walking,
            ..Default::default()
        };
        let mut npc2 = npc1.clone();
        npc1.tick(None, false);
        npc2.tick_with_actors(None, false, &[]);
        assert_eq!(npc1.x, npc2.x);
        assert_eq!(npc1.y, npc2.y);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test npc_tick_blocked_by_actor -- --nocapture 2>&1 | head -20`
Expected: compile error — `tick_with_actors` not defined.

- [ ] **Step 3: Implement `tick_with_actors()`**

In `src/game/npc.rs`, add a new method to the `impl Npc` block. Refactor so the existing `tick()` delegates to `tick_with_actors()` with an empty slice:

Replace the existing `tick()` method body with this:

```rust
    /// Execute one frame of movement (terrain-only collision).
    /// Delegates to `tick_with_actors()` with no actor positions.
    pub fn tick(
        &mut self,
        world: Option<&crate::game::world_data::WorldData>,
        indoor: bool,
    ) {
        self.tick_with_actors(world, indoor, &[]);
    }

    /// Execute one frame of movement with both terrain and actor collision.
    /// `other_actors`: positions of all other live actors (hero + other NPCs)
    /// that this NPC should not overlap with.
    pub fn tick_with_actors(
        &mut self,
        world: Option<&crate::game::world_data::WorldData>,
        indoor: bool,
        other_actors: &[(i32, i32)],
    ) {
        use crate::game::collision::{proxcheck, actor_collides, newx, newy};
        use crate::game::actor::Tactic;

        if !self.active || self.state != NpcState::Walking {
            return;
        }

        let facing = self.facing;
        let dist = 2i32;

        let proposed_x = newx(self.x as u16, facing, dist);
        let proposed_y = newy(self.y as u16, facing, dist, indoor);

        // Race-specific terrain bypass: wraith (race 2) skips terrain checks.
        let terrain_passable = self.race == RACE_WRAITH
            || proxcheck(world, proposed_x as i32, proposed_y as i32);
        let actor_passable = !actor_collides(proposed_x as i32, proposed_y as i32, other_actors);

        if terrain_passable && actor_passable {
            self.x = proposed_x as i16;
            self.y = proposed_y as i16;
        } else {
            // Wall-sliding: try clockwise then counter-clockwise deviation.
            let dev_cw = (facing + 1) & 7;
            let cw_x = newx(self.x as u16, dev_cw, dist);
            let cw_y = newy(self.y as u16, dev_cw, dist, indoor);
            let cw_terrain = self.race == RACE_WRAITH
                || proxcheck(world, cw_x as i32, cw_y as i32);
            let cw_actor = !actor_collides(cw_x as i32, cw_y as i32, other_actors);
            if cw_terrain && cw_actor {
                self.x = cw_x as i16;
                self.y = cw_y as i16;
            } else {
                let dev_ccw = (facing.wrapping_sub(1)) & 7;
                let ccw_x = newx(self.x as u16, dev_ccw, dist);
                let ccw_y = newy(self.y as u16, dev_ccw, dist, indoor);
                let ccw_terrain = self.race == RACE_WRAITH
                    || proxcheck(world, ccw_x as i32, ccw_y as i32);
                let ccw_actor = !actor_collides(ccw_x as i32, ccw_y as i32, other_actors);
                if ccw_terrain && ccw_actor {
                    self.x = ccw_x as i16;
                    self.y = ccw_y as i16;
                } else {
                    // Fully blocked — frustrated.
                    self.state = NpcState::Still;
                    self.tactic = Tactic::Frust;
                }
            }
        }
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test npc_tick -- --nocapture 2>&1 | head -50`
Expected: all NPC tick tests pass (existing + new).

- [ ] **Step 5: Commit**

```bash
git add src/game/npc.rs
git commit -m "feat(npc): add tick_with_actors() for actor-aware NPC movement

Npc::tick() now delegates to tick_with_actors(&[]) so existing callers
are unaffected. tick_with_actors() checks both terrain (proxcheck) and
actor collision (actor_collides) before finalizing movement."
```

---

### Task 3: Wire actor collision into gameplay_scene movement loops

**Files:**
- Modify: `src/game/gameplay_scene.rs`

**Context:** Two call sites need actor collision:
1. **NPC movement loop** (line ~1483): Replace `npc.tick()` with `npc.tick_with_actors()`, passing hero position + all other active NPC positions (excluding self). Positions update sequentially — later NPCs see earlier NPCs' updated positions.
2. **Player movement** (line ~695): After computing `can_move` from terrain `proxcheck()`, also check `actor_collides()` against all active NPC positions.

- [ ] **Step 1: Update NPC movement loop to use `tick_with_actors()`**

In `update_actors()`, replace the NPC movement execution pass (step 2) with an indexed loop that builds actor positions for each NPC:

Find this code block:
```rust
            // 2. Movement execution pass.
            for npc in &mut table.npcs {
                if !npc.active { continue; }
                npc.tick(self.map_world.as_ref(), indoor);
            }
```

Replace with:
```rust
            // 2. Movement execution pass (sequential — later NPCs see earlier updates).
            for i in 0..table.npcs.len() {
                if !table.npcs[i].active { continue; }
                if table.npcs[i].state != NpcState::Walking { continue; }
                // Build collision list: hero + all other active, alive NPCs.
                let mut others: Vec<(i32, i32)> = Vec::with_capacity(MAX_NPCS + 1);
                others.push((hero_x, hero_y));
                for (j, other) in table.npcs.iter().enumerate() {
                    if j == i { continue; }
                    if !other.active { continue; }
                    if other.state == NpcState::Dead { continue; }
                    others.push((other.x as i32, other.y as i32));
                }
                table.npcs[i].tick_with_actors(self.map_world.as_ref(), indoor, &others);
            }
```

Note: add `use crate::game::npc::MAX_NPCS;` to the existing imports at the top of `update_actors()` if not already present.

- [ ] **Step 2: Update player movement to check actor collision**

In the player movement section (around line 695), after computing `can_move` from terrain proxcheck, also check actor collision. Find:

```rust
            let mut can_move = !turtle_blocked
                && (self.state.flying != 0 || self.state.on_raft
                    || collision::proxcheck(self.map_world.as_ref(), new_x as i32, new_y as i32));
```

Replace with:
```rust
            let mut can_move = !turtle_blocked
                && (self.state.flying != 0 || self.state.on_raft
                    || collision::proxcheck(self.map_world.as_ref(), new_x as i32, new_y as i32));

            // Actor collision: player cannot walk into live NPCs (mirrors original proxcheck actor loop).
            if can_move && self.state.flying == 0 {
                let npc_positions: Vec<(i32, i32)> = self.npc_table.as_ref()
                    .map(|t| t.npcs.iter()
                        .filter(|n| n.active && n.state != crate::game::npc::NpcState::Dead)
                        .map(|n| (n.x as i32, n.y as i32))
                        .collect())
                    .unwrap_or_default();
                if collision::actor_collides(new_x as i32, new_y as i32, &npc_positions) {
                    can_move = false;
                }
            }
```

Also add actor collision to the wall-slide deviations. Find the deviation block that checks `dev1_x`/`dev1_y` and `dev2_x`/`dev2_y` with `proxcheck`. Each deviation check should also verify `!collision::actor_collides(dev_x as i32, dev_y as i32, &npc_positions)`.

The npc_positions variable needs to be computed before the deviation block. Move it up before the `can_move` declaration or recompute. Simplest approach: compute `npc_positions` once before `can_move` and reuse it.

Restructured player movement section:

Replace:
```rust
            let mut can_move = !turtle_blocked
                && (self.state.flying != 0 || self.state.on_raft
                    || collision::proxcheck(self.map_world.as_ref(), new_x as i32, new_y as i32));

            // Direction deviation (wall-sliding): fmain.c checkdev1/checkdev2.
```

With:
```rust
            // Gather live NPC positions for actor collision (mirrors original proxcheck actor loop).
            let npc_positions: Vec<(i32, i32)> = if self.state.flying == 0 {
                self.npc_table.as_ref()
                    .map(|t| t.npcs.iter()
                        .filter(|n| n.active && n.state != crate::game::npc::NpcState::Dead)
                        .map(|n| (n.x as i32, n.y as i32))
                        .collect())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

            let mut can_move = !turtle_blocked
                && (self.state.flying != 0 || self.state.on_raft
                    || (collision::proxcheck(self.map_world.as_ref(), new_x as i32, new_y as i32)
                        && !collision::actor_collides(new_x as i32, new_y as i32, &npc_positions)));

            // Direction deviation (wall-sliding): fmain.c checkdev1/checkdev2.
```

Then update the deviation checks too. Find:
```rust
                    if collision::proxcheck(self.map_world.as_ref(), dev1_x as i32, dev1_y as i32) {
```
Replace with:
```rust
                    if collision::proxcheck(self.map_world.as_ref(), dev1_x as i32, dev1_y as i32)
                        && !collision::actor_collides(dev1_x as i32, dev1_y as i32, &npc_positions) {
```

Find:
```rust
                        if collision::proxcheck(self.map_world.as_ref(), dev2_x as i32, dev2_y as i32) {
```
Replace with:
```rust
                        if collision::proxcheck(self.map_world.as_ref(), dev2_x as i32, dev2_y as i32)
                            && !collision::actor_collides(dev2_x as i32, dev2_y as i32, &npc_positions) {
```

- [ ] **Step 3: Run full test suite**

Run: `cargo test 2>&1 | tail -20`
Expected: all tests pass (no regressions).

- [ ] **Step 4: Build to confirm no compile errors**

Run: `cargo build 2>&1 | tail -20`
Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "feat(gameplay): wire actor collision into NPC and player movement

NPC movement loop now passes hero + other NPC positions to
tick_with_actors(), preventing NPCs from stacking on the player.
Player movement also checks actor_collides() against live NPC
positions, preventing the player from walking through NPCs.

Closes: #160"
```
