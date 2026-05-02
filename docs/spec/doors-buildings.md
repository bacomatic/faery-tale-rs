## 16. Doors & Buildings

### 16.1 Door Structure

```
struct door {
    xc1: u16,   // outside world X (pixel-space)
    yc1: u16,   // outside world Y (pixel-space)
    xc2: u16,   // inside world X (pixel-space)
    yc2: u16,   // inside world Y (pixel-space)
    type: i8,   // door visual/orientation type
    secs: i8,   // 1=buildings (region 8), 2=dungeons (region 9)
}
```

`DOORCOUNT = 86`. The table is sorted by `xc1` ascending to support binary search during outdoor→indoor transitions.

**secs field**:

| Value | Target Region | Description |
|-------|--------------|-------------|
| 1 | 8 | Building interiors (F9 image set) |
| 2 | 9 | Dungeons and caves (F10 image set) |

Region assignment: `if secs == 1 { new_region = 8 } else { new_region = 9 }`.

### 16.2 Door Type Constants

Horizontal types have bit 0 set (`type & 1`):

| Constant | Value | Orientation | Entries in doorlist |
|----------|-------|-------------|---------------------|
| HWOOD | 1 | Horizontal | 16 |
| VWOOD | 2 | Vertical | 3 |
| HSTONE | 3 | Horizontal | 11 |
| CRYST | 7 | Horizontal | 2 |
| BLACK | 9 | Horizontal | 5 |
| MARBLE | 10 | Vertical | 7 |
| LOG | 11 | Horizontal | 10 |
| HSTON2 | 13 | Horizontal | 12 |
| VSTON2 | 14 | Vertical | 2 |
| STAIR | 15 | Horizontal | 4 |
| DESERT | 17 | Horizontal | 5 |
| CAVE/VLOG | 18 | Vertical | 14 (4 cave + 10 cabin yard) |

CAVE and VLOG share value 18. Code checking `type == CAVE` also catches VLOG entries. Both use the same teleportation offset.

Unused defined types: VSTONE (4), HCITY (5), VCITY (6) — never appear in `doorlist[]`. SECRET (8) appears only in `open_list[]`.

#### Notable Door Patterns

- Entries 0–3: four identical copies of the same desert fort door (editing artifact, functionally harmless).
- 10 cabin pairs: each cabin has a VLOG "yard" door and a LOG "cabin" door (20 entries total).
- Crystal palace (idx 21–22): two adjacent doors for the same building.
- Stargate (idx 14–15): bidirectional portal. Entry 14 goes outdoor→region 8, entry 15 goes region 8→region 9.
- Village cluster (idx 31–39): 9 doors for the village.
- City cluster (idx 50–61): 12 doors for Marheim.

### 16.3 Locked Door System

#### Key Enum

```
enum ky { NOKEY=0, GOLD=1, GREEN=2, KBLUE=3, RED=4, GREY=5, WHITE=6 };
```

#### `struct door_open`

| Field | Type | Purpose |
|-------|------|---------|
| `door_id` | `u8` | Sector tile ID of the closed door |
| `map_id` | `u16` | Image block number identifying the door's region |
| `new1` | `u8` | Primary replacement tile ID |
| `new2` | `u8` | Secondary replacement tile ID (0 = none) |
| `above` | `u8` | Tile placement mode: 0=none, 1=above, 2=side, 3=back, 4=special cabinet |
| `keytype` | `u8` | Key required: 0=none, 1–6 per enum |

#### `open_list[17]`

| Idx | Key | Description |
|-----|-----|-------------|
| 0 | GREEN | HSTONE door (outdoor stone buildings) |
| 1 | NOKEY | HWOOD door (unlocked wooden doors) |
| 2 | NOKEY | VWOOD door (unlocked vertical wooden doors) |
| 3 | GREY | HSTONE2 door |
| 4 | GREY | VSTONE2 door |
| 5 | KBLUE | CRYST (crystal palace interiors) |
| 6 | GREEN | OASIS entrance |
| 7 | WHITE | MARBLE (keep doors) |
| 8 | GOLD | HGATE (gates) |
| 9 | GOLD | VGATE (vertical gates) |
| 10 | RED | SECRET passage |
| 11 | GREY | TUNNEL |
| 12 | GOLD | GOLDEN door (special 3-tile layout) |
| 13 | NOKEY | HSTON3 (unlocked) |
| 14 | NOKEY | VSTON3 (unlocked) |
| 15 | GREEN | CABINET (special 4-tile layout) |
| 16 | NOKEY | BLUE door (unlocked) |

Door tile changes are transient — they modify live `sector_mem` data only. Changes are lost when the sector reloads from disk. No save mechanism preserves opened door tiles.

### 16.4 `doorfind` Algorithm

Opens LOCKED doors (terrain tile type 15) by modifying map tiles. This system is separate from the `doorlist[]` teleportation system — `doorfind` operates on `open_list[]`.

1. **Locate terrain type 15** — tries `px_to_im(x, y)`, `px_to_im(x+4, y)`, `px_to_im(x-8, y)`
2. **Find top-left corner** — scans left (up to 2×16 px) and down (32 px)
3. **Convert to image coordinates** — `x >>= 4; y >>= 5`
4. **Get sector/region IDs** — `sec_id = *(mapxy(x, y))`, `reg_id = current_loads.image[(sec_id >> 6)]`
5. **Search `open_list[17]`** — match `map_id == reg_id && door_id == sec_id`, with key check `keytype == 0 || keytype == open_list[j].keytype`
6. **Replace tiles** — writes new tile IDs into `sector_mem` via `mapxy()`. Placement varies by `above` field
7. **Failure** — prints "It's locked." (suppressed by `bumped` flag)

#### Key Usage — Menu Handler

Player selects a key from the KEYS submenu. All 9 directions (0–8) at 16-pixel distance are checked via `doorfind(x, y, keytype)`. On success, the key is consumed (`stuff[hit + KEYBASE]--`).

#### Collision-Triggered Opening

When the player bumps terrain type 15, `doorfind(xtest, ytest, 0)` is called automatically. This opens only NOKEY doors (keytype match requires `keytype == 0`).

### 16.5 Region Transitions

#### Outdoor → Indoor (binary search)

Triggered when `region_num < 8` and the hero's aligned position matches a doorlist entry:

1. **Align** to 16×32 tile grid: `xtest = hero_x & 0xfff0; ytest = hero_y & 0xffe0`
2. **Riding check**: if riding, abort — cannot enter doors while mounted
3. **Binary search** on `doorlist` by `xc1`
4. **Orientation check**: horizontal doors skip if `hero_y & 0x10` is set; vertical doors skip if `(hero_x & 15) > 6`
5. **DESERT gate**: if `type == DESERT && stuff[STATBASE] < 5`, abort — need ≥5 gold statues
6. **Destination offset** by type:
   - CAVE/VLOG: `(xc2 + 24, yc2 + 16)`
   - Horizontal: `(xc2 + 16, yc2)`
   - Vertical: `(xc2 - 1, yc2 + 16)`
7. **Teleport**: `xfer(xtest, ytest, FALSE)`
8. **Visual transition**: `fade_page(100, 100, 100, TRUE, pagecolors)`

#### Indoor → Outdoor (linear scan)

Triggered when `region_num >= 8` and the hero matches a doorlist's `xc2`/`yc2`:

1. **Linear scan** through all 86 entries
2. **Match on `xc2`/`yc2`** with wider hit zone for horizontal doors
3. **Destination offset** by type:
   - CAVE/VLOG: `(xc1 - 4, yc1 + 16)`
   - Horizontal: `(xc1 + 16, yc1 + 34)`
   - Vertical: `(xc1 + 20, yc1 + 16)`
4. **Teleport**: `xfer(xtest, ytest, TRUE)` — TRUE recalculates region from position
5. **No fade** — exiting is instant, unlike entering

### 16.6 The `xfer()` Function

Performs teleportation between regions:

1. Adjust map scroll by same delta as hero position
2. Set hero position to destination
3. Clear encounters
4. If exiting indoors (`recalc` flag TRUE): recalculate region from coordinates
5. Load region data
6. Regenerate minimap
7. Force full screen redraw
8. Update music mood
9. Nudge hero downward if colliding with solid object at destination

### 16.7 Quicksand → Dungeon Transition

A non-door transition. When the player fully sinks (`environ == 30`) at `hero_sector == 181`: teleport to `(0x1080, 34950)` in region 9. NPCs caught in the same quicksand die.


