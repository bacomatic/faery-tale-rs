# Game Mechanics Research — Data Structures & Core Systems

Core data structures, actor state machine, and random number generation.

> **Citation format**: `file.c:LINE` or `file.c:START-END`. Speech references: `speak(N)`.
> Split from [RESEARCH.md](RESEARCH.md). See the hub document for the full section index.

---

## 1. Core Data Structures

### 1.1 struct shape — Actor Record

The fundamental actor record, used for the player, NPCs, and enemies. Defined in C at `ftale.h:56-67` and mirrored with byte offsets in assembly at `ftale.i:5-22`.

| Offset | Size | Field | Type | Purpose |
|--------|------|-------|------|---------|
| 0 | 2 | `abs_x` | unsigned short | Absolute world X coordinate |
| 2 | 2 | `abs_y` | unsigned short | Absolute world Y coordinate |
| 4 | 2 | `rel_x` | unsigned short | Screen-relative X position |
| 6 | 2 | `rel_y` | unsigned short | Screen-relative Y position |
| 8 | 1 | `type` | char | Object type number |
| 9 | 1 | `race` | UBYTE | Race (indexes `encounter_chart[]`) |
| 10 | 1 | `index` | char | Current animation frame image index |
| 11 | 1 | `visible` | char | On-screen visibility flag |
| 12 | 1 | `weapon` | char | Weapon type: 0=none, 1=Dirk, 2=mace, 3=sword, 4=bow, 5=wand |
| 13 | 1 | `environ` | char | Environment/terrain state (see [P1](PROBLEMS.md)) |
| 14 | 1 | `goal` | char | Current goal mode ([§2.2](#22-goal-modes)) |
| 15 | 1 | `tactic` | char | Current tactical mode ([§2.3](#23-tactical-modes)) |
| 16 | 1 | `state` | char | Motion/animation state ([§2.1](#21-motion-states)) |
| 17 | 1 | `facing` | char | Direction facing (0–7, see [§5.1](RESEARCH-input-movement.md#51-direction-encoding)) |
| 18 | 2 | `vitality` | short | Hit points; doubles as original object index for NPCs |
| 20 | 1 | `vel_x` | char | X velocity (slippery/ice physics) |
| 21 | 1 | `vel_y` | char | Y velocity (slippery/ice physics) |
| **22** | | | | **Total size** (`l_shape` in `ftale.i`) |

Terminology note: the original source comment at `fmain.c:72` labels weapon 1 as "dagger", but the in-game inventory/item text uses "Dirk" (`fmain.c:381`, `fmain.c:502`, `fmain.c:2850`). This documentation uses "Dirk" for the player-facing item name.

A commented-out `APTR source_struct` field appears in both C and assembly definitions (`ftale.h:66`, `ftale.i:21`), suggesting a removed feature.

#### Actor Array

```c
#define MAXSHAPES 25                    // fmain.c:68
struct shape anim_list[20];             // fmain.c:70
unsigned char anim_index[20];           // fmain.c:74 — depth-sort index
short anix, anix2;                      // fmain.c:75 — monster allocation count
short mdex;                             // fmain.c:76 — missile index
```

- `anim_list[0]` — always the player-controlled hero
- `anim_list[1-2]` — party members / carriers
- `anim_list[3-6]` — enemy actors (up to 4; `anix` tracks count, max 7 per `fmain.c:2064`)
- `anim_list[7-19]` — remaining slots for world objects and set-figures

`MAXSHAPES=25` governs the per-page rendering queue (`sshape[25]`), not the actor array size (`fmain.c:68` vs `fmain.c:70`).

#### Missile System

```c
struct missile {                        // fmain.c:78-85
    unsigned short abs_x, abs_y;
    char missile_type,                  // NULL, arrow, rock, 'thing', or fireball
         time_of_flight,
         speed,                         // 0 = unshot
         direction,
         archer;                        // ID of firing actor
} missile_list[6];                      // 6 missiles max
```

### 1.2 struct fpage — Double-Buffer Page State

Defined at `ftale.h:69-79` and `ftale.i:24-37`. Two instances: `fp_page1`, `fp_page2` (`fmain.c:443`).

| Field | Type | Purpose |
|-------|------|---------|
| `ri_page` | RasInfo* | Amiga RasInfo for this page |
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
| `wflag` | short | Witch effect active flag |

### 1.3 struct seq_info — Sprite Sheet Descriptor

Defined at `ftale.h:81-88` and `ftale.i:39-47`. Array: `seq_list[7]` (`fmain.c:39`).

| Field | Type | Purpose |
|-------|------|---------|
| `width` | short | Frame width in pixels |
| `height` | short | Frame height in pixels |
| `count` | short | Number of frames |
| `location` | unsigned char* | Pointer to image data |
| `maskloc` | unsigned char* | Pointer to mask data |
| `bytes` | short | Bytes per frame |
| `current_file` | short | Currently loaded file index |

Sequence type constants (`ftale.h:90`):

| Value | Name | Purpose |
|-------|------|---------|
| 0 | `PHIL` | Player character sprites |
| 1 | `OBJECTS` | World object sprites |
| 2 | `ENEMY` | Enemy sprites |
| 3 | `RAFT` | Raft/vehicle sprites |
| 4 | `SETFIG` | Set-piece figure sprites (NPCs) |
| 5 | `CARRIER` | Carrier animal sprites |
| 6 | `DRAGON` | Dragon sprites |

### 1.4 struct object — World Object Instance

Defined at `ftale.h:92-95` and `ftale.i:53-58`. 6 bytes per object; 250 objects per sector. Two arrays: `ob_listg[]`, `ob_list8[]` (`fmain.c:378`).

| Field | Type | Size | Purpose |
|-------|------|------|---------|
| `xc` | unsigned short | 2 | World X coordinate |
| `yc` | unsigned short | 2 | World Y coordinate |
| `ob_id` | char | 1 | Object type ID |
| `ob_stat` | char | 1 | Status (0=inactive, 1+=active) |

### 1.5 struct inv_item — Inventory Item Descriptor

Defined at `ftale.h:97-104`. The `inv_list[]` table has 36 entries (`fmain.c:380-418`).

| Field | Type | Purpose |
|-------|------|---------|
| `image_number` | UBYTE | Display image number |
| `xoff` | UBYTE | X offset on inventory screen |
| `yoff` | UBYTE | Y offset on inventory screen |
| `ydelta` | UBYTE | Y increment for stacking |
| `img_off` | UBYTE | Sub-image offset |
| `img_height` | UBYTE | Height of sub-image |
| `maxshown` | UBYTE | Max displayable count (also gold value for coins) |
| `name` | char* | Display name string |

#### Inventory Index Ranges

| Range | Constant | Contents | Source |
|-------|----------|----------|--------|
| 0–4 | — | Weapons: Dirk, Mace, Sword, Bow, Magic Wand | `fmain.c:427` |
| 5–8 | — | Special: Golden Lasso, Sea Shell, Sun Stone, Arrows | `fmain.c:428` |
| 9–15 | `MAGICBASE=9` | Magic items: Blue Stone, Green Jewel, Glass Vial, Crystal Orb, Bird Totem, Gold Ring, Jade Skull | `fmain.c:429` |
| 16–21 | `KEYBASE=16` | Keys: Gold, Green, Blue, Red, Grey, White | `fmain.c:430` |
| 22–24 | — | Quest/stat: Talisman, Rose, Fruit | `fmain.c:380-418` |
| 25–30 | `STATBASE=25` | Collectibles: Gold Statue, Book, Herb, Writ, Bone, Shard | `fmain.c:430` |
| 31–34 | `GOLDBASE=31` | Gold coins: 2gp, 5gp, 10gp, 100gp | `fmain.c:430` |
| 35 | `ARROWBASE=35` | Quiver of arrows | `fmain.c:430` |

Per-brother storage: `julstuff[35]`, `philstuff[35]`, `kevstuff[35]`; active inventory via `UBYTE *stuff` pointer (`fmain.c:432`).

### 1.6 struct need — Asset Loading Descriptor

Defined at `ftale.h:106-108`. Array: `file_index[10]` (one per region F1–F10, `fmain.c:615-625`).

| Field | Type | Purpose |
|-------|------|---------|
| `image[4]` | USHORT | 4 image file indices needed |
| `terra1` | USHORT | Terrain data file 1 |
| `terra2` | USHORT | Terrain data file 2 |
| `sector` | USHORT | Sector data file |
| `region` | USHORT | Region data file |
| `setchar` | USHORT | Set-character file needed |

### 1.7 struct in_work — Input Handler Data

Defined at `ftale.h:110-119`. Single instance: `handler_data` (`fmain.c:694`). Passed to the interrupt handler as `a1`.

| Offset | Field | Type | Purpose |
|--------|-------|------|---------|
| 0 | `xsprite` | short | Mouse pointer X (clamped 5–315) |
| 2 | `ysprite` | short | Mouse pointer Y (clamped 147–195) |
| 4 | `qualifier` | short | Input event qualifier (button/modifier state) |
| 6 | `laydown` | UBYTE | Keyboard buffer write pointer (0–127) |
| 7 | `pickup` | UBYTE | Keyboard buffer read pointer (0–127) |
| 8 | `newdisk` | char | Disk-inserted event flag |
| 9 | `lastmenu` | char | Last mouse-click menu character |
| 10 | `gbase` | GfxBase* | GfxBase pointer for MoveSprite |
| 14 | `pbase` | SimpleSprite* | Pointer sprite (NULL = no updates) |
| 18 | `vbase` | ViewPort* | ViewPort for MoveSprite |
| 22 | `keybuf[128]` | unsigned char | 128-byte circular keyboard buffer |
| 150 | `ticker` | short | Timer heartbeat counter (0–16) |

Byte offsets confirmed from assembly equates at `fsubs.asm:60-62` and field access patterns at `fsubs.asm:74-76`, `fsubs.asm:101`, `fsubs.asm:144`, `fsubs.asm:191-196`.

---


## 2. Actor State Machine

Actors are governed by three orthogonal state variables: **motion state** (animation/physics), **goal mode** (high-level AI objective), and **tactical mode** (low-level navigation behavior). These are stored in `shape.state`, `shape.goal`, and `shape.tactic` respectively ([§1.1](#11-struct-shape--actor-record)).

### 2.1 Motion States

Defined at `ftale.h:9-25` (canonical) and duplicated at `fmain.c:90-103`.

| Value | Name | Purpose |
|-------|------|---------|
| 0–11 | *(fighting frames)* | Combat animation sub-states; figure selected via `statelist[facing*12 + state]` |
| 12 | `WALKING` | Normal walk cycle |
| 13 | `STILL` | Stationary/idle |
| 14 | `DYING` | Death animation in progress |
| 15 | `DEAD` | Fully dead |
| 16 | `SINK` | Sinking (quicksand/water) |
| 17 | `OSCIL` | Oscillation animation 1 (comment: "and 18") — vestigial, never assigned |
| 18 | *(implicit)* | Oscillation animation 2 (paired with OSCIL) — vestigial, never assigned |
| 19 | `TALKING` | SETFIG-only: 15-tick image flicker while speech text displays |
| 20 | `FROZEN` | Frozen in place (freeze spell) |
| 21 | `FLYING` | Vestigial — defined but never assigned; swan flight does not use `state==FLYING` |
| 22 | `FALL` | Falling; velocity-based with 25% friction per tick (`fmain.c:1737-1738`) |
| 23 | `SLEEP` | Sleeping |
| 24 | `SHOOT1` | Bow up — aiming |
| 25 | `SHOOT3` | Bow fired, arrow given velocity |

`FLYING` is a dead enum value in the shipped code: no assignment site uses it. The swan/bird carrier instead uses `type == CARRIER`, `actor_file == 11`, and `riding == 11`. The hero remains in normal WALKING/STILL logic with `environ = -2` forcing inertial movement (`fmain.c:1464`, `fmain.c:1581-1596`), while the swan sprite image is selected directly from facing (`dex = d`, `fmain.c:1507`) rather than from the motion-state table.

### 2.2 Goal Modes

Defined at `ftale.h:29-39` (canonical) and duplicated at `fmain.c:107-117`.

| Value | Name | Purpose |
|-------|------|---------|
| 0 | `USER` | Player-controlled |
| 1 | `ATTACK1` | Attack stupidly (low cleverness) |
| 2 | `ATTACK2` | Attack cleverly (high cleverness) |
| 3 | `ARCHER1` | Archery attack style 1 |
| 4 | `ARCHER2` | Archery attack style 2 |
| 5 | `FLEE` | Run directly away from hero |
| 6 | `STAND` | Stand still, face hero |
| 7 | `DEATH` | Dead character |
| 8 | `WAIT` | Wait to speak to hero |
| 9 | `FOLLOWER` | Follow another character |
| 10 | `CONFUSED` | Run around randomly |

ATTACK1 vs ATTACK2 is determined by the `cleverness` field in `encounter_chart[]` — dispatched at `fmain.c:2150`.

### 2.3 Tactical Modes

Defined at `ftale.h:43-54` (canonical). `fmain.c:121-132` duplicates values 0–10 only; values 11–12 exist only in the header.

| Value | Name | Purpose |
|-------|------|---------|
| 0 | `FRUST` | Frustrated — try a different tactic |
| 1 | `PURSUE` | Move toward hero |
| 2 | `FOLLOW` | Move toward another character |
| 3 | `BUMBLE_SEEK` | Bumble around seeking target |
| 4 | `RANDOM` | Move in random direction |
| 5 | `BACKUP` | Reverse current direction |
| 6 | `EVADE` | Move 90° from hero |
| 7 | `HIDE` | Seek hiding place (planned but never implemented) |
| 8 | `SHOOT` | Shoot an arrow |
| 9 | `SHOOTFRUST` | Arrows not connecting — re-evaluate |
| 10 | `EGG_SEEK` | Snakes seeking turtle eggs |
| 11 | `DOOR_SEEK` | Dark Knight blocking door (replaced by hardcoded logic) |
| 12 | `DOOR_LET` | Dark Knight letting player pass (replaced by hardcoded logic) |

Source comment at `ftale.h:47`: "choices 2–5 can be selected randomly for getting around obstacles."

### 2.4 statelist — Animation Frame Lookup

Defined at `fmain.c:143-205`. An array of 87 `struct state` entries (`fmain.c:138-142`):

```c
struct state { char figure, wpn_no, wpn_x, wpn_y; };
```

Maps `(motion_state, facing, frame)` → `(figure_image, weapon_overlay_index, weapon_x_offset, weapon_y_offset)`.

#### Walk Sequences (8 frames each)

| Index Range | Direction | Source |
|-------------|-----------|--------|
| 0–7 | South | `fmain.c:148-149` |
| 8–15 | West | `fmain.c:152-153` |
| 16–23 | North | `fmain.c:156-157` |
| 24–31 | East | `fmain.c:160-161` |

Walk base is selected via `diroffs[16]` (`fmain.c:1010`):

```c
char diroffs[16] = {16,16,24,24,0,0,8,8,56,56,68,68,32,32,44,44};
```

Indices 0–7 select walk bases; indices 8–15 select fight/shoot bases.

#### Fight Sequences (12 states each)

| Index Range | Direction | Source |
|-------------|-----------|--------|
| 32–43 | South | `fmain.c:164-169` |
| 44–55 | West | `fmain.c:172-177` |
| 56–67 | North | `fmain.c:180-185` |
| 68–79 | East | `fmain.c:188-193` |

Each 12-entry block covers fight states 0–11: states 0–8 are weapon swing positions, state 9 duplicates a swing position, states 10–11 are ranged attack frames (bow/wand entries reference `wpn_no` indices 80+; see [P4](PROBLEMS.md)).

#### Special States

| Index | Purpose | Source |
|-------|---------|--------|
| 80–82 | Death sequence (3 frames) | `fmain.c:196` |
| 83 | Sinking | `fmain.c:198` |
| 84–85 | Oscillation (2 frames) | `fmain.c:201` |
| 86 | Asleep | `fmain.c:203` |

### 2.5 trans_list — Fight Animation Transitions

Defined at `fmain.c:136-146`. Nine entries of `struct transition`:

```c
struct transition { char newstate[4]; };    // fmain.c:136-137
```

| Index | newstate[0] | [1] | [2] | [3] | Source |
|-------|-------------|-----|-----|-----|--------|
| 0 | 1 | 8 | 0 | 1 | `fmain.c:138` |
| 1 | 2 | 0 | 1 | 0 | `fmain.c:139` |
| 2 | 3 | 1 | 2 | 8 | `fmain.c:140` |
| 3 | 4 | 2 | 3 | 7 | `fmain.c:141` |
| 4 | 5 | 3 | 4 | 6 | `fmain.c:142` |
| 5 | 6 | 4 | 5 | 5 | `fmain.c:143` |
| 6 | 8 | 5 | 6 | 4 | `fmain.c:144` |
| 7 | 8 | 6 | 7 | 3 | `fmain.c:145` |
| 8 | 0 | 6 | 8 | 2 | `fmain.c:146` |

The `newstate[0]` column forms a forward cycle: 0→1→2→3→4→5→6→8→0 (state 7 reached via `newstate[3]` and feeds back into the cycle). `newstate[1]` traverses the reverse direction. This implements the sword swing arc animation. See [P3](PROBLEMS.md) for detailed index semantics.

A random element selects which of the 4 transition paths to take: `trans_list[state].newstate[rand4()]` (`fmain.c:1712`).

### 2.6 setfig_table — NPC Type Descriptors

Defined at `fmain.c:21-37`. Maps NPC type index to image file and speech capability.

```c
struct { BYTE cfile_entry, image_base, can_talk; }
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

`cfile_entry` selects the image file (index into `seq_list` loading sequence). `image_base` is the sub-image offset within that file. `can_talk=1` enables the TALKING visual effect (see [§13.2](RESEARCH-npcs-quests.md#132-talk-system)) — it does not gate speech dispatch.

### 2.7 encounter_chart — Monster Combat Stats

Defined at `fmain.c:42-64`. Struct definition at `fmain.c:42-53`:

```c
struct encounter {
    char hitpoints, agressive, arms, cleverness, treasure, file_id;
};
```

| Index | Monster | HP | Aggressive | Arms | Cleverness | Treasure | File ID | Source |
|-------|---------|-----|------------|------|------------|----------|---------|--------|
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
| 10 | Woodcutter | 4 | 0 | 0 | 0 | 0 | 9 | `fmain.c:64` |

**Field semantics:**

- **`hitpoints`** — Base vitality assigned at spawn.
- **`agressive`** *(sic)* — Set to `TRUE` for all races except the Woodcutter (index 10), but **never read** anywhere in the codebase. Vestigial metadata; aggression is governed entirely by goal/tactic state and the extent system. See [§9.7](RESEARCH-ai-encounters.md#97-peace-zones).
- **`arms`** — Indexes into `weapon_probs[]` (`fmain2.c:860-868`): `weapon_probs[arms*4 + wt]` selects the weapon type at spawn (`fmain.c:2757-2758`), where `wt` is a per-batch (or per-spawn, see [§9.5](RESEARCH-ai-encounters.md#95-set_encounter--actor-placement-fmainc2736-2770)) column index 0–3.
- **`cleverness`** — 0 = goal ATTACK1 (stupid pursuit), 1 = goal ATTACK2 (clever pursuit with more frequent re-evaluation).
- **`treasure`** — Indexes into `treasure_probs[]` (`fmain2.c:852-858`): `treasure_probs[treasure*8 + rnd(8)]` selects loot on body search (`fmain.c:3273`).
- **`file_id`** — Image file index for loading monster sprites.

#### weapon_probs — Weapon Selection Table

Defined at `fmain2.c:860-868`. 8 groups of 4 entries (32 total). Indexed by `arms * 4 + rnd(4)`:

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

Weapon type 8 ("touch attack") is not in the standard weapon enum (0–5) and represents contact damage from monsters like Wraiths and Spiders.

#### treasure_probs — Loot Selection Table

Defined at `fmain2.c:852-858`. 5 groups of 8 entries (40 total). Indexed by `treasure * 8 + rnd(8)`:

| Group | Values | Loot Type |
|-------|--------|-----------|
| 0 | 0,0,0,0,0,0,0,0 | Nothing |
| 1 | 9,11,13,31,31,17,17,32 | Stones, vials, totems, gold, keys |
| 2 | 12,14,20,20,20,31,33,31 | Keys, skulls, gold |
| 3 | 10,10,16,16,11,17,18,19 | Magic items and keys |
| 4 | 15,21,0,0,0,0,0,0 | Jade Skull and White Key (rare) |

---


## 3. Random Number Generation

### 3.1 Algorithm

The RNG is a **Linear Congruential Generator (LCG)** implemented in 68000 assembly at `fsubs.asm:299-306`:

```
seed1 = low16(seed1) × 45821 + 1       (mulu.w produces 32-bit result)
output = ror32(seed1, 6) & 0x7FFFFFFF   (rotate, then clear sign bit)
```

Instruction-by-instruction:

| Line | Instruction | Effect |
|------|-------------|--------|
| `fsubs.asm:300` | `move.l _seed1,d0` | Load 32-bit state |
| `fsubs.asm:301` | `mulu.w #45821,d0` | Unsigned 16×16→32 multiply (uses only low 16 bits of d0) |
| `fsubs.asm:302` | `addq.l #1,d0` | Increment by 1 |
| `fsubs.asm:303` | `move.l d0,_seed1` | Store updated state |
| `fsubs.asm:304` | `ror.l #6,d0` | Rotate right 6 bits (mixes high/low bits) |
| `fsubs.asm:305` | `and.l #$7fffffff,d0` | Clear sign bit → non-negative 31-bit result |

**Critical limitation**: The 68000 `mulu.w` instruction operates on the low 16 bits of `d0` only. The upper 16 bits of `seed1` are always a deterministic function of the low 16 bits. The effective state space is 2^16, giving a maximum period of **65536** — not 2^32 (see [P7](PROBLEMS.md)).

The `ror.l #6` rotation scrambles bit positions so low state bits contribute to high output bits and vice versa, but it does not increase the period. The output is 31 bits wide from a 16-bit state, so many 31-bit values can never appear.

### 3.2 Function Family

All functions declared at `fsubs.asm:296-297` and implemented at `fsubs.asm:299-340`:

| Function | Location | Returns | Formula |
|----------|----------|---------|---------|
| `rand()` | `fsubs.asm:299-306` | 0 to 0x7FFFFFFF (31-bit) | Base LCG output |
| `bitrand(x)` | `fsubs.asm:308-310` | `rand() & x` | Masked random |
| `rand2()` | `fsubs.asm:312-314` | 0 or 1 | `rand() & 1` |
| `rand4()` | `fsubs.asm:316-318` | 0–3 | `rand() & 3` |
| `rand8()` | `fsubs.asm:320-322` | 0–7 | `rand() & 7` |
| `rand64()` | `fsubs.asm:324-326` | 0–63 | `rand() & 63` |
| `rand256()` | `fsubs.asm:328-330` | 0–255 | `rand() & 255` |
| `rnd(n)` | `fsubs.asm:332-338` | 0 to n−1 | `(rand() & 0xFFFF) % n` |

The `bitrand`/`randN` variants use bitwise AND, so they produce uniform results only when the mask is a power-of-two minus one. `rnd(n)` uses a true modulo operation via the 68000 `divu.w` instruction (`fsubs.asm:335-337`).

### 3.3 Seeding

**Initial value**: `seed1 = 19837325` (hex `0x012ED98D`), declared at `fmain.c:682`.

A second variable `seed2 = 23098324` is declared on the same line but **never referenced** anywhere in the codebase (vestigial).

**No runtime reseeding**: There is no code that writes to `seed1` other than the `_rand` function itself (`fsubs.asm:303`). The seed is not derived from system time, VBlank counter, user input timing, or any other entropy source.

The developer `notes` file contains a single line (`notes:1`): *"Need to initialize random number generator."* — indicating Talin was aware of this limitation but the TODO was never addressed.

### 3.4 Copy-Protection Entropy

The only source of sequence variation between game sessions is the copy-protection input loop at `fmain2.c:1327`:

```c
while (TRUE) { key = getkey(); ... rand(); }
```

Each keystroke iteration calls `rand()` with the result discarded. Since different players type at different speeds, the RNG state has advanced by a variable (but uncontrolled) number of steps before gameplay begins. This provides incidental — not intentional — seed variation.

### 3.5 Usage Summary

The RNG is called pervasively across combat, AI, encounters, loot, movement, sound, and visual effects. Key usage categories:

| Domain | Example | Source |
|--------|---------|--------|
| AI re-evaluation | `!bitrand(15)` — 1/16 chance per tick | `fmain.c:2132` |
| Tactic selection | `rand4()+2` (ranged), `rand2()+3` (melee) | `fmain.c:2142-2143` |
| Hit detection | `rand256() > brave` — bravery dodge check | `fmain.c:2260` |
| Encounter spawn | `rand64() <= danger_level` | `fmain.c:2085` |
| Loot generation | `rand4()` selects chest tier (0–3) | `fmain.c:3201` |
| Movement deviation | `rand4()` hunger stumble; `rand2()` direction | `fmain.c:1442-1443` |
| Fight transition | `rand4()` selects `trans_list` path | `fmain.c:1712` |
| Sound pitch | `rand256()` or `bitrand(511)` for variation | `fmain.c:1680`, `fmain2.c:239` |

---

