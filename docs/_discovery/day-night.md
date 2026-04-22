# Discovery: Day/Night Cycle & Palette System

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace the day/night cycle counter, palette fading, music selection, and gameplay effects of the day/night system.

## daynight Counter

**Declaration**: `fmain.c:572` — `USHORT daynight, lightlevel;`

The `daynight` variable is an unsigned 16-bit counter that cycles from 0 to 23999, representing a full day cycle.

### Increment Logic — fmain.c:2023-2024
```c
if (!freeze_timer) /* no time in timestop */
    if ((daynight++) >= 24000) daynight = 0;
```
- Increments by 1 each game tick (once per main loop iteration when player is stationary, i.e. `dif_y == 0`).
- Wraps at 24000 → 0.
- **Does NOT advance when `freeze_timer` is active** (time-stop spell effect).

### Initialization — fmain.c:2905
```c
daynight = 8000; lightlevel = 300;
```
Set during `rescue()` (brother death/succession). Starting value of 8000 = dayperiod 4 = morning, lightlevel 300 = peak brightness.

### Sleep Acceleration — fmain.c:2014
```c
if (anim_list[0].state == SLEEP)
{   daynight += 63;
```
While sleeping, daynight advances by 63 per tick (64× normal speed counting the +1 from line 2024).

### Cheat: line fmain.c:1336
```c
else if (key == 18 && cheat1) daynight += 1000;
```
Key 18 (mapped key) advances daynight by 1000 when cheat mode active.

## lightlevel

**Declaration**: `fmain.c:572` — `USHORT daynight, lightlevel;`

### Calculation — fmain.c:2025-2026
```c
lightlevel = daynight/40;
if (lightlevel >= 300) lightlevel = 600 - lightlevel;
```

This creates a **triangular wave** over the day cycle:
- `daynight` 0 → `lightlevel` 0 (midnight, darkest)
- `daynight` 6000 → `lightlevel` 150 (dawn, half brightness)
- `daynight` 12000 → `lightlevel` 300 → mirrored to 300 (noon, peak brightness)
- `daynight` 18000 → `lightlevel` 450 → mirrored to 150 (dusk, half brightness)
- `daynight` 23999 → `lightlevel` 599 → mirrored to 1 (just before midnight)

The result: lightlevel ranges from 0 (midnight) to 300 (noon), with a symmetric rise and fall.

### lightlevel Value Ranges
| daynight Range | lightlevel | Phase |
|---|---|---|
| 0 | 0 | Midnight (darkest) |
| 0–6000 | 0–150 | Late night → dawn |
| 6000–12000 | 150–300 | Dawn → noon |
| 12000 | 300 | Noon (brightest) |
| 12000–18000 | 300→150 | Noon → dusk |
| 18000–24000 | 150→0 | Dusk → midnight |

## dayperiod — Time-of-Day Events

**Declaration**: `fmain.c:598` — `short dayperiod;`

### Calculation — fmain.c:2029-2036
```c
i = (daynight / 2000);
if (i != dayperiod)
{   switch (dayperiod = i) {
    case 0: event(28); break;
    case 4: event(29); break;
    case 6: event(30); break;
    case 9: event(31); break;
    }
}
```

`dayperiod` = `daynight / 2000`, yielding values 0–11. Transitions trigger text events:

| dayperiod | daynight Range | Event | Message (narr.asm) |
|---|---|---|---|
| 0 | 0–1999 | event(28) | "It was midnight." — narr.asm:45 |
| 4 | 8000–9999 | event(29) | "It was morning." — narr.asm:46 |
| 6 | 12000–13999 | event(30) | "It was midday." — narr.asm:47 |
| 9 | 18000–19999 | event(31) | "Evening was drawing near." — narr.asm:48 |
| 1,2,3,5,7,8,10,11 | (other ranges) | (no event) | Silent transitions |

### dayperiod Usage in NPC Dialogue — fmain.c:3407
```c
else if (dayperiod > 7) speak(12);
```
Innkeeper checks `dayperiod > 7` (evening/night) for lodging dialogue.

## day_fade — Palette Interpolation

**Location**: `fmain2.c:1653-1660`

```c
day_fade()
{   register long ll;
    if (light_timer) ll = 200; else ll = 0;
    if ((daynight & 3) == 0 || viewstatus > 97)
        if (region_num < 8) /* no night inside buildings */
             fade_page(lightlevel-80+ll,lightlevel-61,lightlevel-62,TRUE,pagecolors);
        else fade_page(100,100,100,TRUE,pagecolors);
}
```

### Algorithm
1. **Torch bonus**: If `light_timer > 0`, adds 200 to the red (r) parameter. `light_timer` is set by Green Jewel (stuff[10]): `light_timer += 760` — `fmain.c:3306`.
2. **Update rate**: Only executes when `(daynight & 3) == 0` (every 4 ticks) OR `viewstatus > 97` (screen being redrawn, values 98/99).
3. **Indoor override**: If `region_num >= 8` (buildings region 8 or dungeons region 9), applies full brightness `(100,100,100)` with no day/night variation.
4. **Outdoor formula**: Passes to `fade_page()`:
   - **r** = `lightlevel - 80 + ll` (red channel: darkest, enhanced by torch)
   - **g** = `lightlevel - 61` (green channel: slightly brighter than red)
   - **b** = `lightlevel - 62` (blue channel: similar to green)

### Outdoor RGB at Key Times
| Phase | lightlevel | r (no torch) | g | b |
|---|---|---|---|---|
| Midnight | 0 | -80 (clamped 10) | -61 (clamped 25) | -62 (clamped 60) |
| Dawn (daynight=6000) | 150 | 70 | 89 | 88 |
| Noon | 300 | 220 (clamped 100) | 239 (clamped 100) | 238 (clamped 100) |
| Dusk (daynight=18000) | 150 | 70 | 89 | 88 |

With torch (`light_timer > 0`): r gets +200, so midnight r = 120 (bright red/warm tone).

## fade_page — RGB Component Fading

**Location**: `fmain2.c:377-419`

```c
fade_page(r,g,b,limit,colors) short r,g,b,limit; USHORT *colors;
```

### Color 31 Override — fmain2.c:381-386
Before fading, color 31 (background/sky) is set per region:
| Region | Color 31 | Meaning |
|---|---|---|
| 4 (desert) | `0x0980` | Orange-brown desert sky |
| 9 (dungeons), `secret_timer` active | `0x00f0` | Bright green (secret revealed) |
| 9 (dungeons), normal | `0x0445` | Dark grey-blue dungeon |
| All others | `0x0bdf` | Light blue sky |

### Clamping — fmain2.c:389-400
With `limit=TRUE` (normal gameplay):
- **Red**: min 10, max 100
- **Green**: min 25, max 100
- **Blue**: min 60, max 100
- Also computes `g2 = (100-g)/3` — a blue shift factor used later

With `limit=FALSE` (fade_down/fade_normal transitions):
- All channels clamped to 0–100, no blue shift

### Per-Color Fading — fmain2.c:402-416
For each of 32 colors in the palette:
```c
r1 = (colors[i] & 0x0f00)>>4;   // extract red nibble, shift to position
g1 = colors[i] & 0x00f0;         // extract green nibble in position
b1 = colors[i] & 0x000f;         // extract blue nibble
if (light_timer && (r1 < g1)) r1 = g1;  // torch: boost red to at least green
r1 = (r * r1)/1600;              // scale red
g1 = (g * g1)/1600;              // scale green
b1 = (b * b1 + (g2*g1))/100;    // scale blue with green-to-blue shift
```

**Torch effect** (line 407): When `light_timer` active, if a color's red component is less than green, red is boosted to match green — creating a warm/amber torch glow.

**Blue shift** (`g2`): At night when green is low, `g2 = (100-g)/3` adds a portion of the green channel into blue, creating a blue nighttime tint. At g=25 (night minimum), g2=25; at g=100 (full day), g2=0.

**Nighttime blue boost** — fmain2.c:412-413:
```c
if (limit)
{   if (i>= 16 && i<= 24 && g > 20)
    {   if (g < 50) b1+=2; else if (g < 75) b1++; }
    if (b1 > 15) b1 = 15;
}
```
Colors 16–24 (vegetation/nature palette range) get extra blue at twilight:
- g 21–49: +2 blue
- g 50–74: +1 blue
- g ≥ 75 or g ≤ 20: no boost

Final result written to `fader[]` and loaded via `LoadRGB4(&vp_page,fader,32)` — fmain2.c:418.

## fade_down / fade_normal — Quick Transitions

**Location**: `fmain2.c:623-630`

```c
fade_down()
{   register long i;
    for (i=100; i>=0; i-=5) { fade_page(i,i,i,FALSE,pagecolors); Delay(1); } }

fade_normal()
{   register long i;
    for (i=0; i<=100; i+=5) { fade_page(i,i,i,FALSE,pagecolors); Delay(1); } }
```

- **fade_down**: Reduces all channels from 100 to 0 in steps of 5 (21 steps) with 1-tick delay each. Fades screen to black.
- **fade_normal**: Raises all channels from 0 to 100 in steps of 5 (21 steps). Restores to full brightness.
- Both use `limit=FALSE` — clamped 0–100 with no night limits or blue shift.
- Used for screen transitions: map messages (`map_message()` at fmain2.c:601), door transitions, etc.

## setmood — Music Selection

**Location**: `fmain.c:2936-2957`

```c
setmood(now) char now;
{   register long off;
    if (anim_list[0].vitality == 0) off = (6*4);           // Death
    else if (hero_x > 0x2400 && hero_x < 0x3100 &&
            hero_y > 0x8200 && hero_y < 0x8a00)
    { off = (4*4); }                                         // Astral plane
    else if (battleflag) off = 4;                            // Battle
    else if (region_num > 7)
    {   off = (5*4);                                         // Indoor/dungeon
        if (region_num == 9) new_wave[10] = 0x0307;
        else new_wave[10] = 0x0100;
    }
    else if (lightlevel > 120) off = 0;                      // Day (outdoor)
    else off = 8;                                            // Night (outdoor)
```

### Track Offset Mapping
Each mood uses 4 consecutive track pointers (4 channels). `track[]` has 28 entries (7 songs × 4 channels), loaded from `songs` file — fmain2.c:760-778.

| off | Indices | Condition | Music |
|---|---|---|---|
| 0 | track[0–3] | `lightlevel > 120` (outdoor day) | Day theme |
| 4 | track[4–7] | `battleflag` | Battle theme |
| 8 | track[8–11] | `lightlevel <= 120` (outdoor night) | Night theme |
| 12 | track[12–15] | (intro/title) | Title theme — fmain.c:1182 |
| 16 | track[16–19] | Astral plane coordinates | Astral theme |
| 20 | track[20–23] | `region_num > 7` (indoor) | Indoor theme |
| 24 | track[24–27] | `vitality == 0` (death) | Death theme |

### Priority Order
1. **Dead** (vitality == 0) — overrides everything
2. **Astral plane** — coordinate check: hero_x 0x2400–0x3100, hero_y 0x8200–0x8a00
3. **Battle** — `battleflag` set
4. **Indoor/dungeon** — `region_num > 7`
5. **Day outdoor** — `lightlevel > 120`
6. **Night outdoor** — `lightlevel <= 120`

### Day/Night Music Threshold
`lightlevel > 120` = day music, `lightlevel <= 120` = night music. Since lightlevel peaks at 300, the crossover happens at daynight ≈ 4800 (dawn, period 2) and daynight ≈ 19200 (dusk, period 9).

### Playback Control — fmain.c:2951-2956
```c
if (menus[GAME].enabled[6] & 1)   // music enabled in game menu
{   if (now)
        playscore(track[off],track[off+1],track[off+2],track[off+3]);
    else setscore(track[off],track[off+1],track[off+2],track[off+3]);
}
else stopscore();
```
- `now=1` (parameter): `playscore()` — immediate play (used after battle/transitions)
- `now=0`: `setscore()` — queue for crossfade at natural boundary
- Music disabled: `stopscore()`

### Periodic Mood Check — fmain.c:2198
```c
if ((daynight & 7) == 0 && !battleflag) setmood(0);
```
Music mood re-evaluated every 8 ticks (when not in battle).

### Indoor Waveform Tweak — fmain.c:2945-2946
```c
if (region_num == 9) new_wave[10] = 0x0307;
else new_wave[10] = 0x0100;
```
Modifies sound waveform parameter: dungeons (region 9) use `0x0307`, buildings (region 8) use `0x0100`.

## Gameplay Effects

### Spectre Visibility — fmain.c:2027-2028
```c
if (lightlevel < 40) ob_listg[5].ob_stat = 3;
else ob_listg[5].ob_stat = 2;
```
- `lightlevel < 40` (deep night, daynight < 1600 or > 22400): Spectre (`ob_listg[5]`) becomes stat=3 (visible/interactive NPC).
- Otherwise: stat=2 (hidden).

### Encounter Spawning — fmain.c:2058-2091
```c
if ((daynight & 15)==0 && encounter_number && !actors_loading)
```
Encounter placement checked every 16 ticks. Rate is constant regardless of day/night, but:

```c
if ((daynight & 31) == 0 && !actors_on_screen &&
    !actors_loading && !active_carrier && xtype < 50)
```
New random encounters generated every 32 ticks. The `danger_level` calculation (`fmain.c:2082-2083`) does not directly use `lightlevel` or `daynight` — it depends on `region_num` and `xtype` (extent danger value).

### Sleep Mechanics — fmain.c:2014-2021
```c
if (anim_list[0].state == SLEEP)
{   daynight += 63;
    if (fatigue) fatigue--;
    if (fatigue == 0 ||
        (fatigue < 30 && daynight > 9000 && daynight < 10000) ||
        (battleflag && (rand64() == 0)) )
    {   anim_list[0].state = STILL;
```
- Sleep advances time 64× faster (63 extra + 1 normal = 64 per tick).
- Wake conditions: fatigue reaches 0, OR fatigue < 30 and daynight enters 9000–10000 (morning period 4), OR battle interruption.
- Sleeping at an inn wakes the character at morning (daynight 9000–10000).

### Hunger/Fatigue Advancement — fmain.c:2199-2201
```c
if ((daynight & 127) == 0 && anim_list[0].vitality &&
        anim_list[0].state != SLEEP)
{   hunger++;
    fatigue++;
```
Hunger and fatigue increment every 128 ticks. Not directly day/night dependent but tied to the same tick counter.

### Auto-Eat — fmain.c:2195-2196
```c
if (hunger > 30 && stuff[24])
{   stuff[24]--; hunger -= 30; event(37);  }
```
Safe zone check (every 128 ticks at `daynight & 127 == 0`) — auto-eats food if available.

### Vitality Recovery — fmain.c:2041-2045
```c
if ((daynight & 0x3ff) == 0)
{   if (anim_list[0].vitality < (15+brave/4) && anim_list[0].state != DEAD)
    {   anim_list[0].vitality++;
        prq(4);
    }
}
```
Every 1024 ticks, vitality regenerates by 1 (if below max). Not directly day/night dependent.

### Freeze Timer (Time Stop) — fmain.c:2023
```c
if (!freeze_timer) /* no time in timestop */
    if ((daynight++) >= 24000) daynight = 0;
```
When `freeze_timer > 0`, daynight does NOT advance. Additionally, all encounter processing and AI are skipped (`goto stasis` at fmain.c:2048).

## Palette Data

### pagecolors — Static Base Palette — fmain2.c:367-372
```c
USHORT pagecolors [] = {
    0x0000,0x0FFF,0x0E96,0x0B63,0x0631,0x07BF,0x0333,0x0DB8,
    0x0223,0x0445,0x0889,0x0BBC,0x0521,0x0941,0x0F82,0x0FC7,
    0x040,0x0070,0x00B0,0x06F6,0x0005,0x0009,0x000D,0x037F,
    0x0C00,0x0F50,0x0FA0,0x0FF6,0x0EB6,0x0EA5,0x000F,0x0BDF };
```

This is a **hardcoded 32-color palette** in Amiga 12-bit RGB format (0x0RGB). It is:
- NOT loaded from disk image files.
- The **same palette for ALL outdoor regions** (0–7).
- Faded dynamically by `fade_page()` each tick.

Color groups:
- 0–7: Neutrals and earth tones (black, white, tan, brown, dark green, cyan, grey, gold)
- 8–15: Grey scale and reds (dark blue, dark grey, grey, light grey, browns, reds, bright red, pink)
- 16–23: Greens and blues (dark green, green, bright green, lime, navy, blue, medium blue, cyan)
- 24–31: Warm tones and sky (red, orange, yellow-orange, pale yellow, peach, salmon, pure blue, sky blue)

### fader — Working Palette — fmain2.c:365
```c
USHORT fader[32];
```
Scratch buffer written by `fade_page()` and loaded into hardware via `LoadRGB4()`.

### blackcolors — All-Black Palette — fmain.c:481-482
```c
USHORT blackcolors [] =
{   0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0 };
```
Used for instant blackout during transitions (save/load, win sequence).

### textcolors — Status Bar Palette — fmain.c:476-479
```c
USHORT textcolors [] =
{   0x000, 0xFFF, 0xC00, 0xF60, 0x00f, 0xc0f, 0x090, 0xFF0,
    0xf90, 0xf0c, 0xA50, 0xFDB, 0xEB7, 0xCCC, 0x888, 0x444,
    0x000, 0xDB0, 0x740, 0xC70 };
```
20 colors for the hi-res status bar viewport. NOT affected by day/night fading.

### sun_colors — Win Sequence Palette — fmain2.c:1569-1577
53-entry gradient from black through blues/indigos to warm sunset/gold. Used only in `win_colors()` for the victory sunrise animation.

### introcolors — Title Screen Palette — fmain.c:484-488
32-color palette for the title/intro screen. Separate from gameplay palette.

## Indoor vs Outdoor Lighting

### Outdoor (region_num < 8) — fmain2.c:1657-1658
```c
if (region_num < 8) /* no night inside buildings */
    fade_page(lightlevel-80+ll,lightlevel-61,lightlevel-62,TRUE,pagecolors);
```
Full day/night cycle palette fading applied.

### Indoor (region_num >= 8) — fmain2.c:1659
```c
else fade_page(100,100,100,TRUE,pagecolors);
```
Always full brightness (100,100,100). No day/night variation indoors.

### Indoor Color 31 Differences — fmain2.c:381-386
- **Region 9 (dungeons)**: Color 31 = `0x0445` (dark grey-blue), or `0x00f0` (bright green) if `secret_timer` active.
- **Region 4 (desert)**: Color 31 = `0x0980` (orange-brown).
- **All others**: Color 31 = `0x0bdf` (light blue sky).

Note: The color 31 override in `fade_page()` applies to the base `pagecolors` array element (mutating it in place), so the change persists across subsequent ticks.

## Astral Plane Palette

The astral plane uses the **same `pagecolors` array** as all other regions. It is not region 9 (the astral plane is identified by coordinate ranges, not region_num). The astral plane's visual difference comes from:

1. Different tile artwork loaded from disk (image data "astral" at block 840 — rtrack.c:39).
2. The `setmood()` function selects astral music based on hero coordinates:
   ```c
   else if (hero_x > 0x2400 && hero_x < 0x3100 &&
           hero_y > 0x8200 && hero_y < 0x8a00)
   { off = (4*4); }
   ```
3. Normal day/night fading still applies to astral plane tiles (since region_num is set to 9 when entering dungeons, and the astral plane extent at fmain.c:353 has encounter data suggesting it may be entered from a dungeon region).

## colorplay — Teleport Effect — fmain2.c:425-431
```c
colorplay() /* teleport effect */
{   register long i,j;
    for (j=0; j<32; j++)
    {   for (i=1; i<32; i++) fader[i]=bitrand(0xfff);
        LoadRGB4(&vp_page,fader,32);
        Delay(1);
    }
}
```
32 frames of random 12-bit colors for all 32 palette entries (except color 0). Used during teleportation — `fmain.c:3336`.

## light_timer (Torch/Jewel)

**Declaration**: `fmain.c:577` — `short light_timer;`

### Activation — fmain.c:3306
```c
case 6: light_timer += 760; break;
```
Using the Green Jewel (stuff[10], item menu option 6) adds 760 to `light_timer`.

### Countdown — fmain.c:1380
```c
if (light_timer) light_timer--;
```
Decrements by 1 each VBlank interrupt (faster than main game tick).

### Effects
1. **In `day_fade()`** (fmain2.c:1655): `ll = 200` added to red parameter → boosts red channel by 200, making nighttime appear warm/amber.
2. **In `fade_page()`** (fmain2.c:407): `if (light_timer && (r1 < g1)) r1 = g1;` — each color's red component boosted to at least match green, creating warm torch-like lighting.

### Reset — fmain.c:2852
```c
secret_timer = light_timer = freeze_timer = 0;
```
All timers reset on brother death/succession.

## References Found

- fmain.c:572 — declaration — `USHORT daynight, lightlevel;`
- fmain.c:577 — declaration — `short secret_timer, light_timer, freeze_timer;`
- fmain.c:583 — declaration — `char viewstatus; /* 0 = normal, 1 = big, 99 = corrupt */`
- fmain.c:598 — declaration — `short dayperiod;`
- fmain.c:1013 — declaration — `unsigned char *(track[32]);`
- fmain.c:1336 — write — cheat key advances daynight by 1000
- fmain.c:1380 — write — `if (light_timer) light_timer--;` in VBlank
- fmain.c:1382 — write — `if (freeze_timer) freeze_timer--;` in VBlank
- fmain.c:2014 — write — `daynight += 63` during sleep
- fmain.c:2017 — read — sleep wake condition checks daynight 9000–10000
- fmain.c:2023-2024 — write — main daynight increment and wrap
- fmain.c:2025-2026 — write — `lightlevel = daynight/40`, mirror at 300
- fmain.c:2027-2028 — write — spectre visibility based on lightlevel < 40
- fmain.c:2029-2036 — read — dayperiod calculation and time-of-day events
- fmain.c:2041 — read — vitality regen every 1024 ticks
- fmain.c:2048 — read — freeze_timer skips to stasis
- fmain.c:2058 — read — encounter placement every 16 ticks
- fmain.c:2080 — read — random encounter generation every 32 ticks
- fmain.c:2115 — read — CARRIER AI check every 16 ticks
- fmain.c:2188 — read — safe zone check every 128 ticks
- fmain.c:2198 — read — setmood called every 8 ticks
- fmain.c:2199 — read — hunger/fatigue increment every 128 ticks
- fmain.c:2905 — write — `daynight = 8000; lightlevel = 300;` on rescue
- fmain.c:2936-2957 — call — `setmood()` function
- fmain.c:2948 — read — lightlevel > 120 day/night music threshold
- fmain.c:3306 — write — `light_timer += 760` (Green Jewel)
- fmain.c:3407 — read — dayperiod > 7 innkeeper dialogue
- fmain2.c:365 — declaration — `USHORT fader[32];`
- fmain2.c:367-372 — declaration — `pagecolors[]` static palette
- fmain2.c:377-419 — call — `fade_page()` function
- fmain2.c:381-386 — write — color 31 per-region override
- fmain2.c:407 — read — torch red boost in fade_page
- fmain2.c:425-431 — call — `colorplay()` teleport effect
- fmain2.c:601-611 — call — `map_message()` loads pagecolors raw
- fmain2.c:623-630 — call — `fade_down()` / `fade_normal()`
- fmain2.c:760-778 — call — `read_score()` loads songs into track[]
- fmain2.c:1569-1577 — declaration — `sun_colors[]` win sequence palette
- fmain2.c:1653-1660 — call — `day_fade()` function
- fmain.c:481-482 — declaration — `blackcolors[]` all-zero palette
- fmain.c:476-479 — declaration — `textcolors[]` status bar palette
- fmain.c:484-488 — declaration — `introcolors[]` intro screen palette
- fmain.c:669-675 — declaration — `new_wave[]` music waveform config
- narr.asm:45-48 — data — event messages 28–31 (time of day)

## Unresolved

- **Astral plane region_num**: The astral plane extent (fmain.c:353) is identified by coordinates, and `setmood()` uses coordinate checks (not region_num). When a player enters via a door (`d->secs != 1`), `new_region = 9`. The astral plane art is at disk block 840 (rtrack.c:39), loaded as part of region 9 data. Whether the astral plane uses the same palette fading as dungeons (always full brightness) depends on whether region_num stays 9 during astral gameplay — this would mean NO day/night cycle on the astral plane. Need to trace the full door/extent entry path to confirm.
- ~~**Which `colorplay` is linked**~~: **RESOLVED** — `fsupp.asm` is not assembled or linked by the makefile. Only the C version in `fmain2.c:425-432` is compiled into the game.
- **pagecolors[31] mutation persistence**: `fade_page()` writes directly to `pagecolors[31]`, mutating the base palette array. This means the sky color change for region 4 or 9 persists even after leaving those regions, until `fade_page` is called again and re-assigns color 31. The override logic re-runs each call, so this is self-correcting, but the mutation pattern is notable.
- **Song file structure**: The `songs` file contains up to 28 packed score entries (7 songs × 4 channels), but the exact mapping of which song index = which mood cannot be confirmed without decoding the binary file. The offset table (0, 4, 8, 12, 16, 20, 24) is inferred from `setmood()` offsets but the actual music identity requires audio analysis.

## Refinement Log
- 2026-04-06: Initial comprehensive discovery pass. All 10 questions addressed with source citations.
