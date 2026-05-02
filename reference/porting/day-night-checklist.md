# Day/Night Cycle Checklist

Source scope:
- `fmain.c:572` (`daynight`, `lightlevel` declaration)
- `fmain.c:2014-2036` (daynight increment, sleep acceleration, lightlevel calc, dayperiod transitions)
- `fmain.c:598` (`dayperiod` declaration)
- `fmain2.c:1653-1660` (`day_fade()` — palette interpolation)
- `fmain.c:2905` (`rescue()` initialization: `daynight = 8000`)
- `fmain.c:1336` (cheat: advance daynight by 1000)

Purpose:
- Ensure ports implement the day/night counter, light level formula, dayperiod event triggers, palette fade, and hunger/fatigue effects correctly.

## A. `daynight` Counter

- [ ] Unsigned 16-bit counter, range 0–23999 representing one full 24-hour cycle.
- [ ] Increments by 1 each game tick when player is stationary (`dif_y == 0`) AND `freeze_timer == 0` — `fmain.c:2023-2024`.
- [ ] Wraps: `if (daynight++ >= 24000) daynight = 0` — value never reaches 24000 (`fmain.c:2024`).
- [ ] Sleep acceleration: while `state == SLEEP`, `daynight += 63` per tick (64× total with the +1) — `fmain.c:2014`.
- [ ] Freeze: `freeze_timer > 0` completely halts daynight advancement — `fmain.c:2023`.
- [ ] Cheat advance: key 18 in cheat mode adds 1000 — `fmain.c:1336`.
- [ ] Initialization on brother succession: `daynight = 8000` (morning) — `fmain.c:2905`.

## B. `lightlevel` Calculation

- [ ] Derived each tick from `daynight`: `lightlevel = daynight / 40` — `fmain.c:2025`.
- [ ] Triangular mirror: `if (lightlevel >= 300) lightlevel = 600 - lightlevel` — `fmain.c:2026`.
- [ ] Result range: 0 (midnight, darkest) to 300 (noon, brightest).
- [ ] Symmetric: identical brightness at equal distances from midnight.

| `daynight` | `lightlevel` | Phase |
|------------|-------------|-------|
| 0 | 0 | Midnight |
| 6000 | 150 | Dawn/dusk |
| 12000 | 300 | Noon |
| 18000 | 150 | Dusk/dawn |
| 23999 | ~1 | Just before midnight |

## C. `dayperiod` and Time-of-Day Events

- [ ] `dayperiod = daynight / 2000`, giving values 0–11 — `fmain.c:2029`.
- [ ] Check: `if (i != dayperiod)` — transition events fire ONLY when period changes, not every tick — `fmain.c:2030`.
- [ ] Trigger `event()` calls on these transitions only:

| `dayperiod` | `daynight` Range | Event | Message |
|-------------|-----------------|-------|---------|
| 0 | 0–1999 | `event(28)` | "It was midnight." |
| 4 | 8000–9999 | `event(29)` | "It was morning." |
| 6 | 12000–13999 | `event(30)` | "It was midday." |
| 9 | 18000–19999 | `event(31)` | "Evening was drawing near." |

- [ ] Periods 1,2,3,5,7,8,10,11 trigger no message — `dayperiod` still updates.
- [ ] `dayperiod` is NOT saved (it's derived from saved `daynight` on load).

## D. `day_fade()` — Palette Interpolation (`fmain2.c:1653-1660`)

- [ ] Called when `(daynight & 3) == 0` (every 4 ticks) OR `viewstatus > 97` (full redraw needed).
- [ ] Outdoor only: `if (region_num < 8)` — no day/night fade inside buildings (region 8/9) — `fmain2.c:1656`.
- [ ] Indoor: always use `fade_page(100, 100, 100, TRUE, pagecolors)` — fixed 100% brightness indoors.
- [ ] Outdoor formula: `fade_page(lightlevel - 80 + ll, lightlevel - 61, lightlevel - 62, TRUE, pagecolors)`.
  - Red channel: `lightlevel - 80 + ll` (torch adds 200 to red)
  - Green channel: `lightlevel - 61`
  - Blue channel: `lightlevel - 62`
- [ ] Torch bonus: if `light_timer > 0`, set `ll = 200`; else `ll = 0` — `fmain2.c:1654`.
- [ ] `light_timer` set by Green Jewel (`stuff[10]`): `+= 760` per use — `fmain.c:3306`.

## E. Hunger and Fatigue Accumulation

- [ ] `hunger` and `fatigue` are saved in the 80-byte block (offsets 38 and 40).
- [ ] Hunger increments on each tick of movement — verify exact increment rate and threshold from `fmain.c`.
- [ ] Fatigue increments on activity — verify exact increment conditions.
- [ ] Fruit auto-consumed at safe checkpoint when `hunger > 30`: `stuff[24]--`, hunger reduced — `fmain.c:2195-2196`.
- [ ] Safe checkpoint: `safe_x`, `safe_y`, `safe_r` updated when player is in a safe zone.

## F. NPC Dialogue Using `dayperiod`

- [ ] Innkeeper lodging check: `if (dayperiod > 7)` triggers nighttime lodging offer — `fmain.c:3407`.
- [ ] Other NPC time checks: search for `dayperiod` usages in `fmain2.c` NPC dialogue handlers.

## G. `freeze_timer` (Time-Stop Spell)

- [ ] Green Jewel (magic item) sets `light_timer`; verify which spell sets `freeze_timer`.
- [ ] While `freeze_timer > 0`: no `daynight` advancement, no hunger/fatigue accumulation.
- [ ] `freeze_timer` saved in the 80-byte block (offset 68).
- [ ] Decrement `freeze_timer` each tick when active; verify decrement site in `fmain.c`.

## H. `safe_zone` Update

- [ ] `safe_x`, `safe_y`, `safe_r` saved in the 80-byte block.
- [ ] Updated when player is in a designated safe zone (inn, certain buildings).
- [ ] On death/brother succession: player respawns at `safe_x`, `safe_y`, region `safe_r`.
- [ ] `safe_r` must remain a valid world region (0–9).

## I. Known Quirks To Preserve (or Deliberately Normalize)

- [ ] `daynight` advances by 64 per tick while sleeping (63 + 1) — sleep is exactly 64× real time.
- [ ] `lightlevel` is computed fresh each tick from `daynight`; the saved value in the file is redundant but restored for continuity.
- [ ] `day_fade()` runs every 4 ticks, not every tick — palette updates are not frame-rate dependent.
- [ ] Freeze timer halts ALL time-dependent systems: no day advance, no hunger, no music tempo change.

## J. Minimum Parity Test Matrix

- [ ] `daynight` at 8000: `dayperiod` = 4 → triggers "It was morning." event once.
- [ ] `daynight` at 12000: `lightlevel` = 300 (peak brightness).
- [ ] Sleep state: `daynight` advances 64 per tick; full day cycle in ~375 ticks.
- [ ] `freeze_timer > 0`: `daynight` frozen; `lightlevel` and palette unchanged.
- [ ] Green Jewel use: `light_timer` increases; red channel boost of 200 applied in `day_fade()`.
- [ ] Region 8 (indoor): `fade_page(100,100,100)` regardless of time of day.
- [ ] Hunger > 30 at safe zone with Fruit: fruit auto-consumed.
