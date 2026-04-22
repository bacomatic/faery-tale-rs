# Discovery: NPC Movement Speed Per Environ/Terrain Type

**Status**: complete
**Investigated**: 2026-04-19
**Requested by**: orchestrator
**Prompt summary**: Trace the complete code path for how NPC (non-hero) actors determine their movement speed based on terrain/environ type. Produce a comprehensive table of NPC speed values per environ condition.

## Key Finding: Shared Speed Logic

The speed assignment at `fmain.c:1599-1602` applies to **all actors** (hero and NPC alike) except for one hero-only branch. There is no NPC-specific speed logic — NPCs share the same speed calculation as the hero, minus the riding bonus.

## Movement Loop Structure

The main actor movement loop at `fmain.c:1466-1870` iterates over all actors `i=0..anix-1`. For each actor:

1. `k = an->environ` is loaded from the actor's struct — `fmain.c:1474`
2. If `freeze_timer && i > 0`, actor skips all processing via `goto statc` — `fmain.c:1468`
3. Type-specific handling (OBJECTS → skip; DRAGON, CARRIER, SETFIG → special paths)
4. **WALKING state** processing at `fmain.c:1580-1650` determines speed and performs movement
5. **Sinker section** at `fmain.c:1762-1806` updates `an->environ` based on terrain type `j`

## Speed Assignment: WALKING State

### Ice Physics Block (environ == -2): fmain.c:1581-1598

```c
if (k == -2)
{   if (riding == 11) e = 40; else e = 42;
    nvx1 = nvx = an->vel_x + newx(20,d,2)-20;
    nvy1 = nvy = an->vel_y + newy(20,d,2)-20;
    if (nvx1 < 0) nvx1 = -nvx;
    if (nvy1 < 0) nvy1 = -nvy;
    if (nvx1 < e-8) an->vel_x = nvx;
    if (nvy1 < e) an->vel_y = nvy;
    xtest = an->abs_x + an->vel_x/4;
    ytest = an->abs_y + an->vel_y/4;
    if (riding == 11) { set_course(0,-nvx,-nvy,6); d = an->facing; goto newloc; }
    if ((proxcheck(xtest,ytest,i))==0) goto newloc;
    k = 0;
}
```

- **No `i==0` check** — applies to ALL actors on ice terrain
- `riding` is a global variable (`fmain.c:563`) tracking hero mount state, so for NPCs `riding == 11` is never intended but technically possible if the hero is riding a swan simultaneously. In practice, the `else` branch fires: terminal velocity cap `e = 42`
- Terminal velocity X: `|vel_x| < 34` (e-8 = 42-8)
- Terminal velocity Y: `|vel_y| < 42`
- Position delta: `vel_x/4` and `vel_y/4` per frame
- Velocity accumulates: `vel += xdir[facing]` per frame (derived from `newx(20,d,2)-20`)
- If `proxcheck` blocks the ice movement, `k` is reset to 0 and execution falls through to the normal speed assignment below

### Normal Speed Assignment: fmain.c:1599-1602

```c
if (i==0 && riding == 5) e = 3;
else if (k == -3) e = -2; /* walk backwards */
else if (k == -1) e = 4;
else if (k == 2 || k > 6) e = 1; else e = 2;
```

The `if/else` chain is evaluated in order. For NPCs (`i > 0`):

1. `i==0 && riding == 5` → **FALSE** (never matches NPC) — hero-only, turtle riding
2. `k == -3` → `e = -2` (backwards movement)
3. `k == -1` → `e = 4` (fast terrain, 2× normal)
4. `k == 2 || k > 6` → `e = 1` (wading, half speed)
5. default → `e = 2` (normal speed)

## Complete NPC Speed Table

| Environ (k) | Terrain Source | Speed (e) | Pixels/frame (diagonal) | Notes |
|---|---|---|---|---|
| -3 | Terrain 8 (lava/fire) | -2 | ~2-3 reversed | Walks opposite to facing. **But NPCs are blocked from terrain 8 by proxcheck** (see Collision section) |
| -2 | Terrain 7 (ice) | velocity-based | 0 to ~10 | Momentum physics; no fixed speed. Cap: vel_x < 34, vel_y < 42. Effective speed: vel/4 px/frame |
| -1 | Terrain 6 (slippery) | 4 | ~4-6 | 2× normal speed |
| 0 | Terrain 0 (normal) | 2 | ~2-3 | Standard walking speed |
| 2 | Terrain 2 (shallow water) | 1 | ~0-1 | Half speed (some directions yield 0 movement due to integer truncation) |
| 3 | Ramping from terrain 4 | 2 | ~2-3 | Normal speed (not matching k==2 or k>6) |
| 4 | Ramping from terrain 4 | 2 | ~2-3 | Normal speed |
| 5 | Terrain 3 (medium water) | 2 | ~2-3 | Normal speed — assign k=5 directly |
| 6 | Ramping from terrain 4 | 2 | ~2-3 | Normal speed |
| 7-10 | Ramping from terrain 4 | 1 | ~0-1 | Half speed (deep water) |
| 11-15 | Ramping from terrain 5 | 1 | ~0-1 | Half speed; at k>15 actor enters SINK state |
| 16-30 | Deep submersion | 1 | ~0-1 | Half speed; at k==30, drowning damage applies |

### Speed Formula

`newx(x, d, e)` and `newy(y, d, e)` from `fsubs.asm:1280-1313` compute:
- `new_x = x + (xdir[d] * e) / 2`
- `new_y = y + (ydir[d] * e) / 2`

Direction vectors (`fsubs.asm:1277-1278`):
```
xdir: -2, 0, 2, 3, 2, 0, -2, -3   (dir 0=NW..7=W)
ydir: -2, -3, -2, 0, 2, 3, 2, 0
```

Per-speed pixel deltas for cardinal directions (e.g., dir=1 North: xdir=0, ydir=-3):

| Speed (e) | Cardinal px/frame | Diagonal px/frame | Description |
|---|---|---|---|
| -2 | 3 (reversed) | 2 (reversed) | Backwards walk |
| 1 | 1 | 1 | Wading — some diagonal directions get 0 due to `(-2*1)/2 = -1` |
| 2 | 3 | 2 | Normal |
| 3 | 4 | 3 | Turtle riding (hero only) |
| 4 | 6 | 4 | Fast terrain |

## Hero vs NPC Speed Differences

| Condition | Hero Speed | NPC Speed | Citation |
|---|---|---|---|
| Normal terrain (k=0) | e=2 | e=2 | fmain.c:1602 |
| Fast terrain (k=-1) | e=4 | e=4 | fmain.c:1601 |
| Backwards terrain (k=-3) | e=-2 | e=-2 | fmain.c:1600 |
| Ice terrain (k=-2) | velocity (cap 42) | velocity (cap 42) | fmain.c:1581-1598 |
| Wading (k=2 or k>6) | e=1 | e=1 | fmain.c:1602 |
| Turtle riding (riding==5) | e=3 | N/A | fmain.c:1599 — hero only |
| Swan riding (riding==11) | ice cap 40 | N/A | fmain.c:1582 — hero only |
| Freeze timer active | Not affected | **Skipped entirely** | fmain.c:1468 |

**Key finding**: Except for the two riding states, hero and NPC speeds are identical for all environ values.

## Environ Assignment (Sinker Section): fmain.c:1762-1806

The sinker section converts terrain type `j` (from `px_to_im`) to environ value `k`, then writes `an->environ = k`. It applies to **ALL actors**.

```
sinker:
if (i==0 && raftprox) k = 0;                           // hero on raft: immune
else if (j == 0) k = 0;                                 // normal ground
else if (j == 6) k = -1;                                // fast/slippery
else if (j == 7) k = -2;                                // ice
else if (j == 8) k = -3;                                // backwards/lava
else if (j == 9 && i==0 && xtype == 52) { FALL; k=-2; } // pit (hero only)
else if (j == 2) k = 2;                                 // shallow water
else if (j == 3) k = 5;                                 // medium water
else if (j == 4 || j == 5) { ramp toward 10 or 30; }    // deep water
```

### Hero-only environ rules:
- `i==0 && raftprox` → k=0: hero can't drown while near raft/turtle — `fmain.c:1762`
- `j == 9 && i==0 && xtype == 52` → FALL + k=-2: pit traps only affect hero — `fmain.c:1776-1783`
- `riding == 11` → environ forced to -2 before loop: `fmain.c:1464`

### NPC environ rules:
- j=9 (pit/fall terrain) produces **no environ change** for NPCs — no branch matches, k retains its previous value — `fmain.c:1776`
- All other terrain→environ mappings are identical to hero

## NPC Collision: proxcheck

`proxcheck()` at `fmain2.c:277-293` is called for ALL actors with `proxcheck(xtest, ytest, i)`.

### Terrain collision via `prox()` (fsubs.asm:1590-1614)

`prox()` checks two points around the actor:
- Point 1 (x+4, y+2): blocked if terrain == 1 OR terrain >= 10 — `fsubs.asm:1596-1599`
- Point 2 (x-4, y+2): blocked if terrain == 1 OR terrain >= 8 — `fsubs.asm:1606-1609`

### Hero-only terrain passthrough in proxcheck

```c
if (i==0 && (x1 == 8 || x1 == 9)) x1 = 0;    // fmain2.c:282
```

For the hero only, terrain types 8 and 9 returned by `prox()` are zeroed → passable.
For NPCs, terrain 8 and 9 remain blocking. This means:
- **NPCs cannot walk onto terrain 8** (backwards/lava) — blocked by prox's second point check
- **NPCs cannot walk onto terrain 9** (pits) — blocked by prox's second point check
- Therefore environ=-3 (backwards) is effectively unreachable for NPCs in normal gameplay

### Wraith terrain bypass

```c
if (anim_list[i].type != ENEMY || anim_list[i].race != 2) /* wraith */
{   x1=prox(x,y);
    ...
}
```

Wraiths (type ENEMY, race 2) skip the entire terrain collision check — `fmain2.c:280`. They pass through walls and all terrain. Additionally, their terrain is forced to 0 at `fmain.c:1641`:
```c
if (an->race == 2 || an->race == 4) j = 0;
```
So wraiths always have environ=0 (normal speed e=2).

### Crystal shard passthrough (hero only)

```c
if (i==0)
{   if (j==15) { doorfind(xtest,ytest,0); }
    else bumped = 0;
    if (stuff[30] && j==12) goto newloc;    // fmain.c:1609
}
```

Only the hero can pass through crystal walls (terrain 12) with the crystal shard. NPCs are blocked by terrain 12 (which is ≥ 10), always.

### Deviation Logic: Shared

The obstacle deviation code at `fmain.c:1614-1626` applies to ALL actors:
1. Try direction+1: `d = (d+1)&7` — `fmain.c:1615-1618`
2. Try direction-2: `d = (d-2)&7` — `fmain.c:1621-1625`
3. If still blocked: hero → frustflag++; NPC → `an->tactic = FRUST` — `fmain.c:1654-1661`

NPC FRUST handling (`fmain.c:2141-2144`):
```c
if (tactic == FRUST || tactic == SHOOTFRUST)
{   if (an->weapon & 4) do_tactic(i,rand4()+2);  // ranged: FOLLOW..BACKUP
    else do_tactic(i,rand2()+3);                   // melee: BUMBLE_SEEK or RANDOM
}
```

## Special Terrain Effects on NPCs

### Ice (environ -2, terrain 7): YES
- NPCs DO experience ice velocity physics — `fmain.c:1581-1598` has no `i==0` check
- Terminal velocity cap is e=42 (since `riding != 11` for NPCs) — `fmain.c:1582`
- If blocked on ice, `k` resets to 0, NPC gets normal speed — `fmain.c:1597`

### Fast/Slippery (environ -1, terrain 6): YES
- NPCs move at e=4 (double speed) — `fmain.c:1601`

### Backwards (environ -3, terrain 8): THEORETICALLY YES, PRACTICALLY NO
- The speed code at `fmain.c:1600` would give e=-2 for any actor with k==-3
- However, `proxcheck` blocks NPCs from reaching terrain 8 — `fmain2.c:282` only zeros terrain 8/9 for hero
- An NPC could only have environ=-3 if spawned on terrain 8 or placed there by game logic

### Drowning (environ ≥ 16): YES
- NPCs enter SINK state at environ > 15 — `fmain.c:1795`
- At environ 30, NPCs take 1 vitality damage per 8 frames — `fmain.c:1849-1851`
- Exception: race 2 (Wraith) and race 3 (Skeleton) are immune to drowning damage — `fmain.c:1851` (`k` = `an->race` after repurposing at `fmain.c:1802`)
- In sector 181 (Astral Plane): NPCs at environ 30 are killed instantly (`an->vitality = 0`) — `fmain.c:1789`
- Race 2 (wraith) and race 4 (snake) have terrain forced to 0, preventing water environ entirely — `fmain.c:1641`

### Fiery Death Zone: YES
- `fmain.c:1843-1847`: if `fiery_death` flag set:
  - Hero with stuff[23]: environ forced to 0 (immune)
  - Any actor with environ > 15: instant death
  - Any actor with environ > 2: vitality--
- Applies to NPCs — no `i==0` guard on the damage branches

### Pit/Fall (terrain 9): NO for NPCs
- Fall logic at `fmain.c:1776-1783` requires `i==0 && xtype == 52` — hero only
- proxcheck blocks NPCs from terrain 9 anyway — `fmain2.c:282`

## References Found

### Speed Assignment
- fmain.c:1581-1598 — **read** — ice velocity physics block, no i==0 guard
- fmain.c:1582 — **read** — `if (riding == 11) e = 40; else e = 42;` terminal velocity
- fmain.c:1596 — **read** — `if ((proxcheck(xtest,ytest,i))==0) goto newloc;` ice collision
- fmain.c:1597 — **write** — `k = 0;` reset environ when blocked on ice
- fmain.c:1599 — **read** — `if (i==0 && riding == 5) e = 3;` hero-only turtle speed
- fmain.c:1600 — **read** — `else if (k == -3) e = -2;` backwards for all actors
- fmain.c:1601 — **read** — `else if (k == -1) e = 4;` fast terrain for all actors
- fmain.c:1602 — **read** — `else if (k == 2 || k > 6) e = 1; else e = 2;` wading/normal for all

### Environ Assignment
- fmain.c:1464 — **write** — `if (riding==11) anim_list[0].environ = -2;` hero-only swan override
- fmain.c:1468 — **read** — `if (freeze_timer && i > 0) goto statc;` NPC freeze skip
- fmain.c:1474 — **read** — `k = an->environ;` local copy of environ
- fmain.c:1762-1806 — **write** — sinker section, environ assignment for all actors
- fmain.c:1806 — **write** — `if (MAP_STABLE) an->environ = k;` final environ store

### Collision
- fmain2.c:277-293 — **call** — proxcheck definition
- fmain2.c:280 — **read** — wraith terrain bypass
- fmain2.c:282 — **read** — `if (i==0 && (x1 == 8 || x1 == 9)) x1 = 0;` hero-only passthrough
- fsubs.asm:1590-1614 — **call** — prox() terrain probe with asymmetric blocking thresholds
- fmain.c:1605 — **call** — `j=proxcheck(xtest,ytest,i);` main collision check
- fmain.c:1609 — **read** — `if (stuff[30] && j==12) goto newloc;` hero-only crystal pass
- fmain.c:1614-1626 — **read** — deviation logic (shared hero/NPC)
- fmain.c:1661 — **write** — `else an->tactic = FRUST;` NPC blocked response

### Race Immunities
- fmain.c:1641 — **write** — `if (an->race == 2 || an->race == 4) j = 0;` wraith/snake water immunity
- fmain.c:1802 — **write** — `k = an->race;` repurpose k to race for later checks
- fmain.c:1849-1851 — **read** — drowning damage, immune if race 2 (wraith) or race 3 (skeleton)

### Movement Functions
- fsubs.asm:1277-1278 — **read** — direction vectors: xdir and ydir tables
- fsubs.asm:1280-1296 — **call** — `_newx`: `x + (xdir[d] * e) / 2`
- fsubs.asm:1298-1313 — **call** — `_newy`: `y + (ydir[d] * e) / 2`

### AI Tactic Handling
- fmain2.c:1666-1699 — **call** — `do_tactic()`: sets facing/state via set_course, never touches speed
- fmain2.c:66-210 — **call** — `set_course()`: sets facing and state=WALKING, does NOT set speed
- fmain.c:2141-2144 — **read** — FRUST tactic response: randomly selects new movement strategy

### Actor Struct
- ftale.h:60 — **read** — `environ` field: `char` type in struct shape
- ftale.h:61 — **read** — `vel_x, vel_y` fields: `char` type for ice physics velocity
- fmain.c:563 — **read** — `short riding` global variable (hero mount state, not per-actor)

## Code Path

1. **Entry**: `fmain.c:1466` — main for loop `for (i=0; i<anix; i++)`
2. **Environ load**: `fmain.c:1474` — `k = an->environ`
3. **Freeze check**: `fmain.c:1468` — NPCs skip if freeze_timer active
4. **Type dispatch**: `fmain.c:1482-1560` — OBJECTS/DRAGON/CARRIER/SETFIG skip to special handling
5. **WALKING ice**: `fmain.c:1581-1598` — if k==-2, velocity physics for ALL actors
6. **WALKING speed**: `fmain.c:1599-1602` — e assignment shared (hero riding branch, then NPC-applicable branches)
7. **Movement calc**: `fmain.c:1603-1604` — `xtest = newx(abs_x, d, e); ytest = newy(abs_y, d, e)`
8. **Collision**: `fmain.c:1605` — `j = proxcheck(xtest, ytest, i)` — hero: 8/9 passable, NPC: 8/9 blocking
9. **Deviation**: `fmain.c:1614-1626` — try d+1, then d-2, then blocked
10. **Blocked**: `fmain.c:1661` — NPC: `an->tactic = FRUST`
11. **Position update**: `fmain.c:1647-1650` — store new position, clear frustflag
12. **Terrain lookup**: `fmain.c:1641-1645` — `j = px_to_im(xtest,ytest)`, wraith/snake immunity
13. **Sinker**: `fmain.c:1762-1806` — terrain j → environ k for all actors
14. **Drowning**: `fmain.c:1849-1851` — environ 30 damage, race 2/3 immune

## Cross-Cutting Findings

- **fmain.c:1641** — Race 2 (wraith) and race 4 (snake) have terrain forced to 0 in the WALKING movement section, not just in collision. This means they ALSO skip all environ/speed effects from terrain, not just collision blocking. This is a separate immunity from the proxcheck bypass.
- **fmain.c:1802** — The local variable `k` is repurposed from environ to `an->race` AFTER the sinker section. The subsequent drowning check at `fmain.c:1851` uses this repurposed `k`, meaning `k != 2 && k != 3` checks RACE, not environ. This is easy to misread as an environ check.
- **fmain.c:1468** — `freeze_timer` freezes ALL NPCs (i>0), including friendly actors, carriers, and SETFIGs that reached this point. The comment `/* what about wizard? */` suggests Talin was aware this might be too broad.
- **prox asymmetry** — `fsubs.asm:1598-1599` vs `fsubs.asm:1608-1609`: the first point (x+4) blocks at terrain ≥ 10 while the second point (x-4) blocks at ≥ 8. This means terrain 8/9 only block from the "left foot" check. An NPC could theoretically be on terrain 8 from one side but not the other. In practice, most terrain 8 tiles are large enough to catch both points.
- **Wading speed gap** — The check `k == 2 || k > 6` creates a speed anomaly: environ values 3-6 give normal speed (e=2) while 2 and 7+ give slow speed (e=1). For terrain 4 (deep water) where environ ramps from 0→10, actors briefly get: normal(0-1) → slow(2) → normal(3-6) → slow(7-10). This affects both hero and NPCs identically.

## Unresolved

- None. All questions from the prompt have been answered with direct source code citations.

## Refinement Log
- 2026-04-19: Initial discovery — full trace of NPC speed logic, environ assignment, collision, race immunities, and special terrain effects.
