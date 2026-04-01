# Bug Session 2026-04-01 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 6 bugs filed in the 2026-04-01 session (#138–#143), ranging from a trivial two-line weapon init fix to full environ/sinking system ports.

**Architecture:** Bugs are ordered by dependency — shared utilities first (`calc_dist`, `nearest_fig`), then consumers. Bug #141 (weapon) is standalone and trivial. Bug #138 (per-sprite masking) and #143 (environ) are the largest changes. Bug #139 (wall-sliding) is self-contained movement logic.

**Tech Stack:** Rust, SDL2, original C/ASM reference in `original/`

**Spec:** `docs/superpowers/specs/2026-04-01-bugs.md`

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src/game/collision.rs` | Modify | Add `calc_dist()`, `newx()`/`newy()` helpers |
| `src/game/game_state.rs` | Modify | Set `weapon=1` in init paths (#141); rewrite `pickup_world_object()` (#142); add `eat_amount()` |
| `src/game/gameplay_scene.rs` | Modify | Wall-sliding (#139); `nearest_fig()` + talk handlers (#140); Take handler (#142); per-sprite masking loop (#138); environ system (#143) |
| `src/game/sprite_mask.rs` | No change | `apply_sprite_mask()` is called per-sprite instead of batch — no API change needed |
| `src/game/world_objects.rs` | Modify | Add item display name lookup for container loot messages (#142) |

---

## Task 1: Hero starts without weapon (#141)

**Files:**
- Modify: `src/game/game_state.rs:508-533` (`init_first_brother`), `src/game/game_state.rs:451-456` (`activate_brother`)

This is a two-line fix. The original does `stuff[0] = an->weapon = 1` (`fmain.c:3501`). The Rust port sets `stuff[0] = 1` (dirk in inventory) but never sets `actors[0].weapon = 1`.

- [ ] **Step 1: Write test for weapon initialization**

In `src/game/game_state.rs`, add a test at the end of the `#[cfg(test)] mod tests` block:

```rust
#[test]
fn test_init_first_brother_equips_dirk() {
    let mut state = GameState::default();
    state.actors.push(crate::game::actor::Actor::default());
    state.init_first_brother(10, 10, 10, 100, 1000, 2000, 3);
    assert_eq!(state.stuff()[0], 1, "dirk should be in inventory");
    assert_eq!(state.actors[0].weapon, 1, "dirk should be equipped");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_init_first_brother_equips_dirk -- --nocapture`
Expected: FAIL — `actors[0].weapon` is 0.

- [ ] **Step 3: Add `weapon = 1` to `init_first_brother`**

In `src/game/game_state.rs`, in `init_first_brother()`, after the line `self.stuff_mut()[0] = 1;`, add:

```rust
        // Equip dirk (fmain.c:3501: stuff[0] = an->weapon = 1).
        if let Some(player) = self.actors.first_mut() {
            player.weapon = 1;
        }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_init_first_brother_equips_dirk -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/game/game_state.rs
git commit -m "fix: equip dirk on hero initialization (#141)

Set actors[0].weapon = 1 in init_first_brother(), mirroring
fmain.c:3501 (stuff[0] = an->weapon = 1).

Closes: #141

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 2: Add `calc_dist()` utility (#140, #142 shared)

**Files:**
- Modify: `src/game/collision.rs`

Port the octagonal distance approximation from `fmain2.c:446-463`. This is used by `nearest_fig` (bugs #140 and #142) and for Take range checking.

- [ ] **Step 1: Write tests for `calc_dist`**

Add at the end of `src/game/collision.rs`:

```rust
#[cfg(test)]
mod calc_dist_tests {
    use super::calc_dist;

    #[test]
    fn test_calc_dist_cardinal() {
        // Pure X distance: x > 2*y → return x
        assert_eq!(calc_dist(100, 0, 0, 0), 100);
        // Pure Y distance: y > 2*x → return y
        assert_eq!(calc_dist(0, 0, 0, 200), 200);
    }

    #[test]
    fn test_calc_dist_diagonal() {
        // Equal distances: (x+y)*5/7
        // x=70, y=70 → neither > 2*other → (70+70)*5/7 = 100
        assert_eq!(calc_dist(0, 0, 70, 70), 100);
    }

    #[test]
    fn test_calc_dist_asymmetric() {
        // x=10, y=30: y > 2*x → return y = 30
        assert_eq!(calc_dist(0, 0, 10, 30), 30);
        // x=30, y=10: x > 2*y → return x = 30
        assert_eq!(calc_dist(0, 0, 30, 10), 30);
        // x=20, y=15: neither > 2*other → (20+15)*5/7 = 25
        assert_eq!(calc_dist(0, 0, 20, 15), 25);
    }

    #[test]
    fn test_calc_dist_negative_coords() {
        // Uses absolute differences, so sign shouldn't matter
        assert_eq!(calc_dist(100, 200, 100, 200), 0);
        assert_eq!(calc_dist(50, 50, 100, 50), 50);
    }
}
```

- [ ] **Step 2: Write `calc_dist` function**

Add above the test module, after the existing `proxcheck` function:

```rust
/// Octagonal distance approximation from fmain2.c:446-463.
/// Used by nearest_fig() for NPC/object proximity checks.
/// Returns: if x > 2*y → x; if y > 2*x → y; else (x+y)*5/7.
pub fn calc_dist(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    let x = (ax - bx).abs();
    let y = (ay - by).abs();
    if x > y + y {
        x
    } else if y > x + x {
        y
    } else {
        (x + y) * 5 / 7
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test calc_dist_tests -- --nocapture`
Expected: All 4 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/game/collision.rs
git commit -m "feat: add calc_dist octagonal distance approximation

Port of fmain2.c:446-463 calc_dist(). Used by nearest_fig() for
proximity checks in Talk (#140) and Take (#142) handlers.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 3: Add `nearest_fig()` for NPC + setfig search (#140)

**Files:**
- Modify: `src/game/gameplay_scene.rs`

Port `nearest_fig(constraint, dist)` from `fmain2.c:426-442`. Searches BOTH `npc_table` (enemy NPCs) AND `world_objects` (ob_stat=3 setfigs). Returns index info about the nearest figure.

- [ ] **Step 1: Define the `NearestFig` return type and implement `nearest_fig`**

Near the existing `nearest_npc_in_range()` method (around line 755), add:

```rust
    /// What kind of figure was found by nearest_fig.
    enum FigKind {
        /// An enemy NPC from npc_table, with its index.
        Npc(usize),
        /// A setfig from world_objects, with its index and setfig type (ob_id).
        SetFig { world_idx: usize, setfig_type: u8 },
    }

    /// Result of nearest_fig search.
    struct NearestFig {
        kind: FigKind,
        dist: i32,
    }

    /// Port of nearest_fig(constraint, max_dist) from fmain2.c:426-442.
    /// constraint=0: find items (skip OBJECTS with index != 0x1d).
    /// constraint=1: find NPCs (skip OBJECTS entirely).
    /// Searches both npc_table and world_objects (setfigs).
    fn nearest_fig(&self, constraint: u8, max_dist: i32) -> Option<NearestFig> {
        use crate::game::collision::calc_dist;
        let hx = self.state.hero_x as i32;
        let hy = self.state.hero_y as i32;

        let mut best: Option<NearestFig> = None;
        let mut best_dist = max_dist;

        // Search enemy NPCs from npc_table
        if let Some(ref table) = self.npc_table {
            for (i, npc) in table.npcs.iter().enumerate() {
                if !npc.active { continue; }
                // constraint=0 skips OBJECTS-type entries (we don't have type tags on NPCs,
                // but NPC table entries are always actors, not objects — so include them).
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

        // Search world_objects for setfigs (ob_stat=3) and ground items (ob_stat=1)
        for (i, obj) in self.state.world_objects.iter().enumerate() {
            if !obj.visible { continue; }
            if obj.region != self.state.region_num { continue; }

            if constraint == 1 {
                // Looking for NPCs: skip ground items (OBJECTS), include setfigs
                if obj.ob_stat != 3 { continue; }
            } else {
                // Looking for items: skip setfigs, include ground items
                // Original: skip OBJECTS if constraint!=0 || index==0x1d (empty chest)
                if obj.ob_stat == 3 { continue; }
                if obj.ob_id == 0x1d { continue; } // empty chest
            }

            let d = calc_dist(hx, hy, obj.x as i32, obj.y as i32);
            if d < best_dist {
                best_dist = d;
                if obj.ob_stat == 3 {
                    best = Some(NearestFig {
                        kind: FigKind::SetFig { world_idx: i, setfig_type: obj.ob_id },
                        dist: d,
                    });
                } else {
                    best = Some(NearestFig {
                        kind: FigKind::Npc(i), // reuse Npc variant for ground items
                        dist: d,
                    });
                }
            }
        }

        best
    }
```

**Note:** The `FigKind::Npc` variant is reused for ground items when `constraint=0`. This mirrors the original where `nearest_fig(0,...)` returns whatever's nearest — items or NPCs. The Take handler (Task 6) will use `constraint=0`; the Talk handler uses `constraint=1`.

- [ ] **Step 2: Run build to check compilation**

Run: `cargo build 2>&1 | head -20`
Expected: Build succeeds (warnings about unused code are OK for now).

- [ ] **Step 3: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "feat: add nearest_fig() searching NPCs and setfigs

Port of fmain2.c:426-442. Searches both npc_table (enemies) and
world_objects (setfigs, ground items) using calc_dist. Returns
NearestFig with kind discriminant and distance.

Refs: #140, #142

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 4: Port setfig dialogue table for Ask/Say/Yell (#140)

**Files:**
- Modify: `src/game/gameplay_scene.rs` — rewrite Yell, Speak, and Ask handlers

Port the per-setfig dialogue table from `fmain.c:4188-4261`. 14 cases with side effects (priest heals, sorceress gives luck, etc.).

- [ ] **Step 1: Rewrite the Yell handler**

Replace the `GameAction::Yell` match arm with:

```rust
            GameAction::Yell => {
                // Yell: nearest_fig(1, 100). If NPC within 35 → speak(8) "No need to shout!"
                // Otherwise yell the next brother's name (fmain.c:4167-4175).
                let bname = brother_name(&self.state);
                if let Some(fig) = self.nearest_fig(1, 100) {
                    if fig.dist < 35 {
                        self.messages.push(crate::game::events::speak(&self.narr, 8, bname));
                    } else {
                        // NPC in yell range but not close — handle like speak
                        self.handle_setfig_talk(&fig, bname);
                    }
                } else {
                    let next_brother = match self.state.brother {
                        1 => "Phillip",
                        2 => "Kevin",
                        _ => "Julian",
                    };
                    self.messages.push(format!("{}!", next_brother));
                }
            }
```

- [ ] **Step 2: Rewrite the Speak/Ask handler**

Replace the `GameAction::Speak | GameAction::Ask` match arm with:

```rust
            GameAction::Speak | GameAction::Ask => {
                // Talk: nearest_fig(1, 50). Check shopkeeper first, then setfig dialogue.
                let bname = brother_name(&self.state);
                let hero_x = self.state.hero_x as i16;
                let hero_y = self.state.hero_y as i16;
                let near_shop = self.npc_table.as_ref().map_or(false, |t| {
                    crate::game::shop::has_shopkeeper_nearby(&t.npcs, hero_x, hero_y)
                });
                if near_shop {
                    // Shopkeeper buy menu (unchanged from existing code).
                    let items = [
                        (0,  "Food"),
                        (1,  "Arrows"),
                        (11, "Vial"),
                        (8,  "Mace"),
                        (10, "Sword"),
                        (9,  "Bow"),
                        (13, "Totem"),
                    ];
                    let mut menu = String::from("Shopkeeper: What do you need?\n");
                    for (idx, name) in &items {
                        let cost = crate::game::shop::ITEM_COSTS.get(*idx).copied().unwrap_or(0);
                        if cost > 0 {
                            menu.push_str(&format!("  {} - {} gold\n", name, cost));
                        }
                    }
                    menu.push_str(&format!("  (Your gold: {})", self.state.gold));
                    self.messages.push(menu);
                } else if let Some(fig) = self.nearest_fig(1, 50) {
                    self.handle_setfig_talk(&fig, bname);
                } else {
                    self.messages.push("There is no one here to talk to.");
                }
            }
```

- [ ] **Step 3: Add the `handle_setfig_talk` method**

Add a new method on `GameplayScene`:

```rust
    /// Handle dialogue with the nearest NPC/setfig. Ports fmain.c:4188-4261.
    fn handle_setfig_talk(&mut self, fig: &NearestFig, bname: &str) {
        match &fig.kind {
            FigKind::Npc(idx) => {
                // Enemy NPC — use race-based speech (existing logic).
                if let Some(ref table) = self.npc_table {
                    if let Some(npc) = table.npcs.get(*idx) {
                        use crate::game::npc::*;
                        let speech_id: usize = match npc.race {
                            RACE_NORMAL     => 3,
                            RACE_UNDEAD     => 2,
                            RACE_WRAITH     => 2,
                            RACE_ENEMY      => 1,
                            RACE_SNAKE      => 4,
                            RACE_SHOPKEEPER => 12,
                            RACE_BEGGAR     => 23,
                            _               => 6,
                        };
                        self.messages.push(crate::game::events::speak(&self.narr, speech_id, bname));
                    }
                }
            }
            FigKind::SetFig { setfig_type, .. } => {
                let k = *setfig_type as usize;
                // Per-setfig dialogue (fmain.c:4188-4261).
                match k {
                    0 => {
                        // Wizard: kind < 10 → speak(35), else speak(27 + goal).
                        // goal is not tracked yet; use speak(27) as default.
                        if self.state.kind < 10 {
                            self.messages.push(crate::game::events::speak(&self.narr, 35, bname));
                        } else {
                            self.messages.push(crate::game::events::speak(&self.narr, 27, bname));
                        }
                    }
                    1 => {
                        // Priest: heals hero. kind < 10 → speak(40), else speak(36 + daynight%3) + heal.
                        if self.state.kind < 10 {
                            self.messages.push(crate::game::events::speak(&self.narr, 40, bname));
                        } else {
                            let day_mod = (self.state.daynight % 3) as usize;
                            self.messages.push(crate::game::events::speak(&self.narr, 36 + day_mod, bname));
                            // Heal: vitality = 15 + brave/4 (fmain.c:4222).
                            self.state.vitality = 15 + self.state.brave / 4;
                        }
                    }
                    2 | 3 => {
                        // Guard: speak(15).
                        self.messages.push(crate::game::events::speak(&self.narr, 15, bname));
                    }
                    4 => {
                        // Princess: speak(16).
                        self.messages.push(crate::game::events::speak(&self.narr, 16, bname));
                    }
                    5 => {
                        // King: speak(17).
                        self.messages.push(crate::game::events::speak(&self.narr, 17, bname));
                    }
                    6 => {
                        // Noble: speak(20).
                        self.messages.push(crate::game::events::speak(&self.narr, 20, bname));
                    }
                    7 => {
                        // Sorceress: luck boost (fmain.c:4241-4247).
                        if self.state.luck < 64 {
                            self.state.luck += 5;
                        }
                        self.messages.push(crate::game::events::speak(&self.narr, 45, bname));
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
                        self.messages.push(crate::game::events::speak(&self.narr, speech, bname));
                    }
                    9 => {
                        // Witch: speak(46).
                        self.messages.push(crate::game::events::speak(&self.narr, 46, bname));
                    }
                    10 => {
                        // Spectre: speak(47).
                        self.messages.push(crate::game::events::speak(&self.narr, 47, bname));
                    }
                    11 => {
                        // Ghost: speak(49).
                        self.messages.push(crate::game::events::speak(&self.narr, 49, bname));
                    }
                    12 => {
                        // Ranger: region 2 → speak(22), else speak(53 + goal).
                        if self.state.region_num == 2 {
                            self.messages.push(crate::game::events::speak(&self.narr, 22, bname));
                        } else {
                            self.messages.push(crate::game::events::speak(&self.narr, 53, bname));
                        }
                    }
                    13 => {
                        // Beggar: speak(23).
                        self.messages.push(crate::game::events::speak(&self.narr, 23, bname));
                    }
                    _ => {
                        self.messages.push(crate::game::events::speak(&self.narr, 6, bname));
                    }
                }
            }
        }
    }
```

- [ ] **Step 4: Check for `dayperiod` field**

If `GameState` doesn't have a `dayperiod` field, use `self.state.daynight / 32` as a proxy (8 periods per day, 256 daynight ticks). Search: `grep -n dayperiod src/game/game_state.rs`. If missing, replace `self.state.dayperiod` with `(self.state.daynight / 32)`.

- [ ] **Step 5: Build and run tests**

Run: `cargo build && cargo test`
Expected: Build succeeds, all existing tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "feat: port setfig dialogue table for Ask/Say/Yell (#140)

Rewrite talk handlers to use nearest_fig() which searches both
npc_table and world_objects (setfigs). Port all 14 setfig dialogue
cases from fmain.c:4188-4261 including priest healing, sorceress
luck boost, bartender time-of-day greetings.

Closes: #140

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 5: Diagonal wall-sliding (#139)

**Files:**
- Modify: `src/game/collision.rs` — add `newx()`, `newy()` helpers
- Modify: `src/game/gameplay_scene.rs` — add deviation logic in `apply_player_input()`

Port the `checkdev1`/`checkdev2` direction deviation from `fmain.c:1824-1852`.

- [ ] **Step 1: Add `newx` and `newy` helpers to `collision.rs`**

These compute a new position given a direction and distance, using the original xdir/ydir tables. Add after `calc_dist`:

```rust
/// X displacement per direction. Mirrors xdir[] from fsubs.asm.
const XDIR: [i32; 8] = [0, 2, 3, 2, 0, -2, -3, -2];
/// Y displacement per direction. Mirrors ydir[] from fsubs.asm.
const YDIR: [i32; 8] = [-3, -2, 0, 2, 3, 2, 0, -2];

/// Compute new X from current + direction * distance (port of newx from fsubs.asm).
pub fn newx(x: u16, dir: u8, dist: i32) -> u16 {
    let dx = XDIR[(dir & 7) as usize] * dist / 2;
    ((x as i32 + dx).rem_euclid(0x8000)) as u16
}

/// Compute new Y from current + direction * distance (port of newy from fsubs.asm).
pub fn newy(y: u16, dir: u8, dist: i32, indoor: bool) -> u16 {
    let dy = YDIR[(dir & 7) as usize] * dist / 2;
    if indoor {
        (y as i32 + dy) as u16
    } else {
        ((y as i32 + dy).rem_euclid(0x8000)) as u16
    }
}
```

- [ ] **Step 2: Write tests for newx/newy**

```rust
#[cfg(test)]
mod newxy_tests {
    use super::{newx, newy};

    #[test]
    fn test_newx_cardinal() {
        // dir=2 (East), dist=2: dx = 3*2/2 = 3
        assert_eq!(newx(100, 2, 2), 103);
        // dir=6 (West), dist=2: dx = -3*2/2 = -3
        assert_eq!(newx(100, 6, 2), 97);
    }

    #[test]
    fn test_newy_cardinal() {
        // dir=0 (North), dist=2: dy = -3*2/2 = -3
        assert_eq!(newy(100, 0, 2, false), 97);
        // dir=4 (South), dist=2: dy = 3*2/2 = 3
        assert_eq!(newy(100, 4, 2, false), 103);
    }

    #[test]
    fn test_newx_diagonal() {
        // dir=1 (NE), dist=2: dx = 2*2/2 = 2
        assert_eq!(newx(100, 1, 2), 102);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test newxy_tests -- --nocapture`
Expected: PASS

- [ ] **Step 4: Add wall-sliding to `apply_player_input()`**

In `src/game/gameplay_scene.rs`, find the block around line 488 that does:

```rust
        if !turtle_blocked && (self.state.flying != 0 || self.state.on_raft || collision::proxcheck(self.map_world.as_ref(), new_x as i32, new_y as i32)) {
            self.state.hero_x = new_x;
            self.state.hero_y = new_y;
```

Replace it with direction deviation logic. The change wraps the blocked case with checkdev1/checkdev2:

```rust
        let mut final_x = new_x;
        let mut final_y = new_y;
        let mut final_facing = facing;
        let mut can_move = !turtle_blocked
            && (self.state.flying != 0 || self.state.on_raft
                || collision::proxcheck(self.map_world.as_ref(), new_x as i32, new_y as i32));

        // Direction deviation (wall-sliding): fmain.c checkdev1/checkdev2.
        // Only for diagonal directions when the original direction was blocked.
        if !can_move && !turtle_blocked && self.state.flying == 0 && !self.state.on_raft {
            let is_diagonal = matches!(dir, Direction::NE | Direction::SE | Direction::SW | Direction::NW);
            if is_diagonal {
                let indoor = self.state.region_num >= 8;
                // checkdev1: try (facing + 1) & 7
                let dev1 = (facing + 1) & 7;
                let dev1_x = collision::newx(self.state.hero_x, dev1, speed);
                let dev1_y = collision::newy(self.state.hero_y, dev1, speed, indoor);
                if collision::proxcheck(self.map_world.as_ref(), dev1_x as i32, dev1_y as i32) {
                    final_x = dev1_x;
                    final_y = dev1_y;
                    final_facing = dev1;
                    can_move = true;
                } else {
                    // checkdev2: try (dev1 - 2) & 7 = (facing - 1) & 7
                    let dev2 = (dev1.wrapping_sub(2)) & 7;
                    let dev2_x = collision::newx(self.state.hero_x, dev2, speed);
                    let dev2_y = collision::newy(self.state.hero_y, dev2, speed, indoor);
                    if collision::proxcheck(self.map_world.as_ref(), dev2_x as i32, dev2_y as i32) {
                        final_x = dev2_x;
                        final_y = dev2_y;
                        final_facing = dev2;
                        can_move = true;
                    }
                }
            }
        }

        if can_move {
            self.state.hero_x = final_x;
            self.state.hero_y = final_y;
```

The `facing` variable assignment after movement (around line 646) must use `final_facing` instead of converting from `dir`:

```rust
            let facing: u8 = final_facing;
```

Remove the old `let facing: u8 = match dir { ... }` block and replace it with the line above. The `facing` variable was already computed before movement.

**Important:** The `facing` variable is already computed at the top of the `if dir != Direction::None` block for the base deltas. You need to move the direction-to-facing conversion up before the proxcheck, so the deviation code can use it. Restructure as:

Before computing `new_x`/`new_y`, add:

```rust
            let facing: u8 = match dir {
                Direction::N  => 0, Direction::NE => 1, Direction::E  => 2, Direction::SE => 3,
                Direction::S  => 4, Direction::SW => 5, Direction::W  => 6, Direction::NW => 7,
                Direction::None => 0,
            };
```

Then after the movement block, replace the old facing conversion with:

```rust
            let facing = final_facing;
```

- [ ] **Step 5: Build and test**

Run: `cargo build && cargo test`
Expected: Build succeeds, all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/game/collision.rs src/game/gameplay_scene.rs
git commit -m "feat: diagonal movement slides along major axis when blocked (#139)

Port checkdev1/checkdev2 from fmain.c:1824-1852. When diagonal
movement is blocked, try (dir+1)&7 then (dir-1)&7 to slide along
the nearest cardinal axis. Updates facing to match the slide
direction.

Closes: #139

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 6: Port full Take handler with containers (#142)

**Files:**
- Modify: `src/game/world_objects.rs` — add `stuff_index_name()`
- Modify: `src/game/game_state.rs` — rewrite `pickup_world_object()`, add `eat_amount()`
- Modify: `src/game/gameplay_scene.rs` — rewrite `GameAction::Take` handler

- [ ] **Step 1: Add `stuff_index_name()` to `world_objects.rs`**

This returns the display name for a stuff[] index, used by container loot messages. Add after `stuff_index_to_ob_id()`:

```rust
/// Display name for a stuff[] inventory slot (used by container loot messages).
/// Matches inv_list[].name ordering from fmain.c:428.
pub fn stuff_index_name(idx: usize) -> &'static str {
    const NAMES: [&str; 31] = [
        "Dirk", "Mace", "Sword", "Bow", "Magic Wand", "Golden Lasso",
        "Sea Shell", "Sun Stone", "Arrows", "Blue Stone", "Green Jewel",
        "Glass Vial", "Crystal Orb", "Bird Totem", "Gold Ring", "Jade Skull",
        "Gold Key", "Green Key", "Blue Key", "Red Key", "Grey Key", "White Key",
        "Talisman", "Rose", "Fruit", "Gold Statue", "Book", "Herb", "Writ",
        "Bone", "Shard",
    ];
    NAMES.get(idx).copied().unwrap_or("an unknown thing")
}
```

- [ ] **Step 2: Add `eat_amount()` to `game_state.rs`**

The fruit item's special handling needs to reduce hunger by a specific amount. Add after `eat_food()`:

```rust
    /// Reduce hunger by a given amount (used by fruit pickup).
    /// Mirrors fmain.c eat(amount): hunger -= amount, clamped to 0.
    pub fn eat_amount(&mut self, amount: i16) {
        self.hunger = (self.hunger - amount).max(0);
    }
```

- [ ] **Step 3: Rewrite `pickup_world_object` in `game_state.rs`**

Replace the existing `pickup_world_object` method. The new version only handles the "mark as picked up" step — the caller (gameplay_scene.rs) handles special cases:

```rust
    /// Find the nearest visible ground item within range using calc_dist.
    /// Returns (world_objects index, ob_id) of the nearest item, or None.
    /// Does NOT modify state — caller decides what to do with the item.
    pub fn find_nearest_item(&self, region: u8, hero_x: u16, hero_y: u16, max_range: i32) -> Option<(usize, u8)> {
        use crate::game::collision::calc_dist;

        let hx = hero_x as i32;
        let hy = hero_y as i32;
        let mut best_idx = None;
        let mut best_dist = max_range;

        for (i, obj) in self.world_objects.iter().enumerate() {
            if obj.ob_stat == 3 { continue; } // setfigs not pickable
            if !obj.visible { continue; }
            if obj.region != region { continue; }
            if obj.ob_id == 0x1d { continue; } // empty chest (skip per original)

            let d = calc_dist(hx, hy, obj.x as i32, obj.y as i32);
            if d < best_dist {
                best_dist = d;
                best_idx = Some((i, obj.ob_id));
            }
        }
        best_idx
    }

    /// Mark a world object as picked up (ob_stat → hidden).
    pub fn mark_object_taken(&mut self, world_idx: usize) {
        if let Some(obj) = self.world_objects.get_mut(world_idx) {
            obj.visible = false;
        }
    }
```

Keep the old `pickup_world_object` for now (it may be called elsewhere), or rename it. Search for callers first: `grep -n pickup_world_object src/game/`. If only called from `GameAction::Take`, remove it.

- [ ] **Step 4: Rewrite the `GameAction::Take` handler**

Replace the `GameAction::Take` match arm:

```rust
            GameAction::Take => {
                // Take: nearest_fig(0, 30) — find nearest item within range 30 (fmain.c:3876-4000).
                const TAKE_RANGE: i32 = 30;
                if let Some((idx, ob_id)) = self.state.find_nearest_item(
                    self.state.region_num, self.state.hero_x, self.state.hero_y, TAKE_RANGE,
                ) {
                    let bname = brother_name(&self.state);
                    let taken = self.handle_take_item(idx, ob_id, bname);
                    if taken {
                        let wealth = self.state.wealth;
                        self.menu.set_options(self.state.stuff(), wealth);
                    }
                } else {
                    self.messages.push("Nothing here to take.");
                }
            }
```

- [ ] **Step 5: Add `handle_take_item` method**

```rust
    /// Handle taking a specific world item. Ports fmain.c:3880-4000.
    /// Returns true if the item was successfully taken.
    fn handle_take_item(&mut self, world_idx: usize, ob_id: u8, bname: &str) -> bool {
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
            // FRUIT (ob_id 148): eat if hungry, else add to inventory
            148 => {
                if self.state.hunger >= 15 {
                    // Hungry — eat immediately
                    self.state.eat_amount(30);
                    self.dlog(format!("ate fruit, hunger now {}", self.state.hunger));
                } else {
                    // Not hungry — add to inventory (stuff[24])
                    self.state.pickup_item(24);
                    let msg = crate::game::events::event_msg(&self.narr, 36, bname);
                    if !msg.is_empty() { self.messages.push(msg); }
                }
                self.state.mark_object_taken(world_idx);
                return true;
            }
            // BROTHER'S BONES (ob_id 28): combine saved brother's inventory
            28 => {
                self.messages.push(format!("{} found his brother's bones.", bname));
                // vitality byte encodes which brother: 1=Julian, 2=Phillip
                let obj_vit = self.state.world_objects.get(world_idx)
                    .map(|o| o.ob_stat) // we don't have vitality on WorldObject; skip for now
                    .unwrap_or(0);
                // TODO: combine julstuff/philstuff when WorldObject carries vitality field
                let _ = obj_vit;
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
                self.messages.push(format!("{} found {}.", bname, container_name));

                // rand4() determines loot: 0=nothing, 1=one item, 2=two items, 3=three of same
                let roll = (self.state.tick_counter & 3) as u8;
                match roll {
                    0 => {
                        self.messages.push("It was empty.".to_string());
                    }
                    1 => {
                        // One random item from inv_list[rand8()+8]
                        let item_idx = ((self.state.tick_counter >> 2) & 7) as usize + 8;
                        let item_idx = if item_idx == 8 { 35usize } else { item_idx }; // 8→ARROWBASE(35)
                        if item_idx < 35 {
                            self.state.pickup_item(item_idx);
                            self.messages.push(format!("Inside: a {}.", stuff_index_name(item_idx)));
                        }
                    }
                    2 => {
                        // Two different random items
                        let item1 = ((self.state.tick_counter >> 2) & 7) as usize + 8;
                        let item1 = if item1 == 8 { 35 } else { item1 };
                        let mut item2 = ((self.state.tick_counter >> 5) & 7) as usize + 8;
                        if item2 == item1 { item2 = ((item2 + 1) & 7) + 8; }
                        let item2 = if item2 == 8 { 35 } else { item2 };
                        if item1 < 35 { self.state.pickup_item(item1); }
                        if item2 < 35 { self.state.pickup_item(item2); }
                        let n1 = if item1 < 31 { stuff_index_name(item1) } else { "Arrows" };
                        let n2 = if item2 < 31 { stuff_index_name(item2) } else { "Arrows" };
                        self.messages.push(format!("Inside: a {} and a {}.", n1, n2));
                    }
                    3 | _ => {
                        // Three of the same item
                        let item = ((self.state.tick_counter >> 2) & 7) as usize + 8;
                        if item == 8 {
                            // Special: 3 random keys
                            self.messages.push("Inside: 3 keys.".to_string());
                            for shift in [4, 7, 10] {
                                let mut key_idx = ((self.state.tick_counter >> shift) & 7) as usize + 16; // KEYBASE
                                if key_idx == 22 { key_idx = 16; }
                                if key_idx == 23 { key_idx = 20; }
                                self.state.pickup_item(key_idx);
                            }
                        } else {
                            let name = if item < 31 { stuff_index_name(item) } else { "Arrows" };
                            self.messages.push(format!("Inside: 3 {}s.", name));
                            if item < 35 {
                                self.state.pickup_item(item);
                                self.state.pickup_item(item);
                                self.state.pickup_item(item);
                            }
                        }
                    }
                }

                self.state.mark_object_taken(world_idx);
                return true;
            }
            _ => {}
        }

        // Standard itrans pickup
        if let Some(stuff_idx) = ob_id_to_stuff_index(ob_id) {
            if self.state.pickup_item(stuff_idx) {
                let name = stuff_index_name(stuff_idx);
                let msg = crate::game::events::event_msg(&self.narr, 37, bname);
                if !msg.is_empty() {
                    self.messages.push(msg);
                } else {
                    self.messages.push(format!("{} found a {}.", bname, name));
                }
                self.state.mark_object_taken(world_idx);
                return true;
            }
        }

        false
    }
```

- [ ] **Step 6: Write tests for `stuff_index_name`**

In `src/game/world_objects.rs` test module:

```rust
    #[test]
    fn test_stuff_index_name() {
        assert_eq!(super::stuff_index_name(0), "Dirk");
        assert_eq!(super::stuff_index_name(2), "Sword");
        assert_eq!(super::stuff_index_name(8), "Arrows");
        assert_eq!(super::stuff_index_name(16), "Gold Key");
        assert_eq!(super::stuff_index_name(30), "Shard");
        assert_eq!(super::stuff_index_name(99), "an unknown thing");
    }
```

- [ ] **Step 7: Build and test**

Run: `cargo build && cargo test`
Expected: Build succeeds, all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/game/world_objects.rs src/game/game_state.rs src/game/gameplay_scene.rs
git commit -m "feat: port full Take handler with containers and special items (#142)

Replace pickup_world_object with find_nearest_item using calc_dist
range=30. Port container loot (rand4 rolls for URN/CHEST/SACKS),
scrap-of-paper events, fruit eat-when-hungry, and brother's bones
placeholder. Add stuff_index_name() for loot messages.

Closes: #142

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 7: Per-sprite masking (#138)

**Files:**
- Modify: `src/game/gameplay_scene.rs` — move `apply_sprite_mask` call inside the per-sprite render loop

The current code renders ALL sprites first, then masks ALL sprites in a separate loop. The fix moves masking into the per-sprite loop so each sprite is masked before the next one is drawn.

- [ ] **Step 1: Restructure the render loop**

In `src/game/gameplay_scene.rs`, find the masking pass (around line 3466):

```rust
                // Sprite-depth masking for all rendered sprites
                for sprite in &blitted {
                    apply_sprite_mask(mr, sprite, self.state.hero_sector, 0);
                }
```

Delete this batch masking loop. Instead, insert the masking call inside each `RenderKind` arm, immediately BEFORE the sprite blit. The pattern for each sprite is:

1. Compute sprite position and build a `BlittedSprite` struct
2. Call `apply_sprite_mask(mr, &sprite_info, hero_sector, actor_idx)` to stamp tile pixels over the framebuffer region where this sprite WILL go (clearing the "canvas" for it)
3. Blit the sprite pixels

**Important conceptual note:** In the original, the order is `mask_blit` then `shape_blit` — meaning the tile mask is applied first (restoring background pixels), and then the sprite is drawn on top. This means for the CURRENT sprite, mask-then-blit ensures the sprite appears correctly against terrain. For PREVIOUS sprites, their pixels may get overwritten by the current sprite's mask pass — but that's correct because Y-sorted order means sprites lower on screen should occlude sprites higher up.

Restructure the render loop body. For each `RenderKind` arm:

```rust
                        RenderKind::Hero => {
                            // ... compute rel_x, rel_y, frame ...
                            // Build BlittedSprite BEFORE blitting
                            let sprite_info = BlittedSprite {
                                screen_x: rel_x,
                                screen_y: rel_y,
                                width: SPRITE_W,
                                height: SPRITE_H,
                                ground: rel_y + SPRITE_H as i32,
                                is_falling: false,
                            };
                            // Mask THEN blit (per original: mask_blit → shape_blit)
                            apply_sprite_mask(mr, &sprite_info, self.state.hero_sector, 0);
                            if let Some(fp) = sheet.frame_pixels(frame) {
                                Self::blit_sprite_to_framebuf(fp, rel_x, rel_y, &mut mr.framebuf, fb_w, fb_h);
                            }
                            // Weapon overlay after hero blit
                            // ... weapon code unchanged ...

                            blitted.push(sprite_info);
                        }
```

Apply the same pattern (mask → blit → push) for `Enemy`, `SetFig`, and `WorldObj` arms.

- [ ] **Step 2: Fix `_actor_idx` — pass actual index**

In each `apply_sprite_mask` call, pass the correct actor index:
- Hero: `0`
- Enemy NPCs: their index from the NPC table (but since `is_actor_1` only matters for bridge sector raft logic, and enemies aren't the raft, `0` is fine)
- SetFigs: `0` (not the raft)
- WorldObj: `0` (not the raft)

The raft (actor index 1) isn't rendered through this system currently, so this is a future-proofing change. When raft rendering is added, pass `1` for the raft sprite.

- [ ] **Step 3: Remove the `blitted` vector if no longer needed**

After moving masking into the per-sprite loop, the `blitted` vector may only be needed for debug visualization. If nothing else uses it, remove it and the associated `push` calls to simplify the code.

- [ ] **Step 4: Build and test**

Run: `cargo build && cargo test`
Expected: Build succeeds, all tests pass.

- [ ] **Step 5: Manual test**

Run: `cargo run -- --debug --skip-intro`
Walk the hero near a setfig NPC. Verify:
- The hero is NOT overwritten by terrain when standing near the setfig
- Sprites still sort correctly by Y position
- Depth masking still works (trees/buildings occlude sprites behind them)

- [ ] **Step 6: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix: per-sprite masking prevents terrain overwriting hero near setfigs (#138)

Move apply_sprite_mask() call from batch post-pass into the
per-sprite render loop. Each sprite is now masked before blitting,
matching the original fmain.c pipeline: mask_blit → shape_blit
per sprite in Y-sorted order.

Closes: #138

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 8: Port environ sinking system (#143)

**Files:**
- Modify: `src/game/gameplay_scene.rs` — replace binary `submerged` with graduated `environ`

This is the largest change. Replace the binary `submerged` flag with the original's graduated `environ` system from `fmain.c:2019-2074`.

- [ ] **Step 1: Remove `submerged` and `drowning_timer` fields**

In `src/game/gameplay_scene.rs`, find and remove:
- Field declarations: `submerged: bool` (line 254), `drowning_timer: u32` (line 256)
- Field initializers: `submerged: false` (line 314), `drowning_timer: 0` (line 315)
- The water submersion check block (lines 742-752): remove entirely

Also remove the drowning damage block (lines 2914-2922):
```rust
        // Drowning damage (#105): 1 vitality per ~1s while submerged
        if self.submerged { ... }
```

- [ ] **Step 2: Add `fiery_death` field and environ update method**

Add a new field to `GameplayScene`:

```rust
    /// True when hero is in the volcanic region (lava damage active).
    /// Mirrors fiery_death global from fmain.c:1554.
    fiery_death: bool,
```

Initialize as `false` in the constructor.

Add a method that runs the sinker logic each tick:

```rust
    /// Update actor environ based on terrain type at current position.
    /// Port of fmain.c:2019-2074 sinker logic.
    fn update_environ(&mut self) {
        let terrain = if let Some(ref world) = self.map_world {
            collision::px_to_terrain_type(
                world, self.state.hero_x as i32, self.state.hero_y as i32,
            )
        } else {
            return;
        };

        if self.state.on_raft || self.state.flying != 0 {
            if let Some(player) = self.state.actors.first_mut() {
                player.environ = 0;
            }
            return;
        }

        let cur_environ = self.state.actors.first().map_or(0i8, |a| a.environ);
        let mut k: i8 = cur_environ;

        match terrain {
            0 => { k = 0; }
            6 => { k = -1; } // ice
            7 => { k = -2; } // lava
            8 => { k = -3; } // special C
            2 => { k = 2; }  // shallow water/wading
            3 => { k = 5; }  // brush/deep wade
            4 | 5 => {
                let threshold: i8 = if terrain == 4 { 10 } else { 30 };
                if k > threshold {
                    k -= 1;
                } else if k < threshold {
                    k += 1;
                    if k > 15 {
                        // Trigger SINK state
                        if let Some(player) = self.state.actors.first_mut() {
                            if !matches!(player.state, ActorState::Dying | ActorState::Dead) {
                                player.state = ActorState::Sinking;
                            }
                        }
                    }
                }
            }
            _ => {} // types 1, 9-15: no environ change from these
        }

        // Reset SINK state when leaving water
        if k == 0 {
            if let Some(player) = self.state.actors.first_mut() {
                if player.state == ActorState::Sinking {
                    player.state = ActorState::Still;
                }
            }
        }

        if let Some(player) = self.state.actors.first_mut() {
            player.environ = k;
        }
    }
```

- [ ] **Step 3: Add `fiery_death` region bounds check**

Add a method:

```rust
    /// Check if the hero is in the volcanic/lava region.
    /// Mirrors fmain.c:1554: fiery_death = (map_x > 8802 && map_x < 13562 && map_y > 24744 && map_y < 29544).
    fn update_fiery_death(&mut self) {
        let mx = self.state.hero_x as i32;
        let my = self.state.hero_y as i32;
        self.fiery_death = mx > 8802 && mx < 13562 && my > 24744 && my < 29544;
    }
```

- [ ] **Step 4: Add environ-based damage**

Add a method that replaces the old drowning damage:

```rust
    /// Apply environ-based damage: drowning at environ==30, lava in fiery_death region.
    /// Port of fmain.c:2131-2147.
    fn apply_environ_damage(&mut self) {
        let environ = self.state.actors.first().map_or(0i8, |a| a.environ);

        // Lava damage (fiery_death region, fmain.c:2133-2140)
        if self.fiery_death {
            // Rose (stuff[23]) grants fire immunity
            if self.state.stuff()[23] > 0 {
                if let Some(player) = self.state.actors.first_mut() {
                    player.environ = 0;
                }
            } else if environ > 15 {
                self.state.vitality = 0;
            } else if environ > 2 {
                self.state.vitality = (self.state.vitality - 1).max(0);
            }
        }

        // Drowning damage (fmain.c:2142-2146): environ==30 && (cycle & 7)==0
        if environ as i32 == 30 && (self.state.cycle & 7) == 0 {
            self.state.vitality = (self.state.vitality - 1).max(0);
        }
    }
```

- [ ] **Step 5: Call environ/damage methods from the tick**

In the main tick method (wherever `apply_player_input` is called from, and wherever the old drowning damage was), add:

```rust
        self.update_fiery_death();
        self.update_environ();
        self.apply_environ_damage();
```

Place these after `apply_player_input()` and after the old drowning damage block (which was removed in step 1).

- [ ] **Step 6: Update rendering Y-offset to use `environ`**

Find all instances of `if self.submerged { rel_y += 8; }` (lines 2575 and 3332) and replace with:

```rust
                                let environ = self.state.actors.first().map_or(0i8, |a| a.environ);
                                if environ > 29 {
                                    // Fully submerged — skip rendering body, show splash
                                    // TODO: render splash sprite (ob_id 97/98)
                                    continue;
                                } else if environ > 2 {
                                    rel_y += environ as i32;
                                }
```

For the `blit_actors_to_framebuf` path (line 2575), the same pattern applies.

**Note on weapon rendering:** In the original, when `environ > 29`, the weapon is hidden (goto offscreen). When `environ > 2`, the weapon also sinks:
```rust
                                // Weapon rendering with environ offset
                                if environ > 29 {
                                    // No weapon when fully submerged
                                } else if environ > 2 {
                                    wy += environ as i32;
                                    // Clip weapon bottom to ground line
                                }
```

- [ ] **Step 7: Clean up any remaining `self.submerged` references**

Search: `grep -n submerged src/game/gameplay_scene.rs`
Remove or replace any remaining references. The `hero_submerged` parameter in `blit_actors_to_framebuf` signature should be replaced with reading `environ` from `self.state.actors[0]`.

- [ ] **Step 8: Build and test**

Run: `cargo build && cargo test`
Expected: Build succeeds, all tests pass.

- [ ] **Step 9: Manual test**

Run: `cargo run -- --debug --skip-intro`
Walk through bushes and verify:
- Player does NOT take damage in bushes
- Player sprite dips slightly (environ=2 or 5, visible as small Y offset of ~2-5 pixels)
- Walking into water causes gradual sinking
- Walking out of water causes player to surface

- [ ] **Step 10: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix: replace binary submerged flag with graduated environ system (#143)

Port the sinker terrain→environ mapping from fmain.c:2019-2074.
Terrain types set environ: 0=open, 2=shallow, 5=brush, gradual
for types 4/5. Rendering uses environ as Y-offset (proportional
sinking). Drowning only at environ==30. Add fiery_death lava
damage with Rose immunity. Bushes no longer cause damage.

Closes: #143

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 9: Final validation

- [ ] **Step 1: Full build and test**

Run: `cargo build && cargo test`
Expected: Build succeeds, all tests pass.

- [ ] **Step 2: Check for compiler warnings**

Run: `cargo build 2>&1 | grep warning`
Fix any warnings related to unused imports, dead code from removed `submerged`/`drowning_timer`, or unused `pickup_world_object`.

- [ ] **Step 3: Remove dead code**

If `pickup_world_object()` in `game_state.rs` is no longer called (replaced by `find_nearest_item` + `handle_take_item`), remove it. Check: `grep -rn pickup_world_object src/`.

- [ ] **Step 4: Commit cleanup**

```bash
git add -A
git commit -m "chore: remove dead code from bug fix session

Remove unused pickup_world_object, submerged field, and
drowning_timer after environ system port.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```
