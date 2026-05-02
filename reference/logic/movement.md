# Movement — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, fmain2.c, fsubs.asm
> Cross-refs: [RESEARCH §5](../RESEARCH.md#5-movement--direction), [_discovery/movement.md](../_discovery/movement.md), [_discovery/terrain-collision.md](../_discovery/terrain-collision.md), [_discovery/npc-terrain-speed.md](../_discovery/npc-terrain-speed.md), [logic/game-loop.md](game-loop.md), [logic/ai-system.md](ai-system.md), [logic/combat.md](combat.md)

## Overview

Each non-OBJECTS actor is driven once per frame from Phase 9's `actor_tick`
dispatcher. When the actor's motion state is `STATE_WALKING`, `walk_step`
computes a candidate `(xtest, ytest)` from the compass direction + a speed
derived from the current environ, then clears or deviates that candidate
through `proxcheck`. When the state is `STATE_STILL`, `still_step` holds the
position (but still lets residual ice momentum slide the actor). Both paths
end by sampling the terrain at the actor's new position and calling
`update_environ`, which maps the terrain code `j` to the environ code `k`
and applies water-sink / lava / fall-pit transitions.

The primitives `newx` / `newy` (in `fsubs.asm`) translate a `(dir, speed)`
pair into a position delta using the compass vector tables `xdir[]` /
`ydir[]`. `proxcheck` tests the candidate against the terrain-collision
primitive `prox` plus an O(anix) actor bounding-box scan. `set_course`
computes a facing toward (or, for mode 3, away from) a target, snapping to
the dominant axis and — for close-range pursue/follow — wobbling ±1
direction slot per tick. `move_figure` is the one-shot commit used by the
combat knock-back path and is documented in [combat.md](combat.md#move_figure);
this doc lists it only in `Calls:` headers.

Door transitions at terrain code 15 (Wave 12) and the swan-on-ice carrier
branch at `fmain.c:1592` (Wave 14) are explicitly out of scope; this doc
delegates to those waves at the relevant call sites.

## Symbols

No new locals beyond the function-local bindings in each pseudo block. New
globals, enums, and table references that this doc introduces are listed in
the wave report for orchestrator review; until they land in
[SYMBOLS.md](SYMBOLS.md), all numeric literals outside `{-1, 0, 1, 2}`
carry inline citations as required by [STYLE §7](STYLE.md#7-numeric-literals).

## newx

Source: `fsubs.asm:1280-1296`
Called by: `walk_step`, `still_step`, `move_figure`, `melee_swing`, `missile_step`, `resolve_player_state`
Calls: `xdir`

```pseudo
def newx(x: int, dir: int, speed: int) -> int:
    """Advance world X by xdir[dir]*speed/2, masked to the 15-bit world."""
    if dir > 7:                                           # fsubs.asm:1287-1288 — 8/9 = STILL sentinel, no move
        return x
    prod = xdir[dir] * speed                              # fsubs.asm:1291-1292 — muls.w (signed)
    step = wrap_u16(prod) >> 1                            # fsubs.asm:1293 — lsr.w: logical, not arithmetic
    return (x + step) & 0x7fff                            # fsubs.asm:1294-1295, 0x7fff = 15-bit world mask
```

Notes:
- The `lsr.w` on a signed product gives a +1 pixel bias for negative-direction
  steps (discovery reports ~±1 px asymmetry). Porters reproducing
  pixel-exact behavior must use `wrap_u16(...) >> 1`, not arithmetic
  shift.
- Diagonal entries of `xdir[]` have magnitude 2, cardinal entries have
  magnitude 3 (`fsubs.asm:1276`); the near-parity of `3` vs `√(2²+2²) ≈ 2.83`
  is the design's reason for skipping true unit vectors.

## newy

Source: `fsubs.asm:1298-1319`
Called by: `walk_step`, `still_step`, `move_figure`, `melee_swing`, `missile_step`, `resolve_player_state`
Calls: `ydir`

```pseudo
def newy(y: int, dir: int, speed: int) -> int:
    """Advance world Y by ydir[dir]*speed/2; preserves bit 15 of the input as a flag layer."""
    if dir > 7:                                           # fsubs.asm:1301-1302 — 8/9 = STILL, no move
        return y
    flag_bit = y & 0x8000                                 # fsubs.asm:1316, 0x8000 = preserved bit-15 flag
    prod = ydir[dir] * speed                              # fsubs.asm:1309-1310 — muls.w
    step = wrap_u16(prod) >> 1                            # fsubs.asm:1311 — lsr.w
    coord = (y + step) & 0x7fff                           # fsubs.asm:1312-1315, 0x7fff = 15-bit world mask
    return coord | flag_bit                               # fsubs.asm:1317 — restore bit 15
```

Notes:
- Bit 15 of `abs_y` is preserved across every Y step. Its semantic meaning
  is unresolved in source and is logged as an open question in
  [_discovery/movement.md](../_discovery/movement.md#unresolved).

## proxcheck

Source: `fmain2.c:277-296`
Called by: `walk_step`, `move_figure`, `resolve_player_state` (swan dismount probe)
Calls: `prox`, `anim_list`, `anix`, `ENEMY`

```pseudo
def proxcheck(x: int, y: int, i: int) -> int:
    """Return 0 if (x, y) is clear for actor i, else the blocking terrain code (or 16 for actor)."""
    # fmain2.c:278-283 — wraiths (ENEMY race 2) bypass terrain entirely; the hero
    # treats terrain codes 8 (lava) and 9 (pit) as passable.
    is_wraith = anim_list[i].type == ENEMY and anim_list[i].race == 2   # fmain2.c:279, 2 = wraith race
    if not is_wraith:
        t = prox(x, y)
        if i == 0 and (t == 8 or t == 9):                 # fmain2.c:282, 8 = lava, 9 = pit
            t = 0
        if t != 0:
            return t
    # fmain2.c:285-291 — O(anix) actor bounding-box scan; 22x18 px box around (x, y).
    j = 0
    while j < anix:
        other = anim_list[j]
        skip_self = j == i
        skip_raft_slot = j == 1                           # fmain2.c:286, 1 = raft/companion slot
        skip_type = other.type == 5                       # fmain2.c:286, 5 = RAFT type (walk-overable)
        skip_dead = other.state == STATE_DEAD
        if skip_self or skip_raft_slot or skip_type or skip_dead:
            j = j + 1
            continue
        dx = x - other.abs_x
        dy = y - other.abs_y
        in_box = dx < 11 and dx > -11 and dy < 9 and dy > -9   # fmain2.c:289, 11/9 = half-extent of 22x18 box
        if in_box:
            return 16                                     # fmain2.c:289, 16 = actor-collision sentinel
        j = j + 1
    return 0
```

Notes:
- `prox` is the 2-probe terrain test at `fsubs.asm:1590-1614`: probe `(x+4,
  y+2)` blocks for terrain `1` or `≥10`; probe `(x-4, y+2)` blocks for
  terrain `1` or `≥8`. The asymmetric `≥8` / `≥10` thresholds are why the
  hero-only `t == 8 or t == 9` passthrough above exists at all — types 8/9
  only surface at the left probe.
- Returning `16` for the actor collision case overlaps no terrain code
  (terrain is ≤15), so callers can use a single nonzero test.

## walk_step

Source: `fmain.c:1580-1664`
Called by: `actor_tick` (Phase 9, `STATE_WALKING` branch)
Calls: `newx`, `newy`, `proxcheck`, `px_to_im`, `doorfind`, `set_course`, `anim_list`, `stuff`, `riding`, `hero_sector`, `bumped`, `frustflag`, `TACTIC_FRUST`

```pseudo
def walk_step(i: int, an: Shape, d: int, k: int) -> int:
    """WALKING motion body for actor i; returns terrain code j sampled at the new position.
    Caller (actor_tick) then runs update_environ(i, an, j, an.state, k)."""
    # --- Ice (environ -2) momentum branch (fmain.c:1581-1598) ------------------
    if k == -2:                                           # fmain.c:1581, -2 = ice environ code
        if riding == 11:                                  # fmain.c:1582, 11 = riding swan (Wave 14)
            cap = 40                                      # fmain.c:1582, 40 = swan terminal velocity
        else:
            cap = 42                                      # fmain.c:1582, 42 = default ice terminal velocity
        # fmain.c:1583-1584 — accumulate: vel += xdir[d] (newx(20,d,2) - 20 == xdir[d]).
        nvx = an.vel_x + newx(20, d, 2) - 20              # fmain.c:1583, 20 = cancellation base
        nvy = an.vel_y + newy(20, d, 2) - 20              # fmain.c:1584, 20 = cancellation base
        nvx1 = abs(nvx)                                   # fmain.c:1585-1586
        nvy1 = abs(nvy)                                   # fmain.c:1585-1586
        if nvx1 < cap - 8:                                # fmain.c:1588, 8 = X-axis cap reduction
            an.vel_x = nvx
        if nvy1 < cap:                                    # fmain.c:1589
            an.vel_y = nvy
        xtest = an.abs_x + an.vel_x // 4                  # fmain.c:1590, 4 = velocity-to-position divisor
        ytest = an.abs_y + an.vel_y // 4                  # fmain.c:1591, 4 = velocity-to-position divisor
        if riding == 11:                                  # fmain.c:1592 — swan-on-ice: delegated to Wave 14
            set_course(0, -nvx, -nvy, 6)                  # fmain.c:1592, 6 = raw-vector set_course mode
            d = an.facing
            an.facing = d
            an.abs_x = xtest
            an.abs_y = ytest
            j_sample = px_to_im(xtest, ytest)             # fmain.c:1636 — terrain at new pos
            if an.race == 2 or an.race == 4:              # fmain.c:1640, 2 = wraith race, 4 = snake race
                j_sample = 0
            return j_sample
        if proxcheck(xtest, ytest, i) == 0:
            an.facing = d
            an.abs_x = xtest
            an.abs_y = ytest
            j_sample = px_to_im(xtest, ytest)             # fmain.c:1636
            if an.race == 2 or an.race == 4:              # fmain.c:1640, 2 = wraith race, 4 = snake race
                j_sample = 0
            return j_sample
        k = 0                                             # fmain.c:1597 — blocked on ice: fall through to walk

    # --- Speed selection (fmain.c:1599-1602) ----------------------------------
    if i == 0 and riding == 5:                            # fmain.c:1599, 5 = RAFT/turtle type (hero mount)
        e = 3                                             # fmain.c:1599, 3 = hero-on-raft speed
    elif k == -3:                                         # fmain.c:1600, -3 = lava environ
        e = -2                                            # fmain.c:1600, -2 = lava reversed-walk speed
    elif k == -1:                                         # fmain.c:1601, -1 = slippery environ
        e = 4                                             # fmain.c:1601, 4 = slippery-terrain speed
    elif k == 2 or k > 6:                                 # fmain.c:1602, 6 = boundary between medium-water and deep (wade)
        e = 1                                             # fmain.c:1602, 1 = wading/deep-water speed
    else:
        e = 2                                             # fmain.c:1602, 2 = normal walking speed

    # --- Try primary direction, then +1 deviate, then -2 deviate (fmain.c:1603-1626) ---
    xtest = newx(an.abs_x, d, e)
    ytest = newy(an.abs_y, d, e)
    j = proxcheck(xtest, ytest, i)
    if i == 0:
        if j == 15:                                       # fmain.c:1609, 15 = door tile — Wave 12
            doorfind(xtest, ytest, 0)
        else:
            bumped = 0                                    # fmain.c:1609 — reset hero door-nudge latch
        if stuff[30] != 0 and j == 12:                    # fmain.c:1611, 30 = crystal-shard stuff slot, 12 = crystal wall
            j = 0                                         # crystal-shard pass: proceed as clear
    if j != 0:
        d = (d + 1) & 7                                   # fmain.c:1615, 7 = 8-direction mask; +1 = CW deviate
        xtest = newx(an.abs_x, d, e)
        ytest = newy(an.abs_y, d, e)
        if proxcheck(xtest, ytest, i) != 0:
            d = (d - 2) & 7                               # fmain.c:1620, 7 = 8-direction mask; -2 = CCW from original
            xtest = newx(an.abs_x, d, e)
            ytest = newy(an.abs_y, d, e)
            if proxcheck(xtest, ytest, i) != 0:
                # --- Blocked on all three (fmain.c:1654-1663) -----------------
                if i == 0:
                    frustflag = frustflag + 1             # fmain.c:1657 — hero frustration counter drives scratch-head anim
                else:
                    an.tactic = TACTIC_FRUST              # fmain.c:1660
                return px_to_im(an.abs_x, an.abs_y)       # no commit; caller's k retained via update_environ

    # --- newloc: commit (fmain.c:1628-1650) -----------------------------------
    an.facing = d
    j = px_to_im(xtest, ytest)                            # fmain.c:1636 — terrain at new pos
    if an.race == 2 or an.race == 4:                      # fmain.c:1640, 2 = wraith race, 4 = snake race
        j = 0

    # fmain.c:1641-1644 — if deeper-than-shallow and stepping toward drier terrain,
    # decrement environ but do NOT commit position (ramp-out delay).
    if k > 2:
        drier = j == 0
        ramp_from_med = j == 3 and k > 5                  # fmain.c:1642, 3 = medium water, 5 = medium environ
        ramp_from_deep = j == 4 and k > 10                # fmain.c:1642, 4 = deep water, 10 = deep environ
        if drier or ramp_from_med or ramp_from_deep:
            if hero_sector != 181:                        # fmain.c:1643, 181 = special drain-sink sector
                k = k - 1
            # Emulate fmain.c:1643 "goto raise" which skips position commit but still
            # writes k back through update_environ. Encode k onto the actor now so
            # the caller's subsequent update_environ sees the decremented value.
            an.environ = k
            return j

    # --- Commit velocity + position (fmain.c:1646-1650) -----------------------
    an.vel_x = (xtest - an.abs_x) * 4                     # fmain.c:1646, 4 = velocity = displacement*4
    an.vel_y = (ytest - an.abs_y) * 4                     # fmain.c:1647, 4 = velocity = displacement*4
    an.abs_x = xtest
    an.abs_y = ytest
    frustflag = 0                                         # fmain.c:1650
    return j
```

Notes:
- After a successful commit, the inline logic at `fmain.c:1634-1640` zeroes
  the terrain sample for wraiths (`race == 2`) and snakes (`race == 4`)
  so that `update_environ` treats them as always on dry ground. The same
  zeroing is applied in every return path that exposes a terrain sample.
- The `+1` / `-2` deviation sequence means an actor blocked on a wall
  tries CW then CCW from the original heading; three-of-three blocked
  drops into the "blocked" handling (hero frustration anim, NPC goes to
  `TACTIC_FRUST`).
- The swan-on-ice commit path short-circuits `proxcheck` — the swan does
  not collide with terrain or actors while the hero is mounted. See Wave
  14.

## still_step

Source: `fmain.c:1651-1665`, `fmain.c:1740-1746`
Called by: `actor_tick` (Phase 9, `STATE_STILL` branch, and as the fall-through target of `walk_step` when blocked)
Calls: `px_to_im`

```pseudo
def still_step(i: int, an: Shape, k: int) -> int:
    """STILL motion body: hold position but let ice momentum carry the actor one slide tick.
    Returns the terrain code j sampled at the (possibly updated) position."""
    # fmain.c:1663-1664 — on ice, STILL jumps to the cpx label which does the slide.
    # cpx body (fmain.c:1742-1745): abs_x += vel_x/4; abs_y += vel_y/4.
    if k == -2:                                           # fmain.c:1663, -2 = ice environ code
        an.abs_x = an.abs_x + an.vel_x // 4               # fmain.c:1743, 4 = velocity divisor
        an.abs_y = an.abs_y + an.vel_y // 4               # fmain.c:1744, 4 = velocity divisor
    return px_to_im(an.abs_x, an.abs_y)                   # fmain.c:1741 — sample for update_environ
```

Notes:
- The real source jumps here from `walk_step`'s blocked branch via a
  `goto still` label at `fmain.c:1651`, and from the STILL state's `if
  (k == -2) goto cpx` transition at `fmain.c:1663`. The pseudo-code
  collapses both paths into one body.
- Idle animation-frame bookkeeping (`dex = inum + 1` at `fmain.c:1662`)
  is the responsibility of `actor_tick`'s state dispatcher; the
  race-specific frame overrides applied afterwards live in
  [`update_actor_index`](game-loop.md#update_actor_index).

## update_environ

Source: `fmain.c:1759-1801`
Called by: `actor_tick` (Phase 9, after `walk_step` / `still_step` / state-specific bodies)
Calls: `xfer`, `find_place`, `setmood`, `anim_list`, `raftprox`, `xtype`, `hero_sector`, `new_region`, `luck`, `brother`, `fallstates`, `MAP_STABLE`

```pseudo
def update_environ(i: int, an: Shape, j: int, s: int, k: int) -> None:
    """Map terrain code j to environ code k, apply water-ramp / lava / pit transitions,
    write back to an.environ when the map is stable. s is the actor's current motion state."""
    # fmain.c:1762 — hero adjacent to a raft/turtle is treated as dry.
    if i == 0 and raftprox != 0:
        k = 0
    elif j == 0:
        k = 0                                             # fmain.c:1763 — open ground
    elif j == 6:                                          # fmain.c:1764, 6 = slippery terrain code
        k = -1                                            # fmain.c:1764, -1 = slippery environ
    elif j == 7:                                          # fmain.c:1765, 7 = ice terrain code
        k = -2                                            # fmain.c:1765, -2 = ice environ
    elif j == 8:                                          # fmain.c:1766, 8 = lava terrain code
        k = -3                                            # fmain.c:1766, -3 = lava environ
    elif j == 9 and i == 0 and xtype == 52:               # fmain.c:1767, 9 = pit terrain, 52 = pit xtype
        if an.state != STATE_FALL:
            an.index = fallstates[brother * 6]            # fmain.c:1769, 6 = entries-per-brother in fallstates
            an.state = STATE_FALL
            an.tactic = 0                                 # fmain.c:1771 — reused as FALL frame counter
            luck = luck - 2                               # fmain.c:1772, 2 = pit-fall luck penalty
            setmood(True)
        k = -2                                            # fmain.c:1774 — falling uses ice momentum
    elif j == 2:
        k = 2                                             # fmain.c:1776 — shallow-water environ (direct)
    elif j == 3:                                          # fmain.c:1777, 3 = medium water terrain
        k = 5                                             # fmain.c:1777, 5 = medium-water environ (direct jump)
    elif j == 4 or j == 5:                                # fmain.c:1778, 4/5 = deep-water terrain codes
        # fmain.c:1779-1797 — deep-water ramp. j=4 ramps toward 10; j=5 ramps toward 30 (death).
        if j == 4:                                        # fmain.c:1779, 4 = medium-deep terrain
            target = 10                                   # fmain.c:1779, 10 = medium-deep saturation environ
        else:
            target = 30                                   # fmain.c:1779, 30 = death-depth environ
        if k > target:
            k = k - 1
        elif k < target:
            k = k + 1
            dying = s == STATE_DYING or s == STATE_DEAD
            if not dying and k == 30:                     # fmain.c:1784, 30 = death-depth threshold
                if hero_sector == 181:                    # fmain.c:1785, 181 = drain-sink sector
                    if i == 0:
                        k = 0
                        new_region = 9                    # fmain.c:1788, 9 = dungeon region teleport
                        xfer(0x1080, 34950, False)        # fmain.c:1789, 0x1080 / 34950 = region-9 spawn coords
                        find_place(1)
                    else:
                        an.vitality = 0                   # fmain.c:1792 — NPCs dropped into sector 181 die instantly
                an.state = STATE_STILL
            elif not dying and k > 15:                    # fmain.c:1796, 15 = sink-state threshold
                an.state = STATE_SINK
    # fmain.c:1799 — coming out of SINK into dry tile returns to STILL.
    if k == 0 and s == STATE_SINK:
        an.state = STATE_STILL
    # fmain.c:1801 — only write back when the map isn't scrolling (keeps stats stable across redraws).
    if MAP_STABLE:
        an.environ = k
```

Notes:
- The direct `j==3 → k=5` assignment (no ramp) is asymmetric with the
  ramped deep-water branches. The recorded consequence is that a first
  step onto medium water immediately incurs the environ=5 wading speed
  penalty, whereas deep water ramps up over many ticks.
- The `hero_sector == 181` clause combines two effects: a water-hazard
  teleport to region 9 (swamp-to-dungeon drain) for the hero, and an
  instant-kill for any NPC following. Cross-reference:
  [_discovery/astral-plane.md](../_discovery/astral-plane.md) and
  [_discovery/terrain-collision.md](../_discovery/terrain-collision.md).
- Water-damage application (the `fiery_death` branch at `fmain.c:1843-1849`)
  is not part of `update_environ`; it runs one block later in
  `actor_tick` and will be documented with the damage-tick owner.

## set_course

Source: `fmain2.c:57-228`
Called by: `advance_goal` (every AI tactic except `TACTIC_RANDOM`), `walk_step` (swan-on-ice branch), `apply_witch_fx`
Calls: `anim_list`, `com2`

```pseudo
def set_course(object: int, target_x: int, target_y: int, mode: int) -> None:
    """Set anim_list[object].facing (and usually .state = STATE_WALKING) toward or away from (target_x, target_y)."""
    an = anim_list[object]

    # --- Delta setup (fmain2.c:79-88) -----------------------------------------
    if mode == 6:                                         # fmain2.c:79, 6 = raw-vector mode (no subtraction)
        xdif = target_x
        ydif = target_y
    else:
        xdif = an.abs_x - target_x                        # fmain2.c:87 — note: self - target (inverted by com2 lookup)
        ydif = an.abs_y - target_y                        # fmain2.c:88

    # --- Sign + absolute decomposition (fmain2.c:90-109) ----------------------
    if xdif > 0:
        xdir = 1
        xabs = xdif
    elif xdif < 0:
        xdir = -1
        xabs = -xdif
    else:
        xdir = 0
        xabs = 0
    if ydif > 0:
        ydir = 1
        yabs = ydif
    elif ydif < 0:
        ydir = -1
        yabs = -ydif
    else:
        ydir = 0
        yabs = 0

    # --- Dominant-axis snap (fmain2.c:113-126) --------------------------------
    # Mode 4 skips snapping, preserving all 8 diagonals (BUMBLE_SEEK).
    if mode != 4:                                         # fmain2.c:113, 4 = BUMBLE_SEEK (snap-skip) mode
        if (xabs >> 1) > yabs:
            ydir = 0                                      # mostly horizontal: zero vertical
        if (yabs >> 1) > xabs:
            xdir = 0                                      # mostly vertical: zero horizontal

    # --- Deviation + reversal (fmain2.c:130-152) ------------------------------
    deviation = 0
    dist = xabs + yabs
    if mode == 1 and dist < 40:                           # fmain2.c:138, 40 = PURSUE close-range threshold
        deviation = 1
    if mode == 2 and dist < 30:                           # fmain2.c:146, 30 = FOLLOW close-range threshold
        deviation = 1                                     # (source comment says 2; assembly uses 1)
    if mode == 3:                                         # fmain2.c:149, 3 = BACKUP/reverse mode
        xdir = -xdir                                      # fmain2.c:151 — BACKUP: reverse
        ydir = -ydir                                      # fmain2.c:152

    # --- com2 lookup (fmain2.c:153-162) ---------------------------------------
    # idx formula "4 - 3*ydir - xdir" produces 0..8; com2[] maps each (ydir, xdir)
    # sign pair to a compass direction (or 9 = still).
    idx = 4 - 3 * ydir - xdir                             # fmain2.c:153
    j = com2[idx]                                         # fmain2.c:155

    # --- State write-back (fmain2.c:165-187) ----------------------------------
    if j == 9:                                            # fmain2.c:165, 9 = still/no-move sentinel
        an.state = STATE_STILL
        return
    if chance(1, 2):                                      # fmain2.c:174 — rand() bit-1 test; 50/50 wobble direction
        j = j + deviation
    else:
        j = j - deviation
    an.facing = j & 7                                     # fmain2.c:185, 7 = 8-direction mask
    if mode != 5:                                         # fmain2.c:186, 5 = no-walk mode (SHOOT tactic preserves shooting state)
        an.state = STATE_WALKING
```

Notes:
- The "self minus target" sign convention inverts naturally when the
  `com2[]` table is consulted: the resulting facing points from the actor
  toward the target (modes 0–2, 4–6) or away from it (mode 3).
- Mode 5 is the SHOOT-tactic variant: facing is updated but the actor
  remains in `STATE_SHOOT1` / `STATE_SHOOT3` for the aim/release
  animation.
- Mode 4 (BUMBLE_SEEK) skips the dominant-axis snap, which makes the
  actor take pure diagonals more often — a readable signal that a
  "stupid" AI is active ([ai-system.md §advance_goal](ai-system.md#advance_goal)).
- Caller-side tactic-to-mode mapping is in
  [ai-system.md](ai-system.md) and [_discovery/movement.md](../_discovery/movement.md#how-do_tactic-maps-tactics-to-set_course-modes-fmain2c1664-1700).
- `move_figure` — used by `dohit`'s knock-back and the proxcheck-gated
  commit helper — is documented in
  [combat.md#move_figure](combat.md#move_figure); it is not redefined
  here.

## Notes

### Speed table summary

The environ-to-speed mapping at `fmain.c:1599-1602` resolves to the
following effective per-tick displacement for cardinal directions
(diagonal magnitudes are 2/3 of cardinal):

| Environ `k` | Source terrain (j) | Speed `e` | Cardinal px/tick |
|---|---|---|---|
| `-3` | 8 (lava) | `-2` | 3 (reversed facing) |
| `-2` | 7 (ice) | velocity-based | `vel/4`, capped 34 X / 42 Y |
| `-1` | 6 (slippery) | `4` | 6 |
|  `0` | 0 (open) | `2` | 3 |
|  `2` | 2 (shallow water) | `1` | 1 |
|  `3..5` | (ramping) | `2` | 3 |
|  `6` | (ramping) | `2` | 3 |
|  `7..30` | (ramping) | `1` | 1 |
| (hero, `riding == 5`) | any | `3` | 4 |

Hero-only speed paths:
- `riding == 5` (raft/turtle): fixed `e = 3`.
- `riding == 11` (swan): always enters the `k == -2` ice branch
  (`fmain.c:1464`), with the swan-specific `cap = 40`.

### Ice velocity integration

Every tick the ice branch does `vel += xdir[d]` / `vel += ydir[d]` (the
`newx(20, d, 2) - 20` expression collapses to `xdir[d]` because
`20 + xdir[d]*2/2 - 20 == xdir[d]`). Velocity grows until the absolute
value would exceed `cap - 8` (X) or `cap` (Y); further input in the same
direction is discarded. Position advances by `vel // 4`, giving up to
~8 px/tick in X and ~10 px/tick in Y.

### Vel-to-displacement cross-uses

`vel_x` / `vel_y` are set to `displacement * 4` at every non-ramp
`walk_step` commit. Two subsystems read these back:

1. `resolve_player_state` gates swan dismount on
   `abs(vel_x) < 15 and abs(vel_y) < 15` ([game-loop.md](game-loop.md#resolve_player_state)).
2. The `STATE_FALL` frame-counter branch at `fmain.c:1737-1738` decays
   `vel_x` and `vel_y` by `*3 // 4` per tick — ~25% friction, i.e., the
   FALL animation slowly halts within a few ticks of the initial pit
   plunge.

### World wrapping

The 15-bit coordinate mask in `newx` / `newy` implicitly wraps, but the
hero's outdoor wrap at `fmain.c:1831-1839` is a separate post-commit step
that runs only when `region_num < 8`: any of the four out-of-range cases
(`< 300` or `> 32565` on either axis) teleports the hero to the opposite
edge. NPCs never wrap.

### Direction mnemonic

Per [RESEARCH §5](../RESEARCH.md#5-movement--direction) and
[SYMBOLS.md §2.1](SYMBOLS.md#21-directions-fsubsasm-see-research-51):
`0=NW, 1=N, 2=NE, 3=E, 4=SE, 5=S, 6=SW, 7=W`. Values 8 and 9 both signal
"no direction / still" and are special-cased at the top of `newx` and
`newy`.
