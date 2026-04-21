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

### 2.3 Motion states

```pseudo
# Populated in Wave 3 (see spec §6.4). Placeholder header only.
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

*(Additional entries appended as new logic docs are authored.)*

## 7. KeyCode values

```pseudo
# Raw Amiga rawkey codes used by menu dispatch. See fmain.c:1240-1290.
# Populated in Task 15 when menu-system.md is authored.
```
