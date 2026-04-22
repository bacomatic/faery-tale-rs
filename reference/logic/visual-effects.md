# Visual Effects — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, fmain2.c
> Cross-refs: [RESEARCH §20](../ARCHITECTURE.md#9-visual-effects), [ARCHITECTURE.md — Display](../ARCHITECTURE.md#3-display-system), [quests.md#end_game_sequence](quests.md#end_game_sequence), [brother-succession.md#revive](brother-succession.md#revive)

## Overview

This spec covers the non-gameplay visual subsystem: palette fades, full-screen message placards, intro page transitions, viewport zoom, and the victory sunrise animation. Every routine here manipulates the playfield `ViewPort` palette (`vp_page`) or the two off-screen BitMaps (`pagea`, `pageb`) that back the intro; none of them touch actor state.

The central primitive is [`fade_page`](#fade_page), which takes three 0..100 weights (r, g, b) and a source palette and recomputes the 32-entry `fader` table with per-channel scaling, night-mode floors, light-spell tinting, and sky/water blue boost, then `LoadRGB4`s it onto `vp_page`. [`fade_down`](#fade_down) and [`fade_normal`](#fade_normal) are the two standard wrappers that step `fade_page` from 100→0 or 0→100 in 21 frames. They are invoked around every cinematic transition — map/book open, dialogue, death placard, and region change.

The intro sequence chains three pieces: [`copypage`](#copypage) loads an IFF brush pair into `pageb`, [`flipscan`](#flipscan) columnwise-wipes the old `pagea` into the new `pageb` over 22 frames with accelerating column rates from `flip1/flip2/flip3`, and the outer driver at `fmain.c:1199/1209` calls [`screen_size`](#screen_size) to widen or narrow the playfield viewport while `fade_page` synchronises the palette brightness.

Two specialised routines close the system. [`map_message`](#map_message) and [`message_off`](#message_off) flip `vp_text` into and out of `VP_HIDE` so that the playfield takes over the full screen for book pages and placards (victory, game-over, princess rescue). [`win_colors`](#win_colors) is a dedicated 55-frame sunrise animation that runs the `sun_colors[]` gradient across the winpic IFF brush after the victory placard; it is the one visual effect that uses its own hand-rolled palette loop rather than `fade_page`. [`colorplay`](#colorplay) is the 32-frame psychedelic palette storm used for teleport events, and it too bypasses `fade_page` — it writes random 12-bit colours directly into `fader`.

Related effects documented elsewhere:
- The end-game sunrise after the victory placard is `end_game_sequence` in [quests.md#end_game_sequence](quests.md#end_game_sequence); it wraps [`win_colors`](#win_colors).
- Brother-succession placards (death + next-brother-set-out text) are in [brother-succession.md#revive](brother-succession.md#revive).
- The witch beam (XOR polygon, `witch_fx`) is a per-tick rendering effect and lives with combat drawing; it is not re-documented here. See `reference/_discovery/visual-effects.md` for the trace.

## Symbols

All numeric literals in the pseudo blocks below carry inline `# fmain.c:LINE — meaning` or `# fmain2.c:LINE — meaning` annotations rather than being promoted to named constants in SYMBOLS.md. Proposed SYMBOLS.md additions (see final report) are listed below; each is referenced via the `Calls:` line of every function that uses it until they are registered.

Proposed additions (not yet registered):

- `pagecolors: list[u16]` — `fmain2.c:369-374` — 32-entry active-region palette.
- `introcolors: list[u16]` — `fmain2.c` — 32-entry intro/title palette.
- `pagea: BitMap` — `fmain2.c:780` — off-screen brush page A.
- `pageb: BitMap` — `fmain2.c:780` — off-screen brush page B.
- `flip1: list[u8]` — `fmain2.c:793` — per-frame column step rate (22 entries).
- `flip2: list[u8]` — `fmain2.c:794` — per-frame column width (22 entries).
- `flip3: list[u8]` — `fmain2.c:795` — per-frame inter-frame delay (22 entries).
- `skipp: bool` — `fmain2.c:780` — intro-skip latch (set by space bar).
- `rp_map: RastPort` — `fmain2.c` — full-screen placard RastPort.
- `ri_page1: RasInfo` — `fmain.c` — double-buffer page 1 RasInfo.
- `ri_page2: RasInfo` — `fmain.c` — double-buffer page 2 RasInfo.
- `v: View` — `fmain.c` — Intuition View (driven by MakeVPort).

Globals already in [SYMBOLS.md](SYMBOLS.md) used here: `vp_page`, `vp_text`, `fp_drawing`, `bm_draw`, `rp`, `rp_text`, `afont`, `tfont`, `fader`, `blackcolors`, `sun_colors`, `viewstatus`, `region_num`, `secret_timer`, `light_timer`.

## fade_page

Source: `fmain2.c:377-420`
Called by: `fade_down`, `fade_normal`, `screen_size`, `check_door` (day/night ramp, `fmain.c:1890`)
Calls: `LoadRGB4`, `pagecolors`, `fader`, `vp_page`, `region_num`, `secret_timer`, `light_timer`

```pseudo
def fade_page(r: i32, g: i32, b: i32, limit: bool, colors: list) -> None:
    """Scale a 32-entry palette by per-channel weights with night/torch/sky adjustments."""
    # -------- Border color for the UI frame (color 31) depends on region.  fmain2.c:381-386
    if region_num == 4:                                  # fmain2.c:381 — region 4 = desert
        pagecolors[31] = 0x0980                          # fmain2.c:381 — 31 = border slot; 0x0980 = amber
    elif region_num == 9:                                # fmain2.c:382 — region 9 = citadel
        if secret_timer:
            pagecolors[31] = 0x00f0                      # fmain2.c:384 — 0x00f0 = bright green
        else:
            pagecolors[31] = 0x0445                      # fmain2.c:385 — 0x0445 = dark default
    else:
        pagecolors[31] = 0x0bdf                          # fmain2.c:386 — 0x0bdf = light-blue border

    # -------- Clamp channel weights to [0, 100].                           fmain2.c:388-390
    if r > 100:                                          # fmain2.c:388 — 100 = full brightness
        r = 100                                          # fmain2.c:388
    if g > 100:                                          # fmain2.c:389
        g = 100                                          # fmain2.c:389
    if b > 100:                                          # fmain2.c:390
        b = 100                                          # fmain2.c:390

    # -------- Night limits vs. clean zero.                                 fmain2.c:391-400
    if limit:
        if r < 10:                                       # fmain2.c:392 — 10 = night red floor
            r = 10                                       # fmain2.c:392
        if g < 25:                                       # fmain2.c:393 — 25 = night green floor
            g = 25                                       # fmain2.c:393
        if b < 60:                                       # fmain2.c:394 — 60 = night blue floor
            b = 60                                       # fmain2.c:394
        g2 = (100 - g) / 3                               # fmain2.c:395 — 100, 3 = residual-green divisor
    else:
        if r < 0:
            r = 0
        if g < 0:
            g = 0
        if b < 0:
            b = 0
        g2 = 0

    # -------- Per-color scaling loop.                                      fmain2.c:402-416
    i = 0
    while i < 32:                                        # fmain2.c:402 — 32 = palette length
        r1 = (colors[i] & 0x0f00) >> 4                   # fmain2.c:403 — 0x0f00 = red mask; 4 decimates to 0x00f0 range
        g1 = colors[i] & 0x00f0                          # fmain2.c:404 — green nibble
        b1 = colors[i] & 0x000f                          # fmain2.c:405 — blue nibble
        # Torch / light-spell: lift red to match green so palette warms up.  fmain2.c:406
        if light_timer and r1 < g1:
            r1 = g1                                      # fmain2.c:406
        r1 = (r * r1) / 1600                             # fmain2.c:407 — 1600 = 100 * 16 normaliser
        g1 = (g * g1) / 1600                             # fmain2.c:408
        b1 = (b * b1 + g2 * g1) / 100                    # fmain2.c:409 — 100 = blue divisor
        if limit:
            # Sky / water band (colors 16..24): boost blue at dusk/dawn.   fmain2.c:410-412
            if i >= 16 and i <= 24 and g > 20:           # fmain2.c:410 — 16, 24 = sky band; 20 = g floor
                if g < 50:                               # fmain2.c:411 — 50 = big-boost threshold
                    b1 = b1 + 2
                elif g < 75:                             # fmain2.c:411 — 75 = small-boost threshold
                    b1 = b1 + 1
            if b1 > 15:                                  # fmain2.c:413 — 15 = max 4-bit blue
                b1 = 15                                  # fmain2.c:413
        # Pack RGB4: 0xRGB.                                                  fmain2.c:415
        fader[i] = (r1 << 8) + (g1 << 4) + b1            # fmain2.c:415 — 8, 4 = nibble shifts
        i = i + 1

    LoadRGB4(vp_page, fader, 32)                         # fmain2.c:417 — 32 = palette length
```

## colorplay

Source: `fmain2.c:425-431`
Called by: `teleport handlers` (entry point — invoked where a teleport event occurs)
Calls: `bitrand`, `LoadRGB4`, `Delay`, `fader`, `vp_page`

```pseudo
def colorplay() -> None:
    """32-frame random-palette strobe for teleport events; colors 1..31 are randomised each frame."""
    j = 0
    while j < 32:                                        # fmain2.c:427 — 32 = frame count
        i = 1                                            # skip color 0 to keep background
        while i < 32:                                    # fmain2.c:428 — 32 = palette length
            fader[i] = bitrand(0xfff)                    # fmain2.c:428 — 0xfff = 12-bit RGB4 mask
            i = i + 1
        LoadRGB4(vp_page, fader, 32)                     # fmain2.c:429 — 32 = palette length
        Delay(1)                                         # fmain2.c:430 — one tick (~17 ms)
        j = j + 1
```

## fade_down

Source: `fmain2.c:623-625`
Called by: `map_message`, `message_off`, many transitions (region change, death, book, dialogue)
Calls: `fade_page`, `Delay`, `pagecolors`

```pseudo
def fade_down() -> None:
    """Ramp pagecolors from 100% to 0% brightness over 21 frames."""
    i = 100                                              # fmain2.c:624 — 100 = full brightness
    while i >= 0:
        fade_page(i, i, i, False, pagecolors)            # fmain2.c:624
        Delay(1)                                         # fmain2.c:624
        i = i - 5                                        # fmain2.c:624 — 5% per tick
```

## fade_normal

Source: `fmain2.c:627-629`
Called by: `render_loop` (phase 13, `fmain.c:1374` — on viewstatus 3), `message_off` chain, transitions
Calls: `fade_page`, `Delay`, `pagecolors`

```pseudo
def fade_normal() -> None:
    """Ramp pagecolors from 0% to 100% brightness over 21 frames."""
    i = 0
    while i <= 100:                                      # fmain2.c:628 — 100 = full brightness
        fade_page(i, i, i, False, pagecolors)            # fmain2.c:628
        Delay(1)                                         # fmain2.c:628
        i = i + 5                                        # fmain2.c:628 — 5% per tick
```

## map_message

Source: `fmain2.c:601-611`
Called by: `revive` (death/next-brother placards), `rescue` (princess cinematic), `end_game_sequence` (victory placard), book/map UIs
Calls: `fade_down`, `SetDrMd`, `SetAPen`, `SetRast`, `stillscreen`, `LoadRGB4`, `pagechange`, `rp`, `rp_map`, `fp_drawing`, `vp_text`, `vp_page`, `pagecolors`, `viewstatus`

```pseudo
def map_message() -> None:
    """Fade out, retarget rp to the full-screen map RastPort, hide the text VP, fade back in."""
    fade_down()                                          # fmain2.c:602
    rp = rp_map                                          # fmain2.c:603 — switch RastPort
    rp_map.BitMap = fp_drawing.ri_page.BitMap            # fmain2.c:604 — share the page bitmap
    SetDrMd(rp, 1)                                       # fmain2.c:605 — 1 = JAM1 drawing mode
    SetAPen(rp, 24)                                      # fmain2.c:605 — 24 = placard ink color
    SetRast(rp, 0)                                       # fmain2.c:606 — clear bitmap to color 0

    # Hide the hi-res status viewport so the playfield claims the full screen.
    vp_text.Modes = 0x8204                               # fmain2.c:608 — HIRES|SPRITES|VP_HIDE
    stillscreen()                                        # fmain2.c:609
    LoadRGB4(vp_page, pagecolors, 32)                    # fmain2.c:610 — 32 = palette length
    viewstatus = 2                                       # fmain2.c:611 — 2 = full-screen-message mode
```

## message_off

Source: `fmain2.c:613-620`
Called by: `revive`, `rescue`, post-placard chains
Calls: `fade_down`, `pagechange`, `rp`, `rp_text`, `vp_text`, `viewstatus`

```pseudo
def message_off() -> None:
    """Leave full-screen placard mode: fade out, reattach rp to the status VP, flip, queue a fade-in."""
    fade_down()                                          # fmain2.c:614
    rp = rp_text                                         # fmain2.c:615 — restore hi-res RastPort
    vp_text.Modes = 0x8200                               # fmain2.c:616 — HIRES|SPRITES (no VP_HIDE)
    pagechange()                                         # fmain2.c:617
    viewstatus = 3                                       # fmain2.c:618 — 3 = run fade_normal on next tick
```

## copypage

Source: `fmain2.c:781-791`
Called by: `main` (intro, `fmain.c:1203-1205`)
Calls: `Delay`, `BltBitMap`, `unpackbrush`, `skipint`, `flipscan`, `pagea`, `pageb`, `skipp`

```pseudo
def copypage(br1: str, br2: str, x: i32, y: i32) -> None:
    """One intro 'page': hold the current image, load the next brush pair into pageb, flipscan to it."""
    if skipp:                                            # fmain2.c:782 — space bar already hit
        return
    Delay(350)                                           # fmain2.c:783 — 350 = ~5.8 s read time
    # Save the currently-shown pageb onto pagea as the 'old' image.        fmain2.c:784
    BltBitMap(pageb, 0, 0, pagea, 0, 0, 320, 200, 0xC0, 0x1f, 0) # fmain2.c:784 — 320x200 page; 0xC0/0x1f minterm/planemask
    unpackbrush(br1, pageb, 4, 24)                       # fmain2.c:785 — 4, 24 = background brush anchor
    unpackbrush(br2, pageb, x, y)                        # fmain2.c:786 — overlay brush at caller (x,y)
    if skipint():                                        # fmain2.c:787 — poll space bar once
        return
    flipscan()                                           # fmain2.c:788
    skipint()                                            # fmain2.c:789
```

## flipscan

Source: `fmain2.c:796-833`
Called by: `copypage`
Calls: `BltBitMap`, `page_det`, `pagechange`, `Delay`, `fp_drawing`, `pagea`, `pageb`, `flip1`, `flip2`, `flip3`

```pseudo
def flipscan() -> None:
    """22-frame columnar wipe between pagea and pageb using flip1/2/3 as per-frame rate/width/delay tables."""
    i = 0
    while i < 22:                                        # fmain2.c:797 — 22 = flipscan frames
        temp = fp_drawing.ri_page.BitMap                 # fmain2.c:798
        dcol = 0
        rate = flip1[i]                                  # fmain2.c:800 — column step per frame
        wide = flip2[i]                                  # fmain2.c:801 — strip width per frame
        if i < 11:                                       # fmain2.c:802 — 11 = phase-split boundary
            # -------- Phase 1 (right half reveal).                 fmain2.c:803-813
            bcol = 161 - wide                            # fmain2.c:803 — 161 = right-half dest X
            BltBitMap(pageb, 160, 0, temp, 160, 0, 135, 200, 0xC0, 0x1f, 0) # fmain2.c:806 — 160 half; 135x200 panel
            if rate == 0:
                pagechange()                             # fmain2.c:830
                if flip3[i]:
                    Delay(flip3[i])                      # fmain2.c:831
                i = i + 1
                continue
            scol = wide
            h = 7                                        # fmain2.c:812 — 7 = seam-top fallback if loop empty
            while scol < 136:                            # fmain2.c:808 — 136 = column scan limit
                h = page_det(scol)                       # fmain2.c:809 — per-column vertical margin
                BltBitMap(pagea, bcol + scol, h, temp, 161 + dcol, h, wide, 200 - h - h, 0xC0, 0x1f, 0) # fmain2.c:810
                dcol = dcol + wide                       # fmain2.c:811
                scol = scol + rate                       # fmain2.c:808
            BltBitMap(pagea, 296, 7, temp, 161 + dcol, h, 1, 186, 0xC0, 0x1f, 0) # fmain2.c:812 — 296, 7, 186 seam
        else:
            # -------- Phase 2 (left half reveal).                  fmain2.c:815-828
            bcol = 160                                   # fmain2.c:815 — 160 = half-width source base
            if rate == 0:
                BltBitMap(pageb, 24, 0, temp, 24, 0, 135, 200, 0xC0, 0x1f, 0) # fmain2.c:817 — 24 = left origin
                pagechange()                             # fmain2.c:830
                if flip3[i]:
                    Delay(flip3[i])                      # fmain2.c:831
                i = i + 1
                continue
            BltBitMap(pagea, 24, 0, temp, 24, 0, 135, 200, 0xC0, 0x1f, 0) # fmain2.c:819
            scol = wide
            while scol < 136:                            # fmain2.c:820
                h = page_det(scol)
                dcol = dcol + wide
                BltBitMap(pageb, bcol - scol, h, temp, bcol - dcol, h, wide, 200 - h - h, 0xC0, 0x1f, 0) # fmain2.c:823
                scol = scol + rate
            scol = 135                                   # fmain2.c:826 — 135 = last column
            h = 7                                        # fmain2.c:826 — 7 = seam-top
            BltBitMap(pageb, 24, h, temp, 159 - dcol, h, 1, 200 - h - h, 0xC0, 0x1f, 0) # fmain2.c:827

        pagechange()                                     # fmain2.c:830
        if flip3[i]:
            Delay(flip3[i])                              # fmain2.c:831
        i = i + 1
```

## screen_size

Source: `fmain.c:2914-2933`
Called by: `main` (intro zoom in/out, `fmain.c:1187/1199/1209/1220`), `load_game` (`fmain2.c:1613` via `win_colors`)
Calls: `Delay`, `fade_page`, `MakeVPort`, `pagechange`, `ri_page1`, `ri_page2`, `vp_page`, `vp_text`, `introcolors`, `v`

```pseudo
def screen_size(x: i32) -> None:
    """Resize the playfield viewport to 2x by 2y (y = x*5/8); sync palette brightness via fade_page."""
    y = (x * 5) / 8                                      # fmain.c:2917 — 5:8 aspect ratio
    Delay(2)                                             # fmain.c:2919 — 2 = ~33 ms per zoom step

    # -------- Re-center the playfield viewport.                     fmain.c:2921-2925
    ri_page2.RxOffset = 160 - x                          # fmain.c:2921 — 160 = half-screen X
    ri_page1.RxOffset = 160 - x                          # fmain.c:2921
    vp_page.DxOffset = 160 - x                           # fmain.c:2921
    vp_page.DWidth = x + x                               # fmain.c:2922 — width = 2x
    ri_page2.RyOffset = 100 - y                          # fmain.c:2924 — 100 = half-screen Y
    ri_page1.RyOffset = 100 - y                          # fmain.c:2924
    vp_page.DyOffset = 100 - y                           # fmain.c:2924
    vp_page.DHeight = y + y                              # fmain.c:2925 — height = 2y

    # -------- Shrink the text viewport to the margin above the playfield.  fmain.c:2927-2929
    vp_text.DxOffset = 0
    vp_text.DyOffset = 0
    vp_text.DHeight = 95 - y                             # fmain.c:2929 — 95 = status-bar max height

    # -------- Palette tracks viewport growth.                        fmain.c:2931
    fade_page(y + y - 40, y + y - 70, y + y - 100, False, introcolors) # fmain.c:2931 — 40, 70, 100 = dim offsets

    MakeVPort(v, vp_text)                                # fmain.c:2932 — rebuild copper list
    pagechange()                                         # fmain.c:2933
```

## win_colors

Source: `fmain2.c:1605-1636`
Called by: `end_game_sequence` (see [quests.md#end_game_sequence](quests.md#end_game_sequence), invoked at `fmain2.c` end-game chain)
Calls: `placard_text`, `name`, `placard`, `Delay`, `unpackbrush`, `LoadRGB4`, `screen_size`, `bm_draw`, `fp_drawing`, `vp_page`, `vp_text`, `blackcolors`, `fader`, `sun_colors`

```pseudo
def win_colors() -> None:
    """Victory sunrise: show placard, load winpic brush, walk sun_colors[] across 55 frames, black out."""
    # -------- Victory placard: msg7 + hero name + msg7a, bordered.  fmain2.c:1607
    placard_text(6)                                      # fmain2.c:1607 — narr.asm msg7
    name()
    placard_text(7)                                      # fmain2.c:1607 — narr.asm msg7a
    placard()
    Delay(80)                                            # fmain2.c:1607 — 80 = ~1.33 s hold

    # -------- Fresh winpic behind a black palette.                  fmain2.c:1608-1612
    bm_draw = fp_drawing.ri_page.BitMap                  # fmain2.c:1608
    unpackbrush("winpic", bm_draw, 0, 0)                 # fmain2.c:1609
    LoadRGB4(vp_page, blackcolors, 32)                   # fmain2.c:1610 — 32 = palette length
    LoadRGB4(vp_text, blackcolors, 32)                   # fmain2.c:1611
    vp_text.Modes = 0x8204                               # fmain2.c:1611 — HIRES|SPRITES|VP_HIDE
    screen_size(156)                                     # fmain2.c:1612 — 156 = gameplay viewport half-width

    # -------- 55-frame sunrise: i walks 25 → -29.                   fmain2.c:1614-1631
    i = 25                                               # fmain2.c:1614 — 25 = sunrise start
    while i > -30:                                       # fmain2.c:1614 — -30 = sunrise end (exclusive)
        fader[0] = 0                                     # fmain2.c:1615 — color 0 black
        fader[31] = 0                                    # fmain2.c:1615 — color 31 black
        fader[1] = 0xfff                                 # fmain2.c:1615 — color 1 white
        fader[28] = 0xfff                                # fmain2.c:1615 — color 28 white

        # Colors 2..27 follow sun_colors[] offset by i.              fmain2.c:1616-1619
        j = 2                                            # fmain2.c:1616 — 2 = first gradient slot
        while j < 28:                                    # fmain2.c:1616 — 28 = upper bound
            if i + j > 0:
                fader[j] = sun_colors[i + j]             # fmain2.c:1617
            else:
                fader[j] = 0                             # fmain2.c:1618
            j = j + 1

        # Colors 29..30 (reds) hold until i crosses -14, then fade.  fmain2.c:1620-1627
        if i > -14:                                      # fmain2.c:1620 — -14 = crossover frame
            fader[29] = 0x800                            # fmain2.c:1622 — color 29 hold red
            fader[30] = 0x400                            # fmain2.c:1623 — color 30 hold red
        else:
            j = (i + 30) / 2                             # fmain2.c:1626 — 30 offset; 2 divisor
            fader[29] = 0x100 * j                        # fmain2.c:1626 — 0x100 = per-step red scale
            fader[30] = 0x100 * (j / 2)                  # fmain2.c:1627

        LoadRGB4(vp_page, fader, 32)                     # fmain2.c:1628 — 32 = palette length
        if i == 25:                                      # fmain2.c:1629 — 25 = first-frame marker
            Delay(60)                                    # fmain2.c:1629 — 60 = ~1 s extra hold
        Delay(9)                                         # fmain2.c:1630 — 9 = ~150 ms per frame
        i = i - 1

    Delay(30)                                            # fmain2.c:1632 — 30 = ~0.5 s final hold
    LoadRGB4(vp_page, blackcolors, 32)                   # fmain2.c:1633 — 32 = palette length
```

## Notes

Several short routines are not given their own entries because each is ≤3 lines and entirely mechanical; they are summarised here for completeness.

- **`stillscreen()`** — `fmain2.c:631-634`. Zeroes `fp_drawing.ri_page.RxOffset` and `RyOffset` and calls `pagechange()`. Used by [`map_message`](#map_message) and the inventory/book screens (`fmain.c:3141-3143`) to freeze scroll for a static full-screen image.
- **`skipint()`** — `fmain2.c:836`. One-liner: sets and returns `skipp = (getkey() == ' ')`. The intro chain in `main` and [`copypage`](#copypage) poll it to abort the intro when the player presses space.
- **Flasher border blink** — `fmain.c:1368-1370`. Not a function; in the main loop's `viewstatus == 1` branch, `SetRGB4(vp_page, 31, 15, 15, 15)` or `(0,0,0)` is called on alternating 16-tick intervals using `flasher & 16`, blinking color 31 (the dialogue cursor / prompt highlight).
- **`xfer()` viewstatus = 99** — `fmain.c:2625-2645`. Not a visual effect per se, but the teleport entry point sets `viewstatus = 99` to force a full redraw and recolor on the next tick. Included here because `colorplay()` is often scheduled alongside it.
- **Unlinked assembly duplicates** — `fsupp.asm:1-26` (`_colorplay`), `fsupp.asm:27-34` (`_stillscreen`), `fsupp.asm:36-42` (`_skipint`). The makefile does **not** assemble or link `fsupp.asm`; the live versions are the C ones in `fmain2.c`. Porters should ignore these.
- **Witch beam XOR rendering** (`witch_fx`, `fmain2.c:917-965`) is a combat-layer per-tick renderer, not a cinematic transition. See `reference/_discovery/visual-effects.md` for the full trace; it will be documented with combat drawing in a future wave.

### Night/day palette coupling

`fade_page`'s `limit` parameter is set `TRUE` only by `check_door` (`fmain.c:1890`) and the day/night ramp in the main loop. When `limit==TRUE`, the floors (`r≥10, g≥25, b≥60`) prevent total darkness at night, and colors 16..24 (the sky/water band) get an additional blue boost. This is why night renders with a distinct blueish cast and why lava glows (because the red-channel floor keeps warm tones visible).

### light_timer cross-effect

`light_timer` (set by magic item 6 at `fmain.c:3306`, decremented each tick at `fmain.c:1380`) is primarily a visual palette tint — inside [`fade_page`](#fade_page)'s per-color loop, if `r1 < g1`, `r1` is lifted to `g1`, producing a torch-warm palette. It also overrides `lightlevel` to 200 inside `day_fade` (`fmain2.c:1655`, not documented here), so the torch spell both warms and brightens the scene. This coupling is noted so porters don't treat it as a pure cosmetic effect.
