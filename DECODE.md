# Game File Decoding Notes

This document is the canonical reference for reverse-engineering notes and
binary asset/file format details (`songs`, `v6`, and related data files).

For build/run setup, use `README.md`. For roadmap/task tracking, use
`PLAN.md` and `plan_status.toml`.

## `game/songs` — Music Score Data (5,984 bytes)

Loaded by `read_score()` in `fmain2.c`. Holds up to 28 sequencer tracks
organised as **7 song groups × 4 Paula voices**. The original stores them in a
simple length-prefixed format; no SMUS/IFF wrapper is used.

The active group is selected at runtime by `setmood()` in `fmain.c` based on
game state.  The Rust parser lives in `src/game/songs.rs`.

---

### File Layout

Each track is stored as:

| Field | Size | Description |
|-------|------|-------------|
| `packlen` | 4 bytes (big-endian `i32`) | Number of 16-bit words in this track's event stream |
| event bytes | `packlen × 2` bytes | Sequence of `(command, value)` byte pairs |

Tracks are read sequentially.  Loading stops when the cumulative byte count
reaches the 5,900-byte `scoremem` limit (`SCORE_SZ` in `fmain.c`).
All 28 tracks fit within that limit (5,872 bytes of event data + 112 bytes
of headers = 5,984 bytes total, matching the file size exactly).

---

### Event Encoding (from `gdriver.asm` → `_vblank_server` → `newnote`)

Every event is exactly two bytes `(command, value)`:

| Command byte | Meaning |
|---|---|
| 0 – 127 | **Note** — pitch index into `PTABLE` (78 entries; see layout below) |
| 128 (0x80) | **Rest** — silence for the given duration |
| 129 (0x81) | **Set Instrument** — `value & 0x0f` selects a slot from the `new_wave[]` instrument table |
| 144 (0x90) | **Set Tempo** — `value` is written directly to the tempo register (default 150) |
| 255 (0xFF) | **End Track** — `value ≠ 0` loops back to the start; `value = 0` stops the voice |
| other (bit 7 set) | Ignored (the ASM falls through to `newnote`) |

The **duration** of notes and rests comes from the `value` byte (bits 6–7
masked off), used as an index into `NOTE_DURATIONS[0..63]` — 64 tick counts
covering 8 note-length groups (4/4, 6/8, 3/4, 7/8, 5/4, 3/4 alt, 9/8, and a
duplicate 4/4).

The **pitch** byte (0–77) indexes into `PTABLE`, which stores
`(period, wave_offset)` pairs.  `period` is an Amiga Paula hardware period
register value.  The correct frequency formula is:
```
frequency = AMIGA_CLOCK_NTSC / (wave_len × period)
           = 3,579,545 / (wave_len × period)
```
where `wave_len = (32 - wave_offset) × 2` bytes.
`wave_offset` is a 16-bit–word offset into the 128-byte waveform in `wavmem`
(from `v6`) that selects which portion Paula loops, halving the loop length
each step to raise the pitch one octave.

`PTABLE` layout (78 entries across 7 ASM rows):

| Pitches | Entries | wave_offset | wave_len | Notes      | Frequency range |
|---------|---------|-------------|----------|------------|-----------------|
| 0–5     | 6       | 0           | 64       | D#1–G#1    | 38.9–51.9 Hz    |
| 6–17    | 12      | 0           | 64       | A1–G#2     | 55.0–103.8 Hz   |
| 18–29   | 12      | 0           | 64       | A2–G#3     | 110.0–207.7 Hz  |
| 30–41   | 12      | 16          | 32       | A3–G#4     | 220.0–415.3 Hz  |
| 42–53   | 12      | 24          | 16       | A4–G#5     | 440.0–830.6 Hz  |
| 54–65   | 12      | 28          | 8        | A5–G#6     | 880.0–1661.2 Hz |
| 66–77   | 12      | 28          | 8        | A6–G#7     | 1760.0–3322.4 Hz|

Rows 1–6 (pitches 6–77) each start at A and cover a full chromatic octave.
Row 0 (pitches 0–5) is a partial row covering only D#1 through G#1.

---

### Timing

The music sequencer runs in the **VBlank interrupt** at 60 Hz (NTSC).
Each VBlank, the 32-bit `timeclock` counter is incremented by the current
`tempo` value.  A note plays until `timeclock` reaches `event_start + notevals[duration_idx]`.
At the default tempo of 150 this gives **9,000 timeclock units per second**.

---

### Song Groups

Each group occupies four consecutive tracks (one per Amiga Paula voice).
Voice 0 carries the primary melody; voices 1–3 carry harmony/bass/rhythm.
`setmood()` in `fmain.c` chooses the active group based on game state.

| Group | Tracks | Context | Loop | ~Length |
|-------|--------|---------|------|---------|
| 0 | 0 – 3 | **Outdoor daytime** (`lightlevel > 120`) | Yes | ~57–84 s |
| 1 | 4 – 7 | **Battle** (`battleflag` set) | Yes | ~24–36 s |
| 2 | 8 – 11 | **Outdoor nighttime** (low light, outdoors) | Yes | ~65–96 s |
| 3 | 12 – 15 | **Intro sequence** (hardcoded `playscore` call) | No | ~54–75 s |
| 4 | 16 – 19 | **Palace zone** (specific hero map coordinates) | Yes | ~32–54 s |
| 5 | 20 – 23 | **Indoor / dungeon** (`region_num > 7`) | Yes | ~61–78 s |
| 6 | 24 – 27 | **Death / game over** (hero vitality = 0) | No | ~25–34 s |

Group 5 (indoor/dungeon) is also used for **caves** (region 9), but with a
different instrument assigned to slot 10: region 9 (caves) sets
`new_wave[10] = 0x0307`, all other indoor regions use `new_wave[10] = 0x0100`.
The track data is identical — only the timbre of one voice changes.

---

### Parsed Track Statistics

Decoded from the actual `game/songs` file (all 28 tracks loaded, 5,872 bytes
of score data):

```
 #  Group/Context       V   bytes  notes  rests  instr  tempo  loop   ~sec
 0  outdoor-daytime     0     394    193      1      1      1     Y    57.0
 1  outdoor-daytime     1     352    172      2      1      0     Y    83.6
 2  outdoor-daytime     2     130     50     13      1      0     Y    83.6
 3  outdoor-daytime     3     224    108      2      1      0     Y    83.6
 4  battle              0     270    111     21      1      1     Y    24.4
 5  battle              1     388    192      0      1      0     Y    35.8
 6  battle              2      52     24      0      1      0     Y    35.8
 7  battle              3     196     96      0      1      0     Y    35.8
 8  outdoor-night       0     400    192      5      1      1     Y    65.2
 9  outdoor-night       1     340    167      1      1      0     Y    95.6
10  outdoor-night       2     160     78      0      1      0     Y    95.6
11  outdoor-night       3     132     64      0      1      0     Y    95.6
12  intro               0     294    143      1      1      1     N    53.8
13  intro               1     178     87      0      1      0     N    74.7
14  intro               2     180     86      2      1      0     N    74.7
15  intro               3     102     49      0      1      0     N    74.7
16  palace              0     292    135      8      1      1     Y    31.6
17  palace              1     146     61     10      1      0     Y    53.8
18  palace              2      88     41      1      1      0     Y    53.8
19  palace              3     148     72      0      1      0     Y    53.8
20  indoor/dungeon      0     338    166      0      1      1     Y    60.7
21  indoor/dungeon      1     236     94     22      1      0     Y    77.7
22  indoor/dungeon      2     250    111     12      1      0     Y    77.7
23  indoor/dungeon      3     372    164     20      1      0     Y    77.7
24  death/game-over     0      48     19      2      1      1     N    24.8
25  death/game-over     1      54     18      7      1      0     N    34.3
26  death/game-over     2      54     18      7      1      0     N    34.3
27  death/game-over     3      54     18      7      1      0     N    34.3
```

Columns: V = Paula voice (0 = primary melody), `instr` = number of
`SetInstrument` events, `tempo` = number of `SetTempo` events,
`~sec` = approximate loop/play time at NTSC 60 Hz with the tempo set by the
first `SetTempo` event in that track.

---



## `game/v6` — Music Voice/Waveform Data (4,628 bytes)

The file is the **music synthesizer data** for the Amiga's four-voice Paula sound chip. It is loaded at startup by `fmain.c` via:

```c
Read(file, wavmem, S_WAVBUF);         // 1,024 bytes at offset 0
Seek(file, S_WAVBUF, OFFSET_CURRENT); // skip 1,024 bytes
Read(file, volmem, S_VOLBUF);         // 2,560 bytes at offset 2,048
```

These two buffers are passed directly to `init_music()` in `gdriver.asm`, which sets up the VBlank interrupt that drives the four-voice music engine.

### Layout

| Offset | Size | Name | Structure | Description |
|--------|------|------|-----------|-------------|
| 0x000 | 1,024 B | `wavmem` (wave buffer) | 8 waveforms × 128 bytes each | Signed 8-bit PCM sample data. Each 128-byte waveform is a periodic shape (sine-like, triangle, etc.) played by the Amiga's Paula DMA channels. The voice engine looks up `wave_num` per-voice and feeds a sub-range of the 128-byte buffer into Paula's `$a0`/`$a4` (pointer/length) registers. The sub-range depends on the current octave: `wave_offset` from PTABLE selects the start (`wave_offset × 4` bytes in) and the loop length (`(32 − wave_offset) × 2` bytes). |
| 0x400 | 1,024 B | *(skipped)* | — | Deliberately skipped by the `Seek(OFFSET_CURRENT)` call. Likely extra waveform data that isn't used by this version of the engine, or reserved space. The hex dump shows it is all zeros. |
| 0x800 | 2,560 B | `volmem` (volume/envelope buffer) | 10 envelopes × 256 bytes each | Amplitude envelope tables. Each byte is a volume level (0–64 in Amiga terms), except that any byte with the MSB set (≥ 0x80) is a **hold sentinel**: the envelope pointer stops advancing and the current volume is frozen until the next note event. Voices index into these via `vol_num` and step through the table byte-by-byte each VBlank tick, creating ADSR-like attack/decay/sustain/release shapes. When a voice advances past the last byte in its envelope, volume is zeroed (silence). |
| 0x1200 | 20 B | *(trailing zeros)* | — | Padding at end of file, unused. |

### Role in the engine

The `gdriver.asm` music engine runs as an Amiga interrupt server on **VBlank** (60 Hz NTSC). Each tick, for each of the 4 voices it:
1. Reads the next note from the score/track
2. Looks up the waveform by index into `wavmem`
3. Loads the PCM pointer and length into Paula's DMA registers
4. Steps through the envelope in `volmem`, writing each volume byte to Paula's `$a8` register

The "v6" name likely refers to **Version 6** of the music voice data file — the `new_wave[]` array in `fmain.c` defines a 12-element default instrument table that maps tracks to waveform/envelope pairs, and there are in-game branches that swap entries depending on whether the player is indoors vs. outdoors (`new_wave[10] = 0x0307` or `0x0100`).

---

## `game/samples` — Sound Effect Data

Loaded from **ADF block 920**, reading **11 blocks** (5,632 bytes, `SAMPLE_SZ`) into `sample_mem` via `read_sample()` in `fmain.c`.

Six IFF-style length-prefixed sound effects packed sequentially:

```
for each of 6 samples:
  [4 bytes big-endian] length N
  [N bytes]            signed 8-bit PCM mono sample data
```

`effect(num, speed)` calls `playsample(sample[num], sample_size[num] / 2, speed)`.
- `sample[num]` is a pointer into `sample_mem` past the length prefix
- `sample_size[num] / 2` is the length in 16-bit words (Paula DMA uses word count)
- `speed` is an Amiga Paula **period register** value (higher = slower, lower pitch); the `rand` jitter creates pitch variation per hit

| Index | Trigger event | Speed base | Jitter |
|-------|--------------|------------|--------|
| 0 | Hero hit by melee | 800 | +bitrand(511) |
| 1 | Weapon near-miss | 150 | +rand256() |
| 2 | Arrow/bolt hits player | 500 | +rand64() |
| 3 | Monster hit by melee | 400 | +rand256() |
| 4 | Arrow hits a target | 400 | +rand256() |
| 5 | Magic/fireball hit | 3200 | +bitrand(511) |

---

## `game/image` ADF — Sprite Shape Data

All animated character sprites are stored in **ADF `game/image`** (the same 880 KB floppy image used for map data). Sprite data is loaded by `read_shapes()` / `load_track_range()` in `fmain2.c`.

### `cfiles[]` — Sprite File Registry

```c
struct cfile_info {
    UBYTE width;     // sprite width in 16-pixel interleaved words
    UBYTE height;    // sprite height in pixels
    UBYTE count;     // number of animation frames
    UBYTE numblocks; // ADF 512-byte blocks to read
    UBYTE seq_num;   // seq_list[] slot (PHIL=0, OBJECTS=1, ENEMY=2, RAFT=3, SETFIG=4, CARRIER=5, DRAGON=6)
    USHORT file_id;  // starting ADF block number
};
```

**Frame byte size** = `width × height × 2` (one row per word, one bitplane).
**Total data per file** = `frame_bytes × count × 5` (5 bitplanes) + `frame_bytes × count` (mask plane) = `frame_bytes × count × 6`.
`nextshape` advances by `frame_bytes × count × 5`; `seq_list[slot].maskloc` is stored at the 6th chunk.

| cfile# | ADF block | Blocks | W×H | Frames | Slot | Contents |
|--------|-----------|--------|-----|--------|------|----------|
| 0 | 1376 | 42 | 1×32 | 67 | PHIL | Julian (all directions + fight) |
| 1 | 1418 | 42 | 1×32 | 67 | PHIL | Phillip |
| 2 | 1460 | 42 | 1×32 | 67 | PHIL | Kevin |
| 3 | 1312 | 36 | 1×16 | 116 | OBJECTS | World items / loot objects |
| 4 | 1348 | 3  | 2×32 | 2   | RAFT | Raft (two frames) |
| 5 | 1351 | 20 | 2×32 | 16  | CARRIER | Turtle |
| 6 | 960  | 40 | 1×32 | 64  | ENEMY | Ogre / Orc |
| 7 | 1080 | 40 | 1×32 | 64  | ENEMY | Ghost / Wraith / Skeleton / Salamander |
| 8 | 1000 | 40 | 1×32 | 64  | ENEMY | DKnight / Spider |
| 9 | 1040 | 40 | 1×32 | 64  | ENEMY | Necromancer / Loraii / Farmer |
| 10 | 1160 | 12 | 3×40 | 5  | DRAGON | Dragon |
| 11 | 1120 | 40 | 4×64 | 8  | CARRIER | Bird |
| 12 | 1376 | 40 | 1×32 | 64 | ENEMY | Snake / Salamander (shares ADF block with Julian) |
| 13 | 936  | 5  | 1×32 | 8  | SETFIG | Wizard / Priest |
| 14 | 931  | 5  | 1×32 | 8  | SETFIG | Guards / Princess / King / Noble / Sorceress |
| 15 | 941  | 5  | 1×32 | 8  | SETFIG | Bartender |
| 16 | 946  | 5  | 1×32 | 8  | SETFIG | Witch / Spectre / Ghost |
| 17 | 951  | 5  | 1×32 | 8  | SETFIG | Ranger / Beggar |

### Amiga interleaved bitplane layout

Each animation frame is stored in **Amiga interleaved bitplane format**: the bitplanes are interleaved row-by-row within each frame. For a `1×32` (one word × 32 rows, 5 planes) frame:
```
row 0, plane 0: 2 bytes
row 0, plane 1: 2 bytes
row 0, plane 2: 2 bytes
row 0, plane 3: 2 bytes
row 0, plane 4: 2 bytes
row 1, plane 0: 2 bytes
...
row 31, plane 4: 2 bytes   (total: 32 × 5 × 2 = 320 bytes per frame)
```
The mask plane (`maskloc`) follows immediately after all 5 bitplanes of all frames, in the same interleaved row layout but 1 plane only.

### `statelist[]` — Animation Frame Index

`statelist[87]` maps animation state+frame indices to `{figure_frame, weapon_frame, wpn_x, wpn_y}`:
- Frames 0–7: south walk cycle
- Frames 8–15: west walk cycle
- Frames 16–23: north walk cycle
- Frames 24–31: east walk cycle
- Frames 32–43: south fight (9 transition states + 2 death/special)
- Frames 44–55: west fight
- Frames 56–67: north fight
- Frames 68–79: east fight
- Frames 80–82: death sequence
- Frames 83–84: sinking sequence / oscillation
- Frame 86: asleep

`trans_list[9]` is the combat animation transition table: each state maps to the next state for each of the 4 compass directions.

---

## Terrain Collision

Sources: `original/fmain.c`, `original/terrain.c`, `original/fsubs.asm`.

### Overview

Terrain collision is **tile-type-based**, not bitplane-based. There is no dedicated collision bitplane in the ADF. Instead, every image tile (world graphic tile) has an associated 4-byte terrain descriptor stored in `terra_mem`, which is consulted at runtime to determine whether a position is passable and what special behavior applies.

---

### Memory Buffers

| Buffer | Size | Purpose |
|--------|------|---------|
| `sector_mem` | `128×256 + 4096` bytes | Maps (sector, tile-position) → image tile index. 256 sectors, 128 bytes each. |
| `map_mem` | `8 ADF blocks` (4096 bytes) | Maps world region coordinates → sector numbers. |
| `terra_mem` | 1024 bytes (chip RAM) | Terrain descriptor table. 256 entries × 4 bytes (two 512-byte halves, one per terrain file loaded). |

`terra_mem` is loaded from ADF starting at block `TERRA_BLOCK` (149). Each region specifies two terrain file indices (`terra1` and `terra2`); they are loaded into the two 512-byte halves of `terra_mem`:

```c
load_track_range(TERRA_BLOCK + nd->terra1, 1, terra_mem,       1);
load_track_range(TERRA_BLOCK + nd->terra2, 1, terra_mem + 512, 2);
```

---

### Terrain Descriptor Layout (`terra_mem` entry, 4 bytes per tile)

`terrain.c` extracts per-tile descriptor data from each landscape source file and writes 4 bytes per image tile:

| Byte offset | Field | Description |
|-------------|-------|-------------|
| +0 | `maptag` | Bit mask for rendering: controls which sub-cells within a 32×64 tile get the feature blitted (`maskit()` call). |
| +1 | `terrain` | **2 nibbles**: upper nibble = terrain type (returned by `px_to_im`); lower nibble = TODO: verify exact meaning. |
| +2 | `tiles` | 4-bit feature presence mask. Controls which quadrant sub-cells within the tile carry the terrain feature (checked against the position bit `d4` derived from pixel x/y). If the relevant bit is zero, `px_to_im` returns 0 (open terrain) even if the image tile would otherwise have a type. |
| +3 | `big_colors` | Palette index for minimap rendering. |

Access pattern in C (used for masking logic):

```c
cm = minimap[cell] * 4;          // 4 bytes per entry
k  = terra_mem[cm + 1] & 15;     // lower nibble (masking case selector)
maskit(xm, ym, blitwide, terra_mem[cm]); // +0 = maptag bit mask
```

Access pattern in ASM (`px_to_im`):

```asm
and.b   2(a1,d1.w),d4   ; terra_mem[entry+2].tiles & position_bit
beq.s   px99            ; zero = no feature at this sub-cell → return 0
move.b  1(a1,d1.w),d0   ; terra_mem[entry+1].terrain
lsr.b   #4,d0           ; upper nibble = terrain type
```

---

### Coordinate-to-Terrain Lookup: `px_to_im(x, y)`

Implemented in `fsubs.asm`. Converts absolute pixel coordinates to a terrain type (0–15):

```
1. Compute tile position bit (d4 = 0x80, then shifted):
     if x & 8:  d4 >>= 4   (right half of tile)
     if y & 8:  d4 >>= 1   (lower half within row)
     if y & 16: d4 >>= 2   (second tile row)

2. imx = x >> 4            (image x: tile column, 16 px/col)
   imy = y >> 5            (image y: tile row,    32 px/row)

3. secx = (imx >> 4) - xreg, clamped 0–63  (sector column)
   secy = (imy >>  3) - yreg, clamped 0–31  (sector row)

4. sec_num = map_mem[secy * 128 + secx + xreg]

5. offset  = sec_num * 128 + (imy & 7) * 16 + (imx & 15)
   image_n = sector_mem[offset]             (image tile index 0–255)

6. entry   = image_n * 4                   (into terra_mem)
   if (terra_mem[entry+2] & d4) == 0:
       return 0                            (no feature at sub-cell)
   return terra_mem[entry+1] >> 4          (upper nibble = terrain type)
```

---

### Terrain Type Table

Derived from the comment block at `fmain.c:727` and all `px_to_im`/`proxcheck` usage sites:

| Type | Symbolic name | Behavior |
|------|--------------|---------|
| 0 | Open / land | Fully passable; no special effect. |
| 1 | **Impassable** | Hard block (walls, solid mountains, buildings). `proxcheck` always blocks. |
| 2 | Sink (shallow) | Character starts sinking; `environ` → 2. Water — wading possible. |
| 3 | Sink (deep) | Faster sinking; `environ` → 5. |
| 4 | Water (shallow) | Sinking threshold 10; triggers `SINK` state at depth 15; transition to `SINK` at 30. |
| 5 | Water (deep / navigable by raft) | Sinking threshold 30; raft navigates here. |
| 6 | Special A | Sets `environ` = −1. TODO: verify (ice/slippery?). |
| 7 | Special B (lava?) | Sets `environ` = −2. Volcanic region tile; vultures (`xtype==52`) can spawn here. |
| 8 | Special C | Sets `environ` = −3. Blocks left-foot `proxcheck` probe (≥8 threshold). |
| 9 | Pit / fall trap | Triggers `FALL` state for the hero; reduces `luck` by 2. |
| 10–11 | Hard block (high) | Blocks `proxcheck` right-foot probe (≥10 threshold). TODO: verify specific sub-types. |
| 12 | Water passage | Normally blocking (≥10 for right, ≥8 for left), but `stuff[30]` (water-walk item?) allows passage. |
| 13–14 | Hard block | Block both probes. TODO: verify specific sub-types. |
| 15 | **Door** | Triggers `doorfind()` on the hero's attempted move; stops projectiles. |

The comment in `fmain.c` also mentions planned-but-unclear types: "slippery, fiery, changing, climbable, pit trap, danger, noisy, magnetic, stinks, slides, slopes, whirlpool." Only types 0–9 and 15 have verified game behavior in the shipped code.

---

### Collision Check: `proxcheck(x, y, entity_index)` → `_prox` in `fsubs.asm`

`proxcheck` samples **two points** straddling the character's feet (±4 pixels horizontally, +2 pixels vertically from the passed position). It returns 0 if passable, or the terrain type if blocked.

```asm
_prox:
    ; Right foot: (x+4, y+2)
    call px_to_im(x+4, y+2)
    if result == 1:  goto blocked      ; impassable
    if result >= 10: goto blocked      ; hard-block types

    ; Left foot: (x-4, y+2)
    call px_to_im(x-4, y+2)
    if result == 1:  goto blocked      ; impassable
    if result >= 8:  goto blocked      ; hard-block types (lower threshold)

    clr d0                             ; both clear → return 0 (passable)
blocked:
    rts                                ; d0 = terrain type (non-zero = blocked)
```

The asymmetric thresholds (≥10 right, ≥8 left) mean types 8–9 only block the left-foot probe, which may be an artifact of the original code's heuristic collision. This is faithfully reproduced from the source.

**Caller interpretation** of the return value:

- `== 0` → fully passable → allow move
- `== 15` → door tile → call `doorfind()`
- `== 12` → water-walk check (passes if `stuff[30]` is set)
- anything non-zero → blocked; try deviated direction (`checkdev1/2`)

---

### Special Terrain Behaviors

| Condition | Effect |
|-----------|--------|
| Type 2–5 while walking | Increments `environ` (submersion depth); at depth 15 triggers `SINK` animation state. |
| Type 4/5 at depth 30 | Full submersion → `SINK`; at `hero_sector==181` (river crossing) triggers `xfer` to region 9. |
| Type 0 (open) | Resets `environ` toward 0 (character surfaces). |
| `race == 2` (wraith) or `race == 4` (snake) | `px_to_im` result forced to 0 — immune to water sinking. |
| `riding == 5` (on raft) | `raftprox` set; drowning suppressed (`k = 0`). Raft can only navigate type 5 tiles. |
| Type 9 + hero on `xtype==52` (vulture) | Triggers `FALL` state; luck −2. |
| Type 1 or 15 | Stops projectiles (arrows/fireballs) dead. |
| `passmode` set (weapon pass-through) | Sprites rendered without masking; terrain masking skipped. |

---

### Terrain Source Files (`terrain.c`)

The build tool `terrain.c` reads 17 named landscape image files and extracts 64 tile descriptors from each, writing them sequentially to the `terra` output file (which is then stored in the ADF at block 149+):

```
wild, build, rock, mountain1, tower, castle, field, swamp, palace,
mountain2, doom, mountain3, under, cave, furnish, inside, astral
```

Each landscape file is structured as `5 × 64 × 64` bytes of image bitplane data (`IPLAN_SZ`), followed by four 64-byte descriptor arrays: `maptag[64]`, `terrain[64]`, `tiles[64]`, `big_colors[64]`. `terrain.c` seeks past the image planes and reads only the descriptor arrays.

