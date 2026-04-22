# Discovery: Disk Layout Tools (mtrack.c, rtrack.c)

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace the custom disk formatting tools mtrack.c and rtrack.c — how they format raw game disks, what data goes where, and how the game runtime reads it back.

## 1. Overview

Two stand-alone tools write game assets to raw Amiga floppy disk sectors, bypassing the AmigaOS filesystem. This gives the game direct block-level access for fast, predictable loading without filesystem overhead.

- **mtrack.c** — Formats **game disk 1** (drive 0, df0:). The primary and only disk used at runtime. Contains ALL game data: tile images, sectors, terrain, map, character sprites, masks, and audio samples.
- **rtrack.c** — Formats **game disk 2** (drive 1, df1:). Contains a subset of tile/terrain data only (no sprites, masks, or samples). Appears to be an earlier or alternative build tool; the shipped game runs entirely from disk 1.

Evidence: `mtrack.c:86` calls `OpenDevice(TD_NAME, 0, diskreq, 0)` (drive 0); `rtrack.c:55` calls `OpenDevice(TD_NAME, 1, diskreq, 0)` (drive 1). The game runtime opens only drive 0: `hdrive.c:48` and `fmain.c:767`.

## 2. mtrack.c — Game Disk 1 Formatter

### 2.1 Structure

`mtrack.c` defines two data tables and writes their contents to raw disk blocks on drive 0.

### 2.2 diskmap[25] — Main Asset Map (mtrack.c:25-57)

Each entry maps a source file (on the development hard drive `dh0:`) to a destination block range on the floppy:

| # | Source File | Block Start | Block Count | Bytes | Content Type |
|---|---|---|---|---|---|
| 0 | `f6a` | 32 | 64 | 32,768 | Sector data (regions 0–5, 8) |
| 1 | `f9a` | 96 | 64 | 32,768 | Sector data (regions 6–7, 9) |
| 2 | `map1` | 160 | 40 | 20,480 | Region map data (regions 0–7) |
| 3 | `wild` | 320 | 40 | 20,480 | Tile image: wilderness |
| 4 | `build` | 360 | 40 | 20,480 | Tile image: buildings |
| 5 | `rock` | 400 | 40 | 20,480 | Tile image: rocks |
| 6 | `mountain1` | 200 | 40 | 20,480 | Tile image: mountain type 1 |
| 7 | `tower` | 440 | 40 | 20,480 | Tile image: tower |
| 8 | `castle` | 280 | 40 | 20,480 | Tile image: castle |
| 9 | `field` | 240 | 40 | 20,480 | Tile image: field |
| 10 | `swamp` | 520 | 40 | 20,480 | Tile image: swamp |
| 11 | `palace` | 480 | 40 | 20,480 | Tile image: palace |
| 12 | `mountain2` | 560 | 40 | 20,480 | Tile image: mountain type 2 |
| 13 | `doom` | 640 | 40 | 20,480 | Tile image: doom/volcanic |
| 14 | `mountain3` | 600 | 40 | 20,480 | Tile image: mountain type 3 |
| 15 | `under` | 680 | 40 | 20,480 | Tile image: underground |
| 16 | `cave` | 760 | 40 | 20,480 | Tile image: cave |
| 17 | `furnish` | 720 | 40 | 20,480 | Tile image: furnishings |
| 18 | `inside` | 800 | 40 | 20,480 | Tile image: inside |
| 19 | `astral` | 840 | 40 | 20,480 | Tile image: astral plane |
| 20 | `terra` | 149 | 11 | 5,632 | Terrain attribute data (T1–T11) |
| 21 | `mask` | 896 | 8 | 4,096 | Background mask 1 |
| 22 | `mask2` | 904 | 8 | 4,096 | Background mask 2 |
| 23 | `mask3` | 912 | 8 | 4,096 | Background mask 3 |
| 24 | `dh0:z/samples` | 920 | 11 | 5,632 | Audio samples |

Source: `mtrack.c:27-56`

### 2.3 char_map[17] — Character Sprite Map (mtrack.c:60-79)

Each entry maps a character sprite file to a disk block range. These use `transfer_char_file()` which skips a 6-byte header before reading:

| # | Source File | Block Start | Block Count | Bytes | Content / cfiles index |
|---|---|---|---|---|---|
| 0 | `dh0:z/julian` | 1376 | 42 | 21,504 | Brother Julian sprites |
| 1 | `dh0:z/phillip` | 1418 | 42 | 21,504 | Brother Phillip sprites |
| 2 | `dh0:z/kevin` | 1460 | 42 | 21,504 | Brother Kevin sprites |
| 3 | `dh0:z/objects` | 1312 | 36 | 18,432 | Object sprites (items, etc.) |
| 4 | `dh0:z/raft` | 1348 | 3 | 1,536 | Raft sprite |
| 5 | `dh0:z/turtle` | 1351 | 20 | 10,240 | Turtle carrier sprite |
| 6 | `dh0:z/bird` | 1120 | 40 | 20,480 | Bird carrier sprite |
| 7 | `dh0:z/dragon` | 1160 | 12 | 6,144 | Dragon sprite |
| 8 | `dh0:z/ogre` | 960 | 40 | 20,480 | Ogre/orc enemy sprites |
| 9 | `dh0:z/ghost` | 1080 | 40 | 20,480 | Ghost/wraith enemy sprites |
| 10 | `dh0:z/dKnight` | 1000 | 40 | 20,480 | Dark knight enemy sprites |
| 11 | `dh0:z/spirit` | 1040 | 40 | 20,480 | Spirit/necromancer sprites |
| 12 | `dh0:z/royal` | 931 | 5 | 2,560 | Royal setfig NPCs |
| 13 | `dh0:z/wizard` | 936 | 5 | 2,560 | Wizard/priest setfig NPCs |
| 14 | `dh0:z/bartender` | 941 | 5 | 2,560 | Bartender setfig NPCs |
| 15 | `dh0:z/witch` | 946 | 5 | 2,560 | Witch setfig NPCs |
| 16 | `dh0:z/beggar` | 951 | 5 | 2,560 | Beggar/ranger setfig NPCs |

Source: `mtrack.c:62-79`

### 2.4 Bitmap Allocation Table Update

`mtrack.c` maintains AmigaOS filesystem compatibility by updating the bitmap allocation block. It:

1. Reads the root block at block 880: `read_block(880, &rootblock)` — `mtrack.c:91`
2. Extracts the bitmap block pointer from `rootblock.offset[BK_SIZE-49]` (offset 79) — `mtrack.c:92`
3. Reads the bitmap block — `mtrack.c:94`
4. After writing each file's blocks, marks those blocks as allocated in the bitmap — `mtrack.c:175-181`
5. Recalculates the bitmap checksum and writes it back — `mtrack.c:104-105`

The bitmap marking logic (mtrack.c:175-181):
```
bit = i - 2;              /* don't count first 2 blocks (boot blocks) */
off = ((bit/32) + 1);     /* starting from word 1 (word 0 is checksum) */
bit = bit & 31;
mask = 1<<bit;
bmapblock.offset[off] &= (~mask);  /* clear bit = mark as used */
```

This means disk 1 remains a valid AmigaDOS volume — it has a root block and a valid bitmap. This is necessary because the disk also contains AmigaOS filesystem files (the game executable `fmain`, fonts, intro images, etc.) accessed through normal AmigaDOS calls.

### 2.5 main() Flow (mtrack.c:82-113)

1. Allocate 50,000 bytes of CHIP RAM for buffer — `mtrack.c:83`
2. Create message port and IO request — `mtrack.c:85-89`
3. Open trackdisk device unit 0 — `mtrack.c:91`
4. Read root block (880) and bitmap block — `mtrack.c:93-96`
5. Loop through `diskmap[25]`: call `transfer_file()` for each — `mtrack.c:98-103`
6. Loop through `char_map[17]`: call `transfer_char_file()` for each — `mtrack.c:105-110`
7. Write updated bitmap block — `mtrack.c:112-113`
8. Turn off disk motor — `mtrack.c:115-116`
9. Cleanup — `mtrack.c:118-120`

## 3. rtrack.c — Game Disk 2 Formatter

### 3.1 Structure

`rtrack.c` is a simpler tool that writes only tile/terrain data to drive 1 (df1:). It has no `char_map`, no bitmap update, and no filesystem compatibility.

### 3.2 diskmap[21] — Data-Only Map (rtrack.c:18-43)

The first 20 entries are **identical** to `mtrack.c`'s `diskmap[0..19]` (same filenames, same block offsets). The 21st entry differs:

| # | Source File | Block Start | Block Count | Note |
|---|---|---|---|---|
| 0–19 | (same as mtrack.c) | (same) | (same) | Identical tile/sector/map data |
| 20 | `terra` | **882** | 11 | Terrain data — placed at block 882 (vs 149 on disk 1) |

Source: `rtrack.c:20-43`

**Missing from disk 2** (present on disk 1 only):
- `mask`, `mask2`, `mask3` (background masks, blocks 896–919)
- `dh0:z/samples` (audio samples, blocks 920–930)
- All 17 `char_map` entries (character sprites, blocks 931–1501)

### 3.3 Bug: CMD_READ Instead of CMD_WRITE

`rtrack.c:103` sets `io_Command = CMD_READ` but clearly intends to WRITE (the printf says "Block Written" and the function is called `save_track_range`). This is a **bug** — the tool reads FROM the disk INTO the buffer instead of writing to it. The identical function in `mtrack.c:170` correctly uses `CMD_WRITE`.

Source: `rtrack.c:100` vs `mtrack.c:170`

### 3.4 Commented-Out Track-Aligned Version

`rtrack.c:113-128` contains a `#ifdef blarg` block with an alternative `save_track_range()` that splits writes at track boundaries (every 10 blocks). This was apparently an earlier approach abandoned in favor of single bulk writes. The comment `/* write (first_block,buffer,read_length) */` at `rtrack.c:124` shows it was never fully implemented.

### 3.5 No Bitmap Update

Unlike `mtrack.c`, `rtrack.c` does NOT read or update the AmigaOS bitmap allocation table. This means disk 2 would not be a valid AmigaOS volume — it's purely raw block data.

### 3.6 main() Flow (rtrack.c:47-69)

1. Allocate 50,000 bytes — `rtrack.c:48`
2. Create message port and IO request — `rtrack.c:50-53`
3. Open trackdisk device **unit 1** — `rtrack.c:55`
4. Loop through `diskmap[21]`: call `transfer_file()` for each — `rtrack.c:58-63`
5. Turn off disk motor — `rtrack.c:65-66`
6. Cleanup — `rtrack.c:68-69`

## 4. transfer_file() and transfer_char_file()

### 4.1 transfer_file() (mtrack.c:122-133, rtrack.c:78-90)

Reads a file from AmigaDOS filesystem into the buffer, then writes raw blocks to disk:

1. `Open(name, 1005)` — opens file for reading (1005 = `MODE_OLDFILE`) — `mtrack.c:125`
2. `Read(file, buffer, length * 512)` — reads `length` blocks worth of data — `mtrack.c:128`
3. `save_track_range(start, length, buffer)` — writes data to raw disk blocks — `mtrack.c:130`
4. `Close(file)` — `mtrack.c:131`

### 4.2 transfer_char_file() (mtrack.c:135-148)

Same as `transfer_file()` but with one difference:
- `Seek(file, 6, 0)` — skips 6 bytes at the start of the file before reading — `mtrack.c:141`

This 6-byte skip suggests the character sprite source files have a header (likely width/height/frame-count metadata) that is NOT stored on the raw disk. The game runtime stores this metadata in the `cfiles[]` struct instead.

Only `mtrack.c` has `transfer_char_file()`; `rtrack.c` does not write any character sprites.

### 4.3 save_track_range() — mtrack.c version (mtrack.c:150-183)

Writes raw blocks via the Amiga trackdisk device:

```
diskreq->iotd_Req.io_Length = block_count * 512;
diskreq->iotd_Req.io_Data = (APTR)buffer;
diskreq->iotd_Req.io_Command = CMD_WRITE;
diskreq->iotd_Req.io_Offset = first_block * 512;
DoIO(diskreq);
```
— `mtrack.c:168-173`

After successful write, marks blocks as used in the AmigaOS bitmap allocation table — `mtrack.c:175-181`.

### 4.4 save_track_range() — rtrack.c version (rtrack.c:93-107)

Simplified version without bitmap update. **Contains the CMD_READ bug** (see section 3.3).

## 5. Complete Disk 1 Block Map

An Amiga DD floppy has 1760 blocks (80 tracks × 2 sides × 11 sectors/track, 512 bytes each = 880 KB).

### 5.1 Block Allocation Summary

| Block Range | Size (bytes) | Content | Source Entry |
|---|---|---|---|
| 0–1 | 1,024 | Boot blocks (AmigaDOS) | (filesystem) |
| 2–31 | 15,360 | AmigaDOS filesystem area | (filesystem) |
| 32–95 | 32,768 | Sector data `f6a` | diskmap[0] |
| 96–159 | 32,768 | Sector data `f9a` | diskmap[1] |
| 149–159 | 5,632 | **Terrain data `terra`** (overlaps f9a!) | diskmap[20] |
| 160–199 | 20,480 | Region map `map1` | diskmap[2] |
| 200–239 | 20,480 | Tile: `mountain1` | diskmap[6] |
| 240–279 | 20,480 | Tile: `field` | diskmap[9] |
| 280–319 | 20,480 | Tile: `castle` | diskmap[8] |
| 320–359 | 20,480 | Tile: `wild` | diskmap[3] |
| 360–399 | 20,480 | Tile: `build` | diskmap[4] |
| 400–439 | 20,480 | Tile: `rock` | diskmap[5] |
| 440–479 | 20,480 | Tile: `tower` | diskmap[7] |
| 480–519 | 20,480 | Tile: `palace` | diskmap[11] |
| 520–559 | 20,480 | Tile: `swamp` | diskmap[10] |
| 560–599 | 20,480 | Tile: `mountain2` | diskmap[12] |
| 600–639 | 20,480 | Tile: `mountain3` | diskmap[14] |
| 640–679 | 20,480 | Tile: `doom` | diskmap[13] |
| 680–719 | 20,480 | Tile: `under` | diskmap[15] |
| 720–759 | 20,480 | Tile: `furnish` | diskmap[17] |
| 760–799 | 20,480 | Tile: `cave` | diskmap[16] |
| 800–839 | 20,480 | Tile: `inside` | diskmap[18] |
| 840–879 | 20,480 | Tile: `astral` | diskmap[19] |
| 880 | 512 | Root block (AmigaDOS) | (filesystem) |
| 881–895 | 7,680 | AmigaDOS filesystem area | (filesystem) |
| 896–903 | 4,096 | Background mask `mask` | diskmap[21] |
| 904–911 | 4,096 | Background mask `mask2` | diskmap[22] |
| 912–919 | 4,096 | Background mask `mask3` | diskmap[23] |
| 920–930 | 5,632 | Audio `samples` | diskmap[24] |
| 931–935 | 2,560 | Setfig: `royal` | char_map[12] |
| 936–940 | 2,560 | Setfig: `wizard` | char_map[13] |
| 941–945 | 2,560 | Setfig: `bartender` | char_map[14] |
| 946–950 | 2,560 | Setfig: `witch` | char_map[15] |
| 951–955 | 2,560 | Setfig: `beggar` | char_map[16] |
| 956–959 | 2,048 | (unused gap) | — |
| 960–999 | 20,480 | Enemy: `ogre` | char_map[8] |
| 1000–1039 | 20,480 | Enemy: `dKnight` | char_map[10] |
| 1040–1079 | 20,480 | Enemy: `spirit` | char_map[11] |
| 1080–1119 | 20,480 | Enemy: `ghost` | char_map[9] |
| 1120–1159 | 20,480 | Carrier: `bird` | char_map[6] |
| 1160–1171 | 6,144 | Dragon sprite | char_map[7] |
| 1172–1311 | 71,680 | (unused gap) | — |
| 1312–1347 | 18,432 | Object sprites | char_map[3] |
| 1348–1350 | 1,536 | Raft sprite | char_map[4] |
| 1351–1370 | 10,240 | Turtle carrier | char_map[5] |
| 1371–1375 | 2,560 | (unused gap) | — |
| 1376–1417 | 21,504 | Brother: `julian` | char_map[0] |
| 1418–1459 | 21,504 | Brother: `phillip` | char_map[1] |
| 1460–1501 | 21,504 | Brother: `kevin` | char_map[2] |
| 1502–1759 | 131,584 | (unused / filesystem) | — |

### 5.2 Terra Overlap Note

The `terra` entry (blocks 149–159, 11 blocks) **overlaps** with `f9a` (blocks 96–159, 64 blocks). The comment in `mtrack.c:51` says `/* T1 - T11 - inside sector2 */`, indicating the terrain data is intentionally embedded within the tail end of the `f9a` sector data region. This means blocks 149–159 serve dual purpose: they're the last 11 blocks of `f9a` AND contain the terrain attribute tables T1–T11.

This is confirmed by the runtime: `TERRA_BLOCK` is defined as 149 (`fmain.c:608`), and `load_track_range(TERRA_BLOCK + nd->terra1, 1, terra_mem, 1)` loads individual terrain blocks from this range — `fmain.c:3567`.

The `file_index[].terra1` and `terra2` values range 0–10, so terrain blocks span blocks 149–159 (149+0 through 149+10), exactly matching the `terra` diskmap entry.

## 6. Disk 2 Block Map (rtrack.c)

Disk 2 has the same layout as disk 1 for blocks 32–879 (tile/sector/map data). Differences:

| Block Range | Disk 1 | Disk 2 |
|---|---|---|
| 0–31 | Boot blocks + filesystem | Unused/raw |
| 149–159 | Terra data (inside f9a) | Part of f9a only |
| 880–881 | Root block + filesystem | Unused/raw |
| 882–892 | Filesystem area | **Terra data** (rtrack.c:43) |
| 893–1759 | Masks + samples + sprites + filesystem | Unused/raw |

The terra data placement differs: disk 1 puts it at block 149 (inside f9a), disk 2 puts it at block 882 (just after the root block position). Since the game runtime uses `TERRA_BLOCK = 149`, it reads terra from disk 1's location only.

## 7. Relationship to file_index[10] and Runtime Loading

### 7.1 struct need and file_index (ftale.h:104-106, fmain.c:615-626)

```c
struct need {
    USHORT image[4], terra1, terra2, sector, region, setchar;
};
```

`file_index[10]` maps 10 game regions to disk block numbers. Each field holds a block number that corresponds directly to a `diskmap` entry:

| Region | image[0] | image[1] | image[2] | image[3] | terra1 | terra2 | sector | region | setchar |
|---|---|---|---|---|---|---|---|---|---|
| F1 snowy | 320 (wild) | 480 (palace) | 520 (swamp) | 560 (mt2) | 0 | 1 | 32 (f6a) | 160 (map1) | 22 |
| F2 witch wood | 320 (wild) | 360 (build) | 400 (rock) | 440 (tower) | 2 | 3 | 32 (f6a) | 160 (map1) | 21 |
| F3 swampy | 320 (wild) | 360 (build) | 520 (swamp) | 560 (mt2) | 2 | 1 | 32 (f6a) | 168 (map1+8) | 22 |
| F4 plains | 320 (wild) | 360 (build) | 400 (rock) | 440 (tower) | 2 | 3 | 32 (f6a) | 168 (map1+8) | 21 |
| F5 desert | 320 (wild) | 480 (palace) | 520 (swamp) | 600 (mt3) | 0 | 4 | 32 (f6a) | 176 (map1+16) | 0 |
| F6 bay/city | 320 (wild) | 280 (castle) | 240 (field) | 200 (mt1) | 5 | 6 | 32 (f6a) | 176 (map1+16) | 23 |
| F7 volcanic | 320 (wild) | 640 (doom) | 520 (swamp) | 600 (mt3) | 7 | 4 | 32 (f6a) | 184 (map1+24) | 0 |
| F8 forest | 320 (wild) | 280 (castle) | 240 (field) | 200 (mt1) | 5 | 6 | 32 (f6a) | 184 (map1+24) | 24 |
| F9 inside | 680 (under) | 720 (furnish) | 800 (inside) | 840 (astral) | 8 | 9 | 96 (f9a) | 192 (map1+32) | 0 |
| F10 dungeons | 680 (under) | 760 (cave) | 800 (inside) | 840 (astral) | 10 | 9 | 96 (f9a) | 192 (map1+32) | 0 |

Source: `fmain.c:616-626`

The `image[]` values are direct block numbers matching `diskmap` entries. Each image is loaded as 5 × 8-block reads (40 blocks total = 5 bitplanes × 8 blocks each) — `fmain.c:3579-3587`.

The `region` values (160, 168, 176, 184, 192) come from `map1` (block 160, 40 blocks). Each region map is 8 blocks, so map1 contains 5 region maps at offsets 0, 8, 16, 24, 32 from block 160.

The `terra1` and `terra2` values are 0–10 indices added to `TERRA_BLOCK` (149) to get the absolute block number — `fmain.c:3567-3572`.

The `sector` field is either 32 (`f6a`, 64 blocks for outdoor regions) or 96 (`f9a`, 64 blocks for indoor regions).

The `setchar` field is defined and initialized but **never read** by any runtime code. Values (22,21,22,21,0,23,0,24,0,0) — see `reference/_discovery/disk-io.md` for analysis.

### 7.2 cfiles[18] — Character Sprite Block Map (fmain2.c:643-666)

Maps character sprite types to disk block numbers. The `file_id` field is a direct block number matching `char_map` entries:

| # | Content | file_id (block) | numblocks | seq_num | char_map match |
|---|---|---|---|---|---|
| 0 | Julian | 1376 | 42 | PHIL | char_map[0] |
| 1 | Phillip | 1418 | 42 | PHIL | char_map[1] |
| 2 | Kevin | 1460 | 42 | PHIL | char_map[2] |
| 3 | Objects | 1312 | 36 | OBJECTS | char_map[3] |
| 4 | Raft | 1348 | 3 | RAFT | char_map[4] |
| 5 | Turtle | 1351 | 20 | CARRIER | char_map[5] |
| 6 | Ogre | 960 | 40 | ENEMY | char_map[8] |
| 7 | Ghost | 1080 | 40 | ENEMY | char_map[9] |
| 8 | Dark Knight | 1000 | 40 | ENEMY | char_map[10] |
| 9 | Necromancer | 1040 | 40 | ENEMY | char_map[11] |
| 10 | Dragon | 1160 | 12 | DRAGON | char_map[7] |
| 11 | Bird | 1120 | 40 | CARRIER | char_map[6] |
| 12 | Snake/Salamander | 1376 | 40 | ENEMY | (shares julian block!) |
| 13 | Wizard/Priest | 936 | 5 | SETFIG | char_map[13] |
| 14 | Royal | 931 | 5 | SETFIG | char_map[12] |
| 15 | Bartender | 941 | 5 | SETFIG | char_map[14] |
| 16 | Witch | 946 | 5 | SETFIG | char_map[15] |
| 17 | Ranger/Beggar | 951 | 5 | SETFIG | char_map[16] |

Source: `fmain2.c:643-666`

**Notable**: `cfiles[12]` (snake/salamander) has `file_id = 1376`, same as `cfiles[0]` (Julian). This means the snake sprite data reuses/overlaps Julian's disk blocks.

### 7.3 Other Fixed Block Loads

| Block | Count | Content | Code Location |
|---|---|---|---|
| 920 | 11 | Audio samples | `fmain.c:1028` — `load_track_range(920,11,sample_mem,8)` |
| 896 | 24 | Shadow/mask data (all 3 masks) | `fmain.c:1222` — `load_track_range(896,24,shadow_mem,0)` |
| 880 | 1 | Root block (copy protection check) | `fmain2.c:1430` — `load_track_range(880,1,buffer,0)` |

## 8. load_track_range() — Runtime Reader (hdrive.c:121-140)

The game reads back data using the same block numbers that `mtrack.c` wrote:

```c
load_track_range(f_block, b_count, buffer, dr)
short f_block, b_count, dr; APTR buffer;
{
    if (hdrive == FALSE)    /* Floppy path */
    {
        lastreq = &(diskreqs[dr]);
        if (lastreq->iotd_Req.io_Command == CMD_READ) WaitIO(lastreq);
        *lastreq = *diskreq1;
        lastreq->iotd_Req.io_Length = b_count * 512;
        lastreq->iotd_Req.io_Data = buffer;
        lastreq->iotd_Req.io_Command = CMD_READ;
        lastreq->iotd_Req.io_Offset = f_block * 512;
        SendIO(lastreq);
    }
    else                    /* Hard drive path */
    {
        Seek(file, f_block * 512, OFFSET_BEGINNING);
        Read(file, buffer, b_count * 512);
    }
}
```
— `hdrive.c:121-140`

**Key details**:
- **Floppy**: Uses `SendIO()` (asynchronous). The `dr` parameter selects which of the 9 `diskreqs[]` slots to use, enabling parallel DMA transfers. Before issuing a new read, it waits for any in-flight read on the same slot.
- **Hard drive**: Uses synchronous `Seek()`/`Read()` on the `image` file. The `image` file is a 1:1 copy of the entire floppy (1760 blocks = 880 KB), created by `Install-FTA` using `copyimage trackdisk.device 0 0 1760 "{path}/image"` — `Install-FTA:26`.
- **Block addressing**: Identical between write (`mtrack.c`) and read (`hdrive.c`). Both use `block * 512` as the byte offset. The `image` file maps block N to file offset N*512.

## 9. copyimage.c — Hard Drive Installer Tool

`copyimage.c` is a generic raw disk-to-file copy tool. It takes 5 command-line arguments:
1. Device name (e.g., "trackdisk.device")
2. Unit number (e.g., 0)
3. First block
4. Block count
5. Output filename

Source: `copyimage.c:30-36`

The `Install-FTA` script calls it as: `copyimage trackdisk.device 0 0 1760 "{path}/image"` — `Install-FTA:26`

This copies ALL 1760 blocks of disk 1 (drive 0) into the `image` file, creating a complete raw disk image for hard drive installation. The game detects this via `Lock("image", ACCESS_READ)` in `AllocDiskIO()` — `hdrive.c:34`.

## Cross-Cutting Findings

- **cfiles[12] shares Julian's blocks**: `cfiles[12]` (snake/salamander) has `file_id = 1376`, same as Julian (`cfiles[0]`). This means either: (a) the snake sprite data is literally stored at Julian's offset and they share the same raw pixel data, reinterpreted differently at runtime; or (b) this is a bug/oversight. The `numblocks` differs: 42 for Julian vs 40 for snake, and the `width`/`height`/`count` differ (1×32×67 vs 1×32×64), so they'd read different amounts from the same starting block.
- **rtrack.c has a CMD_READ bug**: `rtrack.c:103` uses `CMD_READ` instead of `CMD_WRITE`. This tool would read FROM disk instead of writing TO it. This may mean rtrack.c was abandoned/unused in the final build, or the bug was caught outside version control.
- **Disk 2 purpose unclear**: Since the game runtime only opens drive 0, disk 2's purpose is ambiguous. It may have been: (a) an earlier two-disk design that was consolidated, (b) a data-only backup disk, or (c) used only during development for testing reads from a second drive. Cannot be determined from source code alone.
- **Terra overlap is intentional**: Terrain data (11 blocks) is embedded inside the `f9a` sector data region (64 blocks). The comment `/* T1 - T11 - inside sector2 */` at `mtrack.c:51` confirms this is deliberate. The game runtime treats them as separate resources via different `TERRA_BLOCK` addressing.

## Unresolved

- **rtrack.c purpose**: Cannot determine from source code whether disk 2 was ever used in a shipping version or was purely a development artifact. The CMD_READ bug suggests it may not have been tested/used in its current form.
- **Character sprite 6-byte header**: `transfer_char_file()` skips 6 bytes (`mtrack.c:141`). The format of this header is not defined in any source file. It's likely width (2 bytes) + height (2 bytes) + count (2 bytes), matching the `cfiles[]` struct fields, but this cannot be confirmed from source code alone.
- **disk 1 filesystem region usage**: Blocks 2–31 and 881–895 and 1502–1759 are reserved for/used by the AmigaDOS filesystem but it's unclear exactly what files occupy these blocks beyond the game executable, fonts, intro images, and songs which are accessed through AmigaDOS paths.
- **setchar field**: The `setchar` values in `file_index` (22,21,22,21,0,23,0,24,0,0) appear to correlate with `cfiles[]` indices but the field is never read at runtime. Purpose cannot be determined.

## Refinement Log
- 2026-04-06: Initial comprehensive discovery pass covering both tools, complete block maps, cross-references with runtime loading
