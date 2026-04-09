# Discovery: IFF/ILBM Image Loading

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace the complete IFF/ILBM image loading system — `unpackbrush()`, `unpackpic()`, `_unpack_line`, `_ReadHeader`/`_ReadLength`, chunk handling, color palette loading, ifdef'd-out functions, and game file usage.

## 1. IFF Tag System

The IFF chunk type comparison uses a clever hack. A string of concatenated 4-byte ASCII tags is cast to a `long*` array — `iffsubs.c:19-20`:

```c
char *kluge = "FORMILBMBMHDCMAPGRABBODYCAMGCRNG";
long *tags;
```

Then `#define` macros index into this array — `iffsubs.c:8-15`:

```c
#define FORM    tags[0]    /* "FORM" */
#define ILBM    tags[1]    /* "ILBM" */
#define BMHD    tags[2]    /* "BMHD" */
#define CMAP    tags[3]    /* "CMAP" */
#define GRAB    tags[4]    /* "GRAB" */
#define BODY    tags[5]    /* "BODY" */
#define CAMG    tags[6]    /* "CAMG" */
#define CRNG    tags[7]    /* "CRNG" */
```

At runtime, `tags = (long *) kluge` — `iffsubs.c:89,147`. Each 4-byte IFF chunk type ID is compared against the corresponding long value from this array.

**Note**: `DEST` is referenced in the skip list (`iffsubs.c:101,157`) but is never `#define`d in this file. This is either a symbol defined elsewhere in the original build environment (not found in ftale.h or other headers), or an unresolved reference that only compiles because it appears inside an `#ifdef blarg` block (for `unpackpic`) and in `unpackbrush` where it would cause a compile error if not defined. This is flagged as Unresolved below.

## 2. IFF Chunk Parsing: `_ReadHeader` / `_ReadLength`

Both functions are implemented in assembly for performance — `fsubs.asm:624-656`. The C versions are commented out in `iffsubs.c:62-69`.

### `_ReadHeader` — `fsubs.asm:628-638`

Reads 4 bytes from file into the global `header` variable. Decrements `file_length` by 4.

```asm
_ReadHeader
    movem.l     d1-d3/a0-a1,-(sp)
    move.l      _DOSBase,a6
    move.l      _myfile,d1          ; file handle
    move.l      #_header,d2         ; buffer address = &header
    move.l      #4,d3               ; read 4 bytes
    jsr         Read(a6)            ; AmigaDOS Read()
    sub.l       #4,_file_length     ; file_length -= 4
    movem.l     (sp)+,d1-d3/a0-a1
    rts
```

### `_ReadLength` — `fsubs.asm:639-656`

Reads 4 bytes from file into `blocklength`. Decrements `file_length` by `blocklength + 4` (the 4 bytes just read plus the chunk data that will follow).

```asm
_ReadLength
    movem.l     d1-d3/a0-a1,-(sp)
    move.l      _DOSBase,a6
    move.l      _myfile,d1          ; file handle
    move.l      #_blocklength,d2    ; buffer = &blocklength
    move.l      #4,d3               ; read 4 bytes
    jsr         Read(a6)            ; AmigaDOS Read()
    move.l      _blocklength,d1
    add.l       #4,d1               ; d1 = blocklength + 4
    sub.l       d1,_file_length     ; file_length -= (blocklength + 4)
    movem.l     (sp)+,d1-d3/a0-a1
    rts
```

**Global variables** (exported in `fsubs.asm:626`):
- `_file_length` — remaining bytes in FORM container
- `_blocklength` — size of current chunk data
- `_myfile` — AmigaDOS file handle
- `_header` — 4-byte chunk type ID just read

All declared as C globals in `iffsubs.c:22-24`:
```c
long file_length;
long myfile;
long header;
long blocklength;
```

## 3. `unpackbrush()` — Active IFF Brush Loader

**Definition**: `iffsubs.c:139-186`
**Prototype**: `iffsubs.p:2` — `int unpackbrush(unsigned char *filename, struct BitMap *bitmap, int x, int y);`

### Algorithm

1. **Open file**: `Open(filename, 1005)` — AmigaDOS mode `MODE_OLDFILE` — `iffsubs.c:142`
2. **Parse FORM header**: Read header, verify it equals `FORM`. Read length, set `file_length = blocklength` — `iffsubs.c:147-150`
3. **Chunk loop** (`while (file_length)`) — `iffsubs.c:152`:
   - **ILBM**: No-op (just a type identifier, no data) — `iffsubs.c:154`
   - **BMHD**: Read into global `bmhd` struct — `iffsubs.c:155-156`
   - **CAMG, CRNG, DEST, CMAP, GRAB**: **Skip** (Seek past `blocklength` bytes) — `iffsubs.c:157-159`
   - **BODY**: Decode image data — `iffsubs.c:160-182`
   - **Unknown chunk**: Close file, return FALSE — `iffsubs.c:183`
4. **Close file, return TRUE** — `iffsubs.c:185-186`

### BODY Decoding — `iffsubs.c:160-182`

```c
packdata = shape_mem;                                    /* line 163 */
bytecount = ((bmhd.width+15)/8) & 0xfffe;              /* line 164 — round up to word boundary */
ReadLength();                                            /* line 166 */
Read(myfile,packdata,blocklength);                       /* line 168 — bulk read compressed data */
```

1. **Buffer**: Uses `shape_mem` (78000 bytes, chip RAM) as temporary decompression buffer — `iffsubs.c:163`, `fmain.c:641,922`
2. **Byte width**: `((bmhd.width+15)/8) & 0xfffe` — rounds width to next word (16-bit) boundary in bytes — `iffsubs.c:164`
3. **Bulk read**: Reads entire BODY chunk into `shape_mem` in one call — `iffsubs.c:168`
4. **Bitplane pointers**: Set to `bitmap->Planes[N] + bitoffset` where `bitoffset = x + (bitmap->BytesPerRow) * y` — `iffsubs.c:141,170-174`
5. **Decompression loop**: For each scanline, decompress one line per bitplane — `iffsubs.c:176-181`:

```c
for (i=0; i<bmhd.height; i++)
{   if (bitmap->Depth > 0) unpack_line(plane0); plane0+=bitmap->BytesPerRow;
    if (bitmap->Depth > 1) unpack_line(plane1); plane1+=bitmap->BytesPerRow;
    if (bitmap->Depth > 2) unpack_line(plane2); plane2+=bitmap->BytesPerRow;
    if (bitmap->Depth > 3) unpack_line(plane3); plane3+=bitmap->BytesPerRow;
    if (bitmap->Depth > 4) unpack_line(plane4); plane4+=bitmap->BytesPerRow;
}
```

**Note on C operator precedence bug**: The `if (bitmap->Depth > 0) unpack_line(plane0);` and `plane0+=bitmap->BytesPerRow;` are **not** both conditional — the pointer advance always executes regardless of depth check, because there's no braces. This means all 5 plane pointers always advance, but `unpack_line` is only called for planes that exist. This is correct behavior — it just wastes pointer arithmetic on unused planes.

### Key Differences from `unpackpic`

| Aspect | `unpackbrush` | `unpackpic` |
|--------|--------------|-------------|
| CMAP handling | **Skipped** | Read into `cmap` struct |
| GRAB handling | **Skipped** | Read into `grab` struct |
| Buffer | `shape_mem` (pre-allocated) | `AllocMem(blocklength)` (dynamic) |
| Destination | `bitmap->Planes[N] + offset` | `bitmap->Planes[N]` (origin) |
| Plane stride | `bitmap->BytesPerRow` | Hardcoded `40` (320/8) |
| Byte width formula | `((width+15)/8) & 0xfffe` | `(width+7)/8` then `if (odd) ++` |
| Depth check | `if (bitmap->Depth > N)` | None (always 5 planes) |

## 4. `unpackpic()` — Ifdef'd-Out Full Picture Loader

**Definition**: `iffsubs.c:82-135`, wrapped in `#ifdef blarg` (never compiled).

Would have loaded a full-screen IFF ILBM picture:

1. Opens file, parses FORM/ILBM container — same as `unpackbrush`
2. **CMAP**: Reads color map into `cmap` struct (32 colors × 3 bytes RGB) — `iffsubs.c:99-100`
3. **GRAB**: Reads grab point into `grab` struct — `iffsubs.c:103-104`
4. **BODY**: Allocates temp buffer with `AllocMem(blocklength, 0)`, reads entire chunk, decompresses per-scanline into all 5 bitplanes with hardcoded stride of 40 bytes, then `FreeMem` — `iffsubs.c:105-127`
5. Plane stride is hardcoded to 40 bytes = 320 pixels / 8 — `iffsubs.c:119-123`

The `ColorMap` and `GrabPoint` structs are defined but their instances are commented out — `iffsubs.c:43-53`:

```c
typedef struct {
    unsigned char colors[32][3];
} ColorMap;
/* ColorMap cmap; */

typedef struct {
    short xgrab, ygrab;
} GrabPoint;
/* GrabPoint grab; */
```

## 5. `_unpack_line` — ByteRun1 Decompression

**Definition**: `fsubs.asm:1226-1271`
**Export**: `public _unpack_line,_compress,_bytecount,_packdata` — `fsubs.asm:1226`

This replaces the commented-out C implementation in `iffsubs.c:217-239`.

### Algorithm

Checks global `_compress` byte. If 0, uses uncompressed copy. If non-zero, uses ByteRun1.

#### Uncompressed path (`compress == 0`) — `fsubs.asm:1230-1242`:

```asm
    tst.b   _compress
    bne.s   unpack_line2
    move.l  4+4(sp),a0          ; a0 = dest pointer (function argument)
    move.l  _bytecount,d1       ; d1 = bytes per line
    move.l  _packdata,a1        ; a1 = source data pointer
    subq    #1,d1               ; bytecount - 1 (for dbra)
    bmi.s   upl20               ; skip if bytecount was 0
upl10:
    move.b  (a1)+,(a0)+         ; copy byte
    dbra    d1,upl10            ; loop
upl20:
    move.l  a1,_packdata        ; advance global pointer
```

Simple memcpy of `bytecount` bytes from `packdata` to `dest`.

#### ByteRun1 path (`compress != 0`) — `fsubs.asm:1244-1270`:

```asm
unpack_line2:
    move.l  4+4(sp),a0          ; a0 = dest
    move.l  _packdata,a1        ; a1 = source
    clr.l   d2                  ; d2 = j (bytes output counter)

upl30:
    clr.w   d0                  ; clear upper byte
    move.b  (a1)+,d0            ; d0.b = control byte
    bmi.s   upl35               ; if negative → repeat run

    ; LITERAL RUN: copy (d0+1) bytes
upl32:
    addq.l  #1,d2               ; j++
    move.b  (a1)+,(a0)+         ; *dest++ = *packdata++
    dbra    d0,upl32            ; loop d0+1 times

    bra.s   upl39               ; check if done

upl35:
    ; REPEAT RUN: repeat next byte (-d0+1) times
    neg.b   d0                  ; d0 = -d0 (/* branch overflow?? */ per Talin's comment)
    move.b  (a1)+,d3            ; d3 = byte to repeat
upl36:
    addq.l  #1,d2               ; j++
    move.b  d3,(a0)+            ; *dest++ = repeated byte
    dbra    d0,upl36            ; loop (-original_d0+1) times... but see note

upl39:
    cmp.l   _bytecount,d2       ; if j < bytecount, continue
    blo.s   upl30

upl40:
    move.l  a1,_packdata        ; advance global pointer
```

### IFF ByteRun1 Spec vs. Implementation

The standard IFF ByteRun1 compression defines:
- **n = 0..127**: Copy next (n+1) bytes literally
- **n = -1..-127**: Repeat next byte (-n+1) times
- **n = -128**: NOP (no operation)

The assembly implementation does **NOT** handle the -128 NOP case. Talin's comment `/* branch overflow?? */` on `neg.b d0` (`fsubs.asm:1258`) acknowledges this. When `d0.b = 0x80` (-128), `neg.b d0` produces `0x80` again (two's complement overflow: 0 - 0x80 = 0x80). Since `d0.w` was cleared to 0x0000 before `move.b`, `d0.w = 0x0080 = 128`. `dbra` then loops 129 times, writing garbage.

This is either:
- Intentional — Talin knew the compressor never emits -128 bytes
- A known potential bug that never triggers in practice

The commented-out C version at `iffsubs.c:223-239` **does** check for -128:
```c
else if (upc != -128)
{   upc = 1-upc; j += upc;
    while (upc--) *dest++ = *packdata;
    packdata++;
}
```

### Global Variables Used

| Variable | Type | Set by | Used by |
|----------|------|--------|---------|
| `_compress` | `char` | `unpackbrush` sets from `bmhd.compression` (`iffsubs.c:175`) | `_unpack_line` branch (`fsubs.asm:1230`) |
| `_bytecount` | `long` | `unpackbrush` computes from `bmhd.width` (`iffsubs.c:164`) | `_unpack_line` loop termination (`fsubs.asm:1266`) |
| `_packdata` | `char*` | `unpackbrush` sets to buffer address (`iffsubs.c:163`), then advanced by `_unpack_line` each call (`fsubs.asm:1241,1269`) | `_unpack_line` source pointer |

## 6. IFF Chunk Types Handled

| Chunk | `unpackpic` (ifdef'd out) | `unpackbrush` (active) | Description |
|-------|--------------------------|----------------------|-------------|
| FORM | Validate as container ID | Validate as container ID | IFF container marker |
| ILBM | No-op (subtype marker) | No-op (subtype marker) | Image subtype |
| BMHD | Read into `bmhd` | Read into `bmhd` | Bitmap header (dimensions, compression) |
| CMAP | Read into `cmap` | **Skip** | Color palette (32 × RGB) |
| GRAB | Read into `grab` | **Skip** | Hotspot/grab point |
| BODY | Decompress into bitmap | Decompress into bitmap | Interleaved bitplane data |
| CAMG | Skip | Skip | Amiga viewport mode |
| CRNG | Skip | Skip | Color cycling range |
| DEST | Skip | Skip | Destination merge (referenced but `DEST` symbol undefined — see §1 note) |

### `BitMapHeader` struct — `iffsubs.c:29-41`

```c
typedef struct {
    short   width, height;       /* image dimensions in pixels */
    short   xpic, ypic;         /* image position */
    UBYTE   nPlanes;            /* number of bitplanes */
    UBYTE   masking;            /* masking type */
    UBYTE   compression;        /* 0=none, 1=ByteRun1 */
    UBYTE   pad1;
    short   transcolor;         /* transparent color index */
    short   xAspect, yAspect;  /* pixel aspect ratio */
    short   pageWidth, pageHeight; /* source page dimensions */
} BitMapHeader;
```

Compression constants — `iffsubs.c:26-27`:
```c
#define cmpNone     0
#define cmpByteRun1 1
```

## 7. Color Palette Loading (Ifdef'd-Out Functions)

### `fade_map(level)` — `iffsubs.c:243-250` (commented out)

Fades all 32 palette colors based on `level` parameter:
```c
for (i=0; i < 32; i++)
{   red   = (level * cmap.colors[i][0])/(16*160);
    green = (level * cmap.colors[i][1])/(16*160);
    blue  = (level * cmap.colors[i][2])/(16*160);
    SetRGB4(vp,i,red,green,blue);
}
```

Each RGB byte (0-255 from IFF) is scaled by `level / (16*160)`. At `level = 16*160 = 2560`, full brightness. `SetRGB4` takes 4-bit (0-15) color values per the Amiga API.

The divisor `16*160 = 2560` converts 8-bit IFF color (0-255 range × level) to 4-bit Amiga color (0-15). When `level = 2560 / 255 ≈ 10.04`, one unit of IFF color maps to one unit of Amiga color.

### `low_fade(level)` — `iffsubs.c:252-260` (commented out)

Fades only the first 16 colors, and applies the same values to colors 16-31:
```c
for (i=0; i < 16; i++)
{   /* ... same formula ... */
    SetRGB4(vp,i,red,green,blue);
    SetRGB4(vp,i+16,red,green,blue);   /* mirror to upper 16 */
}
```

### `high_fade(level)` — `iffsubs.c:262-271` (commented out)

Fades colors 16-31 **toward white** instead of toward black:
```c
red   = 15 - (level * (255 - cmap.colors[i][0]) ) / (16*160);
green = 15 - (level * (255 - cmap.colors[i][1]) ) / (16*160);
blue  = 15 - (level * (255 - cmap.colors[i][2]) ) / (16*160);
SetRGB4(vp,i+16,red,green,blue);
```

At level=0: all colors = 15 (white). As level increases, colors approach their true values.

### `erasebrush(bitmap,x,y)` — `iffsubs.c:196-210` (ifdef'd out under `#ifdef blarg`)

Zeroes the bitmap region previously drawn by a brush. Uses the global `bmhd` from the last loaded brush for width/height. Hardcoded stride of 40 bytes. Calls `erase_line(dest)` which zeroes `bytecount` bytes — `iffsubs.c:211-213`.

## 8. Game Files Loaded as IFF Brushes

Based on all `unpackbrush()` call sites:

| File | Call site | Destination bitmap | Position | Purpose |
|------|-----------|-------------------|----------|---------|
| `page0` | `fmain.c:1194` | `&pageb` (back buffer) | (0,0) | Title/intro background |
| `p1a` | `fmain.c:1203` via `copypage` | `&pageb` | (4,24) | Intro story page 1 background |
| `p1b` | `fmain.c:1203` via `copypage` | `&pageb` | (21,29) | Intro story page 1 overlay |
| `p2a` | `fmain.c:1204` via `copypage` | `&pageb` | (4,24) | Intro story page 2 background |
| `p2b` | `fmain.c:1204` via `copypage` | `&pageb` | (20,29) | Intro story page 2 overlay |
| `p3a` | `fmain.c:1205` via `copypage` | `&pageb` | (4,24) | Intro story page 3 background |
| `p3b` | `fmain.c:1205` via `copypage` | `&pageb` | (20,33) | Intro story page 3 overlay |
| `hiscreen` | `fmain.c:1227` | `bm_text` | (0,0) | HUD/status bar graphics |
| `winpic` | `fmain2.c:1609` | `bm_draw` | (0,0) | Win screen image |

The `copypage()` function at `fmain2.c:782-789` is a helper that loads two brushes in sequence with a screen flip transition:
```c
copypage(br1,br2,x,y) char *br1, *br2; short x,y;
{   if (skipp) return;
    Delay(350);
    BltBitMap(&pageb,0,0,&pagea,0,0,320,200,0xC0,0x1f,0);
    unpackbrush(br1,&pageb,4,24);   /* background brush always at (4,24) */
    unpackbrush(br2,&pageb,x,y);    /* overlay brush at variable position */
    if (skipint()) return;
    flipscan();
    skipint();
}
```

All these files reside in the `game/` directory. The game opens files by bare filename, relying on the AmigaDOS current directory being set to `game/` at startup.

### Other game/ files (NOT IFF format)

| File | Format | Loaded by |
|------|--------|-----------|
| `image` | Raw sector/terrain data | `hdrive.c` disk I/O |
| `songs` | Music score data | `mtrack.c` |
| `v6` | Voice/sample data | Direct disk reads |
| `fonts/` | Amiga font data | `OpenDiskFont()` |
| `C.faery`, `E.faery` | Save game files | `fmain2.c` save/load |
| `fmain` | Executable | AmigaDOS |

## Cross-Cutting Findings

- **`shape_mem` dual use**: This 78000-byte chip RAM buffer (`fmain.c:641,922`) is used by BOTH sprite shape loading (`fmain2.c:674`) and IFF brush decompression (`iffsubs.c:163`). Since `unpackbrush` is only called during intro sequence and win screen (not during gameplay), there's no conflict — but this means brush loading cannot happen while sprites are cached.

- **`copypage()` intro presentation system** (`fmain2.c:782-789`): The intro sequence uses `unpackbrush` within a display transition system (`flipscan` at `fmain2.c:795-817`) that does scanline-based reveals. The IFF loader is a building block of the visual presentation pipeline.

- **No runtime palette from IFF**: The active `unpackbrush` function **skips CMAP chunks**. All palettes in the running game are set programmatically via `LoadRGB4()` and `SetRGB4()` calls in `fmain.c` and `fmain2.c`, not loaded from image files.

## Unresolved

- **`DEST` symbol**: Referenced at `iffsubs.c:101,157` in chunk skip lists but never `#define`d. Likely a leftover from earlier code or defined in an Aztec C system header not preserved. Cannot determine from available source.
- **Actual content of IFF files**: Whether the game/ IFF files use ByteRun1 compression or uncompressed data cannot be determined from source code alone — would require examining the binary files.
- **`-128 NOP` handling in `_unpack_line`**: Whether the omission of the -128 NOP case in the assembly decompressor is intentional (compressor never emits it) or a latent bug cannot be determined from source code. Talin's comment `/* branch overflow?? */` at `fsubs.asm:1258` suggests awareness of the issue.

## Refinement Log

- 2026-04-06: Initial discovery pass — complete trace of iffsubs.c, fsubs.asm IFF routines, all call sites, ifdef'd-out code
