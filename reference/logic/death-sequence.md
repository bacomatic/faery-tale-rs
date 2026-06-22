# Death Sequence Timing — Logic Spec

> Fidelity: behavioral  |  Source files: `fmain.c` (animation timing), `fmain2.c` (`fallstates[]`)
> Cross-refs: [game-loop.md#resolve_player_state](game-loop.md#resolve_player_state), [game-loop.md#death_step](game-loop.md#death_step), [brother-succession.md#revive](brother-succession.md#revive), [actor-animation-catalog.md](../data/sprites/actor-animation-catalog.md)

## Overview

This document is the clock-tick reference for what happens after the hero loses all vitality. The sequence has three tightly coupled parts:

1. **The death animation** (`STATE_DYING` → `STATE_DEAD`) — 8 visible ticks.
2. **The `goodfairy` countdown** — 255 ticks that decide rescue vs. brother succession.
3. **The fairy approach animation** — 100 visible ticks within the countdown, only when the hero is rescued.

The high-level state machine lives in `resolve_player_state` (phase 7 of the game loop). The animation frames are selected in `death_step` (phase 9). The `goodfairy` counter is an `unsigned char` initialized to `0` by `revive()` and only decremented while the hero is in `STATE_DEAD` or `STATE_FALL`.

---

## State Diagram

```
                ┌─────────────┐
   damage ─────▶│  STATE_DYING  │◀──── tactic = 7, set by checkdead()
   vitality=0   │  (7 ticks)    │
                └──────┬──────┘
                       │
                       │ tick 8: tactic decrements to 0
                       ▼
                ┌─────────────┐
                │  STATE_DEAD   │
                │  (frame 82)   │
                └──────┬──────┘
                       │
                       │ resolve_player_state() starts goodfairy
                       │ countdown (0 → 255 → 254 → ...)
                       ▼
        ┌──────────────────────────────┐
        │  goodfairy reaches 199 (tick 57) │
        └──────────────┬───────────────┘
                       │
         ┌─────────────┼─────────────┐
         │             │             │
         ▼             ▼             ▼
   luck < 1      state == FALL   luck >= 1 and state == DEAD
         │             │             │
         ▼             ▼             ▼
   revive(TRUE)   revive(FALSE)   continue countdown
   (next brother) (fall recovery)
                       │
                       ▼
              goodfairy reaches 119 (tick 137)
                       │
                       ▼
              ┌─────────────────┐
              │  fairy sprite   │
              │  appears        │
              │  (100 ticks)    │
              └────────┬────────┘
                       │
                       ▼
              goodfairy reaches 19 (tick 237)
                       │
                       ▼
              ┌─────────────────┐
              │  fairy off      │
              │  (18-tick pause)│
              │  corpse remains │
              └────────┬────────┘
                       │
                       ▼
              goodfairy reaches 1 (tick 255)
                       │
                       ▼
                 revive(FALSE)
              (fairy rescue, same brother)
```

---

## Player Death Animation (STATE_DYING)

`checkdead()` (`fmain.c:2769`) is called the moment the hero's vitality drops below 1. It sets:

- `anim_list[0].vitality = 0`
- `anim_list[0].state = DYING`
- `anim_list[0].tactic = 7`  (reused as the death-animation countdown)
- `anim_list[0].goal = DEATH`
- `luck -= 5` (clamped at 0 if it goes negative)

The actual animation runs in `death_step` (`fmain.c:1719-1726`) and the post-step decrement at `fmain.c:1747-1757`.

### Tick table

Tick counting starts at the first game loop frame after `checkdead()` fires.

| Tick | `tactic` before | `tactic` after | State rendered | `an->index` | Physical frame (PHIL) | Notes |
|------|-----------------|----------------|----------------|-------------|-----------------------|-------|
| 1 | 7 | 6 | `DYING` | 80 or 81 | 47 or 63 | Frame A (facing 0/5/6/7 → 80; facing 1/2/3/4 → 81) |
| 2 | 6 | 5 | `DYING` | 80 or 81 | 47 or 63 | Frame A continues |
| 3 | 5 | 4 | `DYING` | 80 or 81 | 47 or 63 | Frame A ends |
| 4 | 4 | 3 | `DYING` | 81 or 80 | 63 or 47 | Frame B (flipped from A) |
| 5 | 3 | 2 | `DYING` | 81 or 80 | 63 or 47 | Frame B continues |
| 6 | 2 | 1 | `DYING` | 81 or 80 | 63 or 47 | Frame B continues |
| 7 | 1 | 0 | `DYING` | 81 or 80 | 63 or 47 | Frame B ends; post-step sets state to `DEAD` |
| 8 | — | — | `DEAD` | 82 | 39 | Corpse frame; `goodfairy` countdown begins this tick |

### Notes

- The `facing` half (`d == 0` or `d > 4` vs. `d == 1..4`) only swaps which of frames 80/81 is shown for the two phases; the timing is identical for every facing.
- The decrement is `!(--an->tactic)` — it is evaluated at the end of the tick, so the visible transition to `DEAD` happens on the **next** frame.
- Frame 82 is the final corpse frame; the hero stays in `STATE_DEAD` for the rest of the `goodfairy` countdown.

---

## `goodfairy` Countdown Phases

`goodfairy` is an `unsigned char` set to `0` whenever `revive()` runs. It only decrements in `resolve_player_state` when the hero is in `STATE_DEAD` or `STATE_FALL`. The first decrement wraps `0` to `255`.

| Phase | `goodfairy` range (after decrement) | Duration | Visible / action | Source |
|-------|-------------------------------------|----------|------------------|--------|
| Death hold + death song | 255 → 200 | 56 ticks | Hero corpse visible; no rescue decision yet | `fmain.c:1388-1390` |
| Luck gate | 199 | 1 tick | `luck < 1` → `revive(TRUE)` (brother succession); `state == FALL` → `revive(FALSE)` (fall recovery); otherwise continue | `fmain.c:1391-1393` |
| Rescue decision hold | 199 → 120 | 80 ticks | If `luck >= 1` and `STATE_DEAD`, nothing visible happens | `fmain.c:1390-1393` |
| Fairy approach | 119 → 20 | 100 ticks | Fairy sprite appears and flies toward hero | `fmain.c:1394-1406` |
| Fairy off / pre-resurrection pause | 19 → 2 | 18 ticks | No sprite; the source comment says "do ressurection effect/glow" but the actual branch is a no-op (`;`) | `fmain.c:1390` |
| Fairy rescue | 1 | 1 tick | `revive(FALSE)` — same brother restored at last safe zone | `fmain.c:1389` |

### Total timing

- From `STATE_DEAD` to brother succession: **57 ticks**.
- From `STATE_DEAD` to fairy rescue: **255 ticks**.
- From the vitality-hit that triggers `checkdead()` to the fairy rescue frame: **7 + 255 = 262 ticks**.

---

## Fairy Animation

While `20 <= goodfairy <= 119` (after decrement), `resolve_player_state` allocates `anim_list[3]` as an `OBJECTS` sprite and positions it at:

```
fairy_x = hero_x + (goodfairy * 2) - 20
fairy_y = hero_y
```

| Tick | `goodfairy` | `fairy_x` offset from hero | `an->index` | Notes |
|------|-------------|----------------------------|-------------|-------|
| 137 | 119 | +218 | 100 or 101 | First fairy frame; sprite appears far right of hero |
| 138 | 118 | +216 | 101 or 100 | Alternates with `cycle & 1` |
| ... | ... | ... | ... | Approaches 2 px per tick |
| 236 | 20 | +20 | 100 or 101 | Last visible fairy frame |

- The sprite frame alternates between `100` and `101` every tick based on the global `cycle` counter (`an->index = 100 + (cycle & 1)`).
- The fairy is not rendered once `goodfairy < 20`; the `--goodfairy < 20` branch is taken before the sprite placement block, and the comment about a "ressurection effect/glow" is not implemented in the actual code.
- `battleflag` is forced to `FALSE` and AI is suspended (`fmain.c:2112`) while the fairy is on screen.

---

## Pit Fall (STATE_FALL) Timing

`STATE_FALL` is a non-lethal variant that shares the same `goodfairy` countdown path but recovers much earlier. It is triggered when the hero walks onto terrain type 9 with `xtype == 52` (`fmain.c:1766-1773`).

- `an->tactic` is reused as a fall-frame counter starting at `0`.
- `fallstates[brother * 6 + (tactic / 5)]` selects the frame.
- Each of the 6 fall frames is held for **5 ticks**, so the visible fall animation lasts **30 ticks**.

| Frame index | `tactic` range | `fallstates[]` value (Julian) | `fallstates[]` value (Phillip) | `fallstates[]` value (Kevin) |
|-------------|----------------|-------------------------------|--------------------------------|------------------------------|
| 0 | 0–4 | 0x20 | 0x24 | 0x37 |
| 1 | 5–9 | 0x22 | 0x27 | 0x38 |
| 2 | 10–14 | 0x3a | 0x3c | 0x3d |
| 3 | 15–19 | 0x6f | 0x6f | 0x6f |
| 4 | 20–24 | 0x70 | 0x70 | 0x70 |
| 5 | 25–29 | 0x71 | 0x71 | 0x71 |

Unlike the `DYING`/`DEAD` frames, the FALL renderer uses these as raw physical sprite frame numbers (it bypasses the `statelist[]` lookup). The rendering sheet switches from `ENEMY` to `OBJECTS` when `tactic >= 16` (`fmain.c:2456-2458`).

After `tactic >= 30`, `death_step` skips the frame selection (`goto cpx`) but the hero remains in `STATE_FALL` until `goodfairy` drops below 200, at which point `revive(FALSE)` is called unconditionally.

### Total fall timing

- From `STATE_FALL` to fall recovery: **57 ticks** (same `goodfairy` countdown as the luck-gate branch of death).
- The visible fall animation is only the first 30 ticks; the remaining 27 ticks are a frozen fall pose waiting for the recovery.

---

## Summary of Transitions

| Event | Ticks after vitality hit | Cumulative tick | Outcome |
|-------|--------------------------|-----------------|---------|
| Vitality reaches 0 | 0 | 0 | `checkdead()` sets `STATE_DYING`, `tactic = 7` |
| Death frame A | 1–3 | 1–3 | `DYING` frame 80/81 |
| Death frame B | 4–7 | 4–7 | `DYING` frame 81/80 |
| Corpse appears | 8 | 8 | `STATE_DEAD`, frame 82; `goodfairy = 255` |
| Luck gate / fall recovery | 57 | 57 | `revive(TRUE)` if `luck < 1`; `revive(FALSE)` if `STATE_FALL` |
| Fairy appears | 137 | 137 | `OBJECTS` sprite at `hero_x + 218` |
| Fairy reaches hero | 236 | 236 | Last visible fairy frame at `hero_x + 20` |
| Fairy off / pre-resurrection pause | 237–254 | 237–254 | No visible sprite; source comment is a no-op |
| Fairy rescue | 255 | 255 | `revive(FALSE)` restores same brother at safe zone |

---

## See also

- [game-loop.md#resolve_player_state](game-loop.md#resolve_player_state) — the phase 7 ladder that drives the countdown.
- [game-loop.md#death_step](game-loop.md#death_step) — the phase 9 animation frame selection.
- [brother-succession.md#revive](brother-succession.md#revive) — what `revive(TRUE)` and `revive(FALSE)` actually do.
- [combat.md#checkdead](combat.md#checkdead) — the vitality → `STATE_DYING` transition and `luck` bookkeeping.
- [actor-animation-catalog.md](../data/sprites/actor-animation-catalog.md) — frame indices 80/81/82 and physical sprite mappings.
