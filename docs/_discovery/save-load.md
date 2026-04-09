# Discovery: Save/Load System

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace the complete save/load system — savegame(), saveload(), mod1save(), file format, disk detection, menu flow, what is and isn't saved, post-load restoration.

## 1. Menu Flow — How Save/Load Is Triggered

### Game → Quit Menu (SAVEX)

The Game menu (cmode=GAME) offers "Quit" at hit=8 and "Load" at hit=9:
- `fmain.c:3445` — `if (hit==8) gomenu(SAVEX);` — opens the SAVEX submenu
- `fmain.c:3446` — `if (hit==9) { svflag = FALSE; gomenu(FILE); }` — sets load mode, opens file picker

### SAVEX Menu — Save or Exit

The SAVEX menu (label8 = "Save Exit ") offers two choices at `fmain.c:3466-3468`:
- `hit==5` → `svflag = TRUE; gomenu(FILE)` — sets save mode, opens file picker
- `hit==6` → `quitflag = TRUE` — exits the game

### FILE Menu — Slot Selection

The FILE menu (label = "  A    B    C    D    E    F    G    H  ") presents 8 save slots (A–H). Enabled array at `fmain.c:540`: all 8 slots have value 10 (immediate+visible).

When user clicks a slot, `do_option` dispatches at `fmain.c:3469-3471`:
```c
case FILE:
    savegame(hit);
    gomenu(GAME);
    break;
```

`hit` ranges 0–7 for slots A–H. After save/load completes, returns to GAME menu.

### Menu enums:
- `fmain.c:494` — `enum cmodes {ITEMS=0, MAGIC, TALK, BUY, GAME, SAVEX, KEYS, GIVE, USE, FILE};`

## 2. savegame() — Main Save/Load Entry Point — `fmain2.c:1474-1549`

### Global State
- `fmain2.c:1390` — `BYTE svflag;` — direction flag: nonzero=save, zero=load
- `fmain2.c:1391` — `long svfile, sverr;` — AmigaDOS file handle and error code
- `fmain2.c:1392` — `char savename[] = "df1:A.faery";` — template filename

### Disk Detection Logic (`fmain2.c:1483-1501`)

Tries each option in priority order:

1. **Hard drive** (`fmain2.c:1486`): `locktest("image",ACCESS_READ)` — if `image` file found in current directory, assume hard drive. Sets `hdrive = TRUE`, `name += 4` (strips "df1:" prefix → uses just `A.faery` in current directory).

2. **df1:** (`fmain2.c:1490`): `locktest("df1:",ACCESS_WRITE)` — data disk in drive 1. Filename stays `df1:A.faery`.

3. **df0:** (`fmain2.c:1493-1494`): `locktest("df0:",ACCESS_WRITE) && !locktest("df0:winpic",ACCESS_READ)` — writable disk in drive 0 that is NOT the game disk (no `winpic` file). Sets `savename[2] = '0'` → `df0:A.faery`.

4. **Fallback** (`fmain2.c:1497-1500`): Prompts "Insert a writable disk in ANY drive." Waits for a new disk via `waitnewdisk()` (polls `handler_data.newdisk` for up to 300 × 5 ticks = ~30 seconds at `fmain2.c:1442-1450`). If timeout, prints "Aborted." and exits without saving.

### Slot Letter Assignment
- `fmain2.c:1502` — `savename[4] = 'A' + hit;` — hit 0–7 → slots A–H
- Produces filenames: `A.faery`, `B.faery`, ... `H.faery`

### File Open
- `fmain2.c:1503` — Save: `Open(name, 1006)` — MODE_NEWFILE (create/truncate)
- `fmain2.c:1504` — Load: `Open(name, 1005)` — MODE_OLDFILE (read existing)

### Data Serialization Order (`fmain2.c:1505-1527`)

If file opens successfully, writes/reads blocks sequentially:

| # | Call | Bytes | Description |
|---|------|-------|-------------|
| 1 | `saveload(&map_x, 80)` | 80 | Misc game variables block |
| 2 | `saveload(&region_num, 2)` | 2 | Current region number |
| 3 | `saveload(&anix, 6)` | 6 | anix, anix2, mdex (actor/missile indices) |
| 4 | `saveload(anim_list, anix * sizeof(struct shape))` | anix × 22 | Active actor list (variable length) |
| 5 | `mod1save()` | 105 + 60 | Brother inventories + missiles (see §3) |
| 6 | `saveload(extent_list, 2 * sizeof(struct extent))` | 24 | First 2 extent entries |
| 7 | `saveload(ob_listg, glbobs * sizeof(struct object))` | 66 | Global objects (11 × 6) |
| 8 | `saveload(mapobs, 20)` | 20 | Per-region object counts |
| 9 | `saveload(dstobs, 20)` | 20 | Per-region distributed flags |
| 10 | Loop: `saveload(ob_table[i], mapobs[i] * sizeof(struct object))` for i=0..9 | variable | All regional object tables |

### Error Handling
- `fmain2.c:1530` — If file fails to open: `sverr = IoErr()`
- `fmain2.c:1531-1533` — If `sverr` nonzero: prints "ERROR: Couldn't save game." or "ERROR: Couldn't load game."

### Post-Save: Game Disk Re-insertion (`fmain2.c:1534-1540`)
If NOT on hard drive, loops waiting for game disk:
```c
if (hdrive == FALSE) while (TRUE)
{   flock = Lock("df0:winpic",ACCESS_READ);
    if (flock) { UnLock(flock); break; }
    print("Please insert GAME disk.");
    waitnewdisk();
}
```

### Post-Load State Restoration (`fmain2.c:1541-1548`)
Only runs on load (`svflag==0`):
```c
wt = encounter_number = 0;
shape_read(); set_options(); viewstatus = 99;
prq(4); prq(7);
print(""); print(""); print("");
encounter_type = actors_loading = 0;
```

Actions:
- Clears encounter tracking (`wt`, `encounter_number`, `encounter_type`, `actors_loading`)
- `shape_read()` (`fmain2.c:673`) — reloads hero sprite graphics based on current `brother`
- `set_options()` (`fmain.c:3527`) — refreshes menu enabled states from inventory (`stuff[]`)
- `viewstatus = 99` — marks status bar as corrupt, forces full redraw
- `prq(4)` — queues status bar update (vitality display)
- `prq(7)` — queues wealth display update
- Three empty `print("")` calls clear the text scroll area

## 3. saveload() — Low-Level Block I/O — `fmain2.c:1553-1558`

```c
saveload(buffer,length) char *buffer; long length;
{   short err;
    if (svflag) err = Write(svfile,buffer,length);
    else err = Read(svfile,buffer,length);
    if (err < 0) sverr = IoErr();
}
```

- Uses AmigaDOS `Write()`/`Read()` — standard file I/O, not raw disk access
- Direction controlled by `svflag` (nonzero=write, zero=read)
- Error stored in `sverr` for later checking
- All data written/read as raw memory dumps — no marshalling, no endianness conversion (native big-endian 68000 byte order)

## 4. mod1save() — Brother Inventories + Missiles — `fmain.c:3621-3631`

```c
mod1save()
{   saveload(julstuff,35);
    saveload(philstuff,35);
    saveload(kevstuff,35);
    stuff = blist[brother-1].stuff;
    saveload((void *)missile_list, 6 * sizeof(struct missile));
}
```

### What is saved:
- `julstuff[35]`, `philstuff[35]`, `kevstuff[35]` — all three brothers' inventories, 35 bytes each (`fmain.c:432`)
- `stuff` pointer reassigned from `blist[brother-1].stuff` — restores the active brother's inventory pointer
- `missile_list[6]` — all 6 missile slots (6 × 10 = 60 bytes)

### Brother inventory layout (`fmain.c:425-432`)
- Items 0–4: weapons (Dirk, Mace, Sword, Bow, Wand)
- Items 5–8: quest items (Lasso, Shell, Sun Stone, Arrows)
- Items 9–15: magic items (MAGICBASE=9)
- Items 16–21: keys (KEYBASE=16)
- Items 22–24: consumables (Talisman, Rose, Fruit)
- Items 25–30: stat items (STATBASE=25)
- Items 31–34: gold denominations (GOLDBASE=31)

### struct missile (`fmain.c:78-85`)
```c
struct missile {
    unsigned short abs_x, abs_y;      // 4 bytes
    char missile_type;                 // 1 byte
    char time_of_flight;               // 1 byte
    char speed;                        // 1 byte
    char direction;                    // 1 byte
    char archer;                       // 1 byte + 1 pad = 10 bytes total
} missile_list[6];
```

## 5. The 80-Byte Misc Variables Block

Starts at `&map_x` (`fmain.c:557`). All are consecutive global variables — saved as raw memory. Each is a 2-byte value (short/USHORT on 68000):

| Offset | Variable | Type | Source |
|--------|----------|------|--------|
| 0 | map_x | unsigned short | fmain.c:557 |
| 2 | map_y | unsigned short | fmain.c:557 |
| 4 | hero_x | unsigned short | fmain.c:558 |
| 6 | hero_y | unsigned short | fmain.c:558 |
| 8 | safe_x | unsigned short | fmain.c:559 |
| 10 | safe_y | unsigned short | fmain.c:559 |
| 12 | safe_r | unsigned short | fmain.c:559 |
| 14 | img_x | unsigned short | fmain.c:560 |
| 16 | img_y | unsigned short | fmain.c:560 |
| 18 | cheat1 | short | fmain.c:562 |
| 20 | riding | short | fmain.c:563 |
| 22 | flying | short | fmain.c:563 |
| 24 | wcarry | short | fmain.c:563 |
| 26 | turtleprox | short | fmain.c:564 |
| 28 | raftprox | short | fmain.c:564 |
| 30 | brave | short | fmain.c:565 |
| 32 | luck | short | fmain.c:565 |
| 34 | kind | short | fmain.c:565 |
| 36 | wealth | short | fmain.c:565 |
| 38 | hunger | short | fmain.c:565 |
| 40 | fatigue | short | fmain.c:565 |
| 42 | brother | short | fmain.c:567 |
| 44 | princess | short | fmain.c:568 |
| 46 | hero_sector | short | fmain.c:569 |
| 48 | hero_place | USHORT | fmain.c:570 |
| 50 | daynight | USHORT | fmain.c:572 |
| 52 | lightlevel | USHORT | fmain.c:572 |
| 54 | actor_file | short | fmain.c:573 |
| 56 | set_file | short | fmain.c:573 |
| 58 | active_carrier | short | fmain.c:574 |
| 60 | xtype | USHORT | fmain.c:575 |
| 62 | leader | short | fmain.c:576 |
| 64 | secret_timer | short | fmain.c:577 |
| 66 | light_timer | short | fmain.c:577 |
| 68 | freeze_timer | short | fmain.c:577 |
| 70 | cmode | short | fmain.c:578 |
| 72 | encounter_type | USHORT | fmain.c:579 |
| 74 | pad1 | USHORT | fmain.c:581 |
| 76 | pad2 | USHORT | fmain.c:581 |
| 78 | pad3 | USHORT | fmain.c:581 |

Note: `fmain.c:581` declares `USHORT pad1,pad2,pad3,pad4,pad5,pad6,pad7;` but only pad1–pad3 (6 bytes) fall within the 80-byte window. pad4–pad7 are NOT saved.

## 6. Complete Save File Format

All multi-byte values are big-endian (Motorola 68000 native).

| Block | Offset | Size | Content | Source |
|-------|--------|------|---------|--------|
| 1 | 0 | 80 | Misc variables (map_x through pad3) | fmain2.c:1508 |
| 2 | 80 | 2 | region_num | fmain2.c:1511 |
| 3 | 82 | 6 | anix, anix2, mdex | fmain2.c:1514 |
| 4 | 88 | anix × 22 | anim_list[0..anix-1] (struct shape) | fmain2.c:1515 |
| 5 | 88 + anix×22 | 35 | julstuff[] inventory | fmain.c:3623 |
| 6 | +35 | 35 | philstuff[] inventory | fmain.c:3624 |
| 7 | +35 | 35 | kevstuff[] inventory | fmain.c:3625 |
| 8 | +35 | 60 | missile_list[6] (10 bytes each) | fmain.c:3630 |
| 9 | +60 | 24 | extent_list[0..1] (12 bytes each) | fmain2.c:1519 |
| 10 | +24 | 66 | ob_listg[11] (6 bytes each) | fmain2.c:1522 |
| 11 | +66 | 20 | mapobs[10] (2 bytes each) | fmain2.c:1523 |
| 12 | +20 | 20 | dstobs[10] (2 bytes each) | fmain2.c:1524 |
| 13 | +20 | Σ mapobs[i]×6 | ob_table[0..9] regional objects | fmain2.c:1525-1526 |

Total file size = 80 + 2 + 6 + (anix × 22) + 105 + 60 + 24 + 66 + 20 + 20 + (Σ mapobs[i] × 6)

With default `anix` and `mapobs` values, a typical save is roughly 1200–1500 bytes.

## 7. What Game State IS Saved

### Player State
- Position: hero_x, hero_y, map_x, map_y, hero_sector, hero_place — `fmain.c:557-570`
- Stats: brave, luck, kind, wealth, hunger, fatigue — `fmain.c:565`
- Active brother: brother (1=Julian, 2=Phillip, 3=Kevin) — `fmain.c:567`
- All three brothers' inventories (35 items each) — `fmain.c:432`
- Safe zone: safe_x, safe_y, safe_r — `fmain.c:559`

### World State
- All global objects (ob_listg, 11 objects) — includes dead brothers, ghost positions, statues, spectre — `fmain2.c:1001-1012`
- All regional objects (ob_table[0..9]) — positions and status — `fmain2.c:1015-1170`
- Per-region object counts (mapobs[10]) — may grow from initial values via item drops — `fmain2.c:1178`
- Per-region distributed flags (dstobs[10]) — `fmain2.c:1179`
- Extent list first 2 entries (bird/turtle extents) — `fmain.c:339-340`

### Combat/Encounter State
- encounter_type — `fmain.c:579`
- leader — `fmain.c:576`
- All active actors (anim_list[0..anix-1]) — positions, type, race, weapons, goals, vitality — `fmain.c:70`
- All 6 missiles (positions, type, speed, direction) — `fmain.c:78-85`
- Actor/missile indices (anix, anix2, mdex) — `fmain.c:75-76`

### Time/Environment
- daynight counter — `fmain.c:572`
- lightlevel — `fmain.c:572` 
- secret_timer, light_timer, freeze_timer — `fmain.c:577`
- active_carrier — `fmain.c:574`
- riding, flying, wcarry — `fmain.c:563`
- xtype (current extent type) — `fmain.c:575`
- princess (rescued flag) — `fmain.c:568`
- cheat1 (cheat mode flag) — `fmain.c:562` — persists across save!

### Asset Loading State
- actor_file, set_file — which sprite files are loaded — `fmain.c:573`
- region_num — current game region — `fmain.c:617`

### Menu State
- cmode (current menu mode) — `fmain.c:578` — saved but irrelevant since FILE menu returns to GAME

## 8. What Game State is NOT Saved

These variables are declared AFTER the 80-byte block or are not in any saved structure:

### Runtime Variables (after pad3 in memory — `fmain.c:583-604`)
- `viewstatus` — reset to 99 on load (`fmain2.c:1544`)
- `flasher` — rendering transient
- `actors_on_screen` — recalculated each frame
- `actors_loading` — reset to 0 on load (`fmain2.c:1548`)
- `safe_flag` — recalculated from position
- `battleflag` — combat transient
- `frustflag` — movement transient
- `quitflag` — application lifecycle
- `witchflag`, `wdir`, `goodfairy` — NPC screen presence (transient)
- `nearest`, `nearest_person`, `perdist`, `last_person` — proximity tracking (recalculated)
- `witchindex` — witch animation state
- `dayperiod` — derived from daynight
- `sleepwait` — temporary timer
- `encounter_number` — reset to 0 on load (`fmain2.c:1542`)
- `danger_level` — recalculated
- `encounter_x`, `encounter_y` — encounter origin (transient)
- `mixflag` — encounter mixing flag
- `wt` — wait timer, reset to 0 on load (`fmain2.c:1542`)

### Other Non-Saved State
- `pad4`, `pad5`, `pad6`, `pad7` — outside 80-byte window (`fmain.c:581`)
- `new_region`, `lregion` — region transition state (`fmain.c:617`)
- `current_loads` — disk cache tracking (`fmain.c:618`)
- Music state — song, tracker position (`mtrack.c`)
- Text scroll buffer contents
- Menu enabled states — rebuilt by `set_options()` on load (`fmain.c:3527`)
- `stuff` pointer — reassigned in `mod1save()` from `blist[brother-1].stuff` (`fmain.c:3627`)
- Display state — copper lists, bitplane setup, rendering buffers
- Input handler state — joystick/mouse position
- Extent list entries 2–21 — only first 2 (bird/turtle) saved (`fmain2.c:1519` uses `2 * sizeof(struct extent)`)

### Implications of NOT saving extent_list[2..21]
The first 2 extents are bird and turtle — their positions can change during gameplay (via `move_extent()` at `fmain2.c:1561`). Extents 2+ are static (dragon, spider pit, etc.) and are reset from initializers on program start. This means only carrier (bird/turtle) extents need persistence.

## 9. Save Slot System

### Slot Names and File Paths
- 8 slots: A through H
- `fmain2.c:1502` — `savename[4] = 'A' + hit` where hit=0..7
- Hard drive: `A.faery` through `H.faery` in current directory
- Floppy: `df0:A.faery` or `df1:A.faery`

### Existing Save Files in Repository
- `game/C.faery` — save slot C (hit=2)
- `game/E.faery` — save slot E (hit=4)

### No Slot Metadata
There is no summary or preview data per slot — the FILE menu shows only the letters A–H. All slots are always available for selection regardless of whether a save exists. Attempting to load a nonexistent slot will fail with an Open error.

## 10. locktest() — Disk Presence Check — `fmain2.c:1401-1405`

```c
locktest(name,access) char *name; long access;
{   flock = Lock(name,access);
    if (flock) UnLock(flock);
    return (int)flock;
}
```

Uses AmigaDOS `Lock()` to test file/device existence. Lock is immediately released. Returns nonzero if the path exists and is accessible.

## 11. Existing Decode Tool

`tools/decode_savegame.py` — a complete Python decoder for the save file format. Handles all 11 blocks, prints human-readable output. Verified consistent with the source code analysis above.

## References Found

- `fmain2.c:1390` — write — `BYTE svflag;` — direction flag declaration
- `fmain2.c:1391` — write — `long svfile, sverr;` — file handle and error
- `fmain2.c:1392` — write — `char savename[] = "df1:A.faery";` — template filename
- `fmain2.c:1401-1405` — call — `locktest()` — disk presence check
- `fmain2.c:1442-1450` — call — `waitnewdisk()` — wait for disk insertion
- `fmain2.c:1474-1549` — call — `savegame(hit)` — main save/load function
- `fmain2.c:1502` — write — `savename[4] = 'A' + hit` — slot letter
- `fmain2.c:1503` — call — `Open(name,1006)` — MODE_NEWFILE for save
- `fmain2.c:1504` — call — `Open(name,1005)` — MODE_OLDFILE for load
- `fmain2.c:1508` — call — `saveload(&map_x,80)` — misc vars block
- `fmain2.c:1511` — call — `saveload(&region_num,2)` — region
- `fmain2.c:1514` — call — `saveload(&anix,6)` — actor indices
- `fmain2.c:1515` — call — `saveload(anim_list,anix*sizeof(struct shape))` — actors
- `fmain2.c:1519` — call — `saveload(extent_list,2*sizeof(struct extent))` — extents (first 2 only)
- `fmain2.c:1522` — call — `saveload(ob_listg,glbobs*sizeof(struct object))` — global objects
- `fmain2.c:1523` — call — `saveload(mapobs,20)` — region object counts
- `fmain2.c:1524` — call — `saveload(dstobs,20)` — distributed flags
- `fmain2.c:1525-1526` — call — `saveload(ob_table[i],...)` loop — regional objects
- `fmain2.c:1528` — call — `Close(svfile)` — close file
- `fmain2.c:1541-1548` — read — post-load restoration logic
- `fmain2.c:1553-1558` — call — `saveload()` — low-level block I/O
- `fmain.c:432` — read — `UBYTE julstuff[35], philstuff[35], kevstuff[35]` — inventory arrays
- `fmain.c:494` — read — `enum cmodes` — menu mode definitions
- `fmain.c:540` — read — FILE menu definition: `{ labelB, 10,5, ...}`
- `fmain.c:557-581` — read — 80-byte misc variables block: map_x through pad3
- `fmain.c:583-604` — read — variables NOT in 80-byte block (after pad3)
- `fmain.c:617` — read — `region_num` declaration
- `fmain.c:70` — read — `struct shape anim_list[20]`
- `fmain.c:75-76` — read — `short anix, anix2; short mdex;`
- `fmain.c:78-85` — read — `struct missile` definition
- `fmain.c:335-345` — read — `struct extent` definition and `extent_list` initializer
- `fmain.c:3445-3471` — read — `do_option` GAME/SAVEX/FILE handlers
- `fmain.c:3527-3545` — read — `set_options()` — menu refresh from inventory
- `fmain.c:3621-3631` — call — `mod1save()` — inventories + missiles
- `fmain2.c:673` — call — `shape_read()` — reload sprites after load

## Cross-Cutting Findings

- **cheat1 persists in saves** (`fmain.c:562`): The cheat flag is at offset 18 in the 80-byte block and survives save/load cycles. A player who enables cheats and saves will have cheats enabled when loading.
- **cmode saved but overwritten** (`fmain.c:578`): The menu mode is saved in the 80-byte block at offset 70, but after `savegame()` returns, `do_option` calls `gomenu(GAME)` (`fmain.c:3471`), overwriting the loaded cmode immediately.
- **encounter_type contradiction**: It's saved in the 80-byte block (offset 72, `fmain.c:579`) but then explicitly reset to 0 on load (`fmain2.c:1548`). The saved value is overwritten.
- **Only 2 of 22 extents saved**: `extent_list` has 22 entries (`fmain.c:339-370`), but `saveload` only writes `2 * sizeof(struct extent)` = 24 bytes (entries 0–1: bird and turtle). All other extents are static initializers. This means bird/turtle positions are the only mutable extents.
- **stuff pointer reset in mod1save** (`fmain.c:3627`): `stuff = blist[brother-1].stuff;` — on load, this correctly re-establishes which brother's inventory array `stuff` points to, using the `brother` value already loaded from the 80-byte block.
- **Object count mutation** (`fmain2.c:1233`): `mapobs[region_num]++` in the distribute-objects code means object counts can grow from their initial values. Saving captures these grown counts.
- **decode_savegame.py** (`tools/decode_savegame.py`): Existing verification tool that validates this analysis. Format documentation in its docstring matches these findings.

## Unresolved

None — all questions answered with source citations.

## Refinement Log

- 2026-04-06: Initial complete discovery pass. Traced all save/load code paths, documented file format, catalogued saved vs. unsaved state, verified against existing decode_savegame.py tool.
