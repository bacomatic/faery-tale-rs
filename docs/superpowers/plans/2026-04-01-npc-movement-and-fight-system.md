# NPC Movement & Fight System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix NPC movement to use collision/terrain checking with wall-sliding (Bug #154), and fix the player fight system to suppress movement, cycle attack animations via `trans_list`, and render fighting sprite frames (Bug #155).

**Architecture:** Two independent changes: (1) NPC `tick()` gains `WorldData` access for `proxcheck()` + 8-way direction + deviation, (2) `apply_player_input()` gains an exclusive fight branch with a 9-state animation state machine that selects fighting sprite frames 32–79.

**Tech Stack:** Rust, SDL2, existing `collision` module helpers (`proxcheck`, `newx`, `newy`)

**Issues:** #154, #155, #156 (follow-up)

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src/game/npc.rs` | Modify | Replace `tick()` with 8-way direction + proxcheck + deviation |
| `src/game/gameplay_scene.rs` | Modify | Pass `WorldData` to NPC tick; restructure `apply_player_input()` with exclusive fight branch; add fight frame selection in `blit_actors_to_framebuf` |
| `src/game/combat.rs` | Modify | Add `rand4()` helper for `trans_list` animation |
| `src/game/collision.rs` | Read-only | Existing `proxcheck`, `newx`, `newy` used by NPC tick |
| `src/game/actor.rs` | Read-only | `ActorState::Fighting(u8)` already exists |
| `src/game/sprites.rs` | Read-only | STATELIST frames 32–79 already exist |

---

## Task 1: NPC 8-way direction + proxcheck + wall-sliding

**Files:**
- Modify: `src/game/npc.rs` (lines 70–93: `tick()` method)
- Modify: `src/game/gameplay_scene.rs` (lines 1226–1240: NPC tick call site)

**Closes:** #154

- [ ] **Step 1: Write failing test for NPC direction calculation**

Add to `src/game/npc.rs` test module:

```rust
#[test]
fn test_npc_direction_to_hero() {
    // Hero directly east of NPC → facing should be 2 (E)
    assert_eq!(direction_to_target(100, 100, 200, 100), 2);
    // Hero directly north → facing 0 (N)
    assert_eq!(direction_to_target(100, 100, 100, 50), 0);
    // Hero NE → facing 1 (NE)
    assert_eq!(direction_to_target(100, 100, 200, 50), 1);
    // Hero SW → facing 5 (SW)
    assert_eq!(direction_to_target(100, 100, 50, 200), 5);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_npc_direction_to_hero`
Expected: FAIL — `direction_to_target` does not exist.

- [ ] **Step 3: Implement `direction_to_target` helper**

Add to `src/game/npc.rs` (above `tick()`):

```rust
/// Compute 8-way compass direction (0=N..7=NW) from (sx,sy) toward (tx,ty).
/// Mirrors the direction LUT in set_course (fmain2.c): uses com2[] mapping
/// from (xsign, ysign) to compass direction.
/// Returns 9 if at same position (STILL).
fn direction_to_target(sx: i16, sy: i16, tx: i16, ty: i16) -> u8 {
    let dx = tx as i32 - sx as i32;
    let dy = ty as i32 - sy as i32;
    if dx == 0 && dy == 0 {
        return 9; // STILL
    }
    // Suppress minor axis if major > 2× minor (set_course mode-1 smart seek).
    let adx = dx.abs();
    let ady = dy.abs();
    let eff_dx = if ady > adx * 2 { 0 } else { dx };
    let eff_dy = if adx > ady * 2 { 0 } else { dy };
    // Map (sign_x, sign_y) → 8-way direction.
    // sign: -1, 0, +1 → index 0, 1, 2
    let xi = (eff_dx.signum() + 1) as usize; // 0=left, 1=center, 2=right
    let yi = (eff_dy.signum() + 1) as usize; // 0=up, 1=center, 2=down
    // com2 LUT: [y_index][x_index] → facing (matches fmain2.c com2[9])
    const COM2: [[u8; 3]; 3] = [
        // dx: -1   0   +1
        [7, 0, 1],   // dy: -1 (NW, N, NE)
        [6, 9, 2],   // dy:  0 (W, STILL, E)
        [5, 4, 3],   // dy: +1 (SW, S, SE)
    ];
    COM2[yi][xi]
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_npc_direction_to_hero`
Expected: PASS

- [ ] **Step 5: Write failing test for NPC tick with collision**

Add to `src/game/npc.rs` test module:

```rust
#[test]
fn test_npc_tick_moves_toward_hero_with_direction_lut() {
    let mut npc = Npc {
        npc_type: NPC_TYPE_ORC,
        race: RACE_ENEMY,
        x: 1000,
        y: 1000,
        vitality: 10,
        gold: 0,
        speed: 2,
        active: true,
    };
    let old_x = npc.x;
    let old_y = npc.y;
    // Hero is directly east at (1100, 1000).
    // With no WorldData, proxcheck always passes.
    let adjacent = npc.tick(1100, 1000, None, false);
    assert!(!adjacent);
    // NPC should have moved east (x increased, y unchanged or nearly so).
    assert!(npc.x > old_x, "NPC should move east toward hero");
}
```

- [ ] **Step 6: Run test to verify it fails**

Run: `cargo test test_npc_tick_moves_toward_hero`
Expected: FAIL — `tick()` signature doesn't accept `WorldData` or `indoor` params.

- [ ] **Step 7: Rewrite `Npc::tick()` with 8-way direction + proxcheck + deviation**

Replace the `tick()` method in `src/game/npc.rs`:

```rust
/// Update NPC position for one frame tick.
/// Uses 8-way direction toward hero + proxcheck collision + wall-sliding.
/// Returns true if NPC is adjacent to hero (triggers encounter).
///
/// `world`: terrain data for collision checks (None = always passable).
/// `indoor`: true for indoor regions (region >= 8), affects Y wrapping.
pub fn tick(
    &mut self,
    hero_x: i16,
    hero_y: i16,
    world: Option<&crate::game::world_data::WorldData>,
    indoor: bool,
) -> bool {
    use crate::game::collision::{proxcheck, newx, newy, calc_dist};

    if !self.active { return false; }

    let dx = hero_x as i32 - self.x as i32;
    let dy = hero_y as i32 - self.y as i32;

    // Only chase within 200px range (original proximity).
    if dx.abs() > 200 || dy.abs() > 200 {
        return false;
    }

    let dist = calc_dist(self.x as i32, self.y as i32, hero_x as i32, hero_y as i32);

    if dist > 0 {
        let facing = direction_to_target(self.x, self.y, hero_x, hero_y);
        if facing < 9 {
            let speed = self.speed.max(1) as i32;
            let proposed_x = newx(self.x as u16, facing, speed);
            let proposed_y = newy(self.y as u16, facing, speed, indoor);

            // Race-specific terrain bypass: wraith (race 2) skips terrain checks.
            let terrain_passable = self.race == crate::game::npc::RACE_WRAITH
                || proxcheck(world, proposed_x as i32, proposed_y as i32);

            if terrain_passable {
                self.x = proposed_x as i16;
                self.y = proposed_y as i16;
            } else {
                // Wall-sliding: try deviation ±1 direction (fmain2.c set_course deviation).
                let dev1 = (facing + 1) & 7;
                let dev1_x = newx(self.x as u16, dev1, speed);
                let dev1_y = newy(self.y as u16, dev1, speed, indoor);
                if proxcheck(world, dev1_x as i32, dev1_y as i32) {
                    self.x = dev1_x as i16;
                    self.y = dev1_y as i16;
                } else {
                    let dev2 = (facing.wrapping_sub(1)) & 7;
                    let dev2_x = newx(self.x as u16, dev2, speed);
                    let dev2_y = newy(self.y as u16, dev2, speed, indoor);
                    if proxcheck(world, dev2_x as i32, dev2_y as i32) {
                        self.x = dev2_x as i16;
                        self.y = dev2_y as i16;
                    }
                    // Else: fully blocked, NPC stays put this frame.
                }
            }
        }
    }

    // Adjacent check: within 16px (original encounter trigger distance).
    dist < 16
}
```

- [ ] **Step 8: Run test to verify it passes**

Run: `cargo test test_npc_tick_moves_toward_hero`
Expected: PASS

- [ ] **Step 9: Update call site in `gameplay_scene.rs`**

In `src/game/gameplay_scene.rs`, update the NPC tick loop (around line 1226):

Replace:
```rust
if let Some(ref mut table) = self.npc_table {
    let hero_x = self.state.hero_x as i16;
    let hero_y = self.state.hero_y as i16;
    let mut any_approach = false;
    for npc in &mut table.npcs {
        let adjacent = npc.tick(hero_x, hero_y);
```

With:
```rust
if let Some(ref mut table) = self.npc_table {
    let hero_x = self.state.hero_x as i16;
    let hero_y = self.state.hero_y as i16;
    let indoor = self.state.region_num >= 8;
    let mut any_approach = false;
    for npc in &mut table.npcs {
        let adjacent = npc.tick(hero_x, hero_y, self.map_world.as_ref(), indoor);
```

- [ ] **Step 10: Run full test suite**

Run: `cargo test`
Expected: All tests pass. No compilation errors.

- [ ] **Step 11: Manual smoke test**

Run: `cargo run -- --debug --skip-intro`
Navigate to an area with enemies. Verify:
- NPCs no longer walk through walls or buildings.
- NPCs slide around obstacles when chasing the hero.
- NPCs still approach and trigger encounters.

- [ ] **Step 12: Commit**

```bash
git add src/game/npc.rs src/game/gameplay_scene.rs
git commit -m "fix: NPC movement uses proxcheck + 8-way direction + wall-sliding

Replace trivial dx/dy chase in Npc::tick() with proper 8-way
direction calculation via direction_to_target(), position updates
via collision::newx()/newy(), and terrain collision via proxcheck().

When blocked, NPCs try ±1 direction deviation for wall-sliding,
matching set_course mode-1 behavior from fmain2.c.

Wraiths (race 2) bypass terrain collision per original.

Closes: #154"
```

---

## Task 2: Exclusive fight branch — suppress movement during attack

**Files:**
- Modify: `src/game/gameplay_scene.rs` (lines 443–750: `apply_player_input()`)

**Closes:** #155 (part 1 of 3)

- [ ] **Step 1: Restructure `apply_player_input()` with fight-first branch**

In `src/game/gameplay_scene.rs`, at the top of `apply_player_input()`, add the exclusive fight branch before the existing direction/movement code.

After the line `let dir = self.current_direction();`, add:

```rust
// Exclusive fight branch — matches fmain.c where fighting is an else-if
// above walking. Movement is suppressed; only facing updates.
if self.input.fight {
    // Update facing from directional input (original: oldir → facing).
    let facing = match dir {
        Direction::N  => 0u8, Direction::NE => 1, Direction::E  => 2, Direction::SE => 3,
        Direction::S  => 4,   Direction::SW => 5, Direction::W  => 6, Direction::NW => 7,
        Direction::None => self.state.facing, // keep current facing
    };
    self.state.facing = facing;

    // Advance fight animation state via trans_list.
    let fight_state = match self.state.actors.first() {
        Some(Actor { state: ActorState::Fighting(s), .. }) => *s,
        _ => 0,
    };
    let next_state = advance_fight_state(fight_state, self.state.cycle);

    if let Some(player) = self.state.actors.first_mut() {
        player.facing = facing;
        player.moving = false;
        player.state = ActorState::Fighting(next_state);
    }

    // Melee combat rate-limited (unchanged logic, moved into fight branch).
    if self.fight_cooldown > 0 {
        self.fight_cooldown -= 1;
    }
    if self.fight_cooldown == 0 {
        self.apply_melee_combat();
        self.fight_cooldown = 10;
    }
    return;
}
```

Then remove the duplicate fight_cooldown / melee combat code that currently runs after the movement block (lines ~756–764), and remove the `if self.input.fight { player.state = ActorState::Fighting(0) }` checks inside the movement and still branches (lines ~738–749).

- [ ] **Step 2: Remove old fight state assignments from movement/still branches**

In the movement branch (the `if dir != Direction::None` block), replace:

```rust
if self.input.fight {
    player.state = ActorState::Fighting(0);
} else {
    player.state = ActorState::Walking;
}
```

With just:

```rust
player.state = ActorState::Walking;
```

And in the still branch (`else` block), replace:

```rust
if self.input.fight {
    player.state = ActorState::Fighting(0);
} else {
    player.state = ActorState::Still;
}
```

With just:

```rust
player.state = ActorState::Still;
```

- [ ] **Step 3: Remove the old fight_cooldown block after movement**

Remove the post-movement melee combat block (around lines 756–764):

```rust
// Melee combat when fight is held (npc-103).
// Rate-limited to one swing every 10 ticks (~1/3 s at 30 Hz), matching
// fmain.c's per-frame proximity check gated by weapon animation state.
if self.fight_cooldown > 0 {
    self.fight_cooldown -= 1;
}
if self.input.fight && self.fight_cooldown == 0 {
    self.apply_melee_combat();
    self.fight_cooldown = 10;
}
```

This logic is now inside the exclusive fight branch from Step 1.

- [ ] **Step 4: Build and run tests**

Run: `cargo test`
Expected: All tests pass. No compilation errors.

- [ ] **Step 5: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix: suppress player movement during fight (exclusive branch)

Restructure apply_player_input() so fighting is an exclusive branch
that runs before the movement pipeline, matching fmain.c's three
mutually exclusive branches: FIGHTING → WALKING → STANDING.

When input.fight is true, movement is suppressed but directional
input still updates the player's facing direction.

Part of #155"
```

---

## Task 3: Implement `trans_list` fight animation state machine

**Files:**
- Modify: `src/game/gameplay_scene.rs` (add `advance_fight_state` function, add `FIGHT_TRANS_LIST` const)
- Modify: `src/game/combat.rs` (add `rand4()` helper)

**Closes:** #155 (part 2 of 3)

- [ ] **Step 1: Write failing test for `rand4`**

Add to `src/game/combat.rs`:

```rust
#[test]
fn test_rand4_range() {
    // rand4 must return 0-3 for any input.
    for tick in 0..100u32 {
        let v = rand4(tick);
        assert!(v < 4, "rand4({tick}) returned {v}, expected 0-3");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_rand4_range`
Expected: FAIL — `rand4` does not exist.

- [ ] **Step 3: Implement `rand4` in `combat.rs`**

Add to `src/game/combat.rs`:

```rust
/// Random 0–3 from game tick, matching original rand4() used by trans_list.
/// Uses tick-seeded hash to avoid SystemTime dependency in animation loop.
pub fn rand4(tick: u32) -> usize {
    let h = tick.wrapping_mul(2246822519).wrapping_add(3266489917);
    (h as usize >> 16) & 3
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_rand4_range`
Expected: PASS

- [ ] **Step 5: Write failing test for fight state transitions**

Add to `src/game/gameplay_scene.rs` test module (or a new test at the bottom):

```rust
#[cfg(test)]
mod fight_tests {
    use super::*;

    #[test]
    fn test_fight_state_advances() {
        // Starting from state 0, advance should produce a valid state (0-8).
        let next = advance_fight_state(0, 42);
        assert!(next <= 8, "fight state {next} out of range 0-8");
    }

    #[test]
    fn test_fight_state_varies_with_tick() {
        // Different ticks should produce different transitions (not always same).
        let mut seen = std::collections::HashSet::new();
        for tick in 0..100u32 {
            seen.insert(advance_fight_state(0, tick));
        }
        assert!(seen.len() > 1, "trans_list should produce varied states");
    }
}
```

- [ ] **Step 6: Run test to verify it fails**

Run: `cargo test fight_tests`
Expected: FAIL — `advance_fight_state` does not exist.

- [ ] **Step 7: Implement `FIGHT_TRANS_LIST` and `advance_fight_state`**

Add to `src/game/gameplay_scene.rs` (near the top, above `apply_player_input`):

```rust
/// Attack animation transition table from fmain.c:132-140.
/// Each entry has 4 possible next states, selected by rand4().
/// States 0-8 represent weapon swing positions.
const FIGHT_TRANS_LIST: [[u8; 4]; 9] = [
    [1, 8, 0, 1], // 0: arm down, weapon low
    [2, 0, 1, 0], // 1: arm down, weapon diagonal down
    [3, 1, 2, 8], // 2: arm swing1, weapon horizontal
    [4, 2, 3, 7], // 3: arm swing2, weapon raised
    [5, 3, 4, 6], // 4: arm swing2, weapon diag up
    [6, 4, 5, 5], // 5: arm swing2, weapon high
    [8, 5, 6, 4], // 6: arm high, weapon up
    [8, 6, 7, 3], // 7: arm high, weapon horizontal
    [0, 6, 8, 2], // 8: arm middle, weapon raise fwd
];

/// Advance the fight animation state using trans_list random transitions.
/// `state`: current fight state (0-8). `tick`: game cycle for randomness.
fn advance_fight_state(state: u8, tick: u32) -> u8 {
    let idx = (state as usize).min(8);
    let col = crate::game::combat::rand4(tick);
    FIGHT_TRANS_LIST[idx][col]
}
```

- [ ] **Step 8: Run test to verify it passes**

Run: `cargo test fight_tests`
Expected: PASS

- [ ] **Step 9: Build full suite**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 10: Commit**

```bash
git add src/game/gameplay_scene.rs src/game/combat.rs
git commit -m "feat: implement trans_list fight animation state machine

Port the 9-state attack animation transition table from fmain.c.
Each frame, advance_fight_state() selects one of 4 possible next
states using rand4(), creating the varied weapon-swing animation
from the original game.

Part of #155"
```

---

## Task 4: Render fighting sprite frames + weapon overlay

**Files:**
- Modify: `src/game/gameplay_scene.rs` (hero rendering in `blit_actors_to_framebuf`, around line 3027)

**Closes:** #155 (part 3 of 3)

- [ ] **Step 1: Add `facing_to_fight_frame_base` helper**

Add to `src/game/gameplay_scene.rs` near `facing_to_frame_base`:

```rust
/// Map facing direction to fighting sprite frame base.
/// Based on diroffs[d+8] from fmain.c:1099, but diagonal directions
/// follow the Rust convention from facing_to_frame_base() (NE→east,
/// SE→south, SW→west, NW→north), not the original's grouping.
/// Frame ranges: southfight=32-43, westfight=44-55, northfight=56-67, eastfight=68-79.
fn facing_to_fight_frame_base(facing: u8) -> usize {
    match facing {
        0 => 56, // N  → northfight
        1 => 68, // NE → eastfight
        2 => 68, // E  → eastfight
        3 => 32, // SE → southfight
        4 => 32, // S  → southfight
        5 => 44, // SW → westfight
        6 => 44, // W  → westfight
        _ => 56, // NW → northfight
    }
}
```

- [ ] **Step 2: Update hero frame selection to use fighting frames**

In `blit_actors_to_framebuf`, replace the hero frame computation block:

```rust
let frame_base = Self::facing_to_frame_base(hero_facing);
// Walking: cycle through 8 frames; still: fmain.c uses diroffs[d]+1.
let anim_offset = if is_moving { (state.cycle as usize) % 8 } else { 1 };
let frame = frame_base + anim_offset;
```

With:

```rust
let hero_state = state.actors.first().map(|a| &a.state);
let frame = if let Some(ActorState::Fighting(fight_state)) = hero_state {
    // Fighting: use fight frame base + current animation state (0-8).
    let fight_base = Self::facing_to_fight_frame_base(hero_facing);
    fight_base + (*fight_state as usize).min(8)
} else {
    // Walking or still: existing logic.
    let frame_base = Self::facing_to_frame_base(hero_facing);
    if is_moving { frame_base + (state.cycle as usize) % 8 } else { frame_base + 1 }
};
```

- [ ] **Step 3: Build and run tests**

Run: `cargo test`
Expected: All tests pass. No compilation errors.

- [ ] **Step 4: Manual smoke test**

Run: `cargo run -- --debug --skip-intro`
Test:
1. Press numpad-0 while standing still → player should show weapon swing animation (not still pose).
2. Hold direction + numpad-0 → player should NOT move, but facing should change.
3. Release numpad-0 → player returns to normal walking/standing.
4. Fight near an enemy → melee combat should still trigger and deal damage.

- [ ] **Step 5: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "feat: render fighting sprite frames during attack animation

Select fighting sprite frames (32-79) from STATELIST when actor
is in Fighting state. Uses facing_to_fight_frame_base() to map
direction to the correct fight frame base (S=32, W=44, N=56, E=68),
then adds the trans_list animation state (0-8) as offset.

Weapon overlay rendering already works via STATELIST entries which
include wpn_no/wpn_x/wpn_y metadata for each fighting frame.

Closes: #155"
```

---

## Task 5: Integration test and cleanup

**Files:**
- Read-only: all modified files

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

- [ ] **Step 3: Manual integration test**

Run: `cargo run -- --debug --skip-intro`

Verify all behaviors together:
1. **NPC collision**: enemies navigate around walls, don't walk through buildings.
2. **NPC wall-sliding**: enemies slide along obstacles toward the hero.
3. **Fight suppresses movement**: pressing numpad-0 stops player movement.
4. **Fight direction change**: holding direction during fight changes facing.
5. **Fight animation**: weapon swing animation cycles visibly (not static pose).
6. **Melee combat**: enemies within reach take damage when fighting.
7. **Fight release**: releasing numpad-0 returns to normal movement.

- [ ] **Step 4: Final commit (if any cleanup needed)**

Only if clippy or integration testing revealed issues.
