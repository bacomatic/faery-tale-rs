# Discovery: Menu System & Player Actions

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace the complete menu/action system — menus[] structure, display, navigation, keyboard shortcuts, set_options dynamic availability, and the full do_option dispatch.

## menus[] Structure

### `struct menu` definition — fmain.c:517-520 (struct at line 517, data at 522-531)

```c
struct menu {
    char    *label_list;
    char    num, color;
    char    enabled[12];
} menus[10];
```

- `label_list`: pointer to a char array of 5-char label strings
- `num`: total number of entries in this menu (including the 5 top-bar entries)
- `color`: background pen color for menu items
- `enabled[12]`: per-entry state byte; encoding described below

### Menu Mode Enum — fmain.c:494

```c
enum cmodes {ITEMS=0, MAGIC, TALK, BUY, GAME, SAVEX, KEYS, GIVE, USE, FILE};
```

### Label Strings — fmain.c:496-506

| Label Var | Contents (5-char fields)                                  | Source            |
|-----------|-----------------------------------------------------------|-------------------|
| `label1`  | `"ItemsMagicTalk Buy  Game "`                             | fmain.c:496       |
| `label2`  | `"List Take Look Use  Give "`                             | fmain.c:497       |
| `label3`  | `"Yell Say  Ask  "`                                       | fmain.c:498       |
| `label4`  | `"PauseMusicSoundQuit Load "`                             | fmain.c:499       |
| `label5`  | `"Food ArrowVial Mace SwordBow  Totem"`                   | fmain.c:500       |
| `label6`  | `"StoneJewelVial Orb  TotemRing Skull"`                   | fmain.c:501       |
| `label7`  | `"Dirk Mace SwordBow  Wand LassoShellKey  Sun  Book "`   | fmain.c:502       |
| `label8`  | `"Save Exit "`                                            | fmain.c:503       |
| `label9`  | `"Gold GreenBlue Red  Grey White"`                        | fmain.c:504       |
| `labelA`  | `"Gold Book Writ Bone "`                                  | fmain.c:505       |
| `labelB`  | `"  A    B    C    D    E    F    G    H  "`              | fmain.c:506       |

### `enabled[]` Encoding — fmain.c:1310-1328

Each `enabled[i]` byte is split:
- **Bit 0** (mask `& 1`): highlight/selected state (0 = off, 1 = on)
- **Bit 1** (mask `& 2`): visibility — item only displayed if bit 1 is set
- **Bits 2-7** (mask `& 0xfc`): action type (`atype`)

Action type values and their click behavior (fmain.c:1310-1328):
| atype | Behavior |
|-------|----------|
| 0     | If `hit < 5`: switch `cmode` to `hit` (top-bar navigation) |
| 4     | Toggle: XOR bit 0, call `do_option(hit)` |
| 8     | Immediate action: highlight then `do_option(hit)` |
| 12    | One-shot highlight: set bit 0, highlight, `do_option(hit)` |

Common encoded values:
- `2` = visible, not highlighted, atype=0 (top-bar nav item)
- `3` = visible + highlighted, atype=0 (top-bar nav, active mode)
- `6` = visible, atype=4 (toggle)
- `7` = visible + on, atype=4 (toggle, currently on)
- `8` = visible, atype=8 (disabled/not available — see note)
- `10` = visible, atype=8 (action button, `hitgo` = true since bit 1 set)

**Note**: Value `8` means bit 1 = 0 (not visible/filtered out by `print_options`), atype=8. Value `10` means bits 1+3 set = visible + atype=8. So `8` = hidden action, `10` = visible action.

Wait, let me re-check: `8` in binary is `00001000`. Bit 1 = `(8 & 2) = 0`, so NOT visible. Bits 2+ = `(8 & 0xfc) = 8`, atype=8. And `10` in binary is `00001010`. Bit 1 = `(10 & 2) = 2`, so visible. Bits 2+ = `(10 & 0xfc) = 8`, atype=8. This is confirmed.

### enabled[] Encoding Source Comment — fmain.c:512-513

```c
/* bit0 = selected, bit1 = displayed, else = type
   4 = toggle, 8 = immediate, 12 = radio buttons, 0 = not changeable */
```

### Complete Menu Definitions — fmain.c:522-531

All entries have indices 0-4 as the top-bar items (Items/Magic/Talk/Buy/Game), displayed via `label1`. Indices 5+ are submenu items displayed via `menus[cmode].label_list`.

#### `menus[0]` — ITEMS (label2, num=10, color=6)
```
enabled: {3, 2, 2, 2, 2, 10, 10, 10, 10, 10, 0, 0}
```
| Index | Label (from label1/label2) | enabled | Visible | atype | Meaning |
|-------|---------------------------|---------|---------|-------|---------|
| 0     | Items (label1)            | 3       | yes     | 0     | Current mode (highlighted) |
| 1     | Magic (label1)            | 2       | yes     | 0     | Nav to MAGIC |
| 2     | Talk (label1)             | 2       | yes     | 0     | Nav to TALK |
| 3     | Buy (label1)              | 2       | yes     | 0     | Nav to BUY |
| 4     | Game (label1)             | 2       | yes     | 0     | Nav to GAME |
| 5     | List (label2)             | 10      | yes     | 8     | Show inventory |
| 6     | Take (label2)             | 10      | yes     | 8     | Take nearby object |
| 7     | Look (label2)             | 10      | yes     | 8     | Look around |
| 8     | Use (label2)              | 10      | yes     | 8     | Go to USE submenu |
| 9     | Give (label2)             | 10      | yes     | 8     | Go to GIVE submenu |

#### `menus[1]` — MAGIC (label6, num=12, color=5)
```
enabled: {2, 3, 2, 2, 2, 8, 8, 8, 8, 8, 8, 8}
```
| Index | Label                     | enabled | Visible | atype | Meaning |
|-------|---------------------------|---------|---------|-------|---------|
| 0-4   | Top bar (Items..Game)     | 2/3/2/2/2 | yes   | 0     | Nav |
| 5     | Stone (label6)            | 8       | no      | 8     | Teleport (stuff[14]) |
| 6     | Jewel (label6)            | 8       | no      | 8     | Light (stuff[15]) — actually stuff[9+i] where i=0..6 |
| 7     | Vial (label6)             | 8       | no      | 8     | Heal (stuff[16]) |
| 8     | Orb (label6)              | 8       | no      | 8     | Secret (stuff[17]) |
| 9     | Totem (label6)            | 8       | no      | 8     | Map (stuff[18]) |
| 10    | Ring (label6)             | 8       | no      | 8     | Freeze (stuff[19]) |
| 11    | Skull (label6)            | 8       | no      | 8     | Kill (stuff[20]) |

Note: All magic items start hidden (8). `set_options()` updates `enabled[i+5] = stuff_flag(i+9)` which returns 10 if owned, 8 if not.

#### `menus[2]` — TALK (label3, num=8, color=9)
```
enabled: {2, 2, 3, 2, 2, 10, 10, 10, 0, 0, 0, 0}
```
| Index | Label           | enabled | Visible | atype | Meaning |
|-------|-----------------|---------|---------|-------|---------|
| 0-4   | Top bar         | 2/2/3/2/2 | yes   | 0     | Nav (Talk highlighted) |
| 5     | Yell (label3)   | 10      | yes     | 8     | Yell at NPC (range 100) |
| 6     | Say (label3)    | 10      | yes     | 8     | Say to NPC (range 50) |
| 7     | Ask (label3)    | 10      | yes     | 8     | Ask NPC (range 50) |

#### `menus[3]` — BUY (label5, num=12, color=10)
```
enabled: {2, 2, 2, 3, 2, 10, 10, 10, 10, 10, 10, 10}
```
| Index | Label            | enabled | Visible | atype | Meaning |
|-------|------------------|---------|---------|-------|---------|
| 0-4   | Top bar          | 2/2/2/3/2 | yes   | 0     | Nav (Buy highlighted) |
| 5     | Food (label5)    | 10      | yes     | 8     | Buy food |
| 6     | Arrow (label5)   | 10      | yes     | 8     | Buy arrows |
| 7     | Vial (label5)    | 10      | yes     | 8     | Buy vial |
| 8     | Mace (label5)    | 10      | yes     | 8     | Buy mace |
| 9     | Sword (label5)   | 10      | yes     | 8     | Buy sword |
| 10    | Bow (label5)     | 10      | yes     | 8     | Buy bow |
| 11    | Totem (label5)   | 10      | yes     | 8     | Buy totem |

#### `menus[4]` — GAME (label4, num=10, color=2)
```
enabled: {2, 2, 2, 2, 3, 6, 7, 7, 10, 10, 0, 0}
```
| Index | Label            | enabled | Visible | atype | Meaning |
|-------|------------------|---------|---------|-------|---------|
| 0-4   | Top bar          | 2/2/2/2/3 | yes   | 0     | Nav (Game highlighted) |
| 5     | Pause (label4)   | 6       | yes     | 4     | Toggle pause |
| 6     | Music (label4)   | 7       | yes     | 4     | Toggle music (starts ON) |
| 7     | Sound (label4)   | 7       | yes     | 4     | Toggle sound (starts ON) |
| 8     | Quit (label4)    | 10      | yes     | 8     | Go to SAVEX submenu |
| 9     | Load (label4)    | 10      | yes     | 8     | Load game (sets svflag=FALSE, go to FILE) |

#### `menus[5]` — SAVEX (label8, num=7, color=0)
```
enabled: {2, 2, 2, 2, 2, 10, 10, 0, 0, 0, 0, 0}
```
| Index | Label            | enabled | Visible | atype | Meaning |
|-------|------------------|---------|---------|-------|---------|
| 0-4   | Top bar          | 2/2/2/2/2 | yes   | 0     | Nav |
| 5     | Save (label8)    | 10      | yes     | 8     | Save (sets svflag=TRUE, go to FILE) |
| 6     | Exit (label8)    | 10      | yes     | 8     | Quit game |

#### `menus[6]` — KEYS (label9, num=11, color=8)
```
enabled: {2, 2, 2, 2, 2, 10, 10, 10, 10, 10, 10, 0}
```
| Index | Label              | enabled | Visible | atype | Meaning |
|-------|--------------------|---------|---------|-------|---------|
| 0-4   | Top bar            | 2/2/2/2/2 | yes   | 0     | Nav |
| 5     | Gold (label9)      | 10*     | dyn     | 8     | Use gold key |
| 6     | Green (label9)     | 10*     | dyn     | 8     | Use green key |
| 7     | Blue (label9)      | 10*     | dyn     | 8     | Use blue key |
| 8     | Red (label9)       | 10*     | dyn     | 8     | Use red key |
| 9     | Grey (label9)      | 10*     | dyn     | 8     | Use grey key |
| 10    | White (label9)     | 10*     | dyn     | 8     | Use white key |

*Dynamically updated by `set_options()`.

#### `menus[7]` — GIVE (labelA, num=9, color=10)
```
enabled: {2, 2, 2, 2, 2, 10, 0, 0, 0, 0, 0, 0}
```
| Index | Label             | enabled | Visible | atype | Meaning |
|-------|-------------------|---------|---------|-------|---------|
| 0-4   | Top bar           | 2/2/2/2/2 | yes   | 0     | Nav |
| 5     | Gold (labelA)     | 10*     | dyn     | 8     | Give gold (wealth>2 → 10) |
| 6     | Book (labelA)     | 0→8    | no      | 8     | Give book (always set to 8 by set_options) |
| 7     | Writ (labelA)     | 0*      | dyn     | 8     | Give writ |
| 8     | Bone (labelA)     | 0*      | dyn     | 8     | Give bone |

#### `menus[8]` — USE (label7, num=10, color=8)
```
enabled: {10, 10, 10, 10, 10, 10, 10, 10, 10, 0, 10, 10}
```
| Index | Label              | enabled | Visible | atype | Meaning |
|-------|--------------------|---------|---------|-------|---------|
| 0     | Dirk (label7)      | 10*     | dyn     | 8     | Equip dirk |
| 1     | Mace (label7)      | 10*     | dyn     | 8     | Equip mace |
| 2     | Sword (label7)     | 10*     | dyn     | 8     | Equip sword |
| 3     | Bow (label7)       | 10*     | dyn     | 8     | Equip bow |
| 4     | Wand (label7)      | 10*     | dyn     | 8     | Equip wand |
| 5     | Lasso (label7)     | 10*     | dyn     | 8     | Equip lasso |
| 6     | Shell (label7)     | 10*     | dyn     | 8     | Use turtle totem |
| 7     | Key (label7)       | 10*     | dyn     | 8     | Go to KEYS submenu |
| 8     | Sun (label7)       | 10*     | dyn     | 8     | Use sunstone |
| 9     | (unused)           | 0       | no      | —     | — |
| 10    | Book (label7)      | 10      | yes     | 8     | — (index 10, beyond hit range?) |
| 11    | (label7 overflow)  | 10      | yes     | 8     | — |

*Dynamically updated by `set_options()`. NOTE: USE menu has no top-bar entries (indices 0-4 are weapons, not nav). The label display path for `cmode >= USE` uses `menus[cmode].label_list` directly (fmain.c:3088).

#### `menus[9]` — FILE (labelB, num=10, color=5)
```
enabled: {10, 10, 10, 10, 10, 10, 10, 10, 0, 0, 0, 0}
```
| Index | Label              | enabled | Visible | atype | Meaning |
|-------|--------------------|---------|---------|-------|---------|
| 0-7   | A through H        | 10      | yes     | 8     | Save/load file slot A-H |
| 8-9   | (unused)           | 0       | no      | —     | — |

### `real_options[12]` — fmain.c:515 (actually line 515)

```c
char real_options[12];
```

This is an indirection array mapping visible screen positions (0-11) to actual `enabled[]` indices. Built by `print_options()`. Only entries where `(enabled[i] & 2) != 0` are included. The mouse/keyboard handler indexes via `real_options[inum]` (fmain.c:1304-1306).

## Menu Display & Navigation

### `print_options()` — fmain.c:3048-3068

Iterates all entries `0..menus[cmode].num-1`. For each entry where `(enabled[i] & 2) != 0` (visible), assigns `real_options[j] = i` and calls `propt(j, x & 1)` to draw. Fills remaining slots (up to 12) with blanks and sets `real_options[j] = -1`.

Layout: Two columns, 6 rows.
- Odd j → x=482, Even j → x=430
- y = (j/2) * 9 + 8

### `propt(j, pena)` — fmain.c:3070-3090

Draws a single menu entry at screen position `j`.

- `pena`: foreground pen (0=off, 1=on)
- `penb`: background pen, determined by:
  - `cmode == USE`: penb = 14
  - `cmode == FILE`: penb = 13
  - `k < 5` (top bar): penb = 4
  - `cmode == KEYS`: penb = `keycolors[k-5]` where `keycolors = {8, 6, 4, 2, 14, 1}` — fmain.c:519
  - `cmode == SAVEX`: penb = k (entry index as color)
  - otherwise: penb = `menus[cmode].color`

Label text source:
- `cmode >= USE`: uses `menus[cmode].label_list + k*5` directly (no label1 prefix)
- `k < 25` (i.e., k*5 < 125): uses `label1 + k*5` (top-bar items from label1)
- `k >= 25`: uses `menus[cmode].label_list + (k-25)*5` (submenu items)

This means for menus ITEMS through GAME (cmode 0-4), the first 5 entries (indices 0-4) are drawn from `label1` ("Items Magic Talk Buy  Game") and entries 5+ from the menu's own `label_list`.

### `gomenu(mode)` — fmain.c:3521-3525

```c
gomenu(mode) short mode;
{   if (menus[GAME].enabled[5] & 1) return;   /* blocked if paused */
    cmode = mode;
    handler_data.lastmenu = 0;
    print_options();
}
```

Switches to a new menu mode. **Blocked if game is paused** (GAME.enabled[5] bit 0 = pause toggle is ON). Resets `handler_data.lastmenu` and redraws.

### Mouse Click Handling — fmain.c:1300-1330

Mouse events arrive as keycodes >= 0x61. The low bits encode the screen position `inum = (key & 0x7f) - 0x61`. The actual menu index is `hit = real_options[inum]`.

- **Mouse up** (key & 0x80 set): un-highlight the entry via `propt(inum, 0)`
- **Mouse down**: reads `atype = menus[cmode].enabled[hit] & 0xfc`, then:
  - atype=4 (toggle): XOR bit 0, redraw, call `do_option(hit)`
  - atype=8 (action): highlight, call `do_option(hit)`
  - atype=12 (one-shot): set bit 0, highlight, call `do_option(hit)`
  - atype=0 + hit<5: switch `cmode = hit` (navigate to that top-bar menu)
  - otherwise: just redraw current state

### Pause Check — fmain.c:1282

```c
notpause = !(menus[GAME].enabled[5] & 1);
```

When paused, most actions are blocked. atype=0 nav is only allowed if `notpause` is true (fmain.c:1326).

## set_options — fmain.c:3527-3543

Dynamically updates `enabled[]` arrays based on current inventory (`stuff[]`).

### `stuff_flag(n)` — fmain2.c:1639-1648

Inline assembly function:
```asm
_stuff_flag
    moveq   #8,d0          ; default return = 8 (hidden)
    move.l  _stuff,a0
    add.l   4(sp),a0       ; index into stuff[]
    tst.b   (a0)
    beq.s   1$             ; if stuff[n] == 0, return 8
    moveq   #10,d0         ; else return 10 (visible action)
1$  rts
```

Returns 8 if `stuff[n] == 0` (item not owned → hidden), 10 if owned (visible action button).

### set_options() logic — fmain.c:3527-3543

```c
set_options()
{   register long i,j;
    for (i=0; i<7; i++)
    {   menus[MAGIC].enabled[i+5] = stuff_flag(i+9);   // magic items stuff[9..15]
        menus[USE].enabled[i] = stuff_flag(i);          // weapons stuff[0..6]
    }
    j = 8;
    for (i=0; i<6; i++)
    {   if ((menus[KEYS].enabled[i+5] = stuff_flag(i+16))==10) j = 10; }
    menus[USE].enabled[7] = j;          // Key option: 10 if any key owned, else 8
    menus[USE].enabled[8] = stuff_flag(7);  // Sun Stone = stuff[7]
    j=8; if (wealth>2) j = 10;
    menus[GIVE].enabled[5] = j;         // Gold: visible if wealth > 2
    menus[GIVE].enabled[6] = 8;         // Book: always hidden (8)
    menus[GIVE].enabled[7] = stuff_flag(28); // Writ
    menus[GIVE].enabled[8] = stuff_flag(29); // Bone
}
```

Called at end of every `do_option()` (fmain.c:3507: `set_options();`).

## do_option Dispatch — fmain.c:3102-3507

### case ITEMS (fmain.c:3110-3297)

- **hit==5** (List): Draws full inventory screen. Iterates `stuff[0..GOLDBASE-1]`, renders item icons using `inv_list[j]` image data onto the drawing page. Sets `viewstatus=4`, calls `prq(5)`. — fmain.c:3113-3148
- **hit==6** (Take): Calls `nearest_fig(0,30)` to find nearest object within range 30. Complex pickup logic:
  - Gold pieces (index 0x0d): adds 50 wealth — fmain.c:3158
  - Scrap of paper (0x14): triggers event(17), then region-dependent event — fmain.c:3161
  - Food (148): if hungry, add to stuff[24]; else eat(30) — fmain.c:3166
  - Turtle eggs (102): skip — fmain.c:3169
  - Brother's bones (28): recovers stored inventory — fmain.c:3170
  - Containers (chest=0x0f, urn=0x0e, sacks=0x10): random loot via `rand4()` giving 0-3 items — fmain.c:3176-3223
  - Other items: lookup via `itrans[]` table — fmain.c:3187-3195
  - Dead enemy bodies: extract weapon + treasure from `treasure_probs[]` — fmain.c:3250-3287
  - Special: picking up item index 22 (`stuff[22]`) triggers win condition (`quitflag = TRUE`) — fmain.c:3228
- **hit==7** (Look): Scans all actors for nearby OBJECTS within range 40, calls `change_object()`. If found, `event(38)`, else `event(20)`. — fmain.c:3289-3297
- **hit==8** (Use): `gomenu(USE)` — fmain.c:3111
- **hit==9** (Give): `gomenu(GIVE)` — fmain.c:3298

### case MAGIC (fmain.c:3299-3367)

Guard: if `hit < 5` or `stuff[4+hit] == 0`, prints "if only I had some magic!" (`event(21)`). Also blocked if `extn->v3 == 9` → `speak(59)`. — fmain.c:3301-3302

| hit | Spell     | Effect | Source |
|-----|-----------|--------|--------|
| 5   | Stone     | Teleport via standing stones — checks hero_sector==144, calculates destination from `stone_list[]` | fmain.c:3327-3348 |
| 6   | Jewel     | `light_timer += 760` — illumination | fmain.c:3305 |
| 7   | Vial      | Heal: `vitality += rand8()+4`, capped at `15+brave/4` | fmain.c:3349-3352 (falls through from case 5) |
| 8   | Orb       | `secret_timer += 360` — reveals secrets | fmain.c:3306 |
| 9   | Totem     | Map view: renders big map with hero position marker, sets `viewstatus=1` | fmain.c:3308-3323 |
| 10  | Ring      | `freeze_timer += 100` — freezes enemies. Blocked if `riding > 1`. | fmain.c:3307 |
| 11  | Skull     | Kill all visible enemies with race < 7: sets vitality=0, calls `checkdead()`, decrements brave | fmain.c:3353-3360 |

After use: `--stuff[4+hit]`; if item depleted, calls `set_options()`. — fmain.c:3363

### case TALK (fmain.c:3368-3425)

- **hit==5** (Yell): `nearest_fig(1, 100)` — range 100
- **hit==6** (Say) / **hit==7** (Ask): `nearest_fig(1, 50)` — range 50

If nearest is SETFIG: extensive switch on `k = an->race & 0x7f`:
| k  | NPC Type   | Response | Source |
|----|------------|----------|--------|
| 0  | Wizard     | speak(35) if kind<10, else speak(27+goal) | fmain.c:3385 |
| 1  | Priest     | Checks writ (stuff[28]), kind, heals | fmain.c:3387-3398 |
| 2,3| Guard      | speak(15) | fmain.c:3399 |
| 4  | Princess   | speak(16) if ob_list8[9].ob_stat | fmain.c:3400 |
| 5  | King       | speak(17) if ob_list8[9].ob_stat | fmain.c:3401 |
| 6  | Noble      | speak(20) | fmain.c:3402 |
| 7  | Sorceress  | luck boost or speak(45) first time | fmain.c:3403-3406 |
| 8  | Innkeeper  | speak(13/12/14) based on fatigue/time | fmain.c:3407-3409 |
| 9  | Witch      | speak(46) | fmain.c:3410 |
| 10 | Spectre    | speak(47) | fmain.c:3411 |
| 11 | Ghost      | speak(49) | fmain.c:3412 |
| 12 | Ranger     | speak(22) in region 2, else speak(53+goal) | fmain.c:3413-3414 |
| 13 | Beggar     | speak(23) | fmain.c:3415 |

If nearest is CARRIER with `active_carrier==5` (turtle): gives shell or speak(57) — fmain.c:3418-3420
If nearest is ENEMY: `speak(an->race)` — fmain.c:3421

### case BUY (fmain.c:3426-3442)

Requires `nearest_person != 0` and their race == `0x88` (shopkeeper). Uses `jtrans[]` table:

```c
char jtrans[] = { 0,3, 8,10, 11,15, 1,30, 2,45, 3,75, 13,20 };
```

Format: pairs of (item_index, cost). `hit = (hit - 5) * 2` maps menu index to jtrans offset.

| Menu hit | jtrans offset | Item (i) | Cost (j) | Item Name |
|----------|---------------|----------|----------|-----------|
| 5 (Food) | 0             | 0        | 3        | food (eat) |
| 6 (Arrow)| 2             | 8        | 10       | arrows (stuff[8] += 10) |
| 7 (Vial) | 4             | 11       | 15       | vial |
| 8 (Mace) | 6             | 1        | 30       | mace |
| 9 (Sword)| 8             | 2        | 45       | sword |
| 10 (Bow) | 10            | 3        | 75       | bow |
| 11 (Totem)| 12           | 13       | 20       | totem |

Food special: `event(22); eat(50)`. Arrows special: `stuff[8] += 10; event(23)`. Others: `stuff[i]++`. — fmain.c:3432-3438

### case GAME (fmain.c:3443-3447)

- **hit==5** (Pause): Handled by toggle atype=4 in click handler, not in `do_option`. The bit 0 toggle on `enabled[5]` gates `notpause` check (fmain.c:1282).
- **hit==6** (Music): `setmood(TRUE)` — restarts/changes music — fmain.c:3444
- **hit==7** (Sound): Not explicitly handled in `do_option`. The toggle (atype=4) flips the bit, and `setmood()` checks `menus[GAME].enabled[6] & 1` to decide whether to play music (fmain.c:2952). Sound (hit==7) with `enabled[7]` likely controls sound effects elsewhere, but no explicit `do_option` code for it.
- **hit==8** (Quit): `gomenu(SAVEX)` — fmain.c:3445
- **hit==9** (Load): `svflag = FALSE; gomenu(FILE)` — fmain.c:3446

### case USE (fmain.c:3448-3466)

- **hit==7** (Key): If `hitgo` (has keys), `gomenu(KEYS)`. Else prints "has no keys!" — fmain.c:3449-3452
- **hit < 5** (Weapons 0-4: Dirk/Mace/Sword/Bow/Wand): If `hitgo`, sets `anim_list[0].weapon = hit+1`. Else prints "doesn't have one." — fmain.c:3453-3456
- **hit==5** (Lasso): Not explicitly handled in USE (no code block for hit==5)
- **hit==6** (Shell/Turtle): If `hitgo`, checks bounds then calls `get_turtle()` — fmain.c:3457-3462
- **hit==8** (Sunstone): If `witchflag`, `speak(60)` — fmain.c:3463
- After all: `gomenu(ITEMS)` — fmain.c:3464

### case SAVEX (fmain.c:3467-3469)

- **hit==5** (Save): `svflag = TRUE; gomenu(FILE)` — fmain.c:3469
- **hit==6** (Exit): `quitflag = TRUE` — fmain.c:3468

### case FILE (fmain.c:3470-3472)

- Calls `savegame(hit)` where hit is 0-7 (file slot A-H) — fmain.c:3470
- Then `gomenu(GAME)` — fmain.c:3471

The `savegame()` function (fmain2.c:1474) uses `svflag` to determine save vs load:
- `svflag == TRUE`: opens file for writing (mode 1006)
- `svflag == FALSE`: opens file for reading (mode 1005)
- Filename: `savename[4] = 'A' + hit` — slot letter

### case KEYS (fmain.c:3473-3485)

- `hit -= 5` (converting to key index 0-5: Gold/Green/Blue/Red/Grey/White)
- If `stuff[hit+KEYBASE]` > 0: scans 9 directions (0-8) around hero, calls `doorfind(x, y, hit+1)`. If door found, decrements key count.
- If no door fits: prints "tried a [key name] but it didn't fit."
- Always returns to ITEMS via `gomenu(ITEMS)` — fmain.c:3485

### case GIVE (fmain.c:3490-3506)

- Requires `nearest_person != 0`
- **hit==5** (Gold): If `wealth > 2`, gives 2 gold. Random kind increase (`rand64() > kind → kind++`). If nearest is beggar (race 0x8d), `speak(24 + goal)`, else `speak(50)`. — fmain.c:3493-3498
- **hit==6** (Book): No explicit code in do_option (always hidden by set_options)
- **hit==7** (Writ): No explicit code in do_option
- **hit==8** (Bone): If `stuff[29]` > 0 and nearest is spectre (race 0x8a): `speak(48)`, clears bone, `leave_item(nearest_person, 140)`. If not spectre: `speak(21)`. — fmain.c:3499-3502
- Returns to ITEMS: `gomenu(ITEMS)` — fmain.c:3503

**Note**: `set_options()` is called at the very end of `do_option()` (fmain.c:3507) to refresh all dynamic menu states.

## Keyboard Shortcuts

### `letter_list[]` — fmain.c:537-547

```c
#define LMENUS 38

struct letters
{   char letter, menu, choice; }
letter_list[] = {
    'I',ITEMS,5,  'T',ITEMS,6,  '?',ITEMS,7,  'U',ITEMS,8,  'G',ITEMS,9,
    'Y',TALK,5,   'S',TALK,6,   'A',TALK,7,
    ' ',GAME,5,   'M',GAME,6,   'F',GAME,7,   'Q',GAME,8,   'L',GAME,9,
    'O',BUY,5,    'R',BUY,6,    '8',BUY,7,    'C',BUY,8,    'W',BUY,9,
        'B',BUY,10,   'E',BUY,11,
    'V',SAVEX,5,  'X',SAVEX,6,
    10,MAGIC,5,   11,MAGIC,6,   12,MAGIC,7,   13,MAGIC,8,   14,MAGIC,9,
        15,MAGIC,10,  16,MAGIC,11,
    '1',USE,0,    '2',USE,1,    '3',USE,2,    '4',USE,3,    '5',USE,4,
        '6',USE,5,    '7',USE,6,    'K',USE,7
};
```

38 entries (LMENUS=38). Function key codes 10-16 map to F1-F7 for magic spells.

### Keyboard Shortcut Table

| Key    | Menu   | choice | Action |
|--------|--------|--------|--------|
| I      | ITEMS  | 5      | List inventory |
| T      | ITEMS  | 6      | Take |
| ?      | ITEMS  | 7      | Look |
| U      | ITEMS  | 8      | Use (go to USE menu) |
| G      | ITEMS  | 9      | Give (go to GIVE menu) |
| Y      | TALK   | 5      | Yell |
| S      | TALK   | 6      | Say |
| A      | TALK   | 7      | Ask |
| SPACE  | GAME   | 5      | Pause |
| M      | GAME   | 6      | Music toggle |
| F      | GAME   | 7      | Sound toggle |
| Q      | GAME   | 8      | Quit |
| L      | GAME   | 9      | Load |
| O      | BUY    | 5      | Buy food |
| R      | BUY    | 6      | Buy arrows |
| 8      | BUY    | 7      | Buy vial |
| C      | BUY    | 8      | Buy mace |
| W      | BUY    | 9      | Buy sword |
| B      | BUY    | 10     | Buy bow |
| E      | BUY    | 11     | Buy totem |
| V      | SAVEX  | 5      | Save |
| X      | SAVEX  | 6      | Exit |
| F1(10) | MAGIC  | 5      | Stone (teleport) |
| F2(11) | MAGIC  | 6      | Jewel (light) |
| F3(12) | MAGIC  | 7      | Vial (heal) |
| F4(13) | MAGIC  | 8      | Orb (secrets) |
| F5(14) | MAGIC  | 9      | Totem (map) |
| F6(15) | MAGIC  | 10     | Ring (freeze) |
| F7(16) | MAGIC  | 11     | Skull (kill) |
| 1      | USE    | 0      | Dirk |
| 2      | USE    | 1      | Mace |
| 3      | USE    | 2      | Sword |
| 4      | USE    | 3      | Bow |
| 5      | USE    | 4      | Wand |
| 6      | USE    | 5      | Lasso |
| 7      | USE    | 6      | Shell (turtle) |
| K      | USE    | 7      | Keys |

### Keyboard Handler Integration — fmain.c:1343-1360

When a key is pressed (and it's not a special key, viewstatus key, dead state key, numeric direction key, or cheat key):

1. KEYS mode special handling: if `cmode == KEYS` and key is '1'-'6', calls `do_option(key - '1' + 5)`. Otherwise exits to ITEMS. — fmain.c:1341-1343
2. Space bar or `notpause` gate: loops through `letter_list[0..37]`, matching `letter_list[i].letter == key`. — fmain.c:1345-1347
3. SAVEX guard: if matched menu is SAVEX but `cmode != SAVEX`, the match is rejected (`break`). You can only use V/X if already in the SAVEX menu. — fmain.c:1350
4. On match: sets `cmode = menu`, reads `hit = letter_list[i].choice`, checks `hitgo` and `atype`, handles toggle for atype=4, then calls `do_option(hit)` and `print_options()`. — fmain.c:1351-1358

### `keycolors[]` — fmain.c:519

```c
char keycolors[] = { 8, 6, 4, 2, 14, 1 };
```

Maps key indices 0-5 (Gold/Green/Blue/Red/Grey/White) to background pen colors in the KEYS menu. Used by `propt()` when `cmode == KEYS`.

## Print Queue System (prq)

`prq(n)` is a non-blocking enqueue function (fmain2.c:1639) that adds a command byte to a circular buffer `print_que[]`. The dequeue function `ppick()` (fmain2.c:442) processes items:

| prq value | Action | Source |
|-----------|--------|--------|
| 2         | Debug: print coords + memory | fmain2.c:451 |
| 3         | Debug: print location info | fmain2.c:453 |
| 4         | Refresh vitality display | fmain2.c:457 |
| 5         | Call `print_options()` (redraw menu) | fmain2.c:459 |
| 7         | Refresh stats bar (Brv/Lck/Knd/Wlth) | fmain2.c:460 |
| 10        | Print "Take What?" | fmain2.c:465 |

## Cross-Cutting Findings

- **Pause blocks navigation**: `gomenu()` checks `menus[GAME].enabled[5] & 1` and returns immediately if paused. This means you cannot switch menus while paused. — fmain.c:3522
- **Music toggle checked in setmood**: `menus[GAME].enabled[6] & 1` (music toggle state) is checked in `setmood()` to decide whether to play or stop score. — fmain.c:2952
- **set_options called after every do_option**: The last line of `do_option()` is `set_options()` (fmain.c:3507), ensuring menu visibility is always current after any action.
- **USE and FILE menus skip top-bar**: For `cmode >= USE`, `propt()` uses `menus[cmode].label_list` directly without the `label1` prefix (fmain.c:3088). These menus don't have the Items/Magic/Talk/Buy/Game top bar.
- **SAVEX keyboard guard**: The letter_list handler explicitly blocks V/X shortcuts unless already in SAVEX mode (fmain.c:1350), preventing accidental save/exit.
- **Win condition in Take handler**: Picking up item with `stuff[22]` set triggers `quitflag = TRUE` — fmain.c:3228-3230. This means the Talisman pickup (stuff[22]) immediately wins the game.
- **hitgo variable**: Set from `menus[cmode].enabled[hit] & 2` before calling do_option. Used in USE case to check if player owns the weapon/item. This is a bit 1 check on the enabled byte. — fmain.c:1354 and fmain.c:3449

## Unresolved

- **Sound toggle (hit==7, GAME)**: The `enabled[7]` bit toggles but `do_option` has no explicit code for it. How sound effects are actually controlled is unclear — possibly `menus[GAME].enabled[7] & 1` is checked elsewhere in the sound playback code. Not found in fmain.c.
- **GIVE Book (hit==6)**: `set_options()` always sets `menus[GIVE].enabled[6] = 8` (hidden). No do_option code handles it. It's unclear if the book can ever be given — the label exists in labelA but appears permanently disabled.
- **GIVE Writ (hit==7)**: `set_options()` makes it visible when `stuff[28] > 0`, but `do_option` has no explicit code for GIVE hit==7. The writ interaction happens in the TALK/Priest handler instead (fmain.c:3388).
- **USE hit==5 (Lasso)**: No explicit code in the USE case handles hit==5. It falls through to `gomenu(ITEMS)`.
- **label line numbers**: Verified: labels at fmain.c:496-506, enum at 494, keycolors at 509, struct menu at 517, menu data at 522-531.

## Refinement Log
- 2026-04-06: Initial comprehensive discovery pass. Traced menus[] structure, all 10 menu modes, print_options/propt/gomenu display system, set_options dynamic availability, complete do_option dispatch for all cases, keyboard shortcuts via letter_list, and print queue system.
