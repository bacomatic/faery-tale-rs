# Symbol Registry (Normative)

Every identifier used in a ` ```pseudo ` block under `docs/logic/` must be resolvable to one of:
- A function argument or local assignment in the same block.
- A function listed in the `Calls:` header of the same function.
- An entry in this registry.
- A built-in primitive from [STYLE.md §5](STYLE.md#5-primitives-the-pseudo-code-stdlib).

Changes to this file are orchestrator-reviewed. Append-mostly.

---

## 1. Constants

```pseudo
# Actor array sizing
MAXSHAPES = 25              # fmain.c:68 — render queue size (not actor array size)
MAX_ACTORS = 20             # fmain.c:70 — anim_list[] length
MAX_MONSTERS = 7            # fmain.c:2064 — concurrent hostile cap

# Weapon codes (fmain.c:77 comment, fmain2.c:231-246, fmain.c:2245-2246)
WEAPON_NONE        = 0
WEAPON_DIRK        = 1
WEAPON_MACE        = 2
WEAPON_SWORD       = 3
WEAPON_BOW         = 4
WEAPON_WAND        = 5
WEAPON_TOUCH       = 8      # fmain.c:2245 — monster-only, clamped to 5 for reach/damage
WEAPON_RANGED_BIT  = bit(2) # fmain.c:2244 — set for bow (4) and wand (5)

# Combat constants
DYING_FRAMES        = 7     # fmain.c:2773 — tactic countdown set on STATE_DYING
MISSILE_MAX_FLIGHT  = 40    # fmain.c:2274 — max ticks before a missile self-expires
ARROW_HIT_RADIUS    = 6     # fmain.c:2279
FIREBALL_HIT_RADIUS = 9     # fmain.c:2280
MONSTER_DODGE_FLOOR = 20    # fmain.c:2283 — missile dodge for non-hero targets
NEAR_MISS_MARGIN    = 2     # fmain.c:2262 — extra px band that triggers the clang
HERO_REACH_BASE     = 5     # fmain.c:2249
BRAVE_PER_REACH     = 20    # fmain.c:2249
REACH_SOFT_CAP      = 14    # fmain.c:2250
REACH_HARD_CEILING  = 15    # fmain.c:2250

# Stat deltas (aftermath / death)
LUCK_PER_DEATH           = 5   # fmain.c:2777
KIND_PER_NPC_KILL        = 3   # fmain.c:2775
AFTERMATH_BRAVELY_THRESH = 5   # fmain2.c:263
SPECIAL_XTYPE_FLOOR      = 50  # fmain2.c:264

# dohit source-channel markers (passed as i)
DOHIT_SOURCE_ARROW    = -1   # fmain2.c:238
DOHIT_SOURCE_FIREBALL = -2   # fmain2.c:239

# checkdead dtype codes — narr.asm message-table indices
DTYPE_HIT_AND_KILLED = 5   # fmain2.c:246 / narr.asm:16
DTYPE_DROWNED        = 6
DTYPE_BURNED         = 7
DTYPE_STONED         = 8

# Missile-type enum (struct missile.missile_type)
MISSILE_INACTIVE = 0
MISSILE_ARROW    = 1
MISSILE_FIREBALL = 2
MISSILE_SPENT    = 3   # fmain.c:2295 — fireball puff frame

# stuff[] slot indices (inventory / quest flags)
STUFF_SUNSTONE = 7   # fmain2.c:233 — zero means no Sunstone
STUFF_FOOD     = 24  # fmain.c:2221 — food ration slot (auto-eat in safe zone)

# sample[] indices passed to effect(sample, pitch) — fmain.c:3616
SFX_PLAYER_HIT   = 0   # fmain2.c:240
SFX_NEAR_MISS    = 1   # fmain.c:2262
SFX_ARROW_HIT    = 2   # fmain2.c:238
SFX_MONSTER_HIT  = 3   # fmain2.c:241
SFX_BOW_RELEASE  = 4   # (fired by shoot_step)
SFX_FIREBALL_HIT = 5   # fmain2.c:239

# Door-type enum subset (fmain.c doorlist)
DOOR_CAVE   = 4
DOOR_DESERT = 6

# Region / stat offsets
STATBASE = 16   # fmain2.c:1602 — first 6 stats live at stuff[STATBASE..STATBASE+5]

# Movement / terrain physics (fmain.c walking/ice/lava blocks)
ICE_VEL_CAP_DEFAULT = 42    # fmain.c:1582 — max |vel_y| on ice
ICE_VEL_CAP_SWAN    = 40    # fmain.c:1582 — swan terminal velocity
SPEED_NORMAL        = 2     # fmain.c:1602 — default walking speed
SPEED_SLOW          = 1     # fmain.c:1602 — wading/deep-water speed
SPEED_FAST          = 4     # fmain.c:1601 — slippery-terrain speed
SPEED_BACKWARDS     = -2    # fmain.c:1600 — lava walk-backward speed
SPEED_RAFT          = 3     # fmain.c:1599 — hero on turtle/raft
VEL_DISPL_MUL       = 4     # fmain.c:1646-1647 — vel stored as displacement*4
WORLD_COORD_MASK    = 0x7fff # fsubs.asm:1295 — 15-bit world coordinate mask
COORD_FLAG_BIT      = 0x8000 # fsubs.asm:1316 — bit 15 of abs_y preserved (semantics unresolved, see PROBLEMS)
DIR_MASK            = 7     # fmain.c — 8-direction wrap mask
DIRECTION_STILL     = 9     # fmain.c:1010, fmain2.c:165 — com2 sentinel "no movement"
ACTOR_BBOX_HALF_X   = 11    # fmain2.c:289 — actor collision half-width
ACTOR_BBOX_HALF_Y   = 9     # fmain2.c:289 — actor collision half-height
COLLIDE_ACTOR       = 16    # fmain2.c:289 — proxcheck return code for actor-actor collision
ICE_VEL_ACCUM_BASE  = 20    # fmain.c:1583 — ice velocity accumulator base (newx(20,d,2)-20 == xdir[d])

# Riding modes (global `riding`)
RIDING_NONE = 0
RIDING_RAFT = 5             # fmain.c:1599 — hero on turtle/raft
RIDING_SWAN = 11            # fmain.c:1582 — hero on swan (ice physics)

# set_course mode constants
COURSE_MODE_DIRECT     = 0
COURSE_MODE_PURSUE     = 1   # fmain2.c:138 — deviate when close
COURSE_MODE_FOLLOW     = 2   # fmain2.c:146 — deviate when close
COURSE_MODE_BACKUP     = 3   # fmain2.c:149 — reverse direction
COURSE_MODE_BUMBLE     = 4   # fmain2.c:113 — skip axis snap
COURSE_MODE_NOWALK     = 5   # fmain2.c:186 — set facing only
COURSE_MODE_RAW_VECTOR = 6   # fmain2.c:79 — use target_x/y as raw delta
PURSUE_DEVIATE_DIST    = 40  # fmain2.c:138
FOLLOW_DEVIATE_DIST    = 30  # fmain2.c:146

# Terrain codes (high nibble of terra_mem[id*4+1]; fsubs.asm:614)
TERRAIN_OPEN        = 0     # passable
TERRAIN_BLOCKED     = 1     # impassable
TERRAIN_WATER_SHAL  = 2
TERRAIN_WATER_MED   = 3
TERRAIN_WATER_DEEP  = 4     # ramps environ toward 10
TERRAIN_WATER_VDEEP = 5     # ramps toward 30, can drown
TERRAIN_SLIPPERY    = 6     # environ -1
TERRAIN_ICE         = 7     # environ -2, velocity-based
TERRAIN_LAVA        = 8     # environ -3, walk-backwards; NPCs blocked
TERRAIN_PIT         = 9     # fall trigger if xtype==52 & i==0
TERRAIN_CRYSTAL     = 12    # fmain.c:1611 — passable with stuff[30] shard
TERRAIN_DOOR        = 15    # fmain.c:1609 — triggers doorfind()

# Environ codes (actor.environ)
ENVIRON_NORMAL    = 0
ENVIRON_SLIP      = -1      # terrain 6
ENVIRON_ICE       = -2      # terrain 7 / pit fall
ENVIRON_LAVA      = -3      # terrain 8
ENVIRON_WADE      = 2       # terrain 2
ENVIRON_BRUSH     = 5       # terrain 3
ENVIRON_DEEP_SAT  = 10      # ramp target for terrain 4
ENVIRON_SINK      = 15      # threshold for STATE_SINK
ENVIRON_DROWN     = 30      # death-depth threshold

# Pit/drain linkage
PITFALL_XTYPE     = 52      # fmain.c:1767
PITFALL_LUCK_COST = 2       # fmain.c:1772
DRAIN_SINK_SECTOR = 181     # fmain.c:1785
DRAIN_DEST_REGION = 9       # fmain.c:1788
DRAIN_DEST_X      = 0x1080  # fmain.c:1789
DRAIN_DEST_Y      = 34950   # fmain.c:1789

# Encounter pipeline (fmain.c:2058-2770, fmain2.c:1714-1720)
MAX_TRY                  = 15         # fmain.c:2736 — set_encounter retry cap
ENCOUNTER_SPREAD         = 63         # fmain.c:2065, 2744 — default jitter box
ENCOUNTER_RETRY_LIMIT    = 10         # fmain.c:2061 — cluster-point attempts per 14i tick
PLACE_CADENCE_MASK       = 15         # fmain.c:2058 — 14i cadence
ROLL_CADENCE_MASK        = 31         # fmain.c:2080 — 14j cadence
SPECIAL_EXTENT_FLOOR     = 50         # fmain.c:2059, 2081
BIOME_UNIFORM_MASK       = 3          # fmain.c:2060 — (xtype & 3)==0 disables mixflag
INDOOR_DANGER_BIAS       = 5          # fmain.c:2082
OUTDOOR_DANGER_BIAS      = 2          # fmain.c:2083
DANGER_ROLL_RANGE        = 63         # fmain.c:2085
SETLOC_RING_MIN_DIST     = 150        # fmain2.c:1717
SETLOC_RING_MAX_DELTA    = 63         # fmain2.c:1717
MIXFLAG_PAIR_BIT         = bit(1)     # fmain.c:2754
MIXFLAG_REROLL_WEAPON    = bit(2)     # fmain.c:2756
XTYPE_SWAMP              = 7          # fmain.c:2087
XTYPE_SPIDER             = 8          # fmain.c:2089
XTYPE_WRAITH_FORCED      = 49         # fmain.c:2090 — reserved
XTYPE_ASTRAL             = 52         # fmain.c:2696, 2746
ASTRAL_VOID_TERRAIN      = 7          # fmain.c:2746
DKNIGHT_PIN_X            = 21635      # fmain.c:2741
DKNIGHT_PIN_Y            = 25762      # fmain.c:2741
DKNIGHT_RACE_FILTER      = 7          # fmain.c:2741
SPRITE_ANCHOR_OFFSET_X   = 8          # fmain.c:2765
SPRITE_ANCHOR_OFFSET_Y   = 26         # fmain.c:2766
DISK_CHAN_ACTOR_SHAPES   = 8          # fmain.c:2052, fmain2.c:745
WEAPON_PROBS_COLUMNS     = 4          # fmain.c:2757
TREASURE_PROBS_COLUMNS   = 8          # fmain.c:3272
SETFIG_RACE_BIT          = 0x80       # fmain.c:3271 — bit 7 marks setfigs (no loot)

# Brother succession / revive
ARROWBASE                = 35         # fmain.c:429 — per-brother inventory array size
STARTING_DIRK            = 1          # fmain.c — stuff[] dirk slot initial count
TAMBRY_SPAWN_X           = 19036      # fmain.c:2893 — hero respawn world X
TAMBRY_SPAWN_Y           = 15755      # fmain.c:2894 — hero respawn world Y
TAMBRY_REGION            = 3          # fmain.c:2895 — starting region
RAFT_INIT_X              = 13668      # fmain.c:2860 — raft carrier world X at new game
RAFT_INIT_Y              = 14470      # fmain.c:2861 — raft carrier world Y at new game
SETFIG_INIT_Y            = 15000      # fmain.c:2869 — initial setfig ob_listg Y
MAP_CAMERA_OFFSET_X      = 144        # fmain.c:2883 — map-coord centering offset X
MAP_CAMERA_OFFSET_Y      = 90         # fmain.c:2884 — map-coord centering offset Y
VIT_BASE                 = 15         # fmain.c:2854 — base HP for revive, priest heal
VIT_BRAVE_DIV            = 4          # fmain.c:2854 — brave/4 HP bonus term
DAYNIGHT_RESET           = 8000       # fmain.c:2898 — daynight timer at revive
LIGHTLEVEL_RESET         = 300        # fmain.c:2899 — lightlevel at revive
VIEWSTATUS_CORRUPT       = 99         # fmain2.c:1544 — save-load failure marker
VIEWSTATUS_PLACARD       = 2          # fmain.c:2871 — game-over placard mode
VIEWSTATUS_PLAYFIELD     = 3          # fmain.c:2911 — normal play view mode
PRINCESS_OBJ_SLOT        = 9          # fmain.c:3397 — ob_list8 princess-captive slot
BONES_OBJ_ID             = 28         # fmain.c:3174 — bones world object type id
GHOST_OFFSET             = 2          # fmain.c — ghost-sprite anim offset for bones
OB_STAT_BONES_ON_GROUND  = 1          # fmain.c:3177 — bones ground state
OB_STAT_SETFIG_ACTIVE    = 3          # fmain.c:2867 — setfig active state
PLACARD_HOLD_TICKS       = 120        # fmain.c — death-placard hold duration
PLACARD_GAP_TICKS        = 80         # fmain.c — placard gap duration
PLACARD_CLEAR_TICKS      = 10         # fmain.c — placard clear duration
GAME_OVER_DELAY          = 500        # fmain.c — delay before all-dead quit
ACTOR_FILE_BROTHER       = 6          # fmain.c:2889 — brother actor shape file id
SET_FILE_TAMBRY          = 13         # fmain.c:2889 — Tambry setfig file id
ANIX_DEFAULT             = 3          # fmain.c:2904 — default anim-index for brother
GAME_OVER_THRESHOLD      = 3          # fmain.c:2871 — brother>=3 → quit
GOLDBASE                 = 34         # fmain.c — stuff[] gold slot index
```

## 2. Enums

### 2.1 Directions (`fsubs.asm:*`, see RESEARCH §5.1)

```pseudo
DIR_NW = 0
DIR_N  = 1
DIR_NE = 2
DIR_E  = 3
DIR_SE = 4
DIR_S  = 5
DIR_SW = 6
DIR_W  = 7
```

### 2.2 Goal modes (`ftale.h:27-37`)

```pseudo
GOAL_USER      = 0    # User-controlled
GOAL_ATTACK1   = 1    # Attack (stupid)
GOAL_ATTACK2   = 2    # Attack (clever)
GOAL_ARCHER1   = 3    # Archery (stupid)
GOAL_ARCHER2   = 4    # Archery (clever)
GOAL_FLEE      = 5
GOAL_STAND     = 6
GOAL_DEATH     = 7
GOAL_WAIT      = 8
GOAL_FOLLOWER  = 9
GOAL_CONFUSED  = 10
```

### 2.3 Motion states (`ftale.h:10-23`, `fmain.c:90-103`)

```pseudo
STATE_FIGHTING = 0          # ftale.h:12
STATE_WALKING  = 12         # ftale.h:10
STATE_STILL    = 13         # ftale.h:11
STATE_DYING    = 14         # ftale.h:13
STATE_DEAD     = 15         # ftale.h:14
STATE_SINK     = 16         # ftale.h:15
STATE_OSCIL    = 17         # ftale.h:16 — also 18
STATE_TALKING  = 19         # ftale.h:18
STATE_FROZEN   = 20         # ftale.h:19
STATE_FLYING   = 21         # ftale.h:20
STATE_FALL     = 22         # ftale.h:20
STATE_SLEEP    = 23         # ftale.h:21
STATE_SHOOT1   = 24         # ftale.h:22 — bow up / aiming
STATE_SHOOT3   = 25         # ftale.h:23 — bow fired, arrow given velocity
```

### 2.4 Menu modes (`fmain.c:494`)

```pseudo
CMODE_ITEMS = 0
CMODE_MAGIC = 1
CMODE_TALK  = 2
CMODE_BUY   = 3
CMODE_GAME  = 4
CMODE_SAVEX = 5
CMODE_KEYS  = 6
CMODE_GIVE  = 7
CMODE_USE   = 8
CMODE_FILE  = 9
```

### 2.5 Tactical modes (`fmain.c:122-132`, `ftale.h:39-50`)

```pseudo
TACTIC_FRUST       = 0      # fmain.c:122 — all tactics frustrated, pick something else
TACTIC_PURSUE      = 1      # fmain.c:123
TACTIC_FOLLOW      = 2      # fmain.c:124
TACTIC_BUMBLE_SEEK = 3      # fmain.c:125
TACTIC_RANDOM      = 4      # fmain.c:126
TACTIC_BACKUP      = 5      # fmain.c:127
TACTIC_EVADE       = 6      # fmain.c:128
TACTIC_HIDE        = 7      # fmain.c:129
TACTIC_SHOOT       = 8      # fmain.c:130
TACTIC_SHOOTFRUST  = 9      # fmain.c:131
TACTIC_EGG_SEEK    = 10     # fmain.c:132 — snakes seeking turtle eggs
```

### 2.6 Actor/sequence types (`ftale.h:88`)

```pseudo
# enum sequences { PHIL, OBJECTS, ENEMY, RAFT, SETFIG, CARRIER, DRAGON };
PHIL    = 0
OBJECTS = 1
ENEMY   = 2
RAFT    = 3
SETFIG  = 4
CARRIER = 5
DRAGON  = 6
```

### 2.7 Monster & setfig race codes (`fmain.c:51-62`, `fmain.c:35-36`)

```pseudo
# ENEMY races — indices into encounter_chart[]
RACE_OGRE        = 0
RACE_ORCS        = 1
RACE_WRAITH      = 2
RACE_SKELETON    = 3
RACE_SNAKE       = 4
RACE_SALAMANDER  = 5
RACE_SPIDER      = 6
RACE_DKNIGHT     = 7
RACE_LORAII      = 8
RACE_NECROMANCER = 9
RACE_WOODCUTTER  = 10

# SETFIG races (bit 7 set; index into setfig_table)
RACE_WITCH       = 0x89
RACE_SPECTRE     = 0x8a
RACE_GHOST       = 0x8b
RACE_PRINCESS    = 0x84
RACE_BEGGAR      = 0x8d
```

## 3. Bitfield flags

### 3.1 Menu entry `enabled[i]` encoding (`fmain.c:1310-1328`, `fmain.c:512-513`)

```pseudo
MENU_FLAG_SELECTED  = bit(0)    # Highlight / on
MENU_FLAG_VISIBLE   = bit(1)    # Displayed
MENU_ATYPE_MASK     = 0xfc      # Upper 6 bits = action type

# Action type values (upper 6 bits, shifted out):
ATYPE_NAV       = 0     # Top-bar nav (switch cmode if hit<5)
ATYPE_TOGGLE    = 4     # XOR bit 0, then do_option
ATYPE_IMMEDIATE = 8     # Highlight then do_option
ATYPE_ONESHOT   = 12    # Set bit 0, highlight, do_option
```

## 4. Structs

### 4.1 `Shape` — actor record (`ftale.h:56-67`, `ftale.i:5-22`)

```pseudo
struct Shape:
    abs_x: u16          # World X
    abs_y: u16          # World Y
    rel_x: u16          # Screen X
    rel_y: u16          # Screen Y
    type: i8
    race: u8            # Indexes TABLE:encounter_chart
    index: i8           # Animation frame
    visible: i8
    weapon: i8          # 0=none, 1=Dirk, 2=mace, 3=sword, 4=bow, 5=wand
    environ: i8
    goal: i8            # GOAL_* enum
    tactic: i8          # Tactical mode
    state: i8           # Motion state
    facing: i8          # DIR_* enum
    vitality: i16       # HP; observable wrap (negative = dead)
    vel_x: i8
    vel_y: i8
```

### 4.2 `MenuEntry` (derived from `struct menu`, `fmain.c:517-520`)

```pseudo
struct MenuEntry:
    label: str          # 5-char fixed-width label
    enabled: u8         # See §3.1

struct Menu:
    entries: list[MenuEntry]    # Length = num
    num: u8
    color: u8
```

## 5. Globals

```pseudo
player: Shape                       # anim_list[0] (fmain.c:70)
anim_list: list[Shape]              # Length MAX_ACTORS
anix: i16                           # Active monster count
menus: list[Menu]                   # Length 10 (fmain.c:517-520)
cmode: i8                           # Current menu mode (CMODE_*)
leader: i8                          # 0 = hero alive, non-zero = brother succession active
hit: i8                             # Last hit menu entry index (fmain.c)
hitgo: i8                           # Latched action trigger
real_options: list[i8]              # fmain.c:515 — length 12; screen-slot → menus[cmode].enabled[] index (-1 = empty)
letter_list: list                   # fmain.c:533-547 — TABLE:letter_list; struct letters { letter, menu, choice }
keydir: i16                         # fmain.c:1289 — current latched keypad direction code (0 = none)
keyfight: bool                      # fmain.c:1291 — '0' held-down fight-mode latch
viewstatus: i8                      # fmain.c:584 — 0 = normal playfield; nonzero = transient full-screen (map, inventory, placard)
cheat1: bool                        # fmain.c:558 — debug cheat enable
handler_data: object                # fmain.c — input-handler shared state; .lastmenu, .qualifier, .pickup, .laydown
hero_x: u16                         # fmain.c:558 — hero world X (shorthand for anim_list[0].abs_x)
hero_y: u16                         # fmain.c:558 — hero world Y
daynight: u16                       # fmain.c:572 — free-running tick counter; low bits drive periodic checks
battleflag: bool                    # fmain.c:588 — at least one hostile is on-screen this tick
actors_on_screen: bool              # fmain.c:585 — any non-hero actor within ±300 px of hero this tick
xtype: u16                          # fmain.c:575 — current extent's encounter type (0 = normal; >59 = special)
extn: object                        # fmain.c:338 — pointer to the currently-active extent_list[] entry; .v3 = race filter
turtle_eggs: bool                   # fmain.c:134 — turtle-eggs-delivered flag
encounter_chart: list               # fmain.c:52 — TABLE:encounter_chart; row per race with .arms .cleverness .hitpoints .treasure .file_id
nearest_person: i16                 # fmain.c — index into anim_list of the closest live actor within 50 px of hero (0 = none)
goodfairy: u8                       # fmain.c:592 — fairy resurrection counter; 1 = immediate revive, <120 shows fairy sprite
brave: i16                          # fmain.c — hero bravery; +1 per kill (checkdead)
luck: i16                           # fmain.c — hero luck; -5 per player death
kind: i16                           # fmain.c — hero kindness; -3 per SETFIG/NPC kill
stuff: list[u8]                     # fmain.c — inventory / quest-item counters; indexed by STUFF_* and inv_list slots
freeze_timer: i16                   # fmain.c:577 — time-stop remaining ticks; nonzero freezes all but hero
missile_list: list                  # fmain.c:2270 — missile[0..5]; fields: missile_type, speed, direction, archer, abs_x, abs_y, time_of_flight
sample: list                        # fmain.c:3616 — SFX buffer indexed by SFX_*
encounter_number: i16               # fmain.c — pending-encounter counter
actors_loading: bool                # fmain.c — async enemy-load in progress
active_carrier: i16                 # fmain.c:574 — non-zero if any carrier actor is active
region_num: u16                     # fmain.c:614 — currently loaded region (0..9)
last_person: i16                    # fmain.c — last greeted NPC race (for proximity narration)
hunger: i16                         # fmain.c:565 — 0..~150, climbs every 128 daynight ticks
ob_list8: list                      # fmain.c — inside-region object list (ob_table[8])
witchflag: bool                     # fmain.c:591 — witch is active
map_x: i16                          # fmain.c — camera world X (top-left of visible playfield)
map_y: i16                          # fmain.c — camera world Y
riding: i8                          # fmain.c:563 — mount code (RIDING_*)
raftprox: u8                        # fmain.c:1459 — 0/1/2 hero proximity to raft/turtle
frustflag: i8                       # fmain.c — hero frustration counter (scratch-head anim)
bumped: i8                          # fmain.c:1609 — hero door-nudge latch
hero_sector: u8                     # fmain.c — current sector index under hero
new_region: i16                     # fmain.c:614 — pending region-transfer target
brother: i8                         # fmain.c — hero identity (0 Julian / 1 Phillip / 2 Kevin)
fallstates: list                    # fmain.c — fall-animation frames per brother
encounter_x: u16                    # fmain.c — cluster-origin x for current spawn batch
encounter_y: u16                    # fmain.c — cluster-origin y for current spawn batch
encounter_type: i16                 # fmain.c — pending race code for load_actors
mixflag: i32                        # fmain.c — per-batch race/weapon mixing flags (bits 1, 2)
wt: i8                              # fmain.c — weapon_probs[] column index for current batch
danger_level: i16                   # fmain.c:2082-2083 — scratch var for 14j roll
actor_file: i8                      # fmain.c — currently loaded enemy shape file id
nextshape: object                   # fmain.c — destination chip-RAM pointer for next read_shapes()
seq_list: list                      # fmain2.c:43 — seq_info[7]; .location .maskloc .width .height .count
princess: i16                       # fmain.c:568 — princess rescue counter (0..3)
wealth: i32                         # fmain.c — hero gold
quitflag: bool                      # fmain.c:590 — main-loop termination latch
fatigue: i16                        # fmain.c — hero rest meter; climbs with time, reset by tavern sleep
dayperiod: i16                      # fmain.c — coarse day-segment index driving bartender branch
nearest: i16                        # fmain.c — side-effect output of nearest_fig: anim_list index of closest live actor within queried radius (0 = none)
safe_x: u16                         # fmain.c:558 — safe-point (last rest) world X
safe_y: u16                         # fmain.c:559 — safe-point world Y
safe_r: u16                         # fmain.c:559 — safe-point region
lightlevel: u16                     # fmain.c:571 — current light/brightness counter
secret_timer: i16                   # fmain.c:577 — countdown for secret events
light_timer: i16                    # fmain.c:577 — countdown for lighting changes
fiery_death: bool                   # fmain.c — fiery-death-box latch
julstuff: list                      # fmain.c:432 — Julian's per-brother inventory array (ARROWBASE size)
philstuff: list                     # fmain.c:432 — Phillip's inventory array
kevstuff: list                      # fmain.c:432 — Kevin's inventory array
blist: list                         # fmain.c:2806-2812 — Bro[3] per-brother stats array
hero_place: u16                     # fmain.c:569 — current place id for hero
tfont: object                       # fmain.c — text (hi-res) TextFont
rp_text: object                     # fmain.c — hi-res text RastPort
wcarry: i8                          # fmain.c:563,1456 — anim_list index of closest carrier this frame (1 or 3)
cycle: u16                          # fmain.c — global animation tick; low bits drive walk-cycle frame selection
ob_listg: list                      # fmain2.c — global object table entries (11 slots incl. scratch [0])
rp: object                          # fmain.c — shared drawing RastPort
afont: object                       # fmain.c — Amber font TextFont
fp_drawing: object                  # fmain.c — current drawing page (FaceRec)
vp_page: object                     # fmain.c — playfield ViewPort
vp_text: object                     # fmain.c — hi-res status-bar ViewPort
bm_draw: object                     # fmain.c — scratch BitMap pointer for drawing page
blackcolors: list                   # fmain.c — 32-entry all-black palette
sun_colors: list                    # fmain2.c:1569-1578 — 53-entry sunrise gradient
fader: list                         # fmain.c — 32-entry palette scratch buffer
screen_size: i16                    # fmain.c — playfield height passed to ScreenSize()
```

## 6. Table references

Every `TABLE:name` used in any pseudo-code block must appear here with a concrete resolution target.

| Name | Resolves to | Notes |
|---|---|---|
| `TABLE:encounter_chart` | [RESEARCH §8](../RESEARCH.md#8-encounters--monster-spawning) | Race stats keyed by `Shape.race` |
| `TABLE:menu_options` | [RESEARCH §13](../RESEARCH.md#13-menu-system) | Per-mode label + enabled byte template |
| `TABLE:item_effects` | [RESEARCH §6](../RESEARCH.md#6-items--inventory) | Inventory slot effects |
| `TABLE:narr_messages` | `narr.asm` | Indexed by `speak(N)` |
| `TABLE:key_bindings` | [logic/menu-system.md](menu-system.md) | Keycode → action map |
| `TABLE:letter_list` | `fmain.c:533-547` | Keyboard-shortcut table: `(letter, menu, choice)` rows consumed by `key_dispatch` |
| `TABLE:movement_vectors_x` | `fsubs.asm:1276` | `xdir[0..9]`: `-2,0,2,3,2,0,-2,-3,0,0` |
| `TABLE:movement_vectors_y` | `fsubs.asm:1277` | `ydir[0..9]`: `-2,-3,-2,0,2,3,2,0,0,0` |
| `TABLE:movement_course_map` | `fmain2.c:55` | `com2[0..8]`: `{0,1,2,7,9,3,6,5,4}`; `(ydir,xdir) → dir or 9=still` |
| `TABLE:fall_states` | `fmain.c` | Per-brother fall-animation frame indices (6 entries each) |
| `TABLE:weapon_probs` | `fmain2.c:860-868` | 8 rows × 4 cols; indexed by `encounter.arms * 4 + wt` |
| `TABLE:treasure_probs` | `fmain2.c:852-858` | 5 rows × 8 cols; indexed by `encounter.treasure * 8 + rand8` |
| `TABLE:blist` | `fmain.c:2806-2812` | 3-row `Bro {brave,luck,kind,wealth,stuff}` per-brother stats array |

*(Additional entries appended as new logic docs are authored.)*

## 7. KeyCode values

Values in the input ring buffer are not raw Amiga rawkey scancodes: the input
handler's `keytrans` table (`fsubs.asm:281-300`) translates each rawkey into one
of four encodings before it reaches `getkey()`:

- ASCII for printable character keys (`'I'`, `'1'`, `' '`, `'?'`, …).
- Small integers `1..4` for cursor arrows and `10..19` for F1–F10
  (`fsubs.asm:240-241`).
- Small integers `20..29` for the numeric keypad, consumed as `keydir`
  (`fmain.c:1287-1290`).
- Synthetic codes `0x61..0x6c` produced on left-mouse-down over the menu
  strip (`fsubs.asm:138-152`), indicating which of the 12 menu slots was hit.

All codes may have bit 7 (`0x80`) OR-ed in to denote a key-up / mouse-up event
(`fsubs.asm:96-98`).

```pseudo
KEY_UP_BIT       = 0x80       # fsubs.asm:96 — OR-ed into code on key-up / mouse-up
KEY_CODE_MASK    = 0x7f       # fmain.c:1290 — mask that strips the up-bit
MOUSE_MENU_BASE  = 0x61       # fsubs.asm:147 — first synthetic menu-slot code ('a'); slots are 0x61..0x6c
KEY_SPACE        = 32         # fmain.c:1345 — ' '; only letter that fires while paused
KEY_FIGHT_DOWN   = 48         # fmain.c:1291 — '0' held-down triggers fight mode
KEY_DIGIT_1      = 49         # fmain.c:1342 — '1', first of the 6-key range on the KEYS submenu
KEY_DIGIT_6      = 54         # fmain.c:1342 — '6', last of the KEYS digit range
KEY_KEYDIR_LO    = 20         # fmain.c:1289 — keypad keydir range low
KEY_KEYDIR_HI    = 29         # fmain.c:1289 — keypad keydir range high
```
