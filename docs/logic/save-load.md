# Save / Load — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, fmain2.c
> Cross-refs: [RESEARCH §19](../RESEARCH.md#19-save-load-system), [_discovery/save-load.md](../_discovery/save-load.md), [_discovery/disk-io.md](../_discovery/disk-io.md)

## Overview

Save and load in *Faery Tale Adventure* are a single function — `savegame` — whose
direction is selected by the byte-wide global `svflag` (`fmain2.c:1390`). The
player opens the menu path `GAME → Quit → Save Exit → Save` (`fmain.c:3467`:
`svflag = TRUE; gomenu(FILE)`) or `GAME → Load` (`fmain.c:3446`:
`svflag = FALSE; gomenu(FILE)`), then picks one of eight slots labelled A–H
on the `FILE` menu (enabled-array literal at `fmain.c:540`). The `FILE` case in
`do_option` passes `hit` (0..7) to `savegame(hit)` (`fmain.c:3469-3471`) which
builds the filename, finds a writable destination, opens the file, and streams
the save record through `saveload`. The same function handles both directions,
closing with a post-load fixup when `svflag == 0`.

**Porter-critical properties:**

- **No checksum, signature, magic byte, or version stamp.** The file is a raw
  little-/big-endian memory dump (big-endian; native 68000). A reader must
  know the structure a priori.
- **No per-slot metadata.** Slot pickers show the letter only; all slots are
  always selectable.
- **One file per slot.** There is no single multi-slot container and no
  per-record offset — each slot is an independent AmigaDOS file whose name is
  derived from the slot letter (`A.faery`..`H.faery`).
- **Auto-save / starting-save:** none. There is no distinct code path that writes
  a save outside of `savegame`. A fresh game starts from static initializers in
  the executable; no save file is consulted at launch.
- **The record size is variable.** `anim_list` is written only for the live
  actor prefix (length `anix`), and each region's object table is written at
  its current (possibly grown) count `mapobs[i]`. Two saves of the same player
  on the same game day can differ in length.

The record layout is documented in the table immediately below. Every multi-byte
integer is big-endian 68000 native. Padding inside structs follows Aztec C's
68000 ABI (2-byte alignment for `short`/`USHORT`; `char` not padded unless
needed to align the next field or the end of the struct).

## Save record layout

The on-disk layout is the concatenation of eleven raw memory ranges written
by `saveload` in the order listed. Blocks 4 and 13 have variable length; all
other blocks are fixed.

**Blocks (outer order):**

| # | Offset | Size (bytes) | Content | Write call |
|---|-------:|-------------:|---------|------------|
| 1 | 0 | 80 | Misc variables (`&map_x` … `pad3`) | `fmain2.c:1508` |
| 2 | 80 | 2 | `region_num` | `fmain2.c:1511` |
| 3 | 82 | 6 | `anix`, `anix2`, `mdex` | `fmain2.c:1514` |
| 4 | 88 | `anix * 22` | `anim_list[0..anix-1]` (`struct shape`) | `fmain2.c:1515` |
| 5 | … | 35 | `julstuff[35]` | `fmain.c:3623` |
| 6 | … | 35 | `philstuff[35]` | `fmain.c:3624` |
| 7 | … | 35 | `kevstuff[35]` | `fmain.c:3625` |
| 8 | … | 60 | `missile_list[6]` (`struct missile`, 10 B each) | `fmain.c:3630` |
| 9 | … | 24 | `extent_list[0..1]` (`struct extent`, 12 B each) | `fmain2.c:1519` |
| 10 | … | 66 | `ob_listg[11]` (`struct object`, 6 B each) | `fmain2.c:1522` |
| 11 | … | 20 | `mapobs[10]` (`USHORT`) | `fmain2.c:1523` |
| 12 | … | 20 | `dstobs[10]` (`USHORT`) | `fmain2.c:1524` |
| 13 | … | Σ `mapobs[i] * 6` for i∈0..9 | `ob_table[i][0..mapobs[i]-1]` | `fmain2.c:1525-1526` |

**Block 1 — 80-byte misc-variable block (`&map_x`, size literal at `fmain2.c:1508`):**

The block is a single linear memory window starting at `&map_x` and covering
exactly 80 consecutive bytes. The variables occupy that range in declaration
order (`fmain.c:557-581`). All 40 fields are 2 bytes (`short`, `USHORT`).

| Offset | Size | Field | Type | Meaning | Source |
|-------:|-----:|-------|------|---------|--------|
| 0 | 2 | `map_x` | `USHORT` | world X of viewport top-left, pixels | `fmain.c:557` |
| 2 | 2 | `map_y` | `USHORT` | world Y of viewport top-left, pixels | `fmain.c:557` |
| 4 | 2 | `hero_x` | `USHORT` | hero world X, pixels | `fmain.c:558` |
| 6 | 2 | `hero_y` | `USHORT` | hero world Y, pixels | `fmain.c:558` |
| 8 | 2 | `safe_x` | `USHORT` | last safe-zone X | `fmain.c:559` |
| 10 | 2 | `safe_y` | `USHORT` | last safe-zone Y | `fmain.c:559` |
| 12 | 2 | `safe_r` | `USHORT` | safe-zone radius | `fmain.c:559` |
| 14 | 2 | `img_x` | `USHORT` | absolute sector X | `fmain.c:560` |
| 16 | 2 | `img_y` | `USHORT` | absolute sector Y | `fmain.c:560` |
| 18 | 2 | `cheat1` | `short` | cheat-mode flag (persists across save) | `fmain.c:562` |
| 20 | 2 | `riding` | `short` | current mount type (0/5/11) | `fmain.c:563` |
| 22 | 2 | `flying` | `short` | bird/flight flag | `fmain.c:563` |
| 24 | 2 | `wcarry` | `short` | witch-carry flag | `fmain.c:563` |
| 26 | 2 | `turtleprox` | `short` | turtle-proximity tick | `fmain.c:564` |
| 28 | 2 | `raftprox` | `short` | raft-proximity tick | `fmain.c:564` |
| 30 | 2 | `brave` | `short` | bravery stat | `fmain.c:565` |
| 32 | 2 | `luck` | `short` | luck stat | `fmain.c:565` |
| 34 | 2 | `kind` | `short` | kindness stat | `fmain.c:565` |
| 36 | 2 | `wealth` | `short` | gold total | `fmain.c:565` |
| 38 | 2 | `hunger` | `short` | hunger counter | `fmain.c:565` |
| 40 | 2 | `fatigue` | `short` | fatigue counter | `fmain.c:565` |
| 42 | 2 | `brother` | `short` | active brother id (1=Julian, 2=Phillip, 3=Kevin) | `fmain.c:567` |
| 44 | 2 | `princess` | `short` | princess-rescued flag | `fmain.c:568` |
| 46 | 2 | `hero_sector` | `short` | current sector id | `fmain.c:569` |
| 48 | 2 | `hero_place` | `USHORT` | current place-name id | `fmain.c:570` |
| 50 | 2 | `daynight` | `USHORT` | day/night minute counter | `fmain.c:572` |
| 52 | 2 | `lightlevel` | `USHORT` | ambient light level | `fmain.c:572` |
| 54 | 2 | `actor_file` | `short` | loaded actor shape-file id | `fmain.c:573` |
| 56 | 2 | `set_file` | `short` | loaded setfig shape-file id | `fmain.c:573` |
| 58 | 2 | `active_carrier` | `short` | bird/turtle active id | `fmain.c:574` |
| 60 | 2 | `xtype` | `USHORT` | current extent type | `fmain.c:575` |
| 62 | 2 | `leader` | `short` | enemy-group leader index | `fmain.c:576` |
| 64 | 2 | `secret_timer` | `short` | secret-reveal timer | `fmain.c:577` |
| 66 | 2 | `light_timer` | `short` | light-spell timer | `fmain.c:577` |
| 68 | 2 | `freeze_timer` | `short` | freeze-spell timer | `fmain.c:577` |
| 70 | 2 | `cmode` | `short` | current menu mode (overwritten post-load) | `fmain.c:578` |
| 72 | 2 | `encounter_type` | `USHORT` | stored, then reset to 0 on load | `fmain.c:579` |
| 74 | 2 | `pad1` | `USHORT` | declared padding | `fmain.c:581` |
| 76 | 2 | `pad2` | `USHORT` | declared padding | `fmain.c:581` |
| 78 | 2 | `pad3` | `USHORT` | declared padding | `fmain.c:581` |

`pad4..pad7` (declared on the same line, `fmain.c:581`) fall outside the 80-byte
window and are **not** saved.

**Block 3 — actor indices (6 bytes, size literal at `fmain2.c:1514`):**

Written as a single 6-byte blob starting at `&anix`; the three fields occupy
consecutive memory slots (`fmain.c:75-76`).

| Offset in block | Size | Field | Type | Source |
|-------:|-----:|-------|------|--------|
| 0 | 2 | `anix` | `short` | `fmain.c:75` |
| 2 | 2 | `anix2` | `short` | `fmain.c:75` |
| 4 | 2 | `mdex` | `short` | `fmain.c:76` |

**Block 4 — `struct shape` (22 bytes per entry, `ftale.h:56-69`):**

| Offset in entry | Size | Field | Type |
|-------:|-----:|-------|------|
| 0 | 2 | `abs_x` | `USHORT` |
| 2 | 2 | `abs_y` | `USHORT` |
| 4 | 2 | `rel_x` | `USHORT` |
| 6 | 2 | `rel_y` | `USHORT` |
| 8 | 1 | `type` | `char` |
| 9 | 1 | `race` | `UBYTE` |
| 10 | 1 | `index` | `char` |
| 11 | 1 | `visible` | `char` |
| 12 | 1 | `weapon` | `char` |
| 13 | 1 | `environ` | `char` |
| 14 | 1 | `goal` | `char` |
| 15 | 1 | `tactic` | `char` |
| 16 | 1 | `state` | `char` |
| 17 | 1 | `facing` | `char` |
| 18 | 2 | `vitality` | `short` |
| 20 | 1 | `vel_x` | `char` |
| 21 | 1 | `vel_y` | `char` |

Size is fixed at 22 bytes — two adjacent `char` fields pack together and the
trailing `vel_x`/`vel_y` pair lies on an even-offset boundary, so no tail
padding is inserted.

**Block 8 — `struct missile` (10 bytes per entry, `fmain.c:78-85`):**

| Offset in entry | Size | Field | Type |
|-------:|-----:|-------|------|
| 0 | 2 | `abs_x` | `USHORT` |
| 2 | 2 | `abs_y` | `USHORT` |
| 4 | 1 | `missile_type` | `char` |
| 5 | 1 | `time_of_flight` | `char` |
| 6 | 1 | `speed` | `char` |
| 7 | 1 | `direction` | `char` |
| 8 | 1 | `archer` | `char` |
| 9 | 1 | *(tail pad)* | — |

The five consecutive `char` fields occupy offsets 4..8; the struct is tail-padded
to 10 bytes so the next array element starts on a `short`-aligned boundary
(`6 * sizeof(struct missile) = 60` matches the block-size expectation at
`fmain.c:3630`).

**Block 9 — `struct extent` (12 bytes per entry, `fmain.c:335-338`):**

| Offset in entry | Size | Field | Type |
|-------:|-----:|-------|------|
| 0 | 2 | `x1` | `UWORD` |
| 2 | 2 | `y1` | `UWORD` |
| 4 | 2 | `x2` | `UWORD` |
| 6 | 2 | `y2` | `UWORD` |
| 8 | 1 | `etype` | `UBYTE` |
| 9 | 1 | `v1` | `UBYTE` |
| 10 | 1 | `v2` | `UBYTE` |
| 11 | 1 | `v3` | `UBYTE` |

Only the first **two** entries of `extent_list` (bird and turtle, `fmain.c:339-340`)
are saved. All other `EXT_COUNT = 22` entries (`fmain.c:374`) are re-initialized
from executable constants on load/launch and are treated as static world
geometry.

**Block 10 — `struct object` (6 bytes per entry, `ftale.h:90-93`):**

| Offset in entry | Size | Field | Type |
|-------:|-----:|-------|------|
| 0 | 2 | `xc` | `USHORT` |
| 2 | 2 | `yc` | `USHORT` |
| 4 | 1 | `ob_id` | `char` |
| 5 | 1 | `ob_stat` | `char` |

Block 10 is 66 bytes (`glbobs = 11`, `fmain2.c:1180`; `11 * 6 = 66`). Block 13
uses the same struct at variable counts `mapobs[0..9]`.

**Block 11 / 12 — `mapobs` and `dstobs` arrays (20 bytes each):**

Each of the ten regions contributes a 2-byte `USHORT` entry in declaration
order (region 0 first, region 9 last, 20-byte total from the size literal at
`fmain2.c:1523` / `fmain2.c:1524`).

## Symbols

All globals used below (`svflag`, `svfile`, `sverr`, `savename`, `flock`,
`stuff`, `anim_list`, `missile_list`, `extent_list`, `ob_listg`, `ob_table`,
`mapobs`, `dstobs`, `julstuff`, `philstuff`, `kevstuff`, `blist`, `brother`,
`region_num`, `anix`, `anix2`, `mdex`, `glbobs`, `wt`, `encounter_number`,
`encounter_type`, `actors_loading`, `viewstatus`, `quitflag`, `handler_data`,
`rp`, `tfont`, `afont`) resolve in [SYMBOLS.md](SYMBOLS.md) or are proposed
additions listed in the wave report. No new locals are introduced beyond each
function's declared parameters.

## savegame

Source: `fmain2.c:1474-1549`
Called by: `option_handler` (via `do_option` `FILE` case at `fmain.c:3469-3471`)
Calls: `locktest`, `waitnewdisk`, `Open`, `Close`, `IoErr`, `serialize_save_record`, `deserialize_save_record`, `shape_read`, `set_options`, `prq`, `print`, `Lock`, `UnLock`, `SetFont`, `svflag`, `svfile`, `sverr`, `savename`, `flock`, `rp`, `tfont`, `afont`, `ACCESS_READ`, `ACCESS_WRITE`, `wt`, `encounter_number`, `viewstatus`, `encounter_type`, `actors_loading`

```pseudo
def savegame(hit: int) -> None:
    """Top-level save/load dispatcher: find a writable disk, open slot file, stream the record, restore on load."""
    sverr = 0                                                # fmain2.c:1477 — error accumulator cleared
    SetFont(rp, tfont)                                       # fmain2.c:1479 — switch to title font for prompts
    hdrive = False                                           # fmain2.c:1476 — begin with floppy assumption
    # Disk-destination search (retry label in source: `stest:` at fmain2.c:1481)
    while True:
        name = savename                                      # fmain2.c:1484 — start with "df1:A.faery"
        if (not hdrive) and locktest("image", ACCESS_READ):  # fmain2.c:1486 — hard-drive detect
            name = savename + 4                              # fmain2.c:1487 — strip "df1:" prefix (4 chars)
            hdrive = True                                    # fmain2.c:1488
            break
        if locktest("df1:", ACCESS_WRITE):                   # fmain2.c:1490 — data disk in df1
            savename[2] = '1'                                # fmain2.c:1491 — keep "df1:" prefix
            break
        if locktest("df0:", ACCESS_WRITE) and (not locktest("df0:winpic", ACCESS_READ)):  # fmain2.c:1493-1494 — writable non-game disk in df0
            savename[2] = '0'                                # fmain2.c:1495 — rewrite prefix to "df0:"
            break
        print("Insert a writable disk in ANY drive.")        # fmain2.c:1497
        if waitnewdisk() == 0:                               # fmain2.c:1498 — polling timeout
            print("Aborted.")                                # fmain2.c:1498
            SetFont(rp, afont)                               # fmain2.c:1549 — restore game font on nosave exit
            return
        # loop retries the detection (source uses `goto stest`)
    savename[4] = 'A' + hit                                  # fmain2.c:1502 — slot letter A..H from hit 0..7
    if svflag:                                               # fmain2.c:1503 — save direction
        svfile = Open(name, 1006)                            # fmain2.c:1503 — MODE_NEWFILE (truncate/create)
    else:
        svfile = Open(name, 1005)                            # fmain2.c:1504 — MODE_OLDFILE (read existing)
    if svfile:
        if svflag:
            serialize_save_record()                          # fmain2.c:1505-1527 — write-direction block stream
        else:
            deserialize_save_record()                        # fmain2.c:1505-1527 — read-direction block stream
        Close(svfile)                                        # fmain2.c:1528
    else:
        sverr = IoErr()                                      # fmain2.c:1530
    if sverr != 0:                                           # fmain2.c:1531
        if svflag:
            print("ERROR: Couldn't save game.")              # fmain2.c:1532
        else:
            print("ERROR: Couldn't load game.")              # fmain2.c:1533
    # Post-op: ensure the game disk is re-inserted before continuing (floppy only).
    if not hdrive:                                           # fmain2.c:1534
        while True:
            flock = Lock("df0:winpic", ACCESS_READ)          # fmain2.c:1536 — game disk has winpic
            if flock:
                UnLock(flock)                                # fmain2.c:1537
                break
            print("Please insert GAME disk.")                # fmain2.c:1538
            waitnewdisk()                                    # fmain2.c:1539
    # Post-load-only fixup: rebuild transient state from the loaded persistent state.
    if svflag == 0:                                          # fmain2.c:1541 — load branch
        wt = 0                                               # fmain2.c:1542 — clear wait-timer
        encounter_number = 0                                 # fmain2.c:1542 — clear pending encounters
        shape_read()                                         # fmain2.c:1544 — reload hero sprites for current brother
        set_options()                                        # fmain2.c:1544 — rebuild menu-enable state from stuff[]
        viewstatus = 99                                      # fmain2.c:1544 — 99 = "corrupt" sentinel forcing redraw (fmain.c:583)
        prq(4)                                               # fmain2.c:1545 — 4 = status-bar redraw queue id
        prq(7)                                               # fmain2.c:1545 — 7 = wealth display queue id
        print("")                                            # fmain2.c:1546 — clear text scroll (3 blank lines)
        print("")                                            # fmain2.c:1546
        print("")                                            # fmain2.c:1546
        encounter_type = 0                                   # fmain2.c:1548
        actors_loading = 0                                   # fmain2.c:1548
    SetFont(rp, afont)                                       # fmain2.c:1549 — restore Amber game font
    return
```

**Notes.** The source uses a `goto stest` back-edge to retry the disk search
after a successful `waitnewdisk`; the pseudo-code expresses the same flow as a
`while True` with `break` on each success branch. There is no per-record
offset computation — `hit` only selects the slot letter, which names a distinct
file. The `SetFont` pair frames the handler: `tfont` for disk prompts,
`afont` restored before returning to gameplay.

## serialize_save_record

Source: `fmain2.c:1505-1527` (save branch of `savegame`)
Called by: `savegame`
Calls: `saveload_block`, `mod1save`, `map_x`, `region_num`, `anix`, `anim_list`, `extent_list`, `ob_listg`, `mapobs`, `dstobs`, `ob_table`, `glbobs`

```pseudo
def serialize_save_record() -> None:
    """Write the thirteen blocks of the save record to the open svfile, in order."""
    saveload_block(map_x, 80)                                # fmain2.c:1508 — block 1: 80-byte misc-var window at &map_x
    saveload_block(region_num, 2)                            # fmain2.c:1511 — block 2: region id
    saveload_block(anix, 6)                                  # fmain2.c:1514 — block 3: anix, anix2, mdex (6 bytes = 3 shorts)
    saveload_block(anim_list, anix * 22)                     # fmain2.c:1515 — block 4: live actor prefix; 22 = sizeof(struct shape)
    mod1save()                                               # fmain2.c:1517 — blocks 5-8: brother inventories + missiles
    saveload_block(extent_list, 24)                          # fmain2.c:1519 — block 9: 2 * sizeof(struct extent) = 24
    saveload_block(ob_listg, glbobs * 6)                     # fmain2.c:1522 — block 10: 11 * 6 = 66; 6 = sizeof(struct object)
    saveload_block(mapobs, 20)                               # fmain2.c:1523 — block 11: 10 region counts * 2 bytes
    saveload_block(dstobs, 20)                               # fmain2.c:1524 — block 12: 10 distributed flags * 2 bytes
    i = 0
    while i < 10:                                            # fmain2.c:1525 — 10 = NO_REGION (fmain.c:614)
        saveload_block(ob_table[i], mapobs[i] * 6)           # fmain2.c:1526 — block 13: variable per-region table; 6 = sizeof(struct object)
        i = i + 1
    return
```

## deserialize_save_record

Source: `fmain2.c:1505-1527` (load branch of `savegame`, same calls with `svflag == 0`)
Called by: `savegame`
Calls: `saveload_block`, `mod1save`, `map_x`, `region_num`, `anix`, `anim_list`, `extent_list`, `ob_listg`, `mapobs`, `dstobs`, `ob_table`, `glbobs`

```pseudo
def deserialize_save_record() -> None:
    """Read the thirteen blocks of the save record from the open svfile, in order."""
    # Direction is selected by svflag inside saveload_block; ordering / sizes match serialize_save_record.
    saveload_block(map_x, 80)                                # fmain2.c:1508 — block 1
    saveload_block(region_num, 2)                            # fmain2.c:1511 — block 2
    saveload_block(anix, 6)                                  # fmain2.c:1514 — block 3: anix is read FIRST, then used to size block 4
    saveload_block(anim_list, anix * 22)                     # fmain2.c:1515 — block 4: 22 = sizeof(struct shape)
    mod1save()                                               # fmain2.c:1517 — blocks 5-8; mod1save also re-aims `stuff` at the loaded brother
    saveload_block(extent_list, 24)                          # fmain2.c:1519 — block 9
    saveload_block(ob_listg, glbobs * 6)                     # fmain2.c:1522 — block 10; 6 = sizeof(struct object)
    saveload_block(mapobs, 20)                               # fmain2.c:1523 — block 11
    saveload_block(dstobs, 20)                               # fmain2.c:1524 — block 12
    i = 0
    while i < 10:                                            # fmain2.c:1525 — 10 = NO_REGION
        saveload_block(ob_table[i], mapobs[i] * 6)           # fmain2.c:1526 — block 13: uses the mapobs value JUST loaded; 6 = sizeof(struct object)
        i = i + 1
    return
```

**Ordering dependency.** The loader requires `anix` to be read (block 3)
before the size of block 4 is computable, and `mapobs` must be read (block 11)
before the per-region tables (block 13) can be sized. Any porter that buffers
the whole file up front must still honor this parse order.

## saveload_block

Source: `fmain2.c:1553-1558`
Called by: `serialize_save_record`, `deserialize_save_record`, `mod1save`
Calls: `Write`, `Read`, `IoErr`, `svflag`, `svfile`, `sverr`

```pseudo
def saveload_block(buffer: bytes, length: int) -> None:
    """Raw block read/write to svfile; direction chosen by svflag, errors accumulated in sverr."""
    if svflag:
        err = Write(svfile, buffer, length)                  # fmain2.c:1555 — AmigaDOS Write, returns bytes written or -1
    else:
        err = Read(svfile, buffer, length)                   # fmain2.c:1556 — AmigaDOS Read, returns bytes read or -1
    if err < 0:                                              # fmain2.c:1557
        sverr = IoErr()                                      # fmain2.c:1557
    return
```

**Native byte order.** Bytes are copied verbatim — no byte-swapping, no
endianness conversion, no field-by-field marshalling. The on-disk bytes are
whatever the 68000 has in memory, which is big-endian for `short` / `USHORT`.

**Short `err`, long return.** The source declares `short err` and assigns it
the return of `Write` / `Read` which are `long`; only the low 16 bits are
kept. The `err < 0` comparison still detects the `-1` error sentinel because
AmigaDOS never returns a block `> 32767` through this path (block sizes are
bounded — see the Save record layout table — and the largest single `saveload`
call in the save record is ~440 bytes for actors at full `anix`).

## mod1save

Source: `fmain.c:3621-3631`
Called by: `serialize_save_record`, `deserialize_save_record`
Calls: `saveload_block`, `julstuff`, `philstuff`, `kevstuff`, `missile_list`, `stuff`, `blist`, `brother`

```pseudo
def mod1save() -> None:
    """Stream blocks 5-8: all three brothers' inventories, reseat stuff at the active brother, then missiles."""
    saveload_block(julstuff, 35)                             # fmain.c:3623 — block 5: ARROWBASE = 35 (fmain.c:429)
    saveload_block(philstuff, 35)                            # fmain.c:3624 — block 6
    saveload_block(kevstuff, 35)                             # fmain.c:3625 — block 7
    stuff = blist[brother - 1].stuff                         # fmain.c:3627 — reseat stuff at the loaded brother's array
    saveload_block(missile_list, 60)                         # fmain.c:3630 — block 8: 6 * sizeof(struct missile) = 60 (6 slots, 10 bytes each)
    return
```

**The `stuff` reassignment runs in both directions.** On save, it is a harmless
no-op (the pointer is already correct). On load, it is the mechanism that
restores the active-brother inventory binding after `brother` has been read
from block 1. The comment at `fmain.c:3622` labels the whole block "save stuff"
regardless of direction.

## locktest

Source: `fmain2.c:1401-1405`
Called by: `savegame`
Calls: `Lock`, `UnLock`, `flock`

```pseudo
def locktest(name: str, access: i32) -> int:
    """Probe whether an AmigaDOS path is reachable under the given access mode; non-destructive."""
    flock = Lock(name, access)                               # fmain2.c:1403
    if flock:
        UnLock(flock)                                        # fmain2.c:1404 — release immediately; we only wanted the presence test
    return flock                                             # fmain2.c:1404 — nonzero BPTR if the path exists
```

## waitnewdisk

Source: `fmain2.c:1442-1450`
Called by: `savegame`
Calls: `Delay`, `handler_data`

```pseudo
def waitnewdisk() -> bool:
    """Poll handler_data.newdisk for up to 300 * 5-tick ticks; clear the flag on hit and return TRUE."""
    i = 0
    while i < 300:                                           # fmain2.c:1444 — 300 = poll iterations cap (~30s at 5-tick Delay each)
        if handler_data.newdisk:                             # fmain2.c:1445 — set by the input handler on disk-insert events
            handler_data.newdisk = 0                         # fmain2.c:1446 — consume
            return True                                      # fmain2.c:1446
        Delay(5)                                             # fmain2.c:1448 — 5 = AmigaDOS ticks (50 Hz → 100 ms)
        i = i + 1
    return False                                             # fmain2.c:1449 — timeout
```

## Notes

### Not saved

The following mutable state is **not** written to the save record. On load, these
are either left at their pre-load values (none of them are used by gameplay
before post-load fixup resets them) or recomputed from scratch.

- `pad4..pad7` — declared on the same line as `pad1..pad3` but beyond the 80-byte window (`fmain.c:581`).
- All `viewstatus`-through-`wt` transients (`fmain.c:583-604`); `viewstatus`, `wt`, `encounter_number`, `encounter_type`, and `actors_loading` are explicitly clobbered in the post-load fixup (`fmain2.c:1542-1548`).
- `extent_list[2..21]` — static world extents rebuilt from executable constants on every launch (`fmain.c:339-372`).
- `new_region`, `lregion`, `current_loads` — disk-load cache state (`fmain.c:617-618`).
- Music state, text scroll contents, menu-enabled states (`set_options` rebuilds them at `fmain2.c:1544`), display double-buffer state, input-handler state.
- The `stuff` pointer itself — re-derived in `mod1save` (`fmain.c:3627`).

### Saved but overwritten

- `cmode` (offset 70): the loaded value is immediately replaced by `gomenu(GAME)` on the return path from `savegame` (`fmain.c:3471`).
- `encounter_type` (offset 72): reset to 0 in the post-load fixup (`fmain2.c:1548`).

### No signature, no versioning

There is no magic-byte header, no record length field, no per-block checksum,
no compile-time version stamp. A loader that opens a file of the wrong size,
the wrong layout, or from a future/past build of the game will silently read
the wrong bytes into memory and likely crash or soft-lock later. The format
is fully trust-based and brittle to struct-layout changes.

### Cheats persist

`cheat1` is saved at offset 18 of the misc block (`fmain.c:562`, `fmain2.c:1508`).
A player who enables cheats, saves, quits, and later loads will still have
cheats enabled.

### Record size bounds

- Fixed per-save overhead: 80 + 2 + 6 + 105 + 60 + 24 + 66 + 20 + 20 = 383 bytes.
- Variable tail: `anix * 22 + Σ mapobs[i] * 6`.
- A freshly started game with `anix ≤ 8` and default `mapobs` typically produces a
  record of roughly 1200–1500 bytes. The worst case is bounded by
  `MAX_ACTORS = 20` (`fmain.c:70`) and the sum of per-region object caps.

### Verification tool

`tools/decode_savegame.py` is a Python decoder that walks the eleven blocks
in order and prints human-readable field values. Two committed saves
(`game/C.faery`, `game/E.faery`) serve as regression inputs.
