#!/usr/bin/env python3
"""
fade_page.py — faithful Python port of the original day/night palette function.

Source of truth: fade_page() in src/fmain2.c:377-419 (driver day_fade() at 1653-1660).
This is the SINGLE function that produces the entire Faery Tale day/night cycle by
manipulating the 32-entry palette. Everything else in this experiment is checked
against the output of this module.

All arithmetic is integer and mirrors the C verbatim (including the truncating
integer divisions /1600 and /100), so the result is bit-exact with the original.
"""

# pagecolors[] — verbatim from src/fmain2.c:367-371 (12-bit Amiga 0x0RGB).
PAGECOLORS = [
    0x0000, 0x0FFF, 0x0E96, 0x0B63, 0x0631, 0x07BF, 0x0333, 0x0DB8,
    0x0223, 0x0445, 0x0889, 0x0BBC, 0x0521, 0x0941, 0x0F82, 0x0FC7,
    0x0040, 0x0070, 0x00B0, 0x06F6, 0x0005, 0x0009, 0x000D, 0x037F,
    0x0C00, 0x0F50, 0x0FA0, 0x0FF6, 0x0EB6, 0x0EA5, 0x000F, 0x0BDF,
]

# The palette indices that receive the contested "vegetation night boost".
VEG_LO, VEG_HI = 16, 24


def expand4(n):
    """4-bit channel -> 8-bit by nibble duplication (0xF->0xFF). Matches
    tools/extract_sprites.py:29-36 and src/palette.rs amiga_color_to_rgba()."""
    return (n << 4) | n


def rgb4_to_rgba8(c, index=None):
    """RGB4 0x0RGB (Amiga OCS 12-bit) -> (r,g,b,a) 8-bit. Alpha 0 only for sprite index 31."""
    r = expand4((c >> 8) & 0xF)
    g = expand4((c >> 4) & 0xF)
    b = expand4(c & 0xF)
    a = 0 if index == 31 else 255
    return (r, g, b, a)


def color31_override(region_num, secret_timer=0):
    """Per-region color-31 override, from fade_page() top (fmain2.c:381-386)."""
    if region_num == 4:
        return 0x0980
    if region_num == 9:
        return 0x00F0 if secret_timer else 0x0445
    return 0x0BDF


def fade_palette(r, g, b, limit, colors=None, light_timer=0,
                 region_num=0, secret_timer=0):
    """Port of fade_page(r,g,b,limit,colors) -> list[32] of 12-bit Amiga colors.

    r, g, b      : light-level params (see day_fade(): r=ll-80(+ll_torch),
                   g=ll-61, b=ll-62). Clamped here exactly as the C does.
    limit        : TRUE for outdoor day/night (applies night floors + veg boost).
    light_timer  : torch/green-jewel flag (0 in this experiment; kept for fidelity).
    """
    colors = list(PAGECOLORS if colors is None else colors)
    colors[31] = color31_override(region_num, secret_timer)

    if r > 100:
        r = 100
    if g > 100:
        g = 100
    if b > 100:
        b = 100
    if limit:
        if r < 10:
            r = 10               # night limits
        if g < 25:
            g = 25
        if b < 60:
            b = 60
        g2 = (100 - g) // 3
    else:
        if r < 0:
            r = 0
        if g < 0:
            g = 0
        if b < 0:
            b = 0
        g2 = 0

    fader = [0] * 32
    for i in range(32):
        r1 = (colors[i] & 0x0F00) >> 4
        g1 = colors[i] & 0x00F0
        b1 = colors[i] & 0x000F
        if light_timer and (r1 < g1):       # green jewel (unused here)
            r1 = g1
        r1 = (r * r1) // 1600
        g1 = (g * g1) // 1600
        b1 = (b * b1 + (g2 * g1)) // 100     # moonlight blue injection
        if limit:
            if VEG_LO <= i <= VEG_HI and g > 20:   # vegetation night boost
                if g < 50:
                    b1 += 2
                elif g < 75:
                    b1 += 1
            if b1 > 15:
                b1 = 15
        fader[i] = (r1 << 8) + (g1 << 4) + b1
    return fader


def day_fade_params(lightlevel, torch=False):
    """day_fade() param derivation (fmain2.c:1653-1660), outdoor region (<8).
    Returns (r, g, b) to feed fade_palette with limit=TRUE."""
    ll_torch = 200 if torch else 0
    return (lightlevel - 80 + ll_torch, lightlevel - 61, lightlevel - 62)


def fade_palette_at(lightlevel, region_num=0, torch=False, secret_timer=0):
    """Convenience: full outdoor day/night palette at a given lightlevel."""
    r, g, b = day_fade_params(lightlevel, torch)
    return fade_palette(r, g, b, True, light_timer=(1 if torch else 0),
                        region_num=region_num, secret_timer=secret_timer)


# The canonical light-level set: deep night -> veg-boost band -> full day.
# 0..86 are all identical deep-night (floors); 111 and 136 are the boost steps.
CANONICAL_LEVELS = [0, 86, 95, 105, 111, 120, 136, 150, 165, 180]
FULL_BRIGHT_LEVEL = 180   # r=g=b=100: fader nibbles == original palette nibbles


if __name__ == "__main__":
    # Sanity: print the palette at each canonical level as 0x0RGB.
    for lv in CANONICAL_LEVELS:
        pal = fade_palette_at(lv)
        print(f"L{lv:>3}: " + " ".join(f"{c:03X}" for c in pal))
