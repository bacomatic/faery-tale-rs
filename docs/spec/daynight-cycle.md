## 17. Day/Night Cycle

### 17.1 Day Counter

`daynight` is a 16-bit unsigned integer (`USHORT`), cycling from 0 to 23999. Incremented by 1 each non-scrolling world-clock tick:

```
if (!freeze_timer)
    if ((daynight++) >= 24000) daynight = 0;
```

Does not advance during freeze spells. During sleep: `daynight += 63` per world-clock tick (plus normal +1 = 64 effective advance). Initialized to 8000 (morning) during `revive()`.

Full cycle = 24000 world-clock ticks. This counter is distinct from both the 30 fps presentation cadence and the 15 Hz animation/AI tick.

### 17.2 Light Level

Triangle wave derived from `daynight`:

```
lightlevel = daynight / 40
if (lightlevel >= 300) lightlevel = 600 − lightlevel
```

| daynight | lightlevel | Phase |
|----------|------------|-------|
| 0 | 0 | Midnight (darkest) |
| 6000 | 150 | Dawn |
| 12000 | 300 | Noon (brightest) |
| 18000 | 150 | Dusk |
| 23999 | 1 | Just before midnight |

### 17.3 Time-of-Day Events

`dayperiod = daynight / 2000` (values 0–11). Transitions trigger text events:

| Period | daynight Range | Event | Message |
|--------|---------------|-------|---------|
| 0 | 0–1999 | event(28) | "It was midnight." |
| 4 | 8000–9999 | event(29) | "It was morning." |
| 6 | 12000–13999 | event(30) | "It was midday." |
| 9 | 18000–19999 | event(31) | "Evening was drawing near." |

Periods 1–3, 5, 7–8, 10–11 are silent transitions.

### 17.4 Spectre Night Visibility

When `lightlevel < 40` (deep night, daynight < 1600 or > 22400): `ob_listg[5].ob_stat = 3` (visible and interactive). Otherwise: `ob_listg[5].ob_stat = 2` (hidden).

### 17.5 Palette Fading (`day_fade`)

Called every world-clock tick. Updates palette every 4 world-clock ticks (`(daynight & 3) == 0`) or during screen rebuild (`viewstatus > 97`):

```
day_fade():
    ll = 200 if light_timer > 0 else 0
    if ((daynight & 3) == 0 || viewstatus > 97):
        if region_num < 8:
            fade_page(lightlevel − 80 + ll, lightlevel − 61, lightlevel − 62, TRUE, pagecolors)
        else:
            fade_page(100, 100, 100, TRUE, pagecolors)
```

- **Green Jewel light bonus**: `light_timer > 0` adds 200 to the red parameter, producing a warm amber glow even at night.
- **Indoor override**: `region_num >= 8` → always full brightness `(100, 100, 100)`.

### 17.6 RGB Component Fading (`fade_page`)

Per-component palette scaler applied to the 32-color palette.

**Color 31 override** (sky color):

| Region | Color 31 | Meaning |
|--------|----------|---------|
| 4 (desert) | `0x0980` | Orange-brown desert sky |
| 9 (dungeon), `secret_timer` active | `0x00f0` | Bright green (secret revealed) |
| 9 (dungeon), normal | `0x0445` | Dark grey-blue |
| All others | `0x0bdf` | Light blue sky |

**Clamping** (with `limit = TRUE`):

- Red: min 10, max 100
- Green: min 25, max 100
- Blue: min 60, max 100
- Blue shift factor: `g2 = (100 − g) / 3`

**Per-color computation** (for each of 32 palette entries):

1. Extract 12-bit RGB components from `pagecolors[]`
2. Green Jewel effect: if `light_timer` active and red < green, boost red to match green
3. Scale: `r1 = (r × r1) / 1600`, `g1 = (g × g1) / 1600`, `b1 = (b × b1 + g2 × g1) / 100`
4. Nighttime vegetation boost (colors 16–24): green 21–49 → +2 blue; green 50–74 → +1 blue

Result written to `fader[]` and loaded to hardware palette.

**Outdoor RGB at key times:**

| Phase | lightlevel | r (no jewel) | g | b |
|-------|------------|-------------|---|---|
| Midnight | 0 | clamped 10 | clamped 25 | clamped 60 |
| Dawn | 150 | 70 | 89 | 88 |
| Noon | 300 | clamped 100 | clamped 100 | clamped 100 |

With Green Jewel active: red parameter gets +200, so midnight red ≈ 120 (warm amber even in darkness).

### 17.7 Screen Transitions (`fade_down` / `fade_normal`)

- **`fade_down()`**: Steps all channels from 100 to 0 in increments of 5 (21 steps, `Delay(1)` each). Fades screen to black.
- **`fade_normal()`**: Steps from 0 to 100 in increments of 5. Fades back to full brightness.

Both use `limit = FALSE` — no night clamping or blue shift. Used for map messages, door transitions, and other screen changes.

### 17.8 Music Mood Selection (`setmood`)

Selects one of 7 four-channel music tracks based on game state. Priority (highest first):

| Track Offset | Indices | Condition | Music |
|-------------|---------|-----------|-------|
| 24 | 24–27 | `vitality == 0` (death) | Death theme |
| 16 | 16–19 | Astral plane coordinates | Astral theme |
| 4 | 4–7 | `battleflag` active | Battle theme |
| 20 | 20–23 | `region_num > 7` (indoor) | Indoor theme |
| 0 | 0–3 | `lightlevel > 120` (outdoor day) | Day theme |
| 8 | 8–11 | `lightlevel ≤ 120` (outdoor night) | Night theme |

Day/night music crossover: `lightlevel > 120` = day, `≤ 120` = night. Crossover at daynight ≈ 4800 (dawn) and ≈ 19200 (dusk).

Playback: `now = TRUE` → `playscore()` (immediate restart); `now = FALSE` → `setscore()` (crossfade). Mood re-evaluated every 8 world-clock ticks. Indoor waveform tweak: dungeons (region 9) use `new_wave[10] = 0x0307`; buildings (region 8) use `0x0100`.

### 17.9 Gameplay Effects

- **Encounter spawning**: Rate is constant regardless of time of day. `danger_level` depends on `region_num` and `xtype`, not `lightlevel`.
- **Innkeeper dialogue**: `dayperiod > 7` (evening/night) triggers lodging speech.
- **Vitality recovery**: Every 1024 world-clock ticks (`(daynight & 0x3FF) == 0`), hero regenerates +1 HP up to max. Tied to `daynight` counter but not time-of-day dependent.
- **Sleep**: Time passes 64× faster. Wake conditions include morning window (daynight 9000–10000).

### 17.10 Palette Data

- **`pagecolors[32]`**: Hardcoded 32-color base palette in 12-bit Amiga RGB. Same for all outdoor regions (0–7). Faded dynamically by `fade_page()`.
- **`textcolors[20]`**: Status bar palette (hi-res viewport). NOT affected by day/night fading.
- **`blackcolors[32]`**: All-zero palette for instant blackout transitions.
- **`sun_colors[53]`**: Sunrise/sunset gradient for the victory sequence `win_colors()`.
- **`introcolors[32]`**: Title/intro screen palette, separate from gameplay.
- **`colorplay()`**: Teleportation effect — 32 frames of random 12-bit RGB colors for all palette entries except color 0.

---
