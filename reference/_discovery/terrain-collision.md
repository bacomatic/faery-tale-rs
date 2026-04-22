# Discovery: Terrain & Collision Detection

**Status**: refined
**Investigated**: 2026-04-05, refined 2026-04-08
**Requested by**: orchestrator
**Prompt summary**: Trace the terrain data system (px_to_im, prox, proxcheck), terrain types, memory layout, terrain.c tool, region/place names, and collision rules per motion state. Refinement pass: item-gated terrain (crystal shard, statues, rose), desert gate dual-check, stone circle teleport, door system, fiery_death definition, environ-terrain damage interaction.

## Terrain Decode Chain

The primary function `_px_to_im` (fsubs.asm:542-620) converts an absolute pixel coordinate (x, y) into a terrain type value (0–15). The chain has four stages:

### Stage 1: Tile Bit Selection (fsubs.asm:548-559)

Starting with `d4 = 0x80` (bit 7 set), three tests select one of 8 sub-tile bits based on the pixel's bit-3 of x, bit-3 of y, and bit-4 of y:

```
d4 = 0x80                              ; start at bit 7
if bit 3 of x is set:  d4 >>= 4        ; shift to bit 3 (0x08)
if bit 3 of y is set:  d4 >>= 1        ; halve it
if bit 4 of y is set:  d4 >>= 2        ; quarter it
```

This selects one of 8 spatial sub-regions within a 16×32 image tile:
- Bits 3 of x and y divide the tile into 4 quadrants (8×16 each)
- Bit 4 of y further subdivides vertically

### Stage 2: Pixel to Image/Sector Coordinates (fsubs.asm:561-589)

```
imx = x >> 4                           ; x / 16 — image x (fsubs.asm:561)
imy = y >> 5                           ; y / 32 — image y (fsubs.asm:562)

secx = (imx >> 4) - xreg               ; sector x = imx/16 - xreg (fsubs.asm:564-565)
```

Sector x-coordinate wrapping (fsubs.asm:567-572):
```
if bit 6 of secx is clear: use secx as-is
else if bit 5 of secx is clear: secx = 63 (0x3F)
else: secx = 0
```

```
secy = (imy >> 3) - yreg               ; sector y = imy/8 - yreg (fsubs.asm:574-575)
if secy < 0: secy = 0                  ; (fsubs.asm:577)
if secy >= 32: secy = 31               ; (fsubs.asm:578-579)
```

Map grid index:
```
map_index = secy * 128 + secx + xreg   ; (fsubs.asm:581-583)
```

### Stage 3: Sector Lookup (fsubs.asm:590-604)

```
sec_num = map_mem[map_index]            ; byte from map grid (fsubs.asm:593)
local_imx = imx & 15                   ; image-within-sector x (fsubs.asm:595)
local_imy = imy & 7                    ; image-within-sector y (fsubs.asm:596)
offset = sec_num * 128 + local_imy * 16 + local_imx  ; (fsubs.asm:598-602)
image_id = sector_mem[offset]           ; the landscape image tile ID (fsubs.asm:604)
```

Each sector is 16×8 tiles = 128 bytes in `sector_mem`. Each byte is a landscape image tile ID.

### Stage 4: Terrain Attribute Lookup (fsubs.asm:606-616)

```
terra_index = image_id * 4              ; 4 bytes per terrain entry (fsubs.asm:607-608)
tbit = terra_mem[terra_index + 2]       ; tile mask byte (fsubs.asm:610)
```

The selected tile bit (`d4` from Stage 1) is AND'd with the mask byte:
```
if (tbit & d4) == 0: return 0          ; open terrain (fsubs.asm:611-612)
else: return terra_mem[terra_index + 1] >> 4  ; terrain type (high nibble) (fsubs.asm:614-615)
```

**Key insight**: The tile mask byte allows sub-tile precision — an image tile can be half-blocking. The 8 sub-tile bits correspond to 8 spatial zones within the 16×32 pixel tile. Only if the specific zone's bit is set does the terrain type apply; otherwise it's passable (type 0).

## Astral Plane / Spirit World and Terrain Type 12

See [astral-plane.md](astral-plane.md) for a dedicated investigation of the astral plane, spirit world geography, terrain type 12 (crystal shard passthrough), and the Crystal Palace vs Crystal Shard distinction.

## Terrain Types

Terrain types are the high nibble of `terra_mem[image_id * 4 + 1]` (fsubs.asm:614-615). The comment at fmain.c:685-686 describes:

| Value | Meaning | Gameplay Effect |
|-------|---------|----------------|
| 0 | Open/passable | No effect — walkable ground |
| 1 | Impassable | Blocked — cannot walk through (fmain.c:685). `_prox` returns it as blocking (fsubs.asm:1601-1602) |
| 2 | Water/sink (slow) | environ increments to 2, no sinking yet (fmain.c:1773) |
| 3 | Water (medium) | environ increments to 5 (fmain.c:1774) |
| 4 | Water (deep) | environ increments toward 10; sinking begins at 15 (fmain.c:1775-1795) |
| 5 | Water (very deep) | environ increments toward 30; at 30 player enters SINK state → death; special sector 181 teleports to region 9 (fmain.c:1775-1793) |
| 6 | Slippery (ice/slide) | environ = -1, speed becomes 4 (fmain.c:1771, 1601) |
| 7 | Velocity-based (ice) | environ = -2, velocity-based physics with momentum (fmain.c:1772, 1580-1595) |
| 8 | Lava/fire | environ = -3, player walks backwards at speed -2 (fmain.c:1770, 1600) |
| 9 | Pit/fall | If hero (i==0) and xtype==52: triggers FALL state, luck -= 2 (fmain.c:1776-1783) |
| 10+ | N/A (blocked by prox) | `_prox` treats 10+ as blocked at second probe point (fsubs.asm:1608-1609). First probe blocks at 8+ (fsubs.asm:1603-1604). |
| 12 | Crystal shard pass | `stuff[30]` (crystal shard) allows passage through type 12 terrain (fmain.c:1611) |
| 15 | Door tile | Triggers `doorfind()` attempt when player bumps into it (fmain.c:1609) |

### Environ Effects Summary

The `environ` field on each actor tracks depth/slide state (fmain.c:1474, 1760-1800):
- `k = 0`: normal ground
- `k = -1`: slippery (speed 4)
- `k = -2`: velocity-based sliding/flying (momentum physics)
- `k = -3`: repulsive/lava (walk backwards)
- `k = 2`: shallow water
- `k = 5`: brush/medium depth
- `k = 10`: deep water
- `k = 15`: threshold for SINK state (fmain.c:1795)
- `k = 30`: death depth/deep-submersion (fmain.c:1784-1793)

Water damage (fmain.c:1844-1846):
- If `stuff[23]` (turtle shell? — player has item), environ forced to 0
- If environ > 15: vitality = 0 (instant death)
- If environ > 2: vitality-- (gradual drowning)

The `fiery_death` flag must be set for this damage to apply (fmain.c:1843).

## Collision Detection

### `_prox` (fsubs.asm:1590-1614)

The `_prox` function performs two terrain probes, returning the terrain type if either probe finds blocking terrain, or 0 if both pass.

**Probe 1** (right of center): `(x+4, y+2)` — fsubs.asm:1593-1594
- Blocking if terrain type == 1 (impassable) — fsubs.asm:1596-1597
- Blocking if terrain type >= 10 — fsubs.asm:1598-1599

**Probe 2** (left of center): `(x-4, y+2)` — fsubs.asm:1601-1604
- Note: source comment at fsubs.asm:1603 says `; x + 4` but the instruction is `subq #4,d0` (x - 4) — this is a comment error in the original source
- Blocking if terrain type == 1 (impassable) — fsubs.asm:1606-1607
- Blocking if terrain type >= 8 — fsubs.asm:1608-1609

**Why two probes with different thresholds**: The right probe blocks at ≥10 while the left probe blocks at ≥8. This asymmetric filtering means:
- Types 1: blocked everywhere
- Types 8-9: blocked only at the left probe (the foot position shifts laterally)  
- Types 10+: blocked at both probes
- Types 2-7 (water/ice): never blocked by `_prox` — these are walk-through with environ effects

If neither probe blocks, `_prox` returns 0.

### `proxcheck` (fmain2.c:277-293)

The C wrapper adds two layers to `_prox`:

1. **Wraith bypass** (fmain2.c:279-280): If the actor is an ENEMY with `race == 2` (wraith), skip terrain collision entirely.

2. **Terrain check** (fmain2.c:281-283):
   ```c
   x1 = prox(x, y);
   if (i==0 && (x1 == 8 || x1 == 9)) x1 = 0;  // player ignores types 8,9
   if (x1) return x1;
   ```
   For the player character only (i==0), terrain types 8 and 9 are treated as passable (not blocking). This means the player can walk into lava/fire and pit areas — they cause effects but don't block movement.

3. **Actor collision** (fmain2.c:285-292): Iterates all active actors checking bounding box overlap:
   ```c
   for (j=0; j<anix; j++)
       if (i != j && j != 1 && anim_list[j].type != 5
           && anim_list[j].state != DEAD)
           if (abs(x - actor_x) < 11 && abs(y - actor_y) < 9)
               return 16;
   ```
   - Skips self (i != j)
   - Skips actor slot 1 (companion/follower)
   - Skips type 5 (purpose unclear — placeholder?)
   - Skips DEAD actors
   - Uses 22×18 bounding box (±11 x, ±9 y)
   - Returns 16 (actor collision code, not a terrain type)

### Movement With Collision (fmain.c:1598-1650)

Walking actors compute test positions using `newx`/`newy`, then call proxcheck. On collision:
1. Try original direction → if blocked, try direction+1 (CW deviate)
2. If that's blocked, try direction-2 (CCW deviate)
3. If all blocked → `goto blocked` (frustflag increments for player)

The speed `e` varies by environ (fmain.c:1599-1603):
- Riding raft (i==0, riding==5): e=3
- Lava (k==-3): e=-2 (walk backwards!)
- Slippery (k==-1): e=4
- Shallow water (k==2) or deep (k>6): e=1 (slow)
- Normal: e=2

Special: crystal shard `stuff[30]` overrides type 12 blocking (fmain.c:1611: `if (stuff[30] && j==12) goto newloc`).

## Memory Layout

### `sector_mem` — Sector Tile Data

Allocated size: `SECTOR_SZ = (128 * 256) + 4096 = 36864 bytes` (fmain.c:643)
- First 32768 bytes: 256 sectors × 128 bytes each
- Each sector: 16 columns × 8 rows = 128 tile IDs (one byte each)
- A tile ID indexes into the image file and the terra_mem attributes
- Last 4096 bytes: region map data (at offset `SECTOR_OFF = 32768`)

`map_mem` points into sector_mem at offset SECTOR_OFF:
```c
map_mem = sector_mem + SECTOR_OFF;    // fmain.c:921
```

### `map_mem` — Map Grid (Region Map)

Size: 4096 bytes (part of sector_mem allocation)
- Organized as a 2D grid: 128 columns × 32 rows
- Each byte is a sector number (0–255)
- Indexed by: `map_mem[secy * 128 + secx + xreg]`
- `xreg`/`yreg` are the region origin offsets — set when loading a region

### `terra_mem` — Terrain Attributes

Allocated: 1024 bytes, MEMF_CHIP (fmain.c:928)
- Two halves of 512 bytes each, loaded from separate terra tracks:
  - `terra_mem[0..511]`: loaded from `TERRA_BLOCK + nd->terra1` (fmain.c:3567)
  - `terra_mem[512..1023]`: loaded from `TERRA_BLOCK + nd->terra2` (fmain.c:3572)
- Each image tile has a 4-byte entry:
  - Byte 0: `maptag` — image characteristics / mask data (used by maskit: fmain.c:2595)
  - Byte 1: high nibble = terrain type (0-15), low nibble = mask application rule (fmain.c:2579)
  - Byte 2: `tiles` — 8-bit tile feature mask (sub-tile collision bits)
  - Byte 3: `big_colors` — dominant color (used for rendering)
- 512 bytes / 4 = 128 entries per half → supports up to 128 image tiles per terrain file
- `TERRA_BLOCK = 149` (fmain.c:608) — disk block offset for terrain data

### Region Loading (fmain.c:3540-3590)

Each of the 10 regions has a `struct need` entry (fmain.c:613-626):
```c
struct need { USHORT image[4], terra1, terra2, sector, region, setchar; };
```

The `file_index[10]` table maps regions to disk resources:
| Region | Description | terra1 | terra2 |
|--------|-------------|--------|--------|
| 0 (F1) | Snowy region | 0 | 1 |
| 1 (F2) | Witch wood | 2 | 3 |
| 2 (F3) | Swampy region | 2 | 1 |
| 3 (F4) | Plains and rocks | 2 | 3 |
| 4 (F5) | Desert area | 0 | 4 |
| 5 (F6) | Bay/city/farms | 5 | 6 |
| 6 (F7) | Volcanic | 7 | 4 |
| 7 (F8) | Forest/wilderness | 5 | 6 |
| 8 (F9) | Inside buildings | 8 | 9 |
| 9 (F10) | Dungeons/caves | 10 | 9 |

Terrain data is only reloaded if `nd->terra1` or `nd->terra2` differ from `current_loads` (fmain.c:3565-3573).

### Mask Application Rules

The low nibble of `terra_mem[image_id * 4 + 1]` controls how the terrain mask is applied during rendering (fmain.c:2579-2596, comment at fmain.c:689-691):

| Value | Rule |
|-------|------|
| 0 | Never apply mask |
| 1 | Apply when sprite is below (down) |
| 2 | Apply when sprite is to the right |
| 3 | Always apply (unless flying) |
| 4 | Only when down AND right |
| 5 | Only when down OR right (either condition) |
| 6 | Full mask if above ground level; partial otherwise |
| 7 | Only when close to top (ystop > 20) |

These control the behind-scenery occlusion masking (fmain.c:2580-2596).

## terrain.c Tool

The `terrain.c` file (standalone tool, not part of the game runtime) generates the `terra` binary file from the landscape IFF image files.

### Algorithm (terrain.c:47-73)

1. Iterates through the `order[]` array in pairs (terrain.c:55)
2. For each pair of landscape files:
   - Calls `load_images(datanames[j])` to read terrain metadata from the image file
   - Copies 4 arrays of 64 bytes each into `outbuffer`: `maptag[64]`, `terrain[64]`, `tiles[64]`, `big_colors[64]`
3. Writes 512 bytes per pair (2 × 256 = 512) to the `terra` output file

### `load_images` (terrain.c:76-84)

Seeks past the image pixel data (`IPLAN_SZ = 5 * 64 * 64 = 20480` bytes) and reads 4 consecutive 64-byte arrays:
1. `maptag[64]` — image characteristics
2. `terrain[64]` — terrain type + mask rule (packed into one byte, high/low nibbles)
3. `tiles[64]` — sub-tile collision bitmask
4. `big_colors[64]` — dominant tile color

### File Order (terrain.c:24-35)

The `order[]` array pairs landscape files for output:
```
Pair 0:  wild(1) + palace(9)      → terra bytes 0-511
Pair 1:  swamp(8) + mountain2(10) → terra bytes 512-1023
Pair 2:  wild(1) + build(2)       → ...
Pair 3:  rock(3) + tower(5)
Pair 4:  swamp(8) + mountain3(12)
Pair 5:  wild(1) + castle(6)
Pair 6:  field(7) + mountain1(4)
Pair 7:  wild(1) + doom(11)
Pair 8:  under(13) + furnish(15)
Pair 9:  inside(16) + astral(17)
Pair 10: under(13) + cave(14)
```

11 pairs × 512 bytes = 5632 bytes total in the `terra` file. At runtime, each half of terra_mem (512 bytes) is loaded from one pair, selected by the region's `terra1`/`terra2` indices.

### Landscape File Names (terrain.c:2-18)

| Index | Name | Description |
|-------|------|-------------|
| 1 | wild | Wilderness |
| 2 | build | Buildings |
| 3 | rock | Rocky terrain |
| 4 | mountain1 | Mountains (type 1) |
| 5 | tower | Towers |
| 6 | castle | Castles |
| 7 | field | Fields |
| 8 | swamp | Swamp |
| 9 | palace | Palace |
| 10 | mountain2 | Mountains (type 2) |
| 11 | doom | Doom citadel |
| 12 | mountain3 | Mountains (type 3) |
| 13 | under | Underground |
| 14 | cave | Caves |
| 15 | furnish | Interior furnishings |
| 16 | inside | Inside buildings |
| 17 | astral | Spirit world |

## Region/Place Names

### Outdoor Places (`_place_tbl` / `_place_msg`: narr.asm:86-193)

The `_place_tbl` is a lookup table of 3-byte entries: `{sector_low, sector_high, message_index}`. When `hero_sector` falls within `[sector_low, sector_high]`, the corresponding message is displayed (fmain.c:2647-2674).

| Sector Range | Msg# | Place Name |
|-------------|------|------------|
| 51-51 | 19 | Small keep |
| 64-69 | 2 | Village of Tambry |
| 70-73 | 3 | Vermillion Manor |
| 80-95 | 6 | City of Marheim |
| 96-99 | 7 | Witch's castle |
| 138-139 | 8 | Graveyard |
| 144-144 | 9 | Great stone ring |
| 147-147 | 10 | Watchtower (lighthouse) |
| 148-148 | 20 | Old castle |
| 159-162 | 17 | Hidden city of Azal |
| 163-163 | 18 | Outlying fort |
| 164-167 | 12 | Crystal Palace |
| 168-168 | 21 | Log cabin |
| 170-170 | 22 | Dark stone tower |
| 171-174 | 14 | Citadel of Doom |
| 176-176 | 13 | Pixle Grove |
| 178-178 | 23 | Isolated cabin (swamp) |
| 179-179 | 24 | Tombs of Hemsath |
| 180-180 | 25 | Forbidden Keep |
| 175-180 | 0 | Lava/elf (nil) |
| 208-221 | 11 | Great Bog |
| 243-243 | 16 | Oasis |
| 250-252 | 0 | Nil (interface) |
| 255-255 | 26 | Cave in hillside (dragon) |
| 78-78 | 4 | Mountains of Frost |
| 187-239 | 4 | Mountains (by type) |
| 0-79 | 0 | Nil |
| 185-254 | 15 | Burning Waste (desert) |
| 0-255 | 0 | Nil (fallthrough) |

Note: The table is scanned sequentially (fmain.c:2660-2663), so the first matching entry wins. Some ranges overlap — the ordering creates priority (e.g., sector 78 matches "Mountains" before the broad 0-79 nil range).

Special mountain logic (fmain.c:2664-2668): message 4 (mountains) is modified based on region:
- If region > 7 (inside): keep message 4
- If region is odd: no message (0)
- If region > 3: message becomes 5 (Plain of Grief)

### Indoor Places (`_inside_tbl` / `_inside_msg`: narr.asm:116-168)

Same 3-byte format. Used when `region_num > 7` (fmain.c:2656).

| Sector Range | Msg# | Place Name |
|-------------|------|------------|
| 2-2 | 2 | Small chamber |
| 7-7 | 3 | Large chamber |
| 4-4 | 4 | Long passageway |
| 5-6 | 5 | Twisting tunnel |
| 9-10 | 6 | Forked intersection |
| 30-30 | 7 | Keep interior |
| 19-33 | 14 | Stone corridor |
| 101-101 | 14 | Stone corridor |
| 130-134 | 14 | Stone corridor |
| 36-36 | 13 | Octagonal room |
| 37-42 | 12 | Large room |
| 46-46 | 0 | Final arena (special) |
| 43-59 | 11 | Spirit world |
| 100-100 | 11 | Spirit world |
| 143-149 | 11 | Spirit world |
| 62-62 | 16 | Small building |
| 65-66 | 18 | Tavern |
| 60-78 | 17 | Building |
| 82-82 | 17 | Building |
| 86-87 | 17 | Building |
| 92-92 | 17 | Priest's building |
| 94-95 | 17 | Small buildings |
| 97-99 | 17 | Building |
| 120-120 | 17 | Building (desert fort) |
| 116-119 | 17 | Building (desert) |
| 139-141 | 17 | Building (desert) |
| 79-96 | 9 | Castle of King Mar |
| 104-104 | 19 | Inn |
| 114-114 | 20 | Tomb inside (crypt) |
| 105-115 | 8 | Castle |
| 135-138 | 8 | Castle (doom tower) |
| 125-125 | 21 | Cabin inside |
| 127-127 | 10 | Elf glade (sanctuary) |
| 142-142 | 22 | Unlocked/lighthouse |
| 121-129 | 22 | Unlocked/entered |
| 150-161 | 15 | Stone maze |
| 0-255 | 0 | Nil (fallthrough) |

### `hero_sector` Computation (fsubs.asm:1207-1221)

Computed in `_genmini` by looking up the map byte at the hero's high-byte coordinates:
```
d0 = hero_x (high byte)
d1 = hero_y (high byte) - yreg
sec_offset = (d1 << 7) + d0
hero_sector = map_mem[sec_offset]
if region_num > 7: hero_sector += 256   ; inside flag
```

The `find_place` function (fmain.c:2647-2674) masks hero_sector to 8 bits, selects the outdoor or indoor table based on `region_num`, then does a linear scan of 3-byte entries until it finds the first range containing the sector value.

## Cross-Cutting Findings

- **Crystal shard (stuff[30])** overrides terrain type 12 blocking — allows passage through crystal terrain (fmain.c:1611). Cross-cuts inventory and terrain systems.
- **Rose (stuff[23])** prevents environ-based damage in fiery_death zones by forcing environ to 0 (fmain.c:1844). Cross-cuts inventory and environ/sinking system.
- **Wraith (race 2)** bypasses all terrain collision in proxcheck (fmain2.c:279-280). Cross-cuts AI, actor types, and collision.
- **Wraith and snake immunity** to water: race 2 and race 4 have terrain type forced to 0 (fmain.c:1638-1639).
- **Sector 181 special teleport**: when environ reaches 30 at sector 181, player (i==0) is teleported to region 9 at coords (0x1080, 34950) instead of dying (fmain.c:1784-1791). This is likely a water portal mechanic.
- **hero_sector 48**: bridge exception — mask application rule 3 is skipped (fmain.c:2588). Prevents incorrect occlusion on bridges.
- **actor_file 11**: swan/carrier physics — sets environ to -2 (velocity-based) and uses momentum up to speed 40 (fmain.c:1464, 1582). Cross-cuts actor and movement systems.
- **Sleeping spots**: `mapxy` used to check if hero is on specific tile IDs (161, 52, 162, 53) for bed detection in region 8 (fmain.c:1876-1887). Cross-cuts terrain data and rest system.
- **Random treasure placement**: `px_to_im` used in a loop to ensure spawned treasures only land on type-0 (open) terrain (fmain2.c:1232). Cross-cuts object spawning and terrain.

## mapxy Function

`_mapxy` (fsubs.asm:1085-1130) is a variant of `_px_to_im` that returns a *pointer* to `sector_mem[offset]` rather than a terrain type. Takes image coordinates (imx, imy) rather than pixel coordinates. Used by `doorfind` and sleeping-spot detection to read/modify the actual tile ID at a map position.

The sector/map lookup logic is identical to `_px_to_im` stages 2-3, but:
- No pixel-to-image division (coordinates are already in image units)
- Returns `&sector_mem[offset]` as a pointer (fsubs.asm:1129-1130), enabling both read and write access to the tile map

## Crystal Walls (Terrain Type 12)

### Crystal Shard (stuff[30]) Gate

Terrain type 12 acts as a blocking wall in the normal collision path. The single override is at fmain.c:1609:
```c
if (stuff[30] && j==12) goto newloc;
```
This is inside the WALKING state handler, after `proxcheck` returns non-zero (`j` holds the proxcheck return). If the player (i==0) has the Shard (stuff[30] != 0) AND the blocking terrain is type 12, movement succeeds (`goto newloc`).

**Item**: stuff[30] is the "Shard" (fmain.c:416: `{ 12, 14,110,0, 8,8, 1, "Shard" }`). Obtained from the Spectre after giving him the Bone (speak 48, narr.asm:489-491: "Take this crystal shard.").

**Location**: Crystal walls associated with the Crystal Palace (sectors 164–167 per `_place_tbl` at narr.asm:98: `dc.b 164,167,12`). The terrain type 12 tiles are encoded in the landscape image files for the regions containing the Crystal Palace. The Crystal Palace is in the outdoor world (regions 0-7), with door entries at fmain.c:261-262 using type CRYST(7), linking to indoor coordinates in region 8/9.

**Note**: `_prox` blocks terrain type ≥10 at the right probe point and ≥8 at the left probe point (fsubs.asm:1598-1609), so type 12 is always blocked by `_prox`, and the stuff[30] check is the only override.

## Desert Gate (stuff[25])

### Gate 1: Door System (fmain.c:1919)

During door transition processing, the check is:
```c
if (d->type == DESERT && (stuff[STATBASE]<5)) break;
```
Where `STATBASE = 25` (fmain.c:428) and `DESERT = 17` (fmain.c:227). If the player has fewer than 5 Gold Statues, the door transition is aborted (`break` exits the search loop without teleporting).

The DESERT-type door entries in doorlist (fmain.c:247-250):
```c
{ 0x1aa0,0x4ba0, 0x13a0,0x95a0, DESERT,1 }, /* oasis #1 */
{ 0x1aa0,0x4c60, 0x13a0,0x9760, DESERT,1 }, /* oasis #4 */
{ 0x1b20,0x4b60, 0x1720,0x9560, DESERT,1 }, /* oasis #2 */
{ 0x1b80,0x4b80, 0x1580,0x9580, DESERT,1 }, /* oasis #3 */
```
These are horizontal doors (DESERT=17, odd number) connecting outdoor oasis locations to indoor desert buildings.

### Gate 2: Map Blocking (fmain.c:3594-3596)

During region loading in `load_new_region()`:
```c
if (new_region == 4 && stuff[STATBASE] < 5) /* are we in desert sector */
{   i = ((11*128)+26);
    map_mem[i] = map_mem[i+1] = map_mem[i+128] = map_mem[i+129] = 254;
}
```
If loading region 4 (F5 — desert area) with fewer than 5 statues, four map grid cells at coordinates `(26, 11)`, `(27, 11)`, `(26, 12)`, `(27, 12)` are overwritten with sector 254. Sector 254 is a non-traversable barrier sector, effectively making a 2×2 block of map impassable. This prevents the player from reaching the desert interior even if they enter the region from an unexpected direction.

**stuff[25]**: "Gold Statue" (fmain.c:411: `{ 21,232, 0,10, 8,8, 5, "Gold Statue" }`). The `maxshown` field is 5, matching the gate requirement.

## Stone Circle (Sector 144) and Blue Stone Teleport

### Sector 144 Location

Sector 144 is the "great stone ring" per `_place_tbl` (narr.asm:93: `dc.b 144,144,9`). Message 9 reads "% came to a great stone ring." (narr.asm:174).

### Teleport Mechanic (fmain.c:3327-3348)

Triggered when player uses the Blue Stone (stuff[9], magic item slot 5) from the USE MAGIC menu:
```c
case 5: /* Blue Stone */
    if (hero_sector == 144)
    {   if ((hero_x & 255)/85 == 1 && (hero_y & 255)/64 == 1)
        {   short x1, y1;
            x = hero_x>>8; y = hero_y>>8;
            for (i=0; i<11; i++)
            {   if (stone_list[i+i]==x && stone_list[i+i+1]==y)
                {   i+=(anim_list[0].facing+1); if (i>10) i-=11;
                    x = (stone_list[i+i]<<8) + (hero_x & 255);
                    y = (stone_list[i+i+1]<<8) + (hero_y & 255);
                    colorplay();
                    xfer(x,y,TRUE);
                    ...
                    break;
                }
            }
        } else return;
    }
    else return;
```

**Requirements**:
1. `hero_sector == 144` — must be standing in the stone ring sector
2. `(hero_x & 255)/85 == 1` and `(hero_y & 255)/64 == 1` — must be positioned in the center of a stone within the sector (specific sub-tile position)
3. The hero's position (high bytes of hero_x, hero_y) must match one of the 11 entries in `stone_list`

### Stone List (fmain.c:374-376)

```c
unsigned char stone_list[] = 
{   54,43, 71,77, 78,102, 66,121, 12,85, 79,40,
    107,38, 73,21, 12,26, 26,53, 84,60 };
```
11 stone circle locations as (x_high, y_high) pairs in the map grid.

**Direction-based destination**: The destination stone is calculated as `i + (facing + 1)`, wrapping at 11. So facing direction determines which of the 11 circles you teleport to. The destination inherits the sub-tile position from the origin (preserving the low byte of hero_x/hero_y).

## fiery_death and Environment Hazard Damage

### fiery_death Flag (fmain.c:1384-1385)

```c
fiery_death =
    (map_x>8802 && map_x<13562 && map_y>24744 && map_y<29544);
```
This is a rectangular region check evaluated every game tick before movement processing. The fiery_death flag is TRUE when the camera (map_x, map_y) is within the volcanic/lava region: approximately x:8802-13562, y:24744-29544.

### Environ-Based Damage (fmain.c:1843-1848)

Only applies when `fiery_death` is TRUE:
```c
if (fiery_death)
{   if (i==0 && stuff[23]) an->environ = 0;
    else if (an->environ > 15) an->vitality = 0;
    else if (an->environ > 2) an->vitality--;
    checkdead(i,27);
}
```

**stuff[23] (Rose)**: "Rose" item (fmain.c:408: `{ 19, 0, 90,0, 8,8, 1, "Rose" }`). Protects the player (i==0 only) from ALL fiery_death-zone environ damage by forcing environ to 0. NPCs/enemies get no protection.

**Damage tiers**:
- environ > 15: instant death (vitality = 0) — deep water/lava
- environ > 2: lose 1 vitality per tick — shallow water/lava
- environ ≤ 2: no damage (shallow water or slippery surfaces)

### Lava-Specific Environ Damage (fmain.c:1849-1851)

Additionally, outside the fiery_death check:
```c
if (an->environ == 30 && (cycle&7) == 0)
{   if (k != 2 && k != 3) { an->vitality--; checkdead(i,6); }
}
```
At environ 30 (fully submerged/deep lava), lose 1 vitality every 8 ticks UNLESS the actor's race is 2 (wraith) or 3. Race check here uses the local `k = an->race` (fmain.c:1802).

### Water/Terrain Environ Interaction Summary (fmain.c:1760-1800)

The "sinker" label block handles terrain-to-environ mapping:

| Terrain j | Environ k | Effect |
|-----------|-----------|--------|
| any, i==0, raftprox | k = 0 | On raft = safe from drowning |
| 0 | k = 0 | Normal ground |
| 6 | k = -1 | Slippery (ice/slide) |
| 7 | k = -2 | Velocity-based sliding (momentum) |
| 8 | k = -3 | Repulsive (lava push backwards) |
| 9 (+ i==0 + xtype==52) | k = -2 + FALL state | Pit trap: triggers FALL, luck -= 2 |
| 2 | k = 2 | Shallow water |
| 3 | k = 5 | Medium depth |
| 4 | increments toward 10 | Deep water; SINK at k > 15 |
| 5 | increments toward 30 | Very deep water; death at k == 30 |

**Sinking progression for types 4 and 5** (fmain.c:1775-1797):
- If already deeper than target (k > e): k decrements by 1 (climbing out)
- If shallower than target (k < e): k increments by 1
- At k > 15: actor enters SINK state (visual sinking animation)
- At k == 30: actor enters STILL state. If hero_sector == 181: teleport to region 9 at (0x1080, 34950) for hero, instant death for NPCs. This is the underwater portal.

**Rising out of water**: When terrain changes back to type 0 and k > 2, the check at fmain.c:1642-1644 applies:
```c
if (k > 2)
{   if ( (j == 0) || ((j == 3) && (k > 5)) ||
        ((j == 4) && (k > 10)) ) 
    {   if (hero_sector != 181) k--; goto raise; }
}
```
environ decrements by 1 per tick when moving to shallower terrain (except at sector 181 where it stays locked — the underwater portal sector).

## Door System and Image Block IDs

### reg_id Mapping (fmain.c:1082-1098)

In `doorfind()`, the `sec_id` is the raw tile ID from `sector_mem` at the door position. The `reg_id` is the image block number:
```c
sec_id = *(mapxy(x,y));
reg_id = current_loads.image[(sec_id>>6)];
```
Each sector tile ID's top 2 bits (bits 7-6) index into the 4-element `current_loads.image[]` array, which holds the disk block numbers for the currently loaded image planes. This maps a tile to which of the 4 loaded image sets it belongs to.

### open_list Door Table (fmain.c:1059-1078)

The `open_list[17]` table defines openable doors:
```c
struct door_open {
    UBYTE   door_id;      /* tile ID that IS the door */
    USHORT  map_id;       /* image block number this door appears in */
    UBYTE   new1, new2;   /* replacement tile IDs when opened */
    UBYTE   above;        /* which adjacent tile to also replace */
    UBYTE   keytype;      /* key needed: NOKEY, GOLD, GREEN, KBLUE, RED, GREY, WHITE */
};
```

Key types (fmain.c:1048): `enum ky {NOKEY=0, GOLD, GREEN, KBLUE, RED, GREY, WHITE};`

Door matching: for each open_list entry, if `map_id == reg_id && door_id == sec_id`, and the player has the matching key type (or NOKEY), the tile at the door position is replaced with `new1`, and optionally an adjacent tile with `new2` (direction controlled by `above` field).

### Door Type Constants (fmain.c:213-229)

| Constant | Value | Orientation | Notes |
|----------|-------|-------------|-------|
| HWOOD | 1 | Horizontal | Odd = horizontal (type & 1) |
| VWOOD | 2 | Vertical | |
| HSTONE | 3 | Horizontal | |
| VSTONE | 4 | Vertical | |
| HCITY | 5 | Horizontal | |
| VCITY | 6 | Vertical | |
| CRYST | 7 | Horizontal | Crystal Palace doors |
| SECRET | 8 | — | Secret passages |
| BLACK | 9 | — | Fortress/gate doors |
| MARBLE | 10 | — | Marble palace doors |
| LOG | 11 | — | Log cabin doors |
| HSTON2 | 13 | Horizontal | |
| VSTON2 | 14 | Vertical | |
| STAIR | 15 | — | Staircases |
| DESERT | 17 | Horizontal | Desert oasis doors |
| CAVE/VLOG | 18 | Vertical | Caves and log cabin yards (shared value) |

## Sector/Region System

### Region-to-File Mapping (fmain.c:613-626)

```c
struct need file_index[10] = {
    { 320,480,520,560,  0,1, 32,160,22 }, /* F1 - snowy region */
    { 320,360,400,440,  2,3, 32,160,21 }, /* F2 - witch wood */
    { 320,360,520,560,  2,1, 32,168,22 }, /* F3 - swampy region */
    { 320,360,400,440,  2,3, 32,168,21 }, /* F4 - plains and rocks */
    { 320,480,520,600,  0,4, 32,176, 0 }, /* F5 - desert area */
    { 320,280,240,200,  5,6, 32,176,23 }, /* F6 - bay / city / farms */
    { 320,640,520,600,  7,4, 32,184, 0 }, /* F7 - volcanic */
    { 320,280,240,200,  5,6, 32,184,24 }, /* F8 - forest and wilderness */
    { 680,720,800,840,  8,9, 96,192, 0 }, /* F9  - inside of buildings */
    { 680,760,800,840, 10,9, 96,192, 0 }  /* F10 - dungeons and caves */
};
```

Each entry: `{ image[0..3], terra1, terra2, sector_block, region_block, setchar }`.

### hero_sector Computation

`hero_sector` is set in `_genmini` (fsubs.asm:1207-1221): takes hero_x high byte and hero_y high byte, looks up `map_mem[(y_high - yreg) * 128 + x_high]` to get the sector ID. In `find_place()` (fmain.c:2651), it's masked to 8 bits, and if region_num > 7 (indoor), 256 is added.

### Map Grid Structure

The `map_mem` region map is 128 columns × 32 rows. Each byte is a sector number (0-255). The sector number indexes into `sector_mem` to get the 128-byte tile data for that sector. `xreg` and `yreg` define the scroll origin for the current region.

## Unresolved

- **Asymmetric prox thresholds**: The right probe blocks at ≥10 and the left probe at ≥8. The exact gameplay rationale is unclear — possibly related to character sprite offset or right-handed collision bias. Could not find a comment explaining the asymmetry.
- **Mask application rule details**: The rendering mask rules (cases 0-7 in fmain.c:2580-2596) need cross-referencing with the blitter occlusion system (`maskit` function) for full understanding.
- **Crystal wall tile IDs**: The specific tile IDs in landscape image files that carry terrain type 12 are embedded in the IFF image metadata and not directly readable from source code. They depend on which tiles in the "mountain3" (index 12) landscape file have high nibble = 12 in their terrain byte. The `terrain.c` tool extracts these from image files that are not in the source tree.
- **Sector 181 identity**: Known to be a special underwater portal sector (fmain.c:1784-1791), but which named location this corresponds to is unclear. It falls within the desert region range (185-254 = "Burning Waste") per `_place_tbl`, but there's no specific place entry for 181. It may be an offshore/underwater location.
- **Stone circle map positions**: The 11 entries in `stone_list` (fmain.c:374-376) give high-byte coordinates but mapping these to named locations requires cross-referencing with the map grid data, which is loaded from disk at runtime.

## Refinement Log
- 2026-04-05: Initial comprehensive discovery pass. All 10 questions answered with citations.
- 2026-04-08: Refined — added Crystal Walls (type 12) + stuff[30] gate detail, Desert Gate dual-check (door + map blocking), Stone Circle teleport mechanic, fiery_death flag definition, Rose (stuff[23]) protection, environ-terrain interaction table, door system reg_id mapping, door type constants table, sector/region system detail. Updated Status to refined. Resolved fiery_death flag (was in Unresolved).
