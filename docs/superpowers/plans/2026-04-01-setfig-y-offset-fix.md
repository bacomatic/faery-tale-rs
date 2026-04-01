# SetFig Y-Offset Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the SetFig NPC Y-offset double-subtraction that renders all SetFig NPCs 18 pixels too high, hiding them behind ceiling tiles in indoor areas.

**Architecture:** Remove the extra `- 18` from the SetFig rendering branch in `gameplay_scene.rs`. The `actor_rel_pos()` function already applies a Y offset of -26, which matches the original game's two-step calculation (-8 base, -18 setfig adjustment). The current code applies both -26 and -18, totaling -44. Add a comment explaining why this differs from the original source structure.

**Tech Stack:** Rust, SDL2

**Closes:** #137
**Related:** #136

---

### Task 1: Fix SetFig Y-offset and add explanatory comment

**Files:**
- Modify: `src/game/gameplay_scene.rs:3445-3447`

- [ ] **Step 1: Apply the fix**

Replace the SetFig Y-position calculation at line 3445-3447:

```rust
// Before:
                                        // Original: ystart -= 18 in set_objects()
                                        let (rel_x, rel_y_base) = Self::actor_rel_pos(obj.x, obj.y, map_x, map_y);
                                        let rel_y = rel_y_base - 18;
```

```rust
// After:
                                        // Original does ystart = yc - map_y - 8; ystart -= 18 (total: -26).
                                        // actor_rel_pos already applies a Y offset of -26, matching that total,
                                        // so no further adjustment is needed here.
                                        let (rel_x, rel_y) = Self::actor_rel_pos(obj.x, obj.y, map_x, map_y);
```

- [ ] **Step 2: Build to verify no compile errors**

Run: `cargo build 2>&1 | tail -5`
Expected: `Finished` with no errors

- [ ] **Step 3: Run all tests**

Run: `cargo test 2>&1 | tail -5`
Expected: All 220+ tests pass, 0 failures

- [ ] **Step 4: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix: correct SetFig NPC Y-offset double-subtraction

actor_rel_pos() already applies a Y offset of -26, matching the
original's two-step calculation (-8 base, then -18 setfig adjustment).
The extra '- 18' produced a total of -44, rendering all SetFig NPCs
18 pixels too high — pushing them behind ceiling tiles in indoor areas
and making them invisible.

Closes: #137

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 2: Update bug spec status

**Files:**
- Modify: `docs/superpowers/specs/2026-04-01-bugs.md`

- [ ] **Step 1: Update bug #137 status and plan reference**

In `docs/superpowers/specs/2026-04-01-bugs.md`, update the Bug #137 section:

Change:
```markdown
**Status:** investigating
```
To:
```markdown
**Status:** planned
```

Change:
```markdown
### Plan Reference
_pending_
```
To:
```markdown
### Plan Reference
`docs/superpowers/plans/2026-04-01-setfig-y-offset-fix.md`
```

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/specs/2026-04-01-bugs.md
git commit -m "docs: update bug #137 status to planned with plan reference

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```
