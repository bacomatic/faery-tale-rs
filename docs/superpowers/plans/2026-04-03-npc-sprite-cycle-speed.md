# NPC Sprite Animation Frame Fix — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix NPC sprite animation cycling to match the original game — state-gated frame selection with race-specific overrides.

**Architecture:** Extract a helper function `npc_animation_frame()` that computes the correct sprite frame index for any NPC based on its state, race, facing, and the game cycle counter. Both NPC render paths call this single function instead of duplicating inline `(cycle % 8)` logic.

**Tech Stack:** Rust, existing `npc::Npc`, `npc::NpcState`, `sprites::SpriteSheet`

**Bug:** [#161](https://github.com/bacomatic/faery-tale-rs/issues/161)

---

### Task 1: Add `npc_animation_frame()` helper with tests

**Files:**
- Modify: `src/game/gameplay_scene.rs` (add helper fn + tests)

The function must implement the original's frame selection logic from `fmain.c:2076–2108`:

| NPC state | Race | Frame formula (original) |
|-----------|------|--------------------------|
| Walking | default | `frame_base + ((cycle + npc_idx) & 7)` |
| Walking | snake (race 4) | `((cycle / 2) & 1) + frame_base` |
| Walking | wraith (race 2) | `frame_base` (no animation) |
| Still | default | `frame_base + 1` |
| Still | snake (race 4) | `(cycle & 1) + frame_base` |
| Dying/Dead | any | `frame_base` (static; dead NPCs deactivate quickly anyway) |

Note: dragon (race 8) uses a complex `(cycle & 3) * 2` formula with `i % 3` slot variations. The current dragon sprite sheet has only 5 frames and is handled by cfile 10 (NPC_TYPE_DRAGON). Since dragons are carrier NPCs rendered differently, we will not add their special formula here — the default walking case with wrapping handles them safely.

- [x] **Step 1: Write the helper function**

Add this method to `impl GameplayScene` (just before `blit_actors_to_framebuf`):

```rust
    /// Compute the sprite frame index for an NPC, matching fmain.c:2076–2108.
    /// `npc_idx` is the NPC's index in the table (provides phase offset like original `cycle + i`).
    /// Returns the frame index clamped to `num_frames`.
    fn npc_animation_frame(
        npc: &crate::game::npc::Npc,
        npc_idx: usize,
        cycle: u32,
        num_frames: usize,
    ) -> usize {
        use crate::game::npc::{NpcState, RACE_WRAITH, RACE_SNAKE};

        let frame_base = Self::facing_to_frame_base(npc.facing);

        let raw = match npc.state {
            NpcState::Walking => {
                if npc.race == RACE_WRAITH {
                    // Wraiths: no walk cycle (fmain.c:2079 — race 2 skips cycle offset)
                    frame_base
                } else if npc.race == RACE_SNAKE {
                    // Snakes walking: 2-frame, changes every 2 ticks (fmain.c:2081)
                    frame_base + ((cycle as usize / 2) & 1)
                } else {
                    // Default: 8-frame walk cycle with per-NPC phase offset (fmain.c:1863)
                    frame_base + ((cycle as usize + npc_idx) & 7)
                }
            }
            NpcState::Still => {
                if npc.race == RACE_SNAKE {
                    // Snakes still: slow 2-frame idle (fmain.c:2079)
                    frame_base + (cycle as usize & 1)
                } else {
                    // Default still: static frame (fmain.c:~1900 — diroffs[d] + 1)
                    frame_base + 1
                }
            }
            // Dying/Dead/Sinking/Fighting/Shooting: static base frame
            _ => frame_base,
        };

        raw % num_frames
    }
```

- [x] **Step 2: Write tests for the helper**

Add these tests to the existing `mod tests` block at the bottom of `gameplay_scene.rs`:

```rust
    #[test]
    fn test_npc_animation_frame_walking_default() {
        use crate::game::npc::{Npc, NpcState, NPC_TYPE_ORC, RACE_ENEMY};
        // Walking orc facing south (frame_base=0): should cycle through 0..7
        let npc = Npc {
            npc_type: NPC_TYPE_ORC, race: RACE_ENEMY,
            facing: 4, state: NpcState::Walking, active: true,
            ..Default::default()
        };
        // cycle=0, npc_idx=0 → frame_base + (0+0)&7 = 0
        assert_eq!(GameplayScene::npc_animation_frame(&npc, 0, 0, 64), 0);
        // cycle=3, npc_idx=2 → frame_base + (3+2)&7 = 5
        assert_eq!(GameplayScene::npc_animation_frame(&npc, 2, 3, 64), 5);
        // cycle=6, npc_idx=3 → frame_base + (6+3)&7 = 1 (wraps)
        assert_eq!(GameplayScene::npc_animation_frame(&npc, 3, 6, 64), 1);
    }

    #[test]
    fn test_npc_animation_frame_still_default() {
        use crate::game::npc::{Npc, NpcState, NPC_TYPE_ORC, RACE_ENEMY};
        // Still orc facing south (frame_base=0): always frame_base + 1 = 1
        let npc = Npc {
            npc_type: NPC_TYPE_ORC, race: RACE_ENEMY,
            facing: 4, state: NpcState::Still, active: true,
            ..Default::default()
        };
        assert_eq!(GameplayScene::npc_animation_frame(&npc, 0, 0, 64), 1);
        assert_eq!(GameplayScene::npc_animation_frame(&npc, 0, 99, 64), 1);
    }

    #[test]
    fn test_npc_animation_frame_wraith_no_cycle() {
        use crate::game::npc::{Npc, NpcState, NPC_TYPE_WRAITH, RACE_WRAITH};
        // Wraith walking: no walk cycle animation, always frame_base
        let npc = Npc {
            npc_type: NPC_TYPE_WRAITH, race: RACE_WRAITH,
            facing: 4, state: NpcState::Walking, active: true,
            ..Default::default()
        };
        assert_eq!(GameplayScene::npc_animation_frame(&npc, 0, 0, 64), 0);
        assert_eq!(GameplayScene::npc_animation_frame(&npc, 0, 50, 64), 0);
    }

    #[test]
    fn test_npc_animation_frame_snake_walking() {
        use crate::game::npc::{Npc, NpcState, NPC_TYPE_SNAKE, RACE_SNAKE};
        // Snake walking south: 2-frame cycle changing every 2 ticks
        let npc = Npc {
            npc_type: NPC_TYPE_SNAKE, race: RACE_SNAKE,
            facing: 4, state: NpcState::Walking, active: true,
            ..Default::default()
        };
        // cycle=0 → (0/2)&1 = 0
        assert_eq!(GameplayScene::npc_animation_frame(&npc, 0, 0, 64), 0);
        // cycle=1 → (1/2)&1 = 0
        assert_eq!(GameplayScene::npc_animation_frame(&npc, 0, 1, 64), 0);
        // cycle=2 → (2/2)&1 = 1
        assert_eq!(GameplayScene::npc_animation_frame(&npc, 0, 2, 64), 1);
        // cycle=3 → (3/2)&1 = 1
        assert_eq!(GameplayScene::npc_animation_frame(&npc, 0, 3, 64), 1);
    }

    #[test]
    fn test_npc_animation_frame_snake_still() {
        use crate::game::npc::{Npc, NpcState, NPC_TYPE_SNAKE, RACE_SNAKE};
        let npc = Npc {
            npc_type: NPC_TYPE_SNAKE, race: RACE_SNAKE,
            facing: 4, state: NpcState::Still, active: true,
            ..Default::default()
        };
        // cycle=0 → 0&1 = 0
        assert_eq!(GameplayScene::npc_animation_frame(&npc, 0, 0, 64), 0);
        // cycle=1 → 1&1 = 1
        assert_eq!(GameplayScene::npc_animation_frame(&npc, 0, 1, 64), 1);
    }

    #[test]
    fn test_npc_animation_frame_wraps_short_sheet() {
        use crate::game::npc::{Npc, NpcState, NPC_TYPE_ORC, RACE_ENEMY};
        // Sheet with only 5 frames — result must wrap
        let npc = Npc {
            npc_type: NPC_TYPE_ORC, race: RACE_ENEMY,
            facing: 4, state: NpcState::Walking, active: true,
            ..Default::default()
        };
        let frame = GameplayScene::npc_animation_frame(&npc, 0, 6, 5);
        assert!(frame < 5, "frame {} must be < num_frames 5", frame);
    }

    #[test]
    fn test_npc_animation_frame_dying() {
        use crate::game::npc::{Npc, NpcState, NPC_TYPE_ORC, RACE_ENEMY};
        // Dying NPC: static frame_base regardless of cycle
        let npc = Npc {
            npc_type: NPC_TYPE_ORC, race: RACE_ENEMY,
            facing: 4, state: NpcState::Dying, active: true,
            ..Default::default()
        };
        assert_eq!(GameplayScene::npc_animation_frame(&npc, 0, 0, 64), 0);
        assert_eq!(GameplayScene::npc_animation_frame(&npc, 0, 99, 64), 0);
    }
```

- [x] **Step 3: Run tests to verify they pass**

Run: `cargo test npc_animation_frame -- --nocapture`
Expected: all 7 new tests pass.

- [x] **Step 4: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "feat: add npc_animation_frame() helper with state/race-gated frame selection

Ports the original fmain.c:2076-2108 logic: walking NPCs get the 8-frame
walk cycle with per-NPC phase offset, still NPCs show a static frame,
wraiths skip animation, snakes use a slower 2-frame cycle.

Ref: #161"
```

---

### Task 2: Replace inline frame computation in both render paths

**Files:**
- Modify: `src/game/gameplay_scene.rs:~3492` (blit_actors_to_framebuf NPC loop)
- Modify: `src/game/gameplay_scene.rs:~4166` and `~4328` (z-sorted render Enemy path)

Both render paths currently compute:
```rust
let frame = ((frame_base % sheet.num_frames) + (state.cycle as usize % 8)) % sheet.num_frames;
```

Replace with a call to the new helper. This also requires:
1. Changing the `for npc in table.npcs.iter().filter(...)` to `for (npc_idx, npc) in table.npcs.iter().enumerate().filter(...)` to get the NPC index.
2. Using `npc.facing` instead of recomputing `npc_facing` from hero position (the NPC AI already sets `npc.facing` correctly in `update_actors`).

- [x] **Step 1: Update blit_actors_to_framebuf NPC loop (~line 3492)**

Replace the NPC rendering block inside `blit_actors_to_framebuf`. Change:

```rust
        if let Some(ref table) = npc_table {
            for npc in table.npcs.iter().filter(|n| n.active) {
                let Some(cfile_idx) = Self::npc_type_to_cfile(npc.npc_type, npc.race) else { continue };
                let Some(Some(ref sheet)) = sprite_sheets.get(cfile_idx) else { continue };

                let (rel_x, rel_y) = Self::actor_rel_pos(npc.x as u16, npc.y as u16, map_x, map_y);

                // Compute facing from NPC position relative to hero (NPCs always chase hero).
                let dx = state.hero_x as i32 - npc.x as i32;
                let dy = state.hero_y as i32 - npc.y as i32;
                let npc_facing = if dx.abs() >= dy.abs() {
                    if dx > 0 { 2u8 } else { 6u8 }  // E or W toward hero
                } else {
                    if dy > 0 { 4u8 } else { 0u8 }  // S or N toward hero
                };

                let frame_base = Self::facing_to_frame_base(npc_facing);
                // Wrap with sheet.num_frames to handle short sheets (e.g. dragon=5).
                let frame = ((frame_base % sheet.num_frames) + (state.cycle as usize % 8)) % sheet.num_frames;

                if let Some(fp) = sheet.frame_pixels(frame) {
                    Self::blit_sprite_to_framebuf(fp, rel_x, rel_y, crate::game::sprites::SPRITE_H, framebuf, fb_w, fb_h);
                }
            }
        }
```

To:

```rust
        if let Some(ref table) = npc_table {
            for (npc_idx, npc) in table.npcs.iter().enumerate().filter(|(_, n)| n.active) {
                let Some(cfile_idx) = Self::npc_type_to_cfile(npc.npc_type, npc.race) else { continue };
                let Some(Some(ref sheet)) = sprite_sheets.get(cfile_idx) else { continue };

                let (rel_x, rel_y) = Self::actor_rel_pos(npc.x as u16, npc.y as u16, map_x, map_y);
                let frame = Self::npc_animation_frame(npc, npc_idx, state.cycle, sheet.num_frames);

                if let Some(fp) = sheet.frame_pixels(frame) {
                    Self::blit_sprite_to_framebuf(fp, rel_x, rel_y, crate::game::sprites::SPRITE_H, framebuf, fb_w, fb_h);
                }
            }
        }
```

- [x] **Step 2: Update z-sorted render Enemy path (~line 4312)**

In the `RenderKind::Enemy(idx)` arm, change:

```rust
                                let dx = self.state.hero_x as i32 - npc.x as i32;
                                let dy = self.state.hero_y as i32 - npc.y as i32;
                                let npc_facing = if dx.abs() >= dy.abs() {
                                    if dx > 0 { 2u8 } else { 6u8 }
                                } else {
                                    if dy > 0 { 4u8 } else { 0u8 }
                                };

                                let frame_base = Self::facing_to_frame_base(npc_facing);
                                let frame = ((frame_base % sheet.num_frames) + (self.state.cycle as usize % 8)) % sheet.num_frames;
```

To:

```rust
                                let frame = Self::npc_animation_frame(npc, idx, self.state.cycle, sheet.num_frames);
```

- [x] **Step 3: Run all tests**

Run: `cargo test`
Expected: all tests pass, including `test_enemy_npc_render_pass_writes_pixels` (it uses `NpcState::default()` = `Still`, which now picks `frame_base + 1` instead of a cycle-animated frame — both are valid opaque frames in the mock sheet).

- [x] **Step 4: Compile check**

Run: `cargo build`
Expected: clean build, no warnings about unused variables (`npc_facing`, `dx`, `dy` removed).

- [x] **Step 5: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix: gate NPC sprite animation on state and race

Replace inline (cycle % 8) frame computation with npc_animation_frame()
in both render paths. Still NPCs now show a static frame, wraiths don't
animate, and snakes use a slower 2-frame cycle — matching the original
fmain.c behavior.

Closes: #161"
```

---

### Task 3: Update RESEARCH.md with NPC animation frame selection notes

**Files:**
- Modify: `RESEARCH.md`

- [x] **Step 1: Add NPC animation frame selection section**

Find the "Walk / still state transitions" section (~line 2019) and add after the "Frustration animation" section (~line 2043):

```markdown
### Enemy NPC animation frame selection (`fmain.c:2076–2108`)

After movement, the original overrides the animation index `dex` based on enemy race:

| Race | State | Frame formula | Notes |
|------|-------|--------------|-------|
| Default enemy | Walking | `diroffs[d] + ((cycle + i) & 7)` | 8-frame walk cycle; `i` = actor index provides per-NPC phase offset |
| Default enemy | Still | `diroffs[d] + 1` | Static standing frame |
| Wraith (race 2) | Walking | `diroffs[d]` | No walk cycle — wraiths glide |
| Snake (race 4) | Still (`< WALKING`) | `(cycle & 1) + diroffs[d]` | Slow 2-frame idle |
| Snake (race 4) | Walking (`< DYING`) | `((cycle / 2) & 1) + diroffs[d]` | 2-frame walk, changes every 2 ticks |
| Dragon (race 8) | Alive | `(cycle & 3) * 2` + slot variation via `i % 3` | Complex pattern with cfile offsets 0x25/0x28/0x30 |
| Dragon (race 8) | Dying | `0x3f` (frame 63) | Death frame |
| Dragon (race 8) | Dead | `abs_x = 0` (removed from screen) | Despawn |
| Skeleton (race 7, vit=0) | Dead | `1` | Collapsed frame |
```

- [x] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: document enemy NPC animation frame selection from fmain.c

Ref: #161"
```
