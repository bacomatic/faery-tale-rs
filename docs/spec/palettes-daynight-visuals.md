## 7. Color Palettes & Day/Night Fading

### 7.1 Palette Definitions

Five palettes are used:

| Palette | Size | Purpose |
|---------|------|---------|
| `pagecolors[32]` | 32 × 12-bit Amiga RGB | Game world palette; same for all outdoor regions (0–7); faded dynamically by `fade_page()` |
| `textcolors[20]` | 20 × 12-bit Amiga RGB | Status bar palette; NOT affected by day/night fading |
| `introcolors[32]` | 32 × 12-bit Amiga RGB | Title/intro screen palette, separate from gameplay; used during `screen_size()` zoom animation |
| `blackcolors[32]` | 32 × all zeros | Instant blackout transitions |
| `sun_colors[53]` | 53 × 12-bit Amiga RGB | Sunrise/sunset gradient for the victory sequence |

12-bit to 24-bit conversion: multiply each 4-bit channel by 17 (0x11).

### 7.2 Day/Night Fading — `day_fade()`

Called every non-scrolling tick from Phase 14d:

```
day_fade():
    if light_timer > 0: ll = 200; else: ll = 0
    if (daynight & 3) == 0 OR viewstatus > 97:
        if region_num < 8:
            fade_page(lightlevel - 80 + ll, lightlevel - 61, lightlevel - 62, TRUE, pagecolors)
        else:
            fade_page(100, 100, 100, TRUE, pagecolors)
```

Key behaviors:
- **Green Jewel light bonus**: `light_timer > 0` adds 200 to the red parameter (warm amber glow)
- **Update rate**: Every 4 ticks (`daynight & 3 == 0`) or during screen rebuild (`viewstatus > 97`)
- **Indoor override**: `region_num >= 8` → always full brightness (100, 100, 100) with no day/night variation

Outdoor RGB at key times of day:

| Phase | lightlevel | Red (no jewel) | Green | Blue |
|-------|------------|----------------|-------|------|
| Midnight (daynight 0) | 0 | clamped 10 | clamped 25 | clamped 60 |
| Dawn (daynight 6000) | 150 | 70 | 89 | 88 |
| Noon (daynight 12000) | 300 | clamped 100 | clamped 100 | clamped 100 |

With Green Jewel active: midnight red = 120 (warm amber tone even in darkness).

### 7.3 Palette Scaling — `fade_page(r, g, b, limit, colors)`

Per-component palette scaler:

**Step 1 — Color 31 override**:

| Region | Color 31 | Meaning |
|--------|----------|---------|
| 4 (desert) | `0x0980` | Orange-brown desert sky |
| 9 (dungeon), `secret_timer` active | `0x00F0` | Bright green (secret revealed) |
| 9 (dungeon), normal | `0x0445` | Dark grey-blue |
| All others | `0x0BDF` | Light blue sky |

**Step 2 — Clamping** (when `limit = TRUE`):
- Red: min 10, max 100
- Green: min 25, max 100
- Blue: min 60, max 100
- Blue shift factor: `g2 = (100 - green_pct) / 3`

**Step 3 — Per-color computation**: For each of 32 palette entries, extract 12-bit RGB components from `colors[]`, then:

1. **Green Jewel light boost**: if `light_timer` active and color's red < green, boost red to match green
2. **Scale channels**:
   ```
   r_out = (r_pct × r_raw) / 1600
   g_out = (g_pct × g_raw) / 1600
   b_out = (b_pct × b_raw + g2 × g_out) / 100
   ```
3. **Twilight vegetation boost** (colors 16–24 only):
   - green% 21–49: add 2 to blue channel
   - green% 50–74: add 1 to blue channel
4. Cap all channels at maximum (15 for 12-bit, 255 for 24-bit)

Result written to `fader[]` and loaded as the active palette.

### 7.4 Screen Transitions — `fade_down()` / `fade_normal()`

- **`fade_down()`**: Steps all channels from 100 to 0 in decrements of 5 (21 steps, `Delay(1)` each). Fades screen to black. Uses `limit = FALSE` — no night clamping or blue shift.
- **`fade_normal()`**: Steps all channels from 0 to 100 in increments of 5. Fades back to full brightness. Also uses `limit = FALSE`.

Used for map messages, door transitions, story placards, and other screen changes.

### 7.5 Teleportation Palette Effect

`colorplay()`: Rapidly sets palette entries 1–31 to random 12-bit values for 32 frames (~0.5 seconds). Entry 0 (background color) is preserved. Creates a psychedelic flash during teleportation.

---


