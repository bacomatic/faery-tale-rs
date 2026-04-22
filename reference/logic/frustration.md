# Frustration — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c
> Cross-refs: [RESEARCH §8.4](../RESEARCH.md#84-frustration-cycle), [_discovery/frustration-mechanics.md](../_discovery/frustration-mechanics.md), [movement.md#walk_step](movement.md#walk_step), [ai-system.md#advance_goal](ai-system.md#advance_goal)

## Overview

"Frustration" is the game's label for two paired responses when an actor is
stuck. The triggering condition lives in the `STATE_WALKING` body
(`walk_step`): after three probes — primary direction, clockwise deviate,
counter-clockwise deviate — all report `proxcheck != 0`, the actor has nowhere
legal to step this tick. The hero (`anim_list[0]`) takes the
**visual** branch: a global `frustflag` counter climbs tick-by-tick while the
player is wedged, crossing thresholds that swap in a scratch-head oscillation
and then a weapon-raised pose. Every other actor takes the **tactical** branch:
its `tactic` field is latched to `TACTIC_FRUST`, and the next AI tick reads
that latch inside `advance_goal` and calls `do_tactic` with a random fallback
so the NPC will try a new plan.

The two branches are dispatched from the same source lines (`fmain.c:1654-1661`),
but they interact with entirely different subsystems. The hero path mutates
only the sprite index written out later in the same animation pass; the NPC
path mutates only the tactic field, to be consumed later in the per-tick AI
loop at `fmain.c:2141-2144`. `TACTIC_SHOOTFRUST` is wired into the same
dispatch but is defined (`fmain.c:131`) and tested (`fmain.c:2141`) without
ever being assigned; it is reachable only if a stale slot inherits the value
`9`.

The `frustflag` counter is zeroed at every successful action anywhere in the
animation loop (successful step, shot, fight, sink, dying) — see
`fmain.c:1577, 1650, 1707, 1715, 1725`. The resets are not guarded by actor
index, so any actor's successful action clears the hero's counter. The tactic
latch has no comparable global reset; it stays `TACTIC_FRUST` until
`resolve_frust_tactic` overwrites it.

## Symbols

No new locals are introduced beyond the per-function bindings shown below. All
other identifiers resolve in [SYMBOLS.md](SYMBOLS.md).

**Proposed SYMBOLS additions** (not applied in this pass — orchestrator
review):

- `FRUST_OSC_THRESHOLD = 20` — `fmain.c:1658` — frustflag level above which
  the hero swaps to scratch-head oscillation.
- `FRUST_FIGHT_THRESHOLD = 40` — `fmain.c:1657` — frustflag level above which
  the hero snaps to the weapon-raised pose.
- `FRUST_SPRITE_OSC_BASE = 84` — `fmain.c:1658` — first of the two
  alternating oscillation sprites (84, 85).
- `FRUST_SPRITE_FIGHT = 40` — `fmain.c:1657` — sprite index for the
  weapon-raised pose (aliases the numeric threshold by coincidence).
- `FRUST_BOW_TACTIC_MIN = 2`, `FRUST_BOW_TACTIC_MAX = 5` — `fmain.c:2142` —
  inclusive bow-wielder random-tactic range (FOLLOW..BACKUP).
- `FRUST_MELEE_TACTIC_MIN = 3`, `FRUST_MELEE_TACTIC_MAX = 4` — `fmain.c:2143`
  — inclusive melee-wielder random-tactic range (BUMBLE_SEEK..RANDOM).

## select_frust_anim

Source: `fmain.c:1655-1659`
Called by: `trigger_frust`
Calls: none

```pseudo
def select_frust_anim(flag: int) -> int:
    """Hero sprite override for the current frustflag; returns -1 for no override."""
    if flag > 40:                                 # fmain.c:1657 — weapon-raised threshold
        return 40                                 # fmain.c:1657 — sprite index 40 = south-fight pose
    if flag > 20:                                 # fmain.c:1658 — oscillation threshold
        return 84 + ((cycle >> 1) & 1)            # fmain.c:1658 — alternates sprites 84 and 85 every 2 cycles
    return -1
```

## trigger_frust

Source: `fmain.c:1654-1661`
Called by: `walk_step`
Calls: `select_frust_anim`

```pseudo
def trigger_frust(i: int, an: Shape) -> int:
    """Blocked-on-all-three dispatch: hero increments frustflag, NPC latches TACTIC_FRUST.

    Returns a sprite-index override for the hero (-1 when no override), or -1 for NPCs.
    """
    if i == 0:
        frustflag = frustflag + 1                 # fmain.c:1656 — hero-only counter
        return select_frust_anim(frustflag)
    an.tactic = TACTIC_FRUST                      # fmain.c:1661 — NPC latch for next AI tick
    return -1
```

## resolve_frust_tactic

Source: `fmain.c:2141-2144`
Called by: `advance_goal`
Calls: `do_tactic`

```pseudo
def resolve_frust_tactic(i: int, an: Shape) -> None:
    """AI-tick handler for a latched TACTIC_FRUST / TACTIC_SHOOTFRUST: pick a new tactic."""
    if (an.weapon & 4) != 0:                      # fmain.c:2142 — weapon bit 2 = bow/wand (ranged)
        do_tactic(i, rand(2, 5))                  # fmain.c:2142 — FOLLOW..BACKUP inclusive (rand4()+2)
        return
    do_tactic(i, rand(3, 4))                      # fmain.c:2143 — BUMBLE_SEEK or RANDOM (rand2()+3)
```

## Notes

- **Hero-vs-NPC split.** The `i == 0` test at `fmain.c:1654` is the only
  discriminator; both branches share the same `blocked:` label and both
  flow into `goto cpx` after setting their respective side-effect.
  `walk_step` ([movement.md#walk_step](movement.md#walk_step)) already
  inlines this dispatch, returning early at the blocked point. This spec
  carves the animation-frame selection out as a pure helper
  (`select_frust_anim`) so porters can reuse it for whatever sprite system
  they target without re-deriving the 20/40 threshold pair.

- **Sprite-index coincidence.** The literal `40` appears twice in
  `fmain.c:1657` — once as a threshold on `frustflag` and once as the
  sprite index returned when the threshold is crossed. They are independent
  constants that happen to collide. The proposed `FRUST_FIGHT_THRESHOLD`
  and `FRUST_SPRITE_FIGHT` symbols preserve that distinction.

- **Reset asymmetry.** `frustflag = 0` fires from at least five unrelated
  animation paths (SINK, successful walk, successful shot, melee, dying —
  see `fmain.c:1577, 1650, 1707, 1715, 1725`); none of them are guarded by
  `i == 0`. Consequence: if any NPC successfully acts during a tick, the
  hero's frustration counter resets too. The thresholded animations at
  `select_frust_anim` therefore tend to fire only when the hero is alone
  and blocked. `tactic = TACTIC_FRUST` has no such cascade — it persists
  until `resolve_frust_tactic` overwrites it on the NPC's next AI tick.

- **`TACTIC_SHOOTFRUST` is unreachable.** `resolve_frust_tactic` treats
  `TACTIC_FRUST` and `TACTIC_SHOOTFRUST` identically (the `==` check lives
  in `advance_goal`, `fmain.c:2141`), but no code path anywhere in fmain.c
  or fmain2.c ever writes `9` into `an->tactic`. The branch exists only to
  catch a stale slot that happens to already hold `9`. See
  [PROBLEMS.md P11](../PROBLEMS.md) for the wider unused-tactics problem.

- **Cross-goal reassignment.** `resolve_frust_tactic` is checked at
  `fmain.c:2141` *before* the goal-mode `if/else` chain at `fmain.c:2146+`,
  so a `GOAL_FLEE` / `GOAL_FOLLOWER` / `GOAL_STAND` / `GOAL_WAIT` /
  `GOAL_CONFUSED` actor whose tactic is latched to `TACTIC_FRUST` will
  receive a random fallback tactic regardless of its mode — including a
  fleeing actor being told to `TACTIC_BUMBLE_SEEK` toward the hero.
  `advance_goal` ([ai-system.md#advance_goal](ai-system.md#advance_goal))
  documents this ordering in its overview; see RESEARCH §8.4 for the
  gameplay implications.
