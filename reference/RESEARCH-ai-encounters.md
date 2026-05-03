# Game Mechanics Research — AI & Encounters

AI behavior system and encounter/spawning mechanics.

> **Citation format**: `file.c:LINE` or `file.c:START-END`. Speech references: `speak(N)`.
> Split from [RESEARCH.md](RESEARCH.md). See the hub document for the full section index.

---

## 8. AI System

Goal modes and tactical modes are enumerated in [§2.2](RESEARCH-data-structures.md#22-goal-modes) and [§2.3](RESEARCH-data-structures.md#23-tactical-modes). This section covers runtime behavior and decision-making.

### 8.1 Goal Mode Assignment

**At spawn** (`fmain.c:2761-2763`):

```c
if (an->weapon & 4) an->goal = ARCHER1 + encounter_chart[race].cleverness;
else an->goal = ATTACK1 + encounter_chart[race].cleverness;
```

Ranged weapon → ARCHER1 (cleverness=0) or ARCHER2 (cleverness=1). Melee → ATTACK1 or ATTACK2.

**Runtime transitions** in the AI loop (`fmain.c:2130-2182`):

| Condition | New Goal | Source |
|-----------|----------|--------|
| Hero dead/falling, no leader | FLEE | `fmain.c:2133-2134` |
| Hero dead/falling, leader exists | FOLLOWER | `fmain.c:2135-2136` |
| Vitality < 2 | FLEE | `fmain.c:2138` |
| Special encounter mismatch (`xtype > 59`, race ≠ extent v3) | FLEE | `fmain.c:2139-2140` |
| Weapon < 1 (unarmed) | CONFUSED | `fmain.c:2151-2152`; **only evaluated on hostile-mode reconsider ticks** (`r == TRUE`, inside the `mode ≤ ARCHER2` block — unarmed FLEE/STAND/WAIT/FOLLOWER actors are unaffected) |
| Vitality < 1 | DEATH (via `checkdead`) | `fmain.c:2774` |

### 8.2 `do_tactic()` Dispatch (`fmain2.c:1664-1699`)

All tactical movement is rate-limited by a random gate:

```c
r = !(rand() & 7);                    // 12.5% chance (fmain2.c:1666)
if (an->goal == ATTACK2) r = !(rand() & 3);  // 25% for clever melee (fmain2.c:1669)
```

When `r` is 0, the actor continues its previous trajectory unchanged.

| Tactic | `set_course` Mode | Target | Rate-limited? | Source |
|--------|-------------------|--------|---------------|--------|
| PURSUE (1) | 0 (smart seek) | Hero | Yes | `fmain2.c:1670` |
| SHOOT (8) | 0 or 5 (face only) | Hero | **No** | `fmain2.c:1671-1682` |
| RANDOM (4) | *(direct)* | Random dir | Facing only | `fmain2.c:1684` |
| BUMBLE_SEEK (3) | 4 (no snap) | Hero | Yes | `fmain2.c:1686` |
| BACKUP (5) | 3 (reverse) | Hero | Yes | `fmain2.c:1687` |
| FOLLOW (2) | 0 (smart seek) | Leader+20y | Yes | `fmain2.c:1688-1691` |
| EVADE (6) | 2 (close proximity) | Neighboring actor | Yes | `fmain2.c:1693-1695` |
| EGG_SEEK (10) | 0 (smart seek) | Fixed (23087, 5667) | Yes | `fmain2.c:1697-1699` |

SHOOT is the only tactic that fires every tick — it checks axis alignment with the hero and transitions between approaching (mode 0) and aiming/shooting (mode 5).

**RANDOM note** (`fmain2.c:1685`): The line `{ if (r) an->facing = rand()&7; an->state = WALKING; }` lacks braces around the `if`, but this is intentional. The `if (r)` gate deliberately rate-limits *direction changes* (~12.5%/tick, or 25% for ATTACK2) — without it, a RANDOM-tactic actor would pick a new heading every tick and walk in place. `an->state = WALKING` runs every tick because the actor must always be moving while RANDOM is active.

**EVADE** (`fmain2.c:1693`): `f = i+i` doubles the actor index instead of incrementing. With at most 4 active enemies, `i` maxes around 5, so `f` stays within `anim_list[20]` bounds in practice. The dead-code branch `if (i == anix) f = i-1` can never execute since the calling loop uses `i < anix`.

**FOLLOW self-targeting bug** (`fmain2.c:1689-1691`): The leader index `f` is loaded from the global `leader` variable. If the actor is itself the current leader (`f == i`), the code sets `an->tactic = RANDOM` but then falls through — with no early return — to `if (r) set_course(i, anim_list[i].abs_x, anim_list[i].abs_y+20, 0)`, targeting the actor's own position plus 20 pixels south. The RANDOM tactic is immediately overridden back to FOLLOW on the next tick when FOLLOWER-mode dispatch calls `do_tactic(i, FOLLOW)` again.

**Unused tactics**: HIDE (7) was planned but never implemented. DOOR_SEEK (11) and DOOR_LET (12) were replaced by hardcoded DKnight logic (`fmain.c:2162-2169`). None have a case in `do_tactic()`.

### 8.3 AI Main Loop (`fmain.c:2109-2183`)

The AI loop processes actors 2 through `anix-1` (skipping player and raft). Processing order:

1. **Goodfairy suspend** (`fmain.c:2112`): If fairy resurrection active (`goodfairy > 0 && < 120`), all AI halts.
2. **CARRIER type** (`fmain.c:2114-2117`): Every 16 ticks, face player with `set_course(i, hero_x, hero_y, 5)`. No other AI.
3. **SETFIG type** (`fmain.c:2119`): Skipped entirely — SETFIGs use special dialogue/rendering, not real-time AI.
4. **Distance & battle detection** (`fmain.c:2123-2131`): Within 300×300 pixels sets `actors_on_screen = TRUE` and `battleflag = TRUE`.
5. **Random reconsider** (`fmain.c:2132`): `r = !bitrand(15)` → 1/16 (6.25%) base probability of reconsidering tactics.
6. **Goal overrides** (`fmain.c:2133-2152`): Hero dead → FLEE/FOLLOWER; low health (`vitality < 2`) → FLEE; in special encounters (`xtype > 59`), any actor whose race does not match `extn->v3` → FLEE (regardless of vitality); unarmed → CONFUSED.
7. **Frustration handling** (`fmain.c:2141-2143`): FRUST or SHOOTFRUST → random escape tactic; see [§8.4](#84-frustration-cycle) for details. Note: SHOOTFRUST is dead code (defined but never assigned — see [§8.4](#84-frustration-cycle)).
8. **SHOOT1 advance** (`fmain.c:2145`): If the actor's animation state is SHOOT1 (arrow mid-release), it advances to SHOOT3. This `else if` branch **bypasses all remaining steps** — the Hostile AI block and every goal-mode handler (FLEE, FOLLOWER, STAND, etc.) do not run that tick. A FLEE-mode archer in SHOOT1 state will complete its shot transition before fleeing.
9. **Hostile AI** (`fmain.c:2146-2171`): ATTACK1–ARCHER2 modes; detailed below.
10. **FLEE** (`fmain.c:2172`): `do_tactic(i, BACKUP)`.
11. **FOLLOWER** (`fmain.c:2173`): `do_tactic(i, FOLLOW)`.
12. **STAND** (`fmain.c:2174-2176`): Face hero, force STILL state.
13. **WAIT** (`fmain.c:2178`): Force STILL state, no facing change.
14. **CONFUSED** and others: No processing — actor continues last trajectory.

At loop end, `leader` is set to the first living active enemy (`fmain.c:2183`).

#### Hostile AI Detail (`fmain.c:2146-2171`)

For modes ≤ ARCHER2, reconsider frequency is adjusted:

```c
if ((mode & 2) == 0) r = !rand4();    // 25% for ATTACK1 and ARCHER2
```

This creates a non-obvious pattern: ATTACK1 and ARCHER2 reconsider often (25%), while ATTACK2 and ARCHER1 keep the base 6.25% rate.

Tactic assignment when reconsidering (`r == TRUE`):

| Condition | Tactic | Source |
|-----------|--------|--------|
| `race==4 && turtle_eggs` | EGG_SEEK | `fmain.c:2150` |
| `weapon < 1` | RANDOM (mode→CONFUSED) | `fmain.c:2151-2152` |
| `vitality < 6 && rand2()` | EVADE | `fmain.c:2153-2154` |
| Archer, xd<40 && yd<30 | BACKUP | `fmain.c:2156` |
| Archer, xd<70 && yd<70 | SHOOT | `fmain.c:2157` |
| Archer, far away | PURSUE | `fmain.c:2158` |
| Melee, default | PURSUE | `fmain.c:2160` |

Melee engagement threshold: `thresh = 14 − mode` (`fmain.c:2162`). DKnight (race 7) overrides to 16 (`fmain.c:2163`). Within threshold, the enemy enters FIGHTING state. Outside, `do_tactic(i, tactic)` is called.

**DKnight special behavior** (`fmain.c:2168-2169`): When alive and not in melee range, DKnight stays STILL facing south (direction 5), overriding all tactical movement.

### 8.4 Frustration Cycle

When an NPC actor is blocked during movement, `tactic` is set to FRUST (`fmain.c:1661`). On the next AI tick, the frustration handler (`fmain.c:2141-2143`) catches this and selects a random escape tactic:

```
walk → blocked → FRUST → random tactic → walk → ...
```

This prevents enemies from getting permanently stuck on obstacles.

#### Blocked Check (`fmain.c:1654-1661`)

The blocked handler is in the WALKING/STILL state section of the animation loop (`fmain.c:1468`). When all three movement directions (original, clockwise +1, counterclockwise −2) are blocked by `proxcheck()` — a deviation sequence shared by all actors (see [§5.4](RESEARCH-input-movement.md#player-collision-deviation-fmainc1612-1626)):

- **Player** (`i == 0`): Increments `frustflag` and plays escalating animations — see [§5.4](RESEARCH-input-movement.md#player-collision-deviation-fmainc1612-1626) for details.
- **NPC** (`i != 0`): Sets `an->tactic = FRUST` for AI resolution next tick

#### Escape Tactic Selection (`fmain.c:2141-2143`)

When an actor's tactic is `FRUST` or `SHOOTFRUST`, a new random tactic is dispatched via `do_tactic`: ranged actors (`weapon & 4` set) get `rand4()+2` (one of FOLLOW / BUMBLE_SEEK / RANDOM / BACKUP); melee actors get `rand2()+3` (BUMBLE_SEEK or RANDOM). Full pseudo-code: [logic/frustration.md § resolve_frust_tactic](logic/frustration.md#resolve_frust_tactic).

| Weapon Type | Expression | Range | Possible Tactics |
|-------------|-----------|-------|------------------|
| Ranged (`weapon & 4` set) | `rand4()+2` | 2–5 | FOLLOW, BUMBLE_SEEK, RANDOM, BACKUP |
| Melee (`weapon & 4` clear) | `rand2()+3` | 3–4 | BUMBLE_SEEK, RANDOM |

Ranged actors get more escape options (4 tactics) than melee actors (2 tactics), giving archers more variety in obstacle navigation.

#### Cross-Goal-Mode Effect

The frustration handler fires **before** the goal-mode dispatch chain (`fmain.c:2144+`), meaning it applies to **all** goal modes — not just hostile ones:

- A **FLEE** actor that gets blocked receives BUMBLE_SEEK or RANDOM instead of BACKUP, potentially moving it *toward* the player
- A **CONFUSED** actor that gets blocked may BUMBLE_SEEK toward the player despite being "confused"
- A **FOLLOWER** that gets blocked may move in a direction that doesn't lead to the leader

This is likely a bug — the frustration handler doesn't check `mode` before reassigning the tactic.

#### SHOOTFRUST: Dead Code

SHOOTFRUST (value 9, `ftale.h:51`) is tested at `fmain.c:2141` but **never assigned anywhere in the codebase**. The comment "arrows not getting through" (`fmain.c:131`) suggests it was planned for a mechanic where archers whose missiles consistently miss would switch tactics, but the detection code was never written. The missile collision code (`fmain.c:2260-2300`) and `dohit()` (`fmain2.c:230-247`) never modify the archer's `tactic` field. See also [PROBLEMS.md §P11](PROBLEMS.md#p11-unused-tactics-hide-7-shootfrust-9-door_seek-11-door_let-12--resolved).

#### Tactic Field Overloading

The `tactic` field in `struct shape` (`ftale.h:63`) is repurposed depending on actor state:

| Actor State | Purpose of `tactic` | Values | Source |
|-------------|---------------------|--------|--------|
| AI-controlled (WALKING/FIGHTING) | Tactical sub-goal | 0–10 (tactic constants) | `fmain.c:2121` |
| TALKING (SETFIG) | Speech image-toggle countdown | 15→0 | `fmain.c:3377`, `fmain.c:1557` |
| DYING | Death animation countdown | 7→0 | `fmain.c:2773`, `fmain.c:1747` |
| FALL | Fall frame counter | 0→30 | `fmain.c:1770`, `fmain.c:1733-1736` |
| FALL (rendering) | Type selector: <16 → ENEMY sprite, ≥16 → OBJECTS sprite | any | `fmain.c:2457` |

These overloaded uses don't conflict in practice: DYING/TALKING/FALL actors are handled in the animation state machine before the AI loop processes them, so their timer values never reach the frustration handler.

#### `set_encounter()` Missing Initialization

`set_encounter()` (`fmain.c:2736-2770`) does not initialize `an->tactic` when spawning a new enemy. The field retains the stale value from the previous occupant of that `anim_list[]` slot. If the stale value is 0 (FRUST), the frustration handler fires on the first AI tick, giving the actor a random direction — a minor cosmetic bug.

### 8.5 Cleverness Effects

The `cleverness` field in `encounter_chart[]` ([§2.7](RESEARCH-data-structures.md#27-encounter_chart--monster-combat-stats)) is 0 or 1. Its effects span multiple systems:

| Property | Cleverness 0 | Cleverness 1 |
|----------|-------------|-------------|
| Goal mode | ATTACK1 / ARCHER1 | ATTACK2 / ARCHER2 |
| `do_tactic` rate | 12.5% per tick | 25% per tick (ATTACK2 only) |
| Tactic reconsider | 25% (ATTACK1) or 6.25% (ARCHER1) | 6.25% (ATTACK2) or 25% (ARCHER2) |
| Melee threshold | 13 (ATTACK1) or 11 (ARCHER1) | 12 (ATTACK2) or 10 (ARCHER2) |

ATTACK2 is the most distinctive: it reconsiders tactics rarely (6.25%) but executes them twice as often (25% vs 12.5%). This creates persistent, aggressive behavior — the actor commits to a tactic and follows through energetically.

Clever enemies (cleverness=1): Orcs, Wraith, Snake, Spider, DKnight, Loraii. Stupid enemies (cleverness=0): Ogre, Skeleton, Salamander, Necromancer, Woodcutter.

### 8.6 CONFUSED Mode

Assigned when a hostile actor loses its weapon (`weapon < 1`, `fmain.c:2151-2152`). On the first tick, `do_tactic(i, RANDOM)` runs. On subsequent ticks, CONFUSED (value 10) fails all goal-mode checks in the dispatch chain (none match), so **no mode-specific processing occurs** — the actor continues walking in its last random direction until blocked.

A CONFUSED actor can still exit via the shared goal-override block (`fmain.c:2133-2140`), which runs every tick before mode dispatch: low vitality (`< 2`) or encounter mismatch transitions to FLEE; hero death transitions to FLEE (no leader) or FOLLOWER (leader exists). These escape paths are modelled in the [normative state diagram](logic/ai-system.md).

> **Normative logic:** [reference/logic/ai-system.md](logic/ai-system.md).

---


## 9. Encounter & Spawning

### 9.1 `extent_list` — Zone Definitions

`extent_list[]` at `fmain.c:339-371` defines 22 rectangular zones plus a whole-world sentinel at index 22. Each zone specifies encounter rules via `struct extent` (`fmain.c:333-337`):

```c
struct extent { UWORD x1, y1, x2, y2; UBYTE etype, v1, v2, v3; };
```

`EXT_COUNT = 22` (`fmain.c:372`). The extent scan is first-match (`fmain.c:2676-2679`) — lower indices have higher priority.

| Idx | Location | etype | v1 | v2 | v3 | Category |
|-----|----------|-------|----|----|----|----------|
| 0 | Bird (swan) | 70 | 0 | 1 | 11 | Carrier |
| 1 | Turtle (movable) | 70 | 0 | 1 | 5 | Carrier |
| 2 | Dragon | 70 | 0 | 1 | 10 | Carrier |
| 3 | Spider pit | 53 | 4 | 1 | 6 | Forced encounter |
| 4 | Necromancer | 60 | 1 | 1 | 9 | Special figure |
| 5 | Turtle eggs | 61 | 3 | 2 | 4 | Special figure |
| 6 | Princess rescue | 83 | 1 | 1 | 0 | Peace (special) |
| 7 | Graveyard | 48 | 8 | 8 | 2 | Regular (very high danger) |
| 8 | Around city | 80 | 4 | 20 | 0 | Peace zone |
| 9 | Astral plane | 52 | 3 | 1 | 8 | Forced encounter |
| 10 | King's domain | 81 | 0 | 1 | 0 | Peace + weapon block |
| 11 | Sorceress domain | 82 | 0 | 1 | 0 | Peace + weapon block |
| 12–14 | Buildings/cabins | 80 | 0 | 1 | 0 | Peace zone |
| 15 | Hidden valley | 60 | 1 | 1 | 7 | Special figure (DKnight) |
| 16 | Swamp region | 7 | 1 | 8 | 0 | Regular (swamp) |
| 17–18 | Spider regions | 8 | 1 | 8 | 0 | Regular (spiders) |
| 19 | Village | 80 | 0 | 1 | 0 | Peace zone |
| 20–21 | Around village/city | 3 | 1 | 3 | 0 | Regular (low danger) |
| *22* | *Whole world* | *3* | *1* | *8* | *0* | *Sentinel fallback* |

Only extents 0 and 1 (bird/turtle) are persisted in savegames — `fmain2.c:1530`. The turtle extent starts at `(0,0,0,0)` (unreachable) and is repositioned via `move_extent()` during gameplay.

### 9.2 Extent Categories

The `etype` field determines zone behavior (`fmain.c:2674-2720`):

| etype Range | Category | Behavior |
|-------------|----------|----------|
| 0–49 | Regular encounter zone | Sets `xtype`; random encounters per danger timer |
| 50–59 | Forced group encounter | Monsters spawn immediately on entry **only when `find_place` is called with `flag == 1`**; `v1` = count, `v3` = monster type |
| 52 | Astral plane (special) | Forces `encounter_type = 8` (Loraii); synchronous load (`fmain.c:2696`) |
| 60–61 | Special figure | Unique NPC spawned at extent center if not already present |
| 70 | Carrier | Loads bird/turtle/dragon via `load_carrier(v3)` (`fmain.c:2716-2719`) |
| 80 | Peace zone | Blocks random encounters (`xtype ≥ 50` fails the `xtype < 50` check) |
| 81 | King peace | Peace + weapon draw blocked: `event(15)` ("Even % would not be stupid enough…") |
| 82 | Sorceress peace | Peace + weapon draw blocked: `event(16)` ("A great calming influence…") |
| 83 | Princess rescue | Triggers `rescue()` if `ob_list8[9].ob_stat` set (`fmain.c:2684-2685`) |

### 9.3 `find_place` — Zone Detection (`fmain.c:2647-2720`)

Called every frame as `find_place(2)` (`fmain.c:2049`). Two phases:

**Phase 1 — Place name** (`fmain.c:2649-2673`): Looks up `hero_sector` (masked to 8 bits) in `_place_tbl` (outdoor, `narr.asm:86`) or `_inside_tbl` (indoor when `region_num > 7`, `narr.asm:117`). Linear scan of 3-byte entries `{sector_low, sector_high, msg_index}` — first match wins.

**Phase 2 — Extent detection** (`fmain.c:2674-2720`): Linear scan of `extent_list[0..21]`. Tests `hero_x > x1 && hero_x < x2 && hero_y > y1 && hero_y < y2` (exclusive bounds). First match wins; if none, the sentinel (index 22, etype=3) applies.

Priority ordering ensures specific zones override general ones: the graveyard (idx 7, etype 48) takes priority over the surrounding city peace zone (idx 8, etype 80); the spider pit (idx 3, etype 53) overrides overlapping peace zones (idx 12–14).

#### 9.3.1 Forced-Encounter Trigger Path (`fmain.c:2682-2720`)

`find_place()` itself dispatches forced encounters whenever `xtype` changes (i.e. the hero crosses an extent boundary). This is a separate, **synchronous** code path — distinct from the periodic checks in [§9.4](#94-danger-level--spawn-logic) — and runs only on the entry tick:

| New `xtype` | Action | Code |
|-------------|--------|------|
| 83 (princess) | Call `rescue()` if `ob_list8[9].ob_stat`, then re-scan via `goto findagain` | `fmain.c:2684-2685` |
| 60 or 61 | If `anim_list[3].race != extn->v3` or `anix < 4`: set encounter origin to extent center, fall through to `force:` | `fmain.c:2687-2693` |
| 52 (astral) | Hardcode `encounter_type = 8` (Loraii); call `load_actors()` + `prep(ENEMY)` synchronously | `fmain.c:2695-2698` |
| 50–59 (other forced) | If `flag == 1`: set encounter origin to hero position; fall through to `force:`. If `flag != 1`, this branch is skipped. | `fmain.c:2699-2700` |

The shared `force:` block (`fmain.c:2702-2713`) sets `encounter_type = extn->v3`, clears `mixflag = wt = 0`, calls `load_actors()` + `prep(ENEMY)` (blocking disk read), sets `encounter_number = extn->v1` (overwriting the random `v1 + rnd(v2)` from `load_actors`), and immediately fills slots 3–6 via `set_encounter(anix, 63)`.

Finally (`fmain.c:2716-2719`):
- If `xtype < 70`: `active_carrier = 0` — any active carrier is dropped on leaving the carrier extent.
- If `xtype == 70` and either no carrier is active, or the loaded shape file doesn't match `extn->v3` while not riding: `load_carrier(extn->v3)` is called.

Because this path runs only when `xtype` changes, leaving and re-entering an etype 60 zone re-spawns the unique NPC (e.g. the DKnight, Necromancer).

> **Normative logic:** [reference/logic/astral-plane.md#find_place](logic/astral-plane.md#find_place) (full `find_place` pseudo-code, covering all dispatch branches).

#### 9.3.2 `find_place(flag)` Call Modes (`fmain.c:1789, 1928, 1951, 2050`)

`flag` controls two independent behaviors in `find_place()`:

1. **Place-name message display**: `if (flag) msg(...)` (`fmain.c:2672-2673`)
2. **Generic forced-extent activation** for `etype 50–59` (excluding astral 52): only when `flag == 1` (`fmain.c:2700`)

Current callsites:

| Callsite | Value | Place-name messages | Generic 50–59 force branch |
|----------|-------|---------------------|------------------------------|
| Main loop tick (`find_place(2)`) | 2 | Enabled | Disabled |
| Outdoor→indoor door transition (`find_place(2)`) | 2 | Enabled | Disabled |
| Indoor→outdoor door transition (`find_place(FALSE)`) | 0 | Disabled | Disabled |
| Whirlpool/sink transfer to region 9 (`find_place(1)`) | 1 | Enabled | Enabled |

Implication for implementation: `etype 60/61`, `etype 52`, `etype 70`, and `etype 83` logic is independent of this gate, but the generic `etype 50–59` branch requires a caller-controlled mode equivalent to `flag == 1`. In the current source, that path is only reached from the `find_place(1)` transfer at `fmain.c:1789`.

#### 9.3.3 Extent State Contract (Required for Ports)

`find_place()` produces two distinct pieces of state each tick:

1. **Current extent row pointer** (`extn`) from the linear rectangle scan (`fmain.c:2675-2680`)
2. **Current extent type** (`xtype = extn->etype`) only when the etype changes (`fmain.c:2682-2683`)

This distinction is required for correctness. Several systems read `extn->v3` directly outside the `xtype`-change block:

| Consumer | Condition | Why full `extn` row is needed |
|----------|-----------|--------------------------------|
| AI flee override | `xtype > 59 && race != extn->v3` | Determines which race is the "protected" special-encounter race (`fmain.c:2138-2140`) |
| Magic-use block | `if (extn->v3 == 9) speak(59)` | Disables magic in the necromancer arena (`fmain.c:3304-3305`) |
| Carrier load target | `load_carrier(extn->v3)` | Chooses bird/turtle/dragon asset ID (`fmain.c:2717-2719`) |
| Forced encounter race | `encounter_type = extn->v3` | Selects spawned race in `force:` block (`fmain.c:2704`) |

Implementation guidance: do not model extents as `xtype` alone. Preserve per-tick rectangle resolution to a concrete extent record (`x1,y1,x2,y2,etype,v1,v2,v3`), then derive `xtype` and transition side-effects from that record.

### 9.4 Danger Level & Spawn Logic

Two periodic checks drive random encounters (`fmain.c:2058-2091`):

#### Placement Check — Every 16 Frames (`fmain.c:2058-2078`)

Places already-loaded monsters into anim_list slots 3–6. Up to 10 random cluster origins are tried via `set_loc()` (`fmain2.c:1714-1720`), which picks a random point 150–213 pixels from the hero in a random compass direction. Each origin must have terrain type 0 (walkable) per `px_to_im()` (`fmain.c:2063`). Once a valid origin is found, the loop fills empty slots 3–6 first, then recycles `DEAD` enemies in the same range (wraith corpses, race=2, are recyclable even while still visible — `fmain.c:2071`).

At the start of each placement pass, `mixflag = rand()` (a 31-bit value) and `wt = rand4()` (`fmain.c:2059-2060`). `mixflag` is forced to 0 when `xtype > 49` (special extents) or when `xtype` is divisible by 4. The two consumed bits are bit 1 (race pairing, see [§9.5](#95-set_encounter--actor-placement-fmainc2736-2770)) and bit 2 (per-spawn weapon-column re-roll).

**Note — failed placements consume queue slots**: `encounter_number--` runs unconditionally inside the slot-fill loop (`fmain.c:2065-2067`), so a `set_encounter()` failure (15 collision retries exhausted) still decrements the queued count. A heavily obstructed cluster origin can therefore "eat" pending spawns silently. The dead-slot recycling loop (`fmain.c:2068-2074`) only decrements when the slot is actually recyclable, so it does not have this issue.

#### Danger Check — Every 32 Frames (`fmain.c:2080-2091`)

Conditions: no actors on screen, no pending load, no active carrier, `xtype < 50`.

Danger level formula (`fmain.c:2082-2083`):

```
Indoor (region_num > 7): danger_level = 5 + xtype
Outdoor:                 danger_level = 2 + xtype
```

Spawn probability: `rand64() <= danger_level` → `(danger_level + 1) / 64`.

| Zone | xtype | Outdoor Danger | Probability |
|------|-------|----------------|-------------|
| Whole world / around village/city | 3 | 5 | 6/64 = 9.4% |
| Swamp region | 7 | 9 | 10/64 = 15.6% |
| Spider region | 8 | 10 | 11/64 = 17.2% |
| Graveyard | 48 | 50 | 51/64 = 79.7% |

Monster type selection (`fmain.c:2086-2090`): base is `rand4()` (0–3 → ogre, orc, wraith, skeleton), with region overrides:

| Override | Condition | Monster | Source |
|----------|-----------|---------|--------|
| Swamp (xtype=7) | Wraith roll (2) → Snake | Snake (4) | `fmain.c:2087-2088` |
| Spider region (xtype=8) | All rolls forced | Spider (6) | `fmain.c:2089` |
| xtype=49 | All rolls forced | Wraith (2) | `fmain.c:2090` |

#### Monster Count — `load_actors()` (`fmain.c:2722-2735`)

```c
encounter_number = extn->v1 + rnd(extn->v2);
```

| Zone | v1 | v2 | Count Range |
|------|----|----|-------------|
| Whole world | 1 | 8 | 1–8 |
| Around village/city | 1 | 3 | 1–3 |
| Spider pit | 4 | 1 | 4 (forced) |
| Graveyard | 8 | 8 | 8–15 |

Only 4 enemy actor slots (indices 3–6) exist, so excess `encounter_number` resolves over time as the placement check recycles dead slots.

### 9.5 `set_encounter` — Actor Placement (`fmain.c:2736-2770`)

`set_encounter(i, spread)` places a single enemy in slot `i`. Up to 15 placement attempts (`MAX_TRY`):

- **DKnight fixed position**: If `extn->v3 == 7`, hardcoded at (21635, 25762) (`fmain.c:2741`). The placement loop is skipped, leaving variable `j` uninitialized — the subsequent `j == MAX_TRY` check reads garbage (technically a bug, but harmless in practice).
- **Normal**: Random offset from encounter origin: `encounter_x + bitrand(spread) - spread/2` (`fmain.c:2743-2744`). Accept if `proxcheck == 0`.
- **Astral special**: Also accept if `px_to_im == 7` (ice terrain, `fmain.c:2746`).

#### Race Mixing (`fmain.c:2753-2755`)

When `mixflag & 2` (and encounter_type ≠ snake): `race = (encounter_type & 0xFFFE) + rand2()`. This allows adjacent types to mix: ogre↔orc (0↔1), wraith↔skeleton (2↔3). `mixflag` is disabled (`= 0`) for `xtype > 49` or `xtype` divisible by 4 (`fmain.c:2059-2060`).

#### Weapon Selection (`fmain.c:2756-2758`)

```c
w = encounter_chart[race].arms * 4 + wt;
an->weapon = weapon_probs[w];
```

`wt` is re-randomized per enemy if `mixflag & 4` (`fmain.c:2756`). Otherwise all enemies in a batch share the same weapon slot index.

`weapon_probs[]` at `fmain2.c:860-868` (8 groups of 4):

| Group | Values | Weapons |
|-------|--------|---------|
| 0 | 0,0,0,0 | None |
| 1 | 1,1,1,1 | All dirks |
| 2 | 1,2,1,2 | Dirks and maces |
| 3 | 1,2,3,2 | Mostly maces, some swords |
| 4 | 4,4,3,2 | Bows and swords |
| 5 | 5,5,5,5 | All magic wands |
| 6 | 8,8,8,8 | Touch attack |
| 7 | 3,3,3,3 | All swords |

### 9.6 Special Extents

#### Carriers — Bird, Turtle, Dragon (etype 70)

Carrier loading is triggered from `find_place()` (`fmain.c:2716-2719`, see [§9.3.1](#931-forced-encounter-trigger-path-fmainc2682-2720)) when the hero enters a carrier extent without an active matching carrier. `load_carrier(n)` at `fmain.c:2784-2804` places the carrier in anim_list[3]:

| v3 | Carrier | Type Set | Notes |
|----|---------|----------|-------|
| 11 | Swan (bird) | CARRIER | Requires Golden Lasso (`stuff[5]`) to mount (`fmain.c:1498`) |
| 5 | Turtle | CARRIER | Extent starts at (0,0,0,0), must be repositioned via `move_extent()` |
| 10 | Dragon | DRAGON | Has its own fireball attack logic |

Carrier extent position: set to a 500×400 box centered on a point via `move_extent()` at `fmain2.c:1560-1566`.

##### Swan — Mount, Dismount, and Flight (`fmain.c:1417-1428`, `fmain.c:1497-1510`, `fmain.c:1581-1596`, `fmain.c:2463-2464`)

**Mount** (`fmain.c:1498`): Three conditions must all hold simultaneously — (1) hero is within 16 px of the swan (`raftprox >= 1`), (2) the carrier slot is active (`wcarry == 3`), (3) the Golden Lasso is owned (`stuff[5] != 0`). When all three hold, `riding = 11`, the swan position is snapped to the hero, and the hero's `environ` is set to −2 each frame to engage inertial flight.

**Flight** (`fmain.c:1581-1596`): The hero actor stays in WALKING state with `environ = -2`. The `walk_step` ice branch accumulates velocity from directional input (max ±32 horizontal, ±40 vertical) and divides by 4 for position updates — giving smooth momentum. No terrain collision check runs; the swan flies over all terrain. Facing is derived from velocity direction, not input.

**Dismount** (`fmain.c:1417-1427`): Triggered by the fire button while `riding == 11`. Three gates are checked in order:

1. *Lava veto*: If `fiery_death` is set (map inside `x: 8802–13562, y: 24744–29544`), dismount is blocked — `event(32)`: *"Ground is too hot for swan to land."*
2. *Velocity gate*: Both `vel_x` and `vel_y` must be in (−15, 15). If either exceeds this, dismount is blocked — `event(33)`: *"Flying too fast to dismount."*
3. *Terrain check*: `proxcheck` at hero `y − 14` and `y − 4` must both return 0. If blocked, dismount silently fails.

On success: `riding = 0` and hero y-position shifts up 14 pixels (landing offset).

**Grounded idle**: When not ridden, the swan holds its last position — no autonomous wandering. Rendering special-cases it to use RAFT sheet frame 1 instead of the carrier sheet (`fmain.c:2463-2464`), giving a small ground sprite. Every 16 game ticks, the carrier AI turns the swan toward the hero via `set_course` (`fmain.c:2114-2117`).

**Sprite model**: 8 frames, one per facing direction. No walk/fly animation cycle — the frame index equals the facing value. `FLYING` (state 21) is never used; flight is entirely implemented through `environ = -2` on the hero.

| Swan Situation | Governing Flags / State | Sprite Selection |
|----------------|-------------------------|------------------|
| Grounded, idle | `type=CARRIER`, `actor_file=11`, `riding=0` | RAFT image 1 (`fmain.c:2463-2464`) |
| Mounted / flying | `type=CARRIER`, `actor_file=11`, `riding=11`; hero `environ=-2` | Carrier image index = facing 0–7 (`fmain.c:1507`) |
| Dismount attempt | fire button while `riding==11` | stay mounted unless all three gates pass (`fmain.c:1417-1427`) |

##### Raft — Mount and Dismount (`fmain.c:1562-1574`, `fmain2.c:643-648`)

**Mount** (`fmain.c:1562-1574`): No item required. Three conditions must all hold: (1) no active carrier (`wcarry == 1`), (2) hero is within 9 px of the raft (`raftprox == 2`), (3) current terrain is 3–5 (brush/shore/water). When all three hold, the raft snaps to the hero position and `riding = 1`.

**Dismount** (`fmain.c:1563`): Implicit per-frame — `riding` is reset to `FALSE` at the start of every RAFT tick. It is only re-latched to 1 if the three mount conditions still hold that frame. Walking off the water edge, or having an active carrier loaded (which sets `wcarry = 3`), immediately ends the ride with no input required.

**Sprite model**: `cfiles[4]`: width=2, height=32, count=2, `seq_num=RAFT`, file\_id=1348. The RAFT handler jumps to `statc` without writing `an->index`, so frame 0 is the only raft image ever displayed. Frame 1 in the same sheet is the grounded-swan sprite, accessed only via the swan rendering override.

| Raft Situation | Governing Flags | Sprite Selection |
|----------------|-----------------|------------------|
| On water near hero | `wcarry==1`, `raftprox==2`, terrain 3–5 | RAFT image 0 — static (`fmain.c:2797`) |
| Out of range / wrong terrain | conditions false | `riding=0`; sprite not drawn over hero |

##### Turtle — Mount, Dismount, and Summoning (`fmain.c:1511-1545`)

**Summoning via Sea Shell** (`fmain.c:3457-3461`): The turtle does not have a fixed world position — it must be summoned. Using the Sea Shell (inventory slot 6) calls `get_turtle()`, which tries up to 25 random nearby locations for terrain type 5 (very deep water). If found, the turtle extent is repositioned there via `move_extent(1, x, y)` and the turtle is loaded. Silently fails if no water is within range. Blocked inside the swamp rectangle (x: 11194–21373, y: 10205–16208). The Sea Shell is obtained by talking to the turtle — first encounter triggers `speak(56)` and sets `stuff[6]` (`fmain.c:3419-3420`).

**Mount** (`fmain.c:1512-1516`): Hero within 16 px (`raftprox >= 1`) and carrier slot active (`wcarry == 3`). No item required — proximity alone is sufficient once the turtle is present. Sets `riding = 5`.

**Dismount** (`fmain.c:1538`): Implicit when proximity is lost. Each tick, if `raftprox && wcarry == 3` fails, the autonomous swim branch runs and sets `riding = FALSE`. Walking away or leaving the water area ends the ride.

**Sprite model**: `cfiles[5]`: width=2, height=32, count=16, `seq_num=CARRIER`, file\_id=1351 — 16 frames (8 directions × 2-frame walk cycle). The turtle handler exits via `goto raise`, so `an->index = dex` is written every tick.

| Turtle Situation | Governing Flags | Sprite Selection |
|------------------|-----------------|------------------|
| Mounted, hero still | `riding=5`, hero `state!=WALKING` | `dex = facing*2` (frame 0 of current direction) |
| Mounted, hero walking | `riding=5`, hero `state==WALKING` | `dex = facing*2 + (cycle&1)` — 2-frame cycle |
| Autonomous (not mounted) | `riding=0`, autonomous handler | `dex = facing*2 + (cycle&1)` — always cycles |

##### Turtle Autonomous Movement (`fmain.c:1520-1542`)

When not ridden, the turtle is restricted to **terrain type 5 only** (very deep water). It uses `px_to_im()` directly — not `proxcheck()` — and only commits position updates when the result is exactly 5 (`fmain.c:1523,1527,1531,1541`). Shallower water (types 2–4) and all land types are impassable to the autonomous turtle.

Each tick, 4 directions are probed in priority order from the current facing `d`: `d`, `(d+1)&7`, `(d-1)&7`, `(d-2)&7`. The first direction landing on terrain 5 is selected (`fmain.c:1521-1534`). If all fail, the turtle does not move. Speed is always 3 (`fmain.c:1521-1522`).

`load_carrier()` does not initialize `an->facing` (`fmain.c:2784-2801`), and the carrier handler exits via `goto raise` (`fmain.c:1545`), bypassing the `an->facing = d` write at `newloc:` (`fmain.c:1633`). The turtle's facing is therefore never persisted — it retries the same directional probe sequence every tick.

**Bug — extent drift**: When no valid direction exists, `xtest`/`ytest` retain the last failed probe's coordinates. `move_extent(1, xtest, ytest)` still executes (`fmain.c:1545`), repositioning the extent to a non-water location while `abs_x`/`abs_y` stay fixed. See [PROBLEMS.md §P22](PROBLEMS.md).

> **Normative logic:** [reference/logic/carrier-transport.md](logic/carrier-transport.md)

#### Spider Pit (etype 53, index 3)

Forced encounter: spawns `v1=4` spiders (`v3=6`) when entered via a `find_place(1)` call path (`fmain.c:2700`). `mixflag=0, wt=0` — no mixing, all spiders get the same touch attack weapon.

#### Necromancer / DKnight (etype 60)

Spawns unique NPC at extent center. Only spawns if the NPC isn't already present (`anim_list[3].race != v3` or `anix < 4`, `fmain.c:2687-2693`).

#### Princess Rescue (etype 83, index 6)

When entered and `ob_list8[9].ob_stat` is set (princess captured), calls `rescue()` (`fmain2.c:1584-1605`): displays placard text, increments `princess` counter, teleports hero to (5511, 33780), and repositions the bird extent via `move_extent(0, 22205, 21231)`.

### 9.7 Peace Zones

Extents with etype 80–83 set `xtype ≥ 50`, which fails the `xtype < 50` guard on the danger check (`fmain.c:2081`). This completely suppresses random encounters.

Additional enforcement for etype 81 (King's domain) and 82 (Sorceress domain): drawing a weapon triggers `event(15)` or `event(16)` respectively — admonishing messages that prevent combat initiation.

The `aggressive` field in `encounter_chart[]` is defined for all monster types but is **never read** by any runtime code (`fmain.c:44`). Peace zones rely entirely on the extent system, not per-monster aggression flags.

### 9.8 Dark Knight (DKnight)

The Dark Knight — called "DKnight" in source, "Knight of Dreams" in narrative text — is a unique fixed-position enemy guarding the elf glade entrance in the hidden valley.

#### 9.8.1 Identity

`encounter_chart[7]` at `fmain.c:61`:

```
{ 40, TRUE, 7, 1, 0, 8 }   /* 7 - DKnight - elf glade */
```

| Field | Value | Meaning |
|-------|-------|---------|
| hitpoints | 40 | Highest non-boss HP (Necromancer has 50) |
| aggressive | TRUE | (field never read at runtime — see §9.7) |
| arms | 7 | `weapon_probs[28–31]` = `3,3,3,3` → sword only (`fmain2.c:867`) |
| cleverness | 1 | Goal = ATTACK1 + 1 = ATTACK2 |
| treasure | 0 | Group 0 — no treasure drops |
| file_id | 8 | `cfiles[8]` = `{ 1,32,64, 40, ENEMY, 1000 }` (`fmain2.c:653`) |

The sprite data (`cfiles[8]`) specifies a 16×32-pixel sprite with 64 animation frames, loaded from disk blocks 1000–1039. Spiders (encounter\_type 6, file\_id 8) reference the same `cfiles` entry; the two enemy types share a single sprite sheet on disk. The comment at `fmain2.c:653` reads `/* dknight file (spiders) */`.

#### 9.8.2 Spawning

The DKnight spawns via `extent_list[15]` at `fmain.c:360`:

```
{ 21405, 25583, 21827, 26028, 60, 1, 1, 7 }   /* hidden valley */
```

- **etype 60** — special figure encounter (shared with the Necromancer).
- **v3 = 7** — encounter\_type 7 (race 7, DKnight).
- Zone bounds: (21405, 25583) to (21827, 26028), a 422×445 world-unit rectangle.

**Spawn trigger** (`fmain.c:2688-2691`): on zone entry, if `anim_list[3].race != extn->v3` or `anix < 4`, a new DKnight is spawned. The DKnight **respawns every time** the player re-enters the hidden valley.

**Hardcoded position** (`fmain.c:2741`): `if (extn->v3==7) { xtest = 21635; ytest = 25762; }` — the random-placement loop is skipped entirely. This fixed positioning is unique to the DKnight among all encounter types.

**Bug — uninitialized `j`**: Because the placement loop is skipped, variable `j` (declared at `fmain.c:2738`) is never assigned. The subsequent `if (j==MAX_TRY) return FALSE;` at `fmain.c:2749` reads an indeterminate value. This is technically undefined behavior but harmless in practice — a garbage register value is unlikely to equal exactly 15.

#### 9.8.3 AI Behavior

The DKnight bypasses the normal tactic system with hardcoded logic at `fmain.c:2162-2169`:

- **Melee threshold override** (`fmain.c:2163`): `if (an->race == 7) thresh = 16;` — normal ATTACK2 enemies use `thresh = 14 - mode` = 12. The DKnight's effective engagement radius is 33% larger.
- **In range** (xd < 16 AND yd < 16): Enters FIGHTING state and attacks the hero with its sword (`fmain.c:2164-2166`).
- **Out of range** (`fmain.c:2168-2169`): `an->state = STILL; an->facing = 5;` — stands motionless facing south (direction 5). Does **not** pursue. Does **not** call `do_tactic()`.
- **No fleeing** (`fmain.c:2139-2140`): For etype 60 zones (`xtype > 59`), actors whose race matches `extn->v3` are exempt from flee mode. Since the DKnight's race (7) matches v3 (7), it never flees, even at vitality 1.

This "stand still facing south" behavior is the door-blocking mechanic: the DKnight physically obstructs passage at its fixed position and only engages when the hero comes within melee range.

#### 9.8.4 Vestigial DOOR\_SEEK / DOOR\_LET

`ftale.h:53-54` defines two goal modes that were evidently planned for the DKnight:

```
#define DOOR_SEEK  11   /* dknight blocking door */
#define DOOR_LET   12   /* dknight letting pass */
```

Neither constant is referenced in any `.c` file. No code ever assigns `goal = 11` or `goal = 12`. The `do_tactic()` switch at `fmain2.c:1664-1700` has no case for either value. These were presumably replaced by the simpler hardcoded logic described above; `fmain.c:121-131` redefines goal constants up to CONFUSED (10) and omits DOOR\_SEEK/DOOR\_LET entirely.

#### 9.8.5 Speech

Two race-specific messages are triggered for the DKnight:

| Event | Call | narr.asm | Text |
|-------|------|----------|------|
| Proximity | `speak(41)` at `fmain.c:2101` | `narr.asm:462-465` | *"Ho there, young traveler!" said the black figure. "None may enter the sacred shrine of the People who came Before!"* |
| Death | `speak(42)` at `fmain.c:2775` | `narr.asm:466-469` | *"Your prowess in battle is great." said the Knight of Dreams. "You have earned the right to enter and claim the prize."* |

The proximity speech fires when the DKnight is `nearest_person` and the hero is nearby (`fmain.c:2094-2103`). The death speech fires inside `checkdead()` when DKnight vitality drops below 1 — it is the only race-specific death speech triggered directly in `checkdead()`.

#### 9.8.6 Quest Connection

The DKnight guards `doorlist[48]` — the **elf glade** entrance (`fmain.c:288`):

```
{ 0x5470, 0x6480, 0x2c80, 0x8d80, HSTONE, 1 }   /* elf glade */
```

The door's outside coordinates (21616, 25728) are 19 pixels from the DKnight's fixed position (21635, 25762). There is no programmatic gate — no code checks whether the DKnight is alive to enable or disable passage. The "door" is simply the DKnight's physical body blocking the path. Because the DKnight respawns on zone re-entry, the player must defeat it each time.

Inside the elf glade, `ob_list8[18]` at `fmain2.c:1092` places the **Sun Stone** (`stuff[7]`), the "prize" referenced in `speak(42)`. The Sun Stone is required to damage the Witch: without it (`stuff[7] == 0`), melee attacks against the Witch (race `0x89`) are blocked with `speak(58)` (`fmain2.c:231-233`). For the full witch combat flow, see [STORYLINE.md §5.4](STORYLINE.md#54-witch-combat-encounter).

On death, the only lasting mechanical effect is `brave++` (`fmain.c:2777`). Bravery affects max vitality (`15 + brave/4`, `fmain.c:2901`), combat strength (`brave/20 + 5`, `fmain.c:2249`), and enemy hit chance (`rand256() > brave`, `fmain.c:2260`). No quest flags are set and no inventory items are granted.

---

