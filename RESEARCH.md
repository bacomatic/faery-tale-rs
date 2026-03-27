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

**`brave`** — melee weapon reach radius `bv = brave/20 + 5` (0–15 cap). Each enemy kill grants `brave++`. Hero death subtracts 5 from `luck`. Vitality cap = `15 + brave/4`.

**`luck`** — modifies fairy spawning: fairy appears when `luck < 1 && goodfairy < 200`. Reduced by enemy hits to player (`luck -= 2` in some death paths, `-5` on hero death).

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
| >100 (every 8 ticks, if vitality > 5) | `vitality -= 2` if also `fatigue > 160` |
| >90 (every 8 ticks, if vitality > 5) | event(2) — starvation warning |
| >140 (every 8 ticks) | event(24), `hunger` clamped to 130, `state = SLEEP` (collapse) |

**Fatigue progression:**

| `fatigue` value | Event |
|-----------------|-------|
| 70              | event(3) — "Weary" message |
| >170 (every 8 ticks, vitality ≤ 5) | event(12), `state = SLEEP` (collapse from exhaustion) |

`fatigue` decrements by 1 per daynight tick passively. Sleep in combat (battleflag) or very low fatigue triggers forced `SLEEP` state. Sleeping on interior tiles 161, 52, 162, or 53 after `sleepwait` reaches 30 also triggers sleep (if `fatigue > 50`).

`eat(amt)`: `hunger -= amt; if hunger < 0 → hunger = 0, event(13)` ("Feeling better"). Used for Fruit (amt=30) and buying food at inns (amt=50).

**Vitality recovery**: every 1024 `daynight` ticks (`(daynight & 0x3ff) == 0`), if `vitality < 15 + brave/4` and not DEAD: `vitality++`, prints HP display.

---

## Day/Night Cycle

`daynight` is a `USHORT` counter [0..23999], incremented by 1 per game tick (30 Hz):
- 24000 ticks = one full in-game day ≈ 800 seconds real time (≈13.3 minutes)

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

`fade_page(r, g, b, limit, colors)` applies per-frame colour scaling. Night limit floor: r≥10, g≥25, b≥60 (ensures blue-tinted night). `light_timer` (Green Jewel light effect) temporarily equalises R and G channels.

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
- Horizontal door (`type & 1 == 1`): player enters only if `(hero_y & 0x10) == 0` (lower half of tile)
- Vertical door (`type & 1 == 0`): player enters only if `(hero_x & 15) > 6`
- Cave type: entry offset `(xc2+24, yc2+16)`, exit offset `(xc1-4, yc1+16)`
- Horizontal non-cave: entry `(xc2+16, yc2)`, exit `(xc1+16, yc1+34)`
- Vertical: entry `(xc2-1, yc2+16)`, exit `(xc1+20, yc1+16)`

**`secs` field** sets `new_region` on crossing: `secs==1` → `new_region=8` (indoor), `secs==2` → `new_region=9` (cave/dungeon). Indoor regions use a linear scan of `doorlist`; outdoor uses binary search.

**Locked doors (`doorfind`)**: called when player tries to walk through an impassable tile. The USE→KEYS menu calls `doorfind(newx(hero_x,i,16), newy(...), keytype)` for all 9 directions. Consumes one key from `stuff[KEYBASE + hit]` on success.

---

## Terrain Collision System

`terra_mem[tile_idx * 4]` holds 4 bytes per tile index (from `sector_mem`/`minimap[]`):

| Offset | Name | Description |
|---|---|---|
| +0 | `maptag` | Bitmask used by `maskit()` for sprite depth-sorting |
| +1 | `terrain` | **Two independent nibbles**: upper nibble (`>> 4`) = walking collision type returned by `px_to_im` (0–15, used by `proxcheck`); lower nibble (`& 0x0f`) = sprite depth/masking block type (0–7, used by `maskit`) |
| +2 | `tiles` | 8-bit terrain feature mask — one bit per sub-tile of the 16×32 tile (2 halves × 4 y-bands). If the bit for the hero's sub-cell is clear, `px_to_im` returns 0 (passable) regardless of terrain type |
| +3 | `big_colors` | Colour zone for lighting |

This 4-byte-per-tile table is the compiled output of `terrain.c`'s `load_images()` run on the original raw tileset files. Each tileset contributes 64 tiles → 256 B to `terra_mem` (two tilesets per 512-byte ADF block).

**Walking collision type** (upper nibble of `terra_mem[cm+1]`, i.e. `>> 4`), returned by `px_to_im` and tested in `prox`:

| Type | Behaviour |
|------|-----------|
| 0 | Open terrain — passable |
| 1 | Hard rock — blocks both feet |
| 2–5 | Water — slows movement; passable on foot |
| 8–9 | Blocks left foot only (`is_hard_block_left`) |
| 10–15 | Blocks both feet (`is_hard_block_right` and `is_hard_block_left`) |
| 12 | mountain3 — passable if hero holds Shard (`stuff[30]`) |
| 15 | Door — triggers `doorfind()` lock/key mechanic |

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
| 10 | Green Jewel | 6 | `light_timer += 760` | ~760 game ticks | **Illumination**: `day_fade()` boosts red channel (`r1 = g1` when `r1 < g1`), adds +200 to lightlevel calculation. Makes night as bright as day. Palette color 31 unaffected. Timer decrements each main-loop tick |
| 11 | Glass Vial | 7 | Heal | Instant | Restores `rand8() + 4` vitality (4–11 HP). Capped at max vitality `15 + brave/4`. Prints "That feels a lot better!" if not already at max |
| 12 | Crystal Orb | 8 | `secret_timer += 360` | ~360 game ticks | **Reveal secrets**: in region 9 (underworld/dungeons), changes palette color 31 from 0x0445 (dark) to 0x00F0 (bright green), revealing hidden passages. Timer decrements each tick |
| 13 | Bird Totem | 9 | World map | Instant | **Minimap display**: draws the world map (`bigdraw(map_x, map_y)`) with hero position marked by "+" at computed pixel offset. Only works in overworld (`region_num < 8`); blocked while `riding > 1`. Sets `viewstatus=1`, waits for keypress |
| 14 | Gold Ring | 10 | `freeze_timer += 100` | ~100 game ticks | **Time stop**: all non-hero figures skip movement updates (`freeze_timer && i > 0` → goto statc). Enemies cannot attack, missiles don't fire. `daynight` clock pauses. Hero can loot frozen enemies' bodies. Also prevents melee hit checks for non-hero figures |
| 15 | Jade Skull | 11 | Mass kill | Instant | **Death spell**: iterates all figures `i=1..anix-1`; if enemy type with vitality > 0, race < 7 → sets vitality=0, calls `checkdead(i, 0)`, decrements `brave` by 1 per kill. Then triggers battle aftermath event(34) if battleflag set |

#### Keys (stuff 16–21, KEYBASE=16)

Accessed via KEYS submenu (USE → Key → color selection). Each use tests 9 positions around hero (8 compass directions + center) via `doorfind(x, y, hit+1)`. If a matching locked door is found, the key is consumed (`stuff[hit + KEYBASE]--`) and door opens. If no matching door: "% tried a [Key Name] but it didn't fit."

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

Loaded from ADF block 920 (11 blocks = 5,632 bytes, `SAMPLE_SZ`) into `sample_mem`. Six samples packed sequentially as length-prefixed raw signed 8-bit PCM:

```
[4-byte big-endian length][PCM bytes] × 6
```

`effect(num, speed)` calls `playsample(sample[num], sample_size[num]/2, speed)` where `speed` is an Amiga Paula period register value — higher = slower/lower pitch.

| Index | Trigger | Speed range | Context |
|-------|---------|-------------|---------|
| 0 | Hero is hit by melee | 800 + bitrand(511) | `dohit` with j==0 |
| 1 | Weapon near-miss | 150 + rand256() | Proximity ≤ bv+2, not wand |
| 2 | Arrow/bolt hits player | 500 + rand64() | `dohit` with i==-1 |
| 3 | Monster is hit by melee | 400 + rand256() | `dohit` with j>0 |
| 4 | Arrow hits target | 400 + rand256() | Missile impact |
| 5 | Magic/fireball hit or monster death | 3200 + bitrand(511) | `dohit` with i==-2, some deaths |

---

## Sprite / Shape File Layout (ADF)

All character sprites stored as interleaved Amiga bitplanes. `cfiles[]` in `fmain2.c` maps logical sprite IDs to ADF locations:

```c
struct cfile_info {
    UBYTE width;     // width in 16-pixel words
    UBYTE height;    // height in pixels
    UBYTE count;     // total animation frame count
    UBYTE numblocks; // ADF blocks to read
    UBYTE seq_num;   // which seq_list slot (PHIL/OBJECTS/RAFT/ENEMY/SETFIG/CARRIER/DRAGON)
    USHORT file_id;  // ADF block number
};
```

Frame byte size = `width * height * 2` (2 bytes = one row of one 16-wide bitplane). Total size per animation file = `width * height * 2 * count * 5` (5 bitplanes) + `width * height * 2 * count` (mask plane) = `size * 6`.

| cfile | ADF block | Blocks | Width×Height | Frames | Slot | Notes |
|-------|-----------|--------|--------------|--------|------|-------|
| 0 | 1376 | 42 | 1×32 | 67 | PHIL | Julian player sprites |
| 1 | 1418 | 42 | 1×32 | 67 | PHIL | Phillip player sprites |
| 2 | 1460 | 42 | 1×32 | 67 | PHIL | Kevin player sprites |
| 3 | 1312 | 36 | 1×16 | 116 | OBJECTS | World objects/items |
| 4 | 1348 | 3  | 2×32 | 2   | RAFT | Raft sprite |
| 5 | 1351 | 20 | 2×32 | 16  | CARRIER | Turtle |
| 6 | 960  | 40 | 1×32 | 64  | ENEMY | Ogre / Orc |
| 7 | 1080 | 40 | 1×32 | 64  | ENEMY | Ghost / Wraith / Skeleton / Salamander |
| 8 | 1000 | 40 | 1×32 | 64  | ENEMY | DKnight / Spider |
| 9 | 1040 | 40 | 1×32 | 64  | ENEMY | Necromancer / Loraii / Farmer |
| 10 | 1160 | 12 | 3×40 | 5  | DRAGON | Dragon |
| 11 | 1120 | 40 | 4×64 | 8  | CARRIER | Bird |
| 12 | 1376 | 40 | 1×32 | 64 | ENEMY | Snake / Salamander |
| 13 | 936  | 5  | 1×32 | 8  | SETFIG | Wizard / Priest |
| 14 | 931  | 5  | 1×32 | 8  | SETFIG | Guards / Princess / King / Noble / Sorceress |
| 15 | 941  | 5  | 1×32 | 8  | SETFIG | Bartender |
| 16 | 946  | 5  | 1×32 | 8  | SETFIG | Witch / Spectre / Ghost |
| 17 | 951  | 5  | 1×32 | 8  | SETFIG | Ranger / Beggar |

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

**`set_course(object, target_x, target_y, mode)`**:
- `mode 1`: ATTACK1 (add deviation if dist < 40)
- `mode 2`: ATTACK2 (add deviation if dist < 30)
- `mode 3`: FLEE (reverse xdir/ydir)
- `mode 4`: diagonal-ok (no 2:1 bias reduction)
- `mode 5`: stand still after movement (no WALKING state)
- `mode 6`: use target_x/y directly as delta instead of computing offset from figure
- Direction LUT `com2[9] = {0,1,2,7,9,3,6,5,4}` maps `(xdir,ydir)` to the 8 compass directions; `j=9` → `STILL`

**`proxcheck(x, y, i)`**:
- Checks terrain collision via `prox(x, y)` (tile-based, returns non-zero for blocked)
- Wraith (race 2) bypasses tile collision
- Hero (i==0) can pass water/sink tiles (prox values 8, 9 are cleared)
- Figure-to-figure collision: 11px horizontal, 9px vertical exclusion zone; dead figures and rafts (j==1) are walkable

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

## World Map: Region Diagrams

### Region selection formula (`gen_mini()`, `fmain.c:3661–3690`)

```c
xs = (hero_x + 7) >> 8        // sector column of viewport centre
ys = (hero_y - 26) >> 8       // sector row of viewport centre
xr = (xs >> 6) & 1            // 0 = west column, 1 = east column
yr = (ys >> 5) & 3            // 0–3 = north → south band
region_num = xr + yr * 2      // 0–7 for outdoor; ≥8 hard-coded (indoor/dungeon)
```

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
