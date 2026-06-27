# Experiment: day/night cycle on prebaked RGBA via shaders

Challenges the claim in `assets/plan.md:119-121` that the original game's **vegetation night
boost (palette indices 16–24) is NOT shader-doable on RGBA**. It is — two ways, both bit-exact.
See **`FINDINGS.md`** for the verdict and **`DECOMPOSITION.md`** for the exact `fade_page()` math.

## Files

| File | What it is |
|------|------------|
| `fade_page.py` | Verbatim integer port of `fade_page()` (`src/fmain2.c:377-419`) — the reference oracle. |
| `extract_experiment.py` | ADF → indexed hero (julian) + F8 terrain → `daynight_lut.json` + baked `frames/`. |
| `daynight_lut.json` | **The concrete RGB table**: per light level × 32 palette entries (`rgb4` + `rgba8`). |
| `daynight_bank.glsl` | Rebuttal #1 — sample/cross-fade a prebaked per-level RGBA bank (no runtime index). |
| `daynight_live.glsl` | Rebuttal #2 — full `fade_page()` live from full-bright RGBA + 1-bit `highlight_mask`. |
| `compare.py` | Bit-exact proof: bank == live == `fade_page()` at every level. Exit 0 = refuted. |
| `frames/` | `hero/` and `terrain/` → `full_bright/`, `highlight_mask/`, and `L<level>/` baked banks. |

## Run order

```bash
cd experiment/shaders
python3 extract_experiment.py          # needs PIL; reads ../../src/assets/image
python3 compare.py                     # prints the per-level diff table, asserts 0
```

`extract_experiment.py --all-frames` bakes all 67 hero frames (default: 8 representative poses).
`extract_experiment.py --image <path>` points at a different ADF.

## What each light level means

`fade_page.py CANONICAL_LEVELS = [0, 86, 95, 105, 111, 120, 136, 150, 165, 180]`: deep night
(`0–86`, identical floor plateau) → twilight where the veg boost is active → full day (`180`,
identical to the original `pagecolors`). The boost steps down at `111` and ends at `136`
(see the band table in `DECOMPOSITION.md`).

## Shader wiring (for the porting team)

- **bank**: upload `frames/<subject>/L*/` as a `sampler2DArray`; drive `uLevel01` from the clock.
  Set `t == 0` (uLevel01 on a baked layer) for pixel-exact original output; lerp for smooth tween.
- **live**: bind `frames/<subject>/full_bright/` and `frames/<subject>/highlight_mask/`; set
  `uLightlevel` (0–300). Reproduces any continuous light level with one extra 1-bit channel.
