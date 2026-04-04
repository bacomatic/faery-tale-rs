# The Faery Tale Adventure — Implementation Specification

> **Target platform:** Rust with SDL2 (via `sdl2` crate)
> **Target fidelity:** Faithful reproduction of the original 1987 Amiga game by MicroIllusions
> **Timing basis:** NTSC-only, 30 fps gameplay tick (60 Hz audio VBL)

This document synthesizes the authoritative reference material ([RESEARCH.md](RESEARCH.md), [ARCHITECTURE.md](ARCHITECTURE.md), [STORYLINE.md](STORYLINE.md)) into a single implementation-ready specification. It defines the systems, data structures, algorithms, and behaviors required to reproduce the original game.

---

## Table of Contents

1. [Display & Rendering](#1-display--rendering)
2. [World Structure](#2-world-structure)
3. [Tile & Map System](#3-tile--map-system)
4. [Scrolling & Camera](#4-scrolling--camera)
5. [Sprite System](#5-sprite-system)
6. [Terrain Masking & Z-Sorting](#6-terrain-masking--z-sorting)
7. [Color Palettes & Day/Night Fading](#7-color-palettes--daynight-fading)
8. [Characters & Animation](#8-characters--animation)
9. [Player Movement & Input](#9-player-movement--input)
10. [Combat System](#10-combat-system)
11. [AI & Behavior](#11-ai--behavior)
12. [Encounter Generation](#12-encounter-generation)
13. [NPCs & Dialogue](#13-npcs--dialogue)
14. [Inventory & Items](#14-inventory--items)
15. [Quest System](#15-quest-system)
16. [Doors & Buildings](#16-doors--buildings)
17. [Day/Night Cycle](#17-daynight-cycle)
18. [Survival Mechanics](#18-survival-mechanics)
19. [Magic System](#19-magic-system)
20. [Death & Revival](#20-death--revival)
21. [Carriers & World Navigation](#21-carriers--world-navigation)
22. [Audio System](#22-audio-system)
23. [Intro & Narrative](#23-intro--narrative)
24. [Save/Load System](#24-saveload-system)
25. [UI & Menu System](#25-ui--menu-system)
26. [Asset Formats & Data Loading](#26-asset-formats--data-loading)
27. [Special Effects](#27-special-effects)

---

## 1. Display & Rendering

### 1.1 Screen Layout

The original game uses a **non-interlaced 320×200 frame** with a mixed-resolution split display: a low-resolution playfield above a hi-resolution status bar. On the Amiga this works by changing pixel timing as the viewport changes.

A faithful port should therefore render each section at its **native resolution** first, then composite into a 640×480 presentation buffer that preserves the intended aspect ratio:

| Area | Purpose | Native Size | Presented Size in 640×480 | Original Amiga Mode |
|------|---------|-------------|----------------------------|---------------------|
| Playfield viewport | Game world | 288×140 | 576×280 (2× scale) | Low-res, 5 bitplanes (32 colors) |
| HI bar | Text, stats, menus, compass | 640×57 | 640×114 (line doubled) | Hi-res, 4 bitplanes (16 colors) |
| Inter-panel gap | Blank separator | 3 lines | 6 pixels | Non-interlaced spacer |

The composed game view occupies **400 vertical pixels** of the 640×480 canvas and should be centered vertically: **40 px top margin**, then the 280 px playfield, 6 px gap, 114 px HI bar, and **40 px bottom margin**. The 576 px-wide playfield is horizontally centered within the 640 px frame.

Key constants:
- `PAGE_DEPTH` = 5 (bitplanes per game page)
- `PHANTA_WIDTH` = 320 (bitmap width including 16px scroll margin on each side)
- `RAST_HEIGHT` = 200 (original non-interlaced frame height)
- `PAGE_HEIGHT` = 143 (game viewport vertical extent in original pixels)
- `TEXT_HEIGHT` = 57 (native HUD/HI bar height before line doubling)

### 1.2 Double Buffering

Two offscreen buffers alternate roles each frame:
- **Drawing page**: Actively being rendered to (off-screen).
- **Viewing page**: Currently displayed on screen.

Each page maintains:
- Last-drawn tile scroll position (`isv_x`, `isv_y`)
- Sprite count for the frame (`obcount`)
- Shape queue for background save/restore (up to `MAXSHAPES` = 25 entries)
- Background save buffer for sprite compositing undo
- Witch FX position state

Page swap presents the drawing page to the screen, then the pages exchange roles.

### 1.3 Rendering Pipeline Per Frame

1. Restore previous frame's sprite backgrounds (reverse order)
2. Detect scroll delta and shift map tiles if needed (or full redraw on teleport)
3. Set sub-tile scroll offsets for smooth scrolling
4. Z-sort all active sprites by Y-coordinate
5. For each sprite (back-to-front):
   a. Determine type, frame, and screen position
   b. Apply sinking/riding/death visual adjustments
   c. Clip to viewport bounds
   d. Save background under sprite footprint
   e. Build terrain occlusion mask
   f. Composite sprite with mask onto drawing page
   g. If sprite has weapon overlay, repeat for weapon pass
6. Swap pages

---

## 2. World Structure

### 2.1 Coordinate System

- Full world coordinate space: 32768×40960 (unsigned 16-bit, `MAXCOORD` = 0x7FFF for X)
- Y range: 0x0000–0x9FFF (40960)
- Coordinates wrap at world boundaries (the world is a torus)
- Tile width: 16 pixels, tile height: 32 pixels (in world coordinates, though tiles are 16×16 pixels on screen — the 32-pixel height reflects the interlaced display mapping)

### 2.2 Region System

The world is divided into **10 regions**:
- Regions 0–7: Outdoor overworld arranged in a 2×4 grid
- Region 8: Building interiors
- Region 9: Dungeon interiors

Region number calculation from coordinates:
```
x_bit = (map_x + 151) >> 14  // bit 6 of sector X: east(1)/west(0)
y_bits = ((map_y + 64) >> 13) & 7  // bits 5-7: north/south band (0-7)
region = x_bit + 2 * y_bits
```

Each region has its own asset configuration defined in the `file_index[10]` table: 4 image bank references, 2 terrain table IDs, sector map start, region map start, and setfig character set ID.

| Index | Name | Description |
|-------|------|-------------|
| 0 | F1 — Snow | Northern ice region |
| 1 | F2 — Witch wood | Dark forest |
| 2 | F3 — Swamp | Marshlands |
| 3 | F4 — Plains | Central grasslands (contains village of Tambry) |
| 4 | F5 — Desert | Burning Waste and hidden city |
| 5 | F6 — Bay/city | Coastal area with Marheim |
| 6 | F7 — Volcanic | Lava fields |
| 7 | F8 — Forest | Southern woodlands |
| 8 | F9 — Buildings | All indoor building interiors |
| 9 | F10 — Dungeons | All cave and dungeon interiors |

### 2.3 Sector Format

Each region map is a grid of **256 sectors**, each 128 bytes. Sectors contain raw tile indices that reference the region's tileset. The sector map is loaded as a contiguous 32768-byte block (256 × 128).

A separate **region map** (4096 bytes, loaded as 8 disk sectors) maps sector IDs to image bank indices.

---

## 3. Tile & Map System

### 3.1 Tileset Structure

Each region's tileset comprises 256 tiles, organized into 4 banks of 64 tiles each. Tiles are 16×16 pixels, 32 colors (5 bitplanes).

Image banks are loaded from disk as 40 sectors each (20480 bytes = 5 planes × 4096 bytes/plane). Four banks compose a complete 256-tile image set totaling 81920 bytes.

### 3.2 Terrain Properties

Terrain data is loaded as two 512-byte halves (one per terrain table ID) into a 1024-byte buffer. Each entry is 4 bytes, encoding:
- Byte 0: mask shape index (for terrain masking/occlusion)
- Byte 1 lower nibble (bits 0–3): terrain walkability (0–3 scale)
- Byte 1 upper nibble (bits 4–7): mask mode (0–7, controlling occlusion behavior)

### 3.3 Map Rendering

`map_draw()` renders the full visible map area from tile data to the offscreen bitmap. For incremental scrolling, `strip_draw()` renders a single column and `row_draw()` renders a single row.

A **minimap** array (19×6 = 114 entries) caches the mapping from viewport tile positions to terrain tile IDs for fast terrain mask lookups during sprite compositing.

---

## 4. Scrolling & Camera

### 4.1 Tile-Level Scrolling

Each frame, compute scroll delta in tile coordinates:
```
dif_x = (map_x >> 4) - last_drawn_tile_x
dif_y = (map_y >> 5) - last_drawn_tile_y
```

For single-tile movement, shift the bitmap contents by one tile and repair the exposed edge. For multi-tile jumps (teleportation), perform a full map redraw.

### 4.2 Sub-Tile Smooth Scrolling

After sprite compositing, apply sub-tile pixel offsets:
```
rx_offset = map_x & 15   // 0–15 horizontal
ry_offset = map_y & 31   // 0–31 vertical
```

In SDL2, this maps to fractional camera offset when rendering the playfield texture.

---

## 5. Sprite System

### 5.1 Actor Slots

Up to **20 actor slots** (`anim_list[20]`), with hardcoded role assignments:

| Slot | Role |
|------|------|
| 0 | Hero (player character) |
| 1 | Raft |
| 2 | Set figure (reserved) |
| 3 | Carrier entity (turtle/bird/dragon) or first encounter enemy |
| 4–6 | Additional encounter enemies |
| 7+ | Overflow actors |

The active actor count `anix` tracks how many slots are in use.

### 5.2 Sprite Data Format

Sprites are stored as 5 bitplanes of image data. A 1-bit mask plane is generated at load time by ORing all 5 image planes together.

Dimensions are defined per sprite set:
- Width in 16-pixel units (1 = 16px, 2 = 32px, 3 = 48px, 4 = 64px)
- Height in pixels

Sprite sets with frame counts:

| Type | Width | Height | Frames | Description |
|------|-------|--------|--------|-------------|
| PHIL (player) | 16px | 32px | 67 | Julian/Phillip/Kevin |
| OBJECTS | 16px | 16px | 116 | Items, effects, fairy |
| RAFT | 32px | 32px | 2 | Raft/grounded bird |
| CARRIER (turtle) | 32px | 32px | 16 | Turtle |
| CARRIER (bird) | 64px | 64px | 8 | Swan |
| ENEMY | 16px | 32px | 64 | Ogre/ghost/spider/etc |
| SETFIG | 16px | 32px | 8 | NPCs (wizard, priest, etc) |
| DRAGON | 48px | 40px | 5 | Dragon |

### 5.3 The `struct shape` (Actor State)

Each actor maintains 22 fields:

| Field | Type | Description |
|-------|------|-------------|
| `abs_x`, `abs_y` | u16 | World position |
| `rel_x`, `rel_y` | i16 | Screen-relative position |
| `type` | u8 | Sprite type (PHIL, ENEMY, OBJECTS, etc.) |
| `state` | u8 | Animation state (one of 26 states) |
| `index` | u8 | Current frame index |
| `facing` | u8 | Direction 0–7 |
| `race` | u8 | NPC/enemy type ID |
| `goal` | u8 | AI goal mode (0–10) |
| `tession` | u8 | AI tactical mode (0–12) |
| `vitality` | i8 | Health points |
| `weapon` | u8 | Equipped weapon index |
| `environ` | i8 | Terrain depth/state |
| `vel_x`, `vel_y` | i16 | Velocity (for carriers) |
| ... | ... | Additional fields |

---

## 6. Terrain Masking & Z-Sorting

### 6.1 Z-Sorting

Sprites are sorted back-to-front by Y-coordinate before rendering. Depth adjustments:
- Dead actors: Y − 32 (render behind living sprites)
- Riding hero: Y − 32 (mount doesn't obscure ground characters)
- Actor slot 1 (mount/companion): Y − 32
- Deeply sunk actors (environ > 25): Y + 32 (render in front)

### 6.2 Terrain Masking

For each word-column and tile-row that a sprite overlaps:
1. Look up the terrain tile ID from the minimap
2. Look up terrain properties from `terra_mem[]`: mask shape index and mask mode
3. Based on mask mode (0–7), determine whether to apply occlusion:

| Mode | Skip Condition | Meaning |
|------|----------------|---------|
| 0 | Always skip | No occlusion (flat ground) |
| 1 | First column | Right-side-only occlusion |
| 2 | `ystop > 35` | Top-only occlusion (low wall) |
| 3 | `hero_sector == 48` and not NPC 1 | Bridge (hero walks over) |
| 4 | First column OR `ystop > 35` | Combined right + top |
| 5 | First column AND `ystop > 35` | Right and top (OR logic) |
| 6 | Not bottom row: full mask (tile 64) | Two-story buildings |
| 7 | `ystop > 20` | Stricter top-only |

4. Write the mask shape into the per-frame occlusion buffer
5. Combine with sprite transparency mask during compositing

Certain sprites skip masking entirely: carriers, arrows, fairy sprites (objects 100–101), certain NPC races.

---

## 7. Color Palettes & Day/Night Fading

### 7.1 Palette Definitions

Four palettes are used:

- **Game palette** (`pagecolors[32]`): 32 colors, 12-bit Amiga RGB. Color 31 overridden per-region.
- **Text palette** (`textcolors[20]`): 20 colors for the HUD.
- **Intro palette** (`introcolors[32]`): Title screen and story pages.
- **Black palette** (`blackcolors[32]`): All zeros, for fade transitions.

12-bit to 24-bit conversion: multiply each 4-bit channel by 17 (0x11).

### 7.2 Day/Night Fading Algorithm

`fade_page(r, g, b, limit, colors)` applies per-channel brightness scaling:

1. Inside buildings (region ≥ 8): always full brightness (100, 100, 100)
2. Outdoors: derive R/G/B percentages from `lightlevel`:
   - Red = lightlevel − 80 (+ 200 if light spell active)
   - Green = lightlevel − 61
   - Blue = lightlevel − 62
3. Clamp percentages to 0–100
4. Apply night minimums when `limit` is true: red ≥ 10%, green ≥ 25%, blue ≥ 60%
5. Compute blue night-shift factor: `g2 = (100 − green_pct) / 3`
6. Per color entry:
   ```
   r_out = (r_pct × r_raw) / 1600
   g_out = (g_pct × g_raw) / 1600
   b_out = (b_pct × b_raw + g2 × g_out_scaled) / 100
   ```
7. Light spell: if active and color's red < green, boost red to match green (warm tint)
8. Twilight boost: colors 16–24 gain extra blue when green% is 20–75
9. Cap all channels at maximum (15 for 12-bit, 255 for 24-bit)

Color 31 region overrides:
- Region 4 (volcanic): 0x0980 (orange)
- Region 9 (dungeon): 0x00F0 (bright green) if `secret_timer` active, else 0x0445
- Default: 0x0BDF (sky blue)

---

## 8. Characters & Animation

### 8.1 Animation States

26 animation states, each mapping to entries in the `statelist[87]` table:

| State | Name | Description |
|-------|------|-------------|
| 0 | FIGHTING | Combat animation |
| 1 | STILL | Idle |
| 2 | WALKING | Movement |
| 3 | DEAD | Deceased |
| 4 | DYING | Death transition |
| 5 | SLEEP | Sleeping |
| 6 | FALL | Falling/collapsing |
| 7–25 | SWING1–SHOOT3 | Various combat animation phases |

The `statelist[87]` maps each (state, direction) combination to a sprite frame index and weapon overlay data.

### 8.2 Direction System

8 compass directions (0 = N, 1 = NE, ... 7 = NW), with inverted Y-axis mapping:

```
diroffs[16] = { 0,-3, -5,-3, -5,0, -5,3, 0,3, 5,3, 5,0, 5,-3 }
```

Walk cycle: 8 frames per direction (indexed by `cycle & 7`), with direction determining the base sprite offset.

### 8.3 Combat Animation FSA

A 9-state finite state automaton controls combat transitions:

| From State | FIGHTING | SWING1 | SWING2 | SWING3 | SWING4 | BACKSWING |
|------------|----------|--------|--------|--------|--------|-----------|
| FIGHTING | random(SWING1–SWING4) | | | | | |
| SWING1 | | → SWING2 or SWING3 (random) | | | | |
| SWING2 | | | → FIGHTING | | | |
| SWING3 | | | | → SWING4 | | |
| SWING4 | | | | | → BACKSWING | |
| BACKSWING | | | | | | → FIGHTING |

Transitions are checked each frame with random branching.

### 8.4 Character Sprite Sets

18 entries in the `cfiles[]` table map sprite sets to disk locations:

| Index | Description | Frames | Disk Sector |
|-------|-------------|--------|-------------|
| 0 | Julian | 67 | 1376 |
| 1 | Phillip | 67 | 1418 |
| 2 | Kevin | 67 | 1460 |
| 3 | Objects | 116 | 1312 |
| 4 | Raft | 2 | 1348 |
| 5 | Turtle | 16 | 1351 |
| 6 | Ogre | 64 | 960 |
| 7 | Ghost | 64 | 1080 |
| 8 | Dark knight/spiders | 64 | 1000 |
| 9 | Necromancer/farmer | 64 | 1040 |
| 10 | Dragon | 5 | 1160 |
| 11 | Bird | 8 | 1120 |
| 12 | Snake/salamander | 64 | 1376 (shared with Julian) |
| 13 | Wizard/priest | 8 | 936 |
| 14 | Royal set | 8 | 931 |
| 15 | Bartender | 8 | 941 |
| 16 | Witch | 8 | 946 |
| 17 | Ranger/beggar | 8 | 951 |

---

## 9. Player Movement & Input

### 9.1 Input Sources (Priority Order)

1. **Mouse/compass click**: If either button held, sprite position mapped to 9 compass regions (3×3 grid). X ≤ 265 produces no direction.
2. **Joystick**: Hardware joy registers read directly, producing signed X/Y → direction lookup.
3. **Keyboard**: Direction key codes 20–29 map to directions 0–8 (plus 9 = none).

Direction lookup table: `{0,1,2,7,9,3,6,5,4}` (N, NE, E, NW, center, SE, SW, S, W).

### 9.2 Fight Detection

Combat stance activates when any of:
- Right mouse button held (`qualifier & 0x2000`)
- Keyboard `0` key held (`keyfight`)
- Joystick button 1 pressed

### 9.3 Movement Speed

Hero walk speed depends on weapon and terrain:
- Base speed: 5 pixels/frame (walking)
- Raft: 1 pixel/frame (attached to hero)
- Turtle (ridden): 3 pixels/frame
- Bird: velocity-based acceleration with cap of 40 (vertical) and 32 (horizontal), position = position + velocity/4

---

## 10. Combat System

### 10.1 Melee Hit Detection

O(n²) scan: every actor pair within range. Detection uses **Chebyshev distance** (max of |dx|, |dy|):
- Hit radius = `11 + (attacker.brave / 4)`
- Only checked when attacker's animation state is SWING or FIGHTING

### 10.2 Damage (`dohit`)

1. **Immunity check**: Witch is immune to all physical damage. Necromancer is immune unless race < 7.
2. **Damage calculation**: `damage = 1 + weapon_bonus + brave/8 + random(0–1)`
3. **Push-back**: victim nudged 3–5 pixels away from attacker's facing direction.
4. **Stat effects**: `brave++` for attacker on hit; `luck--` for the victim (hero only).

### 10.3 Missile System

6 missile slots (`struct missile`):
- Types: arrow (from bow) and fireball (from wand or dragon)
- Speed: 10 pixels/frame (hero arrows), 5 (dragon fireballs)
- Launch states: SHOOT1 (bow aim) → SHOOT3 (release)
- Hit detection: Chebyshev distance ≤ 10 from missile position

### 10.4 Enemy Encounter Chart

11 enemy types with distinct stats:

| Race | Name | HP | Weapon | Cleverness | Treasure | Sprite Set |
|------|------|----|--------|------------|----------|------------|
| 0 | Ogre | 12 | 3 | 3 | 3 | cfiles[6] |
| 1 | Orc | 7 | 2 | 2 | 2 | cfiles[6] (recolor) |
| 2 | Wraith | 14 | 3 | 4 | 4 | cfiles[7] |
| 3 | Skeleton | 6 | 1 | 1 | 1 | cfiles[7] (recolor) |
| 4 | Snake | 4 | 1 | 1 | 1 | cfiles[12] |
| 5 | Salamander | 20 | 4 | 5 | 5 | cfiles[12] (recolor) |
| 6 | Loraii | 10 | 3 | 3 | 3 | cfiles[9] |
| 7 | Dark Knight | 25 | 5 | 5 | 5 | cfiles[8] |
| 8 | Spider | 5 | 1 | 1 | 0 | cfiles[8] (recolor) |
| 9 | Necromancer | 50 | 5 | 5 | 5 | cfiles[9] |
| 10 | Dragon | 50 | 5 | 5 | 5 | cfiles[10] |

### 10.5 Weapon and Treasure Probability Tables

On enemy death, weapon and treasure drops are determined by probability tables indexed by the enemy's `weapon_tier` and `treasure_tier` respectively:
- Weapon table: 8 tiers × 4 entries (selected by `random % 4`)
- Treasure table: 5 tiers × 8 entries (selected by `random % 8`)
- Tier 0 = no drop

---

## 11. AI & Behavior

### 11.1 Goal Modes

11 high-level goal modes:

| Value | Name | Behavior |
|-------|------|----------|
| 0 | USER | Player-controlled (hero only) |
| 1 | SEEK | Move toward hero |
| 2 | FLEE | Move away from hero |
| 3 | RANDOM | Wander randomly |
| 4 | DEATH | Dead — no actions |
| 5 | AIMLESS | Random direction changes |
| 6 | SEEKOBJ | Move toward a specific object |
| 7 | GUARD | Stay in area, attack intruders |
| 8 | RAFTFOLLOW | Follow the raft |
| 9 | PATROL | Walk a fixed route |
| 10 | CONFUSED | Reversed controls / disoriented |

### 11.2 Tactical Modes

13 tactical sub-modes that control frame-by-frame behavior:

| Value | Name | Behavior |
|-------|------|----------|
| 0 | FRUST | Frustrated — change course |
| 1 | AVOID | Detour around obstacle |
| 2 | PURSUE | Direct pursuit |
| 3 | CLOSE | Close to melee range |
| 4 | FIGHT | Active melee combat |
| 5 | BACKUP | Retreat briefly |
| 6 | MISSILE | Fire ranged attack |
| 7 | WANDER | Random patrol |
| 8 | WAIT | Idle timeout |
| 9 | TELEPORT | Relocate to hero |
| 10 | OBEY | Follow path commands |
| 11 | DOOR_SEEK | Navigate toward door |
| 12 | DOOR_LET | Let hero through door |

### 11.3 The `set_course` Algorithm

Computes direction from current position to target. Seven modes select different behaviors (direct approach, offset pursuit, random wander, etc.). Uses compass direction lookup from dx/dy to the 8 compass directions.

### 11.4 AI Decision Loop (Per Tick, Per Actor)

1. Skip if `goodfairy && goodfairy < 120` (fairy animation suppresses AI)
2. Compute distance to hero
3. If enemy, determine engagement: pursue if close, flee if outmatched
4. Execute `do_tactic()` based on current tactical mode
5. Update position based on direction and speed
6. Check terrain collision, adjust or redirect

---

## 12. Encounter Generation

### 12.1 Extent Zones

The `extent_list[23]` array defines rectangular trigger zones. Each entry has:
- Bounding box (`x1, y1, x2, y2`)
- Type code (`etype`)
- Parameters: count (`v1`), random range (`v2`), encounter type (`v3`)

`find_place()` performs a **linear scan** of the first 22 entries each movement tick. First match wins. Entry 22 is the "whole world" fallback.

### 12.2 Extent Type Categories

| etype Range | Category | Behavior |
|-------------|----------|----------|
| 0–49 | Random encounters | Use etype as encounter chart index |
| 50–59 | Set group | Fixed encounter group (52 = astral, 53 = spider pit) |
| 60–61 | Special figure | Force-spawn unique NPC (60 = single, 61 = multiple) |
| 70 | Carrier | Load rideable creature |
| 80–82 | Peace zone | No random combat (80 = general, 81 = king, 82 = sorceress) |
| 83 | Princess | Trigger rescue sequence if princess is captive |

### 12.3 Encounter Timing

- **Placement phase**: every 16 ticks, determine spawn positions
- **Generation phase**: every 32 ticks, generate actual enemies
- **Danger level**: computed from `v1` (base count) + random(0, `v2`) + current encounter type

---

## 13. NPCs & Dialogue

### 13.1 NPC Types (setfig_table)

14 NPC types, each identified by `race & 0x7F`:

| Type | NPC | Speech Logic |
|------|-----|-------------|
| 0 | Wizard | Hint based on per-instance `goal` field (speeches 27–34) |
| 1 | Priest | Heals if kind, gives statue if player has writ |
| 2,3 | Guards | Fixed "State your business!" |
| 4 | Princess | "Rescue me!" (only when captive flag set) |
| 5 | King | "I cannot help you" (only when flag set) |
| 6 | Noble (Lord Trane) | Fixed hint about princess |
| 7 | Sorceress | Gives figurine on first visit; luck boost on repeat |
| 8 | Bartender | Context-dependent (fatigue/time) |
| 9 | Witch | "Look into my eyes and Die!" |
| 10 | Spectre | Requests bones, gives crystal shard |
| 11 | Ghost | Reports dead brother's bone location |
| 12 | Ranger | Directional hints based on region and goal |
| 13 | Beggar | Prophecies (3 per-instance variants) |

### 13.2 Talk Ranges

| Command | Range | Special |
|---------|-------|---------|
| Yell | 100px | If NPC within 35px, "Don't shout!" |
| Say | 50px | Normal conversation |
| Ask | 50px | Same as Say |

### 13.3 Speech System

61 speech entries in the speech catalogue. The `%` character is substituted with the current brother's name at display time. Speech entries include NPC dialogue, enemy grunts, quest progression text, and environmental messages.

### 13.4 GIVE Handler

Items that can be given to NPCs:
- **Gold**: Costs 2 gold. Random kindness increase. Beggars give prophecies.
- **Bone**: If given to spectre, receive crystal shard. Otherwise "no use for it."

### 13.5 Carrier Dialogue (Turtle)

- If hero has sea shell: "Hop on my back."
- If hero doesn't have shell: grants shell, "Thank you for saving my eggs!"

---

## 14. Inventory & Items

### 14.1 Inventory Structure

`stuff[35]` array (UBYTE, per brother):

| Index Range | Category | Items |
|-------------|----------|-------|
| 0–8 | Weapons/tools | Dirk, Mace, Sword, Bow, Wand, Lasso, Shell, Sunstone, Book |
| 9–15 | Magic items | Blue Stone, Green Jewel, Glass Vial, Crystal Orb, Bird Totem, Gold Ring, Jade Skull |
| 16–21 | Keys | Gold, Silver, Jade, Crystal, Ebony, Bronze |
| 22 | Quest | Talisman (triggers win condition) |
| 23 | Consumable | Fiery Fruit (lava immunity) |
| 24 | Consumable | Apples (auto-eaten in safe zones) |
| 25–30 | Statues/quest | Gold Statues (5 needed), Writ, Bone, etc. |
| 31–34 | Currency | Gold and arrows |

Constants: `MAGICBASE` = 9, `KEYBASE` = 16, `STATBASE` = 25, `GOLDBASE` = 31, `ARROWBASE` = 35.

### 14.2 Item Pickup Logic

- Item translation table (`itrans[31]`) maps ground object types to inventory slots
- Special cases for containers (loot tables), known items (keys, named items), and equipment
- Body search on dead enemies: weapon drop (by probability table) then treasure drop

### 14.3 Equipment Effects

Weapon slot 0 (`stuff[0]`–`stuff[4]`) determines melee damage bonus. Bow (`stuff[3]`) enables ranged attacks. Wand (`stuff[4]`) enables fireball ranged attacks.

---

## 15. Quest System

### 15.1 Main Quest Flow

1. Rescue princess (up to 3 princesses: Katra, Karla, Kandy)
2. Obtain writ from king
3. Trade writ for golden statue at priest
4. Collect 5 golden statues (from priest, sorceress, spectre, beggars, quest rewards)
5. Enter hidden city in desert (gated by statue count ≥ 5)
6. Defeat the Necromancer
7. Pick up the Talisman (triggers win condition)

### 15.2 Quest State Flags

| Variable | Meaning |
|----------|---------|
| `ob_list8[9].ob_stat` | Princess captive (nonzero = captive, reset to 3 on brother death) |
| `ob_listg[9].ob_stat` | Sorceress figurine given |
| `ob_listg[10].ob_stat` | Priest statue given |
| `ob_listg[1-2].ob_stat` | Dead brother bones flag |
| `stuff[22]` | Talisman (nonzero = win) |
| `stuff[25]` | Gold statues count (5 needed) |
| `stuff[28]` | Writ |
| `princess` | Rescue counter (affects placard text selection) |

### 15.3 Hidden City Reveal

When entering region 4 (desert) with fewer than 5 golden statues, four tiles at map offset `(11×128)+26` are overwritten with impassable tile 254. With ≥ 5 statues, the tiles remain passable. Patch is applied on every region load (RAM-only modification).

### 15.4 Win Condition

Checked on every item pickup: if `stuff[22]` (Talisman) is nonzero, set `quitflag = TRUE` and `viewstatus = 2`, then launch the victory sequence.

### 15.5 Victory Sequence

1. Display victory placard text
2. Load victory image (`winpic`)
3. Black out display
4. 55-step sunrise fade-in using `sun_colors[53]` table
5. Final fade to black

### 15.6 Stone Ring Teleportation Network

11 stone ring locations. Activation requires:
1. Standing on stone ring tile (sector 144)
2. Center of tile position (sub-tile check)
3. Match against `stone_list[]`

Destination = `(current_stone + facing + 1) % 11`. Direction determines which ring to teleport to. Visual effect: 32 frames of random palette cycling (`colorplay()`).

---

## 16. Doors & Buildings

### 16.1 Door Structure

86 door entries, each:
- Outdoor coords (`xc1, yc1`): overworld position
- Indoor coords (`xc2, yc2`): interior position
- Type: visual/behavioral type (odd = horizontal, even = vertical)
- Secs: destination region (1 = region 8 buildings, 2 = region 9 dungeons)

### 16.2 Door Types

19 door types numbered 1–18 (with aliases). Key types: HWOOD, VWOOD, HSTONE, CRYST, SECRET, BLACK, MARBLE, LOG, DESERT, CAVE/VLOG, STAIR.

### 16.3 Transition Logic

**Outdoor → Indoor (binary search)**: The door table is sorted by `xc1`, enabling O(log n) lookup. Orientation check validates hero alignment relative to door.

**Indoor → Outdoor (linear scan)**: Indoor coordinates are unsorted, requiring O(n) scan.

**DESERT door special case**: Blocked unless `stuff[STATBASE] >= 5`.

### 16.4 Door Opening (Locked Doors)

The `open_list[17]` table defines which tile/region combinations can be opened and with which key:
- Key types: NOKEY (0), GOLD (1), GREEN (2), KBLUE (3), RED (4), GREY (5), WHITE (6)
- On success: replace door tiles with "opened" variants, force screen redraw

### 16.5 The `xfer()` Function

Performs teleportation:
1. Adjust map scroll by same delta as hero position
2. Set hero position to destination
3. Clear encounters
4. If exiting indoors: recalculate region from coordinates
5. Load region data
6. Regenerate minimap
7. Force full screen redraw
8. Update music mood
9. Nudge hero downward if colliding with solid object at destination

---

## 17. Day/Night Cycle

### 17.1 Day Counter

`daynight` is a 16-bit unsigned integer, incrementing by 1 each game tick. Wraps at 24000 (full cycle ≈ 6.7 minutes real time at 60 Hz).

Time does not advance during freeze spells. During sleep: +63 per tick (plus normal +1 = 64 effective).

### 17.2 Light Level

Triangle wave derived from `daynight`:
```
lightlevel = daynight / 40
if (lightlevel >= 300) lightlevel = 600 − lightlevel
```

Range: 0 (midnight) → 300 (midday) → 0 (midnight).

### 17.3 Time-of-Day Events

12 periods of 2000 ticks each:

| Period | daynight Range | Event |
|--------|---------------|-------|
| 0 | 0–1999 | Midnight message |
| 4 | 8000–9999 | Morning message |
| 6 | 12000–13999 | Midday message |
| 9 | 18000–19999 | Evening message |

### 17.4 Turtle Night Indicator

When `lightlevel < 40`, turtle companion object switches to glowing state (3); otherwise normal state (2).

---

## 18. Survival Mechanics

### 18.1 Hunger

Increments by 1 every 128 game ticks (while alive and not sleeping).

| Threshold | Effect |
|-----------|--------|
| 35 | First hunger warning |
| 60 | Increased warning |
| 90 | Severe warning (one-time) |
| > 90 | Starvation warning (every 8th tick) |
| > 100 | Vitality −2 (every 8th tick, combined with fatigue > 160) |
| > 140 | Hero collapses, hunger reset to 130, forced sleep |

### 18.2 Fatigue

Increments on the same 128-tick timer as hunger.

| Threshold | Effect |
|-----------|--------|
| 70 | Tiredness warning |
| > 160 | Vitality −2 (every 8th tick, combined with hunger > 100) |
| > 170 | Forced sleep (only when vitality ≤ 5) |

### 18.3 Sleep

Voluntary sleep: stand on bed tile (IDs 161, 52, 162, 53) in region 8 for 30 ticks. Requires fatigue ≥ 50.

Wake conditions (any):
- Fatigue reaches 0
- Fatigue < 30 AND morning (daynight 9000–10000)
- Enemy present AND random 1-in-64

### 18.4 Health Regeneration

+1 vitality every 1024 ticks of the day counter. Max vitality = `15 + brave/4`. Dead heroes do not regenerate.

### 18.5 Safe Zones

Updated every 128 ticks when ALL conditions met:
- No enemies visible or loading
- No witch encounter active
- Hero on solid ground (environ == 0)
- No danger flag
- Hero alive

Auto-eat: if hunger > 30 and hero has apples, consume one apple (−30 hunger).

### 18.6 Fiery Death Zone

Rectangle: `8802 < map_x < 13562`, `24744 < map_y < 29544`.
- Hero with fiery fruit (`stuff[23]`): immune (environ reset to 0 each tick)
- Environ > 15: instant death
- Environ > 2: −1 vitality per tick

---

## 19. Magic System

### 19.1 Preconditions

1. Must have the item (`stuff[4+hit] > 0`); otherwise "if only I had some Magic!"
2. Cannot use in Necromancer arena (`extn->v3 == 9`); "Your magic won't work here!"

### 19.2 Magic Items

| Item | stuff[] | Effect |
|------|---------|--------|
| Blue Stone | 9 | Stone ring teleport (see §15.6) |
| Green Jewel | 10 | `light_timer += 760` (illumination spell) |
| Glass Vial | 11 | Heal: vitality += random(0–7) + 4, capped at max |
| Crystal Orb | 12 | Display world map with hero marker (outdoor only) |
| Bird Totem | 13 | `secret_timer += 360` (reveal hidden objects) |
| Gold Ring | 14 | `freeze_timer += 100` (time stop; blocked if riding > 1) |
| Jade Skull | 15 | Kill all enemies with race < 7; brave−1 per kill |

### 19.3 Charge Depletion

After use, `stuff[4+hit]--`. If reaches 0, rebuild menu to remove depleted item. Failed uses (Stone Ring position check fails, Crystal Orb used indoors) do NOT consume a charge.

---

## 20. Death & Revival

### 20.1 Death Detection (`checkdead`)

When vitality < 1 and not already DYING/DEAD:
- Set state to DYING, goal to DEATH, tactic to 7
- Hero death: display death event message, luck −= 5
- Non-hero kill: brave++ for the killer
- Special: killing non-hostile SETFIGs → kind −= 3

### 20.2 Good Fairy Mechanic

`goodfairy` is a u8 countdown from 255 after hero enters DEAD or FALL state:

| Range | Behavior |
|-------|----------|
| 255–200 | No visible effect |
| 199–120 | Luck check: if luck < 1 → brother succession. If FALL state → fairy revival |
| 119–20 | Fairy sprite animation (approaches hero) |
| 19–2 | Resurrection glow effect |
| 1 | Fairy revival: revive at safe zone |

### 20.3 Brother Succession (`revive(true)`)

1. Save dying brother's bones (brothers 1–2 only)
2. Reset princess state
3. Load next brother's stats from `blist[]`:

| Property | Julian | Phillip | Kevin |
|----------|--------|---------|-------|
| brave | 35 | 20 | 15 |
| luck | 20 | 35 | 20 |
| kind | 15 | 15 | 35 |
| wealth | 20 | 15 | 10 |
| Max HP | 23 | 20 | 18 |

4. Clear inventory, give single Dirk
5. Reset to village of Tambry (safe zone in region 3)
6. Display narrative placard
7. If brother > 3: game over ("Stay at Home!" ending)

### 20.4 Fairy Revival (`revive(false)`)

- Fade down effect
- Teleport to last safe zone
- Full health, clear hunger/fatigue
- Set to morning (daynight = 8000)

---

## 21. Carriers & World Navigation

### 21.1 Raft

- Actor slot 1, type RAFT
- Activation: within 9px proximity, no active carrier
- Movement: snaps to hero position each frame (no autonomous movement)
- Terrain: only follows hero on water tiles (px_to_im returns 3, 4, or 5)
- Prevents drowning while active (`riding == 1`)

### 21.2 Turtle

- Actor slot 3, type CARRIER, actor_file = 5
- Summoned via USE menu (turtle item)
- Boarding: within proximity, sets `riding = 5`
- Speed: 3 pixels/frame when ridden
- Autonomous movement (unridden): follows water paths via directional probing (try current dir, then ±1, then −2)
- Cannot be summoned in central region (11194–21373 X, 10205–16208 Y)

### 21.3 Bird (Swan)

- Actor slot 3, type CARRIER, actor_file = 11
- Extent zone 0 at (2118, 27237)
- Requires lasso (`stuff[5]`) to board
- Riding state: `riding = 11`, hero environment = −2 (airborne)
- Movement: velocity-based with acceleration, speed cap 40 vertical / 32 horizontal, position += velocity/4
- Dismount conditions: hero action + not fiery terrain + not too fast + no collision

### 21.4 Dragon

- Actor slot 3, type DRAGON, actor_file = 10
- Extent zone 2 (dragon cave area)
- **Hostile** — not rideable
- HP: 50, shoots fireballs (25% chance per tick, speed 5)

---

## 22. Audio System

### 22.1 Music Engine

4-voice tracker driving synthesized audio:
- **Waveforms**: 8 × 128-byte single-cycle 8-bit signed PCM (from `v6` file)
- **Volume envelopes**: 10 × 256-byte envelope curves (from `v6` file, offset 1024)
- **Scores**: 28 tracks (7 moods × 4 channels) packed in custom format (from `songs` file)

### 22.2 Track Data Format

Stream of 2-byte command/value pairs:

| Command | Meaning |
|---------|---------|
| 0–127 | Play note (command = SMUS note index, value = duration) |
| 128 | Rest (value = duration) |
| 129 | Set instrument (value = instrument number) |
| 144 | Set tempo (value = tempo) |
| 255 | End track (value 0 = stop, nonzero = loop) |

Duration table: 64 entries for standard musical durations (whole notes through 128th notes, with dotted, triplet, and extended variants).

### 22.3 Period Table

84 entries (7 octaves × 12 notes). Higher octaves use shorter waveform segments to reduce aliasing. Period-to-Hz: `Hz = 3,579,545 / period` (NTSC).

### 22.4 Mood System

7 musical moods, priority-evaluated:

| Priority | Mood | Condition |
|----------|------|-----------|
| 1 | Death | Hero vitality == 0 |
| 2 | Indoor | Hero in specific coordinate range |
| 3 | Battle | `battleflag` set |
| 4 | Astral | region > 7 |
| 5 | Day | lightlevel > 120 |
| 6 | Night | Fallback |
| — | Intro | Played at startup |

`setmood(TRUE)` restarts playback immediately. `setmood(FALSE)` updates loop points only.

### 22.5 Sound Effects

6 samples loaded from disk sectors 920–930:
- Played on channel 2, temporarily overriding that music track
- Each with variable playback rate (Amiga period units, randomized per call)
- Gated by sound effects menu toggle

| Sample | Usage | Typical Period |
|--------|-------|---------------|
| 0 | Hero injured | 800 + random(0–511) |
| 1 | Weapon swing | 150 + random(0–255) |
| 2 | Ranged hit | 500 + random(0–63) |
| 3 | Enemy hit | 400 + random(0–255) |
| 4 | Door/interaction | 400 + random(0–255) |
| 5 | Environmental | 1800–3200 + random |

---

## 23. Intro & Narrative

### 23.1 Intro Sequence

1. Legal text display (title text on dark blue background)
2. 1-second pause
3. Load audio (music + samples)
4. Start intro music (tracks 12–15)
5. Load title image (`page0`), blit to both pages
6. Vertical zoom-in (0 → 160 in steps of 4, via `screen_size()`)
7. Three story pages with columnar-reveal animation (`copypage` with `flipscan`)
8. Final pause (3.8 seconds)
9. Vertical zoom-out (156 → 0 in steps of −4)
10. Copy protection challenge

Player can skip at multiple checkpoints.

### 23.2 Copy Protection

3 random questions from a pool of 8. Answers are rhyming words from the game manual:
- LIGHT, HEED, DEED, SIGHT, FLIGHT, CREED, BLIGHT, NIGHT
- Case-sensitive comparison, max 9 characters

### 23.3 Event Messages

39 event messages (indices 0–38), triggered throughout gameplay. The `%` character substitutes the current brother's name. Messages cover hunger/fatigue warnings, death descriptions, time-of-day announcements, and narrative events.

### 23.4 Place Names

Location names triggered by entering terrain sectors:
- **Outdoor places**: 29 entries matching sector ranges to location messages
- **Indoor places**: 31 entries matching sector ranges to interior descriptions
- Both tables scanned top-to-bottom, first match wins

### 23.5 Text Rendering

- `print(str)`: Scroll up 10px, render at bottom line
- `print_cont(str)`: Append to current line, no scroll
- `extract(start)`: Word-wrap at 37 characters, `%` substitution, CR for line break
- `ssp(str)`: Placard renderer with embedded XY positioning (byte 128 + x/2 + y)

---

## 24. Save/Load System

### 24.1 File Format

Raw sequential binary dump with no headers, no version field, no checksums. 8 slots named `A.faery` through `H.faery`.

### 24.2 Save Data Layout

| Order | Data | Size |
|-------|------|------|
| 1 | Misc variables (map_x through pad7) | 80 bytes |
| 2 | Region number | 2 bytes |
| 3 | Animation list length + padding | 6 bytes |
| 4 | Animation list | `anix × sizeof(shape)` |
| 5 | Julian's inventory | 35 bytes |
| 6 | Phillip's inventory | 35 bytes |
| 7 | Kevin's inventory | 35 bytes |
| 8 | Missile list | `6 × sizeof(missile)` |
| 9 | Extent list | `2 × sizeof(extent)` |
| 10 | Global object list | `glbobs × sizeof(object)` |
| 11 | Map object counts | 20 bytes |
| 12 | Destination object counts | 20 bytes |
| 13 | Per-region object tables (10 regions) | Variable |

### 24.3 The 80-Byte Misc Variables Block

Contiguous from `map_x`:
- `map_x, map_y, hero_x, hero_y` (8 bytes)
- `safe_x, safe_y, safe_r` (6 bytes)
- `img_x, img_y, cheat1` (6 bytes)
- `riding, flying, wcarry, turtleprox, raftprox` (10 bytes)
- `brave, luck, kind, wealth, hunger, fatigue` (12 bytes)
- `brother, princess, hero_sector, hero_place` (8 bytes)
- `daynight, lightlevel, actor_file, set_file` (8 bytes)
- `active_carrier, xtype, leader` (6 bytes)
- `secret_timer, light_timer, freeze_timer` (6 bytes)
- `cmode, encounter_type` (4 bytes)
- `pad1–pad7` (14 bytes)

### 24.4 Post-Load Cleanup

After loading: reload sprites for current region, refresh menus, force full redraw, clear encounters.

---

## 25. UI & Menu System

### 25.1 Menu Modes

10 menu modes: ITEMS (0), MAGIC (1), TALK (2), BUY (3), GAME (4), SAVEX (5), KEYS (6), GIVE (7), USE (8), FILE (9).

Modes 0–4 share a top row (Items/Magic/Talk/Buy/Game) for mode switching. Mode-specific sub-labels provide the action options.

### 25.2 Option Enable Flags

Each option has an `enabled` byte:
- Bit 0: selected/highlighted
- Bit 1: displayed/visible
- Bits 2–7: type code (0 = unchangeable, 4 = toggle, 8 = immediate, 12 = radio)

### 25.3 Menu Rendering

Options rendered in a 2-column layout:
- Even slots at x=430, odd at x=482
- Y position = (slot/2) × 9 + 8
- Up to 12 entries per menu
- Color determined by menu mode and specific option

### 25.4 Keyboard Mapping

38 keyboard shortcuts map to menu actions. Function keys F1–F7 trigger magic items. Number keys 1–7 select weapons. Letter keys map to specific menu actions (I=Items, T=Take, G=Give, Y=Yell, S=Say, etc.).

### 25.5 Compass Display

8-direction compass rendered on HUD at (567, 15). Base compass image with highlighted direction overlay. Compass table defines rectangles for each direction highlight.

### 25.6 Stats Display

HUD stats rendered via print queue:
- `prq(4)`: Vitality bar at (245, 52)
- `prq(7)`: All stats — Brv at x=14, Lck at x=90, Knd at x=168, Wlth at x=321

---

## 26. Asset Formats & Data Loading

### 26.1 Disk Image (`image` file)

Single 901120-byte file (1760 sectors × 512 bytes). Key sector allocations:

| Sectors | Content |
|---------|---------|
| 32–95 | Sector maps (outdoor regions) |
| 96–159 | Sector maps (indoor regions) |
| 149–159 | Terrain property tables |
| 160–199 | Region maps (5 ranges of 8 sectors) |
| 200–879 | Image banks (40 sectors each, 17 distinct banks) |
| 880 | Copy protection check sector |
| 896–919 | Shadow masks (24 sectors = 12288 bytes) |
| 920–930 | Sound effect samples (11 sectors = 5632 bytes) |
| 931–955 | Setfig sprites (5 sets × 5 sectors) |
| 960–1171 | Enemy + carrier sprites |
| 1312–1501 | Object, raft, turtle, and player sprites |

### 26.2 File-Based Assets

| File | Content | Format |
|------|---------|--------|
| `v6` | Waveforms (1024 bytes) + envelopes (2560 bytes) | Raw binary |
| `songs` | 28 music tracks | Custom packed tracker |
| `fonts/Amber/9` | Proportional bitmap font | Amiga hunk format |
| `hiscreen` | HUD graphics | IFF/ILBM |
| `page0` | Title screen | IFF/ILBM |
| `p1a`–`p3b` | Story page images | IFF/ILBM |
| `winpic` | Victory image | IFF/ILBM |

### 26.3 IFF/ILBM Format

Standard Amiga IFF with chunks: FORM, ILBM, BMHD, CMAP, BODY. Compression: ByteRun1 RLE (byte N ≥ 0: copy N+1 literal; byte N < 0 and ≠ −128: repeat next byte 1−N times; −128: no-op).

### 26.4 Sprite Format

5 bitplanes of image data per frame. Mask plane generated at load time by ORing all 5 image planes. Dimensions from `cfiles[]` table (width in 16px units, height in pixels).

### 26.5 Tileset Format

5-bitplane Amiga planar bitmap. 4 banks per region, 64 tiles per bank = 256 tiles total. Each bank: 40 disk sectors = 20480 bytes (5 planes × 4096 bytes/plane).

---

## 27. Special Effects

### 27.1 Witch Vision Cone

Rotating filled quadrilateral around witch position:
1. Create temporary clip layer (304×192)
2. Look up endpoints from `witchpoints[256]` sine/cosine table
3. Cross-product test: if hero within cone, take damage
4. Draw quadrilateral in COMPLEMENT mode (XOR against bitmap)
5. Rotation direction adjusts randomly toward hero

### 27.2 Teleport Colorplay

32 frames: each frame randomizes all 31 palette colors to random 12-bit RGB values. Duration ≈ 0.64 seconds.

### 27.3 Columnar Page Reveal

22-step animation for story page transitions:
1. Steps 0–10 (right half): blit vertical strips from new page at decreasing intervals
2. Steps 11–21 (left half): replace old strips with new, progressively covering
3. Per-step timing from `flip3[]` delay table
4. Each step calls page swap for intermediate display

### 27.4 Victory Sunrise Fade

55-step loop fading colors from deep blue/black through purple/red to golden tones using the `sun_colors[53]` table. Color registers 2–27 swept, registers 0/31 stay black, 1/28 forced white.

---

## Appendices

### A. Constants Reference

| Constant | Value | Description |
|----------|-------|-------------|
| MAXCOORD | 0x7FFF | Maximum X coordinate |
| MAXSHAPES | 25 | Maximum sprites per frame |
| PAGE_DEPTH | 5 | Bitplanes per game page |
| PAGE_HEIGHT | 143 | Game viewport height |
| TEXT_HEIGHT | 57 | HUD area height |
| PHANTA_WIDTH | 320 | Bitmap width with margins |
| MAGICBASE | 9 | First magic item inventory index |
| KEYBASE | 16 | First key inventory index |
| STATBASE | 25 | Gold statues inventory index |
| GOLDBASE | 31 | Gold inventory index |
| ARROWBASE | 35 | Arrow inventory index |
| EXT_COUNT | 22 | Extent entries scanned |
| DOORCOUNT | 86 | Total door entries |
| VOICE_SZ | 3584 | Audio waveform + envelope buffer size |
| SAMPLE_SZ | 5632 | Sound effect sample buffer size |
| IMAGE_SZ | 81920 | Full tileset image buffer size |
| SHADOW_SZ | 12288 | Shadow mask buffer size |
| SECTOR_SZ | 36864 | Sector + region map buffer size |

### B. Brother Stats

| Property | Julian | Phillip | Kevin |
|----------|--------|---------|-------|
| brave | 35 | 20 | 15 |
| luck | 20 | 35 | 20 |
| kind | 15 | 15 | 35 |
| wealth | 20 | 15 | 10 |
| Max HP | 23 | 20 | 18 |
| Sprite | cfiles[0] | cfiles[1] | cfiles[2] |

### C. Timing Constants

| Parameter | Value | Notes |
|-----------|-------|-------|
| Frame rate | 30 fps | Gameplay ticks (NTSC interlaced) |
| Audio VBL | 60 Hz | Audio interrupt rate |
| Day cycle | 24000 ticks | ≈ 6.7 minutes real time |
| Hunger tick | 128 ticks | ≈ 2.1 seconds |
| Health regen | 1024 ticks | ≈ 17 seconds |
| Safe zone check | 128 ticks | Same as hunger tick |
| Sleep advance | 64 ticks/frame | 63 extra + 1 normal |
| Default tempo | 150 | Timeclock counts per VBL |
