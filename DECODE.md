# Game File Decoding Notes

This document is the canonical reference for reverse-engineering notes,
binary asset/file format details (`songs`, `v6`, and related data files),
and original game mechanics derived from the manual and source code.

For build/run setup, use `README.md`. For roadmap/task tracking, use
`PLAN.md` and `plan_status.toml`.

---

## Input & Command Reference (from original manual)

### Movement

- **Numpad 1–9**: 8-way movement using physical key position (ignore numerals).
  Numpad layout maps to directions:
  ```
  7=NW  8=N   9=NE
  4=W   5=--  6=E
  1=SW  2=S   3=SE
  ```
- **Joystick**: press in desired direction.
- **Mouse**: hold left button over compass point in HI bar.
- Release key/button to stop.

### Combat

- **Numpad 0**: attack (original fire button).
- **Joystick fire button** / **Mouse right button**: attack.
- Attacks are directional — must face the opponent.
- Direction of attack controlled same as movement.

### Command Menu System

The HI bar has 5 category tabs, each revealing a sub-menu.
Activated by mouse click on the labeled bar, or by keyboard shortcut.

#### Items Menu
| Label | Key | Action |
|-------|-----|--------|
| List  | `L` | Show all carried items |
| Take  | `T` | Pick up item from ground / dead body |
| Look  | `?` | Look for hidden items |
| Give  | `G` | Give item to someone |
| Use   | `U` | Opens weapon sub-menu (see below) |

**Use sub-menu** (weapon selection):
| Weapon | Key | Notes |
|--------|-----|-------|
| Dirk   | `1` | Draw dagger |
| Mace   | `2` | Draw mace |
| Sword  | `3` | Draw sword |
| Bow    | `4` | Draw bow and arrow |
| Wand   | `5` | Draw magic wand |
| Key    | `K` | Opens key color sub-menu |

**Key sub-menu** (via `K`):
| Key Color | Shortcut |
|-----------|----------|
| Gold      | `K1`     |
| Green     | `K2`     |
| Blue      | `K3`     |
| Red       | `K4`     |
| Grey      | `K5`     |
| White     | `K6`     |

#### Magic Menu
One-use magic items. Each use consumes one of that item type.

| Label | Key  | Item |
|-------|------|------|
| Stone | `F1` | Blue stone |
| Jewel | `F2` | Green jewel |
| Vial  | `F3` | Glass vial (restorative) |
| Orb   | `F4` | Crystal orb |
| Totem | `F5` | Bird totem |
| Ring  | `F6` | Gold ring |
| Skull | `F7` | Jade skull |

#### Talk Menu
| Label | Key | Action |
|-------|-----|--------|
| Yell  | `Y` | Yell |
| Say   | `S` | Say |
| Ask   | `A` | Ask |

#### Buy Menu
Only works near a merchant character.

| Item   | Key |
|--------|-----|
| Food   | `O` |
| Arrow  | `R` |
| Vial   | `8` |
| Mace   | `C` |
| Sword  | `W` |
| Bow    | `B` |
| Totem  | `E` |

#### Game Menu
| Label  | Key        | Action |
|--------|------------|--------|
| Pause  | `Spacebar` | Pause/unpause the game |
| Music  | `M`        | Toggle music |
| Sound  | `F`        | Toggle sound effects |
| Quit   | `Q`        | Quit — sub-menu: exit or save |
| Load   | `L`        | Load saved game — 8 slots A–H |

### Player Stats (narration scroll)

Five stats displayed on the HI bar scroll area:

| Stat     | Abbr  | Description |
|----------|-------|-------------|
| Bravery  | `Brv` | Battle prowess |
| Luck     | `Lck` | Fairy rescue chance on death |
| Kindness | `Knd` | NPC communication threshold |
| Vitality | `Vit` | Health (0 = death) |
| Wealth   | `Wlt` | Coins carried |

When a character dies with sufficient Luck, a fairy heals him and
teleports him to the last safe location.

### Map Size

The world is 144 screens tall × 100 screens wide.

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
**ADF data per file** = `frame_bytes × count × 5` (5 bitplanes only) = `numblocks × 512` bytes.
`nextshape` advances by `frame_bytes × count × 5`; `seq_list[slot].maskloc` points to the next `frame_bytes × count` bytes of the pre-allocated `shape_mem` buffer.

**The mask is not stored on disk.** It is computed at runtime by `make_mask()` (`fsubs.asm:1614`):
for each word position across all frames, it ORs all plane bits then inverts:
`mask_word = NOT(plane0 AND plane1 AND plane2 AND plane3 AND plane4)`
A pixel is **transparent** when all 5 plane bits are set (color index 31). All other color indices are opaque.
Comment in `fsubs.asm:1617`: "assumes color 31 = transparent".

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

### Bitplane layout

Each animation frame is stored in **plane-major format**: all rows of one plane are stored together, then all rows of the next plane. For a `1×32` (one word × 32 rows, 5 planes) frame:
```
plane 0, row  0: 2 bytes
plane 0, row  1: 2 bytes
...
plane 0, row 31: 2 bytes  (64 bytes total for plane 0)
plane 1, row  0: 2 bytes
...
plane 4, row 31: 2 bytes  (total: 5 × 64 = 320 bytes per frame)
```
Offset formula: plane P, row R of frame F = `data[F*320 + P*64 + R*2]`.

The mask is not stored after the frames — see the note above about `make_mask()`.

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

---

## Compass Rose — Direction Indicator Bitmaps

The HI-bar compass is rendered by `drawcompass(dir)` (`fmain2.c:493–508`).
Two single-plane bitmaps control the compass appearance; they are composited
into bitplane 2 of the text viewport at position **(567, 15)**, sized
**48 × 24** pixels.

### Source data

`_hinor` and `_hivar` are defined in `fsubs.asm` (lines 250–277) as raw
`dc.l` longwords.  At startup `into_chip()` copies them into Chip RAM so the
blitter can access them.

The backing bitmap is initialised as:

```c
InitBitMap(bm_source, 3, 64, 24);   /* 3 planes, 64 px wide, 24 rows */
```

Only **plane 2** is used — planes 0 and 1 of `bm_source` are unused.
Stride is `64 / 8 = 8` bytes per row; each plane occupies `8 × 24 = 192`
bytes.  The compass content occupies the leftmost **48 pixels** (6 bytes) of
each row; the trailing 2 bytes per row are padding.

| Symbol   | Role                                  | Size (bytes) |
|----------|---------------------------------------|--------------|
| `_hinor` | Normal compass (no direction highlighted) | 200 (192 + 8 pad) |
| `_hivar` | All directions highlighted                | 200 (192 + 8 pad) |

### `drawcompass(dir)` algorithm

```
1.  bm_source->Planes[2] = nhinor
2.  BltBitMap(bm_source, 0, 0, bm_text, 567, 15, 48, 24, 0xC0, 4, NULL)
        — blits the entire 48×24 normal compass to the text viewport
3.  if dir < 9:
        bm_source->Planes[2] = nhivar
        BltBitMap(bm_source, xr, yr, bm_text, 567+xr, 15+yr, xs, ys, 0xC0, 4, NULL)
            — overlays only the active direction sub-region with the highlighted variant
```

**BltBitMap parameters:**

| Param    | Value  | Meaning                              |
|----------|--------|--------------------------------------|
| minterm  | `0xC0` | D := A (straight copy, source → dest) |
| mask     | `4`    | Binary `0100` → only plane 2 is copied |

### `comptable[10]` — Direction sub-regions

Each entry defines a rectangle `{xrect, yrect, xsize, ysize}` within the
48 × 24 compass area.  Directions 8 and 9 are "standing still" (1 × 1 no-op).

| Index | Direction | xrect | yrect | xsize | ysize |
|-------|-----------|-------|-------|-------|-------|
| 0     | NW        |  0    |  0    | 16    |  8    |
| 1     | N         | 16    |  0    | 16    |  9    |
| 2     | NE        | 32    |  0    | 16    |  8    |
| 3     | E         | 30    |  8    | 18    |  8    |
| 4     | SE        | 32    | 16    | 16    |  8    |
| 5     | S         | 16    | 13    | 16    | 11    |
| 6     | SW        |  0    | 16    | 16    |  8    |
| 7     | W         |  0    |  8    | 18    |  8    |
| 8     | still     |  0    |  0    |  1    |  1    |
| 9     | still     |  0    |  0    |  1    |  1    |

### How plane 2 produces colour

The text viewport (opened by `setup_screen` in `fmain.c`) uses the
`textcolors[]` palette.  Plane 2 is bit 2 of the 4-bit colour index.
The compass area in `bm_text` gets planes 0, 1, 3 from the hiscreen image;
plane 2 is the only plane modified by `drawcompass()`.

The resulting colour at each pixel is `textcolors[index]` where
`index = (p3 << 3) | (p2 << 2) | (p1 << 1) | p0`.
Setting plane 2 toggles between colour pairs, e.g.:

| Planes 3,1,0 | Plane 2 = 0       | Plane 2 = 1          |
|---------------|-------------------|----------------------|
| `0,0,0`       | `[0]` 0x000 black | `[4]` 0x00F blue     |
| `0,0,1`       | `[1]` 0xFFF white | `[5]` 0xC0F magenta  |
| `0,1,0`       | `[2]` 0xC00 red   | `[6]` 0x090 green    |
| `0,1,1`       | `[3]` 0xF60 orange| `[7]` 0xFF0 yellow   |

### Rust port notes

The extracted compass data lives in `faery.toml` under `[compass]`:

- `[compass.comptable]` — direction sub-regions
- `[compass.hinor]` — normal compass, single-plane BitMap (48 × 24, stride 6)
- `[compass.hivar]` — highlighted compass, single-plane BitMap

At render-resource build time the port extracts the compass region from
the hiscreen `IffImage`, replaces plane 2 with `hinor` / `hivar`, and
converts both composites to RGBA textures using the `textcolors` palette.
During gameplay, the normal compass texture is blitted first; if the player
is moving, the active direction sub-region from the highlighted texture is
overlaid on top.

---

## Input Decoding — `decode_mouse` / `decodekey` (`fsubs.asm:1490–1576`)

All input (mouse, joystick, keyboard) is funnelled through `decode_mouse()`
which produces a single direction value 0–9 stored in `oldir`.  This value
indexes into `comptable[]` (compass highlight) and into the `xdir[]`/`ydir[]`
movement tables.

### Direction index convention

```
Index    Dir    xdir   ydir   Compass
  0      NW      -2     -2    upper-left
  1      N        0     -3    top-center
  2      NE       2     -2    upper-right
  3      E        3      0    right
  4      SE       2      2    lower-right
  5      S        0      3    bottom-center
  6      SW      -2      2    lower-left
  7      W       -3      0    left
  8      still    0      0    (1×1 no-op)
  9      still    0      0    (1×1 no-op)
```

Negative Y = up on screen = north.  The `newx(x,dir,speed)` / `newy(y,dir,speed)`
functions in `fsubs.asm:1274–1319` apply `xdir[dir]*speed/2` and `ydir[dir]*speed/2`.

### `keytrans` table (`fsubs.asm:221–226`)

Maps Amiga raw scancodes (0x00–0x5F) to internal key codes.
Movement-relevant entries:

| Amiga scancode | Physical key | keytrans code | dir (code−20) |
|----------------|--------------|---------------|---------------|
| 0x3D           | Numpad 7     | 20            | 0 = NW        |
| 0x3E           | Numpad 8     | 21            | 1 = N         |
| 0x3F           | Numpad 9     | 22            | 2 = NE        |
| 0x2D           | Numpad 4     | 27            | 7 = W         |
| 0x2E           | Numpad 5     | 29            | 9 = still     |
| 0x2F           | Numpad 6     | 23            | 3 = E         |
| 0x1D           | Numpad 1     | 26            | 6 = SW        |
| 0x1E           | Numpad 2     | 25            | 5 = S         |
| 0x1F           | Numpad 3     | 24            | 4 = SE        |
| 0x0F           | Numpad 0     | `'0'`         | fight (not dir)|

Cursor keys (0x4C–0x4F) map to codes 1–4 which are **not** direction codes
(they fall outside the 20–29 range); in the original they are cheat-only
teleport keys gated by the `cheat1` flag (`fmain.c:1487–1498`).

### `decodekey` path (`fsubs.asm:1565–1572`)

```
if keydir >= 20 && keydir < 30:
    dir = keydir - 20
else:
    dir = 9   (no direction)
```

Key-down sets `keydir = key`; key-up clears it when `(key & 0x7F) == keydir`.

### Joystick decoding (`fsubs.asm:1530–1563`)

Reads `JOY1DAT` ($DFF00C) to extract two axes:

```
xjoy = right_indicator - left_indicator    ∈ {-1, 0, 1}
yjoy = back_indicator  - forward_indicator ∈ {-1, 0, 1}
```

Where forward = joystick pushed away from player (up on screen, north).

A formula produces a 0–8 index: `idx = 4 + yjoy*3 + xjoy`, then `com2[idx]`
gives the direction value.

**`com2` table** (`fsubs.asm:1487`): `0, 1, 2, 7, 9, 3, 6, 5, 4`

```
Joystick grid:        com2 remapping:
 (L,Fwd)=0  (M,Fwd)=1  (R,Fwd)=2     dir 0=NW  dir 1=N   dir 2=NE
 (L,Mid)=3  (Center)=4  (R,Mid)=5     dir 7=W   dir 9=—   dir 3=E
 (L,Bck)=6  (M,Bck)=7  (R,Bck)=8     dir 6=SW  dir 5=S   dir 4=SE
```

### Mouse compass click (`fsubs.asm:1496–1528`)

When the left mouse button is held and the pointer is in the compass area
(x > 265), the pointer coordinates are divided into a 3×3 grid to produce
a direction 0–9:

```
X: <292 = left column     292–300 = middle column     >300 = right column
Y: <166 = top row         166–174 = middle row        >174 = bottom row
```

### Rust port mapping

Our `Direction` enum uses a different order than the original:

| Our facing | Direction | Original dir | comptable index |
|------------|-----------|--------------|-----------------|
| 0          | N         | 1            | 1               |
| 1          | NE        | 2            | 2               |
| 2          | E         | 3            | 3               |
| 3          | SE        | 4            | 4               |
| 4          | S         | 5            | 5               |
| 5          | SW        | 6            | 6               |
| 6          | W         | 7            | 7               |
| 7          | NW        | 0            | 0               |

Formula: `comptable_index = (facing + 1) & 7`.


---

## Menu System (`fmain.c:538–589`, `3758–3820`, `4409–4441`; `fmain2.c:613–675`; `fsubs.asm:120–165`)

### 10 Menu Modes

```
ITEMS = 0   MAGIC = 1   TALK = 2   BUY  = 3   GAME  = 4
SAVEX = 5   KEYS  = 6   GIVE = 7   USE  = 8   FILE  = 9
```

| Mode  | Purpose                              | Label str | Color idx |
|-------|--------------------------------------|-----------|-----------|
| ITEMS | Inventory / object interaction       | `label1`  | 4         |
| MAGIC | Cast spells (F-key driven)           | `label2`  | 5         |
| TALK  | NPC communication                    | `label3`  | 6         |
| BUY   | Purchase items from shops            | `label4`  | 7         |
| GAME  | Pause / Music / Sound / nav          | `label5`  | 8         |
| SAVEX | Save or quit                         | `label6`  | 9         |
| KEYS  | Try a key type on a door             | `label7`  | 10        |
| GIVE  | Give items to NPCs                   | `label8`  | 11        |
| USE   | Equip weapon / use special items     | `label9`  | 12        |
| FILE  | Load / save file slots               | `labelA`  | 13        |

### `enabled[]` Bit Flags

Each menu slot's `enabled[i]` byte encodes both visibility and behaviour:

```
bit 0      : selected / active  (1 = on for toggles)
bit 1      : displayed / visible (must be set to appear in menu)
bits 2–7   : action type
  0x00 (0) : tab header — click switches cmode; always shown
  0x04 (4) : toggle — click flips bit 0
  0x08 (8) : immediate action — fires once on click
  0x0C (12): radio button — sets bit 0 exclusively
```

Common combined values:

| Value | Meaning                                           |
|-------|---------------------------------------------------|
| 0     | Not displayed, not active (empty slot)            |
| 2     | Displayed, not selected, tab type (inactive tab)  |
| 3     | Displayed, selected, tab type (active tab)        |
| 6     | Displayed, not selected, toggle (Pause starts OFF)|
| 7     | Displayed, selected, toggle (Music/Sound start ON)|
| 8     | Immediate, not displayed (hidden until set_options)|
| 10    | Displayed, immediate (standard menu item)         |

### Label Strings (`fmain.c:538–549`)

Each slot is exactly 5 characters (no null terminator; the renderer reads 5 bytes directly):

```
Slots 0–4:  tab labels (shared across all modes)
  "Items" "Magic" "Talk " "Buy  " "Game "   (ITEMS…GAME tabs)
  — extended tab row for SAVEX/KEYS/GIVE/USE/FILE uses per-mode label strings

label1 (ITEMS) : "ItemsMagicTalk Buy  Game Save Keys Give Use  File"
label2–labelB  : same 5-char-per-tab structure for each mode
```

Each `menus[k].label` points into the concatenated string; slots 0–4 are the 5 mode-tab names repeated in the active mode's color.

### Settings Toggles (Critical for Game Behavior)

```
menus[GAME].enabled[5] & 1  → Pause   (1 = paused; freezes game loop)
menus[GAME].enabled[6] & 1  → Music   (1 = on; setmood() plays/stops music)
menus[GAME].enabled[7] & 1  → Sound   (1 = on; effect() plays samples)
```

- Pause starts at `6` (toggle, OFF); Music and Sound start at `7` (toggle, ON).
- `gomenu()` returns immediately without changing mode when Pause is active.

### `gomenu()` (`fmain.c:4409–4414`)

```c
void gomenu(short mode) {
    if (menus[GAME].enabled[5] & 1) return;  // refuse if paused
    cmode = mode;
    handler_data.lastmenu = 0;
    print_options();
}
```

### `print_options()` → `real_options[]` Mapping (`fmain.c:3758–3782`)

```
j = 0   // display slot counter
for i = 0 .. menus[cmode].num:
    if (enabled[i] & 2) == 0: skip   // not visible
    real_options[j] = i               // display slot j → menu index i
    propt(j, enabled[i] & 1)
    j++
// remaining slots:
real_options[j] = -1; draw blank button
```

`real_options[]` lets click/key dispatch translate a display-slot index back to the true `enabled[]` index.

### `propt()` Button Rendering (`fmain.c:3785–3819`)

**Background color** (`penb`):

```
cmode == USE   → 14  (grey,       textcolors[14] = 0x888)
cmode == FILE  → 13  (light grey, textcolors[13] = 0xCCC)
k < 5          →  4  (blue tab,   textcolors[4]  = 0x00F)
cmode == KEYS  → keycolors[k-5]
cmode == SAVEX → k   (slot index used directly as color index)
else           → menus[cmode].color
```

**Foreground color** (`pena`):

```
0 = black (textcolors[0]) — normal / off state
1 = white (textcolors[1]) — selected / on state (toggles)
```

**Screen position** (Amiga lo-res source coordinates):

```
x = 430  (even display slot)  or  482  (odd display slot)
y = (slot / 2) * 9 + 8
```

### `set_options()` Inventory-Driven Visibility (`fmain.c:4417–4441`)

`stuff_flag(x)` returns `8` (hidden/immediate) when `x == 0`, else `10` (displayed/immediate).

| Mode  | Slot(s) | Rule                                               |
|-------|---------|----------------------------------------------------|
| MAGIC | 5–11    | `stuff_flag(stuff[i+9])` — owns magic item?        |
| USE   | 0–6     | `stuff_flag(stuff[i])` — owns weapon `i`?          |
| USE   | 7 (Keys)| 10 if any key type owned, else 8                   |
| USE   | 8 (Sunstone)| `stuff_flag(stuff[7])`                         |
| KEYS  | 5–10    | `stuff_flag(stuff[i+16])` — owns key type `i`?    |
| GIVE  | 5 (Gold)| 10 if `wealth > 2`, else 8                         |
| GIVE  | 6 (Book)| always 8 (permanently hidden)                      |
| GIVE  | 7 (Writ)| `stuff_flag(stuff[28])`                            |
| GIVE  | 8 (Bone)| `stuff_flag(stuff[29])`                            |

`set_options()` is called after every `do_option()` action so the menu reflects the current inventory state.

### `do_option()` Dispatch Table (`fmain.c:3830–3393`)

| cmode | hit   | Action                                              |
|-------|-------|-----------------------------------------------------|
| ITEMS | 5     | Show inventory screen (`viewstatus = 4`)            |
| ITEMS | 6     | Take nearest object                                 |
| ITEMS | 7     | Look (print region / stats)                         |
| ITEMS | 8     | `gomenu(USE)`                                       |
| ITEMS | 9     | `gomenu(GIVE)`                                      |
| MAGIC | 5–11  | Cast spell (if owned)                               |
| TALK  | 5     | Yell                                                |
| TALK  | 6     | Say (speak to nearest NPC)                          |
| TALK  | 7     | Ask (query nearest NPC)                             |
| BUY   | 5–11  | Buy item (via `jtrans[]` price table)               |
| GAME  | 5     | Pause toggle (handled before `do_option`)           |
| GAME  | 6     | Music toggle → `setmood(TRUE)`                      |
| GAME  | 7     | Sound toggle (`effect()` checks `enabled[7] & 1`)  |
| GAME  | 8     | `gomenu(SAVEX)`                                     |
| GAME  | 9     | `gomenu(FILE)`                                      |
| USE   | 0–4   | Set weapon (`anim_list[0].weapon = hit + 1`)        |
| USE   | 6     | Summon turtle (`get_turtle()`)                      |
| USE   | 7     | `gomenu(KEYS)`                                      |
| USE   | 8     | Use Sunstone (if `witchflag`)                       |
| SAVEX | 5     | Save game → `gomenu(FILE)`                          |
| SAVEX | 6     | Quit (`quitflag = TRUE`)                            |
| FILE  | 5–12  | Load/save slot → `savegame(hit)` → `gomenu(GAME)`  |
| KEYS  | 5–10  | Try key type on door → `gomenu(ITEMS)`              |
| GIVE  | 5     | Give gold to nearest NPC (if `wealth > 2`)          |
| GIVE  | 7     | Give Writ of Passage                                |
| GIVE  | 8     | Give Bone                                           |
| All   | —     | Calls `set_options()` after every action            |

### `letter_list[38]` Keyboard Shortcuts (`fmain.c:579–589`)

```
Key    Mode   Slot  Action
'I'    ITEMS  5     List inventory
'T'    ITEMS  6     Take
'?'    ITEMS  7     Look
'U'    ITEMS  8     → Use menu
'G'    ITEMS  9     → Give menu
'Y'    TALK   5     Yell
'S'    TALK   6     Say
'A'    TALK   7     Ask
' '    GAME   5     Toggle Pause
'M'    GAME   6     Toggle Music
'F'    GAME   7     Toggle Sound
'Q'    GAME   8     → Save/Exit menu
'L'    GAME   9     → Load/File menu
'O'    BUY    5     Buy Food
'R'    BUY    6     Buy Arrows
'8'    BUY    7     Buy Vial
'C'    BUY    8     Buy Mace
'W'    BUY    9     Buy Sword
'B'    BUY    10    Buy Bow
'E'    BUY    11    Buy Totem
'V'    SAVEX  5     Save (only fires when cmode == SAVEX)
'X'    SAVEX  6     Exit / Quit
'1'    USE    0     Equip Dirk
'2'    USE    1     Equip Mace
'3'    USE    2     Equip Sword
'4'    USE    3     Equip Bow
'5'    USE    4     Equip Wand
'6'    USE    5     Equip Lasso
'7'    USE    6     Summon Turtle
'K'    USE    7     → Keys menu
F1–F7  MAGIC  5–11  Cast spells (separate F-key path, not letter_list)
```

**Notes:**
- SAVEX entries (`'V'`, `'X'`) only fire when `cmode == SAVEX` (`fmain.c:1510–1511`).
- MAGIC uses F-keys via a separate key-handling path, not `letter_list`.
- KEYS sub-mode: digits `'1'`–`'6'` map directly to `do_option(key - '1' + 5)`.

### `keycolors[6]` (`fmain.c:551`)

```
Index  textcolors idx  Color   Key Type
0      8               0xF90   Gold key
1      6               0x090   Green key
2      4               0x00F   Blue key
3      2               0xC00   Red key
4      14              0x888   Grey key
5      1               0xFFF   White key
```

Used by `propt()` as background color when `cmode == KEYS` and `k >= 5`.

### `prq()` Deferred Action Queue (`fmain2.c:613–675`)

The original engine uses a 32-entry circular buffer for deferred rendering requests:

```
prq(4)   → redraw vitality stat in HI bar
prq(5)   → call print_options() (redraw all menu buttons)
prq(7)   → redraw Brv/Lck/Knd/Wlth stats bar
prq(10)  → print "Take What?" message
```

In the Rust port these are handled directly — no queue is needed because the screen is redrawn every frame.

### Mouse Click → Button Slot Mapping (`fsubs.asm:136–165`)

```
Valid click X range (Amiga hi-res): 430–530
  lo-res equivalent: 215–265

Button index calculation (lo-res coordinates):
  row   = (mouseY - 144) / 9
  col   = (mouseX < 240) ? 0 : 1   // left column = even slots, right = odd
  index = row * 2 + col             // 0–11; maps to display slot

On mouse-down : generates code  0x61 + index  (button press)
On mouse-up   : generates code  0x80 | (0x61 + index)  (button release)
```

The Rust port maps SDL2 mouse coordinates directly without the Amiga lo-res scaling factor.

---

## Known Original Exploits

These bugs exist in the original 1987 release. The port should avoid replicating them.

### Pause-Take duplication (`fmain.c` — do_option / prq path)

When the game is paused (Space), pressing `T` triggers the Take action. Because the game
loop is suspended, the player can press `T` repeatedly to pick up the same ground item
multiple times without it being consumed.

**Fix**: Guard `MenuAction::Take` dispatch (and any other item-consuming immediate action)
behind an `!is_paused()` check, similar to the existing `gomenu()` guard. The `handle_key`
path in `menu.rs` already blocks all keys except Space while paused, so the exploit cannot
occur via the menu key path. Verify that the `GameAction::Take` path in the direct key
binding layer (`key_bindings.rs`) also checks the paused state before acting.

### Key replenishment after save/reload within a session (`fmain.c` — save/load path)

If the player enters an area, saves the game, uses keys to unlock doors, then reloads the
save in the same session, the keys are restored from the save file but the door-unlocked
state is not reset (door state is held in a runtime table, not persisted). The player
effectively gets unlimited key uses.

**Fix**: When implementing `LoadGame`, reset all in-memory door state (the runtime "door
open" flags in `doors.rs`) before restoring from the save file. Alternatively, persist door
state as part of the save file format so reload is fully consistent.
