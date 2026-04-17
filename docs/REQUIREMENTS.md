# The Faery Tale Adventure — Requirements & User Stories

> Derived from [RESEARCH.md](RESEARCH.md), [ARCHITECTURE.md](ARCHITECTURE.md), and [STORYLINE.md](STORYLINE.md).
> Target: Rust/SDL2 reimplementation faithful to the 1987 Amiga original.

Each requirement is a testable statement. User stories follow the format:
*"As a player, I can [action] so that [outcome]."*

---

## Table of Contents

1. [Display & Rendering](#1-display-rendering)
2. [World & Map](#2-world-map)
3. [Scrolling & Camera](#3-scrolling-camera)
4. [Sprites & Animation](#4-sprites-animation)
5. [Terrain Masking & Z-Order](#5-terrain-masking-z-order)
6. [Day/Night Visuals](#6-daynight-visuals)
7. [Player Input & Movement](#7-player-input-movement)
8. [Combat](#8-combat)
9. [AI & Encounters](#9-ai-encounters)
10. [NPCs & Dialogue](#10-npcs-dialogue)
11. [Inventory & Items](#11-inventory-items)
12. [Quest Progression](#12-quest-progression)
13. [Doors & Buildings](#13-doors-buildings)
14. [Day/Night Cycle & Clock](#14-daynight-cycle-clock)
15. [Survival (Hunger, Fatigue, Health)](#15-survival-hunger-fatigue-health)
16. [Magic](#16-magic)
17. [Death & Revival](#17-death-revival)
18. [Carriers (Raft, Turtle, Bird)](#18-carriers-raft-turtle-bird)
19. [Audio](#19-audio)
20. [Intro & Narrative](#20-intro-narrative)
21. [Save/Load](#21-saveload)
22. [UI & Menus](#22-ui-menus)
23. [Asset Loading](#23-asset-loading)
24. [Special Effects](#24-special-effects)

---

## 1. Display & Rendering

### Requirements

| ID | Requirement |
|----|-------------|
| R-DISP-001 | The game shall render a 288×140-pixel playfield viewport (lo-res, 5 bitplanes, 32 colors) scaled to fill the upper portion of a 640×480 logical canvas. |
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


## 2. World & Map

### Requirements

| ID | Requirement |
|----|-------------|
| R-WORLD-001 | The game world shall use pixel coordinates with X range 0–32767 (`MAXCOORD` = 0x7FFF) and Y range 0–40959 (0x9FFF), with unsigned 16-bit wrapping at boundaries. |
| R-WORLD-002 | The world shall be divided into 10 regions: 8 outdoor (2×4 grid, indices 0–7), 1 building interior (index 8), 1 dungeon (index 9). |
| R-WORLD-003 | Region number shall be computed from tile-level sector coordinates using the formula: `region = (xs >> 6) & 1 + ((ys >> 5) & 3) * 2`, where xs and ys are derived from pixel coordinates via `tile_x = map_x >> 4`, `tile_y = map_y >> 5`, `sector_x = tile_x >> 4`, `sector_y = tile_y >> 3`. |
| R-WORLD-004 | Each region shall load its own tileset (4 image banks of 64 tiles each = 256 tiles), two terrain property tables (1024 bytes total), sector map (256 sectors × 128 bytes = 32768 bytes), and region map (4096 bytes). Asset configuration is defined by `file_index[10]`, which specifies 4 image bank references, 2 terrain table IDs, sector map start, region map start, and setfig character set ID. |
| R-WORLD-005 | Terrain properties shall be encoded as 4-byte entries: byte 0 = mask shape index (for terrain occlusion), byte 1 lower nibble (bits 0–3) = walkability (0–3), byte 1 upper nibble (bits 4–7) = mask mode (0–7, controlling occlusion behavior). |
| R-WORLD-006 | Crossing a region boundary shall trigger automatic region data reload. Region transitions occur via outdoor boundary crossing (`gen_mini`), door transitions, or respawn. Non-blocking incremental loading (`load_next`) shall be used during normal gameplay; blocking loading (`load_all`) for immediate transitions. |
| R-WORLD-007 | A minimap cache (19×6 = 114 entries) shall be maintained, mapping viewport tile positions to terrain tile IDs for fast terrain mask lookups during sprite compositing. The `genmini` function resolves world coordinates through the two-level map hierarchy to fill this buffer. |
| R-WORLD-008 | The map shall use a two-level hierarchy: a region map (128×32 grid of sector indices, 4 KB) and sector data (256 sectors × 128 bytes each, where each sector is a 16×8 grid of tile indices). |
| R-WORLD-009 | Each region shall support up to 250 world objects per sector, each described by a 6-byte `struct object` (x, y, object type ID, status byte). Two object arrays (`ob_listg[]`, `ob_list8[]`) shall track active objects. |
| R-WORLD-010 | Desert access restriction: if `region == 4` and the player has fewer than 5 gold statues (`stuff[STATBASE] < 5`), desert map squares shall be blocked. |

### User Stories

- As a player, I can walk seamlessly from one region to another without noticing data loads.
- As a player, I see different terrain tilesets when entering distinct regions (snow, desert, swamp, etc.).
- As a player, I cannot enter the desert until I have collected enough gold statues.

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
| R-ZMASK-004 | Terrain occlusion masks shall be applied per tile-column and tile-row that a sprite overlaps, based on the tile's mask mode (0–7) from terrain properties. Mode 0 = no occlusion; modes 1–7 implement documented skip conditions. |
| R-ZMASK-005 | Carriers, arrows, fairy sprites (object indices 100–101), and certain NPC races shall skip terrain masking entirely. |
| R-ZMASK-006 | Background restoration (`rest_blit`) shall run in reverse compositing order to correctly rebuild overlapping backgrounds. Maximum sprites per frame: `MAXSHAPES` = 25, limited by backsave buffer capacity (5920 bytes per page). |

### User Stories

- As a player, I see my character walk behind trees, buildings, and walls when the character is deeper in the scene.
- As a player, I see sprites layered correctly (behind grounded objects, in front of background tiles).
- As a player, I see flying creatures (bird carrier, fairy) rendered without being occluded by ground terrain.

---


## 6. Day/Night Visuals

### Requirements

| ID | Requirement |
|----|-------------|
| R-FADE-001 | Outdoor palette colors shall be dynamically scaled based on the `lightlevel` triangular wave (0–300, peaking at noon, bottoming at midnight). `lightlevel = daynight / 40`, mirrored at 300: `if lightlevel >= 300 then lightlevel = 600 - lightlevel`. |
| R-FADE-002 | Night palette shall enforce minimum brightness floors when `limit` is true: red ≥ 10%, green ≥ 25%, blue ≥ 60%, with all channels capped at maximum 100%. This produces a blue-tinted night effect. |
| R-FADE-003 | Indoor locations (region ≥ 8) shall always use full brightness (100, 100, 100) with no day/night variation. |
| R-FADE-004 | The Green Jewel light spell (`light_timer > 0`) shall add 200 to the red parameter of the fade calculation. Additionally, for each palette entry where red < green, red shall be boosted to match green, producing a warm amber illumination. |
| R-FADE-005 | Color 31 of the game palette shall be overridden per-region: region 4 (desert) = 0x0980 (orange-brown), region 9 (dungeon) with `secret_timer` active = 0x00F0 (bright green), region 9 (dungeon) normal = 0x0445 (dark grey-blue), all others = 0x0BDF (light blue sky). |
| R-FADE-006 | Twilight vegetation boost: colors 16–24 shall gain extra blue at dusk/dawn — when green% is 21–49: +2 blue per entry; when green% is 50–74: +1 blue per entry. |
| R-FADE-007 | Palette updates shall occur every 4 ticks (`daynight & 3 == 0`) or immediately during screen rebuild (`viewstatus > 97`). |
| R-FADE-008 | The status bar palette (`textcolors[20]`) shall NOT be affected by day/night fading. |
| R-FADE-009 | A blue night-shift factor `g2 = (100 - green%) / 3` shall be applied to the blue channel calculation, creating additional blue tinting as green brightness decreases. |
| R-FADE-010 | Outdoor RGB parameters derived from `lightlevel`: red = `lightlevel − 80` (+ 200 if light spell), green = `lightlevel − 61`, blue = `lightlevel − 62`. |

### User Stories

- As a player, I see a gradual transition from day to night with blue-tinted darkness.
- As a player, entering a building restores full brightness regardless of the time of day.
- As a player, I see a warm amber glow when the Green Jewel light spell is active.
- As a player, I see vegetation colors shift toward blue during twilight hours.

---


## 7. Player Input & Movement

### Requirements

| ID | Requirement |
|----|-------------|
| R-INPUT-001 | Three input sources shall be supported in priority order: mouse/compass (highest), joystick, keyboard (lowest). `decode_mouse()` shall be called every frame. |
| R-INPUT-002 | Mouse direction: when either mouse button is held (`qualifier & 0x6000`), cursor X > 265 shall map the cursor position to a 3×3 compass grid producing one of 8 compass directions. Cursor X ≤ 265 shall produce direction 9 (menu area, no movement). |
| R-INPUT-003 | Joystick direction: JOY1DAT register (`$dff00c`) shall be decoded via XOR of adjacent bits per axis, then indexed through the `com2[9]` lookup table: `{0,1,2,7,9,3,6,5,4}`. |
| R-INPUT-004 | Keyboard direction: numpad keys shall map to translated codes 20–29. Direction = `code − 20`, giving compass values 0–7 plus 9 (stop/center). The `keydir` variable shall persist until a new direction key or stop key is pressed. |
| R-INPUT-005 | Combat stance shall activate when any of: right mouse button held (`qualifier & 0x2000`), keyboard numpad-0 key held (`keyfight` flag), or joystick fire button pressed (CIA-A PRA register `$bfe001` bit 7 == 0, read directly bypassing input.device). |
| R-INPUT-006 | Melee weapon held shall set player state to FIGHTING (state 0–11). Ranged weapon (bow/wand) held shall set state to SHOOT1 (state 24). |
| R-INPUT-007 | 8 compass directions shall use non-uniform displacement vectors: cardinal directions have magnitude 3 (N/S: ±3, E/W: ±3), diagonals have magnitude 2 per axis. Values 8 and 9 both produce zero displacement. |
| R-INPUT-008 | Position update formula: `new_pos = (pos + (dir_vector[dir] * speed) >> 1) & 0x7FFF`. The shift is a logical right shift (`lsr`), not arithmetic. The Y coordinate shall additionally preserve bit 15 from the original value. |
| R-INPUT-009 | Hero walk speed shall vary by terrain: default = 2, slippery (environ −1) = 4, direction reversal (environ −3) = −2, wading (environ 2 or > 6) = 1, riding raft = 3. Non-hero actors use speed 1 in water, 2 otherwise. |
| R-INPUT-010 | Ice terrain (environ −2) shall use velocity-based physics: `vel += dir_vector[dir]` each tick, velocity clamped to magnitude 42 (40 on swan), position updated by `vel / 4`. Facing shall be derived from velocity via `set_course(0, −vel_x, −vel_y, 6)`. |
| R-INPUT-011 | When hunger > 120, each walking tick shall have a 1/4 chance (`!rand4()`) of deflecting the direction by ±1 (50/50 via `rand() & 1`), wrapped to 0–7. |
| R-INPUT-012 | Movement shall be blocked by terrain types returned by the dual-probe `_prox` function: probe at (x+4, y+2) blocks at type 1 or ≥ 10; probe at (x−4, y+2) blocks at type 1 or ≥ 8. Wraiths (race 2) skip terrain checks entirely. |
| R-INPUT-013 | The hero (index 0) shall treat terrain types 8 and 9 as passable (they cause effects but do not block movement). |
| R-INPUT-014 | If the hero has the crystal shard (`stuff[30]`), terrain type 12 shall be treated as passable. Terrain type 12 exists only in terra set 8 (Region 8 building interiors). |
| R-INPUT-015 | When player movement is terrain-blocked, auto-deviation shall try `dir + 1` (clockwise), then `dir − 2` (counterclockwise). If all three directions are blocked, `frustflag` shall increment. At `frustflag > 20`: scratching-head animation. At `frustflag > 40`: special animation index 40. |
| R-INPUT-016 | Actor-to-actor collision shall use a 22×18 pixel bounding box (`|dx| < 11`, `|dy| < 9`). Self, slot 1 (raft), CARRIER type (type 5), and DEAD actors shall be excluded. Actor collision returns code 16. |
| R-INPUT-017 | Outdoor regions (region_num < 8) shall wrap hero coordinates toroidally at boundaries 300 and 32565. Indoor regions shall not wrap. NPC coordinates shall never be wrapped. |
| R-INPUT-018 | Camera tracking shall use a dead zone of ±20 pixels X and ±10 pixels Y. Outside the dead zone, the camera scrolls 1 pixel per tick. Jumps exceeding 70 pixels X or 44/24 pixels Y (asymmetric) shall snap the camera immediately. |
| R-INPUT-019 | The FALL state shall decay velocity by 25% per tick: `vel = (vel * 3) / 4`. Position updates continue via `vel / 4`. |
| R-INPUT-020 | Swan dismount shall only be permitted when `|vel_x| < 15` and `|vel_y| < 15`. |
| R-INPUT-021 | The `letter_list[38]` table shall map keyboard characters to (menu, choice) pairs for all game actions including items, talk, game, buy, save, magic (F1–F7), and use (1–7, K). |
| R-INPUT-022 | The input handler shall install at priority 51 (above Intuition's 50) and process RAWKEY, RAWMOUSE, TIMER, and DISKIN events. TIMER events shall increment a ticker; at ticker == 16, a synthetic key event ($E0) shall be generated to prevent game loop stalls. |
| R-INPUT-023 | Keyboard repeat keys (qualifier bit 9 set) shall be ignored. Scancodes > $5A shall be ignored. Processed key events shall be nullified (type set to 0) before passing to the OS. |
| R-INPUT-024 | Cursor arrow keys ($4C–$4F) shall translate to values 1–4 for cheat movement, NOT to direction codes 20–29. |
| R-INPUT-025 | When compass direction changes from the previous value (`oldir`), `drawcompass()` shall be called to update the compass highlight. |

### User Stories

- As a player, I can control my character using mouse clicks on the compass, keyboard numpad, or joystick, with mouse taking highest priority when held.
- As a player, I can hold the fight button (right mouse, numpad-0, or joystick fire) to enter combat stance and swing my weapon.
- As a player, I slide on ice with momentum-based physics when walking on frozen terrain.
- As a player, my character stumbles when very hungry, creating visible disorientation.
- As a player, my character auto-deviates around obstacles when walking into walls, and shows a frustrated animation if stuck.
- As a player, I can use keyboard shortcuts for all game menus (Items, Talk, Game, Buy, Save, Magic, Use).

---


## 8. Combat

### Requirements

| ID | Requirement |
|----|-------------|
| R-COMBAT-001 | Melee hit detection shall compute a strike point extending `weapon_code * 2` pixels in the attacker's facing direction, with ±3 to ±4 pixel random jitter per axis (`rand8() − 3`). |
| R-COMBAT-002 | Player melee reach (`bv`) shall be `(brave / 20) + 5`, capped at 15. Monster melee reach shall be `2 + rand4()` (2–5), re-rolled each frame. |
| R-COMBAT-003 | Target matching shall use Chebyshev distance (max of `|dx|`, `|dy|`) from strike point to target. A hit requires: distance < `bv`, `freeze_timer == 0`, and for monster attackers only, `rand256() > brave` must pass. Player attacks always hit if in range. |
| R-COMBAT-004 | Melee damage formula: `wt + bitrand(2)` where `wt` is the weapon code. Touch attack (code 8) clamps `wt` to 5 before damage. Vitality floors at 0. |
| R-COMBAT-005 | Necromancer (race 9) shall be immune to weapons with code < 4 (melee only); message `speak(58)` on blocked hit. Witch (race 0x89) shall be immune to weapons < 4 unless Sun Stone (`stuff[7]`) is held. Spectre (race 0x8a) and Ghost (race 0x8b) shall be completely immune to all damage with no feedback. |
| R-COMBAT-006 | Knockback: defender pushed 2 pixels in attacker's facing direction via `move_figure(j, fc, 2)`. If knockback succeeds and attacker is melee (`i >= 0`), attacker slides 2 pixels forward (follow-through). DRAGON and SETFIG types are immune to knockback. |
| R-COMBAT-007 | 6 missile slots shall support arrows and fireballs. Arrow hit radius = 6 pixels; fireball hit radius = 9 pixels. Missile damage = `rand8() + 4` (4–11) for both types. |
| R-COMBAT-008 | Missile dodge: for player target, `bv = brave`; for monsters, `bv = 20`. Only missile slot 0 applies the dodge check `bitrand(512) > bv`; slots 1–5 always hit if in range. `dohit` attacker code = −1 for arrows, −2 for fireballs. |
| R-COMBAT-009 | Bow attacks require SHOOT1 (aiming) → SHOOT3 (release) animation states and arrow inventory. |
| R-COMBAT-010 | Dragon shall have 25% chance per frame (`rand4() == 0`) of launching a fireball at the hero. Witch shall deal `rand2() + 1` (1–2) damage when `witchflag` is set and distance < 100. |
| R-COMBAT-011 | The 9-state `trans_list[]` fight animation shall cycle through states 0→1→2→3→4→5→6→8→0 via `newstate[0]`. Each tick selects a random transition: `trans_list[state].newstate[rand4()]`. Monsters at states 6 or 7 are forced to state 8. |
| R-COMBAT-012 | Weapon types: 0=none, 1=dirk, 2=mace, 3=sword, 4=bow, 5=wand, 8=touch (monster-only). Damage equals weapon code; touch clamps to 5. Strike range = `weapon_code * 2` pixels. |
| R-COMBAT-013 | Near-miss sound shall play when Chebyshev distance < `bv + 2` and weapon ≠ wand: `effect(1, 150 + rand256())`. |
| R-COMBAT-014 | `checkdead(i, dtype)` shall trigger when vitality < 1 and state is not DYING or DEAD. Sets `goal=DEATH`, `state=DYING`, `tactic=7`. DKnight death triggers `speak(42)`. SETFIG (non-witch) death causes `kind −= 3`. Enemy death (i > 0) grants `brave++`. Player death (i == 0) triggers `event(dtype)`, `luck −= 5`, `setmood(TRUE)`. |
| R-COMBAT-015 | Death animation: `tactic` counts down from 7 to 0 (7 frames), sprites 80/81 alternating. At tactic 0, state transitions to DEAD with sprite index 82. |
| R-COMBAT-016 | Body search ("Get" action near dead body): weapon drop = monster's weapon code (1–5); if better than current, auto-equips. Bow drops also give `rand8() + 2` (2–9) arrows. Treasure from `treasure_probs[encounter_chart[race].treasure * 8 + rand8()]`. SetFig races (`race & 0x80`) yield no treasure. |
| R-COMBAT-017 | `aftermath()` fires when `battleflag` transitions from TRUE to FALSE. It counts dead and fleeing enemies for status messages but does not directly grant experience or loot. |
| R-COMBAT-018 | Bravery serves as both passive experience and active combat stat: melee reach = `(brave/20)+5` (max 15), monster dodge = `rand256() > brave`, missile dodge (slot 0) = `bitrand(512) > brave`, starting vitality = `15 + brave/4`, growth = +1 per kill. |
| R-COMBAT-019 | Luck decreases by 5 per player death and by 2 per ledge fall. When luck < 1 at the death countdown gate, the next death is permanent (no fairy rescue). |
| R-COMBAT-020 | Goodfairy countdown: 255→200 death sequence; 200→120 luck gate (luck < 1 → `revive(TRUE)` brother succession, FALL → `revive(FALSE)` non-lethal); 120→20 fairy sprite flies toward hero; 20→2 resurrection glow; 1 → `revive(FALSE)` fairy rescue. |
| R-COMBAT-021 | `revive(TRUE)` (new brother): brother increments (1→Julian, 2→Phillip, 3→Kevin, 4+→game over). Stats reset from `blist[]`. Inventory wiped for indices 0 to GOLDBASE−1. Starting weapon = Dirk. Vitality = `15 + brave/4`. Dead brother's body and ghost placed in world. |
| R-COMBAT-022 | `revive(FALSE)` (fairy rescue): no stat changes. Returns to last safe position (`safe_x`, `safe_y`). Vitality restored to `15 + brave/4`. |
| R-COMBAT-023 | Necromancer on death: transforms to Woodcutter (race 10, vitality 10) and drops Talisman (object 139). Witch on death: drops Golden Lasso (object 27). |

### User Stories

- As a player, I can fight enemies in melee and see damage applied based on my weapon code and random variation.
- As a player, I can use a bow to shoot arrows and a wand to shoot fireballs at enemies from a distance.
- As a player, I find weapons and treasure dropped by defeated enemies via body search.
- As a player, my bravery grows with each kill, making me progressively stronger in combat.
- As a player, if I die with sufficient luck, a fairy revives me; otherwise the next brother takes over.
- As a player, I must use ranged weapons or the Sun Stone to damage the Necromancer and Witch respectively.

---


## 9. AI & Encounters

### Requirements

| ID | Requirement |
|----|-------------|
| R-AI-001 | 11 goal modes shall control high-level NPC behavior: USER (0), ATTACK1 (1), ATTACK2 (2), ARCHER1 (3), ARCHER2 (4), FLEE (5), STAND (6), DEATH (7), WAIT (8), FOLLOWER (9), CONFUSED (10). |
| R-AI-002 | 13 tactical modes shall control per-tick NPC actions: FRUST (0), PURSUE (1), FOLLOW (2), BUMBLE_SEEK (3), RANDOM (4), BACKUP (5), EVADE (6), HIDE (7, planned but unimplemented), SHOOT (8), SHOOTFRUST (9), EGG_SEEK (10), DOOR_SEEK (11, vestigial), DOOR_LET (12, vestigial). |
| R-AI-003 | AI processing shall be suppressed when `goodfairy > 0 && goodfairy < 120` (fairy animation active). |
| R-AI-004 | The `set_course(object, target_x, target_y, mode)` algorithm shall compute movement direction using 7 modes: 0 (toward with snapping), 1 (toward + deviation at distance < 40), 2 (toward + deviation at distance < 30), 3 (away/reverse), 4 (toward without snapping), 5 (toward with snapping, no walk state), 6 (raw direction vector). |
| R-AI-005 | Directional snapping (modes ≠ 4): if one axis dominates (`(|major| >> 1) > |minor|`), the minor axis is zeroed. Mode 4 skips snapping, always allowing diagonal. |
| R-AI-006 | Random deviation in `set_course`: when deviation > 0, `rand() & 2` (bit 1) determines ±deviation, result wrapped with `& 7`. |
| R-AI-007 | Goal mode assignment at spawn: ranged weapon (code & 4) → ARCHER1/ARCHER2; melee → ATTACK1/ATTACK2. Cleverness field selects +0 or +1. |
| R-AI-008 | Runtime goal transitions: hero dead/falling with no leader → FLEE; with leader → FOLLOWER. Vitality < 2 → FLEE. Special extent mismatch (xtype > 59, race ≠ v3) → FLEE. Unarmed (weapon < 1) → CONFUSED. |
| R-AI-009 | All tactical movement shall be rate-limited: base 12.5% (`!(rand() & 7)`), upgraded to 25% (`!(rand() & 3)`) for ATTACK2 goal. When the gate fails, the actor continues its previous trajectory. |
| R-AI-010 | `do_tactic()` dispatch: PURSUE → `set_course` mode 0 to hero; FOLLOW → mode 0 to leader (+20 Y); BUMBLE_SEEK → mode 4 to hero; BACKUP → mode 3 from hero; EVADE → mode 2 to neighboring actor (+20 Y); SHOOT → mode 0 or 5 to hero (every tick, no rate limit); RANDOM → `facing = rand() & 7` directly; EGG_SEEK → mode 0 to fixed coords (23087, 5667). |
| R-AI-011 | AI loop processes actors 2 through `anix−1`. CARRIER type: face player every 16 ticks via `set_course(i, hero_x, hero_y, 5)`. SETFIG type: skipped entirely. |
| R-AI-012 | Battle detection: actors within 300×300 pixels of hero set `actors_on_screen = TRUE` and `battleflag = TRUE`. |
| R-AI-013 | Random reconsider probability: base `!bitrand(15)` = 1/16 (6.25%). For goals where `(mode & 2) == 0` (ATTACK1, ARCHER2): overridden to `!rand4()` = 25%. |
| R-AI-014 | Hostile AI tactic selection on reconsider: snake + turtle_eggs → EGG_SEEK; unarmed → RANDOM (mode→CONFUSED); vitality < 6 && rand2() → EVADE; archer near (xd<40, yd<30) → BACKUP; archer mid (xd<70, yd<70) → SHOOT; archer far → PURSUE; melee default → PURSUE. |
| R-AI-015 | Melee engagement threshold: `thresh = 14 − mode`. DKnight (race 7) overrides to 16. Within threshold (xd < thresh AND yd < thresh), enemy enters FIGHTING state. |
| R-AI-016 | Frustration cycle: blocked actors set `tactic = FRUST`. Next tick, FRUST selects a random escape tactic: ranged picks from {FOLLOW, BUMBLE_SEEK, RANDOM, BACKUP} (`rand4() + 2`); melee picks from {BUMBLE_SEEK, RANDOM} (`rand2() + 3`). SHOOTFRUST uses the same logic. |
| R-AI-017 | Cleverness 0 (stupid): ATTACK1/ARCHER1 goal, 12.5% tactic rate (ATTACK1 reconsiders at 25%). Cleverness 1 (clever): ATTACK2/ARCHER2 goal, 25% tactic rate for ATTACK2 (reconsiders at 6.25%). ATTACK2 creates persistent, aggressive behavior — commits to tactics and executes energetically. |
| R-AI-018 | CONFUSED mode: assigned when hostile actor loses weapon. First tick runs `do_tactic(i, RANDOM)`. Subsequent ticks: no AI processing occurs — actor walks in last random direction until blocked. |
| R-AI-019 | DKnight special behavior: out of melee range → `state = STILL`, `facing = 5` (south), does not pursue or call `do_tactic()`. In range → FIGHTING state. Never flees (exempt from flee when race matches extent v3 for xtype > 59 zones). Respawns every time player re-enters hidden valley. Fixed position at (21635, 25762). |
| R-AI-020 | `leader` shall be set to the first living active enemy at end of AI loop. FOLLOWER goal uses this to follow the pack leader. |
| R-AI-021 | 23 extent zones (22 + sentinel) shall define encounter regions. `find_place()` performs linear scan of entries 0–21 each frame (first match wins). Entry 22 (whole world, etype=3) is the sentinel fallback. |
| R-AI-022 | Extent type categories: 0–49 = random encounter zone (etype used as `xtype`); 50–59 = forced group encounter (monsters spawn immediately, v1=count, v3=type); 52 = astral plane (forces encounter_type=8, Loraii); 60–61 = special figure (unique NPC at extent center); 70 = carrier (load bird/turtle/dragon via `load_carrier(v3)`); 80 = peace zone; 81 = king peace (weapon draw → `event(15)`); 82 = sorceress peace (weapon draw → `event(16)`); 83 = princess rescue. |
| R-AI-023 | Encounter placement shall occur every 16 frames: up to 10 random locations tried via `set_loc()` (150–213 pixels from hero), each validated for terrain type 0 (walkable). Dead enemy slots recycled when all 4 slots full. |
| R-AI-024 | Danger check every 32 frames. Conditions: no actors on screen, no pending load, no active carrier, `xtype < 50`. Danger level: indoor = `5 + xtype`, outdoor = `2 + xtype`. Spawn probability = `rand64() <= danger_level`, i.e., `(danger_level + 1) / 64`. |
| R-AI-025 | Monster type selection: base = `rand4()` (0–3 → ogre/orc/wraith/skeleton). Overrides: swamp (xtype=7) remaps wraith roll to snake (4); spider region (xtype=8) forces spider (6); xtype=49 forces wraith (2). |
| R-AI-026 | Monster count per encounter: `encounter_number = v1 + rnd(v2)`. Only 4 enemy actor slots (indices 3–6) exist; excess resolves over time as dead slots are recycled. |
| R-AI-027 | Race mixing: when `mixflag & 2` (and encounter_type ≠ snake), `race = (encounter_type & 0xFFFE) + rand2()`, allowing adjacent types to mix (ogre↔orc, wraith↔skeleton). `mixflag` disabled for xtype > 49 or xtype divisible by 4. |
| R-AI-028 | Weapon selection at spawn: `weapon_probs[arms * 4 + wt]` where `wt` is re-randomized per enemy if `mixflag & 4`, otherwise shared within batch. |
| R-AI-029 | Peace zones (etype 80–83) set `xtype ≥ 50`, failing the `xtype < 50` guard on danger checks, completely suppressing random encounters. The `aggressive` field in `encounter_chart[]` is never read at runtime. |
| R-AI-030 | Only extents 0 (bird) and 1 (turtle) are persisted in savegames. Turtle extent starts at (0,0,0,0) and must be repositioned via `move_extent()`. |

### User Stories

- As a player, I encounter enemies randomly as I explore the world, with difficulty varying by region and danger level.
- As a player, I experience enemies that pursue, attack, flee, evade, and wander with distinct behaviors driven by their cleverness and goal mode.
- As a player, I find safe areas (towns, temples, king's domain) where no random enemies spawn and weapon draw may be blocked.
- As a player, I face the Dark Knight as a fixed-position guardian that blocks passage, fights only at close range, and respawns on re-entry.
- As a player, I encounter carriers (swan, turtle, dragon) in specific world zones that I can ride.
- As a player, enemy groups may contain mixed types (ogre/orc, wraith/skeleton) for variety.

---


## 10. NPCs & Dialogue

### Requirements

| ID | Requirement |
|----|-------------|
| R-NPC-001 | 14 setfig NPC types shall be supported, each identified by `race & 0x7F` (setfig index). Race code = `index + 0x80`. Vitality = `2 + index*2`. The `goal` field (from object list position) shall select per-instance dialogue variants for wizards, rangers, and beggars. |
| R-NPC-002 | The Talk submenu ("Yell Say Ask") shall offer three options with hit values 5, 6, 7. Yell range = 100 units (`nearest_fig(1,100)`); Say/Ask range = 50 units (`nearest_fig(1,50)`). If target is within 35 units when yelling, respond with `speak(8)` ("No need to shout!"). Ask shall be functionally identical to Say — all three options share the same dispatch logic after the range check. |
| R-NPC-003 | The speech catalogue shall contain entries indexed 0–60+. The `%` character shall be substituted with the current brother's name at display time. Entries 0–7 are enemy responses (indexed directly by race), entries 8–60 cover NPC dialogue and quest text. |
| R-NPC-004 | Wizard NPCs (index 0) shall respond based on `kind` stat: if `kind < 10`, `speak(35)` ("Away with you, ruffian!"); if `kind >= 10`, `speak(27 + goal)` selecting from 8 different hints (speeches 27–34). Eight wizard instances exist across regions with goals 0–7, each giving a unique hint about the quest. |
| R-NPC-005 | Priest NPCs (index 1) shall have three-stage dialogue: (1) if hero has Writ (`stuff[28]`) and `ob_listg[10].ob_stat == 0`, `speak(39)` and reveal a gold statue (`ob_listg[10].ob_stat = 1`); (2) if hero has Writ but statue already given, `speak(19)` ("already gave the statue"); (3) if no Writ and `kind < 10`, `speak(40)` ("Repent, Sinner!"); (4) if no Writ and `kind >= 10`, `speak(36 + daynight%3)` (rotating hints) AND heal hero to max vitality (`15 + brave/4`). Healing occurs on every qualifying visit regardless of which rotating message plays. |
| R-NPC-006 | Sorceress NPC (index 7) shall give a golden figurine on first visit: `speak(45)` and set `ob_listg[9].ob_stat = 1`. On return visits, no speech shall play but `luck` silently increases by 5 if `luck < rand64()`. |
| R-NPC-007 | Spectre NPC (index 10) shall respond with `speak(47)` ("Bring me bones of the ancient King") when talked to. When given a Bone (`stuff[29]`), the Spectre responds with `speak(48)`, clears `stuff[29]`, and drops a Crystal Shard (object 140). The shard (`stuff[30]`) enables passage through terrain type 12 barriers in the Spirit Plane. Spectres and Ghosts shall be absolutely immune to all damage (`dohit()` returns early for races 0x8a/0x8b). |
| R-NPC-008 | Ghost NPCs (index 11) shall say `speak(49)` ("I am the ghost of your dead brother. Find my bones..."). Ghosts appear after a brother dies (`ob_listg[brother+2].ob_stat = 3`). Picking up the dead brother's bones (ob_id 28) shall merge the dead brother's entire 31-slot inventory into the current brother's, and remove both ghost setfigs (`ob_listg[3].ob_stat = ob_listg[4].ob_stat = 0`). |
| R-NPC-009 | Bartender NPC (index 8) dialogue shall depend on fatigue and time: if `fatigue < 5` → `speak(13)` ("Good Morning"); else if `dayperiod > 7` → `speak(12)` ("Would you like to buy something?"); else → `speak(14)` ("Have a drink!"). |
| R-NPC-010 | The GIVE menu ("Gold Book Writ Bone") shall offer 4 options (hit values 5–8). Gold (hit 5): requires `wealth > 2`, costs 2 gold; if `rand64() > kind` then `kind++`; beggars respond with `speak(24 + goal)`, others with `speak(50)`. Book (hit 6): shall always be disabled (hardcoded in `set_options`). Writ (hit 7): enabled when `stuff[28] != 0` but has no processing handler — the Writ is checked passively during Priest talk. Bone (hit 8): requires `stuff[29] != 0`; non-spectre NPCs respond `speak(21)` ("no use for it"); Spectre (0x8a) responds `speak(48)`, clears `stuff[29]`, drops crystal shard (object 140). |
| R-NPC-011 | Turtle carrier dialogue (when `active_carrier == 5`) shall depend on sea shell possession: if hero lacks shell (`stuff[6] == 0`), `speak(56)` ("Thank you for saving my eggs!") and grant shell (`stuff[6] = 1`); if hero has shell, `speak(57)` ("Hop on my back for a ride"). |
| R-NPC-012 | Proximity auto-speech shall trigger when specific NPCs are near the player, independent of the Talk menu. Tracked by `last_person` to prevent re-triggering for the same NPC instance. Auto-speaking NPCs: Beggar (0x8d) → `speak(23)`, Witch (0x89) → `speak(46)`, Princess (0x84) → `speak(16)` (only if `ob_list8[9].ob_stat` set), Necromancer (race 9) → `speak(43)`, DreamKnight (race 7) → `speak(41)`. |
| R-NPC-013 | King NPC (index 5) shall only speak when the princess captive flag is set (`ob_list8[9].ob_stat != 0`): `speak(17)` ("I cannot help you, young man"). When the flag is clear (after rescue), the King produces no dialogue through the talk handler. The King's post-rescue speech `speak(18)` ("Here is a writ...") is triggered by the `rescue()` function, not the talk handler. |
| R-NPC-014 | Noble (index 6) shall always respond with `speak(20)` ("If you could rescue the king's daughter..."). Guards (indices 2–3) shall always respond with `speak(15)` ("State your business!"). Princess (index 4) shall respond with `speak(16)` ("Please, sir, rescue me...") only when `ob_list8[9].ob_stat` is set; Princess also auto-speaks on proximity when captured. |
| R-NPC-015 | Ranger NPCs (index 12) shall give directional hints based on region and goal: if `region_num == 2` (swamp), `speak(22)` ("Dragon's cave is directly north"); otherwise `speak(53 + goal)` where goal 0 → east, goal 1 → west, goal 2 → south. Rangers appear only in ob_list0 (snow, 3 rangers) and ob_list2 (swamp, 1 ranger). |
| R-NPC-016 | DreamKnight (race 7) shall auto-speak `speak(41)` ("None may enter the sacred shrine...") on entering Hidden Valley extent (index 15). On death, `speak(42)` ("You have earned the right to enter...") and `brave++`. The DreamKnight respawns every time the player enters the extent — no death flag is persisted. |
| R-NPC-017 | Necromancer (race 9) shall auto-speak `speak(43)` ("So this is the so-called Hero...") on proximity. Magic shall be explicitly blocked in the Necromancer's arena (`extn->v3 == 9`): `speak(59)` ("Your magic won't work here, fool!"). The Necromancer can only be damaged with Bow or Wand (`weapon >= 4`); lesser weapons produce `speak(58)` ("Can't hurt me with that!"). |
| R-NPC-018 | When the player talks to enemies, the speech index shall equal the enemy's `race` value directly: Ogre(0) → `speak(0)`, Orc(1) → `speak(1)`, Wraith(2) → `speak(2)`, Skeleton(3) → `speak(3)`, Snake(4) → `speak(4)`, Salamander(5) → `speak(5)`, Spider(6) → `speak(6)`, DKnight(7) → `speak(7)`. |
| R-NPC-019 | Bartender NPCs (race 0x88) shall serve as shopkeepers. The BUY menu shall only activate with race 0x88. Seven items available at fixed prices: Food (3 gold, reduces hunger by 50), Arrows (10 gold, `stuff[8] += 10`), Vial (15 gold, `stuff[11]++`), Mace (30 gold, `stuff[1]++`), Sword (45 gold, `stuff[2]++`), Bow (75 gold, `stuff[3]++`), Totem (20 gold, `stuff[13]++`). Purchase requires `wealth > price`. |
| R-NPC-020 | NPCs with the `can_talk` flag set (Wizard, Priest, King, Ranger, Beggar) shall play a talking mouth animation on a 15-tick timer during speech. NPCs without the flag produce speech text but show no talking animation. |
| R-NPC-021 | Witch NPC (index 9) shall respond with `speak(46)` ("Look into my eyes and Die!") when talked to. The Witch is invulnerable when `weapon < 4` AND `stuff[7] == 0` (no Sun Stone). With the Sun Stone (`stuff[7] != 0`), all weapons deal damage. With Bow or Wand (`weapon >= 4`), damage applies regardless of Sun Stone. Killing the Witch (race 0x89) drops the Golden Lasso (`leave_item(i, 27)`, `stuff[5]`). |
| R-NPC-022 | Beggar NPCs (index 13) shall say `speak(23)` ("Alms! Alms for the poor!") when talked to or on proximity auto-speech. When given gold, beggars respond with `speak(24 + goal)`: goal 0 → "Seek two women, one Good, one Evil", goal 1 → "Jewels, glint in the night — gift of Sight", goal 2 → "Where is the hidden city?". The beggar at `ob_list3[3]` has `goal=3`, which overflows the 3 speeches (24–26) and reads `speak(27)` (the first wizard hint) — this is an original bug that shall be reproduced. |

### User Stories

- As a player, I can talk to NPCs and receive contextual dialogue, hints, and quest items based on my stats and progress.
- As a player, I can give gold to beggars to receive prophecy hints, with a chance of increasing my kindness stat.
- As a player, I receive different responses from the same NPC type depending on my `kind` stat, quest items held, and the NPC's per-instance `goal` value.
- As a player, I can buy weapons, supplies, and magic items from bartender NPCs using gold.
- As a player, certain NPCs speak automatically when I approach, without requiring the Talk menu.
- As a player, I can give the King's Bone to the Spectre at night to receive the Crystal Shard, enabling passage through spirit barriers.
- As a player, I can show the Writ to a Priest to receive a golden statue.
- As a player, talking to a Priest with high kindness heals me to full vitality.

---


## 11. Inventory & Items

### Requirements

| ID | Requirement |
|----|-------------|
| R-INV-001 | Each brother shall have a `stuff[]` array of at least 36 elements (indices 0–34 active, index 35 as temporary accumulator): weapons (0–4), special items (5–8), magic consumables (9–15), keys (16–21), quest/stat items (22–30), gold pickups (31–34). Index 35 (`ARROWBASE`) shall serve as a temporary accumulator for quiver pickups, with `stuff[8] += stuff[ARROWBASE] * 10` applied after Take. |
| R-INV-002 | Item pickup shall use the `itrans` translation table (31 ob_id→stuff-index pairs, 0-terminated) to map ground object types to inventory slots. Lookup shall be a linear scan of pairs until the terminator. On match, `stuff[index]` shall be incremented. |
| R-INV-003 | Body search on dead enemies shall roll weapon drop then treasure drop from probability tables. |
| R-INV-004 | Weapon damage ranges: Dirk 1–3, Mace 2–4, Sword 3–5, Bow 4–11 (missile, consumes arrows), Wand 4–11 (missile, no ammo cost). Equipping a weapon via USE shall set `weapon = hit + 1`. |
| R-INV-005 | Inventory state shall be preserved per-brother (separate static arrays for Julian, Phillip, Kevin). The `stuff` pointer shall be bound to the current brother via `blist[brother-1].stuff`. All three inventories shall be saved and loaded. |
| R-INV-006 | Key items (gold, green, blue, red, grey, white at stuff[16–21]) shall be consumed when used to open locked doors. The key handler shall try 9 directions from the hero's position at 16-pixel distance via `doorfind(x, y, keytype)`. |
| R-INV-007 | Bow shall consume one arrow per shot (`stuff[8]--`). When arrows are depleted mid-combat, the system shall auto-switch to the next best weapon. |
| R-INV-008 | All magic consumables (stuff[9–15]) shall be consumed on use (`--stuff[4+hit]`). Magic shall be blocked when `extn->v3 == 9` (restricted areas). |
| R-INV-009 | Magic item effects: Blue Stone teleports via stone circle (sector 144 only). Green Jewel adds 760 to `light_timer`. Glass Vial heals `rand8() + 4` (4–11) vitality, capped at `15 + brave/4`. Crystal Orb adds 360 to `secret_timer`. Bird Totem renders the overhead map with player position. Gold Ring adds 100 to `freeze_timer` (disabled while riding). Jade Skull kills all visible enemies with `vitality > 0`, `type == ENEMY`, `race < 7`, and shall decrement `brave` per kill. |
| R-INV-010 | The shop system (`jtrans`) shall offer 7 items: Food (3g, calls `eat(50)`), Arrows (10g, `stuff[8] += 10`), Glass Vial (15g), Mace (30g), Sword (45g), Bow (75g), Bird Totem (20g). Purchase shall require proximity to a shopkeeper (race `0x88`) and `wealth > price`. Food shall call `eat(50)` rather than adding to any stuff[] slot. |
| R-INV-011 | Container loot (chest, urn, sacks) shall use `rand4()` for tier: 0 = nothing, 1 = one random item (`rand8() + 8` → indices 8–15, where index 8 means quiver), 2 = two different random items from same range (index 8 → 100 gold instead), 3 = three copies of same item (index 8 → 3 random keys from `KEYBASE` to `KEYBASE+5`). |
| R-INV-012 | Gold pickup items (stuff[31–34]) shall add their `maxshown` value (2, 5, 10, 100) to the `wealth` variable instead of being stored in `stuff[]`. |
| R-INV-013 | GIVE mode: giving gold costs 2 gold (`wealth -= 2`), and if `rand64() > kind` then `kind++`. Beggars (race `0x8d`) give a goal speech. Giving bone to Spectre (race `0x8a`) shall produce `speak(48)` and drop a crystal shard. Non-spectre NPCs shall reject bone with `speak(21)`. |
| R-INV-014 | Fruit (stuff[24]) shall auto-consume when `hunger > 30` at safe points, reducing hunger by 30. On pickup when `hunger >= 15`, fruit shall be eaten immediately via `eat(30)` instead of stored. When `hunger < 15`, fruit shall be stored in inventory. |
| R-INV-015 | Rose (stuff[23]) shall grant lava immunity: force `environ = 0` in the fiery_death zone (`map_x` 8802–13562, `map_y` 24744–29544). Without it, `environ > 15` kills instantly and `environ > 2` drains vitality per tick. Only protects the player (actor 0), not carriers or NPCs. |
| R-INV-016 | Sun Stone (stuff[7]) shall make the Witch (race `0x89`) vulnerable to melee weapons. Without it, attacks on the Witch shall produce `speak(58)`: "Stupid fool, you can't hurt me with that!" |
| R-INV-017 | Golden Lasso (stuff[5]) shall enable mounting the swan carrier. The Witch shall drop the lasso on death. |
| R-INV-018 | Sea Shell (stuff[6]) USE shall call `get_turtle()` to summon the sea turtle carrier near water. Summoning shall be blocked inside the rectangle (11194–21373, 10205–16208). |
| R-INV-019 | Crystal Shard (stuff[30]) shall override terrain type 12 collision blocking in dungeons. The shard shall never be consumed. |
| R-INV-020 | On brother succession (`revive(TRUE)`), all inventory shall be wiped. Starting loadout for the new brother shall be one Dirk only (`stuff[0] = 1`). |
| R-INV-021 | Special-cased pickups bypassing `itrans`: gold bag (ob_id 13) adds 50 to wealth; scrap (ob_id 20) triggers `event(17)` plus region-specific event; dead brother bones (ob_id 28) recovers dead brother's full inventory; containers (ob_id 14 urn, 15 chest, 16 sacks) use the container loot system; turtle eggs (ob_id 102) and footstool (ob_id 31) cannot be taken. |
| R-INV-022 | World objects shall use a 6-byte record: world X (u16), world Y (u16), ob_id (i8), ob_stat (i8). ob_stat values: 0 = disabled/skipped, 1 = on ground (pickable), 2 = in inventory/taken (skipped), 3 = setfig NPC, 4 = dead setfig, 5 = hidden (revealed by Look), 6 = cabinet item. |
| R-INV-023 | Objects shall be organized into 1 global list (11 entries, processed every tick) and 10 regional lists (regions 0–9, only current region processed per tick). |
| R-INV-024 | Random treasure (10 items) shall be distributed on first visit to each outdoor region (0–7). Regions 8 (building interiors) and 9 (underground) shall be excluded. Distribution shall be weighted: 4/16 sacks, 3/16 grey key, 2/16 chest, 1/16 each for money, gold key, quiver, red key, bird totem, vial, white key. Positions shall be randomized within the region, rejecting non-traversable terrain. |
| R-INV-025 | Only one dynamically dropped item (`ob_listg[0]`) may exist at a time; each `leave_item()` call overwrites the previous drop slot contents. |
| R-INV-026 | Object state shall be fully persisted in save/load: global list (66 bytes), mapobs counts (20 bytes), dstobs distribution flags (20 bytes), and all 10 regional lists (variable size). |
| R-INV-027 | Menu item availability shall be driven by `stuff_flag(index)`: return 10 (enabled) if `stuff[index] > 0`, else 8 (disabled). The Book entry in the GIVE menu shall be hardcoded disabled. |
| R-INV-028 | Opened chests shall change `ob_id` from CHEST (15) to empty chest (0x1d/29). Other taken objects shall set `ob_stat = 2`. Look-revealed hidden objects (ob_stat 5) shall change to `ob_stat = 1` (pickable). |
| R-INV-029 | Per-tick object processing shall handle global objects first (with flag `0x80`), then regional objects. The setfig/enemy boundary (`anix`) shall be updated when the setfig count exceeds 3. No more than 20 object entries shall be rendered per tick. |
| R-INV-030 | Writ (stuff[28]) obtained from princess rescue shall also grant 100 gold and 3 of each key type (`stuff[16..21] += 3`). Showing Writ to Priest shall trigger `speak(39)` and reveal gold statue at `ob_listg[10]`. |

### User Stories

- As a player, I can pick up items from the ground and see them added to my inventory.
- As a player, I can equip different weapons that affect my combat damage range.
- As a player, my inventory is separate for each brother and is wiped on death, with only a Dirk as starting equipment for the next brother.
- As a player, I can purchase items from shopkeepers when I have enough gold.
- As a player, I can use magic consumable items from my inventory with various tactical effects.
- As a player, I can open containers (chests, urns, sacks) to receive random loot of varying quality.
- As a player, I can give items to NPCs for quest progression (bone to spectre for crystal shard, gold for kindness).
- As a player, I can use LOOK to discover hidden items in the environment.
- As a player, I find random treasure scattered across outdoor regions I visit for the first time.

---


## 12. Quest Progression

### Requirements

| ID | Requirement |
|----|-------------|
| R-QUEST-001 | The main quest shall follow this critical path: rescue princess → obtain Writ from King → show Writ to Priest for gold statue → collect 5 golden statues total → enter hidden city of Azal → obtain Rose for lava immunity → cross lava to Citadel of Doom → traverse Spirit Plane (Crystal Shard required for terrain-12 barriers) → defeat Necromancer (Bow/Wand required) → pick up Talisman → victory. |
| R-QUEST-002 | Up to 3 princesses (Katra, Karla, Kandy) shall be rescuable, tracked by the `princess` counter (0, 1, 2). The counter persists across brother succession; each rescue shows a different princess's narrative text. After `princess >= 3`, no further rescues occur. |
| R-QUEST-003 | The rescue sequence shall be triggered by entering the princess extent zone (xtype 83, coordinates 10820–10877, 35646–35670) with `ob_list8[9].ob_stat != 0`. The `rescue()` function shall: (1) display rescue narrative via `placard_text(8 + princess*3)` through three princess-specific texts with `name()` interpolation; (2) display shared post-rescue text via `placard_text(17)` and `placard_text(18)` with 7.6-second pauses; (3) increment `princess`; (4) teleport hero to King's castle at (5511, 33780); (5) reposition bird extent via `move_extent(0, 22205, 21231)`; (6) place rescued princess NPC in castle (`ob_list8[2].ob_id = 4`); (7) grant Writ (`stuff[28] = 1`); (8) King speaks `speak(18)`; (9) reward 100 gold; (10) clear princess captive flag (`ob_list8[9].ob_stat = 0`); (11) grant +3 of each key type (`stuff[16..21] += 3`). |
| R-QUEST-004 | Quest state shall be tracked via object list `ob_stat` fields: `ob_list8[9].ob_stat` (princess captive, set to 3 by `revive(TRUE)`, cleared to 0 by `rescue()`), `ob_listg[9].ob_stat` (Sorceress statue given), `ob_listg[10].ob_stat` (Priest statue given), `ob_listg[5].ob_stat` (Spectre visibility: 3 if `lightlevel < 40`, else 2), `ob_listg[1-2].ob_stat` (dead brother bones, set to 1 on death), `ob_listg[3-4].ob_stat` (ghost brothers, set to 3 on death, cleared to 0 on bone pickup). Additional quest-relevant state: `stuff[22]` (Talisman), `stuff[25]` (gold statue count), `stuff[28]` (Writ), `stuff[29]` (Bone), `stuff[30]` (Crystal Shard), `princess` counter. |
| R-QUEST-005 | Hidden city access in region 4 (desert) shall be blocked when `stuff[25] < 5`: four map tiles at offset `(11×128)+26` shall be overwritten to impassable tile 254 on every region load. With `stuff[25] >= 5`, tiles remain passable. All 5 DESERT-type oasis doors shall also require `stuff[25] >= 5` to enter. |
| R-QUEST-006 | Win condition: when `stuff[22]` (Talisman) becomes nonzero after item pickup, set `quitflag = TRUE` and `viewstatus = 2`, then launch the victory sequence. |
| R-QUEST-007 | Victory sequence (`win_colors()`): (1) display `placard_text(6)` + `name()` + `placard_text(7)` with placard and 80-tick pause; (2) load victory image (`winpic`); (3) black out both viewports and hide HUD; (4) expand playfield to 156 lines; (5) 55-frame sunrise animation (i=25 to −29) using `sun_colors[53]` gradient table — first frame pauses 60 ticks, subsequent frames at 9 ticks each (total ≈11.1 seconds); (6) final 30-tick pause then fade to black. |
| R-QUEST-008 | 11 stone ring locations shall form a teleportation network; destination = `(current_stone + facing + 1) % 11`. |
| R-QUEST-009 | Stone ring activation requires: standing on sector 144, center-of-tile position check, match against `stone_list[]`. Visual effect: 32 frames of random palette cycling (`colorplay()`). |
| R-QUEST-010 | The princess captive flag (`ob_list8[9].ob_stat`) shall be reset to 3 during `revive(TRUE)` (brother succession), enabling each new brother to trigger one rescue with a different princess. `revive(FALSE)` (fairy rescue of same brother) shall NOT reset this flag. |
| R-QUEST-011 | Five golden figurines of Azal-Car-Ithil are required to access the desert. Sources: (1) Sorceress — first talk sets `ob_listg[9].ob_stat = 1`; (2) Priest — talk with Writ sets `ob_listg[10].ob_stat = 1`; (3) Seahold ground pickup at `ob_listg[6]`, (11092, 38526); (4) Ogre Den ground pickup at `ob_listg[7]`, (25737, 10662); (5) Octal Room ground pickup at `ob_listg[8]`, (2910, 39023). Dialogue-revealed statues work through standard Take: setting `ob_stat = 1` makes the object visible for `itrans` pickup. |
| R-QUEST-012 | The Necromancer (race 9, 50 HP, wand weapon) is the final boss. Invulnerable to weapons with `weapon < 4` (only Bow or Wand deal damage). Magic is blocked in the arena (`extn->v3 == 9`). On death: transforms to Woodcutter (race 10, vitality 10, weapon 0), drops Talisman (object 139 → `stuff[22]`). |
| R-QUEST-013 | The Rose (`stuff[23]`, `ob_list8[51]`) shall grant lava immunity: when hero is in the `fiery_death` area, `stuff[23]` resets environmental damage to 0 (`environ = 0`). Required for reaching the Citadel of Doom which sits inside the volcanic lava zone. |
| R-QUEST-014 | The Crystal Shard (`stuff[30]`) shall enable the hero to walk through terrain type 12 barriers (`stuff[30] && j==12` bypasses terrain collision). Required for navigating the Spirit Plane maze to reach the Necromancer's arena. |
| R-QUEST-015 | The Sun Stone (`stuff[7]`, `ob_list8[18]`, located in the Elf Glade behind door 48) shall make the Witch vulnerable to all weapons. Without it, only Bow/Wand can damage the Witch. Defeating the Witch drops the Golden Lasso. |
| R-QUEST-016 | The Golden Lasso (`stuff[5]`) shall be required to mount the Swan carrier (bird, `actor_file == 11`). Without the lasso, the bird cannot be ridden. The swan enables unrestricted flight over all terrain types. |
| R-QUEST-017 | The Sea Shell (`stuff[6]`) shall be obtained by talking to the Turtle carrier after finding turtle eggs at the extent zone (22945–23225, 5597–5747). USEing the shell anywhere summons the turtle for ocean travel. |
| R-QUEST-018 | Game over shall occur when all three brothers have permanently died (`brother > 3`): `placard_text(5)` ("Stay at Home!"), `Delay(500)` (10 seconds), `quitflag = TRUE`. |
| R-QUEST-019 | On brother succession (`revive(TRUE)`): the `princess` counter and all quest flags (`ob_listg`, `ob_list8` entries) persist. Stats are loaded fresh from `blist[]`, inventory is cleared (hero gets only a Dirk), position resets to Tambry (19036, 15755), and hunger/fatigue reset to 0. Dead brother's bones and ghost are placed in the world. The princess captive flag resets to 3. |
| R-QUEST-020 | When a living brother picks up a dead brother's bones (ob_id 28), both ghost setfigs shall be removed (`ob_listg[3].ob_stat = ob_listg[4].ob_stat = 0`) and the dead brother's entire 31-slot inventory shall be merged into the current brother's inventory. |
| R-QUEST-021 | The DreamKnight (race 7, 40 HP, sword) shall guard the Hidden Valley (extent index 15), blocking access to the Elf Glade (door 48) where the Sun Stone is located. It stands still facing south, fights when the player comes within 16px, never flees, and respawns on every extent entry. On death: `speak(42)` and `brave++`. |
| R-QUEST-022 | Pax zones shall enforce weapon restrictions: King's castle grounds (`xtype == 81`) trigger `event(15)` (weapon sheathed); Sorceress area (`xtype == 82`) trigger `event(16)` (calming influence). |
| R-QUEST-023 | The Spectre (`ob_listg[5]`) shall only be visible when `lightlevel < 40` (nighttime): `ob_stat` set to 3 at night, 2 during day. The hero must visit the Spectre at night to trade the King's Bone for the Crystal Shard. |
| R-QUEST-024 | The King's Bone (`stuff[29]`, `ob_list9[8]`) shall be found in the underground dungeon. It is required for the Spectre trade to obtain the Crystal Shard. |

### User Stories

- As a player, I can rescue captive princesses and receive rewards from the King including a Writ, gold, and keys.
- As a player, I need to collect 5 golden statues from various sources to unlock the hidden city of Azal.
- As a player, I can use stone rings to teleport across the world based on the direction I face.
- As a player, defeating the Necromancer and obtaining the Talisman wins the game with a victory cinematic.
- As a player, I must defeat the DreamKnight to access the Sun Stone, use the Sun Stone to kill the Witch, use the Lasso to ride the Swan, and collect the Rose to survive lava.
- As a player, I can recover my dead brother's inventory by finding his bones in the world.
- As a player, if all three brothers perish, the game ends with a game-over message.
- As a player, each new brother can trigger a new princess rescue, advancing through Katra, Karla, and Kandy.
- As a player, I must visit the Spectre at night and trade the King's Bone for the Crystal Shard to navigate the Spirit Plane.

---


## 13. Doors & Buildings

### Requirements

| ID | Requirement |
|----|-------------|
| R-DOOR-001 | 86 door entries (`DOORCOUNT = 86`) shall be supported, each with outdoor coordinates (`xc1`, `yc1`), indoor coordinates (`xc2`, `yc2`), type (visual/orientation), and secs (1 = region 8 buildings, 2 = region 9 dungeons). The table shall be sorted by `xc1` ascending. |
| R-DOOR-002 | Outdoor-to-indoor transitions shall use O(log n) binary search on the X-sorted door table. Hero position shall be aligned to 16×32 tile grid (`xc1 = hero_x & 0xfff0`, `yc1 = hero_y & 0xffe0`) before lookup. |
| R-DOOR-003 | Indoor-to-outdoor transitions shall use O(n) linear scan matching on `xc2`/`yc2` with a wider hit zone for horizontal doors. |
| R-DOOR-004 | Door orientation: odd type = horizontal, even type = vertical. Horizontal doors shall skip entry if `hero_y & 0x10` is set. Vertical doors shall skip entry if `(hero_x & 15) > 6`. |
| R-DOOR-005 | Locked doors (terrain tile type 15) shall be opened using the `open_list[17]` table matching sector tile ID, region image block, and required key type. |
| R-DOOR-006 | DESERT door type shall be blocked unless `stuff[STATBASE] >= 5` (5 gold statues). |
| R-DOOR-007 | The `xfer()` teleport function shall: adjust map scroll by the same delta as hero position, set hero position to destination, clear encounters, recalculate region from coordinates (on exit only), load region data, regenerate minimap, force full screen redraw, update music mood, and nudge hero downward if colliding with a solid object at destination. |
| R-DOOR-008 | Players shall not be able to enter doors while mounted (`riding` check). |
| R-DOOR-009 | Door entry destination offsets shall vary by type: CAVE/VLOG → (`xc2 + 24`, `yc2 + 16`); horizontal → (`xc2 + 16`, `yc2`); vertical → (`xc2 - 1`, `yc2 + 16`). |
| R-DOOR-010 | Door exit destination offsets shall vary by type: CAVE/VLOG → (`xc1 - 4`, `yc1 + 16`); horizontal → (`xc1 + 16`, `yc1 + 34`); vertical → (`xc1 + 20`, `yc1 + 16`). |
| R-DOOR-011 | Entering a building shall use a visual fade transition (`fade_page(100,100,100,TRUE,pagecolors)`). Exiting shall be instant (no fade). |
| R-DOOR-012 | Quicksand-to-dungeon transition: when `environ == 30` at `hero_sector == 181`, the player shall teleport to `(0x1080, 34950)` in region 9. NPCs in the same quicksand shall die. |
| R-DOOR-013 | Door tile changes from `doorfind()` shall be transient — they modify live `sector_mem` only. Changes are lost when the sector reloads from disk. No save mechanism shall preserve opened door tiles. |
| R-DOOR-014 | The `doorfind()` algorithm shall: locate terrain type 15 at 3 X-offsets (x, x+4, x−8), find the top-left corner by scanning left (up to 32px) and down (32px), convert to image coordinates (`x >>= 4; y >>= 5`), determine sector/region IDs, search `open_list[17]` for a matching entry (map_id and door_id, with key check `keytype == 0 || keytype == open_list[j].keytype`), and replace tiles on success or print "It's locked." on failure (suppressed by `bumped` flag). |
| R-DOOR-015 | Collision-triggered door opening: when the player bumps terrain type 15, `doorfind(xtest, ytest, 0)` shall be called automatically, opening only NOKEY doors. |
| R-DOOR-016 | CAVE and VLOG door types shall share value 18; code checking for CAVE shall also match VLOG entries. Both shall use the same teleportation offset. |
| R-DOOR-017 | Key types for locked doors: NOKEY (0), GOLD (1), GREEN (2), KBLUE (3), RED (4), GREY (5), WHITE (6). Keys shall be consumed on successful use (`stuff[hit + KEYBASE]--`). |

### User Stories

- As a player, I can enter buildings through doors and transition seamlessly between outdoor and indoor areas.
- As a player, I need specific colored keys to open locked doors, and the key is consumed on use.
- As a player, I cannot enter the desert city door until I have collected 5 golden statues.
- As a player, I must dismount my carrier before I can enter any building.
- As a player, I can bump into unlocked doors to open them automatically.
- As a player, sinking fully in quicksand at the correct location teleports me to the dungeon.

---


## 14. Day/Night Cycle & Clock

### Requirements

| ID | Requirement |
|----|-------------|
| R-CLOCK-001 | `daynight` shall be a 16-bit unsigned counter cycling from 0 to 23999, incrementing by 1 per non-scrolling game tick. Full cycle = 24000 ticks (≈ 6.7 minutes at 60 Hz in the original game). |
| R-CLOCK-002 | `lightlevel` shall be a triangle wave: `daynight / 40`; if ≥ 300, then `600 − value`. Range: 0 (midnight) to 300 (midday). |
| R-CLOCK-003 | `dayperiod` shall be `daynight / 2000` (values 0–11). Time-of-day events shall trigger at period transitions: 0 = midnight event(28), 4 = morning event(29), 6 = midday event(30), 9 = evening event(31). Periods 1–3, 5, 7–8, 10–11 are silent. |
| R-CLOCK-004 | Time shall not advance during freeze spells (`freeze_timer > 0`). |
| R-CLOCK-005 | During sleep: `daynight += 63` per tick (plus normal +1 = 64 effective advance per tick). |
| R-CLOCK-006 | Spectre visibility shall switch at `lightlevel < 40` (deep night, daynight < 1600 or > 22400): `ob_listg[5].ob_stat = 3` (visible/interactive); otherwise `ob_stat = 2` (hidden). |
| R-CLOCK-007 | Outdoor palette shall be faded via `day_fade()` every 4 ticks (`(daynight & 3) == 0`) or during screen rebuild (`viewstatus > 97`), using `lightlevel` to scale RGB components with clamping (red min 10 max 100, green min 25 max 100, blue min 60 max 100). |
| R-CLOCK-008 | Indoor areas (`region_num >= 8`) shall use fixed full brightness `(100, 100, 100)` with no day/night palette variation. |
| R-CLOCK-009 | Color 31 (sky) shall be overridden per region: desert (region 4) = `0x0980`, dungeon (region 9) with `secret_timer` active = `0x00f0`, dungeon normal = `0x0445`, all others = `0x0bdf`. |
| R-CLOCK-010 | Green Jewel light effect (`light_timer > 0`) shall add 200 to the red channel parameter in `day_fade()` and boost per-pixel red to match green when red < green, producing a warm amber glow. |
| R-CLOCK-011 | Nighttime vegetation boost: palette colors 16–24 shall receive extra blue at twilight (green 21–49: +2 blue; green 50–74: +1 blue). Blue shift factor: `(100 − green) / 3`. |
| R-CLOCK-012 | Music mood shall transition between day and night themes at `lightlevel` threshold 120 (day > 120, night ≤ 120), corresponding to daynight ≈ 4800 (dawn) and ≈ 19200 (dusk). |
| R-CLOCK-013 | Encounter spawn rate shall NOT vary with time of day; `danger_level` depends on region and extent type, not `lightlevel`. |
| R-CLOCK-014 | Innkeeper lodging dialogue shall trigger when `dayperiod > 7` (evening/night). |
| R-CLOCK-015 | `revive()` shall initialize `daynight = 8000` (morning) and `lightlevel = 300`. |

### User Stories

- As a player, I experience a day/night cycle that affects visibility, palette coloring, and music.
- As a player, sleeping advances time rapidly until morning or I'm rested.
- As a player, spectres are only visible during the deepest part of night.
- As a player, indoor areas have constant lighting unaffected by time of day.
- As a player, the Green Jewel spell provides a warm amber glow in dark outdoor areas.

---


## 15. Survival (Hunger, Fatigue, Health)

### Requirements

| ID | Requirement |
|----|-------------|
| R-SURV-001 | Hunger shall increment by 1 every 128 game ticks (`(daynight & 127) == 0`) while alive and not sleeping. |
| R-SURV-002 | Hunger warnings: event(0) "getting rather hungry" at hunger == 35, event(1) "getting very hungry" at hunger == 60. Starvation warning event(2) "starving!" when hunger > 90 and `(hunger & 7) == 0`. |
| R-SURV-003 | Vitality −2 when `(hunger & 7) == 0` and (`hunger > 100` OR `fatigue > 160`), only when `vitality > 5`. |
| R-SURV-004 | Collapse at hunger > 140: event(24) "passed out!", hunger reset to 130, forced `state = SLEEP`. |
| R-SURV-005 | Fatigue shall increment by 1 on the same 128-tick timer as hunger, same conditions. |
| R-SURV-006 | Fatigue warnings: event(3) "getting tired" at fatigue == 70, event(4) "getting sleepy" at fatigue == 90. |
| R-SURV-007 | Forced fatigue sleep: event(12) when fatigue > 170, only when `vitality ≤ 5`. |
| R-SURV-008 | Health regeneration: +1 vitality every 1024 ticks (`(daynight & 0x3FF) == 0`), up to max vitality = `15 + brave / 4`. During sleep, `daynight` advances 64× faster, so healing occurs ≈63× faster. |
| R-SURV-009 | Voluntary sleep: stand on bed tile (IDs 161, 52, 162, 53) in region 8. `sleepwait` increments each tick; after 30 ticks: fatigue < 50 → event(25) "not sleepy"; fatigue ≥ 50 → event(26) "decided to lie down", `state = SLEEP`. |
| R-SURV-010 | Sleep processing: `daynight += 63` per frame, `fatigue--` per frame (if > 0). Wake conditions (any): fatigue == 0, OR (fatigue < 30 AND daynight ∈ [9000, 10000)), OR (`battleflag` AND `rand64() == 0`). On wake: `state = STILL`, Y-position snapped to grid. |
| R-SURV-011 | Safe zone detection: updated every 128 ticks when no enemies visible/loading, no witch encounter, `environ == 0`, no danger flag, hero alive. |
| R-SURV-012 | Auto-eat: in safe zone when `(daynight & 127) == 0`, if hunger > 30 and `stuff[24] > 0` (Fruit), consume one Fruit: `stuff[24]--; hunger -= 30; event(37)`. Direct subtraction, not via `eat()`. |
| R-SURV-013 | Hunger > 120 movement wobble: direction shifted ±1 with 75% probability (`rand4() != 0` selects wobble; `rand2()` selects ±1 direction). |
| R-SURV-014 | `eat(amt)` function: `hunger -= amt`; if hunger < 0, set to 0 and event(13) "full"; otherwise print "Yum!". Pickup fruit (hungry): `eat(30)`. Buy food from shop: `eat(50)`. |
| R-SURV-015 | Fruit pickup when hunger < 15: fruit stored in inventory (`stuff[24]++; event(36)`) rather than eaten. |
| R-SURV-016 | Drowning damage (`environ == 30`): −1 vitality every 8 ticks. |
| R-SURV-017 | Lava damage zone (`8802 < map_x < 13562`, `24744 < map_y < 29544`): `environ > 2` → −1 vitality per tick; `environ > 15` → instant death. Rose (`stuff[23]`) prevents lava damage by forcing `environ = 0`. |
| R-SURV-018 | Heal vial: `vitality += rand8() + 4` (4–11 HP), capped at `15 + brave / 4`. |
| R-SURV-019 | Priest healing: full heal to `15 + brave / 4`. Requires `kind >= 10`; below 10, priest gives dismissive dialogue. |
| R-SURV-020 | Bravery: +1 per enemy kill, −1 per Jade Skull kill. Affects combat: melee hit range = `brave / 20 + 5`, missile bravery = full `brave`, enemy dodge check = `rand256() > brave`, max HP = `15 + brave / 4`. Initial: Julian = 35, Phillip = 20, Kevin = 15. |
| R-SURV-021 | Luck: −5 on hero death, −2 on pit fall. Probabilistic +5 from sorceress: `if (luck < rand64()) luck += 5`. Clamped ≥ 0 on HUD redraw. Luck < 1 after death triggers brother succession instead of fairy rescue. Initial: Julian = 20, Phillip = 35, Kevin = 20. |
| R-SURV-022 | Kindness: −3 for killing non-witch SETFIGs, clamped ≥ 0. Probabilistic +1 from giving gold: `if (rand64() > kind) kind++`. Below 10, wizards and priests give dismissive dialogue. Initial: Julian = 15, Phillip = 15, Kevin = 35. |
| R-SURV-023 | Wealth: +50 from gold bags, +100 from containers, +100 from princess rescue, +variable from corpse loot. −price for shop purchases, −2 for giving gold. Initial: Julian = 20, Phillip = 15, Kevin = 10. |
| R-SURV-024 | HUD shall display Brv, Lck, Knd, Wlth (via prq(7)) and Vit (via prq(4)). Hunger and fatigue are NOT displayed on the HUD — communicated only through event messages. |

### User Stories

- As a player, I must manage hunger and fatigue to avoid collapsing.
- As a player, I can sleep in beds to recover fatigue.
- As a player, I am warned progressively as hunger and fatigue increase.
- As a player, my health regenerates slowly over time, faster while sleeping.
- As a player, extreme hunger causes my character to stumble while walking.
- As a player, I can eat fruit to reduce hunger, and my character auto-eats in safe zones.
- As a player, my stats (bravery, luck, kindness, wealth) change through gameplay actions.
- As a player, hunger and fatigue are communicated through text messages, not HUD numbers.

---


## 16. Magic

### Requirements

| ID | Requirement |
|----|-------------|
| R-MAGIC-001 | 7 magic items shall be usable from the MAGIC menu, each requiring the item in inventory (`stuff[4 + hit] > 0`); otherwise event(21) "if only I had some Magic!". |
| R-MAGIC-002 | Magic shall be blocked in the Necromancer arena (extent `v3 == 9`): `speak(59)`. |
| R-MAGIC-003 | Blue Stone (`stuff[9]`, hit 5): teleport via stone ring network. Requires `hero_sector == 144`. Destination = `(current_stone + facing + 1) % 11`. |
| R-MAGIC-004 | Green Jewel (`stuff[10]`, hit 6): `light_timer += 760`. Outdoor-only warm amber tint via `day_fade()`. |
| R-MAGIC-005 | Glass Vial (`stuff[11]`, hit 7): heal `vitality += rand8() + 4` (4–11 HP), capped at `15 + brave / 4`. |
| R-MAGIC-006 | Crystal Orb (`stuff[12]`, hit 8): `secret_timer += 360`. Reveals hidden passages while active. In dungeons (region 9), color 31 turns bright green (`0x00f0`). |
| R-MAGIC-007 | Bird Totem (`stuff[13]`, hit 9): renders overhead map with hero position marker. Sets `viewstatus = 1`. |
| R-MAGIC-008 | Gold Ring (`stuff[14]`, hit 10): `freeze_timer += 100`. Freezes all enemies, stops daynight advance, suppresses encounters. Blocked when `riding > 1`. |
| R-MAGIC-009 | Jade Skull (`stuff[15]`, hit 11): kill all visible enemies with `vitality > 0`, `type == ENEMY`, `race < 7`. Brave −1 per kill (counterbalances normal combat brave++). |
| R-MAGIC-010 | After successful use: `stuff[4 + hit]--`. If depleted (reaches 0), rebuild menu via `set_options()`. Failed uses (wrong location, blocked) do NOT consume a charge. |

### User Stories

- As a player, I can use magic items from the menu to aid exploration and combat.
- As a player, magic items have limited charges that deplete with successful use.
- As a player, magic is blocked in the final boss arena.
- As a player, the Jade Skull kills enemies but reduces my bravery.

---


## 17. Death & Revival

### Requirements

| ID | Requirement |
|----|-------------|
| R-DEATH-001 | When any actor's vitality < 1 and state is not DYING/DEAD: set vitality = 0, tactic = 7, goal = DEATH, state = DYING. Death types: 5 = combat, 6 = drowning, 27 = lava. |
| R-DEATH-002 | Hero death effects: display death event message (by death type), `luck -= 5`, `setmood(TRUE)` (death music). |
| R-DEATH-003 | NPC kill effects: `brave++` for the attacker. If killed NPC is a SETFIG and not witch (0x89): `kind -= 3` (clamped ≥ 0). If DreamKnight (race 7): `speak(42)`. |
| R-DEATH-004 | DYING → DEAD transition shall occur when `tactic` counts down to 0 during the death animation. |
| R-DEATH-005 | `goodfairy` shall be a u8 countdown from 255 after hero enters DEAD or FALL state. The death animation and death song always play fully (frames 2–57) before any rescue decision. |
| R-DEATH-006 | Fairy rescue luck gate at `goodfairy` range 199–120: if `luck < 1` → brother succession (`revive(true)`). If FALL state → fairy recovery (`revive(false)`) regardless of luck. If `luck >= 1` and DEAD → fairy rescue proceeds. This gate is fully deterministic with no random element. |
| R-DEATH-007 | Luck cannot change during DEAD state: `checkdead` is guarded against DYING/DEAD states, pit fall requires movement, sorceress requires TALK. If luck ≥ 1 when the gate fires, fairy rescue is guaranteed. |
| R-DEATH-008 | Fairy animation at `goodfairy` 119–20 (fairy sprite approaches hero, `battleflag = FALSE`, AI suspended), resurrection glow at 19–2, revival `revive(false)` at `goodfairy == 1`. |
| R-DEATH-009 | Brother succession (`revive(true)`): place ghost at death location (brothers 1–2 only), reset `ob_list8[9].ob_stat = 3` (princess captive), load next brother stats from `blist[]`, clear inventory (zero 31 slots, give single Dirk), reset all timers to 0, spawn at Tambry (19036, 15755) in region 3, display brother-specific placard, load brother sprites, display journey message. |
| R-DEATH-010 | Fairy revival (`revive(false)`): teleport to last safe zone (`safe_x, safe_y`), full HP (`15 + brave / 4`), clear hunger/fatigue to 0, set `daynight = 8000`, `lightlevel = 300`. Skips ghost placement, stat/inventory reset, and placard text. |
| R-DEATH-011 | Brother base stats: Julian (brave=35, luck=20, kind=15, wealth=20, HP=23), Phillip (brave=20, luck=35, kind=15, wealth=15, HP=20), Kevin (brave=15, luck=20, kind=35, wealth=10, HP=18). Each has an independent 35-byte inventory array. |
| R-DEATH-012 | Max fairy rescues per brother (from initial luck / 5): Julian = 3, Phillip = 6, Kevin = 3. |
| R-DEATH-013 | Succession placard text: Julian → placard(0); Phillip → placard(1) + placard(2) ("Julian's luck ran out…"); Kevin → placard(3) + placard(4) ("Phillip's cleverness could not save him…"). Journey start: event(9), plus event(10) for Phillip or event(11) for Kevin. |
| R-DEATH-014 | Dead brother ghost: bones at death location (`ob_listg[brother]`), ghost setfig activated (`ob_listg[brother + 2].ob_stat = 3`). Only for brothers 1 and 2 (Kevin has no successor). |
| R-DEATH-015 | Bones pickup (ob_id 28): clear both ghost setfigs (`ob_listg[3].ob_stat = ob_listg[4].ob_stat = 0`), merge dead brother's 31-slot inventory into current brother. Ghost dialogue before pickup: `speak(49)`. |
| R-DEATH-016 | Game over when `brother > 3`: placard(5) "And so ends our sad tale. The Lesson of the Story: Stay at Home!", 10-second pause, `quitflag = TRUE`. |
| R-DEATH-017 | Persistence across succession: princess counter, quest flags, world object state, `dstobs[]` persist. Stats, inventory, position, hunger/fatigue, timers, daynight all reset. Princess captive flag (`ob_list8[9].ob_stat`) resets to 3 enabling each brother to trigger a rescue. |

### User Stories

- As a player, when I die, a fairy may revive me depending on my luck stat.
- As a player, if too unlucky, my next brother takes over the quest from the village.
- As a player, if all three brothers die, the game ends with a "Stay at Home" message.
- As a player, killing innocent NPCs reduces my kindness stat.
- As a player, I can find a dead brother's bones and recover their inventory.
- As a player, each brother starts fresh with different strengths but the quest state is preserved.
- As a player, the fairy rescue is deterministic — if I have luck remaining, the fairy always saves me.

---


## 18. Carriers (Raft, Turtle, Bird)

### Requirements

| ID | Requirement |
|----|-------------|
| R-CARRY-001 | Four carrier types shall be implemented: Raft (`riding=1`, RAFT type, cfiles[4]), Turtle (`riding=5`, CARRIER type, cfiles[5]), Swan (`riding=11`, CARRIER type, cfiles[11]), Dragon (DRAGON type, cfiles[10], hostile, not rideable). |
| R-CARRY-002 | Raft shall activate automatically when within 9px proximity, hero on water/shore terrain (px_to_im 3–5), and `wcarry==1`. It shall snap to hero position each frame. Dismount occurs automatically when conditions fail. |
| R-CARRY-003 | Turtle shall be summoned via USE menu (turtle item, `stuff[6]`). Boarding requires within 16px proximity and `wcarry==3`. Ridden speed shall be forced to 3 pixels/frame. Dismount occurs when proximity is lost. |
| R-CARRY-004 | Turtle cannot be summoned in central region bounds (11194–21373 X, 10205–16208 Y). |
| R-CARRY-005 | When unridden, turtle shall move autonomously on water-only terrain (px_to_im == 5) using directional probing (try current direction, then ±1, then −2). |
| R-CARRY-006 | Swan shall require Golden Lasso (`stuff[5]`) to board. Boarding requires within 16px proximity and `wcarry==3`. Sets `riding=11`, `environ=-2` (airborne). |
| R-CARRY-007 | Swan movement shall use inertial flight physics: velocity accumulates via directional acceleration, max horizontal velocity ~32, max vertical ~40, position updates by `vel/4` per frame. No terrain collision (`proxcheck` skipped). |
| R-CARRY-008 | Swan dismount shall require: hero action button + velocity < 15 + clear ground below + not fiery terrain. Dismount blocked in lava zone (event 32: "Ground is too hot") and at high velocity (event 33: "Flying too fast"). |
| R-CARRY-009 | Swan on ground shall render using RAFT sprite. Auto-face into wind via `set_course(0,-nvx,-nvy,6)`. |
| R-CARRY-010 | Dragon shall be stationary, shoot fire missiles (type 2) with 25% chance per frame at speed 5, always face south. HP: 50, can be killed. |
| R-CARRY-011 | Carriers and enemies shall share the ENEMY shape memory slot — they cannot coexist. Carriers always occupy `anim_list[3]`. Loading sets `anix=4`. |
| R-CARRY-012 | While riding: door entry blocked, random encounters suppressed, carriers skip melee/missile hit detection, freeze spell blocked when `riding > 1`. |
| R-CARRY-013 | Stone circle teleport shall move carrier with hero. |
| R-CARRY-014 | Carriers shall skip terrain masking during rendering. |
| R-CARRY-015 | Mounted-turtle melee recoil can push the rider across invalid terrain (original behavior — do not fix). |

### User Stories

- As a player, I can ride a raft across water by walking near it on shore.
- As a player, I can summon and ride a turtle to navigate ocean areas.
- As a player, I can lasso and ride a swan to fly over any terrain.
- As a player, I encounter a hostile dragon that breathes fire.
- As a player, I cannot enter doors or use freeze while riding.

---


## 19. Audio

### Requirements

| ID | Requirement |
|----|-------------|
| R-AUDIO-001 | A 4-voice music tracker shall be driven by a 60 Hz VBL interrupt. `timeclock` shall be incremented by `tempo` each VBL frame, even when `nosound` is set (it doubles as a general-purpose timer). |
| R-AUDIO-002 | 8 waveforms (128-byte single-cycle 8-bit signed PCM) and 10 volume envelopes (256-byte) shall be loaded from the `v6` file. Waveforms = 1024 bytes at offset 0, envelopes = 2560 bytes at offset 1024. |
| R-AUDIO-003 | 28 music tracks (7 moods × 4 channels) shall be loaded from the `songs` file. |
| R-AUDIO-004 | Track data format: 2-byte command/value pairs — notes 0–127 (with TIE/CHORD bits), rest $80, set instrument $81, set tempo $90, end $FF (value 0=stop, nonzero=loop). |
| R-AUDIO-005 | Duration table: `notevals` 8×8 table of SMUS-standard timing values. Articulation gap of 300 counts subtracted from note duration. |
| R-AUDIO-006 | Period table: 84 entries (7 octaves × 12 notes). NTSC base: Hz = 3,579,545 / period. Higher octaves offset into waveform to shorten effective sample. |
| R-AUDIO-007 | 12-entry instrument table (`new_wave`) mapping instrument numbers to waveform/envelope pairs. Entry 10 modified at runtime for underworld region by `setmood()`. |
| R-AUDIO-008 | Per-voice state (4 × 28 bytes): `wave_num`, `vol_num`, `vol_delay`, `vce_stat`, `event_start/stop`, `vol_list`, `trak_ptr/beg/stk`. |
| R-AUDIO-009 | `vce_stat` bit flags: TIE=4, CHORD=8, REST=16, ENDTK=32. On voice 2, `vce_stat` doubles as sample-completion countdown. |
| R-AUDIO-010 | Music player shall skip any voice where `vce_stat != 0`, yielding to sample playback on voice 2. |
| R-AUDIO-011 | 7 musical moods evaluated by priority: Death (vitality==0, offset 24), Zone (specific map zone, offset 16), Battle (battleflag, offset 4), Dungeon (region>7, offset 20), Day (lightlevel>120, offset 0), Night (fallback, offset 8), Intro (startup, tracks 12–15). |
| R-AUDIO-012 | `setmood(TRUE)` / `playscore()` shall restart playback immediately. `setmood(FALSE)` / `setscore()` shall defer change until current tracks loop. |
| R-AUDIO-013 | 6 sound effect samples loaded from disk sectors 920–930, played on voice 2 via `playsample()`. Temporarily overrides music on that voice. |
| R-AUDIO-014 | Sound effect playback shall be gatable via the Sound menu toggle. `effect(num, speed)` checks toggle before calling `playsample()`. |
| R-AUDIO-015 | Sample completion on voice 2: `vce_stat` set to 2, audio interrupt handler decrements per interrupt, voice silenced when counter reaches 0, music resumes. |
| R-AUDIO-016 | Envelope processing: each byte is a volume level per VBL tick. Bit 7 set means "hold current volume". |

### User Stories

- As a player, I hear music that changes based on time of day, combat, location, and death.
- As a player, I hear sound effects for combat, interactions, and environmental events.
- As a player, I can toggle music and sound effects independently.

---


## 20. Intro & Narrative

### Requirements

| ID | Requirement |
|----|-------------|
| R-INTRO-001 | Intro sequence shall follow this order: legal text (title text on dark blue) → 1s pause → load audio (music + samples) → start intro music (tracks 12–15) → load title image (`page0`) blitted to both pages → vertical zoom-in (0→160 in steps of 4 via `screen_size()`) → 3 story pages with columnar-reveal animation (`copypage` with `flipscan`) → final pause (3.8s) → vertical zoom-out (156→0 in steps of −4) → copy protection. |
| R-INTRO-002 | Copy protection: 3 random questions from 8 rhyming-word pairs. Case-sensitive, prefix-only comparison. After correct answer, entry nulled to prevent repeats. First question deterministic from initial RNG seed. |
| R-INTRO-003 | Complete answer table: 0="LIGHT", 1="HEED", 2="DEED", 3="SIGHT", 4="FLIGHT", 5="CREED", 6="BLIGHT", 7="NIGHT". |
| R-INTRO-004 | Disk timestamp check (`cpytest`): validates magic value 230. Floppy path via `FileLock→DeviceList→dl_VolumeDate.ds_Tick`. Hard drive path reads block 880, checks `buffer[123]`. |
| R-INTRO-005 | `NO_PROTECT` compile flag disables riddle comparison and floppy timestamp check. Hard drive block-880 check always executes. |
| R-INTRO-006 | 39 event messages shall display during gameplay via `event(n)` with `%` substitution for the current brother name. |
| R-INTRO-007 | 29 outdoor place names and 31 indoor place names shall trigger on sector entry via `find_place()` (first-match linear scan of sector-range tables). Mountain messages (index 4) vary by region. |
| R-INTRO-008 | 20 placard/story messages via `placard_text(n)` using `ssp()` renderer with embedded XY positioning (byte 128 + x_half + y, X doubled during rendering). |
| R-INTRO-009 | Line width constraints: max 36 chars for scroll text, 29 for placard text. |
| R-INTRO-010 | Player may skip the intro at multiple checkpoints. |
| R-INTRO-011 | `placard()` visual effect: recursive fractal line pattern on `rp_map` using `xmod`/`ymod` offset tables (±4 pixel deltas), mirror-symmetric with center at (284,124), 16×15 outer iterations with 5 inner passes, color 1 for most lines, color 24 for first inner pass. |

### User Stories

- As a player, I see the original intro sequence with title zoom, story pages, and music.
- As a player, I can skip the intro to get into the game quickly.
- As a player, I see location names when entering named areas.
- As a player, I see decorative placard borders during story sequences.

---


## 21. Save/Load

### Requirements

| ID | Requirement |
|----|-------------|
| R-SAVE-001 | 8 save slots named `A.faery` through `H.faery`. |
| R-SAVE-002 | Save format: raw sequential binary dump — no headers, no version field, no checksums. Written in big-endian (68000) byte order. |
| R-SAVE-003 | Save data blocks in order: 80-byte misc vars (map_x through pad7), 2-byte region_num, 6-byte anim indices (anix, anix2, mdex), `anix × 22` byte anim_list, 35-byte Julian inventory, 35-byte Phillip inventory, 35-byte Kevin inventory, 60-byte missiles (6 × 10), 24-byte extents (bird/turtle positions), 66-byte global objects (11 × 6), 20-byte map object counts, 20-byte distributed flags, variable per-region object tables. |
| R-SAVE-004 | Typical save file size: ~1,200–1,500 bytes. |
| R-SAVE-005 | Post-load cleanup: clear encounter_number/wt/actors_loading/encounter_type to 0, set viewstatus=99 (force full redraw), reload sprites via `shape_read()`, rebuild menu states via `set_options()`. |
| R-SAVE-006 | **Persisted**: hero position, stats (brave/luck/kind/wealth/hunger/fatigue), all 3 brothers' inventories, daynight cycle, active actors, missiles, world objects (global + all 10 regions), bird/turtle extent positions, carrier state, cheat flag. |
| R-SAVE-007 | **Not saved**: display state (copper lists, rendering buffers), input handler state, music playback position, extent entries 2–21 (static initializers), viewstatus, battleflag, goodfairy. |
| R-SAVE-008 | Save/load accessible from GAME → SAVEX → FILE menu chain. `svflag` determines save vs. load mode. |

### User Stories

- As a player, I can save my game to one of 8 slots and resume later.
- As a player, loading a save restores the complete game state including position, inventory, and quest progress.
- As a player, I see the game properly reinitialize display and encounters after loading.

---


## 22. UI & Menus

### Requirements

| ID | Requirement |
|----|-------------|
| R-UI-001 | 10 menu modes shall be supported: ITEMS(0), MAGIC(1), TALK(2), BUY(3), GAME(4), SAVEX(5), KEYS(6), GIVE(7), USE(8), FILE(9). |
| R-UI-002 | Modes 0–4 (ITEMS through GAME) share a top bar of 5 entries from `label1` ("Items Magic Talk Buy  Game"). Entries 5+ come from each menu's own `label_list`. USE and FILE skip the top bar. |
| R-UI-003 | `enabled[i]` byte encoding: bit 0 = selected/highlighted, bit 1 = visible, bits 2–7 = action type (atype). atype values: 0=nav, 4=toggle, 8=immediate, 12=one-shot highlight. Common encoded: 2=visible nav, 3=visible+highlighted, 6=visible toggle off, 7=visible toggle on, 8=hidden action, 10=visible action. |
| R-UI-004 | `print_options()` renders on `rp_text2`: 2-column layout (x=430, x=482), 6 rows at 9px spacing starting at y=8. `real_options[12]` indirection array maps screen positions to actual enabled[] indices. |
| R-UI-005 | Background pen varies by mode: USE=14, FILE=13, top bar (k<5)=4, KEYS=`keycolors[k-5]` where `keycolors={8,6,4,2,14,1}`, SAVEX=entry index, others=`menus[cmode].color`. |
| R-UI-006 | `set_options()` shall dynamically update menu enabled states after every `do_option()` call based on inventory: MAGIC indices 5–11 from `stuff[9..15]`, USE indices 0–6 from `stuff[0..6]`, KEYS indices 5–10 from `stuff[16..21]`, USE Sun from `stuff[7]`, GIVE Gold if wealth>2. |
| R-UI-007 | `do_option()` dispatch shall handle all 10 modes with correct sub-actions: ITEMS (List/Take/Look/Use/Give), MAGIC (7 spells with guards), TALK (Yell/Say/Ask with NPC response dispatch), BUY (7 purchasable items with costs), GAME (Pause/Music/Sound/Quit/Load), SAVEX (Save/Exit), KEYS (6 key types with `doorfind`), GIVE (Gold/Book/Writ/Bone), USE (equip weapons/items), FILE (8 save slots). |
| R-UI-008 | `gomenu(mode)` shall be blocked if game is paused (checks `menus[GAME].enabled[5] & 1`). |
| R-UI-009 | 38 keyboard shortcuts via `letter_list[38]`: F1–F7 for magic spells, 1–7 for weapons, letters for actions. SAVEX guard: V and X blocked unless `cmode==SAVEX`. KEYS special: if `cmode==KEYS` and key '1'–'6', dispatch directly. |
| R-UI-010 | 8-direction compass at (567,15) on HUD: base compass (`hinor`, 48×24px) with highlighted direction overlay (`hivar`). Only bitplane 2 differs. Direction regions from `comptable[10]` (8 cardinal/ordinal rectangles + 2 null). |
| R-UI-011 | Stats display via print queue: `prq(7)` full stats at y=52 (Brv x=14, Lck x=90, Knd x=168, Wlth x=321), `prq(4)` vitality at (245,52). |
| R-UI-012 | Print queue: 32-entry circular buffer. `prq(n)` enqueues, `ppick()` dequeues one per call from Phase 14a. Commands: 2=debug coords, 3=debug position, 4=vitality, 5=refresh menu, 7=full stats, 10="Take What?". Empty queue yields to OS via `Delay(1)`. |
| R-UI-013 | Two fonts: Topaz 8 (ROM, for status/menu labels), Amber 9 (custom disk font from `fonts/Amber/9`, for scrolling messages and placard text). |
| R-UI-014 | Text rendering: `print(str)` scrolls up 10px then renders at (TXMIN,42). `print_cont(str)` appends without scroll. Bounds: TXMIN=16, TYMIN=5, TXMAX=400, TYMAX=44. Colors: pen 10 fg, pen 11 bg, JAM2 mode. |
| R-UI-015 | `extract()` template engine: word-wrap at 37 chars, `%` substitutes `datanames[brother-1]` (Julian/Phillip/Kevin), CR(13) forces line break, uses `mesbuf[200]` buffer. |
| R-UI-016 | `cheat1` debug flag: persisted in save files (offset 18 of 80-byte block), only enabled via hex-editing. Gates debug keys (B=summon swan, '.'=random item, R=rescue, '='=prq(2), teleport keys) and map spell region restriction. |

### User Stories

- As a player, I can navigate menus to manage inventory, use magic, talk to NPCs, buy items, and save/load.
- As a player, I can use keyboard shortcuts for quick access to common actions.
- As a player, I see my stats and compass direction on the HUD at all times.
- As a player, I see scrolling text messages for events and dialogue.
- As a player, I see location names when entering new areas.

---


## 23. Asset Loading

### Requirements

| ID | Requirement |
|----|-------------|
| R-ASSET-001 | The `image` file (901120 bytes, 1760 sectors × 512 bytes) shall be the primary data source for tilesets, terrain, sprites, shadow masks, and other binary assets. |
| R-ASSET-002 | `file_index[10]` (one per region) shall map regions to their 4 image bank sector addresses, 2 terrain table sector addresses, sector map start, region map start, and setfig character set ID. Each entry uses the `struct need` format: `image[4]`, `terra1`, `terra2`, `sector`, `region`, `setchar`. |
| R-ASSET-003 | `cfiles[18]` shall map sprite sets to disk sector addresses and dimensions (width in 16px units, height in pixels). |
| R-ASSET-004 | IFF/ILBM files (`page0`, `p1a`–`p3b`, `hiscreen`, `winpic`) shall be loaded with chunk parsing for FORM, ILBM, BMHD, and BODY. The CMAP chunk shall be skipped — the game uses hardcoded programmatic palettes, not embedded palette data. ByteRun1 RLE decompression shall handle control byte N ≥ 0 (copy N+1 literal bytes), N < 0 and ≠ −128 (repeat next byte 1−N times), and −128 (no-op). |
| R-ASSET-005 | Font (Amber 9pt) shall be loaded from hunk-format file `fonts/Amber/9`. The ROM font Topaz 8 is used for status bar and menu text. |
| R-ASSET-006 | Audio data: `v6` file contains waveforms (1024 bytes, 8 × 128-byte waveforms) + volume envelopes (2560 bytes, 10 × 256-byte envelopes); `songs` file contains 28 music tracks (7 songs × 4 channels); sound effect samples loaded from `image` sectors 920–930 (5632 bytes, 6 samples). |
| R-ASSET-007 | Region loading shall load 4 image banks (each 40 sectors = 20480 bytes), 2 terrain tables (each 512 bytes), sector map (32768 bytes), and region map (4096 bytes = 8 sectors), updating the minimap and performing any format conversion needed for display. |
| R-ASSET-008 | Shadow mask data (12288 bytes, 24 sectors from sectors 896–919) shall be loaded into `shadow_mem` for terrain occlusion during sprite compositing. |
| R-ASSET-009 | `shape_mem` (78000 bytes) shall be used as a temporary decompression buffer during IFF BODY loading, since shape loading and IFF image loading never overlap. |

### User Stories

- As a player, the game loads all regions, images, music, and fonts without errors from the original data files.
- As a player, story page images and the victory image display correctly using ByteRun1 decompressed IFF data.

---


## 24. Special Effects

### Requirements

| ID | Requirement |
|----|-------------|
| R-FX-001 | Witch vision cone: a rotating filled wedge-shaped polygon (~11.25° arc) rendered in COMPLEMENT (XOR) mode around the witch position. Endpoints looked up from `witchpoints[256]` (two concentric circles, radii ~10 and ~100 pixels). `witchindex` (u8, 0–255) advances each frame by `wdir` (±1), completing a full rotation over 256 frames. Steering adjusts via cross-product sign, gated by `rand4() == 0` (1-in-4 frames). Hero hit detection: cross-product test within wedge AND distance < 100 pixels, dealing 1–2 HP damage. |
| R-FX-002 | Teleport colorplay: 32 frames of randomized 12-bit RGB values for palette entries 1–31 (preserving entry 0, the background color), creating a psychedelic flash effect lasting ≈ 0.5 seconds. |
| R-FX-003 | Columnar page reveal (`flipscan`): 22-step vertical strip animation for story page transitions — steps 0–10 sweep the right half, steps 11–21 sweep the left half, with per-step timing from the `flip3[]` delay table. Each step performs a page swap for intermediate display. |
| R-FX-004 | Victory sunrise (`win_colors`): 55-step palette fade (index 25 down to −29) using `sun_colors[53]`. Colors 0/31 always black, colors 1/28 always white, colors 2–27 swept from deep blue/black through purple/red to golden tones. Colors 29–30 use red computations. First frame holds 60 ticks (~1 second), subsequent frames 9 ticks (~150 ms), final hold 30 ticks. |
| R-FX-005 | Screen fade-down (`fade_down`): 21 steps from 100% to 0% in decrements of 5, with `Delay(1)` per step and `limit=FALSE` (no night clamping). Screen fade-up (`fade_normal`): 21 steps from 0% to 100% in increments of 5. Both fade the entire palette to/from black. |
| R-FX-006 | Flasher border blink: during dialogue mode (`viewstatus == 1`), color register 31 shall blink white↔black every 16 frames (~0.27 seconds), toggled by bit 4 of the `flasher` counter (which increments each main-loop tick). |
| R-FX-007 | Viewport zoom (`screen_size`): animates viewport dimensions from a point to full screen using 5:8 aspect ratio (y = x × 5 / 8). Normal gameplay uses `screen_size(156)`, yielding a 312×194 viewport slightly inset from the 320×200 frame. Intro sequence reaches `screen_size(160)` for full-screen. |
| R-FX-008 | Full-screen message transitions: `map_message()` fades down, clears playfield, hides status bar, enables drawing on the playfield with pen 24 in JAM1 mode, and sets `viewstatus = 2`. `message_off()` fades down, restores status bar, flips page, and sets `viewstatus = 3`, triggering `fade_normal()` on the next frame. |
| R-FX-009 | Static display reset (`stillscreen`): resets scroll offsets to (0, 0) and flips the page, used for non-scrolling display modes. |
| R-FX-010 | Placard border (`placard`): a recursive fractal line pattern drawn on the playfield using `xmod`/`ymod` offset tables (±4 pixel deltas). The pattern is mirror-symmetric about center (284, 124) with 90°/270° rotations, using 16×15 outer iterations with 5 inner passes. Color 1 for most lines, color 24 for the first inner pass. |

### User Stories

- As a player, I see the witch's spinning vision cone that damages me when caught.
- As a player, I see dramatic palette effects during teleportation, story transitions, and the victory ending.
- As a player, I see a blinking border/prompt effect during dialogue mode.
- As a player, I see smooth viewport zoom-in during the intro sequence.
- As a player, I see decorative fractal borders on story placard screens.

---

## Traceability Matrix

Each requirement ID maps to its specification section:

| Requirement Prefix | Specification Section |
|--------------------|----------------------|
| R-DISP | §1 Display & Rendering |
| R-WORLD | §2 World Structure |
| R-SCROLL | §4 Scrolling & Camera |
| R-SPRITE | §5 Sprite System |
| R-ZMASK | §6 Terrain Masking & Z-Sorting |
| R-FADE | §7 Color Palettes & Day/Night Fading |
| R-INPUT | §9 Player Movement & Input |
| R-COMBAT | §10 Combat System |
| R-AI | §9, §11, §12 AI, Behavior, Encounters |
| R-NPC | §13 NPCs & Dialogue |
| R-INV | §14 Inventory & Items |
| R-QUEST | §15 Quest System |
| R-DOOR | §16 Doors & Buildings |
| R-CLOCK | §17 Day/Night Cycle |
| R-SURV | §18 Survival Mechanics |
| R-MAGIC | §19 Magic System |
| R-DEATH | §20 Death & Revival |
| R-CARRY | §21 Carriers |
| R-AUDIO | §22 Audio System |
| R-INTRO | §23 Intro & Narrative |
| R-SAVE | §24 Save/Load System |
| R-UI | §25 UI & Menu System |
| R-ASSET | §26 Asset Formats |
| R-FX | §27 Special Effects |