# Discovery: screen_size() Function

**Status**: complete
**Investigated**: 2026-04-19
**Requested by**: orchestrator
**Prompt summary**: Trace the `screen_size()` function — its implementation, all call sites, argument meanings, related variables, and Amiga display geometry.

## Implementation

**Definition**: `fmain.c:2914-2933`

```c
screen_size(x) register long x;
{   register long y;

    y = (x*5)/8;

    Delay(2);

    ri_page2.RxOffset = ri_page1.RxOffset = vp_page.DxOffset = 160-x;
    vp_page.DWidth = x+x;

    ri_page2.RyOffset = ri_page1.RyOffset = vp_page.DyOffset = 100-y;
    vp_page.DHeight = y+y;

    vp_text.DxOffset = vp_text.DyOffset = 0;
    vp_text.DHeight = 95-y;

    fade_page(y*2-40,y*2-70,y*2-100,0,introcolors);

    MakeVPort( &v, &vp_text );
    pagechange();
}
```

**Prototype**: `fmain.p:14` — `int screen_size(long x);`

### Mechanical Behavior

Given argument `x` (a half-width value in lo-res pixels):
1. Computes `y = (x*5)/8` — maintains a 5:8 height-to-width aspect ratio
2. Calls `Delay(2)` — 2-tick (40ms) pause for animation pacing
3. Sets viewport horizontal geometry:
   - `RxOffset` (both pages) = `160 - x` — scroll offset into the bitmap
   - `DxOffset` (viewport display) = `160 - x` — screen position
   - `DWidth` = `x * 2` — visible width in pixels
4. Sets viewport vertical geometry:
   - `RyOffset` (both pages) = `100 - y` — scroll offset into the bitmap
   - `DyOffset` (viewport display) = `100 - y` — screen position
   - `DHeight` = `y * 2` — visible height in pixels
5. Adjusts text viewport:
   - `DxOffset` = `DyOffset` = 0
   - `DHeight` = `95 - y` — text area shrinks as playfield grows
6. Calls `fade_page()` to animate colors from `introcolors` palette during zoom
7. Calls `MakeVPort()` to rebuild the text viewport copper list
8. Calls `pagechange()` to swap display buffers and rebuild the page viewport copper list

The function acts as an **animated viewport resize** — when called in a loop, it produces a smooth zoom-in or zoom-out effect centered on the display.

## Complete Call Sites

| File:Line | Argument | Context | Before/After |
|-----------|----------|---------|--------------|
| `fmain.c:1153` | `156` | Initial setup (pre-intro), display legal text | After clearing both page bitmaps, setting `rp = &rp_map`. Before `SetRGB4` color setup and title text display. |
| `fmain.c:1187` | `0` | Reset to zero before intro animation | After setting `fp_page2.ri_page = &ri_page1`. Before `pagechange()` × 2 ("prime the pump"). Collapses viewport to invisible point. |
| `fmain.c:1199` | `0..160` (loop, step 4) | Intro zoom-IN animation | `for (i=0; i<=160; i+=4) screen_size(i);` — 41 iterations zooming from nothing to full-screen. After loading page0 brush into both bitmaps. Before copypage slideshow. |
| `fmain.c:1209` | `156..0` (loop, step -4) | Intro zoom-OUT animation | `for (i=156; i>=0; i-=4) screen_size(i);` — 40 iterations zooming from gameplay size to nothing. After slideshow (or skip). Before entering normal gameplay mode. |
| `fmain.c:1220` | `156` | Pre-gameplay setup (asset loading, copy protection) | After clearing both page bitmaps and loading `blackcolors`. Before loading shadow data, hiscreen, copy-protection screen, and `revive()`. This viewport remains active through the copy-protection quiz at `fmain.c:1233-1238`. |
| `fmain2.c:1613` | `156` | Win sequence (`win_colors()`) | After loading "winpic" brush and setting black colors, hiding text viewport. Before the sunrise color animation loop. |

## Computed Dimensions by Argument Value

| Argument (x) | Width (x×2) | Height (y×2) | DxOffset | DyOffset | vp_text.DHeight | Use |
|---|---|---|---|---|---|---|
| `0` | 0 px | 0 px | 160 | 100 | 95 | Invisible (collapsed) |
| `4` | 8 px | 4 px | 156 | 98 | 93 | First zoom step |
| `80` | 160 px | 100 px | 80 | 50 | 45 | Mid-zoom |
| `136` | 272 px | 170 px | 24 | 15 | 10 | — |
| `156` | 312 px | 194 px | 4 | 3 | -3 (≤0, hidden) | **Normal gameplay** |
| `160` | 320 px | 200 px | 0 | 0 | -5 (≤0, hidden) | Full-screen (intro max) |

Key observations:
- **156** is the standard gameplay size: 312×194 viewport, slightly inset (4px left/right, 3px top/bottom)
- **160** is the absolute maximum reached during the intro zoom-in but NOT used for gameplay
- At **156** and **160**, `vp_text.DHeight` goes negative, effectively hiding the text viewport
- The text viewport lives below `PAGE_HEIGHT` (143) — as `y` approaches 95, the text area disappears

## Related Variables and Constants

### Display Constants (`fmain.c:8-16`)
- `PAGE_DEPTH` = 5 — 5 bitplanes (32 colors)
- `SCREEN_WIDTH` = 288 — lo-res display width (36 bytes × 8)
- `PHANTA_WIDTH` = 320 — full bitmap width (includes scroll margin)
- `PAGE_HEIGHT` = 143 — Y position where text viewport begins
- `RAST_HEIGHT` = 200 — total bitmap height
- `TEXT_HEIGHT` = 57 — text viewport height

### Viewport Structs (`fmain.c:19`)
- `struct ViewPort vp_page` — lo-res game playfield viewport
- `struct ViewPort vp_text` — hi-res text/status viewport
- `struct ViewPort vp_title` — (unused in screen_size)

### RasInfo Structs (`fmain.c:445`)
- `struct RasInfo ri_page1, ri_page2` — dual-buffered page offsets
- Both `RxOffset` and `RyOffset` are set by `screen_size()` to shift the bitmap view

### Initial ViewPort Setup (`fmain.c:808-816`)
```c
vp_page.DWidth = 288;          /* lo-res screen is 36 bytes wide */
vp_text.DWidth = 640;          /* hi-res screen */
vp_page.DxOffset = 16;
vp_page.DyOffset = vp_text.DxOffset = 0;
vp_page.DHeight = 140;
vp_text.DyOffset = PAGE_HEIGHT; /* 143 */
vp_text.DHeight = TEXT_HEIGHT;  /* 57 */
```

### Normal Gameplay Restore (`fmain.c:1250-1255`)
After entering the game loop (after brother revive/load), the viewport is reset to the non-animated "standard" mode:
```c
vp_text.DyOffset = PAGE_HEIGHT;     /* 143 */
vp_text.DHeight = TEXT_HEIGHT;      /* 57 */
vp_page.DHeight = 140;
vp_page.DxOffset = 16;
vp_page.DWidth = 288;
vp_page.DyOffset = 0;
```
This is the "normal gameplay with status bar" configuration — DIFFERENT from what `screen_size(156)` produces.

### Color Palettes
- `introcolors[]` (`fmain.c:484-487`) — 32-entry palette used exclusively by `screen_size()` via `fade_page()` during zoom animations

## Geometry Relationship: screen_size vs. Normal Gameplay

There are **two distinct screen configurations**:

1. **screen_size(156)** — Used for intro/win sequences and the title display:
   - 312×194 px playfield
   - Text viewport hidden (negative height)
   - No status bar visible
   - Viewport centered with 4px horizontal / 3px vertical inset

2. **Standard gameplay** (set at `fmain.c:1250-1255`):
   - 288×140 px playfield
   - Text viewport visible: 640×57 px at Y=143 (hi-res, 4 bitplanes)
   - Status bar / narration visible below playfield
   - 16px horizontal offset (centering the 288px in 320px frame)

The `screen_size(156)` call at `fmain.c:1220` sets up the full-screen mode temporarily while loading assets and the copy-protection screen. After `revive()` completes, the code at `fmain.c:1250-1255` restores the standard split-screen configuration with text viewport.

## Copy Protection Screen Viewport

The copy protection quiz (`fmain.c:1233-1238`) displays under the same viewport set by `screen_size(156)` at `fmain.c:1220`. No viewport change occurs between that call and the quiz. The display configuration is:

- **Viewport**: Config A — 312×194 visible area, 4px horizontal / 3px vertical inset
- **Bitmap**: lo-res playfield, 320×200, 5 bitplanes (32 colors)
- **RastPort**: `rp_map` (assigned at `fmain.c:1231-1232`)
- **Text viewport**: hidden (DHeight ≤ 0)
- **Palette**: colors 0 and 1 set explicitly — background dark blue `(0,0,6)`, text white `(15,15,15)` (`fmain.c:1221,1229`)

### Rendering sequence (`fmain.c:1231-1238`)

```c
rp = &rp_map;
rp_map.BitMap = fp_drawing->ri_page->BitMap;
stillscreen();                    // zero scroll offsets, pagechange
SetAPen(rp,1);                   // pen = white
placard_text(19);                 // msg12 via ssp() — quiz intro text
handler_data.laydown = handler_data.pickup = 0;
k = TRUE;
if (copy_protect_junk()==0) goto quit_all;
```

### Text coordinates (lo-res, via `ssp()` XY command)

`placard_text(19)` displays msg12 (`narr.asm:341-347`) using `ssp()`:

| X stored | X displayed (×2) | Y | Text |
|----------|-------------------|---|------|
| 64 (128/2) | 128 | 19 | "So..." |
| 17 (34/2) | 34 | 65 | "You, game seeker, would guide the" |
| 5 (10/2) | 10 | 75 | "brothers to their destiny? You would" |
| 5 (10/2) | 10 | 85 | "aid them and give directions? Answer," |
| 5 (10/2) | 10 | 95 | "then, these three questions and prove" |
| 5 (10/2) | 10 | 105 | "your fitness to be their advisor:" |

### Answer input coordinates (`copy_protect_junk()` at `fmain2.c:1309-1336`)

Each question+answer line is positioned at:
- Question text: `move(10, 125 + (h * 10))` where h = 0,1,2 — so Y = 125, 135, 145
- Answer cursor: starts at `rp->cp_x` / `rp->cp_y` (immediately after the question text)

These are direct pixel coordinates in the lo-res 320×200 bitmap — NOT doubled. The `move()` wrapper calls `GfxBase->Move(rp, x, y)` directly without any coordinate transformation (`fsubs.asm:477-485`).

### Key point for implementation

The copy protection screen is **entirely lo-res 320×200** (visible as 312×194 through the `screen_size(156)` inset). Using 640×200 would produce a half-filled screen since all text coordinates assume 320-pixel width.

## Cross-Cutting Findings

- `fmain2.c:1613` — `screen_size()` called in `win_colors()`, the game's ending sequence. This means the win screen uses the same full-screen viewport as the intro, bypassing the normal status bar layout.
- `fmain.c:2930` — `fade_page()` is called with `introcolors` every time `screen_size()` fires. This means the intro palette flickers in during any call to `screen_size()` — it's not just a zoom, it's a color-animated zoom specifically tied to the intro/outro palette. Using `screen_size()` outside the intro context (e.g., in `win_colors()`) still fades through `introcolors`, which suggests the win sequence intentionally uses the same aesthetic.

## Unresolved

None — all questions answered with direct code citations.

## Refinement Log
- 2026-06-06: Added Copy Protection Screen Viewport section with coordinate tables, rendering sequence, and implementation note.
- 2026-04-19: Initial complete discovery pass
