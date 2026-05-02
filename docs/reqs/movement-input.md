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
| R-INPUT-009 | Walk speed shall vary by terrain using a single if/else chain applied to **all actors** (hero and NPC share the code path): default = 2, slippery (environ −1) = 4, wading/deep water (environ 2 or > 6) = 1, direction reversal (environ −3) = −2. Exceptions: (a) `i == 0 && riding == 5` forces speed 3 (hero-only turtle mount); (b) ice (environ −2) uses velocity physics — see R-INPUT-010. In practice only the hero reaches environ −3 because NPCs are blocked from terrain 8 by `proxcheck` (see R-INPUT-012). |
| R-INPUT-009a | NPC movement shall be skipped entirely (`goto statc`) when `freeze_timer > 0`. The hero is unaffected by `freeze_timer`. |
| R-INPUT-009b | Race-based terrain immunity: wraiths (race 2) skip terrain collision entirely; wraiths and snakes (race 4) have their terrain forced to 0, so they always use speed 2 regardless of actual terrain. |
| R-INPUT-010 | Ice terrain (environ −2) shall use velocity-based physics: `vel += dir_vector[dir]` each tick, velocity clamped to magnitude 42 (40 on swan), position updated by `vel / 4`. Facing shall be derived from velocity via `set_course(0, −vel_x, −vel_y, 6)`. |
| R-INPUT-011 | When hunger > 120, each walking tick shall have a 1/4 chance (`!rand4()`) of deflecting the direction by ±1 (50/50 via `rand() & 1`), wrapped to 0–7. |
| R-INPUT-012 | Movement shall be blocked by terrain types returned by the dual-probe `_prox` function: probe at (x+4, y+2) blocks at type 1 or ≥ 10; probe at (x−4, y+2) blocks at type 1 or ≥ 8. Wraiths (race 2) skip terrain checks entirely. The hero (only) may enter terrain types 8 and 9 — NPCs are blocked by the second probe's ≥ 8 threshold, so only the hero ever experiences direction-reversal (terrain 8) or pit-fall (terrain 9) effects. |
| R-INPUT-013 | The hero (index 0) shall treat terrain types 8 and 9 as passable (they cause effects but do not block movement). |
| R-INPUT-014 | If the hero has the crystal shard (`stuff[30]`), terrain type 12 shall be treated as passable. Terrain type 12 exists only in terra set 8 (Region 8 building interiors). |
| R-INPUT-015 | Collision deviation runs for **all** actors: when movement is terrain-blocked, try `dir + 1` (clockwise), then `dir − 2` (counterclockwise). If all three directions are blocked, behavior diverges for the player vs. NPCs. **Player** (`i == 0`): an escalating frustration counter tracks consecutive fully-blocked ticks and drives a visual cue. The counter MUST read 0 whenever any enemy NPC is active in the player's current encounter/region, so the animation never plays during combat (matching the original's global-reset side effect via simpler means). Otherwise, the counter increments on full block and resets on the player's own successful walk. Escalating player-only animation: counter 0–20 — normal standing sprite; 21–40 — head-shaking oscillation sprites 84/85 alternating every 2 game cycles (`dex = 84 + ((cycle >> 1) & 1)`, figures 64/65); 41+ — fixed `dex = 40` (figure 35, south-facing pose), with the player sprite snapping to face south regardless of input facing. NPC frustration behavior is covered by R-AI-016. |
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


