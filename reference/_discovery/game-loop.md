# Discovery: Main Game Loop & Tick Structure

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace the complete main game loop in fmain.c, its tick ordering, phase structure, initialization, global control flags, actor processing, region loading, and frame timing.

## Initialization (fmain.c:1129-1268)

The `main()` function at fmain.c:1129 performs initialization in this order:

1. **Workbench startup** (fmain.c:1136-1139): If `argc == 0`, sets current directory from Workbench message.
2. **open_all()** (fmain.c:1142): Opens graphics/layers libraries, allocates bitmaps, disk I/O, loads font, allocates memory pools. Defined at fmain.c:728.
3. **Display setup** (fmain.c:1145-1156): Configures viewports, raster info, clears both page bitmaps, sets screen size to 156, sets initial colors.
4. **Title screen** (fmain.c:1160-1163): Displays `titletext` via `ssp()`, waits 50 ticks.
5. **Audio/font setup** (fmain.c:1165-1168): Sets text rasterport font/pens, calls `read_score()` to load music, `read_sample()` to load sound effects. Waits 50 ticks.
6. **Image memory setup** (fmain.c:1173-1175): Sets up 5 bitplane pointers into `image_mem` for `pagea`/`pageb`. Sets `bm_lim` to `sector_mem`.
7. **Music start** (fmain.c:1177): Plays initial score from `track[12..15]`.
8. **Intro sequence** (fmain.c:1183-1206): Loads and displays `page0` brush, animates screen open, shows story pages (`p1a`/`p1b`, `p2a`/`p2b`, `p3a`/`p3b`) via `copypage()`. Skippable via `skipint()`.
9. **Post-intro setup** (fmain.c:1207-1243): Loads shadow data from disk, loads `hiscreen` brush (status bar), displays copy protection placard (`copy_protect_junk()`), calls `revive(TRUE)` to initialize first brother.
10. **Viewport configuration** (fmain.c:1246-1260): Sets text viewport to `PAGE_HEIGHT` offset, page viewport to 140 height / 288 width, loads text colors, sets `viewstatus = 99`, initializes `cmode = 0`.
11. **Loop entry** (fmain.c:1269): Sets `cheat1 = quitflag = FALSE`, enters main `while (!quitflag)` loop.

## Main Loop Structure (fmain.c:1270-2621)

The main loop is `while (!quitflag)` at fmain.c:1270. Each iteration is one game tick. The phases execute in strict sequential order:

### Phase 1: Tick Counters (fmain.c:1274-1275)
```c
cycle++;
flasher++;
```
`cycle` is a local `short` that increments each tick (used for animation frame selection). `flasher` is a global `char` used for blink effects.

### Phase 2: Input Processing (fmain.c:1277-1365)
- `getkey()` (fmain.c:1277): Reads next key from circular input buffer (fsubs.asm:281). Returns 0 if buffer empty.
- **Pause check** (fmain.c:1279): `notpause = !(menus[GAME].enabled[5] & 1)` — checks if game is paused.
- **Key dispatch** (fmain.c:1280-1363): Processes key input:
  - Direction keys (20-29): Sets `keydir` for movement.
  - '0' key: Toggles `keyfight` for combat.
  - Cheat keys ('B', '.', 'R', '=', etc.): Only if `cheat1` is set.
  - Menu letter keys (0x61+): Processes menu option selection with `do_option()`.
  - Space/other keys: Looks up in `letter_list[]` for menu shortcuts.
- **viewstatus sub-loops** (fmain.c:1365-1374):
  - If `viewstatus == 2`: `Delay(200)` then set to 99.
  - If `viewstatus == 1` or `4`: Flashes color 31, calls `ppick()`, then **`continue`** — skips entire rest of tick. This is a wait/display loop.

### Phase 3: Mouse/Joystick Decode (fmain.c:1376)
```c
decode_mouse();
```
Reads mouse pointer position or joystick state, computes direction (`oldir`) and qualifier bits. Defined in fsubs.asm:1489.

### Phase 4: Pause Gate (fmain.c:1378)
```c
if (menus[GAME].enabled[5] & 1) { Delay(1); continue; }
```
If paused, delays 1 tick and restarts loop — **skips all subsequent phases**.

### Phase 5: Timer Decrements (fmain.c:1380-1382)
```c
if (light_timer) light_timer--;
if (secret_timer) secret_timer--;
if (freeze_timer) freeze_timer--;
```

### Phase 6: Fiery Death Zone Check (fmain.c:1384-1385)
```c
fiery_death = (map_x>8802 && map_x<13562 && map_y>24744 && map_y<29544);
```
Sets flag for the volcanic/fire region.

### Phase 7: Player State Resolution (fmain.c:1387-1459)
Reads `anim_list[0].state` and determines the player's next state based on:
- **DEAD/FALL** (fmain.c:1389-1403): Good fairy resurrection sequence. If `goodfairy == 1`, calls `revive(FALSE)`. Otherwise decrements `goodfairy`, shows fairy sprite animation.
- **DYING/SINK/SLEEP** (fmain.c:1408): No-op (states run their course).
- **Fire button pressed** (fmain.c:1409): Checks `handler_data.qualifier & 0x2000` (left button), `keyfight`, or CIA port read `(*pia & 128) == 0`. Handles:
  - Shooting (xtype > 80): Sets SHOOT1 state.
  - Swan dismount (riding==11): Checks proximity, sets `riding = 0`.
  - Melee: Sets FIGHTING state based on weapon type.
- **No fire button** (fmain.c:1440-1458): SHOOT1 → SHOOT3 (release). Otherwise, if direction set (`oldir < 9`), sets WALKING; with hunger > 120 causes random direction deviation (fmain.c:1443-1446).
- Writes final state back: `anim_list[0].state = inum` (fmain.c:1459).

### Phase 8: Carrier Proximity Check (fmain.c:1462-1472)
Computes `raftprox` and `turtleprox` based on distance between player (`anim_list[0]`) and carrier (`anim_list[wcarry]`). `wcarry = 3` if `active_carrier`, else `1`.
- `raftprox = 1` if within 16 pixels.
- `raftprox = 2` if within 9 pixels.

### Phase 9: Actor Processing Loop (fmain.c:1476-1826)
```c
for (i=0; i<anix; i++)
```
Iterates over all active actors (player + NPCs + carriers). `anix` is the allocation index — typically 3 (player + raft + setfig) up to 7 (with enemies). For each actor:

1. **Freeze check** (fmain.c:1481): If `freeze_timer && i > 0`, skip to `statc` (only player moves during timestop).
2. **Type dispatch** (fmain.c:1485-1498):
   - `OBJECTS`: Jump to `raise`.
   - `DRAGON` (fmain.c:1485-1498): Random fireball shooting, fixed facing.
   - `CARRIER` (fmain.c:1499-1563): Swan (actor_file==11) or raft movement. Swan follows player when `raftprox && stuff[5]`. Raft pathfinds on water tiles.
   - `SETFIG` (fmain.c:1546-1561): Set-piece figures (NPCs). Witch (j==9) uses `set_course()` to track player.
   - `RAFT` (fmain.c:1562-1572): Follows player if very close (`raftprox == 2`) and on water.
3. **Movement states** (fmain.c:1573+):
   - `SINK` (fmain.c:1573-1576): Sinking animation.
   - `WALKING` (fmain.c:1577-1665): Computes new position via `newx()`/`newy()`, checks `proxcheck()` for collision, tries deviation directions. Player door check on collision with terrain 15. Updates `abs_x`/`abs_y`.
   - `STILL` (fmain.c:1666-1676): Standing still, checks terrain.
   - `SHOOT1/SHOOT3` (fmain.c:1677-1717): Shooting logic, missile creation, arrow consumption (`stuff[8]--`).
   - `FIGHTING` (fmain.c:1718-1735): Combat animation state transitions via `trans_list[s].newstate[rand4()]`.
   - `DYING/DEAD/FROZEN/OSCIL/SLEEP/FALL` (fmain.c:1718-1745): Death/special state animations.
4. **Death processing** (fmain.c:1747-1758): When dying tactic reaches 0, sets DEAD. Special drops: race 0x09 drops talisman (item 139), race 0x89 drops lasso (item 27).
5. **Terrain/environment update** (fmain.c:1761-1797): Assigns `environ` based on terrain type `j`:
   - 0 → k=0 (normal), 2 → k=2 (shallow), 3 → k=5, 4/5 → progressive sinking (k increments to 10/30). At k=30: drowning or teleport (hero_sector 181 → region 9).
   - 6 → k=-1 (speed boost), 7 → k=-2 (flying), 8 → k=-3 (walk backwards), 9 → FALL state.
6. **Sprite index assignment** (fmain.c:1798-1820): Picks animation frame `dex` → `an->index`.
7. **Screen coordinate calculation** (fmain.c:1821-1826): Computes `rel_x`/`rel_y` from `abs_x - map_x` with type-specific offsets.

### Phase 10: Post-Actor Updates (fmain.c:1829-1833)
```c
if (anim_list[0].state != STILL) sleepwait = 0;
hero_x = anim_list[0].abs_x;
hero_y = anim_list[0].abs_y;
```

### Phase 11: Bed/Sleep Check (fmain.c:1835-1849, inside only)
If `region_num == 8` (inside buildings), checks if player is on sleeping spot tiles (161, 52, 162, 53). Increments `sleepwait`; at 30, either says "You aren't tired" (fatigue < 50) or enters SLEEP state.

### Phase 12: Door Check (fmain.c:1853-1955)
Binary search through `doorlist[]` (sorted by x coordinate) if `region_num < 8` (outside). Linear search if inside (`region_num >= 8`). On match:
- Determines destination coordinates from `door.xc2`/`yc2`.
- Sets `new_region` (8 = inside, 9 = dungeon/cave).
- Calls `xfer()` to update hero position + map offsets.
- Calls `find_place(2)` and `fade_page()`.

### Phase 13: Map Generation & Scroll (fmain.c:1959-2010)
1. **Hero position update** (fmain.c:1959-1961).
2. **Set draw target** (fmain.c:1963-1964): `bm_draw = fp_drawing->ri_page->BitMap`.
3. **Sprite restore** (fmain.c:1966-1970): `OwnBlitter()`, restores background behind sprites from previous frame via `rest_blit()`. `DisownBlitter()`.
4. **Undo witch FX** (fmain.c:1973-1974): Reverses witch visual distortion if active.
5. **Compute scroll delta** (fmain.c:1976-1981): `dif_x = img_x - fp_drawing->isv_x`, `dif_y = img_y - fp_drawing->isv_y`.
6. **Region loading check** (fmain.c:1987): `if (MAP_FLUX) load_next()` — non-blocking check for async disk I/O completion.
7. **Full redraw or scroll** (fmain.c:1989-2005):
   - `viewstatus == 99/98/3`: Full map redraw via `gen_mini()` + `map_draw()`. Transitions: 99→98→0.
   - `dif_x/dif_y == ±1`: Incremental scroll via `scrollmap(direction)`.
   - `dif_x == 0, dif_y == 0`: No scroll — this is where the **"no motion" sub-block** executes (the largest branch in the loop).
   - Large deltas: Fall through to full `map_draw()`.

### Phase 14: No-Motion Sub-Block (fmain.c:2009-2259)
This block executes when the map didn't scroll (player stationary or just animated). It contains the bulk of periodic game logic:

#### 14a: Print Queue (fmain.c:2009)
`ppick()` — processes one entry from the print queue (fmain2.c:442).

#### 14b: Sleep Advancement (fmain.c:2012-2021)
If player is SLEEP: `daynight += 63` (fast time advance), `fatigue--`. Wakes up when fatigue reaches 0, or at dawn (daynight 9000-10000 and fatigue < 30), or if battle starts.

#### 14c: Day/Night Cycle (fmain.c:2022-2037)
```c
if (!freeze_timer) if ((daynight++) >= 24000) daynight = 0;
lightlevel = daynight/40;
```
- `daynight` wraps at 24000 (one full day cycle).
- `lightlevel` = daynight/40, capped at 300 (symmetric: if ≥300, becomes 600-lightlevel).
- Night threshold: `lightlevel < 40` → `ob_listg[5].ob_stat = 3`.
- **Time-of-day events** (fmain.c:2029-2035): `dayperiod = daynight/2000`. Period changes trigger: 0→event(28) "midnight", 4→event(29) "it is now morning", 6→event(30) "afternoon", 9→event(31) "evening".

#### 14d: Day Fade (fmain.c:2039)
`day_fade()` (fmain2.c:1653) — adjusts palette based on `lightlevel` and `light_timer` (torch). No night effect inside buildings (region_num < 8).

#### 14e: Vitality Regeneration (fmain.c:2041-2046)
Every 1024 ticks (`daynight & 0x3ff == 0`): If vitality < (15 + brave/4) and not dead, vitality++. Updates display via `prq(4)`.

#### 14f: Freeze Gate (fmain.c:2048)
If `freeze_timer`, skips all remaining sub-block logic (jumps to `stasis` label at fmain.c:2259).

#### 14g: Find Place (fmain.c:2050)
`find_place(2)` (fmain.c:2647) — determines `hero_sector`, `hero_place`, `xtype` (extent type) from hero coordinates. Triggers extent-based encounters for special zones (xtype ≥ 50).

#### 14h: Actor Loading Check (fmain.c:2052-2056)
If `actors_loading == TRUE` and disk I/O channel 8 complete: calls `prep(ENEMY)`, sets `actors_loading = FALSE`, `anix = 3`.

#### 14i: Encounter Spawning (fmain.c:2057-2077)
Every 16 ticks (`daynight & 15 == 0`) if `encounter_number` and not loading: tries up to 10 random positions via `set_loc()`, places encounters with `set_encounter()` until `encounter_number` exhausted or `anix` reaches 7.

#### 14j: Random Encounter Generation (fmain.c:2078-2095)
Every 32 ticks (`daynight & 31 == 0`) if no actors on screen, not loading, no carrier, xtype < 50:
- `danger_level = 2 + xtype` (outdoor) or `5 + xtype` (indoor).
- `rand64() <= danger_level` → triggers `load_actors()`.
- `encounter_type` based on `rand4()` with xtype-specific overrides.

#### 14k: NPC Proximity Speech (fmain.c:2096-2106)
Checks `nearest_person` race against known NPCs. Triggers `speak()` for beggar (23), witch (46), princess (16), necromancer (43), dark knight (41).

#### 14l: AI Processing Loop (fmain.c:2107-2211)
```c
for (i=2; i<anix; i++)
```
For each non-player, non-raft actor:
- Skips CARRIER (just periodic `set_course`) and SETFIG types.
- Computes distance to player (xd, yd).
- Sets `actors_on_screen` if within 300 pixels and alive.
- **Goal/tactics evaluation**: FLEE if low vitality, follows leader if player dead, otherwise PURSUE/SHOOT/EVADE/BACKUP based on combat mode, weapon type, distance thresholds.
- Calls `do_tactic(i, tactic)` (fmain2.c:1663) to set facing and state.

#### 14m: Battle State Transitions (fmain.c:2212-2215)
- Battle start: `if (!battle2 && battleflag) setmood(1)` — changes music.
- Battle end: `if (!battleflag && battle2)` → `prq(7)`, `prq(4)`, `aftermath()`.

#### 14n: Safe Zone Update (fmain.c:2216-2224)
Every 128 ticks (`daynight & 127 == 0`) if no actors, no witch, safe terrain, not dead: records `safe_r`/`safe_x`/`safe_y` as respawn point. Also auto-eats food (`stuff[24]--`) if `hunger > 30`.

#### 14o: Mood Music (fmain.c:2225)
Every 8 ticks if not in battle: `setmood(0)` — updates ambient music.

#### 14p: Hunger/Fatigue (fmain.c:2226-2258)
Every 128 ticks if alive and not sleeping:
- `hunger++; fatigue++`.
- hunger == 35 → event(0) "getting hungry".
- hunger == 60 → event(1) "very hungry".
- Every 8 hunger ticks: if vitality > 5 and (hunger > 100 or fatigue > 160) → vitality -= 2. hunger > 90 → event(2).
- fatigue > 170 → event(12) forced sleep.
- hunger > 140 → event(24) forced sleep, hunger reset to 130.
- fatigue == 70 → event(3) "getting tired".
- hunger == 90 → event(4).

### Phase 15: Melee Combat Hit Detection (fmain.c:2262-2296)
For each actor NOT in WALKING state (and not frozen):
- Computes weapon reach position via `newx()`/`newy()` with weapon range (`wt`).
- For each other actor: checks distance. If within `bv` (bravery-scaled) pixels: `dohit(i, j, fc, wt)`.
- Player bravery value: `bv = (brave/20) + 5`, capped at 15.
- Enemy bravery: `bv = 2 + rand4()`.

### Phase 16: Missile Processing (fmain.c:2298-2340)
Iterates `missile_list[0..5]`:
- Checks terrain collision (impassable → destroy).
- Checks proximity to each actor. Hit detection: distance < 6 (arrows) or < 9 (fireballs).
- `dohit(-1, target, ...)` for arrows, `dohit(-2, target, ...)` for fireballs.
- Advances missile position: `newx()`/`newy()` by `speed*2`.
- `time_of_flight > 40` → missile expires.

### Phase 17: Object Processing (fmain.c:2342-2343)
```c
anix2 = anix;
do_objects();
```
`do_objects()` (fmain2.c:1184): Processes global objects (`ob_listg`) and region-specific objects (`ob_table[region_num]`). Sets up display entries, handles set-pieces. May increase `anix` to include visible objects.

### Phase 18: Missile Sprite Setup (fmain.c:2345-2362)
Adds active missiles to `anim_list` as OBJECTS type for rendering. Sets `anix2` up to 20 max.

### Phase 19: Sprite Sorting (fmain.c:2367-2393)
Bubble sort of `anim_index[]` by Y coordinate for correct draw ordering (painter's algorithm). Dead actors sort lower (y-32), sinking actors sort higher (y+32). Also finds `nearest_person` (closest actor within 50 units).

### Phase 20: Map Strip Repair (fmain.c:2396-2397)
After scroll blit completes:
```c
if (dif_x == 1) strip_draw(36); else if (dif_x == -1) strip_draw(0);
if (dif_y == 1) row_draw(10); else if (dif_y == -1) row_draw(0);
```
Draws new column/row tiles exposed by scrolling. `strip_draw()` at fsubs.asm:782, `row_draw()` at fsubs.asm:819.

### Phase 21: Witch Visual Effects (fmain.c:2399-2410)
Sets up witch distortion parameters for drawing page. If witch is active and close (`calc_dist(2,0) < 100`), deals damage to player.

### Phase 22: Sprite Rendering (fmain.c:2412-2609)
For each actor in sorted order:
- Skips offscreen or invisible actors.
- Multi-pass: renders character body, then weapon overlay (if armed).
- Clips to screen bounds (0-319 horizontal, 0-173 vertical).
- Handles environment effects (sinking, underwater sprites).
- `OwnBlitter()` → `save_blit()` → terrain masking → `mask_blit()` → `shape_blit()` → `DisownBlitter()`.
- Sets `an->visible = TRUE`.

### Phase 23: Page Flip (fmain.c:2611-2614)
```c
fp_drawing->ri_page->RxOffset = (map_x & 15);
fp_drawing->ri_page->RyOffset = (map_y & 31);
pagechange();
```
Sets scroll offsets, then swaps drawing/viewing pages.

### Phase 24: Fade Completion (fmain.c:2615)
```c
if (viewstatus == 3) { fade_normal(); viewstatus = 0; }
```

## Actor Processing Loop Details

The `anim_list[]` array has 20 entries (indices 0-19). Active actors are indexed 0 through `anix-1` (plus extended to `anix2` for objects/missiles during rendering).

**Fixed slots**:
- `anim_list[0]` — Player character (type PHIL)
- `anim_list[1]` — Raft (type RAFT, always present)
- `anim_list[2]` — Set-piece figure (type SETFIG, e.g., witch, NPCs)
- `anim_list[3..6]` — Enemies or carrier (type ENEMY/CARRIER)

`anix` default is 3 (player + raft + setfig). Increases to 4-7 when enemies spawn.

The actor loop (Phase 9) processes indices 0 through `anix-1`. The AI loop (Phase 14l) processes indices 2 through `anix-1`, skipping player and raft.

## Global Control Flags

| Flag | Type | Declared | Meaning |
|------|------|----------|---------|
| `quitflag` | char | fmain.c:590 | Loop termination flag. Set TRUE when all brothers die (fmain.c:2873). |
| `riding` | short | fmain.c:563 | 0=walking, 1=on raft, 5=on turtle, 11=on swan. |
| `battleflag` | char | fmain.c:588 | TRUE if hostile actors visible within 300 px. Set per tick in AI loop. |
| `actors_on_screen` | char | fmain.c:585 | TRUE if any living actor within 300 px of player. Reset each tick. |
| `witchflag` | char | fmain.c:591 | TRUE if witch (setfig j==9) is active. Set in `set_objects()`. |
| `viewstatus` | char | fmain.c:583 | Display state: 0=normal, 1=big map (blocks game), 2=delay+redraw, 3=fade-in pending, 4=object pickup wait, 98=redrawing, 99=corrupt/needs full redraw. |
| `freeze_timer` | short | fmain.c:577 | Counts down. While >0, time is frozen: enemies don't move, daynight doesn't advance, encounters don't spawn. |
| `light_timer` | short | fmain.c:577 | Counts down. While >0, `day_fade()` uses lightlevel 200 (torch effect). |
| `secret_timer` | short | fmain.c:577 | Counts down. Purpose: makes secret passages visible temporarily. |
| `goodfairy` | unsigned char | fmain.c:592 | Fairy resurrection counter. 1=immediate revive. Decrements each tick when player dead. At <120, shows fairy sprite. At <200, checks luck for revive. |
| `fiery_death` | (local) | fmain.c:1384 | Computed each tick. TRUE if in volcanic zone (map_x 8802-13562, map_y 24744-29544). |
| `cheat1` | short | fmain.c:562 | Cheat mode flag. Enables debug keys. |
| `daynight` | USHORT | fmain.c:572 | Day-night cycle counter 0-23999. Wraps at 24000. |
| `hunger` | short | fmain.c:565 | Hunger level. Increments every 128 daynight ticks. |
| `fatigue` | short | fmain.c:565 | Fatigue level. Increments every 128 daynight ticks. |
| `anix` | short | fmain.c:75 | Number of active actors (allocation index). Typically 3-7. |
| `region_num` | UWORD | fmain.c:614 | Current loaded region (0-7 outdoor, 8 inside, 9 dungeon/cave). |
| `new_region` | UWORD | fmain.c:614 | Target region for loading. ≥ NO_REGION (10) = stable/no load needed. |
| `safe_flag` | char | fmain.c:587 | Terrain type at hero position (0 = safe ground). |
| `hero_sector` | short | fmain.c:569 | Current map sector of hero. |
| `xtype` | USHORT | fmain.c:575 | Extent type at hero position. Controls encounter types, special behaviors. |
| `active_carrier` | short | fmain.c:574 | Which carrier is active (0=none, 11=swan, other=turtle). |
| `cycle` | (local) | fmain.c:1271 | Local tick counter. Wraps as short overflow. Used for animation frame selection. |
| `flasher` | char | fmain.c:584 | Global tick counter for blink/flash effects. |

## Region Loading

### Trigger Conditions
1. **Cross-region boundary** (fmain.c:2978 in `gen_mini()`): When `lregion != region_num`, sets `new_region = lregion` and calls `load_all()`.
2. **Door transition** (fmain.c:1924-1932): Sets `new_region = 8` (inside) or `9` (dungeon) and calls `xfer()` + `find_place()`.
3. **Revive/respawn** (fmain.c:2903): Sets `new_region = safe_r` and calls `load_all()`.
4. **Drowning teleport** (fmain.c:1785): At environ 30 in hero_sector 181, sets `new_region = 9`.

### MAP_FLUX / MAP_STABLE (fmain.c:612-613)
```c
#define NO_REGION   10
#define MAP_STABLE  (new_region >= NO_REGION)
#define MAP_FLUX    (new_region < NO_REGION)
```
MAP_FLUX means a region transition is in progress. MAP_STABLE means no load needed.

### load_all() (fmain.c:3545)
```c
load_all() { while (MAP_FLUX) load_new_region(); }
```
Blocking loop — calls `load_new_region()` repeatedly until all data loaded.

### load_new_region() (fmain.c:3547-3617)
Loads region data from disk in this order:
1. **Sector data** (64 tracks → `sector_mem`) — if sector changed.
2. **Region map** (8 tracks → `map_mem`) — if region changed.
3. **Terrain blocks** (1 track each → `terra_mem` / `terra_mem+512`) — if changed.
4. **Image planes** (5 × 8 tracks each → `image_mem`) — loads one plane group and returns (incremental loading).
5. **Desert gate check** (fmain.c:3600): If `new_region == 4` and `stuff[STATBASE] < 5`, blocks desert map squares.
6. **Wait for all I/O** (fmain.c:3604-3612): Waits on all 7 disk I/O channels.
7. **Finalize**: `region_num = new_region`, `new_region = NO_REGION`.

### load_next() (fmain2.c:752)
Non-blocking incremental loader called during main loop (Phase 13):
```c
load_next() { if (!IsReadLastDiskIO() || CheckLastDiskIO()) load_new_region(); }
```
Only calls `load_new_region()` if previous I/O is not a read or has completed.

### gen_mini() (fmain.c:2959-2992)
Called during map generation phase. For outdoor regions (< 8):
- If MAP_FLUX and near region boundary, forces `load_all()`.
- Computes `lregion` from `map_x`/`map_y` sector coordinates.
- If `lregion != region_num`, triggers region transition.

## pagechange() (fmain.c:2993-3005)

Double-buffer page flip:
```c
pagechange()
{   temp = fp_drawing;
    fp_drawing = fp_viewing;
    fp_viewing = temp;
    vp_page.RasInfo = temp->ri_page;
    v.LOFCprList = temp->savecop;
    MakeVPort(&v, &vp_page);
    MrgCop(&v);
    LoadView(&v);
    temp->savecop = v.LOFCprList;
    WaitBOVP(&vp_text);
}
```
Swaps `fp_drawing`/`fp_viewing` pointers, rebuilds copper list for new view, loads it, then **waits for vertical blank** via `WaitBOVP(&vp_text)`. This is the frame rate limiter.

## Frame Timing

**The frame rate is locked to the vertical blank (VBLANK)**. `pagechange()` calls `WaitBOVP(&vp_text)` which blocks until the next vertical blanking period of the text viewport. On PAL Amiga this is ~50 Hz, on NTSC ~60 Hz.

There is **one iteration of the main while loop per frame** (one tick = one pagechange = one VBLANK). However:
- The `continue` statements at fmain.c:1374 (viewstatus 1/4 wait) and fmain.c:1378 (pause) skip the pagechange entirely and use `Delay(1)` instead — these still produce ~1 tick delay but no actual frame rendering.
- The "no motion" sub-block (Phase 14) only executes when `dif_x == 0 && dif_y == 0` — meaning the map didn't scroll this tick. When scrolling, Phases 14a-14p are skipped entirely, and only the scroll/strip draw occurs before rendering.

**Key implication**: AI processing, encounter spawning, hunger/fatigue, day/night advancement, and most game logic only happen on frames where the map does not scroll. During continuous scrolling, only actor movement/animation, combat hit detection, and rendering occur.

Additional `Delay()` calls within the loop:
- `ppick()` (fmain2.c:443): `Delay(1)` if print queue empty — minor frame pacing when idle.
- Pause: `Delay(1)` per tick while paused (fmain.c:1378).
- viewstatus 2: `Delay(200)` — 4-second display pause (fmain.c:1366).

## Sub-Loops Within Main Loop

1. **viewstatus 1/4 wait loop** (fmain.c:1367-1374): Uses `continue` to re-enter main loop without processing. Flashes screen color, calls `ppick()`. Player must dismiss (key press → viewstatus 99).
2. **Door search** (fmain.c:1901-1955): Binary search (outdoor) or linear search (indoor) through `doorlist[]`.
3. **Encounter placement** (fmain.c:2058-2077): Tries up to 10 random positions to place encounters.
4. **AI loop** (fmain.c:2107-2211): Nested within no-motion block, iterates active enemies.
5. **Combat hit detection** (fmain.c:2262-2296): Nested loop checking each actor pair.
6. **Missile loop** (fmain.c:2298-2340): Processes up to 6 missiles.
7. **Sprite rendering** (fmain.c:2412-2609): Complex multi-pass render loop with blitter operations.
8. **Sprite sort** (fmain.c:2367-2393): Bubble sort (noted as "YUCKY AWFUL WASTEFUL BUBBLE SORT!! YUCK!!" by Talin).

## The `inside` Flag (region_num == 8)

There is no separate `inside` variable. The inside/outside distinction is controlled by `region_num`:
- `region_num < 8`: Outdoor regions (0-7).
- `region_num == 8`: Inside buildings (file_index entry F9).
- `region_num == 9`: Dungeons and caves (file_index entry F10).

Behavioral changes when `region_num >= 8`:
- **Door search** (fmain.c:1897): Uses linear search (exits use destination coords) instead of binary search.
- **Bed/sleep check** (fmain.c:1835): Only active when `region_num == 8`.
- **Day fade** (fmain2.c:1658): No night effect inside — always `fade_page(100,100,100,...)`.
- **Danger level** (fmain.c:2082): `5 + xtype` instead of `2 + xtype`.
- **Gen mini** (fmain.c:2981): `lregion = region_num` (no cross-region detection).
- **Music mood** (fmain.c:2948): Uses dungeon/cave music track offset `(5*4)`.
- **Wrapping** disabled: Player coordinate wrapping (fmain.c:1828-1838) only applies to `region_num < 8`.

## Cross-Cutting Findings

- **Scroll-gated game logic**: The most significant architectural finding is that AI, encounters, hunger, day/night, and most game logic (Phase 14) only execute on non-scrolling frames. This means a player walking continuously has slower hunger progression and fewer encounter checks than a player standing still. This appears intentional — it naturally reduces computational load during the most expensive operation (scrolling + rendering).
- **`viewstatus` as flow control**: The `viewstatus` flag with its `continue` statements creates implicit sub-loops within the main loop. States 1 and 4 effectively pause the game while keeping the loop running for input processing only.
- **`cycle` is a local variable** (fmain.c:1271): Not a global — it's declared as `short cycle` inside the while loop's block scope but is never re-initialized, so it persists across iterations due to the while loop being one scope. It will eventually overflow as a signed short.
- **Witch damage in rendering phase**: The witch's damage (`dohit(-1,0,...)`) is applied during the witch FX setup (Phase 21, fmain.c:2408-2410), which is in the rendering preparation section rather than the combat section.
- **`goodfairy` resurrection in player state phase**: The fairy resurrection is checked at Phase 7, not in the AI loop, because it modifies the player's own state.

## Unresolved

- **`cycle` overflow behavior**: `cycle` is a local `short` that increments every tick and is never reset. After ~32767 ticks (~9-11 minutes of gameplay) it will overflow to negative. The code uses `cycle & N` bitmasks throughout, which work identically for negative values with two's complement, so this may be benign. Cannot confirm without runtime verification.
- **`WaitBOVP` exact timing**: `WaitBOVP(&vp_text)` waits for the text viewport's vertical blank. Since the text viewport is positioned below the page viewport, this may subtly differ from a standard VBLANK wait. The exact timing depends on Amiga hardware viewport positioning which is not determinable from source alone.
- **`ppick()` Delay(1) interaction with frame timing**: When `ppick()` is called and the print queue is empty, it calls `Delay(1)`, adding a minimum 1/50th second delay. This happens in two locations (viewstatus wait and no-motion block), potentially causing frame drops.

## Refinement Log
- 2026-04-06: Initial discovery pass — full structural trace of main() from fmain.c:1129-2621, all 24 phases documented with line citations.
