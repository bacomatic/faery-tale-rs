## 1. Display & Rendering

### 1.1 Screen Layout

The game renders in three distinct display configurations at different scene boundaries:

| Config | Playfield size | Inset from 320×200 | HUD | Used for |
|--------|---------------|--------------------|-----|----------|
| **Gameplay** | 288×140 | 16 px horiz, 0 px vert (scanlines 0–139; HUD at 143+) | Visible (640×57 below) | Normal gameplay |
| **Cinematic** | 312×194 | 4 px horiz, 3 px vert | Hidden | Title text, asset loading, copy protection, victory sunrise |
| **Storybook** | 320×200 | 0 (edge-to-edge) | Hidden | Intro storybook pages (page0, p1–p3) |

Palette is `pagecolors` in Gameplay, `introcolors` in Cinematic/Storybook (with per-frame fade scaling during zoom — see §27.6), overridden to `sun_colors` during the victory sunrise animation (see §15.8).

**Presentation in 640×480 canvas** (Gameplay config):

| Area | Native | Presented | Notes |
|------|--------|-----------|-------|
| Playfield | 288×140 | 576×280 (2×) | Centered horizontally |
| Gap | 3 lines | 6 px | Blank separator |
| HUD bar | 640×57 | 640×114 (line-doubled) | Full-width |

The composed view is 400 px tall, vertically centered with 40 px top/bottom margins. Cinematic and Storybook configs scale their playfield proportionally into the same 400 px vertical slot with their own inset, status bar hidden.

Key constants:
- `PAGE_DEPTH` = 5 (playfield bitplanes — 32 colors)
- `TEXT_DEPTH` = 4 (HUD bitplanes — 16 colors)
- `SCREEN_WIDTH` = 288, `PAGE_HEIGHT` = 143, `TEXT_HEIGHT` = 57 (Gameplay config)
- `PHANTA_WIDTH` = 320 (playfield bitmap width; scroll margin varies by config)

The 16-pixel per-side scroll margin in Gameplay config allows smooth tile scrolling without exposing unrendered edges. Cinematic and Storybook configs reveal more of the 320-wide bitmap by reducing their inset.

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



## 3. Tile & Map System

### 3.1 Tileset Structure

Each region's tileset comprises 256 tiles, organized into 4 banks of 64 tiles each. Tiles are **16×32 pixels**, 32 colors (5 bitplanes). Each tile requires 64 bytes per bitplane (16 px ÷ 8 = 2 bytes/row × 32 rows).

Image banks are loaded from disk as 40 sectors each (20480 bytes = 5 planes × 4096 bytes/plane). Four banks compose a complete 256-tile image set totaling 81920 bytes (`IMAGE_SZ`).

256 tiles × 64 bytes/plane × 5 planes = 81920 bytes in `image_mem`.

### 3.2 Terrain Properties

Terrain data is loaded as two 512-byte halves (one per terrain table ID) into a 1024-byte buffer (`terra_mem`). Each tile has a terrain entry; the terrain entry format for each tile provides:
- Byte 0: mask shape index (index into shadow mask data for terrain occlusion)
- Byte 1 upper nibble (bits 4–7): terrain type (0–15, drives movement speed and water physics)
- Byte 1 lower nibble (bits 0–3): mask application rule (0–7, controls sprite occlusion behavior)

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

### 5.5 OBJECTS Sheet Half-Height Set & Bit-7 Flag

The OBJECTS sprite sheet is laid out as 16-scanline rows, but a fixed list of frame indices (`inum`) renders as **8-scanline strips packed two-per-row**. Both the size derivation (`compute_sprite_size`, fmain.c:2477–2479) and the source-data Y-offset (`compute_shape_clip`, fmain.c:2524) consult the same `inum`:

| `inum` | Effective height | Source Y offset |
|--------|------------------|-----------------|
| `0x1b` | 8 | 0 |
| `8..=12` | 8 | 0 |
| `25`, `26` | 8 | 0 |
| `0x11..=0x17` | 8 | 0 |
| `inum & 0x80 != 0` | 8 | +8 (lower-half row) |
| Anything else | 16 | 0 |

Bit 7 (`INUM_BIT7_HALF_HEIGHT = 0x80`) has a **dual role**: it forces 8-scanline height *and* shifts the source-data Y-offset by +8 inside the addressed frame, allowing two thin sprites to share one 16-row record. The flag must be stripped (`inum & !0x80`) before indexing the sheet.

### 5.6 Weapon Overlay (passmode = 1)

When the hero is rendered with a weapon equipped, a second pass blits a weapon sprite from the OBJECTS sheet on top of (or behind, depending on facing) the body sprite. The current body animation `frame` (0..86, the `statelist` index) selects an entry whose `wpn_no`, `wpn_x`, `wpn_y` describe the base weapon sprite and its body-relative pixel offset. Per-weapon-class **k offsets** add to `wpn_no`:

| Weapon | k | OBJECTS frame |
|--------|----|---------------|
| Bow (4) | 0 | `wpn_no + 0` *(see special-case below)* |
| Mace (2) | 32 | `wpn_no + 32` |
| Sword (3) | 48 | `wpn_no + 48` |
| Dirk (1) | 64 | `wpn_no + 64` |
| Wand (5) | — | `facing + 103` (DIR_NE shifts Y by −6) |

**Bow special-case** (fmain.c:2412–2425, fmain2.c:877–882). On walk-cycle frames (`frame < 32`) the bow uses two 32-entry per-frame offset tables instead of `wpn_x`/`wpn_y`:

```
bow_x[32] = [ 1, 2, 3, 4, 3, 2, 1, 0,    //  0..7   south-walk
              3, 2, 0,-2,-3,-2, 0, 2,    //  8..15  west-walk
             -3,-3,-3,-3,-3,-3,-3,-2,    // 16..23  north-walk
              0, 1, 1, 1, 0,-2,-3,-2 ]   // 24..31  east-walk
bow_y[32] = [ 8, 8, 8, 7, 8, 8, 8, 8,
             11,12,13,13,13,13,13,12,
              8, 7, 6, 5, 6, 7, 8, 9,
             12,12,12,12,12,12,11,12 ]
```

The bow's overlay `inum` is also derived directionally from the walk-cycle group `frame / 8`:

| Walk group | `inum` |
|------------|--------|
| 0 (south) | `0x53` |
| 1 (west) | `30` |
| 2 (north) | `0x51` |
| 3 (east) | `30` |

**Two-pass body/weapon ordering** (`resolve_pass_params`, fmain.c:2402,2405). The compositor draws each character in two passes; whether the weapon goes behind or in front of the body is determined by XOR-ing the pass index with a facing-derived bit. In the current Rust facing scheme (0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW) the weapon is drawn **behind** the body for facings `{0, 5, 6, 7}` and **in front** for the rest.

The arrow/fireball spawn offsets per facing (used at shot-release time) are:

```
bowshotx[8] = [ 0, 0, 3, 6,-3,-3,-3,-6 ]   // NW,N,NE,E,SE,S,SW,W
bowshoty[8] = [-6,-6,-1, 0, 6, 8, 0,-1 ]
```

### 5.7 Inventory Items-Page Render

The character/inventory items page (`render_inventory_items_page`) treats the OBJECTS sheet as a 5-plane bitmap and blits per-icon strips at fixed positions. For each carried stack `j`:

```
n = inv_list[j].image_number * 80 + inv_list[j].img_off
BltBitMap( OBJECTS, src=(0, n), dst=( xoff + 20, yoff ),
           size=(16, inv_list[j].img_height) )
```

Constants (matching fmain.c originals):

- `OBJ_FRAME_STRIDE = 80` (5 planes × 16 scanlines).
- `OBJ_PLANE_STRIDE = 32` (16 px × 2 bytes = 32, packed planar stride).
- `INV_ICON_X_OFFSET = 20`, `INV_ICON_WIDTH = 16`.
- Each `inv_list[]` entry carries `img_off` (per-icon Y-skip in scanlines) and `img_height` (per-icon scanline count).
- Stacks repeat down a single column spaced by `ydelta`, capped at `maxshown` rows; only items in rows `0..GOLDBASE` are eligible for stacking.

This path uses neither the half-height set nor bit 7 — it always reads `img_height` rows starting at `img_off` from a frame whose stride is `OBJ_FRAME_STRIDE`.

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

### 6.4 Mask Early-Exit Gates

`should_apply_terrain_mask` short-circuits the per-tile mask loop entirely for a fixed list of cases (fmain.c, mask gate ladder). Any *one* of these conditions skips terrain masking for the current sprite:

| # | Condition | Reason |
|---|-----------|--------|
| 1 | `atype == CARRIER` (turtle / bird) | Carriers are flying / swimming and never occluded by ground tiles. |
| 2 | Hero is on the swan boat | Riding a carrier inherits the same exemption. |
| 3 | Sprite is the active fiery-death rectangle | Full-screen wipe overlay is always on top. |
| 4 | `inum ∈ {100, 101}` (fairy/sparkle) | Tiny FX sprites render flat. |
| 5 | NPC race ∈ `{0x85, 0x87}` | Race-specific render exemption. |

When any gate fires the compositor still runs `save_blit` / `shape_blit`, but stages 2–3 (`maskit` / `mask_blit`) are skipped — the sprite renders as a flat cookie-cut without terrain occlusion.

### 6.5 Vestigial `blithigh = 32` Override

Small objects and weapon overlays use a clipped bounding box where `blithigh` (the per-mask scan height) is forced to 32 even for 8- or 16-scanline sprites (`compute_terrain_mask`, fmain.c). This is a vestige of an earlier 32-tall sprite layout; the override is preserved verbatim for fidelity. Notes:

- The sinking-ramp Y-shift (`an.environ > 2`, "sinking" actors) clips the *body* to ground but the weapon-overlay mask still uses the unshifted ground line — the original two-pass mask shares the body's `ground` value across both passes.
- The drowning-bubble override (frames 97/98) draws *without* applying any mask, even when no other early-exit gate fires.

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

An animated iris-zoom effect: the playfield grows/shrinks from a center point, with a synchronized palette fade on `introcolors`. Used for intro and victory.

**Step** (one frame, argument `x` in range 0–160):

1. Set playfield aperture to `(x*2) × (y*2)` where `y = (x*5)/8`, centered in the 320×200 frame.
2. Inversely shrink the HUD: at `x ≥ 152` the HUD is fully hidden; it reappears as `x` shrinks below 152.
3. Fade the `introcolors` palette by per-channel percentage:
   - **R%** = `y*2 − 40`
   - **G%** = `y*2 − 70`
   - **B%** = `y*2 − 100`
   
   Negative percentages clamp to 0 (black). Red brightens first, then green, then blue — a warm sunrise-like fade-in as the viewport opens.

**Key x values:**

| x | Aperture | Config |
|---|----------|--------|
| 0 | closed (single point) | — |
| 152 | 304×190 | HUD just hidden |
| 156 | 312×194 | Cinematic (title text, copy protection, victory sunrise) |
| 160 | 320×200 | Storybook (intro pages) |

**Animations:**

- **Zoom-in**: `x = 0..160` step +4 (41 steps). Aperture opens from center with synchronized red→green→blue palette fade-in.
- **Zoom-out**: `x = 156..0` step −4 (40 steps) with reverse palette fade-out. Starts at 156 rather than 160 purely to skip a redundant no-op first frame (160→160 would not change the display).

**Port requirements:**

- All three effects (aperture resize, HUD shrink, palette fade) MUST be synchronized frame-for-frame.
- `screen_size(156)` alone (not in a zoom loop) switches to the Cinematic config and applies one frame of the `introcolors` fade at full value; the caller may then override the palette (e.g. victory sequence loads `sun_colors`).

See [§1.1](#11-screen-layout) for the three display configurations.

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


