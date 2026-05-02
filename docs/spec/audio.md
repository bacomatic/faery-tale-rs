## 22. Audio System

### 22.1 VBlank-Driven Music Tracker

4-voice custom tracker driven by VBlank interrupt at 60 Hz (NTSC). Processes one tick per vertical blank.

#### Engine State

Global fields: `nosound` (mute flag), `tempo` (playback speed, default 150), `ins_handle` (instrument table), `vol_handle` (envelope data), `wav_handle` (waveform data), `timeclock` (cumulative timer — incremented by `tempo` each VBL even when muted).

Per-voice (4 × 28 bytes): `wave_num`, `vol_num`, `vol_delay`, `vce_stat`, `event_start/stop`, `vol_list`, `trak_ptr/beg/stk`.

#### vce_stat Bit Flags

| Value | Name | Meaning |
|-------|------|---------|
| 4 | TIE | Tied note (no articulation gap) |
| 8 | CHORD | Chorded note |
| 16 | REST | Voice is resting |
| 32 | ENDTK | Track has ended |

On voice 2, `vce_stat` doubles as a sample-completion countdown.

#### Voice Processing

For each voice per VBL tick:
1. If `vce_stat != 0`, skip voice (yields to sample playback on voice 2)
2. Process volume envelope: apply current byte as volume level; bit 7 set = hold current volume
3. If `timeclock >= event_stop`: silence voice
4. If `timeclock >= event_start`: process new event
5. Otherwise: continue current note/rest

### 22.2 Waveform & Envelope Data

Both loaded from `v6` file:

- **Waveforms** (`wavmem`): 1024 bytes — 8 waveforms × 128 bytes each (CHIP memory). Higher octaves offset into waveform to shorten effective sample, raising pitch without resampling.
- **Envelopes** (`volmem`): 2560 bytes — 10 envelopes × 256 bytes each. Each byte is a volume level per VBL tick; bit 7 set = hold current volume.

### 22.3 Instrument Table

12-entry word array (`new_wave`) maps instrument numbers to waveform/envelope pairs. Stored in `ins_handle`, passed to `_init_music` at startup. `setmood()` modifies entry 10 at runtime for underworld region.

### 22.4 Music Data Format

Track data is a byte-pair stream:

| Command byte | Value byte | Action |
|-------------|-----------|--------|
| 0–127 | Note # + TIE/CHORD bits | Play note with duration from `notevals` table |
| 128 ($80) | Duration code | Rest (silence) for duration |
| 129 ($81) | Instrument # (0–15) | Change instrument |
| $90 (144) | New tempo value | Change playback speed |
| 255 ($FF) | 0=stop, non-zero=loop | End of track |

Note durations from `notevals` (8×8 SMUS-standard timing table). Articulation gap: 300 counts subtracted.

### 22.5 Period Table

84 entries (7 octaves × 12 notes). Hz = 3,579,545 / period (NTSC). Higher octaves use shorter waveform segments.

### 22.6 Mood-Based Track Selection

`setmood(now)` selects music based on game state:

| Priority | Condition | Song | Track offset |
|----------|-----------|------|-------------|
| 1 | Hero dead (`vitality == 0`) | Death | 24 |
| 2 | Specific map zone | Zone theme | 16 |
| 3 | In combat (`battleflag`) | Battle | 4 |
| 4 | Underground (`region_num > 7`) | Dungeon | 20 |
| 5 | Daytime (`lightlevel > 120`) | Day | 0 |
| 6 | Nighttime | Night | 8 |
| — | Startup | Intro | 12 |

Each song is 4 tracks (one per voice). `playscore()` resets playback immediately; `setscore()` defers change until current tracks loop.

### 22.7 Sound Effects

6 samples loaded from disk sectors 920–930 into `sample_mem`. Played on channel 2 via `_playsample()`, which overrides that channel's music voice.

Sample completion: `vce_stat` on voice 2 set to 2. Audio interrupt handler (`audio_int`) decrements per interrupt. When counter reaches 0: silence channel (volume → 0, period → 2), music resumes on voice 2.

`effect(num, speed)` C wrapper checks Sound menu toggle before calling `playsample()`.

| Sample | Usage | Typical Period |
|--------|-------|---------------|
| 0 | Hero injured | 800 + random(0–511) |
| 1 | Weapon swing | 150 + random(0–255) |
| 2 | Ranged hit | 500 + random(0–63) |
| 3 | Enemy hit | 400 + random(0–255) |
| 4 | Door/interaction | 400 + random(0–255) |
| 5 | Environmental | 1800–3200 + random |

---


