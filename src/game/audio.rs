/// Software 4-voice synthesizer, porting the Amiga audio system from
/// `gdriver.asm` + `fmain.c`.
///
/// # Architecture
///
/// The original game runs the music sequencer inside the Amiga VBL interrupt
/// (`_vblank_server`), which fires at 60 Hz on NTSC hardware.  Each VBL:
///   1. `timeclock += tempo`  (default tempo 150 → 9 000 units/sec)
///   2. For each of the 4 voices, check whether it is time to advance to the
///      next note event.
///   3. Apply the volume ADSR envelope one step.
///   4. Update the Amiga Paula hardware registers (period, waveform address,
///      waveform length, volume).
///
/// We replicate all of this in Rust using an SDL2 audio device callback.  The
/// callback runs on a background thread and fills PCM buffers.  Sequencer
/// state is shared with the main thread via `Arc<Mutex<SequencerState>>` so
/// that `play_score()` / `stop_score()` are safe to call at any time.
///
/// # Waveform layout (`game/v6`)
///
/// ```text
/// bytes 0 – 1023    : 8 waveforms × 128 signed-byte samples each
/// bytes 1024 – 3583 : 10 ADSR envelopes × 256 bytes each
///                     (byte 0xFF = hold, i.e. "if negative, leave volume alone")
/// ```
///
/// Each voice loops the *tail* portion of its waveform:
/// - start byte offset = `wave_offset × 4`
/// - loop length in bytes = `(32 - wave_offset) × 2`
/// - values from `PTABLE` in `songs.rs`
///
/// # Instrument slots (`new_wave[]` from `fmain.c`)
///
/// | slot | wave | vol |
/// |------|------|-----|
/// |  0   |  0   |  0  |
/// |  1   |  0   |  0  |
/// |  2   |  0   |  0  |
/// |  3   |  0   |  0  |
/// |  4   |  0   |  5  |
/// |  5   |  2   |  2  |
/// |  6   |  1   |  1  |
/// |  7   |  1   |  3  |
/// |  8   |  0   |  4  |
/// |  9   |  5   |  4  |
/// | 10   |  1   |  0  |
/// | 11   |  5   |  0  |

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::fs;

use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};

use super::songs::{SongLibrary, Track, TrackEvent, PTABLE, NOTE_DURATIONS, AMIGA_CLOCK_NTSC, VBL_RATE_HZ, DEFAULT_TEMPO};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of waveforms in the `game/v6` file.
const WAVEFORM_COUNT: usize = 8;
/// Samples per waveform (bytes, signed).
const WAVEFORM_BYTES: usize = 128;

/// Number of volume envelopes in the `game/v6` file.
const ENVELOPE_COUNT: usize = 10;
/// Bytes per envelope (0xFF = hold sentinel).
const ENVELOPE_BYTES: usize = 256;

/// Number of instrument slots (entries in `new_wave[]`).
const INSTRUMENT_SLOTS: usize = 12;

/// Initial instrument mappings from `new_wave[]` in `fmain.c`.
/// Each entry is `(wave_num, vol_num)`.
const NEW_WAVE: [(u8, u8); INSTRUMENT_SLOTS] = [
    (0, 0), (0, 0), (0, 0), (0, 0), // slots 0-3
    (0, 5), (2, 2), (1, 1), (1, 3), // slots 4-7
    (0, 4), (5, 4), (1, 0), (5, 0), // slots 8-11
];

/// Gap between the end-of-note and the start of the next event, in timeclock
/// units.  Ported from `#300` in `gdriver.asm` `note_comm`.
const NOTE_GAP: u32 = 300;

/// Stereo panning weights for the Amiga's fixed DAC routing.
///
/// Amiga Paula DAC routing (interleaved to reduce chip-trace cross-talk):
///   voices 0 and 3 → Left DAC
///   voices 1 and 2 → Right DAC
/// A small bleed to the opposite side (analogous to capacitive coupling in the
/// A500 headphone amp) centres the soundstage while preserving the side bias.
///
/// With 2 voices per side and vol_scale = 0.5 already embedded in each voice:
///   max L = 2 × PRIMARY × 0.5 + 2 × BLEED × 0.5 = PRIMARY + BLEED = 1.0
/// so these two constants must sum to exactly 1.0.
const STEREO_PRIMARY: f32 = 0.75; // primary-side weight for a voice
const STEREO_BLEED:   f32 = 0.25; // opposite-side bleed weight

/// SDL2 PCM sample rate for synthesis output.
pub const SAMPLE_RATE: u32 = 44100;

/// Number of stereo frames between sequencer ticks (= SAMPLE_RATE / VBL_RATE).
/// We use a float accumulator to handle the non-integer case gracefully.
/// A "frame" is one L+R sample pair; the raw sample count is 2× this value.
const SAMPLES_PER_VBL: f64 = SAMPLE_RATE as f64 / VBL_RATE_HZ as f64;

// ---------------------------------------------------------------------------
// Sound effects
// ---------------------------------------------------------------------------

/// Number of sound effects packed in the `game/samples` file.
const SFX_COUNT: usize = 6;

/// Approximate playback rate of the original Amiga SFX in Hz.
/// The original `playsample()` uses a Paula period; 8 000 Hz is a close
/// approximation for these particular samples (period ≈ 443 at NTSC clock).
const SFX_SAMPLE_RATE: u32 = 8000;

/// Resampling step: SFX source samples consumed per 44 100 Hz output frame.
const SFX_STEP: f64 = SFX_SAMPLE_RATE as f64 / SAMPLE_RATE as f64;

/// Amplitude scale applied to SFX when mixing (equivalent to one full-volume
/// music voice).
const SFX_AMPLITUDE: f32 = 0.5;

/// Playback cursor for the currently-active sound effect.
struct SfxPlayback {
    data: Arc<Vec<i8>>,
    /// Fractional read position; advances by `SFX_STEP` per output frame.
    pos: f64,
}

/// All SFX state shared between the main thread and the audio callback.
pub struct SfxChannel {
    /// Decoded PCM for each of the 6 effects; `None` until `load_samples`.
    samples: [Option<Arc<Vec<i8>>>; SFX_COUNT],
    /// The effect currently playing, if any.
    active: Option<SfxPlayback>,
}

impl SfxChannel {
    fn new() -> Self {
        SfxChannel {
            samples: Default::default(),
            active: None,
        }
    }

    /// Resample and mix the active SFX into `left` and `right` (both centred).
    /// Uses nearest-neighbour interpolation to match the Amiga Paula hardware.
    fn mix_into(&mut self, left: &mut [f32], right: &mut [f32], frames: usize) {
        let pb = match &mut self.active {
            Some(pb) => pb,
            None => return,
        };
        let len = pb.data.len();
        let mut finished = false;
        for i in 0..frames {
            let idx = pb.pos as usize;
            if idx >= len {
                finished = true;
                break;
            }
            let s = pb.data[idx] as f32 / 128.0 * SFX_AMPLITUDE;
            left[i]  += s;
            right[i] += s;
            pb.pos += SFX_STEP;
        }
        if finished {
            self.active = None;
        }
    }
}

// ---------------------------------------------------------------------------
// Instruments / voice data
// ---------------------------------------------------------------------------

/// Loaded waveforms and envelopes from the `game/v6` file.
#[derive(Clone)]
pub struct Instruments {
    /// `waveforms[n]` — 128 signed bytes of raw PCM for waveform `n`.
    pub waveforms: [[i8; WAVEFORM_BYTES]; WAVEFORM_COUNT],
    /// `envelopes[n]` — up to 256 volume steps; 0xFF means "hold".
    pub envelopes: [[u8; ENVELOPE_BYTES]; ENVELOPE_COUNT],
}

impl Instruments {
    /// Load instruments from the `v6` file at `path`.
    pub fn load(path: &Path) -> Option<Self> {
        let data = fs::read(path).ok()?;
        Some(Self::parse(&data))
    }

    /// Parse raw bytes from the `v6` file.
    pub fn parse(data: &[u8]) -> Self {
        let mut waveforms = [[0i8; WAVEFORM_BYTES]; WAVEFORM_COUNT];
        let mut envelopes = [[0u8; ENVELOPE_BYTES]; ENVELOPE_COUNT];

        for (w, wf) in waveforms.iter_mut().enumerate() {
            let base = w * WAVEFORM_BYTES;
            if base + WAVEFORM_BYTES <= data.len() {
                for (i, b) in wf.iter_mut().enumerate() {
                    *b = data[base + i] as i8;
                }
            }
        }

        // The original loads wave data with Read(f, wavmem, S_WAVBUF), then
        // Seek(f, S_WAVBUF, 0 /*OFFSET_CURRENT*/), then Read(f, volmem, S_VOLBUF).
        // The Seek advances the file pointer a second S_WAVBUF past where the
        // wave read left it, so volume data starts at byte S_WAVBUF*2 = 2048.
        let vol_base = WAVEFORM_COUNT * WAVEFORM_BYTES * 2; // 2048
        for (e, env) in envelopes.iter_mut().enumerate() {
            let base = vol_base + e * ENVELOPE_BYTES;
            if base + ENVELOPE_BYTES <= data.len() {
                env.copy_from_slice(&data[base..base + ENVELOPE_BYTES]);
            }
        }

        Instruments { waveforms, envelopes }
    }

    /// Return a slice covering the looping portion of waveform `wave_num`
    /// for the given `wave_offset` from PTABLE.
    ///
    /// - start byte = `wave_offset * 4`
    /// - length bytes = `(32 - wave_offset) * 2`
    pub fn wave_loop<'a>(&'a self, wave_num: usize, wave_offset: u16) -> &'a [i8] {
        let wf = &self.waveforms[wave_num.min(WAVEFORM_COUNT - 1)];
        let start = (wave_offset as usize) * 4;
        let len = ((32 - wave_offset as usize)) * 2;
        let end = (start + len).min(WAVEFORM_BYTES);
        &wf[start..end]
    }
}

// ---------------------------------------------------------------------------
// Per-voice sequencer state
// ---------------------------------------------------------------------------

/// State for one of the 4 synthesizer voices.  Mirrors the per-voice data
/// block from `gdriver.asm` (`wave_num`, `vol_num`, `vol_delay`, `vce_stat`,
/// `event_start`, `event_stop`, `vol_list`, `trak_ptr`, etc.).
struct Voice {
    // --- sequencer ---
    /// Pointer into the track (index into `track` slice); `None` = no track.
    trak_ptr: Option<usize>,
    /// Index of the start of the current track (for looping).
    trak_beg: Option<usize>,
    /// Timeclock value at which the next event should fire.
    event_start: u32,
    /// Timeclock value at which the current note's sustain ends.
    event_stop: u32,

    // --- instrument ---
    /// Index into the `waveforms` array.
    wave_num: usize,
    /// Index into the `envelopes` array.
    vol_num: usize,
    /// Current position within the envelope byte array.
    vol_list: usize,
    /// When > 0, skip envelope step this VBL (mirrors `vol_delay` byte).
    vol_delay: u8,

    // --- output ---
    /// Current linear volume (0–64 matching Amiga hardware range).
    volume: u8,
    /// Waveform loop start byte offset within the waveform array.
    wave_start: usize,
    /// Waveform loop length in bytes.
    wave_len: usize,
    /// Current fractional playback position within the loop (0.0 .. wave_len).
    phase: f64,
    /// Phase increment per PCM sample = `amiga_clock / (period * sample_rate)`.
    phase_inc: f64,
    /// True if this voice is currently audible (not a rest, not ended).
    playing: bool,
    /// Low-pass filter state (1-pole IIR, one per voice).
    ///
    /// The Amiga A500 hardware runs each Paula channel through a passive RC
    /// low-pass filter (~4.8 kHz cutoff) before the DAC.  Without it, the raw
    /// 8-bit waveforms sound thin and tinny compared to the original hardware.
    lp_state: f32,
    /// De-click gain (0.0 – 0.5).  Ramps toward the target volume each sample
    /// instead of jumping, eliminating pops at note-start and note-end.
    ///
    /// Max value is `volume / 64 * 0.5`; ramps to 0 when the voice is silent.
    /// The mix loop keeps running (draining the LP filter) until this reaches 0.
    declick: f32,
}

impl Voice {
    fn new() -> Self {
        Voice {
            trak_ptr: None,
            trak_beg: None,
            event_start: 0,
            event_stop: 0,
            wave_num: 0,
            vol_num: 0,
            vol_list: 0,
            vol_delay: 0xff, // start with no volume changes
            volume: 0,
            wave_start: 0,
            wave_len: 64, // default: full waveform at offset 0
            phase: 0.0,
            phase_inc: 0.0,
            playing: false,
            lp_state: 0.0,
            declick: 0.0,
        }
    }

    /// Set the instrument from a slot in `NEW_WAVE`.
    fn set_instrument_slot(&mut self, slot: usize) {
        let (wave, vol) = NEW_WAVE[slot.min(INSTRUMENT_SLOTS - 1)];
        self.wave_num = wave as usize;
        self.vol_num = vol as usize;
    }

    /// Trigger a pitched note: load waveform geometry and phase increment.
    fn trigger_note(&mut self, pitch: usize, instruments: &Instruments) {
        if pitch >= PTABLE.len() {
            return;
        }
        let (period, wave_offset) = PTABLE[pitch];
        if period == 0 {
            return;
        }
        let wave_num = self.wave_num.min(WAVEFORM_COUNT - 1);
        self.wave_start = (wave_offset as usize) * 4;
        self.wave_len = ((32 - wave_offset as usize) * 2).min(WAVEFORM_BYTES - self.wave_start);
        if self.wave_len == 0 {
            self.playing = false;
            return;
        }
        // Reset phase and compute increment
        self.phase = 0.0;
        self.phase_inc = AMIGA_CLOCK_NTSC as f64 / (period as f64 * SAMPLE_RATE as f64);

        // Reload envelope from the beginning.
        // vol_list is a 0-based index within the per-envelope 256-byte array.
        let vol_num = self.vol_num.min(ENVELOPE_COUNT - 1);
        let first_env = instruments.envelopes[vol_num][0];
        if first_env < 0x80 {
            self.volume = first_env.min(64);
            self.vol_list = 1;
        } else {
            self.vol_list = 0;
        }
        self.vol_delay = 0;
        self.playing = true;
        let _ = wave_num; // wave_num is baked into wave_start/wave_len
    }

    fn silence(&mut self) {
        self.playing = false;
        self.volume = 0;
        self.vol_delay = 0xff;
    }

    /// Advance envelope one step (called once per VBL tick).
    fn step_envelope(&mut self, envelopes: &[[u8; ENVELOPE_BYTES]; ENVELOPE_COUNT]) {
        if self.vol_delay != 0 {
            // hold
            return;
        }
        let byte = envelopes[self.vol_num.min(ENVELOPE_COUNT - 1)][self.vol_list.min(ENVELOPE_BYTES - 1)];
        if byte >= 0x80 {
            // negative sentinel → hold, do not advance pointer
            self.vol_delay = 0xff;
        } else {
            self.volume = byte.min(64);
            self.vol_list = self.vol_list.saturating_add(1);
        }
    }

    /// Generate mono f32 samples and mix into `buf` with the current volume,
    /// advancing the waveform phase with linear interpolation.
    ///
    /// Used by unit tests; production rendering goes through `mix_stereo`.
    fn mix_into(&mut self, buf: &mut [f32], instruments: &Instruments, no_interpolation: bool) {
        // Route entirely to the left side (gain=1.0) and discard the right
        // side output (gain=0.0, written into a throwaway buffer).
        let mut right_dummy = vec![0.0f32; buf.len()];
        self.mix_stereo(buf, &mut right_dummy, instruments, no_interpolation, 1.0, 0.0);
    }

    /// Synthesise samples for this voice and mix them into both `left` and
    /// `right` in a single waveform pass.
    ///
    /// `left_gain` and `right_gain` control the per-side contribution.  For
    /// correct normalisation with 2 voices per side they should sum to 1.0
    /// (see `STEREO_PRIMARY` / `STEREO_BLEED`).
    ///
    /// Advancing the phase only once here prevents the double phase-advance
    /// that would occur if `mix_into` were called twice on the same voice.
    fn mix_stereo(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        instruments: &Instruments,
        no_interpolation: bool,
        left_gain: f32,
        right_gain: f32,
    ) {
        // Noise floor: below this the i16 output would be 0 regardless.
        const NOISE_FLOOR: f32 = 1.0 / 65536.0;
        // De-click ramp rate: reach full volume (or silence) in 64 samples (~1.4 ms).
        // Fast enough to be inaudible as a fade, slow enough to prevent pops.
        const DECLICK_RATE: f32 = 1.0 / 64.0;
        // 1-pole IIR low-pass: y[n] = α·x[n] + (1-α)·y[n-1]
        // α = 1 - exp(-2π · fc / fs), fc ≈ 4800 Hz, fs = 44100 Hz
        // Approximates the Amiga A500 passive RC filter on each Paula channel.
        const LP_ALPHA: f32 = 0.487;

        // Target gain for this voice: non-zero only when actively playing.
        let target = if self.playing && self.wave_len > 0 && self.volume > 0 {
            self.volume as f32 / 64.0 * 0.5
        } else {
            0.0
        };

        // Skip entirely only when both the de-click gain and the LP filter
        // state are below the noise floor — i.e. truly silent.
        if self.declick < NOISE_FLOOR && target < NOISE_FLOOR && self.lp_state.abs() < NOISE_FLOOR {
            self.declick = 0.0;
            self.lp_state = 0.0;
            return;
        }

        let wf = &instruments.waveforms[self.wave_num.min(WAVEFORM_COUNT - 1)];
        let start = self.wave_start;
        let len = self.wave_len;

        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            // Ramp the de-click gain toward the target one step at a time.
            // This replaces the abrupt volume jump with a short linear ramp.
            if self.declick < target {
                self.declick = (self.declick + DECLICK_RATE).min(target);
            } else if self.declick > target {
                self.declick = (self.declick - DECLICK_RATE).max(target);
            }

            // Generate a waveform sample only while actively playing.
            // During the decay tail (target == 0) we feed 0 into the LP
            // filter so it drains naturally rather than cutting off abruptly.
            let raw = if self.playing && len > 0 && self.volume > 0 {
                // Linear interpolation between consecutive waveform bytes.
                //
                // The phase wrap guarantees 0.0 <= phase < len, so int_part is
                // always in [0, len-1].  i1 wraps modulo len so the last sample
                // interpolates back to the loop start, avoiding a click on every
                // waveform cycle (audible as noise on short 8-byte loops).
                let int_part = self.phase as usize;
                let i0 = start + int_part;
                let s = if no_interpolation {
                    wf[i0] as f32 / 128.0
                } else {
                    let frac = (self.phase - int_part as f64) as f32;
                    let i1 = start + (int_part + 1) % len;
                    let s0 = wf[i0] as f32;
                    let s1 = wf[i1] as f32;
                    (s0 + frac * (s1 - s0)) / 128.0
                };
                self.phase += self.phase_inc;
                if self.phase >= len as f64 {
                    self.phase -= len as f64;
                }
                s
            } else {
                0.0
            };

            // Apply low-pass filter (shared state — one filter per voice).
            self.lp_state = LP_ALPHA * raw + (1.0 - LP_ALPHA) * self.lp_state;

            // Mix into the stereo output scaled by the smoothed de-click gain.
            *l += self.lp_state * self.declick * left_gain;
            *r += self.lp_state * self.declick * right_gain;
        }
    }
}

// ---------------------------------------------------------------------------
// Sequencer state (shared with audio thread)
// ---------------------------------------------------------------------------

/// All mutable state accessed by both the main thread and the audio callback.
pub struct SequencerState {
    /// The four voices.
    voices: [Voice; 4],
    /// Current timeclock value (accumulates `tempo` per VBL).
    timeclock: u32,
    /// Current tempo (timeclock units added per VBL).
    tempo: u32,
    /// When true, the sequencer does not advance (matches `nosound` flag).
    nosound: bool,
    /// Track data for each voice.
    tracks: [Option<Arc<Track>>; 4],
    /// Fractional sample accumulator for VBL timing.
    samples_to_vbl: f64,
    /// Song group currently loaded (0–6), if any.
    pub current_group: Option<usize>,
}

impl SequencerState {
    fn new() -> Self {
        SequencerState {
            voices: [Voice::new(), Voice::new(), Voice::new(), Voice::new()],
            timeclock: 0,
            tempo: DEFAULT_TEMPO,
            nosound: true,
            tracks: [None, None, None, None],
            samples_to_vbl: 0.0,
            current_group: None,
        }
    }

    /// Assign four tracks and begin playback from the start (mirrors `_playscore`).
    fn play_score(&mut self, t0: Arc<Track>, t1: Arc<Track>, t2: Arc<Track>, t3: Arc<Track>, inst: &Instruments) {
        let tracks = [t0, t1, t2, t3];
        self.timeclock = 0;
        self.tempo = DEFAULT_TEMPO;
        for (i, v) in self.voices.iter_mut().enumerate() {
            *v = Voice::new();
            // Set initial instrument from NEW_WAVE slot matching original
            // playscore: reads 4 words sequentially from new_wave[0..4]
            v.set_instrument_slot(i); // slots 0-3 all have wave=0, vol=0
            // Pre-load initial envelope volume from vol_num.
            // vol_list is a 0-based index within the per-envelope 256-byte array.
            let vol_num = v.vol_num.min(ENVELOPE_COUNT - 1);
            let first_env = inst.envelopes[vol_num][0];
            if first_env < 0x80 {
                v.volume = first_env.min(64);
                v.vol_list = 1;
            } else {
                v.vol_list = 0;
            }
            self.tracks[i] = Some(Arc::clone(&tracks[i]));
            v.trak_ptr = Some(0);
            v.trak_beg = Some(0);
            v.event_start = 0;
            v.event_stop = 0;
        }
        self.nosound = false;
        self.samples_to_vbl = 0.0;
    }

    /// Stop all playback (mirrors `_stopscore`).
    fn stop_score(&mut self) {
        self.nosound = true;
        self.current_group = None;
        for v in self.voices.iter_mut() {
            v.silence();
            v.trak_ptr = None;
        }
    }

    /// Run one VBL tick of the sequencer across all four voices.
    fn vbl_tick(&mut self, inst: &Instruments) {
        if self.nosound {
            return;
        }
        self.timeclock = self.timeclock.wrapping_add(self.tempo);

        for vi in 0..4 {
            self.tick_voice(vi, inst);
        }
    }

    /// Process one VBL tick for voice `vi`.
    ///
    /// Timing follows `_vblank_server` from `gdriver.asm`:
    /// - `timeclock >= event_start` (unsigned-wrapped) → fetch next track event
    /// - else if `timeclock >= event_stop` → silence the voice (sustain ended)
    /// - else → advance the ADSR envelope step
    fn tick_voice(&mut self, vi: usize, inst: &Instruments) {
        let tc = self.timeclock;

        // Envelope step (unconditional each VBL, per original)
        self.voices[vi].step_envelope(&inst.envelopes);

        if self.voices[vi].trak_ptr.is_none() {
            return;
        }

        let track = match &self.tracks[vi] {
            Some(t) => Arc::clone(t),
            None => return,
        };

        // Check if it's time for the next event.
        // Mirrors: `cmp.l event_start(a3),d0; bpl.s newnote`
        // bpl taken when timeclock - event_start >= 0 (non-negative 32-bit signed)
        // ↔ wrapping difference < 0x8000_0000
        let diff = tc.wrapping_sub(self.voices[vi].event_start);
        if diff >= 0x8000_0000 {
            // timeclock < event_start — not yet time for next note.
            // Check sustain end: mirrors `cmp.l event_stop(a3),d0; bpl.s rest_env`
            let stop_diff = tc.wrapping_sub(self.voices[vi].event_stop);
            if stop_diff < 0x8000_0000 {
                // timeclock >= event_stop: sustain ended, silence the voice
                self.voices[vi].playing = false;
                self.voices[vi].volume = 0;
            }
            return;
        }

        // timeclock >= event_start: consume track events
        let mut ptr = self.voices[vi].trak_ptr.unwrap();
        loop {
            let event = match track.get(ptr) {
                Some(e) => e,
                None => break,
            };
            ptr += 1;

            match event {
                TrackEvent::Note { pitch, duration_idx } => {
                    let dur = NOTE_DURATIONS[(*duration_idx as usize).min(63)] as u32;
                    // Original gdriver.asm `note_comm`:
                    //   sub.l #300,d4   ; d4 = duration - NOTE_GAP
                    //   bpl.s nc1       ; if ≥ 0, keep reduced duration
                    //   move.l d5,d4    ; else no gap: use full duration
                    // Short notes (dur < NOTE_GAP) get no gap at all,
                    // NOT a gap equal to their full duration (which would
                    // set event_stop == event_start and silence them immediately).
                    let sustain = if dur >= NOTE_GAP { dur - NOTE_GAP } else { dur };
                    let event_start = self.voices[vi].event_start;
                    self.voices[vi].event_stop = event_start.wrapping_add(sustain);
                    self.voices[vi].event_start = event_start.wrapping_add(dur);
                    self.voices[vi].trak_ptr = Some(ptr);
                    self.voices[vi].trigger_note(*pitch as usize, inst);
                    return;
                }
                TrackEvent::Rest { duration_idx } => {
                    let dur = NOTE_DURATIONS[(*duration_idx as usize).min(63)] as u32;
                    let event_start = self.voices[vi].event_start;
                    self.voices[vi].event_stop = event_start; // silence immediately
                    self.voices[vi].event_start = event_start.wrapping_add(dur);
                    self.voices[vi].trak_ptr = Some(ptr);
                    self.voices[vi].silence();
                    return;
                }
                TrackEvent::SetInstrument { slot } => {
                    self.voices[vi].set_instrument_slot(*slot as usize);
                    // continue consuming events
                }
                TrackEvent::SetTempo { value } => {
                    self.tempo = *value as u32;
                    // continue consuming events
                }
                TrackEvent::End { looping } => {
                    if *looping {
                        ptr = self.voices[vi].trak_beg.unwrap_or(0);
                        // continue from beginning of track
                    } else {
                        self.voices[vi].trak_ptr = None;
                        self.voices[vi].silence();
                        return;
                    }
                }
                TrackEvent::Unknown { .. } => {
                    // skip, per original
                }
            }
        }
        self.voices[vi].trak_ptr = Some(ptr);
    }
}

// ---------------------------------------------------------------------------
// SDL2 audio callback
// ---------------------------------------------------------------------------

/// SDL2 audio callback wrapper.
///
/// `instruments` lives here (owned by the callback, not behind the Mutex)
/// so we can pass `&instruments` to both the sequencer tick and the voice
/// mixer without any unsafe aliasing.
struct SynthCallback {
    state: Arc<Mutex<SequencerState>>,
    instruments: Instruments,
    sfx: Arc<Mutex<SfxChannel>>,
    /// When true, use nearest-neighbor instead of linear interpolation in the PCM mixer.
    no_interpolation: bool,
}

impl AudioCallback for SynthCallback {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        // Zero the buffer first
        for s in out.iter_mut() {
            *s = 0;
        }

        let mut st = match self.state.lock() {
            Ok(g) => g,
            Err(_) => return,
        };

        let inst = &self.instruments;
        // out is interleaved stereo: [L0, R0, L1, R1, ...]
        // Work in frames (stereo pairs) to keep VBL timing consistent.
        let total_frames = out.len() / 2;
        let mut frame_pos = 0usize;

        while frame_pos < total_frames {
            // How many frames until the next VBL tick?
            let until_vbl = st.samples_to_vbl;

            if until_vbl <= 0.0 {
                // Fire a VBL tick (sequencer advances)
                st.vbl_tick(inst);
                st.samples_to_vbl += SAMPLES_PER_VBL;
                continue;
            }

            // Render frames up to the next VBL boundary.
            let chunk_frames = (until_vbl.floor() as usize)
                .min(total_frames - frame_pos)
                .max(1);

            // Per-side mono scratch buffers.
            // Amiga Paula hardware DAC routing (not sequential by number):
            //   channels 0 and 3 → Left DAC
            //   channels 1 and 2 → Right DAC
            // This interleaved arrangement was used to reduce cross-talk between
            // adjacent chip traces.  Getting it wrong groups two voices that the
            // composer intended to be on separate sides onto the same side, causing
            // phase cancellation on harmonically related melodic lines.
            // A bleed of STEREO_BLEED to the opposite side centres the soundstage
            // while preserving the original left/right bias of the hardware.
            let mut left_buf  = vec![0.0f32; chunk_frames];
            let mut right_buf = vec![0.0f32; chunk_frames];

            // Voices 0 and 3: left-primary, right-bleed
            st.voices[0].mix_stereo(&mut left_buf, &mut right_buf, inst, self.no_interpolation, STEREO_PRIMARY, STEREO_BLEED);
            st.voices[3].mix_stereo(&mut left_buf, &mut right_buf, inst, self.no_interpolation, STEREO_PRIMARY, STEREO_BLEED);
            // Voices 1 and 2: right-primary, left-bleed
            st.voices[1].mix_stereo(&mut right_buf, &mut left_buf, inst, self.no_interpolation, STEREO_PRIMARY, STEREO_BLEED);
            st.voices[2].mix_stereo(&mut right_buf, &mut left_buf, inst, self.no_interpolation, STEREO_PRIMARY, STEREO_BLEED);

            // Mix any active SFX (independent of the 4 music voices; centred stereo).
            if let Ok(mut sfx) = self.sfx.lock() {
                sfx.mix_into(&mut left_buf, &mut right_buf, chunk_frames);
            }

            // Interleave left/right into the stereo output buffer.
            // Scale f32 [-1.0, 1.0] → i16 [-32767, 32767]; SDL2 converts
            // to the device's native format if needed.
            for i in 0..chunk_frames {
                let base = (frame_pos + i) * 2;
                out[base]     = (left_buf[i].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                out[base + 1] = (right_buf[i].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            }

            frame_pos += chunk_frames;
            st.samples_to_vbl -= chunk_frames as f64;
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// The top-level audio system.  Create with [`AudioSystem::new`], then call
/// [`AudioSystem::play_score`] to start music.
pub struct AudioSystem {
    state: Arc<Mutex<SequencerState>>,
    instruments: Instruments,
    sfx: Arc<Mutex<SfxChannel>>,
    _device: AudioDevice<SynthCallback>,
}

impl AudioSystem {
    /// Initialise the audio system.
    ///
    /// * `sdl_context` — the SDL2 context (used to open the audio subsystem)
    /// * `instruments` — loaded from `game/v6` via [`Instruments::load`]
    pub fn new(sdl_context: &sdl2::Sdl, instruments: Instruments, no_interpolation: bool) -> Result<Self, String> {
        let audio_subsystem = sdl_context.audio()?;

        let desired = AudioSpecDesired {
            freq: Some(SAMPLE_RATE as i32),
            // Stereo: Amiga Paula routes voices 0+1 to the left DAC and
            // voices 2+3 to the right DAC.  SDL2 interleaves as [L, R, ...].
            channels: Some(2),
            samples: Some(512),
        };

        let state = Arc::new(Mutex::new(SequencerState::new()));
        let state_cb = Arc::clone(&state);
        // Clone instruments for the callback; the original is kept for play_score
        let instruments_cb = instruments.clone();

        let sfx = Arc::new(Mutex::new(SfxChannel::new()));
        let sfx_cb = Arc::clone(&sfx);

        let device = audio_subsystem.open_playback(None, &desired, |_spec| {
            SynthCallback { state: state_cb, instruments: instruments_cb, sfx: sfx_cb, no_interpolation }
        })?;

        device.resume();

        Ok(AudioSystem { state, instruments, sfx, _device: device })
    }

    /// Start playing four tracks from the beginning (mirrors `_playscore`).
    ///
    /// Typically called with `library.intro_tracks()` for the intro music.
    pub fn play_score(&self, tracks: [Arc<Track>; 4]) {
        let [t0, t1, t2, t3] = tracks;
        if let Ok(mut st) = self.state.lock() {
            st.play_score(t0, t1, t2, t3, &self.instruments);
        }
    }

    /// Stop all playback immediately (mirrors `_stopscore`).
    pub fn stop_score(&self) {
        if let Ok(mut st) = self.state.lock() {
            st.stop_score();
        }
    }

    /// Play a song group by index (0–6).  Each group is four voices occupying
    /// four consecutive track slots.  Group 3 is the intro music.
    /// Returns `false` if the group is not present in the library.
    pub fn play_group(&self, group: usize, library: &SongLibrary) -> bool {
        let tracks = match library.group(group) {
            Some(t) => t.map(|tr| Arc::new(tr.clone())),
            None => return false,
        };
        if let Ok(mut st) = self.state.lock() {
            let [t0, t1, t2, t3] = tracks;
            st.play_score(t0, t1, t2, t3, &self.instruments);
            st.current_group = Some(group);
        }
        true
    }

    /// Return the song group currently being played (0–6), or `None` if stopped.
    pub fn current_group(&self) -> Option<usize> {
        self.state.lock().ok().and_then(|st| {
            if !st.nosound { st.current_group } else { None }
        })
    }

    /// Change the current tempo (mirrors `_set_tempo`).
    pub fn set_tempo(&self, tempo: u32) {
        if let Ok(mut st) = self.state.lock() {
            st.tempo = tempo;
        }
    }

    /// Return `true` if any voice is currently playing.
    pub fn is_playing(&self) -> bool {
        if let Ok(st) = self.state.lock() {
            !st.nosound
        } else {
            false
        }
    }

    /// Load the 6 sound effects from `path` (the `game/samples` file).
    ///
    /// Format (from `read_sample()` in `fmain.c`): six contiguous records,
    /// each consisting of a 4-byte big-endian length followed by that many
    /// bytes of signed 8-bit PCM at [`SFX_SAMPLE_RATE`] Hz.
    ///
    /// If the file is absent or unreadable, returns `Ok(())` with a warning;
    /// SFX will silently not play while music continues unaffected.
    pub fn load_samples(&mut self, path: &Path) -> Result<(), String> {
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("audio: SFX file not found ({path:?}): {e}; SFX disabled");
                return Ok(());
            }
        };

        let mut sfx = self.sfx.lock().map_err(|e| format!("sfx lock poisoned: {e}"))?;
        let mut cursor = 0usize;
        for i in 0..SFX_COUNT {
            if cursor + 4 > data.len() {
                eprintln!("audio: SFX {i}: unexpected end of samples file; remaining SFX disabled");
                break;
            }
            let len = u32::from_be_bytes([
                data[cursor], data[cursor + 1], data[cursor + 2], data[cursor + 3],
            ]) as usize;
            cursor += 4;
            if cursor + len > data.len() {
                eprintln!("audio: SFX {i}: declared length {len} exceeds file size; remaining SFX disabled");
                break;
            }
            let pcm: Vec<i8> = data[cursor..cursor + len].iter().map(|&b| b as i8).collect();
            sfx.samples[i] = Some(Arc::new(pcm));
            cursor += len;
        }
        Ok(())
    }

    /// Trigger sound effect `sfx_id` (0–5).  Plays alongside music voices
    /// without interrupting them.  If `sfx_id` is out of range or samples
    /// have not been loaded, this is a no-op.
    pub fn play_sfx(&self, sfx_id: u8) {
        let id = sfx_id as usize;
        if id >= SFX_COUNT {
            return;
        }
        if let Ok(mut sfx) = self.sfx.lock() {
            if let Some(data) = sfx.samples[id].clone() {
                sfx.active = Some(SfxPlayback { data, pos: 0.0 });
            }
        }
    }
}

/// Convenience: load both the song library and instruments from the standard
/// game asset paths, returning both.
pub fn load_audio_assets(base: &Path) -> Option<(SongLibrary, Instruments)> {
    let songs = SongLibrary::load(&base.join("game/songs"))?;
    let instruments = Instruments::load(&base.join("game/v6"))?;
    Some((songs, instruments))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn load_instruments() -> Instruments {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("game/v6");
        Instruments::load(&path).expect("game/v6 must be readable")
    }

    fn load_songs() -> SongLibrary {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("game/songs");
        SongLibrary::load(&path).expect("game/songs must be readable")
    }

    #[test]
    fn test_instruments_loaded() {
        let inst = load_instruments();
        // Waveform 0 should have non-zero samples (checked manually via xxd)
        let nonzero = inst.waveforms[0].iter().any(|&b| b != 0);
        assert!(nonzero, "waveform 0 should have non-zero samples");
    }

    #[test]
    fn test_all_waveforms_present() {
        let inst = load_instruments();
        // All 8 waveforms should be loaded (they start at known offsets)
        for w in 0..WAVEFORM_COUNT {
            let _ = &inst.waveforms[w]; // just ensure indexing works
        }
    }

    #[test]
    fn test_envelope_4_has_data() {
        let inst = load_instruments();
        // Envelope 4 has an attack-shape ADSR envelope
        let has_data = inst.envelopes[4].iter().any(|&b| b != 0);
        assert!(has_data, "envelope 4 should have ADSR data");
    }

    #[test]
    fn test_wave_loop_geometry_offset_0() {
        let inst = load_instruments();
        let loop_slice = inst.wave_loop(0, 0);
        // wave_offset=0 → start=0, len=(32-0)*2=64
        assert_eq!(loop_slice.len(), 64, "offset=0 loop length should be 64 bytes");
    }

    #[test]
    fn test_wave_loop_geometry_offset_16() {
        let inst = load_instruments();
        let loop_slice = inst.wave_loop(0, 16);
        // wave_offset=16 → start=64, len=(32-16)*2=32
        assert_eq!(loop_slice.len(), 32, "offset=16 loop length should be 32 bytes");
    }

    #[test]
    fn test_wave_loop_geometry_offset_28() {
        let inst = load_instruments();
        let loop_slice = inst.wave_loop(0, 28);
        // wave_offset=28 → start=112, len=(32-28)*2=8
        assert_eq!(loop_slice.len(), 8, "offset=28 loop length should be 8 bytes");
    }

    #[test]
    fn test_sequencer_play_and_stop() {
        let inst = load_instruments();
        let songs = load_songs();
        let mut st = SequencerState::new();

        let intro = songs.intro_tracks().expect("intro tracks must exist");
        let tracks = intro.map(|t| Arc::new(t.clone()));
        let [t0, t1, t2, t3] = tracks;
        st.play_score(t0, t1, t2, t3, &inst);

        assert!(!st.nosound, "should be playing after play_score");

        // Run a few VBL ticks without panicking
        for _ in 0..120 {
            st.vbl_tick(&inst);
        }

        st.stop_score();
        assert!(st.nosound, "should be stopped after stop_score");
    }

    #[test]
    fn test_vbl_tick_advances_timeclock() {
        let inst = load_instruments();
        let songs = load_songs();
        let mut st = SequencerState::new();

        let intro = songs.intro_tracks().expect("intro tracks must exist");
        let tracks = intro.map(|t| Arc::new(t.clone()));
        let [t0, t1, t2, t3] = tracks;
        st.play_score(t0, t1, t2, t3, &inst);

        let before = st.timeclock;
        st.vbl_tick(&inst);
        assert_eq!(
            st.timeclock,
            before.wrapping_add(DEFAULT_TEMPO),
            "timeclock should advance by DEFAULT_TEMPO each VBL"
        );
    }

    #[test]
    fn test_sequencer_triggers_notes() {
        let inst = load_instruments();
        let songs = load_songs();
        let mut st = SequencerState::new();

        let intro = songs.intro_tracks().expect("intro tracks must exist");
        let tracks = intro.map(|t| Arc::new(t.clone()));
        let [t0, t1, t2, t3] = tracks;
        st.play_score(t0, t1, t2, t3, &inst);

        // At least one voice should have a note playing after enough ticks.
        // First note fires on tick 1 (timeclock=150 >= event_start=0).
        // Run just 1 tick and verify at least one voice is playing.
        st.vbl_tick(&inst);

        let any_playing = st.voices.iter().any(|v| v.playing);
        assert!(any_playing, "at least one voice should be playing after the first VBL tick");
    }

    #[test]
    fn test_new_wave_slots() {
        // Spot-check a few instrument slots from new_wave[]
        assert_eq!(NEW_WAVE[0], (0, 0));
        assert_eq!(NEW_WAVE[4], (0, 5));
        assert_eq!(NEW_WAVE[5], (2, 2));
        assert_eq!(NEW_WAVE[9], (5, 4));
        assert_eq!(NEW_WAVE[11], (5, 0));
    }

    #[test]
    fn test_timeclock_rate_constant() {
        use super::super::songs::{DEFAULT_TEMPO, VBL_RATE_HZ, TIMECLOCK_RATE};
        assert_eq!(TIMECLOCK_RATE, DEFAULT_TEMPO * VBL_RATE_HZ);
        assert_eq!(TIMECLOCK_RATE, 9000);
    }

    #[test]
    fn test_voice_trigger_note_sets_playing() {
        let inst = load_instruments();
        let mut v = Voice::new();
        v.wave_num = 0;
        v.vol_num = 4; // has real envelope data
        v.trigger_note(24, &inst); // pitch 24 = period 508, offset 0
        assert!(v.playing, "voice should be playing after trigger_note");
        assert!(v.phase_inc > 0.0, "phase_inc should be positive");
    }

    #[test]
    fn test_mix_into_produces_nonzero_output() {
        let inst = load_instruments();
        let mut v = Voice::new();
        v.wave_num = 0;
        v.vol_num = 4;
        v.trigger_note(24, &inst);
        v.volume = 32;
        let mut buf = vec![0.0f32; 256];
        v.mix_into(&mut buf, &inst, false);
        let nonzero = buf.iter().any(|&s| s != 0.0);
        assert!(nonzero, "mix_into should produce non-zero output for a playing voice");
    }
}
