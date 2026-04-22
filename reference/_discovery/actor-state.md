# Discovery: Actor State & Core Data Structures

**Status**: complete
**Investigated**: 2026-04-05
**Requested by**: orchestrator
**Prompt summary**: Trace all core data structures: struct shape, motion/goal/tactical enums, statelist, trans_list, setfig_table, encounter_chart, actor array, and other key structs from ftale.h.

## struct shape

Defined in `ftale.h:56-67` (C) and `ftale.i:5-22` (assembly with byte offsets).

| Offset | Size | C Field | Type | Purpose |
|--------|------|---------|------|---------|
| 0 | 2 | `abs_x` | unsigned short | Absolute world X coordinate |
| 2 | 2 | `abs_y` | unsigned short | Absolute world Y coordinate |
| 4 | 2 | `rel_x` | unsigned short | Screen-relative X coordinate |
| 6 | 2 | `rel_y` | unsigned short | Screen-relative Y coordinate |
| 8 | 1 | `type` | char | What number object is this |
| 9 | 1 | `race` | UBYTE | Race number (indexes encounter_chart) |
| 10 | 1 | `index` | char | Image index for current animation frame |
| 11 | 1 | `visible` | char | On-screen flag |
| 12 | 1 | `weapon` | char | Type of weapon carried (0=none, 1=dagger, 2=mace, 3=sword, 4=bow, 5=wand) |
| 13 | 1 | `environ` | char | Environment variable |
| 14 | 1 | `goal` | char | Current goal mode (see Goal Modes) |
| 15 | 1 | `tactic` | char | Current tactical mode (see Tactical Modes) |
| 16 | 1 | `state` | char | Current movement/animation state (see Motion States) |
| 17 | 1 | `facing` | char | Direction facing (0=NW, 1=N, 2=NE, 3=E, 4=SE, 5=S, 6=SW, 7=W) |
| 18 | 2 | `vitality` | short | Hit points; also original object number for NPCs |
| 20 | 1 | `vel_x` | char | X velocity (for slippery areas) |
| 21 | 1 | `vel_y` | char | Y velocity (for slippery areas) |
| **22** | | | | **Total struct size (l_shape label in ftale.i)** |

Note: A commented-out `APTR source_struct` field appears in both C and asm definitions (`ftale.h:66`, `ftale.i:21`), suggesting a removed feature.

## Motion States

Defined in `ftale.h:9-25` (also duplicated in `fmain.c:90-103`).

| Value | Name | Purpose |
|-------|------|---------|
| 0 | `FIGHTING` | Combat animation (states 0-11 in each fight direction block) |
| 12 | `WALKING` | Normal walk cycle |
| 13 | `STILL` | Stationary/idle |
| 14 | `DYING` | Death animation in progress |
| 15 | `DEAD` | Fully dead |
| 16 | `SINK` | Sinking (quicksand/water) |
| 17 | `OSCIL` | Oscillation animation 1 (comment: "and 18") |
| 18 | *(implicit)* | Oscillation animation 2 (paired with OSCIL=17) |
| 19 | `TALKING` | Speaking/dialogue |
| 20 | `FROZEN` | Frozen in place (freeze spell effect) |
| 21 | `FLYING` | Flying (bird/dragon carrier) |
| 22 | `FALL` | Falling |
| 23 | `SLEEP` | Sleeping |
| 24 | `SHOOT1` | Bow up — aiming |
| 25 | `SHOOT3` | Bow fired, arrow given velocity |

States 0-11 are fighting sub-states (weapon swing phases). States 1-11 have no named #define — they are fight animation frames whose figure is selected via `statelist[]` indexing: `statelist[facing*12 + state]` for the combat blocks starting at index 32.

## Goal Modes

Defined in `ftale.h:29-39` (also duplicated in `fmain.c:107-117`).

| Value | Name | Purpose |
|-------|------|---------|
| 0 | `USER` | Character is player-controlled |
| 1 | `ATTACK1` | Attack character (stupidly — low cleverness) |
| 2 | `ATTACK2` | Attack character (cleverly — high cleverness) |
| 3 | `ARCHER1` | Archery attack style 1 |
| 4 | `ARCHER2` | Archery attack style 2 |
| 5 | `FLEE` | Run directly away from character |
| 6 | `STAND` | Don't move but face character |
| 7 | `DEATH` | A dead character |
| 8 | `WAIT` | Wait to speak to character |
| 9 | `FOLLOWER` | Follow another character |
| 10 | `CONFUSED` | Run around randomly |

Goal is stored in `shape.goal` (offset 14). Goal transitions are driven by the AI dispatch in the main loop. ATTACK1 vs ATTACK2 is determined by the `cleverness` field in `encounter_chart` — `fmain.c:2150` area dispatches tactics based on goal.

## Tactical Modes

Defined in `ftale.h:43-54` (partially duplicated in `fmain.c:121-132` — assembly note: fmain.c omits DOOR_SEEK and DOOR_LET).

| Value | Name | Purpose |
|-------|------|---------|
| 0 | `FRUST` | All tactics frustrated — try something else |
| 1 | `PURSUE` | Go in the direction of the character |
| 2 | `FOLLOW` | Go toward another character |
| 3 | `BUMBLE_SEEK` | Bumble around looking for target |
| 4 | `RANDOM` | Move randomly |
| 5 | `BACKUP` | Opposite direction we were going |
| 6 | `EVADE` | Move 90 degrees from character |
| 7 | `HIDE` | Seek a hiding place |
| 8 | `SHOOT` | Shoot an arrow |
| 9 | `SHOOTFRUST` | Arrows not getting through |
| 10 | `EGG_SEEK` | Snakes going for the turtle eggs |
| 11 | `DOOR_SEEK` | Dark Knight blocking door |
| 12 | `DOOR_LET` | Dark Knight letting player pass |

Note: fmain.c:121-132 defines values 0-10 only. Values 11-12 (DOOR_SEEK, DOOR_LET) are defined only in `ftale.h:53-54`. The comment says "choices 2-5 can be selected randomly for getting around obstacles."

## statelist[]

Defined in `fmain.c:143-205`. An array of 87 `struct state` entries (each: `figure`, `wpn_no`, `wpn_x`, `wpn_y` — all chars, `fmain.c:138-142`).

Maps `(motion_state, facing, frame)` → `(figure_image, weapon_overlay_index, weapon_x_offset, weapon_y_offset)`.

### Walk Sequences (8 frames each)

Indexed as: base + walk_frame (0-7).

| Index | Direction | Source |
|-------|-----------|--------|
| 0-7 | South walk | `fmain.c:148-149` |
| 8-15 | West walk | `fmain.c:152-153` |
| 16-23 | North walk | `fmain.c:156-157` |
| 24-31 | East walk | `fmain.c:160-161` |

### Fight Sequences (12 states each)

Indexed as: base + transition_state (0-11, mapped via `trans_list[]`).

| Index | Direction | Source |
|-------|-----------|--------|
| 32-43 | South fight | `fmain.c:164-169` |
| 44-55 | West fight | `fmain.c:172-177` |
| 56-67 | North fight | `fmain.c:180-185` |
| 68-79 | East fight | `fmain.c:188-193` |

Each fight block has 12 entries: states 0-8 are weapon swing positions, state 9 duplicates a swing position, states 10-11 are ranged attack frames (indices 80+ weapons like bows).

### Special States

| Index | Purpose | Source |
|-------|---------|--------|
| 80-82 | Death sequence (3 frames) | `fmain.c:196` |
| 83 | Sinking sequence | `fmain.c:198` |
| 84-85 | Oscillation (sword at side) | `fmain.c:201` |
| 86 | Asleep | `fmain.c:203` |

### Complete statelist[] Data

```
Idx  fig  wpn   wpn_x  wpn_y   (fmain.c line)
--- ---- ----  -----  -----
  0:   0,  11,    -2,    11    (148 — south walk)
  1:   1,  11,    -3,    11
  2:   2,  11,    -3,    10
  3:   3,  11,    -3,     9
  4:   4,  11,    -3,    10
  5:   5,  11,    -3,    11
  6:   6,  11,    -2,    11
  7:   7,  11,    -1,    11
  8:   8,   9,   -12,    11    (152 — west walk)
  9:   9,   9,   -11,    12
 10:  10,   9,    -8,    13
 11:  11,   9,    -4,    13
 12:  12,   9,     0,    13
 13:  13,   9,    -4,    13
 14:  14,   9,    -8,    13
 15:  15,   9,   -11,    12
 16:  16,  14,    -1,     1    (156 — north walk)
 17:  17,  14,    -1,     2
 18:  18,  14,    -1,     3
 19:  19,  14,    -1,     4
 20:  20,  14,    -1,     3
 21:  21,  14,    -1,     2
 22:  22,  14,    -1,     1
 23:  23,  14,    -1,     1
 24:  24,  10,     5,    12    (160 — east walk)
 25:  25,  10,     3,    12
 26:  26,  10,     2,    12
 27:  27,  10,     3,    12
 28:  28,  10,     5,    12
 29:  29,  10,     6,    12
 30:  30,  10,     6,    11
 31:  31,  10,     6,    12
 32:  32,  11,    -2,    12    (164 — south fight)
 33:  32,  10,     0,    12
 34:  33,   0,     2,    10
 35:  34,   1,     4,     6
 36:  34,   2,     1,     4
 37:  34,   3,     0,     4
 38:  36,   4,    -5,     0
 39:  36,   5,   -10,     1
 40:  35,  12,    -5,     5
 41:  36,   0,     0,     6
 42:  38,  85,    -6,     5
 43:  37,  81,    -6,     5
 44:  40,   9,    -7,    12    (172 — west fight)
 45:  40,   8,    -9,     9
 46:  41,   7,   -10,     5
 47:  42,   7,   -12,     4
 48:  42,   6,   -12,     3
 49:  42,   5,   -12,     3
 50:  44,   5,    -8,     3
 51:  44,  14,    -7,     6
 52:  43,  13,    -7,     8
 53:  42,   5,   -12,     3
 54:  46,  86,    -3,     0
 55:  45,  82,    -3,     0
 56:  48,  14,    -3,     0    (180 — north fight)
 57:  48,   6,    -3,    -1
 58:  49,   5,    -2,    -3
 59:  50,   5,    -3,    -4
 60:  50,   4,     0,     0
 61:  50,   3,     3,     0
 62:  52,   4,     6,     1
 63:  52,  15,     7,     3
 64:  51,  14,     1,     6
 65:  50,   4,     0,     0
 66:  54,  87,     3,     0
 67:  53,  83,     3,     0
 68:  56,  10,     5,    11    (188 — east fight)
 69:  56,   0,     6,     9
 70:  57,   1,    10,     6
 71:  58,   1,    10,     5
 72:  58,   2,     7,     3
 73:  58,   3,     6,     3
 74:  60,   4,     1,     0
 75:  60,   3,     3,     2
 76:  59,  15,     4,     1
 77:  58,   4,     5,     1
 78:  62,  84,     3,     0
 79:  61,  80,     3,     0
 80:  47,   0,     5,    11    (196 — death)
 81:  63,   0,     6,     9
 82:  39,   0,     6,     9
 83:  55,  10,     5,    11    (198 — sinking)
 84:  64,  10,     5,    11    (201 — oscillation 1)
 85:  65,  10,     5,    11    (201 — oscillation 2)
 86:  66,  10,     5,    11    (203 — asleep)
```

How indexing works: For walking, `state = WALKING (12)`, the walk frame counter (0-7) is added to a direction base: S=0, W=8, N=16, E=24. For fighting, `state` is 0-11 (the transition state), added to a direction base: S=32, W=44, N=56, E=68. Special states use fixed indices: DYING=80+frame, SINK=83, OSCIL=84/85, SLEEP=86.

## trans_list[]

Defined in `fmain.c:137-146`. An array of 9 `struct transition` entries. Each has `newstate[4]` — 4 char values controlling fight animation transitions.

```c
struct transition { char newstate[4]; }  // fmain.c:136-137
```

The 4 values per entry are: `[swing_fwd, swing_back, current_hold, swing_reverse]` (inferred from indexing patterns).

| Index | State Description | newstate[0] | newstate[1] | newstate[2] | newstate[3] | Source |
|-------|-------------------|-------------|-------------|-------------|-------------|--------|
| 0 | Arm down, weapon low | 1 | 8 | 0 | 1 | `fmain.c:138` |
| 1 | Arm down, weapon diagonal down | 2 | 0 | 1 | 0 | `fmain.c:139` |
| 2 | Arm swing1, weapon horizontal | 3 | 1 | 2 | 8 | `fmain.c:140` |
| 3 | Arm swing2, weapon raised | 4 | 2 | 3 | 7 | `fmain.c:141` |
| 4 | Arm swing2, weapon diag up | 5 | 3 | 4 | 6 | `fmain.c:142` |
| 5 | Arm swing2, weapon high | 6 | 4 | 5 | 5 | `fmain.c:143` |
| 6 | Arm high, weapon up | 8 | 5 | 6 | 4 | `fmain.c:144` |
| 7 | Arm high, weapon horizontal | 8 | 6 | 7 | 3 | `fmain.c:145` |
| 8 | Arm middle, weapon raised fwd | 0 | 6 | 8 | 2 | `fmain.c:146` |

States form a cycle: 0→1→2→3→4→5→6→(7→)8→0 via newstate[0]. newstate[1] goes backward through the cycle. This implements the sword swing arc animation.

## setfig_table[]

Defined in `fmain.c:21-35`. Maps NPC type index to image file and speech capability.

```c
struct { BYTE cfile_entry, image_base, can_talk; } setfig_table[]   // fmain.c:21-23
```

| Index | NPC Type | cfile_entry | image_base | can_talk | Source |
|-------|----------|-------------|------------|----------|--------|
| 0 | Wizard | 13 | 0 | 1 | `fmain.c:24` |
| 1 | Priest | 13 | 4 | 1 | `fmain.c:25` |
| 2 | Guard (front) | 14 | 0 | 0 | `fmain.c:26` |
| 3 | Guard (back) | 14 | 1 | 0 | `fmain.c:27` |
| 4 | Princess | 14 | 2 | 0 | `fmain.c:28` |
| 5 | King | 14 | 4 | 1 | `fmain.c:29` |
| 6 | Noble | 14 | 6 | 0 | `fmain.c:30` |
| 7 | Sorceress | 14 | 7 | 0 | `fmain.c:31` |
| 8 | Bartender | 15 | 0 | 0 | `fmain.c:32` |
| 9 | Witch | 16 | 0 | 0 | `fmain.c:33` |
| 10 | Spectre | 16 | 6 | 0 | `fmain.c:34` |
| 11 | Ghost | 16 | 7 | 0 | `fmain.c:35` |
| 12 | Ranger | 17 | 0 | 1 | `fmain.c:36` |
| 13 | Beggar | 17 | 4 | 1 | `fmain.c:37` |

`cfile_entry` selects the image file (seq_list index for SETFIG sequence). `image_base` is the sub-image offset within that file. `can_talk=1` means the NPC can initiate generic dialogue. 14 entries total.

## encounter_chart[]

Defined in `fmain.c:42-63`. Maps monster race index to combat stats.

```c
struct encounter {
    char hitpoints, agressive, arms, cleverness, treasure, file_id;
} encounter_chart[]     // fmain.c:42-53
```

| Index | Monster | hitpoints | agressive | arms | cleverness | treasure | file_id | Source |
|-------|---------|-----------|-----------|------|------------|----------|---------|--------|
| 0 | Ogre | 18 | TRUE | 2 | 0 | 2 | 6 | `fmain.c:54` |
| 1 | Orcs | 12 | TRUE | 4 | 1 | 1 | 6 | `fmain.c:55` |
| 2 | Wraith | 16 | TRUE | 6 | 1 | 4 | 7 | `fmain.c:56` |
| 3 | Skeleton | 8 | TRUE | 3 | 0 | 3 | 7 | `fmain.c:57` |
| 4 | Snake | 16 | TRUE | 6 | 1 | 0 | 8 | `fmain.c:58` |
| 5 | Salamander | 9 | TRUE | 3 | 0 | 0 | 7 | `fmain.c:59` |
| 6 | Spider | 10 | TRUE | 6 | 1 | 0 | 8 | `fmain.c:60` |
| 7 | DKnight | 40 | TRUE | 7 | 1 | 0 | 8 | `fmain.c:61` |
| 8 | Loraii | 12 | TRUE | 6 | 1 | 0 | 9 | `fmain.c:62` |
| 9 | Necromancer | 50 | TRUE | 5 | 0 | 0 | 9 | `fmain.c:63` |
| 10 | Woodcutter | 4 | NULL (0) | 0 | 0 | 0 | 9 | `fmain.c:64` |

**Field meanings:**
- `hitpoints` — base vitality for this monster type
- `agressive` — TRUE=hostile on sight, NULL=passive
- `arms` — indexes into `weapon_probs[]` (4 entries per group, so `weapon_probs[arms*4 + random]` selects weapon) — `fmain.c:2758`
- `cleverness` — 0=ATTACK1 (stupid), 1=ATTACK2 (clever) — determines goal mode assigned at spawn
- `treasure` — indexes into `treasure_probs[]` (8 entries per group, so `treasure_probs[treasure*8 + random]` selects loot) — `fmain.c:3273`
- `file_id` — image file index for loading monster sprites

11 entries total. `arms` and `treasure` are indirect indices into `weapon_probs[]` (fmain2.c:860) and `treasure_probs[]` (fmain2.c:852) respectively.

## Actor Array

```c
#define MAXSHAPES 25                        // fmain.c:68
struct shape anim_list[20];                 // fmain.c:70 — "7 people + 7 objects"
unsigned char anim_index[20];              // fmain.c:74 — for sorting (depth order)
short anix, anix2;                         // fmain.c:75 — allocation index (how many monsters + 1)
short mdex;                                // fmain.c:76 — missile index
```

- **anim_list[0]** — always the player-controlled hero
- **anim_list[1-2]** — party members / carriers (bird/turtle occupy index 3 when `active_carrier` set; `fmain.c:1456`)
- **anim_list[3-6]** — enemy actors (up to 4 enemies; `anix` tracks count, max 7 per `fmain.c:2064`)
- **anim_list[7-19]** — remaining slots for objects/setfigs

`MAXSHAPES=25` is used for the `sshape` rendering queue (not anim_list size). The rendering queue has 25 entries per page:

```c
struct sshape {                             // fmain.c:436-440
    unsigned char *backsave;
    short savesize, blitsize, Coff, Cmod;
};
// Allocated: 2 pages × MAXSHAPES entries — fmain.c:882-884
```

`anim_index[20]` is a sort index for depth-ordering actors before rendering.

## Missile System

```c
struct missile {                            // fmain.c:78-85
    unsigned short abs_x, abs_y;
    char missile_type,                     // NULL, arrow, rock, 'thing', or fireball
         time_of_flight,                   // in frames
         speed,                            // 0 = still unshot
         direction,
         archer;                           // ID of archer who fired
} missile_list[6];                         // 6 missiles max
```

## Other Structs

### struct fpage — `ftale.h:69-79`, `ftale.i:24-37`

Double-buffered page state for the display system.

| Field | Type | Purpose |
|-------|------|---------|
| `ri_page` | RasInfo* | Amiga RasInfo structure for this page |
| `savecop` | cprlist* | Copper list pointer |
| `isv_x` | long | Page scroll X position |
| `isv_y` | long | Page scroll Y position |
| `obcount` | short | Number of objects queued for rendering |
| `shape_queue` | sshape* | Pointer to rendering shape queue |
| `backsave` | unsigned char* | Background save buffer |
| `saveused` | long | How much of save buffer is used |
| `witchx` | short | Witch effect X position (for erasure) |
| `witchy` | short | Witch effect Y position |
| `witchdir` | short | Witch effect direction |
| `wflag` | short | Witch effect flag |

Two instances: `fp_page1`, `fp_page2` — `fmain.c:443`.

### struct seq_info — `ftale.h:81-88`, `ftale.i:39-47`

Image sequence descriptor (sprite sheet metadata).

| Field | Type | Purpose |
|-------|------|---------|
| `width` | short | Frame width in pixels |
| `height` | short | Frame height in pixels |
| `count` | short | Number of frames |
| `location` | unsigned char* | Pointer to image data |
| `maskloc` | unsigned char* | Pointer to mask data |
| `bytes` | short | Bytes per frame (calculated) |
| `current_file` | short | Which file is currently loaded |

Enumerated sequence types (`ftale.h:90`): `PHIL=0, OBJECTS=1, ENEMY=2, RAFT=3, SETFIG=4, CARRIER=5, DRAGON=6`.
Array: `seq_list[7]` — `fmain.c:39`.

### struct object — `ftale.h:92-95`, `ftale.i:53-58`

World object instance (250 objects per sector).

| Field | Type | Purpose |
|-------|------|---------|
| `xc` | unsigned short | World X coordinate |
| `yc` | unsigned short | World Y coordinate |
| `ob_id` | char | Object type ID |
| `ob_stat` | char | Object status (0=inactive, 1+=active states) |

6 bytes per object. Two external arrays: `ob_listg[]`, `ob_list8[]` — `fmain.c:378`.

### struct inv_item — `ftale.h:97-104`

Inventory item display descriptor.

| Field | Type | Purpose |
|-------|------|---------|
| `image_number` | UBYTE | Image number for display |
| `xoff` | UBYTE | X offset on inventory screen |
| `yoff` | UBYTE | Y offset on inventory screen |
| `ydelta` | UBYTE | Y increment for stacking display |
| `img_off` | UBYTE | Sub-image offset within image |
| `img_height` | UBYTE | Height of sub-image to draw |
| `maxshown` | UBYTE | Maximum number that can be shown (also gold value for gold items) |
| `name` | char* | Display name string |

Full inventory list: `inv_list[]` (36 entries, indices 0-35) — `fmain.c:380-418`.

Inventory array layout (by index ranges, `fmain.c:427-430`):
- 0-4: Weapons (Dirk, Mace, Sword, Bow, Magic Wand)
- 5-8: Special items (Golden Lasso, Sea Shell, Sun Stone, Arrows)
- 9-15: Magic items (MAGICBASE=9): Blue Stone, Green Jewel, Glass Vial, Crystal Orb, Bird Totem, Gold Ring, Jade Skull
- 16-21: Keys (KEYBASE=16): Gold, Green, Blue, Red, Grey, White
- 22-24: Quest/stat items: Talisman, Rose, Fruit
- 25-30: STATBASE=25: Gold Statue, Book, Herb, Writ, Bone, Shard
- 31-34: Gold (GOLDBASE=31): 2gp, 5gp, 10gp, 100gp
- 35: ARROWBASE=35: "quiver of arrows"

Player inventory stored as: `UBYTE *stuff` (pointer to current brother's array), with per-brother arrays `julstuff[35]`, `philstuff[35]`, `kevstuff[35]` — `fmain.c:432`.

### struct need — `ftale.h:106-108`

Asset loading requirement descriptor.

| Field | Type | Purpose |
|-------|------|---------|
| `image[4]` | USHORT | 4 image file indices needed |
| `terra1` | USHORT | Terrain data file 1 |
| `terra2` | USHORT | Terrain data file 2 |
| `sector` | USHORT | Sector data file |
| `region` | USHORT | Region data file |
| `setchar` | USHORT | Set character file needed |

Used by `file_index[10]` (one per map region F1-F10) — `fmain.c:615-625` — and `current_loads` for tracking what's loaded — `fmain.c:614`.

### struct in_work — `ftale.h:110-120`

Input handler shared data area.

| Field | Type | Purpose |
|-------|------|---------|
| `xsprite` | short | Sprite X position |
| `ysprite` | short | Sprite Y position |
| `qualifier` | short | Input qualifier flags |
| `laydown` | UBYTE | Lay-down-item flag |
| `pickup` | UBYTE | Pick-up-item flag |
| `newdisk` | char | Disk change flag |
| `lastmenu` | char | Last menu selection |
| `gbase` | GfxBase* | Graphics library base |
| `pbase` | SimpleSprite* | Pointer sprite base |
| `vbase` | ViewPort* | Viewport base |
| `keybuf[128]` | unsigned char | Keyboard state buffer (128 keys) |
| `ticker` | short | Input tick counter |

Instance: `handler_data` — `fmain.c:694`.

## Supporting Tables

### weapon_probs[] — `fmain2.c:860-868`
8 groups of 4 entries each (32 total). Indexed by `encounter_chart[race].arms * 4 + rnd(4)`.

```
Group 0: 0,0,0,0       — no weapons
Group 1: 1,1,1,1       — dirks only
Group 2: 1,2,1,2       — dirks and maces
Group 3: 1,2,3,2       — mostly maces
Group 4: 4,4,3,2       — swords and bows
Group 5: 5,5,5,5       — magic wand
Group 6: 8,8,8,8       — touch attack
Group 7: 3,3,3,3       — swords only
```

### treasure_probs[] — `fmain2.c:852-858`
5 groups of 8 entries each (40 total). Indexed by `encounter_chart[race].treasure * 8 + rnd(8)`.

```
Group 0:  0, 0, 0, 0, 0, 0, 0, 0     — no treasure
Group 1:  9,11,13,31,31,17,17,32     — stone, vial, totem, gold, keys
Group 2: 12,14,20,20,20,31,33,31     — keys, skull, gold, nothing
Group 3: 10,10,16,16,11,17,18,19     — magic and keys
Group 4: 15,21, 0, 0, 0, 0, 0, 0     — jade skull and white key
```

### fallstates[] — `fmain2.c:871-874`
24 bytes (4 groups of 6). Used for brother death/fall animations.

```
Group 0: 0x00, 0x00, 0x00, 0x00, 0x00, 0x00   — (unused/default)
Group 1: 0x20, 0x22, 0x3a, 0x6f, 0x70, 0x71
Group 2: 0x24, 0x27, 0x3c, 0x6f, 0x70, 0x71
Group 3: 0x37, 0x38, 0x3d, 0x6f, 0x70, 0x71
```

Referenced at `fmain.c:1735` and `fmain.c:1768` — indexed by brother number.

## Cross-Cutting Findings

- **fmain.c:68 vs fmain.c:70**: `MAXSHAPES=25` but `anim_list` has only 20 entries. MAXSHAPES governs the sshape rendering queue per page, not the actor limit. This is a potential source of confusion.
- **ftale.h vs fmain.c duplication**: All #defines for motion states, goal modes, and tactical modes are duplicated between ftale.h and fmain.c (with ftale.h having 2 extra tactical modes: DOOR_SEEK, DOOR_LET). The fmain.c copies are presumably to allow standalone compilation; the header is the authoritative source.
- **encounter_chart[10] (Woodcutter)**: The only non-aggressive entry (`agressive=NULL`). This is a peaceful NPC sharing the encounter system rather than the setfig system.
- **Weapon code 8** in weapon_probs: "touch attack" — weapon type 8 is not in the standard weapon enum (0-5). This represents monsters that attack by contact rather than with a weapon.
- **`vitality` dual purpose** (`ftale.h:63`): Comment says "also original object number" — for NPCs this field doubles as the ob_list index they were spawned from, not just hitpoints.
- **`environ` field** (`ftale.h:61`): No #define constants found. Purpose and valid values unclear — see Unresolved.

## Unresolved

- **`environ` field values**: The `environ` field in struct shape has no #define constants and no clear documentation of valid values. It is described only as "environment variable." Would need code path tracing of all reads/writes to determine meaning.
- **Motion state 18**: Implicitly part of OSCIL (17) based on comment "and 18", but has no #define. The relationship between states 17 and 18 needs code path tracing to understand how they alternate.
- **trans_list newstate[4] semantics**: The 4 entries are labeled in comments by weapon position, but the exact indexing logic (which index is used for swing forward, backward, parry, etc.) requires tracing the fight animation code that reads `trans_list[state].newstate[direction]`.
- **weapon code 6-8+ overlays**: statelist entries reference weapon indices 80+ (e.g., 80-87) for ranged attack frames. These are beyond the standard 16 weapon overlay images. The mapping from these high wpn_no values to actual image data needs investigation.

## Refinement Log
- 2026-04-05: Initial discovery pass — complete enumeration of all requested structures from ftale.h, ftale.i, and fmain.c.
