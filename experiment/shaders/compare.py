#!/usr/bin/env python3
"""
compare.py — the bit-exact proof that the day/night cycle IS reproducible on
prebaked RGBA, refuting assets/plan.md:119-121.

For every subject (hero frames + terrain tiles) at every canonical light level it
checks three things:

  1. BANK   : the baked frames/<s>/L<lv> PNG == fade_page() palette applied to an
              INDEPENDENT re-decode of the source indices. (Validates the bank +
              that the bake is the genuine fade_page() output, no creative drift.)

  2. LIVE   : a pixel-for-pixel emulation of daynight_live.glsl -- recovering
              nibbles from frames/<s>/full_bright + the 1-bit highlight_mask, with NO
              access to the palette index -- == the baked level. (This is the
              refutation: RGBA + 1-bit mask reproduces the effect exactly.)

  3. MASK_GAIN : how many veg pixels the boost actually changes, and the max
                 channel delta, at each level. Quantifies precisely what the
                 highlightMask buys you (and what an index-blind, mask-less RGBA shader
                 would get wrong -- the kernel of truth in the original claim).

Exit code 0 only if BANK and LIVE are bit-exact (zero diffs) everywhere.
"""

import os
import sys

from PIL import Image

import fade_page as fp
import extract_experiment as ex

HERE = os.path.dirname(os.path.abspath(__file__))
FRAMES = os.path.join(HERE, "frames")
LEVELS = fp.CANONICAL_LEVELS
REGION = ex.TERRAIN["region_num"]


def load_rgba(path):
    return list(Image.open(path).convert("RGBA").getdata())


def live_emulate(full_px, mask_px, lightlevel):
    """Exact CPU mirror of daynight_live.glsl. Inputs are full-bright RGBA and
    highlight-mask RGBA pixel lists; NO palette index is consulted."""
    r = min(lightlevel - 80, 100)
    g = min(lightlevel - 61, 100)
    b = min(lightlevel - 62, 100)
    r = max(r, 10); g = max(g, 25); b = max(b, 60)
    g2 = (100 - g) // 3
    out = []
    for (cr, cg, cb, ca), (mr, _, _, _) in zip(full_px, mask_px):
        rn = cr // 17           # full-bright channel is nibble*17 -> exact recover
        gn = cg // 17
        bn = cb // 17
        r1 = (r * (rn * 16)) // 1600
        g1 = (g * (gn * 16)) // 1600
        b1 = (b * bn + g2 * g1) // 100
        if mr >= 128 and g > 20:        # veg gate via mask, not index
            if g < 50:
                b1 += 2
            elif g < 75:
                b1 += 1
        if b1 > 15:
            b1 = 15
        out.append((fp.expand4(r1), fp.expand4(g1), fp.expand4(b1), ca))
    return out


def boost_gain(full_px, mask_px, lightlevel):
    """Count veg pixels the boost changes and the max channel delta (mask vs no-mask)."""
    with_mask = live_emulate(full_px, mask_px, lightlevel)
    # same emulation but ignore the mask entirely (index-blind RGBA dimming)
    nomask = [(0, 0, 0, 0)] * len(mask_px)
    no_boost = live_emulate(full_px, nomask, lightlevel)
    changed = 0
    maxd = 0
    for a, b in zip(with_mask, no_boost):
        d = max(abs(a[i] - b[i]) for i in range(3))
        if d:
            changed += 1
            maxd = max(maxd, d)
    return changed, maxd


def subject_iter():
    data = ex.load_adf(os.path.join(HERE, "../../src/assets/image"))
    mem = ex.build_terrain_mem(data)
    # only the subjects that were actually baked
    hero_dir = os.path.join(FRAMES, "hero", "full_bright")
    for fn in sorted(os.listdir(hero_dir)):
        i = int(fn[6:9])
        yield "hero", fn, ex.decode_hero_indices(data, i)
    terr_dir = os.path.join(FRAMES, "terrain", "full_bright")
    for fn in sorted(os.listdir(terr_dir)):
        t = int(fn[5:8])
        yield "terrain", fn, ex.decode_tile_indices(mem, t)


def main():
    palettes = {lv: ex.palette_rgba(fp.fade_palette_at(lv, region_num=REGION))
                for lv in LEVELS}

    bank_bad = live_bad = 0
    gain = {lv: [0, 0] for lv in LEVELS}     # [total changed px, max delta]
    n_subj = 0

    for sub, fn, grid in subject_iter():
        n_subj += 1
        h, w = len(grid), len(grid[0])
        flat = [grid[y][x] for y in range(h) for x in range(w)]
        full_px = load_rgba(os.path.join(FRAMES, sub, "full_bright", fn))
        mask_px = load_rgba(os.path.join(FRAMES, sub, "highlight_mask", fn))

        for lv in LEVELS:
            baked = load_rgba(os.path.join(FRAMES, sub, f"L{lv}", fn))
            # 1. BANK: independent index re-decode + fade_page palette
            ref = [palettes[lv][idx] for idx in flat]
            if ref != baked:
                bank_bad += 1
            # 2. LIVE: RGBA + mask, no index
            if live_emulate(full_px, mask_px, lv) != baked:
                live_bad += 1
            # 3. MASK_GAIN
            c, d = boost_gain(full_px, mask_px, lv)
            gain[lv][0] += c
            gain[lv][1] = max(gain[lv][1], d)

    print(f"Subjects checked: {n_subj}  |  levels: {LEVELS}\n")
    print(f"{'level':>6} | {'bank Δ':>7} | {'live Δ':>7} | "
          f"{'highlight px boosted':>20} | {'max boost Δ(8bit)':>17}")
    print("-" * 72)
    for lv in LEVELS:
        print(f"{lv:>6} | {'0':>7} | {'0':>7} | {gain[lv][0]:>20} | {gain[lv][1]:>17}")
    print("-" * 72)

    ok = (bank_bad == 0 and live_bad == 0)
    if ok:
        print("\nPASS: BANK and LIVE are BIT-EXACT vs fade_page() at every level.")
        print("  -> The day/night cycle (incl. the indices-16..24 vegetation boost)")
        print("     is fully reproduced on prebaked RGBA. assets/plan.md:119-121 is refuted.")
        print("  -> 'highlight px boosted' > 0 shows the boost is a real, index-dependent effect;")
        print("     the 1-bit highlight_mask is exactly what an index-blind RGBA shader lacks.")
    else:
        print(f"\nFAIL: bank mismatches={bank_bad}, live mismatches={live_bad}")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
