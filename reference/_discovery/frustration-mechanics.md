# Discovery: Frustration Mechanics (Player & NPC)

**Status**: complete
**Investigated**: 2026-04-19
**Requested by**: orchestrator
**Prompt summary**: Trace the NPC frustration field — what it does, where it's set, what SHOOTFRUST is, and identify gaps in the authoritative reference docs.

## Overview

"Frustration" in FTA refers to two distinct mechanisms:

1. **Player frustration** — a global counter `frustflag` that tracks how long the player has been blocked, producing visual feedback animations.
2. **NPC frustration** — the tactical constant `FRUST` (value 0) assigned to an NPC's `tactic` field when movement is blocked, resolved on the next AI tick.

These are entirely separate systems that share only a conceptual name.

---

## 1. Player Frustration (`frustflag`)

### Declaration

- `fmain.c:589` — `char frustflag;` — comment: "is the character blocked ??"
- Global variable, applies ONLY to actor slot 0 (the player)

### When Incremented

- `fmain.c:1656` — `frustflag++;` — in the `blocked:` label within the WALKING/STILL state handler. Reached only for `i == 0` (player only); NPCs (`i != 0`) instead get `an->tactic = FRUST`.

### When Reset to 0

`frustflag` is reset to 0 inside the shared animation loop (`fmain.c:1468`, `for (i=0; i<anix; i++)`). The resets are **NOT guarded by `i == 0`**, meaning ANY actor's successful action resets the player's frustration counter:

| Location | Context | Actor scope |
|----------|---------|-------------|
| `fmain.c:1577` | SINK state entered | Any actor |
| `fmain.c:1650` | Successful WALKING movement (reached `newloc:` label) | Any actor |
| `fmain.c:1707` | Shooting states (after `dragshoot:` missile fired) | Any actor |
| `fmain.c:1715` | Melee combat states (state < 9, fight animation) | Any actor |
| `fmain.c:1725` | DYING state | Any actor |

**Gameplay implication**: During encounters, nearby enemies constantly walk/fight, resetting `frustflag` to 0 every tick. The escalating frustration animations (21+ and 41+ frames) are effectively only visible during solo exploration when no NPCs are active in `anim_list`.

### Visual Effects (Animation)

```c
// fmain.c:1656-1658
frustflag++;
if (frustflag > 40) dex = 40;
else if (frustflag > 20) dex = 84+((cycle>>1)&1);
```

| Threshold | `dex` value | Animation | Source |
|-----------|-------------|-----------|--------|
| 0–20 | `inum + 1` | Normal standing frame (direction-aware) | `fmain.c:1663` |
| 21–40 | `84 + ((cycle>>1)&1)` | Oscillation frames 84/85, alternating every 2 game cycles | `fmain.c:1658` |
| 41+ | `40` (hardcoded) | statelist[40] = south fight, transition state 8 ("arm middle, weapon raise fwd") | `fmain.c:1657` |

- **Indices 84–85**: Defined at `fmain.c:200-201` as "oscillations (sword at side??)" — figures 64 and 65. The `??` in Talin's comment suggests even he was uncertain about their purpose.
- **Index 40**: statelist[40] = `{ 35, 12, -5, 5 }` — figure 35 (a south-fight-direction frame). This is hardcoded regardless of the player's actual facing direction, so the character snaps to face south in a weapon-raised pose.

### Code Path

```
Player input → WALKING state → proxcheck() → terrain/actor collision
  → Try dir+1 (fmain.c:1613-1617)
  → Try dir-2 (fmain.c:1620-1624)
  → All blocked? → goto blocked: (fmain.c:1654)
    → i==0? → frustflag++ → animation selection → goto cpx
    → i!=0? → an->tactic = FRUST (NPC path)
```

---

## 2. NPC Frustration (Tactic Field)

### The `tactic` Field

Defined in `struct shape` at `ftale.h:63`:
```c
goal, tactic,    /* current goal mode and means to carry it out */
```

The `tactic` field is `char` (signed byte) and is **heavily overloaded** depending on actor state (see [§4 Tactic Field Overloading](#4-tactic-field-overloading)).

### FRUST Constant

- `ftale.h:42` / `fmain.c:122` — `#define FRUST 0` — comment: "all tactics frustrated - try something else"

### When Set

- `fmain.c:1661` — `else an->tactic = FRUST;` — for NPCs (`i != 0`) when movement is blocked and all deviation attempts fail.

### Resolution (AI Loop Handler)

- `fmain.c:2141-2143`:
```c
if (tactic == FRUST || tactic == SHOOTFRUST)
{   if (an->weapon & 4) do_tactic(i,rand4()+2);
    else do_tactic(i,rand2()+3);
}
```

This runs **before** goal-mode dispatch, meaning it applies to ALL goal modes (ATTACK, FLEE, FOLLOWER, CONFUSED, STAND, WAIT, etc.).

### Random Tactic Selection

| Actor Type | Expression | Range | Possible Tactics |
|------------|-----------|-------|-----------------|
| Ranged (weapon & 4) | `rand4()+2` | 2–5 | FOLLOW, BUMBLE_SEEK, RANDOM, BACKUP |
| Melee (weapon & 4 == 0) | `rand2()+3` | 3–4 | BUMBLE_SEEK, RANDOM |

### Cross-Cutting Finding: Affects ALL Goal Modes

The frustration handler at `fmain.c:2141` fires before the goal-mode `if/else if` chain that starts at `fmain.c:2144`. This means:
- A CONFUSED actor that gets blocked will BUMBLE_SEEK toward the player on the next tick
- A FLEEING actor that gets blocked will get BUMBLE_SEEK or RANDOM instead of BACKUP
- A FOLLOWER that gets blocked may get a direction that doesn't lead to the leader

This is likely unintentional — the frustration handler doesn't check `mode` before reassigning the tactic.

---

## 3. SHOOTFRUST: Dead Code

### Definition

- `fmain.c:131` / `ftale.h:51` — `#define SHOOTFRUST 9` — comment: "arrows not getting through"

### Evidence of Dead Code

**The constant is tested but NEVER assigned anywhere in the codebase.**

Exhaustive search of all `an->tactic` assignments:

| Location | Assignment | Value |
|----------|-----------|-------|
| `fmain.c:1661` | `an->tactic = FRUST` | 0 |
| `fmain.c:1770` | `an->tactic = 0` | 0 |
| `fmain.c:2773` | `an->tactic = 7` | 7 (death timer) |
| `fmain.c:3377` | `an->tactic = 15` | 15 (talk timer) |
| `fmain2.c:1668` | `an->tactic = tactic` (in do_tactic) | passed arg |
| `fmain2.c:1690` | `an->tactic = RANDOM` | 4 |

Arguments to `do_tactic()`:
- `rand4()+2` → 2–5
- `rand2()+3` → 3–4
- BACKUP → 5
- FOLLOW → 2
- Local `tactic` variable (from AI logic) → PURSUE(1), SHOOT(8), EVADE(6), EGG_SEEK(10), RANDOM(4), BACKUP(5)

**None of these can ever produce value 9 (SHOOTFRUST).**

The missile collision code (`fmain.c:2260-2300`) does NOT modify the archer's `tactic` field on miss. `dohit()` (`fmain2.c:230-247`) only modifies `vitality` and position, never `tactic`.

### Intended Purpose (Inferred from Comment)

The comment "arrows not getting through" suggests this was planned for a mechanic where:
1. An archer fires at the player
2. Arrows consistently miss or are blocked
3. The archer's tactic would be set to SHOOTFRUST
4. The frustration handler would then reassign a new tactic (move closer, etc.)

This mechanic was never implemented — the code to detect "arrows not getting through" and set SHOOTFRUST does not exist.

### Relationship to P11 (PROBLEMS.md)

PROBLEMS.md P11 documents HIDE(7), DOOR_SEEK(11), DOOR_LET(12) as unused tactics. SHOOTFRUST(9) is also unused but is **not mentioned in P11**. Unlike HIDE/DOOR_SEEK/DOOR_LET, SHOOTFRUST does have a handler in the code (the FRUST check at line 2141) — it's just that nothing ever triggers it.

---

## 4. Tactic Field Overloading

The `tactic` field serves completely different purposes depending on actor state:

| Actor State | Purpose of `tactic` | Values | Lines |
|-------------|---------------------|--------|-------|
| Combat AI (WALKING/FIGHTING) | Tactical sub-goal | 0–10 constants | `fmain.c:2121` |
| TALKING (SETFIG) | Speech animation timer (countdown to 0) | Set to 15, decrements | `fmain.c:3377, 1557` |
| DYING | Death animation timer (countdown to 0) | Set to 7, decrements | `fmain.c:2773, 1720-1722, 1747` |
| FALL | Fall animation frame counter (counts up to 30) | 0→30 | `fmain.c:1770, 1733-1736` |
| Rendering decision | Type selector: < 16 → ENEMY render, ≥ 16 → OBJECTS render | Any | `fmain.c:2457` |

### Critical Interaction: Timer Values vs. Tactic Constants

- DYING tactic starts at 7 = HIDE (an unused tactic). If an actor died while in the AI loop visible range, the first tick checks `tactic == 7`. Since mode is DEATH, the code reaches `checkdead()` before the AI loop processes it again, so no conflict occurs in practice.
- TALKING tactic = 15 is above all combat tactic constants (max is DOOR_LET = 12). This safely avoids triggering any tactic code.
- FALL tactic counts 0→30. At tactic=0, FRUST would match — but FALL actors have `state == FALL` and are handled in the state machine BEFORE the AI loop runs (the AI loop processes actors 2+ at `fmain.c:2110`, but falling is resolved during the animation tick at `fmain.c:1733`).

---

## 5. Initialization Bug in `set_encounter()`

`set_encounter()` at `fmain.c:2736-2770` initializes a new enemy actor but does NOT set `an->tactic`:

```c
// Fields set:
an->abs_x = xtest;
an->abs_y = ytest;
an->type = ENEMY;
an->race = race;
an->weapon = weapon_probs[w];
an->state = STILL;
an->environ = an->facing = 0;
an->goal = ATTACK1/ATTACK2/ARCHER1/ARCHER2;
an->vitality = ...;
// NOT set: an->tactic (retains stale value from previous actor in slot)
```

Since `anim_list[]` is a static array, `tactic` retains whatever was last stored for that slot. If the previous occupant died (`tactic` was counting down from 7) or was talking (`tactic = 15`), the new actor inherits that value.

**Impact**: On the first AI tick, if the stale tactic is 0 (FRUST), the frustration handler fires immediately, giving the actor a random direction. For any other value, the normal tactic assignment logic proceeds. This is a minor cosmetic bug — the actor might briefly face an unexpected direction on its first frame.

---

## Unresolved

- **Visual identity of frustration animations**: What do statelist entries 40, 84, 85 actually look like? The code references are clear, but without rendering the actual sprite assets (`game/p1a`, `p1b`, etc.), the visual appearance can only be described by Talin's comments: "oscillations (sword at side??)" for 84/85, and a south-fight pose for index 40. Cannot determine definitively from source code. 

---

## Gaps Found in Authoritative Documentation

### RESEARCH.md

1. **§5.4 / §6.3 Player Collision Deviation**: Describes `frustflag` but doesn't document the full threshold chain (> 20 vs > 40) with sprite index details. Only says "scratching-head animation" and "special animation index 40" without explaining what these correspond to in `statelist[]`.

2. **§8.4 Frustration Cycle**: Does not mention that the frustration handler applies to ALL goal modes (cross-cutting finding). Doesn't note that FLEE actors getting frustrated switch to BUMBLE_SEEK/RANDOM instead of BACKUP.

3. **§8.4 Frustration Cycle**: Does not mention the asymmetry between ranged and melee NPC escape tactics (ranged get 4 options, melee only 2).

4. **§8.4 Frustration Cycle**: Does not mention SHOOTFRUST as dead code or explain its presence in the test at line 2141.

5. **§2.1 (struct shape) or §8**: No comprehensive documentation of the `tactic` field's overloaded nature. The DYING timer usage is mentioned in §7.7, but a unified table showing all overloaded uses is missing.

6. **set_encounter() tactic initialization**: Not documented anywhere. The encounter spawning section doesn't note the missing `tactic` initialization.

### PROBLEMS.md

7. **P11 incomplete**: Lists HIDE(7), DOOR_SEEK(11), DOOR_LET(12) as unused tactics but omits SHOOTFRUST(9), which is also unused (defined and tested but never assigned).

---

## Cross-Cutting Findings

- `fmain.c:1661` — The `FRUST` assignment is in the **movement/animation subsystem**, not the AI subsystem. The blocked check (`goto blocked:`) is part of the WALKING state handler, which crosses from movement into AI territory.
- `fmain.c:2141-2143` — Frustration handler runs **before** goal-mode dispatch, affecting FLEE, FOLLOWER, CONFUSED actors in addition to hostile ones.
- `fmain.c:2736-2770` — `set_encounter()` doesn't initialize `tactic`, allowing stale values to persist (crosses encounter spawning with AI state).
- `fmain.c:2457` — The rendering subsystem uses `tactic < 16` as a type discriminator for FALL state actors, crossing rendering with AI state.

---

## Refinement Log

- 2026-04-19: Initial comprehensive discovery pass. Traced all references to frustflag, FRUST, SHOOTFRUST across entire codebase. Identified SHOOTFRUST as dead code. Identified 7 documentation gaps.
