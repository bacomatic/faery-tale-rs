# Discovery: Sprite Compositing & Blitter Operations

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace the full sprite rendering pipeline: blitter blit functions, mask creation, background save/restore, shape queue, sprite memory layout, and rendering order.

## Data Structures

### struct sshape — fmain.c:436-440

Per-shape rendering metadata, stored in the shape queue:

```c
struct sshape {
    unsigned char *backsave;    // pointer to saved background data
    short savesize;             // size of backsave in bytes (blitwide * blithigh * 2)
    short blitsize;             // Amiga blitter size register value (blithigh<<6 + blitwide)
    short Coff;                 // byte offset into bitplane for this sprite's screen position
    short Cmod;                 // modulus for screen bitplane (40 - blitwide*2)
};
```

### struct seq_info — ftale.h:83-89

Describes a loaded sprite sheet:

```c
struct seq_info {
    short width, height, count;             // tile dimensions (width in 16px units) and frame count
    unsigned char *location, *maskloc;      // pointers to sprite data and precomputed masks
    short bytes;                            // bytes per single plane of one frame (width*2 * height)
    short current_file;
};
```

The `seq_list` array has 7 entries matching enum `sequences`:
- `fmain.c:41` — `struct seq_info seq_list[7];`
- `ftale.h:91` — `enum sequences {PHIL, OBJECTS, ENEMY, RAFT, SETFIG, CARRIER, DRAGON};`

### struct fpage — ftale.h:70-80

Per-page (double-buffer) rendering state:

```c
struct fpage {
    struct RasInfo *ri_page;        // Amiga RasInfo for this display page
    struct cprlist *savecop;        // saved copper list
    long isv_x, isv_y;             // saved image position (for scroll delta)
    short obcount;                  // number of shapes drawn this frame
    struct sshape *shape_queue;     // array of sshape entries (MAXSHAPES=25 per page)
    unsigned char *backsave;        // base pointer to background save memory
    long saveused;                  // bytes of backsave memory consumed this frame
    short witchx, witchy, witchdir, wflag;  // witch effect state for erasure
};
```

- Two instances: `fp_page1`, `fp_page2` — `fmain.c:443`
- `fp_drawing` and `fp_viewing` pointers swap each frame — `fmain.c:443`
- `MAXSHAPES` is 25 — `fmain.c:68`

### struct cfile_info — fmain2.c:638-645

Describes a sprite file's disk layout and destination:

```c
struct cfile_info {
    UBYTE width, height, count;    // width (in 16px words), height (scanlines), frame count
    UBYTE numblocks;               // disk blocks to load
    UBYTE seq_num;                 // target seq_list slot
    USHORT file_id;                // disk block offset
};
```

Defined in `cfiles[]` table at `fmain2.c:645-672`. Example entries:
- `{ 1,32,67, 42, PHIL, 1376 }` — Julian: 1-word wide, 32 scanlines, 67 frames
- `{ 1,16,116,36, OBJECTS, 1312 }` — Objects: 1-word wide, 16 scanlines, 116 frames
- `{ 2,32,2, 3, RAFT, 1348 }` — Raft: 2-word wide, 32 scanlines, 2 frames
- `{ 4,64,8, 40, CARRIER, 1120 }` — Bird: 4-word wide, 64 scanlines, 8 frames
- `{ 3,40,5, 12, DRAGON, 1160 }` — Dragon: 3-word wide, 40 scanlines, 5 frames

## Sprite Memory Layout

### Allocation

- `shape_mem`: 78000 bytes (`SHAPE_SZ`), allocated in chip RAM — `fmain.c:641,922`
- `shadow_mem`: 12288 bytes (`SHADOW_SZ = 8192+4096`), allocated in chip RAM — `fmain.c:642,924`. Contains pre-built terrain occlusion masks loaded from disk at block 896 — `fmain.c:1222`
- `bmask_mem`: derived from `bm_text->Planes[3] + (TEXT_HEIGHT*80)` — `fmain.c:877`. Reused from text bitmap memory. This is the compositing mask buffer.

### Shape Data Layout in Memory

`read_shapes()` at `fmain2.c:685-700` loads sprite data into chip RAM. Each frame consists of **5 bitplanes** of image data plus **1 plane** of mask data.

For a sprite with dimensions `width` words × `height` scanlines:
- `bytes` = `width * height * 2` (bytes per plane per frame) — `fmain2.c:691`
- Total image data per frame = `bytes * 5` (5 bitplanes)
- Total mask data per frame = `bytes * 1`
- Total size loaded from disk = `bytes * count * 5` (all frames, 5 planes) — loaded via `load_track_range` — `fmain2.c:695-697`

Memory is arranged **interleaved by plane**, not interleaved by scanline:
- `seq_list[slot].location` = start of image data (5 planes × count frames)
- `seq_list[slot].maskloc` = start of mask area, placed immediately after image data — `fmain2.c:698`

The `nextshape` pointer advances through `shape_mem`:
- After image data: `nextshape += size * 5` — `fmain2.c:697`
- After mask area: `nextshape += size` — `fmain2.c:699`

So total memory per sprite set = `bytes * count * 6` (5 image planes + 1 mask plane).

### Shape Data Addressing at Render Time

For a specific frame `inum` of type `atype`:
- Image data: `shapedata = seq_list[atype].location + (planesize * 5 * inum)` — `fmain.c:2540`
- Mask data: `mask_buffer = seq_list[atype].maskloc + (planesize * inum)` — `fmain.c:2541`

Where `planesize = seq_list[atype].bytes` — `fmain.c:2536`.

Individual bitplane N of the shape is accessed as `shapedata + planesize * N` (N = 0..4) — see `_shape_blit` at `fsubs.asm:1839-1841`.

## Loading Pipeline

### shape_read() — fmain2.c:673-682

Orchestrates the full shape load sequence:

```c
shape_read() {
    nextshape = shape_mem;
    read_shapes(3); prep(OBJECTS);       // objects (cfiles[3])
    read_shapes(brother-1); prep(PHIL);  // current brother
    read_shapes(4); prep(RAFT);          // raft
    seq_list[ENEMY].location = nextshape;
    read_shapes(actor_file); prep(cfiles[actor_file].seq_num);
    read_shapes(set_file); prep(SETFIG);
    new_region = region_num; load_all();
    motor_off();
}
```

Load order is: Objects → Player character → Raft → Enemy → Set figure.

### read_shapes(num) — fmain2.c:685-700

Loads raw sprite bitplane data from disk into `shape_mem`:

1. Looks up `cfiles[num]` for dimensions and disk location
2. Calculates `bytes = height * width * 2` per plane per frame — `fmain2.c:691`
3. Sets `seq_list[slot].location = nextshape` — `fmain2.c:693`
4. Bounds-checks: `(nextshape + size*6) <= (shape_mem + SHAPE_SZ)` — `fmain2.c:695`
5. Issues async disk read: `load_track_range(file_id, numblocks, nextshape, 8)` — `fmain2.c:696`
6. Advances `nextshape` past 5 image planes: `nextshape += size * 5` — `fmain2.c:697`
7. Sets `seq_list[slot].maskloc = nextshape` — `fmain2.c:698`
8. Advances `nextshape` past 1 mask plane: `nextshape += size` — `fmain2.c:699`

### prep(slot) — fmain2.c:749-754

Waits for the async disk read to complete, then generates the mask:

```c
prep(slot) short slot; {
    WaitDiskIO(8);
    InvalidDiskIO(8);
    make_mask(seq_list[slot].location, seq_list[slot].maskloc,
        seq_list[slot].width, seq_list[slot].height, seq_list[slot].count);
}
```

## Blitter Operations

All blitter functions are in `fsubs.asm:1798-2050`. They operate through Amiga custom chip registers at `$dff000`.

### Global Variables for Blitter Setup

Set by the C rendering loop before calling blitter functions:

| Variable | Set at | Meaning |
|----------|--------|---------|
| `shp` | `fmain.c:2535` | Pointer to current `sshape` entry in shape queue |
| `shapedata` | `fmain.c:2540` | Pointer to sprite image data (5 planes) for current frame |
| `mask_buffer` | `fmain.c:2541` | Pointer to sprite mask data (1 plane) for current frame |
| `planesize` | `fmain.c:2536` | Bytes per plane per frame |
| `aoff` | `fmain.c:2553` | Offset into mask buffer: `((ystart1)&31) * blitwide * 2` |
| `boff` | `fmain.c:2554` | Offset into shape/mask data: `(xoff>>4)*2 + cwide*yoff` |
| `bmod` | `fmain.c:2557` | Source modulus: `cwide - (blitwide * 2)` |
| `shift` | `fmain.c:2556` | Pixel shift: `xstart & 15` |
| `wmask` | `fmain.c:2556` | Word mask: 0 if sub-word offset, -1 (0xFFFF) otherwise |
| `bmask_mem` | `fmain.c:877` | Compositing mask buffer base address |

### _make_mask — fsubs.asm:1619-1653

**Purpose**: Generate sprite masks from sprite image data by ORing all 5 bitplanes together and inverting. Called from `prep()` via C `make_mask()`.

**Parameters** (from stack):
- `a4` (44(sp)): source image data pointer
- `a5` (48(sp)): destination mask pointer
- `d4` (52(sp)): width in words
- `d3` (56(sp)): height in scanlines
- `d2` (60(sp)): frame count

**Algorithm**:
1. `d4 = width * height` (words per plane) — `fsubs.asm:1630`
2. `d1 = d4 * 2` (plane length in bytes) — `fsubs.asm:1631`
3. For each frame:
   - Set 5 plane pointers: `a0, a1, a2, a3, a4` each offset by `d1` bytes — `fsubs.asm:1634-1638`
   - For each word in the plane:
     - `d0 = (a0) AND (a1) AND (a2) AND (a3) AND (a4)` — `fsubs.asm:1642-1645`
     - `NOT d0` — `fsubs.asm:1646`
     - Store to destination — `fsubs.asm:1647`

**Result**: A 1-plane mask where `1` bits mark pixels that have ANY color (non-zero across all 5 planes). The AND + NOT logic: a pixel is transparent only if it's set in ALL 5 planes (color 31). All other colors produce a `1` (opaque) in the mask.

**Note**: This means color 31 (all bits set = `11111`) is the transparency color. The AND of all 5 planes produces 1 only where all planes are 1. Inverting gives 0 for color 31 (transparent) and 1 for everything else (opaque).

### _clear_blit — fsubs.asm:1799-1810

**Purpose**: Clear a memory area using the blitter (zeroing).

**Parameters**:
- `4(sp)`: destination pointer
- `10(sp)`: blitsize value (encodes height and width)

**Blitter setup**:
- `BLTCON0 = $0100` — use D channel only, miniterm = 0 (all zeros) — `fsubs.asm:1800`
- `BLTCON1 = $0000` — no options — `fsubs.asm:1801`
- `BLTDMOD = 0` — no destination modulus — `fsubs.asm:1802`

**Usage**: Called before each sprite draw to zero `bmask_mem`:
```c
OwnBlitter();
WaitBlit(); clear_blit(bmask_mem, CBK_SIZE);
DisownBlitter();
```
Where `CBK_SIZE = (96<<6)+5` = 6149 → encodes 96 scanlines × 5 words — `fmain.c:680,2417`

### _save_blit — fsubs.asm:1908-1949

**Purpose**: Save the background behind a sprite before drawing. Copies screen bitplane data to a save buffer.

**Parameters**:
- `8*4(sp)` (after register saves): pointer to backsave memory

**Blitter setup** (per plane, 5 planes):
- `BLTCON0 = $05CC` — miniterm `D = B` (straight copy from B to D), channels A+B+D — `fsubs.asm:1924`
- `BLTCON1 = 0` — no shift — `fsubs.asm:1923`
- Masks: both `$FFFF` — `fsubs.asm:1927-1928`
- `BLTBPT` = screen plane + `Coff` (source: screen) — `fsubs.asm:1930`
- `BLTDPT` = backsave + `savesize * planeIndex` (destination: save buffer) — `fsubs.asm:1932-1935`
- `BLTBMOD` = `Cmod` (screen modulus, 40 - blitwide*2) — `fsubs.asm:1937`
- `BLTDMOD` = 0 (packed destination) — `fsubs.asm:1938`
- `BLTSIZE` = `blitsize` from sshape — `fsubs.asm:1939`

Loops over all 5 bitplanes (d2 counts 4→0) — `fsubs.asm:1912`

### _mask_blit — fsubs.asm:1860-1906

**Purpose**: Create the compositing mask by combining the sprite's precomputed mask with the terrain occlusion mask buffer (`bmask_mem`). The result is a mask that determines where the sprite is visible vs occluded by terrain.

**Parameters**: None (uses globals `shp`, `mask_buffer`, `bmask_mem`, `aoff`, `boff`, `shift`, `bmod`)

**Blitter setup** (single pass, 1 plane):
- Source A (`BLTAPT`): `mask_buffer + boff` — sprite's precomputed mask — `fsubs.asm:1877-1879`
- Source C (`BLTCPT`): `bmask_mem + 2 + aoff` — current compositing mask state — `fsubs.asm:1873-1875`
- Dest D (`BLTDPT`): same as C — `fsubs.asm:1873-1875`
- `BLTCON0 = shift<<12 | $0B50` — miniterm `D = A AND (NOT C)` with A shift — `fsubs.asm:1882-1883`
  - Actually: `$0B50` miniterm bits. With A shifted. The logic: D = (shifted A) NAND C? Let me decode: `$0B50` → channels A,C,D active. Miniterm $50 = `01010000`. This is `D = A AND (NOT C)`. This punches the sprite mask into the compositing buffer, but only where the compositing buffer doesn't already have terrain occlusion.

  Correction: Let me re-derive. `$0B50`:
  - High nibble `$0` = no shift (handled separately via BLTCON1 shift bits... wait, this is BLTCON0). Actually bits 15-12 are the A-channel barrel shift, set separately. The `or.w #$0b50,d1` adds the shifted value: `$0b` = channels used (A=1,B=0,C=1,D=1 → ACDused), `$50` = miniterm. Miniterm `$50` = `01010000`:
    - Bit 7 (ABC=111): 0
    - Bit 6 (ABc=110): 1  → A AND (NOT C)
    - Bit 5 (AbC=101): 0
    - Bit 4 (Abc=100): 1  → A AND (NOT B) — but B unused, so A alone (when C=0)
    - Bits 3-0: 0

  Since B is not enabled, this simplifies. With channels A,C,D: `D = A AND (NOT C)`.
  But wait — looking again at the actual value: d1 has the shift in bits 15-12. Then `or.w #$0b50,d1`:
  - Bits 11-8: `$b` = `1011` → USE_A=1, USE_B=0, USE_C=1, USE_D=1
  - Bits 7-0: `$50` = `01010000`

  Miniterm $50 with A and C channels: `D = A AND (NOT C)`. This means: where the sprite mask is 1 (opaque pixel) AND the compositing mask is 0 (not yet occluded by terrain), set D to 1. This effectively creates the final compositing mask.

  Wait, that doesn't sound right for how it's used. Let me re-check. Actually `BLTCPT` and `BLTDPT` both point to `bmask_mem + 2 + aoff`. So this is a read-modify-write on the compositing buffer. The sprite mask goes through channel A (with shift), terrain mask is in C (same as D). The result `D = A AND (NOT C)` means: sprite mask bits that are NOT already set in the compositing buffer get added. But this gives the sprite-only bits — it doesn't preserve the existing mask. 

  Hmm, actually I see: `$50` = miniterm where the only 1-bits are at positions 6 and 4. 

  Position 6: A=1,B=1,C=0 → but B is unused, interpret as don't-care
  Position 4: A=1,B=0,C=0

  With B not used, positions where A=1,C=0 → result = 1. Positions where A=0 or C=1 → result = 0.
  So `D = A AND (NOT C)`.

  But the compositing buffer starts cleared to 0 (`clear_blit` zeroes it). Terrain `maskit()` may have written some 1-bits into it. So `mask_blit` sets D = (shifted sprite mask) AND (NOT terrain mask). This produces a mask that has 1 where the sprite is opaque AND there's no terrain occlusion. The result goes to `bmask_mem`.

  Wait—but then `shape_blit` uses this as channel A. Looking at `_shape_blit`, `BLTCON0 = $0FCA` and BLTAPT = `bmask_mem + 2 + aoff`. So the cookie-cut mask is used as the stencil: D = (B AND A) OR (C AND NOT A). Where the mask is 1, take the sprite; where 0, keep the background.

- `BLTCON1 = 0` — no shift on B channel (only A is shifted) — `fsubs.asm:1884`
- `BLTAMOD = bmod` (source sprite mask modulus) — `fsubs.asm:1889`
- `BLTDMOD = 0` — destination compositing buffer has no extra modulus — `fsubs.asm:1890`
- `BLTCMOD = 0` — same — `fsubs.asm:1891`
- `BLTSIZE` = `shp->blitsize` — `fsubs.asm:1892`

### _shape_blit — fsubs.asm:1812-1857

**Purpose**: Draw the sprite onto the screen using cookie-cut compositing. Uses the compositing mask to combine sprite data with the existing background.

**Parameters**: None (uses globals `shp`, `shapedata`, `bmask_mem`, `aoff`, `boff`, `bmod`, `shift`, `wmask`, `planesize`)

**Blitter setup** (per plane, 5 planes):
- `BLTCON0 = $0FCA` — `fsubs.asm:1828`
  - Bits 11-8: `$F` = `1111` → USE_A=1, USE_B=1, USE_C=1, USE_D=1
  - Miniterm `$CA` = `11001010`:
    - Bit 7 (ABC=111): 1 → where mask=1, sprite=1, bg=1 → 1
    - Bit 6 (ABc=110): 1 → where mask=1, sprite=1, bg=0 → 1
    - Bit 5 (AbC=101): 0 → where mask=1, sprite=0, bg=1 → 0
    - Bit 4 (Abc=100): 0 → where mask=1, sprite=0, bg=0 → 0
    - Bit 3 (aBC=011): 1 → where mask=0, sprite=1, bg=1 → 1
    - Bit 2 (aBc=010): 0 → where mask=0, sprite=1, bg=0 → 0
    - Bit 1 (abC=001): 1 → where mask=0, sprite=0, bg=1 → 1
    - Bit 0 (abc=000): 0
  - This is `D = (A AND B) OR (NOT A AND C)` = cookie-cut: where mask (A) is 1, take sprite (B); where mask is 0, keep background (C).

- `BLTCON1`: B-channel shift = `shift << 12` (pixel alignment) — `fsubs.asm:1826-1827`
- Word masks:
  - `BLTAFWM` = `($FFFF >> shift) AND wmask` — first word mask — `fsubs.asm:1830-1833`
  - `BLTALWM` = `NOT($FFFF >> shift)`, or `$FFFF` if shift produces 0 — `fsubs.asm:1834-1837`

- Channel assignments:
  - A (`BLTAPT`): `bmask_mem + 2 + aoff` — compositing mask — `fsubs.asm:1845-1846`
  - B (`BLTBPT`): `shapedata + boff + planesize * planeIndex` — sprite plane data — `fsubs.asm:1839-1841`
  - C (`BLTCPT`): screen plane + `Coff` (background) — `fsubs.asm:1843-1844`
  - D (`BLTDPT`): same as C (write back to screen) — `fsubs.asm:1843`

- Moduli:
  - `BLTAMOD = 0` — mask buffer packed — `fsubs.asm:1848`
  - `BLTBMOD = bmod` — sprite source modulus — `fsubs.asm:1849`
  - `BLTCMOD = Cmod` — screen modulus (40 - blitwide*2) — `fsubs.asm:1850`
  - `BLTDMOD = Cmod` — same — `fsubs.asm:1851`

Loops over all 5 bitplanes (d3 counts 4→0) — `fsubs.asm:1819`

### _rest_blit — fsubs.asm:1951-1992

**Purpose**: Restore the background from the save buffer after rendering. This undoes the sprite draw from the previous frame.

**Parameters**:
- `8*4(sp)`: pointer to backsave memory (source)

**Blitter setup** (per plane, 5 planes):
- `BLTCON0 = $05CC` — `D = B` (straight copy) — `fsubs.asm:1969`
- `BLTCON1 = 0` — `fsubs.asm:1968`
- Masks: `$FFFF` — `fsubs.asm:1971-1972`
- `BLTBPT` = backsave + `savesize * planeIndex` (source: saved background) — `fsubs.asm:1977-1980`
- `BLTDPT` = screen plane + `Coff` (destination: screen) — `fsubs.asm:1974`
- `BLTDMOD = Cmod` — `fsubs.asm:1982`
- `BLTBMOD = 0` — `fsubs.asm:1983`
- `BLTSIZE` = `shp->blitsize` — `fsubs.asm:1984`

Loops over all 5 bitplanes (d2 counts 4→0) — `fsubs.asm:1958`

Note: `_rest_blit` is the inverse of `_save_blit` — save copies screen→buffer, restore copies buffer→screen.

### _maskit — fsubs.asm:1047-1093

**Purpose**: Stamp a terrain occlusion tile into the compositing mask buffer. This is how foreground terrain (trees, buildings) can partially occlude sprites.

**Parameters** (from stack):
- `d3` (32(sp)): x offset (in characters/tiles)
- `d4` (36(sp)): y offset (in characters/tiles)
- `d2` (40(sp)): modulus (blitwide, in words)
- `d0` (44(sp)): character number (terrain mask index)

**Data sources**:
- `bmask_mem`: destination compositing mask buffer — `fsubs.asm:1050`
- `shadow_mem`: source terrain mask data (loaded from disk block 896) — `fsubs.asm:1052`

**Algorithm**:
1. Offset into shadow_mem: `character * 64` bytes — `fsubs.asm:1056` (each terrain mask tile is 64 bytes = 32 scanlines × 1 word)
2. Offset into bmask_mem: `y * 32 * modulus*2 + x * 2` — `fsubs.asm:1058-1062`
3. Copies 32 scanlines (8 iterations × 4 words per iteration) from shadow_mem to bmask_mem — `fsubs.asm:1065-1074`

## Complete Rendering Pipeline

### Frame Rendering Order

Each frame follows this sequence, executed in the main loop at `fmain.c:1268-2618`:

#### Phase 1: Background Restoration (previous frame cleanup)

At `fmain.c:1968-1974`:
```c
OwnBlitter();
for (i = fp_drawing->obcount; i > 0; i--) {
    shp = &(fp_drawing->shape_queue[i-1]);
    rest_blit(shp->backsave);
}
DisownBlitter();
```

Sprites are restored in **reverse order** (last drawn → first drawn). This is critical: since sprites may overlap, restoring in reverse order correctly rebuilds the background. The `fp_drawing` page being restored is the one that was **drawn last frame** and is about to be drawn on again.

Also undoes witch effects: `if (fp_drawing->wflag) witch_fx(fp_drawing)` — `fmain.c:1978`

#### Phase 2: Map Scrolling

Scroll blitting is done by `scrollmap()` (via `_scrollmap` at `fsubs.asm:1741-1796`), then strip/row drawing repairs the edges — `fmain.c:2367-2368`.

#### Phase 3: Actor Sorting

At `fmain.c:2336-2365`: Bubble sort of `anim_index[]` by Y coordinate. Dead actors and the riding mount get Y-bias (-32) to sort behind. Actors in deep water (`environ > 25`) get Y-bias (+32) to sort in front.

#### Phase 4: Per-Sprite Rendering

Reset counters: `fp_drawing->obcount = crack = fp_drawing->saveused = 0` — `fmain.c:2378`

For each actor in sorted order (`j=0..anix2-1`) at `fmain.c:2380`:

1. **Determine sprite type and frame** — `fmain.c:2395-2467` (weapon pass vs character pass, type overrides)

2. **Clip** — `fmain.c:2509-2528` — clip to screen (319×173 playfield) and compute clipped coordinates

3. **Calculate blitter parameters** — `fmain.c:2530-2558`:
   - `shp` = `&fp_drawing->shape_queue[fp_drawing->obcount]`
   - `planesize`, `shapedata`, `mask_buffer` set from `seq_list[atype]`
   - `savesize`, `blitsize`, `Coff`, `Cmod`, `aoff`, `boff`, `bmod`, `shift`, `wmask` computed

4. **Allocate backsave memory** — `fmain.c:2542-2549`:
   - Small shapes (`savesize < 64` and `crack < 5`): use unused area at end of a bitplane: `planes[crack++] + (192*40)` (byte 7680 into the plane, past the 192-scanline visible area)
   - Larger shapes: sequential allocation from `fp_drawing->backsave` via `backalloc`
   - Safety limit: `fp_drawing->saveused >= (74*80)` = 5920 bytes → stop drawing — `fmain.c:2548`

5. **OwnBlitter()** — `fmain.c:2560`

6. **save_blit(shp->backsave)** — `fmain.c:2561` — Save background under sprite

7. **Terrain occlusion masking** — `fmain.c:2563-2596`:
   - First: `clear_blit(bmask_mem, CBK_SIZE)` — zero the compositing mask — `fmain.c:2417`
   - For each terrain tile overlapping the sprite, check `terra_mem[cm+1] & 15` for occlusion type (0-7)
   - Types determine when terrain masks are applied (never, when down, when right, always, etc.)
   - If applicable: call `maskit(xm, ym, blitwide, terra_mem[cm])` to stamp terrain mask into `bmask_mem`
   - Certain conditions skip masking entirely (goto `nomask`): arrows, carriers, certain races (0x85, 0x87), player riding — `fmain.c:2565-2571`

8. **mask_blit()** — `fmain.c:2600` — Combine sprite mask with terrain occlusion in `bmask_mem`

9. **shape_blit()** — `fmain.c:2601` — Cookie-cut sprite onto screen

10. **Set visible flag**: `an->visible = TRUE` — `fmain.c:2602`

11. **DisownBlitter()** — `fmain.c:2603`

12. **Increment**: `fp_drawing->obcount++` — `fmain.c:2605`

13. **Weapon pass**: If actor has a weapon, loop back to draw the weapon sprite as a second pass (`pass++`, `passmode = 1`) — `fmain.c:2400-2413`

#### Phase 5: Page Flip

At `fmain.c:2609-2614`:
```c
WaitBlit();
fp_drawing->ri_page->RxOffset = (map_x & 15);
fp_drawing->ri_page->RyOffset = (map_y & 31);
pagechange();
```

`pagechange()` at `fmain.c:2993-3008` swaps `fp_drawing`/`fp_viewing` pointers and loads the new copper list for display.

### Double-Buffering

Two pages: `fp_page1` and `fp_page2` — `fmain.c:443`. Each has:
- Its own `RasInfo` (display origin)
- Its own copper list (`savecop`)
- Its own `shape_queue` (25 entries) — `fmain.c:882-884`
- Its own `backsave` memory — `fmain.c:879-880`

Shape queues are allocated from `queue_mem`:
- `fp_page1.shape_queue` = `queue_mem` — `fmain.c:882`
- `fp_page2.shape_queue` = `queue_mem + sizeof(sshape) * MAXSHAPES` — `fmain.c:883-884`

Backsave areas:
- `fp_page1.backsave` = `queue_mem + 962` — `fmain.c:879`
- `fp_page2.backsave` = `bmask_mem + 962` — `fmain.c:880`

Each frame draws to `fp_drawing` while `fp_viewing` is displayed. After drawing, `pagechange()` swaps them.

### Summary Pipeline Diagram

```
For each frame:
  1. RESTORE previous frame's sprites (rest_blit × obcount, reverse order)
  2. UNDO witch effects
  3. SCROLL map (scrollmap blitter)
  4. REPAIR map edges (strip_draw, row_draw)
  5. SORT actors by Y coordinate
  6. For each actor (front-to-back by Y):
     a. clear_blit(bmask_mem)  — zero compositing mask
     b. save_blit(backsave)    — save background
     c. maskit() × N           — stamp terrain masks into bmask_mem
     d. mask_blit()            — combine sprite mask with terrain occlusion
     e. shape_blit()           — cookie-cut draw sprite
  7. PAGE FLIP — swap drawing/viewing pages
```

## Compositing Mask Details

The compositing mask (`bmask_mem`) is a single-plane buffer used for per-sprite masking. Its size is `CBK_SIZE = (96<<6)+5` which encodes a maximum of 96 scanlines × 5 words (80 pixels wide) — `fmain.c:680`.

The buffer is at `bmask_mem + 2` for blitter access (the +2 offset appears in `_shape_blit` at `fsubs.asm:1845` and `_mask_blit` at `fsubs.asm:1873`). This 2-byte offset allows the barrel shifter to handle leftward sub-word alignment without underflow.

### Compositing Logic

For each sprite:

1. `bmask_mem` is cleared to all zeros (`clear_blit`)
2. Terrain occlusion masks are stamped in (`maskit`) — sets 1-bits where terrain foreground exists
3. `mask_blit` computes: `bmask_mem = (sprite_mask >> shift) AND (NOT bmask_mem)`
   - Result: 1 where sprite is opaque AND no terrain occlusion
4. `shape_blit` uses this as cookie-cut stencil: `screen = (bmask AND sprite) OR (NOT bmask AND screen)`
   - Sprite pixels appear where mask is 1; background shows through where mask is 0

So terrain occlusion works by preventing the sprite mask from being set in occluded areas, which means the background (with terrain) shows through.

## 32-Color (5-Bitplane) Compositing

All sprite operations work on **5 bitplanes** individually. Each blitter operation that touches sprite/screen data loops 5 times (d3/d2 counts 4→0):
- `save_blit`: 5 planes saved — `fsubs.asm:1912`
- `rest_blit`: 5 planes restored — `fsubs.asm:1958`
- `shape_blit`: 5 planes drawn — `fsubs.asm:1819`
- `mask_blit`: 1 plane only (compositing mask is single-plane) — `fsubs.asm:1860-1906`
- `make_mask`: generates 1 mask plane from 5 source planes — `fsubs.asm:1619-1653`

The Amiga blitter processes one bitplane at a time, so 5-plane compositing requires 5 sequential blitter operations per draw step (11 blitter passes per sprite for save+mask+shape: 5+1+5).

## Cross-Cutting Findings

- **Backsave optimization**: Small sprites (`savesize < 64`) reuse the bottom of screen bitplanes (past the 192-scanline visible area at offset `192*40 = 7680`) instead of allocating from the backsave pool. Up to 5 small sprites can use this optimization (`crack < 5`) — `fmain.c:2543`
- **Terrain occlusion types** in `terra_mem[cm+1] & 15` control when terrain foreground masks apply (never/down/right/always/conditional) — `fmain.c:2579-2596`. The `shadow_mem` data is loaded once from disk block 896 (24 blocks = 12288 bytes) — `fmain.c:1222`
- **Weapon rendering**: Each actor can generate TWO sprite draws — one for the character, one for the weapon. The weapon pass reuses the same pipeline with different `atype`/`inum` — `fmain.c:2400-2466`
- **Safety limit**: Maximum 25 shapes per frame (`MAXSHAPES`), and backsave memory is limited to 5920 bytes (`74*80`). If exceeded, rendering stops — `fmain.c:2548`
- **Color 31 transparency**: The mask generation (`_make_mask`) uses AND + NOT across all 5 planes, making color 31 (binary `11111`) the designated transparent color — `fsubs.asm:1642-1647`

## Unresolved

- The exact content and format of `shadow_mem` (terrain occlusion mask tiles loaded from disk block 896) is not documented in source. The loading size is 12288 bytes (`SHADOW_SZ = 8192+4096`). Each tile uses 64 bytes (32 scanlines × 1 word). With 12288 bytes that's up to 192 tiles, but the actual number of distinct terrain mask tiles and how they map to terrain types would require examining the binary data.
- The `minimap[]` array (114 shorts, declared `fmain.c:630`) maps screen tile positions to terrain indices, but how it's populated by `gen_mini()`/`genmini()` is only partially traced in this investigation.
- The `+2` offset on `bmask_mem` in blitter operations (`fsubs.asm:1845,1873`) is consistent and appears to be for barrel-shift accommodation, but code comments don't explain it explicitly.

## Refinement Log

- 2026-04-06: Initial comprehensive discovery pass covering all 12 questions.
