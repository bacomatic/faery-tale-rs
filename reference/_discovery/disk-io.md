# Discovery: Disk I/O & Region Loading

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace the entire disk I/O and asset loading system — IO request management, async I/O, raw sector reading, hard drive detection, region-to-block mapping, region loading sequence, async loading pipeline, sprite loading, and the floppy vs hard drive code paths.

## 1. IO Request Management

### AllocDiskIO — `hdrive.c:30-54`

Initializes disk I/O. Detects hard drive vs floppy via `Lock("image", ACCESS_READ)`:

- **Hard drive path**: If `Lock("image")` succeeds, sets `hdrive = TRUE`, opens `"image"` file via `Open("image", MODE_OLDFILE)`. All subsequent I/O uses AmigaDOS `Seek()`/`Read()` on this file handle.
- **Floppy path**: If lock fails, creates a message port (`CreatePort`), creates an extended I/O request (`CreateExtIO` of `sizeof(struct IOExtTD)`), opens `trackdisk.device` unit 0 (`OpenDevice(TD_NAME, 0, ...)`), then clones the template request into `diskreqs[0..8]` (9 copies: `*diskreq1` copied to each).

The allocation flags (`AL_PORT`, `AL_IOREQ`, `AL_TDISK`) track what was successfully allocated for cleanup.

```c
struct MsgPort *diskport;
struct IOExtTD *diskreq1, diskreqs[10], *lastreq;
```
— `hdrive.c:26-27`

**9 active diskreqs** (indices 0–8) plus 1 spare (index 9). Comment says: "5 landscape, 2 Terrain, 1 map, 1 sector, 1 monster" — `hdrive.c:25`.

### FreeDiskIO — `hdrive.c:56-63`

Cleanup: closes the file handle if hard drive, else closes device/IO/port in reverse order using `openflags` tests.

## 2. Async I/O Functions

All functions in `hdrive.c` branch on `hdrive`:

| Function | Floppy behavior | Hard drive behavior | Line |
|---|---|---|---|
| `WaitDiskIO(num)` | `WaitIO(&diskreqs[num])` — blocks until request completes | No-op (reads are synchronous) | `hdrive.c:65-69` |
| `InvalidDiskIO(num)` | Sets `diskreqs[num].io_Command = CMD_INVALID` | No-op | `hdrive.c:71-75` |
| `CheckDiskIO(num)` | `CheckIO(&diskreqs[num])` — non-blocking completion check | Returns `TRUE` always | `hdrive.c:77-83` |
| `IsReadDiskIO(num)` | Tests if `diskreqs[num].io_Command == CMD_READ` | Returns `FALSE` always | `hdrive.c:85-91` |
| `WaitLastDiskIO()` | `WaitIO(lastreq)` | No-op | `hdrive.c:93-97` |
| `InvalidLastDiskIO()` | `lastreq->io_Command = CMD_INVALID` | No-op | `hdrive.c:99-103` |
| `CheckLastDiskIO()` | `CheckIO(lastreq)` | Returns `TRUE` | `hdrive.c:105-111` |
| `IsReadLastDiskIO()` | Tests `lastreq->io_Command == CMD_READ` | Returns `FALSE` | `hdrive.c:113-119` |

**Design pattern**: On hard drive, all loads are synchronous (no async I/O). The "check/wait" functions become no-ops or return "done". On floppy, each `diskreqs[N]` slot tracks an independent async DMA transfer via `SendIO()`.

## 3. load_track_range — `hdrive.c:121-140`

Core low-level read function. Reads `b_count` 512-byte blocks starting at block `f_block` into `buffer`, using request slot `dr`.

```c
load_track_range(f_block, b_count, buffer, dr)
short f_block, b_count, dr; APTR buffer;
```

**Floppy path** (`hdrive.c:125-134`):
1. Sets `lastreq = &diskreqs[dr]`
2. If the slot has a pending CMD_READ, waits for it: `WaitIO(lastreq)`
3. Clones template: `*lastreq = *diskreq1`
4. Sets `io_Length = b_count * 512`, `io_Data = buffer`, `io_Command = CMD_READ`, `io_Offset = f_block * 512`
5. Sends asynchronously: `SendIO(lastreq)`

**Hard drive path** (`hdrive.c:136-139`):
1. `Seek(file, f_block * 512, OFFSET_BEGINNING)`
2. `Read(file, buffer, b_count * 512)`

Both paths address the `game/image` binary using absolute 512-byte block offsets. All block numbers in `file_index` and `cfiles` are indices into this file.

**Note**: The `#if 0` block in `fmain2.c:704-727` contains the original inline version of `load_track_range` and `motor_off` that was replaced by `hdrive.c`. This confirms the refactoring history.

## 4. IsHardDrive — `hdrive.c:152-155`

Simply returns the static `hdrive` boolean. Called by `cpytest()` in `fmain2.c:1411` to decide which copy-protection path to take.

## 5. struct need & file_index[10]

### struct need — `ftale.h:104-106`

```c
struct need {
    USHORT image[4], terra1, terra2, sector, region, setchar;
};
```

9 USHORT fields (18 bytes per entry). Maps a region to the disk blocks needed:
- `image[4]` — 4 landscape tile image block start offsets (each loaded as 5 planes × 8 blocks = 40 blocks)
- `terra1`, `terra2` — terrain attribute table indices (added to `TERRA_BLOCK` = 149 to get block offset)
- `sector` — sector data block start (loaded as 64 blocks = 32KB)
- `region` — region map block start (loaded as 8 blocks = 4KB)
- `setchar` — **UNUSED in code**. Defined in struct and initialized in `file_index`, but never read by any function. Possibly vestigial NPC set indicator.

### file_index[10] — `fmain.c:615-625`

```c
struct need file_index[10] = {
    { 320,480,520,560,  0,1, 32,160,22 }, /* F1 - snowy region */
    { 320,360,400,440,  2,3, 32,160,21 }, /* F2 - witch wood */
    { 320,360,520,560,  2,1, 32,168,22 }, /* F3 - swampy region */
    { 320,360,400,440,  2,3, 32,168,21 }, /* F4 - plains and rocks */
    { 320,480,520,600,  0,4, 32,176, 0 }, /* F5 - desert area */
    { 320,280,240,200,  5,6, 32,176,23 }, /* F6 - bay / city / farms */
    { 320,640,520,600,  7,4, 32,184, 0 }, /* F7 - volcanic */
    { 320,280,240,200,  5,6, 32,184,24 }, /* F8 - forest and wilderness */
    { 680,720,800,840,  8,9, 96,192, 0 }, /* F9  - inside of buildings */
    { 680,760,800,840, 10,9, 96,192, 0 }  /* F10 - dungeons and caves */
};
```

**Observations**:
- All outdoor regions (F1–F8) share image[0] = 320 — a common base tileset.
- Regions F6 and F8 share identical images (320,280,240,200) and terrain (5,6).
- Indoor regions F9/F10 use different image sets starting at 680.
- All outdoor regions use sector block 32; indoor regions use 96.
- Outdoor region maps start at blocks 160, 168, 176, 184 (spaced 8 apart); indoor at 192.

### current_loads — `fmain.c:614`

```c
struct need current_loads = { 0,0,0,0, 1,2,0,0,0 };
```

Tracks what's currently loaded in memory. Each field is compared to the target region's `file_index` entry — only changed assets are reloaded. Initial values (terra1=1, terra2=2) ensure the first load always triggers since no file_index entry uses these specific terra values.

## 6. load_all / load_new_region

### load_all — `fmain.c:3545-3546`

```c
load_all()
{   while (MAP_FLUX) load_new_region(); }
```

Blocking loop: calls `load_new_region()` repeatedly until `new_region >= NO_REGION` (i.e., all loading complete). `MAP_FLUX` = `(new_region < NO_REGION)` — `fmain.c:611`.

### load_new_region — `fmain.c:3548-3614`

Incrementally loads region assets. Called once per game-loop tick via `load_next()`, or looped by `load_all()`.

**Loading sequence** (each step only if the asset differs from `current_loads`):

1. **Sector data** (`fmain.c:3555-3558`): `load_track_range(nd->sector, 64, sector_mem, 0)` — 64 blocks (32KB) into `sector_mem`, using diskreq slot 0.
2. **Region map** (`fmain.c:3560-3563`): `load_track_range(nd->region, 8, map_mem, 0)` — 8 blocks (4KB) into `map_mem`, using diskreq slot 0. (Reuses slot 0, so this implicitly waits for sector data on floppy.)
3. **Terrain table 1** (`fmain.c:3565-3568`): `load_track_range(TERRA_BLOCK + nd->terra1, 1, terra_mem, 1)` — 1 block (512 bytes), slot 1.
4. **Terrain table 2** (`fmain.c:3570-3573`): `load_track_range(TERRA_BLOCK + nd->terra2, 1, terra_mem+512, 2)` — 1 block (512 bytes), slot 2.
5. **Landscape images** (`fmain.c:3576-3591`): For each of the 4 image quadrants, if changed: loads 5 planes × 8 blocks each = 40 blocks (20KB) per quadrant into `image_mem`, using slots 3–7. **Returns after loading one quadrant** — the function must be called again for the next quadrant. This spreads image loading across multiple game loop ticks on floppy.

**Completion** (`fmain.c:3593-3613`):
- If all assets match (no more loading needed), patches desert access if in region 4 and `stuff[STATBASE] < 5` (sets map tiles to 254 at position (26,11)).
- Waits for and invalidates all 7 IO requests (slots 0–6).
- Calls `motor_off()`.
- Sets `region_num = new_region; new_region = NO_REGION`.

### Constants — `fmain.c:608-611, 638-641`

| Constant | Value | Meaning | Line |
|---|---|---|---|
| `TERRA_BLOCK` | 149 | Base block offset for terrain tables | `fmain.c:608` |
| `NO_REGION` | 10 | Sentinel: no region change pending | `fmain.c:609` |
| `QPLAN_SZ` | 4096 | 1 plane of 64 chars (64×64 bytes) | `fmain.c:638` |
| `IPLAN_SZ` | 16384 | 1 plane of 256 chars (256×64 bytes) | `fmain.c:639` |
| `IMAGE_SZ` | 81920 | 5 planes × 256 chars = IPLAN_SZ × 5 | `fmain.c:640` |
| `SECTOR_SZ` | 36864 | 128×256 + 4096 = 32K sectors + 4K region map | `fmain.c:643` |
| `SECTOR_OFF` | 32768 | Offset of map_mem within sector_mem | `fmain.c:644` |

## 7. load_next — `fmain2.c:752-755`

```c
load_next()
{   if (!IsReadLastDiskIO() || CheckLastDiskIO())
        load_new_region();
}
```

Called once per game loop tick at `fmain.c:1987`: `if (MAP_FLUX) load_next()`.

**Logic**: Only calls `load_new_region()` if the last disk request is not a pending read (either it's done, was never a read, or on hard drive where `IsReadLastDiskIO()` returns FALSE). This prevents stalling the game loop waiting for floppy I/O — it skips the load tick if the disk is still busy, letting gameplay continue.

On hard drive, `IsReadLastDiskIO()` always returns FALSE, so `load_new_region()` is called every tick during `MAP_FLUX` — all loading completes almost instantly since reads are synchronous.

## 8. seekn — `fmain2.c:730-741`

```c
seekn()
{   cpytest();
}
```

The body is entirely commented out (the actual disk seek code is in comments). The function now only calls `cpytest()` — the copy protection check. The commented-out code would have read block 0 into `shape_mem` using slot 9 as a seek/verify operation, followed by `prot2()` and `motor_off()`.

**`cpytest()`** — `fmain2.c:1409-1435`:
- On floppy: locks `df0:`, reads the `FileLock` → `DeviceList` → checks `dl_VolumeDate.ds_Tick == 230`. If mismatch, calls `cold()` (jump to address -4, effectively crashing).
- On hard drive: uses `load_track_range(880, 1, buffer, 0)` to read block 880 from the `image` file, checks `buffer[123] == 230`. If mismatch, calls `close_all()`.

## 9. motor_off — `hdrive.c:142-150`

```c
motor_off()
{   if (hdrive == FALSE)
    {   diskreqs[9] = *diskreq1;
        diskreqs[9].iotd_Req.io_Length = 0;
        diskreqs[9].iotd_Req.io_Command = TD_MOTOR;
        DoIO((struct IORequest *)&diskreqs[9]);
    }
}
```

Uses diskreq slot 9 (the spare). Sends `TD_MOTOR` with `io_Length = 0` to turn the floppy drive motor off. Synchronous (`DoIO`). No-op on hard drive. Called after region loading completes (`fmain.c:3609`), after `shape_read` (`fmain2.c:682`), and after forced encounter loading (`fmain.c:2698, 2707`).

## 10. cfiles[18] — Sprite Set Disk Map

### Definition — `fmain2.c:639-657`

```c
struct {
    UBYTE   width, height, count;
    UBYTE   numblocks;
    UBYTE   seq_num;
    USHORT  file_id;
} cfiles[];
```

18 entries mapping sprite sets to disk locations:

| Index | Description | W×H | Count | Blocks | Seq slot | Block ID | Line |
|---|---|---|---|---|---|---|---|
| 0 | Julian | 1×32×67 | 42 | PHIL | 1376 | `fmain2.c:646` |
| 1 | Phillip | 1×32×67 | 42 | PHIL | 1418 | `fmain2.c:647` |
| 2 | Kevin | 1×32×67 | 42 | PHIL | 1460 | `fmain2.c:648` |
| 3 | Objects | 1×16×116 | 36 | OBJECTS | 1312 | `fmain2.c:649` |
| 4 | Raft | 2×32×2 | 3 | RAFT | 1348 | `fmain2.c:650` |
| 5 | Turtle | 2×32×16 | 20 | CARRIER | 1351 | `fmain2.c:652` |
| 6 | Ogre | 1×32×64 | 40 | ENEMY | 960 | `fmain2.c:653` |
| 7 | Ghost | 1×32×64 | 40 | ENEMY | 1080 | `fmain2.c:654` |
| 8 | Dark knight (spiders) | 1×32×64 | 40 | ENEMY | 1000 | `fmain2.c:655` |
| 9 | Necromancer (farmer/loraii) | 1×32×64 | 40 | ENEMY | 1040 | `fmain2.c:656` |
| 10 | Dragon | 3×40×5 | 12 | DRAGON | 1160 | `fmain2.c:658` |
| 11 | Bird | 4×64×8 | 40 | CARRIER | 1120 | `fmain2.c:659` |
| 12 | Snake/salamander | 1×32×64 | 40 | ENEMY | 1376 | `fmain2.c:660` |
| 13 | Wizard/priest | 1×32×8 | 5 | SETFIG | 936 | `fmain2.c:661` |
| 14 | Royal set | 1×32×8 | 5 | SETFIG | 931 | `fmain2.c:662` |
| 15 | Bartender | 1×32×8 | 5 | SETFIG | 941 | `fmain2.c:663` |
| 16 | Witch | 1×32×8 | 5 | SETFIG | 946 | `fmain2.c:664` |
| 17 | Ranger/beggar | 1×32×8 | 5 | SETFIG | 951 | `fmain2.c:665` |

**Note**: `width` here is a multiplier (words?), actual pixel width = width × 16.

### Sequence slot enum — `ftale.h:88`
```c
enum sequences {PHIL, OBJECTS, ENEMY, RAFT, SETFIG, CARRIER, DRAGON};
```
Values: PHIL=0, OBJECTS=1, ENEMY=2, RAFT=3, SETFIG=4, CARRIER=5, DRAGON=6.

## 11. shape_read / read_shapes — Sprite Loading Pipeline

### shape_read — `fmain2.c:673-683`

Loads the initial sprite set. Called at startup, on brother succession, and after save/load.

```c
shape_read()
{   nextshape = shape_mem;
    read_shapes(3); prep(OBJECTS);       // objects (cfiles[3])
    read_shapes(brother-1); prep(PHIL);  // current brother (cfiles[0/1/2])
    read_shapes(4); prep(RAFT);          // raft (cfiles[4])
    seq_list[ENEMY].location = nextshape;
    read_shapes(actor_file); prep(cfiles[actor_file].seq_num);  // enemy
    read_shapes(set_file); prep(SETFIG); // NPC set
    new_region = region_num; load_all(); // reload region assets
    motor_off();
}
```

Each call to `read_shapes(N)` loads sprite data from disk; each `prep(slot)` blocks on the async read (diskreq 8) completing, then generates the mask plane via `make_mask()`.

### read_shapes — `fmain2.c:685-703`

```c
read_shapes(num)
{   slot = cfiles[num].seq_num;
    seq_list[slot].bytes = height * width * 2;
    size = seq_list[slot].bytes * count;
    seq_list[slot].location = nextshape;
    // ... set width, height, count ...
    if ((nextshape + size*6) <= (shape_mem + SHAPE_SZ))
    {   load_track_range(cfiles[num].file_id, cfiles[num].numblocks, nextshape, 8);
        nextshape += size*5;         // 5 bitplanes of sprite data
        seq_list[slot].maskloc = nextshape;
        nextshape += size;           // 1 mask plane
    }
}
```

- Uses diskreq slot 8 exclusively for shape loading.
- Memory layout: 5 bitplanes contiguous, then 1 mask plane.
- `SHAPE_SZ = 78000` bytes total shape memory — `fmain2.c:668`.
- Boundary check prevents overflow: `nextshape + size*6 <= shape_mem + SHAPE_SZ`.

### prep — `fmain2.c:743-750`

```c
prep(slot)
{   WaitDiskIO(8);
    InvalidDiskIO(8);
    make_mask(seq_list[slot].location, seq_list[slot].maskloc,
        seq_list[slot].width, seq_list[slot].height, seq_list[slot].count);
}
```

Waits for the shape read to complete, then calls `make_mask` (in `fsubs.asm:1619`) to generate the transparency mask from the 5-bitplane sprite data.

### Async Actor Loading — `fmain.c:2722-2730`

Enemies can also be loaded asynchronously during gameplay:
```c
load_actors()
{   ...
    read_shapes(actor_file);
    actors_loading = TRUE;
    ...
}
```
— `fmain.c:2722-2730`

The `prep(ENEMY)` call is deferred to the main loop: when `actors_loading == TRUE && CheckDiskIO(8)` — `fmain.c:2052-2055`. This allows the game to continue rendering while enemy sprites load from floppy.

For forced encounters (xtype >= 50), loading is synchronous: `load_actors(); prep(ENEMY); motor_off();` — `fmain.c:2698, 2707`.

## 12. read_score — Music Loading

### read_score — `fmain2.c:759-774`

```c
read_score()
{   if (file = Open("songs", 1005))
    {   for (i=0; i<(4*7); i++)     // up to 28 tracks
        {   Read(file, &packlen, 4);
            if ((packlen*2 + sc_load) > 5900) break;
            track[sc_count] = scoremem + sc_load;
            sc_count++;
            Read(file, scoremem+sc_load, packlen*2);
            sc_load += (packlen*2);
        }
        Close(file);
    }
}
```

Reads from `game/songs` file (not the `image` binary). Each track entry: 4-byte length prefix (in words), then `packlen * 2` bytes of track data. Loads up to 28 tracks into `scoremem` (max 5900 bytes total). Uses AmigaDOS file I/O, not `load_track_range`.

## 13. Floppy vs Hard Drive Code Paths

The system does **not** use `#ifdef HDRIVE` compile-time branching. Instead, `hdrive.c` uses a runtime boolean (`static BOOL hdrive`) set during `AllocDiskIO()`. Every function checks `if (hdrive == FALSE)` to select the code path.

**Key differences**:

| Aspect | Floppy | Hard Drive |
|---|---|---|
| Detection | `Lock("image")` fails | `Lock("image")` succeeds |
| Init | `CreatePort` + `CreateExtIO` + `OpenDevice(TD_NAME)` | `Open("image", MODE_OLDFILE)` |
| Read | Async `SendIO` per diskreq slot | Sync `Seek` + `Read` on file handle |
| Completion check | `CheckIO` / `WaitIO` per slot | Always returns "done" |
| Motor control | `TD_MOTOR` via `DoIO` | No-op |
| Parallelism | Up to 9 async reads in flight | Sequential reads only |
| Game loop impact | `load_next()` skips tick if disk busy | `load_next()` always completes immediately |

**Savegame I/O** (`fmain2.c:1474-1540`) also detects hard drive independently using a local `hdrive` variable and `locktest("image")`. It probes drives in order: hard drive → df1: → df0: (excluding game disk). After save/load on floppy, it loops waiting for the game disk to be reinserted: `Lock("df0:winpic")` — `fmain2.c:1535-1540`.

## 14. Memory Buffers

All loaded from `game/image` unless noted:

| Buffer | Size | Alloc flag | Contents | Line |
|---|---|---|---|---|
| `image_mem` | 81920 (IMAGE_SZ) | `AL_IMAGE` | 5-plane landscape tiles (also used for double-buffered pages) | `fmain.c:917` |
| `sector_mem` | 36864 (SECTOR_SZ) | `AL_SECTOR` | 32K sector data + 4K region map | `fmain.c:919` |
| `map_mem` | — | — | Points to `sector_mem + 32768` | `fmain.c:920` |
| `shape_mem` | 78000 (SHAPE_SZ) | `AL_SHAPE` | All sprite data (5 planes + masks) | `fmain.c:922` |
| `terra_mem` | 1024 | `AL_TERR` | Two 512-byte terrain attribute tables | `fmain.c:928` |
| `shadow_mem` | SHADOW_SZ | `AL_SHADOW` | Shadow/overlay data | `fmain.c:924` |
| `sample_mem` | 5632 (SAMPLE_SZ) | `AL_SAMPLE` | Sound effect samples | `fmain.c:926` |
| `scoremem` | SCORE_SZ | — | Music track data (from `songs` file) | `fmain.c:913` |
| `wavmem` | VOICE_SZ | `AL_MUSIC` | Waveform + volume tables (from `v6` file) | `fmain.c:911` |

## 15. Diskreq Slot Assignment

| Slot | Usage | Loaded by |
|---|---|---|
| 0 | Sector data, then region map | `load_new_region` |
| 1 | Terrain table 1 | `load_new_region` |
| 2 | Terrain table 2 | `load_new_region` |
| 3–7 | Landscape image planes (5 planes per quadrant) | `load_new_region` |
| 8 | Shape/sprite data | `read_shapes` |
| 9 | Motor control / spare | `motor_off`, `cpytest` |

## Cross-Cutting Findings

- **Copy protection via disk I/O**: `seekn()` (`fmain2.c:730`) calls `cpytest()` which uses `load_track_range(880,1,...)` on hard drive to read block 880 and verify a magic value at offset 123 == 230. On floppy, it checks `dl_VolumeDate.ds_Tick == 230`. — `fmain2.c:1409-1435`.
- **Desert gate via map patching**: `load_new_region` patches `map_mem` when loading region 4 if `stuff[STATBASE] < 5`, overwriting tiles at position (26,11) with value 254. This is a quest gate in the terrain data. — `fmain.c:3594-3596`.
- **`setchar` field unused**: The `setchar` field in `struct need` is initialized in `file_index` but never read by any code in `.c` or `.asm` files. Values (22,21,22,21,0,23,0,24,0,0) suggest it was planned for automatic NPC set changes per region but was never wired up.
- **Reuse of `image_mem` for display**: `image_mem` serves double duty — it holds loaded landscape tile data but is also used as the framebuffer: `pagea.Planes[i] = (pageb.Planes[i] = image_mem + (i*8000)) + 40000` — `fmain.c:1179`. This means landscape data and display pages occupy the same 80KB chip RAM allocation.
- **`actors_loading` flag**: Sprites can load asynchronously on floppy. The main game loop checks `CheckDiskIO(8)` to call `prep(ENEMY)` when the load completes — `fmain.c:2052-2055`. This allows gameplay to continue during sprite loading.

## Unresolved

- **`setchar` purpose**: The values (22,21,22,21,0,23,0,24,0,0) in `file_index` correlate with some of the `cfiles` indices but the field is never read. Whether this was cut functionality or a lookup table for another tool cannot be determined from source code alone.
- **`image_mem` overlap with display pages**: The landscape tiles are loaded into `image_mem`, but the display bitplanes also point into `image_mem`. How these coexist without corruption (or whether the tile data IS the display data) requires deeper investigation of the rendering pipeline — see display-system.md and map-rendering.md.

## Refinement Log
- 2026-04-06: Initial comprehensive discovery pass. All 12 questions answered with full citations.
