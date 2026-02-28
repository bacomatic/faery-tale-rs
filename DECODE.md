# Game File Decoding Notes

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
| 0 – 127 | **Note** — pitch index into `PTABLE` (84 entries, 7 octave groups × 12 semitones) |
| 128 (0x80) | **Rest** — silence for the given duration |
| 129 (0x81) | **Set Instrument** — `value & 0x0f` selects a slot from the `new_wave[]` instrument table |
| 144 (0x90) | **Set Tempo** — `value` is written directly to the tempo register (default 150) |
| 255 (0xFF) | **End Track** — `value ≠ 0` loops back to the start; `value = 0` stops the voice |
| other (bit 7 set) | Ignored (the ASM falls through to `newnote`) |

The **duration** of notes and rests comes from the `value` byte (bits 6–7
masked off), used as an index into `NOTE_DURATIONS[0..63]` — 64 tick counts
covering 8 note-length groups (4/4, 6/8, 3/4, 7/8, 5/4, 3/4 alt, 9/8, and a
duplicate 4/4).

The **pitch** byte (0–83) indexes into `PTABLE`, which stores
`(period, wave_offset)` pairs.  `period` is an Amiga Paula hardware period
register value; frequency ≈ `3,579,545 / period` Hz (NTSC clock).
`wave_offset` is a 16-bit–word offset into the 8-byte waveform stored in
`wavmem` (from `v6`) that selects which portion of the instrument waveform
Paula loops.

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
| 0x000 | 1,024 B | `wavmem` (wave buffer) | 128 waveforms × 8 bytes each | Signed 8-bit PCM sample data. Each 8-byte "waveform" is a short looping periodic waveform (sine-like, triangle, etc.) played by the Amiga's Paula DMA channels. The voice engine looks up `wave_num` per-voice and feeds the pointer/length into Paula's `$a0`/`$a4` registers. |
| 0x400 | 1,024 B | *(skipped)* | — | Deliberately skipped by the `Seek(OFFSET_CURRENT)` call. Likely extra waveform data that isn't used by this version of the engine, or reserved space. The hex dump shows it is all zeros. |
| 0x800 | 2,560 B | `volmem` (volume/envelope buffer) | 10 envelopes × 256 bytes each | Amplitude envelope tables. Each byte is a volume level (0–64 in Amiga terms). Voices index into these via `vol_num` and step through the table byte-by-byte each VBlank tick, creating ADSR-like shapes. When a voice hits the end of its envelope data, it zeroes the `$a8` volume register (silence). |
| 0x1200 | 20 B | *(trailing zeros)* | — | Padding at end of file, unused. |

### Role in the engine

The `gdriver.asm` music engine runs as an Amiga interrupt server on **VBlank** (60 Hz NTSC). Each tick, for each of the 4 voices it:
1. Reads the next note from the score/track
2. Looks up the waveform by index into `wavmem`
3. Loads the PCM pointer and length into Paula's DMA registers
4. Steps through the envelope in `volmem`, writing each volume byte to Paula's `$a8` register

The "v6" name likely refers to **Version 6** of the music voice data file — the `new_wave[]` array in `fmain.c` defines a 12-element default instrument table that maps tracks to waveform/envelope pairs, and there are in-game branches that swap entries depending on whether the player is indoors vs. outdoors (`new_wave[10] = 0x0307` or `0x0100`).
