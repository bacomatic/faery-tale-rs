# Discovery: Astral Plane / Spirit World and Terrain Type 12

**Status**: complete
**Investigated**: 2026-04-08
**Requested by**: orchestrator
**Prompt summary**: Trace what the "astral plane" is, where terrain type 12 (crystal shard passthrough) exists, and how the Crystal Palace and Crystal Shard are distinct systems.

## Summary

The game uses three overlapping terms for related but distinct concepts:

1. **"Astral"** in terrain.c → a landscape image file providing visual tiles for the spirit world
2. **"Spirit Plane"** in narr.asm → the in-game name for the astral area (inside_msg #11: "entered the Spirit Plane")
3. **"Astral plane"** in fmain.c → an encounter extent covering the spirit world geography

Terrain type 12 (crystal shard passthrough walls) does **NOT** exist in the spirit world. It exists in the dungeon labyrinth (Region 8 building interiors) — specifically in small chambers, twisting tunnels, forked intersections, and part of the doom tower.

## Terrain.c Landscape Names and Terra Set Structure

terrain.c:3-18 defines 17 landscape image files:

| Index | Name | Description |
|-------|------|-------------|
| 1 | wild | Wilderness |
| 2 | build | Buildings |
| ... | ... | ... |
| 13 | under | Underground base tiles |
| 14 | cave | Cave tiles |
| 15 | furnish | Interior furnishing tiles |
| 16 | inside | Interior building tiles |
| 17 | astral | Spirit plane visual tiles |

terrain.c:20-32 — the `order[]` array pairs landscapes into terra sets:

```
order[0,1]   = wild(1), palace(9)      → terra set 0:  wild+palace
order[2,3]   = swamp(8), mountain2(10) → terra set 1:  swamp+mountain2
order[4,5]   = wild(1), build(2)       → terra set 2:  wild+build
order[6,7]   = rock(3), tower(5)       → terra set 3:  rock+tower
order[8,9]   = swamp(8), mountain3(12) → terra set 4:  swamp+mountain3
order[10,11] = wild(1), castle(6)      → terra set 5:  wild+castle
order[12,13] = field(7), mountain1(4)  → terra set 6:  field+mountain1
order[14,15] = wild(1), doom(11)       → terra set 7:  wild+doom
order[16,17] = under(13), furnish(15)  → terra set 8:  under+furnish
order[18,19] = inside(16), astral(17)  → terra set 9:  inside+astral
order[20,21] = under(13), cave(14)     → terra set 10: under+cave
```

Each terra set has 128 entries (two landscapes × 64 tiles each). Within a terra set:
- Indices 0-63: first landscape of the pair
- Indices 64-127: second landscape of the pair

## Region Layout for Interior Maps

Regions 8 (F9 - buildings) and 9 (F10 - dungeons) share the same map data (region block 192):

| Region | terra1 (tiles 0-127) | terra2 (tiles 128-255) |
|--------|---------------------|----------------------|
| 8 (F9) | Set 8: under+furnish | Set 9: inside+astral |
| 9 (F10) | Set 10: under+cave | Set 9: inside+astral |

Both regions share terra2 = set 9 (inside+astral). Within the tile ID space:
- Tile IDs 0-63: "under" landscape (shared concept, different data in sets 8 vs 10)
- Tile IDs 64-127: "furnish" (Region 8) or "cave" (Region 9)
- Tile IDs 128-191: "inside" landscape
- Tile IDs 192-255: "astral" landscape

## Terrain Type 12: Location and Function

### Where it exists

Terrain type 12 is found ONLY in terra set 8 (under+furnish), tile index 93, mask=0x33 — decode_map_data.py --find-terrain-type 12.

In terra set 10 (under+cave), tile index 93 has terrain type 1 (impassable wall, no crystal shard bypass). So type-12 passthrough is exclusive to Region 8 (building interiors).

### Which sectors contain tile 93

Binary analysis of sector data at block 96:

| Sector | Tile 93 count | Positions (col,row) | Place name (inside_tbl) |
|--------|--------------|---------------------|------------------------|
| 2 | 3 | (4,1), (11,2), (12,3) | small chamber |
| 3 | 1 | (13,0) | — (catch-all nil) |
| 5 | 2 | (2,1), (3,2) | twisting tunnel |
| 6 | 4 | (11,0), (7,4), (8,5), (9,6) | twisting tunnel |
| 7 | 1 | (13,0) | large chamber |
| 8 | 4 | (2,1), (5,3), (8,4), (9,6) | — (catch-all nil) |
| 9 | 3 | (2,1), (8,5), (9,6) | forked intersection |
| 11 | 1 | (12,1) | — (catch-all nil) |
| 12 | 3 | (4,1), (5,2), (6,4) | — (catch-all nil) |
| 35 | 2 | (11,1), (12,2) | — (between stone corridor [19-33] and octagonal room [36]) |
| 137 | 2 | (14,1), (15,1) | castle (doom tower) — inside_tbl dc.b 135,138,08 |
| 138 | 2 | (0,1), (1,1) | castle (doom tower) — inside_tbl dc.b 135,138,08 |

All these sectors are in the LEFT half (cols 0-31) or the doom tower block (rows 18-19, cols 42-43) of the interior map. **None are in the spirit world area.**

### Tile 93 placement pattern

In each sector, tile 93 appears at the **terminus of diagonal corridors** — the tip of a winding passage that appears to be a dead end. The Crystal Shard reveals these as secret passages that continue beyond the apparent wall. This is a classic hidden passage mechanic.

In sectors 137-138, tile 93 at (14,1)-(15,1) and (0,1)-(1,1) forms a continuous wall at the sector boundary — a doorway-like barrier in the doom tower.

### Code path — fmain.c:1609

```c
// fmain.c:1611 (in walking collision handler)
if (stuff[30] && j==12) goto newloc;
```

When the player has the Crystal Shard (stuff[30]) and the terrain type returned by proxcheck is 12, the collision is bypassed and the player moves through. This check runs ONLY for the player (i==0), after the doorfind check for type 15 and before deviation attempts.

## Spirit World / Astral Plane Geography

### Inside Table Entries — narr.asm:130-132

```asm
dc.b    43,59,11        ; spirit world
dc.b    100,100,11      ; spirit world
dc.b    143,149,11      ; spirit world
```

Message 11 (inside_msg): "% entered the Spirit Plane." — narr.asm:213

### Map Position

Spirit world sectors occupy the RIGHT half of the interior region map, at columns 36-48, rows 2-9. Full grid (from raw binary analysis of region block 192):

```
row 2:  59 59 59 59 59 59 59 59 59 59 59 59 59   (border)
row 3:  59 59 59 59 59 59 59 59 59 59 59 59 59   (border)
row 4:  59 50 49 48 43 57 58 43 43 57 43 57 59   (content)
row 5:  59 51 46 47 59 100 45 58 144 59 59 56 59  (content)
row 6:  59 52 53 54 59 58 57 45 145 143 147 44 59 (content)
row 7:  59 59 59 59 59 55 56 59 146 148 149 59 59 (content)
row 8:  59 59 59 59 59 59 45 59 59 59 59 59 59   (boundary)
row 9:  59 59 59 59 59 59 59 59 59 59 59 59 59   (border)
```

Sector 59 forms the border (all-wall sector). Content sectors form two connected island structures:
- **Main island** (cols 37-43): sectors 43, 45, 47-54, 55-58, 100 — the main spirit world rooms
- **Sub-island** (cols 44-47): sectors 143-149, 44, 56 — a secondary connected area

The spirit world is NOT connected to the dungeon labyrinth (cols 0-31) through the map — there is a 3-column empty gap (cols 33-35). Access must be via a teleport mechanism (see Stargate below).

### Tile Composition

ALL spirit world sectors use exclusively astral landscape tiles (IDs 192-255). 23 distinct tile IDs found across all spirit sectors, ranging from 213 to 236. **No terra set 8 tiles appear in any spirit world sector. No type-12 terrain exists in the spirit world.**

Primary tile IDs: 228 (816 occurrences, dominant floor?), 219 (391), 223 (371), 217 (358), 218 (354), 229/230/231 (~165 each, decorative), 222 (191, used for entire sector 46 = final arena).

### Terrain Types in the Astral Landscape

Terra set 9 indices 64-127 (the "astral" half), corresponding to game tile IDs 192-255:

| Index | TType | Meaning |
|-------|-------|---------|
| 89 | 9 | Pit/fall — triggers FALL state in astral extent |
| 90 | 9 | Pit/fall |
| 91 | 9 | Pit/fall |
| 92 | 9 | Pit/fall |
| 93 | 1 | Impassable wall |
| 94 | 6 | Slippery (ice/slide) |
| 100 | 8 | Lava/fire (walk backwards) |
| 101-108 | 7 | Velocity-based ice (momentum physics) |

The astral landscape is a hazardous environment: pits, ice, lava, and momentum-based physics. This matches the cleric's warning at speak(36): "It is hazardous in the extreme. Space may twist, and time itself may run backwards!" — narr.asm:446-448

The "time running backwards" likely refers to terrain type 8 (lava, k=-3) which makes the player walk backwards at speed -2.

### Astral Plane Encounter Extent — fmain.c:351

```c
{0x2400,0x8200,0x3100,0x8a00,52,3, 1, 8 },    /* astral plane */
```

- Coordinates: (9216, 33280) to (12544, 35328) — maps to cols 36-49, rows 2-10
- etype=52: special pit/encounter zone
- v1=3: base enemy count = 3
- v2=1: random addition = rnd(1) → total 3-4 enemies
- v3=8: encounter_type = 8 → **Loraii** (fmain.c:60)

When xtype==52 (inside astral extent):
- Terrain type 9 (pit) triggers FALL state: luck -= 2, dex = fallstates[brother*6] — fmain.c:1766-1775
- Encounter spawns Loraii (12 HP, arms=6, cleverness=1) — fmain.c:60
- Enemy placement allows terrain type 7 (velocity ice) — fmain.c:2746

### Necromancer — Final Boss

The Necromancer extent: {9563, 33883, 10144, 34462, 53, 4, 1, 6} — fmain.c:344

Converts to sector grid: cols 37-39, rows 4-6 — **inside the spirit world**.
Sector 46 (row 5, col 38) is the "final arena" (inside_tbl dc.b 46,46,0).

The Necromancer: 50 HP, arms=5, file_id=9 (same sprite sheet as Loraii) — fmain.c:61

## Stargate: Portal Between Dungeon and Spirit World

Two door entries form a bidirectional portal:

```c
{ 0x2960,0x8760, 0x2b00,0x92c0, STAIR ,1 }, /* stargate forwards */
{ 0x2b00,0x92c0, 0x2960,0x8780, STAIR ,2 }, /* stargate backwards */
```

When INSIDE (region_num >= 8), doors match on xc2/yc2 and teleport to xc1/yc1:

- **From dungeon to spirit world**: Hero at ~(0x2b00, 0x92c0) matches stargate forward's xc2 → teleports to ~(0x2960, 0x8780) = col 41, row 7 = inside the spirit world
- **From spirit world to dungeon**: Hero at ~(0x2960, 0x8780) matches stargate backward's xc2 → teleports to ~(0x2b00, 0x92c0) = col 43, row 18 = doom tower area

The dungeon-side stargate (col 43, row 18) is adjacent to doom tower sectors 135-138 (rows 18-19, cols 42-43). Sectors 137-138 contain tile 93 (type-12 walls). This places the Crystal Shard walls near the stargate entrance.

## Crystal Palace vs Crystal Shard: Two Distinct Systems

### Crystal Palace — door system

The Crystal Palace is an **outdoor location** (sectors 164-167, place_tbl entry 12): "% came to the Crystal Palace." — narr.asm:179

It has two doors with type CRYST (=7):

```c
{ 0x3DE0,0x1BC0, 0x2EE0,0x93C0, CRYST ,1 }, /* crystal palace */  — fmain.c:261
{ 0x3E00,0x1BC0, 0x2F00,0x93C0, CRYST ,1 }, /* crystal palace */  — fmain.c:262
```

These doors require a Blue Key (KBLUE in open_list) — part of the door/key system, not terrain collision.

### Crystal Shard — terrain bypass

The Crystal Shard is inventory item stuff[30], name "Shard" — fmain.c:417.

Obtained from the Spectre (race 0x8a) at coordinates (12439, 36202) = col 48, row 13 (castle area, NOT in the spirit world extent) — fmain.c:3503, ob_listg[5].

Quest flow:
1. Find the Bone (stuff[29]) — at (3723, 39340) in underground ob_list9 — fmain2.c:1167
2. Give Bone to the Spectre — speak(48): "Good! That spirit now rests quietly in my halls. Take this crystal shard." — narr.asm:488-491
3. Code: `stuff[29] = 0; leave_item(nearest_person, 140)` — fmain.c:3503

The Shard allows passage through terrain type 12 walls in the dungeon labyrinth, providing access to hidden passages and secret corridors.

## Spectre Quest Context

- speak(36) — Cleric: "You must seek your enemy on the spirit plane. It is hazardous in the extreme. Space may twist, and time itself may run backwards!" — narr.asm:446-448
- speak(47) — Spectre: "HE has usurped my place as lord of undead. Bring me bones of the ancient King and I'll help you destroy him." — narr.asm:484-487
- speak(48) — Spectre gives Shard: "Good! That spirit now rests quietly in my halls. Take this crystal shard." — narr.asm:488-491

"HE" = the Necromancer (encounter type 9, final boss in the spirit world final arena).

## Cross-Cutting Findings

1. **decode_map_data.py tool bug**: The region map display (`cmd_region_map`) limits display to 32 columns even for indoor maps that have 64 columns. This hides the entire spirit world area. Code at line ~330: `for c in range(min(cols, 32))`.

2. **decode_map_data.py sector_detail bug**: The `terra_lookup` dictionary is built incorrectly. It only includes terra1 indices 0-63 and terra2 indices 64-127 (raw set indices), but doesn't map tile IDs 128-255 to terra2 entries. This causes all astral tiles (192-255) to show "?" terrain type.

3. **Sector 46 (final arena)**: All 128 tiles are tile ID 222, which maps to terra set 9 index 94 = terrain type 6 (slippery ice). The entire final boss arena is an ice rink.

4. **The Spectre is NOT in the spirit world**: Despite being thematically linked, the Spectre's coordinates (12439, 36202) place it at col 48, row 13 — in the castle area below the spirit world extent boundary (y=35328). The Spectre is in an interior castle, not the astral plane.

5. **Terrain type 12 only in Region 8**: Since terra set 10 (used by Region 9) has tile 93 as terrain type 1 (plain wall), the Crystal Shard passthrough works ONLY in Region 8 (building interiors), not Region 9 (dungeons/caves).

## Unresolved

1. **Exact path from dungeon to stargate**: The stargate's dungeon end (col 43, row 18) is in the doom tower area. How the player navigates from the doom tower entrance to the stargate is unclear — requires tracing the actual walkable path through sectors 135-138 and surrounding areas. Do the type-12 walls in sectors 137-138 specifically gate stargate access?

2. **leave_item(nearest_person, 140)**: When the Spectre gives the Crystal Shard, `leave_item` drops object type 140. How this becomes stuff[30] in the player's inventory requires tracing the pickup logic. Type 140 may correspond to a specific inv_list entry.

3. **Spirit world internal connectivity**: The sector map shows two "islands" of content sectors within the spirit world border. Whether the player can navigate between them (through sector 59 border walls, which use astral tiles with impassable terrain) or if they represent separate areas accessed via different stargates is unclear.

4. **Other entries to spirit world**: The stargate is the only identified portal. Whether there are other mechanisms (extent-triggered teleports, Talisman effects, etc.) that can send the player to the spirit world is not confirmed.

## Refinement Log
- 2026-04-08: Initial comprehensive investigation of astral plane, spirit world geography, terrain type 12 placement, Crystal Palace vs Crystal Shard distinction, stargate mechanism, and encounter system.
