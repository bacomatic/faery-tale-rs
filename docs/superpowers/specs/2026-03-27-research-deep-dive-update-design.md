# RESEARCH.md Deep Dive Audit & Update — Design Spec

**Date**: 2026-03-27
**Scope**: Fix factual errors and fill documentation gaps in RESEARCH.md
**Source**: Cross-reference of `original/fmain.c`, `fmain2.c`, `fsubs.asm` against current RESEARCH.md

---

## Problem Statement

A thorough audit of RESEARCH.md against the original source code found:

- **5 factual errors** where RESEARCH.md contradicts the source
- **24 missing game mechanics** not documented anywhere
- **8 incomplete sections** with critical details omitted

The document needs surgical corrections and targeted additions to serve as a reliable reverse-engineering reference for the Rust port.

---

## Phase 1: Fix Factual Errors (In-Place Edits)

### Fix A: Region Selection Formula

**Location**: "World Map: Region Diagrams" section, `gen_mini()` subsection (~line 1905)

**Current** (wrong):
```
xs = (hero_x + 7) >> 8
ys = (hero_y - 26) >> 8
```

**Correct** (from `fmain.c:3661-3665`):
```
xs = (map_x + 151) >> 8   // center of screen in sector coords
ys = (map_y + 64) >> 8
```

Uses `map_x`/`map_y` (viewport top-left), not `hero_x`/`hero_y`. The offsets 151 and 64 center the calculation within the viewport.

### Fix B: Day Fade Green Jewel Description

**Location**: "Day/Night Cycle" section (~line 284)

**Current** (wrong): "light_timer (Green Jewel light effect) temporarily equalises R and G channels"

**Correct**: light_timer adds +200 to the **red channel parameter only** in `day_fade()`:
```c
ll = light_timer ? 200 : 0;
fade_page(lightlevel - 80 + ll, lightlevel - 61, lightlevel - 62, TRUE, pagecolors);
```
R gets `lightlevel + 120`, G stays at `lightlevel - 61`, B at `lightlevel - 62`. Makes the scene warm/bright, not colour-balanced.

### Fix C: Hunger/Fatigue Vitality Damage

**Location**: "Hunger & Fatigue System" section, hunger progression table (~line 243)

**Current** (wrong): ">100 (every 8 ticks, if vitality > 5) | `vitality -= 2` if also `fatigue > 160`"

**Correct** (from `fmain.c:2632-2635`):
```c
if (hunger > 100 || fatigue > 160) {
    anim_list[0].vitality -= 2;
```
It's **OR**, not "if also." Either hunger > 100 or fatigue > 160 independently causes vitality -= 2 (when vitality > 5 and `(hunger & 7) == 0`).

Rewrite the hunger progression table:
- `>100 (every 8 ticks, if vitality > 5)`: `vitality -= 2` (starvation damage)
- Move fatigue > 160 to the fatigue table: `>160 (every 8 ticks, if vitality > 5)`: `vitality -= 2` (exhaustion damage)
- Clarify both trigger independently via `hunger > 100 || fatigue > 160`

### Fix D: Green Jewel lightlevel Claim

**Location**: Magic consumables table, Green Jewel row (~line 673)

**Current** (wrong): "adds +200 to lightlevel calculation"

**Correct**: Adds +200 to the **red channel parameter** in `day_fade()`. Does not modify the `lightlevel` variable itself.

### Fix E: Fatigue Passive Decrement

**Location**: "Hunger & Fatigue System" section (~line 253)

**Current** (wrong): "`fatigue` decrements by 1 per daynight tick passively."

**Correct**: Fatigue has **no passive decrement**. It only decrements during SLEEP state (−1 per frame). Verified by grep — every fatigue reference in the source accounted for. The only `fatigue--` is at `fmain.c:2361` inside the `state == SLEEP` block.

---

## Phase 2: Expand Existing Sections

### Expand: Combat System

Add after the existing hit detection subsection:

**Hero dodge mechanic** — When an NPC (i > 0) attacks the hero, the hit check includes a dodge roll:
```c
if (yd < bv && (i == 0 || rand256() > brave))
```
- Hero (i==0): always hits if in range (no dodge)
- NPCs attacking hero: hit only connects if `rand256() > brave`
- This is the primary defensive scaling — higher brave = more dodges
- Dodge rate = `(brave + 1) / 256`. At brave=35 (Julian start): ~14% dodge. At brave=100: ~39% dodge. At brave=255: ~100% dodge.

**NPC fight state clamping** — NPCs (i > 2) clamp fight animation states 6 and 7 to state 8, limiting their animation variety compared to the hero player character.

### Expand: Hunger & Fatigue System

Add after existing hunger thresholds:

**Hunger stumble** (`fmain.c:1625-1631`) — When `hunger > 120`, on each movement step there is a 1/4 chance (`!rand4()`) the movement direction deviates by ±1:
```c
if (hunger > 120 && !(rand4())) {
    if (rand() & 1)
        oldir = (oldir + 1) & 7;
    else
        oldir = (oldir - 1) & 7;
}
```
This makes the hero stagger drunkenly when severely hungry.

Rewrite vitality damage conditions as described in Fix C above.

Add clarifying note: "Fatigue only decrements during sleep. There is no passive fatigue recovery while awake."

### Expand: Day/Night Cycle

Add frame rate mechanism note at the top of the section:

**Frame rate**: The game loop effectively runs at ~30 Hz. During scrolling, the Amiga blitter is saturated (5-plane scroll + sprite blits exceed one VBlank period). When standing still, an explicit `Delay(1)` in `ppick()` (`fmain2.c:621`) adds a 1/60s pause. `daynight` is incremented in the main game loop (`fmain.c:2370`), not in the VBlank interrupt (which only handles music).

Add indoor brightness note: For `region_num >= 8` (indoor), `day_fade()` uses `fade_page(100, 100, 100, TRUE, pagecolors)` — full brightness, no day/night variation.

### Expand: Sleep System

Currently part of Hunger/Fatigue. Expand the brief mention into a proper subsection:

**Sleep time acceleration**: `daynight += 63` per frame during sleep. At ~30 fps, this advances ~1,890 daynight ticks per second (~6.3 in-game hours per real second).

**Wake conditions** (any of):
1. `fatigue == 0` (fully rested)
2. `fatigue < 30` AND `daynight` in 9000–10000 (dawn — hero wakes at sunrise if reasonably rested)
3. In battle (`battleflag`) AND `rand64() == 0` (1/64 chance per frame)

**Bed mechanics**:
- Bed tiles: sector IDs 52, 53, 161, 162 (only in region 8)
- 30-frame standing timer (`sleepwait`) — must stand still on bed for ~1 second
- Fatigue requirement: ≥ 50 ("Not tired enough" if below)
- Y-axis snap: `hero_y |= 0x1f` on sleep entry, `hero_y &= 0xffe0` on wake (aligns to 32-pixel tile grid)
- Movement resets `sleepwait` counter

### Expand: Death/Revival

Add fairy rescue animation details:

**Fairy rescue animation** (`fmain.c:1557-1582`):
- When hero enters DEAD or FALL state, `goodfairy` counter begins decrementing
- If `goodfairy < 120`: fairy sprite appears at `hero_x + goodfairy*2 - 20`, animated with cycling
- If `goodfairy == 1`: `revive(FALSE)` triggers — fairy rescue succeeds
- If FALL state and `goodfairy < 200`: automatic rescue from pit

**New brother flow** (`revive(TRUE)`) additional details:
- Dead brother position stored: `ob_listg[brother].xc = hero_x`, `.yc = hero_y`, `.ob_stat = 1`
- Ghost brother enabled: `ob_listg[brother + 2].ob_stat = 3` (setfig)
- All inventory cleared: `stuff[0..GOLDBASE-1] = 0`
- Starting dirk given: `stuff[0] = weapon = 1`
- Raft spawns at (13668, 14470)
- Good fairy setfig placed at (13668, 15000)
- `brother > 3` → `quitflag = TRUE` (game over, 500-tick delay before quit)

**Common to both revival paths**: `vitality = 15 + brave/4`, `daynight = 8000`, `lightlevel = 300`, `hunger = fatigue = 0`, `anix = 3` (clear enemies).

### Expand: Riding System

**Raft proximity** (`fmain.c:1645-1651`):
- Distance calculated as `hero - raft - 4` on each axis
- 16 pixels: `raftprox = 1` (near — raft follows hero)
- 9 pixels: `raftprox = 2` (close enough to board)

**Bird/swan flying physics** (`fmain.c:1788-1812`):
- Acceleration-based: `vel += newx/newy(20, dir, 2) - 20` per frame
- Speed caps: horizontal < e-8, vertical < e (e=40 for bird, e=42 otherwise)
- Position update: `abs_x += vel_x / 4`, `abs_y += vel_y / 4`
- No terrain collision while flying — bypasses `proxcheck()`
- Dismount: fight button when velocity both axes within ±15 AND `proxcheck(x, y-14)` passes (passable terrain above)

**Dragon combat** (`fmain.c:1675-1692`):
- Fires randomly: 25% chance per tick (`rand4() == 0`)
- Always fires direction 5 (south/down)
- Fireball: `missile_type = 2`, `speed = 5`
- HP: 50 (from encounter_chart)
- Extent zone 2: coordinates (6749, 34951)–(7249, 35351)

---

## Phase 3: Add New Sections

### New Section: Movement System

Place after "Key Bindings" section (before or after "Input Decoding").

Contents:
- **Movement formula**: `new_pos = old_pos + (dir_table[dir] * speed) / 2`
- **Direction tables**: `xdir = {-2, 0, 2, 3, 2, 0, -2, -3, 0, 0}`, `ydir = {-2, -3, -2, 0, 2, 3, 2, 0, 0, 0}`
- **Speed values table**:

| Condition | Speed `e` | Source |
|-----------|-----------|--------|
| Riding raft (`riding == 5`) | 3 | fmain.c:1817 |
| Backwards terrain (`environ == -3`) | -2 | fmain.c:1819 |
| Slow terrain (`environ == -1`, type 6) | 4 | fmain.c:1821 |
| Sinking (`environ == 2` or `> 6`) | 1 | fmain.c:1822 |
| Normal | 2 | fmain.c:1823 |

- **Walk/still transitions**: `oldir < 9` + qualifier/key held → WALKING (12); `oldir == 9` → STILL (13)
- **Wall sliding**: On terrain block, tries `(direction + 1) & 7`, then `(direction - 2) & 7`. If both fail → frustration.
- **Frustration animation**: `frustflag > 20` → oscillating frames (84-85); `frustflag > 40` → unique pose (frame 40). NPCs use `tactic = FRUST` instead.
- **Coordinate wrapping** (outdoor regions only): `abs_x < 300` → 32565; `abs_x > 32565` → 300; same for Y. Conditions are `else if` chained — only one axis wraps per frame.
- **Y-axis high bit**: `hero_y & 0x8000` indicates indoor coordinates. Wrapping preserves this bit.
- **Velocity tracking**: `vel_x = (xtest - abs_x) * 4`, `vel_y = (ytest - abs_y) * 4`. Used for interpolation, pushback, bird dismount.

### New Section: Encounter Spawning System

Place after "Extents and Encounter Zones" section.

Contents:
- **Random encounter timing**: Every 32 ticks (`daynight & 31 == 0`), when no actors on screen, no actors loading, no carrier, `xtype < 50`.
- **Danger level**: Indoor (region > 7): `5 + xtype`. Outdoor: `2 + xtype`.
- **Spawn chance**: `rand64() <= danger_level`
- **Type selection**: `encounter_type = rand4()` with overrides:
  - Swamp (xtype == 7): type 2 → 4 (snake instead of wraith)
  - Spider zone (xtype == 8): type forced to 6 (spider)
  - xtype == 49: type forced to 2 (wraith)
- **Encounter placement**: Every 16 ticks (`daynight & 15 == 0`) when `encounter_number > 0`. Random direction and distance (150 + rand64 pixels). Up to 10 attempts for passable terrain. Places in slots 3-6 (max 4 enemies). Dead wraiths (race 2) recycled immediately.
- **DKnight fixed position**: Race 7 always spawns at (21635, 25762) instead of random placement.
- **Mix flag** (`mixflag`): `& 2` → race varies within pair (even/odd encounter IDs); `& 4` → weapon varies.
- **Object distribution**: On first visit to a region, 10 random treasure objects scattered using `rand_treasure[bitrand(15)]` at random passable coordinates within region bounds: `x = bitrand(0x3fff) + ((region_num & 1) * 0x4000)`, `y = bitrand(0x1fff) + ((region_num & 6) * 0x1000)`.
- **Battle aftermath** (`aftermath()`): Counts dead/fleeing enemies. Reports "Bravely done!" if hero vitality < 5, otherwise prints tallies. If `turtle_eggs` flag set, calls `get_turtle()`.

### New Section: NPC Interaction Mechanics

Place after or as expansion of existing "NPC Behavior" section.

Contents:
- **Auto-speak on proximity** (`fmain.c:2473-2494`): When `nearest_person` changes race, automatic dialogue triggers:
  - Beggar (0x8d): speak(23)
  - Witch (0x89): speak(46)
  - Princess (0x84): speak(16) if `ob_list8[9].ob_stat`
  - Necromancer (9): speak(43)
  - DKnight (7): speak(41)
- **Talk range**: Yell = 100 pixels, Say/Ask = 50 pixels
- **Sorceress luck bonus** (`fmain.c:4225-4234`): First visit sets `ob_listg[9].ob_stat = 1` and triggers speak(45). Subsequent visits: if `luck < rand64()` (0-63), `luck += 5`.
- **Bartender dialogue** (`fmain.c:4235-4241`): Response varies based on fatigue level and time of day (`fatigue < 5` check).
- **Guard dialogue**: Guards (setfig 2, 3) always say speak(15).
- **Princess rescue rewards** (`fmain2.c:1969-2001`):
  - `wealth += 100`
  - `stuff[28] = 1` (Writ)
  - `stuff[16..21] += 3` (3 of each key type)
  - Teleported to throne room at (5511, 33780)
  - Bird extent moved to (22205, 21231)
  - Noble NPC replaced with princess (ob_id 4)
  - `ob_list8[9].ob_stat = 0` reset
  - `princess++` counter incremented
- **Witch visual attack** (`fmain2.c:1226-1292`): Rotating quadrilateral distortion using `witchpoints[]` (64-point circle table). Damage check: hero inside cone AND `calc_dist(2, 0) < 100` → `dohit(-1, 0, facing, rand2()+1)` (1-2 damage).
- **Necromancer death transformation** (`fmain.c:2006-2017`): On death, necromancer transforms to woodcutter: `race = 10`, `vitality = 10`, `state = STILL`, `weapon = 0`. Then leaves item 139 (talisman).

### New Section: Distance Calculation

Short section, can be placed in Combat System or as standalone utility reference.

```c
long calc_dist(a, b) {
    x = abs(a.abs_x - b.abs_x);
    y = abs(a.abs_y - b.abs_y);
    if (x > y + y) return x;       // mostly horizontal
    if (y > x + x) return y;       // mostly vertical
    return (x + y) * 5 / 7;        // diagonal approximation
}
```

Piecewise linear approximation of Euclidean distance. Used throughout combat hit detection, NPC proximity checks, and encounter zone calculations.

---

## Phase 4: Index Updates

- Add entries in `research_index.toml` for new sections: Movement System, Encounter Spawning, NPC Interaction Mechanics, Distance Calculation
- Bump `last_updated` on all modified existing entries
- Verify all anchor slugs match markdown headings

---

## Out of Scope

- Cheat system activation mechanism (unknown in source — `cheat1` never set)
- Complete `rand_treasure[]` table (would require finding the array definition)
- Witch `witchpoints[]` full data dump (implementation detail, not needed for reference)
- Full extent_list[22] data (already partially documented)
