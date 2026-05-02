## 14. Day/Night Cycle & Clock

### Requirements

| ID | Requirement |
|----|-------------|
| R-CLOCK-001 | `daynight` shall be a 16-bit unsigned counter cycling from 0 to 23999, incrementing by 1 per non-scrolling game tick. Full cycle = 24000 ticks (≈ 6.7 minutes at 60 Hz in the original game). |
| R-CLOCK-002 | `lightlevel` shall be a triangle wave: `daynight / 40`; if ≥ 300, then `600 − value`. Range: 0 (midnight) to 300 (midday). |
| R-CLOCK-003 | `dayperiod` shall be `daynight / 2000` (values 0–11). Time-of-day events shall trigger at period transitions: 0 = midnight event(28), 4 = morning event(29), 6 = midday event(30), 9 = evening event(31). Periods 1–3, 5, 7–8, 10–11 are silent. |
| R-CLOCK-004 | Time shall not advance during freeze spells (`freeze_timer > 0`). |
| R-CLOCK-005 | During sleep: `daynight += 63` per tick (plus normal +1 = 64 effective advance per tick). |
| R-CLOCK-006 | Spectre visibility shall switch at `lightlevel < 40` (deep night, daynight < 1600 or > 22400): `ob_listg[5].ob_stat = 3` (visible/interactive); otherwise `ob_stat = 2` (hidden). |
| R-CLOCK-007 | Outdoor palette shall be faded via `day_fade()` every 4 ticks (`(daynight & 3) == 0`) or during screen rebuild (`viewstatus > 97`), using `lightlevel` to scale RGB components with clamping (red min 10 max 100, green min 25 max 100, blue min 60 max 100). |
| R-CLOCK-008 | Indoor areas (`region_num >= 8`) shall use fixed full brightness `(100, 100, 100)` with no day/night palette variation. |
| R-CLOCK-009 | Color 31 (sky) shall be overridden per region: desert (region 4) = `0x0980`, dungeon (region 9) with `secret_timer` active = `0x00f0`, dungeon normal = `0x0445`, all others = `0x0bdf`. |
| R-CLOCK-010 | Green Jewel light effect (`light_timer > 0`) shall add 200 to the red channel parameter in `day_fade()` and boost per-pixel red to match green when red < green, producing a warm amber glow. |
| R-CLOCK-011 | Nighttime vegetation boost: palette colors 16–24 shall receive extra blue at twilight (green 21–49: +2 blue; green 50–74: +1 blue). Blue shift factor: `(100 − green) / 3`. |
| R-CLOCK-012 | Music mood shall transition between day and night themes at `lightlevel` threshold 120 (day > 120, night ≤ 120), corresponding to daynight ≈ 4800 (dawn) and ≈ 19200 (dusk). |
| R-CLOCK-013 | Encounter spawn rate shall NOT vary with time of day; `danger_level` depends on region and extent type, not `lightlevel`. |
| R-CLOCK-014 | Innkeeper lodging dialogue shall trigger when `dayperiod > 7` (evening/night). |
| R-CLOCK-015 | `revive()` shall initialize `daynight = 8000` (morning) and `lightlevel = 300`. |

### User Stories

- As a player, I experience a day/night cycle that affects visibility, palette coloring, and music.
- As a player, sleeping advances time rapidly until morning or I'm rested.
- As a player, spectres are only visible during the deepest part of night.
- As a player, indoor areas have constant lighting unaffected by time of day.
- As a player, the Green Jewel spell provides a warm amber glow in dark outdoor areas.

---


