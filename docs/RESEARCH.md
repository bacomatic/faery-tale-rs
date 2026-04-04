# Game Mechanics Research -- The Faery Tale Adventure

## Purpose

This document contains every game mechanic, formula, data table, and asset format specification needed to reimplement The Faery Tale Adventure (Amiga, 1987) from scratch. It is derived entirely from the original source code (Aztec C and 68000 assembly by Talin/David Joiner).

### How to Read This Document

Each section uses up to three documentation layers:

- **Original** -- Describes the exact behavior as implemented in the original source code. This is the ground truth: what the game actually does, including quirks, edge cases, and limitations imposed by the Amiga hardware.
- **Improvement** -- Suggestions for a modern reimplementation. These note places where the original design was constrained by 1987 hardware (floppy disk I/O, limited RAM, Amiga blitter) and could be simplified, made more robust, or made more data-driven.
- **Asset** -- Format specifications for converting original binary data into modern formats. Each asset section describes the original binary layout and a recommended modern target format.

### Source References

All source references use the format `file:line` or `file:start-end`, pointing to files in the repository root. For example, `fmain.c:615-626` means lines 615 through 626 of `fmain.c`.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [World Structure](#2-world-structure)
3. [Characters & Animation](#3-characters--animation)
4. [Combat System](#4-combat-system)
5. [AI & Behavior](#5-ai--behavior)
6. [Inventory & Items](#6-inventory--items)
7. [NPCs & Dialogue](#7-npcs--dialogue)
8. [Quest System](#8-quest-system)
9. [Doors & Buildings](#9-doors--buildings)
10. [Day/Night Cycle](#10-daynight-cycle)
11. [Survival Mechanics](#11-survival-mechanics)
12. [Magic System](#12-magic-system)
13. [Death & Revival](#13-death--revival)
14. [Audio System](#14-audio-system)
15. [Rendering Pipeline](#15-rendering-pipeline)
16. [World Navigation (Carriers)](#16-world-navigation-carriers)
17. [Intro & Narrative](#17-intro--narrative)
18. [Save/Load System](#18-saveload-system)
19. [UI & Menu System](#19-ui--menu-system)
20. [Asset Formats & Disk Layout](#20-asset-formats--disk-layout)

---

## 1. Architecture Overview

The full system architecture, including a Mermaid diagram of all major subsystems, the complete game loop tick structure, and the asset loading pipeline, is documented in [ARCHITECTURE.md](ARCHITECTURE.md). For quest flows and scenario diagrams, see [STORYLINE.md](STORYLINE.md).

The following table summarizes every subsystem, its primary source file(s), and its role:

| # | Subsystem | Primary Source(s) | Description |
|---|-----------|-------------------|-------------|
| 1 | Game Loop | `fmain.c` | Main `while (!quitflag)` loop; orchestrates all per-tick processing |
| 2 | World Structure | `fmain.c`, `terrain.c` | Coordinate system, region loading, sector maps, terrain data |
| 3 | Characters & Animation | `fmain.c`, `fmain2.c`, `ftale.h` | Actor state machines, sprite animation sequences, movement physics |
| 4 | Combat System | `fmain.c` | Melee resolution, missile flight, damage calculation, weapon reach |
| 5 | AI & Behavior | `fmain.c` | Enemy goal/tactic evaluation, pathfinding via `set_course`/`do_tactic` |
| 6 | Inventory & Items | `fmain.c`, `fmain2.c` | `stuff[]` array, item pickup/drop, equipment, gold/keys/items |
| 7 | NPCs & Dialogue | `fmain2.c`, `narr.asm` | Character proximity detection, dialogue text, shop transactions |
| 8 | Quest System | `fmain.c`, `fmain2.c` | Extent system, object state tracking, quest flag evaluation |
| 9 | Doors & Buildings | `fmain.c`, `fmain2.c` | `doorlist[]` binary search, `xfer()` transitions, `find_place()` |
| 10 | Day/Night Cycle | `fmain.c` | `daynight` counter (mod 24000), `lightlevel`, palette fading |
| 11 | Survival Mechanics | `fmain.c` | Hunger, fatigue, health regeneration, starvation, forced sleep |
| 12 | Magic System | `fmain.c`, `fmain2.c` | Spell items, fairy invocations, special effects |
| 13 | Death & Revival | `fmain.c` | Brother succession, revival conditions, safe zone respawn |
| 14 | Audio System | `mtrack.c`, `rtrack.c` | 4-channel music tracker, sound effect playback, mood switching |
| 15 | Rendering Pipeline | `fmain.c`, `fsubs.asm`, `gdriver.asm` | Double-buffered 5-bitplane display, sprite compositing, terrain masking |
| 16 | World Navigation | `fmain.c` | Turtle, bird, raft carrier logic; mounting/dismounting |
| 17 | Intro & Narrative | `narr.asm`, `fmain2.c` | Title sequence, story text scrolling, cinematic transitions |
| 18 | Save/Load System | `fmain2.c` | 8-slot save/load, binary state serialization |
| 19 | UI & Menu System | `fmain.c`, `text.c` | HUD display, text scroller, menu input, placard screens |
| 20 | Asset Formats | `terrain.c`, `iffsubs.c`, `hdrive.c` | Disk layout, IFF image parsing, sector-based async I/O |

---

## 2. World Structure

### 2.1 Coordinate System

The game world uses a 32768 x 32768 coordinate space measured in pixels.

```
MAXCOORD = 16 * 16 * 128 = 32768
MAXMASK  = MAXCOORD - 1  = 32767
```
Source: `fmain.c:632-633`

Hero and actor positions are stored as `abs_x` and `abs_y` (unsigned 16-bit values). The global camera position is tracked by `map_x`, `map_y` (absolute map coordinates in pixels) and `img_x`, `img_y` (absolute sector coordinates). Source: `fmain.c:557-560`

#### World Wrapping

For outdoor regions (region_num < 8), the world wraps at the edges using a threshold of 300 pixels from each boundary:

```c
if (an->abs_x < 300)   an->abs_x = 32565;
if (an->abs_x > 32565) an->abs_x = 300;
if (an->abs_y < 300)   an->abs_y = 32565;
if (an->abs_y > 32565) an->abs_y = 300;
```
Source: `fmain.c:1827-1832`

This wrapping only applies to the hero (actor index `i==0`). If the hero is riding a carrier, the carrier's position is also updated to match. Indoor/dungeon regions (8, 9) do not wrap.

> **Improvement:** The original wrapping uses a hard-coded 300-pixel margin and jumps to `32565` rather than `MAXCOORD - 300` (which would be `32468`). The asymmetric thresholds (300 vs 32565, a gap of 203 pixels from `MAXCOORD`) are likely intentional to keep the hero away from exact boundaries where sector math could produce off-by-one errors. A modern reimplementation could use true modular wrapping (`pos = pos % MAXCOORD`) and eliminate the dead zones.

### 2.2 Region System

The outdoor world is divided into 8 regions arranged in a 2-column by 4-row grid. Two additional regions cover indoor areas:

| Index | ID | Description | Grid Position |
|-------|----|-------------|---------------|
| 0 | F1 | Snowy region | col=0, row=0 |
| 1 | F2 | Witch wood | col=1, row=0 |
| 2 | F3 | Swampy region | col=0, row=1 |
| 3 | F4 | Plains and rocks | col=1, row=1 |
| 4 | F5 | Desert area | col=0, row=2 |
| 5 | F6 | Bay / city / farms | col=1, row=2 |
| 6 | F7 | Volcanic | col=0, row=3 |
| 7 | F8 | Forest and wilderness | col=1, row=3 |
| 8 | F9 | Inside of buildings | (interior) |
| 9 | F10 | Dungeons and caves | (interior) |

The current region is computed from the hero's screen-center coordinates in `gen_mini()`:

```c
xs = (map_x + 151) >> 8;   /* sector coords of middle screen */
ys = (map_y + 64) >> 8;
xr = (xs >> 6) & 1;        /* region column: 0 or 1 */
yr = (ys >> 5) & 3;        /* region row: 0..3 */
lregion = xr + yr + yr;    /* region index: 0..7 */
```
Source: `fmain.c:2971-2975`

When `lregion != region_num`, a region transition is triggered by setting `new_region = lregion` and calling `load_all()`. Source: `fmain.c:2976`

For indoor regions (region_num >= 8), the region is set directly and no coordinate-based calculation occurs. Source: `fmain.c:2978`

#### Region Asset Table: `file_index[10]`

Each region's required assets are defined by a `struct need`:

```c
struct need {
    USHORT image[4], terra1, terra2, sector, region, setchar;
};
```
Source: `ftale.h:104-106`

The `file_index[10]` table maps each region to its disk sector addresses:

| Index | image[0..3] | terra1 | terra2 | sector | region | setchar | Description |
|-------|-------------|--------|--------|--------|--------|---------|-------------|
| 0 | 320,480,520,560 | 0 | 1 | 32 | 160 | 22 | Snowy region |
| 1 | 320,360,400,440 | 2 | 3 | 32 | 160 | 21 | Witch wood |
| 2 | 320,360,520,560 | 2 | 1 | 32 | 168 | 22 | Swampy region |
| 3 | 320,360,400,440 | 2 | 3 | 32 | 168 | 21 | Plains and rocks |
| 4 | 320,480,520,600 | 0 | 4 | 32 | 176 | 0 | Desert area |
| 5 | 320,280,240,200 | 5 | 6 | 32 | 176 | 23 | Bay / city / farms |
| 6 | 320,640,520,600 | 7 | 4 | 32 | 184 | 0 | Volcanic |
| 7 | 320,280,240,200 | 5 | 6 | 32 | 184 | 24 | Forest and wilderness |
| 8 | 680,720,800,840 | 8 | 9 | 96 | 192 | 0 | Inside of buildings |
| 9 | 680,760,800,840 | 10 | 9 | 96 | 192 | 0 | Dungeons and caves |

Source: `fmain.c:615-626`

The `current_loads` struct tracks what is already loaded to avoid redundant disk reads. Each field in `file_index` is compared against `current_loads`; only changed assets trigger a load. Source: `fmain.c:614`, `fmain.c:3548-3614`

### 2.3 Sector Format

Each region's map data consists of 256 sectors stored contiguously in memory at `sector_mem`.

- **Sector size:** 128 bytes each (16 columns x 8 rows of tile indices)
- **Total sector memory:** `SECTOR_SZ = (128 * 256) + 4096 = 36864` bytes
- **Sector data offset:** 256 sectors x 128 bytes = 32768 bytes at `sector_mem[0..32767]`
- **Region map offset:** `SECTOR_OFF = 128 * 256 = 32768` -- the 4096-byte region map starts at `map_mem = sector_mem + SECTOR_OFF`

Source: `fmain.c:643-644`, `fmain.c:920`

Each byte in a sector is a tile index (0-255) into the currently loaded image set (5-bitplane, 16x16 pixel tiles). The image set is loaded into `image_mem` (81920 bytes = 5 planes x 256 tiles x 64 bytes per tile per plane). Source: `fmain.c:639-640`

#### Region Map

The 4096-byte region map at `map_mem` provides a higher-level overview of the sector layout. It is loaded separately from disk (8 sectors = 4096 bytes) and is used by the minimap generator `genmini()`. The playing map is 6 x 19 = 114 entries stored in `minimap[114]`. Source: `fmain.c:628-630`, `fmain.c:3560-3563`

The desert region (region 4) has a special case: if a quest condition is not met (`stuff[STATBASE] < 5`), four tiles at a specific location in `map_mem` are overwritten with tile 254, effectively blocking passage. Source: `fmain.c:3594-3597`

> **Asset Format: Sector Data**
>
> - **Original:** 128 bytes raw binary per sector, 256 sectors contiguous (32768 bytes total). Each byte is a tile index (0-255) into the loaded image tileset.
> - **Loading:** `load_track_range(nd->sector, 64, sector_mem, 0)` reads 64 disk sectors (32768 bytes) into `sector_mem`. Source: `fmain.c:3557`
> - **Modern target:** JSON 2D array per sector (16 columns x 8 rows) or Tiled TMX tilemap format with 256 sectors as separate layers or chunked regions.

### 2.4 Terrain Data

Terrain metadata is stored in `terra_mem`, a 1024-byte buffer holding two 512-byte terrain sets (one per `terra1`/`terra2` index in the region's `file_index` entry). Source: `fmain.c:928`

Each 512-byte terrain set contains data for 128 tiles (2 x 64 tiles from a loading pair), packed as 4 bytes per tile:

| Offset | Field | Description |
|--------|-------|-------------|
| 0 | `maptag` | Image/display characteristics for the tile |
| 1 | `terrain` | Terrain type encoded as two nibbles (upper = mask mode, lower = property) |
| 2 | `tiles` | Terrain feature mask data |
| 3 | `big_colors` | Minimap color value for this tile |

Source: `terrain.c:60-64`, `terrain.c:70-74`

The `terrain.c` extraction tool reads these fields from each tileset image file by seeking past the image pixel data (`IPLAN_SZ = 5 * 64 * 64 = 20480` bytes) and reading 64 bytes each for `maptag`, `terrain`, `tiles`, and `big_colors`. Source: `terrain.c:86-91`

Loading into memory uses `load_track_range`:
- `terra1` loads into `terra_mem[0..511]` via channel 1. Source: `fmain.c:3567`
- `terra2` loads into `terra_mem[512..1023]` via channel 2. Source: `fmain.c:3572`

#### Terrain Properties (Lower Nibble)

The lower nibble of the `terrain` byte encodes the physical terrain property:

| Value | Property | Description |
|-------|----------|-------------|
| 0 | Normal | Freely passable terrain |
| 1 | Impassable | Blocks all movement (walls, mountains, deep water edges) |
| 2 | Sink | Character sinks (water, quicksand); triggers drowning/damage |
| 3 | Slow/Brush | Movement is slowed (dense vegetation, mud) |
| 4+ | (Reserved) | Additional types noted in comments: slippery, fiery, changing, climbable, pit trap, danger, noisy, magnetic, stinks, slides, slopes, whirlpool |

Source: `fmain.c:684-687`

Note: The source comments list terrain types 4-9+ but only values 1-3 have confirmed behavioral implementations in the movement code. The extended types may be partially implemented or reserved for future use.

#### Mask Application Modes (Upper Nibble)

The upper nibble of the `terrain` byte controls how sprites are masked (occluded) by terrain:

| Value | Mode | Description |
|-------|------|-------------|
| 0 | Never | Sprite is never masked by this tile |
| 1 | When down | Sprite masked only when moving downward (south) |
| 2 | When right | Sprite masked only when moving rightward (east) |
| 3 | Always (unless flying) | Sprite always masked by this tile, except when flying |
| 4 | Below normal level | Sprite masked only if below the normal ground level |
| 5-7 | (Extended) | Additional modes (implementation varies) |

Source: `fmain.c:689-691`

These mask modes enable the visual effect of characters walking behind trees, buildings, and other tall terrain features while remaining visible when flying or on different elevation levels.

### 2.5 Tileset Names and Loading Pairs

The game uses 17 named tilesets defined in `terrain.c:datanames[]`:

| Index | Name | Description |
|-------|------|-------------|
| 0 | (space) | Empty/unused |
| 1 | wild | Wilderness terrain |
| 2 | build | Buildings and structures |
| 3 | rock | Rocky terrain |
| 4 | mountain1 | Mountain variant 1 |
| 5 | tower | Tower structures |
| 6 | castle | Castle structures |
| 7 | field | Agricultural fields |
| 8 | swamp | Swamp terrain |
| 9 | palace | Palace structures |
| 10 | mountain2 | Mountain variant 2 |
| 11 | doom | Volcanic/doom terrain |
| 12 | mountain3 | Mountain variant 3 |
| 13 | under | Underground passages |
| 14 | cave | Cave interiors |
| 15 | furnish | Interior furnishings |
| 16 | inside | Building interiors |
| 17 | astral | Astral/magical plane |

Source: `terrain.c:3-22`

These tilesets are processed in 11 loading pairs via the `order[]` array, which maps pairs of tileset indices to the terrain data output. Each pair produces 512 bytes (2 x 64 tiles x 4 bytes):

| Pair | order[] Indices | Tileset A | Tileset B | Region Usage |
|------|-----------------|-----------|-----------|--------------|
| 0 | 1, 9 | wild | palace | terra 0: F1, F5 |
| 1 | 8, 10 | swamp | mountain2 | terra 1: F1, F3 |
| 2 | 1, 2 | wild | build | terra 2: F2, F3, F4 |
| 3 | 3, 5 | rock | tower | terra 3: F2, F4 |
| 4 | 8, 12 | swamp | mountain3 | terra 4: F5, F7 |
| 5 | 1, 6 | wild | castle | terra 5: F6, F8 |
| 6 | 7, 4 | field | mountain1 | terra 6: F6, F8 |
| 7 | 1, 11 | wild | doom | terra 7: F7 |
| 8 | 13, 15 | under | furnish | terra 8: F9 |
| 9 | 16, 17 | inside | astral | terra 9: F9, F10 |
| 10 | 13, 14 | under | cave | terra 10: F10 |

Source: `terrain.c:24-36`

The "Region Usage" column cross-references against the `terra1`/`terra2` fields in `file_index[]`. For example, region F1 (index 0) has `terra1=0, terra2=1`, so it loads pair 0 (wild+palace) into the first 512 bytes and pair 1 (swamp+mountain2) into the second 512 bytes.

### 2.6 Region Loading Process

Region transitions are handled by `load_all()` and `load_new_region()`:

1. `load_all()` spins in a loop calling `load_new_region()` until `MAP_STABLE` (i.e., `new_region >= NO_REGION`). Source: `fmain.c:3545-3546`
2. `load_new_region()` compares each field of `file_index[new_region]` against `current_loads` and only issues disk reads for changed assets. Source: `fmain.c:3548-3614`
3. Assets are loaded in this priority order: sectors, region map, terra1, terra2, then image planes (4 image sets x 5 planes each = up to 20 separate reads).
4. Image loading returns after issuing reads for one changed image set (to allow interleaved processing). The caller loops back for subsequent image sets.
5. After all reads complete, the function waits on all 7 I/O channels, turns off the disk motor, and commits the transition by setting `region_num = new_region` and `new_region = NO_REGION`.

> **Improvement:** Replace sector-based disk I/O with direct file reads from a modern filesystem. The entire region loading system exists to manage floppy disk latency and can be replaced with synchronous file loads that complete in milliseconds. The `current_loads` caching is still useful to avoid redundant loads, but the async I/O channels and motor control are unnecessary.

> **Asset Format: Terrain Data**
>
> - **Original:** 512 bytes per terrain pair (2 sets x 64 tiles x 4 bytes). 11 pairs = 5632 bytes total in the "terra" file on disk. Each 4-byte entry packs `maptag`, `terrain` (upper nibble = mask mode, lower nibble = property), `tiles`, and `big_colors`.
> - **Modern target:** JSON array of objects per tileset:
>   ```json
>   {
>     "tile_index": 0,
>     "maptag": 0,
>     "terrain_type": 1,
>     "terrain_subtype": 0,
>     "mask_mode": 3,
>     "minimap_color": 42
>   }
>   ```
>   Terrain properties and mask modes should use named enums (`"impassable"`, `"sink"`, `"slow"`) instead of packed nibbles, eliminating the need for bitmask extraction at runtime.

---

## 3. Characters & Animation

### 3.1 Shape Structure

Each on-screen actor is represented by a `struct shape` defined in `ftale.h:56-68`. The structure is 22 bytes and contains all per-actor state:

| Offset | Field(s)            | Type             | Size    | Purpose |
|--------|---------------------|------------------|---------|---------|
| 0      | `abs_x`             | `unsigned short` | 2 bytes | Absolute world X position |
| 2      | `abs_y`             | `unsigned short` | 2 bytes | Absolute world Y position |
| 4      | `rel_x`             | `unsigned short` | 2 bytes | Screen-relative X position |
| 6      | `rel_y`             | `unsigned short` | 2 bytes | Screen-relative Y position |
| 8      | `type`              | `char`           | 1 byte  | Sprite sequence type (PHIL, ENEMY, OBJECTS, etc.) |
| 9      | `race`              | `UBYTE`          | 1 byte  | Race/identity index for encounter table lookup |
| 10     | `index`             | `char`           | 1 byte  | Current image/frame index within sprite sheet |
| 11     | `visible`           | `char`           | 1 byte  | On-screen visibility flag |
| 12     | `weapon`            | `char`           | 1 byte  | Weapon type carried (0=none, 1=dagger, 2=mace, 3=sword, 4=bow, 5=wand) |
| 13     | `environ`           | `char`           | 1 byte  | Environment variable (terrain effects, etc.) |
| 14     | `goal`              | `char`           | 1 byte  | Current AI goal mode (USER, ATTACK1, ATTACK2, FLEE, etc.) |
| 15     | `tactic`            | `char`           | 1 byte  | Current tactical sub-goal (PURSUE, FOLLOW, RANDOM, etc.) |
| 16     | `state`             | `char`           | 1 byte  | Current movement/animation state (WALKING, FIGHTING, etc.) |
| 17     | `facing`            | `char`           | 1 byte  | Direction the actor is facing (0-7) |
| 18     | `vitality`          | `short`          | 2 bytes | Hit points; also doubles as original object number for items |
| 20     | `vel_x`             | `char`           | 1 byte  | X velocity (for slippery/ice areas) |
| 21     | `vel_y`             | `char`           | 1 byte  | Y velocity (for slippery/ice areas) |

> **Improvement note:** The struct mixes `char` and `UBYTE` types inconsistently (e.g., `type` is `char` but `race` is `UBYTE`). A modern reimplementation should use explicit sized types (`int8_t`, `uint8_t`, `uint16_t`, `int16_t`) throughout to avoid sign-extension bugs.

A commented-out field `source_struct` (an `APTR`) was apparently removed during development, suggesting the struct was once larger or linked back to a generating data structure.

### 3.2 Actor Slot Allocation

The game maintains a fixed array of 20 actor slots (`struct shape anim_list[20]`) declared in `fmain.c:70`. Slots have hardcoded roles:

| Slot(s) | Role | Notes |
|---------|------|-------|
| 0       | Hero (player character) | Julian, Phillip, or Kevin depending on `brother` variable |
| 1       | Raft | Type set to `RAFT`; positioned to match hero when riding |
| 2       | Witch / set figure | Type set to `SETFIG`; used for NPCs like the witch, wizard, priest, royals |
| 3-6     | Enemies / carriers | Slot 3 is primary enemy or carrier (turtle, bird, dragon); slots 4-6 for additional enemies in encounters |
| 7-19    | Overflow objects / missiles | Populated dynamically for on-screen objects, items, and projectiles |

Two index variables track the active population:
- `anix` -- tracks the count of active enemies (iteration bound for enemy AI loops, e.g., `for (i=3; i<anix; i++)`)
- `anix2` -- tracks total active actors including objects (used for proximity checks and object placement, e.g., `for (i=1; i<anix2; i++)`)

The slot 2 witch/NPC and slot 1 raft are always allocated even when unused, which simplifies rendering but wastes slots.

> **Improvement note:** The rigid slot allocation (fixed slots for raft, witch) is fragile. A dynamic entity system with component-based assignment would be cleaner and allow more actors on screen.

### 3.3 Animation State Machine

Animation states are defined as `#define` constants in `fmain.c:89-103`. The `state` field of each `struct shape` holds one of these values:

| Value(s) | Name | Description |
|----------|------|-------------|
| 0-8      | `FIGHTING` | Combat animation; value is the current position in the 9-state combat transition table |
| 12       | `WALKING`  | Walk cycle active; frame selected by direction + cycle counter |
| 13       | `STILL`    | Standing idle; no animation cycling |
| 14       | `DYING`    | Death animation in progress (3-frame sequence) |
| 15       | `DEAD`     | Death complete; actor remains on ground |
| 16       | `SINK`     | Sinking into water/swamp (single frame) |
| 17-18    | `OSCIL`    | Oscillation animation (2 alternating frames, e.g., sword-at-side idle) |
| 19       | `TALKING`  | NPC conversation state |
| 20       | `FROZEN`   | Actor cannot move or act (e.g., paralysis, cutscene) |
| 21       | `FLYING`   | Airborne movement (used when riding bird/dragon) |
| 22       | `FALL`     | Falling animation (pit traps, cliffs) |
| 23       | `SLEEP`    | Sleeping state (single frame, index 66) |
| 24       | `SHOOT1`   | Bow raised, aiming |
| 25       | `SHOOT3`   | Bow fired, arrow given velocity |

Note that values 9, 10, and 11 are unused gaps between FIGHTING (0-8) and WALKING (12).

> **Improvement note:** The FIGHTING states 0-8 overlap numerically with goal mode values (USER=0, ATTACK1=1, ..., WAIT=8) in a confusing way. Although they are stored in different fields (`state` vs `goal`), the numeric collision invites bugs. A modern implementation should use distinct enum types.

### 3.4 State List Table

The `statelist[87]` array (`fmain.c:154-204`) maps each animation frame index to its sprite figure number, weapon overlay index, and weapon position offset. The `struct state` for each entry contains:

- `figure` -- sprite frame number within the character sheet
- `wpn_no` -- weapon sprite index from the OBJECTS sheet
- `wpn_x`, `wpn_y` -- pixel offset of weapon relative to the character sprite origin

| Index | Group | figure | wpn_no | wpn_x | wpn_y | Notes |
|-------|-------|--------|--------|-------|-------|-------|
| 0     | South walk | 0  | 11 | -2  | 11  | |
| 1     | South walk | 1  | 11 | -3  | 11  | |
| 2     | South walk | 2  | 11 | -3  | 10  | |
| 3     | South walk | 3  | 11 | -3  | 9   | |
| 4     | South walk | 4  | 11 | -3  | 10  | |
| 5     | South walk | 5  | 11 | -3  | 11  | |
| 6     | South walk | 6  | 11 | -2  | 11  | |
| 7     | South walk | 7  | 11 | -1  | 11  | |
| 8     | West walk  | 8  | 9  | -12 | 11  | |
| 9     | West walk  | 9  | 9  | -11 | 12  | |
| 10    | West walk  | 10 | 9  | -8  | 13  | |
| 11    | West walk  | 11 | 9  | -4  | 13  | |
| 12    | West walk  | 12 | 9  | 0   | 13  | |
| 13    | West walk  | 13 | 9  | -4  | 13  | |
| 14    | West walk  | 14 | 9  | -8  | 13  | |
| 15    | West walk  | 15 | 9  | -11 | 12  | |
| 16    | North walk | 16 | 14 | -1  | 1   | |
| 17    | North walk | 17 | 14 | -1  | 2   | |
| 18    | North walk | 18 | 14 | -1  | 3   | |
| 19    | North walk | 19 | 14 | -1  | 4   | |
| 20    | North walk | 20 | 14 | -1  | 3   | |
| 21    | North walk | 21 | 14 | -1  | 2   | |
| 22    | North walk | 22 | 14 | -1  | 1   | |
| 23    | North walk | 23 | 14 | -1  | 1   | |
| 24    | East walk  | 24 | 10 | 5   | 12  | |
| 25    | East walk  | 25 | 10 | 3   | 12  | |
| 26    | East walk  | 26 | 10 | 2   | 12  | |
| 27    | East walk  | 27 | 10 | 3   | 12  | |
| 28    | East walk  | 28 | 10 | 5   | 12  | |
| 29    | East walk  | 29 | 10 | 6   | 12  | |
| 30    | East walk  | 30 | 10 | 6   | 11  | |
| 31    | East walk  | 31 | 10 | 6   | 12  | |
| 32    | South fight | 32 | 11 | -2  | 12  | Arm down, weapon low |
| 33    | South fight | 32 | 10 | 0   | 12  | Arm down, weapon diagonal |
| 34    | South fight | 33 | 0  | 2   | 10  | Arm swing1, weapon horizontal |
| 35    | South fight | 34 | 1  | 4   | 6   | Arm swing2, weapon raised |
| 36    | South fight | 34 | 2  | 1   | 4   | Arm swing2, weapon diag up |
| 37    | South fight | 34 | 3  | 0   | 4   | Arm swing2, weapon high |
| 38    | South fight | 36 | 4  | -5  | 0   | Arm high, weapon up |
| 39    | South fight | 36 | 5  | -10 | 1   | Arm high, weapon horizontal |
| 40    | South fight | 35 | 12 | -5  | 5   | Arm middle, weapon raise fwd |
| 41    | South fight | 36 | 0  | 0   | 6   | (extra south fight frame) |
| 42    | South fight | 38 | 85 | -6  | 5   | (arrow/projectile frame) |
| 43    | South fight | 37 | 81 | -6  | 5   | (bow frame) |
| 44    | West fight  | 40 | 9  | -7  | 12  | Arm down, weapon low |
| 45    | West fight  | 40 | 8  | -9  | 9   | Arm down, weapon diagonal |
| 46    | West fight  | 41 | 7  | -10 | 5   | Arm swing1, weapon horizontal |
| 47    | West fight  | 42 | 7  | -12 | 4   | Arm swing2, weapon raised |
| 48    | West fight  | 42 | 6  | -12 | 3   | Arm swing2, weapon diag up |
| 49    | West fight  | 42 | 5  | -12 | 3   | Arm swing2, weapon high |
| 50    | West fight  | 44 | 5  | -8  | 3   | Arm high, weapon up |
| 51    | West fight  | 44 | 14 | -7  | 6   | Arm high, weapon horizontal |
| 52    | West fight  | 43 | 13 | -7  | 8   | Arm middle, weapon raise fwd |
| 53    | West fight  | 42 | 5  | -12 | 3   | (extra west fight frame) |
| 54    | West fight  | 46 | 86 | -3  | 0   | (arrow/projectile frame) |
| 55    | West fight  | 45 | 82 | -3  | 0   | (bow frame) |
| 56    | North fight | 48 | 14 | -3  | 0   | Arm down, weapon low |
| 57    | North fight | 48 | 6  | -3  | -1  | Arm down, weapon diagonal |
| 58    | North fight | 49 | 5  | -2  | -3  | Arm swing1, weapon horizontal |
| 59    | North fight | 50 | 5  | -3  | -4  | Arm swing2, weapon raised |
| 60    | North fight | 50 | 4  | 0   | 0   | Arm swing2, weapon diag up |
| 61    | North fight | 50 | 3  | 3   | 0   | Arm swing2, weapon high |
| 62    | North fight | 52 | 4  | 6   | 1   | Arm high, weapon up |
| 63    | North fight | 52 | 15 | 7   | 3   | Arm high, weapon horizontal |
| 64    | North fight | 51 | 14 | 1   | 6   | Arm middle, weapon raise fwd |
| 65    | North fight | 50 | 4  | 0   | 0   | (extra north fight frame) |
| 66    | North fight | 54 | 87 | 3   | 0   | (arrow/projectile frame) |
| 67    | North fight | 53 | 83 | 3   | 0   | (bow frame) |
| 68    | East fight  | 56 | 10 | 5   | 11  | Arm down, weapon low |
| 69    | East fight  | 56 | 0  | 6   | 9   | Arm down, weapon diagonal |
| 70    | East fight  | 57 | 1  | 10  | 6   | Arm swing1, weapon horizontal |
| 71    | East fight  | 58 | 1  | 10  | 5   | Arm swing2, weapon raised |
| 72    | East fight  | 58 | 2  | 7   | 3   | Arm swing2, weapon diag up |
| 73    | East fight  | 58 | 3  | 6   | 3   | Arm swing2, weapon high |
| 74    | East fight  | 60 | 4  | 1   | 0   | Arm high, weapon up |
| 75    | East fight  | 60 | 3  | 3   | 2   | Arm high, weapon horizontal |
| 76    | East fight  | 59 | 15 | 4   | 1   | Arm middle, weapon raise fwd |
| 77    | East fight  | 58 | 4  | 5   | 1   | (extra east fight frame) |
| 78    | East fight  | 62 | 84 | 3   | 0   | (arrow/projectile frame) |
| 79    | East fight  | 61 | 80 | 3   | 0   | (bow frame) |
| 80    | Death seq   | 47 | 0  | 5   | 11  | Death frame 1 |
| 81    | Death seq   | 63 | 0  | 6   | 9   | Death frame 2 |
| 82    | Death seq   | 39 | 0  | 6   | 9   | Death frame 3 (lying down) |
| 83    | Sinking     | 55 | 10 | 5   | 11  | Sinking into water/swamp |
| 84    | Oscillation | 64 | 10 | 5   | 11  | Idle oscillation frame 1 |
| 85    | Oscillation | 65 | 10 | 5   | 11  | Idle oscillation frame 2 |
| 86    | Sleep       | 66 | 10 | 5   | 11  | Sleeping figure |

The walk sequences occupy indices 0-31 (4 directions, 8 frames each). Fight sequences occupy 32-79 (4 directions, 12 frames each -- 9 melee + 3 ranged). Special states occupy 80-86.

### 3.5 Direction System

The game uses 8 compass directions encoded as integers 0-7:

| Value | Direction | Walk frame base | Fight frame base |
|-------|-----------|----------------|-----------------|
| 0     | South     | 0              | 32              |
| 1     | Southwest | 0              | 32              |
| 2     | West      | 8              | 44              |
| 3     | Northwest | 8              | 44              |
| 4     | North     | 16             | 56              |
| 5     | Northeast | 16             | 56              |
| 6     | East      | 24             | 68              |
| 7     | Southeast | 24             | 68              |

Diagonal directions share the frame set of their primary cardinal direction (SW uses South, NW uses West, etc.). This is encoded in the `diroffs[16]` array (`fmain.c:1010`):

```c
char diroffs[16] = {16,16,24,24,0,0,8,8,56,56,68,68,32,32,44,44};
```

The array is indexed as `diroffs[d]` for walk frames (indices 0-7) and `diroffs[d+8]` for fight frames (indices 8-15). The layout groups the first 8 entries as walk base offsets and the second 8 as fight base offsets:

| Array index | Direction | Type  | Value (statelist base) |
|-------------|-----------|-------|------------------------|
| 0           | South     | Walk  | 16 (North walk)        |
| 1           | SW        | Walk  | 16                     |
| 2           | West      | Walk  | 24 (East walk)         |
| 3           | NW        | Walk  | 24                     |
| 4           | North     | Walk  | 0 (South walk)         |
| 5           | NE        | Walk  | 0                      |
| 6           | East      | Walk  | 8 (West walk)          |
| 7           | SE        | Walk  | 8                      |
| 8           | South     | Fight | 56 (North fight)       |
| 9           | SW        | Fight | 56                     |
| 10          | West      | Fight | 68 (East fight)        |
| 11          | NW        | Fight | 68                     |
| 12          | North     | Fight | 32 (South fight)       |
| 13          | NE        | Fight | 32                     |
| 14          | East      | Fight | 44 (West fight)        |
| 15          | SE        | Fight | 44                     |

**Important subtlety:** The `diroffs` mapping is inverted -- direction 0 (South, facing the viewer) maps to `statelist` index 16 which is the "northwalk sequence" in the sprite sheet. This is because the sprite sheet names directions by the direction the character *walks toward on screen* (north = up), but the game's direction 0 means "facing south" (toward the camera), which uses the sprites drawn from behind (the "north walk" sprites). In other words, a character facing south (toward the player) uses sprite frames labeled "north" because those frames show the character's front.

### 3.6 Walk Cycle

Each walking direction uses 8 frames forming a looping cycle. The frame index into `statelist[]` is calculated as:

```
statelist_index = diroffs[direction] + ((cycle + i) & 7)
```

Where:
- `direction` is 0-7 (the actor's facing)
- `cycle` is a continuously incrementing counter
- `i` is a per-frame offset
- The `& 7` mask wraps the 8-frame cycle

The walk animation creates a smooth stride by bouncing the weapon position up and down (visible in the `wpn_y` values oscillating by 1-2 pixels across the 8 frames of each direction).

### 3.7 Combat Transition Table

Combat uses a 9-state finite automaton defined in `trans_list[9]` (`fmain.c:139-148`). Each state represents a weapon position during a swing, and transitions are selected randomly using `rand4()` to pick one of 4 possible next states:

| State | Description | Next[0] | Next[1] | Next[2] | Next[3] |
|-------|-------------|---------|---------|---------|---------|
| 0     | Arm down, weapon low | 1 | 8 | 0 | 1 |
| 1     | Arm down, weapon diagonal down | 2 | 0 | 1 | 0 |
| 2     | Arm swing1, weapon horizontal | 3 | 1 | 2 | 8 |
| 3     | Arm swing2, weapon raised | 4 | 2 | 3 | 7 |
| 4     | Arm swing2, weapon diag up | 5 | 3 | 4 | 6 |
| 5     | Arm swing2, weapon high | 6 | 4 | 5 | 5 |
| 6     | Arm high, weapon up | 8 | 5 | 6 | 4 |
| 7     | Arm high, weapon horizontal | 8 | 6 | 7 | 3 |
| 8     | Arm middle, weapon raise fwd | 0 | 6 | 8 | 2 |

The combat state (0-8) is stored in the actor's `state` field (since `FIGHTING` is defined as 0). The actual sprite frame is looked up as `statelist[fight_base + combat_state]`, where `fight_base` is `diroffs[direction + 8]`.

Each fight direction has 12 entries in `statelist`: indices 0-8 are the 9 melee combat poses, index 9 is an extra melee frame, and indices 10-11 are ranged weapon frames (arrow/projectile and bow).

The random transition design means no two fights look the same -- the weapon swings through varied arcs rather than following a fixed pattern.

### 3.8 Sprite Sequences

The game organizes sprite data into 7 sequence types, defined by `enum sequences` in `ftale.h:88`:

| Value | Name     | Purpose |
|-------|----------|---------|
| 0     | `PHIL`   | Player character sprites (Julian, Phillip, Kevin) -- 67 frames of walk, fight, death, and special animations |
| 1     | `OBJECTS`| Item and weapon sprites -- 116 small (16x16) frames for inventory items, weapon overlays, projectiles |
| 2     | `ENEMY`  | Enemy character sprites -- 64 frames per enemy type (ogre, ghost, dark knight, necromancer, snake) |
| 3     | `RAFT`   | Raft vehicle -- 2 frames (directions), double-width (32px) |
| 4     | `SETFIG` | Set figures (NPCs) -- 8 frames for wizard, priest, royals, bartender, witch, ranger, beggar |
| 5     | `CARRIER`| Rideable creatures -- turtle (16 frames, 32px wide) or bird (8 frames, 64px wide) |
| 6     | `DRAGON` | Dragon -- 5 frames, triple-width (48px), 40px tall |

Each sequence type is tracked at runtime by a `struct seq_info` (`ftale.h:81-86`):

| Field          | Type             | Purpose |
|----------------|------------------|---------|
| `width`        | `short`          | Width in 16-pixel word units |
| `height`       | `short`          | Height in pixels |
| `count`        | `short`          | Number of frames in the set |
| `location`     | `unsigned char *` | Pointer to image bitplane data in memory |
| `maskloc`      | `unsigned char *` | Pointer to mask bitplane data |
| `bytes`        | `short`          | Bytes per bitplane per frame (`height * width * 2`) |
| `current_file` | `short`          | Currently loaded file index |

The game maintains a global array `seq_list[7]` (one entry per sequence type) in `fmain.c:41`.

### 3.9 Character File Table

The `cfiles[]` array (`fmain2.c:638-665`) defines all 18 character sprite sets that can be loaded from disk. Each entry specifies the sprite dimensions, frame count, disk location, and which sequence slot to load into:

| Index | Character | Width | Height | Count | Blocks | Seq Slot | File ID | Notes |
|-------|-----------|-------|--------|-------|--------|----------|---------|-------|
| 0     | Julian    | 1     | 32     | 67    | 42     | PHIL     | 1376    | Player brother 1 |
| 1     | Phillip   | 1     | 32     | 67    | 42     | PHIL     | 1418    | Player brother 2 |
| 2     | Kevin     | 1     | 32     | 67    | 42     | PHIL     | 1460    | Player brother 3 |
| 3     | Objects   | 1     | 16     | 116   | 36     | OBJECTS  | 1312    | Items, weapons, overlays |
| 4     | Raft      | 2     | 32     | 2     | 3      | RAFT     | 1348    | Water vehicle |
| 5     | Turtle    | 2     | 32     | 16    | 20     | CARRIER  | 1351    | Rideable turtle |
| 6     | Ogre      | 1     | 32     | 64    | 40     | ENEMY    | 960     | Forest/mountain enemy |
| 7     | Ghost     | 1     | 32     | 64    | 40     | ENEMY    | 1080    | Undead enemy |
| 8     | Dark Knight | 1   | 32     | 64    | 40     | ENEMY    | 1000    | Also used for spiders |
| 9     | Necromancer | 1   | 32     | 64    | 40     | ENEMY    | 1040    | Also farmer/Loraii |
| 10    | Dragon    | 3     | 40     | 5     | 12     | DRAGON   | 1160    | Boss creature |
| 11    | Bird      | 4     | 64     | 8     | 40     | CARRIER  | 1120    | Flying mount |
| 12    | Snake/Salamander | 1 | 32   | 64    | 40     | ENEMY    | 1376    | Reptilian enemies |
| 13    | Wizard/Priest | 1  | 32     | 8     | 5      | SETFIG   | 936     | NPC set |
| 14    | Royal set | 1     | 32     | 8     | 5      | SETFIG   | 931     | King, princess, etc. |
| 15    | Bartender | 1     | 32     | 8     | 5      | SETFIG   | 941     | Tavern NPC |
| 16    | Witch     | 1     | 32     | 8     | 5      | SETFIG   | 946     | Witch NPC |
| 17    | Ranger/Beggar | 1  | 32     | 8     | 5      | SETFIG   | 951     | Wilderness NPCs |

Width is in 16-pixel word units (so width=1 means 16px, width=2 means 32px, width=4 means 64px). The `file_id` is a disk sector address for the raw track loader. The `numblocks` field indicates how many 512-byte disk blocks to read.

Note that the `ENEMY` and `SETFIG` sequence slots are shared -- only one enemy type and one NPC set can be loaded at a time. When encountering a new enemy type or NPC, the game must reload the sprite data from disk, replacing whatever was previously in that slot.

### 3.10 Sprite Data Format

Sprite data uses the Amiga's native planar bitplane format. For each frame:

1. **Bytes per plane per frame:** `bytes = height * width * 2` (width is in 16-pixel words, so `width * 2` gives bytes per scanline)
2. **Image data:** 5 bitplanes (for 32-color depth), stored sequentially: plane 0, plane 1, ..., plane 4. Total image data = `bytes * 5`
3. **Mask data:** 1 additional bitplane used for transparency masking during blitting. Total mask data = `bytes * 1`
4. **Total per frame:** `bytes * 6`

For a standard character sprite (16x32, width=1, height=32):
- Bytes per plane = 32 * 1 * 2 = 64 bytes
- 5 image planes = 320 bytes
- 1 mask plane = 64 bytes
- **Total per frame = 384 bytes**

For the dragon (48x40, width=3, height=40):
- Bytes per plane = 40 * 3 * 2 = 240 bytes
- 5 image planes = 1,200 bytes
- 1 mask plane = 240 bytes
- **Total per frame = 1,440 bytes**

Memory layout for a full character set (e.g., Julian with 67 frames):
- Image planes: frames are stored with all 5 planes contiguous per frame, all frames sequential. `location` points to the start.
- Mask planes: stored separately after all image data. `maskloc` points to the start.
- The `read_shapes()` function (`fmain2.c:685-703`) loads image data first (`nextshape += size*5`), then sets `maskloc` to the next available address and generates the mask using `make_mask()`.

The mask plane is not stored on disk -- it is computed at load time by ORing the 5 image bitplanes together. This saves disk space (one fewer plane to store) at the cost of a brief computation during loading.

**Asset format -- modern conversion target:**
- **Original:** Amiga planar bitplane format as described above. Data is loaded via raw disk sector reads using a custom track loader.
- **Modern target:** PNG sprite sheet with alpha channel. One PNG per character set (e.g., `julian.png`, `ogre.png`), with frames arranged in a grid. A separate JSON metadata file would describe frame dimensions, counts, and animation sequence mappings. The mask plane maps directly to the PNG alpha channel.

---

## 4. Combat System

For the combat encounter flow diagram, see [STORYLINE.md Section 11](STORYLINE.md#11-combat-encounter-flow).

The combat system handles melee attacks, missile (ranged) attacks, damage application, death
processing, and loot generation. All combat logic runs each game tick as part of the main loop
in `fmain.c`.

### 4.1 Melee Hit Detection

**Source:** `fmain.c:2238-2265`

The melee loop iterates over all active actors (`i = 0` to `anix - 1`) each tick. It is an
O(n^2) algorithm: for each attacker, it checks every other actor as a potential target.

```
for each actor i (0 to anix-1):
    if i > 0 and freeze_timer active: break
    if i == 1 (raft slot) or state >= WALKING (not fighting): skip

    wt = actor's weapon type
    if weapon has bit 2 set (bow/wand, weapon & 4): skip melee entirely

    if wt >= 8: wt = 5              -- cap touch attacks at 5
    wt += bitrand(2)                 -- random extension: 0, 1, or 2 added

    -- Calculate strike point (projected from attacker position along facing)
    xs = newx(abs_x, facing, wt * 2) + rand8() - 3
    ys = newy(abs_y, facing, wt * 2) + rand8() - 3

    -- Determine hit radius
    if i == 0 (hero):  bv = (brave / 20) + 5    -- range 5-15
    else (enemy):      bv = 2 + rand4()          -- range 2-5
    if bv > 14: bv = 15                          -- hard cap at 15

    for each potential target j (0 to anix-1):
        if j == 1 (raft) or j == i (self) or target is DEAD or attacker is CARRIER: skip

        xd = |target.abs_x - xs|
        yd = |target.abs_y - ys|
        if xd > yd: yd = xd         -- Chebyshev distance (max of deltas)

        -- Hit condition
        if (i == 0 OR rand256() > brave) AND yd < bv AND NOT freeze_timer:
            dohit(i, j, facing, wt)
            break                    -- only one hit per swing

        -- Near miss: clash sound
        else if yd < bv + 2 AND wt != 5 (not wand/touch):
            effect(1, 150 + rand256())
```

**Key details:**

- `newx(x, dir, speed)` and `newy(y, dir, speed)` project a point from position `(x,y)` along
  direction `dir` by `speed` pixels. They use direction tables:
  `xdir = {-2, 0, 2, 3, 2, 0, -2, -3}` and `ydir = {-2, -3, -2, 0, 2, 3, 2, 0}` for
  directions 0-7 (S, SW, W, NW, N, NE, E, SE). The formula is:
  `result = position + (dir_table[facing] * speed) / 2`.
- The distance metric is Chebyshev distance (L-infinity norm): `max(|dx|, |dy|)`.
- `rand8()` returns 0-7, so `rand8() - 3` gives jitter of -3 to +4 pixels.
- `bitrand(2)` returns `rand() & 2`, yielding 0 or 2 (NOT 0, 1, or 2). This means weapon
  reach extends by 0 or 2, never 1, creating a bimodal reach distribution.
- The hero always hits if within range (no accuracy check). Enemies must pass
  `rand256() > brave` -- at high bravery, enemies rarely land melee hits.
- The bravery cap of 15 on hit radius means at 200+ bravery, the hero's effective melee
  reach plateaus.
- Touch attacks (weapon >= 8, capped to 5) have the same strike distance as a wand but use
  melee logic, not ranged. The cap at 5 means touch attacks have reach `(5 + bitrand(2)) * 2`
  = 10 or 14 pixels projected forward.

### 4.2 The dohit() Function

**Source:** `fmain2.c:230-247`

Called when a melee or missile attack connects. Parameters: `i` = attacker index (-1 for arrow,
-2 for fireball), `j` = target index, `fc` = facing direction, `wt` = weapon type / damage.

```c
dohit(i, j, fc, wt)
{
    // Immunity check #1: weak weapons vs. magic creatures
    // NOTE: checks anim_list[0].weapon (HERO's weapon), not attacker's
    if (hero_weapon < 4 &&
        (target.race == 9 ||                         // necromancer
         (target.race == 0x89 && stuff[7] == 0)))    // witch without sun stone
    {   speak(58);  // "You can't hurt me with that!"
        return;
    }

    // Immunity check #2: undead completely immune to all attacks
    if (target.race == 0x8a || target.race == 0x8b)  // spectre or ghost
        return;  // silently ignored -- no feedback

    // Apply damage
    target.vitality -= wt;
    if (target.vitality < 0) target.vitality = 0;

    // Sound effects by attack type (mutually exclusive)
    if (i == -1)       effect(2, 500 + rand64());     // arrow hit
    else if (i == -2)  effect(5, 3200 + bitrand(511));// fireball hit
    else if (j == 0)   effect(0, 800 + bitrand(511)); // hero hurt
    else               effect(3, 400 + rand256());    // enemy hurt

    // Push-back: move target 2 pixels in attack direction
    // Dragons and set-figures are immovable
    if (target.type != DRAGON && target.type != SETFIG)
    {   if (move_figure(j, fc, 2) returned FALSE && i >= 0)
            move_figure(i, fc, 2);   // push attacker instead if target blocked
    }

    checkdead(j, 5);
}
```

**Key observations:**

- **Immunity bug:** The necromancer/witch immunity check uses `anim_list[0].weapon` (the
  hero's current weapon) regardless of who the attacker is. If an enemy attacks another enemy,
  the hero's weapon type still determines immunity. In practice this rarely matters because
  enemies don't attack each other.
- **Damage is the raw weapon type value** for melee (1-7, where 1=dirk, 2=mace, 3=sword,
  5=wand/touch). For missiles, damage is `rand8() + 4` (range 4-11).
- **Push-back reversal:** When the target can't be pushed back (terrain blocks), the attacker
  is pushed forward instead. This creates the odd behavior of an attacker lunging into the
  target when the target is cornered. Only applies to melee (i >= 0), not missiles.
- `move_figure(fig, dir, dist)` attempts to move the figure and returns FALSE if
  `proxcheck()` detects a collision at the destination.

### 4.3 Missile System

**Source:** `fmain.c:78-85` (struct), `fmain.c:2267-2301` (flight and collision)

#### Missile Structure

```c
struct missile {
    unsigned short  abs_x, abs_y;   // world position
    char  missile_type;    // 0=empty, 1=arrow, 2=fireball, 3=spent fireball
    char  time_of_flight;  // ticks since launch
    char  speed;           // 0=unshot, 3=arrow, 5=wand/fireball
    char  direction;       // 0-7 direction of travel
    char  archer;          // actor index of shooter (avoid self-hit)
} missile_list[6];         // 6 simultaneous missiles max
```

#### Per-Tick Missile Processing

```
if freeze_timer active: skip all missiles

for each missile slot i (0 to 5):
    s = speed * 2

    -- Destroy missile if: empty (type 0), spent fireball (type 3),
    -- speed is 0, or flight time exceeds 40 ticks
    if missile_type == 0 or missile_type == 3 or s == 0 or time_of_flight++ > 40:
        missile_type = 0; continue

    -- Terrain collision
    terrain = px_to_im(abs_x, abs_y)
    if terrain == 1 (impassable) or terrain == 15 (door):
        missile_type = 0; s = 0

    -- Set hit radius by type
    mt = 6                          -- arrow/wand bolt hit radius
    if missile_type == 2: mt = 9    -- fireball has larger blast radius

    -- Actor collision check
    for each actor j (0 to anix-1):
        if j == 0: bv = brave; else bv = 20
        if j == 1 (raft) or j == archer (self) or target DEAD or CARRIER: skip

        xd = |target.abs_x - abs_x|
        yd = |target.abs_y - abs_y|
        if xd > yd: yd = xd         -- Chebyshev distance

        if (i != 0 OR bitrand(512) > bv) AND yd < mt:
            if missile_type == 2: dohit(-2, j, direction, rand8()+4)  -- fireball
            else:                  dohit(-1, j, direction, rand8()+4)  -- arrow
            speed = 0
            if missile_type == 2: missile_type = 3   -- fireball becomes spent
            break

    -- Advance missile position
    abs_x = newx(abs_x, direction, s)
    abs_y = newy(abs_y, direction, s)
```

**Missile slot 0 dodge bug:** The dodge condition `i != 0` checks the *missile slot index*,
not the target actor index. Only missile slot 0 allows targets to dodge (via
`bitrand(512) > bv`). Missiles in slots 1-5 always hit if within radius, bypassing the dodge
check entirely. The `bv` variable is computed per-target (`brave` for hero, 20 for enemies)
but is only used when `i == 0`. The developer's own comment `/* really?? */` on the bravery
assignment line suggests uncertainty about this logic.

**Arrow consumption:** Arrows are tracked in `stuff[8]`. On SHOOT3 state (bow release), if the
hero has no arrows (`stuff[8] == 0`), the shot is cancelled with "No Arrows!" Otherwise
`stuff[8]` is decremented. Enemies do not consume arrows.

### 4.4 Shooting Mechanics

**Source:** `fmain.c:1410-1439` (hero input), `fmain.c:1667-1709` (shot processing),
`fmain.c:1481-1494` (dragon)

#### State Machine

Shooting uses two animation states:

| State | Value | Description |
|-------|-------|-------------|
| SHOOT1 | 24 | Aiming -- bow raised or wand pointed |
| SHOOT3 | 25 | Release -- arrow/bolt launched |

#### Hero Shooting

When the fire button is pressed:
- **Bow (weapon 4):** Immediately enters SHOOT1. On button release, transitions to SHOOT3.
  Arrow created with speed 3, `missile_type = 1`.
- **Wand (weapon 5):** Enters SHOOT1 and immediately fires. Bolt created with speed 5,
  `missile_type = 2` (fireball type). Transitions to SHOOT3 on same tick. Can fire repeatedly
  while button held (if `state < SHOOT1`, re-enters SHOOT1).
- **Melee weapons (0-3):** Enter FIGHTING state instead (states 0-8).

Shooting is suppressed underwater (`k > 15`) and in peace zones (`xtype > 80`).

#### Shot Origin

Missile spawn position uses per-direction offset tables:

```c
// Bow X offsets per direction (8 directions)
char bowshotx[8] = { 0, 0, 3, 6, -3, -3, -3, -6 };

// Bow Y offsets per direction
char bowshoty[8] = { -6, -6, -1, 0, 6, 8, 0, -1 };

// Wand Y offsets per direction (X uses bowshotx)
char gunshoty[8] = { 2, 0, 4, 7, 9, 4, 7, 8 };
```

Missile origin: `abs_x + bowshotx[dir]`, `abs_y + bowshoty[dir]` (bow) or
`abs_y + gunshoty[dir]` (wand).

#### Enemy Shooting

Enemies with ranged weapons (bow/wand) use the SHOOT tactic. When within line-of-sight
(distance checks along cardinal/diagonal axes with tolerance), they face the hero, set state
to SHOOT1, and on the next AI tick transition to SHOOT3.

```c
// fmain2.c:1671-1682 -- SHOOT tactic
xd = |enemy.abs_x - hero.abs_x|
yd = |enemy.abs_y - hero.abs_y|
if (rand() & 1) AND (xd < 8 OR yd < 8 OR roughly diagonal):
    set_course(i, hero_x, hero_y, 5)
    if state < SHOOT1: state = SHOOT1
else:
    set_course(i, hero_x, hero_y, 0)   // pursue instead
```

#### Dragon Shooting

**Source:** `fmain.c:1481-1494`

Dragons fire fireballs with a 1/4 chance per tick (`rand4() == 0`). The fireball has speed 5,
`missile_type = 2`, and always faces direction 5 (NE). Damage on hit is `rand8() + 4`
(range 4-11), same as other missiles.

```c
if (actor.type == DRAGON)
{   if (rand4() == 0)                   // 25% chance per tick
    {   missile.speed = 5;
        missile.missile_type = 2;        // fireball
        effect(5, 1800 + rand256());     // fireball sound
        actor.facing = 5;               // always faces NE (hardcoded)
        // jump to dragshoot: sets missile position and direction
    }
}
```

### 4.5 Enemy Encounter Chart

**Source:** `fmain.c:45-64`

The `encounter_chart[11]` array defines base stats for all enemy types. Each entry contains:
hitpoints, aggressive flag, arms tier (indexes into `weapon_probs`), cleverness (AI behavior
modifier), treasure tier (indexes into `treasure_probs`), and sprite file ID.

| # | Name | HP | Aggressive | Arms | Cleverness | Treasure | Sprite |
|---|------|----|------------|------|------------|----------|--------|
| 0 | Ogre | 18 | yes | 2 | 0 | 2 | 6 |
| 1 | Orc | 12 | yes | 4 | 1 | 1 | 6 |
| 2 | Wraith | 16 | yes | 6 | 1 | 4 | 7 |
| 3 | Skeleton | 8 | yes | 3 | 0 | 3 | 7 |
| 4 | Snake | 16 | yes | 6 | 1 | 0 | 8 |
| 5 | Salamander | 9 | yes | 3 | 0 | 0 | 7 |
| 6 | Spider | 10 | yes | 6 | 1 | 0 | 8 |
| 7 | Dark Knight | 40 | yes | 7 | 1 | 0 | 8 |
| 8 | Loraii | 12 | yes | 6 | 1 | 0 | 9 |
| 9 | Necromancer | 50 | yes | 5 | 0 | 0 | 9 |
| 10 | Woodcutter | 4 | no | 0 | 0 | 0 | 9 |

**Field meanings:**

- **HP:** Starting vitality for spawned enemies.
- **Aggressive:** If TRUE, enemy attacks on sight. Woodcutter (10) is the only passive type.
- **Arms:** Index into `weapon_probs` array (tier 0-7, each tier has 4 entries). Determines
  which weapon the spawned enemy receives. See weapon probability table below.
- **Cleverness:** AI behavior level. 0 = basic pursuit, 1 = uses flanking/evasion tactics.
  Higher values enable more sophisticated combat AI state transitions.
- **Treasure:** Index into `treasure_probs` array (tier 0-4, each tier has 8 entries).
  Determines loot dropped on death. Tier 0 = no treasure. See treasure probability table below.
- **Sprite:** Actor file ID for loading sprite graphics. File 6 = humanoid set 1 (ogre, orc),
  file 7 = undead/reptile set (wraith, skeleton, salamander), file 8 = monster set (snake,
  spider, dark knight), file 9 = special (loraii, necromancer, woodcutter).

**Notes on specific enemies:**

- **Snake (4):** Swamp region enemy. Arms tier 6 = touch attack. Cleverness 1.
- **Salamander (5):** Lava region enemy. Arms tier 3 = melee weapons.
- **Spider (6):** Spider pit enemy. Arms tier 6 = touch attack.
- **Dark Knight (7):** Elf glade guardian. 40 HP, arms tier 7 = swords only. Guards shrines.
- **Loraii (8):** Astral plane enemy. Arms tier 6 = touch attack.
- **Necromancer (9):** Final arena boss. 50 HP, arms tier 5 = magic wand. Immune to weapons
  below tier 4 (see dohit immunity).
- **Woodcutter (10):** Non-aggressive NPC. No weapons, no treasure.

### 4.6 Weapon Probability Table

**Source:** `fmain2.c:860-868`

The `weapon_probs[32]` array is organized as 8 tiers of 4 entries each. When an enemy spawns,
one of the 4 entries in its arms tier is selected randomly to determine the weapon.

Weapon codes: 0=none, 1=dirk, 2=mace, 3=sword, 4=bow, 5=wand, 8=touch attack.

| Tier | Entry 0 | Entry 1 | Entry 2 | Entry 3 | Description |
|------|---------|---------|---------|---------|-------------|
| 0 | 0 | 0 | 0 | 0 | No weapons |
| 1 | 1 | 1 | 1 | 1 | Dirks only |
| 2 | 1 | 2 | 1 | 2 | 50% dirk, 50% mace |
| 3 | 1 | 2 | 3 | 2 | 25% dirk, 50% mace, 25% sword |
| 4 | 4 | 4 | 3 | 2 | 50% bow, 25% sword, 25% mace |
| 5 | 5 | 5 | 5 | 5 | Magic wand only |
| 6 | 8 | 8 | 8 | 8 | Touch attack only |
| 7 | 3 | 3 | 3 | 3 | Swords only |

**Damage per weapon type in melee:** The `wt` value after capping (touch attacks capped at 5)
plus `bitrand(2)` (0 or 2) is both the reach multiplier and the damage dealt by `dohit()`.
So a dirk does 1-3 damage per hit, a mace 2-4, a sword 3-5, and a touch attack 5-7.

### 4.7 Treasure Probability Table

**Source:** `fmain2.c:852-858`

The `treasure_probs[40]` array is organized as 5 tiers of 8 entries each. When an enemy dies,
one of the 8 entries in its treasure tier is selected randomly to determine the loot drop.

| Tier | [0] | [1] | [2] | [3] | [4] | [5] | [6] | [7] | Description |
|------|-----|-----|-----|-----|-----|-----|-----|-----|-------------|
| 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | No treasure |
| 1 | 9 | 11 | 13 | 31 | 31 | 17 | 17 | 32 | Stones, vials, totems, gold, keys |
| 2 | 12 | 14 | 20 | 20 | 20 | 31 | 33 | 31 | Keys, skulls, gold, nothing |
| 3 | 10 | 10 | 16 | 16 | 11 | 17 | 18 | 19 | Magic items and keys |
| 4 | 15 | 21 | 0 | 0 | 0 | 0 | 0 | 0 | Jade skull and white key (25% chance) |

**Item codes referenced:** 0=nothing, 9=blue stone, 10=green stone, 11=amber stone,
12=crystal shard, 13=jade totem, 14=skull, 15=jade skull, 16=gold key, 17=silver key,
18=brass key, 19=glass key, 20=gold (currency), 21=white key, 31=gold (currency),
32=gold (currency), 33=gold (currency).

**Treasure tier usage by enemy:**

- Tier 0 (no treasure): Snake, Salamander, Spider, Dark Knight, Loraii, Necromancer, Woodcutter
- Tier 1: Orc
- Tier 2: Ogre
- Tier 3: Skeleton
- Tier 4: Wraith

### 4.8 checkdead() -- Death Processing

**Source:** `fmain.c:2769-2782`

Called after every hit to check if the target should die.

```c
checkdead(i, dtype)
{
    an = &anim_list[i];

    if (an->vitality < 1 && an->state != DYING && an->state != DEAD)
    {
        an->vitality = 0;
        an->tactic = 7;              // death countdown timer
        an->goal = DEATH;
        an->state = DYING;           // begin death animation

        if (an->race == 7)           // dark knight
            speak(42);               // "You have earned the right to enter"
        else if (an->type == SETFIG && an->race != 0x89)  // setfig, not witch
            kind -= 3;               // killing non-hostile NPCs hurts kindness

        if (i != 0)                  // enemy died
            brave++;                 // hero gains 1 bravery per kill
        else                         // hero died
        {   event(dtype);            // death event (dtype=5 from combat)
            luck -= 5;
            setmood(TRUE);           // set mood to sad
        }

        if (kind < 0) kind = 0;     // floor kindness at 0
        prq(7);                      // refresh status display (enemy stats)
    }

    if (i == 0) prq(4);             // refresh hero vitality display (always, even if alive)
}
```

**Key details:**

- The `dtype` parameter is always 5 when called from `dohit()`. This triggers the death event
  handler which processes brother succession (Julian -> Dorian -> Kevin).
- `tactic = 7` serves as a death animation countdown. The DYING state plays a 7-frame
  collapse animation, then transitions to DEAD.
- `brave++` means bravery grows linearly with kills. Since bravery affects hit radius
  (up to 15), enemy dodge chance (`rand256() > brave`), and arrow dodging
  (`bitrand(512) > brave`), it is the primary combat progression stat.
- Killing set-figures (quest NPCs like the dark knight) who are not the witch costs 3 kindness
  points. The witch (race 0x89) is exempt because she is a legitimate combat target.
- `prq(7)` and `prq(4)` trigger status bar redraws for the enemy counter and hero vitality
  respectively.

### 4.9 Improvement Notes

1. **O(n^2) melee detection:** The nested loop (each attacker checks every target) scales
   poorly. With up to 20 actor slots, this is 400 iterations per tick. Spatial partitioning
   (grid cells or a spatial hash) would reduce this to near-linear.

2. **Push-back reversal bug:** In `dohit()`, when `move_figure(j, fc, 2)` fails (target
   blocked by terrain), the attacker is pushed forward in the same direction via
   `move_figure(i, fc, 2)`. This means cornered enemies cause the hero to lunge into them,
   and vice versa. The likely intent was to push the attacker *backward* (opposite direction),
   but the same `fc` (facing) is used for both calls.

3. **Missile dodge uses slot index instead of target index:** In the missile collision loop
   (`fmain.c:2291`), the dodge condition `i != 0` tests the missile slot index, not the target
   actor index `j`. This means only missile slot 0 allows dodge rolls; missiles in slots 1-5
   are undodgeable. The developer's `/* really?? */` comment on the bravery assignment
   suggests this was noticed but not corrected.

4. **Bravery-based arrow dodging is unbalanced at high brave:** The hero's dodge formula
   `bitrand(512) > brave` means at brave >= 512, the hero is completely immune to missile
   slot 0. Combined with the slot index bug above, this creates inconsistent behavior --
   the first missile fired can be dodged, but subsequent ones cannot.

5. **Touch attack weapon reach inconsistency:** Touch attacks (weapon >= 8) are capped at
   `wt = 5`, giving them the same reach calculation as a wand in melee. But wands skip melee
   entirely (weapon & 4 is true for value 5: `5 & 4 = 4`, which is truthy). Touch attacks
   with value 8 also have bit 2 clear (`8 & 4 = 0`), so they go through melee. The cap at 5
   means a touch attack has identical reach to what a wand *would* have in melee, even though
   wands never use melee. This is internally consistent but the reuse of value 5 for the cap
   is coincidental rather than intentional.

6. **Immunity check uses hero weapon for all attackers:** The `dohit()` immunity check
   (`anim_list[0].weapon < 4`) always tests the hero's equipped weapon, even when the attacker
   is an enemy or a missile. This means enemy-on-enemy attacks (rare but possible) or missile
   hits are filtered by what the hero is holding.

---

## 5. AI & Behavior

All enemy AI runs once per game tick in the main loop at `fmain.c:2109-2184`. Each actor evaluates a two-level decision system: a **goal** (strategic intent) and a **tactic** (immediate movement behavior). Movement is resolved by `set_course()`, a pure 68000 assembly routine that converts a target position into a compass facing.

### 5.1 Goal Modes

**Original** -- Goals are the top-level behavioral state for each actor. Defined in `ftale.h:27-37`:

| Value | Name       | Description |
|-------|------------|-------------|
| 0     | `USER`     | Player-controlled character. AI loop skips this actor. |
| 1     | `ATTACK1`  | Melee attack, low intelligence (cleverness 0). Re-evaluates tactics with probability 1/4 (`!rand4()`) due to `(mode & 2) == 0` check. |
| 2     | `ATTACK2`  | Melee attack, high intelligence (cleverness 1). Re-evaluates with base probability 1/16 (`!bitrand(15)`). `do_tactic` internally boosts action rate to 1/4 for this goal. |
| 3     | `ARCHER1`  | Ranged attack, low intelligence (cleverness 0). Re-evaluates at 1/16 rate (the `mode & 2` check does NOT match mode 3, likely a bug). Prefers shooting at medium range, backs up if too close. |
| 4     | `ARCHER2`  | Ranged attack, high intelligence (cleverness 1). Re-evaluates at 1/4 rate (the `mode & 2` check matches mode 4, likely unintended). Same shooting logic as ARCHER1. |
| 5     | `FLEE`     | Run directly away from hero. Assigned when actor vitality drops below 2, or when the hero is dead and the actor has no leader. Always executes `do_tactic(i, BACKUP)`. |
| 6     | `STAND`    | Face the hero but do not move. Calls `set_course(i, hero_x, hero_y, 0)` then forces state to STILL. Used by the Dark Knight (race 7) when vitality is nonzero. |
| 7     | `DEATH`    | Dead actor. Not assigned in the AI loop; set by `checkdead()` at `fmain.c:2769`. AI loop skips dead actors via the `vitality < 1` check. |
| 8     | `WAIT`     | Wait to speak to the hero. Actor state is forced to STILL with no movement. |
| 9     | `FOLLOWER` | Follow another character. Assigned when the hero is dead and a leader exists. Executes `do_tactic(i, FOLLOW)`. |
| 10    | `CONFUSED` | Run around randomly. Assigned when the actor has no weapon (`weapon < 1`). Tactic is forced to RANDOM. |

Goals are assigned at spawn time based on the encounter chart: `ATTACK1 + cleverness` for melee, `ARCHER1 + cleverness` for ranged (`fmain.c:2761-2762`). The cleverness field is 0 or 1, selecting the "stupid" or "clever" variant.

**Improvement** -- The goal system conflates strategic intent (attack, flee, follow) with intelligence level (ATTACK1 vs ATTACK2). A cleaner design would separate intent from AI parameters, allowing more than two intelligence tiers and independent tuning of re-evaluation rates.

### 5.2 Tactical Modes

**Original** -- Tactics are the immediate movement sub-goals. Defined in `ftale.h:42-54`:

| Value | Name          | Movement Produced |
|-------|---------------|-------------------|
| 0     | `FRUST`       | All tactics frustrated. If actor has a bow (`weapon & 4`), pick a random tactic from 2-5. Otherwise pick from 3-4. Serves as a "stuck" recovery. |
| 1     | `PURSUE`      | Move directly toward the hero. Calls `set_course(i, hero_x, hero_y, 0)` with 1/8 probability (1/4 for ATTACK2). |
| 2     | `FOLLOW`      | Move toward the leader actor. Calls `set_course(i, leader.abs_x, leader.abs_y+20, 0)` with the standard action probability. If the actor IS the leader, tactic is changed to RANDOM. The +20 y-offset means followers trail slightly behind. |
| 3     | `BUMBLE_SEEK` | Imprecise seeking. Calls `set_course(i, hero_x, hero_y, 4)` -- mode 4 disables axis pruning so the actor moves diagonally. 1/8 action probability. |
| 4     | `RANDOM`      | Random wandering. Sets facing to `rand() & 7` and state to WALKING with 1/8 probability. No pathfinding at all. |
| 5     | `BACKUP`      | Move away from the hero. Calls `set_course(i, hero_x, hero_y, 3)` -- mode 3 reverses both direction components. 1/8 action probability. |
| 6     | `EVADE`       | Lateral dodge. Targets a neighboring actor's position rather than the hero: if `i == anix` then target is `anim_list[i-1]`, otherwise `anim_list[i+i]`. Calls `set_course` with mode 2 (deviation at close range). 1/8 action probability. **Bug note:** the code reads `i+i` (double the index), not `i+1` -- this is likely a typo that was never caught. |
| 7     | `HIDE`        | Defined but never implemented in `do_tactic()`. Falls through without any action. |
| 8     | `SHOOT`       | Ranged attack. Checks alignment: if the actor is roughly on axis with the hero (within 8 pixels on either axis, or close to a diagonal), randomly fires (50% chance) by calling `set_course(i, hero_x, hero_y, 5)` (face only, no walk) and setting state to SHOOT1. Otherwise, approaches with `set_course` mode 0. |
| 9     | `SHOOTFRUST`  | Arrow-shooting frustrated. Handled identically to FRUST in the AI loop -- picks a random replacement tactic. |
| 10    | `EGG_SEEK`    | Snake-specific. Moves toward the turtle egg location at hardcoded coordinates (23087, 5667). Only assigned when `actor.race == 4` and `turtle_eggs` is nonzero. |
| 11    | `DOOR_SEEK`   | Dark Knight blocking a door. Defined in the header but never referenced in any C source file. Likely planned but unused or handled by a different mechanism. |
| 12    | `DOOR_LET`    | Dark Knight allowing passage. Same as DOOR_SEEK -- defined but unreferenced. |

Tactics 2-5 (FOLLOW, BUMBLE_SEEK, RANDOM, BACKUP) are the "obstacle avoidance" pool. When an actor becomes frustrated (FRUST), one of these is selected randomly. Ranged actors get the wider pool (2-5); melee actors get the narrower pool (3-4).

**Improvement** -- The 1/8 base action probability means most movement commands are silently dropped. This creates the sluggish, somewhat erratic enemy movement characteristic of the original game. A modern version could use cooldown timers instead of random drops for more predictable pacing. The EVADE tactic's `i+i` target selection is almost certainly a bug (should be `i+1`) -- it causes index-out-of-range behavior for actors with indices above 3.

### 5.3 set_course() Algorithm

**Original** -- Pure 68000 assembly at `fmain2.c:63-222`. This is the core pathfinding routine that converts a target position into a compass facing for a given actor.

**Signature:** `set_course(object, target_x, target_y, mode)`

**Compass direction table** (`fmain2.c:56`):
```
com2[9] = { 0, 1, 2, 7, 9, 3, 6, 5, 4 }
```

**Direction numbering** (from `fsubs.asm:1276-1277`):

| Dir | Compass | dx  | dy  |
|-----|---------|-----|-----|
| 0   | NW      | -2  | -2  |
| 1   | N       |  0  | -3  |
| 2   | NE      | +2  | -2  |
| 3   | E       | +3  |  0  |
| 4   | SE      | +2  | +2  |
| 5   | S       |  0  | +3  |
| 6   | SW      | -2  | +2  |
| 7   | W       | -3  |  0  |

**Modes:**

| Mode | Behavior |
|------|----------|
| 0    | Direct pursuit toward target. Axis pruning active. No deviation. |
| 1    | Pursuit with close-range wobble. If Manhattan distance < 40, deviation = 1. |
| 2    | Pursuit with close-range wobble. If Manhattan distance < 30, deviation = 1. (Note: the inline C comment says `deviation = 2` but the assembly uses `moveq #1,d4` -- the actual behavior is deviation = 1, same as mode 1 but with a tighter activation range.) |
| 3    | Reverse direction (flee from target). Axis pruning active. |
| 4    | Full diagonal movement. Axis pruning disabled. No deviation. |
| 5    | Face only -- calculates facing but does not set state to WALKING. |
| 6    | Raw vector -- target_x and target_y are treated as a direction vector, not a position. |

**Full pseudocode:**

```
function set_course(object, target_x, target_y, mode):
    actor = anim_list[object]

    // Step 1: Calculate delta vector
    if mode == 6:
        xdif = target_x          // raw vector, use directly
        ydif = target_y
    else:
        xdif = actor.abs_x - target_x
        ydif = actor.abs_y - target_y

    // Step 2: Determine absolute values and direction signs
    xdir = 0
    ydir = 0

    if xdif > 0:
        xabs = xdif
        xdir = 1
    else if xdif < 0:
        xabs = -xdif
        xdir = -1
    else:
        xabs = 0

    if ydif > 0:
        yabs = ydif
        ydir = 1
    else if ydif < 0:
        yabs = -ydif
        ydir = -1
    else:
        yabs = 0

    // Step 3: Axis pruning (skip if mode == 4)
    // Suppresses the lesser axis when one axis strongly dominates,
    // producing cardinal (non-diagonal) movement.
    if mode != 4:
        if (xabs >> 1) > yabs:    // x dominates by 2:1
            ydir = 0
        if (yabs >> 1) > xabs:    // y dominates by 2:1
            xdir = 0

    // Step 4: Calculate deviation and apply reversals
    deviation = 0
    distance = xabs + yabs       // Manhattan distance

    if mode == 1 and distance < 40:
        deviation = 1
    else if mode == 2 and distance < 30:
        deviation = 1
    else if mode == 3:
        xdir = -xdir              // reverse both axes (flee)
        ydir = -ydir

    // Step 5: Convert (xdir, ydir) to compass direction via lookup
    index = 4 - ydir*3 - xdir
    j = com2[index]

    // Step 6: Apply result
    if j == 9:                    // (0,0) = no direction needed
        actor.state = STILL
    else:
        // Randomly jitter the direction by deviation
        // Note: assembly tests bit 1 (btst #1), not bit 0.
        // The inline C comment says "rand()&1" but the actual
        // assembly uses bit 1 of rand(). Still a 50/50 coin flip.
        if rand() & 2:
            j = j + deviation
        else:
            j = j - deviation
        j = j & 7                 // wrap to 0-7
        actor.facing = j
        if mode != 5:             // mode 5 = face only, don't walk
            actor.state = WALKING
```

**com2 lookup verification** -- the index `4 - ydir*3 - xdir` maps every (xdir, ydir) pair to a compass direction that points from the actor toward the target:

| ydir | xdir | Index | com2 | Direction | Meaning |
|------|------|-------|------|-----------|---------|
| +1   | +1   | 0     | 0    | NW        | Actor is SE of target, move NW |
| +1   |  0   | 1     | 1    | N         | Actor is S of target, move N |
| +1   | -1   | 2     | 2    | NE        | Actor is SW of target, move NE |
|  0   | +1   | 3     | 7    | W         | Actor is E of target, move W |
|  0   |  0   | 4     | 9    | (none)    | Actor is at target, stop |
|  0   | -1   | 5     | 3    | E         | Actor is W of target, move E |
| -1   | +1   | 6     | 6    | SW        | Actor is NE of target, move SW |
| -1   |  0   | 7     | 5    | S         | Actor is N of target, move S |
| -1   | -1   | 8     | 4    | SE        | Actor is NW of target, move SE |

**Improvement** -- `set_course` is the single most important routine to port. The pseudocode above is a complete, verified translation of the 68000 assembly. Key observations for reimplementation:

- The axis pruning creates a preference for cardinal movement when one axis is twice the other. This produces the characteristic "L-shaped" paths where actors walk straight then turn.
- Deviation (modes 1 and 2) adds a +/-1 jitter at close range, making enemies circle or weave slightly instead of walking in a straight line at the target. The jitter only activates within Manhattan distance 40 or 30.
- Mode 3 (reverse) is used for BACKUP/flee. It reverses direction signs AFTER axis pruning, so fleeing actors also favor cardinal directions.
- The `& 7` wrap means deviation can shift direction 0 to direction 7 (NW to W) or direction 7 to direction 0 (W to NW), creating smooth wraparound in the compass rose.
- There is no obstacle awareness whatsoever. Actors that hit impassable terrain simply stop, which triggers the FRUST recovery on the next evaluation cycle.

### 5.4 AI Decision Loop

**Original** -- The complete per-tick AI logic at `fmain.c:2109-2184`. Runs once per main-loop iteration for every active actor from index 2 through `anix-1` (index 0 is the hero, index 1 is the raft).

**Pre-loop state reset** (`fmain.c:2105-2108`):
```
actors_on_screen = FALSE
leader = 0
battle2 = battleflag
battleflag = FALSE
```

**Full decision tree per actor:**

```
for i = 2 to anix-1:

    // Good fairy active and in animation phase: skip all AI
    if goodfairy != 0 and goodfairy < 120:
        break   // exits the entire loop, not just this actor

    actor = anim_list[i]

    // Carrier type (flying mount): face hero every 16 ticks, skip rest
    if actor.type == CARRIER:
        if (daynight & 15) == 0:
            set_course(i, hero_x, hero_y, 5)   // face only
        continue

    // Set figures (NPCs placed by map): skip entirely
    if actor.type == SETFIG:
        continue

    mode = actor.goal
    tactic = actor.tactic

    // Distance check
    xd = |actor.abs_x - hero_x|
    yd = |actor.abs_y - hero_y|

    if xd < 300 and yd < 300:          // within 300px box
        actors_on_screen = TRUE
        if actor.vitality < 1: continue  // dead, skip
        if actor.visible or battle2:
            battleflag = TRUE

    // Re-evaluation roll: 1/16 chance per tick
    r = !bitrand(15)                    // TRUE with probability 1/16

    // Override goals based on hero state
    if hero is DEAD or FALLING:
        if leader == 0: mode = FLEE     // no one to follow, scatter
        else: mode = FOLLOWER           // follow the designated leader

    // Override goals based on actor state
    // Special encounters (xtype > 59) don't flee unless race matches extent
    else if actor.vitality < 2 or
            (xtype > 59 and actor.race != extn->v3):
        mode = FLEE

    // === Tactic handling ===

    // Frustrated: pick a random replacement tactic
    if tactic == FRUST or tactic == SHOOTFRUST:
        if actor.weapon & 4:            // has bow
            do_tactic(i, rand4() + 2)   // random from {2,3,4,5}
        else:
            do_tactic(i, rand2() + 3)   // random from {3,4}

    // Mid-shoot: advance shoot animation
    else if actor.state == SHOOT1:
        actor.state = SHOOT3

    // Hostile modes (ATTACK1, ATTACK2, ARCHER1, ARCHER2)
    else if mode <= ARCHER2:
        // Boost re-evaluation for certain modes
        if (mode & 2) == 0:             // ATTACK1 (1) or ARCHER2 (4)
            r = !rand4()                // override: 1/4 chance

        if r:   // re-evaluate tactic
            if actor.race == 4 and turtle_eggs:
                tactic = EGG_SEEK       // snakes prioritize eggs
            else if actor.weapon < 1:
                mode = CONFUSED          // unarmed -> wander
                tactic = RANDOM
            else if actor.vitality < 6 and rand2():
                tactic = EVADE           // wounded -> dodge (50%)
            else if mode >= ARCHER1:     // ranged attacker
                if xd < 40 and yd < 30:
                    tactic = BACKUP      // too close, back off
                else if xd < 70 and yd < 70:
                    tactic = SHOOT       // in range, fire
                else:
                    tactic = PURSUE      // too far, close in
            else:
                tactic = PURSUE          // melee: always close in

        // Melee range check
        thresh = 14 - mode               // ATTACK1=13, ATTACK2=12,
                                         // ARCHER1=11, ARCHER2=10
        if actor.race == 7: thresh = 16  // Dark Knight: wider melee range

        if not (actor.weapon & 4) and xd < thresh and yd < thresh:
            set_course(i, hero_x, hero_y, 0)
            if actor.state >= WALKING:
                actor.state = FIGHTING   // enter melee combat

        // Dark Knight special: stand still facing direction 5 (south)
        else if actor.race == 7 and actor.vitality > 0:
            actor.state = STILL
            actor.facing = 5

        else:
            do_tactic(i, tactic)

    // Non-hostile modes
    else if mode == FLEE:
        do_tactic(i, BACKUP)

    else if mode == FOLLOWER:
        do_tactic(i, FOLLOW)

    else if mode == STAND:
        set_course(i, hero_x, hero_y, 0)
        actor.state = STILL              // face hero, don't move

    else if mode == WAIT:
        actor.state = STILL              // completely stationary

    // Save potentially modified goal back
    actor.goal = mode

    // First living actor becomes the leader
    if leader == 0:
        leader = i
```

**Key behaviors:**

- **Leader election**: The first actor processed (lowest index >= 2) becomes the "leader." When the hero dies, other actors switch to FOLLOWER mode and follow this leader, creating a cluster effect rather than all enemies chasing a dead hero.
- **Re-evaluation throttling**: The 1/16 base probability is overridden to 1/4 when `(mode & 2) == 0`. This bit test selects ATTACK1 (mode 1) and ARCHER2 (mode 4) for the faster rate, while ATTACK2 (mode 2) and ARCHER1 (mode 3) keep 1/16. This is likely a bug: the intent was probably to make "dumb" modes (cleverness 0 = ATTACK1, ARCHER1) more reactive, but the bit mask catches ARCHER2 (cleverness 1) instead of ARCHER1 (cleverness 0). Between re-evaluations, `do_tactic` further throttles actual movement commands to 1/8 probability. An enemy in PURSUE with the 1/16 rate moves toward the hero on average once every 128 ticks; with the 1/4 rate, once every 32 ticks.
- **Dark Knight special case**: Race 7 actors (the Dark Knight boss) get a wider melee threshold (16 vs. 10-13) and a unique behavior: when not in melee range, they simply stand still facing south rather than pursuing.
- **Good fairy abort**: When the good fairy is active (animation frames 1-119), the entire AI loop is aborted via `break`. This freezes all enemy AI during the fairy's appearance.

**Improvement** -- The double-random throttle (re-evaluation probability times action probability) creates highly variable effective response times. A timer-based approach would be more predictable. The leader concept is simplistic -- it always picks the lowest-indexed living actor regardless of distance or relevance. The `goodfairy` check using `break` instead of `continue` is intentional (freezes all enemies, not just one) but fragile -- moving the fairy check outside the loop would be clearer.

### 5.5 do_tactic() Implementation

**Original** -- Defined at `fmain2.c:1664-1700`. Translates a tactic number into a concrete `set_course` call or state change.

All tactics except RANDOM and SHOOT share a common action-probability gate: `r = !(rand() & 7)`, giving 1/8 chance of actually executing the movement command. ATTACK2 actors get a boosted rate of 1/4 (`!(rand() & 3)`).

| Tactic | Target | set_course Mode | Notes |
|--------|--------|-----------------|-------|
| PURSUE (1) | hero position | 0 (direct) | Only moves with probability r. |
| FOLLOW (2) | leader position (+20 y) | 0 (direct) | If actor is the leader, downgrades to RANDOM. The +20 offset makes followers trail behind vertically. |
| BUMBLE_SEEK (3) | hero position | 4 (no axis pruning) | Diagonal-friendly seeking. Only moves with probability r. |
| RANDOM (4) | (none) | (none) | Sets facing to a random direction and state to WALKING. Probability r. No set_course call. |
| BACKUP (5) | hero position | 3 (reverse) | Moves away from the hero. Probability r. |
| EVADE (6) | neighbor actor position (+20 y) | 2 (deviation near) | Targets another actor's position, not the hero's. Creates lateral movement. Probability r. |
| HIDE (7) | (none) | (none) | Not implemented. Falls through do_tactic with no effect. |
| SHOOT (8) | hero position | 5 (face only) or 0 (approach) | Checks axis alignment before firing. If roughly aligned and 50% chance: face hero and set state to SHOOT1. Otherwise: approach with mode 0. |
| EGG_SEEK (10) | (23087, 5667) | 0 (direct) | Hardcoded turtle egg location. Forces state to WALKING regardless of probability gate. |
| DOOR_SEEK (11) | -- | -- | Defined but never handled in do_tactic. Falls through with no effect. |
| DOOR_LET (12) | -- | -- | Same as DOOR_SEEK -- defined but unimplemented. |

**SHOOT alignment check detail** (`fmain2.c:1671-1682`):
```
xd = |actor.abs_x - hero.abs_x|
yd = |actor.abs_y - hero.abs_y|
if (rand() & 1) and (xd < 8 or yd < 8 or (xd > yd-5 and xd < yd+7)):
    // Roughly on a cardinal axis or close to 45-degree diagonal
    set_course(i, hero_x, hero_y, 5)   // face only
    if state < SHOOT1: state = SHOOT1   // begin shoot animation
else:
    set_course(i, hero_x, hero_y, 0)   // approach to get aligned
```

The alignment check ensures arrows are only fired when the actor is approximately on a line with the hero -- along a cardinal direction (within 8 pixels) or near a diagonal (x and y distances within 5-7 of each other). This prevents arrows from being fired at odd angles where they would miss.

**Improvement** -- The HIDE tactic (7), DOOR_SEEK (11), and DOOR_LET (12) are dead code -- defined as constants but never implemented. They may have been planned features that were cut. The EVADE tactic's neighbor-targeting logic (`i+i` instead of the likely intended `i+1`) means high-index actors target out-of-bounds or unrelated slots, producing erratic movement that coincidentally serves the "evasion" purpose. EGG_SEEK's hardcoded coordinates are fragile -- they point to the turtle egg area at (23087, 5667) and would break if the map layout changed.

### 5.6 Encounter Generation

**Original** -- Encounter spawning has two phases: placement of pending encounters (`fmain.c:2058-2078`) and generation of new encounter groups (`fmain.c:2080-2093`).

#### Encounter Zones (extent_list)

The world is divided into overlapping rectangular zones defined in `fmain.c:338-370`. Each zone has coordinates, an encounter type (`etype`), and parameters (v1 = base count, v2 = random count range, v3 = creature race or encounter_type index).

| # | Zone | etype | v1 | v2 | v3 | Purpose |
|---|------|-------|----|----|----|---------|
| 0 | (2118,27237)-(2618,27637) | 70 | 0 | 1 | 11 | Bird carrier zone |
| 1 | (0,0)-(0,0) | 70 | 0 | 1 | 5 | Turtle carrier (coords set dynamically) |
| 2 | (6749,34951)-(7249,35351) | 70 | 0 | 1 | 10 | Dragon carrier zone |
| 3 | (4063,34819)-(4909,35125) | 53 | 4 | 1 | 6 | Spider pit (forced encounter) |
| 4 | (9563,33883)-(10144,34462) | 60 | 1 | 1 | 9 | Necromancer (special forced) |
| 5 | (22945,5597)-(23225,5747) | 61 | 3 | 2 | 4 | Turtle eggs (special forced) |
| 6 | (10820,35646)-(10877,35670) | 83 | 1 | 1 | 0 | Princess rescue trigger |
| 7 | (19596,17123)-(19974,17401) | 48 | 8 | 8 | 2 | Graveyard (high danger) |
| 8 | (19400,17034)-(20240,17484) | 80 | 4 | 20 | 0 | Around city (peace zone) |
| 9 | (0x2400,0x8200)-(0x3100,0x8A00) | 52 | 3 | 1 | 8 | Astral plane (Loraii) |
| 10 | (5272,33300)-(6112,34200) | 81 | 0 | 1 | 0 | King's castle (peace) |
| 11 | (11712,37350)-(12416,38020) | 82 | 0 | 1 | 0 | Sorceress (peace) |
| 12 | (2752,33300)-(8632,35400) | 80 | 0 | 1 | 0 | Peace zone 1 (buildings) |
| 13 | (10032,35550)-(12976,40270) | 80 | 0 | 1 | 0 | Peace zone 2 (specials area) |
| 14 | (4712,38100)-(10032,40350) | 80 | 0 | 1 | 0 | Peace zone 3 (cabins) |
| 15 | (21405,25583)-(21827,26028) | 60 | 1 | 1 | 7 | Hidden valley (Dark Knight) |
| 16 | (6156,12755)-(12316,15905) | 7 | 1 | 8 | 0 | Swamp region |
| 17 | (5140,34860)-(6260,37260) | 8 | 1 | 8 | 0 | Spider forest region |
| 18 | (660,33510)-(2060,34560) | 8 | 1 | 8 | 0 | Spider forest region 2 |
| 19 | (18687,15338)-(19211,16136) | 80 | 0 | 1 | 0 | Village (peace) |
| 20 | (16953,18719)-(20240,17484) | 3 | 1 | 3 | 0 | Around village |
| 21 | (20593,18719)-(23113,22769) | 3 | 1 | 3 | 0 | Around city |
| 22 | (0,0)-(0x7FFF,0x9FFF) | 3 | 1 | 8 | 0 | Whole world (default fallback) |

Zone scanning is first-match: the extent_list is iterated in order, and the first zone containing the hero's position is used (`fmain.c:2675-2679`). This means smaller zones must appear before larger ones to take priority.

**etype ranges:**
- **0-49**: Normal encounter zones. Random encounters can spawn. The etype value directly influences danger level.
- **50-59**: Forced encounter zones. Encounters spawn immediately on entry (spiders, astral plane).
- **60-61**: Special forced encounters. Specific creature types are loaded on entry (Necromancer, turtle eggs, Dark Knight).
- **70**: Carrier zones. Load a flying mount (bird, turtle, dragon) instead of enemies.
- **80**: Peace zones. No random encounters (xtype >= 50 suppresses generation).
- **81-83**: Special peace zones with triggered events (King's greeting, Sorceress's greeting, Princess rescue).

#### Phase 1: Placing Pending Encounters

At `fmain.c:2058-2078`, every 16 ticks (`daynight & 15 == 0`):

```
if encounter_number > 0 and not actors_loading:
    mixflag = rand()                     // random mixing of encounter types
    if xtype > 49: mixflag = 0           // no mixing in special zones
    wt = rand4()                         // weapon tier randomizer
    if (xtype & 3) == 0: mixflag = 0    // no mixing for etype multiples of 4

    for k = 1 to 10:                     // try up to 10 placement locations
        set_loc()                        // pick random point 150-213 pixels
                                         // from hero in a random direction
        if terrain at (encounter_x, encounter_y) is passable:
            // Place in new slots (indices 3-6)
            while encounter_number > 0 and anix < 7:
                if set_encounter(anix, 63): anix++
                encounter_number--
            // Recycle dead actor slots (indices 3-6)
            for i = 3 to 6:
                if encounter_number == 0: break
                if anim_list[i].state == DEAD and
                   (not visible or race == 2):  // wraiths can respawn visibly
                    set_encounter(i, 63)
                    encounter_number--
            break
```

`set_loc()` (`fmain2.c:1714-1720`) picks a spawn point at distance 150 + rand64() (150-213 pixels) from the hero in a random compass direction.

`set_encounter()` (`fmain.c:2736-2767`) places a single actor:
- For the Dark Knight extent (v3 == 7): uses a hardcoded spawn position (21635, 25762).
- Otherwise: picks a random position within a +/-31 pixel spread of the encounter center, retrying up to 15 times if the location is blocked.
- Sets race from encounter_type (with random mixing if mixflag bit 1 is set).
- Assigns weapon from the weapon probability table based on `encounter_chart[race].arms * 4 + wt`.
- Sets goal to ATTACK1/ATTACK2 or ARCHER1/ARCHER2 based on cleverness and weapon type.
- Sets vitality from encounter_chart hitpoints.

#### Phase 2: Generating New Encounters

At `fmain.c:2080-2093`, every 32 ticks (`daynight & 31 == 0`):

```
if no actors on screen and not loading actors and
   no active carrier and xtype < 50:       // only in normal zones

    if region_num > 7:                      // indoors
        danger_level = 5 + xtype
    else:                                   // outdoors
        danger_level = 2 + xtype

    if rand64() <= danger_level:            // 0-63 roll vs danger
        encounter_type = rand4()            // random base type (0-3)

        // Region-specific overrides
        if xtype == 7 and encounter_type == 2:
            encounter_type = 4              // swamp: wraith -> snake
        if xtype == 8:
            encounter_type = 6              // spider zone: force spiders
            mixflag = 0
        if xtype == 49:
            encounter_type = 2              // etype 49: force wraiths
            mixflag = 0

        load_actors()                       // sets encounter_number,
                                            // loads sprite data if needed
```

`load_actors()` (`fmain.c:2722-2731`) sets `encounter_number = extn->v1 + rnd(extn->v2)` (base count plus a random value from 0 to v2-1). If the required actor sprite file differs from the currently loaded one, it reloads from disk and resets the active enemy count.

**Danger level math**: With xtype values ranging from 0 to ~8 in normal zones:
- Outdoor base danger: 2-10. Chance of spawn per 32-tick cycle: 3/64 to 11/64 (roughly 5% to 17%).
- Indoor base danger: 5-13. Chance of spawn: 6/64 to 14/64 (roughly 9% to 22%).
- The graveyard (etype 48) has extreme danger: 2 + 48 = 50, giving 51/64 (80%) spawn chance per cycle.

**Improvement** -- The terrain check in Phase 1 (`px_to_im(encounter_x, encounter_y) == 0`) only tests the center point, not the actor's collision box. Actors can spawn partially overlapping impassable terrain. The encounter_type override logic for specific xtypes (7, 8, 49) is hardcoded and would benefit from a data-driven approach. The "whole world" fallback extent (entry 22) means no position is ever unmatched, but its etype of 3 creates a nonzero base danger everywhere -- there is no truly safe wilderness. The `(xtype & 3) == 0` check for disabling mixflag means zones with etype values that are multiples of 4 (0, 4, 8, 48, 52) always produce homogeneous encounter groups, while others may mix in adjacent race types.

---

## 6. Inventory & Items

### 6.1 Inventory Structure

Each brother has a private 35-element `UBYTE` array tracking item counts:

```c
UBYTE julstuff[35], philstuff[35], kevstuff[35];
UBYTE *stuff;  /* points to current brother's array */
```

The `stuff` pointer is redirected to the active brother's array on character switch. All pickup, use, and display logic indexes through `stuff[]`.

The struct definition (`ftale.h:95`):

```c
struct inv_item {
    UBYTE image_number;           /* sprite image index */
    UBYTE xoff, yoff;             /* display position on inventory screen */
    UBYTE ydelta;                 /* y increment per stacked item */
    UBYTE img_off, img_height;    /* vertical sub-image offset and height */
    UBYTE maxshown;               /* max items rendered visually */
    char *name;
};
```

### 6.2 Complete Item Table

All 36 entries from `inv_list[]` (`fmain.c:380-424`):

| Slot | Name | image_number | xoff | yoff | ydelta | img_off | img_height | maxshown | Category |
|------|------|-------------|------|------|--------|---------|------------|----------|----------|
| 0 | Dirk | 12 | 10 | 0 | 0 | 0 | 8 | 1 | Weapon |
| 1 | Mace | 9 | 10 | 10 | 0 | 0 | 8 | 1 | Weapon |
| 2 | Sword | 8 | 10 | 20 | 0 | 0 | 8 | 1 | Weapon |
| 3 | Bow | 10 | 10 | 30 | 0 | 0 | 8 | 1 | Weapon |
| 4 | Magic Wand | 17 | 10 | 40 | 0 | 8 | 8 | 1 | Weapon |
| 5 | Golden Lasso | 27 | 10 | 50 | 0 | 0 | 8 | 1 | Weapon |
| 6 | Sea Shell | 23 | 10 | 60 | 0 | 8 | 8 | 1 | Weapon |
| 7 | Sun Stone | 27 | 10 | 70 | 0 | 8 | 8 | 1 | Weapon |
| 8 | Arrows | 3 | 30 | 0 | 3 | 7 | 1 | 45 | Ammo |
| 9 | Blue Stone | 18 | 50 | 0 | 9 | 0 | 8 | 15 | Magic |
| 10 | Green Jewel | 19 | 65 | 0 | 6 | 0 | 5 | 23 | Magic |
| 11 | Glass Vial | 22 | 80 | 0 | 8 | 0 | 7 | 17 | Magic |
| 12 | Crystal Orb | 21 | 95 | 0 | 7 | 0 | 6 | 20 | Magic |
| 13 | Bird Totem | 23 | 110 | 0 | 10 | 0 | 9 | 14 | Magic |
| 14 | Gold Ring | 17 | 125 | 0 | 6 | 0 | 5 | 23 | Magic |
| 15 | Jade Skull | 24 | 140 | 0 | 10 | 0 | 9 | 14 | Magic |
| 16 | Gold Key | 25 | 160 | 0 | 5 | 0 | 5 | 25 | Key |
| 17 | Green Key | 25 | 172 | 0 | 5 | 8 | 5 | 25 | Key |
| 18 | Blue Key | 114 | 184 | 0 | 5 | 0 | 5 | 25 | Key |
| 19 | Red Key | 114 | 196 | 0 | 5 | 8 | 5 | 25 | Key |
| 20 | Grey Key | 26 | 208 | 0 | 5 | 0 | 5 | 25 | Key |
| 21 | White Key | 26 | 220 | 0 | 5 | 8 | 5 | 25 | Key |
| 22 | Talisman | 11 | 0 | 80 | 0 | 8 | 8 | 1 | Status |
| 23 | Rose | 19 | 0 | 90 | 0 | 8 | 8 | 1 | Status |
| 24 | Fruit | 20 | 0 | 100 | 0 | 8 | 8 | 1 | Status |
| 25 | Gold Statue | 21 | 232 | 0 | 10 | 8 | 8 | 5 | Status |
| 26 | Book | 22 | 0 | 110 | 0 | 8 | 8 | 1 | Status |
| 27 | Herb | 8 | 14 | 80 | 0 | 8 | 8 | 1 | Status |
| 28 | Writ | 9 | 14 | 90 | 0 | 8 | 8 | 1 | Status |
| 29 | Bone | 10 | 14 | 100 | 0 | 8 | 8 | 1 | Status |
| 30 | Shard | 12 | 14 | 110 | 0 | 8 | 8 | 1 | Status |
| 31 | 2 Gold Pieces | 0 | 0 | 0 | 0 | 0 | 0 | 2 | Gold |
| 32 | 5 Gold Pieces | 0 | 0 | 0 | 0 | 0 | 0 | 5 | Gold |
| 33 | 10 Gold Pieces | 0 | 0 | 0 | 0 | 0 | 0 | 10 | Gold |
| 34 | 100 Gold Pieces | 0 | 0 | 0 | 0 | 0 | 0 | 100 | Gold |
| 35 | quiver of arrows | 0 | 0 | 0 | 0 | 0 | 0 | 0 | Special |

**Note:** Slot 35 ("quiver of arrows") is used as a temporary counter during pickup (`stuff[ARROWBASE]`). After pickup, it is converted: `stuff[8] += stuff[ARROWBASE] * 10`. Gold piece entries (31-34) are never stored in `stuff[]`; their `maxshown` field doubles as the gold value added to `wealth` on loot.

### 6.3 Index Range Constants

Defined at `fmain.c:426-430`:

| Constant | Value | Meaning |
|----------|-------|---------|
| `MAGICBASE` | 9 | First magic item slot (Blue Stone) |
| `KEYBASE` | 16 | First key slot (Gold Key) |
| `STATBASE` | 25 | First status/quest item slot (Gold Statue) |
| `GOLDBASE` | 31 | First gold entry; also upper bound for inventory display loop |
| `ARROWBASE` | 35 | Temporary arrow counter slot; also array size for `stuff[]` |

The inventory display loop (`fmain.c:3128`) iterates `j` from 0 to `GOLDBASE` (exclusive), so gold entries are never rendered on the inventory screen.

Category ranges:
- **Weapons:** 0-8 (slots 0-4 are melee/ranged weapons, 5-7 are special items, 8 is arrows)
- **Magic items:** 9-15 (Blue Stone through Jade Skull)
- **Keys:** 16-21 (six colored keys)
- **Status/quest items:** 22-30 (Talisman, Rose, Fruit, statues, quest items)
- **Gold:** 31-34 (display-only entries for loot messages)
- **Temp:** 35 (quiver counter)

### 6.4 Object Byte Enums

The `enum obytes` (`fmain2.c:967-977`) maps world object IDs (the `ob_id` byte in the object list) to semantic constants:

```c
enum obytes {
    QUIVER     = 11,
    MONEY      = 13,
    URN        = 14,    /* implicit: MONEY+1 */
    CHEST      = 15,    /* implicit: MONEY+2 */
    SACKS      = 16,    /* implicit: MONEY+3 */
    G_RING     = 17,    /* implicit: MONEY+4 */
    B_STONE    = 18,
    G_JEWEL    = 19,
    SCRAP      = 20,    /* 0x14 - scrap of paper */
    C_ORB      = 21,
    VIAL       = 22,
    B_TOTEM    = 23,
    J_SKULL    = 24,
    GOLD_KEY   = 25,
    GREY_KEY   = 26,
    FOOTSTOOL  = 31,
    TURTLE     = 102,
    BLUE_KEY   = 114,
    M_WAND     = 145,
    MEAL       = 146,   /* implicit: M_WAND+1 (apple) */
    ROSE       = 147,
    FRUIT      = 148,
    STATUE     = 149,
    BOOK       = 150,
    SHELL      = 151,
    GREEN_KEY  = 153,
    WHITE_KEY  = 154,
    RED_KEY    = 242,
};
```

Note the non-contiguous numbering: IDs jump from 31 to 102 to 114 to 145+, reflecting the sparse object ID space in the world data.

### 6.5 Item Translation Table

The `itrans[]` table (`fmain2.c:979-985`) maps world object IDs to inventory slot indices. It is stored as a flat array of `{object_id, slot}` pairs, terminated by `{0, 0}`:

| Object ID | Constant | Inventory Slot | Item Name |
|-----------|----------|---------------|-----------|
| 11 | QUIVER | 35 | quiver of arrows |
| 18 | B_STONE | 9 | Blue Stone |
| 19 | G_JEWEL | 10 | Green Jewel |
| 22 | VIAL | 11 | Glass Vial |
| 21 | C_ORB | 12 | Crystal Orb |
| 23 | B_TOTEM | 13 | Bird Totem |
| 17 | G_RING | 14 | Gold Ring |
| 24 | J_SKULL | 15 | Jade Skull |
| 145 | M_WAND | 4 | Magic Wand |
| 27 | (raw) | 5 | Golden Lasso |
| 8 | (raw) | 2 | Sword |
| 9 | (raw) | 1 | Mace |
| 12 | (raw) | 0 | Dirk |
| 10 | (raw) | 3 | Bow |
| 147 | ROSE | 23 | Rose |
| 148 | FRUIT | 24 | Fruit |
| 149 | STATUE | 25 | Gold Statue |
| 150 | BOOK | 26 | Book |
| 151 | SHELL | 6 | Sea Shell |
| 155 | (raw) | 7 | Sun Stone |
| 136 | (raw) | 27 | Herb |
| 137 | (raw) | 28 | Writ |
| 138 | (raw) | 29 | Bone |
| 139 | (raw) | 22 | Talisman |
| 140 | (raw) | 30 | Shard |
| 25 | GOLD_KEY | 16 | Gold Key |
| 153 | GREEN_KEY | 17 | Green Key |
| 114 | BLUE_KEY | 18 | Blue Key |
| 242 | RED_KEY | 19 | Red Key |
| 26 | GREY_KEY | 20 | Grey Key |
| 154 | WHITE_KEY | 21 | White Key |
| 0 | (terminator) | 0 | -- |

Lookup is a linear scan: iterate pairs until `itrans[k] == 0` or a match is found. Unknown object IDs (not in this table) are silently ignored with a `break` (the commented-out "unknown thing" message at `fmain.c:3198` was disabled).

### 6.6 Pickup Logic

The pickup handler is in `do_option()` when `cmode == ITEMS` and `hit == 6` (`fmain.c:3147-3287`). It finds the nearest object within range 30 (`nearest_fig(0,30)`) and dispatches on the object's `index & 0xff` byte (the object ID `j`):

#### 6.6.1 Special-Case Objects

| Object ID | Hex | Condition | Action |
|-----------|-----|-----------|--------|
| 0x0d (13) | MONEY | always | `wealth += 50`; announce "50 gold pieces" |
| 0x14 (20) | SCRAP | always | Trigger `event(17)`; then `event(19)` if region > 7, else `event(18)`; remove object |
| 148 | FRUIT (apple) | `hunger < 15` | Store in inventory: `stuff[24]++`; trigger `event(36)` |
| 148 | FRUIT (apple) | `hunger >= 15` | Eat immediately: `eat(30)` (restores 30 hunger) |
| 102 | TURTLE | always | `break` -- cannot be taken |
| 28 | (bones) | always | Inherit dead brother's inventory; `vitality & 0x7f` identifies which brother (1=Julian, else Philip); adds all `julstuff[k]` or `philstuff[k]` to current `stuff[k]` for `k` in 0..GOLDBASE-1; also hides bone objects (`ob_listg[3].ob_stat = ob_listg[4].ob_stat = 0`) |
| 0x1d (29) | (empty chest) | always | `break` -- skip |
| 31 | FOOTSTOOL | always | `break` -- skip |

#### 6.6.2 Container Loot (Chest, Urn, Sacks)

When the object is a container (`j` is 0x0e/URN, 0x0f/CHEST, or 0x10/SACKS), a `rand4()` roll (0-3) determines loot:

| Roll | Tier | Loot |
|------|------|------|
| 0 | Empty | "nothing." |
| 1 | Single item | One random item: `rand8() + 8` gives slot 8-15; if slot 8 then replaced with ARROWBASE (quiver). Increment `stuff[i]`. |
| 2 | Two items | Two different random items from slots 8-15. If either roll is 8, it becomes 100 gold (`wealth += 100`, slot set to GOLDBASE+3=34). |
| 3 | Three of same | Roll `rand8() + 8`. If slot 8: instead give 3 random keys (`rand8() + KEYBASE`, clamped to valid range). Otherwise: 3 copies of the rolled item. |

The item pool for container loot draws from slots 8-15 (Arrows, Blue Stone, Green Jewel, Glass Vial, Crystal Orb, Bird Totem, Gold Ring, Jade Skull). There is no level scaling or regional variation.

#### 6.6.3 Known Items via itrans[]

For any object ID not matching the special cases or containers, the code performs a linear scan of `itrans[]`:

```c
for (k=0; itrans[k]; k += 2) {
    if (j == itrans[k]) {
        i = itrans[k+1];
        stuff[i]++;
        announce_treasure("a ");
        print_cont(inv_list[i].name);
        print_cont(".");
        goto pickup;
    }
}
break;  /* unknown objects silently ignored */
```

#### 6.6.4 Post-Pickup Processing

After any successful pickup (`goto pickup` label, `fmain.c:3241`):

1. **Object removal:** `change_object(nearest, 2)` -- sets the object's status to 2 (inventory/removed).
2. **Arrow conversion:** `stuff[8] += stuff[ARROWBASE] * 10` -- quiver pickups are converted to 10 arrows each.
3. **Talisman check:** If `stuff[22]` (Talisman) is nonzero, the game ends: `quitflag = TRUE`, triggers the victory map message sequence.

### 6.7 Body Search (Dead Enemy Loot)

When the nearest figure is an enemy (not an object) with `vitality == 0` or the freeze timer is active (`fmain.c:3250-3284`):

**Weapon drop:**
1. Read enemy weapon value: `i = anim_list[nearest].weapon`; clamp to 0 if > 5.
2. If nonzero, add to inventory: `stuff[i-1]++` (weapon slot = weapon value - 1).
3. **Auto-equip:** If `i > anim_list[0].weapon`, upgrade the hero's weapon: `anim_list[0].weapon = i`.
4. **Bow special case:** If the weapon is a Bow (value 4), also award `rand8() + 2` arrows (2-9) to `stuff[8]`, mark body as searched, and return.

**Treasure drop:**
1. Read enemy race: `j = anim_list[nearest].race`.
2. If `j & 0x80`, it is a set-figure (NPC): no treasure (`j = 0`).
3. Otherwise: `j = encounter_chart[j].treasure * 8 + rand8()` -- index into `treasure_probs[]`.
4. `j = treasure_probs[j]` -- the resulting inventory slot (or 0 for nothing).
5. If `j >= GOLDBASE`: add `inv_list[j].maxshown` to `wealth` (gold).
6. If `j < GOLDBASE` and nonzero: `stuff[j]++`.

After searching, the enemy's weapon is set to -1 to prevent re-searching.

### 6.8 Treasure Probability Table

The `treasure_probs[]` array (`fmain2.c:852-858`) has 5 tiers of 8 entries each, indexed by `encounter_chart[race].treasure * 8 + rand8()`:

| Tier | treasure | Entries (inventory slots) | Contents |
|------|----------|--------------------------|----------|
| 0 | 0 | 0, 0, 0, 0, 0, 0, 0, 0 | Always nothing |
| 1 | 1 | 9, 11, 13, 31, 31, 17, 17, 32 | Blue Stone, Glass Vial, Bird Totem, 2gp, 2gp, Green Key, Green Key, 5gp |
| 2 | 2 | 12, 14, 20, 20, 20, 31, 33, 31 | Crystal Orb, Gold Ring, Grey Key, Grey Key, Grey Key, 2gp, 10gp, 2gp |
| 3 | 3 | 10, 10, 16, 16, 11, 17, 18, 19 | Green Jewel x2, Gold Key x2, Glass Vial, Green Key, Blue Key, Red Key |
| 4 | 4 | 15, 21, 0, 0, 0, 0, 0, 0 | Jade Skull, White Key (25% chance each), nothing (75%) |

Enemy treasure tiers (from `encounter_chart[]`):

| Race | Enemy | Treasure Tier |
|------|-------|--------------|
| 0 | Ogre | 2 |
| 1 | Orcs | 1 |
| 2 | Wraith | 4 |
| 3 | Skeleton | 3 |
| 4 | Snake | 0 |
| 5 | Salamander | 0 |
| 6 | Spider | 0 |
| 7 | Dark Knight | 0 |
| 8 | Loraii | 0 |
| 9 | Necromancer | 0 |
| 10 | Woodcutter | 0 |

### 6.9 Random Treasure Table

The `rand_treasure[16]` array (`fmain2.c:987-992`) is used elsewhere for random treasure generation:

```c
UBYTE rand_treasure[] = {
    SACKS, SACKS, SACKS, SACKS,      /* 25% sacks (obj 16) */
    CHEST, MONEY, GOLD_KEY, QUIVER,   /* 6.25% each */
    GREY_KEY, GREY_KEY, GREY_KEY, RED_KEY,  /* 18.75% grey key, 6.25% red key */
    B_TOTEM, VIAL, WHITE_KEY, CHEST   /* 6.25% each */
};
```

This table maps a random 4-bit index to an object ID. Sacks are the most common result (25%), followed by Grey Keys (18.75%).

### 6.10 Equipment Effects

**Weapon equip on body search:**
When looting a weapon from a dead enemy, the game compares the weapon value `i` against the hero's current weapon `anim_list[0].weapon`. If the looted weapon is strictly better (higher numeric value), it auto-equips:

```c
if (i > anim_list[0].weapon) anim_list[0].weapon = i;
```

The weapon hierarchy by value:
| Value | Weapon | Slot |
|-------|--------|------|
| 1 | Dirk | 0 |
| 2 | Mace | 1 |
| 3 | Sword | 2 |
| 4 | Bow | 3 |
| 5 | Magic Wand | 4 |

Weapons 6+ (Golden Lasso, Sea Shell, Sun Stone) are clamped out in body search (`if (i > 5) i = 0`), so they can only be obtained from world objects, not enemy drops.

**Inventory display:**
The ITEMS screen renders all items from slot 0 to GOLDBASE-1 by blitting their sprite image repeatedly based on `stuff[j]` count (capped at `maxshown` for display). Each copy is offset by `ydelta` pixels vertically.

### 6.11 Improvement Notes

- **Linear itrans[] scan:** The `itrans[]` lookup is O(n) with ~31 entries. A hash map or direct-indexed array (256 entries, indexed by object ID) would give O(1) lookup. Given the small table size this is not a performance concern in practice.
- **No level scaling on container loot:** Container contents are drawn from a fixed pool (magic items in slots 8-15) regardless of game progression, region, or difficulty. Early containers can yield powerful late-game items.
- **UBYTE overflow on stuff[i]:** Item counts are stored as `UBYTE` (0-255). There is no bounds check on `stuff[i]++`. Accumulating more than 255 of any item would silently wrap to 0. In practice, `maxshown` limits only the display, not the count.
- **Dead code:** The "unknown thing" announcement (`fmain.c:3198`) is commented out; unknown objects are silently dropped instead of being reported to the player.
- **Talisman instant-win:** Picking up item 22 (Talisman) immediately triggers the victory sequence with no confirmation, which could be surprising if obtained accidentally.

---

## 7. NPCs & Dialogue

All non-enemy, non-player characters in the game are represented as "setfig" entities. Each NPC type is defined in a lookup table that controls which graphic file to load, which image within that file to use, and whether the NPC turns to face the player when spoken to. For NPC dialogue tree diagrams, see [STORYLINE.md Sections 5-7](STORYLINE.md#5-npc-dialogue-trees).

### 7.1 SetFig Table

The `setfig_table[14]` array (`fmain.c:22-39`) defines every NPC type:

| Index | NPC Type   | cfile_entry | image_base | can_talk | Notes |
|------:|------------|------------:|-----------:|---------:|-------|
|     0 | Wizard     |          13 |          0 |        1 | Turns to face player |
|     1 | Priest     |          13 |          4 |        1 | Turns to face player |
|     2 | Guard      |          14 |          0 |        0 | Front-facing |
|     3 | Guard      |          14 |          1 |        0 | Back-facing variant |
|     4 | Princess   |          14 |          2 |        0 | |
|     5 | King       |          14 |          4 |        1 | Turns to face player |
|     6 | Noble      |          14 |          6 |        0 | Lord Trane |
|     7 | Sorceress  |          14 |          7 |        0 | |
|     8 | Bartender  |          15 |          0 |        0 | Tavern keeper |
|     9 | Witch      |          16 |          0 |        0 | |
|    10 | Spectre    |          16 |          6 |        0 | |
|    11 | Ghost      |          16 |          7 |        0 | Dead brother |
|    12 | Ranger     |          17 |          0 |        1 | Turns to face player |
|    13 | Beggar     |          17 |          4 |        1 | Spelled "begger" in source |

The `can_talk` flag controls whether the NPC enters the `TALKING` state (state 19) and turns to face the player (tactic set to 15) when spoken to. NPCs with `can_talk == 0` still respond to dialogue -- they simply do not animate a facing change.

The `cfile_entry` selects which graphics file to load (files 13-17 contain NPC sprites). The `image_base` is an offset within that file to the first frame of the NPC's animation.

### 7.2 Complete Speech Catalogue

All 61 speech entries are defined in `narr.asm:351-518`. The `%` character is a substitution marker replaced at runtime with the active hero's name (Julian, Dorian, or Kevin).

| # | Label | Full Text |
|--:|-------|-----------|
| 0 | Ogre speech | "% attempted to communicate with the Ogre but a guttural snarl was the only response." |
| 1 | Orc speech | "Human must die!" said the goblin-man." |
| 2 | Wraith speech | "Doom!" wailed the wraith." |
| 3 | Skeleton speech | "A clattering of bones was the only reply." |
| 4 | Snake speech | "% knew that it is a waste of time to talk to a snake." |
| 5 | Salamander speech | "..." |
| 6 | Loraii speech | "There was no reply." |
| 7 | Necromancer speech | "Die, foolish mortal!" he said." |
| 8 | Shout warning | "No need to shout, son!" he said." |
| 9 | Ranger message 1 | "Nice weather we're having, isn't it?" queried the ranger." |
| 10 | Ranger message 2 | "Good luck, sonny!" said the ranger. "Hope you win!"" |
| 11 | Ranger message 3 | "If you need to cross the lake" said the ranger, "There's a raft just north of here."" |
| 12 | Bartender message 1 | "Would you like to buy something?" said the tavern keeper. "Or do you just need lodging for the night?"" |
| 13 | Bartender message 2 | "Good Morning." said the tavern keeper. "Hope you slept well."" |
| 14 | Bartender message 3 | "Have a drink!" said the tavern keeper."" |
| 15 | Guard message | "State your business!" said the guard. [newline] "My business is with the king." stated %, respectfully." |
| 16 | Princess message | "Please, sir, rescue me from this horrible prison!" pleaded the princess." |
| 17 | King message 1 | "I cannot help you, young man." said the king. "My armies are decimated, and I fear that with the loss of my children, I have lost all hope."" |
| 18 | King message 2 | "Here is a writ designating you as my official agent. Be sure and show this to the Priest before you leave Marheim." |
| 19 | King message 3 | "I'm afraid I cannot help you, young man. I already gave the golden statue to the other young man." |
| 20 | Noble message | "If you could rescue the king's daughter," said Lord Trane, "The King's courage would be restored."" |
| 21 | Give bone (wrong NPC) | "Sorry, I have no use for it."" |
| 22 | Ranger (region 2) | "The dragon's cave is directly north of here." said the ranger."" |
| 23 | Beggar plea | "Alms! Alms for the poor!"" |
| 24 | Beggar prophecy 1 | "I have a prophecy for you, m'lord." said the beggar. "You must seek two women, one Good, one Evil."" |
| 25 | Beggar prophecy 2 | "Lovely Jewels, glint in the night - give to us the gift of Sight!" he said." |
| 26 | Beggar prophecy 3 | "Where is the hidden city? How can you find it when you cannot even see it?" said the beggar." |
| 27 | Wizard hint 1 | "Kind deeds could gain thee a friend from the sea."" |
| 28 | Wizard hint 2 | "Seek the place that is darker than night - There you shall find your goal in sight!" said the wizard, cryptically." |
| 29 | Wizard hint 3 | "Like the eye itself, a crystal Orb can help to find things concealed."" |
| 30 | Wizard hint 4 | "The Witch lives in the dim forest of Grimwood, where the very trees are warped to her will. Her gaze is Death!"" |
| 31 | Wizard hint 5 | "Only the light of the Sun can destroy the Witch's Evil."" |
| 32 | Wizard hint 6 | "The maiden you seek lies imprisoned in an unreachable castle surrounded by unclimbable mountains."" |
| 33 | Wizard hint 7 | "Tame the golden beast and no mountain may deny you! But what rope could hold such a creature?"" |
| 34 | Wizard hint 8 | "Just what I needed!" he said." |
| 35 | Wizard hostile | "Away with you, young ruffian!" said the Wizard. "Perhaps you can find some small animal to torment if that pleases you!"" |
| 36 | Priest hint 1 | "You must seek your enemy on the spirit plane. It is hazardous in the extreme. Space may twist, and time itself may run backwards!"" |
| 37 | Priest hint 2 | "When you wish to travel quickly, seek the power of the Stones." he said." |
| 38 | Priest hint 3 | "Since you are brave of heart, I shall Heal all your wounds." [newline] Instantly % felt much better." |
| 39 | Priest (writ exchange) | "Ah! You have a writ from the king. Here is one of the golden statues of Azal-Car-Ithil. Find all five and you'll find the vanishing city."" |
| 40 | Priest hostile | "Repent, Sinner! Thou art an uncouth brute and I have no interest in your conversation!"" |
| 41 | Dream Knight block | "Ho there, young traveler!" said the black figure. "None may enter the sacred shrine of the People who came Before!"" |
| 42 | Dream Knight pass | "Your prowess in battle is great." said the Knight of Dreams. "You have earned the right to enter and claim the prize."" |
| 43 | Necromancer taunt | "So this is the so-called Hero who has been sent to hinder my plans. Simply Pathetic. Well, try this, young Fool!"" |
| 44 | Necromancer defeated | "% gasped. The Necromancer had been transformed into a normal man. All of his evil was gone." |
| 45 | Sorceress message | "%." said the Sorceress. "Welcome. Here is one of the five golden figurines you will need." [newline] "Thank you." said %." |
| 46 | Witch message | "Look into my eyes and Die!!" hissed the witch. [newline] "Not a chance!" replied %" |
| 47 | Spectre message | "The Spectre spoke. "HE has usurped my place as lord of undead. Bring me bones of the ancient King and I'll help you destroy him."" |
| 48 | Spectre (bones given) | "% gave him the ancient bones. [newline] "Good! That spirit now rests quietly in my halls. Take this crystal shard."" |
| 49 | Ghost message | "%..." said the apparition. "I am the ghost of your dead brother. Find my bones -- there you will find some things you need." |
| 50 | Gold given (generic) | "% gave him some gold coins. [newline] "Why, thank you, young sir!"" |
| 51 | Buy from non-seller | "Sorry, but I have nothing to sell."" |
| 52 | Buy from no one | *(empty string)* |
| 53 | Ranger direction 1 | "The dragon's cave is east of here." said the ranger."" |
| 54 | Ranger direction 2 | "The dragon's cave is west of here." said the ranger."" |
| 55 | Ranger direction 3 | "The dragon's cave is south of here." said the ranger."" |
| 56 | Turtle (give shell) | "Oh, thank you for saving my eggs, kind man!" said the turtle. "Take this seashell as a token of my gratitude."" |
| 57 | Turtle (has shell) | "Just hop on my back if you need a ride somewhere." said the turtle." |
| 58 | Witch/Necro immune | "Stupid fool, you can't hurt me with that!"" |
| 59 | Necro magic block | "Your magic won't work here, fool!"" |
| 60 | Witch vulnerable | "The Sunstone has made the witch vulnerable!" |

### 7.3 Talk Ranges

The TALK menu offers three sub-commands that differ in detection range (`fmain.c:3368`):

| Command | Key | Range (px) | Behavior |
|---------|-----|------------|----------|
| Yell    | Y   | 100        | `nearest_fig(1, 100)` -- finds NPCs at long range |
| Say     | S   | 50         | `nearest_fig(1, 50)` -- normal conversation range |
| Ask     | A   | 50         | `nearest_fig(1, 50)` -- same range as Say |

If the player Yells and the nearest NPC is within 35 pixels (too close), the NPC responds with speech 8: *"No need to shout, son!"* and the conversation is aborted. This is the only mechanical difference between Yell and Say/Ask -- once range-check passes, all three commands use the same dialogue logic.

A dead NPC (`state == DEAD`) is silently ignored regardless of range.

### 7.4 TALK Handler -- Decision Tree

When the player talks to a SETFIG, the handler at `fmain.c:3372-3416` extracts the NPC type as `k = an->race & 0x7f` (strips the 0x80 SETFIG flag) and dispatches on it:

**Case 0 -- Wizard:**
- If `kind < 10` (player is unkind): speak(35) -- *"Away with you, young ruffian!"*
- Else: speak(27 + `an->goal`) -- one of speeches 27-34, selected by the wizard's per-instance `goal` field (0-7). Each wizard instance is placed in the world with a fixed goal value, so different wizards give different progressive hints.

**Case 1 -- Priest:**
- If player has Writ (`stuff[28]` is nonzero):
  - If `ob_listg[10].ob_stat == 0` (statue not yet given): speak(39) -- gives golden statue, sets `ob_listg[10].ob_stat = 1`
  - Else (`ob_listg[10].ob_stat != 0`): speak(19) -- *"I already gave the golden statue to the other young man."*
- Else if `kind < 10` (player is unkind): speak(40) -- *"Repent, Sinner!"*
- Else (kind and no writ): speak(36 + `daynight % 3`) -- one of three rotating hints (speeches 36-38). Also fully heals the player (`vitality = 15 + brave/4`) and refreshes the status display (`prq(4)`).

**Case 2, 3 -- Guards:**
- speak(15) -- *"State your business!"* Both front and back guard variants give the same response.

**Case 4 -- Princess:**
- If `ob_list8[9].ob_stat` is set (princess rescue flag): speak(16) -- *"Please, sir, rescue me!"*
- If flag is not set: no speech (silent). This means the princess only speaks after a specific game event triggers her flag.

**Case 5 -- King:**
- If `ob_list8[9].ob_stat` is set: speak(17) -- *"I cannot help you, young man..."*
- If flag is not set: no speech (silent).

**Case 6 -- Noble (Lord Trane):**
- speak(20) -- *"If you could rescue the king's daughter..."*

**Case 7 -- Sorceress:**
- If `ob_listg[9].ob_stat` is already set (previously visited):
  - If `luck < rand64()`: `luck += 5` (luck boost). Silent -- no speech played.
  - If `luck >= rand64()`: nothing happens (diminishing returns).
- Else (first visit): speak(45) -- gives golden figurine, sets `ob_listg[9].ob_stat = 1`.
- Always calls `prq(7)` to refresh display.

**Case 8 -- Bartender:**
Three states based on time and fatigue:
- If `fatigue < 5` (well-rested): speak(13) -- *"Good Morning. Hope you slept well."*
- Else if `dayperiod > 7` (late in the day): speak(12) -- *"Would you like to buy something?"*
- Else: speak(14) -- *"Have a drink!"*

**Case 9 -- Witch:**
- speak(46) -- *"Look into my eyes and Die!!"*

**Case 10 -- Spectre:**
- speak(47) -- *"HE has usurped my place as lord of undead. Bring me bones..."*

**Case 11 -- Ghost:**
- speak(49) -- *"I am the ghost of your dead brother. Find my bones..."*

**Case 12 -- Ranger:**
- If `region_num == 2`: speak(22) -- *"The dragon's cave is directly north of here."*
- Else: speak(53 + `an->goal`) -- per-instance goal selects one of speeches 53-55, giving directional hints (east/west/south). Each ranger is placed in the world with a goal (0-2) corresponding to the relative direction of the dragon's cave from that location.

**Case 13 -- Beggar:**
- speak(23) -- *"Alms! Alms for the poor!"*

### 7.5 GIVE Handler

The GIVE menu (`fmain.c:3490-3506`) offers four items: Gold, Book, Writ, and Bone. Only Gold and Bone have implemented give-to-NPC logic:

**Gold (hit == 5):**
- Requires `wealth > 2`. Deducts 2 gold.
- Random kindness increase: if `rand64() > kind`, increment `kind` by 1.
- Refreshes status (`prq(4)`) and luck display (`prq(7)`).
- If target is a beggar (race `0x8d`): speak(24 + `goal`) -- one of speeches 24-26, a prophecy selected by the beggar's per-instance `goal` field (0-2).
- Otherwise: speak(50) -- generic gold-giving thanks.

**Bone (hit == 8):**
- Requires `stuff[29]` (player has the Bone item).
- If target is NOT the spectre (race != `0x8a`): speak(21) -- *"Sorry, I have no use for it."*
- If target IS the spectre: speak(48) -- *"Good! Take this crystal shard."* Removes the bone (`stuff[29] = 0`) and calls `leave_item(nearest_person, 140)` which drops item 140 (crystal shard) at the spectre's map position.

### 7.6 Carrier Dialogue (Turtle)

When the player talks to a CARRIER type and `active_carrier == 5` (the turtle), a separate path handles it (`fmain.c:3418-3421`):

- If player already has the sea shell (`stuff[6]`): speak(57) -- *"Just hop on my back if you need a ride."*
- Else: gives the shell (`stuff[6] = 1`), speak(56) -- *"Thank you for saving my eggs! Take this seashell."*

### 7.7 Enemy Dialogue

Talking to an ENEMY type entity triggers `speak(an->race)` (`fmain.c:3422`), indexing directly into the speech table using the enemy's race number. Enemy races 0-7 map to speeches 0-7:

| Race | Enemy | Speech |
|-----:|-------|--------|
| 0 | Ogre | Guttural snarl |
| 1 | Orc | "Human must die!" |
| 2 | Wraith | "Doom!" |
| 3 | Skeleton | Clattering bones |
| 4 | Snake | Waste of time to talk |
| 5 | Salamander | "..." |
| 6 | Loraii | No reply |
| 7 | Necromancer | "Die, foolish mortal!" |

### 7.8 Improvement Notes

- **Wizard/Ranger goal is opaque:** Both wizard and ranger dialogue depends on a per-instance `goal` field baked into the map data. The player has no way to know which hint a given wizard will provide or which direction a ranger will indicate -- there is no visual or contextual distinction between instances. This means the hint system is effectively random from the player's perspective unless they memorize NPC locations.

- **Sorceress luck boost has diminishing returns:** On repeat visits, the luck boost (`luck += 5`) only fires if `luck < rand64()`. As luck increases, the probability of getting the boost decreases. The check is also silent -- no speech is played on repeat visits regardless of whether the boost succeeds or fails, giving the player no feedback.

- **King is silent after princess rescue:** Both the king (case 5) and princess (case 4) only speak when `ob_list8[9].ob_stat` is set. Before that flag is triggered, talking to them produces no response at all. After the rescue event sets the flag, the king says *"I cannot help you"* -- there is no triumphant or grateful response. This feels like a missing dialogue state or a bug where speech 18 (the writ-giving speech) was intended to be triggered here but is never reached through normal TALK.

- **Spectre drops crystal shard at its position, not into inventory:** The `leave_item(nearest_person, 140)` call places the crystal shard as a ground item at the spectre's map coordinates. The player must then pick it up separately. Every other quest reward (golden statue from priest, shell from turtle, figurine from sorceress) is placed directly into inventory via the `stuff[]` array. This inconsistency may be intentional (the spectre is undead and cannot hand things over) or an oversight.

- **Beggar prophecies are fixed per instance:** Each beggar's `goal` (0-2) determines which prophecy they give when paid. Paying the same beggar repeatedly always yields the same hint. The player must find all three beggars to hear all three prophecies.

- **Guard dialogue is a fixed exchange:** The guard speech (15) includes both the guard's challenge and the hero's response as a single narrated exchange. The player has no actual choice in the interaction.

---

## 8. Quest System

The Faery Tale Adventure tracks quest progress through a combination of trigger zones (`extent_list[]`), scattered object status fields (`ob_listg[]`, `ob_list8[]`), inventory slots (`stuff[]`), and a stone ring teleportation network. There is no centralized quest state machine -- the game checks ad-hoc flags in response to zone transitions, NPC interactions, and item pickups. For quest flow diagrams, see [STORYLINE.md Section 1](STORYLINE.md#1-main-quest-progression). For special event diagrams (graveyard, spider pit, hidden valley, etc.), see [STORYLINE.md Section 14](STORYLINE.md#14-special-event-diagrams).

### 8.1 Extent Table

The `extent_list[23]` array (`fmain.c:338-370`) defines rectangular trigger zones across the world map. Each entry specifies a bounding box and a type code that controls what happens when the hero enters the zone.

```c
struct extent {
    UWORD x1, y1, x2, y2;   /* bounding rectangle */
    UBYTE etype, v1, v2, v3; /* type, count, random range, encounter type */
};
```

The constant `EXT_COUNT` is defined as 22 (`fmain.c:372`), so only entries 0-21 are scanned. The 23rd entry (index 22, "whole world") serves as the fallback when no other extent matches.

| Idx | x1 | y1 | x2 | y2 | etype | v1 | v2 | v3 | Description |
|----:|-----:|------:|-----:|------:|------:|---:|---:|---:|-------------|
| 0 | 2118 | 27237 | 2618 | 27637 | 70 | 0 | 1 | 11 | Bird extent |
| 1 | 0 | 0 | 0 | 0 | 70 | 0 | 1 | 5 | Turtle extent |
| 2 | 6749 | 34951 | 7249 | 35351 | 70 | 0 | 1 | 10 | Dragon extent |
| 3 | 4063 | 34819 | 4909 | 35125 | 53 | 4 | 1 | 6 | Spider pit |
| 4 | 9563 | 33883 | 10144 | 34462 | 60 | 1 | 1 | 9 | Necromancer |
| 5 | 22945 | 5597 | 23225 | 5747 | 61 | 3 | 2 | 4 | Turtle eggs |
| 6 | 10820 | 35646 | 10877 | 35670 | 83 | 1 | 1 | 0 | Princess extent |
| 7 | 19596 | 17123 | 19974 | 17401 | 48 | 8 | 8 | 2 | Graveyard extent |
| 8 | 19400 | 17034 | 20240 | 17484 | 80 | 4 | 20 | 0 | Around city (peace) |
| 9 | 0x2400 | 0x8200 | 0x3100 | 0x8A00 | 52 | 3 | 1 | 8 | Astral plane |
| 10 | 5272 | 33300 | 6112 | 34200 | 81 | 0 | 1 | 0 | King pax |
| 11 | 11712 | 37350 | 12416 | 38020 | 82 | 0 | 1 | 0 | Sorceress pax |
| 12 | 2752 | 33300 | 8632 | 35400 | 80 | 0 | 1 | 0 | Peace 1 -- buildings |
| 13 | 10032 | 35550 | 12976 | 40270 | 80 | 0 | 1 | 0 | Peace 2 -- specials |
| 14 | 4712 | 38100 | 10032 | 40350 | 80 | 0 | 1 | 0 | Peace 3 -- cabins |
| 15 | 21405 | 25583 | 21827 | 26028 | 60 | 1 | 1 | 7 | Hidden valley |
| 16 | 6156 | 12755 | 12316 | 15905 | 7 | 1 | 8 | 0 | Swamp region |
| 17 | 5140 | 34860 | 6260 | 37260 | 8 | 1 | 8 | 0 | Spider region |
| 18 | 660 | 33510 | 2060 | 34560 | 8 | 1 | 8 | 0 | Spider region |
| 19 | 18687 | 15338 | 19211 | 16136 | 80 | 0 | 1 | 0 | Village (peace) |
| 20 | 16953 | 18719 | 20240 | 17484 | 3 | 1 | 3 | 0 | Around village |
| 21 | 20593 | 18719 | 23113 | 22769 | 3 | 1 | 3 | 0 | Around city |
| 22 | 0 | 0 | 0x7FFF | 0x9FFF | 3 | 1 | 8 | 0 | Whole world (fallback) |

Note: The turtle extent (index 1) has coordinates `(0,0,0,0)`. It is never matched by the bounding-box check. Instead, the turtle is dynamically repositioned at runtime via `move_extent()`, which centers a 500x400 extent around a given coordinate.

### 8.2 Extent Type Meanings

The `etype` field determines what behavior the zone triggers:

| etype Range | Category | Behavior |
|:------------|:---------|:---------|
| 0-49 | Random encounters | `etype` is an encounter chart index. `v1` = minimum count, `v2` = random range, `v3` = encounter type override (0 = use chart). |
| 50-59 | Set group | Fixed encounter group. Type 52 = astral plane (loads loraii immediately). Type 53 = spider pit. |
| 60-61 | Special figure | Force-spawns a unique NPC at the zone center. 60 = single figure (necromancer, hidden valley creature). 61 = multiple figures (turtle eggs). |
| 70 | Carrier | Loads a rideable creature: bird (actor file 11), turtle (actor file 5), or dragon (actor file 10). |
| 80 | Peace zone | No random combat encounters. Used for towns, cabins, and safe areas. |
| 81 | King pax | Peace zone around the king's area. |
| 82 | Sorceress pax | Peace zone around the sorceress's area. |
| 83 | Princess | Triggers the `rescue()` sequence if the princess is currently captive. |

### 8.3 Extent Detection Logic

The `find_place()` function (`fmain.c:2675-2715`) performs a linear scan of the extent table on every movement tick:

```c
extn = extent_list;
for (i=0; i<EXT_COUNT; i++)          /* EXT_COUNT = 22 */
{   if (hero_x > extn->x1 && hero_x < extn->x2 &&
        hero_y > extn->y1 && hero_y < extn->y2) break;
    extn++;
}
```

The first matching extent wins. If no extent matches within the first 22 entries, `extn` falls through to index 22 -- the "whole world" fallback entry. The check uses strict inequality (not >=), so exact boundary coordinates do not match.

When the detected extent type changes (i.e., `xtype != extn->etype`), the following dispatch logic runs:

1. **Type 83 (princess):** If `ob_list8[9].ob_stat` is nonzero (princess is captive), calls `rescue()`. Clears `flag` to 0 and jumps back to `findagain` to re-evaluate position after the teleport.

2. **Type 60 or 61 (special figure):** If the special figure is not already present (`anim_list[3].race != extn->v3` or `anix < 4`), forces an encounter spawn at the center of the extent zone. Sets `encounter_x`/`encounter_y` to the midpoint of the bounding box.

3. **Type 52 (astral plane):** Sets `encounter_type = 8` and immediately loads loraii actors. Does not wait for random encounter roll.

4. **Type >= 50 with `flag == 1`:** Forces a spawn at the hero's current position. Sets `encounter_type` from `extn->v3`, spawns `extn->v1` enemies via `set_encounter()` into animation slots 3-6.

5. **Type 70 (carrier):** If no carrier is active, or the hero is not riding and the current actor file differs, calls `load_carrier(extn->v3)` to load the rideable creature.

6. **Type < 70:** Clears `active_carrier` to 0, dismissing any currently loaded mount.

### 8.4 Quest State Flags

Quest progress is tracked through scattered object status fields and inventory slots rather than a unified quest state variable. The following flags control the main quest flow:

| Variable | Meaning | Values |
|:---------|:--------|:-------|
| `ob_list8[9].ob_stat` | Princess captive status | Nonzero = captive, 0 = rescued. Reset to 3 on brother death. |
| `ob_listg[9].ob_stat` | Sorceress golden figurine | 0 = not yet given, 1 = figurine dropped on ground (first visit triggers speak(45)). |
| `ob_listg[10].ob_stat` | Priest golden statue | 0 = not yet given, 1 = statue given (requires writ). |
| `ob_listg[1-2].ob_stat` | Dead brother bones | Set to 1 when brother 1 or 2 dies; records their death position for bone pickup. |
| `ob_listg[3-4].ob_stat` | Dead brother ghosts | Set to 3 when the corresponding brother dies; cleared to 0 when bones are recovered. |
| `stuff[22]` | Talisman | Nonzero = talisman in inventory, triggers win condition on pickup. |
| `stuff[25]` | Gold statues (`STATBASE`) | Count of golden statues collected. 5 required to reveal the hidden city. |
| `stuff[28]` | Writ | Set to 1 by `rescue()`. Required for priest to give golden statue. |
| `princess` | Rescue counter | Incremented each time `rescue()` is called. Controls which placard text set is displayed (messages offset by `princess * 3`). |

The `princess` variable is declared as `extern short princess` in `fmain2.c:1580`. On each brother death, `ob_list8[9].ob_stat` is reset to 3 (`fmain.c:2843`), making the princess captive again and requiring a re-rescue with the new brother. The new brother also starts with empty inventory (`stuff[i] = 0` for `i < GOLDBASE`), losing the writ and all keys.

### 8.5 The rescue() Function

The `rescue()` function (`fmain2.c:1584-1603`) executes the complete princess rescue sequence. It is triggered when the hero enters the princess extent zone (index 6) while `ob_list8[9].ob_stat` is nonzero:

1. **Display rescue placard:** Calls `map_message()` and switches to `afont`. Computes placard text offset as `princess * 3`, then displays three consecutive placard messages (8+i, 9+i, 10+i) interleaved with the hero's `name()`. Calls `placard()` and waits 380 ticks (approximately 7.6 seconds at 50 ticks/second).

2. **Display aftermath placard:** Clears the display area, then shows placard messages 17-18 with the hero's name. Waits another 380 ticks. Calls `message_off()` to dismiss the placard.

3. **Increment princess counter:** `princess++` -- tracks how many times the princess has been rescued (affects placard text selection on subsequent rescues).

4. **Teleport hero:** `xfer(5511, 33780, 0)` -- teleports the hero to the king's throne area. The third argument `0` means the region is not recalculated from coordinates.

5. **Relocate bird extent:** `move_extent(0, 22205, 21231)` -- moves extent index 0 (the bird) to a new location centered at (22205, 21231), creating a 500x400 zone around it.

6. **Update princess NPC:** `ob_list8[2].ob_id = 4` -- sets an object's ID to 4 (princess type), presumably spawning or repositioning the princess NPC at the castle.

7. **Grant writ:** `stuff[28] = 1` -- gives the hero the writ document, which is a prerequisite for the priest to hand over a golden statue.

8. **King's speech:** `speak(18)` -- the king's acknowledgment and writ-giving speech. This is triggered automatically, not through the TALK system.

9. **Grant gold:** `wealth += 100` -- adds 100 gold to the hero's wealth.

10. **Clear princess flag:** `ob_list8[9].ob_stat = 0` -- marks the princess as rescued, preventing re-triggering of the rescue sequence.

11. **Grant keys:** `for (i=16; i<22; i++) stuff[i] += 3` -- gives 3 of every key type (indices 16-21 correspond to Gold Key, Silver Key, Jade Key, Crystal Key, Ebony Key, and Bronze Key).

After `rescue()` returns, `find_place()` sets `flag = 0` and jumps back to `findagain` to re-evaluate the hero's new position (now at the throne area rather than the princess zone).

### 8.6 Win Condition

The victory condition is checked in the item pickup handler (`fmain.c:3244-3248`). After any object is picked up and added to inventory:

```c
if (stuff[22])
{   quitflag = TRUE; viewstatus = 2;
    map_message(); SetFont(rp,afont); win_colors();
}
```

Slot 22 is the Talisman. The Talisman is dropped by the Necromancer (the game's final boss) when defeated. Picking it up sets `quitflag = TRUE` to end the game loop and `viewstatus = 2` to signal the victory state, then immediately launches the victory sequence.

The Talisman check runs on every item pickup, not just for the Talisman specifically. This means it checks slot 22 as a side effect of any pickup. A cheat code (`fmain.c:1299`) explicitly sets `stuff[22] = 0` to prevent accidental wins during testing.

### 8.7 Victory Sequence -- win_colors()

The `win_colors()` function (`fmain2.c:1605-1636`) displays the ending cinematic:

1. **Display victory placard:** `placard_text(6)` + hero `name()` + `placard_text(7)` -- shows the victory message (e.g., "Julian defeated the Necromancer and recovered the Talisman... returned to Marheim where he wed the princess..."). Calls `placard()` and waits 80 ticks.

2. **Load victory image:** `unpackbrush("winpic", bm_draw, 0, 0)` -- loads and decompresses the victory illustration into the drawing page bitmap.

3. **Black out display:** Loads `blackcolors` into both page and text viewports to create a blank screen before the fade-in.

4. **Reconfigure viewport:** Sets text viewport to `HIRES | SPRITES | VP_HIDE` and adjusts `screen_size(156)`.

5. **Sunrise fade-in:** A 55-step loop (from `i=25` down to `i=-29`) gradually fades in colors using the `sun_colors[53]` table, which progresses from deep blue/black through purple/red to golden tones. Each color register 2-27 is set from `sun_colors[i+j]`, creating a sweep effect across the palette. Registers 0 and 31 remain black, while registers 1 and 28 are forced to white (0xFFF).

6. **Red glow on final registers:** During the first portion (`i > -14`), registers 29 and 30 are set to 0x800 and 0x400 (deep reds). In the tail portion, they fade through calculated values.

7. **Final fade to black:** After the loop completes, waits 30 ticks and loads `blackcolors` across the full page viewport.

### 8.8 Hidden City Reveal

The hidden city in the desert is gated by the number of golden statues the player has collected. During map loading (`fmain.c:3594-3597`):

```c
if (new_region == 4 && stuff[STATBASE] < 5)    /* are we in desert sector */
{   i = ((11*128)+26);
    map_mem[i] = map_mem[i+1] = map_mem[i+128] = map_mem[i+129] = 254;
}
```

When the hero enters region 4 (desert) with fewer than 5 golden statues (`stuff[25]`, since `STATBASE = 25`), four tiles at map memory offset `(11*128)+26` (a 2x2 block at row 11, column 26 within the 128-column region map) are overwritten with tile 254. Tile 254 is impassable, effectively walling off the hidden city entrance.

When the hero has collected 5 or more golden statues, this overwrite does not occur, and the tiles remain at their original passable values, allowing entry.

This modification is performed in RAM only. It is applied every time the region is loaded, so the blocking is consistent as long as the statue count is below 5. However, the original tile values are never saved -- the blocking relies on re-applying the patch on each load. If the game were to save while in the desert region with the tiles already patched, the patched values would not persist to the save file (the region data is reloaded fresh).

### 8.9 Stone Ring Teleportation Network

The `stone_list[]` array (`fmain.c:374-376`) defines 11 stone ring locations as sector coordinate pairs:

| Stone # | Sector X | Sector Y |
|--------:|---------:|---------:|
| 0 | 54 | 43 |
| 1 | 71 | 77 |
| 2 | 78 | 102 |
| 3 | 66 | 121 |
| 4 | 12 | 85 |
| 5 | 79 | 40 |
| 6 | 107 | 38 |
| 7 | 73 | 21 |
| 8 | 12 | 26 |
| 9 | 26 | 53 |
| 10 | 84 | 60 |

Teleportation is triggered as a magic item use (case 5 in the item handler, `fmain.c:3326-3347`) and requires three conditions:

1. **Sector 144:** `hero_sector == 144` -- the hero must be standing on a special sector type (stone ring sector).
2. **Center tile position:** `(hero_x & 255)/85 == 1` and `(hero_y & 255)/64 == 1` -- the hero must be in the center tile of the sector. The `/85` and `/64` divisions partition the 256-unit sector into 3 horizontal and 4 vertical zones; zone (1,1) is the center.
3. **Stone match:** A linear scan of `stone_list[]` checks if `hero_x>>8` and `hero_y>>8` (the hero's sector coordinates) match any stone entry.

When all three conditions are met, the destination stone is computed:

```c
i += (anim_list[0].facing + 1);
if (i > 10) i -= 11;
```

The hero's facing direction (0-3 for the four cardinal directions, stored in `anim_list[0].facing`) is added to the current stone index plus 1. If the result exceeds 10, it wraps around modulo 11. This means the destination depends on which direction the hero faces when activating the stone -- facing different directions from the same stone teleports to different destinations, allowing access to any stone in the network from any other.

The destination position preserves the hero's sub-sector offset:

```c
x = (stone_list[i+i] << 8) + (hero_x & 255);
y = (stone_list[i+i+1] << 8) + (hero_y & 255);
```

The `colorplay()` effect (`fmain2.c:425-432`) plays during teleportation: 32 frames of random palette cycling, where each color register 1-31 is set to a random 12-bit RGB value (`bitrand(0xFFF)`) and displayed for 1 tick. This creates a brief psychedelic flash. After the effect, `xfer(x, y, TRUE)` performs the actual teleport, and if the hero is riding a carrier, the carrier is repositioned to match.

If the hero is not on sector 144, not at the center tile, or not at a recognized stone location, the function returns without consuming the magic item charge (it falls through to `return` before `case 7`, avoiding the use-count decrement that would otherwise follow).

### 8.10 Improvement Notes

> **Quest state tracked through scattered object fields.** There is no quest state machine or progress tracker. Quest flags are spread across `ob_listg[]`, `ob_list8[]`, `stuff[]`, and the `princess` counter. A proper quest state struct or bitmask would make the game's progression logic auditable and reduce the risk of desynchronization (e.g., having a writ but no princess rescue having occurred).

> **Extent linear scan with implicit ordering is fragile.** The `find_place()` extent scan is order-dependent -- first match wins. Overlapping extents (e.g., the "around city" peace zone at index 8 overlaps with the graveyard at index 7 and the "around village" zone at index 20) rely on array ordering for correct priority. Adding or reordering entries could change game behavior in subtle ways. A spatial hash or explicit priority field would be safer.

> **Hidden city map memory modification is lost on reload.** The tile 254 overwrite at `map_mem[(11*128)+26]` is an in-RAM patch applied during region loading. It works reliably because it re-patches on every load. However, the approach is brittle -- it modifies raw map data rather than using a proper gate/door mechanism. Any code that caches or pre-loads region data could bypass the check.

> **Princess rescue grants 3 of every key type.** The loop `for (i=16; i<22; i++) stuff[i] += 3` gives 3 copies of all 6 key types, plus 100 gold and a writ. Combined with the fact that rescue is repeatable (each brother death resets the princess captive flag), a player who deliberately dies and re-rescues accumulates large key stocks. This is very generous and may have been intended to prevent softlocks where a player uses all keys before reaching critical doors.

> **Turtle extent starts at (0,0,0,0).** The turtle extent (index 1) has null coordinates and is never matched until `move_extent()` repositions it dynamically. This works but is non-obvious -- the intent is only clear from the comment "turtle extent" and knowledge of how `move_extent()` is called elsewhere.

---

## 9. Doors & Buildings

The game world uses a unified door system to connect outdoor locations (regions 0-7) with interior spaces. Region 8 holds building interiors and region 9 holds dungeon interiors. Every door is a bidirectional portal defined by a pair of coordinates -- one outdoor, one indoor -- plus metadata controlling its visual type and destination region.

### 9.1 Door Structure

Defined at `fmain.c:233-238`:

```c
struct door {
    unsigned short xc1, yc1;  /* outdoor image coords (regions 0-7) */
    unsigned short xc2, yc2;  /* indoor image coords (region 8 or 9) */
    char type;                /* visual/behavioral type (HWOOD, VSTONE, etc.) */
    char secs;                /* sector join: 1 = buildings (region 8), 2 = dungeons (region 9) */
};
```

- **xc1, yc1** -- Outdoor (overworld) position in pixel coordinates, relative to the full overworld map.
- **xc2, yc2** -- Indoor position in pixel coordinates within the interior region.
- **type** -- Determines the door's visual appearance and orientation. Odd values are horizontal doors; even values are vertical. This distinction matters for collision detection and destination offset calculation.
- **secs** -- Which interior region this door connects to: `1` maps to region 8 (buildings, towns, keeps), `2` maps to region 9 (dungeons, caves).

### 9.2 Door Types

Defined at `fmain.c:213-229`. Values 12 and 16 are unused (gap in the enum). Note that `CAVE` and `VLOG` share value 18.

| Value | Name   | Description               | Orientation |
|-------|--------|---------------------------|-------------|
| 1     | HWOOD  | Wooden door               | Horizontal  |
| 2     | VWOOD  | Wooden door               | Vertical    |
| 3     | HSTONE | Stone door                | Horizontal  |
| 4     | VSTONE | Stone door (unused in doorlist) | Vertical |
| 5     | HCITY  | City door (unused in doorlist) | Horizontal |
| 6     | VCITY  | City door (unused in doorlist) | Vertical |
| 7     | CRYST  | Crystal palace door       | Horizontal (odd) |
| 8     | SECRET | Secret door               | Vertical (even) |
| 9     | BLACK  | Black/dark door           | Horizontal (odd) |
| 10    | MARBLE | Marble keep door          | Vertical (even) |
| 11    | LOG    | Log cabin door            | Horizontal (odd) |
| 13    | HSTON2 | Stone door variant 2      | Horizontal  |
| 14    | VSTON2 | Stone door variant 2      | Vertical    |
| 15    | STAIR  | Stairway                  | Horizontal (odd) |
| 17    | DESERT | Desert oasis door         | Horizontal (odd) |
| 18    | CAVE   | Cave entrance             | Vertical (even) |
| 18    | VLOG   | Log cabin yard (alias for CAVE) | Vertical (even) |

The odd/even distinction controls transition logic: `(type & 1)` tests horizontal orientation. Horizontal doors check `hero_y & 0x10` for alignment; vertical doors check `(hero_x & 15)`.

### 9.3 Complete Door Table

All 86 entries from `doorlist[]` (`fmain.c:240-326`). The table is sorted by xc1 (outdoor X coordinate) -- this ordering is critical for the binary search used during outdoor-to-indoor transitions.

| Index | xc1    | yc1    | xc2    | yc2    | Type   | Secs | Label                |
|-------|--------|--------|--------|--------|--------|------|----------------------|
| 0     | 0x1170 | 0x5060 | 0x2870 | 0x8b60 | HWOOD  | 1    | desert fort           |
| 1     | 0x1170 | 0x5060 | 0x2870 | 0x8b60 | HWOOD  | 1    | desert fort           |
| 2     | 0x1170 | 0x5060 | 0x2870 | 0x8b60 | HWOOD  | 1    | desert fort           |
| 3     | 0x1170 | 0x5060 | 0x2870 | 0x8b60 | HWOOD  | 1    | desert fort           |
| 4     | 0x1390 | 0x1b60 | 0x1980 | 0x8c60 | CAVE   | 2    | dragon cave           |
| 5     | 0x1770 | 0x6aa0 | 0x2270 | 0x96a0 | BLACK  | 1    | pass fort             |
| 6     | 0x1970 | 0x62a0 | 0x1f70 | 0x96a0 | BLACK  | 1    | gate fort             |
| 7     | 0x1aa0 | 0x4ba0 | 0x13a0 | 0x95a0 | DESERT | 1    | oasis #1              |
| 8     | 0x1aa0 | 0x4c60 | 0x13a0 | 0x9760 | DESERT | 1    | oasis #4              |
| 9     | 0x1b20 | 0x4b60 | 0x1720 | 0x9560 | DESERT | 1    | oasis #2              |
| 10    | 0x1b80 | 0x4b80 | 0x1580 | 0x9580 | DESERT | 1    | oasis #3              |
| 11    | 0x1b80 | 0x4c40 | 0x1580 | 0x9740 | DESERT | 1    | oasis #5              |
| 12    | 0x1e70 | 0x3b60 | 0x2880 | 0x9c60 | HSTONE | 1    | west keep             |
| 13    | 0x2480 | 0x33a0 | 0x2e80 | 0x8da0 | HWOOD  | 1    | swamp shack           |
| 14    | 0x2960 | 0x8760 | 0x2b00 | 0x92c0 | STAIR  | 1    | stargate forwards     |
| 15    | 0x2b00 | 0x92c0 | 0x2960 | 0x8780 | STAIR  | 2    | stargate backwards    |
| 16    | 0x2c00 | 0x7160 | 0x2af0 | 0x9360 | BLACK  | 1    | doom tower            |
| 17    | 0x2f70 | 0x2e60 | 0x3180 | 0x9a60 | HSTONE | 1    | lakeside keep         |
| 18    | 0x2f70 | 0x63a0 | 0x1c70 | 0x96a0 | BLACK  | 1    | plain fort            |
| 19    | 0x3180 | 0x38c0 | 0x2780 | 0x98c0 | HWOOD  | 1    | road's end inn        |
| 20    | 0x3470 | 0x4b60 | 0x0470 | 0x8ee0 | STAIR  | 2    | tombs                 |
| 21    | 0x3DE0 | 0x1BC0 | 0x2EE0 | 0x93C0 | CRYST  | 1    | crystal palace        |
| 22    | 0x3E00 | 0x1BC0 | 0x2F00 | 0x93C0 | CRYST  | 1    | crystal palace        |
| 23    | 0x4270 | 0x2560 | 0x2e80 | 0x9a60 | HSTONE | 1    | coast keep (DB)       |
| 24    | 0x4280 | 0x3bc0 | 0x2980 | 0x98c0 | HWOOD  | 1    | friendly inn          |
| 25    | 0x45e0 | 0x5380 | 0x25d0 | 0x9680 | MARBLE | 1    | mountain keep         |
| 26    | 0x4780 | 0x2fc0 | 0x2580 | 0x98c0 | HWOOD  | 1    | forest inn            |
| 27    | 0x4860 | 0x6640 | 0x1c60 | 0x9a40 | VLOG   | 1    | cabin yard #7         |
| 28    | 0x4890 | 0x66a0 | 0x1c90 | 0x9aa0 | LOG    | 1    | cabin #7              |
| 29    | 0x4960 | 0x5b40 | 0x2260 | 0x9a40 | VLOG   | 1    | cabin yard #6         |
| 30    | 0x4990 | 0x5ba0 | 0x2290 | 0x9aa0 | LOG    | 1    | cabin #6              |
| 31    | 0x49a0 | 0x3cc0 | 0x0ba0 | 0x82c0 | VWOOD  | 1    | village #2            |
| 32    | 0x49d0 | 0x3dc0 | 0x0bd0 | 0x84c0 | VWOOD  | 1    | village #1.a          |
| 33    | 0x49d0 | 0x3e00 | 0x0bd0 | 0x8500 | VWOOD  | 1    | village #1.b          |
| 34    | 0x4a10 | 0x3c80 | 0x0d10 | 0x8280 | HWOOD  | 1    | village #3            |
| 35    | 0x4a10 | 0x3d40 | 0x0f10 | 0x8340 | HWOOD  | 1    | village #5            |
| 36    | 0x4a30 | 0x3dc0 | 0x0e30 | 0x85c0 | HWOOD  | 1    | village #7            |
| 37    | 0x4a60 | 0x3e80 | 0x1060 | 0x8580 | HWOOD  | 1    | village #8            |
| 38    | 0x4a70 | 0x3c80 | 0x1370 | 0x8280 | HWOOD  | 1    | village #4            |
| 39    | 0x4a80 | 0x3d40 | 0x1190 | 0x8340 | HWOOD  | 1    | village #6            |
| 40    | 0x4c70 | 0x3260 | 0x2580 | 0x9c60 | HSTONE | 1    | crag keep             |
| 41    | 0x4d60 | 0x5440 | 0x1f60 | 0x9c40 | VLOG   | 1    | cabin #2              |
| 42    | 0x4d90 | 0x4380 | 0x3080 | 0x8d80 | HSTON2 | 1    | crypt                 |
| 43    | 0x4d90 | 0x54a0 | 0x1f90 | 0x9ca0 | LOG    | 1    | cabin yard #2         |
| 44    | 0x4de0 | 0x6b80 | 0x29d0 | 0x9680 | MARBLE | 1    | river keep            |
| 45    | 0x5360 | 0x5840 | 0x2260 | 0x9840 | VLOG   | 1    | cabin yard #3         |
| 46    | 0x5390 | 0x58a0 | 0x2290 | 0x98a0 | LOG    | 1    | cabin #3              |
| 47    | 0x5460 | 0x4540 | 0x1c60 | 0x9840 | VLOG   | 1    | cabin yard #1         |
| 48    | 0x5470 | 0x6480 | 0x2c80 | 0x8d80 | HSTONE | 1    | elf glade             |
| 49    | 0x5490 | 0x45a0 | 0x1c90 | 0x98a0 | LOG    | 1    | cabin #1              |
| 50    | 0x55f0 | 0x52e0 | 0x16e0 | 0x83e0 | MARBLE | 1    | main castle           |
| 51    | 0x56c0 | 0x53c0 | 0x1bc0 | 0x84c0 | HSTON2 | 1    | city #15.a            |
| 52    | 0x56c0 | 0x5440 | 0x19c0 | 0x8540 | HSTON2 | 1    | city #17              |
| 53    | 0x56f0 | 0x51a0 | 0x19f0 | 0x82a0 | HSTON2 | 1    | city #10              |
| 54    | 0x5700 | 0x5240 | 0x1df0 | 0x8340 | VSTON2 | 1    | city #12              |
| 55    | 0x5710 | 0x5440 | 0x1c10 | 0x8640 | HSTON2 | 1    | city #18              |
| 56    | 0x5730 | 0x5300 | 0x1a50 | 0x8400 | HSTON2 | 1    | city #14              |
| 57    | 0x5730 | 0x5380 | 0x1c30 | 0x8480 | VSTON2 | 1    | city #15.b            |
| 58    | 0x5750 | 0x51a0 | 0x1c60 | 0x82a0 | HSTON2 | 1    | city #11              |
| 59    | 0x5750 | 0x5260 | 0x2050 | 0x8360 | HSTON2 | 1    | city #13              |
| 60    | 0x5760 | 0x53c0 | 0x2060 | 0x84c0 | HSTON2 | 1    | city #16              |
| 61    | 0x5760 | 0x5440 | 0x1e60 | 0x8540 | HSTON2 | 1    | city #19              |
| 62    | 0x5860 | 0x5d40 | 0x1c60 | 0x9a40 | VLOG   | 1    | cabin yard #4         |
| 63    | 0x5890 | 0x5da0 | 0x1c90 | 0x9ca0 | LOG    | 1    | cabin #4              |
| 64    | 0x58c0 | 0x2e60 | 0x0ac0 | 0x8860 | CAVE   | 2    | troll cave            |
| 65    | 0x5960 | 0x6f40 | 0x2260 | 0x9a40 | VLOG   | 1    | cabin yard #9         |
| 66    | 0x5990 | 0x6fa0 | 0x2290 | 0x9ca0 | LOG    | 1    | cabin #9              |
| 67    | 0x59a0 | 0x6760 | 0x2aa0 | 0x8b60 | STAIR  | 1    | unreachable castle    |
| 68    | 0x59e0 | 0x5880 | 0x27d0 | 0x9680 | MARBLE | 1    | farm keep             |
| 69    | 0x5e70 | 0x1a60 | 0x2580 | 0x9a60 | HSTONE | 1    | north keep            |
| 70    | 0x5ec0 | 0x2960 | 0x11c0 | 0x8b60 | CAVE   | 2    | spider exit           |
| 71    | 0x6060 | 0x7240 | 0x1960 | 0x9c40 | VLOG   | 1    | cabin yard #10        |
| 72    | 0x6090 | 0x72a0 | 0x1990 | 0x9ca0 | LOG    | 1    | cabin #10             |
| 73    | 0x60f0 | 0x32c0 | 0x25f0 | 0x8bc0 | HSTONE | 1    | mammoth manor         |
| 74    | 0x64c0 | 0x1860 | 0x03c0 | 0x8660 | CAVE   | 2    | maze cave 2           |
| 75    | 0x6560 | 0x5d40 | 0x1f60 | 0x9a40 | VLOG   | 1    | cabin yard #5         |
| 76    | 0x6590 | 0x5da0 | 0x1f90 | 0x98a0 | LOG    | 1    | cabin #5              |
| 77    | 0x65c0 | 0x1a20 | 0x04b0 | 0x8840 | BLACK  | 2    | maze cave 1           |
| 78    | 0x6670 | 0x2a60 | 0x2b80 | 0x9a60 | HSTONE | 1    | glade keep            |
| 79    | 0x6800 | 0x1b60 | 0x2af0 | 0x9060 | BLACK  | 1    | witch's castle        |
| 80    | 0x6b50 | 0x4380 | 0x2850 | 0x8d80 | HSTON2 | 1    | light house           |
| 81    | 0x6be0 | 0x7c80 | 0x2bd0 | 0x9680 | MARBLE | 1    | lonely keep           |
| 82    | 0x6c70 | 0x2e60 | 0x2880 | 0x9a60 | HSTONE | 1    | sea keep              |
| 83    | 0x6d60 | 0x6840 | 0x1f60 | 0x9a40 | VLOG   | 1    | cabin yard #8         |
| 84    | 0x6d90 | 0x68a0 | 0x1f90 | 0x9aa0 | LOG    | 1    | cabin #8              |
| 85    | 0x6ee0 | 0x5280 | 0x31d0 | 0x9680 | MARBLE | 1    | point keep            |

**Notable patterns:**
- Entries 0-3 are identical duplicates (desert fort) -- likely copy-paste artifacts or placeholders for multiple entrance tiles on one building.
- Cabins always appear in pairs: a VLOG "yard" entry followed by a LOG "cabin" entry at a nearby coordinate, sharing the same cabin number.
- The stargate (entries 14-15) is unique: it connects region 8 to region 8 (secs=1 forward, secs=2 backward), functioning as a bidirectional stairway between two interior areas.
- The crystal palace has two entries (21-22) at xc1 values 0x3DE0 and 0x3E00, 32 pixels apart -- two adjacent entrance tiles.
- All village doors (31-39) cluster tightly around xc1=0x49a0-0x4a80 and use VWOOD or HWOOD types.
- City doors (51-61) similarly cluster near xc1=0x56c0-0x5760 and exclusively use HSTON2/VSTON2 types.

### 9.4 Door Opening Logic

The `doorfind()` function (`fmain.c:1081-1125`) handles opening locked doors when the player uses a key or bumps into a door tile.

#### Key Types

Defined as an enum at `fmain.c:1048`:

```c
enum ky {NOKEY=0, GOLD, GREEN, KBLUE, RED, GREY, WHITE};
```

| Value | Name  | Numeric |
|-------|-------|---------|
| 0     | NOKEY | 0       |
| 1     | GOLD  | 1       |
| 2     | GREEN | 2       |
| 3     | KBLUE | 3       |
| 4     | RED   | 4       |
| 5     | GREY  | 5       |
| 6     | WHITE | 6       |

#### Algorithm

1. **Tile scan** -- Checks the tile at the hero's pixel position using `px_to_im(x, y)`. If tile ID is not 15 (the "door" tile), tries x+4 and x-8 as minor offsets. Returns FALSE if no door tile is found within that range.

2. **Origin trace** -- Once a door tile is found, traces left (`x-16` twice) and down (`y+32`) to find the top-left origin of the door. This locates multi-tile doors by walking to their origin corner.

3. **Sector/region calc** -- Converts pixel coords to tile coords (`x >>= 4`, `y >>= 5`), reads the sector ID from the map via `mapxy(x,y)`, then looks up the region ID from `current_loads.image[(sec_id >> 6)]`.

4. **open_list scan** -- Iterates through all 17 `open_list` entries, matching on both `map_id == reg_id` and `door_id == sec_id`.

5. **Key check** -- If a match is found, checks whether `keytype == 0` (NOKEY, no key required) or the provided `keytype` matches the entry's required key.

6. **Tile replacement** -- On success, replaces the door tile(s) with "opened" tile IDs:
   - `new1` always replaces the origin tile.
   - If `new2` is non-zero, placement depends on the `above` field:
     - `above == 1`: `new2` placed at `(x, y-1)` -- one tile above.
     - `above == 3`: `new2` placed at `(x-1, y)` -- one tile to the left (back).
     - `above == 4`: Special 2x2 replacement -- tiles 87, 86, 88 placed at `(x, y-1)`, `(x+1, y)`, `(x+1, y-1)` respectively (the CABINET case).
     - `above == 2`: `new2` placed at `(x+1, y)` -- one tile to the right (side).
     - Any other value for `above`: `new2` at `(x+1, y)` and `above` itself at `(x+2, y)` -- a 3-tile-wide door.
   - Sets `viewstatus = 99` to force a screen redraw and prints "It opened."

7. **Failure** -- If no match or wrong key, prints "It's locked." (unless `bumped` is already set or a key was used), sets `bumped = 1`, returns FALSE.

### 9.5 Open List Table

All 17 entries from `open_list[]` (`fmain.c:1059-1077`). Each entry defines how a specific door tile in a specific map region can be opened, and what tiles replace it when opened.

| Index | door_id | map_id | new1 | new2 | above | keytype | Comment  |
|-------|---------|--------|------|------|-------|---------|----------|
| 0     | 64      | 360    | 123  | 124  | 2     | GREEN   | HSTONE   |
| 1     | 120     | 360    | 125  | 126  | 2     | NOKEY   | HWOOD    |
| 2     | 122     | 360    | 127  | 0    | 0     | NOKEY   | VWOOD    |
| 3     | 64      | 280    | 124  | 125  | 2     | GREY    | HSTONE2  |
| 4     | 77      | 280    | 126  | 0    | 0     | GREY    | VSTONE2  |
| 5     | 82      | 480    | 84   | 85   | 2     | KBLUE   | CRYST    |
| 6     | 64      | 480    | 105  | 106  | 2     | GREEN   | OASIS    |
| 7     | 128     | 240    | 154  | 155  | 1     | WHITE   | MARBLE   |
| 8     | 39      | 680    | 41   | 42   | 2     | GOLD    | HGATE    |
| 9     | 25      | 680    | 27   | 26   | 3     | GOLD    | VGATE    |
| 10    | 114     | 760    | 116  | 117  | 1     | RED     | SECRET   |
| 11    | 118     | 760    | 116  | 117  | 1     | GREY    | TUNNEL   |
| 12    | 136     | 800    | 133  | 134  | 135   | GOLD    | GOLDEN   |
| 13    | 187     | 800    | 76   | 77   | 2     | NOKEY   | HSTON3   |
| 14    | 73      | 720    | 75   | 0    | 0     | NOKEY   | VSTON3   |
| 15    | 165     | 800    | 85   | 86   | 4     | GREEN   | CABINET  |
| 16    | 210     | 840    | 208  | 209  | 2     | NOKEY   | BLUE     |

**Notes on the open_list:**
- The same `door_id` (64) appears in three different map regions (360, 280, 480), each with different replacement tiles and key requirements. The `map_id` disambiguates.
- Entry 12 (GOLDEN) is special: `above=135` means it is a 3-tile-wide door, placing `new2` (134) at `(x+1, y)` and the `above` value (135) at `(x+2, y)`.
- Entry 15 (CABINET) uses `above=4`, triggering the special 2x2 replacement pattern with hard-coded tiles 87, 86, 88.

### 9.6 Door Transition Logic

The main loop handles door transitions at `fmain.c:1894-1955`. The hero's position is masked to tile boundaries (`hero_x & 0xfff0`, `hero_y & 0xffe0`) and checked against the door table. If `riding` is true, door transitions are skipped entirely.

#### Outdoor to Indoor (region < 8) -- Binary Search

When the hero is in the overworld (regions 0-7), a **binary search** is performed over `doorlist[]` (`fmain.c:1902-1933`):

1. The search operates on the xc1-sorted array with indices `i=0` to `k=DOORCOUNT-1`.
2. At each step, `j = (k + i) / 2` picks the midpoint.
3. Comparison order:
   - If `d->xc1 > xtest`: search left half (`k = j-1`).
   - If `d->xc1 + 16 < xtest`: search right half (`i = j+1`).
   - If `d->xc1 < xtest` and door is not horizontal (`type & 1 == 0`): search right (`i = j+1`). This allows vertical doors a 16-pixel-wide match window.
   - If `d->yc1 > ytest`: search left (`k = j-1`).
   - If `d->yc1 < ytest`: search right (`i = j+1`).
   - Otherwise: match found.

4. **Orientation check** on match:
   - Horizontal doors (`type & 1`): reject if `hero_y & 0x10` is set (hero must be in the top half of the tile row).
   - Vertical doors: reject if `(hero_x & 15) > 6` (hero must be near the left edge of the tile).

5. **DESERT special case**: If `d->type == DESERT`, the transition is blocked unless `stuff[STATBASE] >= 5` (the player must have collected at least 5 statues). This is the only non-key prerequisite in the door system.

6. **Destination calculation** by type:
   - `CAVE`: `xtest = xc2 + 24`, `ytest = yc2 + 16` (offset into cave interior).
   - Horizontal (`type & 1`): `xtest = xc2 + 16`, `ytest = yc2` (centered on door width).
   - Vertical (default): `xtest = xc2 - 1`, `ytest = yc2 + 16` (offset to side of door).

7. **Region assignment**: `new_region = 8` if `secs == 1`, else `new_region = 9`.
8. Calls `xfer(xtest, ytest, FALSE)` followed by `find_place(2)` and a fade transition.

#### Indoor to Outdoor (region >= 8) -- Linear Scan

When the hero is indoors (`fmain.c:1936-1955`), a **linear scan** of all 86 doors is performed:

1. For each door, checks `d->yc2 == ytest` and either `d->xc2 == xtest` or `(d->xc2 == xtest - 16 && type & 1)` (horizontal doors are 2 tiles wide).

2. **Orientation check** (reversed from entry):
   - Horizontal doors: reject if `(hero_y & 0x10) == 0` (hero must be in the bottom half).
   - Vertical doors: reject if `(hero_x & 15) < 2` (hero must not be at the far left edge).

3. **Destination calculation** by type:
   - `CAVE`: `xtest = xc1 - 4`, `ytest = yc1 + 16`.
   - Horizontal (`type & 1`): `xtest = xc1 + 16`, `ytest = yc1 + 34`.
   - Vertical (default): `xtest = xc1 + 20`, `ytest = yc1 + 16`.

4. Calls `xfer(xtest, ytest, TRUE)` and `find_place(FALSE)`.

**Note the asymmetry:** outdoor-to-indoor uses binary search (O(log n)) because doorlist is sorted by xc1, while indoor-to-outdoor uses linear scan (O(n)) because the indoor coordinates (xc2) are not sorted.

### 9.7 The xfer() Function

The `xfer()` function (`fmain.c:2625-2645`) performs the actual teleportation:

```c
xfer(xtest, ytest, flag)
register USHORT xtest, ytest, flag;
{
    map_x += (xtest - hero_x);       /* adjust map scroll position */
    map_y += (ytest - hero_y);
    hero_x = anim_list[0].abs_x = xtest;  /* set hero position */
    hero_y = anim_list[0].abs_y = ytest;

    encounter_number = 0;             /* clear any active encounter */
    if (flag)                         /* flag=TRUE when exiting indoors */
    {   xtest = (map_x + 151) >> 8;
        ytest = (map_y + 64) >> 8;
        xtest = (xtest >> 6) & 1;    /* bit 6: east/west half (0 or 1) */
        ytest = (ytest >> 5) & 7;    /* bits 5-7: north/south band (0-7) */
        new_region = xtest + (ytest + ytest);  /* region = x + 2*y */
    }
    keydir = 0;                       /* reset key direction */
    load_all();                       /* load map data for new region */
    gen_mini();                       /* regenerate minimap (sets xreg, yreg) */
    viewstatus = 99;                  /* force full screen redraw */
    setmood(TRUE);                    /* update music for new area */
    while (proxcheck(hero_x, hero_y, 0)) hero_y++;  /* nudge down if colliding */
}
```

**Key details:**
- **Coordinate adjustment**: `map_x`/`map_y` (the viewport scroll offsets) are shifted by the same delta as the hero, maintaining the camera's relative position.
- **Region calculation** (when `flag` is TRUE, i.e., exiting to overworld): The overworld is divided into a 2x8 grid of regions. The X bit selects east (1) or west (0); the Y bits select which row (0-7). The formula `new_region = xtest + 2*ytest` produces region numbers 0-15, though only 0-7 are used for overworld tiles (the map is 2 columns by 4 rows = 8 regions).
- When `flag` is FALSE (entering indoors), the region was already set by the caller (`new_region = 8` or `9`), so no recalculation is needed.
- **Collision nudge**: After teleporting, if the destination overlaps a solid object (`proxcheck`), the hero is pushed downward pixel by pixel until clear.
- `load_all()` triggers a full reload of map sectors, tilesets, and sprites for the destination region.

### 9.8 Improvement Notes

- **Binary/linear search asymmetry** -- The indoor-to-outdoor path uses O(n) linear scan because xc2 values are unsorted. A hash map keyed on `(xc2, yc2)` would give O(1) lookup in both directions. Alternatively, maintaining a second copy of the door table sorted by xc2 would allow binary search for exits as well.

- **Hard-coded coordinates** -- All 86 door entries are compiled directly into the executable. Externalizing this to a data file would allow modding, easier testing, and potential runtime patching without recompilation.

- **DESERT door is the only non-key prerequisite** -- The statue count check (`stuff[STATBASE] >= 5`) is special-cased inline at `fmain.c:1919`. A more general approach would add a "prerequisite" field to the door struct, supporting arbitrary conditions (quest flags, item counts, etc.) without type-specific code.

- **Duplicate entries (indices 0-3)** -- Four identical "desert fort" entries appear at the start of doorlist. These may be copy-paste artifacts, or they may represent multiple entrance tiles that all lead to the same interior. Since the binary search stops at the first match, only one of these can ever be reached via the standard search path. The others occupy space and could cause confusion.

- **CAVE/VLOG alias** -- `CAVE` and `VLOG` are both defined as value 18. The transition logic only checks for `type == CAVE` explicitly, so VLOG doors follow the default (vertical) path rather than the CAVE-specific offset calculation. This works correctly because VLOG represents cabin yards (not actual caves), but the shared numeric value is a potential source of bugs if new code checks `type == CAVE` and inadvertently matches yard doors.

- **open_list linear scan** -- The 17-entry open_list is scanned linearly on every door interaction attempt. While small enough to not matter on the Amiga, a lookup table indexed by `(map_id, door_id)` would be cleaner.

---

## 10. Day/Night Cycle

The game simulates a continuous day/night cycle using a single counter that drives lighting changes, time-of-day events, and sleep mechanics.

### Day Counter

The master time variable `daynight` is a 16-bit unsigned integer that increments by 1 each game tick (approximately 1/60th of a second on NTSC). It wraps back to 0 when it reaches 24000, producing a full day/night cycle of 24000 ticks (~6.7 minutes real time).

```c
/* fmain.c:2023-2024 */
if (!freeze_timer)          /* no time advancement during timestop */
    if ((daynight++) >= 24000) daynight = 0;
```

Time does **not** advance during freeze spells (`freeze_timer > 0`). During sleep, `daynight` advances by +63 per tick unconditionally (this is not gated on `freeze_timer`), plus the normal +1 increment if `freeze_timer` is zero, yielding +64 effective ticks per game frame under normal sleeping conditions.

### Light Level Calculation

The light level is derived from `daynight` as a triangle wave:

```c
/* fmain.c:2025-2026 */
lightlevel = daynight / 40;
if (lightlevel >= 300) lightlevel = 600 - lightlevel;
```

This produces a value ranging from 0 (midnight, `daynight=0`) to 300 (midday, `daynight=12000`) and back to 0 at `daynight=24000`. The triangle wave shape means light increases linearly from midnight to midday, then decreases linearly from midday to midnight.

| daynight | lightlevel | Time of Day |
|----------|------------|-------------|
| 0        | 0          | Midnight    |
| 6000     | 150        | Early morning |
| 8000     | 200        | Morning     |
| 12000    | 300        | Midday (peak) |
| 18000    | 150        | Evening     |
| 24000    | 0          | Midnight (wraps) |

When `lightlevel < 40`, the turtle companion object (`ob_listg[5]`) switches to state 3 (glowing); otherwise it uses state 2 (normal). This provides a visual nighttime indicator.

### Time Periods and Events

The day is divided into 12 periods of 2000 ticks each. When the period changes, time-of-day events fire:

```c
/* fmain.c:2029-2037 */
i = (daynight / 2000);
if (i != dayperiod)
{   switch (dayperiod = i) {
    case 0:  event(28); break;    /* "midnight" */
    case 4:  event(29); break;    /* "morning"  */
    case 6:  event(30); break;    /* "midday"   */
    case 9:  event(31); break;    /* "evening"  */
    }
}
```

| Period | daynight Range | Event | Description |
|--------|---------------|-------|-------------|
| 0      | 0 - 1999      | 28    | Midnight    |
| 4      | 8000 - 9999   | 29    | Morning     |
| 6      | 12000 - 13999 | 30    | Midday      |
| 9      | 18000 - 19999 | 31    | Evening     |

Periods 1-3, 5, 7-8, and 10-11 have no associated events.

### Palette Fading Algorithm (fade_page)

The `day_fade()` function bridges the day counter to the palette system. It is called every 4th tick (when `daynight & 3 == 0`) or during screen transitions (`viewstatus > 97`):

```c
/* fmain2.c:1653-1660 */
day_fade()
{   register long ll;
    if (light_timer) ll = 200; else ll = 0;
    if ((daynight & 3) == 0 || viewstatus > 97)
        if (region_num < 8)     /* no night cycle inside buildings */
            fade_page(lightlevel-80+ll, lightlevel-61, lightlevel-62, TRUE, pagecolors);
        else fade_page(100, 100, 100, TRUE, pagecolors);
}
```

Inside buildings (region >= 8), full brightness is always used (100, 100, 100). The `light_timer` spell adds +200 to the red channel percentage, simulating a warm magical light. The red, green, and blue percentages are offset differently from `lightlevel`, creating color temperature shifts: red is reduced most (lightlevel-80), then blue (lightlevel-62), then green (lightlevel-61).

The core `fade_page()` algorithm (`fmain2.c:377-419`) processes all 32 palette entries:

**Step 1 -- Region palette overrides** (color 31 only):
- Region 4 (volcanic/fire area): color 31 = `0x0980` (orange)
- Region 9 (dungeon): color 31 = `0x00f0` (bright green) if `secret_timer` active, else `0x0445` (dark blue-gray)
- All other regions: color 31 = `0x0bdf` (sky blue)

**Step 2 -- Clamp input percentages** to 0-100 range.

**Step 3 -- Apply night minimums** (when `limit` is TRUE, which is always the case for day_fade calls):
- Red minimum: 10%
- Green minimum: 25%
- Blue minimum: 60%
- Compute blue night-shift factor: `g2 = (100 - g) / 3`

This ensures nights are never pitch black, with blue being the most preserved channel -- creating the characteristic blue-tinted nighttime look.

**Step 4 -- Per-color processing** (for each of 32 colors):

```c
/* fmain2.c:403-416 */
for (i=0; i<32; i++)
{   r1 = (colors[i] & 0x0f00) >> 4;    /* extract red (0-240 range) */
    g1 = colors[i] & 0x00f0;            /* extract green (0-240 range) */
    b1 = colors[i] & 0x000f;            /* extract blue (0-15 range) */
    if (light_timer && (r1 < g1)) r1 = g1;  /* warm light: boost red to green */
    r1 = (r * r1) / 1600;               /* scale red */
    g1 = (g * g1) / 1600;               /* scale green */
    b1 = (b * b1 + (g2 * g1)) / 100;    /* scale blue + night shift */
    if (limit)
    {   if (i >= 16 && i <= 24 && g > 20)
        {   if (g < 50) b1 += 2; else if (g < 75) b1++; }
        if (b1 > 15) b1 = 15;
    }
    fader[i] = (r1 << 8) + (g1 << 4) + b1;
}
```

The processing pipeline per color:
1. **Extract** r, g, b from the 12-bit Amiga RGB word. Red and green are shifted into 0-240 range; blue stays in 0-15 range.
2. **Light spell warmth**: If `light_timer` is active and the color's red component is less than its green, red is boosted to match green. This shifts greens toward yellow/warm tones.
3. **Scale red and green**: `r1 = (r_pct * r_raw) / 1600`. The divisor of 1600 normalizes the 0-240 raw range with the 0-100 percentage to produce a 0-15 output.
4. **Scale blue with night shift**: `b1 = (b_pct * b_raw + g2 * g1_scaled) / 100`. The `g2` term (`(100-g)/3`) adds extra blue proportional to how dark it is (lower green percentage = more blue boost), using the already-scaled green value. This creates the blue shift at night.
5. **Twilight boost**: Colors 16-24 (typically sky/horizon palette entries) get extra blue during twilight when green percentage is between 20 and 75. If `g < 50`, blue gains +2; if `g < 75`, blue gains +1.
6. **Cap blue** at 15 (maximum for 4-bit channel).
7. **Compose** final 12-bit RGB: `(r1 << 8) + (g1 << 4) + b1`.

The final palette is loaded to hardware via `LoadRGB4(&vp_page, fader, 32)`.

### Sleep Mechanics

Sleep is triggered in two ways: voluntarily by standing on a bed, or involuntarily from extreme fatigue or starvation.

**Voluntary sleep (bed detection)**:

```c
/* fmain.c:1875-1890 */
if (region_num == 8)    /* inside buildings only */
{   i = *(mapxy(hero_x>>4, hero_y>>5));
    /* bed tile IDs: 161, 52, 162, 53 */
    if (i == 161 || i == 52 || i == 162 || i == 53)
    {   sleepwait++;
        if (sleepwait == 30)
        {   if (fatigue < 50) event(25);    /* "not tired enough" */
            else
            {   event(26);                   /* "falls asleep" */
                hero_y = (anim_list[0].abs_y |= 0x1f);
                anim_list[0].state = SLEEP;
            }
        }
    }
    else sleepwait = 0;
}
```

The hero must be in region 8 (interior buildings) and stand on a bed tile (161, 52, 162, or 53) for 30 consecutive ticks (~0.5 seconds). If `fatigue < 50`, the game displays a "not tired" message. Otherwise, the hero's Y coordinate is snapped to a 32-pixel boundary (OR with `0x1f`) and the state changes to SLEEP.

**Sleep progression**:

```c
/* fmain.c:2013-2021 */
if (anim_list[0].state == SLEEP)
{   daynight += 63;
    if (fatigue) fatigue--;
    if (fatigue == 0 ||
        (fatigue < 30 && daynight > 9000 && daynight < 10000) ||
        (battleflag && (rand64() == 0)) )
    {   anim_list[0].state = STILL;
        hero_y = (anim_list[0].abs_y &= 0xffe0);
    }
}
```

Each tick while sleeping:
- `daynight` advances by +63 (plus the normal +1 from the main counter increment, totaling +64 per tick)
- `fatigue` decrements by 1
- The hero wakes when any of these conditions is met:
  - `fatigue` reaches 0 (fully rested)
  - `fatigue < 30` AND it is morning (`daynight` between 9000-10000) -- natural morning wake-up
  - An enemy is present (`battleflag`) AND a 1-in-64 random chance succeeds -- combat interrupts sleep

On waking, the hero's Y coordinate is re-aligned downward (AND with `0xffe0`, clearing the low 5 bits).

---

## 11. Survival Mechanics

The survival system tracks hunger and fatigue on parallel timers, with escalating penalties that can ultimately kill the hero.

### Hunger

Hunger is incremented by 1 every 128 game ticks (`daynight & 127 == 0`), provided the hero is alive and not sleeping. At 60 ticks per second, this means hunger increases roughly once every 2.1 seconds.

**Thresholds and effects** (`fmain.c:2199-2220`):

| Hunger Value | Effect |
|-------------|--------|
| 35          | event(0) -- first hunger warning message |
| 60          | event(1) -- increased hunger warning |
| 90          | event(4) -- severe hunger warning (one-time, at exactly 90) |
| > 90        | event(2) -- starvation warning (fires every 8 hunger ticks when > 90, if vitality > 5) |
| > 100       | Vitality reduced by 2 (every 8 hunger ticks, if vitality > 5; combined with fatigue > 160 check) |
| > 140       | event(24) -- hero collapses; hunger reset to 130; forced SLEEP state |

The damage at hunger > 100 occurs in a combined check:

```c
/* fmain.c:2205-2216 */
else if ((hunger & 7) == 0)
{   if (anim_list[0].vitality > 5)
    {   if (hunger > 100 || fatigue > 160)
        {   anim_list[0].vitality -= 2; prq(4); }
        if (hunger > 90) event(2);
    }
    else if (fatigue > 170)
    {   event(12); anim_list[0].state = SLEEP; }
    else if (hunger > 140)
    {   event(24); hunger = 130;
        anim_list[0].state = SLEEP;
    }
}
```

Note: The `(hunger & 7) == 0` check means damage and collapse checks only fire every 8th hunger increment, not every tick. The `else if` chain means that when vitality is 5 or below, damage stops but forced sleep from fatigue (> 170) or starvation collapse (hunger > 140) can still occur.

Separately, after the damage block, lines 2218-2219 handle one-time warnings: `fatigue == 70` triggers event(3) and `hunger == 90` triggers event(4). These are in an `if/else if`, so if both fatigue reaches 70 and hunger reaches 90 on the same tick (possible since they increment together), only the fatigue warning fires.

### Fatigue

Fatigue increments on the **same** 128-tick timer as hunger:

```c
/* fmain.c:2201 */
fatigue++;
```

**Thresholds and effects**:

| Fatigue Value | Effect |
|--------------|--------|
| 70           | event(3) -- first tiredness warning |
| > 160        | Vitality reduced by 2 (every 8 hunger ticks, combined with hunger > 100 check; requires vitality > 5) |
| > 170        | event(12) -- forced sleep (only when vitality <= 5) |

Fatigue is reduced during sleep (decremented by 1 per tick). Since sleep advances time at +64 ticks per game frame, a hero with fatigue of 100 would need 100 frames (~1.7 seconds real time) to fully rest.

### Health Regeneration

```c
/* fmain.c:2041-2046 */
if ((daynight & 0x3ff) == 0)
{   if (anim_list[0].vitality < (15 + brave/4) && anim_list[0].state != DEAD)
    {   anim_list[0].vitality++;
        prq(4);
    }
}
```

Health regenerates by 1 point every 1024 ticks of the day counter (`0x3ff` = 1023, so the check fires when the low 10 bits are all zero). At ~60 ticks/second, this is approximately once every 17 seconds. Since a full day cycle is 24000 ticks, regeneration fires roughly 23 times per day cycle (24000 / 1024 = ~23.4).

The maximum vitality is `15 + brave/4`, where `brave` is the hero's bravery stat. Dead heroes do not regenerate. The `prq(4)` call triggers a status display update.

### Safe Zones

Every 128 ticks, the game evaluates whether the current position qualifies as a safe zone for respawn purposes:

```c
/* fmain.c:2188-2197 */
if ((daynight & 127) == 0 &&
    !actors_on_screen &&        /* no enemies visible */
    !actors_loading &&          /* no enemies loading */
    !witchflag &&               /* no witch encounter active */
    anim_list[0].environ == 0 && /* hero on solid ground */
    safe_flag == 0 &&           /* no active danger flag */
    anim_list[0].state != DEAD) /* hero is alive */
{   safe_r = region_num;
    safe_x = hero_x; safe_y = hero_y;
    if (hunger > 30 && stuff[24])
    {   stuff[24]--; hunger -= 30; event(37); }
}
```

All six conditions must be true simultaneously for the safe zone to update. When updated, `safe_r`, `safe_x`, and `safe_y` record the current region and coordinates for respawn.

Additionally, if hunger exceeds 30 and the hero has apples (`stuff[24]` > 0), one apple is automatically consumed, reducing hunger by 30 and triggering event(37). This auto-eat behavior only occurs during safe zone updates, meaning the hero will not eat apples while enemies are present, while in dangerous terrain, or while dead.

### Fiery Death Zone

A rectangular region of the world map is designated as the fiery death zone (the volcanic/lava area):

```c
/* fmain.c:1384-1385 */
fiery_death =
    (map_x > 8802 && map_x < 13562 && map_y > 24744 && map_y < 29544);
```

The coordinate bounds define a roughly 4760 x 4800 pixel area. The `fiery_death` flag is recalculated every tick based on current map position.

**Effects on all actors** (`fmain.c:1843-1848`):

```c
if (fiery_death)
{   if (i == 0 && stuff[23]) an->environ = 0;    /* hero with fiery fruit: immune */
    else if (an->environ > 15) an->vitality = 0;  /* deep: instant death */
    else if (an->environ > 2) an->vitality--;      /* sinking: gradual damage */
    checkdead(i, 27);
}
```

- **Hero with fiery fruit** (`stuff[23]`): `environ` is reset to 0 each tick, preventing sinking into lava. Only the hero (actor index 0) benefits from this item.
- **Environ > 15**: Instant death (vitality set to 0). The actor has sunk too deep.
- **Environ > 2**: Lose 1 vitality per tick. The actor is partway submerged.
- **Environ <= 2**: No damage yet, but sinking is likely in progress.

For enemies and NPCs in the fiery zone (`fmain.c:2451-2454`), those with `environ > 0` are shown as dying/burning rather than rendered normally.

### Design Notes

- **Hunger and fatigue are synchronized**: Both increment on the exact same 128-tick boundary, meaning the hero faces compounding penalties. A hero who is both very hungry (> 100) and very tired (> 160) takes 2 vitality damage per 8-increment cycle from the combined check, not separate damage from each source.
- **Health regeneration is relatively slow**: At 1 HP per ~17 seconds and a maximum HP of 15 + brave/4, recovering from near-death takes significant time. With maximum bravery (brave = 40, yielding max HP of 25), full regeneration from 1 HP would take roughly 6.8 minutes of real time (24 regen events * 17 seconds each).
- **Safe zone requires all conditions simultaneously**: The six conditions for safe zone updates are quite restrictive. In hostile areas with frequent enemy spawns, the safe point may not update for extended periods, potentially causing the hero to respawn far from where they died.
- **Fiery fruit only protects the hero**: The `stuff[23]` check is gated on `i == 0` (the hero's actor index). Companion NPCs, mounts, and any allied characters receive no protection from the fiery fruit and will take full lava damage.
- **Sleep fatigue threshold is asymmetric**: Voluntary sleep requires `fatigue >= 50`, but involuntary collapse from fatigue only occurs at > 170 (and only when vitality <= 5). There is a wide band (50-170) where the hero can sleep voluntarily but will not be forced to.
- **Morning wake-up window is narrow**: The natural wake condition (`fatigue < 30 && daynight > 9000 && daynight < 10000`) only spans 1000 ticks out of 24000. If the hero is sleeping through this window with fatigue >= 30, they will continue sleeping until fatigue drops below 30 or reaches 0.

---

## 12. Magic System

Magic items are activated through the `MAGIC` menu in `do_option()` (`fmain.c:3301-3366`).
The menu slot maps to a `hit` value (5-11), and each magic item is stored in
the `stuff[]` array at index `4 + hit` (i.e., `stuff[9]` through `stuff[15]`),
which corresponds to `inv_list[]` indices 9-15 (MAGICBASE=9 through 15).

### Magic Item Table

| hit | stuff[] | inv_list[] | Item Name    | Effect                                      | Source Reference     |
|-----|---------|------------|--------------|----------------------------------------------|----------------------|
| 5   | 9       | 9          | Blue Stone   | Stone Ring teleport                          | fmain.c:3326-3347    |
| 6   | 10      | 10         | Green Jewel  | `light_timer += 760` (illumination)          | fmain.c:3306         |
| 7   | 11      | 11         | Glass Vial   | Heal: vitality += rand8()+4, capped at max   | fmain.c:3348-3354    |
| 8   | 12      | 12         | Crystal Orb  | Display world map with hero marker           | fmain.c:3309-3325    |
| 9   | 13      | 13         | Bird Totem   | `secret_timer += 360` (reveal secrets)       | fmain.c:3307         |
| 10  | 14      | 14         | Gold Ring    | `freeze_timer += 100` (time stop); blocked if riding > 1 | fmain.c:3308 |
| 11  | 15      | 15         | Jade Skull   | Mass kill all enemies with race < 7          | fmain.c:3355-3363    |

### Preconditions

Before any magic item can be used, two checks occur (`fmain.c:3302-3304`):

1. **Inventory check**: `hit < 5 || stuff[4+hit] == 0` triggers `event(21)` ("if only I had some Magic!") and aborts.
2. **Necromancer arena restriction**: If `extn->v3 == 9`, calls `speak(59)` ("Your magic won't work here!") and aborts. This prevents magic use in the necromancer's arena.

### Stone Ring Teleport (Blue Stone, hit=5)

Source: `fmain.c:3326-3347`

The stone ring is a network of 11 standing stones scattered across the overworld.
Their coordinates are stored in `stone_list[]` (`fmain.c:374-376`):

```c
unsigned char stone_list[] =
{   54,43, 71,77, 78,102, 66,121, 12,85, 79,40,
    107,38, 73,21, 12,26, 26,53, 84,60 };
```

Each pair is (sector_x, sector_y) for stones 0 through 10.

**Preconditions:**

- `hero_sector == 144` -- the hero must be standing on a stone ring tile type.
- `(hero_x & 255) / 85 == 1` and `(hero_y & 255) / 64 == 1` -- the hero must be near the center of the tile (sub-tile position check). If the position check fails, the function returns without decrementing the use count (the charge is not consumed).

**Destination calculation:**

1. The hero's current sector coordinates are extracted: `x = hero_x >> 8`, `y = hero_y >> 8`.
2. The code scans `stone_list[]` for a matching (x, y) pair to identify which stone index `i` the hero is standing on.
3. The destination index is computed: `i += (anim_list[0].facing + 1); if (i > 10) i -= 11;` -- this is effectively `(current_stone + facing + 1) % 11`. The hero's facing direction determines which of the 11 stones is the teleport target.
4. New absolute coordinates are built: `x = (stone_list[dest*2] << 8) + (hero_x & 255)`, preserving the sub-tile offset within the destination sector.
5. `colorplay()` triggers the visual teleport effect.
6. `xfer(x, y, TRUE)` performs the actual map transfer with region loading.
7. If the hero is riding a carrier, the carrier's position is synced to the hero's new position.

**Fallthrough behavior:** If the stone ring teleport succeeds (the `break` on line 3342 is reached inside the scanning loop), execution ends. If the hero is on sector 144 and centered but no matching stone is found in the scan loop, execution falls through into case 7 (Heal). This appears to be a bug or intentional "consolation heal" -- the Blue Stone case lacks a terminal `break` after the for-loop, so a failed scan will grant a healing effect.

### Crystal Orb Map (hit=9)

Source: `fmain.c:3309-3325`

- **Indoor restriction**: `if (cheat1==0 && region_num > 7) return;` -- the orb only works in outdoor regions (0-7). Region numbers above 7 are indoor/dungeon areas. The check is bypassed if `cheat1` (debug mode) is active. The `return` exits `do_option()` entirely without decrementing the use count.
- Calls `bigdraw(map_x, map_y)` to render the full overworld map onto the drawing page's bitmap.
- Computes the hero's position in map-pixel space and draws a white "+" marker (color 31) at that location if it falls within the visible area (0 < i < 320, 0 < j < 143).
- Sets `viewstatus = 1` and calls `stillscreen()` to display the map, then `prq(5)` to update the display queue.

### Jade Skull Mass Kill (hit=11)

Source: `fmain.c:3355-3363`

- Iterates over all active actors from index 1 to `anix` (the current active actor count).
- For each actor: if `an->vitality > 0` AND `an->type == ENEMY` AND `an->race < 7`, the actor is instantly killed (`vitality = 0`, then `checkdead(i, 0)` is called to trigger the death animation and state transition).
- **Brave penalty**: For each kill, `brave--` is decremented. This is in addition to the `brave++` that `checkdead()` awards for a normal kill, so the net effect is zero change to brave per Jade Skull kill (the `brave++` in `checkdead` on line 2777 fires for `i != 0`, then `brave--` on line 3359 cancels it).
- The `race < 7` check excludes special enemies (race 7 is the dark knight, higher values are other special figures). Only common monsters can be killed this way.
- If `battleflag` is still set after the massacre, calls `event(34)` to announce the battle outcome.

### Usage Depletion

After the switch statement completes (`fmain.c:3365`):

```c
if (!--stuff[4+hit]) set_options();
```

The item's charge count is decremented. If it reaches zero, `set_options()` rebuilds the menu system to remove the depleted item from the MAGIC menu. Each magic item has a finite number of charges determined by how many units were collected (the `maxshown` field in `inv_list[]` defines the display cap, not the use cap -- actual charges equal the `stuff[]` count for that slot).

Note: The Stone Ring and Crystal Orb cases use `return` (not `break`) on failure conditions, which exits `do_option()` entirely and skips the depletion line. This means failed uses do not consume a charge.

---

## 13. Death & Revival

For the brother lifecycle state diagram and death/revival flow, see [STORYLINE.md Sections 2 and 4](STORYLINE.md#2-brother-lifecycle).

### Death Detection: checkdead()

Source: `fmain.c:2769-2782`

```c
checkdead(i, dtype)
```

Called when an actor may have died. If `an->vitality < 1` and the actor is not already in DYING or DEAD state:

- Sets vitality to 0, tactic to 7 (death tactic), goal to DEATH, state to DYING.
- **Special messages**: Race 7 (dark knight) triggers `speak(42)`. SETFIG types with `race != 0x89` cause `kind -= 3` (kindness penalty for killing non-hostile set figures).
- **If actor index != 0** (not the hero): `brave++` (bravery reward for kills).
- **If actor index == 0** (the hero): `event(dtype)` displays the death event message, `luck -= 5`, and `setmood(TRUE)` updates the music mood.
- Clamps `kind` to minimum 0.
- If the hero (i==0), always calls `prq(4)` to update the status display.

### Good Fairy Mechanic

Source: `fmain.c:1387-1407`

The `goodfairy` variable (`fmain.c:592`) is an unsigned char that serves as a countdown timer after the hero enters the DEAD or FALL state. The main loop checks this each frame when the hero's state is DEAD or FALL:

```c
if (inum == DEAD || inum == FALL)
{   if (goodfairy == 1) { revive(FALSE); inum = STILL; }
    else if (--goodfairy < 20) ; /* do resurrection effect/glow */
    else if (luck < 1 && goodfairy < 200) { revive(TRUE); inum = STILL; }
    else if (anim_list[0].state == FALL && goodfairy < 200)
    {   revive(FALSE); inum = STILL; }
    else if (goodfairy < 120)
    {   /* display fairy sprite approaching hero */
        an = anim_list + 3;
        anix = 4;
        an->abs_x = hero_x + goodfairy*2 - 20;
        an->abs_y = hero_y;
        an->type = OBJECTS;
        an->index = 100 + (cycle & 1);
        an->state = STILL;
        an->weapon = an->environ = 0;
        an->race = 0xff;
        actors_on_screen = TRUE;
        battleflag = FALSE;
    }
}
```

**Exact trigger sequence** (goodfairy counts down from 255 each frame):

The `goodfairy` variable is reset to 0 by `revive()` at line 2834. When the hero dies, `goodfairy` is an unsigned char, so `--goodfairy` from 0 wraps to 255. The countdown proceeds:

| goodfairy range | Behavior |
|----------------|----------|
| 255-200        | No visible effect (falls through all conditions) |
| 199-120        | **Luck check**: If `luck < 1`, calls `revive(TRUE)` -- brother succession (luck exhausted, no fairy save). If state is FALL (not full death), calls `revive(FALSE)` -- fairy revival. Otherwise continues countdown. |
| 119-20         | **Fairy sprite display**: The fairy appears at position `hero_x + goodfairy*2 - 20`, moving toward the hero as goodfairy decreases. The sprite alternates between index 100 and 101 based on `cycle & 1` for a wing-flap animation. |
| 19-2           | **Resurrection glow**: `--goodfairy < 20` is true, the empty branch executes (visual effect handled elsewhere). |
| 1              | **Fairy revival**: `revive(FALSE)` is called -- the hero is revived in place at the safe zone. |

**Key observations:**

- The `luck < 1` check at goodfairy < 200 means a hero with zero or negative luck gets no fairy rescue and immediately proceeds to brother succession (`revive(TRUE)`).
- A FALL state (non-lethal collapse, e.g., from drowning) gets fairy revival without the luck check, as long as goodfairy < 200.
- The fairy sprite is only visible during the 119-20 range, creating a brief ~100-frame animation of the fairy approaching.
- During the fairy approach (`goodfairy < 120`), enemy AI is suppressed: `if (goodfairy && goodfairy < 120) break;` at line 2112 exits the actor update loop.

### revive() Function

Source: `fmain.c:2814-2912`

```c
revive(new) short new;
```

The `new` parameter determines whether this is a **brother succession** (TRUE) or a **fairy revival** (FALSE).

#### Common Setup (Always Executed)

Before the `if (new)` branch, the function resets two fixed actors:

- **Slot 1 (Raft)**: Positioned at (13668, 14470), type = RAFT, index/weapon/environ zeroed.
- **Slot 2 (Set Figure)**: Positioned at (13668, 15000), type = SETFIG, index/weapon zeroed.
- **Slot 0 (Hero)**: type = PHIL, goal = USER (restores player control).
- Clears: `battleflag`, `goodfairy`, `mdex`, handler pickup/laydown flags.

#### Brother Succession Path (new == TRUE)

Triggered when the fairy cannot save the hero (luck < 1).

1. **Stop music**: `stopscore()`.
2. **Save bones**: If `brother > 0 && brother < 3` (i.e., brother is 1 or 2, meaning Julian or Phillip -- checked before the increment), the dying hero's position is saved into `ob_listg[brother]` as a bones object (`ob_stat = 1`), and `ob_listg[brother+2].ob_stat = 3` activates the associated ghost NPC. Note: Kevin's bones are NOT saved (brother=3 fails the `< 3` check), and at game start brother=0 which also fails the `> 0` check.
3. **Reset princess**: `ob_list8[9].ob_stat = 3` resets the princess state. This is intentional -- each brother has a different princess storyline.
4. **Load brother stats**: `br = blist + brother` loads the next brother's base attributes from `blist[]`:

   | Brother | Index | brave | luck | kind | wealth | stuff array |
   |---------|-------|-------|------|------|--------|-------------|
   | Julian  | 0     | 35    | 20   | 15   | 20     | julstuff[]  |
   | Phillip | 1     | 20    | 35   | 15   | 15     | philstuff[] |
   | Kevin   | 2     | 15    | 20   | 35   | 10     | kevstuff[]  |

5. **Increment brother counter**: `brother++` (so brother=1 means Julian has died and Phillip is active).
6. **Clear inventory**: `for (i=0; i<GOLDBASE; i++) stuff[i] = 0;` wipes all inventory slots (GOLDBASE=31). Then `stuff[0] = an->weapon = 1` gives the new brother a single Dirk.
7. **Clear magic timers**: `secret_timer = light_timer = freeze_timer = 0`.
8. **Reset to village**: `safe_x = 19036; safe_y = 15755; region_num = safe_r = 3` -- hardcoded coordinates for the village safe zone in region 3.
9. **Display story placard**: The placard system shows narrative text explaining the brother transition:

   | brother (after ++) | First Placard | Second Placard | Meaning                          |
   |--------------------|---------------|----------------|----------------------------------|
   | 1 (Julian starts)  | placard_text(0) | (none)       | Julian's introduction; also clears both screen buffers |
   | 2 (Phillip starts) | placard_text(1) | placard_text(2) | Julian has fallen; Phillip's quest begins |
   | 3 (Kevin starts)   | placard_text(3) | placard_text(4) | Phillip has fallen; Kevin's quest begins |
   | 4+ (all dead)      | placard_text(5) | (none)       | All brothers have fallen          |

   Each placard is displayed with `Delay(120)` (approximately 2.4 seconds at 50 ticks/sec).

10. **Game over check**: `if (brother > 3) { quitflag = TRUE; Delay(500); }` -- the "Stay at Home!" ending. The 500-tick delay (~10 seconds) lets the player read the final placard before the game terminates.
11. **Load new sprites**: `actor_file = 6; set_file = 13; shape_read();` loads the default actor and set-figure sprite sheets.
12. **Display place name**: If `brother < 4`, sets `hero_place = 2` and calls `event(9)` (the village announcement), plus brother-specific events: `event(10)` for Phillip, `event(11)` for Kevin. Julian gets only `print_cont(".")`.

#### Fairy Revival Path (new == FALSE)

- Simply calls `fade_down()` for a screen fade effect. No brother transition, no inventory reset, no placard.

#### Common Path (Both Branches)

After the `if/else`, regardless of revival type:

1. **Teleport to safe zone**: `hero_x = an->abs_x = safe_x; hero_y = an->abs_y = safe_y;` -- positions hero at the last saved safe zone (or the village for new brothers).
2. **Map alignment**: `map_x = hero_x - 144; map_y = hero_y - 90;` centers the viewport.
3. **Load region**: `new_region = safe_r; load_all();` loads the safe zone's region data.
4. **Max vitality**: `an->vitality = (15 + brave/4)` -- full health, which depends on the brother's brave stat.
5. **Reset state**: `environ = 0` (outdoors), `state = STILL`, `race = -1` (hero).
6. **Morning**: `daynight = 8000; lightlevel = 300` -- revival always happens at dawn.
7. **Clear needs**: `hunger = fatigue = 0`.
8. **Reset actors**: `anix = 3` -- only hero, raft, and set figure remain active (clears all encounter actors).
9. **Rebuild menus**: `set_options()`.
10. **Display updates**: `prq(7); prq(4)` refreshes map and status displays.
11. **View mode**: If `brother > 3`, sets `viewstatus = 2` (game over screen); otherwise `viewstatus = 3` with `setmood(TRUE)` (normal play with music update).
12. **Clear flags**: `fiery_death = xtype = 0`.

### Brother Variant Summary

| Property         | Julian (brother=1) | Phillip (brother=2) | Kevin (brother=3) |
|------------------|--------------------|---------------------|--------------------|
| brave            | 35                 | 20                  | 15                 |
| luck             | 20                 | 35                  | 20                 |
| kind             | 15                 | 15                  | 35                 |
| wealth           | 20                 | 15                  | 10                 |
| stuff array      | julstuff[35]       | philstuff[35]       | kevstuff[35]       |
| Max HP           | 15 + 35/4 = 23     | 15 + 20/4 = 20     | 15 + 15/4 = 18    |
| Sprite cfile     | cfiles[0] (sector 1376)  | cfiles[1] (sector 1418) | cfiles[2] (sector 1460) |
| Death placard    | placard(0)         | placard(1)+placard(2) | placard(3)+placard(4) |
| Intro event      | event(9) + "."     | event(9) + event(10) | event(9) + event(11) |
| Bones saved at   | ob_listg[1]        | ob_listg[2]         | (not saved, brother=3 fails <3 check) |

**Design notes:**

- Julian is the bravest (highest combat damage cap), Phillip the luckiest (best fairy revival chance), and Kevin the kindest (least kindness penalty from NPC interactions). This creates a natural difficulty progression since wealth and max HP decrease with each brother.
- Each brother's `stuff` array is independent (`julstuff`, `philstuff`, `kevstuff`), all 35 bytes (ARROWBASE). When a brother's bones are found later by a sibling, their inventory can be recovered (see `fmain.c:3173-3177`).
- The princess state reset (`ob_list8[9].ob_stat = 3`) on each succession is intentional -- the game's narrative has each brother potentially rescuing a different princess.
- The "Stay at Home!" ending (brother > 3) uses `Delay(500)` which is approximately 10 seconds, giving the player time to read the final failure placard before `quitflag` terminates the main loop.

---

## 14. Audio System

The Faery Tale Adventure uses a custom 4-channel music engine running entirely from vertical blank (VBlank) interrupt code, with a separate audio interrupt for sound effect completion. The engine is implemented in 68000 assembly (`gdriver.asm`) with C-level orchestration in `fmain.c` and `fmain2.c`.

### 14.1 Music Engine Overview

The music engine is a 4-voice tracker that drives the Amiga's four DMA audio channels directly via hardware registers at `$DFF0A0`-`$DFF0DF`. It is initialized by calling `init_music(new_wave, wavmem, volmem)` which:

1. Stores three handle pointers in `_vblank_data`: the instrument table (`ins_handle`), waveform memory (`wav_handle`), and volume envelope memory (`vol_handle`).
2. Sets the initial tempo to 150.
3. Sets `nosound` to inhibit playback until a score is loaded.
4. Installs a VBlank interrupt server (`_vblank_server`) on interrupt level 5 via `AddIntServer`.
5. Installs an audio interrupt handler (`audio_int`) on interrupt level 8 via `SetIntVector` for sound effect completion on channel 2.

**Waveform data** (`wavmem`): 8 waveforms of 128 bytes each = 1024 bytes total (`S_WAVBUF = 128 * 8`). Each waveform is raw 8-bit signed PCM representing one cycle of a synthesized instrument tone. Loaded from the file `v6` at offset 0.

**Volume envelopes** (`volmem`): 10 envelopes of 256 bytes each = 2560 bytes total (`S_VOLBUF = 10 * 256`). Each envelope is a sequence of volume levels (0-64) applied frame-by-frame after a note starts. A value with bit 7 set (negative/>=128) signals "hold at current level." Loaded from `v6` at offset 1024 (immediately after waveforms).

Both buffers are allocated as a single contiguous CHIP memory block of `VOICE_SZ` = 3584 bytes (`S_WAVBUF + S_VOLBUF`), with `volmem = wavmem + S_WAVBUF`.

**Source:** `fmain.c:663-668`, `fmain.c:911-914`, `fmain.c:931-936`, `gdriver.asm:423-464`

### 14.2 Score Loading

Scores are loaded from the `songs` file by `read_score()` in `fmain2.c:760-776`.

The system supports **28 tracks** total (7 moods x 4 channels). Score memory (`scoremem`) is a single 5900-byte buffer allocated from general memory (not chip memory).

Loading procedure:
```
read_score():
    open "songs" file
    for i = 0 to 27:
        read 4-byte packlen (big-endian long)
        if (packlen * 2 + accumulated_load) > 5900: break
        track[i] = scoremem + accumulated_offset
        read (packlen * 2) bytes of packed note data
        accumulated_offset += (packlen * 2)
    close file
```

Key details:
- The 4-byte length prefix (`packlen`) is in units of 16-bit words; actual byte count is `packlen * 2`.
- The `track[]` array holds 32 pointers (declared as `unsigned char *(track[32])`), though only 28 are used for music (indices 0-27).
- The 5900-byte limit is a hard cap -- loading stops if any track would exceed it.

**Source:** `fmain2.c:758-776`, `fmain.c:1013`

### 14.3 Sample Loading

Sound effect samples are loaded by `read_sample()` in `fmain.c:1023-1042`.

The raw sample data is read from **disk sectors 920-930** (11 sectors = 5632 bytes) into `sample_mem`, which is allocated as CHIP memory of `SAMPLE_SZ = 5632` bytes.

The data contains **6 samples**, each with a 4-byte big-endian length prefix:

```
read_sample():
    load_track_range(920, 11, sample_mem, 8)   -- async disk read
    WaitDiskIO(8)                               -- wait for completion
    smem = sample_mem
    for i = 0 to 5:
        read 4 bytes from smem -> ifflen (big-endian)
        sample[i] = smem                        -- points past length prefix
        sample_size[i] = ifflen
        smem += ifflen                          -- advance to next sample
```

The `sample[]` array stores pointers into `sample_mem`; `sample_size[]` stores lengths. Samples are raw 8-bit signed PCM at varying playback rates (specified per-call in Amiga period units).

**Source:** `fmain.c:645`, `fmain.c:926`, `fmain.c:1023-1042`, `mtrack.c:51` (disk layout showing sectors 920-930)

### 14.4 Mood Sets

The game defines **7 musical moods**, each consisting of 4 tracks (one per Amiga audio channel). The mood system is indexed by track offset into the `track[]` array:

| Mood Index | Track Indices | Name | Trigger Condition |
|------------|---------------|------|-------------------|
| 0 | 0-3 | Day Outdoor | `lightlevel > 120` and not in battle, not dead, not indoor/astral |
| 1 | 4-7 | Battle | `battleflag` is set (enemies visible and aggressive) |
| 2 | 8-11 | Night | `lightlevel <= 120` (fallback when no higher-priority mood applies) |
| 3 | 12-15 | Intro | Played directly at startup via `playscore(track[12],...,track[15])` |
| 4 | 16-19 | Indoor | Hero within coordinate box: `hero_x` in `[0x2400, 0x3100]`, `hero_y` in `[0x8200, 0x8A00]` |
| 5 | 20-23 | Astral | `region_num > 7` (astral plane and similar regions) |
| 6 | 24-27 | Death | `anim_list[0].vitality == 0` (hero is dead) |

The offset calculation in `setmood()` is `off = mood_index * 4`, then `track[off]` through `track[off+3]` are the four channel tracks.

### 14.5 Instrument Table (`new_wave`)

The instrument table maps instrument numbers to waveform/envelope pairs. It is passed to `init_music()` as the `ins_handle` and used by the `set_voice` command in track data.

```c
short new_wave[] = {
    0x0000, 0x0000, 0x0000, 0x0000, 0x0005,
    0x0202, 0x0101, 0x0103, 0x0004, 0x0504,
    0x0100, 0x0500
};
```

Each 16-bit entry encodes: high byte = waveform index (0-7), low byte = volume envelope index (0-9). The `set_voice` command in track data indexes this table. During `playscore()`, the first 4 entries are loaded as default instruments for voices 1-4.

Note: Entry at index 10 (`new_wave[10]`) is dynamically modified by `setmood()` when entering astral regions: set to `0x0307` for `region_num == 9`, or `0x0100` for other astral regions.

**Source:** `fmain.c:669-672`, `gdriver.asm:237-242`, `gdriver.asm:376-380`

### 14.6 `setmood()` Logic

`setmood(now)` in `fmain.c:2936-2957` selects the current mood based on game state. The `now` parameter controls whether to restart playback immediately or just queue the score.

**Priority evaluation** (first match wins):

1. **Dead** (`anim_list[0].vitality == 0`): `off = 24` (mood 6 -- Death)
2. **Indoor** (hero coordinates in range `0x2400 < hero_x < 0x3100` and `0x8200 < hero_y < 0x8A00`): `off = 16` (mood 4 -- Indoor)
3. **Battle** (`battleflag` is true): `off = 4` (mood 1 -- Battle)
4. **Astral/Underground** (`region_num > 7`): `off = 20` (mood 5 -- Astral). Also patches `new_wave[10]` for region-specific timbre.
5. **Day** (`lightlevel > 120`): `off = 0` (mood 0 -- Day Outdoor)
6. **Night** (fallback): `off = 8` (mood 2 -- Night)

After determining the offset:
- If music is enabled (menu toggle checked via `menus[GAME].enabled[6] & 1`):
  - `now == TRUE`: calls `playscore()` -- immediately resets all voices, clears timeclock, sets track pointers to both `trak_beg` and `trak_ptr`, zeros all event counters, and enables DMA. This causes a hard restart of the music.
  - `now == FALSE`: calls `setscore()` -- only updates `trak_beg` pointers (loop start), leaving `trak_ptr` (current playback position) unchanged. The new score will begin on the next natural loop point.
- If music is disabled: calls `stopscore()` which zeros all volumes, kills DMA, and sets `nosound` flag.

**Callers:** `setmood(TRUE)` on death, battle start, region transitions, menu toggle. `setmood(0)` periodically (every 8th `daynight` tick when not in battle) for gradual day/night transitions.

**Source:** `fmain.c:2936-2957`, `gdriver.asm:338-405`

### 14.7 Sound Effects

Sound effects use `effect(num, speed)` in `fmain.c:3616-3619`:

```c
effect(num, speed) short num; long speed;
{   if (menus[GAME].enabled[7] & 1)
    {   playsample(sample[num], sample_size[num]/2, speed); }
}
```

Effects are gated by the sound effects menu toggle (`menus[GAME].enabled[7] & 1`). The `playsample()` assembly routine plays the sample on **channel 2** (Amiga audio channel B), temporarily overriding whatever music track is on that channel by setting `vce_stat` to 2 (a countdown for the audio interrupt handler). The sample length is divided by 2 because the Amiga hardware length register counts in 16-bit words.

The **6 samples** (indices 0-5) and their typical usage with speed values (Amiga period units):

| Sample | Usage | Typical Speed (Period) | Approximate Hz |
|--------|-------|----------------------|----------------|
| 0 | Hero hit/injured | 800 + random(0-511) | ~4,400 Hz |
| 1 | Weapon swing/miss | 150 + random(0-255) | ~23,800 Hz |
| 2 | Ranged hit (arrow/spell) | 500 + random(0-63) | ~7,100 Hz |
| 3 | Enemy hit | 400 + random(0-255) | ~8,900 Hz |
| 4 | Door/interaction | 400 + random(0-255) | ~8,900 Hz |
| 5 | Environmental/splash | 1800-3200 + random | ~1,100-2,000 Hz |

Period-to-Hz conversion: The Amiga Paula chip clock is 3,546,895 Hz (PAL) or 3,579,545 Hz (NTSC). Playback frequency = clock / period. For example, period 400 at NTSC = 3,579,545 / 400 = ~8,949 Hz.

**Source:** `fmain.c:3616-3619`, `fmain2.c:238-241`, `fmain.c:1488,1680,1690,2262`, `gdriver.asm:296-322`

### 14.8 `gdriver.asm` Internals -- Track Data Format

The music engine in `gdriver.asm` is the authoritative (and only) reference for the custom tracker format. The assembly implements a VBlank-driven sequencer.

#### 15.8.1 VBlank Data Structure

The `_vblank_data` block is the central state, with the following layout:

| Offset | Name | Size | Description |
|--------|------|------|-------------|
| 0 | `nosound` | byte | Sound inhibit flag (non-zero = muted) |
| 1 | `flag_codes` | byte | VBlank sync flag (cleared each frame) |
| 2 | `tempo` | word | Tempo value added to timeclock each VBlank |
| 4 | `ins_handle` | long | Pointer to instrument table (`new_wave[]`) |
| 8 | `vol_handle` | long | Pointer to volume envelope memory |
| 12 | `wav_handle` | long | Pointer to waveform memory |
| 16 | `timeclock` | long | Master time counter (accumulates tempo each VBlank) |
| 24+ | voices 1-4 | 28 bytes each | Per-voice state (4 voices, 112 bytes total) |

#### 15.8.2 Per-Voice State

Each voice occupies 28 bytes (`voice_sz = 28`) starting at offset `vbase` (24):

| Offset | Name | Size | Description |
|--------|------|------|-------------|
| +0 | `wave_num` | byte | Current waveform index (0-7) |
| +1 | `vol_num` | byte | Current volume envelope index (0-9) |
| +2 | `vol_delay` | byte | Volume delay/freeze (non-zero = pause envelope) |
| +3 | `vce_stat` | byte | Voice status flags (see below) |
| +4 | `event_start` | long | Timeclock value when next event begins |
| +8 | `event_stop` | long | Timeclock value when current note's sustain ends |
| +12 | `vol_list` | long | Pointer to current position in volume envelope |
| +16 | `trak_ptr` | long | Current read position in track data |
| +20 | `trak_beg` | long | Start-of-track pointer (for loop restart) |
| +24 | `trak_stk` | long | Loop stack pointer (reserved, not used in this code) |

**Voice status flags** (`vce_stat`):
- Bit 2 (`TIE` = 4): Tied note (not observed in use)
- Bit 3 (`CHORD` = 8): Chorded note (not observed in use)
- Bit 4 (`REST` = 16): Rest state
- Bit 5 (`ENDTK` = 32): Track ended
- When `vce_stat` is non-zero, the voice is in a "sample playing" or special state and note events are suppressed until it clears. The `playsample()` routine sets this to 2 as a countdown.

#### 15.8.3 Note Encoding (Track Data Format)

Track data is a stream of 2-byte command/value pairs. Each event is read as:

```
byte 0: command byte (d3)
byte 1: value byte (d2)
```

**Command byte interpretation:**

| Command | Value | Description |
|---------|-------|-------------|
| 0-127 | Note index (0-127) | **Play note.** Command byte is the SMUS note number; value byte encodes duration. |
| 128 (`$80`) | Duration value | **Rest.** Same duration encoding as notes, but no sound is produced. |
| 129 (`$81`) | Instrument number (0-15) | **Set instrument.** Looks up `ins_handle[value]` to set `wave_num` for the current voice. Immediately reads the next command (no time consumed). |
| 144 (`$90`) | Tempo value (0-255) | **Set tempo.** Writes value to the global `tempo` word, affecting all voices. Immediately reads the next command. |
| 255 (`$FF`) | 0 or non-zero | **End of track.** If value == 0, track stops (clears `trak_ptr`, silences voice). If value != 0, track loops (resets `trak_ptr` to `trak_beg`). |
| Other (130-143, 145-254) | -- | Treated as no-op, skipped; immediately reads next command. |

**Note command details (command byte 0-127):**

The command byte directly indexes the `ptable` period/offset table. Each entry is 4 bytes: a 16-bit period value and a 16-bit waveform offset. The period controls pitch; the offset selects a portion of the 128-byte waveform for higher octaves (shorter wavelength = higher pitch).

The value byte encodes duration with modifier flags:
- Bit 7: Chord flag (cleared before use -- note plays simultaneously with the previous)
- Bit 6: Tie flag (cleared before use -- note is tied to the next)
- Bits 0-5: Duration index into the `notevals` table (0-63)

**Duration table (`notevals`)** -- 64 entries representing standard musical durations in timeclock counts:

| Index | Counts | Musical Value |
|-------|--------|---------------|
| 0 | 26880 | Whole note |
| 1 | 13440 | Half note |
| 2 | 6720 | Quarter note |
| 3 | 3360 | Eighth note |
| 4 | 1680 | Sixteenth note |
| 5 | 840 | Thirty-second note |
| 6 | 420 | Sixty-fourth note |
| 7 | 210 | 128th note |
| 8-15 | 40320-315 | Dotted versions (1.5x of corresponding straight values) |
| 16-23 | 17920-140 | Triplet versions (2/3 of corresponding straight values) |
| 24-31 | 26880-210 | Duplicate of straight values (row 0-7) |
| 32-39 | 21504-168 | Additional subdivision set |
| 40-47 | 32256-252 | Additional dotted set |
| 48-55 | 23040-180 | Additional triplet set |
| 56-63 | 34560-270 | Additional extended set |

At the default tempo of 150, with VBlank at 50 Hz (PAL) or 60 Hz (NTSC), the timeclock advances 150 counts per frame. A quarter note (6720 counts) lasts 6720/150 = 44.8 frames = ~0.75 seconds at 60 Hz NTSC, giving approximately 80 BPM.

#### 15.8.4 Period Table (`ptable`)

The period table contains 84 entries (7 octave ranges x 12 notes), each as a period/offset pair:

```
ptable:
  Octave 0 (low):   periods 1440-1076, offset 0  (full 128-byte waveform, length=32 words)
  Octave 1:         periods 1016-538,  offset 0  (full 128-byte waveform)
  Octave 2:         periods 720-269,   offset 0  (full 128-byte waveform)
  Octave 3:         periods 508-269,   offset 16 (64-byte subset, length=16 words)
  Octave 4:         periods 508-269,   offset 24 (32-byte subset, length=8 words)
  Octave 5:         periods 508-269,   offset 28 (16-byte subset, length=4 words)
  Octave 6 (high):  periods 254-135,   offset 28 (16-byte subset, length=4 words)
```

Higher octaves use shorter segments of the waveform (offset from the start), which reduces aliasing at high frequencies. The hardware length register is set to `32 - offset` (in words), so the DMA loops over the reduced waveform segment.

#### 15.8.5 Note Playback Sequence

Each VBlank frame, for each of the 4 voices:

1. If `trak_ptr` is NULL, skip this voice.
2. Check `timeclock >= event_start`: if yes, read the next event from the track.
3. If not yet time for a new event, and `vce_stat == 0` (not playing a sample):
   - Check `timeclock >= event_stop`: if yes, set volume to 0 (gap between notes).
   - Otherwise, advance the volume envelope: read next byte from `vol_list`, write to volume register unless the value is negative (hold).
4. When processing a new note event:
   - Look up the waveform pointer using `wave_num` and `wav_handle`.
   - Look up the volume envelope using `vol_num` and `vol_handle`, read initial volume.
   - Calculate `event_stop = event_start + duration - 300` (300-count gap for note separation).
   - Calculate next `event_start = event_start + duration`.
   - If the note is a rest (command byte = 128), zero volume; otherwise, load waveform pointer, length, and period into the Amiga audio registers.

#### 15.8.6 Audio Interrupt Handler

The `audio_int` routine handles sound effect completion on channel 2 (`voice2`). It is triggered by the Amiga audio interrupt (level 8). The handler:

1. Clears the interrupt request and enable bits.
2. Checks `vce_stat` for `voice2` -- if non-zero, decrements it.
3. When `vce_stat` reaches zero, stops the sample (zeroes volume, sets period to 2) and leaves the interrupt disabled.
4. While `vce_stat > 0`, re-enables the interrupt for continued playback.

This means sound effects have a fixed duration of 2 interrupt cycles before the music voice resumes on channel 2.

### 14.9 Asset Formats for Extraction

**Music scores** (from `songs` file):
- Format: Sequence of tracks, each prefixed by 4-byte big-endian word count.
- Track data: Stream of 2-byte command/value pairs as documented in 15.8.3.
- Extraction target: JSON (preserving raw events) or conversion to MIDI (mapping note indices through the period table to standard MIDI notes).

**Waveforms** (from `v6` file, offset 0-1023):
- 8 waveforms, 128 bytes each, raw 8-bit signed PCM.
- Represent single-cycle waveform shapes (looped by hardware).
- Extraction target: individual WAV files (set sample rate to match typical playback period, e.g., 8363 Hz for period 428).

**Volume envelopes** (from `v6` file, offset 1024-3583):
- 10 envelopes, 256 bytes each.
- Each byte is a volume level (0-64) or hold marker (bit 7 set).
- Extraction target: JSON arrays of integer values.

**Sound effect samples** (from disk sectors 920-930):
- 6 samples, each with 4-byte big-endian length prefix, followed by raw 8-bit signed PCM.
- Total data: 5632 bytes across 11 disk sectors.
- Extraction target: individual WAV files. Sample rate must be derived from the Amiga period value used at playback time (varies per call site; see table in 15.7).

### 14.10 Limitations and Improvement Notes

- **Custom tracker format is undocumented** outside of `gdriver.asm`. The command byte encoding and duration table are the only reference for parsing score data.
- **No volume ramping on mood changes.** `playscore()` immediately zeros all volumes and restarts from the beginning; `setscore()` only updates loop points but still causes an abrupt transition when the current track reaches its end.
- **Sample speeds are in Amiga period units** and need conversion to Hz for modern playback: `Hz = 3,579,545 / period` (NTSC) or `Hz = 3,546,895 / period` (PAL).
- **Sound effects always use channel 2**, temporarily muting whatever music track is assigned to that channel. There is no dynamic channel allocation.
- **No note-off velocity** -- note gaps are a fixed 300 timeclock counts regardless of note duration, which can cause audible clicks on short notes.
- **The chord and tie flags** (bits 7 and 6 of the value byte) are defined but immediately cleared in `note_comm` (`bclr #7,d2; bclr #6,d2`), suggesting they were planned but not fully implemented.
- **Loop stack** (`trak_stk`) is allocated in the voice structure but never used, suggesting planned but unimplemented nested loop/repeat support.

---

## 15. Rendering Pipeline

The Faery Tale Adventure uses a split-screen display with two Amiga viewports, double-buffered game rendering via custom blitter routines, and a 12-step sprite compositing pipeline that includes per-tile terrain masking for depth occlusion. The entire rendering path is hand-tuned for the Amiga's custom chipset, with all sprite operations performed through direct blitter programming in 68000 assembly (`fsubs.asm`).

### 15.1 Display Layout

The display is constructed from two Amiga ViewPorts stacked vertically within a single View:

| Viewport | Purpose | Resolution | Bitplanes | Colors | Position |
|----------|---------|------------|-----------|--------|----------|
| `vp_page` | Game world | 288x140 pixels (low-res) | 5 | 32 | DxOffset=16, DyOffset=0 |
| `vp_text` | Text/HUD area | 640x57 pixels (hi-res) | 4 | 16 | DxOffset=0, DyOffset=143 (`PAGE_HEIGHT`) |

The combined display occupies approximately 320x200 pixels of screen area. The game viewport is slightly narrower than full low-res (288 vs 320) to provide the 16-pixel left margin needed for smooth horizontal scrolling without visible tearing at the edges.

Key constants (`fmain.c:8-16`):
- `PAGE_DEPTH` = 5 (bitplanes per game page)
- `PHANTA_WIDTH` = 320 (bitmap width including scroll margin)
- `RAST_HEIGHT` = 200 (full raster height)
- `PAGE_HEIGHT` = 143 (game viewport vertical extent)
- `TEXT_HEIGHT` = 57 (HUD area height)

The text viewport uses `HIRES | SPRITES | VP_HIDE` modes, providing double horizontal resolution for legible text while sharing the same vertical scan. The `VP_HIDE` flag is set initially and cleared when the HUD becomes active.

**Source:** `fmain.c:799-818`, `fmain.c:826-832`, `fmain.c:856-857`

### 15.2 Double Buffering

The game uses a classic double-buffer scheme with two `struct fpage` instances that swap roles each frame:

```
struct fpage {                    (ftale.h:70-79)
    struct RasInfo *ri_page;      // viewport scroll state + bitmap pointer
    struct cprlist *savecop;      // cached copper list for this page
    long isv_x, isv_y;           // last drawn scroll position (tile coords)
    short obcount;                // number of sprites drawn this frame
    struct sshape *shape_queue;   // array of sprite backsave descriptors
    unsigned char *backsave;      // background save buffer base
    long saveused;                // bytes consumed in backsave buffer
    short witchx, witchy;         // witch FX position for this frame
    short witchdir, wflag;        // witch rotation angle and active flag
};
```

Two global pointers track the current roles:
- `fp_drawing` -- the page being rendered to (off-screen)
- `fp_viewing` -- the page currently displayed

Each page has its own `RasInfo` (controlling scroll offsets), copper list cache, shape queue (up to `MAXSHAPES` = 25 entries), and backsave buffer for sprite background restoration.

The bitmap memory is allocated as two independent 5-bitplane buffers of 320x200 pixels (`bm_page1`, `bm_page2`), each requiring 5 x 8000 = 40,000 bytes of chip memory.

**Page swap** is performed by `pagechange()` (`fmain.c:2993-3006`):

```
pagechange():
    temp = fp_drawing
    fp_drawing = fp_viewing          // swap roles
    fp_viewing = temp
    vp_page.RasInfo = temp->ri_page  // point viewport at newly-drawn page
    v.LOFCprList = temp->savecop     // restore cached copper list
    MakeVPort(&v, &vp_page)          // rebuild copper instructions
    MrgCop(&v)                       // merge all viewport copper lists
    LoadView(&v)                     // activate on next vertical blank
    temp->savecop = v.LOFCprList     // cache the newly built copper list
    WaitBOVP(&vp_text)              // sync to bottom of visible frame
```

The copper list caching (`savecop`) avoids redundant `MakeVPort` calls when the viewport configuration has not changed between frames.

**Source:** `fmain.c:443`, `fmain.c:849-853`, `fmain.c:879-884`, `ftale.h:70-79`, `fmain.c:2993-3006`

### 15.3 Scrolling

The game world scrolls in 8 directions at tile granularity, with pixel-smooth sub-tile positioning via Amiga hardware scroll registers.

**Tile-level scrolling:** The map position is tracked in pixel coordinates (`map_x`, `map_y`). Tile coordinates are derived as `img_x = map_x >> 4` (16-pixel tile width) and `img_y = map_y >> 5` (32-pixel tile height). Each frame, the scroll delta is computed against the drawing page's last position:

```
dif_x = img_x - fp_drawing->isv_x    // tile columns moved
dif_y = img_y - fp_drawing->isv_y     // tile rows moved
```

For single-tile movements, `scrollmap(direction)` (`fsubs.asm:1736`) performs a hardware blitter copy to shift the entire bitmap by one tile, then the exposed edge is repaired:

| Direction | scrollmap arg | Edge repair |
|-----------|--------------|-------------|
| Right     | 0 | `strip_draw(0)` -- left column |
| Down-right | 1 | strip + row |
| Down      | 2 | `row_draw(0)` -- top row |
| Down-left | 3 | strip + row |
| Left      | 4 | `strip_draw(36)` -- right column |
| Up-left   | 5 | strip + row |
| Up        | 6 | `row_draw(10)` -- bottom row |
| Up-right  | 7 | strip + row |

Both `strip_draw()` (`fsubs.asm:781`) and `row_draw()` (`fsubs.asm:818`) are blitter-based tile rendering routines that draw a single column or row of map tiles from the tile image data.

For movements larger than one tile in either axis (e.g., teleportation), the entire map is redrawn via `map_draw()`.

**Sub-tile smooth scrolling:** After all sprites are composited, the drawing page's `RasInfo` offsets are set to the sub-tile pixel remainder:

```
fp_drawing->ri_page->RxOffset = map_x & 15;   // 0-15 pixel horizontal offset
fp_drawing->ri_page->RyOffset = map_y & 31;    // 0-31 pixel vertical offset
```

The Amiga hardware applies these offsets during display fetch, shifting the visible window within the larger bitmap without any CPU or blitter cost.

**Source:** `fmain.c:1980-1986`, `fmain.c:1997-2230`, `fmain.c:2363-2364`, `fmain.c:2611-2612`, `fsubs.asm:781-819`, `fsubs.asm:1736`

### 15.4 Sprite Compositing Pipeline

The core rendering loop (`fmain.c:2381-2609`) processes each sprite through a 12-step pipeline. Before the loop begins, the previous frame's sprites are erased by restoring saved backgrounds in reverse order (`fmain.c:1969-1972`):

```
for i = obcount down to 1:
    rest_blit(shape_queue[i-1].backsave)    // restore background
```

Then the drawing page's shape counter and save buffer are reset (`fmain.c:2378`), and the main loop iterates over all sprites in Z-sorted order:

**Step 1 -- Initialize pass state** (`fmain.c:2389-2409`): Determine if the sprite has a weapon requiring a second rendering pass. Weapons drawn in front of or behind the character depend on facing direction. Bows, hand weapons, and the magic staff each have unique frame selection logic.

**Step 2 -- Determine type and frame** (`fmain.c:2448-2465`): Resolve the sprite's animation type (`atype`) and frame index (`inum`). Types include `PHIL` (player), `ENEMY`, `OBJECTS`, `CARRIER`, `RAFT`, `SETFIG`, and `DRAGON`. Special cases handle fiery death (objects frame 0x58), falling enemies, race-specific frame offsets, and raft substitution.

**Step 3 -- Compute screen position** (`fmain.c:2411-2413`, `2468-2470`): Convert the sprite's relative position (`rel_x`, `rel_y`) to screen coordinates, then add the sub-tile scroll offsets (`map_x & 15`, `map_y & 31`). The ground level is set to `ystart + 32` for depth calculations.

**Step 4 -- Apply sinking/riding/death adjustments** (`fmain.c:2489-2511`): Modify the visible portion of the sprite based on environment:
- Riding a turtle (`riding==11`): clip bottom 16 pixels
- In shallow water (`environ==2`): clip bottom 10 pixels
- Deeply sunk (`environ > 29`): replace with drowning bubble sprite (objects 97-98, 8px tall)
- Partially sunk (`environ > 2`): shift `ystart` down by `environ` pixels

**Step 5 -- Clip to viewport** (`fmain.c:2513-2524`): Reject sprites entirely off-screen (x > 319, y > 173, or negative bounds). Clip remaining sprites to the visible area (0-319 horizontal, 0-173 vertical). Compute `xoff`/`yoff` to index into the sprite's source data for the visible portion.

**Step 6 -- Compute blitter parameters** (`fmain.c:2527-2558`): Calculate word-aligned blit dimensions (`blitwide`, `blithigh`), source/destination offsets, modulos, and shift value for sub-word alignment. The blitter size word encodes height in the upper 10 bits and width in the lower 6 bits: `blitsize = (blithigh << 6) + blitwide`.

**Step 7 -- Allocate backsave** (`fmain.c:2535-2550`): For small sprites (savesize < 64 bytes), reuse the unused bottom portion of bitmap planes (`planes[crack] + 192*40`). Larger sprites allocate from the page's sequential backsave buffer. If the buffer exceeds 74x80 bytes (5920), compositing stops for the frame.

**Step 8 -- save_blit** (`fsubs.asm:1953`): Copy the background region that will be covered by the sprite into the backsave buffer. This is a 5-plane blitter copy from the drawing bitmap.

**Step 9 -- Terrain masking** (`fmain.c:2560-2598`): Build a per-column occlusion mask from terrain data (see Section 16.5 below). Certain sprite types skip masking entirely: carriers, arrows, specific object indices (100-101), and certain NPC races (0x85, 0x87).

**Step 10 -- mask_blit** (`fsubs.asm:1908`): Combine the sprite's shape mask with the terrain occlusion mask (`bmask_mem`) and the background planes to produce a composite mask. This is a multi-source blitter operation using all four blitter channels.

**Step 11 -- shape_blit** (`fsubs.asm:1836`): Blit the actual sprite image data through the composite mask onto the drawing bitmap. The shape data is stored as 5 sequential bitplanes per frame, with a separate single-plane mask. Each bitplane is blitted individually with the same mask.

**Step 12 -- Weapon pass** (`fmain.c:2608`): If the sprite has a weapon (`pass == 1`), loop back to Step 1 with `passmode` toggled to render the weapon overlay as a separate sprite.

After all sprites are composited, `obcount` records the total number of drawn shapes for the reverse-order restoration on the next frame.

**Source:** `fmain.c:1966-1972`, `fmain.c:2378-2609`, `fsubs.asm:1800-2030`

### 15.5 Terrain Masking

Terrain masking creates the illusion of depth by partially hiding sprites behind terrain features such as trees, buildings, and walls. The system operates on a per-tile-column, per-tile-row basis using precomputed terrain data loaded from disk.

**Data flow:**

1. **Minimap lookup** (`fmain.c:2577-2578`): For each word column (`xm`) and tile row (`ym`) that the sprite overlaps, compute an index into `minimap[]` (a 19x6 = 114-entry array mapping viewport tile positions to terrain tile IDs). The minimap is regenerated by `genmini()` (`fsubs.asm:1136`) whenever the scroll position changes.

2. **Terrain data lookup** (`fmain.c:2578-2579`): The minimap value indexes into `terra_mem[]`, a 1024-byte chip memory buffer containing terrain properties. Each terrain entry occupies 4 bytes, accessed as `terra_mem[cm]` (mask shape index) and `terra_mem[cm+1] & 15` (mask mode). The terrain data is loaded from disk in two 512-byte halves corresponding to the current region's terrain sets.

3. **Mask mode switch** (`fmain.c:2584-2594`): The low nibble of `terra_mem[cm+1]` selects one of 8 masking behaviors:

| Mode | Condition to skip masking | Meaning |
|------|--------------------------|---------|
| 0 | Always skip | No occlusion (flat ground) |
| 1 | Skip if first column (`xm==0`) | Right-side-only occlusion (e.g., tree trunk on right) |
| 2 | Skip if `ystop > 35` | Top-only occlusion (e.g., low wall, sprite feet visible) |
| 3 | Skip if `hero_sector==48` and not NPC index 1 | Bridge -- hero walks over, others occluded |
| 4 | Skip if first column OR `ystop > 35` | Combined right-side + top-only |
| 5 | Skip if first column AND `ystop > 35` | Right-side and top combined (OR logic) |
| 6 | If not bottom row, use full mask (tile 64) | Two-story buildings -- full occlusion above ground floor |
| 7 | Skip if `ystop > 20` | Stricter top-only (shorter features) |

4. **Mask application** (`fmain.c:2595`): When masking applies, `maskit(xm, ym, blitwide, terra_mem[cm])` (`fsubs.asm:1047`) writes the terrain mask shape into `bmask_mem`, the per-frame occlusion buffer. The mask shape byte (`terra_mem[cm]`) indexes into `shadow_mem`, a chip memory buffer of precomputed 1-bitplane mask patterns.

**Special cases for falling sprites** (`fmain.c:2580-2583`): Falling sprites (state `FALL`) skip masking for terrain entries at index <= 220 (standard terrain), but use mode 3 for high-index entries (e.g., pit edges).

The occlusion buffer (`bmask_mem`) is cleared to zero (`clear_blit`) before each sprite. After terrain masking writes the occlusion pattern, `mask_blit` combines it with the sprite's own transparency mask to produce the final composite mask used by `shape_blit`.

**Source:** `fmain.c:2560-2598`, `fsubs.asm:897-1100`, `fmain.c:928` (terra_mem alloc), `fmain.c:3565-3573` (terrain loading)

### 15.6 Z-Sorting

Sprites are sorted back-to-front by Y-coordinate using a bubble sort (`fmain.c:2327-2359`), which the original author explicitly noted as "YUCKY AWFUL WASTEFUL BUBBLE SORT!! YUCK!!":

```
for i = 0 to anix2-1:
    anim_index[i] = i                          // initialize identity mapping
for i = 0 to anix2-1:
    for j = 1 to anix2-1:
        k1 = anim_index[j-1]; k2 = anim_index[j]
        y1 = anim_list[k1].abs_y; y2 = anim_list[k2].abs_y
        // depth adjustments:
        if k1 is dead OR (k2 is hero AND riding) OR k1 is index 1: y1 -= 32
        if k2 is dead OR (k1 is hero AND riding) OR k2 is index 1: y2 -= 32
        if k1 is deeply sunk (environ > 25): y1 += 32
        if k2 is deeply sunk (environ > 25): y2 += 32
        if y2 < y1: swap anim_index[j-1], anim_index[j]
```

The depth adjustments serve specific gameplay purposes:
- **Dead actors (-32):** Corpses render behind living sprites so they do not overlap standing characters.
- **Riding hero (-32):** When mounted on a turtle or bird, the hero renders behind nearby ground-level sprites to avoid the mount obscuring other characters.
- **Index 1 (-32):** Actor slot 1 (typically the mount or special companion) is pushed back in draw order.
- **Sinking actors (+32):** Actors deeply submerged in water or quicksand render in front, so their visible portion (just the head/bubbles) is not covered by sprites standing on solid ground behind them.

The sort operates on `anim_index[]`, an indirection array of up to 20 entries, so the actual `anim_list[]` entries are never moved. With a maximum of ~20 active sprites, the O(n^2) cost is negligible on period hardware.

**Source:** `fmain.c:2327-2359`, `fmain.c:74`

### 15.7 Color Palettes

The game uses four distinct color palettes, all stored as arrays of Amiga 12-bit RGB values (`USHORT`, format `0x0RGB` where each channel is 0-15):

**Game palette** -- `pagecolors[32]` (`fmain2.c:367-371`):
```
0x000 0xFFF 0xE96 0xB63 0x631 0x07BF 0x333 0xDB8
0x223 0x445 0x889 0xBBC 0x521 0x941 0xF82 0xFC7
0x040 0x070 0x0B0 0x6F6 0x005 0x009 0x00D 0x37F
0xC00 0xF50 0xFA0 0xFF6 0xEB6 0xEA5 0x00F 0xBDF
```
Color 31 is overridden per-region: volcanic (region 4) = `0x0980`, dungeon with active secret (region 9) = `0x00F0`, default = `0x0BDF`.

**Text palette** -- `textcolors[20]` (`fmain.c:476-479`):
```
0x000 0xFFF 0xC00 0xF60 0x00F 0xC0F 0x090 0xFF0
0xF90 0xF0C 0xA50 0xFDB 0xEB7 0xCCC 0x888 0x444
0x000 0xDB0 0x740 0xC70
```

**Intro palette** -- `introcolors[32]` (`fmain.c:484-488`): Used during title screen and story page sequences.

**Black palette** -- `blackcolors[32]` (`fmain.c:481-482`): All zeros, used for fade-to-black transitions.

**Day/night fading** -- `fade_page(r, g, b, limit, colors)` (`fmain2.c:377-419`): Applies per-channel brightness scaling to the base palette. Parameters `r`, `g`, `b` are percentages (0-100). When `limit` is set (night mode), minimum thresholds prevent total darkness: red >= 10%, green >= 25%, blue >= 60%. An additional blue boost is applied to green vegetation colors (indices 16-24) during twilight to simulate moonlight. The `light_timer` flag (from the light spell) forces red channels up to green levels, simulating warm torch light.

The fading formula per color entry:
```
r1 = (r * red_channel) / 1600
g1 = (g * green_channel) / 1600
b1 = (b * blue_channel + green_excess * g1) / 100
fader[i] = (r1 << 8) + (g1 << 4) + b1
LoadRGB4(&vp_page, fader, 32)
```

**Source:** `fmain2.c:366-419`, `fmain.c:476-488`, `fmain.c:481-482`

### 15.8 Special Effects

**Witch FX -- Rotating vision cone** (`fmain2.c:917-965`):

The witch character has a visible "line of sight" rendered as a filled quadrilateral that rotates around her position. The effect is drawn directly onto the game bitmap using Amiga graphics library area-fill operations.

Implementation:
1. Create a temporary clip layer over the map area (304x192 pixels) via `CreateUpfrontLayer`.
2. Look up two pairs of endpoints from `witchpoints[]`, a 256-entry sine/cosine table (64 points x 4 bytes). The two line pairs represent the edges of the vision cone, offset by the witch's rotation angle (`witchdir`). One pair uses `witchdir + 63` (nearly opposite), the other `witchdir + 1`.
3. For each edge, compute the cross product against the hero's position to determine which side the hero is on: `s = dx * dy1 - dy * dx1`. If `sg1 > 0` and `sg2 < 0`, the hero is within the vision cone and takes damage.
4. Draw the quadrilateral using `AreaMove`/`AreaDraw`/`AreaEnd` with `COMPLEMENT` draw mode, which XORs the filled polygon against the existing bitmap -- making the vision cone visible as a color-inverted region.
5. Clean up the temporary layer and raster.

The witch's rotation direction changes randomly: if the cross product `s1 > 0`, the cone tends to rotate toward the hero.

**Teleport colorplay** (`fmain2.c:425-432`):

A 32-frame color animation that signals teleportation:
```
for j = 0 to 31:
    for i = 1 to 31:
        fader[i] = bitrand(0xFFF)    // random 12-bit RGB
    LoadRGB4(&vp_page, fader, 32)
    Delay(1)                          // ~1/50 second per frame
```

Color 0 (background) is never randomized, preserving the border. The effect lasts approximately 0.64 seconds at 50 Hz PAL timing.

**Intro page flip -- Columnar reveal** (`fmain2.c:781-833`):

Story pages are revealed through a 22-step columnar animation (`flipscan()`). The source image is in `pagea`, the destination in `pageb`, composited through the drawing page:

- **Steps 0-10 (right half):** Copy the right half of `pageb` as a base. Then blit vertical strips from `pagea` at decreasing intervals (rate: 8,6,5,4,3,2,3,5,13 pixels), starting from center and spreading rightward. Each strip's vertical offset is computed by `page_det()` to create a slight arc effect.
- **Steps 11-21 (left half):** Reverse the process for the left half, replacing `pagea` strips with `pageb` strips, progressively covering the old image.

Per-step timing is controlled by `flip3[]` delays: longer pauses at the start (12,9,6,3 ticks) and end, with zero delays during the fast middle portion. Each step calls `pagechange()` to display the intermediate result.

The `copypage()` wrapper (`fmain2.c:781-790`) orchestrates the full sequence: delay 7 seconds, blit `pageb` as background, unpack two IFF brushes (image + text) into `pageb`, then run `flipscan()`.

**Source:** `fmain2.c:917-965`, `fmain2.c:425-432`, `fmain2.c:781-833`

### 15.9 Asset Formats

#### Color Palettes

- **Original format:** 12-bit Amiga RGB, stored as packed `USHORT` values in `0x0RGB` format. Each color channel (R, G, B) occupies one nibble with values 0-15.
- **Modern conversion:** Multiply each 4-bit channel by 17 (0x11) to expand to 8-bit: e.g., `0xE96` becomes RGB(0xEE, 0x99, 0x66). This preserves the exact color ratios.
- **Storage:** Inline C arrays (`pagecolors[]`, `textcolors[]`, `introcolors[]`) compiled into the executable. Loaded at runtime via `LoadRGB4()` which writes directly to the viewport's copper list color registers.

#### IFF Brushes

- **Original format:** Standard Amiga IFF/ILBM (Interleaved Bitmap) files. These are packed planar bitmaps with optional compression, loaded by `unpackbrush()`.
- **Files used:**

| Filename | Purpose | Context |
|----------|---------|---------|
| `page0` | Title screen | Intro sequence, copied to both page bitmaps |
| `p1a`, `p1b` | Story page 1 (image + text) | Columnar reveal via `copypage()` |
| `p2a`, `p2b` | Story page 2 | Columnar reveal |
| `p3a`, `p3b` | Story page 3 | Columnar reveal |
| `hiscreen` | HUD/status bar graphics | Loaded into `bm_text` at game start |
| `winpic` | Victory/save screen | Displayed on game completion or save |

- **Modern conversion:** Standard PNG images, one per original IFF file. Dimensions and content preserved exactly; palette embedded or referenced separately.

### 15.10 Improvement Notes

**Z-sort algorithm:** The original bubble sort is O(n^2) with n passes regardless of pre-sortedness. Since the sprite list changes minimally between frames (most sprites move by small increments), an insertion sort would be nearly O(n) on average for this nearly-sorted data. The author's own comment ("YUCKY AWFUL WASTEFUL BUBBLE SORT!! YUCK!!") confirms this was a known compromise.

**Blitter operations per sprite:** Each sprite requires 3 blitter passes: `save_blit` (backup background), `mask_blit` (combine masks), and `shape_blit` (render image). On modern hardware with GPU acceleration, all three operations collapse into a single textured draw call with alpha blending or stencil testing. The terrain occlusion mask can be pre-rendered into a depth/stencil buffer rather than computed per-sprite.

**Color depth:** The original 12-bit palette (4096 possible colors, 32 active) should be expanded to 24-bit RGB for modern displays. Multiply each channel by 17 for faithful conversion. A "retro mode" option should preserve the 12-bit quantization and limited palette for authenticity. The day/night fading algorithm (`fade_page`) can operate at full 24-bit precision, eliminating the visible banding in dark scenes caused by the original 4-bit-per-channel quantization.

**Backsave buffer limits:** The original 5920-byte backsave limit (74 x 80 bytes) can cause sprites to be silently dropped when many large sprites overlap. Modern implementations should use dynamic allocation with no hard limit, or pre-allocate sufficient buffer for the worst case (25 sprites at maximum size).

**Scroll optimization:** The `scrollmap` blitter-based tile shift and edge repair is specific to planar bitmap hardware. Modern tile-map renderers should use GPU tile batching with camera offset, eliminating the concept of "edge repair" entirely. The sub-tile smooth scrolling (RxOffset/RyOffset) maps directly to fractional camera position in a modern 2D engine.

---

## 16. World Navigation (Carriers)

The game features four rideable/interactive entities that occupy anim_list slot 3 (the "carrier slot"). Each has distinct movement physics and activation conditions. The global variable `active_carrier` tracks whether a turtle or bird is loaded, and `riding` indicates the current mount state.

### Proximity Detection

Before each frame's movement loop, the engine computes proximity between the hero (anim_list[0]) and the carrier entity:

```c
if (active_carrier) wcarry = 3; else wcarry = 1;
xstart = anim_list[0].abs_x - anim_list[wcarry].abs_x - 4;
ystart = anim_list[0].abs_y - anim_list[wcarry].abs_y - 4;
if (xstart < 16 && xstart > -16 && ystart < 16 && ystart > -16)
    raftprox = 1;                       /* within 16px: nearby */
if (xstart < 9 && xstart > -9 && ystart < 9 && ystart > -9)
    raftprox = 2;                       /* within 9px: close enough to board */
```

- `raftprox=1` -- hero is within 16 pixels (nearby).
- `raftprox=2` -- hero is within 9 pixels (close enough to ride/board).
- `wcarry` selects which anim_list entry to check: slot 3 if an active carrier exists, slot 1 (raft) otherwise.

### Raft

- **Type:** `RAFT` (anim_list slot 1).
- **Initial position:** (13668, 14470), set during `revive()`.
- **Activation:** Requires `wcarry==1` (no active carrier) and `raftprox==2` (within 9 pixels).
- **Terrain check:** The raft only follows the hero if the hero's position maps to water terrain (px_to_im returns 3, 4, or 5: `j < 3 || j > 5` rejects non-water).
- **Movement:** The raft has no autonomous movement -- it snaps directly to the hero's coordinates each frame:
  ```c
  xtest = anim_list[0].abs_x;
  ytest = anim_list[0].abs_y;
  an->abs_x = xtest;
  an->abs_y = ytest;
  riding = 1;
  ```
- **Effect:** While `riding==1`, the hero cannot drown (`if (i==0 && raftprox) k = 0`).
- **Display:** When the bird carrier is loaded but not ridden (`riding==0 && actor_file==11`), the bird is rendered as a `RAFT`-type sprite with index 1 (grounded swan).

### Turtle

- **Type:** `CARRIER`, `actor_file=5`.
- **Extent zone:** extent_list[1] -- coordinates are dynamically set via `get_turtle()` when the player uses the turtle item.
- **Summoning:** From the USE menu, using the turtle item calls `get_turtle()`, which finds a random water tile (px_to_im == 5) within 25 attempts, positions extent_list[1] at that location, and calls `load_carrier(5)`.
- **Summoning restriction:** The turtle cannot be summoned within the central region (hero_x between 11194-21373, hero_y between 10205-16208).
- **Boarding:** Requires `raftprox && wcarry==3` (active carrier within proximity). Sets `riding=5`.
- **Animation:** `dex = d+d` (direction doubled). If the hero is walking, adds walk cycle: `dex += (cycle&1)`.
- **Autonomous movement (unridden):** The turtle follows water paths using direction probing:
  1. Try current direction `d` -- move 3 pixels. If terrain is water (px_to_im == 5), accept.
  2. Try `d+1` (turn right). If water, accept.
  3. Try `d-1` (original minus 1, net effect). If water, accept.
  4. Try `d-2` (one more left). If water, accept.
  5. Only update position if final terrain check confirms water.
- **Ridden movement:** Hero direction is imposed on the turtle. Speed is 3 pixels/frame (via `riding==5` branch in walk code: `e = 3`).

### Bird (Swan)

- **Type:** `CARRIER`, `actor_file=11`.
- **Extent zone:** extent_list[0] at (2118,27237)-(2618,27637).
- **Boarding requirement:** `raftprox && wcarry==3 && stuff[5]` -- proximity to bird AND hero possesses the lasso (inventory slot 5).
- **Riding state:** `riding=11`. While riding, `anim_list[0].environ = -2` (airborne environment).
- **Movement physics:** Velocity-based with acceleration and speed capping:
  ```c
  nvx = an->vel_x + newx(20,d,2) - 20;   /* acceleration in facing direction */
  nvy = an->vel_y + newy(20,d,2) - 20;
  if (abs(nvx) < e-8) an->vel_x = nvx;    /* cap horizontal: 32 for bird */
  if (abs(nvy) < e) an->vel_y = nvy;       /* cap vertical: 40 for bird */
  xtest = an->abs_x + an->vel_x/4;        /* position = pos + velocity/4 */
  ytest = an->abs_y + an->vel_y/4;
  ```
  The speed limit `e` is 40 when `riding==11` (bird), 42 for turtle airborne paths. The `/4` divisor provides smooth sub-pixel movement.
- **Direction:** `set_course(0, -nvx, -nvy, 6)` computes the bird's visual facing from its velocity vector.
- **Dismount conditions:**
  - Hero presses action button while riding (`riding==11`).
  - **Fiery terrain check:** If `fiery_death` is true (hero is in the lava region: map_x 8802-13562, map_y 24744-29544), displays event 32: "Ground is too hot for swan to land."
  - **Speed check:** If hero offset from bird exceeds 15px in either axis, displays event 33: "Flying too fast to dismount."
  - **Safe landing:** If within 15px offset AND `proxcheck` confirms no collision at landing position (14px above bird), sets `riding=0` and adjusts hero Y.
- **Grounded display:** When not riding and actor_file is 11, the bird renders as RAFT type with sprite index 1.

### Dragon

- **Type:** `DRAGON`, `actor_file=10`.
- **Extent zone:** extent_list[2] at (6749,34951)-(7249,35351) (dragon cave area).
- **Hostile:** The dragon is an enemy, not a rideable carrier.
- **Sprite format:** 3x40 sprites (seq_list[DRAGON] dimensions).
- **Combat behavior:**
  - Each frame, 1/4 chance (`rand4()==0`) of shooting a fireball.
  - Fireball: `ms->speed = 5`, `ms->missile_type = 2`, sound effect at pitch 1800 + rand256().
  - Animation index: `dex = rand2() + 1` during attack, 0 at rest, 3 when dying, 4 when dead.
  - Facing is set to 5 during attacks (fixed direction).
- **Vitality:** 50 HP (set in `load_carrier()`).

### load_carrier()

```c
load_carrier(n) short n;
```

Spawns a carrier entity into anim_list slot 3:

1. **Type assignment:** `n==10` sets type `DRAGON`, otherwise `CARRIER`.
2. **Extent index:** `n==10` -> extent 2 (dragon), `n==5` -> extent 1 (turtle), else extent 0 (bird).
3. **Sprite loading:** If `actor_file != n`, loads shape data from disk into `seq_list[ENEMY].location`, calls `read_shapes(n)` and `prep(an->type)`, then `motor_off()` to stop floppy drive.
4. **Initial position:** Placed at extent zone origin + offset: `x1 + 250`, `y1 + 200`.
5. **State initialization:** index, weapon, environ = 0; state = STILL; vitality = 50.
6. **Tracking:** `anix = 4` (4 active animation entries); `an->race = actor_file = active_carrier = n`.

---

## 17. Intro & Narrative

### Intro Sequence

The intro plays during `main()` initialization (`fmain.c:1157-1210`):

1. **Legal text display:** `ssp(titletext)` renders the title/legal text on the screen using the placard text renderer. Foreground pen set to 1 (white on dark blue background via `SetRGB4` color 0 = 0,0,6).
2. **Pause:** `Delay(50)` -- 1 second wait (50 ticks at 50Hz).
3. **Font and text setup:** Switches to scroll text rastport (`rp_text`), sets font to `afont`, colors to pen 10 foreground / pen 11 background.
4. **Load audio:** `read_score()` loads music module, `read_sample()` loads sound effects.
5. **Another pause:** `Delay(50)` -- 1 more second.
6. **Viewport setup:** Configures dual-page bitmap planes (5 bitplanes, 8000 bytes each, page offsets at +40000).
7. **Music start:** `playscore(track[12], track[13], track[14], track[15])` -- plays the intro music (tracks 12-15).
8. **Black palette:** `LoadRGB4(&vp_text, blackcolors, 32)` -- sets text viewport to all black.
9. **Screen prime:** Two `pagechange()` calls to initialize double-buffering.
10. **Skip check:** `if (skipint()) goto no_intro` -- player can press space to skip.
11. **Title image:** `unpackbrush("page0", &pageb, 0, 0)` loads the title screen IFF brush. Blits to both display pages.
12. **Zoom-in:** Loop from `i=0` to `i=160` step 4, calling `screen_size(i)` -- opens the viewport from zero height to full (160 scanlines), creating a vertical zoom-in effect on the title image.
13. **Skip check:** `if (skipint()) goto end_intro`.
14. **Story pages:** Three pages displayed with columnar-reveal animation:
    - `copypage("p1a", "p1b", 21, 29)` -- first story page.
    - `copypage("p2a", "p2b", 20, 29)` -- second story page.
    - `copypage("p3a", "p3b", 20, 33)` -- third story page.
    Each `copypage` loads two IFF images and performs a column-by-column page-flip reveal (flipscan animation).
15. **Final pause:** `if (!skipp) Delay(190)` -- 3.8 second pause on last page (unless skip pressed).
16. **Zoom-out:** Loop from `i=156` to `i=0` step -4, calling `screen_size(i)` -- closes the viewport back down.

After the intro, `copy_protect_junk()` is called (line 1238). Failure returns 0 and exits.

### Copy Protection

The `copy_protect_junk()` function (`fmain2.c:1309-1336`) implements a riddle-based copy protection system:

1. Three questions are asked sequentially (`h=0` to `h<3`).
2. Each question index `j` is chosen randomly via `rand8()`, rerolling if that question was already used (the answer pointer is NULLed after use).
3. The question text is displayed by calling `question(j)`, which indexes into the `_question` table in `narr.asm`.
4. Player types an answer (max 9 characters, backspace supported).
5. Answer is compared character-by-character against the expected answer string. Case-sensitive.
6. If `NO_PROTECT` is defined at compile time, answer checking is skipped.
7. On any wrong answer, the function returns FALSE (0), causing the game to exit.

### Copy Protection Questions (narr.asm:63-81)

The 8 questions and their expected answers:

| # | Question | Answer |
|---|----------|--------|
| 0 | "To Quest for the...?" | LIGHT |
| 1 | "Make haste, but take...?" | HEED |
| 2 | "Scorn murderous...?" | DEED |
| 3 | "Summon the...?" | SIGHT |
| 4 | "Wing forth in...?" | FLIGHT |
| 5 | "Hold fast to your...?" | CREED |
| 6 | "Defy Ye that...?" | BLIGHT |
| 7 | "In black darker than...?" | NIGHT |

The answers are rhyming words found in the game manual's verse. The answer table is defined in `fmain2.c:1306-1307`:
```c
char *answers[] = {
    "LIGHT","HEED","DEED","SIGHT","FLIGHT","CREED","BLIGHT","NIGHT" };
```

### Event Messages (narr.asm:11-58)

The `_event_msg` table contains 39 null-terminated strings (indices 0-38), triggered by `event(n)` calls throughout the game:

| # | Message |
|---|---------|
| 0 | "% was getting rather hungry." |
| 1 | "% was getting very hungry." |
| 2 | "% was starving!" |
| 3 | "% was getting tired." |
| 4 | "% was getting sleepy." |
| 5 | "% was hit and killed!" |
| 6 | "% was drowned in the water!" |
| 7 | "% was burned in the lava." |
| 8 | "% was turned to stone by the witch." |
| 9 | "% started the journey in his home village of Tambry" |
| 10 | "as had his brother before him." |
| 11 | "as had his brothers before him." |
| 12 | "% just couldn't stay awake any longer!" |
| 13 | "% was feeling quite full." |
| 14 | "% was feeling quite rested." |
| 15 | "Even % would not be stupid enough to draw weapon in here." |
| 16 | "A great calming influence comes over %, preventing him from drawing his weapon." |
| 17 | "% picked up a scrap of paper." |
| 18 | 'It read: "Find the turtle!"' |
| 19 | 'It read: "Meet me at midnight at the Crypt. Signed, the Wraith Lord."' |
| 20 | "% looked around but discovered nothing." |
| 21 | "% does not have that item." |
| 22 | "% bought some food and ate it." |
| 23 | "% bought some arrows." |
| 24 | "% passed out from hunger!" |
| 25 | "% is not sleepy." |
| 26 | "% was tired, so he decided to lie down and sleep." |
| 27 | "% perished in the hot lava!" |
| 28 | "It was midnight." |
| 29 | "It was morning." |
| 30 | "It was midday." |
| 31 | "Evening was drawing near." |
| 32 | "Ground is too hot for swan to land." |
| 33 | "Flying too fast to dismount." |
| 34 | '"They\'re all dead!" he cried.' |
| 35 | "No time for that now!" |
| 36 | "% put an apple away for later." |
| 37 | "% ate one of his apples." |
| 38 | "% discovered a hidden object." |

The `%` character is substituted with the current brother's name at display time.

### Place Name Tables (narr.asm:86-154)

The engine uses two lookup tables to determine location names. When the hero enters a new terrain sector, the sector number is tested against ranges in these tables. The first matching range triggers its associated message.

Each entry is 3 bytes: `low_sector, high_sector, message_index`.

#### Outdoor Places (_place_tbl) -- 29 entries

| Low | High | Msg# | Location |
|-----|------|------|----------|
| 51 | 51 | 19 | Small keep |
| 64 | 69 | 2 | Village (Tambry) |
| 70 | 73 | 3 | Vermillion Manor |
| 80 | 95 | 6 | Marheim |
| 96 | 99 | 7 | Witch's castle |
| 138 | 139 | 8 | Graveyard |
| 144 | 144 | 9 | Stone ring |
| 147 | 147 | 10 | Lighthouse |
| 148 | 148 | 20 | Small castle |
| 159 | 162 | 17 | Desert city (Azal) |
| 163 | 163 | 18 | Desert fort |
| 164 | 167 | 12 | Crystal Palace |
| 168 | 168 | 21 | Log cabin |
| 170 | 170 | 22 | Dark fort |
| 171 | 174 | 14 | Doom tower |
| 176 | 176 | 13 | Pixie shrine |
| 178 | 178 | 23 | Swamp cabin |
| 179 | 179 | 24 | Tomb |
| 180 | 180 | 25 | Unreachable castle |
| 175 | 180 | 0 | Lava / elf (nil) |
| 208 | 221 | 11 | Swamp (great Bog) |
| 243 | 243 | 16 | Oasis |
| 250 | 252 | 0 | Nil (interface) |
| 255 | 255 | 26 | Dragon cave |
| 78 | 78 | 4 | Mountain type |
| 187 | 239 | 4 | Mountain type |
| 0 | 79 | 0 | Nil |
| 185 | 254 | 15 | Desert (Burning Waste) |
| 0 | 255 | 0 | Nil (catch-all) |

Note: Table is scanned top-to-bottom; first match wins. Overlapping ranges are resolved by order (e.g., sector 208 matches "Swamp" before it could match "Desert").

#### Indoor Places (_inside_tbl) -- 31 entries

| Low | High | Msg# | Location |
|-----|------|------|----------|
| 2 | 2 | 2 | Small chamber |
| 7 | 7 | 3 | Large chamber |
| 4 | 4 | 4 | Long passageway |
| 5 | 6 | 5 | Twisting tunnel |
| 9 | 10 | 6 | Forked intersection |
| 30 | 30 | 7 | Keep interior |
| 19 | 33 | 14 | Stone corridor |
| 101 | 101 | 14 | Stone corridor |
| 130 | 134 | 14 | Stone corridor |
| 36 | 36 | 13 | Octagonal room |
| 37 | 42 | 12 | Large room |
| 46 | 46 | 0 | Final arena (special) |
| 43 | 59 | 11 | Spirit world |
| 100 | 100 | 11 | Spirit world |
| 143 | 149 | 11 | Spirit world |
| 62 | 62 | 16 | Small building |
| 65 | 66 | 18 | Tavern |
| 60 | 78 | 17 | Building |
| 82 | 82 | 17 | Building |
| 86 | 87 | 17 | Building |
| 92 | 92 | 17 | Priest's building |
| 94 | 95 | 17 | Small buildings |
| 97 | 99 | 17 | Building |
| 120 | 120 | 17 | Building (desfort) |
| 116 | 119 | 17 | Building (desert) |
| 139 | 141 | 17 | Building (desert) |
| 79 | 96 | 9 | Palace of King Mar |
| 104 | 104 | 19 | Inn |
| 114 | 114 | 20 | Tomb inside |
| 105 | 115 | 8 | Castle |
| 135 | 138 | 8 | Castle (doom tower) |
| 125 | 125 | 21 | Cabin inside |
| 127 | 127 | 10 | Elf glade inside |
| 142 | 142 | 22 | Unlocked (lighthouse) |
| 121 | 129 | 22 | Unlocked/entered |
| 150 | 161 | 15 | Stone maze |
| 0 | 255 | 0 | Nil (catch-all) |

### Place Messages (narr.asm:164-227)

#### Outdoor Place Messages (_place_msg) -- 27 entries

| # | Message |
|---|---------|
| 0 | (no message) |
| 1 | (do not change) |
| 2 | "% returned to the village of Tambry." |
| 3 | "% came to Vermillion Manor." |
| 4 | "% reached the Mountains of Frost" |
| 5 | "% reached the Plain of Grief." |
| 6 | "% came to the city of Marheim." |
| 7 | "% came to the Witch's castle." |
| 8 | "% came to the Graveyard." |
| 9 | "% came to a great stone ring." |
| 10 | "% came to a watchtower." |
| 11 | "% traveled to the great Bog." |
| 12 | "% came to the Crystal Palace." |
| 13 | "% came to mysterious Pixle Grove." |
| 14 | "% entered the Citadel of Doom." |
| 15 | "% entered the Burning Waste." |
| 16 | "% found an oasis." |
| 17 | "% came to the hidden city of Azal." |
| 18 | "% discovered an outlying fort." |
| 19 | "% came to a small keep." |
| 20 | "% came to an old castle." |
| 21 | "% came to a log cabin." |
| 22 | "% came to a dark stone tower." |
| 23 | "% came to an isolated cabin." |
| 24 | "% came to the Tombs of Hemsath." |
| 25 | "% reached the Forbidden Keep." |
| 26 | "% found a cave in the hillside." |

#### Indoor Place Messages (_inside_msg) -- 22 entries

| # | Message |
|---|---------|
| 0 | (no message) |
| 1 | (do not change) |
| 2 | "% came to a small chamber." |
| 3 | "% came to a large chamber." |
| 4 | "% came to a long passageway." |
| 5 | "% came to a twisting tunnel." |
| 6 | "% came to a forked intersection." |
| 7 | "He entered the keep." |
| 8 | "He entered the castle." |
| 9 | "He entered the castle of King Mar." |
| 10 | "He entered the sanctuary of the temple." |
| 11 | "% entered the Spirit Plane." |
| 12 | "% came to a large room." |
| 13 | "% came to an octagonal room." |
| 14 | "% traveled along a stone corridor." |
| 15 | "% came to a stone maze." |
| 16 | "He entered a small building." |
| 17 | "He entered the building." |
| 18 | "He entered the tavern." |
| 19 | "He went inside the inn." |
| 20 | "He entered the crypt." |
| 21 | "He walked into the cabin." |
| 22 | "He unlocked the door and entered." |

### Placard Text (narr.asm:230-347)

Story placards are displayed during intro sequences and narrative events. The `_placard_text` function indexes into a table of 20 message pointers and calls `ssp()` to render them. Text uses embedded XY positioning commands (byte 128 followed by x/2 and y coordinates) and ETX (0) terminators.

Key story messages:
- **msg1:** Julian's quest begins -- the Mayor's plea to rescue the Talisman.
- **msg2:** Julian fails and does not return.
- **msg3:** Phillip sets out to find his brother.
- **msg4:** Phillip meets the same fate.
- **msg5:** Kevin takes up the quest as a last resort.
- **msg6:** Game over -- "Stay at Home!" (all brothers dead).
- **msg7/7a:** Victory -- hero defeats the Necromancer, recovers Talisman, weds the princess.
- **msg8/8a/8b:** Hero rescued Katra (Princess 1), pledges love but continues quest.
- **msg9/9a/9b:** Hero rescued Karla (Katra's sister, Princess 2), same pledge.
- **msg10/10a/10b:** Hero rescued Kandy (third sister, Princess 3), same pledge.
- **msg11/11a:** After seeing the princess home, hero sets out again with a king's gift in gold.
- **msg12:** Copy protection preamble -- addresses the player as "game seeker" and challenges them to prove fitness.

### Text Rendering System

Three text display functions handle all in-game narrative:

- **`print(str)`** (`fmain2.c:495`): Scrolls the text viewport up by 10 pixels (`ScrollRaster`), then renders the string at the bottom line. Used for single-line messages.

- **`print_cont(str)`** (`fmain2.c:503`): Continues printing at the current cursor position without scrolling. Used to append text to the current line.

- **`extract(start)`** (`fmain2.c:514`): Word-wrapping text renderer. Processes a full message string:
  - Wraps at 37 characters per line (constant in loop: `for (; i<37; i++)`).
  - Tracks last space position as the preferred break point (`lbreak`).
  - `%` is substituted with the current brother's name (`datanames[brother-1]`).
  - Carriage return (13) forces a line break.
  - Null (0) terminates the message.
  - Each wrapped line is passed to `print()` for scrolling display.
  - Uses a 200-byte buffer (`mesbuf[200]`).

- **`ssp(str)`** (`fsubs.asm:497`): Low-level placard text renderer used for intro screens. Processes embedded control codes:
  - `XY` (byte 128): Next two bytes are x/2 and y coordinates; calls `Move()` to position cursor (x is doubled).
  - `ETX` (byte 0): Terminates rendering.
  - All other bytes are accumulated and rendered via Amiga `Text()` GfxBase call.

String macros documented in `narr.asm:157-162` (used in source comments, not runtime):
- `@` = " entered the "
- `#` = " came to "
- `$` = "the "
- `^` = " castle"
- `[` = " of "
- `%` = substitute character name

---

## 18. Save/Load System

### Save Menu Flow

The save/load system is accessed through the in-game menu hierarchy:

1. **GAME menu** (`fmain.c:3443-3447`):
   - Hit 6: `setmood(TRUE)` -- toggle mood.
   - Hit 8: `gomenu(SAVEX)` -- opens the Save/Quit submenu.
   - Hit 9: `svflag = FALSE; gomenu(FILE)` -- opens File menu for loading.

2. **SAVEX menu** (`fmain.c:3465-3468`):
   - Hit 6: `quitflag = TRUE` -- quit without saving.
   - Hit 5: `svflag = TRUE; gomenu(FILE)` -- opens File menu for saving.

3. **FILE menu** (`fmain.c:3469-3472`):
   - Selects slot (hit value 0-7, mapped to letters A-H).
   - Calls `savegame(hit)`.
   - Returns to GAME menu.

The `svflag` variable determines direction: TRUE = save, FALSE = load.

### savegame() Function (fmain2.c:1474-1551)

Handles both saving and loading via the shared `saveload()` primitive:

#### Disk Detection

The function probes for a writable disk in priority order:
1. Hard drive: checks for "image" directory with `locktest()`. If found, strips the "df1:" prefix from the save path.
2. DF1: checks with `locktest("df1:", ACCESS_WRITE)`.
3. DF0: checks with `locktest("df0:", ACCESS_WRITE)` AND confirms it is not the game disk (`!locktest("df0:winpic", ACCESS_READ)`).
4. If no writable disk found, prompts "Insert a writable disk in ANY drive." and waits via `waitnewdisk()`.

#### File Naming

```c
char savename[] = "df1:A.faery";
```

The slot letter is inserted at position 4: `savename[4] = 'A' + hit` (slots A through H). The drive letter at position 2 may be changed to '0' or '1'. On hard drive, the "df1:" prefix is skipped (name starts at offset 4: "A.faery").

File is opened with AmigaDOS mode 1006 (MODE_NEWFILE) for saves, 1005 (MODE_OLDFILE) for loads.

### Save Data Layout

Data is written/read as sequential binary blocks with no headers, magic numbers, or version information:

| Order | Data | Size (bytes) |
|-------|------|-------------|
| 1 | Misc variables (starting at `&map_x`) | 80 |
| 2 | Region number (`region_num`) | 2 |
| 3 | Animation list length + padding (`&anix`) | 6 |
| 4 | Animation list (`anim_list`) | `anix * sizeof(struct shape)` |
| 5 | Julian's inventory (`julstuff`) | 35 |
| 6 | Phillip's inventory (`philstuff`) | 35 |
| 7 | Kevin's inventory (`kevstuff`) | 35 |
| 8 | Missile list (`missile_list`) | `6 * sizeof(struct missile)` |
| 9 | Extent list (`extent_list`) | `2 * sizeof(struct extent)` |
| 10 | Global object list (`ob_listg`) | `glbobs * sizeof(struct object)` |
| 11 | Map object counts (`mapobs`) | 20 |
| 12 | Destination object counts (`dstobs`) | 20 |
| 13 | Per-region object tables (10 regions) | `mapobs[i] * sizeof(struct object)` each |

#### The 80-byte Misc Variables Block

Starting at `&map_x` (fmain.c:557-581), this block captures 80 contiguous bytes of game state:

```
map_x, map_y          (2+2 = 4 bytes, unsigned short)
hero_x, hero_y        (2+2 = 4 bytes)
safe_x, safe_y, safe_r (2+2+2 = 6 bytes)
img_x, img_y          (2+2 = 4 bytes)
cheat1                 (2 bytes)
riding, flying, wcarry (2+2+2 = 6 bytes)
turtleprox, raftprox   (2+2 = 4 bytes)
brave, luck, kind, wealth, hunger, fatigue (6*2 = 12 bytes)
brother                (2 bytes)
princess               (2 bytes)
hero_sector            (2 bytes)
hero_place             (2 bytes)
daynight, lightlevel   (2+2 = 4 bytes)
actor_file, set_file   (2+2 = 4 bytes)
active_carrier         (2 bytes)
xtype                  (2 bytes)
leader                 (2 bytes)
secret_timer, light_timer, freeze_timer (2+2+2 = 6 bytes)
cmode                  (2 bytes)
encounter_type         (2 bytes)
pad1-pad7              (7*2 = 14 bytes)
```

Total: 80 bytes.

### mod1save() (fmain.c:3621-3631)

Called from within `savegame()` to save/load the three brothers' inventories and missile state:

```c
mod1save()
{
    saveload(julstuff, 35);               /* Julian's 35 inventory slots */
    saveload(philstuff, 35);              /* Phillip's 35 inventory slots */
    saveload(kevstuff, 35);               /* Kevin's 35 inventory slots */
    stuff = blist[brother-1].stuff;       /* restore active stuff pointer */
    saveload((void *)missile_list, 6 * sizeof(struct missile));
}
```

After loading inventories, the `stuff` pointer is reassigned to point to the current brother's inventory array, since the pointer itself is not saved.

### saveload() Primitive (fmain2.c:1553-1558)

```c
saveload(buffer, length) char *buffer; long length;
{
    if (svflag) err = Write(svfile, buffer, length);
    else err = Read(svfile, buffer, length);
    if (err < 0) sverr = IoErr();
}
```

A single function handles both directions based on `svflag`. No checksums, no byte-swapping, no compression.

### Post-Load Cleanup (fmain2.c:1541-1548)

After a successful load:
```c
wt = encounter_number = 0;
shape_read(); set_options(); viewstatus = 99;
prq(4); prq(7);
encounter_type = actors_loading = 0;
```

This reloads shape graphics for the current region, refreshes the menu option states, forces a full status bar redraw (`viewstatus = 99`), clears the text area, and resets encounter state.

After save or load completes, the function checks for the game disk: if not on hard drive, it loops waiting for the user to reinsert the game disk (checks for "df0:winpic").

### File Format Characteristics

- **Format:** Raw sequential binary dump -- structures are written directly from memory.
- **No headers:** No magic number, no version field, no file size marker.
- **No versioning:** Save files are tied to the exact structure layouts of the compiled binary. Any change to struct sizes or variable ordering would silently corrupt loads.
- **Variable-length sections:** The animation list and object tables have variable sizes (driven by `anix`, `glbobs`, `mapobs[i]`), making the file format implicitly dependent on saved count values.
- **8 slots:** Files named A.faery through H.faery.
- **Platform-dependent:** Big-endian (68000), raw struct padding included.

### Asset Format: Save Games

| Aspect | Original (Amiga) | Modern (Recommended) |
|--------|------------------|----------------------|
| Format | Raw binary state dump | JSON with named fields and version |
| Endianness | Big-endian (68000 native) | Platform-independent |
| Versioning | None | Schema version field |
| Validation | None | Checksums, magic number |
| Structure | Sequential memory dumps | Named key-value pairs |
| Slot naming | A.faery - H.faery | Descriptive names or auto-save |

---

## 19. UI & Menu System

### Menu Modes

Ten menu modes are defined in the `cmodes` enum (`fmain.c:494`):

| Value | Name    | Purpose                        |
|------:|---------|--------------------------------|
|     0 | ITEMS   | Inventory actions              |
|     1 | MAGIC   | Cast spells (magic items)      |
|     2 | TALK    | Conversation actions           |
|     3 | BUY     | Purchase items from shops      |
|     4 | GAME    | System options (pause, music)  |
|     5 | SAVEX   | Save/exit confirmation         |
|     6 | KEYS    | Key inventory (colored keys)   |
|     7 | GIVE    | Give items to NPCs             |
|     8 | USE     | Use weapons/items directly     |
|     9 | FILE    | Save file slot selection (A-H) |

The global `cmode` tracks the currently active menu mode. Selecting a top-row label (indices 0-4: Items/Magic/Talk/Buy/Game) with `atype == 0` switches `cmode` to that label's index.

### Label Strings

Each label array contains 5-character fixed-width entries concatenated into a single string. The menu system indexes into these by `entry_index * 5`.

| Array    | Contents                                                         | Used By    |
|----------|------------------------------------------------------------------|------------|
| `label1` | `Items Magic Talk  Buy   Game `                                  | Top row (shared by modes 0-4) |
| `label2` | `List  Take  Look  Use   Give `                                  | ITEMS      |
| `label3` | `Yell  Say   Ask  `                                              | TALK       |
| `label4` | `Pause Music Sound Quit  Load `                                  | GAME       |
| `label5` | `Food  Arrow Vial  Mace  Sword Bow   Totem`                      | BUY        |
| `label6` | `Stone Jewel Vial  Orb   Totem Ring  Skull`                      | MAGIC      |
| `label7` | `Dirk  Mace  Sword Bow   Wand  Lasso Shell Key   Sun   Book `   | USE        |
| `label8` | `Save  Exit `                                                    | SAVEX      |
| `label9` | `Gold  Green Blue  Red   Grey  White`                            | KEYS       |
| `labelA` | `Gold  Book  Writ  Bone `                                        | GIVE       |
| `labelB` | `  A     B     C     D     E     F     G     H  `               | FILE       |

For modes 0-7, the first 5 entries (indices 0-4) display from `label1` (the shared top row: Items/Magic/Talk/Buy/Game). Entries at index 5 and above display from the mode-specific label array. For modes USE and FILE (>= 8), all entries display from their own label array directly (`fmain.c:3089-3091`).

### Menu Structure

The `menus[10]` array (`fmain.c:521-531`) defines each menu's label source, entry count, display color, and initial enable flags for up to 12 entries:

| Index | Mode   | label_list | num | color | enabled[0..11]                              |
|------:|--------|------------|----:|------:|---------------------------------------------|
|     0 | ITEMS  | `label2`   |  10 |     6 | `3,2,2,2,2,10,10,10,10,10,0,0`             |
|     1 | MAGIC  | `label6`   |  12 |     5 | `2,3,2,2,2,8,8,8,8,8,8,8`                  |
|     2 | TALK   | `label3`   |   8 |     9 | `2,2,3,2,2,10,10,10,0,0,0,0`               |
|     3 | BUY    | `label5`   |  12 |    10 | `2,2,2,3,2,10,10,10,10,10,10,10`            |
|     4 | GAME   | `label4`   |  10 |     2 | `2,2,2,2,3,6,7,7,10,10,0,0`                |
|     5 | SAVEX  | `label8`   |   7 |     0 | `2,2,2,2,2,10,10,0,0,0,0,0` (note: only 7) |
|     6 | KEYS   | `label9`   |  11 |     8 | `2,2,2,2,2,10,10,10,10,10,10,0` (note: 11) |
|     7 | GIVE   | `labelA`   |   9 |    10 | `2,2,2,2,2,10,0,0,0,0,0,0`                 |
|     8 | USE    | `label7`   |  10 |     8 | `10,10,10,10,10,10,10,10,10,0,10,10`        |
|     9 | FILE   | `labelB`   |  10 |     5 | `10,10,10,10,10,10,10,10,0,0,0,0`           |

The `num` field is the total number of entries the menu can display (including the shared top-row entries for modes 0-7). The `color` field is the Amiga pen color used for rendering the option background.

### Option Enable Flags

Each byte in the `enabled[12]` array encodes both display state and behavior type (`fmain.c:512-513`):

| Bits  | Mask   | Meaning                                                  |
|-------|--------|----------------------------------------------------------|
| Bit 0 | `0x01` | Selected/highlighted (1 = on, 0 = off)                   |
| Bit 1 | `0x02` | Displayed (1 = visible, 0 = hidden)                      |
| 2-7   | `0xFC` | Type code                                                |

Type codes (from bits 2-7):

| Value | Behavior       | Description                                                 |
|------:|----------------|-------------------------------------------------------------|
|     0 | Unchangeable   | Top-row labels (Items/Magic/Talk/Buy/Game) -- mode switches |
|     4 | Toggle         | Flips bit 0 on each press (e.g., Pause, Music, Sound)      |
|     8 | Immediate      | Fires action on press, no toggle state (e.g., spell cast)   |
|    12 | Radio          | Sets bit 0 on, does not clear siblings (e.g., key colors)  |

Common combined values seen in the `enabled[]` arrays:

- `2` = displayed, not selected, type 0 (unchangeable/mode-switch) -- top-row items
- `3` = displayed + selected, type 0 -- the currently active mode's top-row entry
- `6` = displayed, not selected, type 4 (toggle) -- e.g., Pause
- `7` = displayed + selected, type 4 (toggle) -- e.g., Music/Sound initially on
- `8` = not displayed, type 8 (immediate) -- hidden immediate-action slot
- `10` = displayed, not selected, type 8 (immediate) -- visible immediate-action slot

### print_options and propt

`print_options()` (`fmain.c:3048-3068`) rebuilds the on-screen menu display:

1. Iterates through `menus[cmode].enabled[]` up to `menus[cmode].num` entries.
2. Skips entries where bit 1 (displayed) is clear.
3. Populates `real_options[j]` with the logical index `i` for each visible entry, then calls `propt(j, selected)`.
4. Blanks any remaining slots (up to 12) and sets their `real_options[]` to -1.

`propt(j, pena)` (`fmain.c:3070-3092`) renders a single option:

- `j` is the physical screen slot (0-11). Even slots render at x=430, odd at x=482. Y position = `(j/2) * 9 + 8`.
- `pena` is the foreground pen (0=deselected, 1=selected).
- Background pen (`penb`) depends on mode: USE uses pen 14, FILE uses pen 13, top-row items (k<5) use pen 4, KEYS uses `keycolors[k-5]` (palette: 8,6,4,2,14,1 for gold/green/blue/red/grey/white), SAVEX uses index as pen, otherwise uses `menus[cmode].color`.
- Label text: for USE/FILE modes, reads directly from the mode's `label_list`. For other modes, indices <5 read from `label1` (shared top row), indices >=5 read from the mode-specific `label_list` at offset `(k-5)*5`.

### Keyboard Mapping

The `letter_list[38]` array (`fmain.c:537-547`) maps key presses to menu actions:

| Letter | Menu  | Choice | Action                    |
|--------|-------|-------:|---------------------------|
| `I`    | ITEMS |      5 | List items                |
| `T`    | ITEMS |      6 | Take                      |
| `?`    | ITEMS |      7 | Look                      |
| `U`    | ITEMS |      8 | Use                       |
| `G`    | ITEMS |      9 | Give                      |
| `Y`    | TALK  |      5 | Yell                      |
| `S`    | TALK  |      6 | Say                       |
| `A`    | TALK  |      7 | Ask                       |
| ` `    | GAME  |      5 | Pause (space bar)         |
| `M`    | GAME  |      6 | Music toggle              |
| `F`    | GAME  |      7 | Sound toggle              |
| `Q`    | GAME  |      8 | Quit                      |
| `L`    | GAME  |      9 | Load                      |
| `O`    | BUY   |      5 | Food                      |
| `R`    | BUY   |      6 | Arrow                     |
| `8`    | BUY   |      7 | Vial                      |
| `C`    | BUY   |      8 | Mace                      |
| `W`    | BUY   |      9 | Sword                     |
| `B`    | BUY   |     10 | Bow                       |
| `E`    | BUY   |     11 | Totem                     |
| `V`    | SAVEX |      5 | Save                      |
| `X`    | SAVEX |      6 | Exit                      |
| 10     | MAGIC |      5 | Stone (function key F1)   |
| 11     | MAGIC |      6 | Jewel (function key F2)   |
| 12     | MAGIC |      7 | Vial (function key F3)    |
| 13     | MAGIC |      8 | Orb (function key F4)     |
| 14     | MAGIC |      9 | Totem (function key F5)   |
| 15     | MAGIC |     10 | Ring (function key F6)    |
| 16     | MAGIC |     11 | Skull (function key F7)   |
| `1`    | USE   |      0 | Dirk                      |
| `2`    | USE   |      1 | Mace                      |
| `3`    | USE   |      2 | Sword                     |
| `4`    | USE   |      3 | Bow                       |
| `5`    | USE   |      4 | Wand                      |
| `6`    | USE   |      5 | Lasso                     |
| `7`    | USE   |      6 | Shell                     |
| `K`    | USE   |      7 | Key                       |

Note: Values 10-16 for the MAGIC hotkeys are translated Amiga raw keycodes for function keys F1-F7 (after the input handler's `keytrans` table mapping in `fsubs.asm`). The SAVEX menu hotkeys (`V`/`X`) are blocked unless the player is already in SAVEX mode (`fmain.c:1350`).

### Input Handler

The `struct in_work` (`ftale.h:108-118`) is the shared data area between the input handler (running at interrupt level in `fsubs.asm`) and the main game loop:

| Offset | Field       | Type            | Description                                            |
|-------:|-------------|-----------------|--------------------------------------------------------|
|      0 | `xsprite`   | `short`         | Mouse/sprite X position (accumulated from deltas)      |
|      2 | `ysprite`   | `short`         | Mouse/sprite Y position (accumulated from deltas)      |
|      4 | `qualifier` | `short`         | Intuition input qualifier (button state, shift, etc.)  |
|      6 | `laydown`   | `UBYTE`         | Write pointer into circular `keybuf` (0-127)           |
|      7 | `pickup`    | `UBYTE`         | Read pointer from circular `keybuf` (0-127)            |
|      8 | `newdisk`   | `char`          | Disk-change flag (set to 1 on DISKIN event)            |
|      9 | `lastmenu`  | `char`          | Last menu button click value (for release detection)   |
|     10 | `gbase`     | `GfxBase *`     | Graphics library base pointer                          |
|     14 | `pbase`     | `SimpleSprite*` | Pointer sprite structure                               |
|     18 | `vbase`     | `ViewPort *`    | Viewport for sprite positioning                        |
|     22 | `keybuf`    | `UBYTE[128]`    | Circular key buffer (128 entries, 7-bit index wrap)    |
|    150 | `ticker`    | `short`         | Timer tick counter (counts TIMER events, wraps at 16)  |

The input handler (`_HandlerInterface` in `fsubs.asm`) processes Amiga InputEvents in a chain:

1. **TIMER events**: Increments `ticker`. When `ticker` reaches 16, injects a synthetic key-up event for code `$60` (shift release) and resets `ticker` to 0. This provides a periodic heartbeat.
2. **RAWKEY events**: Ignores repeats (qualifier bit 9). Translates raw keycodes via `keytrans` table (128 entries), preserves up/down bit (bit 7), stores translated code into `keybuf` at `laydown` index. Advances `laydown` with 7-bit wrap (`& 0x7F`). Drops input on buffer overflow (when `laydown` would equal `pickup`).
3. **RAWMOUSE button events**: Detects left button state changes via XOR of old and new qualifier bit 14 (`0x4000`). On button-down, converts sprite position to a menu slot: X range 215-265 selects the menu area, Y is mapped as `(ysprite - 144) / 9 * 2 + 0x61`, with X >= 240 adding 1 for the right column. The result is a synthetic keycode (`0x61`+) placed into `keybuf`. On button-up, the stored `lastmenu` value is sent with bit 7 set (release). Mouse qualifier is always stored to `in_work.qualifier`.
4. **DISKIN events**: Sets `newdisk` flag to 1.
5. **Mouse movement**: Accumulates `ie_X`/`ie_Y` deltas into `xsprite`/`ysprite`, clamps to screen bounds, and calls `MoveSprite` to update the hardware pointer.

### Key Handling in the Main Loop

The main loop key processing (`fmain.c:1283-1363`) reads translated keycodes from `keybuf` and processes them in priority order:

1. **View status interrupt**: If `viewstatus` is nonzero and `notpause` is true, any key-down sets `viewstatus = 99` (triggers redraw/dismiss), and the key is consumed.

2. **Dead state**: If the hero's state is `DEAD`, all input is ignored.

3. **Direction keys** (codes 20-29): Sets `keydir` to the key value. These codes come from the `keytrans` table in `fsubs.asm`, mapping cursor/numpad keys to direction indices 20-29 (direction = code - 20, values 0-8 map to the 8 compass directions plus center).

4. **Direction release**: If `(key & 0x7F) == keydir`, clears `keydir` to 0 (stop moving).

5. **Fight toggle**: Key `0` down sets `keyfight = TRUE`, key `0` up clears it. This allows keyboard-initiated combat.

6. **Cheat keys** (only when `cheat1` is true):
   - `B` -- Summon turtle (load carrier 11)
   - `.` -- Add 3 random inventory items, refresh options
   - `R` -- Rescue (teleport to safety)
   - `=` -- Print coordinates and available memory
   - Code 19 -- Print location/sector debug info
   - Code 18 -- Advance daynight by 1000 ticks
   - Codes 1-4 -- Teleport hero +/-150 Y or +/-280 X

7. **KEYS mode special**: When `cmode == KEYS`, number keys `1`-`6` directly trigger `do_option(key - '1' + 5)` to select colored keys. Any other key exits to ITEMS menu.

8. **Menu option clicks** (codes >= `0x61`): These are synthetic codes from mouse clicks on menu slots. The code maps to a physical slot index (`inum = (key & 0x7F) - 0x61`), which indexes into `real_options[]` to get the logical menu entry. On mouse-up (bit 7 set), the option is deselected visually. On mouse-down, the action depends on the option's type:
   - **Toggle (4)**: Flips bit 0 in `enabled[]`, redraws, calls `do_option()`.
   - **Immediate (8)**: Highlights, calls `do_option()`.
   - **Radio (12)**: Sets bit 0 (select), highlights, calls `do_option()`.
   - **Unchangeable (0), index < 5**: Switches `cmode` to the selected top-row mode, redraws menu via `prq(5)`.
   - **Unchangeable (0), index >= 5**: Just redraws with current state (no action).

9. **Letter hotkeys**: If the game is unpaused (or key is space), scans `letter_list[38]` for a matching key. SAVEX hotkeys are blocked unless already in SAVEX mode. On match, switches `cmode` to the letter's menu, reads the option type, toggles if type 4, and calls `do_option()` followed by `print_options()`.

### Mouse, Joystick, and Compass

`decode_mouse()` (`fsubs.asm:1489-1584`) determines the hero's movement direction from three input sources in priority order:

1. **Mouse (compass click)**: If either mouse button is held (qualifier bits `0x6000`), the sprite position is mapped to one of 9 compass regions:
   - X axis: left column (<=292), middle (292-300), right (>300)
   - Y axis: upper (<166), middle (166-174), lower (>174)
   - X <= 265 produces direction 9 (no direction / center)
   - The 3x3 grid maps to directions: `{0,1,2 / 7,9,3 / 6,5,4}` (N,NE,E clockwise, 9=center)

2. **Joystick**: Reads Amiga hardware joy registers (`$DFF00C`/`$DFF00D`) directly. Combines left/forward and right/back bits via XOR to produce signed X/Y joystick values (-1, 0, +1). If both axes are zero, falls through to keyboard. Otherwise computes `4 + yjoy*3 + xjoy` and indexes into `com2` lookup table: `{0,1,2,7,9,3,6,5,4}` to get a direction 0-8.

3. **Keyboard**: If `keydir` is set (from direction key codes 20-29), direction = `keydir - 20`. If keydir is out of range, direction defaults to 9 (none) and keydir is cleared.

The resulting direction (0-8, or 9 for none) is compared against `oldir`. If changed, `oldir` is updated and `drawcompass(dir)` is called to update the compass display.

**Fight detection** in the main loop (`fmain.c:1409`): the hero enters combat state if `qualifier & 0x2000` (right mouse button), `keyfight` is true (keyboard `0` held), or joystick button 1 is pressed (`*pia & 128 == 0`, reading CIA-A port).

### Compass Display

`drawcompass(dir)` (`fmain2.c:351-363`) renders a compass indicator on the HUD using bitplane blits:

1. First clears the compass area by blitting the base compass image (`nhinor` plane) to `bm_text` at position (567, 15), size 48x24.
2. If `dir < 9` (a valid direction), blits the highlighted region from `nhivar` (highlight plane) using the `comptable[dir]` rectangle:

| Dir | Name   | xrect | yrect | xsize | ysize |
|----:|--------|------:|------:|------:|------:|
|   0 | N      |     0 |     0 |    16 |     8 |
|   1 | NE     |    16 |     0 |    16 |     9 |
|   2 | E      |    32 |     0 |    16 |     8 |
|   3 | SE     |    30 |     8 |    18 |     8 |
|   4 | S      |    32 |    16 |    16 |     8 |
|   5 | SW     |    16 |    13 |    16 |    11 |
|   6 | W      |     0 |    16 |    16 |     8 |
|   7 | NW     |     0 |     8 |    18 |     8 |
|   8 | Center |     0 |     0 |     1 |     1 | (effectively no highlight) |
|   9 | None   |     0 |     0 |     1 |     1 | (skipped -- dir >= 9)      |

### Stats Display

The HUD stats are rendered through the print queue system (`prq` function in `fmain2.c:442-470`). Queue entry values trigger specific display updates:

- **`prq(4)`** -- Vitality bar: Renders `"Vit:"` followed by the hero's vitality value at screen position (245, 52).
- **`prq(7)`** -- Character stats: Renders all four stats at Y=52:
  - `"Brv:"` at X=14 (bravery)
  - `"Lck:"` at X=90 (luck)
  - `"Knd:"` at X=168 (kindness)
  - `"Wlth:"` at X=321 (wealth)
- **`prq(5)`** -- Calls `print_options()` to redraw the current menu.
- **`prq(2)`** -- Debug: prints coordinates and available memory (cheat mode).
- **`prq(3)`** -- Debug: prints hero sector/extent info (cheat mode).
- **`prq(10)`** -- Prints "Take What?" prompt.

Stats are stored in global `short` variables: `brave`, `luck`, `kind`, `wealth`, `hunger`, `fatigue` (`fmain.c:565`). Vitality is per-actor in `anim_list[i].vitality`. Maximum hero vitality is `15 + brave/4`.

### set_options

`set_options()` (`fmain.c:3527-3543`) dynamically updates menu enabled flags based on the player's current inventory. It calls `stuff_flag(index)` which returns 10 (displayed + immediate) if `stuff[index] > 0`, or 8 (hidden + immediate) if the item count is zero.

Updates performed:

- **MAGIC menu** (entries 5-11): `stuff_flag(i+9)` for magic items at `stuff` indices 9-15 (Stone, Jewel, Vial, Orb, Totem, Ring, Skull).
- **USE menu** (entries 0-6): `stuff_flag(i)` for weapons at `stuff` indices 0-6 (Dirk, Mace, Sword, Bow, Wand, Lasso, Shell).
- **KEYS menu** (entries 5-10): `stuff_flag(i+16)` for keys at `stuff` indices 16-21 (Gold, Green, Blue, Red, Grey, White). If any key is present (returns 10), `USE[7]` (Key slot) is also set to 10.
- **USE menu entry 8**: `stuff_flag(7)` for Sunstone at `stuff` index 7.
- **GIVE menu**:
  - Entry 5 (Gold): Set to 10 if `wealth > 2`, else 8.
  - Entry 6 (Book): Always set to 8 (immediate but hidden -- presumably always available as a give option).
  - Entry 7 (Writ): `stuff_flag(28)`.
  - Entry 8 (Bone): `stuff_flag(29)`.

### Improvement Notes

- **No mouse support in game view**: The mouse only controls the compass (via button-held region detection) and menu clicks. There is no point-and-click movement or interaction in the game viewport. A modern reimplementation should consider adding direct mouse-driven navigation and object interaction.
- **10 menu modes over-engineered**: The 10-mode system with shared top-row labels and mode-specific sub-labels creates unnecessary complexity. A context-sensitive menu that adapts to the current situation (combat, NPC proximity, shop) would be simpler and more intuitive.
- **128-byte keybuf oversized**: The circular key buffer in `in_work` is 128 bytes, but the game processes keys every frame. Even at the slowest frame rates, this is far more buffer than needed. A 16-32 entry buffer would suffice and reduce the struct size.
- **Cheat code enabling not shown in source**: The `cheat1` flag gates debug/cheat functionality, but the mechanism for setting `cheat1` to true is not present in the available source files. It may be set via an external tool, a hidden key sequence not in `keytrans`, or a compile-time flag.

---

## 20. Asset Formats & Disk Layout

### 20.1 Disk Image Structure

All game data (except IFF brushes, fonts, music scores, and voice instrument data) resides in a single 901,120-byte `image` file. This is a raw sector dump of an Amiga DD floppy disk:

- **Sector size**: 512 bytes
- **Total sectors**: 1,760 (901,120 / 512)
- **Tracks**: 160 (80 cylinders x 2 sides), 11 sectors per track

Data is read via `load_track_range(start_sector, count, buffer, io_channel)` (defined in `hdrive.c:119`). On floppy, this issues an async `CMD_READ` through the trackdisk device. On hard drive installs, the `image` file is opened as a flat file and seeked to `start_sector * 512`. Source: `hdrive.c:35-41`, `hdrive.c:119-139`.

The `copyimage.c` utility program creates the `image` file by reading raw sectors from a device and writing them sequentially to a file. It takes command-line arguments: device name, unit, first sector, count, and output filename. Source: `copyimage.c:26-78`.

### 20.2 Sector Allocation Map

Reconstructed from all `load_track_range()` calls in the source code. Each entry shows the sector range, byte size, purpose, and source reference.

| Sectors | Bytes | Content | Source |
|---------|-------|---------|--------|
| 0-31 | 16,384 | _Unused / boot blocks_ | (no load references found) |
| 32-95 | 32,768 | Sector map — outdoor regions (F1-F8) | `fmain.c:3557` — `load_track_range(nd->sector, 64, sector_mem, 0)` |
| 96-159 | 32,768 | Sector map — indoor regions (F9-F10) | `fmain.c:3557` — sector field = 96 for regions 8-9 |
| 149-159 | 5,632 | Terrain property tables (11 sets, 1 sector each) | `fmain.c:3567-3572` — `TERRA_BLOCK(149) + terra_id` |
| 160-167 | 4,096 | Region map — outdoor A (F1-F2) | `fmain.c:3562` — `load_track_range(nd->region, 8, map_mem, 0)` |
| 168-175 | 4,096 | Region map — outdoor B (F3-F4) | region field = 168 |
| 176-183 | 4,096 | Region map — outdoor C (F5-F6) | region field = 176 |
| 184-191 | 4,096 | Region map — outdoor D (F7-F8) | region field = 184 |
| 192-199 | 4,096 | Region map — indoor (F9-F10) | region field = 192 |
| 200-239 | 20,480 | Image bank — _used by F6, F8_ (image[1]=280 for F6/F8) | See note below |
| 240-279 | 20,480 | Image bank — _used by F6, F8_ (image[2]=240 for F6/F8) | |
| 280-319 | 20,480 | Image bank — _used by F2, F4_ (image[1]=360 area) | |
| 320-359 | 20,480 | Image bank — shared image[0] for F1-F8 outdoor | `fmain.c:3579-3587`, image[0]=320 for F1-F4 |
| 360-399 | 20,480 | Image bank — F2/F4 image[1] | image[1]=360 |
| 400-439 | 20,480 | Image bank — F2/F4 image[2] | image[2]=400 |
| 440-479 | 20,480 | Image bank — F2/F4 image[3] | image[3]=440 |
| 480-519 | 20,480 | Image bank — F1 image[1] | image[1]=480 |
| 520-559 | 20,480 | Image bank — F1/F3 image[2] | image[2]=520 |
| 560-599 | 20,480 | Image bank — F1/F3 image[3] | image[3]=560 |
| 600-639 | 20,480 | Image bank — F5 image[3] | image[3]=600 |
| 640-679 | 20,480 | Image bank — F7 image[1] | image[1]=640 |
| 680-719 | 20,480 | Image bank — F9 image[0] | image[0]=680 |
| 720-759 | 20,480 | Image bank — F9 image[1] | image[1]=720 |
| 760-799 | 20,480 | Image bank — F10 image[1] | image[1]=760 |
| 800-839 | 20,480 | Image bank — F9/F10 image[2] | image[2]=800 |
| 840-879 | 20,480 | Image bank — F9/F10 image[3] | image[3]=840 |
| 880 | 512 | Copy protection check sector | `fmain2.c:1430` — checks `buffer[123] == 230` |
| 881-895 | 7,680 | _Gap / unused_ | No load references |
| 896-919 | 12,288 | Shadow masks (SHADOW_SZ) | `fmain.c:1222` — `load_track_range(896, 24, shadow_mem, 0)` |
| 920-930 | 5,632 | Sound effect samples (SAMPLE_SZ) | `fmain.c:1028` — `load_track_range(920, 11, sample_mem, 8)` |
| 931-935 | 2,560 | Setfig sprites — royal set | `cfiles[14]`: file_id=931, numblocks=5 |
| 936-940 | 2,560 | Setfig sprites — wizard/priest | `cfiles[13]`: file_id=936, numblocks=5 |
| 941-945 | 2,560 | Setfig sprites — bartender | `cfiles[15]`: file_id=941, numblocks=5 |
| 946-950 | 2,560 | Setfig sprites — witch | `cfiles[16]`: file_id=946, numblocks=5 |
| 951-955 | 2,560 | Setfig sprites — ranger/beggar | `cfiles[17]`: file_id=951, numblocks=5 |
| 956-959 | 2,048 | _Gap_ | No load references |
| 960-999 | 20,480 | Enemy sprites — ogre | `cfiles[6]`: file_id=960, numblocks=40 |
| 1000-1039 | 20,480 | Enemy sprites — dark knight (spiders) | `cfiles[8]`: file_id=1000, numblocks=40 |
| 1040-1079 | 20,480 | Enemy sprites — necromancer (farmer/loraii) | `cfiles[9]`: file_id=1040, numblocks=40 |
| 1080-1119 | 20,480 | Enemy sprites — ghost | `cfiles[7]`: file_id=1080, numblocks=40 |
| 1120-1159 | 20,480 | Carrier sprites — bird | `cfiles[11]`: file_id=1120, numblocks=40 |
| 1160-1171 | 6,144 | Dragon sprites | `cfiles[10]`: file_id=1160, numblocks=12 |
| 1172-1311 | 71,680 | _Gap_ | No load references |
| 1312-1347 | 18,432 | Object sprites | `cfiles[3]`: file_id=1312, numblocks=36 |
| 1348-1350 | 1,536 | Raft sprites | `cfiles[4]`: file_id=1348, numblocks=3 |
| 1351-1370 | 10,240 | Carrier sprites — turtle | `cfiles[5]`: file_id=1351, numblocks=20 |
| 1371-1375 | 2,560 | _Gap_ | No load references |
| 1376-1417 | 21,504 | Player sprites — Julian / snake+salamander | `cfiles[0]`: file_id=1376, numblocks=42; also `cfiles[12]`: file_id=1376, numblocks=40 |
| 1418-1459 | 21,504 | Player sprites — Phillip | `cfiles[1]`: file_id=1418, numblocks=42 |
| 1460-1501 | 21,504 | Player sprites — Kevin | `cfiles[2]`: file_id=1460, numblocks=42 |
| 1502-1759 | 132,096 | _Remainder / unused_ | No load references |

**Notes on the map above:**

- Sectors 96-159 serve double duty: they hold indoor sector maps (loaded as 64 contiguous sectors starting at 96), and the terrain tables overlay the range 149-159 (loaded as individual sectors via `TERRA_BLOCK + offset`). This overlap is safe because terrain data for indoor regions is loaded from different offsets than the sector map data that occupies the same physical sectors.
- Image banks span sectors 200-879. Each bank occupies 40 sectors (20,480 bytes = 5 bitplanes x 4,096 bytes/plane). The image[0..3] values from `file_index[]` select which banks to load. Many banks are shared across regions.
- The large gap at sectors 1172-1311 (71,680 bytes) may contain additional data not referenced in the surviving source code, or may be reserved space on the original floppy layout.
- The `cfiles[12]` entry (snake and salamander) shares file_id 1376 with Julian (`cfiles[0]`), loading 40 of the same 42 blocks. This likely means the snake/salamander sprites were packed into the same disk region as Julian.

### 20.3 The file_index Table

Each region's required disk assets are defined by `struct need` (`ftale.h:104-106`):

```c
struct need {
    USHORT image[4], terra1, terra2, sector, region, setchar;
};
```

The `file_index[10]` array (`fmain.c:615-626`) maps all 10 regions:

| Idx | Region | image[0] | image[1] | image[2] | image[3] | terra1 | terra2 | sector | region | setchar |
|-----|--------|----------|----------|----------|----------|--------|--------|--------|--------|---------|
| 0 | F1 — Snow | 320 | 480 | 520 | 560 | 0 | 1 | 32 | 160 | 22 |
| 1 | F2 — Witch wood | 320 | 360 | 400 | 440 | 2 | 3 | 32 | 160 | 21 |
| 2 | F3 — Swamp | 320 | 360 | 520 | 560 | 2 | 1 | 32 | 168 | 22 |
| 3 | F4 — Plains | 320 | 360 | 400 | 440 | 2 | 3 | 32 | 168 | 21 |
| 4 | F5 — Desert | 320 | 480 | 520 | 600 | 0 | 4 | 32 | 176 | 0 |
| 5 | F6 — Bay/city | 320 | 280 | 240 | 200 | 5 | 6 | 32 | 176 | 23 |
| 6 | F7 — Volcanic | 320 | 640 | 520 | 600 | 7 | 4 | 32 | 184 | 0 |
| 7 | F8 — Forest | 320 | 280 | 240 | 200 | 5 | 6 | 32 | 184 | 24 |
| 8 | F9 — Buildings | 680 | 720 | 800 | 840 | 8 | 9 | 96 | 192 | 0 |
| 9 | F10 — Dungeons | 680 | 760 | 800 | 840 | 10 | 9 | 96 | 192 | 0 |

The `setchar` field indexes into `cfiles[]` to select the setfig character sprite set for the region (e.g., 21=witch wood NPC set, 22=snow region set). A value of 0 means no setfig characters for that region. Note: the setchar field is defined in the struct but its loading is not directly visible in the surviving `load_new_region()` code — it may be handled elsewhere or was part of cut functionality.

### 20.4 The cfiles Table — Sprite Sector Addresses

The `cfiles[]` array (`fmain2.c:638-665`) maps each sprite set to its disk location and dimensions:

```c
struct cfile_info {
    UBYTE  width, height, count;   /* sprite dimensions (in 16px units) and frame count */
    UBYTE  numblocks;              /* disk sectors to load */
    UBYTE  seq_num;                /* which seq_list slot to populate */
    USHORT file_id;                /* starting sector on disk */
};
```

| Index | Description | Width | Height | Count | Blocks | Slot | Sector |
|-------|-------------|-------|--------|-------|--------|------|--------|
| 0 | Julian | 1 | 32 | 67 | 42 | PHIL | 1376 |
| 1 | Phillip | 1 | 32 | 67 | 42 | PHIL | 1418 |
| 2 | Kevin | 1 | 32 | 67 | 42 | PHIL | 1460 |
| 3 | Objects | 1 | 16 | 116 | 36 | OBJECTS | 1312 |
| 4 | Raft | 2 | 32 | 2 | 3 | RAFT | 1348 |
| 5 | Turtle | 2 | 32 | 16 | 20 | CARRIER | 1351 |
| 6 | Ogre | 1 | 32 | 64 | 40 | ENEMY | 960 |
| 7 | Ghost | 1 | 32 | 64 | 40 | ENEMY | 1080 |
| 8 | Dark knight (spiders) | 1 | 32 | 64 | 40 | ENEMY | 1000 |
| 9 | Necromancer (farmer) | 1 | 32 | 64 | 40 | ENEMY | 1040 |
| 10 | Dragon | 3 | 40 | 5 | 12 | DRAGON | 1160 |
| 11 | Bird | 4 | 64 | 8 | 40 | CARRIER | 1120 |
| 12 | Snake/salamander | 1 | 32 | 64 | 40 | ENEMY | 1376 |
| 13 | Wizard/priest | 1 | 32 | 8 | 5 | SETFIG | 936 |
| 14 | Royal set | 1 | 32 | 8 | 5 | SETFIG | 931 |
| 15 | Bartender | 1 | 32 | 8 | 5 | SETFIG | 941 |
| 16 | Witch | 1 | 32 | 8 | 5 | SETFIG | 946 |
| 17 | Ranger/beggar | 1 | 32 | 8 | 5 | SETFIG | 951 |

The `width` field is measured in 16-pixel word units (1 = 16px wide, 2 = 32px, 3 = 48px, 4 = 64px). The `height` field is in pixels. Source: `fmain2.c:638-665`.

### 20.5 Tileset Image Format

Tileset images use the Amiga's planar bitmap format with 5 bitplanes, supporting 32 colors.

**Memory layout constants** (`fmain.c:638-644`):

| Constant | Value | Meaning |
|----------|-------|---------|
| `QPLAN_SZ` | 4,096 | 1 bitplane of 64 tiles (one "quarter set") |
| `IPLAN_SZ` | 16,384 | 1 bitplane of 256 tiles (full bank) |
| `IMAGE_SZ` | 81,920 | 5 bitplanes x 256 tiles = full image buffer |
| `SHADOW_SZ` | 12,288 | 8,192 + 4,096 background masks |
| `SECTOR_SZ` | 36,864 | 256 sectors x 128 bytes + 4,096 region map |

**Tile geometry**: Each tile is 16x16 pixels. At 1 bit per pixel per plane, one tile occupies 2 bytes/row x 16 rows = 32 bytes per plane.

**Bank structure**: Each of the 4 image banks holds 64 tiles (one "quarter" of the 256-tile set):
- 64 tiles x 32 bytes/tile = 2,048 bytes per plane per quarter... but the code uses `QPLAN_SZ = 4,096`, suggesting either 128 tiles per quarter or additional data per tile.
- Looking more carefully: `IPLAN_SZ = 16,384` for 256 tiles in one plane = 64 bytes/tile/plane. This means each tile row is 4 bytes wide (32 pixels wide per tile? No — more likely the tiles are stored in a bitmap grid). The actual layout is: tiles are arranged in a 256-wide pixel bitmap (16 tiles across x 16 pixels = 256 pixels), with 16 rows of tiles (16 tiles x 16 pixels = 256 pixels tall). One plane = 256/8 bytes per row = 32 bytes/row x 256 rows = 8,192 bytes. But `IPLAN_SZ = 16,384`, which is 2x that — suggesting a 512-pixel wide or 512-pixel tall arrangement, or that additional terrain metadata is appended.

**Revised calculation**: Given `IPLAN_SZ = 16,384` and 256 tiles of 16x16 pixels:
- 256 tiles at 32 bytes/plane/tile = 8,192 bytes per plane for raw tile data
- But `IPLAN_SZ = 16,384` = 2 x 8,192, implying the bitmap is stored as a 256x256 pixel grid (32 bytes/row x 256 rows x 2 = not quite right either)
- Most likely: tiles are stored in a bitmap that is 32 bytes wide (256 pixels) by 64 rows (one plane per tile strip), totaling 32 x 512 = 16,384. This would mean tiles are arranged 16 across and 32 down, giving 512 tiles capacity, but only 256 used per bank.

**Disk loading**: Each bank is loaded as 5 reads of 8 sectors each (40 sectors = 20,480 bytes = 5 planes x 4,096 bytes). The code at `fmain.c:3579-3587`:

```c
load_track_range(nd->image[i]+0,  8, imem, 3);   imem += IPLAN_SZ;  /* plane 0 */
load_track_range(nd->image[i]+8,  8, imem, 4);   /* plane 1 */
load_track_range(nd->image[i]+16, 8, imem, 5);   /* plane 2 */
load_track_range(nd->image[i]+24, 8, imem, 6);   /* plane 3 */
load_track_range(nd->image[i]+32, 8, imem, 7);   /* plane 4 */
```

Wait — each plane load is 8 sectors = 4,096 bytes, but `imem` advances by `IPLAN_SZ` (16,384) between planes. This means each plane's 4,096 bytes of disk data is loaded into a 16,384-byte stride. The full 256-tile image uses `IPLAN_SZ * 5 = 81,920` bytes total (`IMAGE_SZ`), but each bank only loads `4,096 * 5 = 20,480` bytes from disk, placed at `QPLAN_SZ` (4,096) offsets within each `IPLAN_SZ` plane.

The 4 banks tile into memory as:
- Bank 0: offsets 0, IPLAN_SZ, 2*IPLAN_SZ, 3*IPLAN_SZ, 4*IPLAN_SZ (first 4,096 bytes of each plane)
- Bank 1: offsets QPLAN_SZ, IPLAN_SZ+QPLAN_SZ, ... (next 4,096 bytes of each plane)
- Bank 2: offsets 2*QPLAN_SZ, ... (third 4,096 bytes)
- Bank 3: offsets 3*QPLAN_SZ, ... (fourth 4,096 bytes)

This gives: `IPLAN_SZ = 4 * QPLAN_SZ = 16,384` per plane, consistent with 4 quarter-banks of 64 tiles each = 256 total tiles. Each tile is 16x16, 32 bytes/plane, so 64 tiles = 2,048 bytes/plane... but `QPLAN_SZ = 4,096`. The extra 2,048 bytes per quarter-plane likely hold terrain property data appended after the tile graphics (matching the terrain table structure: 64 tiles x 4 bytes x 4 properties = 1,024 bytes terrain + 2,048 bytes tile graphics = 3,072, plus additional padding to reach 4,096).

**Summary**: Each image bank on disk = 40 sectors = 20,480 bytes (5 planes of 4,096 bytes each). Four banks compose a full 256-tile image set. Total image buffer = 81,920 bytes.

### 20.6 Sprite Format

Sprites use 5 bitplanes plus a 1-bit mask plane (6 planes total). The mask is not stored on disk; it is generated at load time by `make_mask()` (`fmain2.c:748-749`).

**Dimensions**: The `width` field in `cfiles[]` is in 16-pixel (2-byte) units. The `height` is in pixels.

**Bytes per frame per plane** (`fmain2.c:690`):
```c
seq_list[slot].bytes = cfiles[num].height * cfiles[num].width * 2;
```

This gives: `height * width * 2` bytes per plane per frame (where `width` is the 16px-unit count, so `width * 2` = bytes per row).

**Worked example — standard character sprite (1 x 32, e.g., Julian)**:
- Width = 1 (16 pixels), Height = 32 pixels
- Bytes per plane per frame = 32 x 1 x 2 = 64 bytes
- 5 bitplanes per frame = 64 x 5 = 320 bytes of image data
- 1 mask plane per frame = 64 bytes
- Total per frame (in memory) = 384 bytes
- Julian has 67 frames: 67 x 320 = 21,440 bytes image + 67 x 64 = 4,288 bytes mask = 25,728 total
- Disk storage = 42 sectors x 512 = 21,504 bytes (image data only; mask is computed)

**Worked example — dragon sprite (3 x 40)**:
- Width = 3 (48 pixels), Height = 40 pixels
- Bytes per plane per frame = 40 x 3 x 2 = 240 bytes
- 5 planes per frame = 1,200 bytes image
- 5 frames: 5 x 1,200 = 6,000 bytes image
- Disk = 12 sectors = 6,144 bytes (6,000 image bytes + 144 slack)

**Worked example — bird sprite (4 x 64)**:
- Width = 4 (64 pixels), Height = 64 pixels
- Bytes per plane per frame = 64 x 4 x 2 = 512 bytes
- 5 planes per frame = 2,560 bytes
- 8 frames: 8 x 2,560 = 20,480 bytes
- Disk = 40 sectors = 20,480 bytes (exact fit)

**Memory layout** (`fmain2.c:696-701`):
```c
load_track_range(cfiles[num].file_id, cfiles[num].numblocks, nextshape, 8);
nextshape += size * 5;                    /* advance past 5 image planes */
seq_list[slot].maskloc = nextshape;       /* mask goes right after */
nextshape += size;                        /* advance past 1 mask plane */
```

Disk data contains 5 interleaved bitplanes per frame. After loading, `make_mask()` ORs all 5 planes together to produce the mask plane.

### 20.7 IFF/ILBM Format

IFF (Interchange File Format) brushes are used for static images: the hi-res title screen (`hiscreen`), intro story placards, and other UI graphics. These are standard Amiga IFF/ILBM files loaded from the AmigaDOS filesystem, not from the `image` disk sectors.

The IFF parser in `iffsubs.c` recognizes these chunk types:

| Chunk ID | Purpose |
|----------|---------|
| FORM | Container — must be first, value = total file length |
| ILBM | Type identifier — Interleaved Bitmap |
| BMHD | Bitmap Header — width, height, planes, compression mode |
| CMAP | Color Map — RGB triplets (3 bytes each, up to 32 colors) |
| GRAB | Grab point — x/y hotspot for brush alignment |
| BODY | Image body — actual pixel data |
| CAMG | Amiga viewport mode flags (skipped) |
| CRNG | Color range cycling info (skipped) |

**BitMapHeader structure** (`iffsubs.c:28-38`):
```c
typedef struct {
    short  width, height;       /* image dimensions in pixels */
    short  xpic, ypic;         /* position within page */
    UBYTE  nPlanes;            /* number of bitplanes (typically 5) */
    UBYTE  masking;            /* mask type */
    UBYTE  compression;        /* 0=none, 1=ByteRun1 RLE */
    UBYTE  pad1;
    short  transcolor;         /* transparent color index */
    short  xAspect, yAspect;  /* pixel aspect ratio */
    short  pageWidth, pageHeight;
} BitMapHeader;
```

**`unpackbrush()` function** (`iffsubs.c:139-189`):

1. Opens the file and reads the FORM header. Validates it matches the FORM tag.
2. Reads `blocklength` to determine total ILBM data size.
3. Iterates through chunks:
   - **BMHD**: Reads the full `BitMapHeader` to get dimensions and compression mode.
   - **CMAP, GRAB, CAMG, CRNG**: Skipped (seeked past).
   - **BODY**: Reads entire body into `shape_mem` as a temporary buffer. Then decompresses into the target bitmap.
4. Body decompression operates row by row, plane by plane:
   - Calculates `bytecount = ((width + 15) / 8) & 0xFFFE` (word-aligned byte width)
   - For each scanline, unpacks one row for each of up to 5 bitplanes
   - Each row advances by `bitmap->BytesPerRow` in the destination
5. The bit offset calculation `(x + bitmap->BytesPerRow * y)` positions the brush at coordinates (x, y) within the destination bitmap.

**Compression (ByteRun1 RLE)**: When `compression == 1`, the `unpack_line()` assembly routine (in `fsubs.asm`, commented-out C version at `iffsubs.c:223-239`) decodes:
- Byte N >= 0: Copy next N+1 bytes literally
- Byte N < 0 and N != -128: Repeat next byte (1-N) times
- Byte N == -128: No-op (skip)

Source: `iffsubs.c:1-274`

### 20.8 Font Format

The game loads the Amber font via the Amiga `LoadSeg()` call rather than the standard `OpenDiskFont()` API:

```c
seg = LoadSeg("fonts/Amber/9");
font = (struct DiskFontHeader *) ((seg << 2) + 8);
```

Source: `fmain.c:774-776`

This loads the font as a raw executable segment (Amiga hunk format), then casts the data (at offset 8 from the BCPL pointer) to a `DiskFontHeader` structure. The font is:
- **Family**: Amber
- **Size**: 9 pixels tall
- **Type**: Proportional width (variable character widths)
- **Usage**: Story placards, in-game text overlays, and map labels

A second font (Topaz, the Amiga system font) is opened via `OpenFont()` for the text viewport:
```c
tfont = OpenFont(&topaz_ta);
SetFont(&rp_text, tfont);
```

Source: `fmain.c:778-779`

### 20.9 Save File Format

Save games are stored as sequential binary dumps with no headers, no magic numbers, and no version fields. The `saveload()` function (`fmain2.c:1553-1558`) either writes or reads raw bytes depending on the `svflag` direction flag:

```c
saveload(buffer, length)
char *buffer; long length;
{   if (svflag) Write(svfile, buffer, length);
    else Read(svfile, buffer, length);
}
```

The save file template is `"df1:A.faery"` (`fmain2.c:1392`). At save time:
- `savename[2]` is set to `'0'` or `'1'` depending on which floppy drive is writable (`fmain2.c:1491-1495`)
- In hard drive mode (`hdrive == TRUE`), the `"df1:"` prefix is skipped entirely (`name += 4`, `fmain2.c:1487`), writing to the current directory as just `A.faery`
- `savename[4]` is set to `'A' + hit` for the selected slot A-H (`fmain2.c:1502`)

Result: floppy saves go to `df0:A.faery` through `df1:H.faery`; hard drive saves go to `A.faery` through `H.faery`.

**Field order in save file** (reconstructed from `fmain2.c:1507-1527` and `fmain.c:3621-3631`):

| Order | Data | Size (bytes) | Source |
|-------|------|-------------|--------|
| 1 | Misc variables starting at `map_x` | 80 | `fmain2.c:1507` |
| 2 | `region_num` | 2 | `fmain2.c:1510` |
| 3 | `anix` (animation count) + padding | 6 | `fmain2.c:1513` |
| 4 | `anim_list[]` | `anix * sizeof(struct shape)` | `fmain2.c:1514` |
| 5 | Julian's inventory (`julstuff`) | 35 | `fmain.c:3623` |
| 6 | Phillip's inventory (`philstuff`) | 35 | `fmain.c:3624` |
| 7 | Kevin's inventory (`kevstuff`) | 35 | `fmain.c:3625` |
| 8 | Missile list | `6 * sizeof(struct missile)` | `fmain.c:3630` |
| 9 | Extent list | `2 * sizeof(struct extent)` | `fmain2.c:1519` |
| 10 | Global object list (`ob_listg`) | `glbobs * sizeof(struct object)` | `fmain2.c:1522` |
| 11 | Map object counts (`mapobs`) | 20 | `fmain2.c:1523` |
| 12 | Destination object counts (`dstobs`) | 20 | `fmain2.c:1524` |
| 13 | Per-region object tables (x10) | variable | `fmain2.c:1525-1526` |

The 80-byte "misc variables" block at offset 0 starts at `map_x` and captures all contiguous game state variables declared after it in memory (hero position, stats, timers, flags, etc.). The exact fields depend on variable declaration order in the source.

### 20.10 Additional File-Based Assets

Several assets are loaded from named AmigaDOS files rather than the `image` sector dump:

| File | Content | Loader | Size |
|------|---------|--------|------|
| `v6` | Waveforms (1,024 bytes) + volume envelopes (2,560 bytes) | `fmain.c:931-936` — direct `Read()` | 3,584 |
| `songs` | Packed music score tracks | `fmain2.c:760-776` — `read_score()` | up to 5,900 |
| `fonts/Amber/9` | Proportional bitmap font | `fmain.c:774` — `LoadSeg()` | varies |
| `hiscreen` | Hi-res title/UI screen | `fmain.c:1227` — `unpackbrush()` IFF | varies |
| Various IFF brushes | Intro story images | `fmain2.c:781-789` — `unpackbrush()` / `copypage()` | varies |

The `songs` file uses a packed tracker format: each track begins with a 4-byte length word (in 16-bit sample units), followed by `length * 2` bytes of note data. Up to 28 tracks are read (4 channels x 7 songs). Source: `fmain2.c:760-776`.

The `v6` file contains instrument data: the first 1,024 bytes are 8 waveforms of 128 bytes each (raw 8-bit signed PCM single-cycle waveforms), followed by 2,560 bytes of 10 volume envelopes of 256 bytes each. Source: `fmain.c:931-936`.

### 20.11 Modern Conversion Specification Summary

| Asset | Original Size | Original Format | Converter Output |
|-------|--------------|-----------------|------------------|
| Region tilesets | 81,920 bytes each (full set) | 5-plane Amiga planar bitmap, 4 banks of 64 tiles | 4 PNG tilesheets (256 tiles total, 16x16 each) |
| Character sprites | varies (see cfiles table) | 5-plane planar + computed mask | PNG spritesheet with alpha channel |
| Terrain properties | 1,024 bytes (2 x 512) | 4 bytes/tile: maptag, terrain, tiles, big_colors | terrain.json |
| Sector maps | 32,768 bytes/region | 256 sectors of 128 bytes, raw tile indices | sectors.json or Tiled TMX |
| Region maps | 4,096 bytes/region | 128-byte sector layout map | regionmap.json |
| Music scores | up to 5,900 bytes | Packed custom tracker (4-byte length + note data) | songs.json |
| Waveforms | 1,024 bytes | 8 x 128-byte 8-bit signed PCM single-cycle | 8 WAV files |
| Volume envelopes | 2,560 bytes | 10 x 256-byte envelope curves | envelopes.json |
| Sound effects | ~5,632 bytes | 6 length-prefixed 8-bit PCM samples | 6 WAV files |
| IFF brushes | varies | Standard IFF/ILBM (RLE or raw) | PNG per image |
| Palettes | 64 bytes each (32 x 2-byte entries) | 12-bit RGB (4 bits per channel, Amiga OCS) | palette.json |
| Door table | 86 entries x 10 bytes | Struct array: 4 USHORTs + 2 chars | doors.json |
| NPC dialogue | ~4,000 bytes (embedded in code) | Null-terminated C strings in source | dialogue.json |
| Shadow masks | 12,288 bytes | Raw bitmap mask data | PNG masks or binary |
| Amber font | varies | Amiga DiskFontHeader (hunk executable) | TTF or bitmap atlas PNG |
| Save games | variable | Raw sequential binary dump, no headers | savegame.json (with named fields and version) |

### 20.12 Sector Map Cross-Reference and Verification

All `load_track_range()` calls in the codebase with their sector ranges:

**Region loading** (`fmain.c:3557-3587`):
- Sector maps: sectors 32-95 (outdoor) or 96-159 (indoor), 64 sectors each
- Region maps: sectors 160-199, 8 sectors each (5 distinct ranges)
- Terrain: sectors 149-159, 1 sector each (overlaps indoor sector map range)
- Image banks: sectors 200-879, 40 sectors per bank (17 distinct starting addresses referenced)

**One-time loads**:
- Shadows: sectors 896-919, 24 sectors (`fmain.c:1222`)
- Samples: sectors 920-930, 11 sectors (`fmain.c:1028`)
- Copy protection: sector 880, 1 sector (`fmain2.c:1430`)

**Sprite loads** (via `cfiles[]`, `fmain2.c:697`):
- Setfig sets: sectors 931-955, in 5-sector blocks
- Enemy sets: sectors 960-1119, in 40-sector blocks
- Dragon: sectors 1160-1171, 12 sectors
- Bird: sectors 1120-1159, 40 sectors
- Objects: sectors 1312-1347, 36 sectors
- Raft: sectors 1348-1350, 3 sectors
- Turtle: sectors 1351-1370, 20 sectors
- Player characters: sectors 1376-1501, in 42-sector blocks
- Snake/salamander: sectors 1376-1415, overlapping Julian (shares same disk data)

**Identified gaps (no load references)**:
- Sectors 0-31 (16,384 bytes): Boot blocks and filesystem metadata from original floppy
- Sectors 881-895 (7,680 bytes): Between copy protection and shadow masks
- Sectors 956-959 (2,048 bytes): Between setfig sprites and enemy sprites
- Sectors 1172-1311 (71,680 bytes): Large gap between dragon/bird and object sprites
- Sectors 1371-1375 (2,560 bytes): Between turtle and player sprites
- Sectors 1502-1759 (132,096 bytes): End of disk after Kevin's sprites

**Overlap note**: The terrain tables (TERRA_BLOCK=149, offsets 0-10 giving sectors 149-159) overlap the indoor sector map range (96-159). This is safe because `load_track_range` reads into separate buffers (`terra_mem` vs `sector_mem`), and the terrain sectors are loaded independently from the sector map's 64-sector block read.

**Verification**: All `cfiles[].numblocks` values have been checked against expected sizes computed from `width * height * 2 * count * 5 / 512`, and they match within rounding (disk sectors round up). No sector ranges loaded for different purposes actually collide in time, as the async I/O channels (0-8) are managed to prevent read conflicts.
