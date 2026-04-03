# NPC Goal/Tactic AI System

**Date:** 2026-04-02
**Status:** Approved

## Problem

Enemy NPCs feel too fast and relentless. Root cause: the current `Npc::tick()` re-aims perfectly toward the hero every frame (30 Hz), with no tactic variety. The original game's AI re-aims only ~12.5% of frames via a probabilistic gate, combined with 10+ distinct tactical behaviors (pursue, evade, backup, shoot, random wander, etc.). The net effect is NPCs that feel drunk-walking and organic rather than laser-tracking.

The Goal/Tactic enums exist in `actor.rs` but are dead code — never set, never evaluated. The `do_tactic()` and `set_course()` functions from the original C source are not ported.

## Approach

Add AI fields to the `Npc` struct. Create a new `npc_ai.rs` module with three pure functions (`set_course`, `do_tactic`, `select_tactic`) that port the original AI pipeline. Rewire `update_actors()` in `gameplay_scene.rs` to run the AI pipeline before movement execution. Keep the `Actor` array for rendering only, synced from NPC positions each frame.

## Design

### Npc struct changes (`npc.rs`)

Add fields to `Npc`:

| Field | Type | Purpose |
|-------|------|---------|
| `goal` | `Goal` | High-level AI mode (Attack1, Archer2, Flee, etc.) |
| `tactic` | `Tactic` | Current sub-behavior (Pursue, Shoot, Evade, etc.) |
| `facing` | `u8` | 0–7 compass direction, persists between ticks |
| `state` | `NpcState` | Walking, Still, Fighting, Dead, Shooting, Dying, Sinking |
| `cleverness` | `u8` | From encounter chart — determines goal assignment |

New `NpcState` enum (lightweight, distinct from `ActorState` which carries animation data):
```rust
pub enum NpcState { Still, Walking, Fighting, Dead, Shooting, Dying, Sinking }
```

`Goal` and `Tactic` reuse the existing enums from `actor.rs`.

### AI module (`npc_ai.rs`) — new file

Three public functions that operate on `&mut Npc` with no dependency on SDL2 or `GameplayScene`:

#### `set_course(npc, target_x, target_y, mode)`

Computes `npc.facing` and `npc.state` from position delta. Ports `fmain2.c` `set_course()`.

- **Axis suppression** (modes 0–3): if one axis distance > 2x the other, zero the minor axis
- **Mode 0** — smart seek (default pursuit)
- **Mode 1** — + deviation ±1 when dist < 40
- **Mode 2** — + deviation ±2 when dist < 30
- **Mode 3** — flee (negate direction signs)
- **Mode 4** — bumble (skip axis suppression, allow true diagonals)
- **Mode 5** — aim only (set facing, do NOT set state to Walking)
- **Mode 6** — direct (target_x/y are raw delta, not world position)
- **`com2` lookup**: `[[7,0,1],[6,9,2],[5,4,3]]` maps `(xsign, ysign)` to facing; result 9 → Still
- **Deviation**: 50/50 chance add or subtract from facing, masked to 0–7

#### `do_tactic(npc, tactic, hero_x, hero_y, leader_idx, npcs, tick)`

Ports `fmain2.c:2075`. Gates `set_course()` behind `!(rand & 7)` (~12.5% probability). For `goal == Attack2`, upgrades to `!(rand & 3)` (~25%).

| Tactic | Action on trigger | set_course mode |
|--------|-------------------|----------------|
| Pursue | aim at hero | 0 |
| Shoot | 50% fire if aligned on cardinal/diagonal axis; else aim at hero | 5 or 0 |
| Random | random facing 0–7, state=Walking | N/A (no set_course) |
| BumbleSeek | aim at hero | 4 |
| Backup | flee from hero | 3 |
| Follow | aim at leader NPC + 20px Y offset | 0 |
| Evade | aim at neighboring NPC + 20px Y | 2 |
| EggSeek | aim at fixed coords (23087, 5667), state=Walking | 0 |

Shoot alignment check (from original): `xd < 8 || yd < 8 || (xd > (yd-5) && xd < (yd+7))`. If not aligned, falls through to pursuit — this makes archers maneuver into cardinal/diagonal firing lanes.

#### `select_tactic(npc, hero_x, hero_y, hero_state, leader_idx, xtype, tick)`

Ports the tactic decision tree from `fmain.c:2500–2595`. Runs once per tick per NPC.

**Goal overrides** (checked first):
- Hero dead/falling → Flee or Follower (based on leader assignment)
- Vitality < 2 → Flee
- `xtype > 59` and race != special → Flee

**Recalculation gate `r`:**
- Default: ~6.25% (1-in-16)
- For Attack1/Archer1: ~25% (1-in-4)
- For Attack2/Archer2: stays at 6.25%

**Tactic decision** (only when `r` triggers):
1. Snake + turtle_eggs → EggSeek
2. No weapon → mode=Confused, tactic=Random
3. Low vitality (<6) + 50% → Evade
4. Archer + too close (xd<40, yd<30) → Backup
5. Archer + in range (xd<70, yd<70) → Shoot
6. Archer + far → Pursue
7. Default melee → Pursue

**Close-range melee** (separate from tactic tree):
```
thresh = 14 - goal_value  // Attack1→14, Attack2→13, Archer1→11, Archer2→10
if race == 7 (DKnight): thresh = 16
if melee weapon && xd < thresh && yd < thresh:
    set_course(direct) + state = Fighting   // bypasses do_tactic
```

**Non-hostile modes:**
- Flee → `do_tactic(Backup)`
- Follower → `do_tactic(Follow)`
- Stand → `set_course(direct)` then state=Still
- Wait → state=Still

### Movement execution — `Npc::tick()` refactor

Stop re-aiming every tick. `tick()` becomes a dumb executor: walk in current `facing` direction at `dist=2`, with collision + wall-slide (±1 deviation). Only runs when `npc.state == Walking`.

If all three directions blocked → state=Still, tactic=Frust.

The AI layer (`select_tactic` → `do_tactic` → `set_course`) sets `facing` and `state` before `tick()` runs.

### `update_actors()` rewrite (`gameplay_scene.rs`)

```
fn update_actors():
    // 1. AI decision pass
    for each active npc in npc_table:
        select_tactic(npc, hero, hero_state, leader, xtype, tick)
        do_tactic(npc, npc.tactic, hero, leader, npcs, tick)

    // 2. Movement execution pass
    for each active npc in npc_table:
        if npc.state == Walking:
            npc.tick(world, indoor)

    // 3. Encounter adjacency check (existing)
    // 4. Archer missile firing (from NPC tactic==Shoot + NpcState::Shooting)
    // 5. Mirror NPC positions → Actor array for rendering
```

### Actor array sync

After movement, sync NPC state into Actor array for rendering:
```
actor.abs_x = npc.x
actor.abs_y = npc.y
actor.facing = npc.facing
actor.state = map NpcState → ActorState
actor.moving = npc.state == Walking
```

### Leader tracking

First live hostile NPC becomes `leader`, reset each frame. FOLLOW tactic targets leader position + 20px Y offset. Self-follow → switch to RANDOM.

### Battleflag

Two-frame persistence: `battle2 = battleflag; battleflag = false;` then re-set if any NPC within 300px is visible or was fighting last frame. Reuse existing `actors_on_screen` field.

### Encounter spawning changes (`encounter.rs`)

Add `cleverness` to encounter chart. Initialize goal/tactic on spawn:
- `weapon & 4` (bow/wand) → Archer1/Archer2 based on cleverness
- else → Attack1/Attack2 based on cleverness
- Initial tactic: Pursue
- Initial state: Walking
- Initial facing: toward hero

Cleverness values: Ogre=0, Orc=0, Wraith=1, Skeleton=0, Snake=0, Salamander=0, Spider=0, DKnight=1, Loraii=1, Necromancer=1, Woodcutter=0.

## Files touched

| File | Change |
|------|--------|
| `src/game/npc.rs` | Add goal/tactic/facing/state/cleverness fields; refactor `tick()` to use stored facing |
| `src/game/npc_ai.rs` | **New.** `set_course()`, `do_tactic()`, `select_tactic()` |
| `src/game/encounter.rs` | Add cleverness to chart; initialize goal/tactic/state on spawn |
| `src/game/gameplay_scene.rs` | Rewrite `update_actors()` to call AI pipeline then movement |
| `src/game/mod.rs` | Add `pub mod npc_ai;` |
| `src/game/actor.rs` | No change (Goal/Tactic enums already exist, reused) |
| `src/game/collision.rs` | No change (newx/newy/proxcheck reused) |

## Testing

All AI functions are pure (take `&mut Npc`, coordinates, tick counter). Testable without SDL2.

**set_course tests:** per-mode facing verification, axis suppression, flee reversal, deviation bounds
**do_tactic tests:** probabilistic gate frequency (~12.5%), each tactic branch calls correct mode
**select_tactic tests:** goal overrides, archer range brackets, close-range melee transition, FRUST handling
**Integration test:** spawn NPCs, run 100 ticks, verify tactic diversity (not all on same pixel)
