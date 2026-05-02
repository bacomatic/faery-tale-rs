# Game Loop — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c
> Cross-refs: [ARCHITECTURE.md](../ARCHITECTURE.md), [_discovery/game-loop.md](../_discovery/game-loop.md), [RESEARCH §2](../RESEARCH.md#17-main-game-loop)

## Overview

The game is driven by a single `while (!quitflag)` loop at `fmain.c:1270-2621`.
Every iteration of that loop is one game tick. Frame rate is implicitly locked
to the Amiga vertical blank via `WaitBOVP(&vp_text)` inside `pagechange`, so
one loop iteration equals one rendered frame (≈50 Hz PAL, ≈60 Hz NTSC) unless
the loop short-circuits on an early `continue`.

The tick is split into **24 ordered phases**. Phases 1–13 always run (subject
to the three early-exit gates: viewstatus 1/2/4 at Phase 2, pause at Phase 4,
and the viewstatus-1/4 `continue` inside Phase 2). Phase 14 ("no-motion")
runs only on ticks where the scroll delta is zero and so is the carrier of
most periodic game logic — AI, encounter spawning, day/night advance,
hunger, fatigue. Phases 15–24 run unconditionally on every non-short-circuited
tick. Phase numbers in this document are the canonical anchors that every
other logic doc in `reference/logic/` references when it says "runs in phase N".

## Symbols

This file introduces no new local structs. All identifiers resolve in
[SYMBOLS.md](SYMBOLS.md) or are declared in the `Calls:` header of each
function (functions and globals deferred to later waves; see the
"Deferred Calls" section of the accompanying orchestrator report).

## game_tick

Source: `fmain.c:1270-2621`
Called by: `entry point`
Calls: `getkey`, `process_input_key`, `decode_mouse`, `game_paused`, `decrement_timers`, `update_fiery_death_zone`, `resolve_player_state`, `update_carrier_proximity`, `actor_tick`, `update_post_actor_state`, `check_bed_sleep`, `check_door`, `redraw_or_scroll`, `melee_hit_detection`, `missile_tick`, `do_objects`, `inject_missile_sprites`, `sort_sprites`, `repair_scroll_strip`, `apply_witch_fx`, `render_sprites`, `page_flip`, `finalize_fade`, `cycle`, `flasher`, `anix`, `quitflag`

```pseudo
def game_tick() -> None:
    """One iteration of the main while(!quitflag) loop — the canonical 24-phase sequence."""
    # Phase 1 — tick counters (fmain.c:1274-1275)
    cycle = cycle + 1                                     # fmain.c:1274
    flasher = flasher + 1                                 # fmain.c:1275

    # Phase 2 — input decode + transient-view gate (fmain.c:1277-1374)
    key = getkey()                                        # fmain.c:1277
    if process_input_key(key):
        return                                            # fmain.c:1373 — viewstatus 1/4 short-circuit

    # Phase 3 — mouse / joystick decode (fmain.c:1376)
    decode_mouse()                                        # fmain.c:1376

    # Phase 4 — pause gate (fmain.c:1378); Delay(1) + skip rest of tick
    if game_paused():
        return                                            # fmain.c:1378

    # Phase 5 — timers (fmain.c:1380-1382)
    decrement_timers()                                    # fmain.c:1380-1382

    # Phase 6 — fiery-death zone flag (fmain.c:1384-1385)
    update_fiery_death_zone()                             # fmain.c:1384-1385

    # Phase 7 — player state resolution (fmain.c:1387-1459)
    resolve_player_state()                                # fmain.c:1387-1459

    # Phase 8 — carrier (raft/turtle/swan) proximity (fmain.c:1462-1472)
    update_carrier_proximity()                            # fmain.c:1462-1472

    # Phase 9 — actor processing loop (fmain.c:1476-1826)
    i = 0
    while i < anix:                                       # fmain.c:1476
        actor_tick(i)
        i = i + 1

    # Phase 10 — post-actor globals (fmain.c:1829-1833)
    update_post_actor_state()                             # fmain.c:1829-1833

    # Phase 11 — bed / sleep check (fmain.c:1835-1849, inside only)
    check_bed_sleep()                                     # fmain.c:1835-1849

    # Phase 12 — door/stair/cave transitions (fmain.c:1853-1955)
    check_door()                                          # fmain.c:1853-1955

    # Phase 13 — redraw or scroll dispatch (fmain.c:1959-2394)
    #            Phase 14 ("no-motion tick") runs inside this call when dif_x == 0 and dif_y == 0.
    redraw_or_scroll()                                    # fmain.c:1959-2394

    # Phase 15 — melee hit detection (fmain.c:2262-2296)
    melee_hit_detection()                                 # fmain.c:2262-2296

    # Phase 16 — missile tick (fmain.c:2298-2340)
    missile_tick()                                        # fmain.c:2298-2340

    # Phase 17 — world-object update (fmain.c:2342-2343)
    do_objects()                                          # fmain.c:2343

    # Phase 18 — inject in-flight missiles as OBJECTS sprites (fmain.c:2345-2362)
    inject_missile_sprites()                              # fmain.c:2345-2362

    # Phase 19 — sprite sort + nearest-person scan (fmain.c:2367-2393)
    sort_sprites()                                        # fmain.c:2367-2393

    # Phase 20 — fill in newly-exposed scroll strip/row (fmain.c:2396-2397)
    repair_scroll_strip()                                 # fmain.c:2396-2397

    # Phase 21 — witch visual effects (fmain.c:2399-2410)
    apply_witch_fx()                                      # fmain.c:2399-2410

    # Phase 22 — sprite rendering (fmain.c:2412-2609)
    render_sprites()                                      # fmain.c:2412-2609

    # Phase 23 — page flip (fmain.c:2611-2614) — blocks on vertical blank
    page_flip()                                           # fmain.c:2613

    # Phase 24 — finish any pending fade-in (fmain.c:2615)
    finalize_fade()                                       # fmain.c:2615
```

## process_input_key

Source: `fmain.c:1278-1374`
Called by: `game_tick`
Calls: `key_dispatch`, `Delay`, `ppick`, `set_flash_color`, `flasher`, `viewstatus`

```pseudo
def process_input_key(key: int) -> bool:
    """Phase 2. Dispatch one raw key and service the viewstatus 1/2/4 sub-loops.
    Returns True when the caller should skip the remainder of the tick (the
    `continue` at fmain.c:1373)."""
    # fmain.c:1280 — all keyboard / mouse routing lives in the menu-system doc.
    if key != 0:
        key_dispatch(key)
    # fmain.c:1365-1366 — viewstatus 2: fixed 200-tick placard, then force redraw.
    if viewstatus == 2:                                   # fmain.c:1365, 2 = placard hold
        Delay(200)                                        # fmain.c:1366, 200 = ~4s placard hold
        viewstatus = 99                                   # fmain.c:1366, 99 = corrupt/redraw sentinel
        return False
    # fmain.c:1367-1374 — viewstatus 1 (big map) or 4 (pickup placard): flash color 31, pump print queue, short-circuit.
    if viewstatus == 1 or viewstatus == 4:                # fmain.c:1367, 1 = big map, 4 = pickup wait
        if viewstatus == 1:
            set_flash_color(flasher)                      # fmain.c:1368-1370 — toggles color 31 on bit 4 of flasher
        ppick()                                           # fmain.c:1371 — service one print-queue entry
        return True                                       # fmain.c:1373 — `continue` in the original
    return False
```

## resolve_player_state

Source: `fmain.c:1387-1459`
Called by: `game_tick`
Calls: `revive`, `event`, `proxcheck`, `rand`, `fire_button_down`, `joy_walk_held`, `spawn_fairy_sprite`, `goodfairy`, `luck`, `riding`, `fiery_death`, `oldir`, `hunger`, `STATE_SINK`, `STATE_SLEEP`

```pseudo
def resolve_player_state() -> None:
    """Phase 7. Compute the player's next motion state from input + current state."""
    s = player.state                                      # fmain.c:1387
    # --- Dead / Fall branch: good-fairy resurrection ladder (fmain.c:1388-1407)
    if s == STATE_DEAD or s == STATE_FALL:
        if goodfairy == 1:                                # fmain.c:1390, 1 = immediate revive
            revive(False)
            s = STATE_STILL
        else:
            goodfairy = goodfairy - 1
            if goodfairy >= 20:                           # fmain.c:1391, 20 = end-of-effect threshold
                if luck < 1 and goodfairy < 200:          # fmain.c:1392, 200 = normal fairy cap
                    revive(True)
                    s = STATE_STILL
                elif player.state == STATE_FALL and goodfairy < 200:  # fmain.c:1393, 200 = fairy cap
                    revive(False)
                    s = STATE_STILL
                elif goodfairy < 120:                     # fmain.c:1395, 120 = fairy-sprite visible window
                    spawn_fairy_sprite()                  # fmain.c:1396-1405 — anim_list[3] fairy overlay
        player.state = s
        return

    # --- Locked-in animation states (fmain.c:1408): DYING / SINK / SLEEP run their course untouched.
    if s == STATE_DYING or s == STATE_SINK or s == STATE_SLEEP:
        return

    # --- Fire button / keyfight / joystick button held (fmain.c:1409-1439)
    if fire_button_down():
        dx = player.vel_x
        dy = player.vel_y
        if xtype > 80:                                    # fmain.c:1413, 80 = shooting-gallery xtype floor
            if s != STATE_SHOOT1 and xtype == 81:         # fmain.c:1414, 81 = archery target 1
                event(15)                                 # fmain.c:1414, 15 = target-1 narration
            if s != STATE_SHOOT1 and xtype == 82:         # fmain.c:1415, 82 = archery target 2
                event(16)                                 # fmain.c:1415, 16 = target-2 narration
            s = STATE_SHOOT1
        elif riding == 11:                                # fmain.c:1417, 11 = riding swan
            if fiery_death:
                event(32)                                 # fmain.c:1418, 32 = swan-in-fire narration
            else:
                near_still = (dx > -15 and dx < 15 and dy > -15 and dy < 15)  # fmain.c:1419, 15 = low-velocity dismount gate
                if near_still:
                    ytest = player.abs_y - 14             # fmain.c:1421, 14 = dismount y-offset
                    clear_a = (proxcheck(player.abs_x, ytest, 0) == 0)
                    clear_b = (proxcheck(player.abs_x, ytest + 10, 0) == 0)  # fmain.c:1423, 10 = body-height probe
                    if clear_a and clear_b:
                        riding = 0
                        player.abs_y = ytest              # fmain.c:1425 — swan dismount
                else:
                    event(33)                             # fmain.c:1428, 33 = can't-dismount narration
        else:
            # fmain.c:1431-1438 — melee / bow / wand selection by weapon slot
            if oldir < 9:                                 # fmain.c:1432, 9 = "no joystick direction" sentinel
                player.facing = oldir
            if s >= STATE_WALKING:
                if player.weapon == 4:                    # fmain.c:1434, 4 = bow
                    s = STATE_SHOOT1
                elif player.weapon == 5:                  # fmain.c:1435, 5 = wand
                    if s < STATE_SHOOT1:
                        s = STATE_SHOOT1
                else:
                    s = STATE_FIGHTING
        player.state = s
        return

    # --- No fire button ------------------------------------------------------
    if s == STATE_SHOOT1:
        player.state = STATE_SHOOT3                       # fmain.c:1440 — release the arrow
        return

    # --- Walk / stand (fmain.c:1441-1458)
    if oldir < 9:                                         # fmain.c:1442, 9 = no-direction sentinel
        if hunger > 120 and rand(0, 15) == 0:             # fmain.c:1443, 120 = starvation deviation floor
            if (rand(0, 1) & 1) != 0:                     # fmain.c:1445
                oldir = (oldir + 1) & 7                   # fmain.c:1445, 7 = 8-direction mask
            else:
                oldir = (oldir - 1) & 7                   # fmain.c:1446, 7 = 8-direction mask
        player.facing = oldir
        if joy_walk_held() or keydir != 0:                # fmain.c:1448 — qualifier bit 0x4000 OR keypad latch
            s = STATE_WALKING
    else:
        s = STATE_STILL
    player.state = s
```

## actor_tick

Source: `fmain.c:1476-1826`
Called by: `game_tick`
Calls: `actor_type_dispatch`, `walk_step`, `still_step`, `shoot_step`, `fighting_step`, `death_step`, `checkdead`, `update_environ`, `update_actor_index`, `wrap_player_coords`, `map_adjust`, `compute_rel_coords`, `freeze_timer`, `STATE_SINK`

```pseudo
def actor_tick(i: int) -> None:
    """Phase 9. One per-actor update: type dispatch → motion/state step → environ → sprite index → rel coords."""
    actor = anim_list[i]
    # fmain.c:1481 — time-stop freezes everyone except the hero (i==0).
    if freeze_timer != 0 and i > 0:
        compute_rel_coords(actor, i)                      # fmain.c:1755-1768 — statc label
        return

    # fmain.c:1485-1572 — type-specific early branches (dragon / carrier / setfig / raft).
    # OBJECTS actors jump straight to the environment/index phase.
    handled = actor_type_dispatch(i)                      # returns True if type branch fully handled this tick
    if not handled:
        # fmain.c:1573-1745 — state-specific body for ENEMY and PHIL.
        s = actor.state
        if s == STATE_SINK:
            death_step(i, s)                              # fmain.c:1573-1576
        elif s == STATE_WALKING:
            walk_step(i)                                  # fmain.c:1577-1665 — newx/newy + proxcheck + door probe
        elif s == STATE_STILL:
            still_step(i)                                 # fmain.c:1666-1676
        elif s == STATE_SHOOT1 or s == STATE_SHOOT3:
            shoot_step(i, s)                              # fmain.c:1677-1717
        elif s == STATE_FIGHTING:
            fighting_step(i)                              # fmain.c:1718-1735
        else:
            death_step(i, s)                              # fmain.c:1736-1745 — DYING/DEAD/FROZEN/OSCIL/SLEEP/FALL

    # fmain.c:1747-1758 — dying-countdown → DEAD + loot drops (race 0x09 drops talisman 139; race 0x89 drops lasso 27).
    if actor.state == STATE_DYING:
        checkdead(i, 0)

    # fmain.c:1761-1797 — terrain ↔ environ mapping (water wading, flight, backwards-walk, fall, drowning teleport).
    update_environ(i)

    # fmain.c:1798-1820 — race-specific animation frame overrides (rabbit hop, swarm bug, dead dark knight).
    update_actor_index(i)

    # fmain.c:1821-1838 — hero-only world-wrap, safe_flag capture, fiery-death damage, lava-pool sink damage.
    if i == 0:
        wrap_player_coords()                              # fmain.c:1828-1838
        map_adjust(player.abs_x, player.abs_y)            # fmain.c:1839

    # fmain.c:1755-1768 — statc label: compute rel_x/rel_y for rendering.
    compute_rel_coords(actor, i)
```

## actor_type_dispatch

Source: `fmain.c:1480-1574`
Called by: `actor_tick`
Calls: `set_course`, `carrier_tick`, `raft_tick`, `effect`, `rand`, `setfig_table`, `missile_list`, `mdex`, `bowshotx`, `gunshoty`, `frustflag`, `witchflag`, `hero_x`, `hero_y`, `OBJECTS`, `DRAGON`, `CARRIER`, `SETFIG`, `RAFT`, `STATE_DYING`, `STATE_DEAD`, `STATE_TALKING`, `STATE_STILL`

```pseudo
def actor_type_dispatch(i: i16, an: Shape, d: i8, s: i8, k: i8) -> i8:
    """fmain.c:1480-1574 — actor-type early branches inside actor_tick.
    Returns the post-dispatch routing code:
      0 = fall through to the state dispatcher (run walk/still/shoot/fight/death step)
      1 = goto `raise`  (skip state dispatch; still run update_environ + update_actor_index)
      2 = goto `statc`  (skip both update_environ and update_actor_index)
    Mutates an.index, an.facing, an.state, an.abs_x/y, an.tactic, plus
    the global mount/missile state, in place."""

    if an.type == OBJECTS:                                # fmain.c:1480
        return 1                                          # → raise (no state step; OBJECTS animate via update_actor_index path only)

    if an.type == DRAGON:                                 # fmain.c:1481-1494
        an.index = 0                                      # fmain.c:1482 — idle frame
        if s == STATE_DYING:                              # fmain.c:1483
            an.index = 3                                  # 3 = dragon DYING frame
        elif s == STATE_DEAD:                             # fmain.c:1484
            an.index = 4                                  # 4 = dragon DEAD frame
        elif rand(0, 3) == 0:                             # fmain.c:1485 — 4-in-N fire-breath gate (within hostile extent)
            ms = missile_list[mdex]
            ms.speed = 5                                  # fmain.c:1486, 5 = dragon-fire missile speed
            mdex = mdex + 1
            an.index = rand(1, 2)                         # fmain.c:1487, 1..2 = breath-anim frames
            effect(5, 1800 + rand(0, 255))                # fmain.c:1488, 5 = dragon-fire SFX, 1800 = base pitch
            an.facing = 5                                 # fmain.c:1489, 5 = DIR_S (fixed-south spit)
            ms.missile_type = 2                           # fmain.c:1490, 2 = dragon-fire missile type
            # Spawn the missile inline at the dragshoot label (fmain.c:1698-1707).
            ms.abs_x = an.abs_x + bowshotx[an.facing]     # fmain.c:1699
            ms.abs_y = an.abs_y + gunshoty[an.facing]     # fmain.c:1702 — fireball uses gunshoty
            ms.time_of_flight = 0
            ms.direction = an.facing
            ms.archer = i
            if mdex > 5:                                  # fmain.c:1706, 5 = missile-list cap-1
                mdex = 0
            frustflag = 0
            return 1                                      # → raise (skip state dispatcher)
        an.index = 0                                      # fmain.c:1493 — idle frame
        return 1                                          # → raise

    if an.type == CARRIER:                                # fmain.c:1495-1547 — see carrier_tick
        # See [carrier-transport.md#carrier_tick](carrier-transport.md#carrier_tick) for the full
        # swan/turtle body. The branch ends with `goto raise` at fmain.c:1546,
        # so the caller still runs update_environ + update_actor_index.
        carrier_tick(i)
        return 1                                          # → raise

    if an.type == SETFIG:                                 # fmain.c:1548-1561
        sid = an.race & 0x7f                              # fmain.c:1549, 0x7f = setfig id mask (high bit reserved)
        an.index = setfig_table[sid].image_base           # fmain.c:1550
        if s == STATE_DYING:                              # fmain.c:1551
            an.index = an.index + 2                       # 2 = setfig DYING frame offset
            if sid == 9:                                  # 9 = SETFIG_WITCH (bigger sprite sheet)
                an.index = an.index + 2
        elif s == STATE_DEAD:                             # fmain.c:1552
            an.index = an.index + 3                       # 3 = setfig DEAD frame offset
            if sid == 9:                                  # 9 = SETFIG_WITCH
                an.index = an.index + 2
        elif sid == 9:                                    # fmain.c:1553 — 9 = SETFIG_WITCH (faces hero each tick)
            set_course(i, hero_x, hero_y, 0)              # fmain.c:1554, 0 = face-toward target mode
            an.index = an.facing / 2                      # fmain.c:1554, 2 = frames per facing pair
            witchflag = True                              # fmain.c:1554 — witch present this frame
        elif s == STATE_TALKING:                          # fmain.c:1555
            an.index = an.index + rand(0, 1)              # fmain.c:1556 — talk-anim 2-frame jitter
            an.tactic = an.tactic - 1                     # fmain.c:1557 — talk countdown
            if an.tactic == 0:
                an.state = STATE_STILL                    # fmain.c:1557 — talk timed out
        else:
            return 2                                      # fmain.c:1559 — `goto statc` (skip env + index update)
        return 1                                          # → raise

    if an.type == RAFT:                                   # fmain.c:1562-1574 — see raft_tick
        # See [carrier-transport.md#raft_tick](carrier-transport.md#raft_tick); the branch always
        # exits via `goto statc` at fmain.c:1573 (no env / index update).
        raft_tick(i)
        return 2                                          # → statc

    return 0                                              # fall through to state dispatcher
```

Notes:
- The dragon's missile-spawn happens inline at the `dragshoot` label
  (`fmain.c:1698-1707`) which is shared with `shoot_step`. Both paths
  emit the same `bowshotx`/`gunshoty` deltas and increment `mdex`; only
  the entry conditions differ. The shoot_step pseudocode below repeats
  the spawn block for clarity.
- The CARRIER and RAFT branches are flattened into one-line calls to
  the existing `carrier_tick` / `raft_tick` specs in
  [carrier-transport.md](carrier-transport.md). Inlining the bodies here
  would duplicate ~70 lines of mount/dismount logic; the routing-code
  return faithfully captures the `goto raise` vs `goto statc` exit
  choice each branch makes.

## shoot_step

Source: `fmain.c:1667-1717`
Called by: `actor_tick` (Phase 9, `STATE_SHOOT1` / `STATE_SHOOT3` branch)
Calls: `effect`, `print`, `rand`, `diroffs`, `bowshotx`, `bowshoty`, `gunshoty`, `missile_list`, `mdex`, `stuff`, `xtype`, `ENEMY`, `WEAPON_BOW`, `WEAPON_WAND`, `STATE_STILL`, `STATE_SHOOT1`, `STATE_SHOOT3`

```pseudo
def shoot_step(i: i16, an: Shape, d: i8, s: i8) -> None:
    """fmain.c:1667-1717 — bow/wand attack body. Picks the shoot-pose
    frame from diroffs[d+8] + offset, releases the missile on
    SHOOT3 (bow) or immediately on SHOOT1 (wand), and transitions
    the actor's state. Mutates an.index, an.state, missile_list, and
    mdex; falls into the `cpx` tail (no position commit)."""
    # fmain.c:1669 — ranged attacks gated: deep water (k>15) or pax extents (xtype>80) suppress.
    if an.environ > 15 or xtype > 80:                     # fmain.c:1669, 15 = SINK threshold, 80 = pax-extent floor
        return                                            # → cpx tail (no missile, no anim change)

    ms = missile_list[mdex]

    if s == STATE_SHOOT3:                                 # fmain.c:1670 — release frame
        if an.weapon == 5:                                # fmain.c:1671, 5 = WEAPON_WAND (no SHOOT3 anim)
            an.index = diroffs[d + 8]                     # fmain.c:1672 — fight-base frame for facing
            return                                        # → cpx
        an.index = diroffs[d + 8] + 11                    # fmain.c:1675, 11 = bow-shoot-release frame offset
        an.state = STATE_STILL                            # fmain.c:1676 — bow snaps back to STILL after release
        if i == 0:                                        # fmain.c:1677 — hero arrow inventory check
            if stuff[8] == 0:                             # fmain.c:1677, 8 = ARROW slot in stuff[]
                return                                    # → cpx (no arrows, no spawn)
            stuff[8] = stuff[8] - 1                       # fmain.c:1677 — consume one arrow
        ms.speed = 3                                      # fmain.c:1678, 3 = arrow speed
        mdex = mdex + 1
        effect(4, 400 + rand(0, 255))                     # fmain.c:1680, 4 = bowstring SFX, 400 = base pitch
    elif s == STATE_SHOOT1:                               # fmain.c:1682 — draw frame
        if an.type == ENEMY:                              # fmain.c:1683
            an.index = diroffs[d + 8] + 11                # fmain.c:1683, 11 = ENEMY draw frame
        else:
            an.index = diroffs[d + 8] + 10                # fmain.c:1684, 10 = PHIL draw frame
        if an.weapon == 5:                                # fmain.c:1685, 5 = WEAPON_WAND (single-frame fire)
            ms.speed = 5                                  # fmain.c:1686, 5 = fireball speed
            mdex = mdex + 1
            an.state = STATE_SHOOT3                       # fmain.c:1688 — wand fires on SHOOT1 entry
            an.index = diroffs[d + 8]                     # fmain.c:1689 — wand-cast pose
            effect(5, 1800 + rand(0, 255))                # fmain.c:1690, 5 = wand-fire SFX, 1800 = base pitch
        elif i == 0 and stuff[8] == 0:                    # fmain.c:1693 — hero with no arrows
            print("No Arrows!")                           # fmain.c:1694 — narration (see [messages.md](messages.md))
            return                                        # → cpx (no draw-anim, no spawn)
        else:
            ms.speed = 0                                  # fmain.c:1695 — bow draw: missile not yet in flight

    # fmain.c:1697 — common missile-spawn tail (also entered from DRAGON via dragshoot label).
    ms.missile_type = an.weapon - 3                       # fmain.c:1697, 3 = WEAPON_BOW-1 (bow→1, wand→2)
    ms.abs_x = an.abs_x + bowshotx[d]                     # fmain.c:1699
    ms.abs_y = an.abs_y                                   # fmain.c:1700
    if an.weapon == 4:                                    # fmain.c:1701, 4 = WEAPON_BOW
        ms.abs_y = ms.abs_y + bowshoty[d]                 # fmain.c:1701
    else:
        ms.abs_y = ms.abs_y + gunshoty[d]                 # fmain.c:1702 — wand fireball / dragon
    ms.time_of_flight = 0                                 # fmain.c:1703
    ms.direction = an.facing                              # fmain.c:1704
    ms.archer = i                                         # fmain.c:1705
    if mdex > 5:                                          # fmain.c:1706, 5 = missile-slot cap-1
        mdex = 0                                          # fmain.c:1706 — wrap
    frustflag = 0                                         # fmain.c:1707 — successful action clears frustration
    # → cpx tail: no position commit, falls through to update_environ.
```

Notes:
- The `ms.speed = 0` path on a SHOOT1 bow draw means the missile slot
  is reserved but the missile is not yet airborne; the next tick's
  SHOOT3 release writes the real speed (3) and increments `mdex` for
  the next slot. The rendering side keeps the same `bow_x[]`/`bow_y[]`
  weapon-overlay offset across both ticks.
- `STATE_SHOOT1 → STATE_SHOOT3` happens inside `resolve_player_state`
  for the hero (release on fire-button up, see `fmain.c:1440`), and
  inside `advance_goal` for NPCs. `shoot_step` only handles the frame
  selection and missile spawn — it does not advance the state itself
  except for the wand single-frame collapse at `fmain.c:1688`.

## fighting_step

Source: `fmain.c:1710-1716`
Called by: `actor_tick` (Phase 9, melee state branch — `s` strictly less than `STATE_SHOOT1` and ≥ 9)
Calls: `diroffs`, `trans_list`, `rand`

```pseudo
def fighting_step(i: i16, an: Shape, d: i8, s: i8) -> None:
    """fmain.c:1710-1716 — melee swing animation step. Picks the next
    fight-state via trans_list[s].newstate[rand4()]; writes the
    corresponding diroffs[d+8] + s-derived frame to an.index. Falls
    into `cpx` (no position commit). The actual hit detection is in
    melee_hit_detection (Phase 15), not here — this step only animates
    the swing."""
    inum = diroffs[d + 8]                                 # fmain.c:1711 — fight-base frame for facing
    s = trans_list[s].newstate[rand(0, 3)]                # fmain.c:1712 — pick next fight-substate (0..3)
    an.state = s                                          # fmain.c:1712
    if i > 2 and (s == 6 or s == 7):                      # fmain.c:1713, 2 = leader+brothers slot count, 6/7 = NPC-restricted fight substates
        s = 8                                             # fmain.c:1713 — collapse 6/7 to 8 for actors beyond the brothers
    an.index = s + inum                                   # fmain.c:1714
    frustflag = 0                                         # fmain.c:1715 — successful swing clears frust
    # → cpx tail (no position commit).
```

Notes:
- `trans_list` is the per-state transition table that picks one of four
  successor fight-substates each tick, producing the wind-up → swing →
  recovery cycle. The four-entry rolls give a non-deterministic but
  bounded animation.
- The `i > 2` guard reserves fight substates 6 and 7 (the leader/brother
  block) for the hero and brothers (anim_list[0..2]); enemy NPCs collapse
  to substate 8 to share a smaller fight-frame block.
- This step does not consult `an.weapon` — the weapon overlay is chosen
  at render time by `select_atype_inum` from the current `an.index`.

## death_step

Source: `fmain.c:1573-1576, 1718-1746`
Called by: `actor_tick` (Phase 9, `STATE_SINK` / `STATE_DYING` / `STATE_DEAD` / `STATE_FROZEN` / `STATE_OSCIL` / `STATE_OSCIL+1` / `STATE_SLEEP` / `STATE_FALL` branches)
Calls: `fallstates`, `cycle`, `brother`

```pseudo
def death_step(i: i16, an: Shape, d: i8, s: i8) -> None:
    """fmain.c:1573-1576, 1718-1746 — pose selection for non-motile
    states. Each state pins an.index to a specific frame (or alternates
    between two via `cycle`); FALL also advances tactic and decays
    velocity. Falls into `cpx` (no position commit, but ice momentum
    still slides through cpx)."""
    if s == STATE_SINK:                                   # fmain.c:1575-1578
        if an.vitality > 0:
            an.index = 83                                 # fmain.c:1576, 83 = sinking pose (statelist[83])
        # fmain.c:1577 — vitality<=0 leaves an.index untouched; corpse stays in last pose.
        frustflag = 0                                     # fmain.c:1577
        return                                            # → cpx

    if s == STATE_DYING:                                  # fmain.c:1719-1726
        if an.tactic > 4:                                 # fmain.c:1720, 4 = death-anim midpoint
            if d == 0 or d > 4:                           # fmain.c:1721, facings 0/5/6/7 = north half
                an.index = 80                             # fmain.c:1721, 80 = death frame A
            else:
                an.index = 81                             # fmain.c:1721, 81 = death frame B
        elif an.tactic > 0:                               # fmain.c:1722
            if d == 0 or d > 4:                           # fmain.c:1723, 4 = facing-half boundary
                an.index = 81                             # fmain.c:1723 — second-half flips A/B
            else:
                an.index = 80                             # fmain.c:1723, 80 = death frame A
        else:                                             # fmain.c:1724 — countdown expired
            an.state = STATE_DEAD
            an.index = 82                                 # fmain.c:1724, 82 = corpse frame
        frustflag = 0                                     # fmain.c:1725
        return                                            # → cpx

    if s == STATE_DEAD:                                   # fmain.c:1727
        an.index = 82                                     # 82 = corpse frame
        return                                            # → cpx

    if s == STATE_FROZEN:                                 # fmain.c:1728
        an.index = 82                                     # 82 = same as DEAD pose (ice statue)
        return                                            # → cpx

    if s == STATE_OSCIL:                                  # fmain.c:1729 — sword-at-side oscillation
        an.index = 84 + (cycle & 1)                       # fmain.c:1729, 84/85 = OSCIL frames
        return                                            # → cpx

    if s == STATE_OSCIL + 1:                              # fmain.c:1730
        an.index = 84                                     # fmain.c:1730 — held A pose
        return                                            # → cpx

    if s == STATE_SLEEP:                                  # fmain.c:1731
        an.index = 86                                     # fmain.c:1731, 86 = sleeping (statelist[86])
        return                                            # → cpx

    if s == STATE_FALL:                                   # fmain.c:1732-1738
        if an.tactic >= 30:                               # fmain.c:1733, 30 = fall-frame counter cap
            return                                        # → cpx (frozen at last fall frame)
        j = (an.tactic / 5) + (brother * 6)               # fmain.c:1734, 5 = ticks-per-frame, 6 = entries-per-brother
        an.index = fallstates[j]                          # fmain.c:1735
        an.tactic = an.tactic + 1                         # fmain.c:1736 — advance fall counter
        an.vel_x = (an.vel_x * 3) / 4                     # fmain.c:1737, 3/4 = velocity decay per fall tick
        an.vel_y = (an.vel_y * 3) / 4                     # fmain.c:1738
        return                                            # → cpx (cpx tail still applies vel/4 if k==-2)

    # No match: leave an.index untouched (defensive — original falls through to cpx).
    return
```

Notes:
- `STATE_DYING` reuses `an.tactic` as the death-anim countdown (4
  ticks each at the high frame, 4 ticks at the low frame, then DEAD).
  The transition from DYING to DEAD also runs the post-step decrement
  at `fmain.c:1747-1757` which fires `checkdead` for race-specific
  loot drops; that is shown in the actor_tick spec above and not
  duplicated here.
- `STATE_FALL` uses `an.tactic` as a fall-frame counter (0..29). The
  `fallstates[brother*6 + tactic/5]` lookup means each of the six fall
  frames per brother lasts five ticks; the 30-tick cap freezes the
  last frame indefinitely until the caller transitions out of FALL
  (typically via `update_environ`'s pit/water handling).
- `STATE_OSCIL+1` is a hold-pose convention: the actor pauses on
  frame 84 indefinitely until a state transition takes them out of
  OSCIL+1.

## update_actor_index

Source: `fmain.c:1799-1824`
Called by: `actor_tick` (after `update_environ`, before `compute_rel_coords`)
Calls: `diroffs`, `cycle`

```pseudo
def update_actor_index(i: i16, an: Shape, d: i8, dex: i16) -> None:
    """fmain.c:1799-1824 — race-specific frame overrides applied after
    the state-step has set `dex`. Some races animate at their own
    cadence regardless of state (the swarming bug, the rabbit's hop,
    the dead dark knight). The final value is written through
    an.index so render_sprites and select_atype_inum read the
    overridden frame."""
    k = an.race                                           # fmain.c:1802
    if an.type == ENEMY:                                  # fmain.c:1803
        if k == 4 and an.state < STATE_WALKING:           # fmain.c:1804, 4 = RACE_SNAKE; pre-walk pose
            dex = (cycle & 1) + diroffs[d]                # fmain.c:1804 — snake idle: 2-frame body wiggle on walk-base
        elif k == 4 and an.state < STATE_DYING:           # fmain.c:1805 — snake while alive
            dex = ((cycle / 2) & 1) + diroffs[d]          # fmain.c:1805 — half-rate wiggle
        elif k == 8:                                      # fmain.c:1806, 8 = RACE_BUG_SWARM
            if an.state == STATE_DEAD:                    # fmain.c:1807
                an.abs_x = 0                              # fmain.c:1807 — kill swarm: park off-screen
            elif an.state == STATE_DYING:                 # fmain.c:1808
                dex = 0x3f                                # fmain.c:1808, 0x3f = swarm-dying frame
            else:
                dex = (cycle & 3) * 2                     # fmain.c:1810 — 4-phase swarm cycle, doubled
                if dex > 4:                               # fmain.c:1811
                    dex = dex - 1                         # fmain.c:1811 — collapse 6 → 5
                slot = i % 3                              # fmain.c:1812 — three swarm-instances per cluster
                if slot == 0:                             # fmain.c:1813
                    dex = 0x25                            # fmain.c:1813, 0x25 = swarm body A (static)
                elif slot == 1:                           # fmain.c:1814
                    dex = dex + 0x28                      # fmain.c:1814, 0x28 = swarm body B base
                else:
                    dex = dex + 0x30                      # fmain.c:1815, 0x30 = swarm body C base
        elif k == 7 and an.vitality == 0:                 # fmain.c:1819, 7 = RACE_DARK_KNIGHT (zero-HP undead)
            an.state = STATE_STILL                        # fmain.c:1820 — note: source has bug `an->state == STILL` (no-op compare); spec follows the *intended* assignment
            dex = 1                                       # fmain.c:1821, 1 = dark-knight reanimation pose
    an.index = dex                                        # fmain.c:1824 — final frame written for renderer
```

Notes:
- Source bug at `fmain.c:1820`: the original is `an->state == STILL;`
  (an equality test discarded as an expression statement) where
  `an->state = STILL;` was clearly intended. The spec writes the
  assignment because that is the observable behavior in playtesting
  (race-7 zero-HP knights stop moving). A faithful port can choose to
  reproduce the no-op or write the assignment; the difference is only
  observable on the rare frame where a race-7 actor has just hit
  vitality 0 but is still in WALKING/FIGHTING. See [PROBLEMS.md](../PROBLEMS.md).
- The bug-swarm logic at `fmain.c:1812-1816` is the only animation
  path that uses `i % 3` — it lets three sequential anim_list slots
  (i, i+1, i+2) display three different swarm body frames, producing
  the appearance of a milling cluster from three independent actors.
- Race overrides run *after* update_environ, which means a race-4
  snake that just stepped onto water still gets the snake-idle frame
  override applied this same tick; the environ-driven render shifts
  in `compute_shape_clip` then pull the snake into the wading pose
  visually.

## compute_rel_coords

Source: `fmain.c:1852-1864`
Called by: `actor_tick` (statc label — the unconditional tail of every per-actor tick)
Calls: `wrap`, `map_x`, `map_y`

```pseudo
def compute_rel_coords(an: Shape, i: i16) -> None:
    """fmain.c:1852-1864 — write the per-actor screen-relative anchor
    (rel_x, rel_y) used by render_sprites. Three anchor offsets are
    selected by actor type / mount mode, all computed modulo the
    map-wrap window via `wrap`."""
    if an.type == CARRIER and riding == 11:               # fmain.c:1853, 11 = RIDING_SWAN
        an.rel_x = wrap(an.abs_x - map_x - 32)            # fmain.c:1854, 32 = swan-mount X anchor (wider sprite)
        an.rel_y = wrap(an.abs_y - map_y - 40)            # fmain.c:1855, 40 = swan-mount Y anchor
    elif an.type == RAFT or an.type == CARRIER or an.type == DRAGON:   # fmain.c:1857
        an.rel_x = wrap(an.abs_x - map_x - 16)            # fmain.c:1858, 16 = mount/large-sprite X anchor
        an.rel_y = wrap(an.abs_y - map_y - 16)            # fmain.c:1859, 16 = mount Y anchor
    else:
        an.rel_x = wrap(an.abs_x - map_x - 8)             # fmain.c:1862, 8 = standard actor X anchor (16-wide sprite, centered)
        an.rel_y = wrap(an.abs_y - map_y - 26)            # fmain.c:1863, 26 = standard Y anchor (sprite top from feet-Y)
```

Notes:
- The three anchor sets correspond to the three sprite-sheet sizes:
  swan-mount (64×80, anchor 32/40), other carriers/raft/dragon
  (32×32 large, anchor 16/16), and standard actors (16×32, anchor
  8/26 — the 26 places the sprite's feet at the actor's `abs_y`,
  matching `GROUND_OFFSET = 32` minus the 6-pixel feet-to-bottom
  margin in the standard PHIL/ENEMY sheets).
- `wrap` (see [SYMBOLS.md](SYMBOLS.md)) collapses negative and
  oversized values into the 16-bit world-coordinate window so a
  sprite straddling the world wrap renders at the correct screen
  column.

## wrap_player_coords

Source: `fmain.c:1826-1841`
Called by: `actor_tick` (i == 0 only, after update_actor_index)
Calls: `map_adjust`, `anim_list`

```pseudo
def wrap_player_coords(an: Shape, j: i8) -> None:
    """fmain.c:1826-1841 — hero-only world-edge wrap. When the hero
    walks past one of the four region boundaries (region_num<8 only),
    snap them to the opposite edge; if mounted on a carrier, drag
    anim_list[3] (the swan/turtle slot) along. Always writes
    hero_x/hero_y and refreshes map_adjust for terrain-tile lookups."""
    if region_num < 8:                                    # fmain.c:1827, 8 = inside-buildings region id
        wrapped = True                                    # tracks whether we hit any edge
        if an.abs_x < 300:                                # fmain.c:1828, 300 = west-edge wrap threshold
            an.abs_x = 32565                              # fmain.c:1828, 32565 = east-edge spawn X
        elif an.abs_x > 32565:                            # fmain.c:1829
            an.abs_x = 300                                # fmain.c:1829, 300 = west-edge spawn X
        elif an.abs_y < 300:                              # fmain.c:1830
            an.abs_y = 32565                              # fmain.c:1830, 32565 = south-edge spawn Y
        elif an.abs_y > 32565:                            # fmain.c:1831
            an.abs_y = 300                                # fmain.c:1831, 300 = north-edge spawn Y
        else:
            wrapped = False                               # fmain.c:1832 — `goto jkl` skip
        if wrapped and riding > 1:                        # fmain.c:1833, 1 = ride-on-raft (no carrier slot to drag)
            anim_list[3].abs_x = an.abs_x                 # fmain.c:1834 — drag swan/turtle to wrapped position
            anim_list[3].abs_y = an.abs_y                 # fmain.c:1835
    map_adjust(an.abs_x, an.abs_y)                        # fmain.c:1839 — refresh hero_x/hero_y + sector caches
    safe_flag = j                                         # fmain.c:1840 — capture last-sampled terrain code for sleep/eat checks (j is the local terrain code computed in actor_tick before this tail; see [actor_tick](#actor_tick))
```

Notes:
- The wrap thresholds 300 / 32565 are world-pixel coordinates: 300
  is one in-game tile inside the western edge, 32565 is the symmetric
  inset on the east. The 32-pixel margin prevents the hero from
  spawning inside the regional border tiles.
- The `if (riding > 1)` test excludes ride-on-raft (`riding == 1`)
  because the raft is `anim_list[1]` and is dragged by a separate
  path; it includes turtle (5) and swan (11) which both occupy
  `anim_list[3]`.
- Inside-buildings regions (`region_num >= 8`) skip the wrap entirely:
  building interiors are room-shaped with door tiles as the only
  exit, so out-of-bounds positions cannot occur in normal play.

## check_door

Source: `fmain.c:1853-1955`
Called by: `game_tick`
Calls: `xfer`, `find_place`, `fade_page`, `DOORCOUNT`, `doorlist`, `riding`, `region_num`, `new_region`, `stuff`, `STATBASE`, `DOOR_DESERT`, `DOOR_CAVE`, `pagecolors`

```pseudo
def check_door() -> None:
    """Phase 12. Detect a door/stair/cave straddle and transfer the hero to the other side."""
    # fmain.c:1853-1856 — grid-aligned hero position used as the search key.
    xtest = hero_x & 0xfff0                               # fmain.c:1856, 0xfff0 = 16-pixel grid mask (x)
    ytest = hero_y & 0xffe0                               # fmain.c:1857, 0xffe0 = 32-pixel grid mask (y)
    if riding != 0:
        return                                            # fmain.c:1859 — no door use while mounted

    if region_num < 8:                                    # fmain.c:1860, 8 = inside-buildings region id
        # fmain.c:1861-1899 — outdoor regions: binary search on doorlist[] sorted by xc1.
        lo = 0
        hi = DOORCOUNT - 1
        while hi >= lo:
            j = (lo + hi) // 2                            # fmain.c:1865
            d = doorlist[j]
            if d.xc1 > xtest:
                hi = j - 1
            elif d.xc1 + 16 < xtest:                      # fmain.c:1869, 16 = one-grid-cell tolerance
                lo = j + 1
            elif d.xc1 < xtest and (d.type & 1) == 0:     # fmain.c:1870, bit 0 = horizontal door
                lo = j + 1
            elif d.yc1 > ytest:
                hi = j - 1
            elif d.yc1 < ytest:
                lo = j + 1
            else:
                # fmain.c:1876-1895 — straddle confirmed.
                if (d.type & 1) != 0:                     # fmain.c:1877 — horizontal door
                    if (hero_y & 16) != 0:                # fmain.c:1878, 16 = low-row bit inside a 32-px cell
                        break
                else:
                    if (hero_x & 15) > 6:                 # fmain.c:1879, 15 = sub-cell mask; 6 = north-side threshold
                        break
                if d.type == DOOR_DESERT and stuff[STATBASE] < 5:  # fmain.c:1881, 5 = desert-gate stats wall
                    break
                if d.type == DOOR_CAVE:                   # fmain.c:1882
                    xtest = d.xc2 + 24                    # fmain.c:1882, 24 = cave landing x-offset
                    ytest = d.yc2 + 16                    # fmain.c:1882, 16 = cave landing y-offset
                elif (d.type & 1) != 0:                   # fmain.c:1884 — horizontal landing
                    xtest = d.xc2 + 16                    # fmain.c:1884, 16 = landing x-offset
                    ytest = d.yc2
                else:
                    xtest = d.xc2 - 1
                    ytest = d.yc2 + 16                    # fmain.c:1885, 16 = vertical landing y-offset
                if d.secs == 1:                           # fmain.c:1887, 1 = destination is inside-region 8
                    new_region = 8                        # fmain.c:1887, 8 = inside
                else:
                    new_region = 9                        # fmain.c:1887, 9 = dungeon/cave
                xfer(xtest, ytest, False)
                find_place(2)                             # fmain.c:1889, 2 = full-refresh mode
                fade_page(100, 100, 100, True, pagecolors)  # fmain.c:1890, 100 = neutral grey palette weights
                break
            if lo >= DOORCOUNT or hi < 0:                 # fmain.c:1894
                break
        return

    # fmain.c:1900-1954 — indoor regions (region_num >= 8): linear scan, xc2/yc2 are the inside endpoint.
    j = 0
    while j < DOORCOUNT:
        d = doorlist[j]
        horiz_match = (d.xc2 == xtest - 16 and (d.type & 1) != 0)  # fmain.c:1906, 16 = landing offset
        if d.yc2 == ytest and (d.xc2 == xtest or horiz_match):
            if (d.type & 1) != 0:                         # fmain.c:1908
                if (hero_y & 16) == 0:                    # fmain.c:1909, 16 = low-row bit
                    break
            else:
                if (hero_x & 15) < 2:                     # fmain.c:1910, 15 = sub-cell mask; 2 = south-side threshold
                    break
            if d.type == DOOR_CAVE:                       # fmain.c:1912
                xtest = d.xc1 - 4                         # fmain.c:1912, 4 = cave exit x-offset
                ytest = d.yc1 + 16                        # fmain.c:1912, 16 = cave exit y-offset
            elif (d.type & 1) != 0:                       # fmain.c:1914
                xtest = d.xc1 + 16                        # fmain.c:1914, 16 = horizontal exit x-offset
                ytest = d.yc1 + 34                        # fmain.c:1914, 34 = horizontal exit y-offset
            else:
                xtest = d.xc1 + 20                        # fmain.c:1915, 20 = vertical exit x-offset
                ytest = d.yc1 + 16                        # fmain.c:1915, 16 = vertical exit y-offset
            xfer(xtest, ytest, True)
            find_place(False)
            break
        j = j + 1
```

## redraw_or_scroll

Source: `fmain.c:1959-2394`
Called by: `game_tick`
Calls: `OwnBlitter`, `DisownBlitter`, `rest_blit`, `witch_fx`, `load_next`, `gen_mini`, `map_draw`, `scrollmap`, `no_motion_tick`, `MAP_FLUX`, `fp_drawing`, `map_x`, `map_y`, `img_x`, `img_y`, `dif_x`, `dif_y`

```pseudo
def redraw_or_scroll() -> None:
    """Phase 13. Paint the playfield — full redraw (viewstatus transition),
    1-tile scroll, or no-motion tick (Phase 14)."""
    hero_x = player.abs_x                                 # fmain.c:1959
    hero_y = player.abs_y                                 # fmain.c:1960
    # fmain.c:1963-1970 — restore background under last frame's sprites.
    OwnBlitter()
    q = fp_drawing.obcount
    while q > 0:
        rest_blit(fp_drawing.shape_queue[q - 1].backsave)
        q = q - 1
    DisownBlitter()
    # fmain.c:1973-1974 — undo the witch wavy-FX on the drawing page before we scroll/paint.
    if fp_drawing.wflag:
        witch_fx(fp_drawing)

    # fmain.c:1976-1985 — scroll delta in tile units, sign-extended 10-bit value.
    img_x = map_x >> 4                                    # fmain.c:1976, 4 = log2(16-px tile width)
    img_y = map_y >> 5                                    # fmain.c:1977, 5 = log2(32-px tile height)
    dif_x = img_x - fp_drawing.isv_x                      # fmain.c:1978
    dif_y = img_y - fp_drawing.isv_y                      # fmain.c:1979
    if (dif_x & 0x200) != 0:                              # fmain.c:1980, 0x200 = sign bit of 10-bit delta
        dif_x = dif_x | 0xfc00                            # fmain.c:1980, 0xfc00 = sign-extension mask
    else:
        dif_x = dif_x & 0x3ff                             # fmain.c:1981, 0x3ff = 10-bit mask

    # fmain.c:1987 — kick the incremental region loader if a region swap is in flight.
    if MAP_FLUX():
        load_next()

    # fmain.c:1989-1994 — full redraw sentinels.
    if viewstatus == 99 or viewstatus == 98 or viewstatus == 3:  # fmain.c:1989, 99/98/3 — see Notes
        gen_mini()
        map_draw()
        dif_x = 0
        dif_y = 0
        if viewstatus == 99:                              # fmain.c:1991, 99 = corrupt sentinel
            viewstatus = 98                               # fmain.c:1991, 98 = one-more-tick redraw
        elif viewstatus == 98:                            # fmain.c:1992, 98 = one-more-tick redraw
            viewstatus = 0
        no_motion_tick()                                  # fmain.c:1994 `goto viewchange`
        fp_drawing.isv_x = img_x
        fp_drawing.isv_y = img_y
        return

    if dif_x != 0 or dif_y != 0:
        gen_mini()                                        # fmain.c:1995 — re-derive minimap for new scroll

    # fmain.c:1996-2394 — single-tile scroll dispatch; no-motion only when both deltas are zero.
    if dif_x == 1 and dif_y == 1:
        scrollmap(5)                                      # fmain.c:1997, 5 = up-left
    elif dif_x == 1 and dif_y == 0:
        scrollmap(4)                                      # fmain.c:1998, 4 = left
    elif dif_x == 1 and dif_y == -1:
        scrollmap(3)                                      # fmain.c:1999, 3 = down-left
    elif dif_x == 0 and dif_y == 1:
        scrollmap(6)                                      # fmain.c:2002, 6 = up
    elif dif_x == 0 and dif_y == -1:
        scrollmap(2)                                      # fmain.c:2003, 2 = down
    elif dif_x == 0 and dif_y == 0:
        no_motion_tick()                                  # fmain.c:2006 — Phase 14
    elif dif_x == -1 and dif_y == 1:
        scrollmap(7)                                      # fmain.c:2263, 7 = up-right
    elif dif_x == -1 and dif_y == 0:
        scrollmap(0)                                      # fmain.c:2264, 0 = right
    elif dif_x == -1 and dif_y == -1:
        scrollmap(1)                                      # fmain.c:2265, 1 = down-right
    else:
        # fmain.c:2000/2261/2267 — any larger delta: drop the scroll and force a full map_draw().
        dif_x = 0
        dif_y = 0
        map_draw()

    # fmain.c:2269-2270 — commit the scroll anchor for the drawing page.
    fp_drawing.isv_x = img_x
    fp_drawing.isv_y = img_y
```

## no_motion_tick

Source: `fmain.c:2007-2260`
Called by: `redraw_or_scroll`
Calls: `ppick`, `day_fade`, `find_place`, `prep`, `motor_off`, `load_actors`, `event`, `advance_goal`, `aftermath`, `setmood`, `CheckDiskIO`, `prq`, `danger_level`, `encounter_number`, `actors_loading`, `hunger`, `fatigue`, `witchflag`, `safe_r`, `safe_x`, `safe_y`, `safe_flag`, `stuff`, `last_person`, `brave`, `ob_list8`, `active_carrier`, `freeze_timer`, `region_num`, `sleep_loop`, `daynight_tick`, `place_extent_encounters`, `roll_encounter_type`, `tick_hunger_fatigue`, `STATE_SLEEP`

```pseudo
def no_motion_tick() -> None:
    """Phase 14 — runs when dif_x == 0 and dif_y == 0. Carries most of the
    periodic per-tick game logic (AI, encounters, hunger, day/night)."""
    # 14a — print queue (fmain.c:2009)
    ppick()

    # 14b — sleep advancement (fmain.c:2012-2021)
    if player.state == STATE_SLEEP:
        sleep_loop()                                      # deferred — sleep/wake rules

    # 14c — day/night cycle (fmain.c:2022-2037); daynight wraps at 24000 ticks/day.
    daynight_tick()                                       # deferred to logic/time.md
    # Period-change narration and the lightlevel/ob_listg[5] update happen inside daynight_tick.

    # 14d — palette fade (fmain.c:2039)
    day_fade()

    # 14e — vitality regeneration (fmain.c:2041-2046) — every 1024 daynight ticks.
    if (daynight & 0x3ff) == 0:                           # fmain.c:2041, 0x3ff = 1024-tick mask
        if player.vitality < (15 + brave // 4) and player.state != STATE_DEAD:  # fmain.c:2042, 15 = base cap; 4 = bravery regen divisor
            player.vitality = player.vitality + 1
            prq(4)                                        # fmain.c:2044, 4 = stat-bar redraw request

    # 14f — time-stop short-circuit (fmain.c:2048). Skip AI, encounters, hunger while frozen.
    if freeze_timer != 0:
        return

    # 14g — find_place: recompute hero_sector / hero_place / xtype (fmain.c:2050)
    find_place(2)                                         # fmain.c:2050, 2 = full refresh

    # 14h — actor-loading completion (fmain.c:2052-2056)
    if actors_loading and CheckDiskIO(8):                 # fmain.c:2052, 8 = actor-disk-IO channel
        prep(ENEMY)
        motor_off()
        actors_loading = False
        anix = 3                                          # fmain.c:2055, 3 = reset back to player+raft+setfig

    # 14i — scripted-extent encounter placement (fmain.c:2057-2077), every 16 ticks.
    if (daynight & 15) == 0 and encounter_number != 0 and not actors_loading:  # fmain.c:2057, 15 = 16-tick cadence
        place_extent_encounters()                         # deferred to logic/encounters.md

    # 14j — random wilderness encounter (fmain.c:2078-2095), every 32 ticks.
    if ((daynight & 31) == 0                              # fmain.c:2078, 31 = 32-tick cadence
            and not actors_on_screen
            and not actors_loading
            and active_carrier == 0
            and xtype < 50):                              # fmain.c:2078, 50 = special-extent floor
        if region_num > 7:                                # fmain.c:2082, 7 = last outdoor region id
            danger_level = 5 + xtype                      # fmain.c:2082, 5 = indoor danger bias
        else:
            danger_level = 2 + xtype                      # fmain.c:2083, 2 = outdoor danger bias
        if rand(0, 63) <= danger_level:                   # fmain.c:2084, 63 = rand64 range
            roll_encounter_type()                         # deferred to logic/encounters.md
            load_actors()

    # 14k — NPC proximity narration (fmain.c:2096-2106)
    if nearest_person != 0:
        k = anim_list[nearest_person].race
        if k != last_person:
            if k == 0x8d:                                 # fmain.c:2099, 0x8d = beggar race
                speak(23)                                 # fmain.c:2099, 23 = beggar greeting
            elif k == 0x89:                               # fmain.c:2100, 0x89 = witch race
                speak(46)                                 # fmain.c:2100, 46 = witch taunt
            elif k == 0x84:                               # fmain.c:2101, 0x84 = princess race
                if ob_list8[9].ob_stat != 0:              # fmain.c:2101, 9 = princess object index
                    speak(16)                             # fmain.c:2101, 16 = princess greeting
            elif k == 9:                                  # fmain.c:2102, 9 = necromancer race
                speak(43)                                 # fmain.c:2102, 43 = necromancer line
            elif k == 7:                                  # fmain.c:2103, 7 = dark-knight race
                speak(41)                                 # fmain.c:2103, 41 = dark knight line
            last_person = k

    # 14l — AI loop (fmain.c:2107-2211). advance_goal lives in logic/ai-system.md.
    actors_on_screen = False
    leader = 0
    battle2 = battleflag
    battleflag = False
    i = 2                                                 # fmain.c:2111 — skip player (0) and raft (1)
    while i < anix:
        an = anim_list[i]
        if goodfairy != 0 and goodfairy < 120:            # fmain.c:2113, 120 = fairy-effect cutoff
            break
        advance_goal(an)
        if leader == 0 and an.type != CARRIER and an.type != SETFIG and an.vitality >= 1:
            leader = i                                    # fmain.c:2183
        i = i + 1

    # 14m — battle state transitions (fmain.c:2212-2215)
    if not battle2 and battleflag:
        setmood(1)                                        # fmain.c:2212, 1 = battle-music mode
    if not battleflag and battle2:
        prq(7)                                            # fmain.c:2213, 7 = aftermath text region
        prq(4)                                            # fmain.c:2213, 4 = stat-bar redraw
        aftermath()

    # 14n — safe-zone update (fmain.c:2216-2224), every 128 ticks.
    safe_outdoors = (not actors_on_screen and not actors_loading and not witchflag
                     and player.environ == 0 and safe_flag == 0 and player.state != STATE_DEAD)
    if (daynight & 127) == 0 and safe_outdoors:           # fmain.c:2216, 127 = 128-tick cadence
        safe_r = region_num
        safe_x = hero_x
        safe_y = hero_y
        if hunger > 30 and stuff[24] != 0:                # fmain.c:2221, 30 = auto-eat hunger gate; 24 = food slot
            stuff[24] = stuff[24] - 1                     # fmain.c:2221, 24 = food inventory slot
            hunger = hunger - 30                          # fmain.c:2222, 30 = food-ration relief
            event(37)                                     # fmain.c:2222, 37 = "you eat some food"

    # 14o — ambient music refresh (fmain.c:2225), every 8 ticks out of combat.
    if (daynight & 7) == 0 and not battleflag:            # fmain.c:2225, 7 = 8-tick cadence
        setmood(0)

    # 14p — hunger / fatigue (fmain.c:2226-2258), every 128 ticks when alive and awake.
    if (daynight & 127) == 0 and player.vitality != 0 and player.state != STATE_SLEEP:  # fmain.c:2226, 127 = cadence
        tick_hunger_fatigue()                             # deferred to logic/stats.md
```

## melee_hit_detection

Source: `fmain.c:2262-2296`
Called by: `game_tick`
Calls: `newx`, `newy`, `dohit`, `effect`, `rand`, `bitrand`, `freeze_timer`, `brave`

```pseudo
def melee_hit_detection() -> None:
    """Phase 15. For every actor whose current frame is NOT WALKING (i.e. in a swing/fight/fire pose),
    test weapon-reach proximity against all other actors and call dohit on contact."""
    if freeze_timer != 0:
        return                                            # fmain.c:2262 gate is inside the outer guard below
    i = 0
    while i < anix:
        if i > 0 and freeze_timer != 0:                   # fmain.c:2263 — time-stop only the hero may hit
            break
        # fmain.c:2265 — skip raft (slot 1) and any actor still in its walk cycle.
        if i == 1 or anim_list[i].state >= STATE_WALKING:
            i = i + 1
            continue
        wt = anim_list[i].weapon
        fc = anim_list[i].facing
        if (wt & 4) != 0:                                 # fmain.c:2268, bit 2 = bow (ranged; handled in missile_tick)
            i = i + 1
            continue
        if wt >= 8:                                       # fmain.c:2269, 8 = touch-attack flag; cap reach
            wt = 5                                        # fmain.c:2269, 5 = tuned-down touch range
        wt = wt + bitrand(2)                              # fmain.c:2270, 2 = 4-value coin flip (0..3 extra reach)
        xs = newx(anim_list[i].abs_x, fc, wt + wt) + rand(0, 7) - 3   # fmain.c:2271, 7 = rand8 range; 3 = jitter centre
        ys = newy(anim_list[i].abs_y, fc, wt + wt) + rand(0, 7) - 3   # fmain.c:2272, 7 = rand8 range; 3 = jitter centre
        if i == 0:
            bv = (brave // 20) + 5                        # fmain.c:2273, 20 = bravery scale; 5 = base hero reach
        else:
            bv = 2 + rand(0, 3)                           # fmain.c:2273, 3 = rand4 range; 2 = enemy reach base
        if bv > 14:                                       # fmain.c:2274, 14 = high-bravery soft cap
            bv = 15                                       # fmain.c:2274, 15 = hard ceiling on melee reach
        j = 0
        while j < anix:
            same_or_raft = (j == 1 or j == i)
            dead = (anim_list[j].state == STATE_DEAD)
            if same_or_raft or dead or anim_list[i].type == CARRIER:
                j = j + 1
                continue
            xd = abs(anim_list[j].abs_x - xs)
            yd = abs(anim_list[j].abs_y - ys)
            if xd > yd:
                yd = xd                                   # fmain.c:2283 — Chebyshev distance
            # fmain.c:2284-2287 — player always rolls to hit; enemies gated by rand256 > brave.
            hit_ok = (i == 0) or (rand(0, 255) > brave)   # fmain.c:2284, 255 = rand256 range
            if hit_ok and yd < bv and freeze_timer == 0:
                dohit(i, j, fc, wt)                       # deferred to logic/combat.md
                break
            elif yd < bv + 2 and wt != 5:                 # fmain.c:2287, 2 = near-miss band; 5 = touch weapon
                effect(1, 150 + rand(0, 255))             # fmain.c:2287, 1 = clang sound; 150 = base pitch; 255 = pitch jitter
            j = j + 1
        i = i + 1
```

## missile_tick

Source: `fmain.c:2298-2340`
Called by: `game_tick`
Calls: `px_to_im`, `dohit`, `newx`, `newy`, `bitrand`, `freeze_timer`, `brave`, `missile_list`

```pseudo
def missile_tick() -> None:
    """Phase 16. Advance every entry in missile_list[0..5]: age it out, test terrain,
    test victim proximity, call dohit on contact, then step position by speed*2."""
    if freeze_timer != 0:
        return                                            # fmain.c:2298 — time-stop freezes missiles too
    i = 0
    while i < 6:                                          # fmain.c:2298, 6 = max concurrent missiles
        ms = missile_list[i]
        s = ms.speed * 2
        # fmain.c:2303-2305 — expire missile if type 0/3 (unused/spent), speed 0, or aged past 40 ticks.
        ms.time_of_flight = ms.time_of_flight + 1
        if ms.missile_type == 0 or ms.missile_type == 3 or s == 0 or ms.time_of_flight > 40:  # fmain.c:2303, 40 = max flight ticks
            ms.missile_type = 0
            i = i + 1
            continue

        # fmain.c:2307-2309 — terrain test; impassable (1) or solid (15) kills the missile in place.
        terrain_code = px_to_im(ms.abs_x, ms.abs_y)
        if terrain_code == 1 or terrain_code == 15:       # fmain.c:2308, 15 = solid-terrain code
            ms.missile_type = 0
            s = 0

        fc = ms.direction
        if ms.missile_type == 2:                          # fmain.c:2311, 2 = fireball
            mt = 9                                        # fmain.c:2311, 9 = fireball hit radius
        else:
            mt = 6                                        # fmain.c:2310, 6 = arrow hit radius

        j = 0
        while j < anix:
            if j == 0:
                bv = brave                                # fmain.c:2316, hero dodges by bravery
            else:
                bv = 20                                   # fmain.c:2316, 20 = generic monster dodge
            skip = (j == 1 or ms.archer == j
                    or anim_list[j].state == STATE_DEAD
                    or anim_list[j].type == CARRIER)
            if skip:
                j = j + 1
                continue
            xd = abs(anim_list[j].abs_x - ms.abs_x)
            yd = abs(anim_list[j].abs_y - ms.abs_y)
            if xd > yd:
                yd = xd                                   # fmain.c:2324 — Chebyshev distance
            # fmain.c:2326 — missile 0 always checks; higher missiles gated by bitrand(512) > bravery.
            miss_rolls = (i == 0) or (bitrand(512) > bv)  # fmain.c:2326, 512 = bitrand range
            if miss_rolls and yd < mt:
                if ms.missile_type == 2:                  # fmain.c:2327, 2 = fireball
                    dohit(-2, j, fc, rand(0, 7) + 4)      # fmain.c:2327, -2 = fireball source; 7 = rand8 range; 4 = base damage
                else:
                    dohit(-1, j, fc, rand(0, 7) + 4)      # fmain.c:2328, -1 = arrow source
                ms.speed = 0
                if ms.missile_type == 2:                  # fmain.c:2330, 2 = fireball
                    ms.missile_type = 3                   # fmain.c:2330, 3 = spent/puff frame
                break
            j = j + 1

        # fmain.c:2335-2336 — step missile forward by 2*speed pixels along its direction.
        ms.abs_x = newx(ms.abs_x, fc, s)
        ms.abs_y = newy(ms.abs_y, fc, s)
        i = i + 1
```

## sort_sprites

Source: `fmain.c:2367-2393`
Called by: `game_tick`
Calls: `calc_dist`, `anim_index`, `anix2`, `riding`

```pseudo
def sort_sprites() -> None:
    """Phase 19. Bubble sort anim_index[0..anix2-1] by adjusted Y for back-to-front
    rendering, and pick the closest living NPC as nearest_person for speech triggers."""
    nearest_person = 0
    perdist = 50                                          # fmain.c:2370, 50 = speech proximity radius
    i = 0
    while i < anix2:
        anim_index[i] = i
        i = i + 1

    i = 0
    while i < anix2:
        an = anim_list[i]
        # fmain.c:2373-2376 — nearest-person pick; skip slot 0, OBJECTS, corpses, swan rider.
        if i != 0 and an.type != OBJECTS and an.state != STATE_DEAD and riding != 11:  # fmain.c:2374, 11 = swan mount
            d = calc_dist(i, 0)
            if d < perdist:
                perdist = d
                nearest_person = i

        # fmain.c:2378-2391 — bubble-swap pass, comparing y with a few render-order biases.
        j = 1
        while j < anix2:
            k1 = anim_index[j - 1]
            k2 = anim_index[j]
            y1 = anim_list[k1].abs_y
            y2 = anim_list[k2].abs_y
            if anim_list[k1].state == STATE_DEAD or (k2 == 0 and riding != 0) or k1 == 1:
                y1 = y1 - 32                              # fmain.c:2385, 32 = render-order ground bias
            if anim_list[k2].state == STATE_DEAD or (k1 == 0 and riding != 0) or k2 == 1:
                y2 = y2 - 32                              # fmain.c:2386, 32 = render-order ground bias
            if anim_list[k1].environ > 25:                # fmain.c:2387, 25 = sinking-threshold environ value
                y1 = y1 + 32                              # fmain.c:2387, 32 = sink-under bias
            if anim_list[k2].environ > 25:                # fmain.c:2388, 25 = sinking-threshold environ value
                y2 = y2 + 32                              # fmain.c:2388, 32 = sink-under bias
            if y2 < y1:
                anim_index[j - 1] = k2
                anim_index[j] = k1
            j = j + 1
        i = i + 1
```

## render_sprites

Source: `fmain.c:2412-2609`
Called by: `game_tick`
Calls: `OwnBlitter`, `DisownBlitter`, `WaitBlit`, `clear_blit`, `save_blit`, `mask_blit`, `shape_blit`, `resolve_pass_params`, `needs_weapon_pass`, `select_atype_inum`, `compute_shape_clip`, `compute_terrain_mask`, `should_apply_terrain_mask`, `reserve_save_slot`, `fp_drawing`, `bmask_mem`, `CBK_SIZE`, `anim_index`, `anix2`

Helpers spec'd in [sprite-rendering.md](sprite-rendering.md):
[`resolve_pass_params`](sprite-rendering.md#resolve_pass_params),
[`needs_weapon_pass`](sprite-rendering.md#needs_weapon_pass),
[`select_atype_inum`](sprite-rendering.md#select_atype_inum),
[`compute_shape_clip`](sprite-rendering.md#compute_shape_clip) (which calls
[`compute_sprite_size`](sprite-rendering.md#compute_sprite_size) and applies the
OBJECTS half-height inum-list at `fmain.c:2477-2480`),
[`reserve_save_slot`](sprite-rendering.md#reserve_save_slot),
[`should_apply_terrain_mask`](sprite-rendering.md#should_apply_terrain_mask),
[`compute_terrain_mask`](sprite-rendering.md#compute_terrain_mask) (which
contains the `blithigh = 32` override at `fmain.c:2570`).

```pseudo
def render_sprites() -> None:
    """Phase 22. Walk anim_index[] in sorted order; for every on-screen actor
    blit the character (pass 0) and optionally the wielded weapon (pass 1)
    with terrain-driven vertical masking."""
    fp_drawing.obcount = 0                                # fmain.c:2510
    fp_drawing.saveused = 0                               # fmain.c:2510
    crack = 0                                             # fmain.c:2510 — free-slot index into planes[]
    backalloc = fp_drawing.backsave                       # fmain.c:2511

    j = 0
    while j < anix2:
        i = anim_index[j]
        an = anim_list[i]
        an.visible = 0

        passmode = 0
        pass_count = 0
        # Two-pass loop: body (pass 0), weapon overlay (pass 1 if armed).
        done = 0
        while done == 0:
            # fmain.c:2418-2432 — choose pass params based on weapon, facing, state.
            passmode = resolve_pass_params(an, passmode, pass_count)
            # fmain.c:2446-2498 — type/frame dispatch: hero, enemy, carrier, dragon, setfig, falling, fiery-death.
            clip = compute_shape_clip(an, i, passmode)
            # fmain.c:2499-2505 — off-screen reject.
            if clip.xstart > 319 or clip.ystart > 173 or clip.xstart + 15 < 0 or clip.ystart + 15 < 0:  # fmain.c:2504, 319/173 = playfield extents; 15 = sprite half-width
                if pass_count == 1:
                    pass_count = 0
                    passmode = passmode + 1
                    continue
                done = 1
                continue
            # fmain.c:2512-2528 — reserve a save-under buffer; bail if out of room.
            shp = fp_drawing.shape_queue[fp_drawing.obcount]
            if not reserve_save_slot(shp, crack, backalloc):
                done = 1
                continue
            # fmain.c:2534-2583 — terrain-driven vertical mask build.
            OwnBlitter()
            WaitBlit()
            clear_blit(bmask_mem, CBK_SIZE)
            DisownBlitter()
            if should_apply_terrain_mask(an, i, clip):
                compute_terrain_mask(an, clip)
            # fmain.c:2596-2599 — final blits.
            OwnBlitter()
            save_blit(shp.backsave)
            mask_blit()
            shape_blit()
            DisownBlitter()
            an.visible = True
            fp_drawing.obcount = fp_drawing.obcount + 1
            if pass_count == 1:
                pass_count = 0
                passmode = passmode + 1
                continue
            done = 1
        j = j + 1

    WaitBlit()                                            # fmain.c:2610
```

## Notes

- **Phase 14 is nested inside Phase 13.** The canonical `redraw_or_scroll` function calls `no_motion_tick` when both scroll deltas are zero (or after the viewstatus full-redraw path via the `viewchange` label at `fmain.c:1994`). Other logic reference docs that say "runs in Phase 14" are therefore implicitly "runs inside Phase 13's no-motion branch" — hero-scrolling frames skip the entire block.
- **Three early-exit paths.** The `while (!quitflag)` loop has three `continue` short-circuits that bypass the rendering phases (15–24). From outermost to innermost: viewstatus 1 or 4 inside `process_input_key` (`fmain.c:1373`), pause gate `game_paused()` (`fmain.c:1378`), and — indirectly — the freeze-timer gate inside `no_motion_tick` that skips AI/encounters/hunger but NOT rendering.
- **`viewstatus` transitions.** The playfield gate takes values 0 (normal), 1 (big map, blocks everything but input), 2 (placard hold — `Delay(200)` then bump to 99), 3 (fade-in queued for Phase 24), 4 (pickup placard, same as 1), 98 (mid-redraw: run one more redraw pass), 99 (corrupt sentinel: force two redraws). The 99 → 98 → 0 sequence at `fmain.c:1991-1993` is why a torch-lit or region-load redraw takes two frames to fully settle.
- **Frame rate source of truth.** `page_flip` at Phase 23 calls `WaitBOVP(&vp_text)` inside `pagechange`. This is the hardware gate that caps the loop at the Amiga vertical-blank rate. The `Delay(1)` calls inside the pause branch and the viewstatus-1/4 branch are not frame-rate equivalents — they release the CPU without producing a rendered frame.
- **`goto statc` / `goto raise`.** The actor loop uses two assembly-like jump labels (`fmain.c:1755` `statc`, `fmain.c:1745` `raise`). `actor_tick` flattens these into the explicit `compute_rel_coords` / `update_environ` tail calls.
- **Disk-I/O non-blocking.** Phase 13's `load_next()` at `fmain.c:1987` is non-blocking; it only calls `load_new_region()` if the previous channel has completed. Blocking waits happen only through `load_all()` (outside the main loop) at region transitions and on revive.
