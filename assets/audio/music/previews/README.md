# Music previews (non-authoritative)

Rough renders of the track event streams so a reviewer can recognise each theme by ear.
The JSON next to this directory is the deliverable; these WAVs are convenience artifacts.

- `track_NN.wav` -- one Paula voice (track NN) rendered alone.
- `song_N.wav` -- the 4 voices of song N (tracks 4N..4N+3) rendered together, as
  `playscore()` starts them (fmain.c:2953).

Renderer: a per-vertical-blank simulation of `dovoice` (gdriver.asm:85-200) -- tempo added
to the timeclock each frame, note gap of 300 counts, envelope byte per frame with the
"negative = hold" rule, instrument words split into waveform/envelope, `set_tempo` shared by
all voices. Simplifications:

- NTSC timing: 60 Hz vertical blank, Paula clock 3579545 Hz.
- Output 22050 Hz mono 16-bit, nearest-sample lookup into the waveform slice
  (no Paula DMA latency, no filtering, no aliasing control).
- A new note restarts the waveform slice at phase 0 (Paula would finish the current loop first).
- Volume saturates at 64; the 68000 byte write to AUDxVOL is treated as writing the byte value.
- Each render stops once every voice has either stopped or reached its end-of-track event once
  (looping tracks play a single pass), capped at 120 s.
- Single-voice renders are scaled x4 relative to the 4-voice mixes.
- Sound effects, the mute flag and the `new_wave[10]` swap for region 9 are not modelled.
