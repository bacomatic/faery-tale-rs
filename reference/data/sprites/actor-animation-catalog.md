# Actor Animation Frame Catalog

> Per-actor mapping from motion state to sprite frame index, including the
> sprite sheet (`seq_list` slot / `cfiles` index) and the exact image index
> within that sheet.
>
> Source: `fmain.c`, `fmain2.c`, `ftale.h`.
> Companion files: [actors.md](actors.md) (sheet atlases), [carriers.md](carriers.md)
> (RAFT/CARRIER/DRAGON/SETFIG sheets), [objects.md](objects.md) (weapon/effect overlays).

## How to compute the final frame

The renderer uses two values:

1. **`an->index`** — the "logical" frame index set by the animation state
   machine (`walk_step`, `still_step`, `fighting_step`, `shoot_step`,
   `death_step`, `actor_type_dispatch`). It is usually a `statelist[]` index.
2. **`atype`** — the `seq_list` slot that holds the sprite sheet.

For `PHIL` (hero/brothers) and `ENEMY` bodies, the renderer resolves the
physical sheet frame via `statelist[an->index].figure` (`fmain.c:2458`).
For `SETFIG`, `CARRIER`, `DRAGON`, and `RAFT`, `an->index` is the physical
frame directly.

### Rendering-time adjustments for ENEMY

After the state machine sets `an->index`, `select_atype_inum` applies two
race-specific adjustments before the final blit (`fmain.c:2459-2460`):

- **Race 4 (snake):** if `an->state < DYING`, `an->index += 0x24` (36).
- **All other enemies (`i > 0`):** force the `an->index` LSB to match race
  parity:
  - odd race (`race & 1`): `an->index |= 1`
  - even race (`race & 1 == 0`): `an->index &= 0xFFFE`

The parity adjustment means adjacent walk frames are swapped for odd-race
enemies. Wraith (`race 2`, even) keeps the base frame; Skeleton (`race 3`, odd)
gets the next frame.

---

## 1. Hero / Brothers

> Sheet: `seq_list[PHIL]` (`cfiles[0-2]`, 16×32 px, 67 frames)
> `actor_file` for brothers: `cfiles[6]` at load time (`fmain.c:2883`).

The hero (`anim_list[0]`) and brothers (`anim_list[1-2]`) share the same
PHIL layout. Brothers use the same walk/fight/death indices as the hero; only
the `FALL` animation differs per brother (`fallstates[brother*6 + n]`).

### State-to-frame table

| State | Index formula | Source |
|-------|---------------|--------|
| `WALKING` | `diroffs[d] + ((cycle + i) & 7)` unless mounted (`riding && i==0`), then `diroffs[d]` | `fmain.c:1632` |
| `STILL` | `diroffs[d] + 1` | `fmain.c:1662` |
| `FIGHTING` | `diroffs[d + 8] + s`, where `s` is the `trans_list` substate (0-11) | `fmain.c:1714` |
| `SHOOT1` (bow) | `diroffs[d + 8] + 10` | `fmain.c:1684` |
| `SHOOT3` (bow) | `diroffs[d + 8] + 11` | `fmain.c:1675` |
| `SHOOT1` / `SHOOT3` (wand) | `diroffs[d + 8]` | `fmain.c:1689` |
| `DYING` | `80` or `81` (alternates by `tactic` and facing half) | `fmain.c:1721-1723` |
| `DEAD` | `82` | `fmain.c:1724` |
| `SINK` | `83` | `fmain.c:1576` |
| `FROZEN` | `82` | `fmain.c:1728` |
| `OSCIL` | `84 + (cycle & 1)` | `fmain.c:1729` |
| `OSCIL+1` | `84` | `fmain.c:1730` |
| `SLEEP` | `86` | `fmain.c:1731` |
| `FALL` | `fallstates[brother * 6 + (tactic / 5)]` | `fmain.c:1734` |

`d` = facing (0-7). `diroffs[16] = {16,16,24,24,0,0,8,8,56,56,68,68,32,32,44,44}`
(`fmain.c:1010`). For walking and still, use indices 0-7; for fighting/shooting,
use indices 8-15.

### Physical PHIL frame lookup

The final physical frame is `statelist[an->index].figure`. Example for south
facing (`d = 4`, `diroffs[d] = 0`):

| State | `an->index` | `statelist[].figure` | Description |
|-------|-------------|----------------------|-------------|
| Walk step 0 | 0 | 0 | Walk S — step 0 |
| Walk step 1 | 1 | 1 | Walk S — step 1 |
| Walk step 7 | 7 | 7 | Walk S — step 7 |
| Still | 1 | 1 | Walk S — step 1 |
| Fight swing 0 | 32 | 32 | Fight S — swing 0 |
| Fight swing 2 | 34 | 33 | Fight S — swing 2 |
| Death A | 80 | 47 | Death sequence frame A |
| Death B | 81 | 63 | Death sequence frame B |
| Dead | 82 | 39 | Death sequence frame C |

Full `statelist` table is in [actors.md](actors.md) and
[`logic/sprite-rendering.md`](../logic/sprite-rendering.md).

---

## 2. ENEMY by Race

> Sheet: `seq_list[ENEMY]` (16×32 px, 64 frames). Which `cfiles` entry is
> loaded depends on `encounter_chart[encounter_type].file_id`:

| `encounter_type` | Race | Monster | `file_id` | `cfiles` entry | Sheet loaded |
|------------------|------|---------|-----------|----------------|--------------|
| 0 | 0 | Ogre | 6 | `cfiles[6]` | Ogre file |
| 1 | 1 | Orcs | 6 | `cfiles[6]` | Ogre file |
| 2 | 2 | Wraith | 7 | `cfiles[7]` | Ghost file |
| 3 | 3 | Skeleton | 7 | `cfiles[7]` | Ghost file |
| 4 | 4 | Snake | 8 | `cfiles[8]` | DKnight/Spider file |
| 5 | 5 | Salamander | 7 | `cfiles[7]` | Ghost file |
| 6 | 6 | Spider | 8 | `cfiles[8]` | DKnight/Spider file |
| 7 | 7 | Dark Knight | 8 | `cfiles[8]` | DKnight/Spider file |
| 8 | 8 | Loraii | 9 | `cfiles[9]` | Necromancer/Farmer file |
| 9 | 9 | Necromancer | 9 | `cfiles[9]` | Necromancer/Farmer file |
| 10 | 10 | Woodcutter | 9 | `cfiles[9]` | Necromancer/Farmer file |

### Common ENEMY state formulas

Most enemies use the same `an->index` formulas as the hero. Differences:

| State | Index formula | Notes |
|-------|---------------|-------|
| `WALKING` | `diroffs[d] + ((cycle + i) & 7)` | **Wraith (race 2) skips the offset** and stays on `diroffs[d]` (`fmain.c:1632`) |
| `STILL` | `diroffs[d] + 1` | Same as hero |
| `FIGHTING` | `diroffs[d + 8] + s`, but `s = 6/7` collapses to `8` for `i > 2` | `fmain.c:1713` |
| `SHOOT1` | `diroffs[d + 8] + 11` | Enemy bow draw frame; hero uses `+10` (`fmain.c:1683`) |
| `SHOOT3` | `diroffs[d + 8] + 11` | Same as hero |
| `DYING` | `80` / `81` / `82` | Same as hero |
| `DEAD` | `82` | Same as hero |

### Race-specific rendering overrides

| Race | Special handling | Final physical frame |
|------|------------------|----------------------|
| 0 Ogre | even race: `an->index &= 0xFFFE` | `statelist[even_index].figure` |
| 1 Orcs | odd race: `an->index |= 1` | `statelist[odd_index].figure` |
| 2 Wraith | even race; walk frame frozen | `statelist[even_index].figure`; walk index never advances |
| 3 Skeleton | odd race | `statelist[odd_index].figure` |
| 4 Snake | `an->index += 0x24` if alive | `statelist[index + 36].figure` |
| 5 Salamander | odd race | `statelist[odd_index].figure` |
| 6 Spider | even race | `statelist[even_index].figure` |
| 7 Dark Knight | odd race; at `vitality == 0`, `an->state = STILL` (intended), `an->index = 1` | `statelist[odd_index].figure`; zero-HP pose uses frame `statelist[1].figure = 1` |
| 8 Loraii | even race | `statelist[even_index].figure` |
| 9 Necromancer | odd race | `statelist[odd_index].figure` |
| 10 Woodcutter | even race | `statelist[even_index].figure` |

### Wraith (race 2) detail

The wraith has three documented special cases:

1. **Walk animation frozen** (`fmain.c:1632`): no `((cycle+i)&7)` offset.
2. **Terrain bypass** (`fmain2.c:280`): ignores `proxcheck`.
3. **Water immunity** (`fmain.c:1641`, `fmain.c:1850`): `environ` forced to 0 and
   immune to drowning damage.

| Wraith state | `an->index` | `statelist[].figure` (after even LSB) | Notes |
|--------------|-------------|--------------------------------------|-------|
| Walking S | `0` | `0` | Base frame only |
| Walking W | `8` | `8` | Base frame only |
| Walking N | `16` | `16` | Base frame only |
| Walking E | `24` | `24` | Base frame only |
| Still S | `1` | `0` | `diroffs[4]+1 = 1`, then `& 0xFFFE` → `0` |
| Still W | `9` | `8` | `diroffs[6]+1 = 9`, then `& 0xFFFE` → `8` |
| Still N | `17` | `16` | `diroffs[0]+1 = 17`, then `& 0xFFFE` → `16` |
| Still E | `25` | `24` | `diroffs[2]+1 = 25`, then `& 0xFFFE` → `24` |

Because the wraith is even-race, the LSB-clearing step collapses the `STILL`
frame back to the same physical frame as `WALKING`, so the wraith truly has only
one frame per facing (unlike other races, where the parity adjustment preserves
the still/walk toggle).

### Snake (race 4) detail

The snake uses two independent overrides:

1. `update_actor_index` (k==4) sets `dex = ((cycle/2)&1) + diroffs[d]` while alive.
2. The renderer adds `0x24` (36) to `an->index` if `state < DYING`.

| Snake state | `an->index` before render | `an->index` after `+0x24` | `statelist[].figure` |
|-------------|---------------------------|---------------------------|----------------------|
| Walk S step 0 | `diroffs[4] = 0` | `36` | `34` |
| Walk S step 1 | `diroffs[4] + 1 = 1` | `37` | `34` |
| Walk W step 0 | `diroffs[6] = 8` | `44` | `40` |
| Walk W step 1 | `diroffs[6] + 1 = 9` | `45` | `40` |
| Walk N step 0 | `diroffs[0] = 16` | `52` | `43` |
| Walk N step 1 | `diroffs[0] + 1 = 17` | `53` | `42` |
| Walk E step 0 | `diroffs[2] = 24` | `60` | `50` |
| Walk E step 1 | `diroffs[2] + 1 = 25` | `61` | `50` |

This maps the snake sprites to the second half of the ENEMY sheet (frames
36-63), which matches the layout described in [actors.md](actors.md).

### Dark Knight (race 7) reanimation pose

At `fmain.c:1819-1822`, when an enemy with `race == 7` reaches `vitality == 0`,
the source contains a bug: `an->state == STILL;` is an equality comparison
rather than an assignment. The intended behavior is `an->state = STILL;` and
`an->index = 1`.

| State | `an->index` | `statelist[].figure` (after odd LSB) | Notes |
|-------|-------------|--------------------------------------|-------|
| Zero-HP reanimation | `1` | `1` | Odd race preserves LSB; frame 1 |
| Dying | `80` / `81` | `47` / `63` | Standard death sequence |
| Dead | `82` | `39` | Standard corpse frame |

---

## 3. SETFIG NPCs

> Sheet: `seq_list[SETFIG]` (`cfiles[13-17]`, 16×32 px, 8 frames per file).

SETFIG NPCs use `an->index` directly as the physical frame; no `statelist`
lookup is performed (`fmain.c:1550`). `setfig_table[ob_id]` selects the file
and the base frame.

> **Note:** The Witch (`ob_id 9`) here is the SETFIG town NPC, not the enemy
> Witch encountered in combat. The combat Witch is `type ENEMY`, `race 0x89`,
> loaded from `cfiles[8]` and uses the ENEMY animation rules above.

| `ob_id` | NPC | `cfiles` | Base `an->index` (`image_base`) | Dying | Dead | Talking |
|---------|-----|----------|----------------------------------|-------|------|---------|
| 0 | Wizard | 13 | 0 | `2` | `3` | `0` / `1` |
| 1 | Priest | 13 | 4 | `6` | `7` | `4` / `5` |
| 2 | Guard (front) | 14 | 0 | `2` | `3` | `0` / `1` |
| 3 | Guard (back) | 14 | 1 | `3` | `4` | `1` / `2` |
| 4 | Princess | 14 | 2 | `4` | `5` | `2` / `3` |
| 5 | King | 14 | 4 | `6` | `7` | `4` / `5` |
| 6 | Noble | 14 | 6 | `0` | `1` | `6` / `7` |
| 7 | Sorceress | 14 | 7 | `1` | `2` | `7` / `0` |
| 8 | Bartender | 15 | 0 | `2` | `3` | `0` / `1` |
| 9 | Witch | 16 | 0 | `4` | `5` | `0` / `1` |
| 10 | Spectre | 16 | 6 | `0` | `1` | `6` / `7` |
| 11 | Ghost | 16 | 7 | `1` | `2` | `7` / `0` |
| 12 | Ranger | 17 | 0 | `2` | `3` | `0` / `1` |
| 13 | Beggar | 17 | 4 | `6` | `7` | `4` / `5` |

Dying/dead offsets are `+2` / `+3` normally, but the Witch (`ob_id 9`) gets an
extra `+2` because of her larger sprite sheet: Dying = `0 + 2 + 2 = 4`,
Dead = `0 + 3 + 2 = 5` (`fmain.c:1551-1557`).

---

## 4. Carriers

### Bird / Swan

> Sheet: `seq_list[CARRIER]` loaded from `cfiles[11]` (64×64 px, 8 frames)

| State | `an->index` | Notes |
|-------|-------------|-------|
| Idle / still | `0` | `load_carrier(11)` sets `an->index = 0` (`fmain.c:2784`) |
| Flight | `1`–`7` | Walk cycle driven by `carrier_tick` (`fmain.c:1495-1547`) |
| Grounded (unmounted) | `1` | Rendered as `atype = RAFT`, `inum = 1` (`fmain.c:2463-2464`) |

### Turtle

> Sheet: `seq_list[CARRIER]` loaded from `cfiles[5]` (32×32 px, 16 frames)

| State | `an->index` | Notes |
|-------|-------------|-------|
| Idle / still | `0` | `load_carrier(5)` sets `an->index = 0` |
| Movement | `1`–`7` | Walk cycle driven by `carrier_tick` |

### Dragon

> Sheet: `seq_list[DRAGON]` loaded from `cfiles[10]` (48×40 px, 5 frames)

| State | `an->index` | Notes |
|-------|-------------|-------|
| Idle | `0` | `actor_type_dispatch` sets `an->index = 0` (`fmain.c:1482`) |
| Fire-breath | `1`–`2` | Random 1-in-4 breath animation (`fmain.c:1487`) |
| Dying | `3` | `fmain.c:1483` |
| Dead | `4` | `fmain.c:1484` |

---

## 5. Raft

> Sheet: `seq_list[RAFT]` loaded from `cfiles[4]` (32×32 px, 2 frames)

| State | `atype` / `inum` | Notes |
|-------|------------------|-------|
| Raft on water | `RAFT`, `0` | `actor_type_dispatch` returns `2` (skip `update_actor_index`) (`fmain.c:1562-1574`) |
| Grounded swan | `RAFT`, `1` | Used when `atype == CARRIER && riding == 0 && actor_file == 11` (`fmain.c:2463-2464`) |

---

## 6. Weapon overlays

Weapon overlays are rendered in a second pass (`passmode`) from the `OBJECTS`
sheet. Their index is derived from the body `an->index` and the weapon type:

| Weapon | `an->weapon` | Overlay formula | Source |
|--------|--------------|-----------------|--------|
| Bow | 4 | `statelist[an->index].wpn_no + 0` (see `objects.md` 0x50-0x57) | `fmain.c:2431-2433` |
| Wand | 5 | `an->facing + 103` | `fmain.c:2436` |
| Mace | 2 | `statelist[an->index].wpn_no + 32` | `fmain.c:2440` |
| Sword | 3 | `statelist[an->index].wpn_no + 48` | `fmain.c:2441` |
| Dirk | 1 | `statelist[an->index].wpn_no + 64` | `fmain.c:2442` |

The `wpn_no` values are in the `statelist` table (4th column). See
[objects.md](objects.md) for the resulting `OBJECTS` frame numbers.

---

## 7. Summary of special cases

| Actor | Special rule | Why it matters |
|-------|--------------|----------------|
| Hero mounted | `((cycle+i)&7)` walk offset skipped | Mounted hero is a single base frame |
| Wraith | Walk offset skipped + even parity | Only one frame per facing |
| Skeleton / DKnight / Orcs / Salamander / Necromancer | Odd parity (`\|1`) | Walk frames shifted by one |
| Snake | `+0x24` offset | Sprites live in second half of ENEMY sheet |
| Dark Knight | Zero-HP pose `an->index = 1` | Source bug: `==` vs `=` |
| SETFIG | `an->index` is physical frame | No `statelist` indirection |
| Dragon | Custom `actor_type_dispatch` | 5-frame sheet, not ENEMY layout |
| Grounded swan | Rendered as `RAFT` frame 1 | Reuses raft sheet for the unmounted bird |
