## 25. UI & Menu System

### 25.1 Menu Structure

```c
struct menu {
    char    *label_list;
    char    num, color;
    char    enabled[12];
} menus[10];
```

`enabled[i]` encoding:
- Bit 0 (`& 1`): highlight/selected toggle
- Bit 1 (`& 2`): visibility — displayed only if set
- Bits 2–7 (`& 0xfc`): action type (`atype`)

| atype | Behavior |
|-------|----------|
| 0 | Top-bar navigation: switch `cmode` to `hit` |
| 4 | Toggle: XOR bit 0, call `do_option(hit)` |
| 8 | Immediate action: highlight, `do_option(hit)` |
| 12 | One-shot highlight: set bit 0, `do_option(hit)` |

### 25.2 Menu Modes

| Mode | Label List | Entries | Color | Purpose |
|------|-----------|---------|-------|---------|
| ITEMS (0) | `label2` | 10 | 6 | List/Take/Look/Use/Give |
| MAGIC (1) | `label6` | 12 | 5 | Stone/Jewel/Vial/Orb/Totem/Ring/Skull |
| TALK (2) | `label3` | 8 | 9 | Yell/Say/Ask |
| BUY (3) | `label5` | 12 | 10 | Food/Arrow/Vial/Mace/Sword/Bow/Totem |
| GAME (4) | `label4` | 10 | 2 | Pause/Music/Sound/Quit/Load |
| SAVEX (5) | `label8` | 7 | 0 | Save/Exit |
| KEYS (6) | `label9` | 11 | 8 | Gold/Green/Blue/Red/Grey/White |
| GIVE (7) | `labelA` | 9 | 10 | Gold/Book/Writ/Bone |
| USE (8) | `label7` | 10 | 8 | Dirk/Mace/Sword/Bow/Wand/Lasso/Shell/Key/Sun |
| FILE (9) | `labelB` | 10 | 5 | Slots A–H |

Modes ITEMS through GAME share a top bar (entries 0–4 from `label1`). For `cmode >= USE`, labels draw directly from `menus[cmode].label_list`.

### 25.3 Menu Rendering

`print_options()` renders on `rp_text2` (hi-res status bitmap):
- Iterates `menus[cmode]` entries; for each visible (`enabled[i] & 2`), assigns `real_options[j] = i`
- Layout: 2 columns (x=430, x=482), 6 rows at 9px spacing, starting at y=8
- Each label is 5 characters

Background pen by mode:
- USE: pen 14. FILE: pen 13. Top bar (k<5): pen 4.
- KEYS: `keycolors[k-5]` where `keycolors = {8, 6, 4, 2, 14, 1}`
- SAVEX: pen = entry index. Others: `menus[cmode].color`

`gomenu(mode)`: sets `cmode`, resets `handler_data.lastmenu`, calls `print_options()`. Blocked if paused.

### 25.4 Dynamic Availability (`set_options`)

Called at the end of every `do_option()`. Updates `enabled[]` based on inventory:

| Menu | Indices | Source | Logic |
|------|---------|--------|-------|
| MAGIC | 5–11 | `stuff[9..15]` | `stuff_flag(i+9)` |
| USE | 0–6 | `stuff[0..6]` | `stuff_flag(i)` |
| KEYS | 5–10 | `stuff[16..21]` | `stuff_flag(i+16)` |
| USE key | 7 | any key owned | 10 if yes, 8 if no |
| USE sun | 8 | `stuff[7]` | `stuff_flag(7)` |
| GIVE gold | 5 | wealth > 2 | 10 if yes, 8 if no |
| GIVE book | 5 | — | Always 8 (hidden) |
| GIVE writ | 8 | `stuff[28]` | `stuff_flag(28)` |
| GIVE bone | 9 | `stuff[29]` | `stuff_flag(29)` |

`stuff_flag(n)`: returns 8 if `stuff[n] == 0` (hidden), 10 if owned (visible).

### 25.5 `do_option` Dispatch

#### ITEMS Mode

| hit | Label | Action |
|-----|-------|--------|
| 5 | List | Full inventory screen. Iterate `stuff[0..GOLDBASE-1]`, draw item icons. `viewstatus=4` |
| 6 | Take | `nearest_fig(0,30)`: gold→+50 wealth; food→eat; bones→recover inventory; containers→random loot via `rand4()`; other→`itrans[]` lookup. Dead bodies→extract weapon+treasure. Win check: `stuff[22]` (Talisman) set → `quitflag = TRUE` |
| 7 | Look | Scan OBJECTS within range 40. Found→`event(38)`, else `event(20)` |
| 8 | Use | `gomenu(USE)` |
| 9 | Give | `gomenu(GIVE)` |

#### MAGIC Mode

Guard: `stuff[4+hit] == 0` → `event(21)`. Blocked in necromancer extent: `speak(59)`.

| hit | Spell | Effect |
|-----|-------|--------|
| 5 | Stone | Teleport via standing stones; requires `hero_sector==144`, uses `stone_list[]` |
| 6 | Jewel | `light_timer += 760` |
| 7 | Vial | `vitality += rand8()+4`, capped at `15+brave/4` |
| 8 | Orb | `secret_timer += 360` |
| 9 | Totem | Map view with hero marker, `viewstatus=1`. Blocked underground unless `cheat1` |
| 10 | Ring | `freeze_timer += 100`. Blocked if `riding > 1` |
| 11 | Skull | Kill all visible enemies with `race < 7`. Decrement `brave` |

After use: `--stuff[4+hit]`; if depleted, `set_options()`.

#### TALK Mode

| hit | Label | Range |
|-----|-------|-------|
| 5 | Yell | `nearest_fig(1, 100)` |
| 6 | Say | `nearest_fig(1, 50)` |
| 7 | Ask | `nearest_fig(1, 50)` |

NPC response dispatch on `race & 0x7f`: Wizard(0)→`speak(35/27+goal)`, Priest(1)→checks writ/heals, Guard(2,3)→`speak(15)`, Princess(4)→`speak(16)`, King(5)→`speak(17)`, Noble(6)→`speak(20)`, Sorceress(7)→luck boost or `speak(45)`, Innkeeper(8)→`speak(13/12/14)` based on fatigue/time, Witch(9)→`speak(46)`, Spectre(10)→`speak(47)`, Ghost(11)→`speak(49)`, Ranger(12)→`speak(22/53+goal)`, Beggar(13)→`speak(23)`.

#### BUY Mode

Requires shopkeeper (`race 0x88`). Uses `jtrans[]` pairs (item_index, cost):

| Menu | Item | Cost |
|------|------|------|
| Food | eat(50) | 3 |
| Arrow | `stuff[8] += 10` | 10 |
| Vial | `stuff[11]++` | 15 |
| Mace | `stuff[1]++` | 30 |
| Sword | `stuff[2]++` | 45 |
| Bow | `stuff[3]++` | 75 |
| Totem | `stuff[13]++` | 20 |

#### GAME Mode

| hit | Label | Action |
|-----|-------|--------|
| 5 | Pause | Toggle via atype=4 bit flip; gates `notpause` |
| 6 | Music | `setmood(TRUE)` |
| 7 | Sound | Toggle bit only |
| 8 | Quit | `gomenu(SAVEX)` |
| 9 | Load | `svflag = FALSE; gomenu(FILE)` |

#### SAVEX Mode

| hit | Action |
|-----|--------|
| 5 | Save: `svflag = TRUE; gomenu(FILE)` |
| 6 | Exit: `quitflag = TRUE` |

#### USE Mode

| hit | Action |
|-----|--------|
| 0–4 | Equip weapon: `weapon = hit+1` (Dirk/Mace/Sword/Bow/Wand) |
| 6 | Shell/Turtle: `get_turtle()` |
| 7 | Key: `gomenu(KEYS)` if any key owned |
| 8 | Sunstone: if `witchflag`, `speak(60)` |

Returns to ITEMS: `gomenu(ITEMS)`.

#### KEYS Mode

Convert `hit -= 5` to key index 0–5. If `stuff[hit+KEYBASE] > 0`: scan 9 directions around hero via `doorfind(x, y, hit+1)`. If door found, decrement key. Return to ITEMS.

#### GIVE Mode

Requires `nearest_person != 0`.

| hit | Action |
|-----|--------|
| 5 | Gold: wealth − 2, random `kind++`. Beggar→`speak(24+goal)`, else `speak(50)` |
| 8 | Bone: if spectre (0x8a)→`speak(48)`, drops crystal shard (item 140) |

#### FILE Mode

`savegame(hit)` — slot 0–7 (A–H). `svflag` determines save vs. load. Return to GAME.

### 25.6 Keyboard Shortcuts

38 entries via `letter_list[38]`:

| Key | Action | Key | Action |
|-----|--------|-----|--------|
| I | List inventory | Y | Yell |
| T | Take | S | Say |
| ? | Look | A | Ask |
| U | Use submenu | Space | Pause toggle |
| G | Give submenu | M | Music toggle |
| O | Buy food | F | Sound toggle |
| R | Buy arrows | Q | Quit |
| 8 | Buy vial | L | Load |
| C | Buy mace | V | Save |
| W | Buy sword | X | Exit |
| B | Buy bow | 1–7 | Equip/use items |
| E | Buy totem | K | Keys submenu |
| F1–F7 | Magic spells | | |

SAVEX guard: V and X blocked unless `cmode == SAVEX`. KEYS special: if `cmode == KEYS` and key '1'–'6', dispatch `do_option(key - '1' + 5)` directly.

### 25.7 Compass Display

Two 48×24 pixel bitmaps as raw bitplane data:
- `_hinor`: base compass (all directions normal)
- `_hivar`: highlighted direction segments

Rendered on `bm_text` at (567, 15). Only bitplane 2 differs. Direction regions: `comptable[10]` — 8 cardinal/ordinal rectangles at indices 0–7 plus **2 null entries at indices 8 and 9** used to render "no highlight".

**Highlight source (behavioral requirement).** The highlighted wedge is driven by the **resolved input direction this tick** (keyboard / joystick / mouse-click), not by the player's persistent `facing` value. The resolved direction uses the same `com2[9]` table as movement (§9.1) and takes one of the values 0–7 (NW, N, NE, E, SE, S, SW, W) or **9 (no input this tick)**. When the value is 9, the comptable lookup hits a null region and only `_hinor` is drawn — i.e. **no wedge is highlighted while the player is idle**. `drawcompass()` is invoked only when the resolved direction changes (`oldir` → new), so the base bitmap is not redundantly re-blitted every frame.

Player `facing` is updated from the resolved direction only when the direction is 0–7; when the direction is 9, `facing` retains its last value so the sprite still faces the last-walked direction, but the compass highlight clears.

### 25.8 Stats & HUD

Status bar: `vp_text` (640×57 hi-res). Color palette from `textcolors[20]` — NOT affected by day/night fading.

Stats via print queue:
- `prq(7)`: Full stat line at y=52 — Brv x=14, Lck x=90, Knd x=168, Wlth x=321
- `prq(4)`: Vitality at (245, 52)

Menu display: `print_options()` on `rp_text2`. Two columns, 6 rows, each label 5 chars.

### 25.9 `cheat1` Debug Mode

In the original game, `cheat1` is persisted in the save file (byte offset 18 of the 80-byte block) and enabled only via hex-editing. When set, it gates the following debug keys:

| Key | Effect |
|-----|--------|
| B | Summon Swan; if already active carrier, also grant Golden Lasso (`stuff[5]=1`) |
| . | Add 3 to random `stuff[]` entry (range 0–30) |
| R | Call `rescue()` |
| = | Call `prq(2)` |
| F9 | Advance `daynight` by 1000 |
| F10 | Call `prq(3)` |
| ↑ / ↓ | Teleport hero ±150 in Y |
| ← / → | Teleport hero ±280 in X |

Also gates map spell region restriction: when `cheat1 == 0`, map returns early if `region_num > 7`.

**Port implementation:** The Rust port exposes `cheat1` via a debug-console toggle rather than requiring save-file hex-editing — see DEBUG_SPECIFICATION §"Mutation Commands" (`cheat` command). The gameplay effect of each key above is unchanged; only the enablement path differs.

---


