# Discovery: Visual Effects — Witch Beam, Color Cycling, Screen Transitions

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace all visual effects: witch_fx beam rendering, witchpoints sine table, colorplay teleport effect, flipscan intro transition, screen_size viewport zoom, copypage, fade functions, and other visual effects.

## References Found

### witch_fx
- fmain2.c:917-965 — write — complete `witch_fx()` function body
- fmain.c:2372-2376 — call — witch_fx called during rendering, damage applied if beam encloses hero
- fmain.c:1978 — call — witch_fx called again to erase (XOR undo) the beam from previous frame
- fmain.c:2367-2370 — write — witchx/witchy/witchdir set from witch position and witchindex
- fmain.c:591 — declaration — `witchflag, wdir` globals
- fmain.c:597 — declaration — `witchindex` global (UBYTE)
- ftale.h:78 — declaration — `short witchx, witchy, witchdir, wflag;` in struct fpage

### witchpoints
- fmain2.c:889-916 — write — complete 256-entry sine/cosine lookup table

### colorplay
- fmain2.c:425-431 — write — C version of `colorplay()` (this is the linked version)
- fsupp.asm:1-26 — write — assembly version `_colorplay` (**NOT LINKED** — not assembled or linked by makefile)

### flipscan
- fmain2.c:796-833 — write — complete `flipscan()` function
- fmain2.c:793-795 — write — `flip1[]`, `flip2[]`, `flip3[]` timing/width tables
- fsubs.asm:1448-1489 — write — `_page_det` function (vertical margin calculator)

### copypage
- fmain2.c:781-791 — write — complete `copypage()` function

### fade functions
- fmain2.c:377-420 — write — `fade_page()` full function body
- fmain2.c:623-625 — write — `fade_down()` function
- fmain2.c:627-629 — write — `fade_normal()` function

### screen_size
- fmain.c:2914-2933 — write — `screen_size()` function

### other visual effects
- fmain2.c:601-611 — write — `map_message()` — full-screen message transition
- fmain2.c:613-620 — write — `message_off()` — return from full-screen message
- fmain2.c:631-633 — write — `stillscreen()` C version (this is the linked version)
- fsupp.asm:27-34 — write — `_stillscreen` assembly version (**NOT LINKED** — not assembled or linked by makefile)
- fsupp.asm:36-43 — write — `_skipint` assembly function (**NOT LINKED** — not assembled or linked by makefile)
- fmain2.c:1605-1636 — write — `win_colors()` victory sunrise animation
- fmain.c:1368-1370 — write — flasher-based color 31 blinking (text cursor/border)
- fmain.c:3141-3143 — write — inventory screen color reset via `stillscreen()`

## Code Path

### 1. Witch Beam Effect (witch_fx)

**Setup (per frame)** — fmain.c:2367-2376:
1. Witch screen position calculated: `witchx = anim_list[2].abs_x - (map_x & 0xfff0)`, `witchy = (anim_list[2].abs_y - 15) - (map_y & 0xffe0)` — fmain.c:2367-2368
2. Beam direction updated: `witchindex += wdir` (UBYTE, wraps 0-255) — fmain.c:2370
3. Direction steering: every `rand4()==0` frame, if `s1>0` then `wdir=1` else `wdir=-1` — fmain.c:2369. This makes the beam "hunt" toward the hero.
4. `wflag` copied from `witchflag` global — fmain.c:2371
5. If `witchflag` is set, `witch_fx(fp_drawing)` is called to draw the beam — fmain.c:2373
6. Hit test: if `s1 > 0 && s2 < 0 && calc_dist(2,0) < 100`, then `dohit(-1,0,anim_list[2].facing,rand2()+1)` — fmain.c:2374-2375

**Erasure (next frame)** — fmain.c:1978:
- Before drawing new frame, previous witch effect is undone: `if (fp_drawing->wflag) witch_fx(fp_drawing)` — this works because the beam is drawn in COMPLEMENT (XOR) mode, so drawing it again erases it.

**witch_fx() algorithm** — fmain2.c:917-965:
1. Calculate hero screen position: `xh = hero_x - (map_x & 0xfff0)`, `yh = hero_y - (map_y & 0xffe0)` — fmain2.c:924-925
2. Create a temporary Amiga Layer (LAYERSIMPLE) over the map bitmap for clipped drawing — fmain2.c:927-928
3. Look up two edges of the beam wedge from `witchpoints[]`:
   - Edge 1: index `(witchdir+63)*4` — this is the trailing edge (63 positions behind = ~354° around = -1 step from current) — fmain2.c:929-931
   - Edge 2: index `(witchdir+1)*4` — this is the leading edge (1 position ahead) — fmain2.c:932-934
   - Each entry provides 4 bytes: `[x_far, y_far, x_near, y_near]` — offsets from witch center
   - Points (x1,y1) and (x2,y2) define edge 1; points (x3,y3) and (x4,y4) define edge 2
4. **Cross-product hit test** — determines if hero is inside the beam wedge:
   - Edge 1 cross product: `s1 = dx*(yh-y1) - dy*(xh-x1)` where dx=x2-x1, dy=y2-y1 — fmain2.c:936-939
   - `sg1 = (s1 >= 0) ? 1 : 0` — fmain2.c:939
   - Edge 2 cross product: `s2 = dx*(yh-x3) - dy*(xh-x3)` where dx=x4-x3, dy=y4-y3 — fmain2.c:941-944
   - `sg2 = (s2 >= 0) ? 1 : 0` — fmain2.c:944
   - Hero is inside beam when `s1 > 0 && s2 < 0` (tested at call site, fmain.c:2374)
5. **Draw the beam polygon** using Amiga area-fill in COMPLEMENT mode:
   - `SetDrMd(r, COMPLEMENT)` — fmain2.c:946 — XOR drawing mode
   - Allocate temporary raster and area info — fmain2.c:947-953
   - Draw filled quadrilateral: `AreaMove(r,x1,y1)`, `AreaDraw(r,x2,y2)`, `AreaDraw(r,x4,y4)`, `AreaDraw(r,x3,y3)`, `AreaEnd(r)` — fmain2.c:954-958
   - Clean up resources — fmain2.c:959-960
6. Delete the temporary layer — fmain2.c:962-963

**Beam rotation**: `witchdir` is a UBYTE index (0-255) into the 64-entry table. Since each entry is 4 bytes and the index is multiplied by 4, the effective range is `(witchdir+offset)*4` which wraps at 256 bytes = 64 entries. The table covers a full 360° circle in 64 steps (5.625° per step). The `+63` and `+1` offsets create a wedge spanning 2 entries (~11.25° arc).

**Beam visual**: The filled polygon creates a wedge-shaped beam emanating from the witch's position. In COMPLEMENT (XOR) mode, it inverts all bitplane colors within the polygon, creating a flickering/flashing visual effect. Because the beam is drawn and erased via XOR on alternating frames, it appears as a rapidly-rotating swept beam pattern.

### 2. witchpoints[] Table — fmain2.c:889-916

64 entries × 4 bytes = 256 bytes. Each entry has format: `[x_far, y_far, x_near, y_near]`.

The table encodes points on two concentric circles centered on the witch:
- **Far circle**: radius ~100 pixels (values range from -100 to +100)
- **Near circle**: radius ~10 pixels (values range from -10 to +10)

The table traces a full 360° circle in 64 steps:
- Entry 0: `0,100,0,10` — straight down (0°)
- Entry 8: `70,70,7,7` — 45° (southeast)
- Entry 16: `100,0,10,0` — 90° (east/right)
- Entry 24: `70,-71,7,-8` — 135°
- Entry 32: `0,-100,0,-10` — 180° (straight up)
- Entry 40: `-71,-71,-8,-8` — 225°
- Entry 48: `-100,-1,-10,-1` — 270° (west/left)
- Entry 56: `-39,92,-4,9` — 315°

Values are `round(100 * sin(angle))` and `round(100 * cos(angle))` for the far circle, `round(10 * sin(angle))` and `round(10 * cos(angle))` for the near circle. Stored as BYTE (-128 to +127).

The polygon vertices form a trapezoid-like wedge from the near circle to the far circle, creating a tapered beam emanating from the witch's center outward.

### 3. colorplay() — Teleport Effect

**C version** — fmain2.c:425-431:
```
colorplay()
{   register long i,j;
    for (j=0; j<32; j++)           /* 32 iterations */
    {   for (i=1; i<32; i++)       /* skip color 0 (background) */
            fader[i]=bitrand(0xfff);  /* random 12-bit Amiga color */
        LoadRGB4(&vp_page,fader,32);  /* apply to page viewport */
        Delay(1);                     /* 1/60th second */
    }
}
```

**Assembly version** — fsupp.asm:1-26 (**NOT LINKED** — fsupp.asm is not assembled or linked by the makefile; only the C version above is compiled into the game):
Functionally identical to C version. Loops 32 times (d5 counter), each time fills `_fader[0..31]` with random 12-bit colors via `_rand`, then calls `_LoadRGB4(&_vp_page, _fader, 32)` and `_Delay(1)`.

**Key difference**: The assembly version calls `_rand` (general random) while C calls `bitrand(0xfff)` (masked random). Both produce random 12-bit color values. The assembly version also randomizes fader[0] (all 32 entries starting from index 0), while the C version starts from index 1, preserving fader[0] (background color). The assembly version also masks with `$0fff` after `_rand`, so the result is equivalent.

**Duration**: 32 frames × 1/60th second delay = ~0.53 seconds total.

**Usage**: Called during teleport events. Each frame sets all 32 palette colors to random values, creating a psychedelic flashing visual effect. The `Delay(1)` ensures it runs at display refresh rate.

### 4. flipscan() — Intro Page Transition

**flip timing tables** — fmain2.c:793-795:
```
flip1[] = { 8,6,5,4,3,2,3,5,13,0,0,  13,5,3,2,3,4,5,6,8,0,0 };  /* column step rate */
flip2[] = { 7,5,4,3,2,1,1,1, 1,0,0,   1,1,1,1,2,3,4,5,7,0,0 };  /* column width */
flip3[] = {12,9,6,3,0,0,0,0, 0,0,0,   0,0,0,0,0,0,3,6,9,0,0 };  /* inter-frame delay */
```
22 entries each, split into two phases of 11 frames.

**Algorithm** — fmain2.c:796-833:
The effect transitions between `pagea` (old image) and `pageb` (new image) with a columnar wipe:

**Phase 1 (i=0..10)** — Right half reveal:
1. Copy right half of `pageb` to screen: `BltBitMap(&pageb,160,0,temp,160,0,135,200,...)` — fmain2.c:806
2. If rate==0, skip column processing (frame 9,10 — the rate is 0, meaning no more columns to process)
3. Otherwise, iterate columns from `wide` to 135 stepping by `rate`, for each column:
   - Call `page_det(scol)` to get the vertical margin `h` for that column position — determines how much of the top/bottom border to skip (rounded frame effect)
   - Blit a `wide`-pixel-wide strip from `pagea` at offset `bcol+scol` to screen at `161+dcol` — fmain2.c:810
4. Fill remaining edge with 1-pixel strip from pagea at x=296 — fmain2.c:812

**Phase 2 (i=11..21)** — Left half reveal:
1. If rate==0, copy left half of `pageb` directly — fmain2.c:817
2. Otherwise, copy left half of `pagea` as base — fmain2.c:819
3. Iterate columns, blitting strips from `pageb` over the base — fmain2.c:822-824
4. Fill edge — fmain2.c:827

Each frame ends with `pagechange()` (double-buffer swap) and an optional `Delay(flip3[i])` — fmain2.c:830-831.

**Visual result**: A columnar "squeeze and expand" transition. The old image is compressed into fewer, spaced-apart columns while the new image fills in the gaps. The varying rates and widths create an accelerating/decelerating effect — starting slow, speeding up in the middle, slowing at the end. The right half transitions first, then the left half.

**page_det() function** — fsubs.asm:1448-1489:
Returns a vertical margin based on horizontal screen position, defining the visible area shape:
- Column 0-10: Uses lookup table `pd10 = {9,9,8,7,6,5,5,5,4,4,4}` — fmain2.c:1490
- Column 11-70: returns 3
- Column 71-97: returns 4
- Column 98-122: returns 5
- Column 123-134: returns 6
- Column 135: returns 7
- Column 136+: returns 10

This creates rounded corners on the visible frame — larger margins at the edges, smaller in the center. The `h` value is used as both top margin and `200-h-h` for height, creating symmetric top/bottom borders.

### 5. copypage() — Intro Page Setup

**Function** — fmain2.c:781-791:
```
copypage(br1,br2,x,y) char *br1, *br2; short x,y;
{   if (skipp) return;                                    /* skip if user pressed space */
    Delay(350);                                           /* ~5.8 second pause */
    BltBitMap(&pageb,0,0,&pagea,0,0,320,200,0xC0,0x1f,0); /* copy pageb → pagea (save current) */
    unpackbrush(br1,&pageb,4,24);                         /* load background brush into pageb */
    unpackbrush(br2,&pageb,x,y);                          /* overlay second brush at (x,y) */
    if (skipint()) return;                                 /* check for skip */
    flipscan();                                            /* perform visual transition */
    skipint();                                             /* check for skip again */
}
```

**Usage in intro** — fmain.c:1203-1206:
```
copypage("p1a","p1b",21,29);
copypage("p2a","p2b",20,29);
copypage("p3a","p3b",20,33);
```
Three intro story pages, each with a background brush and overlay brush. The `Delay(350)` gives ~5.8 seconds reading time between pages.

**skipint()** — fmain2.c:836 (fsupp.asm:36-43 contains an unused assembly version — **NOT LINKED**):
```
skipint()
{   return skipp = (getkey()==' '); }
```
Checks if space bar was pressed; if so, sets `skipp` flag to skip remaining intro pages.

### 6. Screen Transitions — fade_page / fade_down / fade_normal

**fade_page(r,g,b,limit,colors)** — fmain2.c:377-420:
Core palette manipulation function. Takes percentage values (0-100) for red, green, blue channels, a `limit` flag, and a source color table.

**Algorithm**:
1. Set color 31 based on region: region 4 → `0x0980` (amber), region 9 → `0x00f0` (green, if secret_timer) or `0x0445` (dark), else `0x0bdf` (light blue) — fmain2.c:381-386
2. Clamp r,g,b to 0-100 range — fmain2.c:388-390
3. If `limit` (night mode): enforce minimums r≥10, g≥25, b≥60; compute green compensation `g2=(100-g)/3` — fmain2.c:391-394. This prevents total darkness at night — blue channel stays brightest.
4. If not limit: allow 0 minimum; `g2=0` — fmain2.c:396-400
5. For each of 32 colors:
   - Extract R, G, B nibbles from source `colors[i]` (4-bit Amiga format) — fmain2.c:403-405
   - If `light_timer` active and R < G, boost R to G — fmain2.c:406 — creates a warmer/redder palette (torch/spell effect)
   - Scale: `r1 = (r * r1) / 1600`, `g1 = (g * g1) / 1600`, `b1 = (b * b1 + g2*g1) / 100` — fmain2.c:407-409
   - If limit and color index 16-24 and g>20: blue boost (+2 if g<50, +1 if g<75) — fmain2.c:410-412. These are the sky/water colors, kept slightly bluer at dusk/dawn.
   - Clamp b1 to 15 max — fmain2.c:413
   - Pack into fader: `fader[i] = (r1<<8) + (g1<<4) + b1` — fmain2.c:415
6. Apply: `LoadRGB4(&vp_page, fader, 32)` — fmain2.c:417

**fade_down()** — fmain2.c:623-625:
```
for (i=100; i>=0; i-=5) { fade_page(i,i,i,FALSE,pagecolors); Delay(1); }
```
21 steps from 100% to 0% brightness, no night limits. ~0.35 seconds. Used before screen transitions (map_message, message_off, death).

**fade_normal()** — fmain2.c:627-629:
```
for (i=0; i<=100; i+=5) { fade_page(i,i,i,FALSE,pagecolors); Delay(1); }
```
21 steps from 0% to 100% brightness. ~0.35 seconds. Used after screen transitions to fade back in.

### 7. screen_size() — Viewport Zoom Animation

**Function** — fmain.c:2914-2933:
```
screen_size(x) register long x;
{   register long y;
    y = (x*5)/8;                              /* maintain 5:8 aspect ratio */
    Delay(2);                                  /* ~33ms per step */
    ri_page2.RxOffset = ri_page1.RxOffset = vp_page.DxOffset = 160-x;
    vp_page.DWidth = x+x;                     /* viewport width = 2x */
    ri_page2.RyOffset = ri_page1.RyOffset = vp_page.DyOffset = 100-y;
    vp_page.DHeight = y+y;                     /* viewport height = 2y */
    vp_text.DxOffset = vp_text.DyOffset = 0;
    vp_text.DHeight = 95-y;                    /* text viewport shrinks as page grows */
    fade_page(y*2-40,y*2-70,y*2-100,0,introcolors);  /* sync palette with zoom */
    MakeVPort(&v, &vp_text);
    pagechange();
}
```

**Parameters**: `x` = half-width of viewport (0 to 160). At x=0 the viewport is invisible; at x=160 it fills the full 320-pixel width.

**Aspect ratio**: y = x × 5/8, so at x=160, y=100, giving full 320×200. At x=156, y=97, giving 312×194.

**Fade sync**: `fade_page(y*2-40, y*2-70, y*2-100, 0, introcolors)` — colors brighten as viewport expands. At full size (y=100): r=160, g=130, b=100 (overcapped at 100). At y=50: r=60, g=30, b=0.

**Usage**:
- Intro zoom-in: `for (i=0; i<=160; i+=4) screen_size(i);` — 41 steps — fmain.c:1199
- Intro zoom-out: `for (i=156; i>=0; i-=4) screen_size(i);` — 40 steps — fmain.c:1209
- Initial setup: `screen_size(156)` — set to gameplay size — fmain.c:1153
- After intro: `screen_size(156)` — restore gameplay size — fmain.c:1220
- Load game: `screen_size(156)` — fmain2.c:1613
- Win sequence: `screen_size(156)` — fmain2.c:1613 in win_colors

Note: Gameplay uses x=156 (not 160), giving viewport 312×194 — slightly inset from full screen.

### 8. Additional Visual Effects

#### stillscreen() — fmain2.c:631-634 (fsupp.asm:27-34 is an unused assembly version — **NOT LINKED**)
Resets scroll offsets to (0,0) and calls `pagechange()`. Used to display static full-screen content (maps, inventory screens). The assembly version does the same: clears `RxOffset`/`RyOffset` on `fp_drawing->ri_page` then calls `_pagechange`.

#### map_message() — fmain2.c:601-611
Full-screen text display mode:
1. `fade_down()` — fade to black
2. Clear bitmap to 0 (`SetRast`)
3. Set text colors (`SetDrMd(rp,JAM1)`, `SetAPen(rp,24)`)
4. Hide text viewport (`VP_HIDE`)
5. `stillscreen()` / `LoadRGB4(&vp_page,pagecolors,32)` — show the page with normal colors
6. Set `viewstatus = 2`

#### message_off() — fmain2.c:613-620
Return from full-screen text:
1. `fade_down()` — fade to black
2. Restore text viewport (`HIRES | SPRITES`, no VP_HIDE)
3. `pagechange()`
4. Set `viewstatus = 3` (triggers `fade_normal()` in main loop at fmain.c:2614)

#### flasher-based border blink — fmain.c:1368-1370
During viewstatus 1 (text/dialogue mode), color register 31 blinks:
```
if ((flasher & 16) && viewstatus == 1)
    SetRGB4(&vp_page,31,15,15,15);   /* white */
else SetRGB4(&vp_page,31,0,0,0);     /* black */
```
`flasher` increments every main loop tick (fmain.c:1276), so bit 4 toggles every 16 frames (~0.27 seconds), creating a slow blink effect on any pixels using color 31. Color 31 is the text border/highlight color — this creates a blinking prompt/cursor effect during dialogue.

#### light_timer visual effect — fmain2.c:407 in fade_page
When `light_timer > 0`, the fade_page function boosts red channel to match green for any color where R < G. This creates a warm orange/red color shift across the entire palette. `light_timer` is set by magic item 6 (`light_timer += 760` at fmain.c:3306) and decrements each frame (fmain.c:1380). This creates a ~12.7 second torch/light spell effect. It also affects day_fade: when active, `ll` is set to 200 (overriding lightlevel) at fmain2.c:1655.

#### win_colors() — Victory Sunrise Animation — fmain2.c:1605-1636
1. Display victory text via `placard_text(6)`, hero `name()`, `placard_text(7)`, `placard()` — fmain2.c:1606
2. Load "winpic" brush into drawing bitmap — fmain2.c:1608
3. Set all colors to black (both viewports) — fmain2.c:1609-1610
4. Hide text viewport, set full screen size — fmain2.c:1611-1612
5. Animated sunrise over 55 frames (i from 25 down to -29):
   - Colors 0 and 31 always black; colors 1 and 28 always white — fmain2.c:1614-1615
   - Colors 2-27 fade in from `sun_colors[]` table — a 53-entry gradient from black through deep blue to warm sunset/gold — fmain2.c:1616-1619
   - Colors 29-30 are red tones that shift: start at `0x800`/`0x400`, later computed as `0x100*(i+30)/2` and half that — fmain2.c:1620-1627
   - First frame holds for 60 ticks (~1 second) before animation begins — fmain2.c:1629
   - Each subsequent frame: `Delay(9)` (~150ms) — fmain2.c:1630
6. Hold 30 ticks, then fade to full black — fmain2.c:1632-1633

#### pagechange() — Double Buffer Swap — fmain.c:2993-3006
Swaps `fp_drawing` and `fp_viewing`, rebuilds copper list, loads new view, waits for vertical blank. This is the display refresh that makes all visual changes visible.

## Cross-Cutting Findings

- **Witch beam as XOR rendering**: The use of COMPLEMENT (XOR) drawing mode means the witch beam requires no separate erase step — just re-drawing it undoes it. This is why `witch_fx()` is called both to draw (fmain.c:2373) and to erase (fmain.c:1978) the beam, using the same code path.

- **s1/s2 cross-product globals used outside witch_fx**: The variables `s1` and `s2` are declared as `SHORT` globals (fmain.c:593 / fmain2.c:906) and computed inside `witch_fx()` but tested at the call site in fmain.c:2374 for hit detection. This is a side-effect coupling — `witch_fx()` both renders the visual AND computes the hit test as a side effect via globals.

- **witchindex is UBYTE**: Declared as `UBYTE witchindex` (fmain.c:597). Incrementing/decrementing it naturally wraps 0-255, which maps to 64 table entries (256/4=64). This gives seamless continuous rotation without explicit modulo.

- **Beam steering hunts the hero**: The `wdir` adjustment at fmain.c:2369 uses `s1`'s sign to steer. `s1` is the cross-product from the trailing edge — when positive, the hero is to the "right" of the beam, so `wdir=1` (rotate clockwise); when negative, `wdir=-1` (rotate counter-clockwise). The `rand4()==0` gate means steering only changes direction 1 in 4 frames, creating a slow seeking behavior.

- **Witch damage via dohit(-1,...)**: When beam encloses hero (`s1>0 && s2<0`) and distance < 100 pixels, calls `dohit(-1, 0, anim_list[2].facing, rand2()+1)` — fmain.c:2375. The `-1` first argument triggers `effect(2, 500+rand64())` — sound effect 2 (magical hit) at fmain2.c:238. Damage is `rand2()+1` = 1 or 2 HP per hit.

- **colorplay preserves background**: The C version (the only linked version) starts loop at `i=1` (fmain2.c:428), keeping `fader[0]` (background color) unchanged. The unused assembly version in fsupp.asm randomizes all 32 entries including index 0, but fsupp.asm is not assembled or linked by the makefile.

- **screen_size couples viewport geometry with palette**: The `fade_page()` call inside `screen_size()` means visual zoom and color fade are inseparable — you can't zoom without also changing colors, and the color formula is hardcoded for the intro palette.

- **light_timer is a visual-only effect with gameplay implications**: While it's primarily a palette tint (warm light), it also affects the day_fade function's behavior by overriding lightlevel to 200 (full brightness equivalent), making it temporarily always daytime-bright. This means the torch/light spell both changes the color tone AND prevents night darkness — these two effects are coupled in a non-obvious way.

## Unresolved

None — all questions answered with direct source citations.

## Refinement Log
- 2026-04-06: Initial discovery pass. Traced all 10 requested visual effects with complete source citations. Read witch_fx, witchpoints, colorplay (C and asm), flipscan, copypage, fade_page/fade_down/fade_normal, screen_size, and identified additional visual effects (map_message, win_colors, flasher blink, light_timer tint, stillscreen).
