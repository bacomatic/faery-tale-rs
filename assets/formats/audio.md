# Audio — music, instruments, sound effects

Directory: `audio/`. The game has no sampled music: 7 songs of 4 Paula voices each are played by
the sequencer in `src/gdriver.asm` from note event streams (`songs` file) through 8 looped
waveforms and 10 volume envelopes (`v6` file). The 6 sound effects are 8-bit PCM samples on the
disk image. **Timing is NTSC**: 60 Hz vertical blank, Paula clock 3,579,545 Hz.

```
audio/
  music/format.json             the synth model: event format, period/duration/instrument tables, timing, volume
  music/track_00 … track_27.json  28 decoded voice tracks (song N = tracks 4N..4N+3)
  music/previews/               song_N.wav (4-voice mix), track_NN.wav (one voice) — rough renders, non-authoritative
  instruments/waveforms.json    8 × 128-byte waveforms
  instruments/envelopes.json    10 × 256-byte volume envelopes
  sfx/sfx_0 … sfx_5.wav         the 6 effects, byte-exact PCM
  sfx/sfx.json                  trigger sites and the per-trigger playback period
  sfx/previews/                 the same effects resampled to 44.1 kHz so any player accepts them
```

## Songs and tracks — `music/track_NN.json`

| song | tracks | theme | loops | played when |
|---|---|---|---|---|
| 0 | 0–3 | day | yes | outdoors, `lightlevel > 120` |
| 1 | 4–7 | combat | yes | `battleflag` |
| 2 | 8–11 | night | yes | outdoors, `lightlevel ≤ 120` |
| 3 | 12–15 | title | no | title/intro |
| 4 | 16–19 | astral | yes | the astral-plane box |
| 5 | 20–23 | indoors | yes | regions 8–9 (`new_wave[10]` is swapped per region, see instruments) |
| 6 | 24–27 | death | no | hero dead |

Selection logic: `setmood()`, `src/fmain.c:2936-2956`; `theme.played_when` in each file states it.

Track file fields:

| Field | Meaning |
|---|---|
| `track`, `song`, `voice` | track = `4*song + voice`; voice 0–3 = Paula channel |
| `theme` | `{name, title, played_when, citations}` |
| `source` | `{file: "songs", offset, bytes, packlen_words, citations}` — the track's bytes in the `songs` file (`src/fmain2.c:765-772`) |
| `format` | `"format.json"` |
| `initial_instrument` | `{waveform, envelope, note, citations}` — what `playscore()` seeds the voice with (`new_wave[voice]`) before any `set_instrument` event |
| `loop` | the end event's repeat flag |
| `event_count`, `counts` | totals per event kind |
| `trailing_bytes_after_end` | bytes after the end event (0 everywhere) |
| `events[]` | the stream, in order (below) |

Every event is 2 bytes, `raw: [command, value]`, with `offset` (byte offset in the track) and `kind`:

| `kind` | command | extra fields |
|---|---|---|
| `note` | `0x00–0x7f` = ptable index | `note` (index), `note_name`, `period`, `wave_offset`, `duration_code` (value & 0x3f), `duration_counts` (`duration_table[code]`), `tie` (bit 6), `chord` (bit 7) — both flags are stored but unused by the player |
| `rest` | `0x80` | `duration_code`, `duration_counts`, `tie`, `chord`; volume 0 for the duration |
| `set_instrument` | `0x81` | `instrument` (value & 0x0f), `waveform`, `envelope` (resolved through `instrument_table`) |
| `set_tempo` | `0x90` | `tempo` (value) — shared by all four voices |
| `end` | `0xff` | `loop` (value ≠ 0 → repeat from start; 0 → voice stops) |

Other command bytes are skipped by the player. Instrument, tempo and end events take no time.

## The synth model — `music/format.json`

- **Clock.** Each vertical blank (60 Hz) every voice adds `tempo` to its time counter; a note or
  rest lasts `duration_counts` counts, i.e. `duration_counts / (tempo * 60)` seconds. The voice
  is silenced `note_gap_counts` = 300 counts before the next event (no gap when the note is
  shorter than that). `initial_tempo` = 150; songs usually set their own with `set_tempo`.
  (`timing`, `src/gdriver.asm:65-67, 149-155, 429`)
- **Durations.** `duration_table.values` = `notevals`, 64 words indexed by duration code: 8 rows
  of 8, each column half the previous (`src/gdriver.asm:228-235`).
- **Pitch.** `period_table.entries[]` (78, `ptable`, `src/gdriver.asm:204-222`): `{index, period,
  wave_offset, wave_start_byte, wave_len_bytes, pitch_hz_ntsc, note_name}`. Paula plays the
  waveform slice starting at byte `wave_offset*4` with length `(32 - wave_offset)*2` bytes, one
  byte per `period` clock ticks, so the cycle frequency is `3579545 / (period * wave_len_bytes)`.
  Higher octaves use the shorter 32/16/8-byte cycles stored after the 64-byte one
  (`instruments/waveforms.json.octave_slices`).
- **Instruments.** `instrument_table.values[]` (12, `new_wave[]`, `src/fmain.c:669-672`):
  `{instrument, word, waveform, envelope}` — high byte = waveform 0–7, low byte = envelope 0–9.
  `setmood()` rewrites entry 10 to `0x0307` in region 9 and `0x0100` in region 8
  (`src/fmain.c:2945-2946`); the shipped table holds the static values.
- **Volume.** When a note starts, byte 0 of its envelope is written to the channel volume; each
  following vertical blank the next byte is written. A byte ≥ 128 holds the current volume (the
  pointer stops advancing) — sustain. Volumes are Paula 0–64 (`volume`, `src/gdriver.asm:98-104,
  167-175`).

## Instruments — `instruments/`

`waveforms.json`: `{count: 8, length: 128, sample_format: "signed 8-bit (Paula)", waveforms: [{index,
samples[128]}], octave_slices, selection, source, v6_unread}` — waveform `n` is bytes `n*128` of
the `v6` file. `octave_slices.slices[]` `{wave_offset, start, length}` are the four cycle lengths
inside each waveform. `v6_unread` preserves (as hex) the bytes of `v6` the game never reads.

`envelopes.json`: `{count: 10, length: 256, sample_format, envelopes: [{index, values[256],
hold_from}], semantics, source}` — envelope `n` is bytes `2048 + n*256` of `v6` (the game's second
`Seek` is relative, hence 2048 not 1024); `hold_from` is the index of the first byte ≥ 128
(sustain), 255 when there is none. `values` are the stored unsigned bytes.

## Sound effects — `sfx/`

`sfx_N.wav`: 8-bit **unsigned** mono PCM, byte-exact (`original signed byte XOR 0x80`), no
resampling. The WAV header rate is the *nominal* rate `3579545 / nominal_period` (rounded); the
game has no fixed rate — every trigger plays at `period = base + random`:

`sfx.json.effects[]`: `{effect, file, buffer_offset, byte_length, nominal_period, nominal_rate_hz,
triggers: [{when, expr, base, rng, rng_max, source}], previews: [{file, period, rate_hz, wav_rate_hz}]}`.
`expr` is the source expression (e.g. `800+bitrand(511)`); actual rate = `3579545 / period`.
Effect 5 has two bases (1800 at the launch sites, 3200 at the hit site); the header uses 1800
and `previews/sfx_5_period3200.wav` renders the other. `format` and `source` describe the on-disk
framing (4-byte big-endian length prefix per sample, blocks 920–930, `src/fmain.c:645, 1033-1041`)
and the playback call (`effect(num, speed)` → channel 2, volume 64, `src/fmain.c:3616-3619`).

| effect | nominal period | what |
|---|---|---|
| 0 | 800 | a melee blow lands on the hero |
| 1 | 150 | melee near miss (swing within reach that does not hit) |
| 2 | 500 | an arrow hits a figure, or the witch's effect hits the hero |
| 3 | 400 | a melee blow lands on an enemy |
| 4 | 400 | an arrow is loosed |
| 5 | 1800 / 3200 | dragon fire or wand bolt launched; bolt hits a figure |

`sfx/previews/*.wav` are the same bytes rendered with zero-order hold at the base period and
resampled to 44,100 Hz (browsers reject rates below 3 kHz); non-authoritative.
