# The Faery Tale Adventure — Implementation Specification

> **Target platform:** Rust with SDL2 (via `sdl2` crate)
> **Target fidelity:** Faithful reproduction of the original 1987 Amiga game by MicroIllusions
> **Timing basis:** NTSC-only, 30 fps gameplay tick (60 Hz audio VBL)

This document synthesizes the authoritative reference material ([RESEARCH.md](RESEARCH.md), [ARCHITECTURE.md](ARCHITECTURE.md), [STORYLINE.md](STORYLINE.md)) into a single implementation-ready specification. It defines the systems, data structures, algorithms, and behaviors required to reproduce the original game.

---

## Table of Contents

1. [Display & Rendering](#1-display-rendering)
2. [World Structure](#2-world-structure)
3. [Tile & Map System](#3-tile-map-system)
4. [Scrolling & Camera](#4-scrolling-camera)
5. [Sprite System](#5-sprite-system)
6. [Terrain Masking & Z-Sorting](#6-terrain-masking-z-sorting)
7. [Color Palettes & Day/Night Fading](#7-color-palettes-daynight-fading)
8. [Characters & Animation](#8-characters-animation)
9. [Player Movement & Input](#9-player-movement-input)
10. [Combat System](#10-combat-system)
11. [AI & Behavior](#11-ai-behavior)
12. [Encounter Generation](#12-encounter-generation)
13. [NPCs & Dialogue](#13-npcs-dialogue)
14. [Inventory & Items](#14-inventory-items)
15. [Quest System](#15-quest-system)
16. [Doors & Buildings](#16-doors-buildings)
17. [Day/Night Cycle](#17-daynight-cycle)
18. [Survival Mechanics](#18-survival-mechanics)
19. [Magic System](#19-magic-system)
20. [Death & Revival](#20-death-revival)
21. [Carriers & World Navigation](#21-carriers-world-navigation)
22. [Audio System](#22-audio-system)
23. [Intro & Narrative](#23-intro-narrative)
24. [Save/Load System](#24-saveload-system)
25. [UI & Menu System](#25-ui-menu-system)
26. [Asset Formats & Data Loading](#26-asset-formats-data-loading)
27. [Special Effects](#27-special-effects)

---

## 1. Display & Rendering

### 1.1 Screen Layout

The original game uses a **non-interlaced 320×200 frame** with a mixed-resolution split display: a low-resolution playfield above a hi-resolution status bar. On the Amiga this works by changing pixel timing mid-frame via the Copper.

A faithful port should render each section at its **native resolution** first, then composite into a 640×480 presentation buffer that preserves the intended aspect ratio:

| Area | Purpose | Native Size | Presented Size in 640×480 | Original Amiga Mode |
|------|---------|-------------|----------------------------|---------------------|
| Playfield viewport | Game world | 288×140 | 576×280 (2× scale) | Low-res, 5 bitplanes (32 colors) |
| HI bar | Text, stats, menus, compass | 640×57 | 640×114 (line doubled) | Hi-res, 4 bitplanes (16 colors) |
| Inter-panel gap | Blank separator | 3 lines | 6 pixels | Non-interlaced spacer |

The composed game view occupies **400 vertical pixels** of the 640×480 canvas and should be centered vertically: **40 px top margin**, then the 280 px playfield, 6 px gap, 114 px HI bar, and **40 px bottom margin**. The 576 px-wide playfield is horizontally centered within the 640 px frame.

Key constants:
- `PAGE_DEPTH` = 5 (bitplanes per game page)
- `TEXT_DEPTH` = 4 (bitplanes per status bar)
- `SCREEN_WIDTH` = 288 (visible playfield width in pixels)
- `PHANTA_WIDTH` = 320 (bitmap width including 16px scroll margin on each side)
- `RAST_HEIGHT` = 200 (original non-interlaced frame height)
- `PAGE_HEIGHT` = 143 (scanline where text viewport begins; game viewport occupies scanlines 0–139 with ~3 blank lines before the text bar)
- `TEXT_HEIGHT` = 57 (native HUD/HI bar height before line doubling)

The playfield viewport has `DxOffset = 16`, `DyOffset = 0`, making the 288-pixel visible window centered within the 320-pixel raster. The scroll margin allows smooth tile scrolling without exposing unrendered edges.

### 1.2 Double Buffering

Two offscreen buffers alternate roles each frame, tracked by `struct fpage` instances (`fp_page1`, `fp_page2`):

- **Drawing page** (`fp_drawing`): Actively being rendered to (off-screen).
- **Viewing page** (`fp_viewing`): Currently displayed on screen.

Each page maintains:
- Last-drawn tile scroll position (`isv_x`, `isv_y`)
- Sprite count for the frame (`obcount`)
- Shape queue for background save/restore (up to `MAXSHAPES` = 25 entries, 10 bytes each via `sshape` struct)
- Background save buffer for sprite compositing undo (up to 5920 bytes)
- Witch FX position state (`witchx`, `witchy`, `witchdir`, `wflag`)

The `struct fpage` layout:

| Field | Type | Purpose |
|-------|------|---------|
| `isv_x` | i32 | Page scroll X position |
| `isv_y` | i32 | Page scroll Y position |
| `obcount` | i16 | Number of objects queued for rendering |
| `shape_queue` | ptr | Pointer to rendering shape queue (25 × `sshape`) |
| `backsave` | ptr | Background save buffer |
| `saveused` | i32 | How much of save buffer is used |
| `witchx` | i16 | Witch effect X position (for erasure) |
| `witchy` | i16 | Witch effect Y position |
| `witchdir` | i16 | Witch effect direction |
| `wflag` | i16 | Witch effect active flag |

Page swap presents the drawing page to the screen, then the pages exchange roles. Each page caches its compiled display list (copper list equivalent) in `savecop`, avoiding full recompilation every frame.

### 1.3 Rendering Pipeline Per Frame

The 24 main-loop phases relevant to rendering execute in this order:

1. **Phase 13 — Map & Scroll**: Set draw target, restore previous frame's sprite backgrounds (reverse order via `rest_blit`), undo witch FX, compute scroll delta. If `MAP_FLUX`: call `load_next()` for incremental region loading. Full redraw for `viewstatus` 99/98/3; incremental `scrollmap()` for ±1 tile deltas.
2. **Phase 14d — Day Fade**: `day_fade()` adjusts palette (only on non-scrolling frames).
3. **Phase 17 — Object Processing**: `do_objects()` processes world objects and sets up display entries.
4. **Phase 18 — Missile Setup**: Active missiles added to `anim_list` as OBJECTS type.
5. **Phase 19 — Sprite Sorting**: Bubble sort of `anim_index[]` by Y-coordinate for painter's algorithm. Dead actors sort at Y−32, sinking actors at Y+32.
6. **Phase 20 — Map Strip Repair**: After scroll, `strip_draw()` for new columns, `row_draw()` for new rows.
7. **Phase 21 — Witch FX**: Set witch distortion parameters; if active and within 100 px, deal damage.
8. **Phase 22 — Sprite Rendering**: For each sorted actor (back-to-front):
   a. Determine type, frame, and screen position
   b. Apply sinking/riding/death visual adjustments
   c. Clip to viewport bounds
   d. `save_blit()` — save background under sprite footprint
   e. `maskit()` × N — build terrain occlusion mask
   f. `mask_blit()` — combine sprite mask with terrain occlusion
   g. `shape_blit()` — cookie-cut composite sprite onto drawing page
   h. If sprite has weapon overlay, repeat for weapon pass
9. **Phase 23 — Page Flip**: Set scroll offsets (`RxOffset`, `RyOffset`), call `pagechange()` — swap drawing/viewing pages, rebuild display list, wait for vertical blank (`WaitBOVP`).
10. **Phase 24 — Fade Completion**: If `viewstatus == 3`: `fade_normal()`, then `viewstatus = 0`.

### 1.4 Transparency

Color 31 (all 5 bitplanes set = binary `11111`) is the transparency color. The mask generation function ORs all 5 bitplanes together and inverts — a pixel is transparent only when all planes are 1. Where the mask is 1 (sprite visible), the sprite pixel is drawn; where the mask is 0 (occluded or transparent), the background is preserved.

### 1.5 Hardware Pointer

A single hardware sprite (sprite 0) is used for the mouse pointer. The pointer sprite is enabled in the status bar viewport via the `SPRITES` display flag. Mouse position is clamped to X: 5–315, Y: 147–195.

---


## 2. World Structure

### 2.1 Coordinate System

- Full world coordinate space: X range 0–32767 (`MAXCOORD` = 0x7FFF), Y range 0–40959 (0x9FFF)
- Coordinates wrap at world boundaries (the world is a torus)
- Tile width: 16 pixels, tile height: 32 pixels
- Sub-tile viewport offsets: `RxOffset = map_x & 15` (0–15), `RyOffset = map_y & 31` (0–31)

Coordinate hierarchy:

| Level | Variable(s) | Conversion | Range |
|-------|------------|------------|-------|
| Pixel | `map_x`, `map_y` | — | 0–32767 (X), 0–40959 (Y) |
| Tile | `img_x`, `img_y` | `map_x >> 4`, `map_y >> 5` | — |
| Sector | — | `(tile_x >> 4) - xreg`, `(tile_y >> 3) - yreg` | 0–63 / 0–31 |
| Region | `region_num` | `(sector_x >> 6) & 1 + ((sector_y >> 5) & 3) * 2` | 0–9 |

### 2.2 Region System

The world is divided into **10 regions**:
- Regions 0–7: Outdoor overworld arranged in a 2×4 grid
- Region 8: Building interiors
- Region 9: Dungeon interiors

Each region has its own asset configuration defined in the `file_index[10]` table using `struct need`:

| Field | Type | Purpose |
|-------|------|---------|
| `image[4]` | u16[4] | 4 image bank file indices |
| `terra1` | u16 | Terrain data file 1 |
| `terra2` | u16 | Terrain data file 2 |
| `sector` | u16 | Sector data file |
| `region` | u16 | Region data file |
| `setchar` | u16 | Set-character file needed |

Region index mapping:

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

### 2.3 Two-Level Map Hierarchy

1. **Region map** (`map_mem`, 4 KB): 128×32 grid of sector indices. Each outdoor region occupies a 64-wide × variable-high band.
2. **Sector data** (`sector_mem`, 32 KB): 256 sectors × 128 bytes each. Each sector is a **16×8 grid of tile indices**.

The `genmini` function resolves world pixel coordinates through this hierarchy: pixel → tile → sector (via region map) → tile index (via sector data), filling the 19×6 `minimap[]` buffer that `map_draw` renders directly.

### 2.4 World Objects

Each world object uses `struct object` (6 bytes):

| Field | Type | Size | Purpose |
|-------|------|------|---------|
| `xc` | u16 | 2 | World X coordinate |
| `yc` | u16 | 2 | World Y coordinate |
| `ob_id` | u8 | 1 | Object type ID |
| `ob_stat` | u8 | 1 | Status (0=inactive, 1+=active) |

Up to 250 objects per sector. Two arrays: `ob_listg[]` (global outdoor objects) and `ob_list8[]` (indoor objects).

### 2.5 Region Loading

Region transitions are triggered by outdoor boundary crossing (`gen_mini`), door transitions, or respawn.

- `MAP_FLUX` / `MAP_STABLE`: `new_region < NO_REGION(10)` means transition in progress.
- `load_all()`: Blocking loop — `while (MAP_FLUX) load_new_region()`.
- `load_next()`: Non-blocking incremental loader called during Phase 13 of the main loop.
- `load_new_region()`: Loads sector data, region map, terrain blocks, and 5 image planes incrementally. Desert gate: if `new_region == 4` and `stuff[STATBASE] < 5`, desert map squares are blocked.

---


## 3. Tile & Map System

### 3.1 Tileset Structure

Each region's tileset comprises 256 tiles, organized into 4 banks of 64 tiles each. Tiles are **16×32 pixels**, 32 colors (5 bitplanes). Each tile requires 64 bytes per bitplane (16 px ÷ 8 = 2 bytes/row × 32 rows).

Image banks are loaded from disk as 40 sectors each (20480 bytes = 5 planes × 4096 bytes/plane). Four banks compose a complete 256-tile image set totaling 81920 bytes (`IMAGE_SZ`).

256 tiles × 64 bytes/plane × 5 planes = 81920 bytes in `image_mem`.

### 3.2 Terrain Properties

Terrain data is loaded as two 512-byte halves (one per terrain table ID) into a 1024-byte buffer (`terra_mem`). Each tile has a terrain entry; the terrain entry format for each tile provides:
- Byte 0: mask shape index (index into shadow mask data for terrain occlusion)
- Byte 1 lower nibble (bits 0–3): terrain walkability (0–3 scale, where 0 = fully walkable)
- Byte 1 upper nibble (bits 4–7): mask mode (0–7, controlling occlusion behavior)

### 3.3 Map Rendering

The visible playfield displays a **19×6 grid of tiles** (each 16×32 pixels):
- Width: 19 tiles × 16 px = 304 pixels (fits within 320-pixel raster with scroll margin)
- Height: 6 tiles × 32 px = 192 scanlines (fits within 200-scanline raster)

`map_draw()` renders the full visible map area from tile data to the offscreen bitmap using CPU register-to-register moves (not blitter). For incremental scrolling:
- `strip_draw()` renders a single column of tiles
- `row_draw()` renders a single row of tiles

A **minimap** array (19×6 = 114 entries) caches the mapping from viewport tile positions to terrain tile IDs for fast terrain mask lookups during sprite compositing.

---


## 4. Scrolling & Camera

### 4.1 Two-Level Scrolling Architecture

The renderer supports two distinct scrolling mechanisms:

1. **Continuous viewport drift**: Every frame, sub-tile pixel offsets `RxOffset = map_x & 15` and `RyOffset = map_y & 31` are applied. This provides smooth pixel-level camera movement within the already-loaded 19×6 tile window.

2. **Incremental tile scroll**: Only when tile coordinates change (`img_x = map_x >> 4` or `img_y = map_y >> 5`) does the engine shift bitmap contents and repair exposed edges.

### 4.2 Tile-Level Scrolling

Each frame, compute scroll delta in tile coordinates:
```
dif_x = (map_x >> 4) - last_drawn_tile_x
dif_y = (map_y >> 5) - last_drawn_tile_y
```

| Condition | Action |
|-----------|--------|
| `viewstatus == 99/98` | Full redraw: `gen_mini()` + `map_draw()` |
| `dif_x/dif_y == ±1` | Incremental tile scroll via `scrollmap()` + edge repair |
| `dif_x == 0, dif_y == 0` | No tile scroll — game logic sub-block (Phase 14) executes |
| Large delta | Fallback to full `map_draw()` |

**Tile scrolling** (`scrollmap`): Shifts all 5 bitplanes by one tile in any of 8 directions using a straight copy operation (equivalent to blitter `BLTCON0 = $09F0`, A→D). Exposed edges are then filled by `strip_draw()` (one column) or `row_draw()` (one row).

### 4.3 Scroll-Gated Game Logic

The most significant architectural characteristic: AI processing, encounter spawning, hunger/fatigue, day/night advancement, and most game logic (Phase 14) **only execute on frames where the map did not scroll**. During continuous scrolling, only actor movement/animation, combat, and rendering occur. This naturally reduces computational load during scrolling but means a walking player has slower hunger progression and fewer encounter checks than a stationary one.

### 4.4 Display State Machine

The `viewstatus` flag controls rendering behavior:

| Value | State | Behavior |
|-------|-------|----------|
| 0 | Normal | Standard rendering |
| 1 | Picking (dialogue) | Skips rest of tick via `continue`, flasher blink active |
| 2 | Map message | Full-screen text overlay mode |
| 3 | Fade-in | After next `pagechange()`, fires `fade_normal()`, then transitions to 0 |
| 4 | Picking (alt) | Same as 1, skips rest of tick |
| 98 | Rebuild | Full map redraw |
| 99 | Rebuild (init) | Full map redraw |

---


## 5. Sprite System

### 5.1 Actor Slots

Up to **20 actor slots** (`anim_list[20]`), with hardcoded role assignments:

| Slot | Role |
|------|------|
| 0 | Hero (player character) — always active |
| 1–2 | Party members / carriers |
| 3–6 | Enemy actors (up to 4; `anix` tracks active count, max 7) |
| 7–19 | World objects and set-figures |

A separate 20-element `anim_index[]` array is used for depth-sort indexing. The active actor count `anix` tracks how many slots are in use. `anix2` tracks the total including missile sprites (max 20). `mdex` tracks the missile index.

### 5.2 Sprite Data Format

Sprites are stored as 5 bitplanes of image data, loaded contiguously into `shape_mem` (78000 bytes). A 1-bit mask plane is generated at load time by ORing all 5 image planes together and inverting — a pixel is transparent only when all planes are 1 (color 31).

Dimensions are defined per sprite set via `struct seq_info`:

| Field | Type | Purpose |
|-------|------|---------|
| `width` | i16 | Frame width in pixels |
| `height` | i16 | Frame height in pixels |
| `count` | i16 | Number of frames |
| `location` | ptr | Pointer to image data in `shape_mem` |
| `maskloc` | ptr | Pointer to mask data in `shape_mem` |
| `bytes` | i16 | Bytes per frame (one plane) |
| `current_file` | i16 | Currently loaded file index |

Seven sprite sequence slots (`seq_list[7]`):

| Index | Type | Width | Height | Frames | Description |
|-------|------|-------|--------|--------|-------------|
| 0 | PHIL | 16px | 32px | 67 | Player character (Julian/Phillip/Kevin) |
| 1 | OBJECTS | 16px | 16px | 116 | Items, effects, fairy |
| 2 | ENEMY | 16px | 32px | 64 | Ogre/ghost/spider/etc. |
| 3 | RAFT | 32px | 32px | 2 | Raft/grounded vehicle |
| 4 | SETFIG | 16px | 32px | 8 | NPCs (wizard, priest, etc.) |
| 5 | CARRIER | 32px/64px | 32px/64px | 16/8 | Turtle (32×32, 16 frames) or Swan (64×64, 8 frames) |
| 6 | DRAGON | 48px | 40px | 5 | Dragon |

Frame addressing for frame `inum` of type `atype`:
```
planesize = seq_list[atype].bytes
image_ptr = seq_list[atype].location + (planesize × 5 × inum)
mask_ptr  = seq_list[atype].maskloc  + (planesize × inum)
```

### 5.3 The `struct shape` (Actor State)

Each actor occupies 22 bytes with 17 named fields:

| Offset | Size | Field | Rust Type | Description |
|--------|------|-------|-----------|-------------|
| 0 | 2 | `abs_x` | u16 | Absolute world X coordinate |
| 2 | 2 | `abs_y` | u16 | Absolute world Y coordinate |
| 4 | 2 | `rel_x` | u16 | Screen-relative X position |
| 6 | 2 | `rel_y` | u16 | Screen-relative Y position |
| 8 | 1 | `type` | u8 | Sprite type (PHIL=0, OBJECTS=1, ENEMY=2, RAFT=3, SETFIG=4, CARRIER=5, DRAGON=6) |
| 9 | 1 | `race` | u8 | Race/NPC type ID (indexes `encounter_chart[]`) |
| 10 | 1 | `index` | u8 | Current animation frame image index |
| 11 | 1 | `visible` | u8 | On-screen visibility flag |
| 12 | 1 | `weapon` | u8 | Weapon type: 0=none, 1=dagger, 2=mace, 3=sword, 4=bow, 5=wand |
| 13 | 1 | `environ` | u8 | Environment/terrain state (water depth, sinking) |
| 14 | 1 | `goal` | u8 | Current AI goal mode (0–10) |
| 15 | 1 | `tactic` | u8 | Current AI tactical mode (0–12) |
| 16 | 1 | `state` | u8 | Motion/animation state (one of 26 states) |
| 17 | 1 | `facing` | u8 | Direction facing (0–7) |
| 18 | 2 | `vitality` | i16 | Hit points (doubles as original object index for NPCs) |
| 20 | 1 | `vel_x` | i8 | X velocity (slippery/ice physics) |
| 21 | 1 | `vel_y` | i8 | Y velocity (slippery/ice physics) |

### 5.4 Missile System

Up to 6 simultaneous missiles tracked via `missile_list[6]`:

| Field | Type | Purpose |
|-------|------|---------|
| `abs_x` | u16 | World X coordinate |
| `abs_y` | u16 | World Y coordinate |
| `missile_type` | u8 | NULL, arrow, rock, 'thing', or fireball |
| `time_of_flight` | u8 | Flight counter; expires at > 40 |
| `speed` | u8 | Movement speed (0 = unshot) |
| `direction` | u8 | Flight direction (0–7) |
| `archer` | u8 | ID of firing actor |

Active missiles are added to `anim_list` as OBJECTS type during Phase 18. Hit detection: arrows use `dohit(-1,...)`, fireballs use `dohit(-2,...)`.

---


## 6. Terrain Masking & Z-Sorting

### 6.1 Z-Sorting

Sprites are sorted back-to-front by Y-coordinate using a **bubble sort** of `anim_index[]` before rendering. Depth adjustments:
- Dead actors: Y − 32 (render behind living sprites)
- Riding hero: Y − 32 (mount doesn't obscure ground characters)
- Actor slot 1 (mount/companion): Y − 32
- Deeply sunk actors (environ > 25): Y + 32 (render in front)

The sort also identifies `nearest_person` for NPC proximity interaction.

### 6.2 Sprite Compositing Pipeline

Sprites are rendered using a 4-stage pipeline that implements cookie-cut compositing with terrain occlusion:

```
1. save_blit    → Save background: Screen → backsave
2. maskit × N   → Stamp terrain masks: shadow_mem → compositing buffer
3. mask_blit    → Combine sprite mask with terrain occlusion
4. shape_blit   → Cookie-cut draw: Sprite → screen
```

| Stage | Operation | Miniterm Logic |
|-------|-----------|----------------|
| Save background | `save_blit` | D = B (copy screen to backsave) |
| Terrain occlusion | `maskit` | CPU copy of shadow mask data |
| Combine masks | `mask_blit` | D = A AND NOT(C) — sprite mask minus terrain occlusion |
| Cookie-cut draw | `shape_blit` | D = (A·B) + (¬A·C) — where A=mask, B=sprite, C=background |

The cookie-cut formula: where the compositing mask is 1 (sprite visible), take the sprite pixel; where the mask is 0 (occluded or transparent), keep the background.

Background restoration (`rest_blit`) runs in **reverse compositing order** to correctly rebuild overlapping backgrounds. Maximum sprites per frame: `MAXSHAPES` = 25, limited by backsave buffer capacity (5920 bytes per page).

### 6.3 Terrain Masking

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

4. Write the mask shape from `shadow_mem` into the per-frame compositing buffer
5. Combine with sprite transparency mask during `mask_blit`

Sprites that **skip masking entirely**: carriers, arrows, fairy sprites (object indices 100–101), certain NPC races.

---


## 7. Color Palettes & Day/Night Fading

### 7.1 Palette Definitions

Five palettes are used:

| Palette | Size | Purpose |
|---------|------|---------|
| `pagecolors[32]` | 32 × 12-bit Amiga RGB | Game world palette; same for all outdoor regions (0–7); faded dynamically by `fade_page()` |
| `textcolors[20]` | 20 × 12-bit Amiga RGB | Status bar palette; NOT affected by day/night fading |
| `introcolors[32]` | 32 × 12-bit Amiga RGB | Title/intro screen palette, separate from gameplay; used during `screen_size()` zoom animation |
| `blackcolors[32]` | 32 × all zeros | Instant blackout transitions |
| `sun_colors[53]` | 53 × 12-bit Amiga RGB | Sunrise/sunset gradient for the victory sequence |

12-bit to 24-bit conversion: multiply each 4-bit channel by 17 (0x11).

### 7.2 Day/Night Fading — `day_fade()`

Called every non-scrolling tick from Phase 14d:

```
day_fade():
    if light_timer > 0: ll = 200; else: ll = 0
    if (daynight & 3) == 0 OR viewstatus > 97:
        if region_num < 8:
            fade_page(lightlevel - 80 + ll, lightlevel - 61, lightlevel - 62, TRUE, pagecolors)
        else:
            fade_page(100, 100, 100, TRUE, pagecolors)
```

Key behaviors:
- **Green Jewel light bonus**: `light_timer > 0` adds 200 to the red parameter (warm amber glow)
- **Update rate**: Every 4 ticks (`daynight & 3 == 0`) or during screen rebuild (`viewstatus > 97`)
- **Indoor override**: `region_num >= 8` → always full brightness (100, 100, 100) with no day/night variation

Outdoor RGB at key times of day:

| Phase | lightlevel | Red (no jewel) | Green | Blue |
|-------|------------|----------------|-------|------|
| Midnight (daynight 0) | 0 | clamped 10 | clamped 25 | clamped 60 |
| Dawn (daynight 6000) | 150 | 70 | 89 | 88 |
| Noon (daynight 12000) | 300 | clamped 100 | clamped 100 | clamped 100 |

With Green Jewel active: midnight red = 120 (warm amber tone even in darkness).

### 7.3 Palette Scaling — `fade_page(r, g, b, limit, colors)`

Per-component palette scaler:

**Step 1 — Color 31 override**:

| Region | Color 31 | Meaning |
|--------|----------|---------|
| 4 (desert) | `0x0980` | Orange-brown desert sky |
| 9 (dungeon), `secret_timer` active | `0x00F0` | Bright green (secret revealed) |
| 9 (dungeon), normal | `0x0445` | Dark grey-blue |
| All others | `0x0BDF` | Light blue sky |

**Step 2 — Clamping** (when `limit = TRUE`):
- Red: min 10, max 100
- Green: min 25, max 100
- Blue: min 60, max 100
- Blue shift factor: `g2 = (100 - green_pct) / 3`

**Step 3 — Per-color computation**: For each of 32 palette entries, extract 12-bit RGB components from `colors[]`, then:

1. **Green Jewel light boost**: if `light_timer` active and color's red < green, boost red to match green
2. **Scale channels**:
   ```
   r_out = (r_pct × r_raw) / 1600
   g_out = (g_pct × g_raw) / 1600
   b_out = (b_pct × b_raw + g2 × g_out) / 100
   ```
3. **Twilight vegetation boost** (colors 16–24 only):
   - green% 21–49: add 2 to blue channel
   - green% 50–74: add 1 to blue channel
4. Cap all channels at maximum (15 for 12-bit, 255 for 24-bit)

Result written to `fader[]` and loaded as the active palette.

### 7.4 Screen Transitions — `fade_down()` / `fade_normal()`

- **`fade_down()`**: Steps all channels from 100 to 0 in decrements of 5 (21 steps, `Delay(1)` each). Fades screen to black. Uses `limit = FALSE` — no night clamping or blue shift.
- **`fade_normal()`**: Steps all channels from 0 to 100 in increments of 5. Fades back to full brightness. Also uses `limit = FALSE`.

Used for map messages, door transitions, story placards, and other screen changes.

### 7.5 Teleportation Palette Effect

`colorplay()`: Rapidly sets palette entries 1–31 to random 12-bit values for 32 frames (~0.5 seconds). Entry 0 (background color) is preserved. Creates a psychedelic flash during teleportation.

---


## 8. Characters & Animation

### 8.1 Actor Record (`struct shape`)

The fundamental actor record, 22 bytes total, used for player, NPCs, and enemies:

| Offset | Size | Field | Type | Purpose |
|--------|------|-------|------|---------|
| 0 | 2 | `abs_x` | u16 | Absolute world X coordinate |
| 2 | 2 | `abs_y` | u16 | Absolute world Y coordinate |
| 4 | 2 | `rel_x` | u16 | Screen-relative X position |
| 6 | 2 | `rel_y` | u16 | Screen-relative Y position |
| 8 | 1 | `type` | u8 | Object type number |
| 9 | 1 | `race` | u8 | Race (indexes `encounter_chart[]`) |
| 10 | 1 | `index` | u8 | Current animation frame image index |
| 11 | 1 | `visible` | u8 | On-screen visibility flag |
| 12 | 1 | `weapon` | u8 | Weapon: 0=none, 1=dirk, 2=mace, 3=sword, 4=bow, 5=wand, 8=touch |
| 13 | 1 | `environ` | i8 | Environment/terrain state |
| 14 | 1 | `goal` | u8 | Current goal mode (§11.1) |
| 15 | 1 | `tactic` | u8 | Current tactical mode (§11.2) |
| 16 | 1 | `state` | u8 | Motion/animation state (§8.2) |
| 17 | 1 | `facing` | u8 | Direction facing (0–7) |
| 18 | 2 | `vitality` | i16 | Hit points |
| 20 | 1 | `vel_x` | i8 | X velocity (ice/slippery physics) |
| 21 | 1 | `vel_y` | i8 | Y velocity (ice/slippery physics) |

#### Actor Array Layout

- `anim_list[0]` — player-controlled hero
- `anim_list[1]` — raft (always present) / party member
- `anim_list[2]` — NPC set-piece figure
- `anim_list[3–6]` — enemy actors (up to 4; `anix` tracks count, max 7)
- `anim_list[7–19]` — remaining slots for world objects and set-figures

`MAXSHAPES = 25` governs the per-page rendering queue, not the actor array size (20 entries).

### 8.2 Motion States

26 animation states stored in `shape.state`:

| Value | Name | Purpose |
|-------|------|---------|
| 0–11 | *(fighting frames)* | Combat animation sub-states; `statelist[facing*12 + state]` selects figure |
| 12 | WALKING | Normal walk cycle |
| 13 | STILL | Stationary/idle |
| 14 | DYING | Death animation in progress |
| 15 | DEAD | Fully dead |
| 16 | SINK | Sinking (quicksand/water) |
| 17 | OSCIL | Oscillation anim 1 — vestigial, never assigned |
| 18 | *(implicit)* | Oscillation anim 2 — vestigial, never assigned |
| 19 | TALKING | Speaking/dialogue |
| 20 | FROZEN | Frozen in place (freeze spell) |
| 21 | FLYING | Vestigial — defined but never assigned; swan uses WALKING + `riding` |
| 22 | FALL | Falling; velocity decays 25% per tick |
| 23 | SLEEP | Sleeping |
| 24 | SHOOT1 | Bow up — aiming |
| 25 | SHOOT3 | Bow fired, arrow given velocity |

### 8.3 Direction System

8 compass directions plus 2 stop values. Direction vectors defined by `xdir[10]` / `ydir[10]`:

| Value | Direction | xdir | ydir |
|-------|-----------|------|------|
| 0 | NW | −2 | −2 |
| 1 | N | 0 | −3 |
| 2 | NE | +2 | −2 |
| 3 | E | +3 | 0 |
| 4 | SE | +2 | +2 |
| 5 | S | 0 | +3 |
| 6 | SW | −2 | +2 |
| 7 | W | −3 | 0 |
| 8 | Still | 0 | 0 |
| 9 | Still | 0 | 0 |

Cardinals have magnitude 3, diagonals 2 per axis (displacement √8 ≈ 2.83), yielding near-parity between cardinal and diagonal speed.

Walk base offsets via `diroffs[16]`:

```
diroffs[16] = {16,16,24,24,0,0,8,8,56,56,68,68,32,32,44,44}
```

Indices 0–7 select walk animation bases; indices 8–15 select fight/shoot bases.

### 8.4 `statelist[87]` — Animation Frame Lookup

Maps `(motion_state, facing, frame)` → `(figure_image, weapon_overlay_index, weapon_x_offset, weapon_y_offset)`:

```rust
struct State { figure: i8, wpn_no: i8, wpn_x: i8, wpn_y: i8 }
```

#### Walk Sequences (8 frames each)

| Index Range | Direction |
|-------------|-----------|
| 0–7 | South |
| 8–15 | West |
| 16–23 | North |
| 24–31 | East |

#### Fight Sequences (12 states each)

| Index Range | Direction |
|-------------|-----------|
| 32–43 | South |
| 44–55 | West |
| 56–67 | North |
| 68–79 | East |

Each 12-entry block: states 0–8 = weapon swing positions, state 9 = duplicate swing, states 10–11 = ranged attack frames.

#### Special States

| Index | Purpose |
|-------|---------|
| 80–82 | Death sequence (3 frames) |
| 83 | Sinking |
| 84–85 | Oscillation (2 frames) |
| 86 | Asleep |

### 8.5 Combat Animation FSA — `trans_list[9]`

Nine `struct transition` entries controlling fight swing animation:

```rust
struct Transition { newstate: [i8; 4] }
```

| Index | newstate[0] | [1] | [2] | [3] |
|-------|-------------|-----|-----|-----|
| 0 | 1 | 8 | 0 | 1 |
| 1 | 2 | 0 | 1 | 0 |
| 2 | 3 | 1 | 2 | 8 |
| 3 | 4 | 2 | 3 | 7 |
| 4 | 5 | 3 | 4 | 6 |
| 5 | 6 | 4 | 5 | 5 |
| 6 | 8 | 5 | 6 | 4 |
| 7 | 8 | 6 | 7 | 3 |
| 8 | 0 | 6 | 8 | 2 |

Forward cycle via `newstate[0]`: 0→1→2→3→4→5→6→8→0 (state 7 reached via other paths). Each tick: `trans_list[state].newstate[rand4()]`. Monsters at states 6 or 7 forced to state 8.

### 8.6 Missile System

```rust
struct Missile {
    abs_x: u16, abs_y: u16,
    missile_type: u8,    // NULL, arrow, rock, 'thing', fireball
    time_of_flight: u8,
    speed: u8,           // 0 = unshot
    direction: u8,
    archer: u8,          // ID of firing actor
}
```

6 missile slots maximum. Slots assigned round-robin via `mdex`.

### 8.7 Sprite Sheet Descriptors

Sequence type constants:

| Value | Name | Purpose |
|-------|------|---------|
| 0 | PHIL | Player character sprites |
| 1 | OBJECTS | World object sprites |
| 2 | ENEMY | Enemy sprites |
| 3 | RAFT | Raft/vehicle sprites |
| 4 | SETFIG | Set-piece figure sprites (NPCs) |
| 5 | CARRIER | Carrier animal sprites |
| 6 | DRAGON | Dragon sprites |

### 8.8 NPC Type Descriptors — `setfig_table[14]`

| Index | NPC Type | cfile_entry | image_base | can_talk |
|-------|----------|-------------|------------|----------|
| 0 | Wizard | 13 | 0 | 1 |
| 1 | Priest | 13 | 4 | 1 |
| 2 | Guard (front) | 14 | 0 | 0 |
| 3 | Guard (back) | 14 | 1 | 0 |
| 4 | Princess | 14 | 2 | 0 |
| 5 | King | 14 | 4 | 1 |
| 6 | Noble | 14 | 6 | 0 |
| 7 | Sorceress | 14 | 7 | 0 |
| 8 | Bartender | 15 | 0 | 0 |
| 9 | Witch | 16 | 0 | 0 |
| 10 | Spectre | 16 | 6 | 0 |
| 11 | Ghost | 16 | 7 | 0 |
| 12 | Ranger | 17 | 0 | 1 |
| 13 | Beggar | 17 | 4 | 1 |

`cfile_entry` selects the image file. `image_base` is the sub-image offset. `can_talk=1` enables generic dialogue initiation.

---


## 9. Player Movement & Input

### 9.1 Input Sources (Priority Order)

1. **Mouse/compass click** (highest): when either button held (`qualifier & 0x6000`), cursor X > 265 maps position to 3×3 compass grid. X ≤ 265 → direction 9 (menu area, no movement).
2. **Joystick**: JOY1DAT register at `$dff00c` decoded via XOR of adjacent bits per axis, then `com2[4 + yjoy*3 + xjoy]`.
3. **Keyboard** (lowest): stored `keydir` value from numpad keys (codes 20–29). Direction = `keydir − 20`.

Direction lookup table `com2[9]`:

```
com2 = {0, 1, 2, 7, 9, 3, 6, 5, 4}
```

| yjoy\\xjoy | −1 | 0 | +1 |
|---|---|---|---|
| −1 | 0 (NW) | 1 (N) | 2 (NE) |
| 0 | 7 (W) | 9 (stop) | 3 (E) |
| +1 | 6 (SW) | 5 (S) | 4 (SE) |

### 9.2 Input Handler Architecture

The input handler installs at priority 51 (above Intuition's 50). Handler data in `struct in_work` (150+ bytes):

- `xsprite`/`ysprite`: mouse position, clamped X: 5–315, Y: 147–195 (confines pointer to 48-pixel status bar)
- `keybuf[128]`: circular keyboard FIFO with `laydown`/`pickup` pointers (`& 0x7F` wrap)
- `ticker`: heartbeat counter 0–16; at 16, synthesizes fake key event `$E0` to prevent stalls
- `qualifier`: button/modifier state word

Event processing:
- **TIMER** (type 6): increments ticker; at 16, generates synthetic RAWKEY
- **RAWKEY** (type 1): ignores repeats (qualifier bit 9); ignores scancodes > `$5A`; nullifies event (type=0); translates via `keytrans[]`; queues in circular buffer
- **RAWMOUSE** (type 2): XOR detects button transitions; left press in menu area (X: 215–265) computes character code; left press outside menu → direction 9
- **DISKIN** (type $10): sets `newdisk = 1`
- **All events**: apply mouse delta, clamp position, call MoveSprite if `pbase ≠ NULL`

### 9.3 Keyboard Translation

`keytrans[91]` maps Amiga raw scancodes to game-internal codes:

**Numpad direction codes (20–29)**:

```
7=NW(20)   8=N(21)    9=NE(22)
4=W(27)    5=stop(29)  6=E(23)
1=SW(26)   2=S(25)    3=SE(24)
```

Cursor keys ($4C–$4F) → values 1–4 (cheat movement only, NOT direction codes).
Function keys F1–F10 ($50–$59) → values 10–19.

### 9.4 Fight Detection

Combat stance activates when any of:
- Right mouse button held: `qualifier & 0x2000`
- Keyboard numpad-0 held: `keyfight` flag (set on key-down, cleared on key-up)
- Joystick fire button: CIA-A PRA register `$bfe001` bit 7 == 0 (active low, bypasses input.device)

Melee weapon → `state = FIGHTING`; ranged weapon (bow/wand) → `state = SHOOT1`.

Walk trigger: `qualifier & 0x4000` (left mouse) OR `keydir != 0`.

### 9.5 Movement Speed by Terrain

Speed value `e` passed to `newx`/`newy`:

| Condition | Speed | Effect |
|-----------|-------|--------|
| Riding raft (`riding == 5`) | 3 | Fast overland |
| `environ == −3` (terrain 8, direction reversal) | −2 | Backward movement (near Necromancer) |
| `environ == −2` (terrain 7, ice) | N/A | Velocity-based physics |
| `environ == −1` (terrain 6, slippery) | 4 | Fast terrain |
| `environ == 2` or `> 6` (wading/deep water) | 1 | Slow |
| Default | 2 | Normal walking |

Non-hero actors: speed 1 in water, 2 otherwise (same environ exceptions).

Negative speed (−2) causes backward movement — the signed multiply in `newx`/`newy` handles inversion.

### 9.6 Position Update — `newx` / `newy`

```
newx(x, dir, speed):
    if dir > 7: return x
    return (x + (xdir[dir] * speed) >> 1) & 0x7FFF

newy(x, dir, speed):
    same as newx using ydir[], plus preserves bit 15 of original y
```

The `>> 1` is a logical right shift. The `& 0x7FFF` clamps to 15-bit range [0, 32767], providing implicit world wrapping.

### 9.7 Velocity System

#### Ice Physics (environ == −2)

```
vel_x += xdir[dir]
vel_y += ydir[dir]
clamp |vel| to 42 (40 on swan)
position += vel / 4
facing derived from velocity: set_course(0, −vel_x, −vel_y, 6)
```

#### Normal Walking — Velocity Recording

After each non-ice movement: `vel = (new_pos − old_pos) * 4`. Feeds swan dismount check: dismount only when `|vel_x| < 15 && |vel_y| < 15`.

#### FALL State Friction

```
vel_x = (vel_x * 3) / 4
vel_y = (vel_y * 3) / 4
```

Velocity halves approximately every 3 ticks. Position continues updating by `vel / 4`.

### 9.8 Collision Deviation

When player movement is terrain-blocked:
1. Try `dir + 1` (clockwise) — if clear, commit
2. Try `dir − 2` (counterclockwise from original) — if clear, commit
3. All three blocked: `frustflag++`
   - At `frustflag > 20`: scratching-head animation
   - At `frustflag > 40`: special animation index 40

### 9.9 World Wrapping

Outdoor regions (`region_num < 8`): hero coordinates wrap toroidally at 300 and 32565:

```
if abs_x < 300:      abs_x = 32565
else if abs_x > 32565: abs_x = 300
else if abs_y < 300:  abs_y = 32565
else if abs_y > 32565: abs_y = 300
```

Indoor regions do not wrap. NPCs are never wrapped.

### 9.10 Camera Tracking — `map_adjust`

Dead zone: ±20 pixels X, ±10 pixels Y. Outside dead zone: scroll 1 pixel per tick.

Large jump thresholds: > 70 pixels X, > 44 pixels Y (downward), > 24 pixels Y (upward) → snap immediately. The asymmetric Y thresholds account for the player sprite being offset from screen center.

### 9.11 Hunger Stumble

When `hunger > 120`: 1/4 chance per walking tick of deflecting direction by ±1 (50/50 via `rand() & 1`), wrapped with `& 7`.

### 9.12 Keyboard Shortcuts — `letter_list[38]`

| Key | Menu | Choice | Action |
|-----|------|--------|--------|
| I | ITEMS (0) | 5 | Items menu |
| T | ITEMS (0) | 6 | Take |
| ? | ITEMS (0) | 7 | Look |
| U | ITEMS (0) | 8 | Use |
| G | ITEMS (0) | 9 | Give |
| Y | TALK (2) | 5 | Yell |
| S | TALK (2) | 6 | Say |
| A | TALK (2) | 7 | Ask |
| Space | GAME (4) | 5 | Pause |
| M | GAME (4) | 6 | Music toggle |
| F | GAME (4) | 7 | Sound toggle |
| Q | GAME (4) | 8 | Quit |
| L | GAME (4) | 9 | Load |
| O | BUY (3) | 5 | Buy item |
| R | BUY (3) | 6 | Buy item |
| 8 | BUY (3) | 7 | Buy item |
| C | BUY (3) | 8 | Buy Mace |
| W | BUY (3) | 9 | Buy Sword |
| B | BUY (3) | 10 | Buy Bow |
| E | BUY (3) | 11 | Buy Totem |
| V | SAVEX (5) | 5 | Save |
| X | SAVEX (5) | 6 | Exit |
| F1–F7 | MAGIC (1) | 5–11 | Magic spells 1–7 |
| 1–7 | USE (8) | 0–6 | Use item slots 1–7 |
| K | USE (8) | 7 | Use key |

### 9.13 Crystal Shard Terrain Bypass

When the hero attempts to move and `proxcheck()` returns terrain type 12 (blocked), the movement is permitted if `stuff[30]` (crystal shard) is nonzero. Checked after the door check (terrain 15) and before deviation. Terrain type 12 tiles exist only in terra set 8 (Region 8 building interiors) — tile index 93 in 12 sectors containing small chambers, twisting tunnels, forked intersections, and doom tower. Terra set 10 maps the same tile 93 to type 1/impassable (not crystal wall).

---


## 10. Combat System

### 10.1 Melee Hit Detection

Per-frame check for every actor in fighting state (states 0–11):

**Strike Point**:
```
xs = newx(attacker.abs_x, facing, weapon*2) + rand8() - 3
ys = newy(attacker.abs_y, facing, weapon*2) + rand8() - 3
```

Strike point extends `weapon_code * 2` pixels in facing direction, with ±3 to ±4 pixel random jitter.

**Reach (`bv`)**:

| Attacker | Reach | Notes |
|----------|-------|-------|
| Player | `(brave / 20) + 5`, max 15 | Julian starts at 6, maxes at 15 at brave=200 |
| Monster | `2 + rand4()` = 2–5 | Re-rolled each frame |

**Target Matching**: Chebyshev distance (max of `|dx|`, `|dy|`) from strike point to target. All conditions must be true:
1. Distance < `bv`
2. `freeze_timer == 0`
3. Player attacks: automatic hit
4. Monster attacks: `rand256() > brave` must pass (bravery = dodge probability)

Monster hit probability: `(256 − brave) / 256`. At brave=35 → 86% hit rate. At brave=100 → 61%.

**Near-miss**: when distance < `bv + 2` and weapon ≠ wand → `effect(1, 150 + rand256())`.

### 10.2 Damage — `dohit(i, j, fc, wt)`

Parameters: `i` = attacker (−1=arrow, −2=fireball, 0=player, 3+=monster), `j` = defender, `fc` = facing, `wt` = damage.

**Immunity checks**:

| Target | Condition | Effect |
|--------|-----------|--------|
| Necromancer (race 9) | weapon < 4 | Immune; `speak(58)` |
| Witch (race 0x89) | weapon < 4 AND no Sun Stone (`stuff[7]==0`) | Immune; `speak(58)` |
| Spectre (race 0x8a) | Always | Immune, silent return |
| Ghost (race 0x8b) | Always | Immune, silent return |

**Damage**: `vitality -= wt` (weapon code IS the damage). Vitality floors at 0.

**Melee damage formula**: `wt + bitrand(2)` where `wt` = weapon code. Touch attack (code 8) clamps `wt` to 5 before calculation.

**Missile damage**: `rand8() + 4` = 4–11 for both arrows and fireballs.

**Knockback**: defender pushed 2 pixels in attacker's facing direction via `move_figure(j, fc, 2)`. If knockback succeeds and attacker is melee (`i >= 0`), attacker slides 2 pixels forward. DRAGON and SETFIG types immune to knockback.

Every `dohit()` call ends with `checkdead(j, 5)`.

### 10.3 Weapon Types & Damage

| Code | Name | Type | Damage Range | Strike Range |
|------|------|------|-------------|-------------|
| 0 | None | Melee | 0–2 | 0–4 px |
| 1 | Dirk | Melee | 1–3 | 2–6 px |
| 2 | Mace | Melee | 2–4 | 4–8 px |
| 3 | Sword | Melee | 3–5 | 6–10 px |
| 4 | Bow | Ranged | 4–11 | mt=6 px |
| 5 | Wand | Ranged | 4–11 | mt=9 px |
| 8 | Touch | Melee | 5–7 | 10–14 px |

Touch attack (code 8) is monster-only, used by Wraiths, Snakes, Spiders, and Loraii (arms group 6).

### 10.4 Missile System

6 missile slots. Assigned round-robin via `mdex`.

| Property | Arrow | Fireball |
|----------|-------|----------|
| Hit radius (`mt`) | 6 pixels | 9 pixels |
| Damage | `rand8() + 4` = 4–11 | `rand8() + 4` = 4–11 |
| `dohit` attacker code | −1 | −2 |

**Dodge check**: player target → `bv = brave`; monster target → `bv = 20`. Only slot 0 applies dodge (`bitrand(512) > bv`); slots 1–5 always hit if in range. ~17% of projectiles are dodge-eligible.

**Special ranged attacks**:

| Attacker | Damage | Rate |
|----------|--------|------|
| Dragon | 4–11 (fireball) | 25% per frame (`rand4() == 0`) |
| Witch | `rand2() + 1` = 1–2 | When `witchflag` set and distance < 100 |

### 10.5 Enemy Encounter Chart

| Index | Monster | HP | Aggressive | Arms | Cleverness | Treasure | File ID |
|-------|---------|-----|------------|------|------------|----------|---------|
| 0 | Ogre | 18 | TRUE | 2 | 0 | 2 | 6 |
| 1 | Orcs | 12 | TRUE | 4 | 1 | 1 | 6 |
| 2 | Wraith | 16 | TRUE | 6 | 1 | 4 | 7 |
| 3 | Skeleton | 8 | TRUE | 3 | 0 | 3 | 7 |
| 4 | Snake | 16 | TRUE | 6 | 1 | 0 | 8 |
| 5 | Salamander | 9 | TRUE | 3 | 0 | 0 | 7 |
| 6 | Spider | 10 | TRUE | 6 | 1 | 0 | 8 |
| 7 | DKnight | 40 | TRUE | 7 | 1 | 0 | 8 |
| 8 | Loraii | 12 | TRUE | 6 | 1 | 0 | 9 |
| 9 | Necromancer | 50 | TRUE | 5 | 0 | 0 | 9 |
| 10 | Woodcutter | 4 | 0 | 0 | 0 | 0 | 9 |

Field semantics:
- **hitpoints**: base vitality at spawn
- **aggressive**: TRUE = hostile (field is never read at runtime; peace zones use extent system)
- **arms**: indexes `weapon_probs[arms*4 + rnd(4)]` for weapon selection
- **cleverness**: 0 = ATTACK1 (stupid), 1 = ATTACK2 (clever)
- **treasure**: indexes `treasure_probs[treasure*8 + rnd(8)]` for loot
- **file_id**: image file index for sprite loading

### 10.6 Weapon Probability Table — `weapon_probs[32]`

8 groups of 4 entries, indexed by `arms * 4 + rnd(4)`:

| Group | Values | Weapons |
|-------|--------|---------|
| 0 | 0,0,0,0 | None |
| 1 | 1,1,1,1 | All dirks |
| 2 | 1,2,1,2 | Dirks and maces |
| 3 | 1,2,3,2 | Mostly maces, some swords |
| 4 | 4,4,3,2 | Bows and swords |
| 5 | 5,5,5,5 | All magic wands |
| 6 | 8,8,8,8 | Touch attack |
| 7 | 3,3,3,3 | All swords |

### 10.7 Treasure Probability Table — `treasure_probs[40]`

5 groups of 8 entries, indexed by `treasure * 8 + rnd(8)`:

**Group 0** (treasure=0): `{0,0,0,0,0,0,0,0}` — nothing. Used by Snake, Salamander, Spider, DKnight, Loraii, Necromancer, Woodcutter.

**Group 1** (treasure=1, Orcs): `{9,11,13,31,31,17,17,32}`:

| Roll | Index | Item |
|------|-------|------|
| 0 | 9 | Blue Stone |
| 1 | 11 | Glass Vial |
| 2 | 13 | Bird Totem |
| 3–4 | 31 | 2 Gold Pieces |
| 5–6 | 17 | Green Key |
| 7 | 32 | 5 Gold Pieces |

**Group 2** (treasure=2, Ogres): `{12,14,20,20,20,31,33,31}`:

| Roll | Index | Item |
|------|-------|------|
| 0 | 12 | Crystal Orb |
| 1 | 14 | Gold Ring |
| 2–4 | 20 | Grey Key |
| 5, 7 | 31 | 2 Gold Pieces |
| 6 | 33 | 10 Gold Pieces |

**Group 3** (treasure=3, Skeletons): `{10,10,16,16,11,17,18,19}`:

| Roll | Index | Item |
|------|-------|------|
| 0–1 | 10 | Green Jewel |
| 2–3 | 16 | Gold Key |
| 4 | 11 | Glass Vial |
| 5 | 17 | Green Key |
| 6 | 18 | Blue Key |
| 7 | 19 | Red Key |

**Group 4** (treasure=4, Wraiths): `{15,21,0,0,0,0,0,0}`:

| Roll | Index | Item |
|------|-------|------|
| 0 | 15 | Jade Skull |
| 1 | 21 | White Key |
| 2–7 | 0 | Nothing |

### 10.8 Death System — `checkdead(i, dtype)`

Triggers when `vitality < 1` and state ≠ DYING and state ≠ DEAD:

| Effect | Condition |
|--------|-----------|
| Set `goal=DEATH`, `state=DYING`, `tactic=7` | Always |
| DKnight death speech: `speak(42)` | race == 7 |
| `kind −= 3` | SETFIG type, not witch (race ≠ 0x89) |
| `brave++` | Enemy (i > 0) |
| `event(dtype)`, `luck −= 5`, `setmood(TRUE)` | Player (i == 0) |

Death event messages: dtype 5 = killed, 6 = drowned, 7 = burned, 8 = turned to stone.

Death animation: `tactic` counts down 7→0 (7 frames), sprites 80/81 alternating. At 0 → `state = DEAD`, sprite index 82.

**Special death drops**:

| Monster | On Death |
|---------|----------|
| Necromancer (race 0x09) | Transforms to Woodcutter (race 10, vitality 10); drops Talisman (object 139) |
| Witch (race 0x89) | Drops Golden Lasso (object 27) |

### 10.9 Goodfairy & Brother Succession

When player is DEAD or FALL, `goodfairy` countdown (u8, starts at 0):

| Range | Duration | Effect |
|-------|----------|--------|
| 255→200 | ~56 frames | Death sequence — corpse visible, death song |
| 199→120 | ~80 frames | **Luck gate**: luck < 1 → `revive(TRUE)` (brother succession); FALL → `revive(FALSE)` (non-lethal) |
| 119→20 | ~100 frames | Fairy sprite flies toward hero (only if luck ≥ 1) |
| 19→2 | ~18 frames | Resurrection glow effect |
| 1 | 1 frame | `revive(FALSE)` — fairy rescue, same character |

Luck cannot change during DEAD state. Fairy rescues from starting stats: Julian 3, Phillip 6, Kevin 3 (each death costs 5 luck; falls cost 2).

**`revive(TRUE)` — New brother**: brother increments (1→Julian, 2→Phillip, 3→Kevin, 4+→game over). Stats from `blist[]`. Inventory wiped for indices 0 to GOLDBASE−1. Starting weapon = Dirk. Vitality = `15 + brave/4`. Dead brother's body and ghost placed in world.

**`revive(FALSE)` — Fairy rescue**: no stat changes. Returns to `safe_x`/`safe_y`. Vitality = `15 + brave/4`.

### 10.10 Bravery & Luck

Bravery is both passive experience and active combat stat:

| Effect | Formula |
|--------|---------|
| Melee reach | `(brave / 20) + 5`, max 15 |
| Monster dodge | `rand256() > brave` must pass |
| Missile dodge (slot 0) | `bitrand(512) > brave` |
| Starting vitality | `15 + brave / 4` |
| Growth | +1 per enemy kill |

Compounding feedback loop: more kills → higher brave → longer reach + better dodge + more HP → more kills.

Luck: −5 per death, −2 per ledge fall. When depleted, next death is permanent.

---


## 11. AI & Behavior

### 11.1 Goal Modes

11 high-level goal modes stored in `shape.goal`:

| Value | Name | Purpose |
|-------|------|---------|
| 0 | USER | Player-controlled |
| 1 | ATTACK1 | Attack stupidly (cleverness 0) |
| 2 | ATTACK2 | Attack cleverly (cleverness 1) |
| 3 | ARCHER1 | Archery attack style 1 |
| 4 | ARCHER2 | Archery attack style 2 |
| 5 | FLEE | Run directly away from hero |
| 6 | STAND | Stand still, face hero |
| 7 | DEATH | Dead character |
| 8 | WAIT | Wait to speak to hero |
| 9 | FOLLOWER | Follow another character |
| 10 | CONFUSED | Run around randomly |

ATTACK1 vs ATTACK2 determined by `cleverness` in `encounter_chart[]`.

### 11.2 Tactical Modes

13 tactical modes stored in `shape.tactic`:

| Value | Name | Purpose |
|-------|------|---------|
| 0 | FRUST | Frustrated — try a different tactic |
| 1 | PURSUE | Move toward hero |
| 2 | FOLLOW | Move toward another character |
| 3 | BUMBLE_SEEK | Bumble around seeking target |
| 4 | RANDOM | Move in random direction |
| 5 | BACKUP | Reverse current direction |
| 6 | EVADE | Move 90° from hero |
| 7 | HIDE | Seek hiding place (planned, never implemented) |
| 8 | SHOOT | Shoot an arrow |
| 9 | SHOOTFRUST | Arrows not connecting — re-evaluate |
| 10 | EGG_SEEK | Snakes seeking turtle eggs |
| 11 | DOOR_SEEK | Vestigial — replaced by hardcoded DKnight logic |
| 12 | DOOR_LET | Vestigial — replaced by hardcoded DKnight logic |

Source comment: "choices 2–5 can be selected randomly for getting around obstacles."

### 11.3 `set_course` Algorithm

`set_course(object, target_x, target_y, mode)` — 7 pathfinding modes:

**Direction computation**:
1. Mode 6: uses target_x/target_y directly as xdif/ydif (raw vector)
2. All other modes: `xdif = self.abs_x − target_x`, `ydif = self.abs_y − target_y`
3. Compute `xdir = sign(xdif)`, `ydir = sign(ydif)`

**Directional snapping** (mode ≠ 4): if one axis dominates, minor axis zeroed:
- `(|xdif| >> 1) > |ydif|` → `ydir = 0` (mostly horizontal)
- `(|ydif| >> 1) > |xdif|` → `xdir = 0` (mostly vertical)

Mode 4 skips snapping, always allowing diagonal.

**com2 lookup**: `j = com2[4 − 3*ydir − xdir]`. If j == 9 (at target): `state = STILL`, return.

**Random deviation**: if deviation > 0: `rand() & 2` (bit 1, not bit 0) determines `j += deviation` or `j −= deviation`, then `j &= 7`.

| Mode | Behavior | Deviation |
|------|----------|-----------|
| 0 | Toward target with snapping | 0 |
| 1 | Toward target + deviation when distance < 40 | 1 |
| 2 | Toward target + deviation when distance < 30 | 1 |
| 3 | Away from target (reverses direction) | 0 |
| 4 | Toward target without snapping (always diagonal) | 0 |
| 5 | Toward target with snapping; does NOT set state to WALKING | 0 |
| 6 | Uses target_x/target_y as raw direction vector | 0 |

**Important**: these mode numbers are NOT the tactical mode constants. `do_tactic()` maps tactics to `set_course` modes:

| Tactic | set_course mode | Target |
|--------|----------------|--------|
| PURSUE (1) | 0 | Hero |
| FOLLOW (2) | 0 | Leader (+20 Y offset) |
| BUMBLE_SEEK (3) | 4 | Hero (no snap) |
| BACKUP (5) | 3 | Hero (reversed) |
| EVADE (6) | 2 | Neighboring actor (+20 Y) |
| SHOOT (8) | 0 or 5 | Hero (face only when aligned) |
| EGG_SEEK (10) | 0 | Fixed coords (23087, 5667) |
| RANDOM (4) | *(direct)* | `facing = rand() & 7` |

### 11.4 `do_tactic()` Dispatch

All tactical movement rate-limited by random gate:
```
base: !(rand() & 7) = 12.5% chance
ATTACK2 goal: !(rand() & 3) = 25% chance
```

When gate fails, actor continues previous trajectory unchanged.

| Tactic | Rate-limited? | Notes |
|--------|---------------|-------|
| PURSUE | Yes | `set_course` mode 0 to hero |
| SHOOT | **No** — fires every tick | Checks axis alignment, transitions between mode 0 and 5 |
| RANDOM | Facing only | `state = WALKING` unconditional; facing changes when gate passes |
| BUMBLE_SEEK | Yes | `set_course` mode 4 (no snap) |
| BACKUP | Yes | `set_course` mode 3 (reverse) |
| FOLLOW | Yes | `set_course` mode 0 to leader + 20 Y |
| EVADE | Yes | `set_course` mode 2; target = `anim_list[i*2]` |
| EGG_SEEK | Yes | `set_course` mode 0 to fixed (23087, 5667) |

### 11.5 AI Main Loop

Processes actors 2 through `anix−1` (skipping player and raft). Processing order:

1. **Goodfairy suspend**: if `goodfairy > 0 && < 120`, all AI halts
2. **CARRIER type**: every 16 ticks, face player via `set_course(i, hero_x, hero_y, 5)`. No other AI
3. **SETFIG type**: skipped entirely
4. **Distance & battle detection**: within 300×300 pixels → `actors_on_screen = TRUE`, `battleflag = TRUE`
5. **Random reconsider**: base `!bitrand(15)` = 1/16 (6.25%)
6. **Goal overrides**: hero dead → FLEE/FOLLOWER; vitality < 2 → FLEE; extent mismatch → FLEE; unarmed → CONFUSED
7. **Frustration handling**: FRUST/SHOOTFRUST → random tactic: ranged = `rand4()+2` → {FOLLOW, BUMBLE_SEEK, RANDOM, BACKUP}; melee = `rand2()+3` → {BUMBLE_SEEK, RANDOM}
8. **Hostile AI** (modes ≤ ARCHER2): reconsider frequency adjustment, tactic selection, melee engagement
9. **FLEE**: `do_tactic(i, BACKUP)`
10. **FOLLOWER**: `do_tactic(i, FOLLOW)`
11. **STAND**: face hero, force STILL state
12. **WAIT**: force STILL, no facing change
13. **CONFUSED** and others: no processing — actor continues last trajectory

`leader` set to first living active enemy at loop end.

### 11.6 Hostile AI Detail

For modes ≤ ARCHER2, reconsider frequency: `if ((mode & 2) == 0) r = !rand4()` → 25% for ATTACK1 and ARCHER2. ATTACK2 and ARCHER1 keep base 6.25%.

Tactic assignment on reconsider:

| Condition | Tactic |
|-----------|--------|
| race==4 && turtle_eggs | EGG_SEEK |
| weapon < 1 | RANDOM (mode→CONFUSED) |
| vitality < 6 && rand2() | EVADE |
| Archer, xd<40 && yd<30 | BACKUP |
| Archer, xd<70 && yd<70 | SHOOT |
| Archer, far | PURSUE |
| Melee, default | PURSUE |

Melee engagement: `thresh = 14 − mode`. DKnight overrides to 16. Within thresh → FIGHTING state. Outside → `do_tactic(i, tactic)`.

### 11.7 Cleverness Effects

| Property | Cleverness 0 | Cleverness 1 |
|----------|-------------|-------------|
| Goal mode | ATTACK1 / ARCHER1 | ATTACK2 / ARCHER2 |
| `do_tactic` rate | 12.5% per tick | 25% per tick (ATTACK2 only) |
| Tactic reconsider | 25% (ATTACK1) or 6.25% (ARCHER1) | 6.25% (ATTACK2) or 25% (ARCHER2) |
| Melee threshold | 13 (ATTACK1) or 11 (ARCHER1) | 12 (ATTACK2) or 10 (ARCHER2) |

Clever enemies: Orcs, Wraith, Snake, Spider, DKnight, Loraii.
Stupid enemies: Ogre, Skeleton, Salamander, Necromancer, Woodcutter.

### 11.8 DKnight Special Behavior

- Fixed position at (21635, 25762); does not pursue, does not call `do_tactic()`
- Out of range: `state = STILL`, `facing = 5` (south) — physically blocks passage
- In range (xd < 16, yd < 16): enters FIGHTING state
- Never flees (exempt when race matches extent v3 for xtype > 59)
- Respawns every time player re-enters hidden valley zone
- Proximity speech: `speak(41)`; death speech: `speak(42)`

### 11.9 CONFUSED Mode

Assigned when hostile actor loses weapon (`weapon < 1`). First tick: `do_tactic(i, RANDOM)`. Subsequent ticks: CONFUSED (10) fails all goal-mode checks — no AI processing occurs. Actor walks in last random direction until blocked.

---


## 12. Encounter Generation

### 12.1 Extent Zones

`extent_list[23]` — 22 zones plus sentinel at index 22. Each entry:

```rust
struct Extent {
    x1: u16, y1: u16, x2: u16, y2: u16,  // bounding box (exclusive)
    etype: u8,                              // zone type
    v1: u8, v2: u8, v3: u8,               // parameters
}
```

`find_place()` performs linear scan of entries 0–21; first match wins. Entry 22 (etype=3, whole world) is sentinel fallback.

| Idx | Location | etype | v1 | v2 | v3 | Category |
|-----|----------|-------|----|----|----|----------|
| 0 | Bird (swan) | 70 | 0 | 1 | 11 | Carrier |
| 1 | Turtle (movable) | 70 | 0 | 1 | 5 | Carrier |
| 2 | Dragon | 70 | 0 | 1 | 10 | Carrier |
| 3 | Spider pit | 53 | 4 | 1 | 6 | Forced encounter |
| 4 | Necromancer | 60 | 1 | 1 | 9 | Special figure |
| 5 | Turtle eggs | 61 | 3 | 2 | 4 | Special figure |
| 6 | Princess rescue | 83 | 1 | 1 | 0 | Peace (special) |
| 7 | Graveyard | 48 | 8 | 8 | 2 | Regular (very high) |
| 8 | Around city | 80 | 4 | 20 | 0 | Peace zone |
| 9 | Astral plane | 52 | 3 | 1 | 8 | Forced encounter |
| 10 | King's domain | 81 | 0 | 1 | 0 | Peace + weapon block |
| 11 | Sorceress domain | 82 | 0 | 1 | 0 | Peace + weapon block |
| 12–14 | Buildings/cabins | 80 | 0 | 1 | 0 | Peace zone |
| 15 | Hidden valley | 60 | 1 | 1 | 7 | Special figure (DKnight) |
| 16 | Swamp region | 7 | 1 | 8 | 0 | Regular (swamp) |
| 17–18 | Spider regions | 8 | 1 | 8 | 0 | Regular (spiders) |
| 19 | Village | 80 | 0 | 1 | 0 | Peace zone |
| 20–21 | Around village/city | 3 | 1 | 3 | 0 | Regular (low) |
| *22* | *Whole world* | *3* | *1* | *8* | *0* | *Sentinel fallback* |

Only extents 0 (bird) and 1 (turtle) are persisted in savegames. Turtle starts at (0,0,0,0) and is repositioned via `move_extent()`.

### 12.2 Extent Type Categories

| etype Range | Category | Behavior |
|-------------|----------|----------|
| 0–49 | Regular encounter zone | Sets `xtype`; random encounters per danger timer |
| 50–59 | Forced group encounter | Monsters spawn immediately; v1=count, v3=type |
| 52 | Astral plane (special) | Forces encounter_type=8 (Loraii); synchronous load |
| 60–61 | Special figure | Unique NPC at extent center if not already present |
| 70 | Carrier | Loads bird/turtle/dragon via `load_carrier(v3)` |
| 80 | Peace zone | Blocks random encounters (`xtype ≥ 50` fails guard) |
| 81 | King peace | Peace + weapon draw blocked: `event(15)` |
| 82 | Sorceress peace | Peace + weapon draw blocked: `event(16)` |
| 83 | Princess rescue | Triggers `rescue()` if `ob_list8[9].ob_stat` set |

### 12.3 Danger Level & Spawn Logic

**Placement check — every 16 frames**: places already-loaded monsters into anim_list[3–6]. Up to 10 random locations tried via `set_loc()` (150–213 pixels from hero). Each must have terrain type 0 (walkable). Dead slots recycled when all 4 full.

**Danger check — every 32 frames**: conditions = no actors on screen, no pending load, no carrier, `xtype < 50`.

Danger level formula:
```
Indoor (region_num > 7): danger_level = 5 + xtype
Outdoor:                 danger_level = 2 + xtype
```

Spawn probability: `rand64() <= danger_level` → `(danger_level + 1) / 64`:

| Zone | xtype | Outdoor Danger | Probability |
|------|-------|----------------|-------------|
| World / village area | 3 | 5 | 6/64 = 9.4% |
| Swamp region | 7 | 9 | 10/64 = 15.6% |
| Spider region | 8 | 10 | 11/64 = 17.2% |
| Graveyard | 48 | 50 | 51/64 = 79.7% |

### 12.4 Monster Type Selection

Base: `rand4()` (0–3 → ogre, orc, wraith, skeleton). Overrides:

| Condition | Override |
|-----------|----------|
| Swamp (xtype=7) | Wraith roll (2) → Snake (4) |
| Spider region (xtype=8) | All rolls → Spider (6) |
| xtype=49 | All rolls → Wraith (2) |

### 12.5 Monster Count

```
encounter_number = v1 + rnd(v2)
```

| Zone | v1 | v2 | Count Range |
|------|----|----|-------------|
| Whole world | 1 | 8 | 1–8 |
| Around village/city | 1 | 3 | 1–3 |
| Spider pit | 4 | 1 | 4 (forced) |
| Graveyard | 8 | 8 | 8–15 |

Only 4 enemy slots (indices 3–6); excess encounter_number resolves over time.

### 12.6 Actor Placement — `set_encounter(i, spread)`

Up to 15 attempts (`MAX_TRY`):

- **DKnight** (v3 == 7): hardcoded position (21635, 25762), placement loop skipped
- **Normal**: random offset from encounter origin: `encounter_x + bitrand(spread) − spread/2`. Accept if `proxcheck == 0`
- **Astral**: also accept if `px_to_im == 7` (ice terrain)

**Race mixing**: `mixflag & 2` → `race = (encounter_type & 0xFFFE) + rand2()`. Disabled for xtype > 49 or xtype % 4 == 0.

**Weapon selection**: `weapon_probs[arms * 4 + wt]`. `wt` re-randomized per enemy if `mixflag & 4`, otherwise shared in batch.

### 12.7 Special Extents

**Carriers** (etype 70): `load_carrier(v3)` places carrier in anim_list[3]:

| v3 | Carrier | Notes |
|----|---------|-------|
| 11 | Swan | Requires Golden Lasso (`stuff[5]`) to mount |
| 5 | Turtle | Extent starts at (0,0,0,0); repositioned via `move_extent()` |
| 10 | Dragon | Has own fireball attack logic |

Carrier extent: 500×400 box centered on point via `move_extent()`.

**Spider pit** (etype 53, idx 3): spawns 4 spiders immediately. `mixflag=0, wt=0`.

**Necromancer / DKnight** (etype 60): unique NPC at extent center. Only spawns if not already present (`anim_list[3].race != v3` or `anix < 4`).

**Princess rescue** (etype 83, idx 6): when `ob_list8[9].ob_stat` set → `rescue()`: displays placard, increments `princess` counter, teleports hero to (5511, 33780), repositions bird extent via `move_extent(0, 22205, 21231)`.

### 12.8 Peace Zones

Extents with etype 80–83 set `xtype ≥ 50`, failing the `xtype < 50` guard. The `aggressive` field in `encounter_chart[]` is **never read** at runtime.

- etype 81 (King): weapon draw → `event(15)` admonishing message
- etype 82 (Sorceress): weapon draw → `event(16)` calming message

---

### C. Timing Constants

| Parameter | Value | Notes |
|-----------|-------|-------|
| Frame rate | 30 fps | Gameplay ticks (NTSC) |
| Audio VBL | 60 Hz | Audio interrupt rate |
| Day cycle | 24000 ticks | ≈ 13.3 minutes real time at 30 fps |
| Hunger tick | 128 ticks | ≈ 4.3 seconds |
| Health regen | 1024 ticks | ≈ 34 seconds |
| Safe zone check | 128 ticks | Same as hunger tick |
| Sleep advance | 64 ticks/frame | 63 extra + 1 normal |
| Default tempo | 150 | Timeclock counts per VBL |
| AI reconsider (base) | 1/16 per tick | 6.25% per frame via `!bitrand(15)` |
| AI reconsider (ATTACK1/ARCHER2) | 1/4 per tick | 25% per frame via `!rand4()` |
| Tactic execution (base) | 1/8 per tick | 12.5% per frame via `!(rand()&7)` |
| Tactic execution (ATTACK2) | 1/4 per tick | 25% per frame via `!(rand()&3)` |
| Encounter placement | 16 frames | ≈ 0.53 seconds |
| Encounter generation | 32 frames | ≈ 1.07 seconds |
| Carrier facing update | 16 ticks | Every 16 ticks for CARRIER type |
| Death animation | 7 frames | tactic counts down 7→0 |
| Goodfairy total | 255 frames | ≈ 8.5 seconds |
| Goodfairy luck gate | < 200 | ~56 frames after death |
| Goodfairy fairy visible | < 120 | ~80 frames after luck gate |
| Timer heartbeat | 16 events | ticker 0→16 synthesizes key |
| FALL friction | 25%/tick | `(vel * 3) / 4` per frame |
| Ice velocity cap | 42 (normal), 40 (swan) | Max magnitude per axis |
| Camera dead zone X | ±20 pixels | No scroll within zone |
| Camera dead zone Y | ±10 pixels | No scroll within zone |
| Camera snap X | > 70 pixels | Instant camera reposition |
| Camera snap Y (down) | > 44 pixels | Asymmetric for sprite offset |
| Camera snap Y (up) | > 24 pixels | Asymmetric for sprite offset |
| Frustration threshold 1 | > 20 | Scratching-head animation |
| Frustration threshold 2 | > 40 | Special animation index 40 |
| Hunger stumble threshold | > 120 | 1/4 chance of direction deflection |

### B. Brother Stats

| Property | Julian | Phillip | Kevin |
|----------|--------|---------|-------|
| brave | 35 | 20 | 15 |
| luck | 20 | 35 | 20 |
| kind | 15 | 15 | 35 |
| wealth | 20 | 15 | 10 |
| Starting vitality | 23 | 20 | 18 |
| Vitality formula | `15 + brave/4` | `15 + brave/4` | `15 + brave/4` |
| Starting melee reach | 6 | 6 | 5 |
| Monster hit rate at start | 86% | 92% | 94% |
| Fairy rescues (from start) | 3 | 6 | 3 |
| Starting weapon | Dirk (1) | Dirk (1) | Dirk (1) |


## 13. NPCs & Dialogue

### 13.1 NPC Types (setfig_table)

14 NPC types defined in `setfig_table[]` (`fmain.c:22-36`):

| Index | NPC | cfile | image_base | can_talk | Race | Vitality |
|-------|-----|-------|------------|----------|------|----------|
| 0 | Wizard | 13 | 0 | Yes | 0x80 | 2 |
| 1 | Priest | 13 | 4 | Yes | 0x81 | 4 |
| 2 | Guard | 14 | 0 | No | 0x82 | 6 |
| 3 | Guard (back) | 14 | 1 | No | 0x83 | 8 |
| 4 | Princess | 14 | 2 | No | 0x84 | 10 |
| 5 | King | 14 | 4 | Yes | 0x85 | 12 |
| 6 | Noble | 14 | 6 | No | 0x86 | 14 |
| 7 | Sorceress | 14 | 7 | No | 0x87 | 16 |
| 8 | Bartender | 15 | 0 | No | 0x88 | 18 |
| 9 | Witch | 16 | 0 | No | 0x89 | 20 |
| 10 | Spectre | 16 | 6 | No | 0x8a | 22 |
| 11 | Ghost | 16 | 7 | No | 0x8b | 24 |
| 12 | Ranger | 17 | 0 | Yes | 0x8c | 26 |
| 13 | Beggar | 17 | 4 | Yes | 0x8d | 28 |

- Race code: `id + 0x80` (OR'd in `set_objects` — `fmain2.c:1280`).
- Vitality: `2 + id*2` (`fmain2.c:1274`).
- `goal` field: stores the object list index (`fmain2.c:1275`), selecting variant dialogue for wizards, rangers, and beggars.
- `can_talk`: controls the talking mouth animation (15-tick timer — `fmain.c:3376-3379`). All 14 types have speech dispatch code regardless of this flag.

### 13.2 Talk System

Entry point: `do_option()` — `fmain.c:3367-3422`.

**Submenu** (`fmain.c:497`): `"Yell Say  Ask  "` — three options:

| Option | Hit Value | Range | Special |
|--------|-----------|-------|---------|
| Yell | 5 | `nearest_fig(1,100)` — 100 units | If target within 35 units: `speak(8)` ("No need to shout!") |
| Say | 6 | `nearest_fig(1,50)` — 50 units | Standard dispatch |
| Ask | 7 | `nearest_fig(1,50)` — 50 units | Functionally identical to Say |

**Dispatch logic** (`fmain.c:3375-3422`):

1. If no target found or target is DEAD → break (no speech).
2. **SETFIG NPC**: switch on `k = an->race & 0x7f` (setfig index) — see §13.3.
3. **CARRIER** with `active_carrier == 5` (turtle): shell dialogue — see §13.7.
4. **ENEMY**: `speak(an->race)` — enemy race directly indexes speech table — see §13.8.

### 13.3 NPC Speech Dispatch

**Wizard** (index 0 — `fmain.c:3380-3381`):
- `kind < 10` → `speak(35)` ("Away with you, ruffian!")
- `kind >= 10` → `speak(27 + an->goal)` — goal selects from 8 hints:

| Goal | Speech | Hint |
|------|--------|------|
| 0 | speak(27) | "Kind deeds could gain thee a friend from the sea" (Turtle) |
| 1 | speak(28) | "Seek the place darker than night" |
| 2 | speak(29) | "Crystal Orb helps find concealed things" |
| 3 | speak(30) | "The Witch lives in the dim forest of Grimwood" |
| 4 | speak(31) | "Only the light of the Sun can destroy the Witch's Evil" |
| 5 | speak(32) | "Maiden imprisoned in unreachable castle" |
| 6 | speak(33) | "Tame the golden beast" (Swan) |
| 7 | speak(34) | "Just what I needed!" |

Wizard locations by object list:

| Region | Object List Entry | Goal |
|--------|-------------------|------|
| 0 (Snow) | ob_list0 | 0–2 (3 wizards) |
| 2 (Swamp) | ob_list2[0], ob_list2[1] | 0, 1 |
| 3 (Tambry) | ob_list3[2] | 2 |
| 5 (Farm/City) | ob_list5[3], ob_list5[4] | 3, 4 |
| 8 (Indoors) | ob_list8[5], ob_list8[6] | 5, 6 |
| 9 (Underground) | ob_list9[6] | 6 |

**Priest** (index 1 — `fmain.c:3382-3394`):
1. Has Writ (`stuff[28]`):
   - If `ob_listg[10].ob_stat == 0` → `speak(39)` ("Here is a golden statue"), set `ob_listg[10].ob_stat = 1`.
   - Else → `speak(19)` ("Already gave the statue").
2. `kind < 10` → `speak(40)` ("Repent, Sinner!").
3. `kind >= 10` → `speak(36 + daynight%3)` (rotating advice) AND heal to `15 + brave/4`:
   - `daynight%3 == 0` → speak(36): "Seek enemy on spirit plane"
   - `daynight%3 == 1` → speak(37): "Seek power of the Stones"
   - `daynight%3 == 2` → speak(38): "I shall Heal all your wounds"

All three speeches (36–38) trigger healing; the rotation selects only the text.

**Guard** (indices 2–3 — `fmain.c:3394`): `speak(15)` ("State your business!").

**Princess** (index 4 — `fmain.c:3397`): `speak(16)` ("Please, sir, rescue me...") — only when `ob_list8[9].ob_stat` is set.

**King** (index 5 — `fmain.c:3398`): `speak(17)` ("I cannot help you, young man") — only when `ob_list8[9].ob_stat` is set. No dialogue when flag is clear.

**Noble** (index 6 — `fmain.c:3396`): `speak(20)` ("If you could rescue the king's daughter...").

**Sorceress** (index 7 — `fmain.c:3400-3405`):
- First visit (`ob_listg[9].ob_stat == 0`): `speak(45)` ("Welcome. Here is one of the five golden figurines"), set `ob_listg[9].ob_stat = 1`.
- Return visits: no speech. Silent luck boost: if `luck < rand64()` then `luck += 5`.

**Bartender** (index 8 — `fmain.c:3405-3407`):
- `fatigue < 5` → `speak(13)` ("Good Morning").
- `fatigue >= 5 && dayperiod > 7` → `speak(12)` ("Would you like to buy something?").
- Else → `speak(14)` ("Have a drink!").

**Witch** (index 9 — `fmain.c:3408`): `speak(46)` ("Look into my eyes and Die!!").

**Spectre** (index 10 — `fmain.c:3409`): `speak(47)` ("HE has usurped my place... Bring me bones of the ancient King").

**Ghost** (index 11 — `fmain.c:3410`): `speak(49)` ("I am the ghost of your dead brother. Find my bones...").

**Ranger** (index 12 — `fmain.c:3411-3413`):
- `region_num == 2` (swamp) → `speak(22)` ("Dragon's cave is directly north").
- Else → `speak(53 + an->goal)`:
  - goal 0 → speak(53): "Dragon's cave is east"
  - goal 1 → speak(54): "Dragon's cave is west"
  - goal 2 → speak(55): "Dragon's cave is south"

Rangers appear only in ob_list0 (snow, 3 rangers) and ob_list2 (swamp, 1 ranger).

**Beggar** (index 13 — `fmain.c:3414`): `speak(23)` ("Alms! Alms for the poor!").

### 13.4 Proximity Auto-Speech

Certain NPCs speak automatically when near the player (`fmain.c:2094-2102`), independent of the Talk menu. Tracked by `last_person` to prevent re-triggering for the same NPC.

| Race | NPC | Speech | Condition |
|------|-----|--------|-----------|
| 0x8d | Beggar | `speak(23)` | Always |
| 0x89 | Witch | `speak(46)` | Always |
| 0x84 | Princess | `speak(16)` | Only if `ob_list8[9].ob_stat` set |
| 9 | Necromancer | `speak(43)` | Always (extent entry) |
| 7 | DreamKnight | `speak(41)` | Always (extent entry) |

DreamKnight death speech: `speak(42)` ("You have earned the right to enter...") — `fmain.c:2775`.

### 13.5 Give System

Entry point: `do_option()` — `fmain.c:3490-3508`.

Menu (`fmain.c:506`): `"Gold Book Writ Bone "` — four options (hit values 5–8):

| Option | Hit | Condition | Effect |
|--------|-----|-----------|--------|
| Gold | 5 | `wealth > 2` | `wealth -= 2`. If `rand64() > kind` → `kind++`. Beggar: `speak(24 + goal)`. Others: `speak(50)`. |
| Book | 6 | ALWAYS DISABLED | Hardcoded disabled in `set_options` (`fmain.c:3540`). |
| Writ | 7 | `stuff[28] != 0` | Menu enabled but **no processing handler**. Writ is checked passively during Priest Talk. |
| Bone | 8 | `stuff[29] != 0` | Non-spectre: `speak(21)` ("no use for it"). Spectre (0x8a): `speak(48)` ("Take this crystal shard"), `stuff[29] = 0`, drops crystal shard (object 140). |

**Beggar give-gold prophecies** (`speak(24 + goal)`):

| Goal | Speech | Prophecy |
|------|--------|----------|
| 0 | speak(24) | "Seek two women, one Good, one Evil" |
| 1 | speak(25) | "Jewels, glint in the night — gift of Sight" |
| 2 | speak(26) | "Where is the hidden city?" |
| 3 | speak(27) | **Bug**: overflows to first wizard hint ("Kind deeds...") — `ob_list3[3]` has `goal=3` |

### 13.6 Shop System

**Bartender identification**: setfig index 8, race `0x88`. BUY menu only activates with race `0x88` (`fmain.c:3426`).

**`jtrans[]`** (`fmain2.c:850`) defines 7 purchasable items as `{stuff_index, price}` pairs:

| # | Item | Price | Effect | Menu Text |
|---|------|-------|--------|-----------|
| 1 | Food | 3 | `eat(50)` — reduces hunger by 50 | Food |
| 2 | Arrows | 10 | `stuff[8] += 10` | Arrow |
| 3 | Vial | 15 | `stuff[11]++` (Glass Vial / healing potion) | Vial |
| 4 | Mace | 30 | `stuff[1]++` | Mace |
| 5 | Sword | 45 | `stuff[2]++` | Sword |
| 6 | Bow | 75 | `stuff[3]++` | Bow |
| 7 | Totem | 20 | `stuff[13]++` (Bird Totem) | Totem |

Menu string (`fmain.c:501`): `"Food ArrowVial Mace SwordBow  Totem"`.

Purchase requires `wealth > price` (`fmain.c:3430`). On purchase, `wealth -= price`.

### 13.7 Carrier Dialogue (Turtle)

When `active_carrier == 5` (turtle carrier active — `fmain.c:3418-3421`):

- If `stuff[6] == 0` (no sea shell): `speak(56)` ("Thank you for saving my eggs!"), set `stuff[6] = 1`.
- If `stuff[6] != 0` (has shell): `speak(57)` ("Hop on my back for a ride").

### 13.8 Enemy Speech

When talking to enemies, `speak(an->race)` — the race value directly indexes the speech table (`fmain.c:3422`):

| Race | Enemy | Speech |
|------|-------|--------|
| 0 | Ogre | speak(0): "A guttural snarl was the only reply." |
| 1 | Orc | speak(1): "Human must die!" |
| 2 | Wraith | speak(2): "Doom!" |
| 3 | Skeleton | speak(3): "A clattering of bones" |
| 4 | Snake | speak(4): "A waste of time to talk to a snake" |
| 5 | Salamander | speak(5): "..." |
| 6 | Spider | speak(6): "There was no reply." |
| 7 | DKnight | speak(7): "Die, foolish mortal!" |

Note: `narr.asm` labels speak(6) as "loraii" and speak(7) as "necromancer", reflecting an earlier race table. Loraii (race 8) and Necromancer (race 9) have special auto-speak handlers that preempt the generic talk path, so misaligned speeches at indices 8–9 are never heard in normal gameplay.

### 13.9 Message Tables

**Event messages** (`narr.asm:11-74`, called via `event(n)` — `fmain2.c:554-558`):

| Index | Text | Usage |
|-------|------|-------|
| 0–2 | Hunger warnings ("rather hungry", "very hungry", "starving!") | `fmain.c:2203-2209` |
| 3–4 | Fatigue warnings ("getting tired", "getting sleepy") | `fmain.c:2218-2219` |
| 5–7 | Death messages (combat, drowning, lava) | `checkdead()` dtype |
| 12 | "couldn't stay awake any longer!" | Forced sleep |
| 15–16 | Pax zone messages (castle, sorceress) | `fmain.c:1413-1414` |
| 17–19 | Paper pickup (regionally-variant text) | `fmain.c:3163-3168` |
| 22–23 | Shop purchase messages | `fmain.c:3433-3434` |
| 24 | "passed out from hunger!" | `fmain.c:2213` |
| 27 | "perished in the hot lava!" | `fmain.c:1847` |
| 28–31 | Time-of-day announcements | `fmain.c:2033` |
| 32 | "Ground is too hot for swan to land" | Swan dismount blocked in lava |
| 33 | "Flying too fast to dismount" | Swan dismount blocked at speed |
| 36–37 | Fruit pickup/auto-eat | `fmain.c:3166`, `fmain.c:2196` |

**Placard texts** (`narr.asm:230-343`): formatted narrative screens with XY positioning.

| Index | Content |
|-------|---------|
| 0 | Julian's quest begins ("Rescue the Talisman!") |
| 1–2 | Julian falls, Phillip starts |
| 3–4 | Phillip falls, Kevin starts |
| 5 | Game over ("Stay at Home!") |
| 6–7 | Victory sequence ("Having defeated the Necromancer...") |
| 8–10 | Princess Katra rescue |
| 11–13 | Princess Karla rescue |
| 14–16 | Princess Kandy rescue |
| 17–18 | Shared post-rescue text (all princesses) |
| 19 | Copy protection prompt |

---


## 14. Inventory & Items

### 14.1 Inventory Structure

Three static arrays per brother, plus a pointer for the active brother:

```
UBYTE julstuff[ARROWBASE], philstuff[ARROWBASE], kevstuff[ARROWBASE];
UBYTE *stuff; // bound to current brother via blist[brother-1].stuff
```

`stuff[]` layout (indices 0–34 active, index 35 as temporary accumulator):

| Index | Category | Item |
|-------|----------|------|
| 0 | Weapon | Dirk |
| 1 | Weapon | Mace |
| 2 | Weapon | Sword |
| 3 | Weapon | Bow |
| 4 | Weapon | Magic Wand |
| 5 | Special | Golden Lasso |
| 6 | Special | Sea Shell |
| 7 | Special | Sun Stone |
| 8 | Special | Arrows (integer count, max display 45) |
| 9 | Magic | Blue Stone |
| 10 | Magic | Green Jewel |
| 11 | Magic | Glass Vial |
| 12 | Magic | Crystal Orb |
| 13 | Magic | Bird Totem |
| 14 | Magic | Gold Ring |
| 15 | Magic | Jade Skull |
| 16 | Key | Gold Key |
| 17 | Key | Green Key |
| 18 | Key | Blue Key |
| 19 | Key | Red Key |
| 20 | Key | Grey Key |
| 21 | Key | White Key |
| 22 | Quest | Talisman (win condition) |
| 23 | Quest | Rose (lava immunity) |
| 24 | Quest | Fruit (portable food) |
| 25 | Quest | Gold Statue (5 needed for desert gate) |
| 26 | Quest | Book (vestigial — not obtainable) |
| 27 | Quest | Herb (vestigial — not obtainable) |
| 28 | Quest | Writ (royal commission) |
| 29 | Quest | Bone |
| 30 | Quest | Crystal Shard (terrain 12 bypass) |
| 31–34 | Gold pickup | Values 2, 5, 10, 100 → added to `wealth` |

Constants: `MAGICBASE = 9`, `KEYBASE = 16`, `STATBASE = 25`, `GOLDBASE = 31`, `ARROWBASE = 35`.

On brother succession (`revive(TRUE)`): all items wiped, starting loadout is one Dirk (`stuff[0] = 1`). The `stuff` pointer is rebound to the new brother.

### 14.2 Weapon Details

| Index | Item | Melee Damage | Notes |
|-------|------|-------------|-------|
| 0 | Dirk | 1–3 | Starting weapon for each brother |
| 1 | Mace | 2–4 | Purchasable (30 gold) |
| 2 | Sword | 3–5 | Purchasable (45 gold) |
| 3 | Bow | 4–11 (missile) | Purchasable (75 gold); consumes `stuff[8]` per shot; auto-switches to next best weapon on depletion |
| 4 | Magic Wand | 4–11 (missile) | Fires fireballs; no ammo cost |

Equip via USE menu: `weapon = hit + 1`.

### 14.3 Special Items

| Index | Item | Effect |
|-------|------|--------|
| 5 | Golden Lasso | Enables mounting the swan carrier. Dropped by Witch (race `0x89`) on death. Requires Sun Stone first (witch must be killable). |
| 6 | Sea Shell | USE calls `get_turtle()` to summon sea turtle carrier near water. Blocked inside rectangle `(11194–21373, 10205–16208)`. Obtained from turtle NPC dialogue or ground pickup at `(10344, 36171)` in ob_list2/ob_list8. |
| 7 | Sun Stone | Makes Witch (race `0x89`) vulnerable to melee. Without it, attacks produce `speak(58)`. Ground pickup at `(11410, 36169)` in ob_list8. |
| 8 | Arrows | Integer count (max display 45). Consumed by bow (`stuff[8]--` per shot). Purchased in batches of 10 for 10 gold. |

### 14.4 Magic Consumables (`MAGICBASE = 9`)

All consumed on use (`--stuff[4+hit]`). Guarded by `extn->v3 == 9` check — magic does not work in certain restricted areas.

| Index | Item | Effect |
|-------|------|--------|
| 9 | Blue Stone | Teleport via stone circle (only at sector 144) |
| 10 | Green Jewel | `light_timer += 760` — temporary light effect brightening dark outdoor areas |
| 11 | Glass Vial | Heal: `vitality += rand8() + 4` (4–11), capped at `15 + brave/4` |
| 12 | Crystal Orb | `secret_timer += 360` — reveals hidden passages |
| 13 | Bird Totem | Renders overhead map with player position |
| 14 | Gold Ring | `freeze_timer += 100` — freezes all enemies (disabled while riding) |
| 15 | Jade Skull | Kill spell: kills all visible enemies with `vitality > 0`, `type == ENEMY`, `race < 7`. Decrements `brave` per kill — the only item that reduces bravery. |

### 14.5 Quest & Stat Items

| Index | Item | Effect |
|-------|------|--------|
| 22 | Talisman | Win condition: collecting triggers end sequence. Dropped by Necromancer (race `0x09`) on death; Necromancer transforms to normal man (race 10) and speaks `speak(44)`. |
| 23 | Rose | Lava immunity: forces `environ = 0` in fiery_death zone (`map_x` 8802–13562, `map_y` 24744–29544). Without it, `environ > 15` kills instantly; `environ > 2` drains vitality per tick. Protects player only (actor 0), not carriers or NPCs. Ground pickup at `(5473, 38699)`. |
| 24 | Fruit | Auto-consumed when `hunger > 30` at safe points, reducing hunger by 30. On pickup: stored only when `hunger < 15`; otherwise eaten immediately via `eat(30)`. 10 fruits placed in ob_list8. |
| 25 | Gold Statue | Desert gate key: need 5 to access Azal. Dual-gated: DESERT door type blocked when `stuff[25] < 5`, AND region 4 map tiles overwritten to impassable sector 254 at load time. |
| 26 | Book | Vestigial — defined in inventory system but no world placement, no handler, not obtainable. |
| 27 | Herb | Vestigial — defined in inventory system but no world placement, no handler, not obtainable. |
| 28 | Writ | Royal commission: obtained from `rescue()` after saving princess. Grants `princess++`, 100 gold, and 3 of each key type (`stuff[16..21] += 3`). Shown to Priest triggers `speak(39)` and reveals gold statue (`ob_listg[10]` set to stat 1). GIVE menu entry exists but has no handler — the Writ functions only as a passive dialogue check. |
| 29 | Bone | Found underground at `(3723, 39340)` in ob_list9. Given to Spectre (race `0x8a`): `speak(48)`, drops crystal shard. Non-spectre NPCs reject with `speak(21)`. |
| 30 | Crystal Shard | Overrides terrain type 12 collision blocking. Type-12 walls appear in dungeon labyrinth sectors (2, 3, 5–9, 11–12, 35) and doom tower sectors 137–138 near the stargate portal. Never consumed. Obtained from Spectre trade. |

#### Gold Statue Locations

All 5 statues use object ID `STATUE` (149), mapped to `stuff[25]` via `itrans`:

| # | Source | Location | How Obtained |
|---|--------|----------|-------------|
| 1 | `ob_listg[6]` | Seahold `(11092, 38526)` | Ground pickup (ob_stat=1) |
| 2 | `ob_listg[7]` | Ogre Den `(25737, 10662)` | Ground pickup (ob_stat=1) |
| 3 | `ob_listg[8]` | Octagonal Room `(2910, 39023)` | Ground pickup (ob_stat=1) |
| 4 | `ob_listg[9]` | Sorceress `(12025, 37639)` | Talk to Sorceress — revealed on first visit (stat set to 1) |
| 5 | `ob_listg[10]` | Priest `(6700, 33766)` | Show Writ to Priest — `speak(39)`, requires `stuff[28]` |

### 14.6 Gold Handling

Gold pickup items (stuff[31–34]) have `maxshown` values (2, 5, 10, 100) added to the `wealth` variable instead of `stuff[]`. Gold bag world object (ob_id 13) adds 50 to wealth directly as a special-cased pickup.

### 14.7 `itrans` — World Object to Inventory Mapping

31 `(ob_id, stuff_index)` pairs, terminated by `(0, 0)`. Lookup is a linear scan.

| World Object ID | Name | → stuff[] Index | Inventory Item |
|-----------------|------|----------------|----------------|
| 11 (QUIVER) | Quiver | 35 | Arrows (×10 via ARROWBASE accumulator) |
| 18 (B_STONE) | Blue Stone | 9 | Blue Stone |
| 19 (G_JEWEL) | Green Jewel | 10 | Green Jewel |
| 22 (VIAL) | Glass Vial | 11 | Glass Vial |
| 21 (C_ORB) | Crystal Orb | 12 | Crystal Orb |
| 23 (B_TOTEM) | Bird Totem | 13 | Bird Totem |
| 17 (G_RING) | Gold Ring | 14 | Gold Ring |
| 24 (J_SKULL) | Jade Skull | 15 | Jade Skull |
| 145 (M_WAND) | Magic Wand | 4 | Magic Wand |
| 27 | — | 5 | Golden Lasso |
| 8 | — | 2 | Sword |
| 9 | — | 1 | Mace |
| 12 | — | 0 | Dirk |
| 10 | — | 3 | Bow |
| 147 (ROSE) | Rose | 23 | Rose |
| 148 (FRUIT) | Fruit | 24 | Fruit |
| 149 (STATUE) | Gold Statue | 25 | Gold Statue |
| 150 (BOOK) | Book | 26 | Book |
| 151 (SHELL) | Sea Shell | 6 | Sea Shell |
| 155 | — | 7 | Sun Stone |
| 136 | — | 27 | Herb |
| 137 | — | 28 | Writ |
| 138 | — | 29 | Bone |
| 139 | — | 22 | Talisman |
| 140 | — | 30 | Crystal Shard |
| 25 (GOLD_KEY) | Gold Key | 16 | Gold Key |
| 153 (GREEN_KEY) | Green Key | 17 | Green Key |
| 114 (BLUE_KEY) | Blue Key | 18 | Blue Key |
| 242 (RED_KEY) | Red Key | 19 | Red Key |
| 26 (GREY_KEY) | Grey Key | 20 | Grey Key |
| 154 (WHITE_KEY) | White Key | 21 | White Key |

### 14.8 Special-Cased Pickups

These bypass `itrans` in the Take handler:

| ob_id | Item | Special Handling |
|-------|------|-----------------|
| 13 (MONEY) | Gold bag | `wealth += 50` |
| 20 (SCRAP) | Scrap | `event(17)` + region-specific event |
| 28 | Dead brother bones | Recovers dead brother's full inventory |
| 15 (CHEST) | Chest | Container → random loot (see §14.10) |
| 14 (URN) | Brass urn | Container → random loot |
| 16 (SACKS) | Sacks | Container → random loot |
| 102 (TURTLE) | Turtle eggs | Cannot be taken |
| 31 (FOOTSTOOL) | Footstool | Cannot be taken |

### 14.9 Shop System (`jtrans`)

7 purchasable items defined as `(stuff_index, price)` pairs. Requires proximity to shopkeeper (race `0x88`) and `wealth > price`.

| Menu Label | Item | Price | Effect |
|------------|------|-------|--------|
| Food | (special) | 3 gold | Calls `eat(50)` — reduces hunger by 50, not stored in stuff[] |
| Arrow | Arrows | 10 gold | `stuff[8] += 10` |
| Vial | Glass Vial | 15 gold | `stuff[11]++` |
| Mace | Mace | 30 gold | `stuff[1]++` |
| Sword | Sword | 45 gold | `stuff[2]++` |
| Bow | Bow | 75 gold | `stuff[3]++` |
| Totem | Bird Totem | 20 gold | `stuff[13]++` |

Menu label string: `"Food ArrowVial Mace SwordBow  Totem"`.

### 14.10 Container Loot

When a container (chest, urn, sacks) is opened, `rand4()` determines the tier:

| Roll | Result | Details |
|------|--------|---------|
| 0 | Nothing | "nothing." |
| 1 | One item | `rand8() + 8` → indices 8–15 (arrows or magic items). Index 8 → quiver. |
| 2 | Two items | Two different random items from same range. Index 8 → 100 gold. |
| 3 | Three of same | Three copies of one item. Index 8 → 3 random keys (`KEYBASE` to `KEYBASE+5`). |

### 14.11 GIVE Mode

| Menu Hit | Action |
|----------|--------|
| Gold | Give 2 gold to NPC. `wealth -= 2`. If `rand64() > kind`, `kind++`. Beggars (`0x8d`) give goal speech. |
| Writ | Handled via TALK/Priest interaction, not the GIVE handler. |
| Bone | Give to Spectre (`0x8a`): `speak(48)`, drops crystal shard via `leave_item()`. Non-spectre NPCs: `speak(21)`. |

### 14.12 Menu System

#### Menu Modes

```
enum cmodes {ITEMS=0, MAGIC, TALK, BUY, GAME, SAVEX, KEYS, GIVE, USE, FILE};
```

Menu item availability managed by `set_options()`, which calls `stuff_flag(index)` — returns 10 (enabled) if `stuff[index] > 0`, else 8 (disabled). The Book in the GIVE menu is hardcoded disabled: `menus[GIVE].enabled[6] = 8`.

### 14.13 World Object Structure

Each world object is a 6-byte record (`struct object`):

| Field | Type | Purpose |
|-------|------|---------|
| `xc` | `u16` | World X coordinate (pixel-space, 0–65535) |
| `yc` | `u16` | World Y coordinate (pixel-space, 0–65535) |
| `ob_id` | `i8` | Object type identifier (see §14.15) |
| `ob_stat` | `i8` | Object status code |

#### ob_stat Values

| Value | Meaning | Rendering |
|-------|---------|-----------|
| 0 | Disabled | Skipped |
| 1 | On ground (pickable) | OBJECTS type, `race=1` |
| 2 | In inventory / taken | Skipped |
| 3 | Setfig (NPC character) | NPC with `state=STILL` |
| 4 | Dead setfig | NPC with `state=DEAD` |
| 5 | Hidden (revealed by Look) | OBJECTS type, `race=0` |
| 6 | Cabinet item | OBJECTS type, `race=2` |

The `race` values on OBJECTS entries encode interaction behavior: `race=0` = not directly pickable (revealed by Look), `race=1` = normal ground item, `race=2` = cabinet item.

### 14.14 Region Object Lists

Objects are organized into one global list (always processed) and 10 regional lists (only the current region's list is processed each tick).

#### ob_listg — Global Objects (11 entries)

| Index | ob_id | Initial ob_stat | Purpose |
|-------|-------|----------------|---------|
| 0 | 0 | 0 | Drop slot — overwritten by `leave_item()` for dynamically dropped items |
| 1 | 28 (bones) | 0 | Dead brother 1 (Julian) — coordinates filled at death |
| 2 | 28 (bones) | 0 | Dead brother 2 (Phillip) — coordinates filled at death |
| 3 | 11 (ghost) | 0 | Ghost brother 1 — activated during succession |
| 4 | 11 (ghost) | 0 | Ghost brother 2 — activated during succession |
| 5 | 10 (spectre) | 3 | Spectre NPC — toggles visibility: `lightlevel < 40` → stat 3, else stat 2 |
| 6 | STATUE (149) | 1 | Gold statue — Seahold |
| 7 | STATUE (149) | 1 | Gold statue — Ogre Den |
| 8 | STATUE (149) | 1 | Gold statue — Octagonal Room |
| 9 | STATUE (149) | 0 | Gold statue — Sorceress (hidden until first talk → stat 1) |
| 10 | STATUE (149) | 0 | Gold statue — Priest (hidden until writ presented → stat 1) |

#### Regional Lists (ob_list0–ob_list9)

Each outdoor region (0–7) has pre-placed objects plus 10 blank slots (`TENBLANKS`) reserved for random treasure. Regions 8 and 9 have larger fixed lists and no random scatter slots.

| Region | List | Initial Count | Description |
|--------|------|---------------|-------------|
| 0 | `ob_list0` | 3 | Snow Land — 3 rangers |
| 1 | `ob_list1` | 1 | Maze Forest North — turtle eggs |
| 2 | `ob_list2` | 5 | Swamp Land — 2 wizards, ranger, sacks, shell |
| 3 | `ob_list3` | 12 | Tambry / Manor area — mixed NPCs and items |
| 4 | `ob_list4` | 3 | Desert — 2 dummies + beggar |
| 5 | `ob_list5` | 5 | Farm & City — beggar, 2 wizards, ring, chest |
| 6 | `ob_list6` | 1 | Lava Plain — dummy object |
| 7 | `ob_list7` | 1 | Southern Mountains — dummy object |
| 8 | `ob_list8` | 77 | Building Interiors — indices 0–15 setfig NPCs, 16–60 ground items, 61–76 hidden Look items (ob_stat=5) |
| 9 | `ob_list9` | 9 | Underground — 4 wands, 2 chests, wizard, money, king's bone |

### 14.15 Object ID Registry

Named constants from the `enum obytes`:

| Value | Constant | Description |
|-------|----------|-------------|
| 0–10 | (setfig NPCs) | Wizard (0), Priest (1), Guard (2/3), Princess (4), King (5), Noble (6), Sorceress (7), Bartender (8), Witch (9), Spectre (10) |
| 11 | `QUIVER` | Quiver of arrows |
| 13 | `MONEY` | 50 gold pieces |
| 14 | `URN` | Brass urn (container) |
| 15 | `CHEST` | Chest (container) |
| 16 | `SACKS` | Sacks (container) |
| 17–24 | `G_RING`..`J_SKULL` | Ring, stone, jewel, scrap, orb, vial, totem, skull |
| 25–26 | `GOLD_KEY`, `GREY_KEY` | Keys |
| 28 | (dead brother) | Brother's bones |
| 29 | 0x1d | Opened/empty chest |
| 31 | `FOOTSTOOL` | Cannot be taken |
| 102 | `TURTLE` | Turtle eggs — cannot be taken |
| 114 | `BLUE_KEY` | Blue key |
| 139 | (talisman) | Dropped by Necromancer on death |
| 140 | (crystal shard) | Dropped when giving bone to Spectre |
| 145–151 | `M_WAND`..`SHELL` | Wand, meal, rose, fruit, statue, book, shell |
| 153–154 | `GREEN_KEY`, `WHITE_KEY` | Keys |
| 242 | `RED_KEY` | Red key |

### 14.16 Object Management Arrays

**`ob_table[10]`**: Maps region numbers to object list pointers (`ob_list0`..`ob_list9`).

**`mapobs[10]`**: Tracks current entry count per region. Initial values: `{ 3, 1, 5, 12, 3, 5, 1, 1, 77, 9 }`. Mutable — incremented as random treasure is distributed.

**`dstobs[10]`**: Tracks whether random treasure has been distributed per region. Initial values: `{ 0, 0, 0, 0, 0, 0, 0, 0, 1, 1 }`. Regions 8 and 9 start as 1 (excluded). Set to 1 after distribution.

### 14.17 Per-Tick Object Processing (`do_objects`)

1. Set `j1 = 2` — starting anim_list index for setfig NPCs (0 = hero, 1 = carrier are reserved)
2. Call `set_objects(ob_listg, glbobs, 0x80)` — process global objects with flag `0x80`
3. Call `set_objects(ob_table[region_num], mapobs[region_num], 0)` — process regional objects
4. If `j1 > 3`, update `anix = j1` — adjusts the setfig/enemy boundary in anim_list

### 14.18 `set_objects` — Region Load and Rendering

**Random treasure distribution**: On first region load (when `dstobs[region_num] == 0` and `new_region >= 10`), 10 random objects are scattered from `rand_treasure[]`:

| Distribution | Items |
|-------------|-------|
| 4/16 | SACKS |
| 3/16 | GREY_KEY |
| 2/16 | CHEST |
| 1/16 each | MONEY, GOLD_KEY, QUIVER, RED_KEY, B_TOTEM, VIAL, WHITE_KEY |

Positions randomized within the region's quadrant, rejecting non-traversable terrain via `px_to_im()`. After distribution, `dstobs[region_num]` is set to 1 and `mapobs[region_num]` incremented per new object.

**Per-object processing**: For each object, performs a screen-bounds check, then:
- **Setfigs** (ob_stat 3/4): Loads sprite via `setfig_table[id]`, creates anim_list entry with `type=SETFIG`, `race = id + 0x80`, `vitality = 2 + id*2`, `goal = i` (list index). Dead setfigs (stat 4) get `state=DEAD`. Witch presence sets `witchflag = TRUE`.
- **Items** (ob_stat 1/5/6): Creates anim_list entry with `type=OBJECTS`, `index=ob_id`, `vitality = i + f` (list index + global flag `0x80`).
- **Resource limit**: Returns early if `anix2 >= 20`.

### 14.19 Object State Mutation (`change_object`)

Decodes the anim_list entry's `vitality` field to locate the original `struct object`: bit 7 selects global vs. regional list (`vitality & 0x80`), bits 0–6 give the list index (`vitality & 0x7f`).

- Normal objects: sets `ob_stat = flag`
- Chests: changes `ob_id` from CHEST (15) to `0x1d` (29, empty chest) instead of modifying `ob_stat`

Callers:
- Take handler: `change_object(nearest, 2)` — marks object as taken
- Look handler: `change_object(i, 1)` — reveals hidden items (ob_stat 5 → 1)

### 14.20 `leave_item` — Drop Item in World

Always uses `ob_listg[0]` (the dedicated drop slot), setting coordinates to the actor's position (+10 Y offset) and `ob_stat = 1`. Only one dynamically dropped item can exist at a time — each call overwrites the previous.

Callers:
- Necromancer death (race `0x09`) → talisman (ob_id 139)
- Witch death (race `0x89`) → golden lasso (ob_id 27)
- Bone given to Spectre → crystal shard (ob_id 140)

### 14.21 Save/Load of Object State

Object state is fully persisted:

1. `ob_listg` — 66 bytes (11 × 6)
2. `mapobs` — 20 bytes (10 × 2, current counts including random additions)
3. `dstobs` — 20 bytes (10 × 2, distribution flags)
4. All 10 regional lists — variable size based on current `mapobs[i]`

---


## 15. Quest System

### 15.1 Main Quest Flow

1. **Rescue princess** (up to 3 times): Enter princess extent with `ob_list8[9].ob_stat` set. Rewards: Writ, 100 gold, +3 of each key type, bird extent repositioned.
2. **Show Writ to Priest**: Talk to priest (setfig index 1) with `stuff[28]` → `speak(39)`, reveals golden statue (`ob_listg[10].ob_stat = 1`).
3. **Collect 5 golden figurines** (`stuff[25]`): Sorceress gives one on first visit (`ob_listg[9].ob_stat = 1`); Priest gives one with Writ; three ground pickups at Seahold, Ogre Den, and Octal Room.
4. **Enter hidden city of Azal**: Desert/oasis doors require `stuff[25] >= 5`. Find the Rose (`stuff[23]`) inside Azal for lava immunity.
5. **Obtain Crystal Shard**: Find King's Bone (`stuff[29]`) in underground, give to Spectre (night only) → Crystal Shard (`stuff[30]`) for terrain-12 barrier bypass.
6. **Obtain Sun Stone**: Defeat DreamKnight (40 HP, race 7) at Hidden Valley → access Elf Glade (door 48) → pick up Sun Stone (`stuff[7]`).
7. **Defeat the Witch**: Sun Stone makes Witch (race 0x89) vulnerable to all weapons; Bow/Wand work regardless. Witch drops Golden Lasso (`stuff[5]`) → enables swan flight.
8. **Cross lava to Citadel of Doom** (door 16): Rose provides fire immunity. Enter Doom castle interior, then Stargate portal (door 15) to Spirit Plane.
9. **Navigate Spirit Plane**: Crystal Shard required to pass terrain-12 barriers. Reach Necromancer's arena (sector 46, extent index 4).
10. **Defeat the Necromancer** (race 9, 50 HP): Only Bow or Wand (`weapon >= 4`) can damage. Magic blocked in arena. On death → transforms to Woodcutter (race 10), drops Talisman (object 139).
11. **Pick up the Talisman**: `stuff[22]` set → `quitflag = TRUE` → `win_colors()` victory sequence.

Note: Steps are not strictly ordered — the world is nonlinear — but item gates create this natural progression.

### 15.2 Quest State Flags

| Flag | Meaning | Set By | Cleared By |
|------|---------|--------|------------|
| `ob_list8[9].ob_stat` | Princess captive (nonzero = captive) | `revive(TRUE)` → 3 (`fmain.c:2843`) | `rescue()` → 0 (`fmain2.c:1601`) |
| `ob_listg[9].ob_stat` | Sorceress statue given | First talk → 1 (`fmain.c:3403`) | Never cleared |
| `ob_listg[10].ob_stat` | Priest statue given | Writ presented → 1 (`fmain.c:3384-3385`) | Never cleared |
| `ob_listg[5].ob_stat` | Spectre visibility | `lightlevel < 40` → 3, else → 2 (`fmain.c:2027-2028`) | Dynamically toggled by light level |
| `ob_listg[1-2].ob_stat` | Dead brother bones | Brother death → 1 (`fmain.c:2839`) | Bones picked up → 0 (implicit) |
| `ob_listg[3-4].ob_stat` | Ghost brothers | Brother death → 3 (`fmain.c:2841`) | Bones picked up → 0 (`fmain.c:3174`) |
| `stuff[22]` | Talisman held | Necromancer death → pickup | Triggers win |
| `stuff[25]` | Gold statue count | Various (§15.3) | Never decremented |
| `stuff[28]` | Writ | `rescue()` → 1 (`fmain2.c:1598`) | Never cleared |
| `stuff[29]` | King's Bone | Ground pickup (`ob_list9[8]`) | Spectre trade → 0 (`fmain.c:3503`) |
| `stuff[30]` | Crystal Shard | Spectre drops object 140 | Never cleared |
| `princess` | Rescue counter (0–2) | `rescue()` → increment (`fmain2.c:1594`) | Never reset (persists across brothers) |

### 15.3 Gold Statue Sources

Five golden figurines required to access desert/Azal (`stuff[25] >= 5`):

| # | Source | Object | Location (x, y) | Mechanism |
|---|--------|--------|-----------------|-----------|
| 1 | Sorceress | ob_listg[9] | (12025, 37639) | Talk → `speak(45)`, sets `ob_listg[9].ob_stat = 1` |
| 2 | Priest | ob_listg[10] | (6700, 33766) | Talk with Writ → `speak(39)`, sets `ob_listg[10].ob_stat = 1` |
| 3 | Seahold | ob_listg[6] | (11092, 38526) | Ground pickup via `itrans` |
| 4 | Ogre Den | ob_listg[7] | (25737, 10662) | Ground pickup via `itrans` |
| 5 | Octal Room | ob_listg[8] | (2910, 39023) | Ground pickup via `itrans` |

Dialogue-revealed statues (Sorceress, Priest) work through standard Take: setting `ob_stat = 1` makes the world object visible, and the player picks it up via `itrans` like any ground object.

### 15.4 Key Quest Items

| Item | stuff[] | Obtained From | Purpose |
|------|---------|---------------|---------|
| Talisman | stuff[22] | Necromancer drops on death (obj 139) | Picking it up wins the game |
| Rose | stuff[23] | Ground pickup, `ob_list8[51]` | Lava immunity (`environ = 0` in `fiery_death` zone) |
| Gold Statues ×5 | stuff[25] | Various (§15.3) | Gate to desert/Azal |
| Writ | stuff[28] | Princess rescue → `fmain2.c:1598` | Show to Priest for Gold Statue |
| King's Bone | stuff[29] | Ground pickup, `ob_list9[8]` | Give to Spectre for Crystal Shard |
| Crystal Shard | stuff[30] | Give Bone to Spectre (obj 140) | Walk through terrain type 12 barriers |
| Sun Stone | stuff[7] | Ground pickup, `ob_list8[18]`, inside Elf Glade (door 48) | Makes Witch vulnerable to all weapons |
| Golden Lasso | stuff[5] | Witch drops on death (obj 27) | Enables riding the Swan |
| Sea Shell | stuff[6] | Talk to Turtle carrier | Summon Turtle for ocean travel |

### 15.5 Quest State Gates

| Gate | Condition | Effect | Citation |
|------|-----------|--------|----------|
| Desert/Azal entrance | `stuff[STATBASE] < 5` | DESERT-type doors blocked | `fmain.c:1919` |
| Azal city map | `stuff[25] < 5` | Tiles overwritten to impassable 254 | `fmain.c:3594-3596` |
| King's castle pax | `xtype == 81` | `event(15)` — weapon sheathed | `fmain.c:1413` |
| Sorceress pax | `xtype == 82` | `event(16)` — calming influence | `fmain.c:1414` |
| Witch invulnerability | `weapon < 4 && (race==9 \|\| (race==0x89 && stuff[7]==0))` | Damage blocked | `fmain2.c:231-233` |
| Necromancer invulnerability | `weapon < 4` | Damage blocked to race 9 | `fmain2.c:231-232` |
| Spectre/Ghost immunity | Absolute | `dohit()` returns early for 0x8a/0x8b | `fmain2.c:234` |
| Magic blocked in necro arena | `extn->v3 == 9` | `speak(59)` ("Your magic won't work here") | `fmain.c:3305` |
| Crystal shard passwall | `stuff[30] && j==12` | Bypass terrain type 12 collision | `fmain.c:1609` |
| Rose lava protection | `stuff[23]` | `environ = 0` (no lava damage) | `fmain.c:1844` |
| Golden lasso + bird | `stuff[5]` | Required to ride bird carrier | `fmain.c:1498` |

### 15.6 Princess Rescue Sequence

Triggered when the player enters the princess extent (`xtype == 83`, coordinates 10820–10877, 35646–35670) and `ob_list8[9].ob_stat` is set (`fmain.c:2684-2685`). Cheat shortcut: `'R' && cheat1` (`fmain.c:1333`).

`rescue()` function (`fmain2.c:1584-1603`):

1. `map_message()` + `SetFont(rp, afont)` — enter fullscreen text mode with Amber font.
2. Compute text offset `i = princess * 3` — indexes princess-specific placard text.
3. Display rescue story: `placard_text(8+i)`, `name()`, `placard_text(9+i)`, `name()`, `placard_text(10+i)`, then `placard()` + `Delay(380)` (~7.6 sec).
4. Clear inner rectangle, display post-rescue text: `placard_text(17)` + `name()` + `placard_text(18)`, `Delay(380)` (~7.6 sec).
5. `message_off()` — restore normal display.
6. `princess++` — advance counter.
7. `xfer(5511, 33780, 0)` — teleport hero near King's castle.
8. `move_extent(0, 22205, 21231)` — reposition bird extent from southern mountains to Marheim farmlands.
9. `ob_list8[2].ob_id = 4` — place rescued princess NPC in castle.
10. `stuff[28] = 1` — give Writ item.
11. `speak(18)` — King says "Here is a writ designating you as my official agent…"
12. `wealth += 100` — gold reward.
13. `ob_list8[9].ob_stat = 0` — clear princess captive flag.
14. `for (i=16; i<22; i++) stuff[i] += 3` — give +3 of each key type.

**Three princesses**:

| `princess` value | Name | Placard Text Indices |
|------------------|------|----------------------|
| 0 | Katra | 8, 9, 10 |
| 1 | Karla | 11, 12, 13 |
| 2 | Kandy | 14, 15, 16 |

Shared post-rescue texts: `placard_text(17)` and `placard_text(18)`, used for all three princesses.

The `princess` counter persists across brother succession — `revive(TRUE)` does NOT reset it. However, `ob_list8[9].ob_stat` IS reset to 3 during `revive(TRUE)`, enabling each new brother to trigger a rescue with different princess text. After `princess >= 3`, no further rescues can fire because the third `rescue()` call clears `ob_stat` to 0 and subsequent placard indices would overflow.

### 15.7 Necromancer and Talisman

**Necromancer stats**: Race 9, 50 HP, weapon 5 (wand), aggressive. Extent at coordinates 9563–10144, 33883–34462 (`fmain.c:343`).

**Combat**: Only Bow (`weapon == 4`) or Wand (`weapon == 5`) can damage. Magic is blocked in the arena (`extn->v3 == 9` → `speak(59)`). Proximity auto-speak: `speak(43)` ("So this is the so-called Hero… Simply Pathetic.").

**On death** (`fmain.c:1750-1755`):
- Transforms to Woodcutter: `an->race = 10`, `an->vitality = 10`, `an->state = STILL`, `an->weapon = 0`.
- Drops the Talisman: `leave_item(i, 139)`.

World object 139 maps to `stuff[22]` via the `itrans` lookup (`fmain2.c:983`).

### 15.8 Win Condition and Victory Sequence

**Win check** — after every item pickup (`fmain.c:3244-3247`):

```
if (stuff[22])
{   quitflag = TRUE; viewstatus = 2;
    map_message(); SetFont(rp,afont); win_colors();
}
```

**Victory sequence** (`win_colors()` — `fmain2.c:1605-1636`):

1. Display victory placard: `placard_text(6)` + `name()` + `placard_text(7)` — "Having defeated the villainous Necromancer and recovered the Talisman, [name] returned to Marheim where he wed the princess…". `placard()` + `Delay(80)`.
2. Load win picture: `unpackbrush("winpic", bm_draw, 0, 0)` — IFF image from `game/winpic`.
3. Black out both viewports and hide HUD: `vp_text.Modes = HIRES | SPRITES | VP_HIDE`.
4. Expand playfield: `screen_size(156)`.
5. Sunrise animation — 55 frames (i=25 down to −29): slides a window across the `sun_colors[53]` gradient table (53 entries of 12-bit RGB values). Colors 2–27 fade in progressively, colors 29–30 transition through reds. First frame pauses 60 ticks; subsequent frames at 9 ticks each. Total: ~555 ticks ≈ 11.1 seconds.
6. Final pause `Delay(30)`, then blackout via `LoadRGB4(&vp_page, blackcolors, 32)`.

### 15.9 Game Termination

`quitflag` (`fmain.c:590`) controls the main loop `while (!quitflag)`:

| Trigger | Value | Meaning |
|---------|-------|---------|
| Game start | `FALSE` | Reset at `fmain.c:1269` |
| All brothers dead | `TRUE` | `fmain.c:2872` — game over after `placard_text(5)` + `Delay(500)` |
| Talisman picked up | `TRUE` | `fmain.c:3245` — victory |
| SAVEX → Exit | `TRUE` | `fmain.c:3466` — player quit |

After loop exits: `stopscore()` at `fmain.c:2616`, then `close_all()` at `fmain.c:2619-2620`.

### 15.10 Hidden City Reveal

When entering region 4 (desert) with fewer than 5 golden statues (`stuff[25] < 5`), four tiles at map offset `(11×128)+26` are overwritten with impassable tile 254 (`fmain.c:3594-3596`). With ≥ 5 statues, the tiles remain passable. Patch is applied on every region load (RAM-only modification).

Additionally, all 5 DESERT-type oasis doors (door indices 7–11) require `stuff[25] >= 5` to enter (`fmain.c:1919`). Without sufficient statues, door entry is blocked.

### 15.11 Stone Ring Teleportation Network

11 stone ring locations. Activation requires:
1. Standing on stone ring tile (sector 144)
2. Center-of-tile position (sub-tile check)
3. Match against `stone_list[]`

Destination = `(current_stone + facing + 1) % 11`. Direction determines which ring to teleport to. Visual effect: 32 frames of random palette cycling (`colorplay()`).

### 15.12 Brother Succession and Quest Continuity

On brother death with `luck < 1` (permanent), `revive(TRUE)` activates the next brother:

**Persists across brothers**:
- `princess` counter (advances through Katra → Karla → Kandy)
- All quest flags (`ob_listg`, `ob_list8` entries)
- Princess captive flag reset to 3 (`ob_list8[9].ob_stat = 3`) — enables next rescue
- Dead brother's bones (`ob_listg[1-2].ob_stat = 1`) and ghost (`ob_listg[3-4].ob_stat = 3`) placed in world

**Resets for new brother**:
- Stats loaded fresh from `blist[]` (Julian/Phillip/Kevin have different brave/luck/kind/wealth)
- Inventory cleared — new brother starts with only a Dirk (`stuff[0] = 1`)
- Position resets to Tambry (19036, 15755)
- Hunger and fatigue reset to 0

**Inventory recovery**: When a living brother picks up dead brother's bones (ob_id 28, `fmain.c:3173-3177`):
1. Both ghost setfigs removed: `ob_listg[3].ob_stat = ob_listg[4].ob_stat = 0`
2. Dead brother's entire 31-slot inventory merged into current brother's inventory

**Three brothers** (from `blist[]` — `fmain.c:2807-2812`):

| Brother | Brave | Luck | Kind | Wealth | Starting Vitality |
|---------|-------|------|------|--------|-------------------|
| Julian | 35 | 20 | 15 | 20 | 23 |
| Phillip | 20 | 35 | 15 | 15 | 20 |
| Kevin | 15 | 20 | 35 | 10 | 18 |

Vitality = `15 + brave/4`. Placard texts for succession:
- Julian starts: `placard_text(0)` + `event(9)`
- Phillip starts: `placard_text(1)`, `placard_text(2)` + `event(9)`, `event(10)`
- Kevin starts: `placard_text(3)`, `placard_text(4)` + `event(9)`, `event(11)`
- All dead: `placard_text(5)` ("Stay at Home!") → `quitflag = TRUE`


## 16. Doors & Buildings

### 16.1 Door Structure

```
struct door {
    xc1: u16,   // outside world X (pixel-space)
    yc1: u16,   // outside world Y (pixel-space)
    xc2: u16,   // inside world X (pixel-space)
    yc2: u16,   // inside world Y (pixel-space)
    type: i8,   // door visual/orientation type
    secs: i8,   // 1=buildings (region 8), 2=dungeons (region 9)
}
```

`DOORCOUNT = 86`. The table is sorted by `xc1` ascending to support binary search during outdoor→indoor transitions.

**secs field**:

| Value | Target Region | Description |
|-------|--------------|-------------|
| 1 | 8 | Building interiors (F9 image set) |
| 2 | 9 | Dungeons and caves (F10 image set) |

Region assignment: `if secs == 1 { new_region = 8 } else { new_region = 9 }`.

### 16.2 Door Type Constants

Horizontal types have bit 0 set (`type & 1`):

| Constant | Value | Orientation | Entries in doorlist |
|----------|-------|-------------|---------------------|
| HWOOD | 1 | Horizontal | 16 |
| VWOOD | 2 | Vertical | 3 |
| HSTONE | 3 | Horizontal | 11 |
| CRYST | 7 | Horizontal | 2 |
| BLACK | 9 | Horizontal | 5 |
| MARBLE | 10 | Vertical | 7 |
| LOG | 11 | Horizontal | 10 |
| HSTON2 | 13 | Horizontal | 12 |
| VSTON2 | 14 | Vertical | 2 |
| STAIR | 15 | Horizontal | 4 |
| DESERT | 17 | Horizontal | 5 |
| CAVE/VLOG | 18 | Vertical | 14 (4 cave + 10 cabin yard) |

CAVE and VLOG share value 18. Code checking `type == CAVE` also catches VLOG entries. Both use the same teleportation offset.

Unused defined types: VSTONE (4), HCITY (5), VCITY (6) — never appear in `doorlist[]`. SECRET (8) appears only in `open_list[]`.

#### Notable Door Patterns

- Entries 0–3: four identical copies of the same desert fort door (editing artifact, functionally harmless).
- 10 cabin pairs: each cabin has a VLOG "yard" door and a LOG "cabin" door (20 entries total).
- Crystal palace (idx 21–22): two adjacent doors for the same building.
- Stargate (idx 14–15): bidirectional portal. Entry 14 goes outdoor→region 8, entry 15 goes region 8→region 9.
- Village cluster (idx 31–39): 9 doors for the village.
- City cluster (idx 50–61): 12 doors for Marheim.

### 16.3 Locked Door System

#### Key Enum

```
enum ky { NOKEY=0, GOLD=1, GREEN=2, KBLUE=3, RED=4, GREY=5, WHITE=6 };
```

#### `struct door_open`

| Field | Type | Purpose |
|-------|------|---------|
| `door_id` | `u8` | Sector tile ID of the closed door |
| `map_id` | `u16` | Image block number identifying the door's region |
| `new1` | `u8` | Primary replacement tile ID |
| `new2` | `u8` | Secondary replacement tile ID (0 = none) |
| `above` | `u8` | Tile placement mode: 0=none, 1=above, 2=side, 3=back, 4=special cabinet |
| `keytype` | `u8` | Key required: 0=none, 1–6 per enum |

#### `open_list[17]`

| Idx | Key | Description |
|-----|-----|-------------|
| 0 | GREEN | HSTONE door (outdoor stone buildings) |
| 1 | NOKEY | HWOOD door (unlocked wooden doors) |
| 2 | NOKEY | VWOOD door (unlocked vertical wooden doors) |
| 3 | GREY | HSTONE2 door |
| 4 | GREY | VSTONE2 door |
| 5 | KBLUE | CRYST (crystal palace interiors) |
| 6 | GREEN | OASIS entrance |
| 7 | WHITE | MARBLE (keep doors) |
| 8 | GOLD | HGATE (gates) |
| 9 | GOLD | VGATE (vertical gates) |
| 10 | RED | SECRET passage |
| 11 | GREY | TUNNEL |
| 12 | GOLD | GOLDEN door (special 3-tile layout) |
| 13 | NOKEY | HSTON3 (unlocked) |
| 14 | NOKEY | VSTON3 (unlocked) |
| 15 | GREEN | CABINET (special 4-tile layout) |
| 16 | NOKEY | BLUE door (unlocked) |

Door tile changes are transient — they modify live `sector_mem` data only. Changes are lost when the sector reloads from disk. No save mechanism preserves opened door tiles.

### 16.4 `doorfind` Algorithm

Opens LOCKED doors (terrain tile type 15) by modifying map tiles. This system is separate from the `doorlist[]` teleportation system — `doorfind` operates on `open_list[]`.

1. **Locate terrain type 15** — tries `px_to_im(x, y)`, `px_to_im(x+4, y)`, `px_to_im(x-8, y)`
2. **Find top-left corner** — scans left (up to 2×16 px) and down (32 px)
3. **Convert to image coordinates** — `x >>= 4; y >>= 5`
4. **Get sector/region IDs** — `sec_id = *(mapxy(x, y))`, `reg_id = current_loads.image[(sec_id >> 6)]`
5. **Search `open_list[17]`** — match `map_id == reg_id && door_id == sec_id`, with key check `keytype == 0 || keytype == open_list[j].keytype`
6. **Replace tiles** — writes new tile IDs into `sector_mem` via `mapxy()`. Placement varies by `above` field
7. **Failure** — prints "It's locked." (suppressed by `bumped` flag)

#### Key Usage — Menu Handler

Player selects a key from the KEYS submenu. All 9 directions (0–8) at 16-pixel distance are checked via `doorfind(x, y, keytype)`. On success, the key is consumed (`stuff[hit + KEYBASE]--`).

#### Collision-Triggered Opening

When the player bumps terrain type 15, `doorfind(xtest, ytest, 0)` is called automatically. This opens only NOKEY doors (keytype match requires `keytype == 0`).

### 16.5 Region Transitions

#### Outdoor → Indoor (binary search)

Triggered when `region_num < 8` and the hero's aligned position matches a doorlist entry:

1. **Align** to 16×32 tile grid: `xtest = hero_x & 0xfff0; ytest = hero_y & 0xffe0`
2. **Riding check**: if riding, abort — cannot enter doors while mounted
3. **Binary search** on `doorlist` by `xc1`
4. **Orientation check**: horizontal doors skip if `hero_y & 0x10` is set; vertical doors skip if `(hero_x & 15) > 6`
5. **DESERT gate**: if `type == DESERT && stuff[STATBASE] < 5`, abort — need ≥5 gold statues
6. **Destination offset** by type:
   - CAVE/VLOG: `(xc2 + 24, yc2 + 16)`
   - Horizontal: `(xc2 + 16, yc2)`
   - Vertical: `(xc2 - 1, yc2 + 16)`
7. **Teleport**: `xfer(xtest, ytest, FALSE)`
8. **Visual transition**: `fade_page(100, 100, 100, TRUE, pagecolors)`

#### Indoor → Outdoor (linear scan)

Triggered when `region_num >= 8` and the hero matches a doorlist's `xc2`/`yc2`:

1. **Linear scan** through all 86 entries
2. **Match on `xc2`/`yc2`** with wider hit zone for horizontal doors
3. **Destination offset** by type:
   - CAVE/VLOG: `(xc1 - 4, yc1 + 16)`
   - Horizontal: `(xc1 + 16, yc1 + 34)`
   - Vertical: `(xc1 + 20, yc1 + 16)`
4. **Teleport**: `xfer(xtest, ytest, TRUE)` — TRUE recalculates region from position
5. **No fade** — exiting is instant, unlike entering

### 16.6 The `xfer()` Function

Performs teleportation between regions:

1. Adjust map scroll by same delta as hero position
2. Set hero position to destination
3. Clear encounters
4. If exiting indoors (`recalc` flag TRUE): recalculate region from coordinates
5. Load region data
6. Regenerate minimap
7. Force full screen redraw
8. Update music mood
9. Nudge hero downward if colliding with solid object at destination

### 16.7 Quicksand → Dungeon Transition

A non-door transition. When the player fully sinks (`environ == 30`) at `hero_sector == 181`: teleport to `(0x1080, 34950)` in region 9. NPCs caught in the same quicksand die.


## 17. Day/Night Cycle

### 17.1 Day Counter

`daynight` is a 16-bit unsigned integer (`USHORT`), cycling from 0 to 23999. Incremented by 1 each non-scrolling game tick:

```
if (!freeze_timer)
    if ((daynight++) >= 24000) daynight = 0;
```

Does not advance during freeze spells. During sleep: `daynight += 63` per tick (plus normal +1 = 64 effective advance). Initialized to 8000 (morning) during `revive()`.

Full cycle = 24000 ticks (≈ 6.7 minutes at 60 Hz in the original game).

### 17.2 Light Level

Triangle wave derived from `daynight`:

```
lightlevel = daynight / 40
if (lightlevel >= 300) lightlevel = 600 − lightlevel
```

| daynight | lightlevel | Phase |
|----------|------------|-------|
| 0 | 0 | Midnight (darkest) |
| 6000 | 150 | Dawn |
| 12000 | 300 | Noon (brightest) |
| 18000 | 150 | Dusk |
| 23999 | 1 | Just before midnight |

### 17.3 Time-of-Day Events

`dayperiod = daynight / 2000` (values 0–11). Transitions trigger text events:

| Period | daynight Range | Event | Message |
|--------|---------------|-------|---------|
| 0 | 0–1999 | event(28) | "It was midnight." |
| 4 | 8000–9999 | event(29) | "It was morning." |
| 6 | 12000–13999 | event(30) | "It was midday." |
| 9 | 18000–19999 | event(31) | "Evening was drawing near." |

Periods 1–3, 5, 7–8, 10–11 are silent transitions.

### 17.4 Spectre Night Visibility

When `lightlevel < 40` (deep night, daynight < 1600 or > 22400): `ob_listg[5].ob_stat = 3` (visible and interactive). Otherwise: `ob_listg[5].ob_stat = 2` (hidden).

### 17.5 Palette Fading (`day_fade`)

Called every tick. Updates palette every 4 ticks (`(daynight & 3) == 0`) or during screen rebuild (`viewstatus > 97`):

```
day_fade():
    ll = 200 if light_timer > 0 else 0
    if ((daynight & 3) == 0 || viewstatus > 97):
        if region_num < 8:
            fade_page(lightlevel − 80 + ll, lightlevel − 61, lightlevel − 62, TRUE, pagecolors)
        else:
            fade_page(100, 100, 100, TRUE, pagecolors)
```

- **Green Jewel light bonus**: `light_timer > 0` adds 200 to the red parameter, producing a warm amber glow even at night.
- **Indoor override**: `region_num >= 8` → always full brightness `(100, 100, 100)`.

### 17.6 RGB Component Fading (`fade_page`)

Per-component palette scaler applied to the 32-color palette.

**Color 31 override** (sky color):

| Region | Color 31 | Meaning |
|--------|----------|---------|
| 4 (desert) | `0x0980` | Orange-brown desert sky |
| 9 (dungeon), `secret_timer` active | `0x00f0` | Bright green (secret revealed) |
| 9 (dungeon), normal | `0x0445` | Dark grey-blue |
| All others | `0x0bdf` | Light blue sky |

**Clamping** (with `limit = TRUE`):

- Red: min 10, max 100
- Green: min 25, max 100
- Blue: min 60, max 100
- Blue shift factor: `g2 = (100 − g) / 3`

**Per-color computation** (for each of 32 palette entries):

1. Extract 12-bit RGB components from `pagecolors[]`
2. Green Jewel effect: if `light_timer` active and red < green, boost red to match green
3. Scale: `r1 = (r × r1) / 1600`, `g1 = (g × g1) / 1600`, `b1 = (b × b1 + g2 × g1) / 100`
4. Nighttime vegetation boost (colors 16–24): green 21–49 → +2 blue; green 50–74 → +1 blue

Result written to `fader[]` and loaded to hardware palette.

**Outdoor RGB at key times:**

| Phase | lightlevel | r (no jewel) | g | b |
|-------|------------|-------------|---|---|
| Midnight | 0 | clamped 10 | clamped 25 | clamped 60 |
| Dawn | 150 | 70 | 89 | 88 |
| Noon | 300 | clamped 100 | clamped 100 | clamped 100 |

With Green Jewel active: red parameter gets +200, so midnight red ≈ 120 (warm amber even in darkness).

### 17.7 Screen Transitions (`fade_down` / `fade_normal`)

- **`fade_down()`**: Steps all channels from 100 to 0 in increments of 5 (21 steps, `Delay(1)` each). Fades screen to black.
- **`fade_normal()`**: Steps from 0 to 100 in increments of 5. Fades back to full brightness.

Both use `limit = FALSE` — no night clamping or blue shift. Used for map messages, door transitions, and other screen changes.

### 17.8 Music Mood Selection (`setmood`)

Selects one of 7 four-channel music tracks based on game state. Priority (highest first):

| Track Offset | Indices | Condition | Music |
|-------------|---------|-----------|-------|
| 24 | 24–27 | `vitality == 0` (death) | Death theme |
| 16 | 16–19 | Astral plane coordinates | Astral theme |
| 4 | 4–7 | `battleflag` active | Battle theme |
| 20 | 20–23 | `region_num > 7` (indoor) | Indoor theme |
| 0 | 0–3 | `lightlevel > 120` (outdoor day) | Day theme |
| 8 | 8–11 | `lightlevel ≤ 120` (outdoor night) | Night theme |

Day/night music crossover: `lightlevel > 120` = day, `≤ 120` = night. Crossover at daynight ≈ 4800 (dawn) and ≈ 19200 (dusk).

Playback: `now = TRUE` → `playscore()` (immediate restart); `now = FALSE` → `setscore()` (crossfade). Mood re-evaluated every 8 ticks. Indoor waveform tweak: dungeons (region 9) use `new_wave[10] = 0x0307`; buildings (region 8) use `0x0100`.

### 17.9 Gameplay Effects

- **Encounter spawning**: Rate is constant regardless of time of day. `danger_level` depends on `region_num` and `xtype`, not `lightlevel`.
- **Innkeeper dialogue**: `dayperiod > 7` (evening/night) triggers lodging speech.
- **Vitality recovery**: Every 1024 ticks (`(daynight & 0x3FF) == 0`), hero regenerates +1 HP up to max. Tied to `daynight` counter but not time-of-day dependent.
- **Sleep**: Time passes 64× faster. Wake conditions include morning window (daynight 9000–10000).

### 17.10 Palette Data

- **`pagecolors[32]`**: Hardcoded 32-color base palette in 12-bit Amiga RGB. Same for all outdoor regions (0–7). Faded dynamically by `fade_page()`.
- **`textcolors[20]`**: Status bar palette (hi-res viewport). NOT affected by day/night fading.
- **`blackcolors[32]`**: All-zero palette for instant blackout transitions.
- **`sun_colors[53]`**: Sunrise/sunset gradient for the victory sequence `win_colors()`.
- **`introcolors[32]`**: Title/intro screen palette, separate from gameplay.
- **`colorplay()`**: Teleportation effect — 32 frames of random 12-bit RGB colors for all palette entries except color 0.

---


## 18. Survival Mechanics

### 18.1 Player Stats Overview

Six core stats declared as global `short` variables: `brave`, `luck`, `kind`, `wealth`, `hunger`, `fatigue`.

Vitality is per-actor (`struct shape.vitality`), NOT a global. The hero's vitality is `anim_list[0].vitality`.

All stats are part of a contiguous saved block serialized via `saveload()`.

### 18.2 Hunger

**Increment**: +1 every 128 game ticks (`(daynight & 127) == 0`), when hero is alive and not sleeping.

**Thresholds:**

| Hunger | Effect |
|--------|--------|
| == 35 | event(0) — "getting rather hungry" (one-time) |
| == 60 | event(1) — "getting very hungry" (one-time) |
| > 90, `(hunger & 7) == 0` | event(2) — "starving!" (periodic, every 8th hunger increment) |
| > 100, `(hunger & 7) == 0` | `vitality -= 2` (only when `vitality > 5`) |
| > 120 | Movement wobble: direction ±1 with 75% probability (`rand4() != 0`; `rand2()` selects ±1) |
| > 140 | event(24) "passed out!", `hunger = 130`, `state = SLEEP` |

The `(hunger & 7) == 0` condition gates both starvation warnings and HP damage, firing every 8th hunger increment (≈ every 1024 daynight ticks).

HP damage fires when **either** `hunger > 100` **OR** `fatigue > 160` (logical OR):

```
if (anim_list[0].vitality > 5)
    if (hunger > 100 || fatigue > 160)
        anim_list[0].vitality -= 2; prq(4);
```

**Auto-Eating**: When `(daynight & 127) == 0` in a safe zone, if `hunger > 30` and `stuff[24] > 0` (has Fruit): `stuff[24]--; hunger -= 30; event(37)`. Uses direct subtraction, not via `eat()`.

### 18.3 Fatigue

**Increment**: +1 alongside hunger, same 128-tick timer and conditions.

**Thresholds:**

| Fatigue | Effect |
|---------|--------|
| == 70 | event(3) — "getting tired" (one-time) |
| == 90 | event(4) — "getting sleepy" (one-time) |
| > 160, `(hunger & 7) == 0` | `vitality -= 2` (shared condition with hunger > 100, see §18.2) |
| > 170 | event(12) — forced sleep (only when `vitality ≤ 5`) |

The forced sleep at `fatigue > 170` is in the `else` branch of the `vitality > 5` check — it only fires when HP is critically low.

### 18.4 Sleep Mechanics

**Voluntary sleep**: Inside buildings (region 8), standing on bed terrain tiles (IDs 161, 52, 162, 53) increments `sleepwait`. After 30 ticks:

- `fatigue < 50` → event(25) "not sleepy" — no sleep
- `fatigue >= 50` → event(26) "decided to lie down and sleep", `state = SLEEP`

**Sleep processing** (each frame while sleeping):

- `daynight += 63` — time advances rapidly (64× normal with the +1 increment)
- `fatigue--` (if > 0)
- Wake conditions (any): `fatigue == 0`, OR (`fatigue < 30` AND `daynight ∈ [9000, 10000)`), OR (`battleflag` AND `rand64() == 0`)
- On waking: `state = STILL`, Y-position snapped to grid alignment

### 18.5 `eat()` Function

```
eat(amt):
    hunger -= amt
    if hunger < 0: hunger = 0; event(13)   // "full"
    else: print("Yum!")
```

| Food Source | Amount | Notes |
|-------------|--------|-------|
| Pickup fruit (hungry, hunger ≥ 15) | `eat(30)` | Via `eat()` function |
| Buy food from shop | `eat(50)` | Via `eat()` function |
| Auto-eat fruit in safe zone | `hunger -= 30` | Direct subtraction, not via `eat()` |

When `hunger < 15`, picked-up fruit is stored instead of eaten: `stuff[24]++; event(36)`.

### 18.6 Vitality / HP

**Max HP formula**: `15 + brave / 4`

Used at: natural healing cap, revive, heal vial cap, priest healing.

**Natural healing**: Every 1024 ticks (`(daynight & 0x3FF) == 0`), hero gains +1 vitality up to max. During sleep, `daynight` advances by 63 per frame, so healing occurs ≈63× faster.

**Heal vial**: `vitality += rand8() + 4` (4–11 HP), capped at max.

**Priest healing**: Full heal to `15 + brave / 4`. Requires `kind >= 10`; below 10, priest gives dismissive dialogue.

**Damage sources:**

| Source | Amount | Condition |
|--------|--------|-----------|
| Combat hits | `vitality -= wt` (weapon table value) | Per hit |
| Hunger/fatigue | −2 | When `(hunger & 7) == 0` and (hunger > 100 OR fatigue > 160) and vitality > 5 |
| Drowning (`environ == 30`) | −1 | Every 8 ticks |
| Lava (`environ > 2`) | −1 per tick | `environ > 15` = instant death |

Rose (`stuff[23]`) prevents lava damage by forcing `environ = 0` each tick.

### 18.7 Stat Changes

#### Bravery

| Change | Condition |
|--------|-----------|
| +1 | Kill any non-hero actor |
| −1 | Per target killed by Jade Skull |

Initial values: Julian = 35, Phillip = 20, Kevin = 15.

Combat effects: hero melee hit range = `brave / 20 + 5`, hero missile bravery = full `brave` value, enemy hit dodge = `rand256() > brave`.

#### Luck

| Change | Condition |
|--------|-----------|
| −5 | Hero death |
| −2 | Fall into pit |
| +5 (probabilistic) | Sorceress talk: `if (luck < rand64()) luck += 5` |

Clamped ≥ 0 on HUD redraw. Initial values: Julian = 20, Phillip = 35, Kevin = 20. Luck < 1 after death triggers brother succession instead of fairy rescue.

#### Kindness

| Change | Condition |
|--------|-----------|
| −3 | Kill non-witch SETFIG |
| +1 (probabilistic) | Give gold: `if (rand64() > kind) kind++` |

Clamped ≥ 0 in `checkdead`. Initial values: Julian = 15, Phillip = 15, Kevin = 35. Below 10: wizards and priests give dismissive dialogue.

#### Wealth

| Change | Condition |
|--------|-----------|
| +50 | Loot gold bag (MONEY pickup) |
| +100 | Container gold |
| +100 | Princess rescue reward |
| +variable | Corpse loot (`inv_list[j].maxshown` for gold items) |
| −price | Buy item from shop |
| −2 | Give gold to NPC |

Initial values: Julian = 20, Phillip = 15, Kevin = 10.

### 18.8 Safe Zones

Updated every 128 ticks when ALL conditions met:

- No enemies visible or loading
- No witch encounter active
- Hero on solid ground (`environ == 0`)
- No danger flag
- Hero alive

Safe zone coordinates (`safe_x, safe_y`) stored for fairy rescue respawn point.

### 18.9 Fiery Death Zone

Rectangle: `8802 < map_x < 13562`, `24744 < map_y < 29544`.

- Hero with rose (`stuff[23]`): immune (`environ` reset to 0 each tick)
- `environ > 15`: instant death
- `environ > 2`: −1 vitality per tick

### 18.10 HUD Display

Stats rendered via print queue:

- **prq(7)**: `Brv`, `Lck`, `Knd`, `Wlth` — four stat values
- **prq(4)**: `Vit` — vitality value

Hunger and fatigue are **not** displayed on the HUD — communicated only through event messages.

### 18.11 Random Number Generation

All random values in the game are produced by a Linear Congruential Generator (LCG):

```
seed1 = low16(seed1) × 45821 + 1       // 16×16→32 unsigned multiply, then +1
output = ror32(seed1, 6) & 0x7FFFFFFF   // rotate right 6 bits, clear sign bit
```

Initial seed: `19837325` (hex `0x012ED98D`). The 68000 `mulu.w` operates on the low 16 bits only, so the effective state space is 2^16 with a maximum period of 65536.

No runtime reseeding — the seed is not derived from system time, VBlank counter, or user input. Sequence variation between sessions comes only from the copy-protection input loop, where each keystroke calls `rand()` with the result discarded.

**Function family:**

| Function | Returns | Formula |
|----------|---------|---------|
| `rand()` | 0 to 0x7FFFFFFF (31-bit) | Base LCG output |
| `bitrand(x)` | `rand() & x` | Masked random |
| `rand2()` | 0 or 1 | `rand() & 1` |
| `rand4()` | 0–3 | `rand() & 3` |
| `rand8()` | 0–7 | `rand() & 7` |
| `rand64()` | 0–63 | `rand() & 63` |
| `rand256()` | 0–255 | `rand() & 255` |
| `rnd(n)` | 0 to n−1 | `(rand() & 0xFFFF) % n` |

The `bitrand`/`randN` variants use bitwise AND, producing uniform results only when the mask is a power-of-two minus one. `rnd(n)` uses true modulo via 16-bit division.

---


## 19. Magic System

### 19.1 Preconditions

1. Must have the item (`stuff[4 + hit] > 0`); otherwise event(21) "if only I had some Magic!"
2. Cannot use in Necromancer arena (`extn->v3 == 9`); `speak(59)`

### 19.2 Magic Items

| hit | Item | stuff[] | Effect |
|-----|------|---------|--------|
| 5 | Blue Stone | 9 | Teleport via standing stones. Requires `hero_sector == 144`, uses `stone_list[]`. Destination = `(current_stone + facing + 1) % 11`. |
| 6 | Green Jewel | 10 | `light_timer += 760`. Temporary light-magic effect: adds 200 to red channel in `day_fade()`, producing warm amber glow outdoors. |
| 7 | Glass Vial | 11 | Heal: `vitality += rand8() + 4` (4–11 HP), capped at `15 + brave / 4`. |
| 8 | Crystal Orb | 12 | `secret_timer += 360`. Reveals hidden passages while countdown active. In dungeons (region 9), changes color 31 to bright green (`0x00f0`). |
| 9 | Bird Totem | 13 | Renders overhead map with hero position marker. Sets `viewstatus = 1`. |
| 10 | Gold Ring | 14 | `freeze_timer += 100`. Freezes all enemies, stops daynight advance, suppresses encounters. Blocked if `riding > 1`. |
| 11 | Jade Skull | 15 | Kill spell: kills all visible enemies with `vitality > 0`, `type == ENEMY`, `race < 7`. `brave--` per kill (counterbalances normal combat `brave++`). |

### 19.3 Timer Effects

| Timer | Declared | While > 0 |
|-------|----------|-----------|
| `freeze_timer` | short | Enemies frozen, `daynight` frozen, encounters suppressed |
| `light_timer` | short | Green Jewel warm amber glow in `day_fade()` |
| `secret_timer` | short | Secret passages visible; dungeon color 31 = bright green |

All timers decrement by 1 each tick (when nonzero). All reset to 0 on brother succession.

### 19.4 Charge Depletion

After use: `stuff[4 + hit]--`. If the count reaches 0, `set_options()` rebuilds the menu to remove the depleted item. Failed uses (Blue Stone position check fails, Gold Ring blocked by riding) do NOT consume a charge.

---


## 20. Death & Revival

### 20.1 Death Detection (`checkdead`)

Triggers when `an->vitality < 1` and actor state is not already DYING or DEAD:

```
an->vitality = 0
an->tactic = 7
an->goal = DEATH
an->state = DYING
```

**Hero death** (`i == 0`): `event(dtype)` (death message by type), `luck -= 5`, `setmood(TRUE)` (death music).

**NPC kill** (`i != 0`):
- If SETFIG and not witch (0x89): `kind -= 3`
- If DreamKnight (race 7): `speak(42)`
- Always: `brave++`

Death types: 5 = combat, 6 = drowning, 27 = lava. DYING → DEAD transition occurs when `tactic` counts down to 0 during the death animation.

### 20.2 Fairy Rescue Mechanism

Activates when hero's state is DEAD or FALL. Uses `goodfairy` counter (`unsigned char`, starts at 0, wraps to 255 on first decrement).

**Timeline after hero enters DEAD/FALL state:**

| `goodfairy` Range | Frames | Behavior |
|-------------------|--------|----------|
| 255–200 | 2–57 | **Death sequence plays.** Death animation and death song always complete fully before any rescue decision. |
| 199–120 | 58–137 | **Luck gate**: `luck < 1` → `revive(TRUE)` (brother succession). FALL state → `revive(FALSE)` (non-lethal recovery). `luck >= 1` and DEAD → countdown continues toward fairy rescue. |
| 119–20 | 138–237 | Fairy sprite visible, flying toward hero. `battleflag = FALSE`. AI suspended. |
| 19–2 | 238–255 | Resurrection glow effect. |
| 1 | 256 | `revive(FALSE)` — fairy rescues hero, same brother continues. |

**Key design property**: The luck gate is **fully deterministic** with no random element. `checkdead()` sets `luck -= 5` on hero death. Luck cannot change during DEAD state because:

- `checkdead` is guarded by `state != DYING && state != DEAD`
- Pit fall luck loss requires movement
- Sorceress luck gain requires TALK interaction

If `luck >= 1` when the gate first fires, fairy rescue is guaranteed. Since luck cannot change during DEAD state, the gate is effectively a one-time decision at the moment `goodfairy` first drops below 200. FALL state always gets `revive(FALSE)` regardless of luck (pit falls are non-lethal).

### 20.3 Brother Base Stats (`blist[]`)

| Brother | `brother` | brave | luck | kind | wealth | Starting HP (`15 + brave/4`) | Max Fairy Rescues |
|---------|-----------|-------|------|------|--------|------------------------------|-------------------|
| Julian | 1 | 35 | 20 | 15 | 20 | 23 | 3 |
| Phillip | 2 | 20 | 35 | 15 | 15 | 20 | 6 |
| Kevin | 3 | 15 | 20 | 35 | 10 | 18 | 3 |

Each brother has an independent 35-byte inventory array (`ARROWBASE = 35`): `julstuff`, `philstuff`, `kevstuff`.

Design: Julian is the strongest fighter (highest bravery/HP), Phillip has the most fairy rescues available (highest luck), Kevin is the diplomat (highest kindness, weakest combatant).

### 20.4 `revive()` — Resurrection and Succession

`revive(new)`: `new = TRUE` for brother succession, `new = FALSE` for fairy rescue/fall recovery.

#### Common Setup (both paths)

- `anim_list[1]` placed as RAFT, `anim_list[2]` as SETFIG (reset carriers)
- `handler_data.laydown = handler_data.pickup = 0`
- `battleflag = goodfairy = mdex = 0`

#### New Brother Path (`new == TRUE`)

1. **Place dead brother ghost** (brothers 1–2 only; Kevin has no successor):
   - `ob_listg[brother].xc/yc = hero_x/hero_y; ob_stat = 1` — bones at death location
   - `ob_listg[brother + 2].ob_stat = 3` — ghost setfig activated

2. **Load new brother stats**:
   - `ob_list8[9].ob_stat = 3` — re-enable princess as captive
   - Load stats from `blist[brother]`: `brave, luck, kind, wealth`
   - `stuff` pointer switches to new brother's inventory array
   - `brother++`

3. **Clear inventory**: Zero first 31 slots (`GOLDBASE`). Give one Dirk: `stuff[0] = an->weapon = 1`.

4. **Reset timers**: `secret_timer = light_timer = freeze_timer = 0`. Spawn at `(19036, 15755)` in region 3 (Tambry area).

5. **Display placard** (brother-specific):
   - Brother 1 (Julian): `placard_text(0)` — "Rescue the Talisman!"
   - Brother 2 (Phillip): `placard_text(1)` + `placard_text(2)` — Julian failed / Phillip sets out
   - Brother 3 (Kevin): `placard_text(3)` + `placard_text(4)` — Phillip failed / Kevin takes quest

6. **Load sprites**: `shape_read()` → `read_shapes(brother − 1)` for correct character sprite.

7. **Journey message**: event(9) "started the journey in Tambry", with brother-specific suffix: event(10) "as had his brother before him" for Phillip, event(11) "as had his brothers before him" for Kevin.

#### Fairy Rescue Path (`new == FALSE`)

Skips ghost placement, stat/inventory reset, and placard text. Hero respawns at current `safe_x, safe_y` with current stats. Only vitality, hunger, and fatigue are reset.

#### Common Finalization (both paths)

- Position: `hero_x = safe_x, hero_y = safe_y`
- Vitality: `15 + brave / 4` (full HP)
- Time: `daynight = 8000, lightlevel = 300` (morning)
- `hunger = fatigue = 0`
- `an->state = STILL; an->race = -1`

### 20.5 Dead Brother Ghost and Bones

**Ghost placement** (during succession): Bones object placed at hero's death coordinates. Ghost setfig activated to allow interaction. Ghost dialogue: `speak(49)` — "I am the ghost of your dead brother. Find my bones…"

**Bones pickup** (ob_id 28): When a living brother picks up bones:

1. Both ghost setfigs cleared: `ob_listg[3].ob_stat = ob_listg[4].ob_stat = 0`
2. Dead brother's inventory merged: `for (k = 0; k < GOLDBASE; k++) stuff[k] += dead_brother_stuff[k]`

Dead brother stuff pointer: index 1 = Julian's stuff, index 2 = Phillip's stuff.

### 20.6 Inventory Serialization (`mod1save`)

Serializes all three brothers' inventory arrays (35 bytes each) and the missile list. After loading, `stuff = blist[brother − 1].stuff` reassigns the active inventory pointer.

### 20.7 Game Over

When `brother > 3` (all three brothers dead):

- `placard_text(5)`: "And so ends our sad tale. The Lesson of the Story: Stay at Home!"
- `quitflag = TRUE`
- `Delay(500)` — 10-second pause (500 ticks at 50 Hz Amiga `Delay()` timing)

### 20.8 What Persists Across Brothers

| Persists | Resets |
|----------|--------|
| Princess counter (`princess`) | Stats (loaded fresh from `blist[]`) |
| Quest flags (`ob_listg`, `ob_list8` stats) | Inventory (zeroed; only a Dirk given) |
| Object world state (all `ob_list` data) | Position (back to Tambry 19036, 15755) |
| `dstobs[]` distribution flags | Hunger / fatigue (→ 0) |
| | Timers (secret, light, freeze → 0) |
| | `daynight` (→ 8000), `lightlevel` (→ 300) |

Princess counter persists across succession. However, `ob_list8[9].ob_stat` IS reset to 3 during `revive()`, enabling each new brother to trigger a rescue. After `princess >= 3`, no further rescues fire because `ob_list8[9].ob_stat` stays 0 after the third `rescue()` call.
```

**Key corrections from existing docs:**
1. **R-SURV-003**: Fixed `AND` → `OR` for hunger/fatigue HP damage condition (research clearly shows `||`)


## 21. Carriers & World Navigation

### 21.1 Carrier Types

| Carrier | `riding` value | Type constant | Sprite file | File ID | Rideable? |
|---------|---------------|---------------|-------------|---------|-----------|
| Raft | 1 | `RAFT` | cfiles[4] | 1348 | Yes (auto) |
| Turtle | 5 | `CARRIER` | cfiles[5] | 1351 | Yes (proximity) |
| Swan/Bird | 11 | `CARRIER` | cfiles[11] | 1120 | Yes (requires Golden Lasso) |
| Dragon | — | `DRAGON` | cfiles[10] | 1160 | **No** (hostile NPC) |

### 21.2 Raft

- Actor slot 1, type RAFT
- Activation: within 9px proximity, no active carrier, `wcarry==1`, terrain codes 3–5 (water/shore)
- Movement: snaps to hero position each frame (no autonomous movement)
- Prevents drowning while active (`riding == 1`)
- Dismount: automatic when proximity or terrain conditions fail

### 21.3 Turtle

- Actor slot 3, type CARRIER, `actor_file = 5`
- Summoned via USE menu (turtle item `stuff[6]`)
- Boarding: within 16px proximity, `wcarry==3`, sets `riding = 5`
- Speed: forced to 3 pixels/frame when ridden
- Autonomous movement (unridden): probes candidate positions with `px_to_im()`, only commits moves where terrain code is exactly 5 (water); tries current direction, then ±1, then −2
- Cannot be summoned in central region bounds (11194–21373 X, 10205–16208 Y)
- Mounted-turtle exploit: melee recoil from `dohit()` pushes rider via `move_figure(i,fc,2)`, which only checks `proxcheck()` and bypasses the turtle's `px_to_im(...)==5` water-only rule, enabling transit over invalid terrain (original behavior)

### 21.4 Swan (Bird)

- Actor slot 3, type CARRIER, `actor_file = 11`
- Extent zone 0 at (2118, 27237)
- Requires Golden Lasso (`stuff[5]`) to board
- Riding state: `riding = 11`, hero `environ = -2` (airborne)
- Movement: inertial flight physics
  - Velocity accumulates via directional acceleration
  - Max horizontal velocity ~32, max vertical ~40
  - Position updates by `vel/4` per frame
  - No terrain collision — `proxcheck` skipped
  - Auto-faces into wind via `set_course(0,-nvx,-nvy,6)`
- Dismount conditions: hero action button + velocity < 15 + clear ground below + not fiery terrain
  - Blocked in lava zone: event 32 ("Ground is too hot")
  - Blocked at high velocity: event 33 ("Flying too fast")
- On ground: renders as RAFT sprite

### 21.5 Dragon

- Actor slot 3, type DRAGON, `actor_file = 10`
- Extent zone 2 (dragon cave area)
- **Hostile** — not rideable
- HP: 50, shoots fireballs (type 2 missiles) with 25% chance per frame at speed 5
- Always faces south
- Can be killed

### 21.6 Carrier Loading

`load_carrier(n)` loads carrier sprites into the ENEMY shape memory slot — carriers and enemies share memory and cannot coexist. Carriers always occupy `anim_list[3]`. Loading sets `anix = 4` and positions the carrier at the center of its extent zone.

### 21.7 Carrier Interactions

| Interaction | Behavior |
|-------------|----------|
| Doors | All riding values block door entry |
| Random encounters | Suppressed while `active_carrier != 0` |
| Combat | Carriers skip melee/missile hit detection |
| Freeze spell | Blocked when `riding > 1` (turtle or swan) |
| Stone circle teleport | Carrier teleports with hero |
| Rendering | Carriers skip terrain masking; swan on ground renders as RAFT sprite |

---


## 22. Audio System

### 22.1 VBlank-Driven Music Tracker

4-voice custom tracker driven by VBlank interrupt at 60 Hz (NTSC). Processes one tick per vertical blank.

#### Engine State

Global fields: `nosound` (mute flag), `tempo` (playback speed, default 150), `ins_handle` (instrument table), `vol_handle` (envelope data), `wav_handle` (waveform data), `timeclock` (cumulative timer — incremented by `tempo` each VBL even when muted).

Per-voice (4 × 28 bytes): `wave_num`, `vol_num`, `vol_delay`, `vce_stat`, `event_start/stop`, `vol_list`, `trak_ptr/beg/stk`.

#### vce_stat Bit Flags

| Value | Name | Meaning |
|-------|------|---------|
| 4 | TIE | Tied note (no articulation gap) |
| 8 | CHORD | Chorded note |
| 16 | REST | Voice is resting |
| 32 | ENDTK | Track has ended |

On voice 2, `vce_stat` doubles as a sample-completion countdown.

#### Voice Processing

For each voice per VBL tick:
1. If `vce_stat != 0`, skip voice (yields to sample playback on voice 2)
2. Process volume envelope: apply current byte as volume level; bit 7 set = hold current volume
3. If `timeclock >= event_stop`: silence voice
4. If `timeclock >= event_start`: process new event
5. Otherwise: continue current note/rest

### 22.2 Waveform & Envelope Data

Both loaded from `v6` file:

- **Waveforms** (`wavmem`): 1024 bytes — 8 waveforms × 128 bytes each (CHIP memory). Higher octaves offset into waveform to shorten effective sample, raising pitch without resampling.
- **Envelopes** (`volmem`): 2560 bytes — 10 envelopes × 256 bytes each. Each byte is a volume level per VBL tick; bit 7 set = hold current volume.

### 22.3 Instrument Table

12-entry word array (`new_wave`) maps instrument numbers to waveform/envelope pairs. Stored in `ins_handle`, passed to `_init_music` at startup. `setmood()` modifies entry 10 at runtime for underworld region.

### 22.4 Music Data Format

Track data is a byte-pair stream:

| Command byte | Value byte | Action |
|-------------|-----------|--------|
| 0–127 | Note # + TIE/CHORD bits | Play note with duration from `notevals` table |
| 128 ($80) | Duration code | Rest (silence) for duration |
| 129 ($81) | Instrument # (0–15) | Change instrument |
| $90 (144) | New tempo value | Change playback speed |
| 255 ($FF) | 0=stop, non-zero=loop | End of track |

Note durations from `notevals` (8×8 SMUS-standard timing table). Articulation gap: 300 counts subtracted.

### 22.5 Period Table

84 entries (7 octaves × 12 notes). Hz = 3,579,545 / period (NTSC). Higher octaves use shorter waveform segments.

### 22.6 Mood-Based Track Selection

`setmood(now)` selects music based on game state:

| Priority | Condition | Song | Track offset |
|----------|-----------|------|-------------|
| 1 | Hero dead (`vitality == 0`) | Death | 24 |
| 2 | Specific map zone | Zone theme | 16 |
| 3 | In combat (`battleflag`) | Battle | 4 |
| 4 | Underground (`region_num > 7`) | Dungeon | 20 |
| 5 | Daytime (`lightlevel > 120`) | Day | 0 |
| 6 | Nighttime | Night | 8 |
| — | Startup | Intro | 12 |

Each song is 4 tracks (one per voice). `playscore()` resets playback immediately; `setscore()` defers change until current tracks loop.

### 22.7 Sound Effects

6 samples loaded from disk sectors 920–930 into `sample_mem`. Played on channel 2 via `_playsample()`, which overrides that channel's music voice.

Sample completion: `vce_stat` on voice 2 set to 2. Audio interrupt handler (`audio_int`) decrements per interrupt. When counter reaches 0: silence channel (volume → 0, period → 2), music resumes on voice 2.

`effect(num, speed)` C wrapper checks Sound menu toggle before calling `playsample()`.

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

1. Legal text display (title text on dark blue background) via `ssp(titletext)`
2. 1-second pause
3. Load audio: music + samples from `v6` and `songs` files
4. Start intro music (tracks 12–15)
5. Load title image (`page0`), blit to both display pages
6. Vertical zoom-in: 0 → 160 in steps of 4 via `screen_size()`
7. Three story pages with columnar-reveal animation (`copypage` with `flipscan`)
8. Final pause (3.8 seconds)
9. Vertical zoom-out: 156 → 0 in steps of −4
10. Copy protection challenge

Player can skip at multiple checkpoints.

### 23.2 Copy Protection

#### Riddle System (`copy_protect_junk`)

Presents 3 random fill-in-the-blank questions from 8 question/answer pairs. Comparison is case-sensitive (uppercase required), prefix-only — the loop walks the correct answer until NUL terminator but does not verify length. After each correct answer, entry nulled to prevent repeats within session. Failure triggers `goto quit_all`.

First question effectively deterministic: RNG seed starts at 19837325, no `rand()` consumption before reaching `copy_protect_junk()`, first `rand8()` resolves to index 1 → "Make haste, but take...?"

| Index | Question | Answer |
|-------|----------|--------|
| 0 | "To Quest for the...?" | LIGHT |
| 1 | "Make haste, but take...?" | HEED |
| 2 | "Scorn murderous...?" | DEED |
| 3 | "Summon the...?" | SIGHT |
| 4 | "Wing forth in...?" | FLIGHT |
| 5 | "Hold fast to your...?" | CREED |
| 6 | "Defy Ye that...?" | BLIGHT |
| 7 | "In black darker than...?" | NIGHT |

#### Disk Timestamp Check (`cpytest`)

Validates magic value 230:
- Floppy: navigates `FileLock → fl_Volume → DeviceList`, checks `dl_VolumeDate.ds_Tick`. Failure: `cold()` → `jmp -4` (guru meditation crash).
- Hard drive: reads block 880, checks `buffer[123]`. Failure: `close_all()` (graceful shutdown).

`NO_PROTECT` compile flag disables riddle comparison and floppy timestamp check. Hard drive block-880 check always executes.

### 23.3 Event Messages

39 event messages (indices 0–38) via `event(n)` function, which indexes `_event_msg` table and calls `extract()`. The `%` character substitutes the current brother's name via `datanames[brother-1]`.

### 23.4 Place Names

`find_place()` called from Phase 14g. Determines `hero_sector`, selects message table:
- Outdoor (`region_num < 8`): `_place_tbl` / `_place_msg` — 29 entries
- Indoor (`region_num > 7`): `_inside_tbl` / `_inside_msg` — 31 entries

Each table entry: 3 bytes `{min_sector, max_sector, message_index}`. Linear scan, first match wins. Mountain messages (index 4) vary by region.

### 23.5 Text System

#### Fonts

- **Topaz 8** (`tfont`): ROM font via `OpenFont()`. Used for status bar labels, menu text, map-mode text.
- **Amber 9** (`afont`): Custom disk font from `fonts/Amber/9` via `LoadSeg`. Used for scrolling messages and placard text. Applied with pen 10 foreground, pen 11 background.

#### `ssp` — Scrolling String Print

Embedded positioning via escape code `XY` (byte 128/$80). Format: printable ASCII segments interspersed with `{XY, x_half, y}` triples. X coordinate stored at half value, doubled during rendering.

Algorithm: read byte → if 0: exit; if 128: read (x/2, y), Move(rp, x×2, y); else: scan printable bytes, Text(rp, buffer, count); loop.

Line width: max 36 chars for scroll text, 29 for placard text.

#### `placard` — Decorative Border

Fractal line pattern on `rp_map`: offset tables `xmod`/`ymod` (±4 pixel deltas), mirror-symmetric with center at (284,124) and two 90°/270° rotations, 16×15 outer iterations with 5 inner passes, color 1 for most lines, color 24 for first inner pass.

#### `print` / `print_cont`

- `print(str)`: Scroll `rp_text` up 10px via `ScrollRaster(rp, 0, 10, TXMIN, TYMIN, TXMAX, TYMAX)`, render at (TXMIN, 42). Bounds: TXMIN=16, TYMIN=5, TXMAX=400, TYMAX=44.
- `print_cont(str)`: Append on same line, no scroll.
- Both use global `rp` (set to `rp_text` during gameplay). Text colors: pen 10 fg, pen 11 bg, JAM2 mode.

#### `extract` — Template Engine

Word-wrap at 37 chars using `mesbuf[200]` buffer. `%` substitutes `datanames[brother-1]`. CR (13) forces line break.

#### `prdec` — Decimal Number Printing

Converts number to ASCII digits in `numbuf[11]`, divides by 10 repeatedly, space-fills leading positions.

#### Print Queue (`prq` / `ppick`)

32-entry circular buffer (`print_que[32]`, `prec`/`pplay` indices). `prq(n)` enqueues (drops silently if full). `ppick()` dequeues one per call from Phase 14a:

| Code | Action |
|------|--------|
| 2 | Debug: coords + available memory |
| 3 | Debug: position, sector, extent |
| 4 | Vitality at (245,52) |
| 5 | Refresh menu via `print_options()` |
| 7 | Full stats: Brv(14), Lck(90), Knd(168), Wlth(321) at y=52 |
| 10 | "Take What?" |

Empty queue: `Delay(1)` yields to OS.

### 23.6 Message Dispatch

Three functions indexing into null-terminated string tables and calling `extract()`:
- `event(n)` — `_event_msg` table: hunger, drowning, journey start, etc.
- `speak(n)` — `_speeches` table: NPC dialogue by speech number.
- `msg(table, n)` — generic: explicit table + index.

Common handler `msg1`: skips `n` null-terminated strings to find target, then calls `extract()`.

### 23.7 Placard Text Messages

20 story messages via `placard_text(n)`:

| Index | Message |
|-------|---------|
| 0 | Julian's quest intro |
| 1 | Julian's failure |
| 2 | Phillip sets out |
| 3 | Phillip's failure |
| 4 | Kevin sets out |
| 5 | Game over ("Stay at Home!") |
| 6–7 | Victory / Talisman recovered |
| 8–10 | Princess Katra rescue |
| 11–13 | Princess Karla rescue |
| 14–16 | Princess Kandy rescue |
| 17–18 | After seeing princess home |
| 19 | Copy protection intro |

### 23.8 Location Messages

`map_message()`: switch to fullscreen text overlay — fade down, clear playfield, hide status bar (VP_HIDE), set `rp = &rp_map`, `viewstatus = 2`.

`message_off()`: return to gameplay — fade down, restore `rp = &rp_text`, show status bar, `viewstatus = 3`.

---


## 24. Save/Load System

### 24.1 File Format

Raw sequential binary dump with no headers, no version field, no checksums. 8 slots named `A.faery` through `H.faery`. Written in native big-endian (68000) byte order via AmigaDOS `Write()`/`Read()`.

### 24.2 Save Data Layout

| Order | Data | Size |
|-------|------|------|
| 1 | Misc variables (map_x through pad7) | 80 bytes |
| 2 | `region_num` | 2 bytes |
| 3 | `anix`, `anix2`, `mdex` | 6 bytes |
| 4 | Active actor list (`anim_list[0..anix-1]`) | anix × 22 bytes |
| 5 | Julian's inventory (`julstuff`) | 35 bytes |
| 6 | Phillip's inventory (`philstuff`) | 35 bytes |
| 7 | Kevin's inventory (`kevstuff`) | 35 bytes |
| 8 | Missile list | 6 × 10 bytes (60) |
| 9 | Extent entries 0–1 (bird/turtle positions) | 24 bytes |
| 10 | Global object list (`ob_listg`, 11 × 6) | 66 bytes |
| 11 | Per-region object counts (`mapobs`) | 20 bytes |
| 12 | Per-region distributed flags (`dstobs`) | 20 bytes |
| 13 | All regional object tables | Σ mapobs[i]×6 |

Typical file size: ~1,200–1,500 bytes.

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

`cheat1` persists at byte offset 18 — only way to enable is hex-editing a save file.

### 24.4 Disk Detection

`savegame()` probes writable media in priority order: hard drive (`Lock("image")`) → df1: → df0: (if not game disk, verified by absence of `winpic`). Falls back to prompting for disk insertion with 30-second timeout.

### 24.5 Post-Load Cleanup

Reset on load: `encounter_number`, `wt`, `actors_loading`, `encounter_type` all cleared to 0. `viewstatus` set to 99 (force full redraw). `shape_read()` reloads all sprite data. `set_options()` rebuilds menu states from inventory.

### 24.6 Persistence Rules

**Persisted**: Hero position, stats (brave/luck/kind/wealth/hunger/fatigue), all 3 brothers' inventories (35 items each), daynight cycle, active actors, missiles, world objects (global + all 10 regions), bird/turtle extent positions, carrier state, cheat flag.

**Reset on load**: encounter_number, wt, actors_loading, encounter_type, viewstatus.

**Not saved**: Display state (copper lists, rendering buffers), input handler state, music playback position, extent entries 2–21 (static initializers), battleflag, goodfairy, etc.

---


## 25. UI & Menu System

### 25.1 Menu Structure

```c
struct menu {
    char    *label_list;
    char    num, color;
    char    enabled[12];
} menus[10];
```

`enabled[i]` encoding:
- Bit 0 (`& 1`): highlight/selected toggle
- Bit 1 (`& 2`): visibility — displayed only if set
- Bits 2–7 (`& 0xfc`): action type (`atype`)

| atype | Behavior |
|-------|----------|
| 0 | Top-bar navigation: switch `cmode` to `hit` |
| 4 | Toggle: XOR bit 0, call `do_option(hit)` |
| 8 | Immediate action: highlight, `do_option(hit)` |
| 12 | One-shot highlight: set bit 0, `do_option(hit)` |

### 25.2 Menu Modes

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

Modes ITEMS through GAME share a top bar (entries 0–4 from `label1`). For `cmode >= USE`, labels draw directly from `menus[cmode].label_list`.

### 25.3 Menu Rendering

`print_options()` renders on `rp_text2` (hi-res status bitmap):
- Iterates `menus[cmode]` entries; for each visible (`enabled[i] & 2`), assigns `real_options[j] = i`
- Layout: 2 columns (x=430, x=482), 6 rows at 9px spacing, starting at y=8
- Each label is 5 characters

Background pen by mode:
- USE: pen 14. FILE: pen 13. Top bar (k<5): pen 4.
- KEYS: `keycolors[k-5]` where `keycolors = {8, 6, 4, 2, 14, 1}`
- SAVEX: pen = entry index. Others: `menus[cmode].color`

`gomenu(mode)`: sets `cmode`, resets `handler_data.lastmenu`, calls `print_options()`. Blocked if paused.

### 25.4 Dynamic Availability (`set_options`)

Called at the end of every `do_option()`. Updates `enabled[]` based on inventory:

| Menu | Indices | Source | Logic |
|------|---------|--------|-------|
| MAGIC | 5–11 | `stuff[9..15]` | `stuff_flag(i+9)` |
| USE | 0–6 | `stuff[0..6]` | `stuff_flag(i)` |
| KEYS | 5–10 | `stuff[16..21]` | `stuff_flag(i+16)` |
| USE key | 7 | any key owned | 10 if yes, 8 if no |
| USE sun | 8 | `stuff[7]` | `stuff_flag(7)` |
| GIVE gold | 5 | wealth > 2 | 10 if yes, 8 if no |
| GIVE book | 5 | — | Always 8 (hidden) |
| GIVE writ | 8 | `stuff[28]` | `stuff_flag(28)` |
| GIVE bone | 9 | `stuff[29]` | `stuff_flag(29)` |

`stuff_flag(n)`: returns 8 if `stuff[n] == 0` (hidden), 10 if owned (visible).

### 25.5 `do_option` Dispatch

#### ITEMS Mode

| hit | Label | Action |
|-----|-------|--------|
| 5 | List | Full inventory screen. Iterate `stuff[0..GOLDBASE-1]`, draw item icons. `viewstatus=4` |
| 6 | Take | `nearest_fig(0,30)`: gold→+50 wealth; food→eat; bones→recover inventory; containers→random loot via `rand4()`; other→`itrans[]` lookup. Dead bodies→extract weapon+treasure. Win check: `stuff[22]` (Talisman) set → `quitflag = TRUE` |
| 7 | Look | Scan OBJECTS within range 40. Found→`event(38)`, else `event(20)` |
| 8 | Use | `gomenu(USE)` |
| 9 | Give | `gomenu(GIVE)` |

#### MAGIC Mode

Guard: `stuff[4+hit] == 0` → `event(21)`. Blocked in necromancer extent: `speak(59)`.

| hit | Spell | Effect |
|-----|-------|--------|
| 5 | Stone | Teleport via standing stones; requires `hero_sector==144`, uses `stone_list[]` |
| 6 | Jewel | `light_timer += 760` |
| 7 | Vial | `vitality += rand8()+4`, capped at `15+brave/4` |
| 8 | Orb | `secret_timer += 360` |
| 9 | Totem | Map view with hero marker, `viewstatus=1`. Blocked underground unless `cheat1` |
| 10 | Ring | `freeze_timer += 100`. Blocked if `riding > 1` |
| 11 | Skull | Kill all visible enemies with `race < 7`. Decrement `brave` |

After use: `--stuff[4+hit]`; if depleted, `set_options()`.

#### TALK Mode

| hit | Label | Range |
|-----|-------|-------|
| 5 | Yell | `nearest_fig(1, 100)` |
| 6 | Say | `nearest_fig(1, 50)` |
| 7 | Ask | `nearest_fig(1, 50)` |

NPC response dispatch on `race & 0x7f`: Wizard(0)→`speak(35/27+goal)`, Priest(1)→checks writ/heals, Guard(2,3)→`speak(15)`, Princess(4)→`speak(16)`, King(5)→`speak(17)`, Noble(6)→`speak(20)`, Sorceress(7)→luck boost or `speak(45)`, Innkeeper(8)→`speak(13/12/14)` based on fatigue/time, Witch(9)→`speak(46)`, Spectre(10)→`speak(47)`, Ghost(11)→`speak(49)`, Ranger(12)→`speak(22/53+goal)`, Beggar(13)→`speak(23)`.

#### BUY Mode

Requires shopkeeper (`race 0x88`). Uses `jtrans[]` pairs (item_index, cost):

| Menu | Item | Cost |
|------|------|------|
| Food | eat(50) | 3 |
| Arrow | `stuff[8] += 10` | 10 |
| Vial | `stuff[11]++` | 15 |
| Mace | `stuff[1]++` | 30 |
| Sword | `stuff[2]++` | 45 |
| Bow | `stuff[3]++` | 75 |
| Totem | `stuff[13]++` | 20 |

#### GAME Mode

| hit | Label | Action |
|-----|-------|--------|
| 5 | Pause | Toggle via atype=4 bit flip; gates `notpause` |
| 6 | Music | `setmood(TRUE)` |
| 7 | Sound | Toggle bit only |
| 8 | Quit | `gomenu(SAVEX)` |
| 9 | Load | `svflag = FALSE; gomenu(FILE)` |

#### SAVEX Mode

| hit | Action |
|-----|--------|
| 5 | Save: `svflag = TRUE; gomenu(FILE)` |
| 6 | Exit: `quitflag = TRUE` |

#### USE Mode

| hit | Action |
|-----|--------|
| 0–4 | Equip weapon: `weapon = hit+1` (Dirk/Mace/Sword/Bow/Wand) |
| 6 | Shell/Turtle: `get_turtle()` |
| 7 | Key: `gomenu(KEYS)` if any key owned |
| 8 | Sunstone: if `witchflag`, `speak(60)` |

Returns to ITEMS: `gomenu(ITEMS)`.

#### KEYS Mode

Convert `hit -= 5` to key index 0–5. If `stuff[hit+KEYBASE] > 0`: scan 9 directions around hero via `doorfind(x, y, hit+1)`. If door found, decrement key. Return to ITEMS.

#### GIVE Mode

Requires `nearest_person != 0`.

| hit | Action |
|-----|--------|
| 5 | Gold: wealth − 2, random `kind++`. Beggar→`speak(24+goal)`, else `speak(50)` |
| 8 | Bone: if spectre (0x8a)→`speak(48)`, drops crystal shard (item 140) |

#### FILE Mode

`savegame(hit)` — slot 0–7 (A–H). `svflag` determines save vs. load. Return to GAME.

### 25.6 Keyboard Shortcuts

38 entries via `letter_list[38]`:

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
| F1–F7 | Magic spells | | |

SAVEX guard: V and X blocked unless `cmode == SAVEX`. KEYS special: if `cmode == KEYS` and key '1'–'6', dispatch `do_option(key - '1' + 5)` directly.

### 25.7 Compass Display

Two 48×24 pixel bitmaps as raw bitplane data:
- `_hinor`: base compass (all directions normal)
- `_hivar`: highlighted direction segments

Rendered on `bm_text` at (567, 15). Only bitplane 2 differs. Direction regions: `comptable[10]` — 8 cardinal/ordinal rectangles at indices 0–7 plus **2 null entries at indices 8 and 9** used to render "no highlight".

**Highlight source (behavioral requirement).** The highlighted wedge is driven by the **resolved input direction this tick** (keyboard / joystick / mouse-click), not by the player's persistent `facing` value. The resolved direction uses the same `com2[9]` table as movement (§9.1) and takes one of the values 0–7 (NW, N, NE, E, SE, S, SW, W) or **9 (no input this tick)**. When the value is 9, the comptable lookup hits a null region and only `_hinor` is drawn — i.e. **no wedge is highlighted while the player is idle**. `drawcompass()` is invoked only when the resolved direction changes (`oldir` → new), so the base bitmap is not redundantly re-blitted every frame.

Player `facing` is updated from the resolved direction only when the direction is 0–7; when the direction is 9, `facing` retains its last value so the sprite still faces the last-walked direction, but the compass highlight clears.

### 25.8 Stats & HUD

Status bar: `vp_text` (640×57 hi-res). Color palette from `textcolors[20]` — NOT affected by day/night fading.

Stats via print queue:
- `prq(7)`: Full stat line at y=52 — Brv x=14, Lck x=90, Knd x=168, Wlth x=321
- `prq(4)`: Vitality at (245, 52)

Menu display: `print_options()` on `rp_text2`. Two columns, 6 rows, each label 5 chars.

### 25.9 `cheat1` Debug Mode

Persisted in save file (byte offset 18 of 80-byte block). Only enabled via hex-editing. Gates debug keys:

| Key | Effect |
|-----|--------|
| B | Summon Swan; if already active carrier, also grant Golden Lasso (`stuff[5]=1`) |
| . | Add 3 to random `stuff[]` entry (range 0–30) |
| R | Call `rescue()` |
| = | Call `prq(2)` |
| Key 19 | Call `prq(3)` |
| Key 18 | Advance `daynight` by 1000 |
| Keys 1–4 | Teleport hero ±150 Y / ±280 X |

Also gates map spell region restriction: when `cheat1 == 0`, map returns early if `region_num > 7`.

---


## 26. Asset Formats & Data Loading

### 26.1 Disk Image (`image` file)

Single 901120-byte file (1760 sectors × 512 bytes). Key sector allocations:

| Sectors | Content | Size |
|---------|---------|------|
| 32–95 | Sector maps (outdoor regions) | — |
| 96–159 | Sector maps (indoor regions) | — |
| 149–159 | Terrain property tables | — |
| 160–199 | Region maps (5 ranges of 8 sectors each) | — |
| 200–879 | Image banks (40 sectors each, 17 distinct banks) | 20480 bytes/bank |
| 880 | Copy protection check sector | 512 bytes |
| 896–919 | Shadow masks | 12288 bytes (24 sectors) |
| 920–930 | Sound effect samples (6 samples) | 5632 bytes (11 sectors) |
| 931–955 | Setfig sprites (5 sets × 5 sectors) | — |
| 960–1171 | Enemy + carrier sprites | — |
| 1312–1501 | Object, raft, turtle, and player sprites | — |

### 26.2 File-Based Assets

| File | Content | Format |
|------|---------|--------|
| `v6` | Waveforms (1024 bytes: 8 × 128-byte waveforms) + envelopes (2560 bytes: 10 × 256-byte envelopes) | Raw binary |
| `songs` | 28 music tracks (7 songs × 4 channels) | Custom packed tracker |
| `fonts/Amber/9` | Proportional bitmap font (Amber 9pt) — used for in-game scrolling messages and placard text | Amiga hunk format |
| `hiscreen` | HUD/status bar graphics | IFF/ILBM |
| `page0` | Title screen | IFF/ILBM |
| `p1a`–`p3b` | Story page images (3 pairs) | IFF/ILBM |
| `winpic` | Victory image | IFF/ILBM |

The system ROM font **Topaz 8** is used for status bar labels, menu text, and map-mode text.

### 26.3 IFF/ILBM Format

`unpackbrush()` loads IFF ILBM images with the following chunk handling:

| Chunk | Handling |
|-------|---------|
| FORM | Validate as IFF container |
| ILBM | Subtype marker (no-op) |
| BMHD | Read bitmap header (dimensions, compression mode) |
| CMAP | **Skipped** — game uses hardcoded programmatic palettes, not embedded palette data |
| GRAB | **Skipped** |
| CAMG, CRNG | **Skipped** |
| BODY | Decompress into target bitmap |

**ByteRun1 decompression**:
- Control byte 0–127: Copy next (N+1) bytes literally
- Control byte −1 to −127: Repeat next byte (1−N) times
- Control byte −128: No-op (not handled by the original assembly routine; the C fallback does handle it; in practice the game's compressor never emits −128)

The `compress` global selects between raw copy (0) and ByteRun1 decompression. Data is bulk-read into `shape_mem` as a temporary buffer, then decompressed scanline-by-scanline into destination bitplanes.

### 26.4 Sprite Format

5 bitplanes of image data per frame, loaded contiguously into `shape_mem` (78000 bytes). A 1-bit mask plane is generated at load time by ORing all 5 image planes and inverting. Dimensions from `cfiles[]` table (width in 16px units, height in pixels).

Each sprite set stores image data followed by mask data:
- Image data for frame `inum`: `seq_list[type].location + (planesize × 5 × inum)`
- Mask data for frame `inum`: `seq_list[type].maskloc + (planesize × inum)`

### 26.5 Tileset Format

5-bitplane Amiga planar bitmap. 4 banks per region, 64 tiles per bank = 256 tiles total. Each tile: 16×32 pixels = 64 bytes per bitplane. Each bank: 40 disk sectors = 20480 bytes (5 planes × 4096 bytes/plane). Total: 81920 bytes (`IMAGE_SZ`).

### 26.6 Memory Buffer Sizes

| Buffer | Size (bytes) | Purpose |
|--------|-------------|---------|
| `image_mem` | 81920 | Tile image data (256 tiles × 5 planes) |
| `sector_mem` | 36864 | Sector map (32 KB) + region map (4 KB) |
| `terra_mem` | 1024 | Terrain attribute tables (2 × 512 bytes) |
| `shape_mem` | 78000 | Sprite sheet data (all character sprites); also used as temp IFF decompression buffer |
| `shadow_mem` | 12288 | Terrain occlusion masks |
| `sample_mem` | 5632 | Audio sample data (6 samples) |
| `wavmem` | 1024 | Waveform data (8 × 128 bytes) |
| `scoremem` | 5900 | Music score data (7 songs × 4 tracks) |
| `volmem` | 2560 | Volume envelope data (10 × 256 bytes) |

### 26.7 Palette Loading

All game palettes are managed programmatically — CMAP chunks in IFF files are always skipped:
- Playfield: 32 colors loaded from `pagecolors[]`, modulated by `fade_page()` for day/night
- Text bar: 20 colors loaded from `textcolors[]`
- Intro: `introcolors[]` used during `screen_size()` zoom animation

---


## 27. Special Effects

### 27.1 Witch Vision Cone

Rotating filled wedge-shaped polygon drawn in **COMPLEMENT (XOR) mode** around the witch position:

1. **Geometry**: Two edges from `witchpoints[256]`, a 64-entry sine/cosine table encoding points on two concentric circles (radii ~10 and ~100 pixels). The wedge spans ~11.25° of arc.
2. **Rotation**: `witchindex` (u8, 0–255) advances each frame by `wdir` (±1), wrapping the beam in a full circle over 256 frames.
3. **Steering**: `wdir` adjusts based on the cross-product sign of the hero's position relative to the beam, gated by `rand4() == 0` (1-in-4 frames). When the hero is clockwise of the beam, `wdir = 1`; otherwise `wdir = -1`. This creates a slow seeking rotation toward the hero.
4. **Hit test**: Cross-product determines if the hero falls within the beam wedge (both cross-products have correct signs) AND distance < 100 pixels. On hit: `dohit(-1, 0, facing, rand2()+1)` deals 1–2 HP damage. The `-1` actor argument triggers a magical-hit sound effect.
5. **Rendering**: Creates a temporary clipping layer (304×192), draws the filled quadrilateral in XOR mode against the bitmap. The same XOR draw call erases the previous frame's beam.

### 27.2 Teleport Colorplay

`colorplay()`: 32 frames of randomized 12-bit RGB values for palette entries 1–31. Entry 0 (background color) is preserved. Duration ≈ 0.5 seconds (32 frames at ~60 Hz). Creates a psychedelic flash effect during teleportation.

### 27.3 Columnar Page Reveal (`flipscan`)

22-step animation for story page transitions:
1. Steps 0–10 (right half): Blit vertical strips from new page at decreasing intervals, progressively revealing
2. Steps 11–21 (left half): Replace old strips with new, progressively covering
3. Per-step timing from `flip3[]` delay table
4. Each step calls page swap for intermediate display

Used for intro story sequences: loads two IFF brushes into `pageb`, then `flipscan()` transition.

### 27.4 Victory Sunrise Fade (`win_colors`)

55-step palette animation after the victory placard:

1. Display victory text via `placard_text(6)`, hero name, `placard_text(7)`, `placard()` border
2. Load `winpic` into drawing bitmap, set both viewports to black
3. Hide text viewport, set `screen_size(156)` for full playfield
4. Animate 55 frames (index `i` from 25 down to −29):
   - Colors 0/31: always black
   - Colors 1/28: always white
   - Colors 2–27: fade in from `sun_colors[53]` gradient (black → deep blue → warm gold)
   - Colors 29–30: red tones — initially `0x800`/`0x400`, later computed as `0x100 × (i+30)/2` and half that
5. First frame holds 60 ticks (~1 second), subsequent frames 9 ticks (~150 ms) each
6. Final hold of 30 ticks, then fade to full black

### 27.5 Screen Fade Transitions

- **`fade_down()`**: 21 steps from 100% to 0% in decrements of 5, with `Delay(1)` per step. Uses `limit = FALSE` (no night clamping or blue shift). Fades screen to black.
- **`fade_normal()`**: 21 steps from 0% to 100% in increments of 5, same timing. Fades back to full brightness.

Both are used for map messages, door transitions, story placards, and screen changes.

### 27.6 Viewport Zoom (`screen_size`)

Animates viewport dimensions from a point to full screen using 5:8 aspect ratio: `y = x × 5 / 8`. Normal gameplay uses `screen_size(156)`, yielding a 312×194 viewport slightly inset from the 320×200 frame. The intro sequence reaches `screen_size(160)` for full-screen display.

### 27.7 Static Display Reset (`stillscreen`)

Resets scroll offsets to (0, 0) and flips page. Used for non-scrolling display modes (story pages, map messages).

### 27.8 Flasher Border Blink

During dialogue mode (`viewstatus == 1`), color register 31 blinks white↔black every 16 frames (~0.27 seconds). Controlled by `flasher & 16` where `flasher` increments every main-loop tick (bit 4 toggles at a steady cadence). Any pixels using color 31 produce a blinking cursor/prompt effect.

### 27.9 Full-Screen Message Transitions

**`map_message()`**: Enters full-screen text mode:
1. `fade_down()` — fade to black
2. Clear bitmap via `SetRast`
3. Set text drawing mode: pen 24, `JAM1` mode
4. Hide text viewport (`VP_HIDE`)
5. `stillscreen()` — reset scroll
6. Load page colors
7. Set `viewstatus = 2`

**`message_off()`**: Returns to gameplay:
1. `fade_down()` — fade to black
2. Restore text viewport (`HIRES | SPRITES`)
3. `pagechange()` — flip page
4. Set `viewstatus = 3` — main loop detects this after next `pagechange()` and fires `fade_normal()` to brighten back in

### 27.10 Placard Border

`placard()`: A recursive fractal line pattern drawn on the playfield using `xmod`/`ymod` offset tables (±4 pixel deltas). The pattern is mirror-symmetric: draws lines at the original position, center-mirrored at (284, 124), and two 90°/270° rotations. Uses 16×15 outer iterations with 5 inner passes. Color 1 for most lines, color 24 for the first inner pass. Called during story sequences after `placard_text()`.

---


## Appendices

### A. Constants Reference

| Constant | Value | Description |
|----------|-------|-------------|
| MAXCOORD | 0x7FFF (32767) | Maximum X coordinate |
| MAXSHAPES | 25 | Maximum sprites per rendering frame (per page) |
| PAGE_DEPTH | 5 | Bitplanes per game page (32 colors) |
| TEXT_DEPTH | 4 | Bitplanes per status bar (16 colors) |
| SCREEN_WIDTH | 288 | Visible playfield width (pixels) |
| PAGE_HEIGHT | 143 | Scanline where text viewport begins |
| TEXT_HEIGHT | 57 | HUD/status bar height (pixels) |
| PHANTA_WIDTH | 320 | Full bitmap width including scroll margins |
| RAST_HEIGHT | 200 | Full raster height per page |
| TILE_WIDTH | 16 | Tile width in pixels |
| TILE_HEIGHT | 32 | Tile height in pixels |
| TILES_X | 19 | Visible tile columns |
| TILES_Y | 6 | Visible tile rows |
| ACTOR_SLOTS | 20 | Maximum actor array size (`anim_list[20]`) |
| MAX_MISSILES | 6 | Maximum simultaneous missiles |
| MAX_OBJECTS_PER_SECTOR | 250 | World objects per sector |
| SHAPE_BYTES | 22 | Size of `struct shape` in bytes |
| MAGICBASE | 9 | First magic item inventory index |
| KEYBASE | 16 | First key inventory index |
| STATBASE | 25 | Gold statues inventory index |
| GOLDBASE | 31 | Gold inventory index |
| ARROWBASE | 35 | Arrow inventory index |
| EXT_COUNT | 22 | Extent entries scanned |
| DOORCOUNT | 86 | Total door entries |
| VOICE_SZ | 3584 | Audio waveform + envelope buffer size |
| SAMPLE_SZ | 5632 | Sound effect sample buffer size (6 samples) |
| IMAGE_SZ | 81920 | Full tileset image buffer size (256 tiles × 5 planes) |
| SHADOW_SZ | 12288 | Shadow/terrain occlusion mask buffer size |
| SECTOR_SZ | 36864 | Sector (32 KB) + region map (4 KB) buffer size |
| SHAPE_MEM_SZ | 78000 | Sprite sheet data buffer size |
| TERRA_MEM_SZ | 1024 | Terrain property table buffer size (2 × 512) |
| WAV_MEM_SZ | 1024 | Waveform data buffer size (8 × 128) |
| VOL_MEM_SZ | 2560 | Volume envelope buffer size (10 × 256) |
| SCORE_MEM_SZ | 5900 | Music score data buffer size |
| BACKSAVE_LIMIT | 5920 | Background save buffer capacity per page |
| DAYNIGHT_MAX | 24000 | Day/night counter wrap point (0–23999) |
| DAYNIGHT_INIT | 8000 | Initial daynight value (morning) on revive |
| LIGHTLEVEL_MAX | 300 | Peak lightlevel at noon |