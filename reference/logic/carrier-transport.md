# Carrier Transport — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, fmain2.c
> Cross-refs: [RESEARCH §9.6](../RESEARCH.md#96-special-extents), [_discovery/carrier-transport-system.md](../_discovery/carrier-transport-system.md), [_discovery/carriers.md](../_discovery/carriers.md), [logic/movement.md](movement.md), [logic/quests.md#get_turtle](quests.md#get_turtle), [logic/game-loop.md](game-loop.md)

## Overview

The carrier subsystem owns four rideable / transport actors that share two
fixed slots in `anim_list[]`: the raft lives permanently at slot 1, while
the swan, turtle, and (non-rideable) dragon all load into slot 3, the
"carrier slot", overwriting one another on demand. The global `riding`
discriminates the current mount (0 none / 1 raft / 5 turtle / 11 swan) and
is re-derived every frame. `active_carrier` records which carrier file id
is loaded into slot 3 (0 / 5 / 10 / 11) and gates random-encounter
suppression and shape-reload decisions.

Each frame the game loop first samples the hero↔slot-`wcarry` distance to
set `raftprox` (Phase 7 tail; `fmain.c:1455-1464`) before the Phase 9
actor tick dispatches by `type`. The CARRIER body (`fmain.c:1494-1547`)
runs mount tests for swan and turtle plus the turtle's water-constrained
autonomous swim; the RAFT body (`fmain.c:1562-1573`) snaps the raft onto
the hero when the hero stands on terrain 3-5. Swan dismount fires from
the player-input branch (`fmain.c:1417-1428`) with a velocity gate and a
fiery-lava veto. The Sea Shell item handler (`fmain.c:3457-3461`) is the
quest-driven spawn point for the turtle; region-entry extent changes
(`fmain.c:2716-2719`) handle auto-spawn and auto-despawn.

Mid-flight hero movement on the swan is owned by `walk_step` via the
`environ == -2` / `riding == 11` branch documented in
[logic/movement.md](movement.md#walk_step). Turtle spawn placement
(`get_turtle`) is owned by [logic/quests.md](quests.md#get_turtle). This
doc covers the boarding, dismount, extent-driven activation, and
per-tick carrier-body logic not covered there.

## Symbols

Locals are function-scoped. Globals and new constants are listed in the
wave report for orchestrator review; until they land in
[SYMBOLS.md](SYMBOLS.md), numeric literals outside `{-1, 0, 1, 2}` carry
inline citations per [STYLE §7](STYLE.md#7-numeric-literals).

## compute_raftprox

Source: `fmain.c:1455-1464`
Called by: `game_tick` (Phase 7, end of player-state resolution)
Calls: `anim_list`

```pseudo
def compute_raftprox() -> None:
    """Derive wcarry, raftprox, and the swan ice-physics environ latch once per frame."""
    raftprox = 0                                          # fmain.c:1455 — FALSE
    if active_carrier != 0:
        wcarry = 3                                        # fmain.c:1456, 3 = carrier slot (anim_list[3])
    else:
        wcarry = 1                                        # fmain.c:1456, 1 = raft slot (anim_list[1])
    xstart = anim_list[0].abs_x - anim_list[wcarry].abs_x - 4   # fmain.c:1457, 4 = sprite anchor bias
    ystart = anim_list[0].abs_y - anim_list[wcarry].abs_y - 4   # fmain.c:1458, 4 = sprite anchor bias
    near = xstart < 16 and xstart > -16 and ystart < 16 and ystart > -16   # fmain.c:1459, 16 = near threshold px
    if near:
        raftprox = 1                                      # fmain.c:1460 — near (mount eligible)
    very_near = xstart < 9 and xstart > -9 and ystart < 9 and ystart > -9  # fmain.c:1461, 9 = very-near threshold px
    if very_near:
        raftprox = 2                                      # fmain.c:1462 — very near (raft snap eligible)
    if riding == RIDING_SWAN:                             # fmain.c:1464 — latch ice physics for flight
        anim_list[0].environ = ENVIRON_ICE
```

Notes:
- The `-4` anchor bias in both axes compensates for the hero sprite's
  foot-origin vs the carrier's center-origin so the proximity box is
  roughly centered on the visible overlap.
- Writing `environ = ENVIRON_ICE` (−2) every frame is what routes the
  hero into the inertial swan branch in `walk_step` (see
  [movement.md#walk_step](movement.md#walk_step)). There is no separate
  FLYING motion-state transition; flight is just the ice code path.

## load_carrier

Source: `fmain.c:2784-2802`
Called by: `carrier_extent_update`, `get_turtle`, `process_input_key` (cheat 'B')
Calls: `anim_list`, `read_shapes`, `prep`, `motor_off`, `extent_list`, `seq_list`

```pseudo
def load_carrier(n: int) -> None:
    """Place a carrier actor (swan, turtle, or dragon) in anim_list[3] from file id n."""
    an = anim_list[3]                                     # fmain.c:2787, 3 = carrier slot
    if n == 10:                                           # fmain.c:2788, 10 = dragon file id
        an.type = DRAGON
    else:
        an.type = CARRIER
    # Map file id -> extent_list index (fmain.c:2789).
    if n == 10:                                           # fmain.c:2789, 10 = dragon file id
        i = 2                                             # fmain.c:2789, 2 = dragon extent slot
    elif n == 5:                                          # fmain.c:2789, 5 = turtle file id
        i = 1                                             # fmain.c:2789, 1 = turtle extent slot
    else:
        i = 0                                             # fmain.c:2789, 0 = swan extent slot
    if actor_file != n:                                   # fmain.c:2790 — swap shapes only on change
        nextshape = seq_list[ENEMY].location              # fmain.c:2791 — carriers share ENEMY shape RAM
        read_shapes(n)                                    # fmain.c:2792 — async disk load
        prep(an.type)                                     # fmain.c:2792 — wait + build mask table
        motor_off()                                       # fmain.c:2793
    an.abs_x = extent_list[i].x1 + 250                    # fmain.c:2795, 250 = extent half-width
    an.abs_y = extent_list[i].y1 + 200                    # fmain.c:2796, 200 = extent half-height
    an.index = 0
    an.weapon = 0
    an.environ = 0                                        # fmain.c:2797
    an.state = STATE_STILL
    an.vitality = 50                                      # fmain.c:2799, 50 = carrier HP
    anix = 4                                              # fmain.c:2800, 4 = expand actor count to cover slot 3
    an.race = n                                           # fmain.c:2801
    actor_file = n                                        # fmain.c:2801
    active_carrier = n                                    # fmain.c:2801
```

Notes:
- The swan/turtle/dragon share the same `anim_list[3]` slot and share the
  ENEMY shape memory (`seq_list[ENEMY].location`). Loading any carrier
  therefore invalidates any in-flight random enemy and vice versa — the
  carrier-present case is gated out of encounter rolls in
  [encounters.md](encounters.md#roll_wilderness_encounter).
- Position is set to `(extent.x1 + 250, extent.y1 + 200)`, i.e. the
  center of the 500×400 extent (see `move_extent` under `carrier_tick`).

## carrier_extent_update

Source: `fmain.c:2716-2719`
Called by: `game_tick` (Phase 12b, extent re-scan after xfer / place change)
Calls: `load_carrier`

```pseudo
def carrier_extent_update() -> None:
    """Auto-spawn / despawn the carrier in slot 3 on extent-type changes."""
    # Runs as the tail of the extent/xtype change handler after xtype has been
    # latched to extn.etype for the newly-entered extent (fmain.c:2682).
    if xtype < 70:                                        # fmain.c:2716, 70 = EXTENT_ETYPE_CARRIER
        active_carrier = 0                                # leaving a carrier extent despawns the carrier
        return
    if xtype != 70:                                       # fmain.c:2717 — other specials (>=70) are not carriers
        return
    # Reload only if nothing active, OR the hero isn't currently riding and
    # the requested carrier differs from the one sitting in the slot.
    reload = active_carrier == 0                          # fmain.c:2717
    if not reload and riding == 0 and actor_file != extn.v3:   # fmain.c:2717-2718
        reload = True
    if reload:
        load_carrier(extn.v3)                             # fmain.c:2718-2719 — extn.v3 = carrier file id
```

Notes:
- The carrier extents live at `extent_list[0..2]` with `etype == 70` and
  `v3 ∈ {5, 10, 11}` (swan/turtle/dragon): `fmain.c:339-341`. Other
  `etype >= 70` values reach this code path but are not carriers
  (`xtype == 83` = princess rescue, `xtype == 81/82` = king/sorceress
  placards), so the guard `xtype != 70` short-circuits them.
- The turtle extent starts degenerate at `(0,0)-(0,0)` (`fmain.c:340`)
  and is only ever entered after `get_turtle` / the cheat / the eggs
  aftermath repositions it via `move_extent(1, ...)`.
- When the hero is *riding* (`riding != 0`), re-entry of the same
  carrier extent does not reload, preventing the mount from being
  teleported back to extent center mid-ride.

## carrier_tick

Source: `fmain.c:1494-1547`
Called by: `actor_tick` (Phase 9, `type == CARRIER` branch)
Calls: `anim_list`, `newx`, `newy`, `px_to_im`, `move_extent`, `stuff`

```pseudo
def carrier_tick(i: int, an: Shape, d: int) -> None:
    """Per-tick body for the carrier actor (turtle or swan) in anim_list[3]."""
    # fmain.c:1494 — CARRIER body always clears the environ/terrain locals for
    # the caller (k,j) since carriers ignore normal terrain physics.
    k = 0                                                 # fmain.c:1494 — an.environ ignored this tick
    j = 0
    if actor_file == 11:                                  # fmain.c:1497, 11 = swan file id
        # --- Swan branch (fmain.c:1497-1509) -----------------------------
        if raftprox != 0 and wcarry == 3 and stuff[5] != 0:   # fmain.c:1498, 3 = carrier wcarry, stuff[5] = Golden Lasso
            # Mount: snap swan onto hero and latch riding (no motion commit;
            # the hero drives position via walk_step's ice branch while flying).
            d = anim_list[0].facing                       # fmain.c:1500
            an.abs_x = anim_list[0].abs_x                 # fmain.c:1501
            an.abs_y = anim_list[0].abs_y                 # fmain.c:1501
            xtest = an.abs_x
            ytest = an.abs_y
            riding = RIDING_SWAN                          # fmain.c:1502
        else:
            # Not mounting: swan holds position (no autonomous flight).
            xtest = an.abs_x                              # fmain.c:1506
            ytest = an.abs_y                              # fmain.c:1506
        an.index = d                                      # fmain.c:1508 — 8 frames, one per facing
        e = 0                                             # fmain.c:1509, 0 = swan extent index for move_extent
    else:
        # --- Turtle branch (fmain.c:1511-1542) ---------------------------
        if raftprox != 0 and wcarry == 3:                 # fmain.c:1513, 3 = carrier wcarry — no item needed
            # Mount: snap turtle onto hero and latch riding.
            d = anim_list[0].facing                       # fmain.c:1515
            xtest = anim_list[0].abs_x                    # fmain.c:1516
            ytest = anim_list[0].abs_y                    # fmain.c:1516
            riding = RIDING_TURTLE                         # fmain.c:1517, 5 = turtle riding value
            dex = d + d                                   # fmain.c:1518 — 2 frames per facing (16 total)
            if anim_list[0].state == STATE_WALKING:
                dex = dex + (cycle & 1)                   # fmain.c:1519 — alternate walk frame
            an.index = dex
        else:
            # Autonomous swim: try current direction, then +1, -2, -1
            # (fmain.c:1523-1535); keep only if destination is water.
            xtest = newx(an.abs_x, d, 3)                  # fmain.c:1523, 3 = turtle swim speed
            ytest = newy(an.abs_y, d, 3)                  # fmain.c:1524, 3 = turtle swim speed
            if px_to_im(xtest, ytest) != 5:               # fmain.c:1525, 5 = TERRAIN_WATER_VDEEP
                d = (d + 1) & 7                           # fmain.c:1527, 7 = DIR_MASK
                xtest = newx(an.abs_x, d, 3)              # fmain.c:1528, 3 = turtle swim speed
                ytest = newy(an.abs_y, d, 3)              # fmain.c:1529, 3 = turtle swim speed
                if px_to_im(xtest, ytest) != 5:           # fmain.c:1530, 5 = TERRAIN_WATER_VDEEP
                    d = (d - 2) & 7                       # fmain.c:1531, 7 = DIR_MASK
                    xtest = newx(an.abs_x, d, 3)          # fmain.c:1532, 3 = turtle swim speed
                    ytest = newy(an.abs_y, d, 3)          # fmain.c:1533, 3 = turtle swim speed
                    if px_to_im(xtest, ytest) != 5:       # fmain.c:1534, 5 = TERRAIN_WATER_VDEEP
                        d = (d - 1) & 7                   # fmain.c:1535, 7 = DIR_MASK
                        xtest = newx(an.abs_x, d, 3)      # fmain.c:1536, 3 = turtle swim speed
                        ytest = newy(an.abs_y, d, 3)      # fmain.c:1537, 3 = turtle swim speed
            riding = 0                                    # fmain.c:1538 — dismount: hero isn't close enough
            an.index = d + d + (cycle & 1)                # fmain.c:1539 — 16-frame walk cycle
        j = px_to_im(xtest, ytest)
        if j == 5:                                        # fmain.c:1542, 5 = TERRAIN_WATER_VDEEP
            an.abs_x = xtest                              # fmain.c:1542 — commit only onto water
            an.abs_y = ytest
        e = 1                                             # fmain.c:1543, 1 = turtle extent index for move_extent
    an.facing = d
    move_extent(e, xtest, ytest)                          # fmain.c:1544 — drag activation zone with the carrier
```

Notes:
- `move_extent(e, x, y)` rewrites `extent_list[e]` to `(x-250, y-200,
  x+250, y+200)` (`fmain2.c:1560-1565`). Dragging the extent along with
  the carrier keeps the region-entry check in
  `carrier_extent_update` latched while the hero is riding: the hero
  is always inside its own carrier's extent, so `xtype` stays at 70 and
  `active_carrier` stays set.
- The swan has no autonomous movement code; when not ridden it holds the
  position it was last placed at (by spawn or by the last ride's
  dismount).
- The turtle's swim is confined to terrain 5 (very-deep water) — any
  candidate on non-water is rejected. With all four candidates rejected
  the turtle stays in place (`xtest`/`ytest` from the last failed probe
  is still passed to `move_extent`, but the `j == 5` guard blocks the
  position commit).
- Rider animation while mounted is handled by the swan/turtle branches
  directly (`dex = d` for swan, `dex = d+d (+ cycle&1)` for turtle). The
  hero's own walk-cycle animation is suppressed while riding; see
  `actor_tick` in [game-loop.md](game-loop.md#actor_tick).

## raft_tick

Source: `fmain.c:1562-1573`
Called by: `actor_tick` (Phase 9, `type == RAFT` branch)
Calls: `anim_list`, `px_to_im`

```pseudo
def raft_tick(an: Shape) -> None:
    """Per-tick body for the raft actor (anim_list[1]). Snap to hero on water-edge."""
    riding = 0                                            # fmain.c:1563 — dismount unless we reacquire this tick
    if wcarry != 1 or raftprox != 2:                      # fmain.c:1564, 1 = raft wcarry, 2 = very-near threshold
        return                                            # fmain.c:1564 — not close enough to board
    xtest = anim_list[0].abs_x                            # fmain.c:1566
    ytest = anim_list[0].abs_y                            # fmain.c:1566
    j = px_to_im(xtest, ytest)                            # fmain.c:1568
    if j < 3 or j > 5:                                    # fmain.c:1569, 3..5 = water-adjacent terrain range
        return
    an.abs_x = xtest                                      # fmain.c:1570
    an.abs_y = ytest                                      # fmain.c:1571
    riding = RIDING_RAFT                                      # fmain.c:1572 — latch raft ride
```

Notes:
- Raft mount has no item requirement: standing adjacent (`raftprox == 2`,
  ~9 px) on terrain 3 (medium water), 4 (deep water), or 5 (very-deep
  water) is enough. Terrain 2 (shallow water) is excluded — you cannot
  board the raft from dry shore.
- The raft is dismounted implicitly each frame (`riding = 0` at entry);
  staying on the raft requires the proximity and terrain conditions to
  hold every frame. Walking off the water drops the ride.
- `wcarry == 1` is the condition "no carrier is loaded" (see
  `compute_raftprox`). An active swan/turtle therefore disables the raft
  entirely until the carrier is unloaded.

## swan_dismount

Source: `fmain.c:1417-1428`
Called by: `resolve_player_state` (Phase 7, fire-button / `pia` branch, when `riding == RIDING_SWAN`)
Calls: `proxcheck`, `event`, `anim_list`

```pseudo
def swan_dismount(dif_x: int, dif_y: int) -> None:
    """Attempt to put the swan down. Only called when the fire-button branch is taken with riding==11."""
    if fiery_death:                                       # fmain.c:1418 — lava-plain veto
        event(32)                                         # fmain.c:1418, 32 = "Ground is too hot for swan to land."
        return
    fast = dif_x <= -15 or dif_x >= 15                    # fmain.c:1419, 15 = dismount velocity gate
    fast = fast or dif_y <= -15 or dif_y >= 15            # fmain.c:1419, 15 = dismount velocity gate
    if fast:
        event(33)                                         # fmain.c:1427, 33 = "Flying too fast to dismount."
        return
    ytest = anim_list[0].abs_y - 14                       # fmain.c:1420, 14 = landing y-offset (land above current)
    upper_clear = proxcheck(anim_list[0].abs_x, ytest, 0) == 0         # fmain.c:1421
    lower_clear = proxcheck(anim_list[0].abs_x, ytest + 10, 0) == 0    # fmain.c:1421, 10 = foot-probe offset
    if upper_clear and lower_clear:
        riding = 0                                        # fmain.c:1423 — dismount commit
        anim_list[0].abs_y = ytest                        # fmain.c:1424
```

Notes:
- `dif_x` / `dif_y` are the hero's current velocity components
  (`anim_list[0].vel_x` / `vel_y`), captured by the caller at
  `fmain.c:1411-1412`. Because swan flight uses the ice branch of
  `walk_step`, these can reach ±40 (y) / ±32 (x) before the cap
  engages. The ±15 gate therefore allows dismount only after the player
  has released the joystick long enough for velocity to decay.
- `fiery_death` is the lava-box latch set each frame at
  `fmain.c:1384-1385`: `map_x ∈ (8802, 13562) && map_y ∈ (24744, 29544)`.
- The two `proxcheck` probes verify both the hero's torso position
  (`ytest`) and the foot position (`ytest + 10`) are clear; the `−14`
  offset lifts the hero slightly so the landing sprite doesn't overlap
  the ground tile the swan was flying above.
- On successful dismount the swan itself stays at its last position in
  `anim_list[3]` (no snap-out), and will stand there until the player
  either re-enters its extent (despawn on `xtype < 70`) or re-mounts.

## use_sea_shell

Source: `fmain.c:3457-3461`
Called by: `option_handler` (CMODE_USE, `hit == 6`)
Calls: `get_turtle`

```pseudo
def use_sea_shell() -> None:
    """USE Sea Shell: outside the swamp block, delegate to get_turtle to spawn the turtle carrier."""
    if not hitgo:                                         # fmain.c:3457 — slot empty: no shell to use
        return
    # Swamp-rectangle veto (fmain.c:3458). The shell is silently inert in this box.
    in_swamp = hero_x < 21373 and hero_x > 11194          # fmain.c:3458, 11194/21373 = swamp x-range
    in_swamp = in_swamp and hero_y < 16208 and hero_y > 10205  # fmain.c:3458, 10205/16208 = swamp y-range
    if in_swamp:
        return                                            # fmain.c:3458 — 'break' out of the case without calling get_turtle
    get_turtle()                                          # fmain.c:3460 — delegate: see quests.md#get_turtle
```

Notes:
- `hit == 6` selects the Sea Shell inventory slot (`stuff[6]`). The
  shell is obtained by talking to the turtle with `active_carrier == 5`
  (`fmain.c:3418-3421`), which grants it on first encounter (speak 56)
  and acknowledges on subsequent (speak 57).
- `get_turtle` rolls up to 25 random ring locations, keeps the first
  that lands on terrain 5 (very-deep water), calls
  `move_extent(1, x, y)` to reposition the degenerate turtle extent,
  and then calls `load_carrier(5)`. Full logic lives in
  [quests.md#get_turtle](quests.md#get_turtle); this handler is just
  the gating wrapper.
- The swamp veto is a hardcoded coordinate rectangle, not a sector test.
  Using the shell anywhere inside it is a no-op (not even a beep).
