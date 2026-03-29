# RESEARCH.md Deep Dive Update — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 5 factual errors, expand 6 existing sections, and add 4 new sections to RESEARCH.md based on the source code audit.

**Architecture:** Pure documentation edits to `RESEARCH.md` and `research_index.toml`. No code changes. Each task is a logically grouped set of edits to one or two sections, committed separately.

**Tech Stack:** Markdown, TOML

**Spec:** `docs/superpowers/specs/2026-03-27-research-deep-dive-update-design.md`

---

### Task 1: Fix factual errors in Hunger & Fatigue section

**Files:**
- Modify: `RESEARCH.md` (lines 229–258, Hunger & Fatigue System section)

- [ ] **Step 1: Fix the hunger/fatigue vitality damage condition (Fix C)**

In `RESEARCH.md`, replace the hunger progression table row that says "if also fatigue > 160" with the correct OR condition, and add fatigue damage to the fatigue table:

Replace:
```markdown
| `hunger` value | Event |
|----------------|-------|
| 35             | event(0) — "Getting hungry" message |
| 60             | event(1) — "Very hungry" message |
| 90             | event(4) — "Famished" message |
| >100 (every 8 ticks, if vitality > 5) | `vitality -= 2` if also `fatigue > 160` |
| >90 (every 8 ticks, if vitality > 5) | event(2) — starvation warning |
| >140 (every 8 ticks) | event(24), `hunger` clamped to 130, `state = SLEEP` (collapse) |

**Fatigue progression:**

| `fatigue` value | Event |
|-----------------|-------|
| 70              | event(3) — "Weary" message |
| >170 (every 8 ticks, vitality ≤ 5) | event(12), `state = SLEEP` (collapse from exhaustion) |

`fatigue` decrements by 1 per daynight tick passively. Sleep in combat (battleflag) or very low fatigue triggers forced `SLEEP` state. Sleeping on interior tiles 161, 52, 162, or 53 after `sleepwait` reaches 30 also triggers sleep (if `fatigue > 50`).
```

With:
```markdown
| `hunger` value | Event |
|----------------|-------|
| 35             | event(0) — "Getting hungry" message |
| 60             | event(1) — "Very hungry" message |
| 90             | event(4) — "Famished" message |
| >100 (every 8 ticks, if vitality > 5) | `vitality -= 2` (starvation damage) |
| >90 (every 8 ticks, if vitality > 5) | event(2) — starvation warning |
| >140 (every 8 ticks) | event(24), `hunger` clamped to 130, `state = SLEEP` (collapse) |

**Hunger stumble** (`fmain.c:1625–1631`): when `hunger > 120`, on each movement step there is a 1/4 chance (`!rand4()`) the direction deviates by ±1:
```c
if (hunger > 120 && !(rand4())) {
    if (rand() & 1)
        oldir = (oldir + 1) & 7;
    else
        oldir = (oldir - 1) & 7;
}
```
This makes the hero stagger drunkenly when severely hungry.

**Fatigue progression:**

| `fatigue` value | Event |
|-----------------|-------|
| 70              | event(3) — "Weary" message |
| >160 (every 8 ticks, if vitality > 5) | `vitality -= 2` (exhaustion damage) |
| >170 (every 8 ticks, vitality ≤ 5) | event(12), `state = SLEEP` (collapse from exhaustion) |

**Vitality damage**: starvation and exhaustion trigger independently (`fmain.c:2632–2635`): `if (hunger > 100 || fatigue > 160) { vitality -= 2; }` when `vitality > 5` and `(hunger & 7) == 0`.

`fatigue` has **no passive decrement** — it only decreases during the SLEEP state (−1 per frame while asleep). There is no fatigue recovery while awake.
```

- [ ] **Step 2: Verify edit renders correctly**

Visually scan the replacement to confirm:
- Table pipes are aligned
- Code block is properly fenced
- No orphaned text

- [ ] **Step 3: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: fix hunger/fatigue vitality damage condition and add stumble mechanic

The hunger > 100 vitality damage is OR'd with fatigue > 160, not
AND'd. Also documents the hunger stumble mechanic (direction deviation
at hunger > 120) and clarifies fatigue has no passive decrement.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 2: Fix factual errors in Day/Night and Green Jewel sections

**Files:**
- Modify: `RESEARCH.md` (Day/Night Cycle section ~line 261, Magic consumables table ~line 673)

- [ ] **Step 1: Fix day fade description (Fix B) and add frame rate/indoor notes**

In the Day/Night Cycle section, replace lines 263–264 with expanded content including frame rate mechanism note. Replace:
```markdown
`daynight` is a `USHORT` counter [0..23999], incremented by 1 per game tick (30 Hz):
- 24000 ticks = one full in-game day ≈ 800 seconds real time (≈13.3 minutes)
```

With:
```markdown
`daynight` is a `USHORT` counter [0..23999], incremented by 1 per game tick (~30 Hz):
- 24000 ticks = one full in-game day ≈ 800 seconds real time (≈13.3 minutes)
- `daynight` is incremented in the main game loop (`fmain.c:2370`), not in the VBlank interrupt (which only handles music)

**Frame rate**: the game loop effectively runs at ~30 Hz. During scrolling, the Amiga blitter is saturated (5-plane scroll + sprite blits exceed one 16.7 ms VBlank period). When standing still, an explicit `Delay(1)` in `ppick()` (`fmain2.c:621`) throttles the idle loop to match.
```

Then replace the `fade_page` paragraph (line 284):
```markdown
`fade_page(r, g, b, limit, colors)` applies per-frame colour scaling. Night limit floor: r≥10, g≥25, b≥60 (ensures blue-tinted night). `light_timer` (Green Jewel light effect) temporarily equalises R and G channels.
```

With:
```markdown
**`day_fade()`** (`fmain2.c:2059–2071`) calls `fade_page()` every 4th `daynight` tick (`daynight & 3 == 0`) or on viewstatus changes:

```c
ll = light_timer ? 200 : 0;
if (region_num < 8)
    fade_page(lightlevel - 80 + ll, lightlevel - 61, lightlevel - 62, TRUE, pagecolors);
else
    fade_page(100, 100, 100, TRUE, pagecolors);  // full brightness indoors
```

`fade_page(r, g, b, limit, colors)` applies per-frame colour scaling. Night limit floor: r≥10, g≥25, b≥60 (ensures blue-tinted night). `light_timer` (Green Jewel) adds +200 to the **red channel parameter only**, making the scene warm and bright. Indoor regions (`region_num ≥ 8`) always use full brightness (100, 100, 100) — no day/night variation.
```

- [ ] **Step 2: Fix Green Jewel description in magic items table (Fix D)**

In the magic consumables table, replace the Green Jewel row (line 673):
```markdown
| 10 | Green Jewel | 6 | `light_timer += 760` | ~760 game ticks | **Illumination**: `day_fade()` boosts red channel (`r1 = g1` when `r1 < g1`), adds +200 to lightlevel calculation. Makes night as bright as day. Palette color 31 unaffected. Timer decrements each main-loop tick |
```

With:
```markdown
| 10 | Green Jewel | 6 | `light_timer += 760` | ~760 game ticks | **Illumination**: `day_fade()` adds +200 to red channel parameter in `fade_page()`, making night nearly as bright as day. Does not modify `lightlevel` itself. Palette color 31 unaffected. Timer decrements each main-loop tick |
```

- [ ] **Step 3: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: fix day/night fade formula and add frame rate mechanism note

Corrects Green Jewel description (adds +200 to red channel param, not
lightlevel). Adds frame rate note (30 Hz blitter-bound). Documents
indoor full brightness. Shows day_fade() source formula.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 3: Fix region selection formula

**Files:**
- Modify: `RESEARCH.md` (World Map: Region Diagrams section ~line 1902)

- [ ] **Step 1: Fix gen_mini() formula (Fix A)**

Replace:
```markdown
### Region selection formula (`gen_mini()`, `fmain.c:3661–3690`)

```c
xs = (hero_x + 7) >> 8        // sector column of viewport centre
ys = (hero_y - 26) >> 8       // sector row of viewport centre
xr = (xs >> 6) & 1            // 0 = west column, 1 = east column
yr = (ys >> 5) & 3            // 0–3 = north → south band
region_num = xr + yr * 2      // 0–7 for outdoor; ≥8 hard-coded (indoor/dungeon)
```
```

With:
```markdown
### Region selection formula (`gen_mini()`, `fmain.c:3661–3690`)

```c
xs = (map_x + 151) >> 8       // sector column at viewport centre
ys = (map_y + 64) >> 8        // sector row at viewport centre
xr = (xs >> 6) & 1            // 0 = west column, 1 = east column
yr = (ys >> 5) & 3            // 0–3 = north → south band
region_num = xr + yr * 2      // 0–7 for outdoor; ≥8 hard-coded (indoor/dungeon)
```

Uses `map_x`/`map_y` (viewport top-left), not `hero_x`/`hero_y`. The offsets 151 and 64 centre the calculation within the 288 × 140 playfield.
```

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: fix region selection formula — uses map_x not hero_x

gen_mini() computes region from map_x + 151 and map_y + 64 (viewport
centre), not hero_x/hero_y.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 4: Expand Combat System section

**Files:**
- Modify: `RESEARCH.md` (Combat System section, after the `checkdead` subsection ~line 557)

- [ ] **Step 1: Add hero dodge mechanic and NPC fight clamping**

After the line that reads `- If hero (i == 0): `event(dtype)`, `luck -= 5`, `setmood(TRUE)` → death music` (end of checkdead subsection), and before `**Missile / Arrow combat**:`, insert:

```markdown

**Hero dodge mechanic**: when an NPC (`i > 0`) swings at the hero, the hit includes a dodge roll (`fmain.c:2707`):
```
if yd < bv: hit connects only if (i == 0) or (rand256() > brave)
```
- Hero (`i == 0`) always hits if in range — no dodge on outgoing attacks
- NPCs attacking hero: hit only connects if `rand256() > brave`
- This is the primary defensive scaling — higher `brave` = more dodges
- Effective dodge rate ≈ `brave / 256`. Julian start (brave=35): ~14%. Brave=100: ~39%. Brave=255: ~100%.

**NPC fight state clamping** (`fmain.c:1958–1959`): NPCs (`i > 2`) clamp fight animation states 6 and 7 to state 8, limiting their combat animation variety compared to the hero.

**Battle flag management** (`fmain.c:2497–2527`): `battleflag` is cleared each AI tick, then re-set if any living enemy is within 300 × 300 pixels of the hero and is visible (or was already flagged). Battle start triggers `setmood(1)` (battle music). Battle end calls `aftermath()` — counts dead/fleeing enemies and reports tallies; prints "Bravely done!" if hero vitality < 5.

**Distance calculation** — `calc_dist(a, b)` (`fmain2.c:446–463`): piecewise linear approximation of Euclidean distance used throughout combat and NPC proximity:
```c
x = abs(a.abs_x - b.abs_x);
y = abs(a.abs_y - b.abs_y);
if (x > y + y) return x;       // mostly horizontal
if (y > x + x) return y;       // mostly vertical
return (x + y) * 5 / 7;        // diagonal approximation
```
```

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: add hero dodge mechanic and battle flag to combat section

Documents the rand256() > brave dodge roll (the primary defensive
scaling), NPC fight state clamping, battle flag management, and the
calc_dist() piecewise distance approximation.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 5: Expand Death/Revival and Sleep sections

**Files:**
- Modify: `RESEARCH.md` (Player Character Stats section ~line 212, and Hunger & Fatigue section ~line 253)

- [ ] **Step 1: Expand revive() details in Player Character Stats**

After the line `- Raft spawns at `(13668, 14470)`, goodfairy setfig at `(13668, 15000)`` (line 217), and before the `**brave**` stat description, insert:

```markdown

**Fairy rescue animation** (`fmain.c:1557–1582`):
- When hero enters DEAD or FALL state, `goodfairy` counter begins decrementing each frame
- When `goodfairy < 120`: fairy sprite appears at `hero_x + goodfairy*2 - 20`, animated with cycling
- When `goodfairy == 1`: `revive(FALSE)` triggers — fairy rescue, hero returns to safe position
- FALL state with `goodfairy < 200`: automatic rescue from pit
- Fairy rescue condition: `luck >= 1` OR `goodfairy >= 200` (if luck < 1 and goodfairy < 200, a new brother spawns instead)

**New brother flow** (`revive(TRUE)`):
- Dead brother's position stored in `ob_listg[brother]` (`xc = hero_x, yc = hero_y, ob_stat = 1`)
- Ghost brother enabled: `ob_listg[brother + 2].ob_stat = 3` (appears as setfig)
- Stats loaded from `blist[brother]` (see table above)
- All inventory cleared: `stuff[0..GOLDBASE-1] = 0`
- Starting dirk given: `stuff[0] = weapon = 1`
- Timers reset: `secret_timer = light_timer = freeze_timer = 0`
- If `brother > 3` after increment → `quitflag = TRUE` (game over, 500-tick delay)

**Common to both revival paths**: hero placed at `safe_x, safe_y`; `vitality = 15 + brave/4`; `daynight = 8000`; `lightlevel = 300`; `hunger = fatigue = 0`; `anix = 3` (clear all enemies).
```

- [ ] **Step 2: Add sleep mechanics subsection**

After the `eat(amt)` paragraph and before the `**Vitality recovery**` paragraph in the Hunger & Fatigue section, insert a new subsection:

```markdown

### Sleep System

**Bed detection** (`fmain.c:2162–2188`): only in `region_num == 8` (inside buildings). Bed tiles are sector IDs 52, 53, 161, 162. Hero must stand still on a bed tile for 30 frames (`sleepwait` counter). If `fatigue < 50`: event(25) "Not tired enough". Otherwise: event(26), Y-axis snaps (`hero_y |= 0x1f`), `state = SLEEP`.

**Sleep processing** (`fmain.c:2357–2368`): while in SLEEP state, `daynight += 63` per frame (63× time acceleration — ~1,890 daynight ticks/second at 30 fps, or ~6.3 in-game hours per real second). `fatigue` decrements by 1 per frame.

**Wake conditions** (any of):
1. `fatigue == 0` (fully rested)
2. `fatigue < 30` AND `daynight` in 9000–10000 (dawn wake — hero wakes at sunrise if reasonably rested)
3. `battleflag` AND `rand64() == 0` (1/64 chance per frame — interrupted by combat)

On wake: `state = STILL`, Y-axis unsnaps (`hero_y &= 0xffe0`).

**Forced sleep**:
- `fatigue > 170` and `vitality ≤ 5`: event(12), `state = SLEEP` (exhaustion collapse)
- `hunger > 140` (every 8 ticks): event(24), `hunger = 130`, `state = SLEEP` (starvation collapse)
```

- [ ] **Step 3: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: expand death/revival details and add sleep system section

Documents fairy rescue animation, new brother flow, game over
condition, sleep bed detection, time acceleration (daynight += 63),
wake conditions including dawn wake, and forced sleep triggers.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 6: Expand Riding System details

**Files:**
- Modify: `RESEARCH.md` (a new subsection within or after existing content — the riding system is currently scattered; add a dedicated subsection after the Inventory System section at ~line 813)

- [ ] **Step 1: Add Riding System section**

Before the `## \`setmood()\`` line (line 815), insert:

```markdown
## Riding / Carrier System

Four carrier types: raft, turtle, bird (swan), and dragon. `riding` variable tracks current mount; `wcarry` tracks carrier slot; `active_carrier` tracks loaded carrier sprite.

### Raft

`anim_list[1]` is always the raft. Proximity detection (`fmain.c:1645–1651`):
```
xstart = hero_x - raft_x - 4;  ystart = hero_y - raft_y - 4;
if |xstart| < 16 && |ystart| < 16: raftprox = 1  (near — raft follows)
if |xstart| < 9  && |ystart| < 9:  raftprox = 2  (close enough to board)
```

- Boarding: `wcarry == 1` AND `raftprox == 2` AND hero on water terrain
- Speed: hero speed = 3 when `riding == 5`
- Raft follows hero when hero is in water (terrain types 3–5) and `raftprox == 1`

### Turtle

Carrier slot `active_carrier = 5`, sprite file 5. Loaded via `load_carrier(5)`.

- Movement: follows hero when `raftprox` and `wcarry == 3`; hero facing adopted; animation `dex = d + d + (cycle & 1)`; speed = 3
- Autonomous: when not near hero, turtle wanders toward water (terrain type 5), tries current direction then ±1, ±2 directions
- `riding = FALSE` when turtle not near hero
- Summoning via `get_turtle()` (`fmain.c:4397–4407`): places turtle at water tile (terrain type 5) near hero; up to 25 random attempts. Blocked if hero within (11194 < x < 21373, 10205 < y < 16208)

### Bird / Swan

Carrier slot `active_carrier = 11`, sprite file 11. Loaded via `load_carrier(11)`.

- Requires `stuff[5] > 0` (golden lasso) AND `raftprox` (proximity to carrier)
- `riding = 11`, triggers flying movement (acceleration-based)
- **Flying physics** (`fmain.c:1788–1812`):
  - Velocity accumulates: `vel_x += newx(20, dir, 2) - 20`, `vel_y += newy(20, dir, 2) - 20`
  - Speed caps: `|vel_x| < e - 8`, `|vel_y| < e` where `e = 40` for bird, `e = 42` otherwise
  - Position update: `abs_x += vel_x / 4`, `abs_y += vel_y / 4`
  - No terrain collision while airborne — bypasses `proxcheck()`
- **Dismount** (`fmain.c:1595–1607`): fight button when `|vel_x| < 15` AND `|vel_y| < 15` AND `proxcheck(hero_x, hero_y - 14)` passes (passable terrain above)

### Dragon

Separate type `DRAGON` (not `CARRIER`). Extent zone 2: coordinates (6749, 34951)–(7249, 35351). Loaded via `load_carrier(10)`.

- `vitality = 50`, `weapon = 5` (fireball)
- Fires randomly: 25% chance per tick (`rand4() == 0`)
- Always fires direction 5 (south)
- Fireball: `missile_type = 2`, `speed = 5`

### `load_carrier(n)` (`fmain.c:3421–3448`)

Places carrier in `anim_list[3]` at extent origin + (250, 200). `n=5` → turtle (extent 1), `n=10` → dragon (extent 2), `n=11` → bird (extent 0). Sets `anix = 4`, `active_carrier = n`.

---

```

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: add Riding / Carrier System section

Documents raft proximity tiers, turtle summoning/autonomous movement,
bird flying physics (acceleration, speed caps, dismount conditions),
dragon combat behavior, and load_carrier() mechanics.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 7: Add Movement System section

**Files:**
- Modify: `RESEARCH.md` (insert after Input Decoding section, before Menu System section)

- [ ] **Step 1: Insert Movement System section**

Before the `## Menu System` line (~line 1415), insert:

```markdown
## Movement System

Sources: `fmain.c:1614–1896`, `fsubs.asm:1274–1319`.

### Movement formula

`newx(x, dir, speed)` / `newy(y, dir, speed)` (`fsubs.asm:1281–1319`):
```
new_pos = old_pos + (dir_table[dir] * speed) / 2
```

Direction tables (`fsubs.asm:1277–1278`):
```
xdir: -2,  0,  2,  3,  2,  0, -2, -3,  0,  0   (dirs 0–9)
ydir: -2, -3, -2,  0,  2,  3,  2,  0,  0,  0
```

Maximum displacement per frame at speed 2: 3 pixels (east/west), 3 pixels (north/south), 2 pixels diagonal.

### Speed values

| Condition | Speed `e` | Effect |
|-----------|-----------|--------|
| Normal | 2 | Standard walking speed |
| Riding raft (`riding == 5`) | 3 | Faster water travel |
| Slow terrain (`environ == -1`, type 6) | 4 | Higher speed value but same formula |
| Sinking (`environ == 2` or `> 6`) | 1 | Half speed in water |
| Backwards terrain (`environ == -3`) | -2 | Hero walks backwards |

### Walk / still state transitions (`fmain.c:1624–1637`)

- `oldir < 9` AND input held (qualifier or keydir) → `state = WALKING` (12)
- `oldir == 9` (no directional input) → `state = STILL` (13)
- Animation index: `dex = diroffs[d] + ((cycle + i) & 7)` — 8-frame walk cycle
- Still index: `dex = diroffs[d] + 1`

### Wall sliding (direction deviation) (`fmain.c:1839–1852`)

When `proxcheck()` blocks the attempted move:
1. Try `(direction + 1) & 7` at same position
2. If still blocked, try `(direction - 2) & 7`
3. If both fail → increment `frustflag`, hero stays put

### Frustration animation (`fmain.c:1889–1896`)

| `frustflag` | Animation |
|-------------|-----------|
| > 40 | Unique frustrated pose (frame 40) |
| > 20 | Oscillating animation (frames 84–85, `(cycle >> 1) & 1`) |
| ≤ 20 | Normal still frame |

NPCs use `tactic = FRUST` instead of the frustflag counter.

### Coordinate wrapping (`fmain.c:2111–2127`)

For outdoor regions (0–7), hero position wraps at world boundaries:
```
if abs_x < 300:     abs_x = 32565
else if abs_x > 32565: abs_x = 300
else if abs_y < 300:     abs_y = 32565
else if abs_y > 32565: abs_y = 300
```

Conditions are **`else if` chained** — only one axis wraps per frame. The Y-axis high bit (`hero_y & 0x8000`) indicates indoor coordinates and is preserved during wrapping.

### Velocity tracking (`fmain.c:1881–1882`)

```
vel_x = (xtest - abs_x) * 4
vel_y = (ytest - abs_y) * 4
```

Used for smooth interpolation, pushback calculations, and bird dismount velocity checks.

---

```

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: add Movement System section

Documents movement formula, speed values, walk/still transitions,
wall sliding, frustration animation, coordinate wrapping, and velocity
tracking.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 8: Add Encounter Spawning System section

**Files:**
- Modify: `RESEARCH.md` (insert after Extents and Encounter Zones section, before Key Bindings: Design section)

- [ ] **Step 1: Insert Encounter Spawning section**

After the `---` that closes the Extents and Encounter Zones section (~line 1288), insert:

```markdown
## Encounter Spawning System

Sources: `fmain.c:2422–2472`, `fmain.c:3350–3392`.

### Random encounter trigger (`fmain.c:2451–2472`)

Every 32 ticks (`daynight & 31 == 0`), when **all** conditions met:
- No actors on screen (`actors_on_screen == 0`)
- No actors loading (`actors_loading == 0`)
- No active carrier
- Not in a forced encounter zone (`xtype < 50`)

**Danger level**:
- Indoor (`region_num > 7`): `5 + xtype`
- Outdoor: `2 + xtype`

**Spawn chance**: `rand64() <= danger_level`

**Type selection**: base `encounter_type = rand4()` with overrides:
- Swamp zone (`xtype == 7`): type 2 → 4 (snake replaces wraith)
- Spider zone (`xtype == 8`): type forced to 6 (spider)
- `xtype == 49`: type forced to 2 (wraith)

### Encounter placement (`fmain.c:2422–2449`)

Every 16 ticks (`daynight & 15 == 0`) when `encounter_number > 0`:
- `set_loc()` picks random direction and distance (150 + rand64 pixels from hero)
- Up to 10 attempts to find passable terrain via `proxcheck()`
- Places encounters in `anim_list` slots 3–6 (max 4 active enemies)
- Dead wraiths (race 2) are recycled immediately (slot reused)

### `set_encounter()` (`fmain.c:3350–3392`)

- DKnight (race 7): always spawns at fixed position (21635, 25762) instead of random
- Others: random within `spread/2` of encounter origin, up to 15 attempts for passable terrain
- **Mix flag** (`mixflag`): bit 1 (`& 2`) → race alternates within pair (even/odd encounter IDs); bit 2 (`& 4`) → weapon varies
- Goal assignment: `ARCHER1/2` if weapon has bit 2 set (bow/wand), otherwise `ATTACK1/2` based on `cleverness`

### Object distribution (`fmain2.c:1561–1583`)

On first visit to a region (`dstobs[region_num] == 0`), 10 random treasure objects are scattered:
```
for each of 10 objects:
    x = bitrand(0x3fff) + ((region_num & 1) * 0x4000)
    y = bitrand(0x1fff) + ((region_num & 6) * 0x1000)
    retry until px_to_im(x, y) == 0  (passable terrain)
    ob_id = rand_treasure[bitrand(15)]
```
Region is then marked distributed (`dstobs[region_num] = 1`).

---

```

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: add Encounter Spawning System section

Documents random encounter trigger timing, danger levels, spawn
chance formula, type overrides, placement logic, DKnight fixed
position, mix flag, and region object distribution.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 9: Add NPC Interaction Mechanics section

**Files:**
- Modify: `RESEARCH.md` (expand existing NPC Behavior section by adding subsections after it, before the Extents section)

- [ ] **Step 1: Insert NPC Interaction Mechanics after NPC Behavior section**

After the `---` that closes the NPC Behavior section (~line 1259), and before the `## Extents and Encounter Zones` heading, insert:

```markdown
## NPC Interaction Mechanics

Sources: `fmain.c:2473–2494`, `fmain.c:4167–4271`, `fmain2.c:1226–1292`, `fmain2.c:1969–2001`.

### Auto-speak on proximity (`fmain.c:2473–2494`)

When `nearest_person` changes race (new NPC enters range), automatic dialogue triggers:

| Race | NPC | Dialogue |
|------|-----|----------|
| 7 | DKnight | speak(41) |
| 9 | Necromancer | speak(43) |
| 0x84 | Princess | speak(16) if `ob_list8[9].ob_stat` set |
| 0x89 | Witch | speak(46) |
| 0x8d | Beggar | speak(23) |

### Talk range

- **Yell** (hit==5): `nearest_fig(1, 100)` — 100-pixel range
- **Say / Ask** (hit==6,7): `nearest_fig(1, 50)` — 50-pixel range

### Setfig dialogue details

| Type | NPC | Key behavior |
|------|-----|-------------|
| 0 | Wizard | `kind < 10` → rebuff; else random wisdom based on `goal` |
| 1 | Priest | Has writ (`stuff[28]`) → quest trigger; `kind < 10` → rebuff; else heal to full vitality |
| 2, 3 | Guards | Always speak(15) |
| 4 | Princess | Only speaks if `ob_list8[9].ob_stat` set |
| 5 | King | Only speaks if `ob_list8[9].ob_stat` set |
| 6 | Noble | speak(20) |
| 7 | Sorceress | First visit: speak(45), sets `ob_listg[9].ob_stat = 1`. Subsequent: if `luck < rand64()` (0–63) then `luck += 5` |
| 8 | Bartender | Response varies by fatigue (`fatigue < 5` check) and time of day |
| 9 | Witch | speak(46) |
| 10 | Spectre | speak(47) |
| 11 | Ghost | speak(49) |
| 12 | Ranger | Region-dependent dialogue |
| 13 | Beggar | speak(23) |

### Talking state

When a setfig has `can_talk == TRUE`: `state = TALKING`, `tactic = 15` (15-frame timer). Each frame `tactic--`; at 0, returns to STILL.

### Princess rescue (`fmain2.c:1969–2001`)

Triggered when entering extent type 83 with `ob_list8[9].ob_stat` set.

**Rewards**:
- `wealth += 100`
- `stuff[28] = 1` (Writ of Passage)
- `stuff[16..21] += 3` each (3 of every key colour)
- Hero teleported to throne room at (5511, 33780)
- Bird extent relocated to (22205, 21231)
- Noble NPC replaced with Princess (ob_id 4)
- `ob_list8[9].ob_stat = 0` (flag reset)
- `princess++` counter incremented

### Necromancer death transformation (`fmain.c:2006–2017`)

When Necromancer (race 9) dies: transforms to Woodcutter — `race = 10`, `vitality = 10`, `state = STILL`, `weapon = 0`. Then `leave_item(i, 139)` drops the Talisman.

### Witch visual attack (`fmain2.c:1226–1292`)

The Witch creates a rotating quadrilateral visual distortion using `witchpoints[]` (64-point circle table). A cross-product test determines if the hero is inside the attack cone. If inside AND `calc_dist(2, 0) < 100`: `dohit(-1, 0, facing, rand2() + 1)` — 1–2 damage. The attack direction oscillates based on the sign of the cross product.

---

```

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: add NPC Interaction Mechanics section

Documents auto-speak proximity triggers, talk range, setfig dialogue
table, sorceress luck bonus, princess rescue rewards, necromancer
death transformation, and witch visual attack mechanics.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 10: Update research_index.toml

**Files:**
- Modify: `research_index.toml`

- [ ] **Step 1: Add new section entries and bump last_updated**

Add these new entries at the end of `research_index.toml`:

```toml
[[entry]]
id = "player.movement"
title = "Movement System"
doc = "RESEARCH.md"
anchor = "#movement-system"
last_updated = "2026-03-29"
tags = ["player", "movement", "speed", "collision", "wrapping"]

[[entry]]
id = "player.riding"
title = "Riding / Carrier System"
doc = "RESEARCH.md"
anchor = "#riding--carrier-system"
last_updated = "2026-03-29"
tags = ["player", "riding", "raft", "turtle", "bird", "dragon", "carrier"]

[[entry]]
id = "npc.interaction"
title = "NPC Interaction Mechanics"
doc = "RESEARCH.md"
anchor = "#npc-interaction-mechanics"
last_updated = "2026-03-29"
tags = ["npc", "dialogue", "talk", "princess", "witch", "necromancer"]

[[entry]]
id = "world.encounter_spawning"
title = "Encounter Spawning System"
doc = "RESEARCH.md"
anchor = "#encounter-spawning-system"
last_updated = "2026-03-29"
tags = ["world", "encounters", "spawning", "danger", "objects"]

[[entry]]
id = "player.sleep"
title = "Sleep System"
doc = "RESEARCH.md"
anchor = "#sleep-system"
last_updated = "2026-03-29"
tags = ["player", "sleep", "fatigue", "bed", "rest"]
```

Also bump `last_updated` on these existing entries that were modified:
- `player.hunger_fatigue` → `last_updated = "2026-03-29"`
- `world.day_night` → `last_updated = "2026-03-29"`
- `combat.system` → `last_updated = "2026-03-29"`
- `player.stats` → `last_updated = "2026-03-29"`
- `world.region-diagrams` → `last_updated = "2026-03-29"`

Update the top-level `last_updated`:
```toml
last_updated = "2026-03-29"
```

- [ ] **Step 2: Commit**

```bash
git add research_index.toml
git commit -m "docs: update research_index.toml with new sections and timestamps

Adds entries for Movement System, Riding/Carrier System, NPC
Interaction Mechanics, Encounter Spawning, and Sleep System. Bumps
last_updated on all modified sections.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 11: Final verification

**Files:**
- Read: `RESEARCH.md`, `research_index.toml`

- [ ] **Step 1: Verify all anchor slugs match**

Run this to extract all `## ` and `### ` headings from RESEARCH.md and compare against anchors in research_index.toml:

```bash
grep -E '^#{2,3} ' RESEARCH.md | head -50
grep 'anchor' research_index.toml
```

Verify each `anchor` value in the toml matches a real heading in RESEARCH.md.

- [ ] **Step 2: Spot-check the 5 factual fixes**

Verify each fix is present:
1. Search for "map_x + 151" (Fix A — region formula)
2. Search for "red channel parameter" (Fix B — day fade)
3. Search for "hunger > 100 || fatigue > 160" (Fix C — OR condition)
4. Search for "Does not modify `lightlevel` itself" (Fix D — Green Jewel)
5. Search for "no passive decrement" (Fix E — fatigue)

```bash
grep -n "map_x + 151" RESEARCH.md
grep -n "red channel parameter" RESEARCH.md
grep -n "hunger > 100 || fatigue > 160" RESEARCH.md
grep -n "Does not modify" RESEARCH.md
grep -n "no passive decrement" RESEARCH.md
```

All 5 should return matches.

- [ ] **Step 3: Verify section ordering**

```bash
grep -n '^## ' RESEARCH.md
```

Confirm new sections appear in logical order:
- Movement System (after Input Decoding, before Menu System)
- Riding / Carrier System (after Inventory System, before setmood)
- NPC Interaction Mechanics (after NPC Behavior, before Extents)
- Encounter Spawning System (after Extents, before Key Bindings Design)
