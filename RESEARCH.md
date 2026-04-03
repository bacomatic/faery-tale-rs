## RESEARCH.md

Canonical reverse-engineering reference for The Faery Tale Adventure (1987
Amiga). Covers game systems, binary file formats (`songs`, `v6`, ADF layout),
original game mechanics, and implementation notes derived from the manual and
source code.

For build/run setup, see `README.md`. Stable agent lookup keys live in
`research_index.toml`.

## Maintenance workflow

- Add or update the human-readable note here first.
- Add or update a matching `[[entry]]` in `research_index.toml` with a stable `id`.
- Keep the `title` exactly aligned with this document section heading.
- Point `anchor` at the markdown heading slug in this file.
- Bump `last_updated` in `research_index.toml` when entries change.

---

## Game World & Map System: Data format

All game world data lives in `game/image`, an Amiga 880KB floppy disk image accessed as a flat file. `load_track_range(block, count, buf)` reads `count × 512` bytes from offset `block × 512`.

**Memory regions loaded from `game/image`:**

| Buffer | ADF source | Size | Description |
|---|---|---|---|
| `sector_mem` | `nd->sector`, 64 blocks | 32 768 B | 256 sectors × 128 bytes; tile indices (`SECTOR_SZ = 128*256`, stride confirmed by `lsl.w #7` in fsubs.asm) |
| `map_mem` | `nd->region`, 8 blocks | 4 096 B | Region map; maps `(secx, secy)` → sector number |
| `terra_mem[0..512]` | `TERRA_BLOCK + nd->terra1`, 1 block | 512 B | Terrain type table, layer 1 |
| `terra_mem[512..1024]` | `TERRA_BLOCK + nd->terra2`, 1 block | 512 B | Terrain type table, layer 2 |
| `image_mem` | 4 tilesets × (5 planes × 8 blocks) | 81 920 B | Tile graphics; 4 groups × 64 tiles × 5 bitplanes × 64 B |

**Tile format:**
- 16 × 32 pixels, 5 bitplanes (→ 32 colours from a 32-entry palette)
- 64 bytes per plane per tile (2 bytes/row × 32 rows)
- `image_mem` layout: plane stride = `IPLAN_SZ` = 16 384 B; tileset stride within each plane = `QPLAN_SZ` = 4 096 B
- Tile `n` in group `g`, plane `p`: byte offset = `p × IPLAN_SZ + g × QPLAN_SZ + n × 64`

**Viewport and coordinate system:**
- World: up to `MAXCOORD = 32 768` units each axis (pixel coordinates)
- Image units: 16 px wide × 32 px tall per tile
- Sectors: 16 × 8 image units = 256 × 256 pixels (128 sectors per axis via `map_mem` grid)
- Screen playfield: 288 × 140 lores px, x-offset 16 px from left edge
- Viewport tile grid: `minimap[114]` = 19 columns × 6 rows (each short = tile index)
- `map_x = hero_x − 144`, `map_y = hero_y − 90` (hero centred)
- `img_x = map_x >> 4`, `img_y = map_y >> 5`

**Region table (`file_index[10]`):** Ten outdoor/indoor regions (F1–F10); each `struct need` holds: `image[4]` (4 tileset ADF block numbers), `terra1`, `terra2`, `sector`, `region`, `setchar`. Region 0–7 = outdoor, 8–9 = indoor/dungeon. `region_num` selects the active region.

**`genmini(img_x, img_y)`** (asm in `fsubs.asm`): Fills `minimap[114]` by walking the 19×6 tile grid, looking up each tile's sector via `map_mem`, then its byte value from `sector_mem`. Used as the tile index for rendering.

**`map_draw()`** (asm in `fsubs.asm`): Blits all 114 minimap tiles to the 5-plane screen bitplanes, column by column (19 strips × 6 tiles each). Tile `image_mem` offset = `plane_base + char_idx × 64`.

## Game World & Map System: Constants, addresses, and implementation notes

### Key constants and block addresses (region F4, region_num=3 is starting region)

```
file_index[3] = {320, 360, 400, 440, 2, 3, 32, 168, 21}
  image[0]=320, image[1]=360, image[2]=400, image[3]=440
  terra1=2, terra2=3, sector=32, region=168, setchar=21
TERRA_BLOCK = 149
```

So starting map data (region 3):
- sector_mem: ADF offset 32×512 = 16 384
- map_mem: ADF offset 168×512 = 86 016
- terra_mem[0]: ADF offset (149+2)×512 = 77 312
- terra_mem[512]: ADF offset (149+3)×512 = 77 824
- image[0] plane 0: ADF offset 320×512 = 163 840

### Notes

- The original renders via 5-plane Amiga bitplanes; we use RGBA32 SDL2 textures. The decode step (planar → packed pixel) is the same as used for IFF images in `bitmap.rs`.
- `minimap` in the original is a `short[114]` (tile index as signed 16-bit); in our port use `u8[114]` since tile indices fit in one byte (0–255 from `sector_mem`).
- Scroll: original uses Amiga copper-list `RxOffset`/`RyOffset` for sub-tile pixel scrolling; we achieve the same by shifting blit destination rects.
- `xreg` and `yreg` track which 64×32 sector block is currently loaded into `sector_mem`. Initially 0. Updated by `load_new_region()` as hero moves.

---

## Key Bindings: Original game key map

### Movement

- **Numpad 1–9**: 8-way movement using physical key position (ignore numerals).
  Numpad layout maps to directions:
  ```
  7=NW  8=N   9=NE
  4=W   5=--  6=E
  1=SW  2=S   3=SE
  ```
- **Joystick**: press in desired direction.
- **Mouse**: hold left button over compass point in HI bar.
- Release key/button to stop.

### Combat

- **Numpad 0**: attack (original fire button).
- **Joystick fire button** / **Mouse right button**: attack.
- Attacks are directional — must face the opponent.
- Direction of attack controlled same as movement.

### Command Menu System

The HI bar has 5 category tabs, each revealing a sub-menu.
Activated by mouse click on the labeled bar, or by keyboard shortcut.

#### Items Menu
| Label | Key | Action |
|-------|-----|--------|
| List  | `L` | Show all carried items |
| Take  | `T` | Pick up item from ground / dead body |
| Look  | `?` | Look for hidden items |
| Give  | `G` | Give item to someone |
| Use   | `U` | Opens weapon sub-menu (see below) |

**Use sub-menu** (weapon selection):
| Weapon | Key | Notes |
|--------|-----|-------|
| Dirk   | `1` | Draw dagger |
| Mace   | `2` | Draw mace |
| Sword  | `3` | Draw sword |
| Bow    | `4` | Draw bow and arrow |
| Wand   | `5` | Draw magic wand |
| Key    | `K` | Opens key color sub-menu |

**Key sub-menu** (via `K`):
| Key Color | Shortcut |
|-----------|----------|
| Gold      | `K1`     |
| Green     | `K2`     |
| Blue      | `K3`     |
| Red       | `K4`     |
| Grey      | `K5`     |
| White     | `K6`     |

#### Magic Menu
One-use magic items. Each use consumes one of that item type.

| Label | Key  | Item |
|-------|------|------|
| Stone | `F1` | Blue stone |
| Jewel | `F2` | Green jewel |
| Vial  | `F3` | Glass vial (restorative) |
| Orb   | `F4` | Crystal orb |
| Totem | `F5` | Bird totem |
| Ring  | `F6` | Gold ring |
| Skull | `F7` | Jade skull |

#### Talk Menu
| Label | Key | Action |
|-------|-----|--------|
| Yell  | `Y` | Yell |
| Say   | `S` | Say |
| Ask   | `A` | Ask |

#### Buy Menu
Only works near a merchant character.

| Item   | Key |
|--------|-----|
| Food   | `O` |
| Arrow  | `R` |
| Vial   | `8` |
| Mace   | `C` |
| Sword  | `W` |
| Bow    | `B` |
| Totem  | `E` |

#### Game Menu
| Label  | Key        | Action |
|--------|------------|--------|
| Pause  | `Spacebar` | Pause/unpause the game |
| Music  | `M`        | Toggle music |
| Sound  | `F`        | Toggle sound effects |
| Quit   | `Q`        | Quit — sub-menu: exit or save |
| Load   | `L`        | Load saved game — 8 slots A–H |

### Player Stats (narration scroll)

Five stats displayed on the HI bar scroll area:

| Stat     | Abbr  | Description |
|----------|-------|-------------|
| Bravery  | `Brv` | Battle prowess |
| Luck     | `Lck` | Fairy rescue chance on death |
| Kindness | `Knd` | NPC communication threshold |
| Vitality | `Vit` | Health (0 = death) |
| Wealth   | `Wlt` | Coins carried |

When a character dies with sufficient Luck, a fairy heals him and
teleports him to the last safe location.

### Map Size

The world is 144 screens tall × 100 screens wide.

---

## Player Character Stats

Three playable brothers, each with distinct starting attributes (`blist[]` in `fmain.c`):

| Character | `brave` | `luck` | `kind` | `wealth` | Starting weapon |
|-----------|---------|--------|--------|----------|-----------------|
| Julian    | 35      | 20     | 15     | 20       | Dirk (1)        |
| Phillip   | 20      | 35     | 15     | 15       | Dirk (1)        |
| Kevin     | 15      | 20     | 35     | 10       | Dirk (1)        |

On spawn (`revive(TRUE)`):
- `hero_x = safe_x = 19036`, `hero_y = safe_y = 15755`, `region_num = 3`
- `anim_list[0].vitality = 15 + brave / 4` (Julian=23, Phillip=20, Kevin=18)
- `daynight = 8000` (early morning), `lightlevel = 300` (computed from daynight)
- `hunger = fatigue = 0`
- Raft spawns at `(13668, 14470)`, goodfairy setfig at `(13668, 15000)`

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

**`brave`** — melee weapon reach radius `bv = brave/20 + 5` (0–15 cap). Each enemy kill grants `brave++`. Hero death subtracts 5 from `luck`. Vitality cap = `15 + brave/4`.

**`luck`** — modifies fairy rescue: if `luck < 1 && goodfairy < 200`, the fairy does **not** appear and a new brother spawns instead (`revive(TRUE)`). Reduced by enemy hits to player (`luck -= 2` in some death paths, `-5` on hero death).

**`kind`** — moral stat. Killing a non-evil `SETFIG` NPC: `kind -= 3` (floored at 0). Giving 2 gold to a beggar: if `rand64() > kind`, then `kind++`. Used to gate some dialogue/quest paths.

**`wealth`** — gold. Starts from `blist[]`. Buy menu deducts; looting treasure adds; giving to beggars deducts 2.

---

## Hunger & Fatigue System

Both counters tick simultaneously every 127 `daynight` ticks (`(daynight & 127) == 0`), i.e., approximately every 4.2 seconds of real time at 30 Hz when `daynight` increments by 1 per game tick.

**Auto-eat**: in safe zones, if `hunger > 30 && stuff[24] > 0` (has Fruit), one Fruit is consumed and `hunger -= 30`.

**Hunger progression:**

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

`eat(amt)`: `hunger -= amt; if hunger < 0 → hunger = 0, event(13)` ("Feeling better"). Used for Fruit (amt=30) and buying food at inns (amt=50).

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

**Vitality recovery**: every 1024 `daynight` ticks (`(daynight & 0x3ff) == 0`), if `vitality < 15 + brave/4` and not DEAD: `vitality++`, prints HP display.

---

## Day/Night Cycle

`daynight` is a `USHORT` counter [0..23999], incremented by 1 per game tick (~30 Hz):
- 24000 ticks = one full in-game day ≈ 800 seconds real time (≈13.3 minutes)
- `daynight` is incremented in the main game loop (`fmain.c:2370`), not in the VBlank interrupt (which only handles music)

**Frame rate**: the game loop effectively runs at ~30 Hz. During scrolling, the Amiga blitter is saturated (5-plane scroll + sprite blits exceed one 16.7 ms VBlank period). When standing still, an explicit `Delay(1)` in `ppick()` (`fmain2.c:621`) throttles the idle loop to match.

`lightlevel = daynight / 40` then if `lightlevel >= 300`: `lightlevel = 600 - lightlevel`.
This makes a symmetric triangle wave: 0 → 300 → 0 over the day.

| `lightlevel` | Condition |
|---|---|
| > 120 | Daytime (music group 0 or palace group 4) |
| ≤ 120 | Nighttime (music group 2) |
| < 40  | `ob_listg[5].ob_stat = 3` (night lit-object variant) |

**Day periods** (`dayperiod = daynight / 2000`, 12 periods per day):

| `dayperiod` value | Event |
|---|---|
| 0 | event(28) — midnight |
| 4 | event(29) — dawn |
| 6 | event(30) — noon |
| 9 | event(31) — dusk |

**`day_fade()`** (`fmain2.c:2059–2071`) calls `fade_page()` every 4th `daynight` tick (`daynight & 3 == 0`) or on viewstatus changes:

```c
ll = light_timer ? 200 : 0;
if (region_num < 8)
    fade_page(lightlevel - 80 + ll, lightlevel - 61, lightlevel - 62, TRUE, pagecolors);
else
    fade_page(100, 100, 100, TRUE, pagecolors);  // full brightness indoors
```

`fade_page(r, g, b, limit, colors)` applies per-frame colour scaling. Night limit floor: r≥10, g≥25, b≥60 (ensures blue-tinted night). `light_timer` (Green Jewel) adds +200 to the **red channel parameter only**, making the scene warm and bright. Indoor regions (`region_num ≥ 8`) always use full brightness (100, 100, 100) — no day/night variation.

### Palette saturation / desaturation thresholds

The `day_fade()` parameters passed to `fade_page()` are (without Green Jewel):

| Channel | Parameter formula    | Clamp range | Hits floor when    | Hits ceiling when    |
|---------|----------------------|-------------|--------------------|----------------------|
| Red     | `lightlevel - 80`    | [10, 100]   | `lightlevel ≤ 90`  | `lightlevel ≥ 180`  |
| Green   | `lightlevel - 61`    | [25, 100]   | `lightlevel ≤ 86`  | `lightlevel ≥ 161`  |
| Blue    | `lightlevel - 62`    | [60, 100]   | `lightlevel ≤ 122` | `lightlevel ≥ 162`  |

Converting `lightlevel` to `daynight` ticks: `daynight = lightlevel × 40` (ascending half),
`daynight = (600 − lightlevel) × 40` (descending half). Time of day: `hours = daynight / 1000`,
`minutes = (daynight % 1000) × 60 / 1000`.

**Dawn sequence** (ascending lightlevel, midnight → noon):

| Time  | daynight | lightlevel | Event                                     |
|-------|----------|------------|-------------------------------------------|
| 03:26 |     3440 |         86 | Green rises off floor (25→26)             |
| 03:36 |     3600 |         90 | Red rises off floor (10→11)               |
| 04:52 |     4880 |        122 | Blue rises off floor (60→61)              |
| 06:26 |     6440 |        161 | Green saturates (reaches 100)             |
| 06:28 |     6480 |        162 | Blue saturates (reaches 100)              |
| 07:12 |     7200 |        180 | Red saturates — **full brightness begins** |

**Dusk sequence** (descending lightlevel, noon → midnight):

| Time  | daynight | lightlevel | Event                                      |
|-------|----------|------------|--------------------------------------------|
| 16:48 |    16800 |        180 | Red drops below 100 — **full brightness ends** |
| 17:31 |    17520 |        162 | Blue drops below 100                       |
| 17:33 |    17560 |        161 | Green drops below 100                      |
| 19:07 |    19120 |        122 | Blue hits floor (60)                       |
| 20:24 |    20400 |         90 | Red hits floor (10)                        |
| 20:33 |    20560 |         86 | Green hits floor — **full darkness begins** |

**Summary zones:**

| Period          | Time range      | Description                                     |
|-----------------|-----------------|--------------------------------------------------|
| Full darkness   | 20:33 – 03:26   | All channels clamped to floor (r=10, g=25, b=60) |
| Dawn transition | 03:26 – 07:12   | Channels rise off floor → ceiling progressively  |
| Full brightness | 07:12 – 16:48   | All channels clamped to 100 (unmodified palette)  |
| Dusk transition | 16:48 – 20:33   | Channels drop from ceiling → floor progressively |

Full brightness lasts **9 hours 36 minutes** in-game; full darkness lasts **6 hours 53 minutes**.
Dawn transition spans **3 hours 46 minutes**; dusk transition mirrors at **3 hours 45 minutes**
(rounding from tick boundaries).

**With Green Jewel** (`light_timer`): the red parameter becomes `lightlevel − 80 + 200 =
lightlevel + 120`, which exceeds 100 at all lightlevels — red is permanently clamped to 100.
Green and blue thresholds are unaffected. The per-color boost (`if r1 < g1: r1 = g1`) further
ensures warm tones. Net effect: the jewel eliminates the red dimming entirely, leaving only
green/blue to cycle.

---

## Door / Portal System

`doorlist[DOORCOUNT]` — 86 doors, sorted ascending by `xc1` for binary search (outdoor→indoor direction). Each door:

```c
struct door {
    USHORT xc1, yc1;  // outdoor world coords
    USHORT xc2, yc2;  // indoor world coords
    char type;
    char secs;         // 1=buildings (region 8), 2=caves (region 9)
};
```

**Door types** (LSB = horizontal):

| Constant | Value | Notes |
|---|---|---|
| HWOOD | 1 | Horizontal wood door |
| VWOOD | 2 | Vertical wood door |
| HSTONE | 3 | Horizontal stone door |
| VSTONE | 4 | Vertical stone door |
| HCITY | 5 | Horizontal city gate |
| VCITY | 6 | Vertical city gate |
| CRYST | 7 | Crystal palace gate |
| SECRET | 8 | Secret passage |
| BLACK | 9 | Black iron gate |
| MARBLE | 10 | Marble archway |
| LOG | 11 | Log cabin door |
| HSTON2 | 13 | Heavy stone door |
| VSTON2 | 14 | |
| STAIR | 15 | Staircase portal |
| DESERT | 17 | Oasis entrance (needs `stuff[STATBASE] >= 5`) |
| CAVE / VLOG | 18 | Cave entrance / log cabin yard |

**Entry/exit position logic:**
- Horizontal door (`type & 1 == 1`): player enters only if `(hero_y & 0x10) == 0` (upper half of tile — hero walks from lower half and triggers on crossing into upper half)
- Vertical door (`type & 1 == 0`): player enters only if `(hero_x & 15) > 6`
- Cave type: entry offset `(xc2+24, yc2+16)`, exit offset `(xc1-4, yc1+16)`
- Horizontal non-cave: entry `(xc2+16, yc2)`, exit `(xc1+16, yc1+34)`
- Vertical: entry `(xc2-1, yc2+16)`, exit `(xc1+20, yc1+16)`

**`secs` field** sets `new_region` on crossing: `secs==1` → `new_region=8` (indoor), `secs==2` → `new_region=9` (cave/dungeon). Indoor regions use a linear scan of `doorlist`; outdoor uses binary search.

---

### Two-Phase Door Interaction

Doors operate in two distinct phases. The teleport (`xfer`) **never** fires at bump time; it fires on a separate every-frame scan after the tile has been made passable.

#### Phase 1 — Bump / Open (`doorfind`)

Triggered from the hero sprite-movement loop when `proxcheck(xtest, ytest, 0)` returns **15** (terrain type 15 = door tile) for the hero's proposed new position:

```c
j = proxcheck(xtest, ytest, i);   // returns terrain type at proposed pos
if (i == 0) {                      // i==0 = hero sprite only
    if (j == 15) {
        doorfind(xtest, ytest, 0); // open attempt with no key (keytype=0)
    } else
        bumped = 0;                // reset dedup flag on any non-door block
}
if (j) goto checkdev1;            // still blocked — hero does NOT move
```

`doorfind(x, y, keytype)` (**fmain.c** ~line 1174):
1. Probes `px_to_im(x, y)`, `px_to_im(x+4, y)`, `px_to_im(x-4, y)` to find a terrain-15 cell.
2. Grid-aligns to find the top-left tile corner: steps `x -= 16` while `px_to_im(x-16, y) == 15`; steps `y += 32` if `px_to_im(x, y+32) == 15`.
3. Converts to image coords: `x >>= 4`, `y >>= 5`.
4. Reads the tile index from `sector_mem` via `mapxy(x, y)`.
5. Looks up the tile in `open_list[17]` — the tile-graphic replacement table.
6. If a matching entry is found and `keytype` matches (or `keytype == NOKEY`):
   - **Replaces tiles in `sector_mem`** to show the open-door graphic.
   - Sets `viewstatus = 99` (forces screen refresh).
   - Prints `"It opened."`.
   - Returns `TRUE`.
7. If no match: if `!bumped && !keytype`, prints `"It's locked."`; sets `bumped = 1`. Returns `FALSE`.

After `doorfind` returns, the hero's move is **still blocked** (goto checkdev1 runs regardless). The tile replacement means that on the very next frame, `proxcheck` returns a passable value and the hero can walk forward.

**`bumped` global flag**: prevents `"It's locked."` from printing every frame while the player holds a direction key into a locked door. Reset to 0 when `proxcheck` returns a non-15 value, and to 0 explicitly at the start of any key-usage attempt (USE→KEYS menu).

#### `open_list` — Door graphic replacement table

```c
struct door_open {
    UBYTE  door_id;   // closed-door tile index in sector_mem
    USHORT map_id;    // tileset region ID (from current_loads.image[])
    UBYTE  new1;      // replacement tile for the main cell
    UBYTE  new2;      // replacement tile for secondary cell (0 = none)
    UBYTE  above;     // direction of secondary cell: 1=above, 2=right, 3=left, 4=multi
    UBYTE  keytype;   // required key: NOKEY=0, GOLD=1, GREEN=2, KBLUE=3, RED=4, GREY=5, WHITE=6
} open_list[17];
```

`above == 4` (multi-tile, used for large gates): sets three additional cells — `mapxy(x, y-1) = 87`, `mapxy(x+1, y) = 86`, `mapxy(x+1, y-1) = 88`.

Key type mapping (`enum ky`): `NOKEY=0, GOLD=1, GREEN=2, KBLUE=3, RED=4, GREY=5, WHITE=6`. Matches key inventory slots `stuff[16..21]` (KEYBASE=16). Talisman (BLACK gates) and DESERT doors are not handled via `open_list`; they are checked by special-case logic before the `open_list` loop.

#### Phase 2 — Walk-through / Teleport (every-frame door scan)

After every hero move, the main game loop runs a door scan (**fmain.c** ~line 2190) using the *current* hero position:

```c
xtest = hero_x & 0xfff0;   // grid-align to 16px X cell
ytest = hero_y & 0xffe0;   // grid-align to 32px Y cell
```

**Outdoor (region\_num < 8)** — binary search on `doorlist` sorted by `xc1`:
- Matches when `xc1 <= xtest <= xc1+16` and `yc1 == ytest`.
- Sub-tile check: horizontal doors require `(hero_y & 0x10) == 0`; vertical require `(hero_x & 15) <= 6`.
- DESERT gate special case: if `stuff[STATBASE] < 5`, silently blocks (no message, no teleport).
- On match: compute indoor spawn via the entry offset formulas above, set `new_region`, call `xfer(ix, iy, FALSE)`.

**Indoor (region\_num >= 8)** — linear scan matching on `xc2/yc2`:
- Matches when `yc2 == ytest` and (`xc2 == xtest` or `xc2 == xtest-16` for horizontal doors).
- Same sub-tile checks as outdoor (inverted: horizontal require `(hero_y & 0x10) != 0`; vertical require `(hero_x & 15) >= 2`).
- On match: compute outdoor spawn via exit offset formulas, call `xfer(ox, oy, TRUE)`.

The scan fires **unconditionally every frame** regardless of door open/closed state — it matches on position only. It is the tile-replacement in Phase 1 that makes the position reachable.

#### Key door flow (USE→KEYS menu)

```c
case KEYS:
    hit -= 5;            // hit 0–5 → key slot 0–5
    bumped = 0;          // reset dedup flag before attempting
    if (stuff[hit + KEYBASE]) {
        for (i = 0; i < 9; i++) {
            x = newx(hero_x, i, 16);  // probe 8 compass dirs + center, 16px radius
            y = newy(hero_y, i, 16);
            if (doorfind(x, y, hit + 1)) {   // keytype = hit+1 (1–6)
                stuff[hit + KEYBASE]--;       // consume one key
                break;
            }
        }
        if (i > 8) {
            // "% tried a [Key] but it didn't fit."
        }
    }
```

`doorfind` is called with `keytype = hit + 1` (non-zero). The `open_list` check succeeds when `open_list[j].keytype == keytype`. On success: tiles are replaced, `"It opened."` is printed. The hero then walks through on the next move (Phase 2 as normal).

---

## Terrain Collision System

Sources: `original/fmain.c`, `original/terrain.c`, `original/fsubs.asm`.

### Overview

Terrain collision is **tile-type-based**, not bitplane-based. There is no dedicated collision bitplane in the ADF. Instead, every image tile (world graphic tile) has an associated 4-byte terrain descriptor stored in `terra_mem`, which is consulted at runtime to determine whether a position is passable and what special behavior applies.

---

### Memory Buffers

| Buffer | Size | Purpose |
|--------|------|---------|
| `sector_mem` | `128×256 + 4096` bytes | Maps (sector, tile-position) → image tile index. 256 sectors, 128 bytes each. |
| `map_mem` | `8 ADF blocks` (4096 bytes) | Maps world region coordinates → sector numbers. |
| `terra_mem` | 1024 bytes (chip RAM) | Terrain descriptor table. 256 entries × 4 bytes (two 512-byte halves, one per terrain file loaded). |

`terra_mem` is loaded from ADF starting at block `TERRA_BLOCK` (149). Each region specifies two terrain file indices (`terra1` and `terra2`); they are loaded into the two 512-byte halves of `terra_mem`:

```c
load_track_range(TERRA_BLOCK + nd->terra1, 1, terra_mem,       1);
load_track_range(TERRA_BLOCK + nd->terra2, 1, terra_mem + 512, 2);
```

---

### Terrain Descriptor Layout (`terra_mem` entry, 4 bytes per tile)

`terrain.c` extracts per-tile descriptor data from each landscape source file and writes 4 bytes per image tile:

| Byte offset | Field | Description |
|-------------|-------|-------------|
| +0 | `maptag` | Bit mask for rendering: controls which sub-cells within a 32×64 tile get the feature blitted (`maskit()` call). |
| +1 | `terrain` | **2 nibbles**: upper nibble = terrain type (returned by `px_to_im`); lower nibble = TODO: verify exact meaning. |
| +2 | `tiles` | 4-bit feature presence mask. Controls which quadrant sub-cells within the tile carry the terrain feature (checked against the position bit `d4` derived from pixel x/y). If the relevant bit is zero, `px_to_im` returns 0 (open terrain) even if the image tile would otherwise have a type. |
| +3 | `big_colors` | Palette index for minimap rendering. |

Access pattern in C (used for masking logic):

```c
cm = minimap[cell] * 4;          // 4 bytes per entry
k  = terra_mem[cm + 1] & 15;     // lower nibble (masking case selector)
maskit(xm, ym, blitwide, terra_mem[cm]); // +0 = maptag bit mask
```

Access pattern in ASM (`px_to_im`):

```asm
and.b   2(a1,d1.w),d4   ; terra_mem[entry+2].tiles & position_bit
beq.s   px99            ; zero = no feature at this sub-cell → return 0
move.b  1(a1,d1.w),d0   ; terra_mem[entry+1].terrain
lsr.b   #4,d0           ; upper nibble = terrain type
```

**Sprite depth/masking block type** (lower nibble of `terra_mem[cm+1]`, i.e. `& 0x0f`), used by `maskit`:

| k | Name | Masking condition (skip masking if…) |
|---|---|---|
| 0 | Transparent | Always skip (fully passable) |
| 1 | Right-half | `xm == 0` (left column only) |
| 2 | Ground-level | `ystop > 35` (above ground line) |
| 3 | Bridge | `hero_sector != 48 || i != 1` (bridge sector special) |
| 4 | Right+Ground | `xm == 0 OR ystop > 35` |
| 5 | Right OR Ground | `xm == 0 AND ystop > 35` |
| 6 | Full-if-above | If `ym != 0`: substitute tile 64 as solid mask |
| 7 | Near-top | `ystop > 20` |

This table controls sprite-depth overlap (whether a sprite is drawn in front of or behind terrain tiles), not walking passability. Walking passability is handled separately by `proxcheck()`, which tests for hard collisions with tile geometry via `prox()`.

---

### Coordinate-to-Terrain Lookup: `px_to_im(x, y)`

Implemented in `fsubs.asm`. Converts absolute pixel coordinates to a terrain type (0–15):

```
1. Compute tile position bit (d4 = 0x80, then shifted):
     if x & 8:  d4 >>= 4   (right half of tile)
     if y & 8:  d4 >>= 1   (lower half within row)
     if y & 16: d4 >>= 2   (second tile row)

2. imx = x >> 4            (image x: tile column, 16 px/col)
   imy = y >> 5            (image y: tile row,    32 px/row)

3. secx = (imx >> 4) - xreg, clamped 0–63  (sector column)
   secy = (imy >>  3) - yreg, clamped 0–31  (sector row)

4. sec_num = map_mem[secy * 128 + secx + xreg]

5. offset  = sec_num * 128 + (imy & 7) * 16 + (imx & 15)
   image_n = sector_mem[offset]             (image tile index 0–255)

6. entry   = image_n * 4                   (into terra_mem)
   if (terra_mem[entry+2] & d4) == 0:
       return 0                            (no feature at sub-cell)
   return terra_mem[entry+1] >> 4          (upper nibble = terrain type)
```

---

### Terrain Type Table

Derived from the comment block at `fmain.c:727` and all `px_to_im`/`proxcheck` usage sites:

| Type | Symbolic name | Behavior |
|------|--------------|---------|
| 0 | Open / land | Fully passable; no special effect. |
| 1 | **Impassable** | Hard block (walls, solid mountains, buildings). `proxcheck` always blocks. |
| 2 | Sink (shallow) | Character starts sinking; `environ` → 2. Water — wading possible. |
| 3 | Sink (deep) | Faster sinking; `environ` → 5. |
| 4 | Water (shallow) | Sinking threshold 10; triggers `SINK` state at depth 15; transition to `SINK` at 30. |
| 5 | Water (deep / navigable by raft) | Sinking threshold 30; raft navigates here. |
| 6 | Special A | Sets `environ` = −1. TODO: verify (ice/slippery?). |
| 7 | Special B (lava?) | Sets `environ` = −2. Volcanic region tile; vultures (`xtype==52`) can spawn here. |
| 8 | Special C | Sets `environ` = −3. Blocks left-foot `proxcheck` probe (≥8 threshold). |
| 9 | Pit / fall trap | Triggers `FALL` state for the hero; reduces `luck` by 2. |
| 10–11 | Hard block (high) | Blocks `proxcheck` right-foot probe (≥10 threshold). TODO: verify specific sub-types. |
| 12 | Water passage | Normally blocking (≥10 for right, ≥8 for left), but `stuff[30]` (water-walk item?) allows passage. |
| 13–14 | Hard block | Block both probes. TODO: verify specific sub-types. |
| 15 | **Door** | Triggers `doorfind()` on the hero's attempted move; stops projectiles. |

The comment in `fmain.c` also mentions planned-but-unclear types: "slippery, fiery, changing, climbable, pit trap, danger, noisy, magnetic, stinks, slides, slopes, whirlpool." Only types 0–9 and 15 have verified game behavior in the shipped code.

---

### Collision Check: `proxcheck(x, y, entity_index)` → `_prox` in `fsubs.asm`

`proxcheck` samples **two points** straddling the character's feet (±4 pixels horizontally, +2 pixels vertically from the passed position). It returns 0 if passable, or the terrain type if blocked.

```asm
_prox:
    ; Right foot: (x+4, y+2)
    call px_to_im(x+4, y+2)
    if result == 1:  goto blocked      ; impassable
    if result >= 10: goto blocked      ; hard-block types

    ; Left foot: (x-4, y+2)
    call px_to_im(x-4, y+2)
    if result == 1:  goto blocked      ; impassable
    if result >= 8:  goto blocked      ; hard-block types (lower threshold)

    clr d0                             ; both clear → return 0 (passable)
blocked:
    rts                                ; d0 = terrain type (non-zero = blocked)
```

The asymmetric thresholds (≥10 right, ≥8 left) mean types 8–9 only block the left-foot probe, which may be an artifact of the original code's heuristic collision. This is faithfully reproduced from the source.

**Caller interpretation** of the return value:

- `== 0` → fully passable → allow move
- `== 15` → door tile → call `doorfind()`
- `== 12` → water-walk check (passes if `stuff[30]` is set)
- anything non-zero → blocked; try deviated direction (`checkdev1/2`)

---

### Special Terrain Behaviors

| Condition | Effect |
|-----------|--------|
| Type 2–5 while walking | Increments `environ` (submersion depth); at depth 15 triggers `SINK` animation state. |
| Type 4/5 at depth 30 | Full submersion → `SINK`; at `hero_sector==181` (river crossing) triggers `xfer` to region 9. |
| Type 0 (open) | Resets `environ` toward 0 (character surfaces). |
| `race == 2` (wraith) or `race == 4` (snake) | `px_to_im` result forced to 0 — immune to water sinking. |
| `riding == 5` (on raft) | `raftprox` set; drowning suppressed (`k = 0`). Raft can only navigate type 5 tiles. |
| Type 9 + hero on `xtype==52` (vulture) | Triggers `FALL` state; luck −2. |
| Type 1 or 15 | Stops projectiles (arrows/fireballs) dead. |
| `passmode` set (weapon pass) | Weapon rendered with its own bounding box; masking applied per-pass (see "Weapon rendering and two-pass sprite masking"). |

---

### Terrain Source Files (`terrain.c`)

The build tool `terrain.c` reads 17 named landscape image files and extracts 64 tile descriptors from each, writing them sequentially to the `terra` output file (which is then stored in the ADF at block 149+):

```
wild, build, rock, mountain1, tower, castle, field, swamp, palace,
mountain2, doom, mountain3, under, cave, furnish, inside, astral
```

Each landscape file is structured as `5 × 64 × 64` bytes of image bitplane data (`IPLAN_SZ`), followed by four 64-byte descriptor arrays: `maptag[64]`, `terrain[64]`, `tiles[64]`, `big_colors[64]`. `terrain.c` seeks past the image planes and reads only the descriptor arrays.

---

## Combat System

Combat runs every VBlank in the main loop. All figures in `anim_list[0..anix-1]` that are not in `WALKING` or `DEAD` state and not index 1 (raft) attempt to attack.

**Weapon strength (`wt`)**:
```
wt = anim_list[i].weapon   // weapon index
if wt >= 8: wt = 5         // cap touch attack
wt += bitrand(2)           // random bonus 0–2
```

Weapons map: 0=none, 1=Dirk, 2=Mace, 3=Sword, 4=Bow, 5=Wand, 6+=special/touch.

**Hit detection (melee)**:
```
xs = newx(abs_x, facing, wt * 2) + rand8() - 3   // weapon tip X
ys = newy(abs_y, facing, wt * 2) + rand8() - 3   // weapon tip Y
bv = (player: brave/20+5) or (NPC: 2+rand4())     // hit box radius
bv = min(bv, 15)
hit_check: yd = max(|dx|, |dy|) for each target
if yd < bv: dohit(attacker, target, facing, wt)
if yd < bv+2 and wt != 5: effect(1)   // near-miss sound
```

**`dohit(i, j, fc, wt)`**:
- Reduces `anim_list[j].vitality -= wt`; floor at 0
- Special guard: Necromancer (race 9) or witch (race 0x89) immune unless `anim_list[0].weapon >= 4`
- Sound: player hit → `effect(0, 800+bitrand(511))`; arrow→player → `effect(2, 500+rand64())`; monster hit → `effect(3, 400+rand256())`; special hit → `effect(5, 3200+bitrand(511))`
- Pushback: `move_figure(j, fc, 2)` on target; if hits and attacker is player, also `move_figure(i, fc, 2)` (recoil)
- `checkdead(j, dtype)` called after damage

**`checkdead(i, dtype)`**:
- If `vitality < 1` and not already DYING/DEAD: set DYING state
- If killed enemy (i > 0): `brave++`
- If killed friendly SETFIG (NPC): `kind -= 3` (floored at 0)
- If hero (i == 0): `event(dtype)`, `luck -= 5`, `setmood(TRUE)` → death music

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

**Missile / Arrow combat**:
- 6 active missiles in `missile_list[]`; each has position, direction, speed, archer ID
- Move `speed * 2` pixels per tick; expire after 40 ticks or hitting terrain (tile 1 or 15)
- Damage: `rand8() + 4` per hit; magic bolt uses `effect(5)` (fireball), arrow uses `effect(2)` or `effect(4)`
- Arrow hit box radius: 6 for arrow, 9 for magic bolt

---

## Enemy Types (Encounter Chart)

From `encounter_chart[]` in `fmain.c`:

| ID | Name | HP | Arms | Clever | Treasure | File |
|----|------|----|------|--------|----------|------|
| 0  | Ogre       | 18 | 2 | 0 | 2 | 6 |
| 1  | Orcs       | 12 | 4 | 1 | 1 | 6 |
| 2  | Wraith     | 16 | 6 | 1 | 4 | 7 |
| 3  | Skeleton   | 8  | 3 | 0 | 3 | 7 |
| 4  | Snake      | 16 | 6 | 1 | 0 | 8 |
| 5  | Salamander | 9  | 3 | 0 | 0 | 7 |
| 6  | Spider     | 10 | 6 | 1 | 0 | 8 |
| 7  | DKnight    | 40 | 7 | 1 | 0 | 8 |
| 8  | Loraii     | 12 | 6 | 1 | 0 | 9 |
| 9  | Necromancer| 50 | 5  | 0 | 0 | 9 |
| 10 | Woodcutter | 4  | 0 | 0 | 0 | 9 (friendly) |

- `arms` selects weapon from `weapon_probs[arms * 4 + rand4()]` (4 possible weapons per enemy type)
- `cleverness 0` → `ATTACK1` (simple pursue), `cleverness 1` → `ATTACK2` (clever pathfinding)
- Archers: `ARCHER1` (clever=0) or `ARCHER2` (clever=1) if weapon has bow bit set (`weapon & 4`)
- `file` maps to `cfiles[]` index for sprite loading (file 6=ogre, 7=ghost/wraith/skeleton/salamander, 8=dknight/spider/snake, 9=necromancer/loraii/woodcutter)
- `mixflag & 2` enables enemy type blending: `race = (encounter_type & 0xfffe) + rand2()` (alternates between even/odd encounter IDs)

---

## Inventory System

`stuff[]` is a `UBYTE[35]` array (one per character: `julstuff`, `philstuff`, `kevstuff`). Each element holds item quantity. Index ranges:

| Range | Macro | Items |
|-------|-------|-------|
| 0–8 | — | Weapons and tools (Dirk=0, Mace=1, Sword=2, Bow=3, Magic Wand=4, Golden Lasso=5, Sea Shell=6, Sun Stone=7, Arrows=8) |
| 9–15 | `MAGICBASE=9` | Magic consumables (Blue Stone, Green Jewel, Glass Vial, Crystal Orb, Bird Totem, Gold Ring, Jade Skull) |
| 16–21 | `KEYBASE=16` | Keys (Gold Key, Green Key, Blue Key, Red Key, Grey Key, White Key) |
| 22–24 | — | Special items (Talisman=22, Rose=23, Fruit=24) |
| 25–30 | `STATBASE=25` | Quest items (Gold Statue, Book, Herb, Writ, Bone, Shard) |
| 31–34 | `GOLDBASE=31` | Gold piles (2gp, 5gp, 10gp, 100gp) |

`stuff[0]` = Dirk count, but `anim_list[0].weapon` holds the *equipped* weapon index (1=Dirk … 5=Wand); the two are separate. On spawn: `stuff[0] = anim_list[0].weapon = 1` (equip Dirk, count 1).

`set_options()` refreshes all menu enable/disable flags from `stuff[]`.

**Item display screen** (ITEMS menu, hit=5): renders all items from `seq_list[OBJECTS]` using `inv_list[]` layout metadata (xoff, yoff, ydelta, img_off, img_height, maxshown).

**`inv_item` struct**:
```c
struct inv_item {
    UBYTE image_number;        // sprite index in OBJECTS sequence
    UBYTE xoff, yoff;          // position on inventory screen
    UBYTE ydelta;              // y-spacing for stacked items
    UBYTE img_off, img_height; // which rows of the sprite to blit
    UBYTE maxshown;            // max displayed on-screen
    char *name;
};
```

### Item catalog: effects, type, and mechanics

Every item is classified as **weapon**, **passive**, **active-use**, **magic** (consumable), or **quest** (plot-gate). Source references are to `fmain.c` and `fmain2.c`.

#### Weapons (stuff 0–4)

Equipped via USE menu (hit 0–4) → sets `anim_list[0].weapon = hit + 1`. Only one weapon active at a time.

| Index | Name | weapon val | Type | Melee/Ranged | Notes |
|-------|------|-----------|------|--------------|-------|
| 0 | Dirk | 1 | Weapon | Melee | Starting weapon for all brothers. Reach = `wt + bitrand(2)` × 2 pixels |
| 1 | Mace | 2 | Weapon | Melee | Same melee mechanics as Dirk, slightly longer base reach |
| 2 | Sword | 3 | Weapon | Melee | Best melee weapon. Melee hit range = `wt + bitrand(2)` × 2, wt derived from weapon index |
| 3 | Bow | 4 | Weapon | Ranged | Uses arrows (stuff[8]). On SHOOT3 state: decrements stuff[8], spawns missile_type=1 (arrow), speed=3 |
| 4 | Magic Wand | 5 | Weapon | Ranged | Fires magic bolts (missile_type=2, speed=5). Does **not** consume arrows. Effect sound: 5 (1800+rand256). On SHOOT3 state, uses `diroffs[d+8]` animation but does NOT fire again (only SHOOT1 fires wand) |

**Melee damage**: attacker weapon value `wt` (+ `bitrand(2)`) is subtracted from target `vitality` in `dohit()`. Hero bonus: `bv = brave/20 + 5` (capped at 15); enemies: `bv = 2 + rand4()`. Hit detection is max(|dx|,|dy|) < bv within weapon reach.

**Ranged damage**: arrow/bolt hits deal `rand8() + 4` damage. Arrows blocked by terrain types 1 (wall) and 15 (furniture). Missiles travel max 40 ticks then expire.

**Cannot damage Necromancer** (race 9) with weapons 1–3 (melee). Requires Bow or Wand. Also cannot damage the witch (race 0x89) with melee unless Sun Stone is held (stuff[7] > 0). `dohit()` speaks(58) "Your weapon has no effect!" and returns without damage in these cases.

**Immune races**: race 0x8a and 0x8b (spectre/ghost setfigs) cannot be damaged at all — `dohit()` returns immediately.

#### Arrows (stuff 8)

| Index | Name | Type | Notes |
|-------|------|------|-------|
| 8 | Arrows | Ammo (consumable) | Consumed 1 per Bow shot. "No Arrows!" message if stuff[8]==0 and bow equipped. Quiver pickups (ob_id QUIVER=11) grant 10 arrows via `stuff[8] += stuff[ARROWBASE] * 10` on pickup. Can be bought from shopkeeper (10 arrows for 10 gold) |

#### Tools (stuff 5–7)

| Index | Name | Type | USE action | Passive effect |
|-------|------|------|------------|----------------|
| 5 | Golden Lasso | Passive | USE menu hit=5: no effect (falls through) | **Enables swan riding**: when near bird carrier (actor_file==11), proximity detected, and stuff[5] > 0 → sets `riding=11`. Without lasso, swan cannot be mounted. Obtained from killing the witch (race 0x89) (`leave_item(i, 27)` on death) |
| 6 | Sea Shell | Active-use | USE menu hit=6: calls `get_turtle()` — summons turtle carrier near hero on water terrain (px_to_im==5). Blocked if hero is within coordinates (11194 < x < 21373, 10205 < y < 16208). | Talking to turtle when shell already owned: speak(57). Obtained by first talking to turtle: speak(56), stuff[6]=1 |
| 7 | Sun Stone | Active-use + Passive | USE menu hit=8: if `witchflag` is set, speaks(60) to communicate with witch. | **Combat passive**: allows melee weapons (1–3) to damage the witch (race 0x89) (without it, melee bounces with speak(58)). **Stone Ring teleportation**: from MAGIC menu hit=5, if hero is on a stone ring tile (hero_sector==144, centered in tile), teleports hero to next stone ring in `stone_list[]` offset by `facing + 1` (wraps at 11). Consumes 1 charge like other magic items |

`stone_list[]` contains 11 pairs of (x_sector, y_sector) coordinates:
```
{54,43}, {71,77}, {78,102}, {66,121}, {12,85}, {79,40}, {107,38}, {73,21}, {12,26}, {26,53}, {84,60}
```

#### Magic consumables (stuff 9–15, MAGICBASE=9)

Accessed via MAGIC menu (hit 5–11 maps to stuff[9–15]). Each use **decrements** the item count by 1 (`--stuff[4 + hit]`). If count reaches 0, the menu option is disabled. Blocked if `extn->v3 == 9` (astral plane, speak(59)).

| Index | Name | MAGIC hit | Timer/Mechanic | Duration/Value | Effect |
|-------|------|-----------|----------------|----------------|--------|
| 9 | Blue Stone | 5 | Stone Ring transport | — | `hero_sector==144` and hero centered in tile → teleports to next stone ring in `stone_list[]` (offset by facing direction + 1, wraps mod 11). If not on a stone ring, returns without consuming. Only works in overworld (region < 8); blocked in underworld unless `cheat1` |
| 10 | Green Jewel | 6 | `light_timer += 760` | ~760 game ticks | **Illumination**: `day_fade()` adds +200 to red channel parameter in `fade_page()`, making night nearly as bright as day. Does not modify `lightlevel` itself. Palette color 31 unaffected. Timer decrements each main-loop tick |
| 11 | Glass Vial | 7 | Heal | Instant | Restores `rand8() + 4` vitality (4–11 HP). Capped at max vitality `15 + brave/4`. Prints "That feels a lot better!" if not already at max |
| 12 | Crystal Orb | 8 | `secret_timer += 360` | ~360 game ticks | **Reveal secrets**: in region 9 (underworld/dungeons), changes palette color 31 from 0x0445 (dark) to 0x00F0 (bright green), revealing hidden passages. Timer decrements each tick |
| 13 | Bird Totem | 9 | World map | Instant | **Minimap display**: draws the world map (`bigdraw(map_x, map_y)`) with hero position marked by "+" at computed pixel offset. Only works in overworld (`region_num < 8`); blocked while `riding > 1`. Sets `viewstatus=1`, waits for keypress |
| 14 | Gold Ring | 10 | `freeze_timer += 100` | ~100 game ticks | **Time stop**: all non-hero figures skip movement updates (`freeze_timer && i > 0` → goto statc). Enemies cannot attack, missiles don't fire. `daynight` clock pauses. Hero can loot frozen enemies' bodies. Also prevents melee hit checks for non-hero figures |
| 15 | Jade Skull | 11 | Mass kill | Instant | **Death spell**: iterates all figures `i=1..anix-1`; if enemy type with vitality > 0, race < 7 → sets vitality=0, calls `checkdead(i, 0)`, decrements `brave` by 1 per kill. Then triggers battle aftermath event(34) if battleflag set |

#### Keys (stuff 16–21, KEYBASE=16)

Accessed via KEYS submenu (USE → Key → color selection). Each use probes 9 positions around hero (8 compass directions + center, 16px radius) via `doorfind(x, y, hit+1)`. `doorfind` checks terrain type 15, grid-aligns to find the door tile, matches it against `open_list` with the given `keytype`. On success: tiles replaced in `sector_mem` (door is now passable), `"It opened."` printed, key consumed (`stuff[hit + KEYBASE]--`). Hero must then walk forward to cross the threshold (Phase 2 scan fires the teleport). If no matching door found at any of the 9 probes: `"% tried a [Key Name] but it didn't fit."`

| Index | Name | KEYS hit | `doorfind` keytype |
|-------|------|----------|-------------------|
| 16 | Gold Key | 0 | 1 |
| 17 | Green Key | 1 | 2 |
| 18 | Blue Key | 2 | 3 |
| 19 | Red Key | 3 | 4 |
| 20 | Grey Key | 4 | 5 |
| 21 | White Key | 5 | 6 |

#### Special items (stuff 22–24)

| Index | Name | Type | Effect |
|-------|------|------|--------|
| 22 | Talisman | Quest (win-game) | **Passive, pickup-triggered**: when any item is picked up and stuff[22] > 0, sets `quitflag = TRUE`, `viewstatus = 2`, calls `map_message()` and `win_colors()` → triggers the win/ending sequence. Obtained from Necromancer (race 9) on death (`leave_item(i, 139)`) |
| 23 | Rose | Passive | **Fire immunity**: in the volcanic/fiery death zone (8802 < map_x < 13562, 24744 < map_y < 29544), if hero has stuff[23] > 0, sets `environ=0` (safe) instead of taking fire damage. Without it, environ > 2 → vitality--, environ > 15 → instant death |
| 24 | Fruit | Passive (auto-use) | **Auto-consumed food**: when hero is in a safe zone, no enemies active, `environ==0`, `safe_flag==0`, and `hunger > 30`: consumes one fruit (`stuff[24]--`), reduces hunger by 30, triggers event(37). Also picked up from MEAL objects (ob_id 148): if hunger < 15 → stored as stuff[24], else eaten immediately via `eat(30)` |

#### Quest items (stuff 25–30, STATBASE=25)

| Index | Name | Type | Effect |
|-------|------|------|--------|
| 25 | Gold Statue | Quest-gate | **Desert access**: 5 statues required (`stuff[STATBASE] >= 5`) to enter the desert region (region 4). Door type `DESERT` blocks entry if count < 5. Also blocks map data loading: desert map tiles are overwritten to impassable (tile 254) when count < 5. 6 statues exist in the world: seahold, ogre den, octal room, sorceress (revealed by talking), priest (revealed with Writ) |
| 26 | Book | Quest (inert) | **No active use**. USE menu position 9 ("Book") is hardcoded disabled (`enabled[9] = 0`). GIVE menu position 6 ("Book") also hardcoded disabled (`enabled[6] = 8`). Inventory display only. May be related to witch/NPC dialogue triggers not fully implemented in source |
| 27 | Herb | Quest (inert) | **No coded effect**. stuff[27] is never read in game logic. Display-only inventory item |
| 28 | Writ | Quest (NPC trigger) | **Priest dialogue gate**: when talking to the Priest (setfig type 1) with stuff[28] > 0, and `ob_listg[10].ob_stat == 0`: speaks(39) and sets `ob_listg[10].ob_stat = 1`, revealing a Gold Statue location. If statue already revealed: speaks(19). GIVE menu shows Writ status but has no GIVE action in code |
| 29 | Bone | Giveable (quest) | **Give to Spectre**: via GIVE menu hit=8. If nearest person is race 0x8a (spectre): speaks(48), consumes bone (`stuff[29] = 0`), `leave_item(nearest, 140)` — the spectre leaves behind item ob_id 140 (Shard). If target is not spectre: speaks(21) "Wrong person" |
| 30 | Shard | Passive | **Phase through mountains**: when hero walks into terrain type 12 (mountain3) and stuff[30] > 0, terrain collision is bypassed (`goto newloc` instead of blocking). Allows accessing otherwise impassable mountain areas |

#### Gold (stuff 31–34, GOLDBASE=31)

Gold items aren't stored in `stuff[]` — they are added directly to the `wealth` variable on pickup:

| Index | Name | maxshown | Wealth added |
|-------|------|----------|-------------|
| 31 | 2 Gold Pieces | 2 | +2 |
| 32 | 5 Gold Pieces | 5 | +5 |
| 33 | 10 Gold Pieces | 10 | +10 |
| 34 | 100 Gold Pieces | 100 | +100 |

Gold is spent via the BUY menu (shopkeeper, race 0x88) and GIVE menu (give 2gp to beggars, chance to increase `kind`).

### World object types (`obytes` enum)

Maps `ob_id` byte values in `ob_listg[]`/`ob_listN[]` to world-placed object identifiers. The `itrans[]` table translates ob_id → stuff[] index on pickup.

| ob_id | Constant | stuff index | Notes |
|-------|----------|-------------|-------|
| 8 | — | 2 (Sword) | |
| 9 | — | 1 (Mace) | |
| 10 | — | 3 (Bow) | |
| 11 | QUIVER | 35 (→ arrows ×10) | Grants 10 arrows |
| 12 | — | 0 (Dirk) | |
| 13 | MONEY | — | +50 gold (hardcoded in pickup) |
| 14 | URN | — | Container (random treasure) |
| 15 | CHEST | — | Container (random treasure) |
| 16 | SACKS | — | Container (random treasure) |
| 17 | G_RING | 14 (Gold Ring) | |
| 18 | B_STONE | 9 (Blue Stone) | |
| 19 | G_JEWEL | 10 (Green Jewel) | |
| 20 | SCRAP | — | Scrap of paper; triggers event(17) + regional event (18 or 19) |
| 21 | C_ORB | 12 (Crystal Orb) | |
| 22 | VIAL | 11 (Glass Vial) | |
| 23 | B_TOTEM | 13 (Bird Totem) | |
| 24 | J_SKULL | 15 (Jade Skull) | |
| 25 | GOLD_KEY | 16 (Gold Key) | |
| 26 | GREY_KEY | 20 (Grey Key) | |
| 27 | — | 5 (Golden Lasso) | Dropped by the witch (race 0x89) on death |
| 28 | — | — | Dead brother's bones; absorbs dead brother's inventory |
| 31 | FOOTSTOOL | — | Blocks pickup (break) |
| 102 | TURTLE | — | Turtle eggs; blocks pickup (break) |
| 114 | BLUE_KEY | 18 (Blue Key) | |
| 136 | — | 27 (Herb) | |
| 137 | — | 28 (Writ) | |
| 138 | — | 29 (Bone) | |
| 139 | — | 22 (Talisman) | Dropped by Necromancer on death |
| 140 | — | 30 (Shard) | Left by spectre when given Bone |
| 145 | M_WAND | 4 (Magic Wand) | |
| 146 | MEAL | — | Food; if hunger<15 stored as Fruit, else eaten (−30 hunger) |
| 147 | ROSE | 23 (Rose) | |
| 148 | FRUIT | 24 (Fruit) | |
| 149 | STATUE | 25 (Gold Statue) | |
| 150 | BOOK | 26 (Book) | |
| 151 | SHELL | 6 (Sea Shell) | |
| 153 | GREEN_KEY | 17 (Green Key) | |
| 154 | WHITE_KEY | 21 (White Key) | |
| 155 | — | 7 (Sun Stone) | |
| 242 | RED_KEY | 19 (Red Key) | |

### Container treasure generation

Containers (Chest, Urn, Sacks) use `rand4()` (0–3) to determine loot:

| Roll | Result |
|------|--------|
| 0 | Nothing |
| 1 | 1 random magic/key item (`rand8() + 8`; if index 8 → arrows instead) |
| 2 | 2 different random items (same pool; if first is index 8 → 100gp instead) |
| 3 | 3 copies of one item (same pool; if index 8 → 3 random keys instead) |

### Random enemy loot

On searching a dead enemy body, `encounter_chart[race].treasure` selects a loot table row from `treasure_probs[]` (8 entries per row × `rand8()` column):

| treasure | Loot table row | Contents |
|----------|---------------|----------|
| 0 | all zeros | No treasure drops |
| 1 | Blue Stone, Vial, Totem, 2×gold(5gp), 2×keys, gold(10gp) | Common magic/key |
| 2 | Orb, Ring, 3×Grey Key, gold(5gp/10gp/2gp) | Mid-tier magic/keys |
| 3 | Jewel×2, Gold Key, Skull, Jewel, keys (Green/Blue/Red) | Rare magic/keys |
| 4 | Jade Skull, White Key, 6×nothing | Very rare; only Jade Skull and White Key |

Enemy bodies also drop their equipped weapon (if weapon > 0): `stuff[weapon-1]++`. Auto-equips if better than current (`weapon > anim_list[0].weapon`). Bows additionally grant `rand8() + 2` arrows.

### BUY menu (shopkeeper race 0x88)

Items purchasable via `jtrans[]` cost table:

| BUY hit | Item | stuff index | Cost (gold) |
|---------|------|-------------|-------------|
| 5 | Food | 0 (eat) | 3 |
| 6 | Arrows | 8 (+10) | 10 |
| 7 | Glass Vial | 11 | 15 |
| 8 | Mace | 1 | 30 |
| 9 | Sword | 2 | 45 |
| 10 | Bow | 3 | 75 |
| 11 | Bird Totem | 13 | 20 |

Food (hit=5) is special: calls `eat(50)` directly (−50 hunger) + event(22), not stored in inventory.

---

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

## `setmood()` — Music State Machine

Called on every state change, on player death, region cross, and periodically via `(daynight & 7) == 0`. Selects the active 4-track group from `track[]` (28 track pointers loaded by `read_score()`):

```c
// Priority order (highest first):
if (hero vitality == 0)           → group 6  (tracks 24–27) death/game-over (no loop)
else if (hero in palace zone*)    → group 4  (tracks 16–19) palace
else if (battleflag)              → group 1  (tracks 4–7)   battle
else if (region_num > 7)          → group 5  (tracks 20–23) indoor/dungeon
  (region 9 uses new_wave[10]=0x0307; others use 0x0100)
else if (lightlevel > 120)        → group 0  (tracks 0–3)   outdoor daytime
else                              → group 2  (tracks 8–11)  outdoor nighttime
```

\* Palace zone: `0x2400 < hero_x < 0x3100` AND `0x8200 < hero_y < 0x8a00`

Music is gated by `menus[GAME].enabled[6] & 1`. If music is off: `stopscore()`. `now=TRUE` → `playscore()` (restart from beginning); `now=FALSE` → `setscore()` (crossfade without restart). Setmood also polled every 7 daynight ticks via `(daynight & 7) == 0`.

---

## `game/songs` — Music Score Data (5,984 bytes)

Loaded by `read_score()` in `fmain2.c`. Holds up to 28 sequencer tracks
organised as **7 song groups × 4 Paula voices**. The original stores them in a
simple length-prefixed format; no SMUS/IFF wrapper is used.

The active group is selected at runtime by `setmood()` in `fmain.c` based on
game state.  The Rust parser lives in `src/game/songs.rs`.

---

### File Layout

Each track is stored as:

| Field | Size | Description |
|-------|------|-------------|
| `packlen` | 4 bytes (big-endian `i32`) | Number of 16-bit words in this track's event stream |
| event bytes | `packlen × 2` bytes | Sequence of `(command, value)` byte pairs |

Tracks are read sequentially.  Loading stops when the cumulative byte count
reaches the 5,900-byte `scoremem` limit (`SCORE_SZ` in `fmain.c`).
All 28 tracks fit within that limit (5,872 bytes of event data + 112 bytes
of headers = 5,984 bytes total, matching the file size exactly).

---

### Event Encoding (from `gdriver.asm` → `_vblank_server` → `newnote`)

Every event is exactly two bytes `(command, value)`:

| Command byte | Meaning |
|---|---|
| 0 – 127 | **Note** — pitch index into `PTABLE` (78 entries; see layout below) |
| 128 (0x80) | **Rest** — silence for the given duration |
| 129 (0x81) | **Set Instrument** — `value & 0x0f` selects a slot from the `new_wave[]` instrument table |
| 144 (0x90) | **Set Tempo** — `value` is written directly to the tempo register (default 150) |
| 255 (0xFF) | **End Track** — `value ≠ 0` loops back to the start; `value = 0` stops the voice |
| other (bit 7 set) | Ignored (the ASM falls through to `newnote`) |

The **duration** of notes and rests comes from the `value` byte (bits 6–7
masked off), used as an index into `NOTE_DURATIONS[0..63]` — 64 tick counts
covering 8 note-length groups (4/4, 6/8, 3/4, 7/8, 5/4, 3/4 alt, 9/8, and a
duplicate 4/4).

The **pitch** byte (0–77) indexes into `PTABLE`, which stores
`(period, wave_offset)` pairs.  `period` is an Amiga Paula hardware period
register value.  The correct frequency formula is:
```
frequency = AMIGA_CLOCK_NTSC / (wave_len × period)
           = 3,579,545 / (wave_len × period)
```
where `wave_len = (32 - wave_offset) × 2` bytes.
`wave_offset` is a 16-bit–word offset into the 128-byte waveform in `wavmem`
(from `v6`) that selects which portion Paula loops, halving the loop length
each step to raise the pitch one octave.

`PTABLE` layout (78 entries across 7 ASM rows):

| Pitches | Entries | wave_offset | wave_len | Notes      | Frequency range |
|---------|---------|-------------|----------|------------|-----------------|
| 0–5     | 6       | 0           | 64       | D#1–G#1    | 38.9–51.9 Hz    |
| 6–17    | 12      | 0           | 64       | A1–G#2     | 55.0–103.8 Hz   |
| 18–29   | 12      | 0           | 64       | A2–G#3     | 110.0–207.7 Hz  |
| 30–41   | 12      | 16          | 32       | A3–G#4     | 220.0–415.3 Hz  |
| 42–53   | 12      | 24          | 16       | A4–G#5     | 440.0–830.6 Hz  |
| 54–65   | 12      | 28          | 8        | A5–G#6     | 880.0–1661.2 Hz |
| 66–77   | 12      | 28          | 8        | A6–G#7     | 1760.0–3322.4 Hz|

Rows 1–6 (pitches 6–77) each start at A and cover a full chromatic octave.
Row 0 (pitches 0–5) is a partial row covering only D#1 through G#1.

---

### Timing

The music sequencer runs in the **VBlank interrupt** at 60 Hz (NTSC).
Each VBlank, the 32-bit `timeclock` counter is incremented by the current
`tempo` value.  A note plays until `timeclock` reaches `event_start + notevals[duration_idx]`.
At the default tempo of 150 this gives **9,000 timeclock units per second**.

---

### Song Groups

Each group occupies four consecutive tracks (one per Amiga Paula voice).
Voice 0 carries the primary melody; voices 1–3 carry harmony/bass/rhythm.
`setmood()` in `fmain.c` chooses the active group based on game state.

| Group | Tracks | Context | Loop | ~Length |
|-------|--------|---------|------|---------|
| 0 | 0 – 3 | **Outdoor daytime** (`lightlevel > 120`) | Yes | ~57–84 s |
| 1 | 4 – 7 | **Battle** (`battleflag` set) | Yes | ~24–36 s |
| 2 | 8 – 11 | **Outdoor nighttime** (low light, outdoors) | Yes | ~65–96 s |
| 3 | 12 – 15 | **Intro sequence** (hardcoded `playscore` call) | No | ~54–75 s |
| 4 | 16 – 19 | **Palace zone** (specific hero map coordinates) | Yes | ~32–54 s |
| 5 | 20 – 23 | **Indoor / dungeon** (`region_num > 7`) | Yes | ~61–78 s |
| 6 | 24 – 27 | **Death / game over** (hero vitality = 0) | No | ~25–34 s |

Group 5 (indoor/dungeon) is also used for **caves** (region 9), but with a
different instrument assigned to slot 10: region 9 (caves) sets
`new_wave[10] = 0x0307`, all other indoor regions use `new_wave[10] = 0x0100`.
The track data is identical — only the timbre of one voice changes.

---

### Parsed Track Statistics

Decoded from the actual `game/songs` file (all 28 tracks loaded, 5,872 bytes
of score data):

```
 #  Group/Context       V   bytes  notes  rests  instr  tempo  loop   ~sec
 0  outdoor-daytime     0     394    193      1      1      1     Y    57.0
 1  outdoor-daytime     1     352    172      2      1      0     Y    83.6
 2  outdoor-daytime     2     130     50     13      1      0     Y    83.6
 3  outdoor-daytime     3     224    108      2      1      0     Y    83.6
 4  battle              0     270    111     21      1      1     Y    24.4
 5  battle              1     388    192      0      1      0     Y    35.8
 6  battle              2      52     24      0      1      0     Y    35.8
 7  battle              3     196     96      0      1      0     Y    35.8
 8  outdoor-night       0     400    192      5      1      1     Y    65.2
 9  outdoor-night       1     340    167      1      1      0     Y    95.6
10  outdoor-night       2     160     78      0      1      0     Y    95.6
11  outdoor-night       3     132     64      0      1      0     Y    95.6
12  intro               0     294    143      1      1      1     N    53.8
13  intro               1     178     87      0      1      0     N    74.7
14  intro               2     180     86      2      1      0     N    74.7
15  intro               3     102     49      0      1      0     N    74.7
16  palace              0     292    135      8      1      1     Y    31.6
17  palace              1     146     61     10      1      0     Y    53.8
18  palace              2      88     41      1      1      0     Y    53.8
19  palace              3     148     72      0      1      0     Y    53.8
20  indoor/dungeon      0     338    166      0      1      1     Y    60.7
21  indoor/dungeon      1     236     94     22      1      0     Y    77.7
22  indoor/dungeon      2     250    111     12      1      0     Y    77.7
23  indoor/dungeon      3     372    164     20      1      0     Y    77.7
24  death/game-over     0      48     19      2      1      1     N    24.8
25  death/game-over     1      54     18      7      1      0     N    34.3
26  death/game-over     2      54     18      7      1      0     N    34.3
27  death/game-over     3      54     18      7      1      0     N    34.3
```

Columns: V = Paula voice (0 = primary melody), `instr` = number of
`SetInstrument` events, `tempo` = number of `SetTempo` events,
`~sec` = approximate loop/play time at NTSC 60 Hz with the tempo set by the
first `SetTempo` event in that track.

---

## `game/v6` — Music Voice/Waveform Data (4,628 bytes)

The file is the **music synthesizer data** for the Amiga's four-voice Paula sound chip. It is loaded at startup by `fmain.c` via:

```c
Read(file, wavmem, S_WAVBUF);         // 1,024 bytes at offset 0
Seek(file, S_WAVBUF, OFFSET_CURRENT); // skip 1,024 bytes
Read(file, volmem, S_VOLBUF);         // 2,560 bytes at offset 2,048
```

These two buffers are passed directly to `init_music()` in `gdriver.asm`, which sets up the VBlank interrupt that drives the four-voice music engine.

### Layout

| Offset | Size | Name | Structure | Description |
|--------|------|------|-----------|-------------|
| 0x000 | 1,024 B | `wavmem` (wave buffer) | 8 waveforms × 128 bytes each | Signed 8-bit PCM sample data. Each 128-byte waveform is a periodic shape (sine-like, triangle, etc.) played by the Amiga's Paula DMA channels. The voice engine looks up `wave_num` per-voice and feeds a sub-range of the 128-byte buffer into Paula's `$a0`/`$a4` (pointer/length) registers. The sub-range depends on the current octave: `wave_offset` from PTABLE selects the start (`wave_offset × 4` bytes in) and the loop length (`(32 − wave_offset) × 2` bytes). |
| 0x400 | 1,024 B | *(skipped)* | — | Deliberately skipped by the `Seek(OFFSET_CURRENT)` call. Likely extra waveform data that isn't used by this version of the engine, or reserved space. The hex dump shows it is all zeros. |
| 0x800 | 2,560 B | `volmem` (volume/envelope buffer) | 10 envelopes × 256 bytes each | Amplitude envelope tables. Each byte is a volume level (0–64 in Amiga terms), except that any byte with the MSB set (≥ 0x80) is a **hold sentinel**: the envelope pointer stops advancing and the current volume is frozen until the next note event. Voices index into these via `vol_num` and step through the table byte-by-byte each VBlank tick, creating ADSR-like attack/decay/sustain/release shapes. When a voice advances past the last byte in its envelope, volume is zeroed (silence). |
| 0x1200 | 20 B | *(trailing zeros)* | — | Padding at end of file, unused. |

### Role in the engine

The `gdriver.asm` music engine runs as an Amiga interrupt server on **VBlank** (60 Hz NTSC). Each tick, for each of the 4 voices it:
1. Reads the next note from the score/track
2. Looks up the waveform by index into `wavmem`
3. Loads the PCM pointer and length into Paula's DMA registers
4. Steps through the envelope in `volmem`, writing each volume byte to Paula's `$a8` register

The "v6" name likely refers to **Version 6** of the music voice data file — the `new_wave[]` array in `fmain.c` defines a 12-element default instrument table that maps tracks to waveform/envelope pairs, and there are in-game branches that swap entries depending on whether the player is indoors vs. outdoors (`new_wave[10] = 0x0307` or `0x0100`).

---

## Save / Load Format

`savegame(hit)` writes to a file named `savename[4] = 'A' + hit` (save slots A–?). `svflag=TRUE` = save, `FALSE` = load. `saveload(buf, len)` does the actual read/write.

Sequential layout of the save file:

| # | `saveload()` call | Content | Notes |
|---|---|---|---|
| 1 | `saveload(&map_x, 80)` | Main game-state block | Starts at `map_x`, covers ~40 `short`/`USHORT` fields: map_x/y, hero_x/y, safe_x/y/r, img_x/y, cheat1, riding, flying, wcarry, turtleprox, raftprox, brave, luck, kind, wealth, hunger, fatigue, brother, princess, hero_sector, hero_place, daynight, lightlevel, actor_file, set_file, active_carrier, xtype, leader, secret_timer, light_timer, freeze_timer, cmode, encounter_type, and padding |
| 2 | `saveload(&region_num, 2)` | Active region | `UWORD` |
| 3 | `saveload(&anix, 6)` | Figure count + misc | anix, anix2, mdex |
| 4 | `saveload(anim_list, anix * 22)` | All active `shape` structs | 22 bytes each |
| 5 | `saveload(julstuff, 35)` | Julian's inventory | `UBYTE[35]` |
| 6 | `saveload(philstuff, 35)` | Phillip's inventory | `UBYTE[35]` |
| 7 | `saveload(kevstuff, 35)` | Kevin's inventory | `UBYTE[35]` |
| 8 | `saveload(missile_list, 6*14)` | Active missiles | 6 × `struct missile` |
| 9 | `saveload(extent_list, 2*16)` | Bird & turtle extents | First 2 `struct extent` entries |
| 10 | `saveload(ob_listg, glbobs*8)` | Global persistent objects | `struct object` = 8 bytes |
| 11 | `saveload(mapobs, 20)` | Per-region object counts | `short[10]` |
| 12 | `saveload(dstobs, 20)` | Per-region object offsets | `short[10]` |
| 13–22 | `saveload(ob_table[i], mapobs[i]*8)` | Per-region object lists | For each of 10 regions |

`struct shape` layout (22 bytes):
```
abs_x   : u16  (0)
abs_y   : u16  (2)
rel_x   : u16  (4)
rel_y   : u16  (6)
type    : i8   (8)
race    : u8   (9)
index   : i8   (10)
visible : i8   (11)
weapon  : i8   (12)
environ : i8   (13)
goal    : i8   (14)
tactic  : i8   (15)
state   : i8   (16)
facing  : i8   (17)
vitality: i16  (18)
vel_x   : i8   (20)
vel_y   : i8   (21)
```

`struct missile` layout:
```
abs_x          : u16
abs_y          : u16
missile_type   : i8   (0=none, 1=arrow, 2=fireball, 3=spent)
time_of_flight : i8
speed          : i8
direction      : i8
archer         : i8   (ID of firing figure)
```

`struct object` layout (8 bytes):
```
xc     : u16
yc     : u16
ob_id  : i8
ob_stat: i8
(2 bytes padding)
```

---

## Sound Effects (`game/samples`)

Loaded from **ADF block 920**, reading **11 blocks** (5,632 bytes, `SAMPLE_SZ`) into `sample_mem` via `read_sample()` in `fmain.c`.

Six IFF-style length-prefixed sound effects packed sequentially:

```
for each of 6 samples:
  [4 bytes big-endian] length N
  [N bytes]            signed 8-bit PCM mono sample data
```

`effect(num, speed)` calls `playsample(sample[num], sample_size[num] / 2, speed)`.
- `sample[num]` is a pointer into `sample_mem` past the length prefix
- `sample_size[num] / 2` is the length in 16-bit words (Paula DMA uses word count)
- `speed` is an Amiga Paula **period register** value (higher = slower, lower pitch); the `rand` jitter creates pitch variation per hit

| Index | Trigger event | Speed base | Jitter |
|-------|--------------|------------|--------|
| 0 | Hero hit by melee | 800 | +bitrand(511) |
| 1 | Weapon near-miss | 150 | +rand256() |
| 2 | Arrow/bolt hits player | 500 | +rand64() |
| 3 | Monster hit by melee | 400 | +rand256() |
| 4 | Arrow hits a target | 400 | +rand256() |
| 5 | Magic/fireball hit | 3200 | +bitrand(511) |

---

## Sprite / Shape File Layout (ADF)

All animated character sprites are stored in **ADF `game/image`** (the same 880 KB floppy image used for map data). Sprite data is loaded by `read_shapes()` / `load_track_range()` in `fmain2.c`.

### `cfiles[]` — Sprite File Registry

```c
struct cfile_info {
    UBYTE width;     // sprite width in 16-pixel interleaved words
    UBYTE height;    // sprite height in pixels
    UBYTE count;     // number of animation frames
    UBYTE numblocks; // ADF 512-byte blocks to read
    UBYTE seq_num;   // seq_list[] slot (PHIL=0, OBJECTS=1, ENEMY=2, RAFT=3, SETFIG=4, CARRIER=5, DRAGON=6)
    USHORT file_id;  // starting ADF block number
};
```

**Frame byte size** = `width × height × 2` (one row per word, one bitplane).
**ADF data per file** = `frame_bytes × count × 5` (5 bitplanes only) = `numblocks × 512` bytes.
`nextshape` advances by `frame_bytes × count × 5`; `seq_list[slot].maskloc` points to the next `frame_bytes × count` bytes of the pre-allocated `shape_mem` buffer.

**The mask is not stored on disk.** It is computed at runtime by `make_mask()` (`fsubs.asm:1614`):
for each word position across all frames, it ORs all plane bits then inverts:
`mask_word = NOT(plane0 AND plane1 AND plane2 AND plane3 AND plane4)`
A pixel is **transparent** when all 5 plane bits are set (color index 31). All other color indices are opaque.
Comment in `fsubs.asm:1617`: "assumes color 31 = transparent".

| cfile# | ADF block | Blocks | W×H | Frames | Slot | Contents |
|--------|-----------|--------|-----|--------|------|----------|
| 0 | 1376 | 42 | 1×32 | 67 | PHIL | Julian (all directions + fight) |
| 1 | 1418 | 42 | 1×32 | 67 | PHIL | Phillip |
| 2 | 1460 | 42 | 1×32 | 67 | PHIL | Kevin |
| 3 | 1312 | 36 | 1×16 | 116 | OBJECTS | World items / loot objects |
| 4 | 1348 | 3  | 2×32 | 2   | RAFT | Raft (two frames) |
| 5 | 1351 | 20 | 2×32 | 16  | CARRIER | Turtle |
| 6 | 960  | 40 | 1×32 | 64  | ENEMY | Ogre / Orc |
| 7 | 1080 | 40 | 1×32 | 64  | ENEMY | Ghost / Wraith / Skeleton / Salamander |
| 8 | 1000 | 40 | 1×32 | 64  | ENEMY | DKnight / Spider |
| 9 | 1040 | 40 | 1×32 | 64  | ENEMY | Necromancer / Loraii / Farmer |
| 10 | 1160 | 12 | 3×40 | 5  | DRAGON | Dragon |
| 11 | 1120 | 40 | 4×64 | 8  | CARRIER | Bird |
| 12 | 1376 | 40 | 1×32 | 64 | ENEMY | Snake / Salamander (shares ADF block with Julian) |
| 13 | 936  | 5  | 1×32 | 8  | SETFIG | Wizard / Priest |
| 14 | 931  | 5  | 1×32 | 8  | SETFIG | Guards / Princess / King / Noble / Sorceress |
| 15 | 941  | 5  | 1×32 | 8  | SETFIG | Bartender |
| 16 | 946  | 5  | 1×32 | 8  | SETFIG | Witch / Spectre / Ghost |
| 17 | 951  | 5  | 1×32 | 8  | SETFIG | Ranger / Beggar |

### Bitplane layout

Each animation frame is stored in **plane-major format**: all rows of one plane are stored together, then all rows of the next plane. For a `1×32` (one word × 32 rows, 5 planes) frame:
```
plane 0, row  0: 2 bytes
plane 0, row  1: 2 bytes
...
plane 0, row 31: 2 bytes  (64 bytes total for plane 0)
plane 1, row  0: 2 bytes
...
plane 4, row 31: 2 bytes  (total: 5 × 64 = 320 bytes per frame)
```
Offset formula: plane P, row R of frame F = `data[F*320 + P*64 + R*2]`.

The mask is not stored after the frames — see the note above about `make_mask()`.

### `statelist[]` — Animation Frame Index

`statelist[87]` maps animation state+frame indices to `{figure_frame, weapon_frame, wpn_x, wpn_y}`:
- Frames 0–7: south walk cycle
- Frames 8–15: west walk cycle
- Frames 16–23: north walk cycle
- Frames 24–31: east walk cycle
- Frames 32–43: south fight (9 transition states + 2 death/special)
- Frames 44–55: west fight
- Frames 56–67: north fight
- Frames 68–79: east fight
- Frames 80–82: death sequence
- Frames 83–84: sinking sequence / oscillation
- Frame 86: asleep

`trans_list[9]` is the combat animation transition table: each state maps to the next state for each of the 4 compass directions.

**`setfig_table[]`** maps `setfig_type` (0–13) → `{cfile_entry, image_base, can_talk}`:

| Type | Name | cfile | Base frame | Can talk |
|------|------|-------|------------|----------|
| 0 | Wizard    | 13 | 0 | Yes |
| 1 | Priest    | 13 | 4 | Yes |
| 2 | Guard     | 14 | 0 | No |
| 3 | Guard (back) | 14 | 1 | No |
| 4 | Princess  | 14 | 2 | No |
| 5 | King      | 14 | 4 | Yes |
| 6 | Noble     | 14 | 6 | No |
| 7 | Sorceress | 14 | 7 | No |
| 8 | Bartender | 15 | 0 | No |
| 9 | Witch     | 16 | 0 | No |
| 10 | Spectre  | 16 | 6 | No |
| 11 | Ghost    | 16 | 7 | No |
| 12 | Ranger   | 17 | 0 | Yes |
| 13 | Beggar   | 17 | 4 | Yes |

### Weapon rendering and two-pass sprite masking (`fmain.c:2880–3184`)

The rendering loop uses a **two-pass** system (`pass` 0 and 1) per entity, controlled by `passmode`. Each pass draws either the body or the weapon, then applies depth-masking independently using that pass's bounding box.

**Weapon draw order (`passmode`)**:

For most weapons, `(facing - 2) & 4` selects whether the weapon draws before the body (behind) or after (in front):

| Original facing | Direction | `(f-2)&4` | Weapon draws… |
|-----------------|-----------|-----------|---------------|
| 0 | NW | 4 | Behind body |
| 1 | N  | 4 | Behind body |
| 2 | NE | 0 | In front |
| 3 | E  | 0 | In front |
| 4 | SE | 0 | In front |
| 5 | S  | 0 | In front |
| 6 | SW | 4 | Behind body |
| 7 | W  | 4 | Behind body |

(Rust facing convention is rotated: 0=N, 1=NE, …, 7=NW; the same set of four directions map to `weapon_behind`.)

Bow (weapon type 4) has an inverted test: `(facing & 4) == 0` reverses the draw order.

**Masking applies to BOTH passes independently.**  Each pass computes its own bounding box:

- **Pass 0 (body)**: `xstart = rel_x`, `ystart = rel_y`, size 16×32, `ground = ystart + 32`.
- **Pass 1 (weapon)**: `xstart = rel_x + wpn_x`, `ystart = rel_y + wpn_y`, size 16×16, same `ground` (body feet).

Both passes run through the same `maskit()` loop (`fmain.c:3134–3184`), checking tile mask types and applying `shadow_mem` bitmasks. This ensures the weapon is hidden behind foreground terrain (buildings, trees) even when its pixels extend beyond the body sprite's bounding box.

**Key invariant**: the weapon pass uses the **body's ground line** (`ystart + 32`, not the weapon's own bottom edge) for depth comparison. This is because depth sorting is always relative to the character's feet position on the ground plane.

---

## NPC Behavior (Goal/Tactic System)

NPC AI runs on `anim_list[3..anix-1]` (figures beyond the 3 reserved for player, raft, setfig).

**Goal modes** (`an->goal`):
- `USER(0)` — player-controlled
- `ATTACK1(1)` — dumb pursue
- `ATTACK2(2)` — clever pursue (uses `set_course` smart seek, xdir/ydir filtering)
- `ARCHER1(3)`, `ARCHER2(4)` — ranged attack styles
- `FLEE(5)` — run directly away from hero
- `STAND(6)` — face hero but don't move
- `DEATH(7)` — dying state
- `WAIT(8)` — wait to speak
- `FOLLOWER(9)` — follow another figure
- `CONFUSED(10)` — random movement

**Tactic modes** (`an->tactic`, sub-goal within goal):
- `FRUST(0)` — blocked, try something else
- `PURSUE(1)` — move toward hero
- `FOLLOW(2)` — go to another figure
- `BUMBLE_SEEK(3)` — wander toward target
- `RANDOM(4)` — random movement
- `BACKUP(5)` — move opposite to current direction
- `EVADE(6)` — move 90° from hero
- `HIDE(7)` — seek cover
- `SHOOT(8)` — fire ranged attack
- `SHOOTFRUST(9)` — ranged blocked
- `EGG_SEEK(10)` — snakes seeking turtle eggs
- `DOOR_SEEK(11)` / `DOOR_LET(12)` — DKnight blocking/permitting door passage

### `do_tactic(i, tactic)` — Probabilistic tactic execution

Source: `fmain2.c:2075–2135`.

Called from the main AI decision loop. Rolls `r = !(rand() & 7)` (~12.5% / 1-in-8 chance) to gate direction recalculation. When `an->goal == ATTACK2`, upgrades to `r = !(rand() & 3)` (~25% / 1-in-4). This means NPCs only re-aim toward the hero occasionally — the rest of the time they keep walking in their current direction. This is the single biggest contributor to the "drunken walk" feel of the original NPC movement.

| Tactic | On trigger (r==true) | Mode | Notes |
|--------|---------------------|------|-------|
| PURSUE | `set_course(i, hero_x, hero_y, 0)` | Smart seek | Default melee chase |
| SHOOT | 50% chance fire arrow if in line-of-sight band; else `set_course(mode 0)` | 0 or 5 | Range check: xd<8 or yd<8 or diagonal band ±5–7px |
| RANDOM | `facing = rand() & 7`; `state = WALKING` | N/A | No `set_course` call |
| BUMBLE_SEEK | `set_course(i, hero_x, hero_y, 4)` | Diagonal-ok | Ignores 2:1 axis suppression |
| BACKUP | `set_course(i, hero_x, hero_y, 3)` | Flee | Reverses direction vectors |
| FOLLOW | `set_course(i, leader_x, leader_y+20, 0)` | Smart seek | Targets `leader` NPC; self-follow → RANDOM |
| EVADE | `set_course(i, other_npc_x, other_npc_y+20, 2)` | Deviation | BUG: target NPC index is `i+i` (doubles); mode 2 adds ±2 deviation |
| EGG_SEEK | `set_course(i, 23087, 5667, 0)`; `state = WALKING` | Smart seek | Fixed world coords for turtle nest |

### AI decision loop — tactic selection

Source: `fmain.c:2500–2595`.

Runs once per frame for each hostile NPC (`i = 2..anix`). Skips CARRIER (waypoint movement only) and SETFIG (stationary dialogue) types.

**Recalculation gate (`r`):**
- Default: `r = !bitrand(15)` → ~6.25% (1-in-16)
- For `mode & 2 == 0` (ATTACK1, ARCHER1): upgrades to `r = !rand4()` → ~25%
- For ATTACK2/ARCHER2: stays at 6.25%

**Tactic decision tree** (only when `r == true`):
1. Snake + turtle_eggs → `EGG_SEEK`
2. No weapon → `mode = CONFUSED`, `tactic = RANDOM`
3. Low vitality (<6) + 50% → `EVADE`
4. Archer + too close (xd<40, yd<30) → `BACKUP`
5. Archer + in range (xd<70, yd<70) → `SHOOT`
6. Archer + far away → `PURSUE`
7. Default melee → `PURSUE`

**Close-range melee transition** (separate from tactic tree):
```
thresh = 14 - mode;  // ATTACK1→14, ATTACK2→13, ARCHER1→11, ARCHER2→10
if (race == 7) thresh = 16;  // DKnight extended range
if (!(weapon & 4) && xd < thresh && yd < thresh) {
    set_course(i, hero_x, hero_y, 0);
    state = FIGHTING;  // bypass do_tactic, go straight to melee
}
```

**Non-hostile modes:**
- `FLEE` → `do_tactic(i, BACKUP)`
- `FOLLOWER` → `do_tactic(i, FOLLOW)`
- `STAND` → `set_course(direct)` then `state = STILL`
- `WAIT` → `state = STILL`

### `set_course(object, target_x, target_y, mode)` — Direction computation

Source: `fmain2.c` (search for `set_course`).

Computes facing direction from current position to target, with mode-dependent behavior:

**Smart-seek axis suppression** (modes 0–3):
```
if (xabs >> 1) > yabs: zero ydir  // X distance >> Y → go horizontal
if (yabs >> 1) > xabs: zero xdir  // Y distance >> X → go vertical
```
This prevents diagonal zigzag when one axis dominates.

**Facing deviation:**
- Mode 1: deviation=1 when dist<40 (slight collision avoidance)
- Mode 2: deviation=2 when dist<30 (aggressive collision avoidance)
- Applied randomly: 50% add, 50% subtract from facing

**Direction lookup** (`com2[9]`):
```
com2[] = {0, 1, 2, 7, 9, 3, 6, 5, 4}
Index = 4 - 3*ydir - xdir  (ydir,xdir ∈ {-1,0,+1})
```
Maps `(xdir, ydir)` sign pair to 8-way facing. Result `9` = STILL (at target).

**Mode-specific:**
- Mode 0: smart seek (default)
- Mode 3: FLEE — negate both `xdir` and `ydir` before lookup
- Mode 4: BUMBLE — skip axis suppression, allow true diagonals
- Mode 5: set facing but do NOT set state to WALKING (aiming pose)
- Mode 6: interpret target_x/y as raw delta vector, not world position

**`proxcheck(x, y, i)`** — Terrain + figure collision:
- Checks terrain collision via `prox(x, y)` (tile-based, returns non-zero for blocked)
- Wraith (race 2) bypasses tile collision
- Hero (i==0) can pass water/sink tiles (prox values 8, 9 are cleared)
- Figure-to-figure collision: 11px horizontal, 9px vertical exclusion zone; dead figures and rafts (j==1) are walkable

---

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

## Extents and Encounter Zones

`extent_list[EXT_COUNT]` (22 entries) — axis-aligned rectangles triggering encounters or events:

```c
struct extent {
    UWORD x1, y1, x2, y2;
    UBYTE etype;  // 0-49=random, 50=setgroup, 52=astral, 53=spiders, 60+=special, 70+=carriers
    UBYTE v1;     // encounter count or carrier ID
    UBYTE v2;     // spread/flags
    UBYTE v3;     // encounter_type (enemy race index)
};
```

Key entries:
- `extent_list[0]` — bird location (xtype=70, v3=11=bird cfile)
- `extent_list[1]` — turtle location (xtype=70, v3=5=turtle cfile)
- `extent_list[2]` — dragon area (xtype=70, v3=10)
- `extent_list[3]` — spider pit (xtype=53, encounter_type=4=snake? No, type=6=spider, spread=1)
- `extent_list[4]` — necromancer (xtype=60, encounter_type=9)
- `extent_list[5]` — turtle eggs rescue zone (xtype=61)
- `extent_list[6]` — princess zone (xtype=83)
- `extent_list[9]` — astral plane (xtype=52, type=8=Loraii)
- `extent_list[21]` — whole-world catch-all for random encounters (type=3, spread=8)

`xtype >= 50` triggers forced encounters; `xtype >= 60` triggers special carrier/NPC loads via `load_carrier()`.

---

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

### RNG implementation (`fsubs.asm:300–311`, `fmain.c:724`)

The game uses a single global LCG (linear congruential generator) for all randomness:
```asm
_rand:  move.l  _seed1,d0
        mulu.w  #45821,d0
        addq.l  #1,d0
        move.l  d0,_seed1
        ror.l   #6,d0
        and.l   #$7fffffff,d0
        rts

_bitrand:  bsr.s  _rand       ; rand() & mask
           and.l  4(sp),d0
           rts
```

**The seed is hardcoded and never re-initialized:**
```c
long seed1 = 19837325, seed2 = 23098324;   // fmain.c:724
```

The `original/notes` file contains a one-line reminder — *"Need to initialize random number generator."* — indicating the developer intended to seed from a clock or other source but never did. As a result, **all "random" values are fully deterministic**: every new game produces the identical RNG sequence, and scattered objects always appear at the same positions.

`seed2` is declared but never referenced; only `seed1` drives the LCG.

### Object distribution (`fmain2.c:1561–1583`)

On first visit to a region (`dstobs[region_num] == 0`), 10 treasure objects are scattered using `bitrand()`. Because the RNG seed is fixed (see above), these positions are **deterministic** — the same objects appear at the same coordinates every new game.

```
for each of 10 objects:
    x = bitrand(0x3fff) + ((region_num & 1) * 0x4000)
    y = bitrand(0x1fff) + ((region_num & 6) * 0x1000)
    retry until px_to_im(x, y) == 0  (passable terrain)
    ob_id = rand_treasure[bitrand(15)]
```
Region is then marked distributed (`dstobs[region_num] = 1`).

### Static object lists (`fmain2.c:1347–1508`)

In addition to the scatter system, the original game defines **hardcoded static object lists** for each region. These are populated unconditionally at region entry, not via the RNG.

- `ob_listg[]` — 11 global objects, visible in all regions
- `ob_list0[]` through `ob_list9[]` — per-region lists (typically 7–16 entries each)
- Total: ~120 static objects across all regions

Each entry is `(x, y, ob_id, ob_stat)`:
- `ob_stat 0` = nonexistent
- `ob_stat 1` = ground item (pickable, visible)
- `ob_stat 2` = in inventory
- `ob_stat 3` = setfig (NPC sprite)
- `ob_stat 4` = dead setfig
- `ob_stat 5` = hidden (revealed by Look command)
- `ob_stat 6` = cabinet item (inside furniture)

The starting chest near (19298, 16128) is in `ob_list3` (Region 3) with `ob_id = 15` (CHEST) and `ob_stat = 1`.

#### Implementation note

In the Rust port, static object data lives in `faery.toml` as `[[objects]]` entries rather than hardcoded arrays. The `itrans[]` translation table (ob_id → stuff[] index) remains in `src/game/world_objects.rs`.

---

## Key Bindings: Design and compatibility notes

- The original game's `letter_list[]` is a flat array scanned linearly on each keypress — we replace this with a `HashMap` reverse index for O(1) lookup.
- Direction keys need special handling: the original tracks key-down/key-up separately (`keydir` set on press, cleared on release), so we need to track held-key state.
- The KEYS menu (`SelectKey1`..`SelectKey6`) is only active when `cmode == KEYS` in the original; our implementation can context-gate these actions.
- Buy menu keys are only relevant when a shop interface is open — scene-level filtering handles this.
- Numpad movement (`1`–`9`) should be first-class defaults, not secondary aliases.
- Controller mapping should remain logical-action based so keyboard/controller rebinding share one action graph.
- Preserve original one-fire-button gameplay semantics as baseline; extra controller buttons are optional shortcuts to existing actions.
- Cheat keys from the original (`B`, `.`, `R`, `=`, arrows-teleport) are intentionally excluded from the rebindable system and handled separately as debug/cheat commands.

---

## Input Decoding — `decode_mouse` / `decodekey` (`fsubs.asm:1490–1576`)

All input (mouse, joystick, keyboard) is funnelled through `decode_mouse()`
which produces a single direction value 0–9 stored in `oldir`.  This value
indexes into `comptable[]` (compass highlight) and into the `xdir[]`/`ydir[]`
movement tables.

### Direction index convention

```
Index    Dir    xdir   ydir   Compass
  0      NW      -2     -2    upper-left
  1      N        0     -3    top-center
  2      NE       2     -2    upper-right
  3      E        3      0    right
  4      SE       2      2    lower-right
  5      S        0      3    bottom-center
  6      SW      -2      2    lower-left
  7      W       -3      0    left
  8      still    0      0    (1×1 no-op)
  9      still    0      0    (1×1 no-op)
```

Negative Y = up on screen = north.  The `newx(x,dir,speed)` / `newy(y,dir,speed)`
functions in `fsubs.asm:1274–1319` apply `xdir[dir]*speed/2` and `ydir[dir]*speed/2`.

### `keytrans` table (`fsubs.asm:221–226`)

Maps Amiga raw scancodes (0x00–0x5F) to internal key codes.
Movement-relevant entries:

| Amiga scancode | Physical key | keytrans code | dir (code−20) |
|----------------|--------------|---------------|---------------|
| 0x3D           | Numpad 7     | 20            | 0 = NW        |
| 0x3E           | Numpad 8     | 21            | 1 = N         |
| 0x3F           | Numpad 9     | 22            | 2 = NE        |
| 0x2D           | Numpad 4     | 27            | 7 = W         |
| 0x2E           | Numpad 5     | 29            | 9 = still     |
| 0x2F           | Numpad 6     | 23            | 3 = E         |
| 0x1D           | Numpad 1     | 26            | 6 = SW        |
| 0x1E           | Numpad 2     | 25            | 5 = S         |
| 0x1F           | Numpad 3     | 24            | 4 = SE        |
| 0x0F           | Numpad 0     | `'0'`         | fight (not dir)|

Cursor keys (0x4C–0x4F) map to codes 1–4 which are **not** direction codes
(they fall outside the 20–29 range); in the original they are cheat-only
teleport keys gated by the `cheat1` flag (`fmain.c:1487–1498`).

### `decodekey` path (`fsubs.asm:1565–1572`)

```
if keydir >= 20 && keydir < 30:
    dir = keydir - 20
else:
    dir = 9   (no direction)
```

Key-down sets `keydir = key`; key-up clears it when `(key & 0x7F) == keydir`.

### Joystick decoding (`fsubs.asm:1530–1563`)

Reads `JOY1DAT` ($DFF00C) to extract two axes:

```
xjoy = right_indicator - left_indicator    ∈ {-1, 0, 1}
yjoy = back_indicator  - forward_indicator ∈ {-1, 0, 1}
```

Where forward = joystick pushed away from player (up on screen, north).

A formula produces a 0–8 index: `idx = 4 + yjoy*3 + xjoy`, then `com2[idx]`
gives the direction value.

**`com2` table** (`fsubs.asm:1487`): `0, 1, 2, 7, 9, 3, 6, 5, 4`

```
Joystick grid:        com2 remapping:
 (L,Fwd)=0  (M,Fwd)=1  (R,Fwd)=2     dir 0=NW  dir 1=N   dir 2=NE
 (L,Mid)=3  (Center)=4  (R,Mid)=5     dir 7=W   dir 9=—   dir 3=E
 (L,Bck)=6  (M,Bck)=7  (R,Bck)=8     dir 6=SW  dir 5=S   dir 4=SE
```

### Mouse compass click (`fsubs.asm:1496–1528`)

When the left mouse button is held and the pointer is in the compass area
(x > 265), the pointer coordinates are divided into a 3×3 grid to produce
a direction 0–9:

```
X: <292 = left column     292–300 = middle column     >300 = right column
Y: <166 = top row         166–174 = middle row        >174 = bottom row
```

### Rust port mapping

Our `Direction` enum uses a different order than the original:

| Our facing | Direction | Original dir | comptable index |
|------------|-----------|--------------|-----------------|
| 0          | N         | 1            | 1               |
| 1          | NE        | 2            | 2               |
| 2          | E         | 3            | 3               |
| 3          | SE        | 4            | 4               |
| 4          | S         | 5            | 5               |
| 5          | SW        | 6            | 6               |
| 6          | W         | 7            | 7               |
| 7          | NW        | 0            | 0               |

Formula: `comptable_index = (facing + 1) & 7`.

**Important:** any bitwise checks on `an->facing` from the original must be
remapped to Rust facing values. Example: the weapon draw-order test
`(facing - 2) & 4` evaluates true for original dirs 0,1,6,7 (NW,N,SW,W).
In Rust that corresponds to facing 0,5,6,7 (N,SW,W,NW). Always convert
through the table above rather than copying the original expression directly.

---

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

## Menu System (`fmain.c:538–589`, `3758–3820`, `4409–4441`; `fmain2.c:613–675`; `fsubs.asm:120–165`)

### 10 Menu Modes

```
ITEMS = 0   MAGIC = 1   TALK = 2   BUY  = 3   GAME  = 4
SAVEX = 5   KEYS  = 6   GIVE = 7   USE  = 8   FILE  = 9
```

| Mode  | Purpose                              | Label str | Color idx |
|-------|--------------------------------------|-----------|-----------|
| ITEMS | Inventory / object interaction       | `label1`  | 4         |
| MAGIC | Cast spells (F-key driven)           | `label2`  | 5         |
| TALK  | NPC communication                    | `label3`  | 6         |
| BUY   | Purchase items from shops            | `label4`  | 7         |
| GAME  | Pause / Music / Sound / nav          | `label5`  | 8         |
| SAVEX | Save or quit                         | `label6`  | 9         |
| KEYS  | Try a key type on a door             | `label7`  | 10        |
| GIVE  | Give items to NPCs                   | `label8`  | 11        |
| USE   | Equip weapon / use special items     | `label9`  | 12        |
| FILE  | Load / save file slots               | `labelA`  | 13        |

### `enabled[]` Bit Flags

Each menu slot's `enabled[i]` byte encodes both visibility and behaviour:

```
bit 0      : selected / active  (1 = on for toggles)
bit 1      : displayed / visible (must be set to appear in menu)
bits 2–7   : action type
  0x00 (0) : tab header — click switches cmode; always shown
  0x04 (4) : toggle — click flips bit 0
  0x08 (8) : immediate action — fires once on click
  0x0C (12): radio button — sets bit 0 exclusively
```

Common combined values:

| Value | Meaning                                           |
|-------|---------------------------------------------------|
| 0     | Not displayed, not active (empty slot)            |
| 2     | Displayed, not selected, tab type (inactive tab)  |
| 3     | Displayed, selected, tab type (active tab)        |
| 6     | Displayed, not selected, toggle (Pause starts OFF)|
| 7     | Displayed, selected, toggle (Music/Sound start ON)|
| 8     | Immediate, not displayed (hidden until set_options)|
| 10    | Displayed, immediate (standard menu item)         |

### Label Strings (`fmain.c:538–549`)

Each slot is exactly 5 characters (no null terminator; the renderer reads 5 bytes directly):

```
Slots 0–4:  tab labels (shared across all modes)
  "Items" "Magic" "Talk " "Buy  " "Game "   (ITEMS…GAME tabs)
  — extended tab row for SAVEX/KEYS/GIVE/USE/FILE uses per-mode label strings

label1 (ITEMS) : "ItemsMagicTalk Buy  Game Save Keys Give Use  File"
label2–labelB  : same 5-char-per-tab structure for each mode
```

Each `menus[k].label` points into the concatenated string; slots 0–4 are the 5 mode-tab names repeated in the active mode's color.

### Settings Toggles (Critical for Game Behavior)

```
menus[GAME].enabled[5] & 1  → Pause   (1 = paused; freezes game loop)
menus[GAME].enabled[6] & 1  → Music   (1 = on; setmood() plays/stops music)
menus[GAME].enabled[7] & 1  → Sound   (1 = on; effect() plays samples)
```

- Pause starts at `6` (toggle, OFF); Music and Sound start at `7` (toggle, ON).
- `gomenu()` returns immediately without changing mode when Pause is active.

### `gomenu()` (`fmain.c:4409–4414`)

```c
void gomenu(short mode) {
    if (menus[GAME].enabled[5] & 1) return;  // refuse if paused
    cmode = mode;
    handler_data.lastmenu = 0;
    print_options();
}
```

### `print_options()` → `real_options[]` Mapping (`fmain.c:3758–3782`)

```
j = 0   // display slot counter
for i = 0 .. menus[cmode].num:
    if (enabled[i] & 2) == 0: skip   // not visible
    real_options[j] = i               // display slot j → menu index i
    propt(j, enabled[i] & 1)
    j++
// remaining slots:
real_options[j] = -1; draw blank button
```

`real_options[]` lets click/key dispatch translate a display-slot index back to the true `enabled[]` index.

### `propt()` Button Rendering (`fmain.c:3785–3819`)

**Background color** (`penb`):

```
cmode == USE   → 14  (grey,       textcolors[14] = 0x888)
cmode == FILE  → 13  (light grey, textcolors[13] = 0xCCC)
k < 5          →  4  (blue tab,   textcolors[4]  = 0x00F)
cmode == KEYS  → keycolors[k-5]
cmode == SAVEX → k   (slot index used directly as color index)
else           → menus[cmode].color
```

**Foreground color** (`pena`):

```
0 = black (textcolors[0]) — normal / off state
1 = white (textcolors[1]) — selected / on state (toggles)
```

**Screen position** (Amiga lo-res source coordinates):

```
x = 430  (even display slot)  or  482  (odd display slot)
y = (slot / 2) * 9 + 8
```

### `set_options()` Inventory-Driven Visibility (`fmain.c:4417–4441`)

`stuff_flag(x)` returns `8` (hidden/immediate) when `x == 0`, else `10` (displayed/immediate).

| Mode  | Slot(s) | Rule                                               |
|-------|---------|----------------------------------------------------|
| MAGIC | 5–11    | `stuff_flag(stuff[i+9])` — owns magic item?        |
| USE   | 0–6     | `stuff_flag(stuff[i])` — owns weapon `i`?          |
| USE   | 7 (Keys)| 10 if any key type owned, else 8                   |
| USE   | 8 (Sunstone)| `stuff_flag(stuff[7])`                         |
| KEYS  | 5–10    | `stuff_flag(stuff[i+16])` — owns key type `i`?    |
| GIVE  | 5 (Gold)| 10 if `wealth > 2`, else 8                         |
| GIVE  | 6 (Book)| always 8 (permanently hidden)                      |
| GIVE  | 7 (Writ)| `stuff_flag(stuff[28])`                            |
| GIVE  | 8 (Bone)| `stuff_flag(stuff[29])`                            |

`set_options()` is called after every `do_option()` action so the menu reflects the current inventory state.

### `do_option()` Dispatch Table (`fmain.c:3830–3393`)

| cmode | hit   | Action                                              |
|-------|-------|-----------------------------------------------------|
| ITEMS | 5     | Show inventory screen (`viewstatus = 4`)            |
| ITEMS | 6     | Take nearest object                                 |
| ITEMS | 7     | Look (print region / stats)                         |
| ITEMS | 8     | `gomenu(USE)`                                       |
| ITEMS | 9     | `gomenu(GIVE)`                                      |
| MAGIC | 5–11  | Cast spell (if owned)                               |
| TALK  | 5     | Yell                                                |
| TALK  | 6     | Say (speak to nearest NPC)                          |
| TALK  | 7     | Ask (query nearest NPC)                             |
| BUY   | 5–11  | Buy item (via `jtrans[]` price table)               |
| GAME  | 5     | Pause toggle (handled before `do_option`)           |
| GAME  | 6     | Music toggle → `setmood(TRUE)`                      |
| GAME  | 7     | Sound toggle (`effect()` checks `enabled[7] & 1`)  |
| GAME  | 8     | `gomenu(SAVEX)`                                     |
| GAME  | 9     | `gomenu(FILE)`                                      |
| USE   | 0–4   | Set weapon (`anim_list[0].weapon = hit + 1`)        |
| USE   | 6     | Summon turtle (`get_turtle()`)                      |
| USE   | 7     | `gomenu(KEYS)`                                      |
| USE   | 8     | Use Sunstone (if `witchflag`)                       |
| SAVEX | 5     | Save game → `gomenu(FILE)`                          |
| SAVEX | 6     | Quit (`quitflag = TRUE`)                            |
| FILE  | 5–12  | Load/save slot → `savegame(hit)` → `gomenu(GAME)`  |
| KEYS  | 5–10  | Try key type on door → `gomenu(ITEMS)`              |
| GIVE  | 5     | Give gold to nearest NPC (if `wealth > 2`)          |
| GIVE  | 7     | Give Writ of Passage                                |
| GIVE  | 8     | Give Bone                                           |
| All   | —     | Calls `set_options()` after every action            |

### `letter_list[38]` Keyboard Shortcuts (`fmain.c:579–589`)

```
Key    Mode   Slot  Action
'I'    ITEMS  5     List inventory
'T'    ITEMS  6     Take
'?'    ITEMS  7     Look
'U'    ITEMS  8     → Use menu
'G'    ITEMS  9     → Give menu
'Y'    TALK   5     Yell
'S'    TALK   6     Say
'A'    TALK   7     Ask
' '    GAME   5     Toggle Pause
'M'    GAME   6     Toggle Music
'F'    GAME   7     Toggle Sound
'Q'    GAME   8     → Save/Exit menu
'L'    GAME   9     → Load/File menu
'O'    BUY    5     Buy Food
'R'    BUY    6     Buy Arrows
'8'    BUY    7     Buy Vial
'C'    BUY    8     Buy Mace
'W'    BUY    9     Buy Sword
'B'    BUY    10    Buy Bow
'E'    BUY    11    Buy Totem
'V'    SAVEX  5     Save (only fires when cmode == SAVEX)
'X'    SAVEX  6     Exit / Quit
'1'    USE    0     Equip Dirk
'2'    USE    1     Equip Mace
'3'    USE    2     Equip Sword
'4'    USE    3     Equip Bow
'5'    USE    4     Equip Wand
'6'    USE    5     Equip Lasso
'7'    USE    6     Summon Turtle
'K'    USE    7     → Keys menu
F1–F7  MAGIC  5–11  Cast spells (separate F-key path, not letter_list)
```

**Notes:**
- SAVEX entries (`'V'`, `'X'`) only fire when `cmode == SAVEX` (`fmain.c:1510–1511`).
- MAGIC uses F-keys via a separate key-handling path, not `letter_list`.
- KEYS sub-mode: digits `'1'`–`'6'` map directly to `do_option(key - '1' + 5)`.

### `keycolors[6]` (`fmain.c:551`)

```
Index  textcolors idx  Color   Key Type
0      8               0xF90   Gold key
1      6               0x090   Green key
2      4               0x00F   Blue key
3      2               0xC00   Red key
4      14              0x888   Grey key
5      1               0xFFF   White key
```

Used by `propt()` as background color when `cmode == KEYS` and `k >= 5`.

### `prq()` Deferred Action Queue (`fmain2.c:613–675`)

The original engine uses a 32-entry circular buffer for deferred rendering requests:

```
prq(4)   → redraw vitality stat in HI bar
prq(5)   → call print_options() (redraw all menu buttons)
prq(7)   → redraw Brv/Lck/Knd/Wlth stats bar
prq(10)  → print "Take What?" message
```

In the Rust port these are handled directly — no queue is needed because the screen is redrawn every frame.

### Mouse Click → Button Slot Mapping (`fsubs.asm:136–165`)

```
Valid click X range (Amiga hi-res): 430–530
  lo-res equivalent: 215–265

Button index calculation (lo-res coordinates):
  row   = (mouseY - 144) / 9
  col   = (mouseX < 240) ? 0 : 1   // left column = even slots, right = odd
  index = row * 2 + col             // 0–11; maps to display slot

On mouse-down : generates code  0x61 + index  (button press)
On mouse-up   : generates code  0x80 | (0x61 + index)  (button release)
```

The Rust port maps SDL2 mouse coordinates directly without the Amiga lo-res scaling factor.

---

## Compass Rose — Direction Indicator Bitmaps

The HI-bar compass is rendered by `drawcompass(dir)` (`fmain2.c:493–508`).
Two single-plane bitmaps control the compass appearance; they are composited
into bitplane 2 of the text viewport at position **(567, 15)**, sized
**48 × 24** pixels.

### Source data

`_hinor` and `_hivar` are defined in `fsubs.asm` (lines 250–277) as raw
`dc.l` longwords.  At startup `into_chip()` copies them into Chip RAM so the
blitter can access them.

The backing bitmap is initialised as:

```c
InitBitMap(bm_source, 3, 64, 24);   /* 3 planes, 64 px wide, 24 rows */
```

Only **plane 2** is used — planes 0 and 1 of `bm_source` are unused.
Stride is `64 / 8 = 8` bytes per row; each plane occupies `8 × 24 = 192`
bytes.  The compass content occupies the leftmost **48 pixels** (6 bytes) of
each row; the trailing 2 bytes per row are padding.

| Symbol   | Role                                  | Size (bytes) |
|----------|---------------------------------------|--------------|
| `_hinor` | Normal compass (no direction highlighted) | 200 (192 + 8 pad) |
| `_hivar` | All directions highlighted                | 200 (192 + 8 pad) |

### `drawcompass(dir)` algorithm

```
1.  bm_source->Planes[2] = nhinor
2.  BltBitMap(bm_source, 0, 0, bm_text, 567, 15, 48, 24, 0xC0, 4, NULL)
        — blits the entire 48×24 normal compass to the text viewport
3.  if dir < 9:
        bm_source->Planes[2] = nhivar
        BltBitMap(bm_source, xr, yr, bm_text, 567+xr, 15+yr, xs, ys, 0xC0, 4, NULL)
            — overlays only the active direction sub-region with the highlighted variant
```

**BltBitMap parameters:**

| Param    | Value  | Meaning                              |
|----------|--------|--------------------------------------|
| minterm  | `0xC0` | D := A (straight copy, source → dest) |
| mask     | `4`    | Binary `0100` → only plane 2 is copied |

### `comptable[10]` — Direction sub-regions

Each entry defines a rectangle `{xrect, yrect, xsize, ysize}` within the
48 × 24 compass area.  Directions 8 and 9 are "standing still" (1 × 1 no-op).

| Index | Direction | xrect | yrect | xsize | ysize |
|-------|-----------|-------|-------|-------|-------|
| 0     | NW        |  0    |  0    | 16    |  8    |
| 1     | N         | 16    |  0    | 16    |  9    |
| 2     | NE        | 32    |  0    | 16    |  8    |
| 3     | E         | 30    |  8    | 18    |  8    |
| 4     | SE        | 32    | 16    | 16    |  8    |
| 5     | S         | 16    | 13    | 16    | 11    |
| 6     | SW        |  0    | 16    | 16    |  8    |
| 7     | W         |  0    |  8    | 18    |  8    |
| 8     | still     |  0    |  0    |  1    |  1    |
| 9     | still     |  0    |  0    |  1    |  1    |

### How plane 2 produces colour

The text viewport (opened by `setup_screen` in `fmain.c`) uses the
`textcolors[]` palette.  Plane 2 is bit 2 of the 4-bit colour index.
The compass area in `bm_text` gets planes 0, 1, 3 from the hiscreen image;
plane 2 is the only plane modified by `drawcompass()`.

The resulting colour at each pixel is `textcolors[index]` where
`index = (p3 << 3) | (p2 << 2) | (p1 << 1) | p0`.
Setting plane 2 toggles between colour pairs, e.g.:

| Planes 3,1,0 | Plane 2 = 0       | Plane 2 = 1          |
|---------------|-------------------|----------------------|
| `0,0,0`       | `[0]` 0x000 black | `[4]` 0x00F blue     |
| `0,0,1`       | `[1]` 0xFFF white | `[5]` 0xC0F magenta  |
| `0,1,0`       | `[2]` 0xC00 red   | `[6]` 0x090 green    |
| `0,1,1`       | `[3]` 0xF60 orange| `[7]` 0xFF0 yellow   |

### Rust port notes

The extracted compass data lives in `faery.toml` under `[compass]`:

- `[compass.comptable]` — direction sub-regions
- `[compass.hinor]` — normal compass, single-plane BitMap (48 × 24, stride 6)
- `[compass.hivar]` — highlighted compass, single-plane BitMap

At render-resource build time the port extracts the compass region from
the hiscreen `IffImage`, replaces plane 2 with `hinor` / `hivar`, and
converts both composites to RGBA textures using the `textcolors` palette.
During gameplay, the normal compass texture is blitted first; if the player
is moving, the active direction sub-region from the highlighted texture is
overlaid on top.

---

## Screen Layout: Amiga Mixed-Resolution Viewports

### Original Amiga display geometry

The game opens a single 640×200 HIRES (non-interlaced) Amiga screen (`form.c:26`:
`NewScreen = {0, 0, 640, 200, 3, 0, 1, HIRES, CUSTOMSCREEN, …}`). Two
Copper-switched viewports with **different resolutions** tile it vertically.
This is a standard Amiga technique — the Copper switches the display mode
between scanlines, so each viewport can use its own pixel clock.

| Viewport   | Field           | Value | Source (`fmain.c`) |
|------------|-----------------|-------|--------------------|
| `vp_page`  | `DxOffset`      | 16    | LO-RES, 2px = 1 physical px wide |
| `vp_page`  | `DWidth`        | 288   | 288 lo-res px = 576 physical px |
| `vp_page`  | `DyOffset`      | 0     | starts at top |
| `vp_page`  | `DHeight`       | 140   | game playfield |
| `vp_text`  | `DxOffset`      | 0     | HI-RES, 1px = 1 physical px |
| `vp_text`  | `DWidth`        | 640   | full HIRES width |
| `vp_text`  | `DyOffset`      | 143   | `PAGE_HEIGHT` — just below playfield |
| `vp_text`  | `DHeight`       | 57    | `TEXT_HEIGHT` — HI bar |

There is a 3 lo-res scanline gap between DyOffset=0+DHeight=140 and vp_text's
DyOffset=143. This gap contains no display data and appears black on real
hardware.

The playfield bitmap is `InitBitMap(bm_page1, PAGE_DEPTH, PHANTA_WIDTH=320, RAST_HEIGHT=200)` — 320 lo-res columns, of which 288 are displayed (DxOffset=16 clips 16 px on the left). The HI bar bitmap is `InitBitMap(bm_text, 4, 640, TEXT_HEIGHT=57)` — native HIRES pixels.

### SDL port mapping (640×480 logical canvas)

The SDL canvas uses `set_logical_size(640, 480)`. The entire game area (both
viewports) is **2× line-doubled** vertically, producing a 640×400 active
region centered in the 640×480 canvas with 40px margins top and bottom.

```
Canvas (640×480)
┌──────────────────────────────────────────────────────────┐  y=0
│                    40px top margin                       │
├──────────────────────────────────────────────────────────┤  y=40
│  32px │         playfield (576×280)         │  32px      │
│  left │                                     │  right     │
│       │  vp_page, LO-RES 288×140 → 2×       │            │
├──┬────┴─────────────────────────────────────┴────────────┤  y=320
│  │                6px gap                                │
├──┴────────────────────────────────────────────────────────┤  y=326
│              HI bar (640×114)                             │
│         vp_text, HIRES 640×57 → 2× line-doubled          │
├──────────────────────────────────────────────────────────┤  y=440
│                   40px bottom margin                     │
└──────────────────────────────────────────────────────────┘  y=480
```

| Zone | Amiga geometry | SDL dest rect | Scale |
|------|---------------|---------------|-------|
| Playfield | LO-RES 288×140 px | `(32, 40, MAP_DST_W*2, MAP_DST_H*2)` | 2× both axes |
| Gap | 3 lo-res scanlines | y=320–325, 6px | — |
| HI bar | HIRES 640×57 px | `(0, 326, 640, 114)` | 2× vertical, 1× horizontal |

`CANVAS_MARGIN_Y = 40`. `PLAYFIELD_X = 32` (DxOffset=16 lo-res × 2).
`HIBAR_Y = 40 + 280 + 6 = 326`. `HIBAR_H = 57 × 2 = 114`.

### HI bar coordinate system

All UI elements inside the HI bar (`propt()`, compass, messages, stat line)
use Amiga HIRES pixel coordinates within the 57px band. In the SDL port these
are **scaled by 2** to match the 2× line-doubling — `HIBAR_Y = 326`,
`HIBAR_H = 114`.

| Element | Amiga source coords (within 57px band) | SDL canvas coords |
|---------|----------------------------------------|-----------------|
| Button column 0 (left) | x=430 | x=430 |
| Button column 1 (right) | x=482 | x=482 |
| Button row n baseline | y = n×9+8 | y = 326 + n×18+16 |
| Compass top-left | (567, 15) | (567, 326+30=356) |
| Compass size | 48×24 | 48×48 (2× tall) |
| Messages bottom | y=56 | y = 326+112=438 |

The `propt()` formula from `fmain.c:3812`: `y = ((j / 2) * 9) + 8` maps slot
index j to HIRES pixel rows within `vp_text`; the SDL port doubles all y
values: `HIBAR_Y + (j / 2) * 18 + 16`.

### Key source references

- `original/form.c:25–26` — `NewScreen` definition (640×200 HIRES)
- `original/fmain.c:853–887` — `setup_screen()`: viewport init, `InitBitMap` calls, `vp_page`/`vp_text` field assignments
- `original/fmain.c:10–15` — `#define` block: `SCREEN_WIDTH=288`, `PAGE_HEIGHT=143`, `RAST_HEIGHT=200`, `TEXT_HEIGHT=57`
- `original/fmain.c:3785–3822` — `propt()`: button placement formula
- `src/game/gameplay_scene.rs` — `CANVAS_MARGIN_Y`, `PLAYFIELD_X`, `PLAYFIELD_Y`, `HIBAR_H`, `HIBAR_Y` constants; `render_by_viewstatus()` for playfield 2× blit and HI bar 2×-vertical blit

---

## Known Original Exploits

These bugs exist in the original 1987 release. The port should avoid replicating them.

### Pause-Take duplication (`fmain.c` — do_option / prq path)

When the game is paused (Space), pressing `T` triggers the Take action. Because the game
loop is suspended, the player can press `T` repeatedly to pick up the same ground item
multiple times without it being consumed.

**Fix**: Guard `MenuAction::Take` dispatch (and any other item-consuming immediate action)
behind an `!is_paused()` check, similar to the existing `gomenu()` guard. The `handle_key`
path in `menu.rs` already blocks all keys except Space while paused, so the exploit cannot
occur via the menu key path. Verify that the `GameAction::Take` path in the direct key
binding layer (`key_bindings.rs`) also checks the paused state before acting.

### Key replenishment after save/reload within a session (`fmain.c` — save/load path)

If the player enters an area, saves the game, uses keys to unlock doors, then reloads the
save in the same session, the keys are restored from the save file but the door-unlocked
state is not reset (door state is held in a runtime table, not persisted). The player
effectively gets unlimited key uses.

**Fix**: When implementing `LoadGame`, reset all in-memory door state (the runtime "door
open" flags in `doors.rs`) before restoring from the save file. Alternatively, persist door
state as part of the save file format so reload is fully consistent.

---

## World Map: Region Diagrams

### Region selection formula (`gen_mini()`, `fmain.c:3661–3690`)

```c
xs = (map_x + 151) >> 8       // sector column at viewport centre
ys = (map_y + 64) >> 8        // sector row at viewport centre
xr = (xs >> 6) & 1            // 0 = west column, 1 = east column
yr = (ys >> 5) & 3            // 0–3 = north → south band
region_num = xr + yr * 2      // 0–7 for outdoor; ≥8 hard-coded (indoor/dungeon)
```

Uses `map_x`/`map_y` (viewport top-left), not `hero_x`/`hero_y`. The offsets 151 and 64 centre the calculation within the 288 × 140 playfield.

The outdoor world is a 2-column × 4-row grid of regions. All coordinate
ranges are in world pixel units (0–32 767 on each axis). Region transitions
are seamless; `new_region` is set and `load_all()` is called when the hero
crosses a boundary.

### World overview (outdoor regions)

```
         x: 0 – 16 376          x: 16 377 – 32 767
         ┌──────────────────────┬──────────────────────┐
y: 0–    │  F1  [id=0]          │  F2  [id=1]          │
8 217    │  Snowy Region        │  Witch Wood          │
         ├──────────────────────┼──────────────────────┤
y: 8218– │  F3  [id=2]          │  F4  [id=3] ★start   │
16 409   │  Swampy Region       │  Plains & Rocks      │
         ├──────────────────────┼──────────────────────┤
y: 16410–│  F5  [id=4]          │  F6  [id=5]          │
24 601   │  Desert Area         │  Bay / City / Farms  │
         ├──────────────────────┼──────────────────────┤
y: 24602–│  F7  [id=6]          │  F8  [id=7]          │
32 767   │  Volcanic            │  Forest & Wilderness │
         └──────────────────────┴──────────────────────┘

★ Player starts at (19 036, 15 755), region_num = 3
```

### Coordinate boundaries (derived)

| Boundary | World coord | Formula |
|---|---|---|
| West/East split | x = 16 377 | `xs=64` when `hero_x+7 = 16384` |
| Row 0/1 split | y = 8 218 | `ys=32` when `hero_y-26 = 8192` |
| Row 1/2 split | y = 16 410 | `ys=64` when `hero_y-26 = 16384` |
| Row 2/3 split | y = 24 602 | `ys=96` when `hero_y-26 = 24576` |

---

### F1 — Snowy Region (id = 0)

x: 0 – 16 376   y: 0 – 8 217   (grid cell ≈ 410 × 512 world units)

```
     x=0                                  x=16376
y=0  +----------------------------------------+
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
y≈8k |            dc                      cp  |
     +----------------------------------------+
```

| Code | Place | x | y |
|------|-------|---|---|
| dc | Dragon Cave | 4 992 | 7 008 |
| cp | Crystal Palace | 15 840 | 7 104 |

---

### F2 — Witch Wood (id = 1)

x: 16 377 – 32 767   y: 0 – 8 217   (cell ≈ 410 × 512)

```
     x=16377                              x=32767
y=0  +----------------------------------------+
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                te                      |
     |                                        |
     |                   nk mc               |
y≈8k |                         wc             |
     +----------------------------------------+
```

| Code | Place | x | y |
|------|-------|---|---|
| te | Turtle Eggs | 22 945–23 225 | 5 597–5 747 |
| nk | North Keep | 24 176 | 6 752 |
| mc | Maze Caves 1 & 2 | 25 792 / 26 048 | 6 240 / 6 688 |
| wc | Witch's Castle | 26 624 | 7 008 |

---

### F3 — Swampy Region (id = 2)

x: 0 – 16 376   y: 8 218 – 16 409   (cell ≈ 410 × 512)

```
     x=0                                  x=16376
y≈8k +----------------------------------------+
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                   ss      lk           |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                            ri          |
     |                                        |
     |                   wk                   |
y≈16k+----------------------------------------+
```

| Code | Place | x | y |
|------|-------|---|---|
| ss | Swamp Shack | 9 344 | 13 216 |
| lk | Lakeside Keep | 12 144 | 11 872 |
| ri | Road's End Inn | 12 672 | 14 528 |
| wk | West Keep | 7 792 | 15 200 |

---

### F4 — Plains and Rocks (id = 3) ★ Starting Region

x: 16 377 – 32 767   y: 8 218 – 16 409   (cell ≈ 410 × 512)

```
     x=16377                              x=32767
y≈8k +----------------------------------------+
     |                                        |
     |                                        |
  ck |                                        |
     |                                        |
     |                   sx      gk           |
     |                                        |
     |                                        |
     |  fi      tc               sk           |
     |                                        |
     |     cr      mm                         |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
  fr |                                        |
y≈16k|TB                                      |
     +----------------------------------------+
```

| Code | Place | x | y |
|------|-------|---|---|
| ck | Coast Keep | 17 008 | 9 568 |
| fi | Forest Inn | 18 304 | 12 224 |
| sx | Spider Exit (cave) | 24 256 | 10 592 |
| gk | Glade Keep | 26 224 | 10 848 |
| tc | Troll Cave | 22 720 | 11 872 |
| sk | Sea Keep | 27 760 | 11 872 |
| cr | Crag Keep | 19 568 | 12 896 |
| mm | Mammoth Manor | 24 816 | 12 992 |
| fr | Friendly Inn | 17 024 | 15 296 |
| TB | Tambry ★ (safe spawn) | 18 848–19 072 | 15 552–16 000 |

---

### F5 — Desert Area (id = 4)

x: 0 – 16 376   y: 16 410 – 24 601   (cell ≈ 410 × 512)

Note: the desert interior is blocked unless `stuff[STATBASE] >= 5`
(five Gold Statues collected).

```
     x=0                                  x=16376
y≈16k+----------------------------------------+
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                 oa      to             |
     |                                        |
     |                                        |
     |           df                           |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
y≈24k+----------------------------------------+
```

| Code | Place | x | y |
|------|-------|---|---|
| oa | Oasis (5 doors) | 6 816–7 040 | 19 296–19 360 |
| to | Tombs of Hemsath | 13 424 | 19 296 |
| df | Desert Fort (4 doors) | 4 464 | 20 576 |

---

### F6 — Bay / City / Farms (id = 5)

x: 16 377 – 32 767   y: 16 410 – 24 601   (cell ≈ 410 × 512)

```
     x=16377                              x=32767
y≈16k+----------------------------------------+
     |                                        |
     |        gc c1                  lh       |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |    mk  c2 MH                  pk       |
     |                                        |
     |                                        |
     |                fk                      |
     |                                        |
     |      c6                                |
y≈24k+----------------------------------------+
```

| Code | Place | x | y |
|------|-------|---|---|
| gc | Graveyard / Crypt | 19 596–19 974 / 19 856 | 17 034–17 401 / 17 280 |
| c1 | Cabin #1 yard+door | 21 600–21 648 | 17 728–17 824 |
| lh | Lighthouse | 27 472 | 17 280 |
| mk | Mountain Keep | 17 888 | 21 376 |
| c2 | Cabin #2 yard+door | 19 808–19 856 | 21 568–21 664 |
| MH | Marheim (city + castle) | 21 984–22 368 | 20 896–21 568 |
| pk | Point Keep | 28 384 | 21 120 |
| fk | Farm Keep | 23 008 | 22 656 |
| c6 | Cabin #6 yard+door | 18 784–18 832 | 23 360–23 456 |

Additional cabins in F6: Cabin #3 (21 344–21 392, 22 592–22 688), Cabin #4 (22 624–22 672, 23 872–23 968), Cabin #5 (25 952–26 000, 23 872–23 968).
River Keep (19 936, 27 520) and Lonely Keep (27 616, 31 872) are in F8, not F6.

---

### F7 — Volcanic (id = 6)

x: 0 – 16 376   y: 24 602 – 32 767   (cell ≈ 410 × 512)

Note: the lava zone (`LV`, hero needs Rose item for fire immunity) covers
roughly x = 8 946–13 706, y = 24 834–29 634 (`fmain.c` Rose check).

```
     x=0                                  x=16376
y≈24k+----------------------------------------+
     |                gf             pf       |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |      bi    pf                 LV       |
     |                                        |
     |                                        |
     |                                        |
     |                            dt LV       |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
y≈32k+----------------------------------------+
```

| Code | Place | x | y |
|------|-------|---|---|
| gf | Gate Fort | 6 512 | 25 248 |
| pf | Plain Fort | 12 144 | 25 504 |
| bi | Bird (swan) start | 2 118–2 618 | 27 237–27 637 |
| pf | Pass Fort | 6 000 | 27 296 |
| LV | Lava Zone | 8 946–13 706 | 24 834–29 634 |
| dt | Doom Tower | 11 264 | 29 024 |

---

### F8 — Forest and Wilderness (id = 7)

x: 16 377 – 32 767   y: 24 602 – 32 767   (cell ≈ 410 × 512)

```
     x=16377                              x=32767
y≈24k+----------------------------------------+
     |                                        |
     |             eg                         |
     |                                        |
     |     c7      uc               c8        |
     |                                        |
     |                                        |
     |         rk                             |
     |                                        |
     |                uc9                     |
     |                    ca                  |
     |                                        |
     |                                        |
     |                                        |
     |                                        |
     |                            lp          |
y≈32k+----------------------------------------+
```

| Code | Place | x | y |
|------|-------|---|---|
| eg | Elf Glade | 21 616 | 25 728 |
| c7 | Cabin #7 yard+door | 18 528–18 576 | 26 176–26 272 |
| uc | Unreachable Castle | 22 944 | 26 464 |
| c8 | Cabin #8 yard+door | 27 936–28 048 | 26 688–26 784 |
| rk | River Keep | 19 936 | 27 520 |
| uc9 | Cabin #9 yard+door | 22 880–22 928 | 28 480–28 576 |
| ca | Cabin #10 yard+door | 24 672–24 720 | 29 248–29 344 |
| lp | Lonely Keep | 27 616 | 31 872 |

Hidden Valley encounter zone: x = 21 405–21 827, y = 25 583–26 028 (overlaps eg / uc area).

---

### F9 — Inside Buildings (id = 8) and F10 — Dungeons & Caves (id = 9)

Indoor regions do **not** use the coordinate-formula region select. Once
`secs` on a door entry sets `new_region = 8` (buildings) or `9` (dungeons),
the player stays in that region until exiting through any door with the
matching secs value.

**Interior coordinate system** (from `doorlist[].xc2, yc2`):

| Axis | Range observed in `doorlist` |
|------|------------------------------|
| x (interior) | 960 – 12 752 |
| y (interior) | 33 408 – 40 096 (= 0x8280 – 0x9CA0) |

The interior y-origin is offset by `+32768` (0x8000) above the outdoor
world; both F9 and F10 share this address space but reference different
`sector_block = 96` sectors.

**Key F9 interior locations** (from `doorlist[].xc2, yc2`):

| Interior | xc2 | yc2 |
|----------|-----|-----|
| Tambry buildings (8 doors) | 3 024–5 232 | 33 024–34 496 |
| Keep interior | 10 352 | 35 680 |
| Marheim city buildings (10 doors) | 6 400–8 288 | 33 408–34 752 |
| Marheim main castle | 9 600 | 33 920 |
| Palace of King Mar (F9) | 9 728–12 544 | 33 024–35 584 |
| Desert fort interior | 10 352 | 35 680 |
| Oasis interior (5 doors) | 4 992–6 528 | 37 728–38 848 |

**Key F10 interior locations** (cave/dungeon doors):

| Interior | xc2 | yc2 |
|----------|-----|-----|
| Dragon Cave | 6 528 | 35 936 |
| Troll Cave | 4 544 | 35 680 |
| Spider Cave exit | 4 544 | 35 680 |
| Maze Cave 1 | 1 200 | 34 880 |
| Maze Cave 2 | 960 | 34 400 |
| Tombs | 1 136 | 36 576 |

**Astral Plane** — special sub-zone within F9 triggered by the
`extent_list[9]` rectangle: indoor coords x = 9 216–12 544, y = 33 280–35 328
(`0x2400–0x3100`, `0x8200–0x8A00`). Music switches to palace group 4.

## Message Trigger Reference

All in-game text messages are defined in `faery.toml` across four tables:
`event_msg` (gameplay events), `place_msg` (outdoor locations), `inside_msg`
(interior locations), and `speeches` (NPC dialogue). The original source
defines these in `narr.asm` and triggers them via `event(index)` and
`speak(index)` calls throughout `fmain.c` and `fmain2.c`.

The `%` token in message text is substituted with the active brother's name
(Julian, Phillip, or Kevin) at display time.

**Key finding:** All 148 messages are reachable in the original game — none
are orphaned or unused.

### Display architecture (original)

- `event(index)` — extracts from `_event_msg[]` array, formats with hero
  name, pushes to the HI bar message scroll area.
- `speak(index)` — extracts from `_speeches[]` array, same display path.
- `find_place()` — data-driven lookup; when the player enters a POI tile
  sector, looks up `_place_tbl[sector]` or `_inside_tbl[sector]` and calls
  `event()` with the resulting index.

All messages render in the parchment/message area of the HI bar (640×57
hires region at screen bottom).

### event_msg (indices 0–38)

Source: `narr.asm` line 11+; triggers in `fmain.c` and `fmain2.c`.

| Idx | Text | Original trigger | Rust status |
|-----|------|------------------|-------------|
| 0 | "% was getting rather hungry." | `hunger == 35` (`fmain.c:2628`) | ✅ `game_state.rs` |
| 1 | "% was getting very hungry." | `hunger == 60` (`fmain.c:2630`) | ✅ `game_state.rs` |
| 2 | "% was starving!" | `hunger > 90 && vitality > 5`, every 8 ticks (`fmain.c:2638`) | ✅ `game_state.rs` |
| 3 | "% was getting tired." | `fatigue == 70` (`fmain.c:2649`) | ✅ `game_state.rs` |
| 4 | "% was getting sleepy." | `fatigue == 90` (`fmain.c:2651`) | ✅ `game_state.rs` (was bug: checked hunger) |
| 5 | "% was hit and killed!" | Combat death via `checkdead(0, 5)` (`fmain.c:3409`) | ❌ Not wired |
| 6 | "% was drowned in the water!" | `environ == 30 && (cycle & 7) == 0` — deep water (`fmain.c:2145`) | ❌ Not wired |
| 7 | "% was burned in the lava." | Lava damage via `checkdead(i, 7)` (`fmain.c:2140`) | ❌ Not wired |
| 8 | "% was turned to stone by the witch." | Witch petrify special (`fmain2.c:1226`) | ❌ Not wired |
| 9 | "% started the journey in his home village of Tambry" | Game start / brother transition (`fmain.c:2556`) | ✅ `gameplay_scene.rs` |
| 10 | "as had his brother before him." | Phillip's start (one brother dead) | ✅ `gameplay_scene.rs` |
| 11 | "as had his brothers before him." | Kevin's start (two brothers dead) | ✅ `gameplay_scene.rs` |
| 12 | "% just couldn't stay awake any longer!" | `fatigue > 170 && vitality <= 5` (`fmain.c:2640`) | ✅ `game_state.rs` |
| 13 | "% was feeling quite full." | After eating when `hunger < 0` (`fmain2.c:2139`) | ❌ Not wired |
| 14 | "% was feeling quite rested." | After sleeping/resting (`fmain.c:4238`) | ❌ Not wired |
| 15 | "Even % would not be stupid enough to draw weapon in here." | Draw weapon in sanctuary (`xtype == 81`, `fmain.c:1591`) | ❌ Not wired |
| 16 | "A great calming influence comes over %..." | Draw weapon in temple (`xtype == 82`, `fmain.c:1593`) | ❌ Not wired |
| 17 | "% picked up a scrap of paper." | Picking up scrap object (`fmain.c:4067`) | ✅ `gameplay_scene.rs` |
| 18 | "It read: \"Find the turtle!\"" | Scrap content (quest hint) | ✅ `gameplay_scene.rs` |
| 19 | "It read: \"Meet me at midnight at the Crypt...\"" | Scrap content (quest hint) | ✅ `gameplay_scene.rs` |
| 20 | "% looked around but discovered nothing." | LOOK action finds nothing (`fmain.c:4069`) | ❌ Not wired |
| 21 | "% does not have that item." | GIVE item not in inventory (`stuff[4+hit] == 0`, `fmain.c:4076`) | ❌ Not wired |
| 22 | "% bought some food and ate it." | Tavern purchase food (`i == 0`, `fmain.c:4286`) | ❌ Not wired |
| 23 | "% bought some arrows." | Tavern purchase arrows (`i == 8`, `fmain.c:4290`) | ❌ Not wired |
| 24 | "% passed out from hunger!" | `hunger > 140 && vitality <= 5` (`fmain.c:2643`) | ✅ `game_state.rs` |
| 25 | "% is not sleepy." | SLEEP when `fatigue < 50` (`fmain.c:2179`) | ❌ Not wired |
| 26 | "% was tired, so he decided to lie down and sleep." | SLEEP action accepted (`fmain.c:2175`) | ✅ `gameplay_scene.rs` |
| 27 | "% perished in the hot lava!" | Fatal lava, `fiery_death && environ > 15` (`fmain.c:2140`) | ❌ Not wired |
| 28 | "It was midnight." | `dayperiod == 0` transition (`fmain.c:2383`) | ❌ Not wired |
| 29 | "It was morning." | `dayperiod == 4` transition (`fmain.c:2385`) | ❌ Not wired |
| 30 | "It was midday." | `dayperiod == 6` transition (`fmain.c:2388`) | ❌ Not wired |
| 31 | "Evening was drawing near." | `dayperiod == 9` transition (`fmain.c:2392`) | ❌ Not wired |
| 32 | "Ground is too hot for swan to land." | Swan landing on lava (`riding == 11 && fiery_death`, `fmain.c:1597`) | ❌ Not wired |
| 33 | "Flying too fast to dismount." | Dismount while flying fast (`riding == 11 && velocity > threshold`, `fmain.c:1607`) | ❌ Not wired |
| 34 | "\"They're all dead!\" he cried." | All three brothers dead — game over (`fmain.c:2560`) | ❌ Not wired |
| 35 | "No time for that now!" | Invalid action during combat/encounter (`fmain.c:1560`) | ❌ Not wired |
| 36 | "% put an apple away for later." | Fruit pickup when not hungry | ✅ `gameplay_scene.rs` |
| 37 | "% ate one of his apples." | EAT food action | ✅ `gameplay_scene.rs` |
| 38 | "% discovered a hidden object." | LOOK action finds object (`flag == 1`, `fmain.c:4067`) | ❌ Not wired |

### place_msg (indices 0–26)

Source: `narr.asm` line 100+; triggered by `find_place()` in `fmain.c`.

The `find_place()` function checks the player's map sector against
`_place_tbl[]` (outdoor) or `_inside_tbl[]` (indoor, when `region_num > 7`).
Each sector maps to a message index. Index 0 means "no message" (empty
string). Messages fire once per sector entry.

| Idx | Text | Location |
|-----|------|----------|
| 0 | *(empty — no message)* | Default / untagged sectors |
| 1 | *(empty — reserved)* | Reserved |
| 2 | "% returned to the village of Tambry." | Tambry village |
| 3 | "% came to Vermillion Manor." | Vermillion Manor |
| 4 | "% reached the Mountains of Frost" | Mountains of Frost |
| 5 | "% reached the Plain of Grief." | Plain of Grief |
| 6 | "% came to the city of Marheim." | Marheim city |
| 7 | "% came to the Witch's castle." | Witch's castle |
| 8 | "% came to the Graveyard." | Graveyard |
| 9 | "% came to a great stone ring." | Stone ring (teleport) |
| 10 | "% came to a watchtower." | Watchtower / lighthouse |
| 11 | "% traveled to the great Bog." | Great Bog / swamp |
| 12 | "% came to the Crystal Palace." | Crystal Palace |
| 13 | "% came to mysterious Pixle Grove." | Pixie shrine |
| 14 | "% entered the Citadel of Doom." | Citadel of Doom |
| 15 | "% entered the Burning Waste." | Burning Waste / desert |
| 16 | "% found an oasis." | Oasis |
| 17 | "% came to the hidden city of Azal." | Hidden city of Azal |
| 18 | "% discovered an outlying fort." | Outlying fort |
| 19 | "% came to a small keep." | Small keep |
| 20 | "% came to an old castle." | Old castle |
| 21 | "% came to a log cabin." | Log cabin |
| 22 | "% came to a dark stone tower." | Dark stone tower |
| 23 | "% came to an isolated cabin." | Isolated / swamp cabin |
| 24 | "% came to the Tombs of Hemsath." | Tombs of Hemsath |
| 25 | "% reached the Forbidden Keep." | Forbidden Keep |
| 26 | "% found a cave in the hillside." | Dragon cave |

**Rust status:** Config loaded in `game_library.rs` but **entirely unwired**.
The `find_place()` sector-lookup mechanism is not yet implemented. Requires
sector-to-message mapping data (from `_place_tbl[]` in `narr.asm`) and a
trigger in the movement/region-transition code.

### inside_msg (indices 0–22)

Source: `narr.asm` line 200+; triggered by `find_place()` when
`region_num > 7` (interior regions).

| Idx | Text | Location type |
|-----|------|---------------|
| 0 | *(empty — no message)* | Default |
| 1 | *(empty — reserved)* | Reserved |
| 2 | "% came to a small chamber." | Small chamber |
| 3 | "% came to a large chamber." | Large chamber |
| 4 | "% came to a long passageway." | Long passageway |
| 5 | "% came to a twisting tunnel." | Twisting tunnel |
| 6 | "% came to a forked intersection." | Forked intersection |
| 7 | "He entered the keep." | Keep interior |
| 8 | "He entered the castle." | Castle interior |
| 9 | "He entered the castle of King Mar." | Castle of King Mar |
| 10 | "He entered the sanctuary of the temple." | Temple sanctuary |
| 11 | "% entered the Spirit Plane." | Spirit Plane / astral |
| 12 | "% came to a large room." | Large room |
| 13 | "% came to an octagonal room." | Octagonal room |
| 14 | "% traveled along a stone corridor." | Stone corridor |
| 15 | "% came to a stone maze." | Stone maze |
| 16 | "He entered a small building." | Small building |
| 17 | "He entered the building." | Generic building |
| 18 | "He entered the tavern." | Tavern |
| 19 | "He went inside the inn." | Inn |
| 20 | "He entered the crypt." | Crypt / tomb |
| 21 | "He walked into the cabin." | Cabin |
| 22 | "He unlocked the door and entered." | Locked-door entry |

**Rust status:** Config loaded in `game_library.rs` but **entirely unwired**.
Same `find_place()` mechanism as `place_msg`, but uses `_inside_tbl[]` when
`region_num > 7`.

### speeches (indices 0–60)

Source: `narr.asm` line 351+; triggered via NPC talk dispatch in
`fmain.c:4167–4271` and combat/item-use handlers in `fmain2.c`.

#### Creature combat dialogue (talk to hostile enemy)

Triggered by TALK menu when target is `an->type == ENEMY`. The creature's
`race` field directly indexes the speech: `speak(an->race)`.

| Idx | Text | Creature | Rust |
|-----|------|----------|------|
| 0 | "% attempted to communicate with the Ogre..." | Ogre (race 0) | ❌ |
| 1 | "\"Human must die!\" said the goblin-man." | Orc/goblin (race 1) | ✅ |
| 2 | "\"Doom!\" wailed the wraith." | Wraith (race 2) | ✅ |
| 3 | "A clattering of bones was the only reply." | Skeleton (race 3) | ✅ |
| 4 | "% knew that it is a waste of time to talk to a snake." | Snake (race 4) | ✅ |
| 5 | "..." | Salamander (race 5) | ❌ |
| 6 | "There was no reply." | Loraii/Spider (race 6) | ✅ |
| 7 | "\"Die, foolish mortal!\" he said." | Necromancer (race 7) | ❌ |

#### Named NPC dialogue (setfig system)

Named NPCs use `an->race & 0x7f` as a setfig index. Each NPC has
conditional dialogue based on quest state, character stats, and time of day.

**Wizard** (setfig 0) — `fmain.c:4189–4193`:

| Idx | Text | Condition | Rust |
|-----|------|-----------|------|
| 27 | "\"Kind deeds could gain thee a friend from the sea.\"" | `kind >= 10 && goal == 0` | ✅ (default only) |
| 28 | "\"Seek the place that is darker than night...\"" | `kind >= 10 && goal == 1` | ❌ goal not tracked |
| 29 | "\"Like the eye itself, a crystal Orb can help...\"" | `kind >= 10 && goal == 2` | ❌ goal not tracked |
| 30 | "\"The Witch lives in the dim forest of Grimwood...\"" | `kind >= 10 && goal == 3` | ❌ goal not tracked |
| 31 | "\"Only the light of the Sun can destroy the Witch's Evil.\"" | `kind >= 10 && goal == 4` | ❌ goal not tracked |
| 32 | "\"The maiden you seek lies imprisoned...\"" | `kind >= 10 && goal == 5` | ❌ goal not tracked |
| 33 | "\"Tame the golden beast...\"" | `kind >= 10 && goal == 6` | ❌ goal not tracked |
| 34 | "\"Just what I needed!\" he said." | `kind >= 10 && goal == 7` | ❌ goal not tracked |
| 35 | "\"Away with you, young ruffian!\"" | `kind < 10` | ✅ |

**Priest/Cleric** (setfig 1) — `fmain.c:4195–4208`:

| Idx | Text | Condition | Rust |
|-----|------|-----------|------|
| 36 | "\"You must seek your enemy on the spirit plane...\"" | `kind >= 10 && daynight % 3 == 0` | ✅ |
| 37 | "\"When you wish to travel quickly, seek the power of the Stones.\"" | `kind >= 10 && daynight % 3 == 1` | ✅ |
| 38 | "\"Since you are brave of heart, I shall Heal...\"" + heals player | `kind >= 10 && daynight % 3 == 2` | ✅ |
| 39 | "\"Ah! You have a writ from the king...golden statues...\"" | `stuff[28] == 1 && ob_listg[10].ob_stat == 0` (first visit with writ) | ❌ quest state |
| 40 | "\"Repent, Sinner!...\"" | `kind < 10` | ✅ |

**Guard** (setfig 2, 3) — `fmain.c:4209`:

| Idx | Text | Condition | Rust |
|-----|------|-----------|------|
| 15 | "\"State your business!\" said the guard." | Always (auto-speak on proximity) | ✅ |

**Princess** (setfig 4) — `fmain.c:4214`:

| Idx | Text | Condition | Rust |
|-----|------|-----------|------|
| 16 | "\"Please, sir, rescue me...\"" | `ob_list8[9].ob_stat` (imprisoned) | ✅ |

**King** (setfig 5) — `fmain.c:4218–4221`:

| Idx | Text | Condition | Rust |
|-----|------|-----------|------|
| 17 | "\"I cannot help you, young man...\"" | Default (no writ, princess imprisoned) | ✅ |
| 18 | "\"Here is a writ designating you as my official agent...\"" | `stuff[28] == 0` — gives writ to player | ❌ quest state |
| 19 | "\"I already gave the golden statue to the other young man.\"" | `ob_listg[10].ob_stat != 0` — statue already given | ❌ quest state |

**Noble (Lord Trane)** (setfig 6) — `fmain.c:4223`:

| Idx | Text | Condition | Rust |
|-----|------|-----------|------|
| 20 | "\"If you could rescue the king's daughter...\"" | Always | ✅ |

**Sorceress** (setfig 7) — `fmain.c:4226–4232`:

| Idx | Text | Condition | Rust |
|-----|------|-----------|------|
| 45 | "\"Welcome. Here is one of the five golden figurines...\"" | One-time gift (sets `ob_stat`) | ✅ (partial) |

**Bartender** (setfig 8) — `fmain.c:4236–4241`:

| Idx | Text | Condition | Rust |
|-----|------|-----------|------|
| 12 | "\"Would you like to buy something?...\"" | `dayperiod > 7` (late night) | ✅ |
| 13 | "\"Good Morning...Hope you slept well.\"" | `fatigue < 5` (just rested) | ✅ |
| 14 | "\"Have a drink!\"" | Default (other times) | ✅ |

**Witch** (setfig 9) — `fmain.c:4243`:

| Idx | Text | Condition | Rust |
|-----|------|-----------|------|
| 46 | "\"Look into my eyes and Die!!\"" | Always (auto-speak) | ✅ |

**Spectre** (setfig 10) — `fmain.c:4247`:

| Idx | Text | Condition | Rust |
|-----|------|-----------|------|
| 47 | "\"HE has usurped my place as lord of undead...\"" | First meeting (no bones yet) | ✅ |
| 48 | "\"Good! That spirit now rests...Take this crystal shard.\"" | Player gives bones (`stuff[29] == 1`) | ❌ quest state |

**Ghost** (setfig 11) — `fmain.c:4251`:

| Idx | Text | Condition | Rust |
|-----|------|-----------|------|
| 49 | "\"I am the ghost of your dead brother...\"" | Always | ✅ |

**Ranger** (setfig 12) — `fmain.c:4252–4256`:

| Idx | Text | Condition | Rust |
|-----|------|-----------|------|
| 9 | "\"Nice weather we're having...\"" | Generic ranger greeting (not dragon-direction) | ❌ goal not tracked |
| 10 | "\"Good luck, sonny!...\"" | Ranger variation | ❌ goal not tracked |
| 11 | "\"If you need to cross the lake...raft north of here.\"" | Ranger variation (raft hint) | ❌ goal not tracked |
| 22 | "\"The dragon's cave is directly north...\"" | `region_num == 2` (Grimwood) | ✅ |
| 53 | "\"The dragon's cave is east...\"" | `region_num != 2 && goal == 0` | ✅ (default only) |
| 54 | "\"The dragon's cave is west...\"" | `region_num != 2 && goal == 1` | ❌ goal not tracked |
| 55 | "\"The dragon's cave is south...\"" | `region_num != 2 && goal == 2` | ❌ goal not tracked |

**Beggar** (setfig 13) — `fmain.c:4259, 4372–4381`:

| Idx | Text | Condition | Rust |
|-----|------|-----------|------|
| 23 | "\"Alms! Alms for the poor!\"" | TALK (always) | ✅ |
| 24 | "\"I have a prophecy...seek two women...\"" | GIVE gold, `goal == 0` | ❌ quest state |
| 25 | "\"Lovely Jewels, glint in the night...\"" | GIVE gold, `goal == 1` | ❌ quest state |
| 26 | "\"Where is the hidden city?...\"" | GIVE gold, `goal == 2` | ❌ quest state |

#### Other dialogue contexts

| Idx | Text | Trigger | Rust |
|-----|------|---------|------|
| 8 | "\"No need to shout, son!\"" | YELL near any NPC within 35 units (`fmain.c:4161`) | ✅ |
| 21 | "\"Sorry, I have no use for it.\"" | GIVE non-gold item to NPC that doesn't want it | ❌ |
| 41 | "\"None may enter the sacred shrine...\"" | Dream Knight blocks entry (combat challenge) | ❌ quest |
| 42 | "\"Your prowess in battle is great...earned the right...\"" | Dream Knight defeated — grants entry | ❌ quest |
| 43 | "\"So this is the so-called Hero...Simply Pathetic.\"" | Necromancer encounter (Spirit Plane boss fight) | ❌ quest |
| 44 | "% gasped. The Necromancer had been transformed..." | Necromancer defeated — quest complete | ❌ quest |
| 50 | "% gave him some gold coins..." | GIVE gold to non-beggar NPC (`fmain.c:4381`) | ❌ |
| 51 | "\"Sorry, but I have nothing to sell.\"" | BUY from NPC with no shop inventory | ❌ |
| 52 | *(empty)* | Placeholder / null | N/A |
| 56 | "\"Oh, thank you for saving my eggs...\"" | Turtle egg rescue (`stuff[6] == 0`, `fmain.c:4264`) | ❌ quest state |
| 57 | "\"Just hop on my back...\"" | Turtle already has shell (`stuff[6] == 1`, `fmain.c:4267`) | ❌ quest state |
| 58 | "\"Stupid fool, you can't hurt me with that!\"" | Melee attack on witch or unarmed vs necromancer (`fmain2.c:321`) | ❌ |
| 59 | "\"Your magic won't work here, fool!\"" | MAGIC in Necromancer's zone (`extn->v3 == 9`) | ❌ |
| 60 | "The Sunstone has made the witch vulnerable!" | USE Sunstone item against witch (`witchflag`) | ❌ |

### Implementation gap summary

Grouped by feature system, with the message indices each gap covers:

| Feature gap | event_msg | speeches | Notes |
|-------------|-----------|----------|-------|
| Place message system (`find_place()`) | — | — | All 27 place_msg + 23 inside_msg unwired |
| Death type messages | 5, 6, 7, 8, 27, 34 | — | Combat, drowning, lava, petrification, all-dead |
| Time-of-day announcements | 28, 29, 30, 31 | — | `dayperiod` transitions |
| Terrain/environment warnings | 32, 33 | — | Swan-on-lava, flying-too-fast |
| Post-action feedback | 13, 14, 25 | — | Felt full, felt rested, not sleepy |
| Search/discovery system | 20, 38 | — | LOOK found nothing / found hidden object |
| Shopping messages | 22, 23 | — | Buy food, buy arrows |
| Sanctuary weapon restriction | 15, 16 | — | Draw weapon in sacred zones |
| Combat-time action restriction | 35 | — | "No time for that now!" |
| Give-item dialogue | 21 | 21, 50 | Missing item, unwanted item, give gold |
| Creature combat dialogue | — | 0, 5, 7 | Ogre, Salamander, Necromancer talk |
| Wizard quest variations | — | 28–34 | `goal`-based hint cycling |
| Ranger dialogue variations | — | 9–11, 54, 55 | Generic greetings + direction variants |
| Beggar quest dialogue | — | 24–26 | GIVE-gold prophecy cycling |
| King quest states | — | 18, 19 | Writ granting, statue given |
| Priest writ quest | — | 39 | Writ → golden statue exchange |
| Spectre bones quest | — | 48 | Give bones → crystal shard |
| Turtle quest dialogue | — | 56, 57 | Egg rescue + ride offer |
| Dream Knight quest | — | 41, 42 | Shrine challenge + entry granted |
| Necromancer quest | — | 43, 44 | Boss encounter + defeat |
| Witch combat responses | — | 58, 60 | Invulnerability + Sunstone |
| Necromancer combat responses | — | 59 | Magic blocked in zone |
| Non-seller response | — | 51 | BUY from non-merchant |
