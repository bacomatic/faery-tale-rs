# Bug: NPC sprites cycle animation frames too fast — [#161](https://github.com/bacomatic/faery-tale-rs/issues/161)

**Filed:** 2026-04-03
**Status:** done

## Description

NPC sprites cycle through walk animation frames regardless of their actual state (Walking, Still, Dying, etc.), making them appear to animate way too fast — particularly noticeable when NPCs are standing still but their sprites cycle through all 8 walk frames at 30Hz (completing every 267ms).

## Investigation

- **Affected files:**
  - `src/game/gameplay_scene.rs` — lines ~3509 and ~4328 (both render paths)
- **Root cause:** confirmed — NPC rendering always uses `(state.cycle % 8)` to select the walk frame, without checking `npc.state`. The original game (`fmain.c:1900`) only animated NPCs in the `WALKING` state; still NPCs showed a static frame (`diroffs[d] + 1`).
- **Reproduction path:** Start the game, encounter any NPCs. Observe that all active NPC sprites rapidly cycle through walk frames even when standing still.
- **Evidence:**

  **Original code** (`fmain.c:1863`): only WALKING NPCs get animated frames:
  ```c
  dex = inum;
  if (!(riding && i == 0) && an->race != 2)
      dex += ((cycle + i) & 7);
  ```
  STILL NPCs get (`fmain.c:~1900`): `dex = inum + 1` (static frame).

  **Rust port** (gameplay_scene.rs:3509):
  ```rust
  let frame = ((frame_base % sheet.num_frames) + (state.cycle as usize % 8)) % sheet.num_frames;
  ```
  No state check — always animates.

  **Secondary issue**: Race-specific animation overrides are missing. The original has slower/different animation rates for specific enemy races (snakes use `((cycle/2)&1)`, dragons use `(cycle&3)*2`).

## Fix Design

**Approach C — Extract helper function**: Create `npc_animation_frame()` that encapsulates all original fmain.c frame selection logic (state-gating + race-specific overrides). Both render paths call it, eliminating duplication.

## Plan Reference

`docs/superpowers/plans/2026-04-03-npc-sprite-cycle-speed.md`
