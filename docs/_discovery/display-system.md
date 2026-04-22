# Discovery: Double Buffering & Display System

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace the full display architecture including struct fpage, double buffering, pagechange, MakeBitMap, screen_size, display geometry, copper lists, stillscreen, and Amiga chipset interaction.

## 1. struct fpage

Defined at `ftale.h:72-82`:

```c
struct fpage {
    struct RasInfo *ri_page;        /* pointer to RasInfo for this page's bitmap */
    struct cprlist *savecop;        /* saved copper list for this page */
    long isv_x, isv_y;             /* last image scroll position (img_x, img_y when page was drawn) */
    short obcount;                  /* number of sprite shapes queued on this page */
    struct sshape *shape_queue;     /* array of shape descriptors for blitter restore */
    unsigned char *backsave;        /* background save buffer for sprite underdraw */
    long saveused;                  /* bytes used in backsave so far */
    short witchx, witchy, witchdir, wflag;  /* witch effect: position, direction index, active flag */
};
```

### Field Details

| Field | Type | Purpose | Citation |
|-------|------|---------|----------|
| `ri_page` | `struct RasInfo *` | Points to the RasInfo that links this page's BitMap. Page 1 → `&ri_page1`, Page 2 → `&ri_page2` (normally). | ftale.h:73, fmain.c:850-851 |
| `savecop` | `struct cprlist *` | Stores the compiled copper list (LOFCprList) for this page. Swapped in/out during pagechange. | ftale.h:74, fmain.c:2998,3004 |
| `isv_x`, `isv_y` | `long` | The image-space scroll coordinates when this page was last drawn. Used to compute `dif_x` / `dif_y` for incremental scrolling. | ftale.h:75, fmain.c:1982-1983, 2233-2234 |
| `obcount` | `short` | Count of sprites drawn on this page, used for restore-blit loop. | ftale.h:76, fmain.c:1969, 2378, 2535 |
| `shape_queue` | `struct sshape *` | Array of `MAXSHAPES` (25) sshape descriptors. Each records the blitter parameters needed to restore the background under a drawn sprite. | ftale.h:77, fmain.c:882-884, 1970 |
| `backsave` | `unsigned char *` | Buffer for background pixel data saved before sprites are drawn. Used by `save_blit`/`rest_blit`. | ftale.h:78, fmain.c:879-880, 2379 |
| `saveused` | `long` | Running total of bytes consumed in `backsave`. Capped at `74*80 = 5920` bytes. | ftale.h:79, fmain.c:2378, 2549 |
| `witchx`, `witchy` | `short` | Screen-relative position of the witch for the witch-effect polygon. | ftale.h:80, fmain.c:2367-2368 |
| `witchdir` | `short` | Index into `witchpoints` table for the rotating witch polygon. | ftale.h:80, fmain.c:2370 |
| `wflag` | `short` | Whether witch effect is active on this page, set from global `witchflag`. | ftale.h:80, fmain.c:2371, 1978 |

### struct sshape

Defined at `fmain.c:436-440`:

```c
struct sshape {
    unsigned char *backsave;  /* pointer into fpage.backsave where background was saved */
    short savesize;           /* size of one plane's saved data in bytes */
    short blitsize;           /* BLTSIZE register value: (height<<6)+width_words */
    short Coff;               /* byte offset into bitplane for destination (xbw*2 + ystart*40) */
    short Cmod;               /* blitter C/D modulus (40 - blitwide*2) */
};
```

## 2. Double Buffering

### Overview

The game uses classic Amiga double buffering with two full playfield pages. While one page is displayed (viewed), the other is drawn into. After drawing completes, the pages swap via `pagechange()`.

### Data Structures

Two `fpage` instances and two pointer variables — `fmain.c:443`:

```c
struct fpage fp_page1, fp_page2, *fp_drawing, *fp_viewing;
```

Two `RasInfo` structures — `fmain.c:445`:

```c
struct RasInfo ri_page1, ri_page2, ri_text, ri_title;
```

Two `BitMap` pointers — `fmain.c:446`:

```c
struct BitMap *bm_page1, *bm_page2, *bm_text, *bm_lim, *bm_draw, *bm_source;
```

### Initialization

At `fmain.c:850-852`:

```c
fp_page1.ri_page = &ri_page1;        /* page1 → ri_page1 → bm_page1 */
fp_page2.ri_page = &ri_page1;        /* initially BOTH point to ri_page1 */
fp_drawing = &fp_page1; fp_viewing = &fp_page2;
```

Note: `fp_page2.ri_page` is initially set to `&ri_page1` (same as page1). It is later changed to `&ri_page2` at `fmain.c:1149` and `fmain.c:1186` when gameplay begins. During the intro zoom animation, it is temporarily set to `&ri_page1` so both pages show the same bitmap (`fmain.c:1198`), then restored to `&ri_page2` (`fmain.c:1200`).

The RasInfo → BitMap linkage is at `fmain.c:841-842`:

```c
ri_page1.BitMap = bm_page1;
ri_page2.BitMap = bm_page2;
```

### Per-Frame Cycle

The main loop (starting at `fmain.c:1265`) follows this pattern each frame:

1. **Restore backgrounds**: Iterate `fp_drawing->obcount` sprites in reverse, call `rest_blit()` to put saved background data back (`fmain.c:1969-1972`).
2. **Undo witch effect**: If `fp_drawing->wflag`, call `witch_fx()` (XOR polygon) to remove it (`fmain.c:1978`).
3. **Scroll/redraw map**: Compute `dif_x`, `dif_y` from `img_x/img_y` vs. `fp_drawing->isv_x/isv_y`. Call `scrollmap()` or `map_draw()` as needed (`fmain.c:1982-2232`).
4. **Save new isv**: `fp_drawing->isv_x = img_x; fp_drawing->isv_y = img_y;` (`fmain.c:2233-2234`).
5. **Apply witch effect**: Set witch parameters on `fp_drawing`, call `witch_fx()` if active (`fmain.c:2367-2375`).
6. **Reset shape queue**: `fp_drawing->obcount = crack = fp_drawing->saveused = 0;` (`fmain.c:2378`).
7. **Draw sprites**: For each actor, compute clip bounds, call `save_blit()` → `mask_blit()` → `shape_blit()`, increment `fp_drawing->obcount` (`fmain.c:2380-2607`).
8. **Set scroll offsets**: `fp_drawing->ri_page->RxOffset = (map_x & 15); fp_drawing->ri_page->RyOffset = (map_y & 31);` (`fmain.c:2611-2612`).
9. **Flip**: Call `pagechange()` (`fmain.c:2613`).

## 3. pagechange()

Defined at `fmain.c:2993-3008`:

```c
pagechange()
{   register struct fpage *temp;

    temp = fp_drawing; fp_drawing = fp_viewing; fp_viewing = temp;
    vp_page.RasInfo = temp->ri_page;
    v.LOFCprList = temp->savecop;

    MakeVPort( &v, &vp_page );
    MrgCop( &v );
    LoadView(&v);
    temp->savecop = v.LOFCprList;

    WaitBOVP(&vp_text);
}
```

### Step-by-step

1. **Swap pointers**: `fp_drawing` ↔ `fp_viewing`. After swap, `temp` points to the page that was just drawn (now being viewed). (`fmain.c:2995`)
2. **Set RasInfo**: `vp_page.RasInfo = temp->ri_page` — links the playfield viewport to the newly-drawn page's RasInfo (which has the correct BitMap and RxOffset/RyOffset). (`fmain.c:2997`)
3. **Restore copper list**: `v.LOFCprList = temp->savecop` — restores the previously compiled copper list for this page. (`fmain.c:2998`)
4. **Rebuild copper**: `MakeVPort(&v, &vp_page)` regenerates viewport copper instructions; `MrgCop(&v)` merges all viewport copper lists into the View's master list. (`fmain.c:3000-3001`)
5. **Display**: `LoadView(&v)` tells the Amiga OS to install the new copper list on the next vertical blank. (`fmain.c:3002`)
6. **Save copper list**: `temp->savecop = v.LOFCprList` — saves the newly compiled copper list back to the fpage so it can be restored next time. (`fmain.c:3004`)
7. **Sync**: `WaitBOVP(&vp_text)` waits for the beam to pass the bottom of `vp_text` (the status bar), ensuring the display has fully switched before the game continues drawing. (`fmain.c:3005`)

### Why save/restore copper lists?

Each page has different bitplane pointers (different bitmap). The copper list contains the hardware register values for bitplane pointers. Rather than recompiling both pages' copper lists every frame, only the newly-visible page's copper list is rebuilt and saved. This is a common Amiga double-buffering optimization.

## 4. MakeBitMap / UnMakeBitMap

Defined in `MakeBitMap.asm` (full file). Both assembly and C versions exist in the same file.

### MakeBitMap(bm, depth, width, height)

Assembly version (`MakeBitMap.asm:10-49`):

1. Validates `bm` pointer is non-NULL.
2. Calls `InitBitMap(bm, depth, width, height)` via GfxBase.
3. Clears all plane pointers to NULL.
4. Allocates each bitplane with `AllocRaster(width, height)`.
5. If any allocation fails, calls `UnMakeBitMap(bm)` and returns 0 (FALSE).
6. Returns 1 (TRUE) on success.

C version (`MakeBitMap.asm:80-92`) — identical logic as a fallback:

```c
MakeBitMap(b,depth,width,height) {
    if (!b) return FALSE;
    for (i=0; i<depth; i++) b->Planes[i] = NULL;
    InitBitMap(b,depth,width,height);
    for (i=0; i<depth && success; i++)
        if (!(b->Planes[i] = AllocRaster(width,height))) success = FALSE;
    if (!success) UnMakeBitMap(b);
    return success;
}
```

### UnMakeBitMap(bm)

Assembly version (`MakeBitMap.asm:51-74`):

1. Reads `BytesPerRow`, `Rows`, `Depth` from the BitMap struct.
2. Sets `Depth = 0` immediately.
3. For each plane: if non-NULL, calls `FreeRaster(plane, width_pixels, height)` and clears the pointer.

C version (`MakeBitMap.asm:96-105`):

```c
UnMakeBitMap(b) {
    for (i=0; i<b->Depth; i++)
        if (b->Planes[i]) {
            FreeRaster(b->Planes[i], b->BytesPerRow, b->Rows);
            b->Planes[i] = NULL;
        }
        b->Depth = 0;
}
```

Note: The C version has a subtle bug — `b->Depth = 0` is inside the for loop body (no braces), so it gets set to 0 on the first iteration, terminating the loop after freeing only one plane. The assembly version correctly sets depth=0 before the free loop and uses a separate counter. The assembly version is the one actually linked (it's declared with `DECLARE` macro).

### Usage

Called in `open_all()` at `fmain.c:744`:

```c
if (!MakeBitMap(&work_bm,2,640,200)) return 2;
```

This creates `work_bm` — a 2-plane 640×200 bitmap used as `wb_bmap` for the text/status area. Freed in `close_all()` at `fmain.c:998`:

```c
if (TSTFN(AL_BMAP)) UnMakeBitMap(&work_bm);
```

## 5. screen_size()

Defined at `fmain.c:2914-2933`:

```c
screen_size(x) register long x;
{   register long y;
    y = (x*5)/8;                           /* maintain 5:8 aspect ratio */

    Delay(2);                               /* brief pause for visual effect */

    ri_page2.RxOffset = ri_page1.RxOffset = vp_page.DxOffset = 160-x;
    vp_page.DWidth = x+x;                  /* viewport width = 2*x */

    ri_page2.RyOffset = ri_page1.RyOffset = vp_page.DyOffset = 100-y;
    vp_page.DHeight = y+y;                 /* viewport height = 2*y */

    vp_text.DxOffset = vp_text.DyOffset = 0;
    vp_text.DHeight = 95-y;                /* text area shrinks as playfield grows */

    fade_page(y*2-40,y*2-70,y*2-100,0,introcolors);  /* fade colors in sync */

    MakeVPort( &v, &vp_text );
    pagechange();
}
```

### Purpose

Creates a "zoom" animation effect. Parameter `x` ranges from 0 to ~160. The playfield viewport grows from a point at center (160,100) outward.

### Geometry at key sizes

| x | DxOffset | DWidth | DyOffset | DHeight | Text DHeight |
|---|----------|--------|----------|---------|-------------|
| 0 | 160 | 0 | 100 | 0 | 95 |
| 80 | 80 | 160 | 50 | 100 | 45 |
| 156 | 4 | 312 | 2.5→2 | 195 | -2.5→N/A |
| 160 | 0 | 320 | 0 | 200 | -5→N/A |

### Usage

- Intro zoom-in: `for (i=0; i<=160; i+=4) screen_size(i);` (`fmain.c:1199`)
- Intro zoom-out: `for (i=156; i>=0; i-=4) screen_size(i);` (`fmain.c:1202`)
- Initial position after open_all: `screen_size(156);` (`fmain.c:1155`)

During gameplay, `screen_size()` is NOT called. The viewport dimensions are set directly at `fmain.c:1253-1256`:

```c
vp_text.DyOffset = PAGE_HEIGHT;     /* 143 */
vp_text.DHeight = TEXT_HEIGHT;      /* 57 */
vp_page.DHeight = 140;
vp_page.DxOffset = 16;
vp_page.DWidth = 288;
vp_page.DyOffset = 0;
```

## 6. Display Geometry

### Constants

Defined at `fmain.c:8-16`:

| Constant | Value | Purpose | Citation |
|----------|-------|---------|----------|
| `PAGE_DEPTH` | 5 | Number of bitplanes in playfield | fmain.c:8 |
| `TEXT_DEPTH` | 4 | Number of bitplanes in text area | fmain.c:9 |
| `SCREEN_WIDTH` | 288 | Visible playfield pixel width | fmain.c:11 |
| `PHANTA_WIDTH` | 320 | Full raster width (includes scroll margin) | fmain.c:12 |
| `PAGE_HEIGHT` | 143 | Y offset where text viewport begins | fmain.c:14 |
| `RAST_HEIGHT` | 200 | Full raster height per page | fmain.c:15 |
| `TEXT_HEIGHT` | 57 | Height of the status/text bar | fmain.c:16 |

### Viewport Layout (Gameplay Mode)

Set at `fmain.c:808-818` and `fmain.c:1253-1257`:

```
┌─────────────────────────────────────────┐  scanline 0
│                                         │
│     vp_page (lo-res playfield)          │
│     DxOffset=16, DyOffset=0             │
│     DWidth=288, DHeight=140             │
│     5 bitplanes, 32 colors              │
│     Modes: (none set = LORES)           │
│                                         │
├─────────────────────────────────────────┤  scanline 143 (PAGE_HEIGHT)
│     vp_text (hi-res status bar)         │
│     DxOffset=0, DyOffset=143            │
│     DWidth=640, DHeight=57              │
│     4 bitplanes, 16 colors              │
│     Modes: HIRES | SPRITES | VP_HIDE    │
│                                         │
└─────────────────────────────────────────┘  scanline 200
```

Total visible: 200 scanlines.

### Raster Geometry

- **Playfield bitmaps** (`bm_page1`, `bm_page2`): 5 planes × 320×200 pixels. The visible window is 288×140, but the full 320×200 raster exists for hardware scroll offsets. `RxOffset` = `map_x & 15` (0–15 pixels), `RyOffset` = `map_y & 31` (0–31 pixels). (`fmain.c:826-827, 2611-2612`)
- **Text bitmap** (`bm_text`): 4 planes × 640×57 pixels. Planes 0,1 come from `work_bm` (2-plane 640×200). Planes 2,3 are offset within the same memory: `bm_text->Planes[2] = bm_text->Planes[0] + (TEXT_HEIGHT*80)`. (`fmain.c:828, 871-874`)
- **work_bm**: 2 planes × 640×200, allocated via `MakeBitMap(&work_bm,2,640,200)`. Serves as memory pool for text planes and is assigned as `wb_bmap`. (`fmain.c:744, 748`)
- **pagea, pageb**: 5 planes × 320×200, initialized at `fmain.c:830-831`. Plane pointers are set to offsets within `image_mem` at `fmain.c:1179`: `pagea.Planes[i] = (pageb.Planes[i]=image_mem+(i*8000)) + 40000`. Used as scratch bitmaps for IFF image loading/display. Not used for the main playfield.
- **bm_scroll**: 1 plane × 640×57 (`fmain.c:832`). Plane 0 shares `bm_text->Planes[0]`. Used for text scrolling. (`fmain.c:938`)
- **bm_lim**: 1 plane × 320×200 (`fmain.c:829`). Plane 0 set to `sector_mem` at `fmain.c:1180`. Used for collision/limit checking.
- **bm_source**: 3 planes × 64×24 (`fmain.c:833`). Used for small sprite source operations.

### Memory Reuse

The `queue_mem` and `bmask_mem` buffers are carved from excess memory in `bm_text` planes 2 and 3 (`fmain.c:876-877`):

```c
queue_mem = bm_text->Planes[2] + (TEXT_HEIGHT*80);
bmask_mem = bm_text->Planes[3] + (TEXT_HEIGHT*80);
```

The `backsave` buffers for each page are carved from these:

```c
fp_page1.backsave = queue_mem + 962;     /* after shape_queue array */
fp_page2.backsave = bmask_mem + 962;
```

Shape queues are at the start:

```c
fp_page1.shape_queue = (struct sshape *) queue_mem;
fp_page2.shape_queue = (struct sshape *)(queue_mem + sizeof(struct sshape)*MAXSHAPES);
```

## 7. Copper Lists

The game does NOT directly construct copper lists. It uses the Amiga OS View/ViewPort system which generates copper lists automatically.

### Structure

- `struct View v` — the master View (`fmain.c:18`).
- `v.ViewPort = &vp_text` — the first viewport in the chain (`fmain.c:804`).
- `vp_text.Next = &vp_page` — links to the playfield viewport (`fmain.c:805`).
- `vp_page.Next = NULL` — end of chain (`fmain.c:806`).

The OS copper list builder (`MrgCop`) generates copper instructions that:

1. Set hi-res mode, 4-plane bitplane pointers, and 16-color palette for `vp_text`.
2. At scanline 143, switch to lo-res mode, set 5-plane bitplane pointers, 32-color palette for `vp_page`.
3. Handle the `VP_HIDE` flag on `vp_text` (viewport initially hidden).

### Copper List Caching

Each `fpage` caches its compiled copper list in `savecop`:

- `fp_page1.savecop` and `fp_page2.savecop` initialized to NULL (`fmain.c:850`).
- During `pagechange()`, the current page's copper list is restored from `savecop` before `MakeVPort`/`MrgCop` rebuild it, then saved back (`fmain.c:2998, 3004`).
- Freed in `close_all()` at `fmain.c:967-968`:

```c
FreeCprList(fp_page1.savecop);
FreeCprList(fp_page2.savecop);
```

Also freed: `v.SHFCprList` (short-frame copper list, for interlace — likely NULL in this non-interlaced game) at `fmain.c:969`.

### Display Modes

- `vp_page`: No mode flags set → **LORES** (320-pixel). 5-bitplane (32 colors). (`fmain.c:808, 826`)
- `vp_text`: `HIRES | SPRITES | VP_HIDE` → **HIRES** (640-pixel), sprites enabled, initially hidden. 4-bitplane (16 colors). (`fmain.c:818, 828`)

The `VP_HIDE` flag is set at init time. The text viewport becomes visible when `MakeVPort` is called and the viewport dimensions are set to valid values during gameplay setup (`fmain.c:1253-1257`).

## 8. _stillscreen

Defined in `fmain2.c:631-634` (the C version is the linked version; `fsupp.asm:28-33` contains an unused assembly equivalent that is **not assembled or linked** by the makefile):

```c
stillscreen()
{   fp_drawing->ri_page->RxOffset = fp_drawing->ri_page->RyOffset = 0;
    pagechange();
}
```

### Purpose

Resets the scroll offset on the drawing page to (0,0) and immediately flips. This is used when displaying a static screen (e.g., the placard/legal text screen at `fmain.c:1232`). It ensures the bitmap is displayed from its top-left corner with no scroll displacement.

### RasInfo struct layout context

In the Amiga `struct RasInfo`, offset 8 is where `RxOffset` (SHORT at offset 8) and `RyOffset` (SHORT at offset 10) reside. The `clr.l 8(a1)` clears both the 16-bit `RxOffset` and 16-bit `RyOffset` in a single 32-bit write.

## 9. Other fsupp.asm Functions (NOT LINKED)

> **Note:** `fsupp.asm` is not assembled or linked by the makefile. These functions exist only as unused source code; the C equivalents in `fmain2.c` are what the game actually uses.

### _colorplay (`fsupp.asm:1-27`) — superseded by `fmain2.c:425-432`

Loads random colors into `vp_page` 32 times with 1-tick delays between each. Creates a psychedelic color flash effect:

```asm
_colorplay
    ; outer loop: 32 iterations (d5 = 31..0)
    ;   inner loop: fill fader[] with 32 random 12-bit colors
    ;   LoadRGB4(&vp_page, fader, 32)
    ;   Delay(1)
```

### _skipint (`fsupp.asm:35-42`) — superseded by `fmain2.c:836`

Calls `getkey()`, checks if result is space (32). If so, sets `_skipp` to 0xFF (true); otherwise 0. Returns the same value in d0. Used for skip-intro detection.

## 10. ViewPorts, RastPorts, and BitMaps Summary

### Views

| Variable | Type | Purpose | Citation |
|----------|------|---------|----------|
| `v` | `struct View` | Master view, owns the copper lists | fmain.c:18 |
| `oldview` | `struct View *` | Saved workbench view, restored on exit | fmain.c:18, 741, 964 |

### ViewPorts

| Variable | Mode | Width×Height | DxOff | DyOff | Colors | BitMap(s) | Citation |
|----------|------|-------------|-------|-------|--------|-----------|----------|
| `vp_text` | HIRES, SPRITES, VP_HIDE | 640×57 | 0 | 143 | 16 (4-plane) | `bm_text` via `ri_text` | fmain.c:19, 809, 815-818, 828 |
| `vp_page` | LORES | 288×140 | 16 | 0 | 32 (5-plane) | `bm_page1`/`bm_page2` via `ri_page1`/`ri_page2` | fmain.c:19, 808, 811-814, 826-827 |
| `vp_title` | (declared but not used in main display) | — | — | — | — | — | fmain.c:19 |

### RastPorts

| Variable | BitMap | Purpose | Citation |
|----------|--------|---------|----------|
| `rp_map` | `fp_drawing->ri_page->BitMap` (varies) | Drawing playfield map tiles and sprites | fmain.c:448, 1150-1151, 1977 |
| `rp_text` | `wb_bmap` initially, then `&bm_scroll` for intro | Text rendering on status bar | fmain.c:448, 749, 1170 |
| `rp_text2` | `bm_text` | Secondary text rendering | fmain.c:448, 836 |
| `rp` | pointer, switches between `&rp_text` and `&rp_map` | Current active RastPort | fmain.c:448 |

### BitMaps

| Variable | Depth | Width×Height | Memory | Purpose | Citation |
|----------|-------|-------------|--------|---------|----------|
| `bm_page1` | 5 | 320×200 | CHIP, AllocRaster | Playfield page 1 | fmain.c:826, 865 |
| `bm_page2` | 5 | 320×200 | CHIP, AllocRaster | Playfield page 2 | fmain.c:827, 867 |
| `bm_text` | 4 | 640×57 | Shares work_bm planes | Status bar display | fmain.c:828, 871-874 |
| `work_bm` | 2 | 640×200 | CHIP, MakeBitMap | Memory pool for text planes | fmain.c:744 |
| `bm_lim` | 1 | 320×200 | Shares sector_mem | Collision mask | fmain.c:829, 1180 |
| `bm_source` | 3 | 64×24 | (allocated with bm_page1 block) | Small sprite source | fmain.c:833 |
| `pagea` | 5 | 320×200 | Shares image_mem | IFF image scratch A | fmain.c:830, 1179 |
| `pageb` | 5 | 320×200 | Shares image_mem | IFF image scratch B | fmain.c:831, 1179 |
| `bm_scroll` | 1 | 640×57 | Shares bm_text plane 0 | Text scroll buffer | fmain.c:832, 938 |

### RasInfo

| Variable | BitMap | Purpose | Citation |
|----------|--------|---------|----------|
| `ri_page1` | `bm_page1` | Playfield page 1 RasInfo | fmain.c:841 |
| `ri_page2` | `bm_page2` | Playfield page 2 RasInfo | fmain.c:842 |
| `ri_text` | `bm_text` | Text viewport RasInfo | fmain.c:845, 886 |

## 11. Amiga Custom Chipset Interaction

### Copper

The game uses the Amiga OS abstraction layer (`View`/`ViewPort`/`MakeVPort`/`MrgCop`/`LoadView`) rather than directly programming copper lists. The OS generates copper instructions that:

- Set up bitplane pointers (BPLxPT) for each viewport.
- Switch display modes (lo-res → hi-res) at the correct scanline via WAIT instructions.
- Load color palettes for each viewport.
- The dual-viewport split happens automatically at scanline 143 (PAGE_HEIGHT).

### Blitter

The blitter is used extensively via direct custom register writes in `fsubs.asm`. Key operations:

1. **scrollmap** (`fsubs.asm:1736`): Scrolls all 5 bitplanes by one tile in any of 8 directions. Uses `OwnBlitter`/`DisownBlitter` for exclusive access. Writes directly to `$dff000` + register offsets (BLTCON0, BLTCON1, BLTAPT, BLTDPT, BLTAMOD, BLTDMOD, BLTAFWM, BLTALWM, BLTSIZE).

2. **save_blit** (`fsubs.asm:1953`): Saves 5 planes of background under a sprite into `sshape.backsave`. BLTCON0 = `$05CC` (D=B, copy source to dest). Source = bitplane + Coff, dest = save buffer.

3. **rest_blit** (`fsubs.asm:2001`): Reverse of save_blit — restores saved background from backsave buffer to bitplanes. Same miniterm `$05CC`.

4. **mask_blit** (`fsubs.asm:1908`): Creates composite mask from terrain shadow data and sprite shape mask. BLTCON0 = `$0b50` (D=A·NOT(C)). Processes single plane (mask buffer).

5. **shape_blit** (`fsubs.asm:1836`): Draws the actual sprite shape through the mask. BLTCON0 = `$0FCA` (D = A·B + NOT(A)·C = masked cookie-cut). Processes all 5 planes.

6. **map_draw** (`fsubs.asm:664`): Draws the full 20×6-tile map by copying image data from `image_mem` (the tileset) into all 5 bitplanes. Pure CPU copy, not blitter — uses register-to-register moves for speed.

7. **clear_blit** (`fsubs.asm:1808`): Blitter fill of memory with zeros. BLTCON0 = `$0100` (D only, miniterm 0).

### Sprites

Hardware sprite 0 is used for the mouse pointer:
- `FreeSprite(0); GetSprite(&pointer,0);` (`fmain.c:796-797`)
- `ChangeSprite(&vp_text,&pointer,(void *)sprite_data);` (`fmain.c:942`)
- The `SPRITES` flag on `vp_text` enables sprite DMA in the text viewport.

### Palette

- Playfield: 32 colors via `LoadRGB4(&vp_page, colors, 32)`. Modulated by `fade_page()` for day/night cycle at `fmain2.c:377-420`.
- Text bar: 20 colors via `LoadRGB4(&vp_text, textcolors, 20)` at `fmain.c:1257`.

### Vertical Beam Sync

- `WaitBOVP(&vp_text)` in `pagechange()` waits for beam past the text viewport — ensures the page flip is complete before proceeding (`fmain.c:3005`).

## 12. fade_page()

Defined at `fmain2.c:377-420`:

```c
fade_page(r,g,b,limit,colors) short r,g,b,limit; USHORT *colors;
```

Scales each color in the 32-entry `pagecolors` palette by `r/100`, `g/100`, `b/100` percentages. When `limit` is TRUE (gameplay), applies minimum night-time floor values (r≥10, g≥25, b≥60) and blue tint bias. When `limit` is FALSE (intro), allows full fade to black. Result stored in `fader[]` and loaded via `LoadRGB4(&vp_page, fader, 32)`.

Region-specific color 31 overrides: region 4 → `0x0980`, region 9 with secret_timer → `0x00f0`, otherwise → `0x0bdf` (`fmain2.c:382-388`).

## Cross-Cutting Findings

- **witch_fx uses Layer system**: `witch_fx()` at `fmain2.c:917` creates a temporary `CreateUpfrontLayer()` on the drawing bitmap to draw a clipped polygon (the witch's rotating beam). This is a rare use of the Layers library on the playfield bitmap. The XOR drawing mode means it can be applied and removed by calling it twice.
- **bm_lim doubles as collision surface**: `bm_lim->Planes[0] = sector_mem` (`fmain.c:1180`) — the same memory used for sector map data is treated as a 1-plane bitmap for collision testing.
- **Memory sharing between text/queue/backsave**: The text bitmap planes are carved to also hold the shape queues and background save buffers, maximizing use of chip RAM. This means text plane 2 and 3 are partially overwritten by sprite queue data — the visible text area is only 57 scanlines tall, so the overflow area beyond that is safe to reuse.
- **fp_page2.ri_page temporarily set to ri_page1 during intro**: During the zoom animation (`fmain.c:1198`), both pages point to the same bitmap so the zoom effect displays identically regardless of which page is shown. This is a single-buffer mode trick.
- **map_draw is pure CPU, not blitter**: Despite the game's heavy blitter use for sprites and scrolling, the full-screen tile redraw at `fsubs.asm:664` uses CPU register moves, not blitter DMA.
- **MAXSHAPES = 25** (`fmain.c:68`): Maximum number of sprites per page frame. Limited by both the shape_queue array size and the backsave buffer capacity (`74*80 = 5920` bytes at `fmain.c:2549`).
- **gdriver.asm is the audio driver**: Despite its name suggesting "graphics driver," `gdriver.asm` contains the music/audio interrupt handler, not graphics code. The blitter/display routines are in `fsubs.asm`.

## Unresolved

- **Color table `pagecolors`**: Declared as `extern USHORT pagecolors[]` at `fmain.c:474`. Not defined in fmain.c — must be defined elsewhere (likely loaded from a file or defined in another source file). Cannot determine the base palette values from the files read.
- **VP_HIDE semantics**: `vp_text.Modes` includes `VP_HIDE` at init time (`fmain.c:818`). This flag is documented to hide a viewport. It's unclear exactly when/if it gets cleared — it may be implicitly handled by the OS when DHeight is set to a valid value and `MakeVPort` is called, or it may remain set (the Amiga OS may treat VP_HIDE differently across Kickstart versions).
- **witch_fx full implementation**: Only the first half was read (`fmain2.c:917-960`). The AreaEnd/polygon drawing is clear, but the rest of the function (cleanup) was not fully traced.
- **vp_title**: Declared at `fmain.c:19` alongside `vp_page` and `vp_text`, but never observed being initialized or used in the display chain. Its purpose is unknown.

## Refinement Log

- 2026-04-06: Initial comprehensive discovery pass. All 10 questions answered with source citations. Read ftale.h, fmain.c (open_all, close_all, pagechange, screen_size, main loop rendering pipeline), fmain2.c (fade_page, witch_fx), MakeBitMap.asm (full), fsupp.asm (stillscreen, colorplay, skipint), fsubs.asm (scrollmap, map_draw, save_blit, rest_blit, mask_blit, shape_blit, clear_blit).
