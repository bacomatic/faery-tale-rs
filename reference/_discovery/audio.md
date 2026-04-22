# Discovery: Audio System — Music Tracker & Sound Effects

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace the complete audio system: VBlank interrupt-driven music engine, music data format, voice state machine, sound effect playback, mood-based track selection, and all public API functions.

## VBlank Server

The music engine runs as a VBlank interrupt handler (`_vblank_server`, gdriver.asm:57). It is installed via `AddIntServer` on interrupt #5 (vertical blank) during `_init_music` (gdriver.asm:440-441).

### Interrupt Handler Flow (gdriver.asm:57-80)

```
_vblank_server:
  1. Save registers d0-d5/a0-a5
  2. Clear flag_codes(a1)        — signals VBlank occurred (sync byte)
  3. Add tempo(a1) to timeclock(a1) — always, even when muted
  4. Test nosound(a1):
     - If non-zero → skip to exit (no audio processing)
  5. Set a3 = a1 (voice data pointer), a4 = $DFF000 (hardware regs)
  6. Call dovoice 4 times (sequentially, voices 1-4)
  7. Return d0=0 (continue server chain)
```

- **a1** points to `_vblank_data` — the complete engine state block
- **a4** points to Amiga custom chip base `$DFF000`
- The handler always updates `timeclock` regardless of `nosound` — other parts of the program use it as a timer (gdriver.asm:63-67)
- `nosound` byte at offset 0: when set to -1 ($FF), inhibits all sound processing (gdriver.asm:71-72)

### Audio Interrupt Handler (gdriver.asm:264-282)

A separate interrupt handler `audio_int` is installed on interrupt #8 (audio channel 2) via `SetIntVector` (gdriver.asm:453-457). This handles sample playback completion:

```
audio_int:
  1. Clear INTENA bit 8 and INTREQ bit 8
  2. Check vce_stat for voice2 (sample status counter)
     - If 0 → exit, leave interrupt disabled
     - Decrement vce_stat
     - If vce_stat reaches 0 → shut off sample: volume=0 on $B8, period=2 on $B6
     - If vce_stat still >0 → re-enable interrupt (INTENA bit 8 set)
```

- gdriver.asm:266-267: clears `INTENA` bit 8 (`%0000000100000000`) and `INTREQ` bit 8
- gdriver.asm:269: checks `vce_stat+voice2` — voice2 is the sample effects channel
- gdriver.asm:275: when done, writes 0 to `$B8(a0)` (volume) and 2 to `$B6(a0)` (period)

## `_vblank_data` — Engine State Block

Defined in dseg at gdriver.asm:492. Total size: 2 longs + `vbase + (4 * voice_sz) + 16` bytes.

### Global Fields (gdriver.asm:17-23)

| Offset | Name | Size | Description |
|--------|------|------|-------------|
| 0 | `nosound` | byte | Sound inhibit: 0=play, -1=mute |
| 1 | `flag_codes` | byte | Sync byte, cleared each VBlank |
| 2 | `tempo` | word | Tempo counter (added to timeclock each VBlank) |
| 4 | `ins_handle` | long | Pointer to instrument table (`new_wave` array) |
| 8 | `vol_handle` | long | Pointer to volume/envelope data (`volmem`) |
| 12 | `wav_handle` | long | Pointer to waveform data (`wavmem`) |
| 16 | `timeclock` | long | Cumulative time counter (tempo added each VBlank) |
| 20 | — | 4 bytes | (padding to vbase=24) |

### Per-Voice Fields (gdriver.asm:25-38)

`vbase` = 24. Each voice is `voice_sz` = 28 bytes. 4 voices = 112 bytes total.

| Offset (from voice base) | Name | Size | Description |
|---------------------------|------|------|-------------|
| 0 | `wave_num` | byte (as word) | Index to waveform number |
| 1 | `vol_num` | byte (as word) | Index to envelope number |
| 2 | `vol_delay` | byte (as word) | Volume status / delay flag |
| 3 | `vce_stat` | byte | Effect/rest status (bit flags) |
| 4 | `event_start` | long | Timeclock value when next event starts |
| 8 | `event_stop` | long | Timeclock value when current note should stop |
| 12 | `vol_list` | long | Pointer to current position in volume envelope |
| 16 | `trak_ptr` | long | Read head pointer (current playback position) |
| 20 | `trak_beg` | long | Pointer to track beginning (for looping) |
| 24 | `trak_stk` | long | Pointer to loop stack |

Voice offsets (gdriver.asm:43-46):
- `voice1` = 0
- `voice2` = 28 (voice_sz)
- `voice3` = 56 (voice_sz*2)
- `voice4` = 84 (voice_sz*3)

### `vce_stat` Bit Flags (gdriver.asm:40-42)

| Bit | Value | Name | Meaning |
|-----|-------|------|---------|
| 2 | 4 | TIE | Tied note (no gap between notes) |
| 3 | 8 | CHORD | Chorded note |
| 4 | 16 | REST | Voice is resting |
| 5 | 32 | ENDTK | Track has ended |

Additionally, `vce_stat` is used as a sample countdown in voice2 — `_playsample` sets it to 2 (gdriver.asm:309), and `audio_int` decrements it to track sample completion.

## Music Data Format

### Score Data (SMUS-derived)

Scores are loaded from the file `"songs"` on disk (fmain2.c:765). The format is a sequence of tracks, each preceded by a 4-byte length word:

```
For each track (up to 4*7 = 28 tracks):
  [4 bytes] packlen (long) — length in words (multiply by 2 for bytes)
  [packlen*2 bytes] track data
```
— fmain2.c:760-776

Track data is a byte-pair stream processed by `newnote` (gdriver.asm:117):

```
For each event:
  byte 0: command byte (d3)
  byte 1: value byte (d2)
```

### Command Byte Encoding (gdriver.asm:126-140)

| Command (d3) | Meaning | Value byte (d2) |
|-------------|---------|-----------------|
| bit7=0 (0-127) | **Note** | Note number (bits 0-5) + TIE (bit6) + CHORD (bit7) |
| 128 ($80) | **Rest** | Duration code (same as note) |
| 129 ($81) | **Set Instrument** | Instrument number (0-15) |
| $90 (144) | **Set Tempo** | New tempo value |
| 255 ($FF) | **End Track** | 0=stop, non-zero=repeat from beginning |

### Note Numbering and Duration

The value byte for notes/rests (after clearing CHORD bit7 and TIE bit6) indexes into `notevals` (gdriver.asm:213-220) to get duration in timeclock counts:

```
notevals (8 rows × 8 columns):
  Row 0: 26880, 13440,  6720, 3360, 1680,  840,  420,  210
  Row 1: 40320, 20160, 10080, 5040, 2520, 1260,  630,  315
  Row 2: 17920,  8960,  4480, 2240, 1120,  560,  280,  140
  Row 3: 26880, 13440,  6720, 3360, 1680,  840,  420,  210
  Row 4: 21504, 10752,  5376, 2688, 1344,  672,  336,  168
  Row 5: 32256, 16128,  8064, 4032, 2016, 1008,  504,  252
  Row 6: 23040, 11520,  5760, 2880, 1440,  720,  360,  180
  Row 7: 34560, 17280,  8640, 4320, 2160, 1080,  540,  270
```

These are SMUS-standard timing values. The comment at gdriver.asm:210 says "these are the timing values for interpreting an SMUS score."

Each column within a row halves the previous, suggesting:
- Column 0 = whole note, column 1 = half, column 2 = quarter, ..., column 7 = 128th
- Each row represents a different base duration (dotted, triplet, etc.)

Note gap: 300 timeclock counts are subtracted from note duration for articulation gap (gdriver.asm:149-151). If the note is shorter than 300 counts, no gap is applied.

### Note-to-Frequency Conversion (`ptable`, gdriver.asm:193-209)

The command byte (d3) for notes is shifted left by 2 (×4) and used to index `ptable`, which contains pairs of (period, waveform_offset):

```
ptable format: [period_word, offset_word] per note

Octave layout (12 notes per octave, 7 octaves):
  Octave 0 (extra low):  periods 1440-1076, offset 0
  Octave 1:              periods 1016-538,  offset 0
  Octave 2:              periods 508-269,   offset 0
  Octave 3:              periods 508-269,   offset 16
  Octave 4:              periods 508-269,   offset 24
  Octave 5:              periods 508-269,   offset 28
  Octave 6 (high):       periods 254-135,   offset 28
```

For higher octaves, the waveform offset increases (16, 24, 28) — this shifts the read pointer into the 128-byte waveform, shortening the effective waveform length. The waveform length is calculated as `32 - offset` words (gdriver.asm:174-175).

Period values are standard Amiga Paula chip periods based on the PAL/NTSC clock.

### Waveform Data

Waveforms are loaded from file `"v6"` on disk (fmain.c:931-935):
- `wavmem`: 1024 bytes (128 × 8 waveforms), allocated as MEMF_CHIP — `S_WAVBUF = 128 * 8` (fmain.c:663)
- Each waveform is 128 bytes (64 words)
- Waveform selection: `wave_num * 128 + wav_handle` (gdriver.asm:166-168)

### Envelope/Volume Data

Also loaded from `"v6"` (fmain.c:932-934), after seeking past the waveform data:
- `volmem`: 2560 bytes (10 envelopes × 256 bytes each) — `S_VOLBUF = 10 * 256` (fmain.c:664)
- `volmem` is at `wavmem + S_WAVBUF` (fmain.c:913)
- Selection: `vol_num * 256 + vol_handle` (gdriver.asm:170-172)
- First byte of envelope is the starting volume level (gdriver.asm:173)
- Subsequent bytes are read sequentially each VBlank as new volume values (gdriver.asm:100-106)
- A **negative** value (bit 7 set) means "hold current volume" (gdriver.asm:103)

### Instrument Table (`new_wave`)

Defined in fmain.c:669-671:
```c
short new_wave[] = 
{ 0x0000, 0x0000, 0x0000, 0x0000, 0x0005,
  0x0202, 0x0101, 0x0103, 0x0004, 0x0504, 
  0x0100, 0x0500 };
```

This is passed as `ins_handle` to `_init_music` (fmain.c:914). Each word maps an instrument number to a waveform index. When a "Set Instrument" command (129/$81) is encountered:
- `d2` = instrument number (masked to 0-15)
- `wave_num(a3)` = `ins_handle[d2]` (gdriver.asm:248-250)

The first 4 entries are waveform 0; entry 4 is waveform 5, etc. Note: `setmood` modifies `new_wave[10]` for the underworld region (fmain.c:2946-2947).

## Voice State Machine

### Per-Voice Processing: `dovoice` (gdriver.asm:82-191)

Called 4 times per VBlank, once per voice. Advances `a3` by `voice_sz` (28) and `a4` by 16 (next audio register set) on exit.

```
dovoice:
  1. If trak_ptr == NULL → skip voice (vocx)
  2. Compare timeclock vs event_start:
     - If timeclock >= event_start → goto newnote (process next event)
  3. If vce_stat != 0 → skip voice (sample playing, don't interrupt)
  4. Compare timeclock vs event_stop:
     - If timeclock >= event_stop → goto rest_env (note ended, silence)
  5. If vol_delay != 0 → skip voice
  6. Read next byte from vol_list (envelope):
     - If negative → skip (hold volume)
     - Otherwise → set volume register $A8(a4), advance vol_list pointer
```

### `newnote` — New Event Processing (gdriver.asm:116-191)

```
newnote:
  1. Read 2 bytes from trak_ptr: command (d3), value (d2)
  2. Advance trak_ptr
  3. Dispatch on command byte:
     - bit7=0 → normal note (note_comm)
     - 128 → rest (note_comm, but dorestnote path)
     - 129 → set_voice (change instrument)
     - $90 → set_tempo (change playback speed)
     - 255 → endtrak (end of track)
     - other → skip (bra newnote — process next event)
```

### `note_comm` — Note/Rest Processing (gdriver.asm:141-191)

```
note_comm:
  1. Clear CHORD bit (bit7) of value — ignoring chorded notes
  2. Clear TIE bit (bit6) of value
  3. Look up note duration from notevals[value]
  4. Calculate event_stop = event_start + duration - 300 (gap)
  5. Calculate new event_start = old event_start + duration
  6. If vce_stat != 0 → skip (sample playing on this voice)
  7. Look up waveform: wave_num * 128 + wav_handle
  8. Look up envelope: vol_num * 256 + vol_handle
  9. Read starting volume from envelope
  10. If command = 128 (rest) → dorestnote (set volume 0)
  11. Otherwise:
      a. Look up period and waveform offset from ptable[note*4]
      b. Calculate effective waveform length = 32 - offset (words)
      c. Add offset*4 to waveform pointer
      d. Set Paula registers:
         - $A8(a4) = initial volume
         - $A0(a4) = waveform pointer
         - $A4(a4) = waveform length
         - $A6(a4) = period
```

### `rest_env` — Note Release (gdriver.asm:107-108)

Sets volume register to 0: `move.b #0,$a8(a4)`.

### `dorestnote` — Rest Note (gdriver.asm:110-113)

Sets volume to 0 and sets `event_stop` = current `timeclock` (comment says "KLUGE!!").

### `endtrak` — End of Track (gdriver.asm:248-257)

```
endtrak:
  - If value byte (d2) is non-zero → repeat: reset trak_ptr = trak_beg, goto newnote
  - If value byte is 0 → stop: clear trak_ptr, set vol_delay=-1, volume=0
```

### State Transitions Summary

```
Voice States:
  IDLE (trak_ptr=NULL) → no processing
  PLAYING (timeclock < event_start) → envelope processing
  NOTE_END (timeclock >= event_stop) → rest_env (silence)
  NEW_EVENT (timeclock >= event_start) → newnote dispatches:
    → note_comm → set hardware regs, begin envelope
    → dorestnote → silence for rest duration
    → set_voice → change instrument, process next event immediately
    → set_tempo → change tempo, process next event immediately
    → endtrak → either loop (repeat) or stop (IDLE)
  SAMPLE (vce_stat != 0) → immune to note processing
```

## Music API

### `_init_music(ins_handle, wav_handle, vol_handle)` (gdriver.asm:423-464)

Initializes the music engine:
1. Stores `ins_handle`, `wav_handle`, `vol_handle` into `_vblank_data` (gdriver.asm:426-428)
2. Sets initial tempo to 150 (gdriver.asm:429)
3. Sets `nosound` = -1 (muted until `_playscore` is called) (gdriver.asm:430)
4. Creates and installs VBlank interrupt server node (gdriver.asm:432-441):
   - `server_node.is_Data` = `_vblank_data`
   - `server_node.is_Code` = `_vblank_server`
   - Calls `AddIntServer(5)` — INTB_VERTB (vertical blank)
5. Creates and installs audio channel 2 interrupt handler (gdriver.asm:445-457):
   - `aud_node.is_Data` = `_vblank_data`
   - `aud_node.is_Code` = `audio_int`
   - Calls `SetIntVector(8)` — saves old handler in `old_handler3`
6. Clears pending audio interrupt and disables audio interrupt initially (gdriver.asm:460-461)

Called from: fmain.c:914 — `init_music(new_wave, wavmem, volmem)`

### `_wrap_music()` (gdriver.asm:466-484)

Shuts down the music engine:
1. Calls `_stopscore` (gdriver.asm:467)
2. Removes VBlank server via `RemIntServer(5)` (gdriver.asm:470-472)
3. Disables audio interrupt (gdriver.asm:474)
4. Restores old audio interrupt handler via `SetIntVector(8)` (gdriver.asm:476-478)

Called from: fmain.c:959 — `wrap_music()` during `close_all()`

### `_playscore(t1, t2, t3, t4)` (gdriver.asm:352-405)

Starts playing 4 tracks simultaneously:
1. Sets both `trak_beg` and `trak_ptr` for all 4 voices (gdriver.asm:358-373)
2. Zeros all 4 volume registers ($A8, $B8, $C8, $D8) (gdriver.asm:375-378)
3. Initializes `wave_num` for each voice from `ins_handle` (first 4 words) (gdriver.asm:380-383)
4. Sets `vol_delay` = -1 for all voices (disable envelopes initially) (gdriver.asm:385-388)
5. Resets `timeclock` to 0 (gdriver.asm:389)
6. Resets `event_start` and `event_stop` to 0 for all voices (gdriver.asm:390-395)
7. Clears `vce_stat` for all voices (gdriver.asm:396-399)
8. Sets `nosound` = 0 (enable playback) (gdriver.asm:401)
9. Enables DMA for all 4 audio channels: `$820F → $96(a0)` (DMACON) (gdriver.asm:402)

### `_setscore(t1, t2, t3, t4)` (gdriver.asm:338-350)

Sets `trak_beg` for all 4 voices **without** resetting playback state. Unlike `_playscore`, it does NOT:
- Set `trak_ptr` (continues current playback position)
- Reset timeclock, event timers, or vce_stat
- Touch volume registers or DMACON

This is used for "deferred" track changes: the new tracks take effect when the current ones loop (via `endtrak` → `repeat` which copies `trak_beg` to `trak_ptr`).

### `_stopscore()` (gdriver.asm:407-421)

Stops all music immediately:
1. Zeros all 4 volume registers (gdriver.asm:411-414)
2. Clears `vol_delay` and `vce_stat` (as a word, clearing both) for all 4 voices (gdriver.asm:415-418)
3. Kills audio DMA: `$000F → $96(a0)` (DMACON clear bits) (gdriver.asm:420)
4. Sets `nosound` = -1 (gdriver.asm:421)

### `_set_tempo(value)` (gdriver.asm:332-336)

Sets the `tempo` word in `_vblank_data`. Initial value is 150 (gdriver.asm:429). This controls how fast `timeclock` advances each VBlank — higher values = faster playback.

### Track Selection in `setmood()` and `_setscore`/`_playscore`

`setmood()` (fmain.c:2936) calls either `playscore()` (full reset) or `setscore()` (deferred change) depending on the `now` parameter:
- `now=TRUE` → `playscore()` — immediate track change with full reset
- `now=FALSE` → `setscore()` — deferred change, takes effect at loop point

## Sound Effects

### `_playsample(effect, length, rate)` (gdriver.asm:296-322)

Plays a one-shot sample on **audio channel 2** (voice2):
1. Disables DMA channel 2: `$0002 → $96(a0)` (gdriver.asm:298)
2. Sets period to 2 temporarily (gdriver.asm:299)
3. Sets `vce_stat+voice2` = 2 (prevents note engine from overwriting) (gdriver.asm:309)
4. Sets `vol_delay+voice2` = -1 (no envelope changes) (gdriver.asm:310)
5. Sets volume to 64 (max) on register `$B8` (gdriver.asm:311)
6. Loads waveform pointer → `$B0`, length → `$B4`, rate → `$B6` (gdriver.asm:312-314)
7. Clears pending audio interrupt, enables audio interrupt (gdriver.asm:316-317)
8. Enables DMA channel 2 with repeat: `$8202 → $96(a0)` (gdriver.asm:319)

The `audio_int` handler (gdriver.asm:264-282) decrements `vce_stat+voice2` on each audio interrupt. When it reaches 0, the sample is stopped (volume zeroed).

### `_stopsample()` (gdriver.asm:324-330)

Stops sample playback:
1. Clears bit 0 of `vce_stat+voice4` (gdriver.asm:326) — note: this references voice4, possibly a typo or secondary channel
2. Zeros volume on `$B8` (channel 2) (gdriver.asm:328)
3. Sets period to 2 on `$B6` (gdriver.asm:329)

### `effect(num, speed)` — C wrapper (fmain.c:3616-3619)

```c
effect(num, speed) short num; long speed;
{   if (menus[GAME].enabled[7] & 1)
    {   playsample(sample[num], sample_size[num]/2, speed); }
}
```

- Checks `menus[GAME].enabled[7]` bit 0 — this is the **Sound** toggle in the Game menu (fmain.c:501: label4="PauseMusicSoundQuit Load ", index 7=Sound)
- `sample[num]` = pointer to sample data in `sample_mem`
- `sample_size[num]/2` = length in words (Paula expects word count)
- `speed` = period value (lower = higher pitch)

### Sample Loading: `read_sample()` (fmain.c:1023-1040)

Samples are loaded from disk blocks 920-930 (11 blocks) into `sample_mem` (MEMF_CHIP, 5632 bytes):
- Uses `load_track_range(920, 11, sample_mem, 8)` (fmain.c:1028)
- Format: sequence of 6 samples, each preceded by a 4-byte length:
  ```
  [4 bytes] ifflen (long)
  [ifflen bytes] raw 8-bit signed PCM sample data
  ```
- fmain.c:1034-1039: iterates 6 samples, storing pointers in `sample[0..5]` and sizes in `sample_size[0..5]`

### Effect Index Usage (Cross-Cutting Findings)

| Index | Context | Speed | Citation |
|-------|---------|-------|----------|
| 0 | Hero hit by enemy | 800+bitrand(511) | fmain2.c:240 |
| 1 | Near miss in combat | 150+rand256() | fmain.c:2262 |
| 2 | Killing blow? | 500+rand64() | fmain2.c:238 |
| 3 | Enemy hit by hero | 400+rand256() | fmain2.c:241 |
| 4 | Arrow fired | 400+rand256() | fmain.c:1680 |
| 5 | Dragon fire breath / fire attack | 1800+rand256() | fmain.c:1488, 1690 |
| 5 | Special ranged attack | 3200+bitrand(511) | fmain2.c:239 |

Effect 5 is reused for fire/special attacks with different speed ranges.

## Mood System

### `setmood(now)` (fmain.c:2936-2957)

Selects music track based on game state. The `off` variable is a track offset into the `track[]` array. Each "song" is 4 tracks (one per voice), so offsets are multiples of 4.

```c
setmood(now) char now;
{   register long off;
    if (anim_list[0].vitality == 0) off = (6*4);          // Dead: song 6
    else if (hero_x > 0x2400 && hero_x < 0x3100 &&
            hero_y > 0x8200 && hero_y < 0x8a00)
    { off = (4*4); }                                       // Specific map area: song 4
    else if (battleflag) off = 4;                          // Combat: song 1 (offset 4)
    else if (region_num > 7)
    {   off = (5*4);                                       // Underground/indoors: song 5
        if (region_num == 9) new_wave[10] = 0x0307;        // Astral plane instrument
        else new_wave[10] = 0x0100;                        // Normal underground instrument
    }
    else if (lightlevel > 120) off = 0;                    // Daytime: song 0
    else off = 8;                                          // Nighttime: song 2 (offset 8)

    if (menus[GAME].enabled[6] & 1)                        // Music toggle enabled?
    {   if (now)
            playscore(track[off],track[off+1],track[off+2],track[off+3]);
        else setscore(track[off],track[off+1],track[off+2],track[off+3]);
    }
    else stopscore();
}
```

### Track Index Map

| Song # | Offset | Condition | Citation |
|--------|--------|-----------|----------|
| 0 | 0 | Daytime (lightlevel > 120) | fmain.c:2949 |
| 1 | 4 | Combat (battleflag set) | fmain.c:2942 |
| 2 | 8 | Nighttime (lightlevel <= 120) | fmain.c:2950 |
| 3 | 12 | Title/intro screen | fmain.c:1182 |
| 4 | 16 | Specific map zone (hero_x 0x2400-0x3100, hero_y 0x8200-0x8A00) | fmain.c:2939-2941 |
| 5 | 20 | Underground/indoors (region_num > 7) | fmain.c:2943-2947 |
| 6 | 24 | Death (vitality == 0) | fmain.c:2938 |

### Song Loading: `read_score()` (fmain2.c:760-776)

Loads from `"songs"` file. Reads up to `4*7 = 28` tracks (7 songs × 4 voices). Each track:
1. Read 4-byte `packlen` (in words)
2. Check if `packlen*2 + sc_load > 5900` (SCORE_SZ) — if so, stop loading
3. Store pointer `scoremem + sc_load` into `track[sc_count]`
4. Read `packlen*2` bytes of track data
5. Advance `sc_load` by `packlen*2`

`scoremem` is allocated as 5900 bytes (non-CHIP memory, fmain.c:912): `SCORE_SZ = 5900`

### `setmood` Callers (Cross-Cutting)

| Caller | now | Context | Citation |
|--------|-----|---------|----------|
| Main game loop | 0 (FALSE) | Every 8th daynight tick, if not in battle | fmain.c:2198 |
| Main game loop | 1 (TRUE) | When battle starts | fmain.c:2185 |
| `xfer()` | TRUE | After teleport/region change | fmain.c:2643 |
| `checkdead()` | TRUE | Hero death | fmain.c:2777 |
| Terrain fall | TRUE | Falling off ledge | fmain.c:1771 |
| `init_brother()` | TRUE | Brother succession | fmain.c:2910 |
| Menu GAME hit 6 | TRUE | Music toggle changed | fmain.c:3444 |

### Audio Device Allocation (fmain.c:889-906)

Before the music engine is initialized, the Amiga audio device is opened:
1. Create audio port (fmain.c:889)
2. Create IOAudio extended IO request (fmain.c:891)
3. Open `"audio.device"` with channel mask `0x0F` (all 4 channels) (fmain.c:897)
4. Send `CMD_RESET` to claim channels (fmain.c:899-904)
5. Set `audio_open = 1` (fmain.c:906)

Cleanup in `close_all()`: `CloseDevice`, `DeleteExtIO`, `DeletePort` (fmain.c:981-983)

## Cross-Cutting Findings

1. **Instrument table modification**: `setmood()` directly modifies `new_wave[10]` based on `region_num` (fmain.c:2946-2947). Region 9 (astral plane) gets instrument 0x0307 vs 0x0100 for other underground areas. This changes the instrument mapping on-the-fly during playback.

2. **VBlank timeclock dual use**: The comment at gdriver.asm:63 says "we have to update tempo clock regardless of what else is happening, because other parts of the program use it for a timer." The timeclock is updated even when `nosound` is set. Other code outside the audio system may depend on this timing.

3. **Voice2 shared between music and SFX**: `_playsample` uses audio channel 2 (voice2), which is also used for music. The `vce_stat` field is overloaded — for music it's a bit-flag field, but for samples it's a countdown from 2. This means sound effects temporarily replace voice2's music track.

4. **stopsample references voice4**: At gdriver.asm:326, `_stopsample` does `and.b #$fe,vce_stat+voice4(a1)` — this references voice4 rather than voice2 where samples play. This may be a bug or may clear a secondary flag; the intent is unclear.

5. **Menu toggles**: Music and Sound are independently toggleable via `menus[GAME].enabled[6]` (Music) and `menus[GAME].enabled[7]` (Sound) — both are type 7 (toggle) per fmain.c:530.

## Unresolved

1. **`trak_stk` (loop stack)**: Defined at offset 24 in the voice structure (gdriver.asm:37) but never referenced in any code path. It may be vestigial from a more complex SMUS implementation that supported nested loops.

2. **`flag_codes` sync byte**: Cleared each VBlank (gdriver.asm:60) but never read anywhere in the source. May have been used by tools or debugging code not included in the repository.

3. **Instrument table semantics**: The `new_wave` array has 12 entries but the `set_voice` command masks to 0-15. Only the first few entries map to meaningful waveforms. The high byte vs low byte semantics of each word (e.g., 0x0202 = wave 2 and vol 2?) are undetermined — the code only copies the word to `wave_num` which is defined as a byte at vbase+0 (gdriver.asm:248-250 stores a word to `wave_num`, but only the low byte would be used for waveform indexing at gdriver.asm:166).

4. **Map zone for song 4**: The coordinates hero_x 0x2400-0x3100, hero_y 0x8200-0x8A00 (fmain.c:2939-2940) correspond to a specific map area. Without map coordinate documentation, the exact location/significance is unknown.

5. **`_stopsample` voice4 reference**: As noted in cross-cutting findings, `_stopsample` at gdriver.asm:326 operates on `vce_stat+voice4` rather than `voice2` where samples are played. This may be a bug.

6. **SMUS standard compliance**: The format is described as "NOT SMUS STANDARD" for the $90 (set_tempo) command (gdriver.asm:133). The extent of deviation from the standard SMUS format is unclear.

7. **CHORD and TIE bits**: The code clears CHORD bit (bit7) with `bclr #7,d2` and TIE bit (bit6) with `bclr #6,d2` in `note_comm` (gdriver.asm:142-143), but never uses them. The comment says "ignore chorded notes" and "don't ignore ties" with a question mark. These may be parsed but not implemented.

8. **`v6` file format**: The waveform+envelope data file `"v6"` (fmain.c:931) is loaded with a seek past `S_WAVBUF` then read of `S_VOLBUF`. The seek at fmain.c:933 (`Seek(file, S_WAVBUF, 0)`) with offset_mode=0 (from beginning) seems redundant after already reading S_WAVBUF bytes — unless the file has additional data between the waveform and volume sections.

## Refinement Log
- 2026-04-06: Initial comprehensive discovery pass covering all 13 questions from the orchestrator prompt
