# Day/Night Cycle — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, fmain2.c
> Cross-refs: [RESEARCH §3](../RESEARCH.md#19-daynight-cycle), [game-loop.md#no_motion_tick](game-loop.md#no_motion_tick), [visual-effects.md#fade_page](visual-effects.md#fade_page)

## Overview

Time of day is tracked by a single `u16` counter, `daynight`, that advances once per no-motion tick (i.e. while the map is not scrolling, see `game-loop.md#no_motion_tick`) and wraps at 24000. Three observable quantities are derived from it each tick: `lightlevel` — a triangular 0..300 brightness ramp; `dayperiod` — an integer 0..11 bucket that fires four narrator announcements ("midnight", "morning", "midday", "evening") at its transitions; and an implicit deep-night flag (`lightlevel < 40`) that gates the Spectre NPC's visibility.

The counter is frozen while the time-stop spell runs (`freeze_timer > 0`) and accelerated 64× while the hero is asleep (`STATE_SLEEP`). Sleep is the only "rest" mechanic in the game: fatigue drains one point per sleep-tick and the hero wakes either on full rest, on the 9000–10000 dawn window when still partially rested, or on a 1-in-64 roll while a battle is live.

Downstream, three subsystems poll the counter without themselves being documented here:

- **Palette** — [`day_fade`](#day_fade) runs every 4 ticks (or whenever the frame is being redrawn) and feeds the triangular `lightlevel` into [`fade_page`](visual-effects.md#fade_page) as per-channel R/G/B weights, with an indoor override (regions 8/9 always render at full brightness) and a torch-spell warm tint (`light_timer`).
- **Music** — [`setmood`](#setmood) is called every 8 ticks outside of combat (see [`hunger_fatigue_tick`](#hunger_fatigue_tick)) and switches to the day or night outdoor score at `lightlevel > 120` / `<= 120`, with higher-priority overrides for death, astral plane, battle, and indoor regions.
- **Hunger/fatigue/safe-zone/auto-eat/vitality regen** — cadence-masked blocks in the no-motion tick (`daynight & 127`, `daynight & 7`, `daynight & 0x3ff`). These are part of [`hunger_fatigue_tick`](#hunger_fatigue_tick) and [`tick_daynight`](#tick_daynight).

Encounter placement (`daynight & 15` / `& 31`) is documented in [encounters.md](encounters.md) and is not repeated here.

## Symbols

All globals used below are referenced via the per-function `Calls:` line until they are registered in [SYMBOLS.md](SYMBOLS.md). Proposed SYMBOLS.md additions (see final report) are listed as part of the report, not applied in this PR.

Globals already registered and used here: `anim_list`, `menus` (and `CMODE_GAME`), `STATE_DEAD`, `STATE_SLEEP`, `STATE_STILL`.

Ad-hoc globals/functions referenced through `Calls:` in this file: `daynight`, `lightlevel`, `dayperiod`, `light_timer`, `freeze_timer`, `fatigue`, `hunger`, `battleflag`, `region_num`, `hero_x`, `hero_y`, `viewstatus`, `actors_on_screen`, `actors_loading`, `witchflag`, `safe_flag`, `safe_r`, `safe_x`, `safe_y`, `stuff`, `brave`, `ob_listg`, `new_wave`, `track`, `pagecolors`, `GAME`, `event`, `prq`, `rand64`, `fade_page`, `playscore`, `setscore`, `stopscore`, `day_fade`, `setmood`, `STUFF_FOOD`.

## tick_daynight

Source: `fmain.c:2014-2045`
Called by: `no_motion_tick`
Calls: `sleep_tick`, `day_fade`, `event`, `prq`, `daynight`, `lightlevel`, `dayperiod`, `freeze_timer`, `ob_listg`, `anim_list`, `brave`, `STATE_DEAD`

```pseudo
def tick_daynight() -> None:
    """Advance the daynight counter, reclassify dayperiod, refresh lightlevel, gate spectre visibility, regen vitality."""
    sleep_tick()                                              # fmain.c:2014-2021 — applies only while STATE_SLEEP
    if not freeze_timer:                                      # fmain.c:2023 — time-stop spell freezes the clock
        daynight = daynight + 1                               # fmain.c:2024 — base +1 per no-motion tick
        if daynight >= 24000:                                 # fmain.c:2024 — 24000 = one full day cycle
            daynight = 0                                      # fmain.c:2024 — wrap to midnight
    lightlevel = daynight / 40                                # fmain.c:2025 — 40 = daynight-to-lightlevel scale (600 buckets)
    if lightlevel >= 300:                                     # fmain.c:2026 — 300 = noon peak (mirror pivot)
        lightlevel = 600 - lightlevel                         # fmain.c:2026 — 600 = 2×noon (triangular fold)
    if lightlevel < 40:                                       # fmain.c:2027 — 40 = deep-night spectre threshold
        ob_listg[5].ob_stat = 3                               # fmain.c:2027 — slot 5 = Spectre world object; 3 = visible/interactive
    else:
        ob_listg[5].ob_stat = 2                               # fmain.c:2028 — 2 = hidden
    bucket = daynight / 2000                                  # fmain.c:2029 — 2000 = dayperiod bucket width (12 buckets/day)
    if bucket != dayperiod:
        dayperiod = bucket
        match dayperiod:
            case 0:
                event(28)                                     # fmain.c:2032 — speak "midnight" (narr.asm:45)
            case 4:                                           # fmain.c:2033 — 4 = morning bucket
                event(29)                                     # fmain.c:2033 — speak "morning"
            case 6:                                           # fmain.c:2034 — 6 = midday bucket
                event(30)                                     # fmain.c:2034 — speak "midday"
            case 9:                                           # fmain.c:2035 — 9 = evening bucket
                event(31)                                     # fmain.c:2035 — speak "evening"
    day_fade()                                                # fmain.c:2038 — per-tick palette refresh
    if (daynight & 0x3ff) == 0:                               # fmain.c:2040 — 0x3ff = every 1024 ticks
        if anim_list[0].vitality < (15 + brave / 4):          # fmain.c:2041 — 15 = base HP cap; 4 = brave divisor
            if anim_list[0].state != STATE_DEAD:              # fmain.c:2041 — no regen on dead hero
                anim_list[0].vitality = anim_list[0].vitality + 1
                prq(4)                                        # fmain.c:2043 — 4 = "vitality-up" sound queue
```

## sleep_tick

Source: `fmain.c:2014-2021`
Called by: `tick_daynight`
Calls: `rand64`, `daynight`, `fatigue`, `battleflag`, `anim_list`, `hero_y`, `STATE_SLEEP`, `STATE_STILL`

```pseudo
def sleep_tick() -> None:
    """Accelerate the daynight clock and drain fatigue while the hero is asleep; watch wake conditions."""
    if anim_list[0].state != STATE_SLEEP:
        return
    daynight = daynight + 63                                  # fmain.c:2015 — +63 plus tick_daynight's +1 → 64× rate
    if fatigue:
        fatigue = fatigue - 1                                 # fmain.c:2016 — one fatigue point per sleep-tick
    wake = False
    if fatigue == 0:                                          # fmain.c:2017 — fully rested
        wake = True
    elif fatigue < 30 and daynight > 9000 and daynight < 10000:   # fmain.c:2018 — 30 = near-rested, 9000/10000 = dawn wake window (dayperiod 4)
        wake = True
    elif battleflag and rand64() == 0:                        # fmain.c:2019 — 1-in-64 wake per tick during battle
        wake = True
    if wake:
        anim_list[0].state = STATE_STILL                      # fmain.c:2020
        anim_list[0].abs_y = anim_list[0].abs_y & 0xffe0      # fmain.c:2021 — 0xffe0 = snap to 32-px tile row
        hero_y = anim_list[0].abs_y                           # fmain.c:2021
```

## day_fade

Source: `fmain2.c:1653-1660`
Called by: `tick_daynight`, `check_door` (region change, `fmain.c:1890`)
Calls: `fade_page`, `light_timer`, `daynight`, `viewstatus`, `region_num`, `lightlevel`, `pagecolors`

```pseudo
def day_fade() -> None:
    """Cadence-masked driver for fade_page: outdoor regions ramp with lightlevel, indoor regions stay at full brightness, torch spell warms the red channel."""
    if light_timer:
        ll = 200                                              # fmain2.c:1655 — 200 = Green-Jewel torch red-channel boost
    else:
        ll = 0
    if (daynight & 3) == 0 or viewstatus > 97:                # fmain2.c:1656 — 3 = every-4th-tick mask; 97 = minimum viewstatus that forces a redraw (98/99)
        if region_num < 8:                                    # fmain2.c:1657 — 8 = first indoor region (8=buildings, 9=citadel)
            fade_page(lightlevel - 80 + ll, lightlevel - 61, lightlevel - 62, True, pagecolors)   # fmain2.c:1658 — 80/61/62 = per-channel night offsets from lightlevel
        else:
            fade_page(100, 100, 100, True, pagecolors)        # fmain2.c:1659 — 100 = full brightness (indoor override, no day/night)
```

## setmood

Source: `fmain.c:2936-2957`
Called by: `tick_daynight` (indirect, via `hunger_fatigue_tick` every 8 ticks), `check_door`, `aftermath`, `rescue`
Calls: `playscore`, `setscore`, `stopscore`, `anim_list`, `hero_x`, `hero_y`, `battleflag`, `region_num`, `lightlevel`, `new_wave`, `menus`, `track`, `GAME`

```pseudo
def setmood(now: i8) -> None:
    """Select and (cross-)fade to the music track matching the current death/location/combat/time-of-day mood."""
    if anim_list[0].vitality == 0:
        off = 6 * 4                                           # fmain.c:2938 — 6 = death song slot; 4 = tracks per song (channels)
    elif hero_x > 0x2400 and hero_x < 0x3100 and hero_y > 0x8200 and hero_y < 0x8a00:   # fmain.c:2939-2940 — astral plane coord box (map tile ~0x29,0x84 region)
        off = 4 * 4                                           # fmain.c:2941 — 4 = astral song slot
    elif battleflag:
        off = 4                                               # fmain.c:2942 — 4 = battle track base (slot 1)
    elif region_num > 7:                                      # fmain.c:2943 — 7 = last outdoor region (indoor = 8, 9)
        off = 5 * 4                                           # fmain.c:2944 — 5 = indoor song slot
        if region_num == 9:                                   # fmain.c:2945 — 9 = dungeon region (citadel)
            new_wave[10] = 0x0307                             # fmain.c:2945 — wave param slot 10; 0x0307 = dungeon timbre
        else:
            new_wave[10] = 0x0100                             # fmain.c:2946 — 0x0100 = building timbre
    elif lightlevel > 120:                                    # fmain.c:2948 — 120 = day/night music threshold (lightlevel peaks at 300)
        off = 0
    else:
        off = 8                                               # fmain.c:2949 — 8 = night song track base (slot 2)
    if menus[GAME].enabled[6] & 1:                            # fmain.c:2951 — 6 = "Music" menu entry in GAME submenu
        if now:
            playscore(track[off], track[off + 1], track[off + 2], track[off + 3])   # fmain.c:2953 — immediate start
        else:
            setscore(track[off], track[off + 1], track[off + 2], track[off + 3])    # fmain.c:2954 — queue crossfade
    else:
        stopscore()                                           # fmain.c:2956
```

## hunger_fatigue_tick

Source: `fmain.c:2188-2220`
Called by: `no_motion_tick`
Calls: `setmood`, `event`, `prq`, `daynight`, `battleflag`, `anim_list`, `actors_on_screen`, `actors_loading`, `witchflag`, `safe_flag`, `safe_r`, `safe_x`, `safe_y`, `region_num`, `hero_x`, `hero_y`, `hunger`, `fatigue`, `stuff`, `STATE_DEAD`, `STATE_SLEEP`, `STUFF_FOOD`

```pseudo
def hunger_fatigue_tick() -> None:
    """Every-128-tick safe-zone capture + auto-eat; every-8-tick music refresh; every-128-tick hunger/fatigue advance with warnings, HP drain, and forced sleep."""
    if (daynight & 127) == 0:                                 # fmain.c:2188 — 127 = every-128-tick mask (safe-zone cadence)
        if (not actors_on_screen and not actors_loading and not witchflag
                and anim_list[0].environ == 0
                and safe_flag == 0
                and anim_list[0].state != STATE_DEAD):        # fmain.c:2188-2192 — safe-zone predicate
            safe_r = region_num                               # fmain.c:2193
            safe_x = hero_x                                   # fmain.c:2194
            safe_y = hero_y                                   # fmain.c:2194
            if hunger > 30 and stuff[STUFF_FOOD]:             # fmain.c:2195 — 30 = auto-eat hunger threshold
                stuff[STUFF_FOOD] = stuff[STUFF_FOOD] - 1
                hunger = hunger - 30                          # fmain.c:2196 — 30 = hunger satisfied per ration
                event(37)                                     # fmain.c:2196 — speak "You eat some food"
    if (daynight & 7) == 0 and not battleflag:                # fmain.c:2198 — 7 = every-8-tick mask (music refresh)
        setmood(0)
    if (daynight & 127) != 0:                                 # fmain.c:2199 — 127 = every-128-tick mask (hunger cadence)
        return
    if not anim_list[0].vitality:                             # fmain.c:2199 — no hunger on dead hero
        return
    if anim_list[0].state == STATE_SLEEP:                     # fmain.c:2200 — asleep: sleep_tick handles fatigue
        return
    hunger = hunger + 1                                       # fmain.c:2201
    fatigue = fatigue + 1                                     # fmain.c:2202
    if hunger == 35:                                          # fmain.c:2203 — 35 = first-hunger warning
        event(0)                                              # fmain.c:2203 — speak "getting hungry"
    elif hunger == 60:                                        # fmain.c:2204 — 60 = second-hunger warning
        event(1)
    elif (hunger & 7) == 0:                                   # fmain.c:2205 — 7 = every-8-hunger-tick inner cadence
        if anim_list[0].vitality > 5:                         # fmain.c:2206 — 5 = HP floor before exhaustion branch
            if hunger > 100 or fatigue > 160:                 # fmain.c:2207 — 100/160 = HP-drain thresholds
                anim_list[0].vitality = anim_list[0].vitality - 2   # fmain.c:2208 — -2 HP per punishment tick
                prq(4)                                        # fmain.c:2208 — 4 = pain-grunt sound queue
            if hunger > 90:                                   # fmain.c:2209 — 90 = chronic-hunger complaint
                event(2)
        elif fatigue > 170:                                   # fmain.c:2211 — 170 = forced-sleep fatigue cap
            event(12)                                         # fmain.c:2212 — speak "collapse"
            anim_list[0].state = STATE_SLEEP
        elif hunger > 140:                                    # fmain.c:2213 — 140 = starve-sleep cap
            event(24)                                         # fmain.c:2214 — speak "faint from hunger"
            hunger = 130                                      # fmain.c:2214 — 130 = post-faint hunger reset
            anim_list[0].state = STATE_SLEEP
    if fatigue == 70:                                         # fmain.c:2218 — 70 = first-fatigue warning
        event(3)                                              # fmain.c:2218 — speak "weary"
    elif hunger == 90:                                        # fmain.c:2219 — 90 = hunger-echo warning (distinct from 2209)
        event(4)                                              # fmain.c:2219 — speak "starving"
```

## Notes

- **Why `tick_daynight` calls `sleep_tick` first**: the source inlines the sleep block *before* the counter increment, so the `+63` inside `sleep_tick` plus the `+1` inside `tick_daynight` sum to the documented 64× sleep rate.
- **Dayperiod buckets that fire no event**: buckets 1, 2, 3, 5, 7, 8, 10, 11 transition silently — see `fmain.c:2031`'s `switch` default (no `default:` arm).
- **No rest-at-the-inn function**: sleeping at an inn is gameplay wrapping around the bartender speech (`bartender_speech` in [npc-dialogue.md](npc-dialogue.md)) plus the normal `STATE_SLEEP` wake rules. The "morning" wake-up at an inn is produced by the `fatigue < 30 && daynight in (9000, 10000)` clause in `sleep_tick` — not by a dedicated function.
- **Cheat advance**: `fmain.c:1336` adds 1000 to `daynight` on cheat key `18`. Not included in the per-tick spec because it is an input handler, not a clock primitive.
- **Encounter cadence** (`daynight & 15`, `daynight & 31`) is covered by [encounters.md](encounters.md); those blocks live in the same `no_motion_tick` dispatcher, not in `tick_daynight`.
