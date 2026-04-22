# Discovery: Turtle Autonomous Behavior and Terrain Types

**Status**: complete
**Investigated**: 2026-04-19
**Requested by**: orchestrator
**Prompt summary**: Trace turtle autonomous movement AI, terrain restrictions, extent tracking, carrier proximity/mounting, and any special behaviors when not ridden.

## Turtle Identity and Type System

### Actor Type and File ID
- `fmain.c:2788-2789` — `load_carrier(5)` sets `an->type = CARRIER` (not DRAGON). Extent index = 1.
- `fmain2.c:650` — Turtle sprite file definition: `{ 2,32,16, 20,CARRIER,1351 }` — width 2 words (32px), height 32px, 16 frames, 20 disk blocks, seq slot CARRIER, file ID 1351.
- `fmain.c:574` — `active_carrier = 5` when turtle is loaded.
- `fmain.c:563` — `riding = 5` when hero is on the turtle.

### Distinguishing Turtle from Bird from Raft
- `fmain.c:1497` — Swan branch: `if (actor_file == 11)` within CARRIER handler.
- `fmain.c:1511` — Turtle branch: the `else` of the swan check — any CARRIER with `actor_file != 11` is treated as turtle.
- Raft has its own type `RAFT` (not CARRIER) and is handled at `fmain.c:1556-1573`.
- Dragon has type `DRAGON` and is handled at `fmain.c:1479-1493`.

## Autonomous Movement AI

### Code Location: fmain.c:1520-1542

When the turtle is NOT being ridden (`raftprox == 0` OR `wcarry != 3`), the `else` branch at `fmain.c:1520` executes:

```c
xtest = newx(an->abs_x,d,3);           // fmain.c:1521 — try current facing, speed 3
ytest = newy(an->abs_y,d,3);           // fmain.c:1522
if (px_to_im(xtest,ytest) != 5)        // fmain.c:1523 — not deep water?
{   d = (d+1)&7;                        // fmain.c:1524 — try CW+1
    xtest = newx(an->abs_x,d,3);       // fmain.c:1525
    ytest = newy(an->abs_y,d,3);       // fmain.c:1526
    if (px_to_im(xtest,ytest) != 5)    // fmain.c:1527
    {   d = (d-2)&7;                    // fmain.c:1528 — try original-1 (CCW)
        xtest = newx(an->abs_x,d,3);   // fmain.c:1529
        ytest = newy(an->abs_y,d,3);   // fmain.c:1530
        if (px_to_im(xtest,ytest) != 5)// fmain.c:1531
        {   d = (d-1)&7;               // fmain.c:1532 — try original-2 (further CCW)
            xtest = newx(an->abs_x,d,3); // fmain.c:1533
            ytest = newy(an->abs_y,d,3); // fmain.c:1534
        }
    }
}
riding = FALSE;                         // fmain.c:1537
dex = d+d+(cycle&1);                    // fmain.c:1538 — animation frame
```

Then the shared turtle code at `fmain.c:1540-1542`:
```c
j = px_to_im(xtest,ytest);             // fmain.c:1540
if (j == 5) { an->abs_x = xtest; an->abs_y = ytest; }  // fmain.c:1541
e = 1;                                  // fmain.c:1542 — extent index for turtle
```

### Direction Probe Sequence

Given the turtle's current facing `d` (read from `an->facing` at `fmain.c:1475`), the autonomous movement tries 4 directions in order. If `d` is the original facing:

| Priority | Direction Tried | Code Reference |
|----------|----------------|----------------|
| 1st | d (original facing) | fmain.c:1521-1523 |
| 2nd | (d+1) & 7 — clockwise neighbor | fmain.c:1524-1527 |
| 3rd | (d-1) & 7 — counter-clockwise neighbor | fmain.c:1528-1531 |
| 4th | (d-2) & 7 — two steps counter-clockwise | fmain.c:1532-1534 |

If none of the 4 directions have deep water (terrain type 5), the 4th direction's xtest/ytest is used for the final terrain check. Since that check also fails (not water), the turtle does NOT move — `an->abs_x`/`an->abs_y` remain unchanged.

### Critical Finding: Facing Direction Never Updated

`load_carrier()` (`fmain.c:2784-2801`) does NOT set `an->facing`. The CARRIER handler exits via `goto raise` (`fmain.c:1545`), which bypasses the `newloc:` label (`fmain.c:1633`) where `an->facing = d` is normally written. Therefore:

- **The turtle's facing direction (`an->facing`) is NEVER written by the carrier handler.**
- The local variable `d` is modified during direction probing but is never persisted back to `an->facing`.
- Each frame, `d = an->facing` reads the SAME original value.
- The turtle always prefers the same direction and tries the same 4-direction sequence every tick.
- Initial facing depends on whatever was previously stored in `anim_list[3].facing`: 0 (NW) for a fresh game (global zero-init), or the residual facing from a previously loaded carrier (e.g., swan).

### Speed

- Autonomous speed: 3 (third argument to `newx`/`newy`) — `fmain.c:1521-1522`.
- `newx`/`newy` (`fsubs.asm:1280-1316`) compute displacement as `(direction_vector * speed) / 2`.

Direction vectors from `fsubs.asm:1275-1276`:
```
xdir: -2, 0, 2, 3, 2, 0, -2, -3
ydir: -2, -3, -2, 0, 2, 3, 2, 0
```

Per-tick pixel displacements at speed 3:

| Dir | Name | dx | dy |
|-----|------|----|----|
| 0 | NW | -3 | -3 |
| 1 | N | 0 | -5 |
| 2 | NE | 3 | -3 |
| 3 | E | 4 | 0 |
| 4 | SE | 3 | 3 |
| 5 | S | 0 | 4 |
| 6 | SW | -3 | 3 |
| 7 | W | -5 | 0 |

Note: N/S and E/W are asymmetric by 1 pixel due to `lsr.w` (logical shift right) rounding of negative values. `fsubs.asm:1292` uses unsigned shift on signed products.

### Movement Frequency

The turtle moves EVERY frame of the main game loop (no rate-limiting). The carrier handler runs in the same `for (i=0; i<anix; i++)` loop as all actors (`fmain.c:1467`). Only `freeze_timer` pauses movement (`fmain.c:1472`: `if (freeze_timer && i > 0) goto statc`).

### Animation

- Autonomous: `dex = d+d+(cycle&1)` — `fmain.c:1538`. Two frames per direction, alternating each global cycle.
- Riding: `dex = d+d; if (anim_list[0].state == WALKING) dex += (cycle&1)` — `fmain.c:1517-1518`.

## Terrain Restrictions

### Autonomous Turtle: ONLY Terrain Type 5

The turtle uses `px_to_im()` directly (NOT `proxcheck()`) for terrain checks. It tests `px_to_im(xtest,ytest) != 5` to determine if a direction is blocked (`fmain.c:1523, 1527, 1531`). The final position is accepted only `if (j == 5)` (`fmain.c:1541`).

**Terrain type 5 = "Water (very deep)"** — from the terrain type table in `fmain.c:685-686` and the environ logic at `fmain.c:1782-1793`.

The turtle CANNOT autonomously move onto:
- Type 0: open land
- Type 1: impassable walls
- Type 2: shallow water
- Type 3: medium water
- Type 4: deep water
- Types 6-15: ice, lava, pits, etc.

**Only terrain type 5 (deepest water) is valid for autonomous turtle movement.**

### Carrier Environ Override

At the CARRIER handler entry (`fmain.c:1495`): `k = j = 0` — the environ variable is explicitly zeroed. The handler exits via `goto raise`, skipping the `sinker:` code block (`fmain.c:1767-1800`) entirely. Therefore the turtle actor itself never accumulates water environ effects and has no drowning/sinking behavior.

### Hero Riding Turtle: Standard Terrain Collision

When riding (riding==5), the hero's movement goes through normal `proxcheck()` at `fmain.c:1605` with speed `e = 3` (`fmain.c:1599`). The hero can walk on ANY terrain that `proxcheck` allows (everything except types 1 and 10+). The raftprox environ override at `fmain.c:1768` (`if (i==0 && raftprox) k = 0`) prevents drowning in water.

**The hero riding the turtle is NOT restricted to water.** They can walk onto land at speed 3. The turtle's position (`an->abs_x/y`) only updates when the hero is on terrain type 5 (`fmain.c:1541`). When the hero walks onto land, the turtle stays at the water's edge.

### Implicit Dismount Mechanic

There is no explicit fire-button dismount for the turtle (unlike swan at `fmain.c:1417-1427`). Dismount occurs automatically:
1. Hero walks onto land → turtle position stays at last water tile.
2. Distance between hero and turtle increases.
3. When distance exceeds 16px in either axis (`fmain.c:1459-1462`), `raftprox` drops to 0.
4. Next frame, the carrier handler enters the `else` (autonomous) branch instead of the riding branch, setting `riding = FALSE` (`fmain.c:1537`).

### Hero Speed While Riding

`fmain.c:1599`: `if (i==0 && riding == 5) e = 3` — overrides ALL terrain-dependent speed. This check is FIRST in the speed selection chain (before environ-based speeds). Comparison:

| Terrain | Normal Speed | Riding Turtle |
|---------|-------------|---------------|
| Open ground | 2 | 3 |
| Deep water | 1 | 3 |
| Ice (slippery) | 4 | 3 |
| Lava | -2 (backwards) | 3 (forward!) |

## Extent System

### Turtle Extent: extent_list[1]

`fmain.c:340`: `{ 0, 0, 0, 0, 70, 0, 1, 5 }` — starts degenerate (zero coordinates). Must be repositioned via `move_extent(1, x, y)` before the turtle can be activated.

### Extent Repositioning

The extent tracks the turtle/hero position every frame:
- `fmain.c:1545`: `move_extent(e,xtest,ytest)` — e=1 for turtle. xtest/ytest = hero position (if riding) or last probed autonomous position.
- `fmain2.c:1560-1565`: `move_extent` creates a 500×400 rectangle centered on (x,y).

### Extent Drift When Stuck

When the turtle is autonomous and cannot find water in any of 4 probed directions, xtest/ytest reflect the LAST probed position (direction d-2), which is NOT water. `move_extent` still runs, moving the extent to this non-water position while the turtle's actual position stays fixed. This can cause the extent to gradually drift away from the turtle's actual position.

### Carrier Activation on Extent Entry

`fmain.c:2716-2719`:
```c
if (xtype < 70) active_carrier = 0;
else if (xtype == 70 &&
    (!active_carrier || (!riding && actor_file != extn->v3)) )
        load_carrier(extn->v3);
```

When the hero enters the turtle extent (etype 70, v3=5):
- If `active_carrier == 0`: `load_carrier(5)` fires — turtle spawns at extent center.
- If already active and riding same carrier: no reload.
- If not riding and wrong actor file loaded: reload with turtle.

When leaving any carrier extent: `active_carrier = 0` — but extent position and `anix` persist.

### Extent Persistence Across Regions

The `extent_list[]` is a global static array (`fmain.c:338-370`). Entries are never reset. The turtle extent remains wherever `move_extent(1,...)` last placed it, even across region transitions. The turtle is deactivated (`active_carrier = 0`) when leaving the extent, but the extent position survives.

## Turtle Summoning (get_turtle)

### Via Sea Shell (stuff[6])

`fmain.c:3457-3461` — USE menu with hit==6:
```c
if (hit == 6 && hitgo)
{   if (hero_x<21373 && hero_x>11194 && hero_y<16208 && hero_y>10205)
        break;                             // blocked in swamp region
    get_turtle();
}
```

### get_turtle() — fmain.c:3510-3517

```c
get_turtle()
{   for (i=0; i<25; i++)
    {   set_loc();
        if (px_to_im(encounter_x,encounter_y) == 5) break;
    }
    if (i==25) return;
    move_extent(1,encounter_x,encounter_y);
    load_carrier(5);
}
```

Tries up to 25 random nearby positions (`set_loc()` at `fmain2.c:1714-1719`: random direction, 150-213px). Requires terrain type 5 (deep water). Fails silently if no water found.

### Via Turtle Eggs Aftermath

`fmain2.c:274`: `if (turtle_eggs) get_turtle()` — called in `aftermath()` after combat near turtle eggs.
`fmain2.c:1284`: `if (list->ob_id == TURTLE) turtle_eggs = anix2` — set when TURTLE object (id 102) is visible in current region.

### Turtle Eggs Object

`fmain2.c:1022`: `{23087,5667,TURTLE,1}` — TURTLE object at (23087,5667) in region 1 (maze forest).

Turtle egg extent at `fmain.c:345`: `{ 22945, 5597, 23225, 5747, 61, 3, 2, 4 }` — etype 61 (special encounter), spawns snakes (encounter_type 4, v1=3 snakes, v2=2 extra random).

## Carrier Proximity and Mounting

### Proximity Calculation

`fmain.c:1455-1462`:
```c
raftprox = turtleprox = FALSE;
if (active_carrier) wcarry = 3; else wcarry = 1;
xstart = anim_list[0].abs_x - anim_list[wcarry].abs_x - 4;
ystart = anim_list[0].abs_y - anim_list[wcarry].abs_y - 4;
if (xstart < 16 && xstart > -16 && ystart < 16 && ystart > -16)
    raftprox = 1;
if (xstart < 9 && xstart > -9 && ystart < 9 && ystart > -9)
    raftprox = 2;
```

When active_carrier is set, proximity is measured against `anim_list[3]` (carrier slot). The -4 offset biases the measurement slightly.

### turtleprox Variable

`fmain.c:564` — declared: `short turtleprox, raftprox;`
`fmain.c:1455` — reset every frame: `raftprox = turtleprox = FALSE;`

**`turtleprox` is vestigial/unused.** It is reset to FALSE every frame but never set to any other value anywhere in the codebase.

### Turtle Mounting Condition

`fmain.c:1511-1516` — inside CARRIER handler, non-swan branch:
```c
if (raftprox && wcarry == 3)
{   d = anim_list[0].facing;
    xtest = anim_list[0].abs_x;
    ytest = anim_list[0].abs_y;
    riding = 5;
    dex = d+d;
    if (anim_list[0].state == WALKING) dex += (cycle&1);
}
```

Conditions: `raftprox != 0` (hero within 16px of turtle) AND `wcarry == 3` (carrier is active). **No item requirement** — unlike the swan (requires Golden Lasso / stuff[5]), the turtle can be mounted with no prerequisites.

### Talking to Turtle

`fmain.c:3418-3421`:
```c
else if (an->type == CARRIER && active_carrier == 5)
{   if (stuff[6]) speak(57);
    else { stuff[6] = 1; speak(56); }
}
```

First conversation gives Sea Shell (stuff[6]) with speak(56). Subsequent conversations: speak(57).

## World Wrap While Riding

`fmain.c:1830-1837`:
```c
if (region_num < 8)
{   if (an->abs_x < 300) an->abs_x = 32565;
    else if (an->abs_x > 32565) an->abs_x = 300;
    else if (an->abs_y < 300) an->abs_y = 32565;
    else if (an->abs_y > 32565) an->abs_y = 300;
    else goto jkl;
    if (riding > 1)
    {   anim_list[3].abs_x = an->abs_x;
        anim_list[3].abs_y = an->abs_y;
    }
}
```

When riding > 1 (turtle=5 or swan=11) and the hero world-wraps, the carrier's position is snapped to the hero's new position. This applies to i==0 (hero) only.

## Rendering

### Swan Ground Rendering Override (NOT turtle)

`fmain.c:2463-2464`:
```c
if (atype == CARRIER && riding == 0 && actor_file == 11)
{   atype = RAFT; inum = 1; }
```

This applies ONLY to the swan (actor_file==11). The turtle has no such override — it renders normally as CARRIER type with its standard frame index.

### Screen Position Offset

`fmain.c:1855-1858`:
```c
if (an->type == CARRIER && riding == 11)
{   an->rel_x = wrap(an->abs_x - map_x - 32);
    an->rel_y = wrap(an->abs_y - map_y - 40);
}
else if (an->type==RAFT || an->type==CARRIER || an->type==DRAGON)
{   an->rel_x = wrap(an->abs_x - map_x - 16);
    an->rel_y = wrap(an->abs_y - map_y - 16);
}
```

Turtle (CARRIER, not riding==11): offset (-16, -16) from map position. Swan while riding: offset (-32, -40).

## Cross-Cutting Findings

- `fmain.c:2150` — Snake AI interacts with turtle eggs: `if (an->race==4 && turtle_eggs) tactic = EGG_SEEK` — snakes near turtle eggs use the EGG_SEEK tactic, targeting coordinates (23087, 5667) per `fmain2.c:1698`.
- `fmain.c:1768` — `raftprox` protects hero from drowning on ANY carrier (comment says "raft" but applies to turtle too): `if (i==0 && raftprox) k = 0`.
- `fmain.c:1599` — Riding turtle speed (3) overrides lava push-back (would be -2), effectively neutralizing lava's backward push while mounted.
- `fmain.c:1900` — `if (riding) goto nodoor3` — hero cannot enter doors while riding ANY carrier, including turtle.
- `fmain.c:3308` — `if (riding > 1) return` — freeze spell has no effect while riding turtle or swan.
- `fmain.c:3338` — `if (riding)` check in blue stone teleport handler — blocks teleport while riding.

## Unresolved

None — all questions from the prompt are answered with code citations.

## Refinement Log
- 2026-04-19: Initial comprehensive discovery pass covering all 6 requested areas.
