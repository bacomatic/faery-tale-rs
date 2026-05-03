# Game Mechanics Research — Terrain & Combat

Terrain/collision systems and the combat system.

> **Citation format**: `file.c:LINE` or `file.c:START-END`. Speech references: `speak(N)`.
> Split from [RESEARCH.md](RESEARCH.md). See the hub document for the full section index.

---

## 6. Terrain & Collision

### 6.1 Terrain Decode Chain — `px_to_im`

The `_px_to_im` function (`fsubs.asm:542-620`) converts absolute pixel coordinates `(x, y)` to a terrain type value (0–15). The chain has four stages.

#### Stage 1: Sub-Tile Bit Selection (`fsubs.asm:548-559`)

A mask byte `d4` selects one of 8 spatial zones within a 16×32 image tile. It starts as `0x80` and is shifted right by 4, 1, and 2 bits depending on bits 3 of `x`, bit 3 of `y`, and bit 4 of `y` respectively, yielding one of eight distinct bit positions. The 8 sub-tile zones allow a single tile to be partially passable — e.g., a wall tile with a walkable gap on one side. Full pseudo-code: [logic/terrain-collision.md § px_to_im](logic/terrain-collision.md#px_to_im).

#### Stage 2: Pixel to Sector Coordinates (`fsubs.asm:561-589`)

Pixel coordinates are reduced to image-tile coordinates (`imx = x >> 4`, `imy = y >> 5`; tiles are 16×32 pixels), then to sector coordinates (`secx = (imx >> 4) − xreg`, `secy = (imy >> 3) − yreg`). Sector X is wrapped: if bit 6 of `secx` is set, it is clamped to 0 or 63 based on bit 5 (`fsubs.asm:567-572`). Sector Y is clamped to 0–31 (`fsubs.asm:577-579`). The final map grid index is `secy * 128 + secx + xreg` (`fsubs.asm:581-583`). Full pseudo-code: [logic/terrain-collision.md § px_to_im](logic/terrain-collision.md#px_to_im).

#### Stage 3: Sector Tile Lookup (`fsubs.asm:590-604`)

The map grid index from Stage 2 selects a sector number (`sec_num = map_mem[map_index]`). The low bits of `imx` and `imy` (`imx & 15`, `imy & 7`) index into that sector's 16×8 = 128-tile layout, yielding the image ID (`sector_mem[sec_num*128 + (imy&7)*16 + (imx&15)]`). Full pseudo-code: [logic/terrain-collision.md § px_to_im](logic/terrain-collision.md#px_to_im).

#### Stage 4: Terrain Attribute Lookup (`fsubs.asm:606-616`)

Each image ID has a 4-byte record in `terra_mem`. Byte +2 is the sub-tile collision mask; if it ANDed with Stage 1's `d4` bit is zero, the zone is passable and `px_to_im` returns 0. Otherwise the function returns the high nibble of byte +1 — the terrain type code. The per-zone AND is what makes a single image tile partially passable. Full pseudo-code: [logic/terrain-collision.md § px_to_im](logic/terrain-collision.md#px_to_im).

### 6.2 Terrain Types

Terrain types are the high nibble of `terra_mem[image_id * 4 + 1]`. The source comment at `fmain.c:685-686` lists basic categories. Behavior is determined across `fmain.c:1770-1795` and `fsubs.asm:1596-1609`:

| Value | Meaning | `environ` Set | Gameplay Effect |
|-------|---------|---------------|-----------------|
| 0 | Open/passable | 0 | No effect — walkable ground |
| 1 | Impassable | — | Blocked by `_prox` at both probes (`fsubs.asm:1596-1597`, `1606-1607`) |
| 2 | Shallow water | 2 | Slow (speed 1, `fmain.c:1603`); gradual drowning if no turtle item (`fmain.c:1844-1846`) |
| 3 | Medium water | 5 | Same as type 2 with deeper `environ` |
| 4 | Deep water | 10 | Sinking begins at environ 15 (`fmain.c:1795`) |
| 5 | Very deep water | 30 | Death at environ 30 (`fmain.c:1784-1793`); sector 181 triggers underwater teleport to region 9 instead (`fmain.c:1784-1791`) |
| 6 | Slippery | −1 | Speed becomes 4 (`fmain.c:1601`, `1771`) |
| 7 | Velocity ice | −2 | Momentum-based physics with directional impulse (`fmain.c:1580-1595`) |
| 8 | Direction reversal | −3 | Walk backwards at speed −2 (`fmain.c:1600`, `1770`); reverses player input near Necromancer area |
| 9 | Pit/fall | — | If hero (i==0) and `xtype==52`: triggers FALL state, `luck -= 2` (`fmain.c:1766-1774`) |
| 10+ | Blocked | — | `_prox` blocks at second probe point (`fsubs.asm:1608-1609`); first probe blocks at ≥10 (`fsubs.asm:1598-1599`) |
| 12 | Crystal wall | — | Blocked unless `stuff[30]` (crystal shard) is held (`fmain.c:1611`). Exists only in terra set 8 (under+furnish, Region 8 building interiors) — tile index 93, found in 12 sectors: small chambers, twisting tunnels, forked intersections, and doom tower. **Not present** in the spirit world or dungeons (terra set 10 maps tile 93 to type 1/impassable). |
| 15 | Door | — | Triggers `doorfind()` attempt when player bumps it (`fmain.c:1609`) |

#### Environ Effects

The `environ` field on each actor tracks terrain depth/slide state (`fmain.c:1760-1800`). When the actor stands on a non-zero terrain type, `environ` is adjusted toward the target value. The sinker section applies to **all actors** — both hero and NPCs get environ updates identically (except where noted). Key thresholds:

- `environ > 15`: instant death — `vitality = 0` (`fmain.c:1845`)
- `environ > 2`: gradual drowning — `vitality--` per tick (`fmain.c:1846`)
- `stuff[23]` (turtle item) forces `environ = 0`, preventing all water damage (`fmain.c:1844`) — hero only

Water damage is gated by the `fiery_death` flag (`fmain.c:1843`); see [P17](PROBLEMS.md).

**NPC drowning immunity by race**: The drowning damage check at `fmain.c:1849-1851` uses the local variable `k`, which is repurposed from environ to `an->race` at `fmain.c:1802`. The condition `k != 2 && k != 3` therefore checks **race**, not environ — race 2 (wraith) and race 3 (skeleton) are immune to drowning damage. Additionally, wraiths (`race == 2`) and snakes (`race == 4`) have their terrain forced to 0 at `fmain.c:1639`, preventing them from entering water environ at all.

**Hero-only environ rules**:
- `i==0 && raftprox` → environ forced to 0 (turtle proximity prevents drowning, `fmain.c:1761`)
- `j == 9 && i==0 && xtype == 52` → FALL state + environ −2 (pit traps, `fmain.c:1766-1774`)
- `riding == 11` → environ forced to −2 before the loop (swan mount, `fmain.c:1464`)

For NPCs, terrain 9 (pit/fall) produces no environ change — no sinker branch matches, so `k` retains its previous value (`fmain.c:1766`).

### 6.3 Collision Detection

#### `_prox` — Terrain Probe (`fsubs.asm:1590-1614`)

Two terrain probes at offset positions from the actor's feet:

**Probe 1** (`fsubs.asm:1593-1599`): `(x+4, y+2)`
- Blocks if terrain type == 1 (impassable) or terrain type ≥ 10

**Probe 2** (`fsubs.asm:1601-1609`): `(x−4, y+2)`
- Blocks if terrain type == 1 or terrain type ≥ 8

The probes have **asymmetric thresholds**: the right probe blocks at ≥10 while the left blocks at ≥8. This means terrain types 8–9 (lava, pit) block only at the left probe. The original source has a comment error at `fsubs.asm:1603` — the comment says `; x + 4` but the instruction is `subq #4,d0` (x − 4).

Returns the blocking terrain code, or 0 if both probes pass.

#### `proxcheck` — Full Collision Test (`fmain2.c:277-293`)

Wraps `_prox` with three additional layers:

1. **Wraith bypass** (`fmain2.c:279-280`): Actors with `race == 2` (wraith) skip terrain collision entirely.
2. **Player override** (`fmain2.c:281-283`): For the hero (`i==0`), terrain types 8 and 9 are treated as passable — the player walks *into* lava and pits (they cause effects but don't block).
3. **Actor collision** (`fmain2.c:285-292`): Checks all active actors for bounding-box overlap (22×18 pixels: `|dx| < 11`, `|dy| < 9`). Skips self, slot 1 (raft/companion), CARRIER type (type 5), and DEAD actors. Returns 16 on actor collision.

#### Collision Deviation (`fmain.c:1612-1626`)

When any actor's movement is blocked, the game auto-deviates (this is NOT player-only):

1. Try `dir + 1` (clockwise) — if clear, commit
2. Try `dir − 2` (counterclockwise from original) — if clear, commit
3. All three blocked: player gets `frustflag++` with escalating animations; NPCs get `an->tactic = FRUST`. See [§5.4](RESEARCH-input-movement.md#player-collision-deviation-fmainc1612-1626) for threshold details and the `frustflag` scope bug.

### 6.4 Movement Speed by Terrain

Speed value `e` for `newx`/`newy` during WALKING, elaborating [§5.8](RESEARCH-input-movement.md#58-movement-speed-by-terrain). The speed assignment at `fmain.c:1599-1602` is a single if/else chain evaluated for every actor:

| Condition | Speed | Source | Applies To |
|-----------|-------|--------|------------|
| `i==0 && riding==5` | 3 | `fmain.c:1599` | Hero only (turtle mount) |
| `environ == −3` (lava) | −2 | `fmain.c:1600` | All actors (but NPCs blocked from terrain 8 by `proxcheck`) |
| `environ == −1` (slippery) | 4 | `fmain.c:1601` | All actors |
| `environ == 2` or `> 6` | 1 | `fmain.c:1602` | All actors |
| Default | 2 | `fmain.c:1602` | All actors |

Per-speed pixel displacement per frame (from direction vectors at `fsubs.asm:1277-1278`):

| Speed (e) | Cardinal px/frame | Diagonal px/frame |
|-----------|-------------------|--------------------|
| −2 | 3 (reversed) | 2 (reversed) |
| 1 | 1 | 1 |
| 2 | 3 | 2 |
| 3 | 4 | 3 |
| 4 | 6 | 4 |

Crystal shard (`stuff[30]`) overrides terrain type 12 blocking: `if (stuff[30] && j==12) goto newloc` (`fmain.c:1611`). Hero-only — NPCs are always blocked by terrain ≥ 10.

**NPC freeze**: When `freeze_timer` is active, all non-hero actors (`i > 0`) skip movement processing entirely via `goto statc` (`fmain.c:1473`). Talin's inline comment `/* what about wizard? */` suggests this blanket freeze may have been broader than intended.

### 6.5 Memory Layout

#### `sector_mem` — Sector Tile Data

Allocated as `SECTOR_SZ = (128 * 256) + 4096 = 36864` bytes (`fmain.c:643`):

- **Bytes 0–32767**: 256 sectors × 128 bytes each. Each sector is a 16×8 grid of tile IDs (one byte per tile).
- **Bytes 32768–36863**: Region map data (`map_mem`), pointed to via `map_mem = sector_mem + SECTOR_OFF` where `SECTOR_OFF = 32768` (`fmain.c:921`).

#### `map_mem` — Region Map Grid

4096 bytes (part of `sector_mem` allocation). Organized as 128 columns × 32 rows. Each byte is a sector number (0–255). Indexed by `secy * 128 + secx + xreg`. The `xreg`/`yreg` values are region origin offsets set during region loading.

#### `terra_mem` — Terrain Attributes

Allocated as 1024 bytes in MEMF_CHIP (`fmain.c:928`). Two halves of 512 bytes, loaded from separate terrain data tracks:

- `terra_mem[0..511]`: from `TERRA_BLOCK + nd->terra1` (`fmain.c:3567`)
- `terra_mem[512..1023]`: from `TERRA_BLOCK + nd->terra2` (`fmain.c:3572`)

where `TERRA_BLOCK = 149` (`fmain.c:608`).

Each image tile has a 4-byte entry:

| Byte | Name | Purpose |
|------|------|---------|
| 0 | `maptag` | Image characteristics / mask data (used by `maskit`: `fmain.c:2595`) |
| 1 | terrain | High nibble = terrain type (0–15); low nibble = sprite mask application rule |
| 2 | `tiles` | 8-bit sub-tile collision bitmask |
| 3 | `big_colors` | Dominant tile color (used for rendering) |

512 bytes / 4 = 128 entries per half — supports up to 128 distinct image tiles per terrain file.

### 6.6 Region Loading

Each region has a `struct need` entry (`ftale.h:106-108`) in the `file_index[10]` array (`fmain.c:615-625`). The `terra1`/`terra2` fields select which terrain attribute files to load:

| Region | Description | terra1 | terra2 | Source |
|--------|-------------|--------|--------|--------|
| 0 (F1) | Snowy region | 0 | 1 | `fmain.c:615` |
| 1 (F2) | Witch wood | 2 | 3 | `fmain.c:616` |
| 2 (F3) | Swampy region | 2 | 1 | `fmain.c:617` |
| 3 (F4) | Plains and rocks | 2 | 3 | `fmain.c:618` |
| 4 (F5) | Desert area | 0 | 4 | `fmain.c:619` |
| 5 (F6) | Bay/city/farms | 5 | 6 | `fmain.c:620` |
| 6 (F7) | Volcanic | 7 | 4 | `fmain.c:621` |
| 7 (F8) | Forest/wilderness | 5 | 6 | `fmain.c:622` |
| 8 (F9) | Inside buildings | 8 | 9 | `fmain.c:623` |
| 9 (F10) | Dungeons/caves | 10 | 9 | `fmain.c:624` |

Terrain data is only reloaded when `terra1` or `terra2` differ from `current_loads` (`fmain.c:3565-3573`), avoiding redundant disk I/O when adjacent regions share terrain files.

### 6.7 Mask Application Rules

The low nibble of `terra_mem[image_id * 4 + 1]` controls sprite occlusion during rendering (`fmain.c:2579-2596`, comment at `fmain.c:689-691`):

| Value | Rule | Source |
|-------|------|--------|
| 0 | Never apply mask | `fmain.c:2580` |
| 1 | Apply when sprite is below (down) | `fmain.c:2582` |
| 2 | Apply when sprite is to the right | `fmain.c:2584` |
| 3 | Always apply (unless flying) | `fmain.c:2586` |
| 4 | Only when down AND right | `fmain.c:2588` |
| 5 | Only when down OR right | `fmain.c:2590` |
| 6 | Full mask if above ground level; partial otherwise | `fmain.c:2592` |
| 7 | Only when close to top (`ystop > 20`) | `fmain.c:2596` |

An exception: `hero_sector == 48` (bridge) skips mask rule 3 to prevent incorrect occlusion (`fmain.c:2588`).

### 6.8 `terrain.c` — Offline Tool

The `terrain.c` file is a standalone build tool (not part of the game runtime) that generates the `terra` binary from IFF landscape image files.

**Algorithm** (`terrain.c:47-73`): Iterates landscape files in pairs via an `order[]` array (`terrain.c:24-35`). For each pair, calls `load_images()` which seeks past `IPLAN_SZ = 5 * 64 * 64 = 20480` bytes of image data (`terrain.c:76-84`) and reads 4 arrays of 64 bytes: `maptag`, `terrain`, `tiles`, `big_colors`. Each pair produces 512 bytes of output.

**Pairing order** (`terrain.c:24-35`):

| Pair | Files | Output Bytes |
|------|-------|--------------|
| 0 | wild + palace | 0–511 |
| 1 | swamp + mountain2 | 512–1023 |
| 2 | wild + build | 1024–1535 |
| 3 | rock + tower | 1536–2047 |
| 4 | swamp + mountain3 | 2048–2559 |
| 5 | wild + castle | 2560–3071 |
| 6 | field + mountain1 | 3072–3583 |
| 7 | wild + doom | 3584–4095 |
| 8 | under + furnish | 4096–4607 |
| 9 | inside + astral | 4608–5119 |
| 10 | under + cave | 5120–5631 |

Total: 11 pairs × 512 = 5632 bytes.

### 6.9 Region & Place Names

#### Outdoor Places (`narr.asm:86-193`)

The `_place_tbl` is a 3-byte-entry lookup table: `{sector_low, sector_high, msg_index}`. When `hero_sector` falls within the range, the corresponding `_place_msg` string is displayed. The table is scanned sequentially (`fmain.c:2660-2663`) — first match wins, so overlapping ranges resolve by index order.

Selected entries:

| Sector Range | Place Name |
|-------------|------------|
| 64–69 | Village of Tambry |
| 70–73 | Vermillion Manor |
| 80–95 | City of Marheim |
| 96–99 | Witch's castle |
| 138–139 | Graveyard |
| 144 | Great stone ring |
| 159–162 | Hidden city of Azal |
| 164–167 | Crystal Palace |
| 171–174 | Citadel of Doom |
| 176 | Pixle Grove |
| 208–221 | Great Bog |
| 243 | Oasis |

Special mountain logic (`fmain.c:2664-2668`): When message #4 (mountains) matches, region modifiers apply — odd regions suppress the message; regions > 3 change it to "Plain of Grief" (message 5).

#### Indoor Places (`narr.asm:116-168`)

Same 3-byte format, used when `region_num > 7` (`fmain.c:2656`). Selected entries:

| Sector Range | Place Name |
|-------------|------------|
| 43–59, 100, 143–149 | Spirit world |
| 60–78, 82, 86–87, 92–99, 116–120, 139–141 | Building (various) |
| 65–66 | Tavern |
| 79–96 | Castle of King Mar |
| 105–115, 135–138 | Castle |
| 150–161 | Stone maze |

#### `hero_sector` Computation (`fsubs.asm:1207-1221`)

Computed in `_genmini` from the hero's high-byte coordinates:

```
sec_offset = ((hero_y_high - yreg) << 7) + hero_x_high
hero_sector = map_mem[sec_offset]
```

For indoor regions (`region_num > 7`), 256 is added to `hero_sector` to select the indoor lookup table (`fmain.c:2655-2656`).

#### `mapxy` — Tile Pointer Lookup (`fsubs.asm:1085-1130`)

A variant of `_px_to_im` that returns a *pointer* into `sector_mem` rather than a terrain type. Takes image coordinates (already divided by tile size) instead of pixel coordinates. Used by `doorfind()` and sleeping-spot detection (`fmain.c:1876-1887`) to read or modify the actual tile ID at a map position.

---


## 7. Combat System

### 7.1 Damage Formula — `dohit()`

`dohit(i, j, fc, wt)` at `fmain2.c:230-248` applies damage from attacker `i` to defender `j`.

- `i`: attacker index (−1 = arrow, −2 = fireball, 0 = player, 3+ = monster)
- `j`: defender index
- `fc`: attacker's facing direction (0–7)
- `wt`: damage amount (weapon code, possibly modified)

**Damage application** (`fmain2.c:236-237`):

```c
anim_list[j].vitality -= wt;
if (anim_list[j].vitality < 0) anim_list[j].vitality = 0;
```

Damage equals `wt` directly — the weapon code IS the damage value. Vitality floors at 0.

#### Immunity Checks (`fmain2.c:231-235`)

| Target | Condition | Effect | Source |
|--------|-----------|--------|--------|
| Necromancer (`race 9`) | `weapon < 4` | Immune; `speak(58)` | `fmain2.c:231-233` |
| Witch (`race 0x89`) | `weapon < 4` AND no Sun Stone (`stuff[7]==0`) | Immune; `speak(58)` | `fmain2.c:232-233` |
| Spectre (`race 0x8a`) | Always | Completely immune, silent return | `fmain2.c:234` |
| Ghost (`race 0x8b`) | Always | Completely immune, silent return | `fmain2.c:234` |

The Necromancer and Witch can only be damaged by ranged weapons (bow ≥ 4 or wand = 5). The Witch becomes vulnerable to all weapons when the player holds the Sun Stone (`stuff[7] != 0`). Spectre and Ghost (dead brothers) are non-combatants, intentionally immune to all damage with no feedback.

#### Knockback (`fmain2.c:243-245`)

After damage, the defender is pushed 2 pixels in the attacker's facing direction via `move_figure(j, fc, 2)`. If knockback succeeds and the attacker is melee (`i >= 0`), the attacker also slides 2 pixels forward (follow-through). DRAGON and SETFIG types are immune to knockback.

Every `dohit()` call ends with `checkdead(j, 5)` (`fmain2.c:246`).

### 7.2 Hit Detection — Melee Swing

The hit detection loop runs once per frame for every actor in a fighting state (`fmain.c:2237-2264`):

#### Strike Point (`fmain.c:2247-2248`)

```c
xs = newx(anim_list[i].abs_x, fc, wt+wt) + rand8()-3;
ys = newy(anim_list[i].abs_y, fc, wt+wt) + rand8()-3;
```

The strike point extends `wt * 2` pixels in the attacker's facing direction, with ±3 to ±4 pixels random jitter per axis. Longer weapons probe further from the attacker.

#### Hit Window — Bravery as Reach (`fmain.c:2249-2250`)

```c
if (i==0) bv = (brave/20)+5; else bv = 2 + rand4();
if (bv > 14) bv = 15;
```

| Attacker | Reach (`bv`) | Notes |
|----------|-------------|-------|
| Player | `(brave / 20) + 5`, max 15 | Grows with kills; Julian starts at 6, maxes at 15 at brave=200 |
| Monster | `2 + rand4()` = 2–5 | Re-rolled each frame |

#### Target Matching (`fmain.c:2252-2263`)

Uses **Chebyshev distance** (max of |dx|, |dy|) from strike point to target. All conditions must be true for a hit:

1. Distance < `bv` (reach)
2. `freeze_timer == 0`
3. **Player attacks** (`i==0`): automatic hit
4. **Monster attacks** (`i > 0`): must pass `rand256() > brave` — bravery acts as dodge probability

Monster hit probability: `(256 − brave) / 256`. At Julian's starting brave of 35, monsters land 86% of swings. At brave=100, only 61%.

Near-miss sound plays when distance < `bv + 2` and weapon ≠ wand: `effect(1, 150 + rand256())` (`fmain.c:2263`).

### 7.3 Missile Combat (`fmain.c:2266-2299`)

Arrows and fireballs use the `missile_list[6]` system ([§1.1](RESEARCH-data-structures.md#11-struct-shape--actor-record)).

| Property | Arrow | Fireball |
|----------|-------|----------|
| Hit radius (`mt`) | 6 pixels | 9 pixels |
| Damage | `rand8() + 4` = 4–11 | `rand8() + 4` = 4–11 |
| `dohit` attacker code | −1 | −2 |
| Source | `fmain.c:2280-2281` | `fmain.c:2280-2281` |

Dodge check: for player target (`j==0`), `bv = brave`; for monsters, `bv = 20`. Only missile slot 0 has the dodge check `bitrand(512) > bv` — slots 1–5 always hit if in range (`fmain.c:2289`). With 6 slots assigned round-robin (`mdex` at `fmain.c:1479`), ~17% of projectiles are dodge-eligible. This limits dodge frequency since projectiles are already harder to aim than melee.

#### Special Ranged Attacks

| Attacker | Damage | Rate | Source |
|----------|--------|------|--------|
| Witch (`fmain.c:2375`) | `rand2() + 1` = 1–2 | When `witchflag` set and distance < 100 | `fmain.c:2375` |
| Dragon (`fmain.c:1489-1497`) | 4–11 (fireball) | 25% per frame (`rand4()==0`) | `fmain.c:1493` |

### 7.4 Weapon Types & Damage

| Code | Name | Type | Damage Range | Strike Range (`wt*2`) | Source |
|------|------|------|-------------|----------------------|--------|
| 0 | None | Melee | 0–2 | 0–4 px | `fmain.c:2244-2245` |
| 1 | Dirk | Melee | 1–3 | 2–6 px | `fmain.c:2244-2245` |
| 2 | Mace | Melee | 2–4 | 4–8 px | `fmain.c:2244-2245` |
| 3 | Sword | Melee | 3–5 | 6–10 px | `fmain.c:2244-2245` |
| 4 | Bow | Ranged | 4–11 | mt=6 | `fmain.c:2292-2293` |
| 5 | Wand | Ranged | 4–11 | mt=9 | `fmain.c:2292-2293` |
| 8 | Touch | Melee | 5–7 | 10–14 px | `fmain.c:2244-2245` |

Melee damage formula: `wt + bitrand(2)` where `wt` is the weapon code (clamped to 5 for touch attacks: `if (wt >= 8) wt = 5` at `fmain.c:2245`). Missile damage: `rand8() + 4` for both arrows and fireballs.

Touch attack (code 8) is monster-only, used by Wraiths, Snakes, Spiders, and Loraii (arms group 6 in `weapon_probs[]`).

### 7.5 Swing State Machine

The 9-state `trans_list[]` (`fmain.c:138-146`, detailed in [§2.5](RESEARCH-data-structures.md#25-trans_list--fight-animation-transitions)) drives the sword swing animation. Each tick, a random transition is selected: `trans_list[state].newstate[rand4()]` (`fmain.c:1712`).

The forward cycle through `newstate[0]` traces: 0→1→2→3→4→5→6→8→0 (state 7 reached via other paths). Monsters that reach states 6 or 7 (overhead swings) are forced to state 8 (`fmain.c:1715`).

Fight entry:
- **Player** (`fmain.c:1431-1436`): melee weapon → `state = FIGHTING`; ranged weapon → `state = SHOOT1`
- **Enemy** (`fmain.c:2166`): transitions from WALKING to FIGHTING when within melee threshold

### 7.6 Post-Kill Rewards — `aftermath()`

`aftermath()` (`fmain2.c:253-275`) fires when `battleflag` transitions from TRUE to FALSE (`fmain.c:2192`). It counts dead and fleeing enemies for status messages but does **not** grant experience or loot directly. Rewards come from:

1. **`checkdead()`** (`fmain.c:2769-2784`): Each enemy kill grants `brave++` (`fmain.c:2777`).
2. **Body search** (`fmain.c:3254-3281`): The "Get" action near a dead body yields:
   - **Weapon drop**: The monster's weapon code (1–5). If better than current, auto-equips. Bow drops also give `rand8() + 2` = 2–9 arrows (`fmain.c:3265-3268`).
   - **Treasure**: Indexed by `treasure_probs[encounter_chart[race].treasure * 8 + rand8()]` (`fmain.c:3273`). SetFig races (`race & 0x80`) yield no treasure.

### 7.7 Treasure Drop Tables

`treasure_probs[]` at `fmain2.c:852-858` (5 groups of 8 entries, indexed by `treasure * 8 + rand8()`):

**Group 0** (treasure=0): No drops. Used by Snake, Salamander, Spider, DKnight, Loraii, Necromancer, Woodcutter.

**Group 1** (treasure=1, used by Orcs):

| Roll | Index | Item |
|------|-------|------|
| 0 | 9 | Blue Stone |
| 1 | 11 | Glass Vial |
| 2 | 13 | Bird Totem |
| 3–4 | 31 | 2 Gold Pieces |
| 5–6 | 17 | Green Key |
| 7 | 32 | 5 Gold Pieces |

**Group 2** (treasure=2, used by Ogres):

| Roll | Index | Item |
|------|-------|------|
| 0 | 12 | Crystal Orb |
| 1 | 14 | Gold Ring |
| 2–4 | 20 | Grey Key |
| 5, 7 | 31 | 2 Gold Pieces |
| 6 | 33 | 10 Gold Pieces |

**Group 3** (treasure=3, used by Skeletons):

| Roll | Index | Item |
|------|-------|------|
| 0–1 | 10 | Green Jewel |
| 2–3 | 16 | Gold Key |
| 4 | 11 | Glass Vial |
| 5 | 17 | Green Key |
| 6 | 18 | Blue Key |
| 7 | 19 | Red Key |

**Group 4** (treasure=4, used by Wraiths):

| Roll | Index | Item |
|------|-------|------|
| 0 | 15 | Jade Skull |
| 1 | 21 | White Key |
| 2–7 | 0 | Nothing (`j==0` treated as no treasure in the body search code) |

### 7.8 Death System — `checkdead()`

`checkdead(i, dtype)` at `fmain.c:2769-2784`. Triggers when `vitality < 1` and state is not already DYING or DEAD:

| Effect | Condition | Source |
|--------|-----------|--------|
| Set `goal=DEATH`, `state=DYING`, `tactic=7` | Always | `fmain.c:2774` |
| DKnight death speech: `speak(42)` | `race == 7` | `fmain.c:2775` |
| `kind -= 3` | SETFIG type, not witch (`race != 0x89`) | `fmain.c:2776` |
| `brave++` | Enemy (`i > 0`) | `fmain.c:2777` |
| `event(dtype)`, `luck -= 5`, `setmood(TRUE)` | Player (`i == 0`) | `fmain.c:2777` |

Death event messages (from `narr.asm:11+`): dtype 5 = hit/killed, 6 = drowned, 7 = burned, 8 = turned to stone.

The dying animation uses `tactic` as a frame countdown from 7 to 0 (`fmain.c:1718-1728`). After countdown, state transitions to DEAD with sprite index 82.

#### Special Death Drops (`fmain.c:1751-1756`)

| Monster | On Death | Source |
|---------|----------|--------|
| Necromancer (`race 0x09`) | Transforms to Woodcutter (race 10, vitality 10); drops Talisman (object 139) | `fmain.c:1751-1753` |
| Witch (`race 0x89`) | Drops Golden Lasso (object 27) | `fmain.c:1756` |

### 7.9 Good Fairy & Brother Succession

When the player is DEAD or FALL, `goodfairy` (unsigned char, starts at 0) undergoes a countdown (`fmain.c:1388-1407`):

1. **DYING phase** (before countdown): `checkdead()` sets `tactic = 7`, `state = DYING` (`fmain.c:2773-2774`). Tactic decrements each frame (7→0) — 7 frames of death animation (sprites 80/81 alternating, `fmain.c:1719-1724`). At tactic 0 → `state = DEAD`, corpse sprite (82). `goodfairy` countdown begins.
2. **goodfairy 255→200** (~56 frames): Death sequence continues — corpse visible, death song plays. No code branches match in this range.
3. **goodfairy 199→120** (~80 frames): **Luck gate** — `luck < 1` → `revive(TRUE)` (brother succession). FALL → `revive(FALSE)` (non-lethal). If `luck >= 1`: no visible effect, countdown continues.
4. **goodfairy 119→20** (~100 frames): Fairy sprite flies toward hero (only reached if `luck >= 1`).
5. **goodfairy 19→2** (~18 frames): Resurrection glow effect.
6. **goodfairy 1**: `revive(FALSE)` — fairy rescue, same character returns.

**Design note**: The luck gate at `goodfairy < 200` is positioned *after* the death animation completes (255→200). This is a deliberate design choice — the death sequence (DYING animation + corpse + death song) always plays fully before the outcome is determined. Luck cannot change during the DEAD state — the four luck-modifying code paths all require the hero to be alive or interacting (`checkdead` guards with `state != DYING && state != DEAD`, pit falls require movement, sorceress requires TALK). So the gate is effectively a one-time decision: if luck ≥ 1 when the countdown crosses 200, the fairy is guaranteed to appear and rescue the hero. Since each death costs exactly 5 luck, the number of fairy rescues from starting stats is exactly `floor((luck - 1) / 5)`: Julian: 3, Phillip: 6, Kevin: 3. Falls cost 2 luck each and reduce this total.

#### `revive()` — `fmain.c:2812-2900`

**`revive(TRUE)` — New brother**:
- `brother` increments: 1→Julian, 2→Phillip, 3→Kevin, 4+→game over
- Stats reset from `blist[]` (`fmain.c:2803-2805`): Julian {brave=35, luck=20, kind=15, wealth=20}, Phillip {brave=20, luck=35, kind=15, wealth=15}, Kevin {brave=15, luck=20, kind=35, wealth=10}
- Inventory wiped: `for (i=0; i<GOLDBASE; i++) stuff[i] = 0` (`fmain.c:2849`)
- Starting weapon: Dirk (weapon 1) (`fmain.c:2850`)
- Vitality: `15 + brave/4` (`fmain.c:2897`)
- Dead brother's body and ghost placed in world (`fmain.c:2840-2844`)

**`revive(FALSE)` — Fairy rescue**: No stat changes. Returns to last safe position (`safe_x`, `safe_y`). Vitality restored to `15 + brave/4`.

### 7.10 Bravery & Luck in Combat

Bravery serves dual duty as passive experience and active combat stat:

| Effect | Formula | Source |
|--------|---------|--------|
| Melee reach | `(brave / 20) + 5`, max 15 | `fmain.c:2249` |
| Monster dodge chance | `rand256() > brave` must pass for hit | `fmain.c:2260` |
| Missile dodge (slot 0) | `bitrand(512) > brave` | `fmain.c:2289` |
| Starting vitality | `15 + brave / 4` | `fmain.c:2897` |
| Growth | +1 per enemy kill | `fmain.c:2777` |

This creates a **compounding feedback loop**: more kills → higher brave → longer reach + better dodge + more HP → more kills. Combat gets progressively easier.

Luck decreases by 5 per player death (`fmain.c:2777`) and by 2 per ledge fall (`fmain.c:1783`). When depleted, the next death is permanent — there is no fairy rescue.

---

