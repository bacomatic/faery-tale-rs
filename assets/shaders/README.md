# Reference shaders — palette effects on prebaked RGBA

**The GLSL in this directory is reference material only. It is validated by the porting effort,
not by this pipeline.** The ground truth for every effect is `fade_page()` in
`src/fmain2.c:377-419` (the one function that produces the whole day/night cycle and every fade
by rewriting the 32-entry hardware palette), as ported verbatim in
`experiment/shaders/fade_page.py`. The renders in [`previews/`](previews/README.md) come from
that port; the review app runs these shaders in WebGL2 and diffs them against those renders, but
that is an aid, not the acceptance of the GLSL.

The original game changes palette entries; a port that ships RGBA textures has to do the same
per pixel. That works because every RGBA sample is a 4-bit Amiga nibble `n` stored as `n*17`
(`asset_common.rgb4_to_rgba8`), so the nibble is recovered exactly as `round(c*15)` and the
original integer math can run unchanged. See `experiment/shaders/FINDINGS.md` for the proof.

## Shaders

All files are GLSL ES 3.00 fragment shaders (`#version 300 es`, runnable verbatim in WebGL2 and
trivially portable to desktop GLSL / HLSL / WGSL). They expect a full-screen quad with `vUV`
= (0,0) at the top-left texel, textures sampled with NEAREST filtering and no premultiplied alpha.
Alpha is passed through unchanged (sprite index 31 is alpha 0 in the shipped sheets).
[`shaders.json`](shaders.json) lists textures and uniforms machine-readably.

| File | Effect | Game source | Textures | Uniforms |
|---|---|---|---|---|
| `daynight_live.glsl` | **Complete** outdoor day/night (`limit = TRUE` path): brightness scale, night floors, moonlight blue, vegetation boost, Green Jewel | `fmain2.c:377-419`, `day_fade` `1653-1660` | `uBase`, `uHighlight` | `int uLightlevel` 0..300, `bool uLightTimer` |
| `daynight_bank.glsl` | Same effect from a prebaked per-level texture array (the `previews/*_light*.png` set), optional lerp between layers | `fmain2.c:377-419` | `sampler2DArray uBank` | `float uLevel01`, `int uLayerCount` |
| `fade_to_black.glsl` | `fade_down`/`fade_normal` (21 steps of 5) and the intro zoom's per-channel fade (`limit = FALSE` path: no floors, no moonlight, no boost) | `fmain2.c:623-629`, `fmain.c:2914-2930` | `uBase` | `ivec3 uWeight` 0..100, `bool uLightTimer` |

How the game drives `uWeight`: `fade_down` steps `(i, i, i)` for `i = 100, 95, …, 0` one tick
apart, `fade_normal` the reverse (`fmain2.c:623-629`); the intro zoom calls `screen_size(x)` for
`x = 0, 4, …, 160` (and back `156 … 0`), with `y = x*5/8` and weights `(2y-40, 2y-70, 2y-100)`
(`fmain.c:1199, 1209, 2917, 2930`) — red leads and blue lags, so the opening zoom warms up from
black. `shaders.json` lists these as `modes` of the shader; the review viewer exposes them.

### Driving values

- `lightlevel = daynight/40`, mirrored to a 0..300 triangle (`fmain.c:2025-2026`); `daynight`
  counts 0..24000 once per tick, frozen while the Gold Ring's `freeze_timer` runs (`fmain.c:2023-2024`).
  `day_fade` is applied every 4 ticks or during a full redraw (`fmain2.c:1656`).
- Outdoor weights: `r = lightlevel-80 (+200 with the jewel)`, `g = lightlevel-61`, `b = lightlevel-62`,
  each clamped to 100 and floored to the night limits 10/25/60 (`fmain2.c:388-395`). Everything
  below lightlevel 87 is therefore the same deep-night palette; lightlevel ≥ 180 is the original
  palette. The vegetation boost is +2 blue for `g` in 21..49 (lightlevel ≤ 110), +1 for 50..74
  (111..135), none from 136.
- Indoors (regions 8 and 9) `day_fade` passes `(100,100,100)` — no night inside
  (`fmain2.c:1657-1659`); set `uLightlevel` ≥ 180.
- Green Jewel: `light_timer += 760` (`fmain.c:3306`), decremented each tick (`fmain.c:1380`).
- Colour 31 is per region — `0x0980` in region 4, `0x0445`/`0x00f0` (Crystal Orb `secret_timer`)
  in region 9, else `0x0bdf` (`fmain2.c:381-386`). The shipped tile atlases already carry it; it
  fades like every other colour, so the shaders need no special case.
- Every fade is quantised: each channel is an integer `0..15` after truncating division, so a fade
  moves in visible 4-bit steps. Do not replace the integer math with a float multiply if the goal
  is to look like the original.

## Effects matrix — indexed path vs RGBA + shader

| Effect | Needs | On indexed atlas + palette LUT | On RGBA |
|---|---|---|---|
| Brightness scale, night floors | light level | recompute the 32 colours | per pixel from recovered nibbles — exact |
| Moonlight blue term | light level | same | per pixel — exact (needs each pixel's own green nibble, which RGBA has) |
| Vegetation night boost (indices 16–24) | **palette index** | same | per pixel **only with the 1-bit `highlight_mask`** shipped next to every sheet/atlas (`*_highlightmask.png`, `atlas_highlightmask.png`: set where index ∈ 16–24) — or by sampling the prebaked bank. An index-blind RGBA shader gets those pixels wrong by up to two blue steps. |
| Green Jewel lift | `light_timer` | same | per pixel — exact |
| Fade to/from black, intro per-channel fade | weights | same | per pixel — exact |
| Colour-31 per region / Crystal Orb reveal | region, `secret_timer` | swap entry 31 | bake per region (done: the atlases ship with their region's colour 31; region 9's two secret tiles are extra master-atlas variants, see `tiles/master/master.json`) |
| `colorplay` teleport strobe (`fmain2.c:425-432`): entries 1..31 random each frame for 32 frames | random | 32 random palettes | needs the index → use the indexed PNGs, or accept a non-identical RGBA approximation |
| `win_colors` sunrise (`fmain2.c:1605-1636`): hand-rolled `sun_colors` sweep over `winpic` | `sun_colors` | drive the indexed `screens/winpic` | needs the index — indexed path |

Only the last two effects genuinely require the palette index at run time; both are one-off
cinematics over indexed images that ship in this bundle anyway.

## Reviewing in the app

Each shader has an item in `verify.json` (view `shader`). The viewer compiles the file as-is,
lets you pick any region atlas or actor sheet from `previews/previews.json`, exposes the
uniforms, and — whenever the uniforms correspond to a baked level — reports whether the GPU
output is pixel-identical to the Python render. "identical" on every baked level is the expected
result for `daynight_live`, `daynight_bank` and `fade_to_black` (at full weights).
