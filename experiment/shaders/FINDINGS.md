# Findings: the day/night cycle *is* shader-doable on prebaked RGBA

## The claim under test

`assets/plan.md` (Graphics §5a, lines 119–121):

> the **vegetation night boost on palette indices 16–24 is NOT shader-doable on RGBA** and must
> use the indexed atlas + a palette LUT.

## Verdict: **false as stated.** The full cycle, including the indices-16–24 vegetation boost, is reproduced **bit-exactly** on prebaked RGBA.

`compare.py` checks 264 subjects (8 hero frames + 256 F8 terrain tiles) across all 10 canonical
light levels and reports **zero** differences for both reproduction paths:

```
 level |  bank Δ |  live Δ | highlight px boosted | max boost Δ(8bit)
     0 |       0 |       0 |          25980 |                34
    95 |       0 |       0 |          25980 |                34
   111 |       0 |       0 |          25980 |                17
   136 |       0 |       0 |              0 |                 0
   180 |       0 |       0 |              0 |                 0
   ...all levels: bank Δ = live Δ = 0...
PASS: BANK and LIVE are BIT-EXACT vs fade_page() at every level.
```

## Two independent mechanisms, both exact

1. **Per-light-level bake (`daynight_bank.glsl`).** Bake one RGBA frame per light level
   (`extract_experiment.py`). The veg boost is precomputed into the texels, so runtime needs no
   palette index at all — the shader samples the bank (and may cross-fade adjacent levels for
   smooth time-of-day). This *is* "the indexed atlas + palette LUT" the claim demands, just
   evaluated at bake time instead of frame time.

2. **RGBA + 1-bit highlight-mask, live (`daynight_live.glsl`).** A single fragment shader reproduces
   `fade_page()` at any continuous `lightlevel` directly from the full-bright RGBA, recovering the
   original 4-bit nibbles via `channel // 17` (full-bright == `nibble*17`, exact) and running the
   identical integer math. `compare.py`'s LIVE column is an independent CPU reimplementation of
   this shader (different code path from the bake) and is bit-exact everywhere.

## What the claim got *right* — and the one bit it costs to fix

The boost is genuinely **index-dependent**: 25,980 vegetation pixels change, by up to 34/255 per
channel (two 4-bit steps) at night. An *index-blind* RGBA shader — plain brightness dimming with no
extra data — **cannot** know which pixels were indices 16–24 and would omit the boost. That is the
kernel of truth in the original concern.

The fix is small and concrete: **one extra bit per pixel** (the `highlight_mask`) restores exactly the
information baking discarded. With it, the live shader is bit-exact; without it (the `MASK_GAIN`
column with the mask zeroed) those 25,980 pixels diverge — which is precisely the quantity the mask
recovers. So the accurate statement is not "NOT shader-doable on RGBA" but:

> The vegetation night boost is shader-doable on RGBA **given a 1-bit highlight mask** (or,
> equivalently, by baking one RGBA frame per light level). Pure index-blind RGBA dimming cannot
> reproduce it.

## Recommended correction to `assets/plan.md`

Replace the absolute "NOT shader-doable on RGBA" wording (lines 119–121, cross-referenced in §15
`palette-effects matrix`) with the qualified statement above, and add `daynight_live.glsl` +
`daynight_bank.glsl` (with the `highlight_mask` channel) to the shader deliverables as the supported
RGBA path. The indexed atlas remains a valid option, but is no longer *required*.

## Reproduce

```
cd experiment/shaders
python3 extract_experiment.py     # decode + LUT + bake bank/masks  (PIL only)
python3 compare.py                # bit-exact proof; exit 0 = refuted
```
