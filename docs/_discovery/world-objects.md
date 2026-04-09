# Discovery: World Objects & Region Object Lists

**Status**: complete
**Investigated**: 2026-04-05
**Requested by**: orchestrator
**Prompt summary**: Trace the complete world object system — struct object, all region lists, management arrays, per-tick processing (do_objects), region loading (set_objects), object state mutation (change_object / leave_item), and the full object ID registry.

## struct object

Defined at `ftale.h:92-95`:

```c
struct object {             /* 250 objects, for a start */
    unsigned short  xc, yc;
    char    ob_id, ob_stat;
};
```

| Field | Type | Meaning |
|-------|------|---------|
| `xc` | `unsigned short` | World X coordinate (pixel-space, 0–65535) |
| `yc` | `unsigned short` | World Y coordinate (pixel-space, 0–65535) |
| `ob_id` | `char` | Object type identifier (see Object ID Registry below) |
| `ob_stat` | `char` | Object status code (see ob_stat Values below) |

Size: 6 bytes per entry (`sizeof(struct object)` = 6).

### ob_stat Values

Defined in comment at `fmain2.c:998`:

| Value | Meaning | Effect in set_objects |
|-------|---------|----------------------|
| 0 | Non-existent / disabled | Skipped (`goto loopend`) — `fmain2.c:1262` |
| 1 | On ground (pickable item) | Rendered as OBJECTS type with `an->race=1` — `fmain2.c:1291` |
| 2 | In inventory / taken | Skipped (`goto loopend`) — `fmain2.c:1262` |
| 3 | Setfig (NPC character) | Processed as SETFIG type with `an->state=STILL` — `fmain2.c:1265-1282` |
| 4 | Dead setfig | Processed as SETFIG type with `an->state=DEAD` — `fmain2.c:1275` |
| 5 | Hidden object (revealed by Look) | Rendered as OBJECTS type with `an->race=0` — `fmain2.c:1290` |
| 6 | Cabinet item | Rendered as OBJECTS type with `an->race=2` — `fmain2.c:1289` |

## Object ID Registry (enum obytes)

Defined at `fmain2.c:967-977`:

```c
enum obytes {
    QUIVER=11,
    MONEY=13, URN, CHEST, SACKS, G_RING, B_STONE, G_JEWEL,
        SCRAP, C_ORB, VIAL, B_TOTEM, J_SKULL,
    GOLD_KEY=25, GREY_KEY=26,
    FOOTSTOOL=31,
    TURTLE=102,
    BLUE_KEY=114,
    M_WAND=145, MEAL, ROSE, FRUIT, STATUE, BOOK, SHELL,
    GREEN_KEY=153, WHITE_KEY=154, RED_KEY=242,
};
```

Full value mapping:

| Value | Constant | Description |
|-------|----------|-------------|
| 0 | (wizard) | Setfig NPC — wizard (ob_stat=3) |
| 1 | (priest) | Setfig NPC — priest (ob_stat=3) |
| 2 | (guard) | Setfig NPC — guard (ob_stat=3) |
| 3 | (guard back) | Setfig NPC — guard facing back (ob_stat=3) |
| 4 | (princess) | Setfig NPC — princess (ob_stat=3) |
| 5 | (king) | Setfig NPC — king (ob_stat=3) |
| 6 | (noble) | Setfig NPC — noble (ob_stat=3) |
| 7 | (sorceress) | Setfig NPC — sorceress (ob_stat=3) |
| 8 | (bartender) | Setfig NPC — bartender (ob_stat=3) |
| 9 | (witch) | Setfig NPC — witch (ob_stat=3) |
| 10 | (spectre) | Setfig NPC — spectre (ob_stat=3) |
| 11 | QUIVER | Quiver of arrows (inventory item) |
| 13 | MONEY | 50 gold pieces — `fmain.c:3157` |
| 14 | URN | Brass urn (container) — `fmain.c:3182` |
| 15 | CHEST | Chest (container) — `fmain.c:3181` |
| 16 | SACKS | Sacks (container) — `fmain.c:3183` |
| 17 | G_RING | Gold ring |
| 18 | B_STONE | Blue stone |
| 19 | G_JEWEL | Green jewel |
| 20 | SCRAP | Scrap of paper — triggers event(17), event(18 or 19) — `fmain.c:3163-3167` |
| 21 | C_ORB | Crystal orb |
| 22 | VIAL | Vial |
| 23 | B_TOTEM | Bird totem |
| 24 | J_SKULL | Jade skull |
| 25 | GOLD_KEY | Gold key |
| 26 | GREY_KEY | Grey key |
| 27 | (lasso) | Lasso — used raw value 27 in `itrans[]` entry `27,5` — `fmain2.c:982` |
| 28 | (dead brother) | Dead brother's bones — `fmain.c:3172-3176` |
| 29 | 0x1d | Opened/empty chest — `fmain2.c:1208`, `fmain.c:3184` |
| 31 | FOOTSTOOL | Footstool — cannot be taken (`break`) — `fmain.c:3186` |
| 102 | TURTLE | Turtle eggs — cannot be taken (`break`) — `fmain.c:3170` |
| 114 | BLUE_KEY | Blue key |
| 128+10 | (king's bone) | King's bone — `fmain2.c:1174` (value 138 = 128+10) |
| 128+27 | (sunstone) | Sunstone — `fmain2.c:1095` (value 155 = 27+128) |
| 136 | (mapped via itrans) | itrans maps 136→27 (stuff index) — `fmain2.c:983` |
| 137 | (mapped via itrans) | itrans maps 137→28 — `fmain2.c:983` |
| 138 | (mapped via itrans) | itrans maps 138→29 — `fmain2.c:983` |
| 139 | (talisman) | Talisman — dropped by sorceress on death via `leave_item(i,139)` — `fmain.c:1754` |
| 140 | (shard) | Crystal shard — dropped when giving bone to spectre via `leave_item(nearest_person,140)` — `fmain.c:3503` |
| 145 | M_WAND | Magic wand |
| 146 | MEAL | Meal — defined in enum but never placed in any object list |
| 147 | ROSE | Rose |
| 148 | FRUIT | Fruit — eaten if `hunger<15` (`stuff[24]++; event(36)`), else `eat(30)` — `fmain.c:3166-3169` |
| 149 | STATUE | Gold statue |
| 150 | BOOK | Book |
| 151 | SHELL | Sea shell |
| 153 | GREEN_KEY | Green key |
| 154 | WHITE_KEY | White key |
| 242 | RED_KEY | Red key |

### itrans[] — Object ID to Inventory Index Translation

Defined at `fmain2.c:979-985`. Maps `ob_id` values to `stuff[]` inventory indices:

```c
UBYTE itrans[] = {
    QUIVER,35,
    B_STONE,9,G_JEWEL,10,VIAL,11,C_ORB,12,B_TOTEM,13,G_RING,14,J_SKULL,15,
    M_WAND,4, 27,5, 8,2, 9,1, 12,0, 10,3, ROSE,23, FRUIT,24, STATUE,25,
    BOOK,26, SHELL,6, 155,7, 136,27, 137,28, 138,29, 139,22, 140,30,
    GOLD_KEY,16,GREEN_KEY,17,BLUE_KEY,18,RED_KEY,19,GREY_KEY,20,WHITE_KEY,21,
    0,0 };
```

Used in the Take handler at `fmain.c:3187-3196` to look up `stuff[]` index from `ob_id`.

## Region Object Lists

### ob_listg — Global Objects (11 entries)

Defined at `fmain2.c:1001-1012`. `glbobs = 11` — `fmain2.c:1180`.

| Index | xc | yc | ob_id | ob_stat | Description |
|-------|----|----|-------|---------|-------------|
| 0 | 0 | 0 | 0 | 0 | Special slot — used by `leave_item()` for dropped items |
| 1 | 0 | 0 | 28 | 0 | Dead brother 1 — coordinates filled in at runtime |
| 2 | 0 | 0 | 28 | 0 | Dead brother 2 — coordinates filled in at runtime |
| 3 | 19316 | 15747 | 11 | 0 | Ghost brother 1 — setfig (ghost, ob_id=11) |
| 4 | 18196 | 15735 | 11 | 0 | Ghost brother 2 — setfig (ghost, ob_id=11) |
| 5 | 12439 | 36202 | 10 | 3 | Spectre — can be disabled (ob_id=10 maps to spectre setfig) |
| 6 | 11092 | 38526 | STATUE | 1 | Gold statue — seahold |
| 7 | 25737 | 10662 | STATUE | 1 | Gold statue — ogre den |
| 8 | 2910 | 39023 | STATUE | 1 | Gold statue — octal room |
| 9 | 12025 | 37639 | STATUE | 0 | Gold statue — sorceress (initially hidden, set to 1 by talking to sorceress) |
| 10 | 6700 | 33766 | STATUE | 0 | Gold statue — priest (initially hidden, set to 1 by presenting writ to priest) |

**Runtime mutations**:
- `ob_listg[0]`: Overwritten by `leave_item()` to drop items — `fmain2.c:1191-1195`
- `ob_listg[1-2]`: Dead brothers get coordinates set during brother succession — `fmain.c:2839-2840`
- `ob_listg[3-4]`: Ghost brothers become visible (stat=3) during succession — `fmain.c:2841`; stat set to 0 when bones are picked up — `fmain.c:3174`
- `ob_listg[5]`: Spectre visibility tied to day/night: stat=3 if `lightlevel<40`, stat=2 otherwise — `fmain.c:2027-2028`
- `ob_listg[9]`: Sorceress statue revealed (stat=1) when talking to sorceress — `fmain.c:3403`
- `ob_listg[10]`: Priest statue revealed (stat=1) when presenting writ (stuff[28]) to priest — `fmain.c:3384-3385`

### ob_list0 — Snow Land (3 + 10 blank = 13 entries)

Defined at `fmain2.c:1013-1018`. `mapobs[0] = 3`.

| Index | xc | yc | ob_id | ob_stat | Description |
|-------|----|----|-------|---------|-------------|
| 0 | 3340 | 6735 | 12 | 3 | Ranger west (setfig) |
| 1 | 9678 | 7035 | 12 | 3 | Ranger east (setfig) |
| 2 | 4981 | 6306 | 12 | 3 | Ranger north (setfig) |
| 3–12 | 0 | 0 | 0 | 0 | TENBLANKS — available for random treasure |

### ob_list1 — Maze Forest North (1 + 10 blank = 11 entries)

Defined at `fmain2.c:1019-1023`. `mapobs[1] = 1`.

| Index | xc | yc | ob_id | ob_stat | Description |
|-------|----|----|-------|---------|-------------|
| 0 | 23087 | 5667 | TURTLE | 1 | Turtle eggs |
| 1–10 | 0 | 0 | 0 | 0 | TENBLANKS |

### ob_list2 — Swamp Land (5 + 10 blank = 15 entries)

Defined at `fmain2.c:1024-1031`. `mapobs[2] = 5`.

| Index | xc | yc | ob_id | ob_stat | Description |
|-------|----|----|-------|---------|-------------|
| 0 | 13668 | 15000 | 0 | 3 | Wizard (setfig) |
| 1 | 10627 | 13154 | 0 | 3 | Wizard (setfig) |
| 2 | 4981 | 10056 | 12 | 3 | Ranger (setfig) |
| 3 | 13950 | 11087 | SACKS | 1 | Sacks (container) |
| 4 | 10344 | 36171 | SHELL | 1 | Sea shell |
| 5–14 | 0 | 0 | 0 | 0 | TENBLANKS |

### ob_list3 — Maze Forest South / Manor / Tambry (12 + 10 blank = 22 entries)

Defined at `fmain2.c:1032-1048`. `mapobs[3] = 12`.

| Index | xc | yc | ob_id | ob_stat | Description |
|-------|----|----|-------|---------|-------------|
| 0 | 19298 | 16128 | CHEST | 1 | Chest (container) |
| 1 | 18310 | 15969 | 13 | 3 | Beggar (setfig) |
| 2 | 20033 | 14401 | 0 | 3 | Wizard (setfig) |
| 3 | 24794 | 13102 | 13 | 3 | Beggar (setfig) |
| 4 | 21626 | 15446 | B_STONE | 1 | Blue stone — at stone ring |
| 5 | 21616 | 15456 | MONEY | 1 | Money — at stone ring |
| 6 | 21636 | 15456 | G_RING | 1 | Gold ring — at stone ring |
| 7 | 20117 | 14222 | G_JEWEL | 1 | Green jewel |
| 8 | 24185 | 9840 | SACKS | 1 | Sacks (container) |
| 9 | 25769 | 10617 | MONEY | 1 | Money |
| 10 | 25678 | 10703 | B_STONE | 1 | Blue stone |
| 11 | 17177 | 10599 | SCRAP | 1 | Scrap of paper |
| 12–21 | 0 | 0 | 0 | 0 | TENBLANKS |

### ob_list4 — Desert (3 + 10 blank = 13 entries)

Defined at `fmain2.c:1049-1054`. `mapobs[4] = 3`.

| Index | xc | yc | ob_id | ob_stat | Description |
|-------|----|----|-------|---------|-------------|
| 0 | 0 | 0 | 0 | 0 | Dummy |
| 1 | 0 | 0 | 0 | 0 | Dummy |
| 2 | 6817 | 19693 | 13 | 3 | Beggar — "must be 3rd object" per comment |
| 3–12 | 0 | 0 | 0 | 0 | TENBLANKS |

### ob_list5 — Farm and City (5 + 10 blank = 15 entries)

Defined at `fmain2.c:1055-1062`. `mapobs[5] = 5`.

| Index | xc | yc | ob_id | ob_stat | Description |
|-------|----|----|-------|---------|-------------|
| 0 | 22184 | 21156 | 13 | 3 | Beggar (setfig) |
| 1 | 18734 | 17595 | G_RING | 1 | Gold ring |
| 2 | 21294 | 22648 | CHEST | 1 | Chest (container) |
| 3 | 22956 | 19955 | 0 | 3 | Wizard (setfig) |
| 4 | 28342 | 22613 | 0 | 3 | Wizard (setfig) |
| 5–14 | 0 | 0 | 0 | 0 | TENBLANKS |

### ob_list6 — Lava Plain (1 + 10 blank = 11 entries)

Defined at `fmain2.c:1063-1066`. `mapobs[6] = 1`.

| Index | xc | yc | ob_id | ob_stat | Description |
|-------|----|----|-------|---------|-------------|
| 0 | 24794 | 13102 | 13 | 3 | Dummy object (beggar position, same coords as ob_list3[3]) |
| 1–10 | 0 | 0 | 0 | 0 | TENBLANKS |

### ob_list7 — Southern Mountains (1 + 10 blank = 11 entries)

Defined at `fmain2.c:1067-1070`. `mapobs[7] = 1`.

| Index | xc | yc | ob_id | ob_stat | Description |
|-------|----|----|-------|---------|-------------|
| 0 | 23297 | 5797 | TURTLE | 1 | Dummy object (turtle, same coords as commented-out ob_list1 entry) |
| 1–10 | 0 | 0 | 0 | 0 | TENBLANKS |

### ob_list8 — Building Interiors (61+16 = 77 entries)

Defined at `fmain2.c:1071-1168`. `mapobs[8] = 61+16 = 77`.

This is the largest list. The first 61 entries are regular objects/NPCs; the last 16 are "look" items (ob_stat=5, hidden until LOOK action reveals them).

**Setfig NPCs (ob_stat=3):**

| Index | xc | yc | ob_id | ob_stat | Description |
|-------|----|----|-------|---------|-------------|
| 0 | 6700 | 33756 | 1 | 3 | Priest in chapel |
| 1 | 5491 | 33780 | 5 | 3 | King on throne |
| 2 | 5592 | 33764 | 6 | 3 | Noble |
| 3 | 5514 | 33668 | 2 | 3 | Guard |
| 4 | 5574 | 33668 | 2 | 3 | Guard |
| 5 | 8878 | 38995 | 0 | 3 | Wizard |
| 6 | 7776 | 34084 | 0 | 3 | Wizard |
| 7 | 5514 | 33881 | 3 | 3 | Guard (back-facing) |
| 8 | 5574 | 33881 | 3 | 3 | Guard (back-facing) |
| 9 | 10853 | 35656 | 4 | 3 | Princess |
| 10 | 12037 | 37614 | 7 | 3 | Sorceress |
| 11 | 11013 | 36804 | 9 | 3 | Witch |
| 12 | 9631 | 38953 | 8 | 3 | Bartender |
| 13 | 10191 | 38953 | 8 | 3 | Bartender |
| 14 | 10649 | 38953 | 8 | 3 | Bartender |
| 15 | 2966 | 33964 | 8 | 3 | Bartender |

**Ground items (ob_stat=1):**

| Index | xc | yc | ob_id | ob_stat | Description |
|-------|----|----|-------|---------|-------------|
| 16 | 9532 | 40002 | FOOTSTOOL | 1 | Footstool |
| 17 | 6747 | 33751 | FOOTSTOOL | 1 | Footstool |
| 18 | 11410 | 36169 | 27+128 (155) | 1 | Sunstone |
| 19 | 9550 | 39964 | B_TOTEM | 1 | Bird totem — cabinet item |
| 20 | 9552 | 39964 | B_TOTEM | 1 | Bird totem |
| 21 | 9682 | 39964 | B_TOTEM | 1 | Bird totem — cabinet item |
| 22 | 9684 | 39964 | B_TOTEM | 1 | Bird totem |
| 23 | 9532 | 40119 | B_TOTEM | 1 | Bird totem — on table |
| 24 | 9575 | 39459 | URN | 1 | Brass urn |
| 25 | 9590 | 39459 | URN | 1 | Brass urn |
| 26 | 9605 | 39459 | URN | 1 | Brass urn |
| 27 | 9680 | 39453 | VIAL | 1 | Vial |
| 28 | 9682 | 39453 | VIAL | 1 | Vial |
| 29 | 9784 | 39453 | VIAL | 1 | Vial |
| 30 | 9668 | 39554 | CHEST | 1 | Chest (container) |
| 31 | 11090 | 39462 | MONEY | 1 | Money |
| 32 | 11108 | 39458 | B_TOTEM | 1 | Bird totem |
| 33 | 11118 | 39459 | B_TOTEM | 1 | Bird totem |
| 34 | 11128 | 39459 | B_TOTEM | 1 | Bird totem |
| 35 | 11138 | 39458 | B_TOTEM | 1 | Bird totem |
| 36 | 11148 | 39459 | B_TOTEM | 1 | Bird totem |
| 37 | 11158 | 39459 | B_TOTEM | 1 | Bird totem |
| 38 | 11855 | 36206 | FOOTSTOOL | 1 | Footstool |
| 39 | 11909 | 36198 | CHEST | 1 | Chest (container) |
| 40 | 11918 | 36246 | B_TOTEM | 1 | Bird totem — cabinet items |
| 41 | 11928 | 36246 | B_TOTEM | 1 | Bird totem |
| 42 | 11938 | 36246 | B_TOTEM | 1 | Bird totem |
| 43 | 12212 | 38481 | CHEST | 1 | Chest (container) |
| 44 | 11652 | 38481 | RED_KEY | 1 | Red key |
| 45 | 10427 | 39977 | FOOTSTOOL | 1 | Footstool |
| 46 | 10323 | 40071 | URN | 1 | Brass urn |
| 47 | 10059 | 38472 | SACKS | 1 | Sacks |
| 48 | 10344 | 36171 | SHELL | 1 | Sea shell |
| 49 | 11936 | 36207 | SCRAP | 1 | Scrap of paper — spectre note |
| 50 | 9674 | 35687 | URN | 1 | Brass urn |
| 51 | 5473 | 38699 | ROSE | 1 | Rose |
| 52 | 7185 | 34342 | FRUIT | 1 | Fruit |
| 53 | 7190 | 34342 | FRUIT | 1 | Fruit |
| 54 | 7195 | 34342 | FRUIT | 1 | Fruit |
| 55 | 7185 | 34347 | FRUIT | 1 | Fruit |
| 56 | 7190 | 34347 | FRUIT | 1 | Fruit |
| 57 | 7195 | 34347 | FRUIT | 1 | Fruit |
| 58 | 6593 | 34085 | FRUIT | 1 | Fruit |
| 59 | 6598 | 34085 | FRUIT | 1 | Fruit |
| 60 | 6593 | 34090 | FRUIT | 1 | Fruit |
| 61 | 6598 | 34090 | FRUIT | 1 | Fruit — note: comment says 61 fixed entries (index 0–60) |

**Hidden "Look" items (ob_stat=5, indices 61–76):**

| Index | xc | yc | ob_id | ob_stat | Description |
|-------|----|----|-------|---------|-------------|
| 61 | 3872 | 33546 | GOLD_KEY | 5 | Gold key (hidden) |
| 62 | 3887 | 33510 | B_TOTEM | 5 | Bird totem (hidden) |
| 63 | 4495 | 33510 | VIAL | 5 | Vial (hidden) |
| 64 | 3327 | 33383 | J_SKULL | 5 | Jade skull (hidden) |
| 65 | 4221 | 34119 | QUIVER | 5 | Quiver (hidden) |
| 66 | 7610 | 33604 | VIAL | 5 | Vial (hidden) |
| 67 | 7616 | 33522 | MONEY | 5 | Money (hidden) |
| 68 | 9570 | 35768 | B_STONE | 5 | Blue stone (hidden) |
| 69 | 9668 | 35769 | QUIVER | 5 | Quiver (hidden) |
| 70 | 9553 | 38951 | G_RING | 5 | Gold ring (hidden) |
| 71 | 10062 | 39005 | J_SKULL | 5 | Jade skull (hidden) |
| 72 | 10577 | 38951 | VIAL | 5 | Vial (hidden) |
| 73 | 11062 | 39514 | MONEY | 5 | Money (hidden) |
| 74 | 8845 | 39494 | WHITE_KEY | 5 | White key (hidden) |
| 75 | 6542 | 39494 | G_JEWEL | 5 | Green jewel (hidden) |
| 76 | 7313 | 38992 | RED_KEY | 5 | Red key (hidden) |

### ob_list9 — Underground Areas (9 entries)

Defined at `fmain2.c:1169-1177`. `mapobs[9] = 9`.

| Index | xc | yc | ob_id | ob_stat | Description |
|-------|----|----|-------|---------|-------------|
| 0 | 7540 | 38528 | M_WAND | 1 | Magic wand |
| 1 | 9624 | 36559 | M_WAND | 1 | Magic wand |
| 2 | 9624 | 37459 | M_WAND | 1 | Magic wand |
| 3 | 8337 | 36719 | M_WAND | 1 | Magic wand |
| 4 | 8154 | 34890 | CHEST | 1 | Chest (container) |
| 5 | 7826 | 35741 | CHEST | 1 | Chest (container) |
| 6 | 3460 | 37260 | 0 | 3 | Wizard (setfig) |
| 7 | 8485 | 35725 | MONEY | 1 | Money |
| 8 | 3723 | 39340 | 128+10 (138) | 1 | King's bone (maps to stuff[29] via itrans) |

## Object Management Arrays

### ob_table[10]

Defined at `fmain2.c:1178-1179`. Maps region numbers (0–9) to their object list:

```c
struct object *ob_table[10] = {
    ob_list0, ob_list1,
    ob_list2, ob_list3,
    ob_list4, ob_list5,
    ob_list6, ob_list7,
    ob_list8, ob_list9 };
```

### mapobs[10]

Defined at `fmain2.c:1181`. Tracks the **current count** of valid entries in each region's object list:

```c
short mapobs[10] = { 3, 1, 5, 12, 3, 5, 1, 1, 61+16, 9 };
```

| Region | Initial Count | Region Name |
|--------|---------------|-------------|
| 0 | 3 | Snow land |
| 1 | 1 | Maze forest north |
| 2 | 5 | Swamp land |
| 3 | 12 | Maze forest south / Manor / Tambry |
| 4 | 3 | Desert |
| 5 | 5 | Farm and city |
| 6 | 1 | Lava plain |
| 7 | 1 | Southern mountains |
| 8 | 77 (61+16) | Building interiors |
| 9 | 9 | Underground areas |

This array is **mutable** — when random objects are distributed during first region load, `mapobs[region_num]` is incremented for each new object — `fmain2.c:1234`.

### dstobs[10]

Defined at `fmain2.c:1182`. Tracks whether random treasure has been distributed in each region:

```c
short dstobs[10] = { 0, 0, 0, 0, 0, 0, 0, 0, 1, 1 };
```

- `0`: Region has not yet received random treasure
- `1`: Region has been distributed (or is excluded: regions 8 and 9 start as 1, meaning interiors/underground never get random scatter)

Set to 1 after distribution at `fmain2.c:1238`.

### glbobs

Defined at `fmain2.c:1180`:

```c
short glbobs = 11;
```

Count of entries in `ob_listg[]`.

## do_objects — Per-Tick Processing

Defined at `fmain2.c:1184-1189`. Called once per game tick from the main loop at `fmain.c:2304`.

```c
do_objects()
{   j1 = 2;
    set_objects(ob_listg,glbobs,0x80);
    set_objects(ob_table[region_num],mapobs[region_num],0);
    if (j1>3) anix = j1;
}
```

**Flow**:
1. Sets global variable `j1 = 2` — this is the starting anim_list index for setfig NPCs (indices 0=hero, 1=raft/carrier are reserved) — `fmain2.c:1185`
2. Calls `set_objects(ob_listg, glbobs, 0x80)` — processes the 11 global objects with flag `0x80` (marks them as global in `an->vitality`) — `fmain2.c:1186`
3. Calls `set_objects(ob_table[region_num], mapobs[region_num], 0)` — processes regional objects with flag `0` — `fmain2.c:1187`
4. If `j1 > 3`, updates `anix` (first non-setfig actor index) to `j1` — `fmain2.c:1188`

**Context**: Called at `fmain.c:2304` after `anix2 = anix` resets the objects-and-items portion of the anim_list. The anim_list layout is:
- Index 0: hero
- Index 1: raft/carrier
- Indices 2..anix-1: setfig NPCs (managed by j1)
- Indices anix..anix2-1: enemies/objects (managed by anix2)

## set_objects — Region Load / Rendering

Defined at `fmain2.c:1218-1301`. Called by `do_objects()` twice per tick — once for globals, once for regional objects.

### Random Treasure Distribution

Condition at `fmain2.c:1225`: `if (dstobs[region_num] == 0 && new_region >= 10)`

When a region is loaded for the first time (and `new_region >= 10` — meaning the game has loaded at least 10 regions to this point): 10 random objects are scattered across the region — `fmain2.c:1226-1238`.

```c
for (i=0; i<10; i++)
{   do
    {   xstart = bitrand(0x3fff) + ((region_num & 1) * 0x4000);
        ystart = bitrand(0x1fff) + ((region_num & 6) * 0x1000);
    } while (px_to_im(xstart,ystart));
    k = mapobs[region_num]++;
    l2[k].xc = xstart;
    l2[k].yc = ystart;
    l2[k].ob_id = rand_treasure[bitrand(15)];
    l2[k].ob_stat = 1;
}
dstobs[region_num] = 1;
```

- Random position within the region's quadrant, avoiding non-zero terrain (loops `while (px_to_im(xstart,ystart))` to find traversable terrain)
- Random object type from `rand_treasure[]` — `fmain2.c:987-992`
- Appended to end of region's object list; `mapobs[]` count incremented
- `dstobs[]` set to 1 to prevent re-distribution

### rand_treasure[] — Random Loot Table

Defined at `fmain2.c:987-992`:

```c
UBYTE rand_treasure[] = {
    SACKS, SACKS, SACKS, SACKS,
    CHEST, MONEY, GOLD_KEY, QUIVER,
    GREY_KEY, GREY_KEY, GREY_KEY, RED_KEY,
    B_TOTEM, VIAL, WHITE_KEY, CHEST
};
```

Distribution: 4/16 SACKS, 3/16 GREY_KEY, 2/16 CHEST, 1/16 each for MONEY, GOLD_KEY, QUIVER, RED_KEY, B_TOTEM, VIAL, WHITE_KEY.

### Per-Object Processing Loop

For each object in the list (`fmain2.c:1240-1301`):

1. **Screen bounds check**: `xstart = list->xc - map_x - 8; ystart = list->yc - map_y - 8` — `fmain2.c:1243-1244`

2. **Setfig NPC handling** (ob_stat == 3 or 4):
   - Extended visibility check: within 100–400 pixel range of screen — `fmain2.c:1247`
   - Loads appropriate character sprite file via `setfig_table[id]` — `fmain2.c:1250-1256`
   - If witch (ob_id==9, ob_stat==3): sets `witchflag = TRUE` — `fmain2.c:1258`
   - Tight visibility check: within 20–340 × 20–190 pixels — `fmain2.c:1263`
   - Creates anim_list entry at index `j1++` with:
     - `type = SETFIG` — `fmain2.c:1270`
     - `index = setfig_table[id].image_base` — `fmain2.c:1272`
     - `vitality = 2 + id + id` (2 + 2×ob_id) — `fmain2.c:1274`
     - `goal = i` (object list index) — `fmain2.c:1275`
     - `race = id + 0x80` — `fmain2.c:1279`
     - If previously dead (ob_stat==4): `an->state = DEAD` — `fmain2.c:1278`
   - If actor already at this position and DEAD: updates ob_stat to 4 — `fmain2.c:1268-1269`

3. **Skipped objects**: ob_stat 0 or 2 are skipped — `fmain2.c:1262`

4. **Item/object handling** (ob_stat 1, 5, or 6):
   - Tight visibility check: within -20..340 × -20..190 pixels — `fmain2.c:1263`
   - If turtle eggs: records position in `turtle_eggs = anix2` — `fmain2.c:1284`
   - Creates anim_list entry at index `anix2++` with:
     - `type = OBJECTS` — `fmain2.c:1285`
     - `index = list->ob_id` — `fmain2.c:1286`
     - `vitality = i + f` (object index + global flag) — `fmain2.c:1288`
     - Race based on ob_stat: 6→race=2, 5→race=0, else→race=1 — `fmain2.c:1289-1291`

5. **Resource limit**: if `anix2 >= 20`, return early — `fmain2.c:1242`

### Object Race Values in anim_list

For OBJECTS type entries, `an->race` encodes interaction behavior:
- `race = 0` (ob_stat 5): Hidden item — not pickable, only revealed by LOOK — used in Look handler at `fmain.c:3293-3295`
- `race = 1` (ob_stat 1): Normal ground item — pickable via Take
- `race = 2` (ob_stat 6): Cabinet item — rendering only, not interactable in standard way
- `race = 0xff`: Missile (set in main loop at `fmain.c:2319`, not from set_objects)

## change_object — Object State Mutation

Defined at `fmain2.c:1200-1208`. Called when taking an object or using LOOK.

```c
change_object(id,flag) register long id, flag;
{   register struct shape *an;
    register struct object *ob;
    an = anim_list + id;
    id = an->vitality & 0x07f;
    if (an->vitality & 0x080) ob = ob_listg + id;
    else ob = ob_table[region_num] + id;

    if (ob->ob_id==CHEST) ob->ob_id = 0x1d; else ob->ob_stat = flag;
}
```

**Parameters**:
- `id`: anim_list index of the object actor
- `flag`: new ob_stat value (1 = reveal, 2 = taken/consumed)

**Logic**:
1. Looks up the anim_list entry to get the object's `vitality` — `fmain2.c:1203`
2. `vitality & 0x7f` gives the index into the object list — `fmain2.c:1204`
3. `vitality & 0x80` determines whether it's a global object (from ob_listg) or regional — `fmain2.c:1205`
4. **Special case**: If the object is a CHEST (ob_id==15), changes `ob_id` to `0x1d` (29, opened/empty chest) instead of changing `ob_stat` — `fmain2.c:1208`
5. Otherwise: sets `ob_stat = flag` — `fmain2.c:1208`

**Callers**:
- Take handler: `change_object(nearest, 2)` — marks object as taken — `fmain.c:3242`
- Look handler: `change_object(i, flag = 1)` — reveals hidden objects (ob_stat 5→1) — `fmain.c:3294`

## leave_item — Drop Item in World

Defined at `fmain2.c:1191-1196`.

```c
leave_item(i,object) short i,object;
{   ob_listg[0].xc = anim_list[i].abs_x;
    ob_listg[0].yc = anim_list[i].abs_y + 10;
    ob_listg[0].ob_id = object;
    ob_listg[0].ob_stat = 1;
}
```

Always uses `ob_listg[0]` (the special global slot). Sets coordinates to actor `i`'s position (with +10 Y offset), assigns the object type, and makes it visible (stat=1).

**Callers**:
- Necromancer death drops talisman: `leave_item(i, 139)` — `fmain.c:1754`
- Witch (race 0x89) death drops lasso: `leave_item(i, 27)` — `fmain.c:1756`
- Give bone to spectre drops shard: `leave_item(nearest_person, 140)` — `fmain.c:3503`

**Limitation**: Only one dropped item can exist at a time since it always uses `ob_listg[0]`. Each new call overwrites the previous dropped item.

## Treasure Containers

The Take handler at `fmain.c:3148-3241` processes containers (CHEST, URN, SACKS) with random loot:

### Container Types

| ob_id | Value | Container Name | Citation |
|-------|-------|---------------|----------|
| CHEST | 15 (0x0f) | "a chest" | `fmain.c:3181` |
| URN | 14 (0x0e) | "a brass urn" | `fmain.c:3182` |
| SACKS | 16 (0x10) | "some sacks" | `fmain.c:3183` |
| 0x1d | 29 | Empty chest — `break` (cannot interact) | `fmain.c:3184` |

### Container Loot Generation

When a container is opened at `fmain.c:3200`:

```c
k = rand4();
```

| k | Result | Logic |
|---|--------|-------|
| 0 | "nothing" | `fmain.c:3201` |
| 1 | 1 random item | `rand8() + 8` → inventory index, clamp index 8→ARROWBASE — `fmain.c:3203-3207` |
| 2 | 2 random items | Two `rand8()+8` draws, with gold amount special case — `fmain.c:3210-3220` |
| 3 | 3 of same item or 3 random keys | If index==8, gives 3 random keys; else 3 of same item — `fmain.c:3222-3235` |

After looting, `change_object(nearest, 2)` marks the object as taken. For chests specifically, `change_object` converts `ob_id` from CHEST(15) to 0x1d(29) rather than changing ob_stat, leaving an empty chest visual.

## Save/Load

Object state is fully persisted in save games — `fmain2.c:1522-1527`:

```c
saveload((void *)ob_listg,glbobs * (sizeof (struct object)));
saveload((void *)mapobs,20);
saveload((void *)dstobs,20);
for (i=0; i<10; i++)
    saveload((void *)ob_table[i],mapobs[i] * (sizeof (struct object)));
```

Save order:
1. `ob_listg` — 66 bytes (11 × 6)
2. `mapobs` — 20 bytes (10 × 2) — current counts including any random additions
3. `dstobs` — 20 bytes (10 × 2) — distribution flags
4. All 10 regional lists — variable size based on current `mapobs[i]` count

## Cross-Cutting Findings

- **Day/night affects object visibility**: `ob_listg[5]` (spectre) toggles between stat=3 (visible NPC) and stat=2 (hidden) based on `lightlevel` — `fmain.c:2027-2028`
- **Brother succession modifies global objects**: On death, coordinates of dead brother written to `ob_listg[brother]`, ghost brother activated at `ob_listg[brother+2]` — `fmain.c:2839-2841`
- **Princess rescue tracked via ob_list8[9].ob_stat**: Princess entry in interiors list; its stat is checked for dialogue changes (king speaks differently, princess speaks differently) — `fmain.c:2099,2684,2843,3397-3398`
- **Witch detection via set_objects**: The `witchflag` global is set during `set_objects()` when witch NPC (ob_id==9, ob_stat==3) is within extended range — `fmain2.c:1258`. This flag affects safe zone tracking (`fmain.c:2190`), witch visual effects (`fmain.c:2371-2372`), and Sun Stone USE action (`fmain.c:3462`).
- **Turtle eggs tracked via set_objects**: `turtle_eggs` variable is set to anix2 value when TURTLE object is found on screen — `fmain2.c:1284`. This triggers snake AI behavior (EGG_SEEK tactic) and `get_turtle()` call after combat — `fmain.c:2150`, `fmain2.c:274`.
- **nearest_fig skips OBJECTS and empty chests**: At `fmain2.c:301-302`, `nearest_fig()` skips actors of type OBJECTS (and specifically 0x1d empty chests) unless `constraint==0`, meaning NPCs and enemies are preferred targets for targeting.
- **Terrain interaction**: `px_to_im()` is used in random treasure placement to ensure objects land on traversable terrain (value 0) — `fmain2.c:1232`.
- **Object limit**: Maximum 20 anim_list entries total (`anix2 >= 20` check at `fmain2.c:1242`), shared between setfigs, enemies, and item objects.

## Unresolved

None — all questions in the prompt have been answered with source citations.

## Refinement Log
- 2026-04-05: Initial discovery — complete trace of world object system across fmain2.c and fmain.c
