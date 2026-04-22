# Discovery: Inventory System & Item Definitions

**Status**: complete
**Investigated**: 2026-04-05
**Requested by**: orchestrator
**Prompt summary**: Trace the complete inventory system including inv_list, stuff[], itrans[], jtrans[], do_option item handling, ppick, announce functions, item effects, quest items, and weapon equipping.

## inv_list — Complete Item Table

Defined at `fmain.c:380-424`. The struct is `struct inv_item` (`ftale.h:95-102`):

```c
struct inv_item {
    UBYTE  image_number;  /* what image number to use */
    UBYTE  xoff, yoff;    /* x and y offset on image screen */
    UBYTE  ydelta;        /* y increment value */
    UBYTE  img_off, img_height; /* what part of the image to draw */
    UBYTE  maxshown;      /* maximum number that can be shown */
    char   *name;
};
```

### Complete Table (36 entries + 1 alias)

| Index | image_number | xoff | yoff | ydelta | img_off | img_height | maxshown | Name |
|-------|-------------|------|------|--------|---------|------------|----------|------|
| 0 | 12 | 10 | 0 | 0 | 0 | 8 | 1 | Dirk |
| 1 | 9 | 10 | 10 | 0 | 0 | 8 | 1 | Mace |
| 2 | 8 | 10 | 20 | 0 | 0 | 8 | 1 | Sword |
| 3 | 10 | 10 | 30 | 0 | 0 | 8 | 1 | Bow |
| 4 | 17 | 10 | 40 | 0 | 8 | 8 | 1 | Magic Wand |
| 5 | 27 | 10 | 50 | 0 | 0 | 8 | 1 | Golden Lasso |
| 6 | 23 | 10 | 60 | 0 | 8 | 8 | 1 | Sea Shell |
| 7 | 27 | 10 | 70 | 0 | 8 | 8 | 1 | Sun Stone |
| 8 | 3 | 30 | 0 | 3 | 7 | 1 | 45 | Arrows |
| 9 | 18 | 50 | 0 | 9 | 0 | 8 | 15 | Blue Stone |
| 10 | 19 | 65 | 0 | 6 | 0 | 5 | 23 | Green Jewel |
| 11 | 22 | 80 | 0 | 8 | 0 | 7 | 17 | Glass Vial |
| 12 | 21 | 95 | 0 | 7 | 0 | 6 | 20 | Crystal Orb |
| 13 | 23 | 110 | 0 | 10 | 0 | 9 | 14 | Bird Totem |
| 14 | 17 | 125 | 0 | 6 | 0 | 5 | 23 | Gold Ring |
| 15 | 24 | 140 | 0 | 10 | 0 | 9 | 14 | Jade Skull |
| 16 | 25 | 160 | 0 | 5 | 0 | 5 | 25 | Gold Key |
| 17 | 25 | 172 | 0 | 5 | 8 | 5 | 25 | Green Key |
| 18 | 114 | 184 | 0 | 5 | 0 | 5 | 25 | Blue Key |
| 19 | 114 | 196 | 0 | 5 | 8 | 5 | 25 | Red Key |
| 20 | 26 | 208 | 0 | 5 | 0 | 5 | 25 | Grey Key |
| 21 | 26 | 220 | 0 | 5 | 8 | 5 | 25 | White Key |
| 22 | 11 | 0 | 80 | 0 | 8 | 8 | 1 | Talisman |
| 23 | 19 | 0 | 90 | 0 | 8 | 8 | 1 | Rose |
| 24 | 20 | 0 | 100 | 0 | 8 | 8 | 1 | Fruit |
| 25 | 21 | 232 | 0 | 10 | 8 | 8 | 5 | Gold Statue |
| 26 | 22 | 0 | 110 | 0 | 8 | 8 | 1 | Book |
| 27 | 8 | 14 | 80 | 0 | 8 | 8 | 1 | Herb |
| 28 | 9 | 14 | 90 | 0 | 8 | 8 | 1 | Writ |
| 29 | 10 | 14 | 100 | 0 | 8 | 8 | 1 | Bone |
| 30 | 12 | 14 | 110 | 0 | 8 | 8 | 1 | Shard |
| 31 | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 2 Gold Pieces |
| 32 | 0 | 0 | 0 | 0 | 0 | 0 | 5 | 5 Gold Pieces |
| 33 | 0 | 0 | 0 | 0 | 0 | 0 | 10 | 10 Gold Pieces |
| 34 | 0 | 0 | 0 | 0 | 0 | 0 | 100 | 100 Gold Pieces |
| 35 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | quiver of arrows |

**Index group constants** — `fmain.c:426-430`:
- `MAGICBASE = 9` — first magic consumable (Blue Stone)
- `KEYBASE = 16` — first key (Gold Key)
- `STATBASE = 25` — Gold Statue
- `GOLDBASE = 31` — first gold entry (items 0–30 are "real" inventory)
- `ARROWBASE = 35` — quiver-of-arrows alias

### Display Logic

The inventory screen is rendered in `do_option` case `ITEMS`, hit==5 — `fmain.c:3114-3145`:
- Iterates `j = 0` to `GOLDBASE` (indices 0–30 only; gold pieces are never drawn)
- For each item, draws `min(stuff[j], inv_list[j].maxshown)` copies
- Each copy is a 16-pixel-wide blit from the OBJECTS image sheet
- Position: `(xoff+20, yoff)`, incrementing Y by `ydelta` per copy
- Image source: row = `image_number * 80 + img_off`, height = `img_height`

## stuff[] Array — Inventory Storage

**Declaration** — `fmain.c:432`:
```c
UBYTE *stuff, julstuff[ARROWBASE], philstuff[ARROWBASE], kevstuff[ARROWBASE];
```

- `stuff` is a pointer to the current brother's inventory array.
- `ARROWBASE = 35`, so each array has 35 elements (indices 0–34).
- Three static arrays: `julstuff[35]`, `philstuff[35]`, `kevstuff[35]`.
- `stuff` pointer is set via `blist[brother-1].stuff` — `fmain.c:2848`, `fmain.c:3628`.

### Brother Initialization Table

`struct bro blist[]` — `fmain.c:2807-2812`:
```c
struct bro {
    char brave, luck, kind, wealth;
    UBYTE *stuff;
} blist[] = {
    { 35,20,15,20,julstuff },   // Julian
    { 20,35,15,15,philstuff },  // Phillip
    { 15,20,35,10,kevstuff }    // Kevin
};
```

### Storage Semantics

- **stuff[0..4]**: Weapons — count of each (Dirk, Mace, Sword, Bow, Magic Wand). Binary present/absent for display (maxshown=1).
- **stuff[5]**: Golden Lasso — binary flag (0 or 1)
- **stuff[6]**: Sea Shell — binary flag (0 or 1)
- **stuff[7]**: Sun Stone — binary flag (0 or 1)
- **stuff[8]**: Arrows — integer count (maxshown=45)
- **stuff[9..15]**: Magic consumables (Blue Stone, Green Jewel, Glass Vial, Crystal Orb, Bird Totem, Gold Ring, Jade Skull) — integer counts, consumed on use
- **stuff[16..21]**: Keys (Gold, Green, Blue, Red, Grey, White) — integer counts, consumed when used on a door
- **stuff[22]**: Talisman — binary flag. Collecting it triggers the **win condition** (`fmain.c:3244-3247`).
- **stuff[23]**: Rose — binary flag. Grants fire immunity to the player (`fmain.c:1844`).
- **stuff[24]**: Fruit — integer count. Auto-consumed when `hunger > 30` on safe checkpoints (`fmain.c:2195-2196`), or eaten from meals.
- **stuff[25]**: Gold Statue — integer count (maxshown=5). Need 5 to access desert sector (`fmain.c:1919`, `fmain.c:3594`).
- **stuff[26]**: Book — binary flag. Appears in Give menu but no special use effect found.
- **stuff[27]**: Herb — binary. Picked up but no use effect found beyond possession.
- **stuff[28]**: Writ — binary flag. Given by the princess (`fmain2.c:1598`). Showing it to Priest triggers Statue reveal (`fmain.c:3383-3386`).
- **stuff[29]**: Bone — binary flag. Can be given to the Spectre (`fmain.c:3501-3503`).
- **stuff[30]**: Shard — binary flag. Allows walking through terrain type 12 (crystal walls) — `fmain.c:1609`.
- **stuff[31..34]**: Gold pieces — not stored in stuff[]; gold entries use `inv_list[j].maxshown` as the gold value, added to the `wealth` variable instead.
- **stuff[35]**: Used as a temporary accumulator for quiver pickups — set to 0 before Take action, then `stuff[8] += stuff[ARROWBASE] * 10` after pickup — `fmain.c:3150`, `fmain.c:3243`.

### New Brother Initialization

On `revive(new=TRUE)` — `fmain.c:2849-2850`:
```c
for (i=0; i<GOLDBASE; i++) stuff[i] = 0;  /* has no stuff */
stuff[0] = an->weapon = 1;                /* okay, a dirk, then */
```
Each new brother starts with just one Dirk.

### Save/Load

`mod1save()` — `fmain.c:3623-3628`:
```c
saveload(julstuff,35);
saveload(philstuff,35);
saveload(kevstuff,35);
stuff = blist[brother-1].stuff;
```
All three brother inventories are saved/loaded. After load, `stuff` pointer is re-bound to the current brother.

## itrans Mapping — World Object to Inventory Item

**Definition** — `fmain2.c:979-985`:
```c
UBYTE itrans[] = {
    QUIVER,35,
    B_STONE,9, G_JEWEL,10, VIAL,11, C_ORB,12, B_TOTEM,13, G_RING,14, J_SKULL,15,
    M_WAND,4, 27,5, 8,2, 9,1, 12,0, 10,3, ROSE,23, FRUIT,24, STATUE,25,
    BOOK,26, SHELL,6, 155,7, 136,27, 137,28, 138,29, 139,22, 140,30,
    GOLD_KEY,16, GREEN_KEY,17, BLUE_KEY,18, RED_KEY,19, GREY_KEY,20, WHITE_KEY,21,
    0,0
};
```

### Resolved Mapping Table

Using the `enum obytes` values from `fmain2.c:968-977`:

| World Object ID (`ob_id`) | Enum Name | → stuff[] Index | Inventory Item |
|---|---|---|---|
| 11 | QUIVER | 35 | quiver of arrows |
| 18 | B_STONE | 9 | Blue Stone |
| 19 | G_JEWEL | 10 | Green Jewel |
| 22 | VIAL | 11 | Glass Vial |
| 21 | C_ORB | 12 | Crystal Orb |
| 23 | B_TOTEM | 13 | Bird Totem |
| 17 | G_RING | 14 | Gold Ring |
| 24 | J_SKULL | 15 | Jade Skull |
| 145 | M_WAND | 4 | Magic Wand |
| 27 | (raw) | 5 | Golden Lasso |
| 8 | (raw) | 2 | Sword |
| 9 | (raw) | 1 | Mace |
| 12 | (raw) | 0 | Dirk |
| 10 | (raw) | 3 | Bow |
| 147 | ROSE | 23 | Rose |
| 148 | FRUIT | 24 | Fruit |
| 149 | STATUE | 25 | Gold Statue |
| 150 | BOOK | 26 | Book |
| 151 | SHELL | 6 | Sea Shell |
| 155 | (raw) | 7 | Sun Stone |
| 136 | (raw) | 27 | Herb |
| 137 | (raw) | 28 | Writ |
| 138 | (raw) | 29 | Bone |
| 139 | (raw) | 22 | Talisman |
| 140 | (raw) | 30 | Shard |
| 25 | GOLD_KEY | 16 | Gold Key |
| 153 | GREEN_KEY | 17 | Green Key |
| 114 | BLUE_KEY | 18 | Blue Key |
| 242 | RED_KEY | 19 | Red Key |
| 26 | GREY_KEY | 20 | Grey Key |
| 154 | WHITE_KEY | 21 | White Key |

### Lookup Algorithm

Used in `do_option` ITEMS/Take handler — `fmain.c:3186-3194`:
```c
for (k=0; itrans[k]; k += 2)
{   if (j == itrans[k])
    {   i = itrans[k+1];
        stuff[i]++;
        announce_treasure("a ");
        print_cont(inv_list[i].name);
        print_cont(".");
        goto pickup;
    }
}
```
Iterates pairs until hitting the `0,0` terminator. Matches world object `ob_id` (byte `j`) to inventory index.

### Enum obytes — World Object ID Constants

`fmain2.c:968-977`:
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

Resolved sequential values:
- MONEY=13, URN=14, CHEST=15, SACKS=16, G_RING=17, B_STONE=18, G_JEWEL=19, SCRAP=20, C_ORB=21, VIAL=22, B_TOTEM=23, J_SKULL=24
- GOLD_KEY=25, GREY_KEY=26
- M_WAND=145, MEAL=146, ROSE=147, FRUIT=148, STATUE=149, BOOK=150, SHELL=151
- GREEN_KEY=153, WHITE_KEY=154, RED_KEY=242

### Special-Cased World Objects (Not in itrans)

These world object types bypass the itrans lookup in the Take handler (`fmain.c:3155-3183`):

| ob_id | Enum | Behavior |
|-------|------|----------|
| 0x0d (13) | MONEY | `wealth += 50`; announces "50 gold pieces" |
| 0x14 (20) | SCRAP | Triggers `event(17)` + region-specific event (18 or 19) |
| 148 (0x94) | FRUIT (alt) | If `hunger < 15`, adds to stuff[24]; else calls `eat(30)` |
| 102 | TURTLE | Break — cannot be taken (turtle eggs) |
| 28 | dead brother bones | Recovers dead brother's inventory: adds julstuff[] or philstuff[] to current stuff[] |
| 0x0f (15) | CHEST | Announces "a chest" as container; falls through to random loot |
| 0x0e (14) | URN | Announces "a brass urn" as container |
| 0x10 (16) | SACKS | Announces "some sacks" as container |
| 0x1d (29) | (empty chest) | Break — empty, cannot loot |
| 31 | FOOTSTOOL | Break — cannot be taken |

## jtrans Shop System — Purchase Mechanics

**Definition** — `fmain2.c:850`:
```c
char jtrans[] = { 0,3, 8,10, 11,15, 1,30, 2,45, 3,75, 13,20 };
```

This is 7 pairs of `(stuff_index, price)`:

| Pair# | Menu Hit | stuff[] Index | Item Name | Price (gold) |
|-------|----------|-------------|-----------|------|
| 0 | 5 | 0 | Food (special) | 3 |
| 1 | 6 | 8 | Arrows | 10 |
| 2 | 7 | 11 | Glass Vial | 15 |
| 3 | 8 | 1 | Mace | 30 |
| 4 | 9 | 2 | Sword | 45 |
| 5 | 10 | 3 | Bow | 75 |
| 6 | 11 | 13 | Bird Totem | 20 |

### Buy Menu Labels

From `fmain.c:501`:
```c
char label5[] = "Food ArrowVial Mace SwordBow  Totem";
```
Matches the 7 shop items exactly (menu positions 5–11).

### Purchase Logic

`do_option` case `BUY` — `fmain.c:3424-3441`:
```c
if (anim_list[nearest].race == 0x88)   /* shopkeeper */
{   hit = (hit - 5) * 2;              /* convert menu position to jtrans index */
    i = jtrans[hit++]; j = jtrans[hit]; /* i=stuff index, j=price */
    if (wealth > j)
    {   wealth -= j;
        if (i==0) { event(22); eat(50); }     /* Food: eat(50) reduces hunger */
        else if (i==8) { stuff[i] += 10; }    /* Arrows: +10 bulk purchase */
        else { stuff[i]++; }                  /* Other items: +1 */
    }
    else print("Not enough money!");
}
```
- Only works near a shopkeeper (race `0x88`).
- Food (index 0) is special: doesn't add to stuff[0] (that's Dirk); instead calls `eat(50)` to reduce hunger by 50.
- Arrows are purchased in batches of 10.

## do_option Item Handling

### Menu Modes (enum cmodes) — `fmain.c:494`:
```c
enum cmodes {ITEMS=0, MAGIC, TALK, BUY, GAME, SAVEX, KEYS, GIVE, USE, FILE};
```

### ITEMS Mode — `fmain.c:3102-3297`

- **hit==5 (List)**: Renders inventory screen — blits item images for all stuff[] with count > 0.
- **hit==6 (Take)**: Picks up nearest object. The Take handler is a large multi-path handler:
  1. Checks `nearest_fig(0,30)` for nearby objects.
  2. Specific type checks for MONEY, SCRAP, fruit, turtle eggs, dead brothers.
  3. Container types (CHEST, URN, SACKS) → random loot with `rand4()` determining 0–3 items.
  4. Generic `itrans[]` lookup for all other object types.
  5. After pickup, calls `change_object(nearest, 2)` to mark object as taken.
  6. Quiver conversion: `stuff[8] += stuff[ARROWBASE] * 10`.
  7. **Win check**: If `stuff[22]` (Talisman) is now nonzero, triggers win sequence.
- **hit==7 (Look)**: Reveals hidden objects (ob_stat==5→1) within distance 40.
- **hit==8 (Use)**: Switches to USE submenu.
- **hit==9 (Give)**: Switches to GIVE submenu.

### Container Random Loot — `fmain.c:3198-3239`

When a container (chest/urn/sacks) is opened:
```
k = rand4()   // 0=nothing, 1=one item, 2=two items, 3=three of same
```
- **k==0**: "nothing."
- **k==1**: One random item from `rand8() + 8` (indices 8–15: arrows or magic items). Index 8 becomes ARROWBASE (quiver).
- **k==2**: Two different random items, same range. Index 8 → GOLDBASE+3 for +100 wealth.
- **k==3**: Three of the same. Index 8 → 3 random keys (KEYBASE to KEYBASE+5, wrapping 22→16, 23→20).

### Searching Bodies — `fmain.c:3249-3282`

When examining a dead or frozen enemy:
1. Weapon drop: `anim_list[nearest].weapon` (1–5) → adds weapon to stuff[weapon-1]. Auto-equips if better than current.
2. If weapon is Bow (4), also gives `rand8()+2` arrows.
3. Treasure from `treasure_probs[]` table: indexed by `encounter_chart[j].treasure * 8 + rand8()`.
4. Gold items (index ≥ GOLDBASE) add `inv_list[j].maxshown` to wealth instead of stuff[].

### MAGIC Mode — `fmain.c:3300-3365`

Menu labels from `fmain.c:502`: `"StoneJewelVial Orb  TotemRing Skull"`

Magic items use `stuff[4+hit]` where hit ranges 5–11:

| hit | stuff[] | Item | Effect |
|-----|---------|------|--------|
| 5 | stuff[9] | Blue Stone | Teleport via stone circle (only works at a stone in sector 144) |
| 6 | stuff[10] | Green Jewel | `light_timer += 760` — illuminates dark areas |
| 7 | stuff[11] | Glass Vial | Heal: `vitality += rand8()+4`, capped at `15 + brave/4` |
| 8 | stuff[12] | Crystal Orb | `secret_timer += 360` — reveals secret passages |
| 9 | stuff[13] | Bird Totem | Map: renders overhead map view with player position marker |
| 10 | stuff[14] | Gold Ring | `freeze_timer += 100` — freezes all enemies (not while riding) |
| 11 | stuff[15] | Jade Skull | Kill spell: kills all visible enemies with `vitality > 0`, `type==ENEMY`, and `race < 7`. Decrements `brave` per kill. |

- Guard: `if (hit < 5 || stuff[4+hit] == 0) event(21)` — "if only I had some Magic!" — `fmain.c:3303`
- Guard: `if (extn->v3 == 9) speak(59)` — magic doesn't work in some area — `fmain.c:3304`
- **Consumption**: `if (!--stuff[4+hit]) set_options()` — `fmain.c:3365`. Decrements count; if now zero, updates menu to grey out.

### USE Mode — `fmain.c:3444-3467`

Menu labels: `"Dirk Mace SwordBow  Wand LassoShellKey  Sun  Book "` — `fmain.c:503`

| hit | Action |
|-----|--------|
| 0–4 | Weapon equip: `anim_list[0].weapon = hit+1` if `hitgo` (item available). `hit+1` maps to weapon 1=Dirk, 2=Mace, 3=Sword, 4=Bow, 5=Wand. |
| 5 | Lasso: no explicit USE handler. Lasso works passively — checked in carrier mounting code (`fmain.c:1498`). |
| 6 | Sea Shell: if `hitgo`, calls `get_turtle()` to summon a rideable sea creature (carrier 5). Blocked if hero is in certain coordinate range (inland check). |
| 7 | Keys: if `hitgo` (any keys owned), switches to KEYS submenu; else "has no keys!" |
| 8 | Sun Stone: if `witchflag`, triggers `speak(60)` (witch-related dialogue). |
| 9 | Book: no explicit handler in USE block. |

### KEYS Mode — `fmain.c:3468-3485`

Menu labels: `"Gold GreenBlue Red  Grey White"` — `fmain.c:505`

```c
hit -= 5;  // convert to key index 0-5
if (stuff[hit + KEYBASE])
{   for (i=0; i<9; i++)  // try 9 directions
    {   x = newx(hero_x, i, 16);
        y = newy(hero_y, i, 16);
        if (doorfind(x, y, hit+1)) { stuff[hit+KEYBASE]--; break; }
    }
    // if no door found, print "tried a <key> but it didn't fit."
}
```
- Keys are consumed on successful door unlock.
- `doorfind()` (`fmain.c:1081-1123`) locates terrain type 15 (STAIR/door) nearby, looks it up in `open_list[]` for matching `keytype`, and modifies the map tile to "open" it.

### GIVE Mode — `fmain.c:3486-3506`

Menu labels: `"Gold Book Writ Bone "` — `fmain.c:506`

| hit | Action |
|-----|--------|
| 5 (Gold) | Give 2 gold to nearest NPC. `wealth -= 2`. If `rand64() > kind`, `kind++`. Beggars (0x8d) give goal-specific speech. Others: `speak(50)`. |
| 6 (Book) | No handler — `menus[GIVE].enabled[6] = 8` (always shown but no code path). |
| 7 (Writ) | Shown if `stuff[28]` > 0 via `stuff_flag(28)`. No give-specific code — handled in TALK/Priest interaction. |
| 8 (Bone) | Give bone to Spectre: if `k == 0x8a` (spectre NPC), `speak(48)`, `stuff[29] = 0`, `leave_item(nearest_person, 140)` (drops shard at spectre location). If not spectre: `speak(21)`. |

## stuff_flag — Menu Enable Check

Assembly function — `fmain2.c:1639-1648`:
```asm
_stuff_flag
    moveq   #8,d0           ; default return = 8 (disabled)
    move.l  _stuff,a0
    add.l   4(sp),a0        ; index argument
    tst.b   (a0)
    beq.s   1$              ; if stuff[index] == 0, return 8
    moveq   #10,d0          ; else return 10 (enabled)
1$  rts
```
Returns 8 if `stuff[index] == 0` (disabled), 10 if nonzero (enabled). Used by `set_options()` to enable/disable menu items.

## set_options — Menu State Update

`fmain.c:3526-3542`:
```c
set_options()
{   for (i=0; i<7; i++)
    {   menus[MAGIC].enabled[i+5] = stuff_flag(i+9);   // magic items 9-15
        menus[USE].enabled[i] = stuff_flag(i);          // weapons/items 0-6
    }
    j = 8;
    for (i=0; i<6; i++)
    {   if ((menus[KEYS].enabled[i+5] = stuff_flag(i+16)) == 10) j = 10; }
    menus[USE].enabled[7] = j;       // Keys submenu: enabled if any key owned
    menus[USE].enabled[8] = stuff_flag(7);  // Sun Stone
    j=8; if (wealth>2) j = 10;
    menus[GIVE].enabled[5] = j;      // Gold: enabled if wealth > 2
    menus[GIVE].enabled[6] = 8;      // Book: always disabled (hardcoded 8)
    menus[GIVE].enabled[7] = stuff_flag(28); // Writ
    menus[GIVE].enabled[8] = stuff_flag(29); // Bone
}
```

Note: `menus[GIVE].enabled[6] = 8` means Book in the Give menu is **always disabled**. This appears intentional — the Book has no give interaction.

## ppick — Print Queue Processing

`fmain2.c:442-480`. This is actually a **print queue consumer**, not an item picker. It processes queued messages via `print_que[]`:

| Queue code | Output |
|------------|--------|
| 2 | Debug: coordinates + available memory |
| 3 | Location: hero position + sector + extent |
| 4 | Update vitality display on HUD |
| 5 | Call `print_options()` (redraw menu) |
| 7 | Stats: Brv/Lck/Knd/Wlth display |
| 10 | "Take What?" prompt |

Messages are enqueued via `prq()` (assembly at `fmain2.c:481-494`), a circular buffer of 32 entries.

## announce_container / announce_treasure

`fmain2.c:579-590`:

```c
announce_container(s) char *s;
{   print(datanames[brother-1]);   // prints brother name
    print_cont(" found ");
    print_cont(s);
    print_cont(" containing ");   // expects further output
}

announce_treasure(s) char *s;
{   print(datanames[brother-1]);
    print_cont(" found ");
    print_cont(s);                // complete message
}
```

Used in Take handler to format pickup messages: "Julian found a chest containing ..." or "Julian found a Sword."

## Item Use Effects — Complete Reference

### Weapons (stuff[0..4])

When equipped via USE menu (`fmain.c:3448-3451`):
- `anim_list[0].weapon = hit+1` (1=Dirk, 2=Mace, 3=Sword, 4=Bow, 5=Wand)
- Only changes if `hitgo` (item available in inventory)
- Bow (weapon 4) consumes arrows on each shot: `if (i==0) { if (stuff[8]==0) goto cpx; else stuff[8]--; }` — `fmain.c:1677`
- When arrows run out mid-combat: auto-switches to next best weapon — `fmain.c:1693`

### Combat Immunity

`dohit()` — `fmain2.c:229-234`:
- Enemies with `race == 9` (spectre?) or `race == 0x89` when `stuff[7] == 0` (no Sun Stone): attack has no effect. `speak(58)` tells player.
- Enemies with `race == 0x8a` or `0x8b`: completely immune to attack (spectre/ghost NPCs).

### Golden Lasso (stuff[5])

- Enables mounting the swan carrier: `if (raftprox && wcarry == 3 && stuff[5])` — `fmain.c:1498`
- Obtained by defeating witch (race 0x89): `leave_item(i, 27)` drops lasso on death — `fmain.c:1756`
- Can also be obtained via cheat key 'B': `stuff[5] = 1` — `fmain.c:1294`

### Sea Shell (stuff[6])

- Obtained from talking to sea carrier (turtle/dolphin): `else { stuff[6] = 1; speak(56); }` — `fmain.c:3420`
- Having it changes dialogue: `if (stuff[6]) speak(57)` — `fmain.c:3419`

### Sun Stone (stuff[7])

- Required for damaging race 0x89 enemy (see Combat Immunity above).
- USE menu hit==8: if `witchflag`, triggers `speak(60)` — `fmain.c:3462`

### Rose (stuff[23])

- Grants fire immunity: `if (i==0 && stuff[23]) an->environ = 0` — `fmain.c:1844`
- When player (i==0) is in fiery terrain and has Rose, environment damage is zeroed.

### Shard (stuff[30])

- Allows passage through crystal walls (terrain type 12): `if (stuff[30] && j==12) goto newloc` — `fmain.c:1609`
- Normal collision would block on terrain 12; Shard bypasses this.

### Fruit (stuff[24])

- **Auto-eat**: At each safe checkpoint tick, if `hunger > 30 && stuff[24]`, then `stuff[24]--; hunger -= 30; event(37)` — `fmain.c:2195-2196`
- **Pickup**: If picked up from ground and `hunger < 15`, stored in inventory; else immediately eaten via `eat(30)` — `fmain.c:3166-3167`
- `eat(amt)` — `fmain2.c:1704-1708`: `hunger -= amt; if (hunger < 0) hunger = 0`

### Talisman (stuff[22])

- **Win condition**: Immediately after any Take action completes, if `stuff[22]` is nonzero: `quitflag = TRUE; viewstatus = 2; map_message(); win_colors()` — `fmain.c:3244-3247`
- Cheat code: `stuff[22] = 0` (key `.` with cheat) — `fmain.c:1299`
- World object ID 139 → stuff[22] via itrans — `fmain2.c:983`

### Gold Statue (stuff[25])

- Need ≥5 statues to enter the desert sector door: `if (d->type == DESERT && (stuff[STATBASE]<5)) break` — `fmain.c:1919`
- When loading desert region with <5 statues, map tiles are overwritten to block access: `fmain.c:3594-3596`
- 5 statues placed in the world: seahold (ob_listg[6]), ogre den ([7]), octal room ([8]), sorceress ([9]), priest ([10]) — `fmain2.c:1008-1012`

### Writ (stuff[28])

- Given by the princess event: `stuff[28] = 1` — `fmain2.c:1598`
- Presentation to Priest (TALK, setfig case 1): if `stuff[28]`, triggers `speak(39)` and sets `ob_listg[10].ob_stat = 1` (reveals priest's Gold Statue) — `fmain.c:3383-3386`
- Also: `for (i=16; i<22; i++) stuff[i] += 3` — princess gives 3 of each key — `fmain2.c:1602`

### Bone (stuff[29])

- GIVE to Spectre (race 0x8a): `speak(48); stuff[29] = 0; leave_item(nearest_person, 140)` — drops Shard (ob_id 140) at spectre location — `fmain.c:3503`

## Quest Items — Progression Chain

Based on cross-cutting references:

1. **Gold Statues ×5** (stuff[25]): Collect from various locations. Gate the desert sector.
2. **Writ** (stuff[28]): Obtained from princess event (`fmain2.c:1598`). Used to convince Priest to reveal statue.
3. **Bone** (stuff[29]): Found in world (ob_id 138). Given to Spectre to receive Shard.
4. **Shard** (stuff[30]): Obtained from Spectre in exchange for Bone. Allows passage through crystal walls.
5. **Rose** (stuff[23]): Found in world. Provides fire immunity for navigating fire areas.
6. **Sun Stone** (stuff[7]): Required for combat against certain enemies (race 0x89). Also witch interaction.
7. **Golden Lasso** (stuff[5]): Dropped by witch (race 0x89) on death. Required to ride swan.
8. **Sea Shell** (stuff[6]): Obtained from sea carrier dialogue. Dialogue progression flag.
9. **Talisman** (stuff[22]): The final quest item. Picking it up triggers the win sequence immediately.

## rand_treasure — Random Loot Table

`fmain2.c:986-992`:
```c
UBYTE rand_treasure[] = {
    SACKS, SACKS, SACKS, SACKS,       // group 0: all sacks
    CHEST, MONEY, GOLD_KEY, QUIVER,    // group 1
    GREY_KEY, GREY_KEY, GREY_KEY, RED_KEY,  // group 2
    B_TOTEM, VIAL, WHITE_KEY, CHEST    // group 3
};
```
(This table is used in `set_objects()` at `fmain2.c:1236` when first loading a region—10 random objects are scattered with `ob_id = rand_treasure[bitrand(15)]`.)

## treasure_probs — Enemy Treasure Drop Table

`fmain2.c:852-858`:
```c
char treasure_probs[] = {
    0, 0, 0, 0, 0, 0, 0, 0,          // group 0: no treasure
    9,11,13,31,31,17,17,32,           // group 1: varied magic/gold/keys
    12,14,20,20,20,31,33,31,          // group 2: keys/skull/gold
    10,10,16,16,11,17,18,19,          // group 3: magic and keys
    15,21, 0, 0, 0, 0, 0, 0          // group 4: jade skull and white key
};
```

Indexed by `encounter_chart[race].treasure * 8 + rand8()` — `fmain.c:3272-3273`. The result is a stuff[] index (or 0 for nothing). Items ≥ GOLDBASE (31) add to wealth instead.

## change_object — Object State Management

`fmain2.c:1200-1212`:
```c
change_object(id, flag) register long id, flag;
{   register struct shape *an;
    register struct object *ob;
    an = anim_list + id;
    id = an->vitality & 0x07f;
    if (an->vitality & 0x080) ob = ob_listg + id;
    else ob = ob_table[region_num] + id;
    if (ob->ob_id == CHEST) ob->ob_id = 0x1d;  // chest → empty chest
    else ob->ob_stat = flag;
}
```
- Flag values: 1=show (make visible), 2=take (mark as picked up)
- Chests get their `ob_id` changed to 0x1d (empty chest) rather than being hidden
- Other objects get `ob_stat` set to the flag value

## leave_item — Drop Item in World

`fmain2.c:1191-1196`:
```c
leave_item(i, object) short i, object;
{   ob_listg[0].xc = anim_list[i].abs_x;
    ob_listg[0].yc = anim_list[i].abs_y + 10;
    ob_listg[0].ob_id = object;
    ob_listg[0].ob_stat = 1;
}
```
Uses the global slot `ob_listg[0]` to place an object at an NPC/enemy's position. Used for:
- Talisman (139) dropped by defeated enemy race 0x09 — `fmain.c:1754`
- Lasso (27) dropped by defeated enemy race 0x89 — `fmain.c:1756`
- Shard (140) dropped by Spectre when given Bone — `fmain.c:3503`

## Maximum Carry Limits

The `stuff[]` array uses UBYTE (0–255). There is **no explicit cap** on item counts beyond the UBYTE range. The `maxshown` field in `inv_list` only limits display, not storage:

- Weapons (0–4): Practically binary (0 or 1), but can accumulate via body searching
- Arrows (8): No cap; purchased in batches of 10, found in quivers of 10
- Magic items (9–15): No cap; consumed by 1 per use
- Keys (16–21): No cap; consumed by 1 per door unlock. Princess gives +3 of each.
- Unique items (5–7, 22–30): Functionally binary (set to 1, checked for nonzero)

## Cross-Cutting Findings

- **stuff[30] (Shard) in movement/collision** (`fmain.c:1609`): Terrain passability check in the main movement loop, not in any inventory subsystem.
- **stuff[23] (Rose) in combat/environment** (`fmain.c:1844`): Fire immunity check in the environment damage tick.
- **stuff[5] (Golden Lasso) in carrier/mounting** (`fmain.c:1498`): Gate for swan riding in the carrier movement subsystem.
- **stuff[7] (Sun Stone) in dohit()** (`fmain2.c:233`): Combat damage gating in the hit handler.
- **stuff[24] (Fruit) in daynight/survival** (`fmain.c:2195`): Auto-consumption in the safe checkpoint tick.
- **stuff[STATBASE] in doors** (`fmain.c:1919`): Desert gate in the door traversal subsystem.
- **stuff[STATBASE] in sector loading** (`fmain.c:3594`): Map tile overwrite in region loading.
- **stuff[22] (Talisman) in menu cheats** (`fmain.c:1299`): Set to 0 via cheat key (preventing accidental win).
- **menus[GIVE].enabled[6] = 8** (`fmain.c:3540`): Book give option is always disabled, suggesting incomplete feature or intentional omission.

## Unresolved

- **Raw object IDs 27, 8, 9, 12, 10** in itrans: These small numbers (27→Lasso, 8→Sword, 9→Mace, 12→Dirk, 10→Bow) don't correspond to named enum values. They may be sprite/image indices or leftover numeric IDs from an earlier object numbering scheme. The enum `obytes` doesn't cover them, suggesting they predate the named constants.
- **Object ID 155 → Sun Stone**: Not in enum obytes. Possibly a late addition.
- **Object ID 136–140**: Map to Herb, Writ, Bone, Talisman, Shard. Not in enum.
- **Book (stuff[26])**: No use effect, no give effect (give menu always disabled for Book). Its purpose is unclear — possibly purely a collectible or an abandoned feature.
- **Herb (stuff[27])**: Found in itrans (ob_id 136 → stuff[27]) but no usage of `stuff[27]` found in any code path. May be unused/abandoned.
- **rand_treasure[] usage**: Used in `set_objects()` at `fmain2.c:1236`: `l2[k].ob_id = rand_treasure[bitrand(15)]`. When a region is first loaded and hasn't been populated yet (`dstobs[region_num] == 0`), 10 random objects are scattered across the region with random `ob_id` values from this table. This seeds the world with random loot in containers and loose items.
- **USE mode hit==5 (Lasso)**: No handler exists in the USE switch for hit==5. The Lasso works passively — it's checked in the carrier mounting code (`fmain.c:1498`) when near the swan, not via an explicit USE action. Hit==6 (Shell) correctly calls `get_turtle()` which summons a sea creature (carrier type 5) via `load_carrier(5)` — `fmain.c:3507-3514`. This is distinct from the swan carrier (type 11) used by the Lasso.
- **USE mode hit==9 (Book)**: No handler in the USE switch. Book's use is unknown.

## Refinement Log
- 2026-04-05: Initial comprehensive discovery pass. All 12 questions answered with citations. Tool `extract_item_effects.py` exists but was not run — findings gathered directly from source.
