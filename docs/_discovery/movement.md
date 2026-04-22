# Discovery: Movement & Direction System

**Status**: complete
**Investigated**: 2026-04-05
**Requested by**: orchestrator
**Prompt summary**: Trace the complete movement and direction-encoding system including direction vectors, set_course pathfinding, move_figure, world wrapping, velocity system, and movement state effects.

## Direction Encoding

The game uses a compass-rose direction system with 9 values (`ftale.h:92`, `fmain.c:1010`):

| Value | Direction | xdir | ydir |
|-------|-----------|------|------|
| 0     | NW        | -2   | -2   |
| 1     | N         |  0   | -3   |
| 2     | NE        |  2   | -2   |
| 3     | E         |  3   |  0   |
| 4     | SE        |  2   |  2   |
| 5     | S         |  0   |  3   |
| 6     | SW        | -2   |  2   |
| 7     | W         | -3   |  0   |
| 8     | Still     |  0   |  0   |
| 9     | Still     |  0   |  0   |

Note: values 8 and 9 both map to zero movement. `oldir = 9` signals "no direction input" (`fsubs.asm:1577`). The animation system uses `diroffs[16]` to map facing to animation frame base (`fmain.c:1010`):

```c
char diroffs[16] = {16,16,24,24,0,0,8,8,56,56,68,68,32,32,44,44};
```

Indices 0–7 are walk frames, 8–15 are fight/shoot frames. Each pair duplicates for two sub-frames.

## Direction Vectors (xdir/ydir)

Defined at `fsubs.asm:1276-1277`:

```asm
xdirdc.w-2,0,2,3,2,0,-2,-3,0,0
ydirdc.w-2,-3,-2,0,2,3,2,0,0,0
```

These are 10-entry word (16-bit) tables. The vectors are NOT unit vectors — cardinal directions have magnitude 3, diagonals have magnitude 2 per axis. This means diagonal movement displaces √(2²+2²) = 2.83 per speed unit, while cardinal movement displaces 3.0 per speed unit. Near-parity by design.

## _newx / _newy — Position Update Functions

### _newx (`fsubs.asm:1280-1295`)

Signature: `newx(x, dir, speed)` — returns new x coordinate.

```
if dir > 7: return x (no change)
result = x + (xdir[dir] * speed) lsr 1
result = result & 0x7FFF         (mask to 15-bit world coordinate)
```

Assembly trace:
1. `d0 = x`, `d2 = dir`, `d3 = speed` — loaded from stack (`fsubs.asm:1282-1286`)
2. `cmp.b #7,d2; bhi.s newxx` — if dir > 7, return x unchanged (`fsubs.asm:1287-1288`)
3. `add.b d2,d2` — double dir for word-size table index (`fsubs.asm:1290`)
4. `move.w (a0,d2),d2` — load xdir[dir] (`fsubs.asm:1291`)
5. `muls.w d2,d3` — signed multiply: direction × speed → 32-bit result (`fsubs.asm:1292`)
6. `lsr.w #1,d3` — **logical** (unsigned) right shift of low word by 1 (`fsubs.asm:1293`)
7. `add.w d3,d0` — add to x coordinate (`fsubs.asm:1294`)
8. `and.l #$07fff,d0` — mask to 15-bit range [0, 32767] (`fsubs.asm:1295`)

**Critical detail**: Step 6 uses `lsr.w` (logical shift), not `asr.w` (arithmetic shift). For negative products, this introduces a +1 pixel bias compared to true signed division. Example: xdir=-3, speed=4 → product=-12 → word 0xFFF4 → lsr.w #1 → 0x7FFA (32762). Added to x=1000 via 16-bit add and 15-bit mask: (1000+32762) & 0x7FFF = 995 (expected 994). This asymmetry is negligible in practice (±1 pixel).

### _newy (`fsubs.asm:1298-1318`)

Same formula as `_newx` but with ydir table and one additional step — **preserves bit 15 of the original y coordinate**:

```asm
move.l  d0,d1           ; save original y (fsubs.asm:1307)
; ... same computation as _newx but with ydir ...
and.l   #$07fff,d0      ; mask to 15 bits (fsubs.asm:1315)
and.w   #$8000,d1       ; extract bit 15 of original y (fsubs.asm:1316)
or.w    d1,d0           ; restore bit 15 (fsubs.asm:1317)
```

This means bit 15 of abs_y acts as a persistent flag that survives movement calculations. Its purpose may relate to indoor/underground state, but this is not confirmed from `_newy` alone. See Unresolved section.

## set_course Pathfinding (`fmain2.c:57-228`)

`set_course(object, target_x, target_y, mode)` — sets an actor's facing direction (and optionally state) based on a target position and a pathfinding mode.

### Setup (`fmain2.c:69-84`)

1. Loads `a1 = &anim_list[object]` (`fmain2.c:73-75`)
2. Mode 6 special case: uses target_x/target_y directly as xdif/ydif without subtraction from current position (`fmain2.c:79-80`)
3. All other modes: `xdif = object.abs_x - target_x`, `ydif = object.abs_y - target_y` (`fmain2.c:85-88`)

### Direction Computation (`fmain2.c:90-109`)

Converts xdif/ydif into sign indicators:
- `xdir = sign(xdif)` ∈ {-1, 0, 1}
- `ydir = sign(ydif)` ∈ {-1, 0, 1}
- `xabs = |xdif|`, `yabs = |ydif|`

### Directional Snapping — All Modes Except 4 (`fmain2.c:113-126`)

For mode ≠ 4: if one axis dominates, zero the minor axis direction:
- If `(xabs >> 1) > yabs`: `ydir = 0` — mostly horizontal, suppress vertical
- If `(yabs >> 1) > xabs`: `xdir = 0` — mostly vertical, suppress horizontal

This snapping prevents diagonal movement when the target is clearly along a cardinal axis. Mode 4 (RANDOM-like) skips this, allowing all 8 directions from any angle.

### Deviation Computation (`fmain2.c:130-147`)

- `deviation = 0` by default
- Mode 1: if `(xabs + yabs) < 40`, `deviation = 1` (`fmain2.c:136-139`)
- Mode 2: if `(xabs + yabs) < 30`, `deviation = 1` (`fmain2.c:143-146`)

The deviation adds randomness to close-range pursuit — the actor wobbles ±1 direction from ideal when near the target. Note: the comment for mode 2 says "deviation = 2" but the assembly sets `moveq #1,d4` — **deviation is 1 for both modes** (`fmain2.c:145`).

### Direction Reversal — Mode 3 (`fmain2.c:149-152`)

For mode 3: `xdir = -xdir; ydir = -ydir`. This causes the actor to face **away** from the target. Used for BACKUP/retreat behavior.

### com2 Direction Lookup (`fmain2.c:155-162`)

The com2 table converts (xdir, ydir) sign pair into a compass direction:

```c
com2[9] = {0, 1, 2, 7, 9, 3, 6, 5, 4};
```

Index formula: `j = com2[4 - 3*ydir - xdir]`

| ydir | xdir | Index | com2 | Direction |
|------|------|-------|------|-----------|
| -1   | -1   | 8     | 4    | SE        |
| -1   |  0   | 7     | 5    | S         |
| -1   |  1   | 6     | 6    | SW        |
|  0   | -1   | 5     | 3    | E         |
|  0   |  0   | 4     | 9    | Still     |
|  0   |  1   | 3     | 7    | W         |
|  1   | -1   | 2     | 2    | NE        |
|  1   |  0   | 1     | 1    | N         |
|  1   |  1   | 0     | 0    | NW        |

The mapping produces the direction TOWARD the target (since xdif = self - target, positive xdif means self is right of target, and com2 maps that to leftward directions). With mode 3 reversal, it becomes the direction AWAY from the target.

### com2 Result = 9 → STILL (`fmain2.c:165-168`)

If `j == 9` (both axes zeroed, actor is at target), sets `state = STILL` and returns.

### Random Deviation Application (`fmain2.c:172-179`)

If `j != 9`:
- `if (rand() & 2)`: `j += deviation` else `j -= deviation`
- `j = j & 7` — wraps to valid direction range

Note: the random test is `btst #1,d0` (bit 1 of rand()), not bit 0.

### State Assignment (`fmain2.c:183-187`)

- Sets `facing = j & 7`
- If mode ≠ 5: sets `state = WALKING` (value 12)
- If mode = 5: leaves state unchanged (allows SHOOT states to persist)

### Mode Summary

| Mode | Name in Source | Behavior in set_course |
|------|----------------|------------------------|
| 0    | (default)      | Direct toward target with directional snapping |
| 1    | PURSUE         | Same as 0 but adds ±1 deviation when distance < 40 |
| 2    | FOLLOW         | Same as 0 but adds ±1 deviation when distance < 30 |
| 3    | (reverse)      | Reverses direction to face AWAY from target |
| 4    | (unsnapped)    | Toward target WITHOUT directional snapping (always allows diagonal) |
| 5    | (no-walk)      | Same as 0 but does NOT set state to WALKING |
| 6    | (direct-vec)   | Uses target_x/target_y as raw xdif/ydif (not subtracted from position) |
| 7+   | (unused)       | Falls through all checks — identical to mode 0 |

**Important**: These mode numbers are NOT the same as the tactic constants (FRUST=0, PURSUE=1, etc.). The tactic system in `do_tactic()` maps its own constants to set_course mode parameters.

### How do_tactic Maps Tactics to set_course Modes (`fmain2.c:1664-1700`)

| Tactic (AI) | set_course call | Mode | Target |
|-------------|-----------------|------|--------|
| PURSUE (1)  | `set_course(i, hero_x, hero_y, 0)` | 0 | Hero |
| FOLLOW (2)  | `set_course(i, leader_x, leader_y+20, 0)` | 0 | Leader +20y |
| BUMBLE_SEEK (3) | `set_course(i, hero_x, hero_y, 4)` | 4 | Hero (no snap) |
| BACKUP (5)  | `set_course(i, hero_x, hero_y, 3)` | 3 | Hero (reversed) |
| EVADE (6)   | `set_course(i, neighbor_x, neighbor_y+20, 2)` | 2 | Neighboring actor |
| SHOOT (8)   | `set_course(i, hero_x, hero_y, 5)` | 5 | Hero (no walk) |
| EGG_SEEK (10) | `set_course(i, 23087, 5667, 0)` | 0 | Fixed coordinates |
| RANDOM (4)  | Does NOT call set_course — sets `facing = rand()&7` directly (`fmain2.c:1686`) |
| FRUST (0)   | Handled by caller — picks new random tactic (`fmain.c:2141-2143`) |

Tactics HIDE (7), SHOOTFRUST (9), DOOR_SEEK (11), DOOR_LET (12) are defined in `ftale.h:49-54` but have no case in `do_tactic()`. HIDE and DOOR_SEEK/DOOR_LET are never dispatched by any NPC AI code found in the source. SHOOTFRUST triggers the same random tactic selection as FRUST (`fmain.c:2141`).

### Stochastic Dispatch

Most tactics only actually call `set_course` when a random check passes:
- PURSUE: `r = !(rand()&7)` → 1/8 chance per tick (`fmain2.c:1670`)
- ATTACK2 goal: upgrades to `r = !(rand()&3)` → 1/4 chance per tick (`fmain2.c:1669`)
- BUMBLE_SEEK, BACKUP, FOLLOW, EVADE: same 1/8 (or 1/4 for ATTACK2) check

This means most NPCs only change direction every 4–8 ticks, creating sluggish/organic movement.

## move_figure (`fmain2.c:322-330`)

```c
move_figure(fig, dir, dist) short fig, dir, dist;
{   register unsigned short xtest, ytest;
    xtest = newx(anim_list[fig].abs_x, dir, dist);
    ytest = newy(anim_list[fig].abs_y, dir, dist);
    if (proxcheck(xtest, ytest, fig)) return FALSE;
    anim_list[fig].abs_x = xtest;
    anim_list[fig].abs_y = ytest;
    return TRUE;
}
```

Simple wrapper: computes candidate position via `newx`/`newy`, checks collision via `proxcheck`, commits position if clear. Returns TRUE on success, FALSE if blocked. Used primarily for combat knockback (`fmain2.c:250`: `move_figure(j, fc, 2)` pushes back a hit target).

**Note**: `move_figure` is NOT used for normal per-tick walking — that's handled inline in the main game loop (`fmain.c:1596-1650`), which performs `newx`/`newy` and `proxcheck` directly, then applies terrain effects, velocity recording, and animation.

## proxcheck — Collision Detection (`fmain2.c:277-296`)

```c
proxcheck(x, y, i)  // i = figure to exclude from check
```

Two-phase check:
1. **Terrain collision** via `prox(x, y)` (`fsubs.asm:1604-1622`): checks terrain at (x+4, y+2) and (x-4, y+2). Returns terrain code if blocked (1 = wall, ≥8 for hero = special passable, ≥10 = blocked). Wraiths (`race == 2`) skip terrain checks entirely (`fmain2.c:279`). Hero (`i==0`) can pass terrain codes 8 and 9 (`fmain2.c:280`).
2. **Actor collision**: loops through all actors; if another actor (not self, not slot 1, not dead, not type 5/raft) is within 11×9 pixel box, returns 16 (`fmain2.c:284-290`).

Returns 0 if clear; terrain code if terrain-blocked; 16 if actor-blocked.

## World Wrapping

### _wrap — Sign Extension (`fsubs.asm:1350-1356`)

```asm
_wrap:  move.l 4(sp),d0
        btst   #14,d0
        beq.s  wrap1
        or.w   #$8000,d0    ; if bit 14 set → set bit 15 (make negative)
        rts
wrap1:  and.w  #$7fff,d0    ; if bit 14 clear → clear bit 15 (positive)
        rts
```

Sign-extends a 15-bit world-coordinate difference to a 16-bit signed value. Used to convert `abs_x - map_x` differences for screen-relative positioning. If the raw difference has bit 14 set (≥ 16384), it's treated as negative (wrapping around the 32K world).

Called at `fmain.c:1846-1854` to compute `rel_x`/`rel_y` for rendering:
```c
an->rel_x = wrap(an->abs_x - map_x - 8);
an->rel_y = wrap(an->abs_y - map_y - 26);
```

### _newx/_newy Masking

Both `_newx` and `_newy` mask the result with `and.l #$07fff` — clamping to 15-bit [0, 32767] range. This provides implicit wrapping when coordinates overflow.

### Hero World Wrap (`fmain.c:1831-1839`)

For outdoor regions (region_num < 8), the hero wraps around world edges:
```c
if (an->abs_x < 300) an->abs_x = 32565;
else if (an->abs_x > 32565) an->abs_x = 300;
else if (an->abs_y < 300) an->abs_y = 32565;
else if (an->abs_y > 32565) an->abs_y = 300;
```

The wrap boundaries are 300 and 32565, creating a toroidal world for the overworld map. Note: this only applies to the hero (i==0), not NPCs. Indoor regions (region_num ≥ 8) do not wrap.

## _map_adjust — Camera Tracking (`fsubs.asm:1359-1437`)

`map_adjust(x, y)` — adjusts the global camera position (`_map_x`, `_map_y`) to follow the given world coordinates (typically hero position).

### Algorithm (`fsubs.asm:1362-1437`)

1. Compute screen-centered position: `d0 = x - 144`, `d1 = y - 70` (`fsubs.asm:1365-1366`)
2. Compute delta from current map position: `d2 = d0 - map_x`, `d3 = d1 - map_y` (`fsubs.asm:1369-1370`)
3. Sign-extend deltas from 15-bit: `lsl.w #1` then `asr.w #1` on d2 and d3 (`fsubs.asm:1372-1375`)
4. X-axis adjustment (`fsubs.asm:1377-1397`):
   - If delta < -70: large jump — `map_x = d0 + 70` (snap forward)
   - If delta > 70: large jump — `map_x = d0 - 70` (snap forward)
   - If delta < -20: `map_x -= 1` (scroll left)
   - If delta > 20: `map_x += 1` (scroll right)
5. Y-axis adjustment (`fsubs.asm:1398-1419`):
   - If delta < -24: large jump — `map_y = d1 + 24`
   - If delta > 44: large jump — `map_y = d1 - 44`
   - If delta < -10: `map_y -= 1`
   - If delta > 10: `map_y += 1`

The camera has a dead zone: ±20 pixels X, ±10 pixels Y. Within the dead zone, the camera doesn't scroll. Outside the dead zone, it scrolls 1 pixel per tick. Large jumps (>70 X or >44/24 Y) cause immediate camera repositioning.

The asymmetric Y thresholds (-24 vs +44) account for the character sprite being offset from screen center — there's more visible space below the character than above.

## Velocity System

### struct shape vel_x/vel_y Fields (`ftale.h:72`)

```c
char vel_x, vel_y;  /* velocity for slippery areas */
```

Signed byte values (-128 to 127). The comment says "for slippery areas" — and that's their primary function.

### Ice/Slippery Physics (environ == -2) (`fmain.c:1581-1597`)

When `environ == -2` (terrain type 7, ice):

```c
nvx = vel_x + newx(20, d, 2) - 20;   // Add directional impulse
nvy = vel_y + newy(20, d, 2) - 20;
// newx(20, d, 2) = 20 + (xdir[d]*2)/2 = 20 + xdir[d]
// So: nvx = vel_x + xdir[d]
if (|nvx| < e-8) vel_x = nvx;  // Clamp magnitude to e-8
if (|nvy| < e) vel_y = nvy;    // Clamp magnitude to e
xtest = abs_x + vel_x / 4;     // Position from velocity (integer division)
ytest = abs_y + vel_y / 4;
```

Velocity limits: `e = 42` normally, `e = 40` when riding swan. Max velocity = ±34 for x, ±42 for y. Position change = velocity/4, so max 8–10 pixels/tick.

For swan riding on ice, facing is set from velocity: `set_course(0, -nvx, -nvy, 6)` — mode 6 takes the negated velocity vector directly as the direction to face (`fmain.c:1592`).

### Normal Walking — Velocity Recording (`fmain.c:1646-1647`)

After non-ice movement commits a new position:
```c
vel_x = ((short)(xtest - abs_x)) * 4;
vel_y = ((short)(ytest - abs_y)) * 4;
```

This records the actual displacement ×4. It's used by the dismount check (`fmain.c:1420-1425`): the hero can only dismount the swan when velocity is low enough (|vel_x| < 15 && |vel_y| < 15), preventing dismount at high speed.

### FALL State Friction (`fmain.c:1737-1738`)

During the FALL animation state, velocity decays each tick:
```c
vel_x = (vel_x * 3) / 4;
vel_y = (vel_y * 3) / 4;
```

This gives 25% friction per tick — velocity halves roughly every 3 ticks.

### Velocity-Based Position Update in Non-Walking States (`fmain.c:1743-1744`)

For states reaching the `cpx` label with environ == -2 (ice), position is still updated by velocity:
```c
if (k == -2) {
    abs_x += vel_x / 4;
    abs_y += vel_y / 4;
}
```

This allows sliding on ice while in non-walking states (STILL, FIGHTING, etc.).

## Movement States and Speed

The movement speed `e` for WALKING state is determined by environ/terrain and riding status (`fmain.c:1599-1604`):

| Condition | Speed (e) | Context |
|-----------|-----------|---------|
| `i==0 && riding==5` | 3 | Hero riding raft (horse) |
| `environ == -3` (terrain 8) | -2 | Walk backwards (reversed facing) |
| `environ == -2` (terrain 7) | N/A | Ice physics (velocity-based, no direct speed) |
| `environ == -1` (terrain 6) | 4 | Fast terrain |
| `environ == 2` or `> 6` | 1 | Wading / deep water |
| all other | 2 | Default walking speed |

For non-hero actors: `if (k == 2 || k > 6) e = 1; else e = 2` with exceptions above.

Speed is passed directly to `newx`/`newy`. Negative speed (-2 for terrain 8) causes backward movement along the current facing direction. This creates the "slippery backwards" terrain behavior.

### Player Movement Input Flow (`fmain.c:1408-1460`)

1. Joystick/keyboard handler sets `oldir` (0–9) via `fsubs.asm:1573-1595`
2. If button pressed: determines fight/shoot vs walk attack (`fmain.c:1412-1444`)
3. If no button: if `oldir < 9`: set `facing = oldir`, and if shift/key active: `state = WALKING` (`fmain.c:1446-1454`)
4. If `oldir == 9` (no input): `state = STILL` (`fmain.c:1455`)
5. Hunger > 120: random ±1 direction deviation (`fmain.c:1448-1451`)

### Deviation on Collision (Player) (`fmain.c:1612-1626`)

When the player hits a wall during WALKING:
1. Try `d+1` (clockwise deviation) — if clear, go there (`fmain.c:1613-1617`)
2. If still blocked, try `d-2` (counterclockwise from original) (`fmain.c:1620-1624`)
3. If all three blocked: go to `blocked` label → `frustflag++` (`fmain.c:1654-1660`)

The frustflag counter triggers animations: >20 = scratching head, >40 = special animation index 40.

### Movement State Definitions (`ftale.h:9-26`)

| Value | Name     | Movement Effect |
|-------|----------|-----------------|
| 12    | WALKING  | Normal movement per tick via newx/newy |
| 13    | STILL    | No position change |
| 0–8   | FIGHTING | No position change (attack animation) |
| 14    | DYING    | No position change (death animation) |
| 15    | DEAD     | No position change |
| 16    | SINK     | No active movement |
| 17    | OSCIL    | No movement (oscillating animation) |
| 19    | TALKING  | No movement (NPC dialogue) |
| 20    | FROZEN   | No movement |
| 21    | FLYING   | Defined but **not used as a movement state in any code path** |
| 22    | FALL     | Velocity-based position update with 25% friction per tick |
| 23    | SLEEP    | No movement |
| 24-25 | SHOOT    | No position change (ranged attack) |

FLYING (21) is defined in `ftale.h:21` and `fmain.c:99` but never assigned to any actor's state in any code path found. Swan/raft riding uses WALKING state with `riding` variable and ice environ flag instead.

## Cross-Cutting Findings

- **fmain.c:1646-1647** — `vel_x`/`vel_y` set to displacement×4 during normal WALKING, even though the comment says "for slippery areas." This recorded velocity is cross-referenced by the swan dismount check (`fmain.c:1420-1425`) and the ice physics system.
- **fmain.c:1592** — `set_course(0, -nvx, -nvy, 6)` — the player character (object 0) gets its facing set by `set_course` ONLY when riding the swan on ice. In all other cases, the player's facing is set directly from joystick input.
- **fmain.c:1448-1451** — Hunger > 120 introduces random ±1 directional wobble to player movement, simulating disorientation from starvation.
- **fsubs.asm:1316-1317** — `_newy` preserves bit 15 of the original y coordinate through movement. This bit is carried through all movement and wrapping, acting as a flag layer on top of the 15-bit y coordinate system.
- **fmain2.c:145** — Comment says `deviation = 2` for mode 2 (FOLLOW) but assembly sets `moveq #1,d4` — deviation is actually 1 for both modes 1 and 2. The C pseudocode above the assembly is inaccurate for this value.
- **fmain2.c:1688-1689** — EVADE tactic follows a neighboring actor (i+1 or i-1), not the hero. It uses mode 2 (with deviation) and targets `abs_y + 20`, offset downward from the neighbor. This creates erratic movement near pack-mates, appearing as evasion.
- **fmain.c:1831-1839** — World wrapping only applies to the hero in outdoor regions. NPCs are not wrapped — they can theoretically exist at any coordinate without wrapping.

## Unresolved

- **Bit 15 of abs_y**: `_newy` preserves bit 15 (`fsubs.asm:1316-1317`), suggesting it is used as a flag. Its meaning is not determined from the movement code alone. Cross-referencing save/load or region-transfer code may reveal its purpose.
- **FLYING state (21)**: Defined but no code path assigns it. May be vestigial or used in an unreachable code path.
- **HIDE (7), DOOR_SEEK (11), DOOR_LET (12) tactics**: Defined in `ftale.h:49-54` but never dispatched by any NPC AI logic found in the source. May be vestigial/planned features.
- **_newx lsr.w asymmetry**: The logical shift on negative products creates a +1 pixel bias for negative-direction movement. This appears to be an oversight rather than intentional, but cannot be confirmed without author intent.
- **Terrain types 6, 7, 8 semantic names**: The code maps terrain 6→environ -1 (fast), 7→environ -2 (ice), 8→environ -3 (backwards). What in-game terrains these correspond to is not clear from code alone (would need map data analysis).

## Refinement Log
- 2026-04-05: Initial comprehensive discovery pass. All code paths traced with citations.
