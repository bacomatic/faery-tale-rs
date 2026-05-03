# Game Mechanics Research — Game Systems

Main game loop, menu system, day/night cycle, and text/message display.

> **Citation format**: `file.c:LINE` or `file.c:START-END`. Speech references: `speak(N)`.
> Split from [RESEARCH.md](RESEARCH.md). See the hub document for the full section index.

---

## 17. Main Game Loop

### 17.1 Initialization

The `main()` function at `fmain.c:1129` performs initialization:

1. **Workbench startup** (`fmain.c:1136-1139`): Sets current directory from Workbench message if `argc == 0`.
2. **`open_all()`** (`fmain.c:1142`): Opens graphics/layers libraries, allocates bitmaps, loads font, sets up disk I/O. Defined at `fmain.c:728`.
3. **Display setup** (`fmain.c:1145-1156`): Configures viewports, clears page bitmaps, sets screen size to 156.
4. **Title screen** (`fmain.c:1160-1163`): Displays `titletext` via `ssp()`, waits 50 ticks.
5. **Audio/font setup** (`fmain.c:1165-1168`): Sets text font/pens, loads music (`read_score()`), loads sound effects (`read_sample()`).
6. **Image memory** (`fmain.c:1173-1175`): Sets up 5 bitplane pointers for `pagea`/`pageb`.
7. **Music start** (`fmain.c:1177`): Plays title score from `track[12..15]`.
8. **Intro sequence** (`fmain.c:1183-1206`): Loads `page0` brush, animates screen open, shows story pages (`p1a`/`p1b`, `p2a`/`p2b`, `p3a`/`p3b`). Skippable via `skipint()`.
9. **Post-intro** (`fmain.c:1207-1243`): Loads shadow data, loads `hiscreen` brush (status bar), displays copy protection placard, calls `revive(TRUE)` for first brother.
10. **Viewport config** (`fmain.c:1246-1260`): Text viewport at `PAGE_HEIGHT` offset, page viewport 140×288, loads text colors, sets `viewstatus = 99`, `cmode = 0`.
11. **Loop entry** (`fmain.c:1269`): `cheat1 = quitflag = FALSE`, enters main loop.

### 17.2 Tick Structure

The main loop at `fmain.c:1270` is `while (!quitflag)`. Each iteration is one game tick. The 24 phases execute in strict order:

1. **Tick Counters** (`fmain.c:1274-1275`): `cycle++; flasher++`. `cycle` is a local `short`; `flasher` is a global `char` for blink effects.
2. **Input Processing** (`fmain.c:1277-1365`): `getkey()` reads from circular buffer (`fsubs.asm:281`). Dispatches direction keys, combat toggle ('0'), cheat keys, menu shortcuts via `letter_list[]`, and mouse-click keycodes (≥ 0x61). Also handles `viewstatus` sub-loops: state 1/4 uses `continue` (skip rest of tick); state 2 delays 200 ticks.
3. **Mouse/Joystick Decode** (`fmain.c:1376`): `decode_mouse()` — reads pointer/joystick, computes `oldir` and qualifier bits (`fsubs.asm:1489`).
4. **Pause Gate** (`fmain.c:1378`): If paused (`menus[GAME].enabled[5] & 1`), `Delay(1)` and `continue` — skips all remaining phases.
5. **Timer Decrements** (`fmain.c:1380-1382`): Decrements `light_timer`, `secret_timer`, `freeze_timer` if nonzero.
6. **Fiery Death Zone** (`fmain.c:1384-1385`): Sets `fiery_death` flag if player is in volcanic region (map_x 8802–13562, map_y 24744–29544).
7. **Player State Resolution** (`fmain.c:1387-1459`): Reads `anim_list[0].state`, resolves fire-button combat/shoot, direction-based walking, death/fairy rescue, hunger-induced direction drift (`fmain.c:1443-1446`).
8. **Carrier Proximity** (`fmain.c:1462-1472`): Computes `raftprox` (1 = within 16 px, 2 = within 9 px) between player and carrier (`anim_list[wcarry]`).
9. **Actor Processing Loop** (`fmain.c:1476-1826`): `for (i=0; i<anix; i++)` — iterates all active actors. Per-actor: freeze check, type dispatch (DRAGON/CARRIER/SETFIG/RAFT), movement state machine (WALKING/STILL/SHOOT/FIGHTING/DYING/DEAD), death processing (race 0x09 drops talisman at `fmain.c:1754`), terrain/environment update, sprite index assignment, screen coordinate calculation.
10. **Post-Actor Updates** (`fmain.c:1829-1833`): Copies `hero_x`/`hero_y` from `anim_list[0]`. Resets `sleepwait` if moving.
11. **Bed/Sleep Check** (`fmain.c:1835-1849`): Only in `region_num == 8` (buildings). Checks sleep-spot tiles (161, 52, 162, 53). At `sleepwait == 30`: enters SLEEP if `fatigue >= 50`.
12. **Door Check** (`fmain.c:1853-1955`): Binary search through sorted `doorlist[]` (outdoor) or linear search (indoor). On match: `xfer()` to destination, `find_place(2)`, `fade_page()`.
13. **Map Generation & Scroll** (`fmain.c:1959-2010`): Sets draw target, restores sprite backgrounds via `rest_blit()`, undoes witch FX, computes scroll delta. If `MAP_FLUX`: `load_next()`. Full redraw for `viewstatus` 99/98/3; incremental `scrollmap()` for ±1 deltas.
14. **No-Motion Sub-Block** (`fmain.c:2009-2259`): Executes only when map didn't scroll (`dif_x == 0 && dif_y == 0`). Contains the bulk of periodic game logic — see [§17.3](#173-no-motion-sub-block).
15. **Melee Combat Hit Detection** (`fmain.c:2262-2296`): For each non-WALKING actor: computes weapon reach via `newx()`/`newy()`, checks distance to all other actors. Player bravery range: `bv = (brave/20) + 5`, capped at 15.
16. **Missile Processing** (`fmain.c:2298-2340`): Iterates `missile_list[0..5]`. Checks terrain/actor collision. Hit: `dohit(-1,...)` for arrows, `dohit(-2,...)` for fireballs. Expires at `time_of_flight > 40`.
17. **Object Processing** (`fmain.c:2342-2343`): `do_objects()` (`fmain2.c:1184`) — processes global and region-specific objects, sets up display entries.
18. **Missile Sprite Setup** (`fmain.c:2345-2362`): Adds active missiles to `anim_list` as OBJECTS type. Max `anix2 = 20`.
19. **Sprite Sorting** (`fmain.c:2367-2393`): Bubble sort of `anim_index[]` by Y coordinate for painter's-algorithm draw order. Dead actors sort at y−32, sinking actors at y+32. Also finds `nearest_person`.
20. **Map Strip Repair** (`fmain.c:2396-2397`): After scroll: `strip_draw()` for new columns (`fsubs.asm:782`), `row_draw()` for new rows (`fsubs.asm:819`).
21. **Witch Visual Effects** (`fmain.c:2399-2410`): Sets witch distortion parameters. If witch active and within 100 px, deals damage via `dohit(-1,0,...)`.
22. **Sprite Rendering** (`fmain.c:2412-2609`): Multi-pass render for each sorted actor: clips to screen, handles environment offsets, blits via `save_blit()` → `mask_blit()` → `shape_blit()`.
23. **Page Flip** (`fmain.c:2611-2614`): Sets scroll offsets, calls `pagechange()` — swaps drawing/viewing pages, rebuilds copper list, waits for vertical blank via `WaitBOVP(&vp_text)`.
24. **Fade Completion** (`fmain.c:2615`): If `viewstatus == 3`: `fade_normal()`, then `viewstatus = 0`.

### 17.3 No-Motion Sub-Block

Phase 14 only runs when the map did not scroll. It contains all periodic game logic, organized as sub-phases:

- **14a** Print Queue (`fmain.c:2009`): `ppick()` — processes one deferred display command.
- **14b** Sleep Advancement (`fmain.c:2012-2021`): `daynight += 63` (64× speed). Wakes on fatigue=0, dawn (daynight 9000–10000 with fatigue < 30), or battle.
- **14c** Day/Night Cycle (`fmain.c:2022-2037`): `daynight++` wraps at 24000. `lightlevel = daynight/40`, mirrored at 300. Spectre visibility at lightlevel < 40. Time-of-day events at period changes (see [§19](#19-daynight-cycle)).
- **14d** Day Fade (`fmain.c:2039`): `day_fade()` adjusts palette (see [§19.2](#192-day_fade--palette-interpolation)).
- **14e** Vitality Regen (`fmain.c:2041-2046`): Every 1024 ticks: vitality++ if below `15+brave/4`.
- **14f** Freeze Gate (`fmain.c:2048`): If `freeze_timer`, skip to `stasis`.
- **14g** Find Place (`fmain.c:2050`): `find_place(2)` — determines `hero_sector`, `hero_place`, `xtype`.
- **14h** Actor Loading (`fmain.c:2052-2056`): Checks async disk I/O for enemy sprite data.
- **14i** Encounter Placement (`fmain.c:2057-2077`): Every 16 ticks: places queued encounters via `set_loc()`/`set_encounter()`, up to 10 attempts.
- **14j** Random Encounters (`fmain.c:2078-2095`): Every 32 ticks if no actors on screen: `danger_level = 2 + xtype` (outdoor) or `5 + xtype` (indoor). `rand64() <= danger_level` → `load_actors()`.
- **14k** NPC Proximity Speech (`fmain.c:2096-2106`): Beggar `speak(23)`, witch `speak(46)`, princess `speak(16)`, necromancer `speak(43)`, dark knight `speak(41)`.
- **14l** AI Loop (`fmain.c:2107-2211`): `for (i=2; i<anix; i++)` — evaluates goal/tactics: FLEE, PURSUE, SHOOT, EVADE, BACKUP. Calls `do_tactic(i, tactic)`.
- **14m** Battle Transitions (`fmain.c:2212-2215`): Start: `setmood(1)`. End: `aftermath()`.
- **14n** Safe Zone (`fmain.c:2216-2224`): Every 128 ticks: records `safe_r/safe_x/safe_y` respawn point. Auto-eats food if `hunger > 30`.
- **14o** Mood Music (`fmain.c:2225`): Every 8 ticks (non-battle): `setmood(0)`.
- **14p** Hunger/Fatigue (`fmain.c:2226-2258`): Every 128 ticks: `hunger++; fatigue++`. Threshold events and forced sleep.

### 17.4 Scroll-Gated Logic

The most significant architectural characteristic: AI processing, encounter spawning, hunger/fatigue, day/night advancement, and most game logic (Phase 14) only execute on frames where the map does not scroll. During continuous scrolling, only actor movement/animation, combat, and rendering occur. This naturally reduces computational load during scrolling but means a walking player has slower hunger progression and fewer encounter checks than a stationary one.

### 17.5 Global Control Flags

| Flag | Type | Declared | Purpose |
|------|------|----------|---------|
| `quitflag` | char | `fmain.c:590` | Loop termination. TRUE on win, game-over, or exit. |
| `viewstatus` | char | `fmain.c:583` | Display state: 0=normal, 1/4=picking, 2=map-message, 3=fade-in, 98/99=rebuild. |
| `battleflag` | char | `fmain.c:588` | TRUE if hostile actors within 300 px. Set per tick in AI loop. |
| `freeze_timer` | short | `fmain.c:577` | Countdown. While >0: enemies frozen, daynight frozen, encounters suppressed. |
| `light_timer` | short | `fmain.c:577` | Countdown. While >0: Green Jewel light-magic effect in `day_fade()`. |
| `secret_timer` | short | `fmain.c:577` | Countdown. While >0: secret passages visible. |
| `riding` | short | `fmain.c:563` | 0=walking, 1=raft, 5=turtle, 11=swan. |
| `anix` | short | `fmain.c:75` | Active actor count (typically 3–7). |
| `fiery_death` | local | `fmain.c:1384` | TRUE if in volcanic zone. Computed each tick. |

### 17.6 Region Loading

Region transitions are triggered by crossing outdoor boundaries (`gen_mini()` at `fmain.c:2959-2992`), door transitions (`fmain.c:1924-1932`), or respawn (`fmain.c:2903`).

- `MAP_FLUX` / `MAP_STABLE`: `new_region < NO_REGION(10)` means transition in progress — `fmain.c:612-613`.
- `load_all()` (`fmain.c:3545`): Blocking loop — `while (MAP_FLUX) load_new_region()`.
- `load_next()` (`fmain2.c:752`): Non-blocking incremental loader called during Phase 13.
- `load_new_region()` (`fmain.c:3547-3617`): Loads sector data, region map, terrain blocks, and 5 image planes incrementally. Desert gate check at `fmain.c:3600`: if `new_region == 4` and `stuff[STATBASE] < 5`, blocks desert map squares.

### 17.7 Frame Timing

The frame rate is locked to vertical blank. `pagechange()` at `fmain.c:2993-3005` calls `WaitBOVP(&vp_text)`, blocking until the next VBLANK period. On PAL Amiga this is ~50 Hz, NTSC ~60 Hz. One main loop iteration = one frame = one VBLANK.

Exceptions: `continue` statements at `fmain.c:1374` (viewstatus 1/4) and `fmain.c:1378` (pause) skip page-flip and use `Delay(1)` instead.

---


## 18. Menu System

### 18.1 `menus[10]` Structure

Defined at `fmain.c:517-531`:

```c
struct menu {
    char    *label_list;
    char    num, color;
    char    enabled[12];
} menus[10];
```

Each `enabled[i]` byte encodes state and behavior — `fmain.c:512-513`:
- **Bit 0** (`& 1`): highlight/selected toggle
- **Bit 1** (`& 2`): visibility — entry displayed only if set
- **Bits 2–7** (`& 0xfc`): action type (`atype`)

| atype | Behavior |
|-------|----------|
| 0 | Top-bar navigation: switch `cmode` to `hit` |
| 4 | Toggle: XOR bit 0, call `do_option(hit)` |
| 8 | Immediate action: highlight, `do_option(hit)` |
| 12 | One-shot highlight: set bit 0, `do_option(hit)` |

Common encoded values: 2 = visible nav item, 3 = visible + highlighted, 6 = visible toggle (off), 7 = visible toggle (on), 8 = hidden action, 10 = visible action.

Ten menu modes — `fmain.c:494`:

```c
enum cmodes {ITEMS=0, MAGIC, TALK, BUY, GAME, SAVEX, KEYS, GIVE, USE, FILE};
```

| Mode | Label List | Entries | Color | Purpose |
|------|-----------|---------|-------|---------|
| ITEMS (0) | `label2` | 10 | 6 | List/Take/Look/Use/Give |
| MAGIC (1) | `label6` | 12 | 5 | Stone/Jewel/Vial/Orb/Totem/Ring/Skull |
| TALK (2) | `label3` | 8 | 9 | Yell/Say/Ask |
| BUY (3) | `label5` | 12 | 10 | Food/Arrow/Vial/Mace/Sword/Bow/Totem |
| GAME (4) | `label4` | 10 | 2 | Pause/Music/Sound/Quit/Load |
| SAVEX (5) | `label8` | 7 | 0 | Save/Exit |
| KEYS (6) | `label9` | 11 | 8 | Gold/Green/Blue/Red/Grey/White |
| GIVE (7) | `labelA` | 9 | 10 | Gold/Book/Writ/Bone |
| USE (8) | `label7` | 10 | 8 | Dirk/Mace/Sword/Bow/Wand/Lasso/Shell/Key/Sun |
| FILE (9) | `labelB` | 10 | 5 | Slots A–H |

Menus ITEMS through GAME (modes 0–4) share a top bar of 5 entries (indices 0–4) drawn from `label1` ("Items Magic Talk Buy  Game"). Entries 5+ come from each menu's `label_list`. USE and FILE skip the top bar — for `cmode >= USE`, labels draw directly from `menus[cmode].label_list` (`fmain.c:3088`).

### 18.2 Display: `print_options`, `propt`, `gomenu`

**`print_options()`** at `fmain.c:3048-3068` renders the right-side menu panel on `rp_text2` (hi-res status bitmap). Iterates `menus[cmode]` entries; for each where `(enabled[i] & 2) != 0` (visible), assigns `real_options[j] = i` and calls `propt(j, highlight)`. Layout: 2 columns (x=430/482), 6 rows spaced 9 px, starting at y=8.

**`propt(j, pena)`** at `fmain.c:3070-3090` draws a single label. Background pen varies:
- USE: pen 14. FILE: pen 13. Top bar (k<5): pen 4.
- KEYS: `keycolors[k-5]` where `keycolors = {8, 6, 4, 2, 14, 1}` — `fmain.c:519`.
- SAVEX: pen = entry index. Others: `menus[cmode].color`.

**`gomenu(mode)`** at `fmain.c:3521-3525`: Sets `cmode`, resets `handler_data.lastmenu`, calls `print_options()`. **Blocked if paused** — checks `menus[GAME].enabled[5] & 1`.

**`real_options[12]`** at `fmain.c:515`: Indirection array mapping visible screen positions to actual `enabled[]` indices. Built by `print_options()`. Mouse/keyboard handlers index through `real_options[inum]` — `fmain.c:1304-1306`.

### 18.3 `set_options` — Dynamic Availability

Defined at `fmain.c:3527-3543`. Called at the end of every `do_option()` (`fmain.c:3507`). Updates `enabled[]` arrays based on current inventory:

- **MAGIC** (indices 5–11): `menus[MAGIC].enabled[i+5] = stuff_flag(i+9)` — magic items `stuff[9..15]`.
- **USE** (indices 0–6): `menus[USE].enabled[i] = stuff_flag(i)` — weapons `stuff[0..6]`.
- **KEYS** (indices 5–10): `menus[KEYS].enabled[i+5] = stuff_flag(i+16)` — keys `stuff[16..21]`. Also `menus[USE].enabled[7] = 10` if any key owned, else 8.
- **USE Sun**: `menus[USE].enabled[8] = stuff_flag(7)` — Sun Stone `stuff[7]`.
- **GIVE Gold**: `enabled[5] = 10 if wealth > 2`, else 8.
- **GIVE Book**: Always 8 (hidden) — `fmain.c:3540`.
- **GIVE Writ/Bone**: `stuff_flag(28)` / `stuff_flag(29)`.

`stuff_flag(n)` at `fmain2.c:1639-1648` returns 8 if `stuff[n] == 0` (hidden), 10 if owned (visible).

### 18.4 `do_option` Dispatch

Defined at `fmain.c:3102-3507`. Dispatches on `cmode` and `hit` (menu entry index):

#### ITEMS (`fmain.c:3110-3297`)

| hit | Label | Action |
|-----|-------|--------|
| 5 | List | Renders full inventory screen. Iterates `stuff[0..GOLDBASE-1]`, draws item icons. Sets `viewstatus=4`. |
| 6 | Take | `nearest_fig(0,30)`: gold pieces (0x0d) → +50 wealth; food (148) → eat; brother's bones (28) → recover inventory; containers (chest/urn/sack) → random loot via `rand4()`; other → `itrans[]` lookup. Dead bodies → extract weapon+treasure. **Win check**: `stuff[22]` (Talisman) set → `quitflag = TRUE` — `fmain.c:3244`. |
| 7 | Look | Scans nearby OBJECTS within range 40. Found → `event(38)`, else `event(20)`. |
| 8 | Use | `gomenu(USE)` |
| 9 | Give | `gomenu(GIVE)` |

#### MAGIC (`fmain.c:3299-3367`)

Guard: `stuff[4+hit] == 0` → `event(21)` "if only I had some magic!". Blocked in necromancer extent: `speak(59)` — `fmain.c:3301-3302`.

| hit | Spell | Effect |
|-----|-------|--------|
| 5 | Stone | Teleport via standing stones; requires `hero_sector==144`, uses `stone_list[]` — `fmain.c:3327-3348` |
| 6 | Jewel | `light_timer += 760` — `fmain.c:3305` |
| 7 | Vial | `vitality += rand8()+4`, capped at `15+brave/4` — `fmain.c:3349-3352` |
| 8 | Orb | `secret_timer += 360` — `fmain.c:3306` |
| 9 | Totem | Map view: renders big map with hero marker, `viewstatus=1` — `fmain.c:3308-3323` |
| 10 | Ring | `freeze_timer += 100`. Blocked if `riding > 1`. — `fmain.c:3307` |
| 11 | Skull | Kills all visible enemies with `race < 7`. Decrements `brave`. — `fmain.c:3353-3360` |

After use: `--stuff[4+hit]`; if depleted, `set_options()` — `fmain.c:3363`.

#### TALK (`fmain.c:3368-3425`)

| hit | Label | Range |
|-----|-------|-------|
| 5 | Yell | `nearest_fig(1, 100)` |
| 6 | Say | `nearest_fig(1, 50)` |
| 7 | Ask | `nearest_fig(1, 50)` |

NPC responses switch on `an->race & 0x7f`: Wizard(0)→`speak(35/27+goal)`, Priest(1)→checks writ/heals, Guard(2,3)→`speak(15)`, Princess(4)→`speak(16)`, King(5)→`speak(17)`, Noble(6)→`speak(20)`, Sorceress(7)→luck boost or `speak(45)`, Innkeeper(8)→`speak(13/12/14)` based on fatigue/time, Witch(9)→`speak(46)`, Spectre(10)→`speak(47)`, Ghost(11)→`speak(49)`, Ranger(12)→`speak(22/53+goal)`, Beggar(13)→`speak(23)` — `fmain.c:3385-3415`.

#### BUY (`fmain.c:3426-3442`)

Requires shopkeeper (race `0x88`). Uses `jtrans[]` pairs (item_index, cost) — `fmain.c:3426`:

| Menu | Item | Cost |
|------|------|------|
| Food | eat(50) | 3 |
| Arrow | `stuff[8] += 10` | 10 |
| Vial | `stuff[11]++` | 15 |
| Mace | `stuff[1]++` | 30 |
| Sword | `stuff[2]++` | 45 |
| Bow | `stuff[3]++` | 75 |
| Totem | `stuff[13]++` | 20 |

#### GAME (`fmain.c:3443-3447`)

| hit | Label | Action |
|-----|-------|--------|
| 5 | Pause | Toggle via atype=4 bit flip; gates `notpause` at `fmain.c:1282` |
| 6 | Music | `setmood(TRUE)` — `fmain.c:3444` |
| 7 | Sound | Toggle bit only; checked elsewhere in sound playback |
| 8 | Quit | `gomenu(SAVEX)` |
| 9 | Load | `svflag = FALSE; gomenu(FILE)` |

#### SAVEX (`fmain.c:3467-3469`)

| hit | Action |
|-----|--------|
| 5 | Save: `svflag = TRUE; gomenu(FILE)` |
| 6 | Exit: `quitflag = TRUE` |

#### USE (`fmain.c:3448-3466`)

| hit | Action |
|-----|--------|
| 0–4 | Equip weapon: `anim_list[0].weapon = hit+1` (Dirk/Mace/Sword/Bow/Wand) |
| 6 | Shell/Turtle: calls `get_turtle()` |
| 7 | Key: `gomenu(KEYS)` if any key owned |
| 8 | Sunstone: if `witchflag`, `speak(60)` |

Returns to ITEMS: `gomenu(ITEMS)` — `fmain.c:3464`.

#### KEYS (`fmain.c:3473-3485`)

Converts `hit -= 5` to key index 0–5 (Gold/Green/Blue/Red/Grey/White). If `stuff[hit+KEYBASE] > 0`: scans 9 directions around hero via `doorfind(x, y, hit+1)`. If door found, decrements key. Returns to ITEMS.

#### GIVE (`fmain.c:3490-3506`)

Requires `nearest_person != 0`.

| hit | Action |
|-----|--------|
| 5 | Gold: wealth − 2, random `kind++`. Beggar → `speak(24+goal)`, else `speak(50)`. |
| 8 | Bone: if spectre (0x8a) → `speak(48)`, drops crystal shard (item 140). |

#### FILE (`fmain.c:3470-3472`)

Calls `savegame(hit)` — slot 0–7 (A–H). `svflag` determines save vs. load. Returns to GAME.

### 18.5 Keyboard Shortcuts

Defined via `letter_list[38]` at `fmain.c:537-547`:

| Key | Action | Key | Action |
|-----|--------|-----|--------|
| I | List inventory | Y | Yell |
| T | Take | S | Say |
| ? | Look | A | Ask |
| U | Use submenu | Space | Pause toggle |
| G | Give submenu | M | Music toggle |
| O | Buy food | F | Sound toggle |
| R | Buy arrows | Q | Quit |
| 8 | Buy vial | L | Load |
| C | Buy mace | V | Save |
| W | Buy sword | X | Exit |
| B | Buy bow | 1–7 | Equip/use items |
| E | Buy totem | K | Keys submenu |
| F1–F7 | Magic spells (keycodes 10–16) | | |

The keyboard handler at `fmain.c:1343-1360` loops through `letter_list`, matches the key, sets `cmode` and `hit`, checks `hitgo` and `atype`, then calls `do_option(hit)`.

**SAVEX guard** (`fmain.c:1350`): V and X shortcuts are blocked unless `cmode == SAVEX`, preventing accidental save/exit.

**KEYS special** (`fmain.c:1341-1343`): If `cmode == KEYS` and key is '1'–'6', calls `do_option(key - '1' + 5)` directly.

> **Normative logic:** [reference/logic/menu-system.md](logic/menu-system.md).

---


## 19. Day/Night Cycle

### 19.1 `daynight` Counter

Declared at `fmain.c:572` as `USHORT daynight`. Cycles from 0 to 23999, representing one full day. Incremented once per non-scrolling tick — `fmain.c:2023-2024`:

```c
if (!freeze_timer)
    if ((daynight++) >= 24000) daynight = 0;
```

Does NOT advance when `freeze_timer` is active (time-stop spell). Initialized to 8000 (morning) during `revive()` — `fmain.c:2905`. During sleep, advances 64× faster: `daynight += 63` — `fmain.c:2014`.

#### `lightlevel` — Brightness Curve

Computed at `fmain.c:2025-2026`:

```c
lightlevel = daynight / 40;
if (lightlevel >= 300) lightlevel = 600 - lightlevel;
```

This creates a **triangular wave** peaking at 300 (noon) and bottoming at 0 (midnight):

| daynight | lightlevel | Phase |
|----------|------------|-------|
| 0 | 0 | Midnight (darkest) |
| 6000 | 150 | Dawn |
| 12000 | 300 | Noon (brightest) |
| 18000 | 150 | Dusk |
| 23999 | 1 | Just before midnight |

#### `dayperiod` — Time-of-Day Events

Computed as `daynight / 2000` (values 0–11) — `fmain.c:2029-2036`. Transitions trigger text events:

| Period | daynight Range | Event | Message |
|--------|---------------|-------|---------|
| 0 | 0–1999 | `event(28)` | "It was midnight." — `narr.asm:45` |
| 4 | 8000–9999 | `event(29)` | "It was morning." — `narr.asm:46` |
| 6 | 12000–13999 | `event(30)` | "It was midday." — `narr.asm:47` |
| 9 | 18000–19999 | `event(31)` | "Evening was drawing near." — `narr.asm:48` |

Periods 1–3, 5, 7–8, 10–11 are silent transitions.

### 19.2 `day_fade` — Palette Interpolation

Defined at `fmain2.c:1653-1660`. Called every tick from Phase 14d (`fmain.c:2039`). The routine only runs its palette update every 4 ticks (`daynight & 3 == 0`) or while a screen rebuild is in progress (`viewstatus > 97`). Outdoors (`region_num < 8`) it calls `fade_page(lightlevel − 80 + ll, lightlevel − 61, lightlevel − 62, TRUE, pagecolors)`, where `ll = 200` when the Green Jewel `light_timer` is active and `ll = 0` otherwise. Indoors (`region_num ≥ 8`) it forces full brightness `(100, 100, 100)` with no day/night variation. Full pseudo-code: [logic/day-night.md § day_fade](logic/day-night.md#day_fade).

- **Green Jewel light bonus**: `light_timer > 0` adds 200 to red parameter (warm amber glow).
- **Update rate**: Every 4 ticks (`daynight & 3 == 0`) or during screen rebuild (`viewstatus > 97`).
- **Indoor override**: `region_num >= 8` → always full brightness `(100,100,100)` with no day/night variation.

### 19.3 `fade_page` — RGB Component Fading

Defined at `fmain2.c:377-419`. Per-component palette scaler:

**Color 31 override** (`fmain2.c:381-386`):

| Region | Color 31 | Meaning |
|--------|----------|---------|
| 4 (desert) | `0x0980` | Orange-brown desert sky |
| 9 (dungeon), `secret_timer` active | `0x00f0` | Bright green (secret revealed) |
| 9 (dungeon), normal | `0x0445` | Dark grey-blue |
| All others | `0x0bdf` | Light blue sky |

**Clamping** (with `limit=TRUE`) — `fmain2.c:389-400`:
- Red: min 10, max 100
- Green: min 25, max 100
- Blue: min 60, max 100
- Blue shift factor: `g2 = (100-g)/3`

**Per-color computation** (`fmain2.c:402-416`): For each of 32 palette entries, extracts 12-bit RGB components from `pagecolors[]`, then:
- Green Jewel light effect (`fmain2.c:407`): if `light_timer` active and red < green, boosts red to match green.
- Scales: `r1 = (r × r1) / 1600`, `g1 = (g × g1) / 1600`, `b1 = (b × b1 + g2 × g1) / 100`.
- Nighttime vegetation boost (`fmain2.c:412-413`): Colors 16–24 get extra blue at twilight (g 21–49: +2; g 50–74: +1).

Result written to `fader[]` and loaded via `LoadRGB4(&vp_page, fader, 32)`.

#### Outdoor RGB at Key Times

| Phase | lightlevel | r (no jewel effect) | g | b |
|-------|------------|-------------|---|---|
| Midnight | 0 | clamped 10 | clamped 25 | clamped 60 |
| Dawn | 150 | 70 | 89 | 88 |
| Noon | 300 | clamped 100 | clamped 100 | clamped 100 |

With the Green Jewel effect active: red gets +200, so midnight red = 120 (warm amber tone even in darkness).

### 19.4 `fade_down` / `fade_normal` — Screen Transitions

Defined at `fmain2.c:623-630`:

- **`fade_down()`**: Steps all channels from 100 to 0 in increments of 5 (21 steps, `Delay(1)` each). Fades screen to black.
- **`fade_normal()`**: Steps all channels from 0 to 100 in increments of 5. Fades back to full brightness.

Both use `limit=FALSE` — no night clamping or blue shift. Used for map messages, door transitions, and other screen changes.

### 19.5 `setmood` — Music Selection

Defined at `fmain.c:2936-2957`. Selects one of 7 four-channel music tracks based on game state:

| Track Offset | Indices | Condition | Music |
|-------------|---------|-----------|-------|
| 0 | track[0–3] | `lightlevel > 120` (outdoor day) | Day theme |
| 4 | track[4–7] | `battleflag` | Battle theme |
| 8 | track[8–11] | `lightlevel ≤ 120` (outdoor night) | Night theme |
| 12 | track[12–15] | (intro/title) | Title theme |
| 16 | track[16–19] | Astral plane coordinates | Astral theme |
| 20 | track[20–23] | `region_num > 7` (indoor) | Indoor theme |
| 24 | track[24–27] | `vitality == 0` (death) | Death theme |

Priority (highest first): Dead → Astral plane → Battle → Indoor → Day/Night outdoor.

Day/night music crossover: `lightlevel > 120` = day, `≤ 120` = night. Crossover at daynight ≈ 4800 (dawn) and ≈ 19200 (dusk).

Playback at `fmain.c:2951-2956`: if music enabled (`menus[GAME].enabled[6] & 1`), `now=1` → `playscore()` (immediate), `now=0` → `setscore()` (crossfade). Mood re-evaluated every 8 ticks via `fmain.c:2198`.

Indoor waveform tweak (`fmain.c:2945-2946`): dungeons (region 9) use `new_wave[10] = 0x0307`; buildings (region 8) use `0x0100`.

### 19.6 Gameplay Effects

**Spectre visibility** (`fmain.c:2027-2028`): `lightlevel < 40` (deep night, daynight < 1600 or > 22400) → `ob_listg[5].ob_stat = 3` (visible/interactive). Otherwise stat = 2 (hidden).

**Sleep** (`fmain.c:2014-2021`): Time passes 64× faster. Wake conditions: fatigue = 0, OR fatigue < 30 and daynight 9000–10000 (morning), OR battle interruption (`battleflag && rand64() == 0`). Inn guests wake at morning.

**Encounter spawning** (`fmain.c:2058-2091`): Checked every 16/32 ticks. Rate is constant regardless of day/night — `danger_level` depends on `region_num` and `xtype`, not `lightlevel`.

**Innkeeper dialogue** (`fmain.c:3407`): `dayperiod > 7` (evening/night) triggers lodging speech.

**Vitality recovery** (`fmain.c:2041-2045`): Every 1024 ticks, regenerates 1 HP if below max. Tied to `daynight` counter but not time-of-day dependent.

### 19.7 Palette Data

**`pagecolors[32]`** at `fmain2.c:367-372`: Hardcoded 32-color base palette in 12-bit Amiga RGB. Same for all outdoor regions (0–7). Faded dynamically by `fade_page()` each tick.

**`textcolors[20]`** at `fmain.c:476-479`: Status bar palette (hi-res viewport). NOT affected by day/night fading.

**`blackcolors[32]`** at `fmain.c:481-482`: All-zero palette for instant blackout transitions.

**`sun_colors[53]`** at `fmain2.c:1569-1578`: Sunrise/sunset gradient for the victory sequence `win_colors()` (see [§16.5](RESEARCH-npcs-quests.md#165-win-check-and-victory-sequence)).

**`introcolors[32]`** at `fmain.c:484-488`: Title/intro screen palette, separate from gameplay.

**`colorplay()`** at `fmain2.c:425-431`: Teleportation effect — 32 frames of random 12-bit colors for all palette entries (except color 0). Used at `fmain.c:3336`.

---


## 20. Text & Message Display

### 20.1 Display Architecture

The display operates in three configurations (see [ARCHITECTURE.md §3.6](ARCHITECTURE.md#36-screen-configurations)).

**Normal gameplay — split-screen with status bar:**

- **`vp_page`** — lo-res (288×140) playfield for the game world — `fmain.c:14` (`PAGE_HEIGHT 143`), `fmain.c:1250-1255`.
- **`vp_text`** — hi-res (640×57) status bar at bottom — `fmain.c:16` (`TEXT_HEIGHT 57`), `fmain.c:815-818`.

**Cinematic scenes — near-full-screen playfield** (title text, asset loading, victory):

- **`vp_page`** — lo-res (312×194), set by `screen_size(156)` — `fmain.c:2914-2933`. Slightly inset (4px horizontal, 3px vertical) from the full 320×200 frame.
- **`vp_text`** — hidden (`DHeight ≤ 0`). No status bar visible.

**Storybook pages — true full-screen** (intro page0 and p1–p3):

- **`vp_page`** — lo-res (320×200), reached by `screen_size(160)` at the peak of the intro zoom-in loop (`fmain.c:1199`). No border, no inset — edge-to-edge display.
- **`vp_text`** — hidden (`DHeight ≤ 0`).
- The zoom-in animates from 0 to 160 in steps of 4, opening an iris onto the storybook art. The zoom-out starts from 156 (not 160), snapping the viewport to 312×194 on the first frame before animating closed (`fmain.c:1209`). Both loops use `introcolors` palette fading (`fmain.c:2914-2933`).

Key RastPort assignments — `fmain.c:448`:
- `rp_map` — for drawing on playfield pages (used during `map_message()` story screens).
- `rp_text` — for scrolling message text (backed by `bm_scroll`, a 1-bitplane 640×57 bitmap sharing plane 0 with `bm_text` — `fmain.c:832`, `fmain.c:938`).
- `rp_text2` — for hi-res status bar labels/menus (backed by `bm_text`, a 4-bitplane 640×57 bitmap — `fmain.c:835`).
- `rp` — global pointer swapped between `rp_map` and `rp_text` as needed.

### 20.2 Font System

Two fonts:

1. **Topaz 8** (`tfont`) — ROM font, loaded via `OpenFont(&topaz_ta)` — `fmain.c:650`, `fmain.c:779`. Used for status bar labels, menu text, and map-mode text.
2. **Amber 9** (`afont`) — custom disk font from `fonts/Amber/9` — `fmain.c:774-782`. Loaded via `LoadSeg` (not `OpenDiskFont`), cast through `DiskFontHeader`. Used for in-game scrolling messages and placard text. Applied at `fmain.c:1168`: `SetFont(rp,afont); SetAPen(rp,10); SetBPen(rp,11)`.

Note: [text.c](../text.c) is a standalone test program referencing `"sapphire.font"` at size 19 — unrelated to the game's font system.

### 20.3 `_ssp` — Scrolling String Print

Defined at `fsubs.asm:497-536`. Prints a formatted string to the current `rp` with embedded positioning commands.

**Escape code**: `XY` (byte value 128, `$80`) — `fsubs.asm:228`.

**String format**: Segments of printable ASCII (bytes 1–127) interspersed with `XY, x_half, y` positioning commands. The X coordinate is stored at half value and doubled during rendering (`add.w d0,d0` — `fsubs.asm:529`).

**Algorithm** (`fsubs.asm:501-531`):
1. Read byte. If 0: exit. If 128 (`XY`): read next two bytes as (x/2, y), call `GfxBase->Move(rp, x×2, y)`.
2. Otherwise: scan forward counting printable bytes, call `GfxBase->Text(rp, buffer, count)`.
3. Loop until null terminator.

**Example** — title text at `fsubs.asm:236-241`:
```asm
_titletext  dc.b  XY,(160-26*4)/2,33,$22,"The Faery Tale Adventure",$22
```
Called via `ssp(titletext)` at `fmain.c:1163`.

### 20.4 `_placard` — Decorative Border

Defined at `fsubs.asm:382-475`. Despite the name, this is a **visual effect**, not a text routine. Draws a recursive fractal line pattern on `rp_map` using offset tables `xmod`/`ymod` (±4 pixel deltas — `fsubs.asm:381-382`).

The pattern is mirror-symmetric: draws lines at original position, center-mirrored at (284,124), and two 90°/270° rotations. Uses 16×15 outer iterations with 5 inner passes. Color 1 for most lines, color 24 for the first inner pass — `fsubs.asm:411-414`.

Called during story sequences: `placard()` renders after `placard_text()` — e.g., `fmain.c:2869-2870`.

### 20.5 `_placard_text` — Story Message Dispatch

Defined at `narr.asm:235-248`. Indexes into the `mst` pointer table (20 story messages) and tail-calls `_ssp`:

| Index | Message | Citation |
|-------|---------|----------|
| 0 | Julian's quest intro | `narr.asm:252-259` |
| 1 | Julian's failure | `narr.asm:261-264` |
| 2 | Phillip sets out | `narr.asm:266-269` |
| 3 | Phillip's failure | `narr.asm:271-274` |
| 4 | Kevin sets out | `narr.asm:276-283` |
| 5 | Game over | `narr.asm:284-287` |
| 6–7 | Victory / Talisman recovered | `narr.asm:288-296` |
| 8–10 | Princess Katra rescue | `narr.asm:298-305` |
| 11–13 | Princess Karla rescue | `narr.asm:307-314` |
| 14–16 | Princess Kandy rescue | `narr.asm:316-322` |
| 17–18 | After seeing princess home | `narr.asm:330-335` |
| 19 | Copy protection intro | `narr.asm:337-347` |

Line width constraints (per `narr.asm:1-2`): max 36 chars for scroll text, 29 for placard text.

### 20.6 `_prdec` — Decimal Number Printing

Defined at `fsubs.asm:342-378`. Converts a number to ASCII digits in `numbuf[11]` (`fmain.c:492`), then calls `GfxBase->Text()`:

1. `ion6` subroutine (`fsubs.asm:367-377`): divides by 10 repeatedly, stores ASCII digits (`$30` + remainder) right-to-left, space-fills leading positions.
2. Adjusts pointer to display the requested number of digits.
3. Renders via `GfxBase->Text(rp, buffer, length+1)` — `fsubs.asm:350-353`.

Usage: `prdec(anim_list[0].vitality, 3)` — `fmain2.c:461`; `prdec(brave, 3)` — `fmain2.c:464`.

### 20.7 `_move` / `_text` — Low-Level Wrappers

Both at `fsubs.asm:477-495`. Thin wrappers around Amiga GfxBase routines using the global `_rp`:

- **`_move(x, y)`** (`fsubs.asm:477-485`): Calls `GfxBase->Move(rp, x, y)`.
- **`_text(string, length)`** (`fsubs.asm:487-495`): Calls `GfxBase->Text(rp, string, length)`.

### 20.8 Print Queue (`prq` / `ppick`)

A deferred display system using a 32-entry circular buffer — `fmain2.c:434-435`:

```c
char print_que[32];
short prec=0, pplay=0;
```

**`prq(n)`** at `fmain2.c:473-488` (inline assembly): enqueues a command byte. Silently drops if buffer full.

**`ppick()`** at `fmain2.c:443-470`: dequeues and executes one command per call. Called from Phase 14a (`fmain.c:2009`):

| Code | Action | Citation |
|------|--------|----------|
| 2 | Debug: coords + available memory | `fmain2.c:449-451` |
| 3 | Debug: position, sector, extent | `fmain2.c:452-456` |
| 4 | Display vitality at (245,52) | `fmain2.c:457-459` |
| 5 | Refresh menu via `print_options()` | `fmain2.c:460` |
| 7 | Full stat bar: Brv/Lck/Knd/Wlth | `fmain2.c:461-466` |
| 10 | Print "Take What?" | `fmain2.c:467` |

If queue empty: `Delay(1)` — yields to OS.

### 20.9 `print` / `print_cont` — C Text Output

**`print(str)`** at `fmain2.c:495-500`: Scrolls the text region up 10 pixels via `ScrollRaster(rp, 0, 10, TXMIN, TYMIN, TXMAX, TYMAX)`, then renders at (TXMIN, 42). Bounds: `TXMIN=16`, `TYMIN=5`, `TXMAX=400`, `TYMAX=44` — `fmain2.c:490-493`.

**`print_cont(str)`** at `fmain2.c:502-505`: Continues on the same line without scrolling.

Both use the global `rp` (set to `rp_text` during gameplay — `fmain.c:1167`). Text colors: pen 10 foreground, pen 11 background, JAM2 mode — `fmain.c:1168`.

### 20.10 `extract` — Template Engine

Defined at `fmain2.c:515-548`. Performs word-wrapping and hero name substitution:

- Uses local buffer `mesbuf[200]` — `fmain2.c:509`.
- Scans input character by character, max 37 chars per line — `fmain2.c:523`.
- `%` character → substitutes `datanames[brother-1]` (Julian/Phillip/Kevin) — `fmain2.c:528-530`, `fmain.c:604`.
- Carriage return (13) forces line break.
- At wrap boundary: calls `print(lstart)` to output line.

### 20.11 Message Dispatch (`speak`, `event`, `msg`)

Three inline-assembly functions at `fmain2.c:557-577` that index into null-terminated string tables and tail-call `extract()`:

- **`event(n)`** — uses `_event_msg` table (`narr.asm:10-30`): hunger, drowning, journey start, etc.
- **`speak(n)`** — uses `_speeches` table (`narr.asm:351+`): NPC dialogue indexed by speech number.
- **`msg(table, n)`** — generic: takes explicit string table and index.

Common handler `msg1` (`fmain2.c:572-577`): skips `n` null-terminated strings to find the target, then calls `extract()`.

### 20.12 Location Messages

**`find_place()`** at `fmain.c:2653-2680`: Called from Phase 14g. Determines `hero_sector`, selects appropriate message table:
- Outdoor (`region_num < 8`): `_place_tbl` / `_place_msg` — `narr.asm:100-148`, `narr.asm:164-195`.
- Indoor (`region_num > 7`): `_inside_tbl` / `_inside_msg` — `narr.asm:199-223`.

Each table entry is 3 bytes: `{min_sector, max_sector, message_index}`. Scans until `hero_sector` falls within range — `fmain.c:2663`. Mountain messages (index 4) vary by region — `fmain.c:2668-2671`.

**`map_message()`** at `fmain2.c:601-613`: Switches to fullscreen text overlay — fades down, clears playfield, hides status bar (`VP_HIDE`), sets `rp = &rp_map`, `viewstatus = 2`.

**`message_off()`** at `fmain2.c:615-620`: Returns to gameplay — fades down, restores `rp = &rp_text`, shows status bar, sets `viewstatus = 3`.

**`name()`** at `fmain2.c:593`: Prints current brother's name via `print_cont(datanames[brother-1])`.

### 20.13 Status Bar & HUD

The status bar occupies `vp_text` (640×57 hi-res). Color palette from `textcolors[20]` (`fmain.c:476-479`) — NOT affected by day/night fading.

**Stat display** via print queue:
- `prq(7)`: Full stat line at y=52 — `Brv:` at x=14, `Lck:` at x=90, `Knd:` at x=168, `Wlth:` at x=321 — `fmain2.c:461-466`.
- `prq(4)`: Vitality at (245,52) — `fmain2.c:457-459`.

**Menu display**: `print_options()` renders on `rp_text2`. Two columns (x=430, x=482), 6 rows at 9 px spacing, starting at y=8. Each label is 5 characters — `fmain.c:3064-3067`.

### 20.14 Compass

**`drawcompass()`** at `fmain2.c:351-365`. Two 48×24 pixel bitmaps stored as raw bitplane data in assembly:
- `_hinor` — base compass (all directions normal) — `fsubs.asm:249-260`.
- `_hivar` — highlighted direction segments — `fsubs.asm:262-275`.

Copied to chip RAM at init (`fmain.c:944-945`). Direction regions defined in `comptable[10]` (`fmain2.c:332-344`): 8 cardinal/ordinal rectangles plus 2 null entries.

Rendering: blits full normal compass to `bm_text` at (567,15), then overlays the highlighted direction segment from `_hivar`. Only bitplane 2 differs between the two images — `bm_source->Planes[2]` is swapped (`fmain2.c:357-361`).

Called from `_decode_mouse` in `fsubs.asm:1582`.

