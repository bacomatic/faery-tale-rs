## 1. Display & Rendering

### Requirements

| ID | Requirement |
|----|-------------|
| R-DISP-001 | The game shall render its playfield in three configurations: (a) **Gameplay** — 288×140 with the HUD bar (640×57) below, 16 px horizontal inset; (b) **Cinematic** — 312×194 with HUD hidden, 4 px horizontal / 3 px vertical inset from the 320×200 frame, used for title text, asset loading, copy protection, and the victory sunrise; (c) **Storybook** — 320×200 edge-to-edge with HUD hidden, used for the three intro storybook pages at peak zoom. Transitions between configs match the original's scene boundaries (see R-INTRO-001, R-QUEST-007, R-FX-007). |
| R-DISP-002 | A 640×57-pixel HUD bar (hi-res, 4 bitplanes, 16 colors, single-buffered) shall be rendered below the playfield. |
| R-DISP-003 | The rendering pipeline shall use double-buffered page flipping: one drawing page and one viewing page, swapped each frame. |
| R-DISP-004 | Each page shall maintain its own scroll position (`isv_x`, `isv_y`), sprite count (`obcount`), background save queue (up to `MAXSHAPES` = 25 sprites), background save buffer (up to 5920 bytes), and witch FX position state. |
| R-DISP-005 | Sprite backgrounds shall be restored in reverse compositing order before the next frame's tile rendering. |
| R-DISP-006 | The target frame rate shall be 30 fps (NTSC timing). Each main loop iteration equals one frame, synchronized to vertical blank. |
| R-DISP-007 | The full raster shall be 320×200 pixels (`PHANTA_WIDTH` × `RAST_HEIGHT`), with the visible playfield area inset by a 16-pixel scroll margin on each side. |
| R-DISP-008 | Color 31 (all 5 bitplanes set = `11111`) shall be the transparency color for sprite compositing. |
| R-DISP-009 | A hardware sprite (sprite 0) shall serve as the mouse pointer cursor, rendered in the status bar viewport. |
| R-DISP-010 | Page swap shall set scroll offsets, rebuild the copper/display list, and wait for vertical blank before proceeding. Each page shall cache its compiled display list to avoid full recompilation every frame. |

### User Stories

- As a player, I see a game world viewport above a status bar, matching the original Amiga split-screen layout.
- As a player, I experience smooth rendering at 30 fps without screen tearing or flicker.
- As a player, I see a mouse pointer cursor in the status bar area.

---



## 3. Scrolling & Camera

### Requirements

| ID | Requirement |
|----|-------------|
| R-SCROLL-001 | The map shall support two distinct scrolling mechanisms: (a) continuous sub-tile viewport drift via pixel offsets `RxOffset = map_x & 15` and `RyOffset = map_y & 31`, updated every frame; (b) incremental tile-level scrolling when tile coordinates change (`img_x = map_x >> 4` or `img_y = map_y >> 5`), shifting bitmap contents by one tile and repairing the exposed edge. |
| R-SCROLL-002 | On teleportation or multi-tile jump, a full map redraw shall be performed via `gen_mini()` + `map_draw()`. Full redraws shall also occur when `viewstatus` is 99, 98, or 3. |
| R-SCROLL-003 | Incremental tile scrolling shall shift all 5 bitplanes by one tile in any of 8 directions. After shifting, `strip_draw()` shall repair a single exposed column and `row_draw()` shall repair a single exposed row. |
| R-SCROLL-004 | The game logic sub-block (Phase 14: AI, encounters, hunger/fatigue, day/night advancement) shall only execute on frames where the map did not scroll (`dif_x == 0 && dif_y == 0`). During continuous scrolling, only actor movement, combat, and rendering occur. |
| R-SCROLL-005 | The visible playfield shall display a 19×6 grid of tiles (each 16×32 pixels), fitting within the 320-pixel raster width (304 pixels of tiles + 16 px scroll margin) and 200-scanline raster height (192 scanlines of tiles). |
| R-SCROLL-006 | The `viewstatus` display-state flag shall implement the following states: `0` = normal rendering; `1` = picking/dialogue (skip rest of tick, flasher blink active); `2` = full-screen map-message overlay; `3` = fade-in (fires `fade_normal()` after next `pagechange()`, then returns to 0); `4` = alternate picking (same as 1); `98` = full rebuild; `99` = full rebuild on init. Full map redraws shall occur for values 99, 98, and 3. |

### User Stories

- As a player, I see smooth pixel-level scrolling as my character walks, with no visible tile-popping.
- As a player, when I teleport, the screen updates instantly to the new location.
- As a player, I notice that game events (hunger, encounters) progress more when standing still than when moving, matching the original's scroll-gated design.

---



## 4. Sprites & Animation

### Requirements

| ID | Requirement |
|----|-------------|
| R-SPRITE-001 | Up to 20 actor slots (`anim_list[20]`) shall be supported, with hardcoded role assignments: slot 0 = hero, slots 1–2 = party members/carriers, slots 3–6 = enemy actors (up to 4, tracked by `anix`), slots 7–19 = world objects and set-figures. |
| R-SPRITE-002 | Sprite data shall be stored as 5-bitplane Amiga planar format with a 1-bit mask plane generated at load time by ORing all 5 image planes together and inverting (a pixel is transparent only when all planes are 1, i.e., color 31). |
| R-SPRITE-003 | Seven sprite sequence slots (`seq_list[7]`) shall be supported with type constants: PHIL=0 (player), OBJECTS=1 (world objects), ENEMY=2, RAFT=3, SETFIG=4 (NPC set-figures), CARRIER=5 (turtle or bird), DRAGON=6. The CARRIER slot loads different data depending on the active carrier type. |
| R-SPRITE-004 | Each actor shall maintain a 22-byte `struct shape` with 17 fields: `abs_x`/`abs_y` (u16, world position), `rel_x`/`rel_y` (u16, screen-relative), `type` (u8), `race` (u8), `index` (u8, animation frame), `visible` (u8, on-screen flag), `weapon` (u8), `environ` (u8, terrain state), `goal` (u8), `tactic` (u8), `state` (u8, animation state), `facing` (u8, direction 0–7), `vitality` (i16, hit points), `vel_x`/`vel_y` (i8, velocity for slippery/ice physics). |
| R-SPRITE-005 | 26 animation states shall be implemented, driven by the 87-entry `statelist` table. |
| R-SPRITE-006 | The walk cycle shall use 8 frames per direction, indexed by `cycle & 7` with direction-based base offset. |
| R-SPRITE-007 | The combat animation finite state automaton shall implement 9 states (FIGHTING, SWING1–4, BACKSWING, SHOOT1–3) with random branching between swing states. |
| R-SPRITE-008 | Palette remapping shall be supported for recolored enemy variants (e.g., Ogre vs Orc share sprites with different palettes). |
| R-SPRITE-009 | Up to 6 missiles shall be tracked simultaneously via `missile_list[6]`. Each missile has world position, type (NULL/arrow/rock/thing/fireball), time of flight, speed (0 = unshot), direction, and archer ID. Active missiles shall be added to `anim_list` as OBJECTS type, up to `anix2 = 20` total slots. |
| R-SPRITE-010 | Each sprite set shall track its dimensions and loading state via `struct seq_info`: width (pixels), height (pixels), frame count, image data pointer, mask data pointer, bytes per frame, and currently loaded file index. |
| R-SPRITE-011 | Sprite frame addressing shall use the formula: image = `seq_list[type].location + (planesize × 5 × frame_index)`, mask = `seq_list[type].maskloc + (planesize × frame_index)`. |
| R-SPRITE-012 | Walk-cycle base offsets shall be driven by the 16-entry `diroffs[]` table `{16,16,24,24,0,0,8,8,56,56,68,68,32,32,44,44}`. Indices 0–7 (paired) select walk animation bases per facing direction (0=N, 1=NE, …, 7=NW); indices 8–15 select fight/shoot animation bases. |
| R-SPRITE-013 | The 87-entry `statelist` table shall map `(motion_state, facing, frame)` tuples to 4-byte records `{figure: i8, wpn_no: i8, wpn_x: i8, wpn_y: i8}`. Layout: indices 0–31 = 8-frame walk cycles (S, W, N, E × 8), 32–79 = 12-state fight cycles (S, W, N, E × 12, with states 0–8 = swing, 9 = duplicate swing, 10–11 = ranged frames), 80–82 = death (3 frames), 83 = sinking, 84–85 = oscillation, 86 = asleep. |
| R-SPRITE-014 | OBJECTS-sheet frames shall render as 8 scanlines (half-height, packed two-per-row) when `inum` is in `{0x1b, 8..=12, 25, 26, 0x11..=0x17}` *or* `inum & 0x80` is set; all other frames render as 16 scanlines. Bit 7 (`0x80`) has dual role: it forces 8-scanline height *and* shifts the source-data Y-offset by +8 inside the addressed frame; the bit shall be stripped from `inum` before indexing the sheet. |
| R-SPRITE-015 | The hero weapon-overlay pass shall add per-weapon-class `k` offsets to `statelist[frame].wpn_no`: bow = 0, mace = 32, sword = 48, dirk = 64. The wand path shall use `inum = facing + 103` and shift Y by −6 when `facing == DIR_NE`. The bow shall, on walk-cycle frames (`frame < 32`), use the 32-entry `bow_x[]`/`bow_y[]` offset tables instead of `wpn_x`/`wpn_y`, and shall select its OBJECTS frame directionally from `frame / 8`: south → `0x53`, west → `30`, north → `0x51`, east → `30`. The two literal tables (`bow_x[32]`, `bow_y[32]`) shall be reproduced verbatim from fmain2.c:877–882 (see SPEC §5.6). |
| R-SPRITE-016 | The compositor shall render each character in two passes (body and weapon); whether the weapon draws behind or in front of the body shall be determined by XOR-ing the pass index with a facing-derived bit (`resolve_pass_params`). In Rust facing space (0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW) the weapon shall draw **behind** the body for facings `{0, 5, 6, 7}` and **in front** otherwise. |
| R-SPRITE-017 | The inventory items-page renderer shall blit each carried stack from the OBJECTS bitmap at source offset `n = inv_list[j].image_number * 80 + inv_list[j].img_off` with size `(16, inv_list[j].img_height)` and destination `(xoff + 20, yoff)`. Stacks shall repeat down the column spaced by `ydelta` up to `maxshown` rows; only `inv_list` rows `0..GOLDBASE` shall be eligible for stacking. This path shall not consult the half-height set or bit 7. |

### User Stories

- As a player, I see my character animate through walk, idle, combat, and death states correctly for all 8 directions.
- As a player, I see distinct enemy types with appropriate appearances and recolored variants.
- As a player, I see arrows and fireballs travel across the screen as distinct animated projectiles.

---



## 5. Terrain Masking & Z-Order

### Requirements

| ID | Requirement |
|----|-------------|
| R-ZMASK-001 | Sprites shall be Z-sorted back-to-front by Y-coordinate using a bubble sort of `anim_index[]` before rendering each frame. The sort also determines `nearest_person` for NPC interaction. |
| R-ZMASK-002 | Depth adjustments: dead actors render at Y−32, riding hero at Y−32, actor slot 1 (mount/companion) at Y−32, deeply sunk actors (environ > 25) at Y+32. |
| R-ZMASK-003 | Sprite compositing shall use a 4-stage pipeline: (1) `save_blit` — save background under sprite footprint; (2) `maskit` × N — stamp terrain occlusion masks from `shadow_mem` into compositing buffer for each overlapping tile-column; (3) `mask_blit` — combine sprite transparency mask with terrain occlusion mask; (4) `shape_blit` — cookie-cut draw sprite onto screen using formula D = (A·B) + (¬A·C), where A = compositing mask, B = sprite data, C = screen background. |
| R-ZMASK-004 | Terrain occlusion masks shall be applied per tile-column and tile-row that a sprite overlaps, using the mask mode read from the lower nibble of the terrain-rule byte (`terra_mem[image_id*4 + 1] & 15`). Modes 0–7 apply the following per-column skip conditions (see SPEC §6.3): mode 0 = always skip (no occlusion, flat ground); mode 1 = skip first column (right-side-only occlusion); mode 2 = skip when `ystop > 35` (top-only, low wall); mode 3 = skip when `hero_sector == 48` and actor is not NPC 1 (bridge — hero walks over); mode 4 = skip when first column OR `ystop > 35` (combined right + top); mode 5 = skip when first column AND `ystop > 35` (right AND top); mode 6 = full-tile mask (tile 64) when not on the bottom row (two-story buildings); mode 7 = skip when `ystop > 20` (stricter top-only). Mask mode values 8–15 are not used by any shipped terra set. |
| R-ZMASK-005 | Carriers, arrows, fairy sprites (object indices 100–101), and certain NPC races shall skip terrain masking entirely. |
| R-ZMASK-006 | Background restoration (`rest_blit`) shall run in reverse compositing order to correctly rebuild overlapping backgrounds. Maximum sprites per frame: `MAXSHAPES` = 25, limited by backsave buffer capacity (5920 bytes per page). |
| R-ZMASK-007 | The compositor shall short-circuit terrain masking (skip `maskit`/`mask_blit`, run only `save_blit`/`shape_blit`) when *any* of the following hold: (a) `atype == CARRIER`; (b) the hero is riding the swan boat; (c) the sprite is the active fiery-death overlay rectangle; (d) `inum ∈ {100, 101}` (fairy / sparkle FX); (e) NPC race ∈ `{0x85, 0x87}`. |
| R-ZMASK-008 | Drowning bubble frames (97 and 98) shall render without applying any terrain mask, even when no other early-exit gate fires. |
| R-ZMASK-009 | The vestigial `blithigh = 32` override shall be preserved verbatim for small-object and weapon-overlay mask paths (`compute_terrain_mask`). The two-pass body/weapon mask shall share the body's `ground` value across both passes; the sinking-ramp Y-shift (`an.environ > 2`) clips the body but does not lift the weapon mask. |

### User Stories

- As a player, I see my character walk behind trees, buildings, and walls when the character is deeper in the scene.
- As a player, I see sprites layered correctly (behind grounded objects, in front of background tiles).
- As a player, I see flying creatures (bird carrier, fairy) rendered without being occluded by ground terrain.

---


