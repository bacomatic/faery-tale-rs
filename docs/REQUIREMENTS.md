# The Faery Tale Adventure — Requirements & User Stories

> Derived from [RESEARCH.md](RESEARCH.md), [ARCHITECTURE.md](ARCHITECTURE.md), and [STORYLINE.md](STORYLINE.md).
> Target: Rust/SDL2 reimplementation faithful to the 1987 Amiga original.

Each requirement is a testable statement. User stories follow the format:
*"As a player, I can [action] so that [outcome]."*

---

## Table of Contents

1. [Display & Rendering](#1-display--rendering)
2. [World & Map](#2-world--map)
3. [Scrolling & Camera](#3-scrolling--camera)
4. [Sprites & Animation](#4-sprites--animation)
5. [Terrain Masking & Z-Order](#5-terrain-masking--z-order)
6. [Day/Night Visuals](#6-daynight-visuals)
7. [Player Input & Movement](#7-player-input--movement)
8. [Combat](#8-combat)
9. [AI & Encounters](#9-ai--encounters)
10. [NPCs & Dialogue](#10-npcs--dialogue)
11. [Inventory & Items](#11-inventory--items)
12. [Quest Progression](#12-quest-progression)
13. [Doors & Buildings](#13-doors--buildings)
14. [Day/Night Cycle & Clock](#14-daynight-cycle--clock)
15. [Survival (Hunger, Fatigue, Health)](#15-survival-hunger-fatigue-health)
16. [Magic](#16-magic)
17. [Death & Revival](#17-death--revival)
18. [Carriers (Raft, Turtle, Bird)](#18-carriers-raft-turtle-bird)
19. [Audio](#19-audio)
20. [Intro & Narrative](#20-intro--narrative)
21. [Save/Load](#21-saveload)
22. [UI & Menus](#22-ui--menus)
23. [Asset Loading](#23-asset-loading)
24. [Special Effects](#24-special-effects)

---

## 1. Display & Rendering

### Requirements

| ID | Requirement |
|----|-------------|
| R-DISP-001 | The game shall render a 288×140-pixel playfield viewport scaled to fill the upper portion of a 640×480 logical canvas. |
| R-DISP-002 | A 640×57-pixel HUD bar shall be rendered below the playfield. |
| R-DISP-003 | The rendering pipeline shall use double-buffered page flipping: one drawing page and one viewing page, swapped each frame. |
| R-DISP-004 | Each page shall maintain its own scroll position, sprite count, and background save queue (up to 25 sprites). |
| R-DISP-005 | Sprite backgrounds shall be restored in reverse compositing order before the next frame's tile rendering. |
| R-DISP-006 | The target frame rate shall be 30 fps (NTSC interlaced timing). |

### User Stories

- As a player, I see a game world viewport above a status bar, matching the original Amiga layout.
- As a player, I experience smooth rendering at 30 fps without screen tearing or flicker.

---

## 2. World & Map

### Requirements

| ID | Requirement |
|----|-------------|
| R-WORLD-001 | The game world shall use a 32768×40960 coordinate space with unsigned 16-bit wrapping at boundaries. |
| R-WORLD-002 | The world shall be divided into 10 regions: 8 outdoor (2×4 grid), 1 building interior, 1 dungeon. |
| R-WORLD-003 | Region number shall be computed from hero coordinates using the documented bit-shift formula. |
| R-WORLD-004 | Each region shall load its own tileset (4 image banks, 256 tiles), terrain properties, sector map (256 sectors × 128 bytes), and region map (4096 bytes). |
| R-WORLD-005 | Terrain properties shall encode walkability (lower nibble, 0–3) and mask mode (upper nibble, 0–7) per tile. |
| R-WORLD-006 | Crossing a region boundary shall trigger automatic region data reload. |
| R-WORLD-007 | A minimap cache (19×6 entries) shall be maintained for terrain mask lookups during sprite compositing. |

### User Stories

- As a player, I can walk seamlessly from one region to another without noticing data loads.
- As a player, I see different terrain tilesets when entering distinct regions (snow, desert, swamp, etc.).

---

## 3. Scrolling & Camera

### Requirements

| ID | Requirement |
|----|-------------|
| R-SCROLL-001 | When the hero moves one tile, the offscreen bitmap shall be shifted by one tile and the exposed edge repaired with new tile data. |
| R-SCROLL-002 | On teleportation (multi-tile jump), a full map redraw shall be performed. |
| R-SCROLL-003 | Sub-tile smooth scrolling shall be applied using pixel offsets (X mod 16, Y mod 32) after sprite compositing. |

### User Stories

- As a player, I see smooth scrolling as my character walks, with no visible tile-popping.
- As a player, when I teleport, the screen updates instantly to the new location.

---

## 4. Sprites & Animation

### Requirements

| ID | Requirement |
|----|-------------|
| R-SPRITE-001 | Up to 20 actor slots shall be supported simultaneously (slot 0 = hero, slot 1 = raft, slot 3 = carrier/enemy). |
| R-SPRITE-002 | Sprite data shall be loaded from 5-bitplane Amiga format with a generated 1-bit mask (OR of all planes). |
| R-SPRITE-003 | 8 sprite set types shall be supported: PHIL (67 frames), OBJECTS (116), RAFT (2), CARRIER/turtle (16), CARRIER/bird (8), ENEMY (64), SETFIG (8), DRAGON (5). |
| R-SPRITE-004 | Each actor shall maintain a `struct shape` with 22+ fields: position, type, state, frame, facing, race, goal, tactic, vitality, weapon, environ, velocity. |
| R-SPRITE-005 | 26 animation states shall be implemented, driven by the 87-entry `statelist` table. |
| R-SPRITE-006 | The walk cycle shall use 8 frames per direction, indexed by `cycle & 7` with direction-based base offset. |
| R-SPRITE-007 | The combat animation finite state automaton shall implement 9 states (FIGHTING, SWING1–4, BACKSWING, SHOOT1–3) with random branching between swing states. |
| R-SPRITE-008 | Palette remapping shall be supported for recolored enemy variants (e.g., Ogre vs Orc share sprites with different palettes). |

### User Stories

- As a player, I see my character animate through walk, idle, combat, and death states correctly for all 8 directions.
- As a player, I see distinct enemy types with appropriate appearances and recolored variants.

---

## 5. Terrain Masking & Z-Order

### Requirements

| ID | Requirement |
|----|-------------|
| R-ZMASK-001 | Sprites shall be Z-sorted back-to-front by Y-coordinate before rendering each frame. |
| R-ZMASK-002 | Dead actors shall render with Y−32 depth adjustment; riding hero with Y−32; deep-sinking actors (environ > 25) with Y+32. |
| R-ZMASK-003 | Terrain occlusion masks shall be applied per tile-column during sprite compositing based on the tile's mask mode (0–7). |
| R-ZMASK-004 | Mask mode 0 shall produce no occlusion; modes 1–7 shall implement their documented skip conditions. |
| R-ZMASK-005 | Carriers, arrows, and fairy sprites shall skip terrain masking. |

### User Stories

- As a player, I see my character walk behind trees, buildings, and walls when the character is deeper in the scene.
- As a player, I see sprites layered correctly (behind grounded objects, in front of background tiles).

---

## 6. Day/Night Visuals

### Requirements

| ID | Requirement |
|----|-------------|
| R-FADE-001 | Outdoor palette colors shall be dynamically scaled based on the `lightlevel` triangle wave (0–300). |
| R-FADE-002 | Night palette shall have minimum channels: red ≥ 10%, green ≥ 25%, blue ≥ 60%, producing a blue-tinted night effect. |
| R-FADE-003 | Indoor locations (region ≥ 8) shall always use full brightness. |
| R-FADE-004 | The Green Jewel light spell shall add 200 to the red component calculation of the fade, producing warm illumination. |
| R-FADE-005 | Color 31 of the game palette shall be overridden per-region: orange for volcanic, conditional green/dark for dungeon, sky blue elsewhere. |
| R-FADE-006 | Twilight boost: colors 16–24 shall gain extra blue when green% is between 20 and 75. |

### User Stories

- As a player, I see a gradual transition from day to night with blue-tinted darkness.
- As a player, entering a building restores full brightness regardless of the time of day.

---

## 7. Player Input & Movement

### Requirements

| ID | Requirement |
|----|-------------|
| R-INPUT-001 | Three input sources shall be supported in priority order: mouse/compass, joystick, keyboard. |
| R-INPUT-002 | Mouse input shall map compass/screen positions to one of 9 directions (8 compass + center/stop). |
| R-INPUT-003 | Keyboard direction keys (codes 20–29) shall map to directions via the lookup table `{0,1,2,7,9,3,6,5,4}`. |
| R-INPUT-004 | Combat stance shall activate when right mouse button, keyboard `0` key, or joystick button 1 is held. |
| R-INPUT-005 | Hero walk speed shall be 5 pixels/frame on normal terrain. |
| R-INPUT-006 | Movement shall be blocked by terrain with walkability 0 (impassable). |

### User Stories

- As a player, I can control my character using mouse clicks on the compass, keyboard, or joystick.
- As a player, I can hold the fight button to enter combat stance and swing my weapon.

---

## 8. Combat

### Requirements

| ID | Requirement |
|----|-------------|
| R-COMBAT-001 | Melee hit detection shall use O(n²) pairwise Chebyshev distance check: hit radius = `11 + (attacker.brave / 4)`. |
| R-COMBAT-002 | Hits shall only register when the attacker is in SWING or FIGHTING animation state. |
| R-COMBAT-003 | Damage formula: `1 + weapon_bonus + brave/8 + random(0–1)`. |
| R-COMBAT-004 | The witch shall be immune to all physical damage. The necromancer shall be immune to attackers with race < 7. |
| R-COMBAT-005 | On hit: victim pushed 3–5 pixels away from attacker; attacker brave+1; victim (hero) luck−1. |
| R-COMBAT-006 | 6 missile slots shall support arrows (10px/frame) and fireballs (5px/frame). |
| R-COMBAT-007 | Missile hit detection shall use Chebyshev distance ≤ 10. |
| R-COMBAT-008 | Bow attacks require SHOOT1–SHOOT3 animation states and arrow inventory. |
| R-COMBAT-009 | Enemy death shall trigger weapon drop (probability table, 8 tiers × 4 entries) and treasure drop (5 tiers × 8 entries). |
| R-COMBAT-010 | Dragon shall have 25% chance per tick of launching a fireball at the hero. |

### User Stories

- As a player, I can fight enemies in melee and see damage applied based on my stats and weapon.
- As a player, I can use a bow to shoot arrows at enemies from a distance.
- As a player, I find weapons and treasure dropped by defeated enemies.

---

## 9. AI & Encounters

### Requirements

| ID | Requirement |
|----|-------------|
| R-AI-001 | 11 goal modes shall control high-level NPC behavior (USER, SEEK, FLEE, RANDOM, DEATH, AIMLESS, SEEKOBJ, GUARD, RAFTFOLLOW, PATROL, CONFUSED). |
| R-AI-002 | 13 tactical modes shall control per-tick NPC actions (FRUST, AVOID, PURSUE, CLOSE, FIGHT, BACKUP, MISSILE, WANDER, WAIT, TELEPORT, OBEY, DOOR_SEEK, DOOR_LET). |
| R-AI-003 | AI processing shall be suppressed during fairy revival animation (`goodfairy` between 120 and 255 exclusive). |
| R-AI-004 | The `set_course` algorithm shall compute movement direction from current position to target using 7 directional modes. |
| R-AI-005 | 23 extent zones shall define encounter regions; `find_place()` shall perform linear scan of the first 22 entries per movement tick (first match wins). |
| R-AI-006 | Extent types 0–49 trigger random encounters; 50–59 set groups; 60–61 special figures; 70 carriers; 80–83 peace zones. |
| R-AI-007 | Encounter placement shall occur every 16 ticks; generation every 32 ticks. |
| R-AI-008 | Danger level = `v1 + random(0, v2) + encounter_type`. |

### User Stories

- As a player, I encounter enemies randomly as I explore the world, with difficulty varying by region.
- As a player, I experience enemies that pursue, attack, flee, and wander with distinct behaviors.
- As a player, I find safe areas (towns, temples) where no random enemies spawn.

---

## 10. NPCs & Dialogue

### Requirements

| ID | Requirement |
|----|-------------|
| R-NPC-001 | 14 NPC types shall be supported via the `setfig_table`, each with distinct dialogue logic. |
| R-NPC-002 | TALK range: Yell = 100px, Say/Ask = 50px. If NPC within 35px when yelling, respond "Don't shout!" |
| R-NPC-003 | 61 speech entries shall be supported with `%` character substitution for the current brother's name. |
| R-NPC-004 | Wizard NPCs shall give hints based on their per-instance `goal` field (speeches 27–34). |
| R-NPC-005 | Priest NPCs shall heal the player if kind is high, and give a golden statue if the player has a writ. |
| R-NPC-006 | Sorceress shall give a figurine on first visit and luck boost on repeat visits. |
| R-NPC-007 | Spectre NPCs shall accept bones and give crystal shards in return. |
| R-NPC-008 | Ghost NPCs shall reveal the location of a dead brother's bones. |
| R-NPC-009 | Bartender dialogue shall be context-dependent on fatigue and time of day. |
| R-NPC-010 | GIVE handler: Gold costs 2 units, random kindness increase. Bones given to spectre yield crystal shard. |
| R-NPC-011 | Turtle carrier dialogue depends on whether hero has the sea shell. |

### User Stories

- As a player, I can talk to NPCs and receive contextual dialogue, hints, and quest items.
- As a player, I can give gold to NPCs to increase kindness and unlock dialogue options.
- As a player, I receive different responses from the same NPC depending on my progress and stats.

---

## 11. Inventory & Items

### Requirements

| ID | Requirement |
|----|-------------|
| R-INV-001 | Each brother shall have a 35-element `stuff[]` array: weapons (0–8), magic (9–15), keys (16–21), quest items (22–30), currency (31–34). |
| R-INV-002 | Item pickup shall use the `itrans[31]` translation table to map ground object types to inventory slots. |
| R-INV-003 | Body search on dead enemies shall roll weapon drop then treasure drop from probability tables. |
| R-INV-004 | Equipment effects: weapon slot determines melee damage bonus; bow enables ranged attacks; wand enables fireballs. |
| R-INV-005 | Inventory state shall be preserved per-brother (separate arrays for Julian, Phillip, Kevin). |
| R-INV-006 | Key items (gold, silver, jade, crystal, ebony, bronze) shall be consumed when used to open locked doors. |

### User Stories

- As a player, I can pick up items and see them added to my inventory.
- As a player, I can equip different weapons that affect my combat damage.
- As a player, my inventory is separate for each brother and transfers do not occur on death.

---

## 12. Quest Progression

### Requirements

| ID | Requirement |
|----|-------------|
| R-QUEST-001 | The main quest shall follow: rescue princess → obtain writ from king → trade writ for statue at priest → collect 5 golden statues → enter hidden city → defeat necromancer → obtain talisman. |
| R-QUEST-002 | Up to 3 princesses (Katra, Karla, Kandy) shall be rescuable, tracked by the `princess` counter. |
| R-QUEST-003 | The rescue sequence shall follow the 11-step documented flow: check captive flag → approach NPC → display text → update state → set princess-following mode → escort to extent boundary → grant reward. |
| R-QUEST-004 | Quest flags: `ob_list8[9]` (princess captive), `ob_listg[9]` (sorceress), `ob_listg[10]` (priest), `stuff[22]` (talisman), `stuff[25]` (statues), `stuff[28]` (writ). |
| R-QUEST-005 | Hidden city access in region 4 shall be blocked (tiles overwritten to impassable 254) when `stuff[25] < 5`. |
| R-QUEST-006 | Win condition: when `stuff[22]` (talisman) becomes nonzero, set quit flag and launch victory sequence. |
| R-QUEST-007 | Victory sequence: display placard → load `winpic` → black out → 55-step sunrise fade using `sun_colors[53]` → final fade to black. |
| R-QUEST-008 | 11 stone ring locations shall form a teleportation network; destination = `(current_stone + facing + 1) % 11`. |
| R-QUEST-009 | Stone ring activation requires: standing on sector 144, center position check, match against `stone_list[]`. |
| R-QUEST-010 | Princess captive flag shall reset to 3 on brother death. |

### User Stories

- As a player, I can rescue captive princesses and receive rewards from the king.
- As a player, I need to collect 5 golden statues to unlock the hidden city.
- As a player, I can use stone rings to teleport across the world based on the direction I face.
- As a player, defeating the necromancer and obtaining the talisman wins the game.

---

## 13. Doors & Buildings

### Requirements

| ID | Requirement |
|----|-------------|
| R-DOOR-001 | 86 door entries shall be supported, each with outdoor coordinates, indoor coordinates, type, and destination region. |
| R-DOOR-002 | Outdoor-to-indoor transitions shall use O(log n) binary search on the X-sorted door table. |
| R-DOOR-003 | Indoor-to-outdoor transitions shall use O(n) linear scan. |
| R-DOOR-004 | Door orientation: odd type = horizontal, even type = vertical. Hero alignment relative to door must be validated. |
| R-DOOR-005 | Locked doors shall be opened using the `open_list[17]` table matching door tile type, region, and required key. |
| R-DOOR-006 | DESERT door type shall be blocked unless `stuff[STATBASE] >= 5`. |
| R-DOOR-007 | The `xfer()` teleport function shall: adjust scroll, set position, clear encounters, recalculate region, load region data, regenerate minimap, force redraw, update mood, nudge hero if colliding. |

### User Stories

- As a player, I can enter buildings through doors and transition seamlessly between outdoor and indoor areas.
- As a player, I need specific keys to open locked doors.
- As a player, I cannot enter the desert city door until I have 5 golden statues.

---

## 14. Day/Night Cycle & Clock

### Requirements

| ID | Requirement |
|----|-------------|
| R-CLOCK-001 | `daynight` shall be a 16-bit counter wrapping at 24000, incrementing by 1 per game tick. Full cycle ≈ 6.7 minutes at 60 Hz. |
| R-CLOCK-002 | `lightlevel` shall be a triangle wave: `daynight/40`; if ≥ 300, then `600 − value`. Range: 0 (midnight) to 300 (midday). |
| R-CLOCK-003 | Time events shall trigger at documented periods: midnight (0), morning (8000), midday (12000), evening (18000). |
| R-CLOCK-004 | Time shall not advance during freeze spells (`freeze_timer > 0`). |
| R-CLOCK-005 | During sleep: `daynight += 63` per tick (plus normal +1 = 64 effective advance). |
| R-CLOCK-006 | Turtle glow state shall switch at `lightlevel < 40`. |

### User Stories

- As a player, I experience a day/night cycle that affects visibility, music, and encounter behavior.
- As a player, sleeping advances time rapidly until morning or I'm rested.

---

## 15. Survival (Hunger, Fatigue, Health)

### Requirements

| ID | Requirement |
|----|-------------|
| R-SURV-001 | Hunger shall increment by 1 every 128 game ticks while alive and not sleeping. |
| R-SURV-002 | Hunger warnings at thresholds 35, 60, 90. Starvation warnings every 8th tick when > 90. |
| R-SURV-003 | Vitality −2 every 8th tick when hunger > 100 AND fatigue > 160. |
| R-SURV-004 | Collapse at hunger > 140: hunger reset to 130, forced sleep. |
| R-SURV-005 | Fatigue increments on the same 128-tick timer as hunger. |
| R-SURV-006 | Fatigue warning at 70. Forced sleep at fatigue > 170 only when vitality ≤ 5. |
| R-SURV-007 | Health regeneration: +1 vitality every 1024 ticks. Max vitality = `15 + brave/4`. |
| R-SURV-008 | Voluntary sleep: stand on bed tile (IDs 161, 52, 162, 53) in region 8 for 30 ticks with fatigue ≥ 50. |
| R-SURV-009 | Wake conditions: fatigue == 0, OR (fatigue < 30 AND morning window 9000–10000), OR (enemy present AND 1-in-64 random). |
| R-SURV-010 | Safe zone detection: no enemies, no witch, environ == 0, no danger flag, hero alive. Updated every 128 ticks. |
| R-SURV-011 | Auto-eat: if hunger > 30 and hero has apples, consume one apple (−30 hunger) in safe zones. |
| R-SURV-012 | Fiery death zone: `8802 < map_x < 13562`, `24744 < map_y < 29544`. Environ > 15 = instant death. Environ > 2 = −1 vitality/tick. Hero with fiery fruit: immune. |

### User Stories

- As a player, I must manage hunger and fatigue to avoid collapsing.
- As a player, I can sleep in beds to recover fatigue.
- As a player, I am warned progressively as hunger and fatigue increase.
- As a player, my health regenerates slowly over time outside of combat.

---

## 16. Magic

### Requirements

| ID | Requirement |
|----|-------------|
| R-MAGIC-001 | 7 magic items shall be usable from the MAGIC menu, each requiring the item in inventory. |
| R-MAGIC-002 | Magic shall be blocked in the Necromancer arena (extent v3 == 9). |
| R-MAGIC-003 | Blue Stone: teleport via stone ring network (see R-QUEST-008/009). |
| R-MAGIC-004 | Green Jewel: `light_timer += 760`. Outdoor only visual warm tint effect. |
| R-MAGIC-005 | Glass Vial: heal `random(0–7) + 4` vitality, capped at max HP. |
| R-MAGIC-006 | Crystal Orb: display world map with hero marker. Blocked indoors (region ≥ 8). |
| R-MAGIC-007 | Bird Totem: `secret_timer += 360`. Reveals hidden objects. |
| R-MAGIC-008 | Gold Ring: `freeze_timer += 100`. Blocked when riding > 1. |
| R-MAGIC-009 | Jade Skull: kill all enemies with race < 7. Brave −1 per kill. |
| R-MAGIC-010 | After successful use: `stuff[4+item]--`. Failed uses (wrong location, blocked) do NOT consume a charge. |

### User Stories

- As a player, I can use magic items from the menu to aid exploration and combat.
- As a player, magic items have limited charges that deplete with successful use.
- As a player, magic is blocked in the final boss arena.

---

## 17. Death & Revival

### Requirements

| ID | Requirement |
|----|-------------|
| R-DEATH-001 | When hero vitality < 1: set DYING state, goal DEATH, luck −= 5, display death message. |
| R-DEATH-002 | `goodfairy` u8 countdown from 255. At range 199–120: check `luck < 1`. If true → brother succession. |
| R-DEATH-003 | Fairy animation at `goodfairy` 119–20, resurrection glow at 19–2, revival at 1. |
| R-DEATH-004 | Brother succession (`revive(true)`): save bones, reset princess, load next brother stats from `blist[]`, clear inventory (give dirk), reset to Tambry village, display placard. |
| R-DEATH-005 | Game over when `brother > 3` (all three brothers dead). |
| R-DEATH-006 | Fairy revival (`revive(false)`): fade down, teleport to safe zone, full health, clear hunger/fatigue, set daynight = 8000. |
| R-DEATH-007 | Killing non-hostile NPCs (SETFIGs) shall penalize: kind −= 3. |
| R-DEATH-008 | Killing enemies: attacker brave+1. |

### User Stories

- As a player, when I die, a fairy may revive me depending on my luck stat.
- As a player, if too unlucky, my next brother takes over the quest from the village.
- As a player, if all three brothers die, the game ends.
- As a player, killing innocent NPCs reduces my kindness stat.

---

## 18. Carriers (Raft, Turtle, Bird)

### Requirements

| ID | Requirement |
|----|-------------|
| R-CARRY-001 | Raft: slot 1, activates within 9px proximity, snaps to hero position, only follows on water tiles. |
| R-CARRY-002 | Turtle: slot 3, summoned via USE menu (requires turtle item), 3px/frame ridden speed, autonomous water pathfinding when unridden. |
| R-CARRY-003 | Turtle cannot be summoned in central region bounds (11194–21373 X, 10205–16208 Y). |
| R-CARRY-004 | Bird/Swan: slot 3, requires lasso to board, velocity-based movement with acceleration and speed caps (40 vertical, 32 horizontal), `riding = 11`, `environ = −2` (airborne). |
| R-CARRY-005 | Bird dismount conditions: hero action + not fiery terrain + not too fast + no collision at destination. |
| R-CARRY-006 | Dragon: slot 3, hostile (not rideable), 50 HP, 25% fireball chance per tick at speed 5. |

### User Stories

- As a player, I can ride a raft across water.
- As a player, I can summon and ride a turtle to navigate ocean areas.
- As a player, I can lasso and ride a swan to fly over terrain.
- As a player, I encounter a hostile dragon that breathes fire.

---

## 19. Audio

### Requirements

| ID | Requirement |
|----|-------------|
| R-AUDIO-001 | A 4-voice software synthesizer shall generate audio from 8 waveforms (128-byte 8-bit PCM) and 10 envelopes (256-byte). |
| R-AUDIO-002 | Waveforms and envelopes shall be loaded from the `v6` file. Scores (28 tracks) from the `songs` file. |
| R-AUDIO-003 | Track data format: 2-byte command/value pairs (notes 0–127, rest $80, set instrument $81, set tempo $90, end $FF). |
| R-AUDIO-004 | Period table: 84 entries (7 octaves × 12 notes). NTSC base: Hz = 3,579,545 / period. |
| R-AUDIO-005 | 7 musical moods (Death, Indoor, Battle, Astral, Day, Night, Intro) evaluated by priority. |
| R-AUDIO-006 | `setmood(TRUE)` shall restart playback immediately; `setmood(FALSE)` update loop points only. |
| R-AUDIO-007 | 6 sound effect samples loaded from disk sectors 920–930, played on channel 2 with randomized period. |
| R-AUDIO-008 | Sound effect playback shall be gatable via menu toggle. |
| R-AUDIO-009 | Audio VBL interrupt rate: 60 Hz. |

### User Stories

- As a player, I hear music that changes based on time of day, combat, and location.
- As a player, I hear sound effects for combat, interactions, and environmental events.

---

## 20. Intro & Narrative

### Requirements

| ID | Requirement |
|----|-------------|
| R-INTRO-001 | Intro sequence: legal text → 1s pause → load audio → start intro music → load title image → vertical zoom-in → 3 story pages with columnar reveal → pause → zoom-out → copy protection. |
| R-INTRO-002 | Copy protection: 3 random questions from 8 rhyming-word answers (LIGHT, HEED, DEED, SIGHT, FLIGHT, CREED, BLIGHT, NIGHT). Case-sensitive, max 9 characters. |
| R-INTRO-003 | 39 event messages shall display during gameplay with `%` substitution for the brother name. |
| R-INTRO-004 | 29 outdoor place names and 31 indoor place names shall trigger on sector entry (first-match linear scan). |
| R-INTRO-005 | Text rendering: `print()` scroll up 10px and render at bottom; `print_cont()` append; `extract()` word-wrap at 37 chars with `%` substitution. |
| R-INTRO-006 | Placard text: `ssp()` renderer with embedded XY positioning (byte 128 + x/2 + y). 20 story messages available. |
| R-INTRO-007 | Player may skip the intro at multiple checkpoints. |

### User Stories

- As a player, I see the original intro sequence with title zoom, story pages, and music.
- As a player, I can skip the intro to get into the game quickly.
- As a player, I see location names when entering named areas.

---

## 21. Save/Load

### Requirements

| ID | Requirement |
|----|-------------|
| R-SAVE-001 | 8 save slots named `A.faery` through `H.faery`. |
| R-SAVE-002 | Save format: raw sequential binary dump — 80-byte misc vars, region, anim list, 3 brother inventories, missiles, extents, object lists. No headers, no version, no checksums. |
| R-SAVE-003 | The 80-byte misc block shall be written contiguously starting from `map_x` through `pad7`. |
| R-SAVE-004 | Post-load: reload sprites, refresh menus, force full redraw, clear encounters. |
| R-SAVE-005 | Save/load shall be accessible from the GAME → SAVEX → FILE menu chain. |

### User Stories

- As a player, I can save my game to one of 8 slots and resume later.
- As a player, loading a save restores the complete game state including position, inventory, and quest progress.

---

## 22. UI & Menus

### Requirements

| ID | Requirement |
|----|-------------|
| R-UI-001 | 10 menu modes: ITEMS, MAGIC, TALK, BUY, GAME, SAVEX, KEYS, GIVE, USE, FILE. |
| R-UI-002 | Modes 0–4 share a top row for mode switching; mode-specific sub-labels provide options. |
| R-UI-003 | Options rendered in 2-column layout: even at x=430, odd at x=482, up to 12 entries. |
| R-UI-004 | Option enable flags: bit 0 = selected, bit 1 = displayed, bits 2–7 = type (unchangeable, toggle, immediate, radio). |
| R-UI-005 | 38 keyboard shortcuts shall be mapped: F1–F7 for magic, 1–7 for weapons, letters for actions (I=Items, T=Take, G=Give, Y=Yell, S=Say, etc.). |
| R-UI-006 | 8-direction compass displayed on HUD at (567, 15) with highlighted direction overlay. |
| R-UI-007 | Stats display: vitality bar at (245, 52); stats Brv/Lck/Knd/Wlth at documented X positions. |
| R-UI-008 | `set_options()` shall dynamically update menu enabled states based on current game state (inventory, proximity, etc.). |

### User Stories

- As a player, I can navigate menus to manage inventory, use magic, talk, and save/load.
- As a player, I can use keyboard shortcuts for quick access to common actions.
- As a player, I see my stats and compass direction on the HUD at all times.

---

## 23. Asset Loading

### Requirements

| ID | Requirement |
|----|-------------|
| R-ASSET-001 | The `image` file (901120 bytes, 1760 sectors × 512 bytes) shall be the primary data source for tilesets, terrain, sprites, and other binary assets. |
| R-ASSET-002 | `file_index[10]` shall map regions to their image bank, terrain, sector, region, and setfig sector addresses. |
| R-ASSET-003 | `cfiles[18]` shall map sprite sets to disk sector addresses and dimensions. |
| R-ASSET-004 | IFF/ILBM files (`page0`, `p1a`–`p3b`, `hiscreen`, `winpic`) shall be loaded with BMHD, CMAP, and BODY chunk parsing with ByteRun1 RLE decompression. |
| R-ASSET-005 | Font (Amber 9pt) shall be loaded from hunk-format file `fonts/Amber/9`. |
| R-ASSET-006 | Audio data: `v6` (waveforms + envelopes), `songs` (28 tracks), samples from `image` sectors 920–930. |
| R-ASSET-007 | Region loading shall load 4 image banks, 2 terrain tables, sector map, and region map, updating the minimap and performing tileset-to-SDL2-texture conversion. |

### User Stories

- As a player, the game loads all regions, images, music, and fonts without errors from the original data files.

---

## 24. Special Effects

### Requirements

| ID | Requirement |
|----|-------------|
| R-FX-001 | Witch vision cone: rotating filled quadrilateral using `witchpoints[256]` sine/cosine data, cross-product hero detection, XOR rendering. |
| R-FX-002 | Teleport colorplay: 32 frames of randomized palette for all 31 colors, ≈ 0.64 seconds duration. |
| R-FX-003 | Columnar page reveal: 22-step vertical strip animation for story page transitions using the `flip3[]` timing table. |
| R-FX-004 | Victory sunrise: 55-step palette fade using `sun_colors[53]`, sweeping colors 2–27. |
| R-FX-005 | Screen fade-down and fade-up shall smoothly interpolate palette between game colors and black. |

### User Stories

- As a player, I see the witch's spinning vision cone that damages me when caught.
- As a player, I see dramatic palette effects during teleportation, story transitions, and the victory ending.

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
