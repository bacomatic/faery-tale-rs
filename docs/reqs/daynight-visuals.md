## 6. Day/Night Visuals

### Requirements

| ID | Requirement |
|----|-------------|
| R-FADE-001 | Outdoor palette colors shall be dynamically scaled based on the `lightlevel` triangular wave (0–300, peaking at noon, bottoming at midnight). `lightlevel = daynight / 40`, mirrored at 300: `if lightlevel >= 300 then lightlevel = 600 - lightlevel`. |
| R-FADE-002 | Night palette shall enforce minimum brightness floors when `limit` is true: red ≥ 10%, green ≥ 25%, blue ≥ 60%, with all channels capped at maximum 100%. This produces a blue-tinted night effect. |
| R-FADE-003 | Indoor locations (region ≥ 8) shall always use full brightness (100, 100, 100) with no day/night variation. |
| R-FADE-004 | The Green Jewel light spell (`light_timer > 0`) shall add 200 to the red parameter of the fade calculation. Additionally, for each palette entry where red < green, red shall be boosted to match green, producing a warm amber illumination. |
| R-FADE-005 | Color 31 of the game palette shall be overridden per-region: region 4 (desert) = 0x0980 (orange-brown), region 9 (dungeon) with `secret_timer` active = 0x00F0 (bright green), region 9 (dungeon) normal = 0x0445 (dark grey-blue), all others = 0x0BDF (light blue sky). |
| R-FADE-006 | Twilight vegetation boost: colors 16–24 shall gain extra blue at dusk/dawn — when green% is 21–49: +2 blue per entry; when green% is 50–74: +1 blue per entry. |
| R-FADE-007 | Palette updates shall occur every 4 ticks (`daynight & 3 == 0`) or immediately during screen rebuild (`viewstatus > 97`). |
| R-FADE-008 | The status bar palette (`textcolors[20]`) shall NOT be affected by day/night fading. |
| R-FADE-009 | A blue night-shift factor `g2 = (100 - green%) / 3` shall be applied to the blue channel calculation, creating additional blue tinting as green brightness decreases. |
| R-FADE-010 | Outdoor RGB parameters derived from `lightlevel`: red = `lightlevel − 80` (+ 200 if light spell), green = `lightlevel − 61`, blue = `lightlevel − 62`. |
| R-FADE-011 | Per-color palette scaling in `fade_page(r, g, b, limit, colors)` shall apply: (1) Green Jewel boost — if `light_timer > 0` and a palette entry's red < green, raise red to match green; (2) per-channel scale `r1 = (r × r1) / 1600`, `g1 = (g × g1) / 1600`, `b1 = (b × b1 + g2 × g1) / 100`, where `g2 = (100 − g) / 3`; (3) twilight vegetation boost for colors 16–24 — green% 21–49 → +2 blue, green% 50–74 → +1 blue; (4) clamping when `limit == TRUE` — red ∈ [10,100], green ∈ [25,100], blue ∈ [60,100]. Color 31 is overridden before scaling per-region (see R-FADE-005). Results are written to `fader[]` and loaded to the hardware palette. |

### User Stories

- As a player, I see a gradual transition from day to night with blue-tinted darkness.
- As a player, entering a building restores full brightness regardless of the time of day.
- As a player, I see a warm amber glow when the Green Jewel light spell is active.
- As a player, I see vegetation colors shift toward blue during twilight hours.

---


