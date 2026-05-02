## 19. Audio

### Requirements

| ID | Requirement |
|----|-------------|
| R-AUDIO-001 | A 4-voice music tracker shall be driven by a 60 Hz VBL interrupt. `timeclock` shall be incremented by `tempo` each VBL frame, even when `nosound` is set (it doubles as a general-purpose timer). |
| R-AUDIO-002 | 8 waveforms (128-byte single-cycle 8-bit signed PCM) and 10 volume envelopes (256-byte) shall be loaded from the `v6` file. Waveforms = 1024 bytes at offset 0, envelopes = 2560 bytes at offset 1024. |
| R-AUDIO-003 | 28 music tracks (7 moods × 4 channels) shall be loaded from the `songs` file. |
| R-AUDIO-004 | Track data format: 2-byte command/value pairs — notes 0–127 (with TIE/CHORD bits), rest $80, set instrument $81, set tempo $90, end $FF (value 0=stop, nonzero=loop). |
| R-AUDIO-005 | Duration table: `notevals` 8×8 table of SMUS-standard timing values. Articulation gap of 300 counts subtracted from note duration. |
| R-AUDIO-006 | Period table: 84 entries (7 octaves × 12 notes). NTSC base: Hz = 3,579,545 / period. Higher octaves offset into waveform to shorten effective sample. |
| R-AUDIO-007 | 12-entry instrument table (`new_wave`) mapping instrument numbers to waveform/envelope pairs. Entry 10 modified at runtime for underworld region by `setmood()`. |
| R-AUDIO-008 | Per-voice state (4 × 28 bytes): `wave_num`, `vol_num`, `vol_delay`, `vce_stat`, `event_start/stop`, `vol_list`, `trak_ptr/beg/stk`. |
| R-AUDIO-009 | `vce_stat` bit flags: TIE=4, CHORD=8, REST=16, ENDTK=32. On voice 2, `vce_stat` doubles as sample-completion countdown. |
| R-AUDIO-010 | Music player shall skip any voice where `vce_stat != 0`, yielding to sample playback on voice 2. |
| R-AUDIO-011 | 7 musical moods evaluated by priority: Death (vitality==0, offset 24), Zone (specific map zone, offset 16), Battle (battleflag, offset 4), Dungeon (region>7, offset 20), Day (lightlevel>120, offset 0), Night (fallback, offset 8), Intro (startup, tracks 12–15). |
| R-AUDIO-012 | `setmood(TRUE)` / `playscore()` shall restart playback immediately. `setmood(FALSE)` / `setscore()` shall defer change until current tracks loop. |
| R-AUDIO-013 | 6 sound effect samples loaded from disk sectors 920–930, played on voice 2 via `playsample()`. Temporarily overrides music on that voice. |
| R-AUDIO-014 | Sound effect playback shall be gatable via the Sound menu toggle. `effect(num, speed)` checks toggle before calling `playsample()`. |
| R-AUDIO-015 | Sample completion on voice 2: `vce_stat` set to 2, audio interrupt handler decrements per interrupt, voice silenced when counter reaches 0, music resumes. |
| R-AUDIO-016 | Envelope processing: each byte is a volume level per VBL tick. Bit 7 set means "hold current volume". |

### User Stories

- As a player, I hear music that changes based on time of day, combat, location, and death.
- As a player, I hear sound effects for combat, interactions, and environmental events.
- As a player, I can toggle music and sound effects independently.

---


