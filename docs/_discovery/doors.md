# Discovery: Door System

**Status**: complete
**Investigated**: 2026-04-05
**Requested by**: orchestrator
**Prompt summary**: Trace the complete door system — doorlist table, doorfind algorithm, door types, keys, region transitions, indoor/outdoor handling.

## Door Data Structures

### struct door (fmain.c:233-239)

```c
struct door {
    unsigned short xc1, yc1;  /* outside image coords relative to F1 */
    unsigned short xc2, yc2;  /* inside image coords relative to F9 */
    char type;                /* door visual/orientation type */
    char secs;                /* 1=buildings (region 8), 2=dungeons (region 9) */
};
```

### Door Type Constants (fmain.c:210-229)

Source comment: "three types of doors + F1/F9 doors, F1/F10 doors and F9/F10 doors" — fmain.c:210

"horizontals all have lsb set" — fmain.c:212

| Constant | Value | Orientation (bit 0) | Used in doorlist | Used in open_list |
|----------|-------|---------------------|------------------|-------------------|
| HWOOD    | 1     | Horizontal          | Yes (16 entries) | Yes               |
| VWOOD    | 2     | Vertical            | Yes (3 entries)  | Yes               |
| HSTONE   | 3     | Horizontal          | Yes (11 entries) | Yes               |
| VSTONE   | 4     | Vertical            | No               | No                |
| HCITY    | 5     | Horizontal          | No               | No                |
| VCITY    | 6     | Vertical            | No               | No                |
| CRYST    | 7     | Horizontal          | Yes (2 entries)  | Yes               |
| SECRET   | 8     | Vertical            | No               | Yes (open_list only) |
| BLACK    | 9     | Horizontal          | Yes (5 entries)  | No                |
| MARBLE   | 10    | Vertical            | Yes (7 entries)  | Yes               |
| LOG      | 11    | Horizontal          | Yes (10 entries) | No                |
| (12)     | 12    | —                   | No               | No                |
| HSTON2   | 13    | Horizontal          | Yes (12 entries) | No                |
| VSTON2   | 14    | Vertical            | Yes (2 entries)  | No                |
| STAIR    | 15    | Horizontal          | Yes (4 entries)  | No                |
| (16)     | 16    | —                   | No               | No                |
| DESERT   | 17    | Horizontal          | Yes (5 entries)  | No                |
| CAVE     | 18    | Vertical            | Yes (4 entries)  | No                |
| VLOG     | 18    | Vertical            | Yes (10 entries) | No                |

**Critical**: CAVE and VLOG share value 18 — fmain.c:228-229. All code checking `d->type == CAVE` also catches VLOG entries. Both use the same teleportation offset (xc2+24, yc2+16 inbound; xc1-4, yc1+16 outbound).

### secs Field

| Value | Target Region | File Index Entry | Description |
|-------|--------------|------------------|-------------|
| 1     | 8            | F9 — inside of buildings | fmain.c:624 |
| 2     | 9            | F10 — dungeons and caves | fmain.c:625 |

Region assignment at fmain.c:1926: `if (d->secs == 1) new_region = 8; else new_region = 9;`

## doorlist — Complete 86-Entry Table

Source: fmain.c:240-325. `#define DOORCOUNT 86` at fmain.c:231.

Table is sorted by xc1 (ascending) to support binary search in the outdoor→indoor code path.

| Idx | xc1    | yc1    | xc2    | yc2    | Type   | Secs | Comment            |
|-----|--------|--------|--------|--------|--------|------|--------------------|
| 0   | 0x1170 | 0x5060 | 0x2870 | 0x8b60 | HWOOD  | 1    | desert fort        |
| 1   | 0x1170 | 0x5060 | 0x2870 | 0x8b60 | HWOOD  | 1    | desert fort        |
| 2   | 0x1170 | 0x5060 | 0x2870 | 0x8b60 | HWOOD  | 1    | desert fort        |
| 3   | 0x1170 | 0x5060 | 0x2870 | 0x8b60 | HWOOD  | 1    | desert fort        |
| 4   | 0x1390 | 0x1b60 | 0x1980 | 0x8c60 | CAVE   | 2    | dragon cave        |
| 5   | 0x1770 | 0x6aa0 | 0x2270 | 0x96a0 | BLACK  | 1    | pass fort          |
| 6   | 0x1970 | 0x62a0 | 0x1f70 | 0x96a0 | BLACK  | 1    | gate fort          |
| 7   | 0x1aa0 | 0x4ba0 | 0x13a0 | 0x95a0 | DESERT | 1    | oasis #1           |
| 8   | 0x1aa0 | 0x4c60 | 0x13a0 | 0x9760 | DESERT | 1    | oasis #4           |
| 9   | 0x1b20 | 0x4b60 | 0x1720 | 0x9560 | DESERT | 1    | oasis #2           |
| 10  | 0x1b80 | 0x4b80 | 0x1580 | 0x9580 | DESERT | 1    | oasis #3           |
| 11  | 0x1b80 | 0x4c40 | 0x1580 | 0x9740 | DESERT | 1    | oasis #5           |
| 12  | 0x1e70 | 0x3b60 | 0x2880 | 0x9c60 | HSTONE | 1    | west keep          |
| 13  | 0x2480 | 0x33a0 | 0x2e80 | 0x8da0 | HWOOD  | 1    | swamp shack        |
| 14  | 0x2960 | 0x8760 | 0x2b00 | 0x92c0 | STAIR  | 1    | stargate forwards  |
| 15  | 0x2b00 | 0x92c0 | 0x2960 | 0x8780 | STAIR  | 2    | stargate backwards |
| 16  | 0x2c00 | 0x7160 | 0x2af0 | 0x9360 | BLACK  | 1    | doom tower         |
| 17  | 0x2f70 | 0x2e60 | 0x3180 | 0x9a60 | HSTONE | 1    | lakeside keep      |
| 18  | 0x2f70 | 0x63a0 | 0x1c70 | 0x96a0 | BLACK  | 1    | plain fort         |
| 19  | 0x3180 | 0x38c0 | 0x2780 | 0x98c0 | HWOOD  | 1    | road's end inn     |
| 20  | 0x3470 | 0x4b60 | 0x0470 | 0x8ee0 | STAIR  | 2    | tombs              |
| 21  | 0x3DE0 | 0x1BC0 | 0x2EE0 | 0x93C0 | CRYST  | 1    | crystal palace     |
| 22  | 0x3E00 | 0x1BC0 | 0x2F00 | 0x93C0 | CRYST  | 1    | crystal palace     |
| 23  | 0x4270 | 0x2560 | 0x2e80 | 0x9a60 | HSTONE | 1    | coast keepDB       |
| 24  | 0x4280 | 0x3bc0 | 0x2980 | 0x98c0 | HWOOD  | 1    | friendly inn       |
| 25  | 0x45e0 | 0x5380 | 0x25d0 | 0x9680 | MARBLE | 1    | mountain keep      |
| 26  | 0x4780 | 0x2fc0 | 0x2580 | 0x98c0 | HWOOD  | 1    | forest inn         |
| 27  | 0x4860 | 0x6640 | 0x1c60 | 0x9a40 | VLOG   | 1    | cabin yard #7      |
| 28  | 0x4890 | 0x66a0 | 0x1c90 | 0x9aa0 | LOG    | 1    | cabin #7           |
| 29  | 0x4960 | 0x5b40 | 0x2260 | 0x9a40 | VLOG   | 1    | cabin yard #6      |
| 30  | 0x4990 | 0x5ba0 | 0x2290 | 0x9aa0 | LOG    | 1    | cabin #6           |
| 31  | 0x49a0 | 0x3cc0 | 0x0ba0 | 0x82c0 | VWOOD  | 1    | village #2         |
| 32  | 0x49d0 | 0x3dc0 | 0x0bd0 | 0x84c0 | VWOOD  | 1    | village #1.a       |
| 33  | 0x49d0 | 0x3e00 | 0x0bd0 | 0x8500 | VWOOD  | 1    | village #1.b       |
| 34  | 0x4a10 | 0x3c80 | 0x0d10 | 0x8280 | HWOOD  | 1    | village #3         |
| 35  | 0x4a10 | 0x3d40 | 0x0f10 | 0x8340 | HWOOD  | 1    | village #5         |
| 36  | 0x4a30 | 0x3dc0 | 0x0e30 | 0x85c0 | HWOOD  | 1    | village #7         |
| 37  | 0x4a60 | 0x3e80 | 0x1060 | 0x8580 | HWOOD  | 1    | village #8         |
| 38  | 0x4a70 | 0x3c80 | 0x1370 | 0x8280 | HWOOD  | 1    | village #4         |
| 39  | 0x4a80 | 0x3d40 | 0x1190 | 0x8340 | HWOOD  | 1    | village #6         |
| 40  | 0x4c70 | 0x3260 | 0x2580 | 0x9c60 | HSTONE | 1    | crag keep          |
| 41  | 0x4d60 | 0x5440 | 0x1f60 | 0x9c40 | VLOG   | 1    | cabin #2           |
| 42  | 0x4d90 | 0x4380 | 0x3080 | 0x8d80 | HSTON2 | 1    | crypt              |
| 43  | 0x4d90 | 0x54a0 | 0x1f90 | 0x9ca0 | LOG    | 1    | cabin yard #2      |
| 44  | 0x4de0 | 0x6b80 | 0x29d0 | 0x9680 | MARBLE | 1    | river keep         |
| 45  | 0x5360 | 0x5840 | 0x2260 | 0x9840 | VLOG   | 1    | cabin yard #3      |
| 46  | 0x5390 | 0x58a0 | 0x2290 | 0x98a0 | LOG    | 1    | cabin #3           |
| 47  | 0x5460 | 0x4540 | 0x1c60 | 0x9840 | VLOG   | 1    | cabin yard #1      |
| 48  | 0x5470 | 0x6480 | 0x2c80 | 0x8d80 | HSTONE | 1    | elf glade          |
| 49  | 0x5490 | 0x45a0 | 0x1c90 | 0x98a0 | LOG    | 1    | cabin #1           |
| 50  | 0x55f0 | 0x52e0 | 0x16e0 | 0x83e0 | MARBLE | 1    | main castle        |
| 51  | 0x56c0 | 0x53c0 | 0x1bc0 | 0x84c0 | HSTON2 | 1    | city #15.a         |
| 52  | 0x56c0 | 0x5440 | 0x19c0 | 0x8540 | HSTON2 | 1    | city #17           |
| 53  | 0x56f0 | 0x51a0 | 0x19f0 | 0x82a0 | HSTON2 | 1    | city #10           |
| 54  | 0x5700 | 0x5240 | 0x1df0 | 0x8340 | VSTON2 | 1    | city #12           |
| 55  | 0x5710 | 0x5440 | 0x1c10 | 0x8640 | HSTON2 | 1    | city #18           |
| 56  | 0x5730 | 0x5300 | 0x1a50 | 0x8400 | HSTON2 | 1    | city #14           |
| 57  | 0x5730 | 0x5380 | 0x1c30 | 0x8480 | VSTON2 | 1    | city #15.b         |
| 58  | 0x5750 | 0x51a0 | 0x1c60 | 0x82a0 | HSTON2 | 1    | city #11           |
| 59  | 0x5750 | 0x5260 | 0x2050 | 0x8360 | HSTON2 | 1    | city #13           |
| 60  | 0x5760 | 0x53c0 | 0x2060 | 0x84c0 | HSTON2 | 1    | city #16           |
| 61  | 0x5760 | 0x5440 | 0x1e60 | 0x8540 | HSTON2 | 1    | city #19           |
| 62  | 0x5860 | 0x5d40 | 0x1c60 | 0x9a40 | VLOG   | 1    | cabin yard #4      |
| 63  | 0x5890 | 0x5da0 | 0x1c90 | 0x9ca0 | LOG    | 1    | cabin #4           |
| 64  | 0x58c0 | 0x2e60 | 0x0ac0 | 0x8860 | CAVE   | 2    | troll cave         |
| 65  | 0x5960 | 0x6f40 | 0x2260 | 0x9a40 | VLOG   | 1    | cabin yard #9      |
| 66  | 0x5990 | 0x6fa0 | 0x2290 | 0x9ca0 | LOG    | 1    | cabin #9           |
| 67  | 0x59a0 | 0x6760 | 0x2aa0 | 0x8b60 | STAIR  | 1    | unreachable castle |
| 68  | 0x59e0 | 0x5880 | 0x27d0 | 0x9680 | MARBLE | 1    | farm keep          |
| 69  | 0x5e70 | 0x1a60 | 0x2580 | 0x9a60 | HSTONE | 1    | north keep         |
| 70  | 0x5ec0 | 0x2960 | 0x11c0 | 0x8b60 | CAVE   | 2    | spider exit        |
| 71  | 0x6060 | 0x7240 | 0x1960 | 0x9c40 | VLOG   | 1    | cabin yard #10     |
| 72  | 0x6090 | 0x72a0 | 0x1990 | 0x9ca0 | LOG    | 1    | cabin #10          |
| 73  | 0x60f0 | 0x32c0 | 0x25f0 | 0x8bc0 | HSTONE | 1    | mammoth manor      |
| 74  | 0x64c0 | 0x1860 | 0x03c0 | 0x8660 | CAVE   | 2    | maze cave 2        |
| 75  | 0x6560 | 0x5d40 | 0x1f60 | 0x9a40 | VLOG   | 1    | cabin yard #5      |
| 76  | 0x6590 | 0x5da0 | 0x1f90 | 0x98a0 | LOG    | 1    | cabin #5           |
| 77  | 0x65c0 | 0x1a20 | 0x04b0 | 0x8840 | BLACK  | 2    | maze cave 1        |
| 78  | 0x6670 | 0x2a60 | 0x2b80 | 0x9a60 | HSTONE | 1    | glade keep         |
| 79  | 0x6800 | 0x1b60 | 0x2af0 | 0x9060 | BLACK  | 1    | witch's castle     |
| 80  | 0x6b50 | 0x4380 | 0x2850 | 0x8d80 | HSTON2 | 1    | light house        |
| 81  | 0x6be0 | 0x7c80 | 0x2bd0 | 0x9680 | MARBLE | 1    | lonely keep        |
| 82  | 0x6c70 | 0x2e60 | 0x2880 | 0x9a60 | HSTONE | 1    | sea keep           |
| 83  | 0x6d60 | 0x6840 | 0x1f60 | 0x9a40 | VLOG   | 1    | cabin yard #8      |
| 84  | 0x6d90 | 0x68a0 | 0x1f90 | 0x9aa0 | LOG    | 1    | cabin #8           |
| 85  | 0x6ee0 | 0x5280 | 0x31d0 | 0x9680 | MARBLE | 1    | point keep         |

### Notable Patterns

1. **Entries 0-3 are identical** — four copies of the same desert fort door. Likely padding or a binary-search artifact (the search needs entries to exist at the start of the sorted range). — fmain.c:240-243
2. **Cabin pairs** — Each cabin has two entries: a VLOG "yard" door and a LOG "cabin" door with slightly offset coordinates. The yard is the outdoor gate, the cabin is the actual building door. There are 10 cabins (20 entries total).
3. **Crystal palace has two entries** (idx 21-22) — two adjacent doors for the same building.
4. **Stargate pair** (idx 14-15) — bidirectional portal. Entry 14 goes forward (to region 8), entry 15 goes backward (to region 9). See Stargate section below.
5. **Village cluster** (idx 31-39) — 9 doors for the village. VWOOD (3) and HWOOD (6) types.
6. **City cluster** (idx 50-61) — 12 doors (main castle + 10 city buildings + city #19). MARBLE (1), HSTON2 (9), VSTON2 (2) types.

### Door Type Distribution by secs

- **secs=1** (→ region 8, buildings): 78 entries
- **secs=2** (→ region 9, dungeons/caves): 8 entries — dragon cave (4), stargate backwards (15), tombs (20), troll cave (64), spider exit (70), maze cave 2 (74), maze cave 1 (77), stargate backwards technically connects regions

## doorfind Algorithm

Source: fmain.c:1081-1128.

`doorfind()` opens LOCKED doors (terrain tile type 15) by modifying map tiles. This is SEPARATE from the doorlist traversal system — doorfind deals with `open_list[]`, not `doorlist[]`.

### Function Signature

```c
doorfind(x,y,keytype) register USHORT x,y; register ULONG keytype;
```

- `x, y`: pixel coordinates near the door
- `keytype`: 0 = no key (just bumping), 1-6 = key type being used

### Algorithm

1. **Find terrain type 15** — fmain.c:1083-1085
   - Check `px_to_im(x, y)` for terrain type 15
   - Try `px_to_im(x+4, y)` if not found
   - Try `px_to_im(x-8, y)` if still not found
   - If none return 15, return FALSE

2. **Find top-left corner of door** — fmain.c:1087-1089
   - Scan left: if `px_to_im(x-16, y) == 15` then `x -= 16` (twice max)
   - Scan down: if `px_to_im(x, y+32) == 15` then `y += 32`

3. **Convert to image coordinates** — fmain.c:1090-1091
   - `x >>= 4` (divide by 16 → image X)
   - `y >>= 5` (divide by 32 → image Y)

4. **Get sector and region IDs** — fmain.c:1093-1095
   - `sec_id = *(mapxy(x, y))` — the tile ID at this map position
   - `reg_id = current_loads.image[(sec_id >> 6)]` — which image block this tile belongs to

5. **Search open_list** — fmain.c:1097-1118
   - Iterate `j = 0..16` through `open_list[17]`
   - Match: `open_list[j].map_id == reg_id && open_list[j].door_id == sec_id`
   - Key check: `open_list[j].keytype == 0` (no key needed) OR `open_list[j].keytype == keytype`
   - If match: replace map tiles and print "It opened." — fmain.c:1117

6. **Tile replacement** — fmain.c:1100-1114
   - Primary tile: `*(mapxy(x,y)) = open_list[j].new1`
   - Secondary tile: `k = open_list[j].new2`. If nonzero:
     - `above == 1`: place k at (x, y-1) — above
     - `above == 3`: place k at (x-1, y) — left/back
     - `above == 4`: special cabinet layout — place 87 at (x,y-1), 86 at (x+1,y), 88 at (x+1,y-1)
     - else: place k at (x+1, y) — right/side. If `above != 2`, also place `above` at (x+2, y)

7. **Failure** — fmain.c:1121-1123
   - If no match found and `!bumped && !keytype`: print "It's locked."
   - Set `bumped = 1` to suppress repeated messages
   - Return FALSE

### px_to_im (fsubs.asm:542-617)

Converts pixel coordinates to terrain type:
1. Takes (x, y) pixel coordinates
2. Computes image coordinates: imx = x >> 4, imy = y >> 5
3. Computes sector coordinates: secx = imx >> 4 - xreg, secy = imy >> 3 - yreg
4. Looks up sector number from `map_mem[secy * 128 + secx + xreg]`
5. Computes tile offset: `sector_mem[sec_num * 128 + (imy & 7) * 16 + (imx & 15)]`
6. Reads terrain type from `terra_mem[tile_id * 4 + 1] >> 4` (upper nibble of byte 1)

## Door Types & Keys (open_list)

### Key Enum (fmain.c:1049)

```c
enum ky {NOKEY=0, GOLD=1, GREEN=2, KBLUE=3, RED=4, GREY=5, WHITE=6};
```

### Key Inventory Mapping

Keys stored in `stuff[KEYBASE + n]` where KEYBASE=16 — fmain.c:427.

| Key Type | Enum Value | stuff[] Index | Item Name  |
|----------|------------|---------------|------------|
| GOLD     | 1          | stuff[16]     | Gold Key   |
| GREEN    | 2          | stuff[17]     | Green Key  |
| KBLUE    | 3          | stuff[18]     | Blue Key   |
| RED      | 4          | stuff[19]     | Red Key    |
| GREY     | 5          | stuff[20]     | Grey Key   |
| WHITE    | 6          | stuff[21]     | White Key  |

Source: inv_list at fmain.c:396-407.

### Key Usage (Menu Handler)

Source: fmain.c:3472-3488 (KEYS case in `do_option()`).

```c
case KEYS:
    hit -= 5;
    bumped = 0;
    if (stuff[hit+KEYBASE])
    {   for (i=0; i<9; i++)
        {   x = newx(hero_x,i,16);
            y = newy(hero_y,i,16);
            if (doorfind(x,y,hit+1)) { stuff[hit+KEYBASE]--; break; }
        }
        if (i > 8) { /* "tried a [key] but it didn't fit." */ }
    }
```

Algorithm:
1. Player selects a key from inventory menu
2. Check all 9 directions (0-8) at distance 16 pixels from hero
3. Call `doorfind()` with keytype = hit+1 (1-6 corresponding to enum values)
4. If successful, decrement key count (`stuff[hit+KEYBASE]--`)
5. Key is consumed on use

### struct door_open (fmain.c:1053-1058)

```c
struct door_open {
    UBYTE   door_id;        /* sector tile ID of the closed door */
    USHORT  map_id;         /* image block number where this tile exists */
    UBYTE   new1, new2;     /* replacement tile IDs (open door graphics) */
    UBYTE   above;          /* tile placement: 0=none, 1=above, 2=side, 3=back, 4=special */
    UBYTE   keytype;        /* key needed: 0=none, 1=GOLD, 2=GREEN, etc. */
};
```

### open_list[17] — Complete Table (fmain.c:1059-1078)

| Idx | door_id | map_id | new1 | new2 | above | keytype | Comment     |
|-----|---------|--------|------|------|-------|---------|-------------|
| 0   | 64      | 360    | 123  | 124  | 2     | GREEN   | HSTONE      |
| 1   | 120     | 360    | 125  | 126  | 2     | NOKEY   | HWOOD       |
| 2   | 122     | 360    | 127  | 0    | 0     | NOKEY   | VWOOD       |
| 3   | 64      | 280    | 124  | 125  | 2     | GREY    | HSTONE2     |
| 4   | 77      | 280    | 126  | 0    | 0     | GREY    | VSTONE2     |
| 5   | 82      | 480    | 84   | 85   | 2     | KBLUE   | CRYST       |
| 6   | 64      | 480    | 105  | 106  | 2     | GREEN   | OASIS       |
| 7   | 128     | 240    | 154  | 155  | 1     | WHITE   | MARBLE      |
| 8   | 39      | 680    | 41   | 42   | 2     | GOLD    | HGATE       |
| 9   | 25      | 680    | 27   | 26   | 3     | GOLD    | VGATE       |
| 10  | 114     | 760    | 116  | 117  | 1     | RED     | SECRET      |
| 11  | 118     | 760    | 116  | 117  | 1     | GREY    | TUNNEL      |
| 12  | 136     | 800    | 133  | 134  | 135   | GOLD    | GOLDEN      |
| 13  | 187     | 800    | 76   | 77   | 2     | NOKEY   | HSTON3      |
| 14  | 73      | 720    | 75   | 0    | 0     | NOKEY   | VSTON3      |
| 15  | 165     | 800    | 85   | 86   | 4     | GREEN   | CABINET     |
| 16  | 210     | 840    | 208  | 209  | 2     | NOKEY   | BLUE        |

### Open_list map_id to File Index Mapping

The `map_id` field matches `current_loads.image[N]` values from `file_index[]` (fmain.c:615-625):

| map_id | Appears In Region(s) | Image Block |
|--------|---------------------|-------------|
| 240    | Region 8 (F9)       | image[2]    |
| 280    | Region 8 (F9) / Regions 1,3 | image[1] for F2/F4, F9 |
| 360    | Regions 0,2 (F1/F3) | image[1]    |
| 480    | Regions 0,4 (F1/F5) | image[2] for F1, image[0] for F5 |
| 680    | Region 8 (F9)       | image[0]    |
| 720    | Region 9 (F10)      | image[1]    |
| 760    | Region 9 (F10)      | image[2]    |
| 800    | Regions 8,9 (F9/F10)| image[2] for F9, image[3] for F10 |
| 840    | Region 9 (F10)      | image[3]    |

### Key-to-Door Mappings (Which Keys Open What)

| Key   | open_list Entries          | Locations                |
|-------|---------------------------|--------------------------|
| NOKEY | idx 1 (HWOOD), 2 (VWOOD), 13 (HSTON3), 14 (VSTON3), 16 (BLUE) | Various unlocked doors |
| GOLD  | idx 8 (HGATE), 9 (VGATE), 12 (GOLDEN) | Gates and golden doors |
| GREEN | idx 0 (HSTONE), 6 (OASIS), 15 (CABINET) | Stone doors, oasis interior, cabinets |
| KBLUE | idx 5 (CRYST)             | Crystal palace interiors  |
| RED   | idx 10 (SECRET)           | Secret passages            |
| GREY  | idx 3 (HSTONE2), 4 (VSTONE2), 11 (TUNNEL) | Stone doors and tunnels |
| WHITE | idx 7 (MARBLE)            | Marble/keep doors          |

## Region Transitions

### Region Map (file_index, fmain.c:615-625)

| Region | File | Description              | Image Sets         |
|--------|------|--------------------------|-------------------|
| 0      | F1   | Snowy region             | 320,480,520,560    |
| 1      | F2   | Witch wood               | 320,360,400,440    |
| 2      | F3   | Swampy region            | 320,360,520,560    |
| 3      | F4   | Plains and rocks         | 320,360,400,440    |
| 4      | F5   | Desert area              | 320,480,520,600    |
| 5      | F6   | Bay / city / farms       | 320,280,240,200    |
| 6      | F7   | Volcanic                 | 320,640,520,600    |
| 7      | F8   | Forest and wilderness    | 320,280,240,200    |
| 8      | F9   | Inside of buildings      | 680,720,800,840    |
| 9      | F10  | Dungeons and caves       | 680,760,800,840    |

### Outdoor → Indoor Transition (fmain.c:1894-1935)

Triggered in main loop when `region_num < 8` and player position matches a doorlist entry:

1. **Alignment** — fmain.c:1898-1899: `xtest = hero_x & 0xfff0; ytest = hero_y & 0xffe0;` (align to 16×32 tile grid)
2. **Riding check** — fmain.c:1901: `if (riding) goto nodoor3;` — cannot enter doors while mounted
3. **Binary search** on doorlist by xc1 — fmain.c:1903-1913
4. **Orientation check** — fmain.c:1914-1916:
   - Horizontal doors (`type & 1`): skip if `hero_y & 0x10` is set (player not at door's Y position precisely)
   - Vertical doors: skip if `(hero_x & 15) > 6` (player not close enough to left edge)
5. **DESERT restriction** — fmain.c:1919: `if (d->type == DESERT && (stuff[STATBASE]<5)) break;` — need ≥5 Gold Statues to enter oasis doors
6. **Destination offset by type** — fmain.c:1920-1923:
   - CAVE/VLOG (type==18): `xtest = xc2 + 24; ytest = yc2 + 16`
   - Horizontal (type & 1): `xtest = xc2 + 16; ytest = yc2`
   - Vertical: `xtest = xc2 - 1; ytest = yc2 + 16`
7. **Set new region** — fmain.c:1926: `if (d->secs == 1) new_region = 8; else new_region = 9;`
8. **Teleport** — fmain.c:1927: `xfer(xtest, ytest, FALSE)` — FALSE means don't recalculate region from position
9. **Place lookup** — fmain.c:1928: `find_place(2)` — flag=2 displays the location name
10. **Visual transition** — fmain.c:1929: `fade_page(100,100,100,TRUE,pagecolors)` — indoor color fade

### Indoor → Outdoor Transition (fmain.c:1936-1954)

Triggered when `region_num >= 8` and player position matches a doorlist entry's xc2:

1. **Linear scan** through all DOORCOUNT entries — fmain.c:1937
2. **Match on xc2/yc2** — fmain.c:1938-1939: `d->yc2==ytest && (d->xc2==xtest || (d->xc2==xtest-16 && d->type & 1))`
   - Horizontal doors match xc2 or xc2-16 (wider hit zone)
3. **Orientation check** — fmain.c:1940-1941:
   - Horizontal: skip if `(hero_y & 0x10) == 0`
   - Vertical: skip if `(hero_x & 15) < 2`
4. **Destination offset by type** — fmain.c:1943-1946:
   - CAVE/VLOG: `xtest = xc1 - 4; ytest = yc1 + 16`
   - Horizontal: `xtest = xc1 + 16; ytest = yc1 + 34`
   - Vertical: `xtest = xc1 + 20; ytest = yc1 + 16`
5. **Teleport** — fmain.c:1948: `xfer(xtest, ytest, TRUE)` — TRUE means recalculate region from destination position
6. **Place lookup** — fmain.c:1949: `find_place(FALSE)` — no location message displayed
7. **No fade_page** — exiting is instant, no color fade

### xfer() Function (fmain.c:2625-2645)

```c
xfer(xtest,ytest,flag) {
    map_x += (xtest-hero_x);
    map_y += (ytest-hero_y);
    hero_x = anim_list[0].abs_x = xtest;
    hero_y = anim_list[0].abs_y = ytest;
    encounter_number = 0;
    if (flag) {
        // Recalculate region from map coordinates
        xtest = (map_x + 151) >> 8;
        ytest = (map_y + 64) >> 8;
        xtest = (xtest >> 6) & 1;
        ytest = (ytest >> 5) & 7;
        new_region = xtest + (ytest + ytest);
    }
    keydir = 0;
    load_all();         // Triggers disk loading if region changed
    gen_mini();         // Update minimap, sets xreg/yreg
    viewstatus = 99;    // Force screen redraw
    setmood(TRUE);      // Change music for new area
    while (proxcheck(hero_x,hero_y,0)) hero_y++;  // Nudge out of walls
}
```

### Collision-Triggered Door Opening (fmain.c:1607)

When the player bumps into terrain type 15 during movement:
```c
if (i==0 && j==15) { doorfind(xtest,ytest,0); }
```
Source: fmain.c:1607. This is the `i==0` check (player character only, not NPCs). The `proxcheck()` return of 15 identifies a door tile. `doorfind` is called with `keytype=0`, which only opens NOKEY doors in open_list.

## Indoor/Outdoor Display Differences

### Music (setmood, fmain.c:2936-2956)

When `region_num > 7`:
- Music offset = `5*4` (indoor track set) — fmain.c:2944
- Region 9 (dungeons): `new_wave[10] = 0x0307` — fmain.c:2945
- Region 8 (buildings): `new_wave[10] = 0x0100` — fmain.c:2946

### Color Palette (fade_page, fmain2.c:381-387)

Background color (palette entry 31) varies by region:
- Region 4 (desert): `pagecolors[31] = 0x0980` (orange-brown) — fmain2.c:381
- Region 9 (dungeons): `pagecolors[31] = 0x0445` (dark grey-blue) — fmain2.c:384
  - If `secret_timer` active: `pagecolors[31] = 0x00f0` (green) — fmain2.c:383
- All others: `pagecolors[31] = 0x0bdf` (light blue sky) — fmain2.c:386

### Entering Transition Effects

- **Outdoor → Indoor**: `fade_page(100,100,100,TRUE,pagecolors)` called — smooth color transition — fmain.c:1929
- **Indoor → Outdoor**: No fade_page called — instant transition — fmain.c:1948-1949

## Stargate Portal

Entries 14-15 form a special bidirectional portal using STAIR type:

- **Entry 14** (stargate forwards): xc1=(0x2960,0x8760) → xc2=(0x2b00,0x92c0), secs=1 (region 8) — fmain.c:254
- **Entry 15** (stargate backwards): xc1=(0x2b00,0x92c0) → xc2=(0x2960,0x8780), secs=2 (region 9) — fmain.c:255

Note: Entry 15's xc1 equals entry 14's xc2 — they share the intermediate coordinate. Entry 14 transitions from the overworld to region 8 (buildings), while entry 15 transitions from region 8 to region 9 (dungeons).

The STAIR type (value 15) is treated as horizontal (`15 & 1 = 1`), using the horizontal destination offset: `xc2 + 16, yc2`.

## Quicksand → Dungeon Transition (fmain.c:1784-1791)

A separate non-door mechanism for entering region 9. When the player sinks fully (environ reaches 30) at `hero_sector == 181`:

```c
if (hero_sector == 181)
{   if (i==0)          // player character only
    {   k=0;
        new_region = 9;
        xfer(0x1080, 34950, FALSE);
        find_place(1);
    }
    else an->vitality = 0;  // NPCs die in quicksand
}
```

Source: fmain.c:1784-1791. This teleports the player to fixed coordinates (0x1080, 34950=0x8886) in region 9 (dungeons). NPCs in the same quicksand are killed.

## Cross-Cutting Findings

1. **VLOG == CAVE (both 18)** — fmain.c:228-229. Cabin yard doors are treated identically to cave doors in terms of teleportation offsets. This is intentional since both are vertically-oriented entrances, but it means any code checking `d->type == CAVE` also catches all cabin yards.

2. **DESERT doors gated by quest progress** — fmain.c:1919. Oasis entries (5 doors, indices 7-11) require `stuff[STATBASE] >= 5` (5+ Gold Statues). STATBASE=25 (fmain.c:428). The same check blocks region 4 map loading at fmain.c:3594.

3. **Riding blocks ALL doors** — fmain.c:1901. The `if (riding) goto nodoor3` check jumps past the entire door check. Players must dismount (horse=5, turtle=11, boat=1) before entering any door.

4. **Door opening modifies live map data** — fmain.c:1100-1114. `doorfind()` uses `mapxy()` to write new tile IDs directly into `sector_mem`. These changes persist only until the sector is reloaded from disk. There is no save/restore mechanism for opened doors — they reset when the region is reloaded.

5. **bumped flag prevents message spam** — fmain.c:1121-1123. The "It's locked." message only prints once per door encounter. `bumped` is reset to 0 when `proxcheck` returns non-15 (player moves away from door) at fmain.c:1608, and also reset to 0 in the KEYS menu handler at fmain.c:3474.

6. **Indoor→outdoor linear scan is O(86)** — fmain.c:1937-1953. Unlike the outdoor→indoor binary search, the indoor exit uses a linear scan of all 86 entries every frame the player stands on a door tile. This is less efficient but works because there are fewer indoor door positions.

7. **open_list vs doorlist are independent systems** — doorlist handles coordinate-based teleportation between outdoor/indoor maps. open_list handles tile modification for locked doors within maps. They operate on different data and serve different purposes, despite both being called "door" related.

8. **Defined but unused door types** — VSTONE (4), HCITY (5), VCITY (6) are defined at fmain.c:216-218 but never appear in doorlist[]. SECRET (8) appears only in open_list (idx 10), not in doorlist.

9. **viewstatus = 99** — set both in doorfind (fmain.c:1115) and xfer (fmain.c:2642). Value 99 means "corrupt display, needs full redraw" per fmain.c:583 comment. Ensures the screen is redrawn after door state changes or teleportation.

## Unresolved

1. **Why are entries 0-3 identical?** — Four copies of the same desert fort door at fmain.c:240-243. Possible explanations: binary search padding, multi-tile wide entrance, or development artifact. Cannot determine from code alone.

2. **Stargate coordinate ranges** — Entry 14's xc1 Y coordinate (0x8760) appears to be above MAXCOORD (0x8000). It's unclear whether this wraps, represents a map edge position, or is simply a valid coordinate in a larger-than-expected outdoor space. The map coordinate system and wrapping behavior need experimental verification.

3. **open_list map_id values** — The relationship between map_id values (240, 280, 360, etc.) and specific image block loading is complex. The exact mapping depends on which region loads which image blocks and how `current_loads.image[sec_id>>6]` resolves. Full verification would require an experiment decoding the actual map data.

4. **One-way doors** — The stargate backward entry (idx 15) has xc1 in the indoor coordinate range, so it can only be matched from the indoor→outdoor scan (matching xc2). This effectively makes it only usable going indoor-to-indoor (region 8 → region 9). Whether the reverse (region 9 → region 8) also works depends on whether entry 14's xc2 matches from region 9. The "unreachable castle" (idx 67, STAIR, secs=1) is also notable — its name suggests intentional inaccessibility.

5. **Secret doors** — The open_list has a SECRET entry (idx 10, map_id=760, keytype=RED) and TUNNEL (idx 11, map_id=760, keytype=GREY) but these are tile types within dungeon maps (region 9), not doorlist teleportation entries. Whether there are hidden doorlist entries or secret passages not in this data cannot be determined from code alone.

## Refinement Log

- 2026-04-05: Initial comprehensive discovery pass. All doorlist entries decoded, doorfind algorithm traced, open_list fully documented, region transition mechanism traced, indoor/outdoor display differences catalogued.
