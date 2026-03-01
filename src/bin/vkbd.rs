//! Virtual keyboard for the Faery Tale Adventure audio engine.
//!
//! Plays notes interactively using the game's instrument waveforms and ADSR
//! envelopes, rendered in the terminal via crossterm.  Supports PTABLE live
//! editing and sine wave instrument mode for frequency tuning.
//!
//! Usage:
//!   cargo run --bin vkbd
//!
//! Controls:
//!   ASDF… row   — white keys (C D E F G A B …)
//!   QWERTY row  — black keys (C# D# F# G# A# …)
//!   Shift+key   — latch/unlatch note (sustains after release)
//!   ←/→         — re-pitch latched note ±1 semitone
//!   ↑/↓         — jump to nearest black/white key
//!   Z / X       — octave down / up
//!   1–9         — select instrument slot directly
//!   [ / ]       — cycle instrument slot down / up
//!   Tab         — toggle sine wave mode
//!   F2/F3 +/-   — adjust harmonic 2/3 amplitude (sine mode)
//!   KP2 / KP8  — fine-tune period ±1 (Shift = ±10)
//!   Q / Esc     — quit (prints modified PTABLE if dirty)

// ---------------------------------------------------------------------------
// Bring in the songs module directly (no intra-crate deps).
// ---------------------------------------------------------------------------
#[allow(dead_code, unused_imports)]
#[path = "../game/songs.rs"]
mod songs;

use songs::{AMIGA_CLOCK_NTSC, PTABLE};

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossterm::{
    cursor,
    event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
        KeyboardEnhancementFlags, PushKeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    },
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};

use sdl2::audio::{AudioCallback, AudioSpecDesired};

// ---------------------------------------------------------------------------
// Audio constants
// ---------------------------------------------------------------------------

const SAMPLE_RATE: u32 = 44_100;
const WAVEFORM_COUNT: usize = 8;
const WAVEFORM_BYTES: usize = 128;
const ENVELOPE_COUNT: usize = 10;
const ENVELOPE_BYTES: usize = 256;
const STEREO_PRIMARY: f32 = 0.75;
const STEREO_BLEED: f32 = 0.25;

/// VBL rate for envelope stepping (60 Hz NTSC).
const VBL_RATE: u32 = 60;
/// Samples per VBL tick (~735 at 44100 Hz / 60 Hz).
const SAMPLES_PER_VBL: f64 = SAMPLE_RATE as f64 / VBL_RATE as f64;

/// Volume units to subtract per VBL tick during release phase.
/// At 60 Hz, a release from vol 64 takes 64/4 = 16 ticks ≈ 267 ms.
const RELEASE_RATE: u8 = 4;

/// NEW_WAVE table: (wave_num, vol_num) for each instrument slot.
const NEW_WAVE: [(u8, u8); 12] = [
    (0, 0), (0, 0), (0, 0), (0, 0),
    (0, 5), (2, 2), (1, 1), (1, 3),
    (0, 4), (5, 4), (1, 0), (5, 0),
];

const INSTRUMENT_NAMES: [&str; 12] = [
    "Piano",       // slot 0
    "Piano",       // slot 1
    "Piano",       // slot 2
    "Piano",       // slot 3
    "Strings",     // slot 4
    "Brass",       // slot 5
    "Harpsichrd",  // slot 6
    "Woodwind",    // slot 7
    "Flute",       // slot 8
    "Organ",       // slot 9
    "Pluck",       // slot 10
    "Pad",         // slot 11
];

// ---------------------------------------------------------------------------
// Note names
// ---------------------------------------------------------------------------

const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

/// Convert a PTABLE pitch index to a note name string like "C#4".
fn pitch_to_name(pitch: u8) -> String {
    if (pitch as usize) >= PTABLE.len() {
        return "?".to_string();
    }
    let (period, wave_offset) = PTABLE[pitch as usize];
    if period == 0 {
        return "---".to_string();
    }
    let wave_len = (32 - wave_offset as usize) * 2;
    let freq = AMIGA_CLOCK_NTSC as f64 / (period as f64 * wave_len as f64);
    let midi = 69.0 + 12.0 * (freq / 440.0).log2();
    let midi_i = midi.round() as i32;
    let name = NOTE_NAMES[((midi_i % 12 + 12) % 12) as usize];
    let octave = midi_i / 12 - 1;
    format!("{}{}", name, octave)
}

// ---------------------------------------------------------------------------
// Keyboard → semitone mapping (DAW layout)
// ---------------------------------------------------------------------------

/// Map a keyboard char to a semitone offset (0–16) within the current octave.
/// Returns None if the key is not a piano key.
fn key_to_semitone(c: char) -> Option<u8> {
    match c {
        'a' => Some(0),   // C
        'w' => Some(1),   // C#
        's' => Some(2),   // D
        'e' => Some(3),   // D#
        'd' => Some(4),   // E
        'f' => Some(5),   // F
        't' => Some(6),   // F#
        'g' => Some(7),   // G
        'y' => Some(8),   // G#
        'h' => Some(9),   // A
        'u' => Some(10),  // A#
        'j' => Some(11),  // B
        'k' => Some(12),  // C+1
        'o' => Some(13),  // C#+1
        'l' => Some(14),  // D+1
        'p' => Some(15),  // D#+1
        ';' => Some(16),  // E+1
        _ => None,
    }
}

/// Reverse: given a semitone offset, what key character(s) map to it?
/// Returns the primary key char for display.
#[allow(dead_code)]
fn semitone_to_key(semi: u8) -> Option<char> {
    match semi {
        0  => Some('A'),
        1  => Some('W'),
        2  => Some('S'),
        3  => Some('E'),
        4  => Some('D'),
        5  => Some('F'),
        6  => Some('T'),
        7  => Some('G'),
        8  => Some('Y'),
        9  => Some('H'),
        10 => Some('U'),
        11 => Some('J'),
        12 => Some('K'),
        13 => Some('O'),
        14 => Some('L'),
        15 => Some('P'),
        16 => Some(';'),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Instruments
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Instruments {
    waveforms: [[i8; WAVEFORM_BYTES]; WAVEFORM_COUNT],
    envelopes: [[u8; ENVELOPE_BYTES]; ENVELOPE_COUNT],
}

impl Instruments {
    fn load(path: &Path) -> Option<Self> {
        let data = std::fs::read(path).ok()?;
        Some(Self::parse(&data))
    }

    fn parse(data: &[u8]) -> Self {
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
        let vol_base = WAVEFORM_COUNT * WAVEFORM_BYTES * 2;
        for (e, env) in envelopes.iter_mut().enumerate() {
            let base = vol_base + e * ENVELOPE_BYTES;
            if base + ENVELOPE_BYTES <= data.len() {
                env.copy_from_slice(&data[base..base + ENVELOPE_BYTES]);
            }
        }
        Instruments { waveforms, envelopes }
    }
}

// ---------------------------------------------------------------------------
// Voice (adapted for manual sustain)
// ---------------------------------------------------------------------------

struct Voice {
    wave_num: usize,
    vol_num: usize,
    vol_list: usize,
    vol_delay: u8,
    volume: u8,
    wave_start: usize,
    wave_len: usize,
    phase: f64,
    phase_inc: f64,
    playing: bool,
    lp_state: f32,
    declick: f32,
    /// When true, this voice sustains indefinitely (no envelope decay).
    manual: bool,
    /// When true, the voice is fading out after key release.
    releasing: bool,
    /// Which keyboard key triggered this voice (for display).
    key_char: Option<char>,
}

impl Voice {
    fn new() -> Self {
        Voice {
            wave_num: 0, vol_num: 0, vol_list: 0, vol_delay: 0xff,
            volume: 0, wave_start: 0, wave_len: 64,
            phase: 0.0, phase_inc: 0.0, playing: false,
            lp_state: 0.0, declick: 0.0, manual: false,
            releasing: false, key_char: None,
        }
    }

    fn set_instrument_slot(&mut self, slot: usize) {
        let (wave, vol) = NEW_WAVE[slot.min(NEW_WAVE.len() - 1)];
        self.wave_num = wave as usize;
        self.vol_num = vol as usize;
    }

    fn trigger_note(&mut self, pitch: usize, ptable: &[(u16, u16); 78], inst: &Instruments) {
        if pitch >= ptable.len() { return; }
        let (period, wave_offset) = ptable[pitch];
        if period == 0 { return; }
        self.wave_start = (wave_offset as usize) * 4;
        self.wave_len = ((32 - wave_offset as usize) * 2).min(WAVEFORM_BYTES - self.wave_start);
        if self.wave_len == 0 { self.playing = false; return; }
        self.phase = 0.0;
        self.phase_inc = AMIGA_CLOCK_NTSC as f64 / (period as f64 * SAMPLE_RATE as f64);
        self.releasing = false;
        if self.manual {
            // In manual mode, set a fixed volume and skip envelope
            self.volume = 48;
            self.vol_delay = 0xff;
        } else {
            let vol_num = self.vol_num.min(ENVELOPE_COUNT - 1);
            let first = inst.envelopes[vol_num][0];
            if first < 0x80 { self.volume = first.min(64); self.vol_list = 1; }
            else { self.vol_list = 0; }
            self.vol_delay = 0;
        }
        self.playing = true;
    }

    /// Re-pitch without resetting phase (for live PTABLE editing / arrow keys).
    fn retrigger_pitch(&mut self, pitch: usize, ptable: &[(u16, u16); 78]) {
        if pitch >= ptable.len() { return; }
        let (period, wave_offset) = ptable[pitch];
        if period == 0 { return; }
        self.wave_start = (wave_offset as usize) * 4;
        self.wave_len = ((32 - wave_offset as usize) * 2).min(WAVEFORM_BYTES - self.wave_start);
        if self.wave_len == 0 { self.playing = false; return; }
        self.phase_inc = AMIGA_CLOCK_NTSC as f64 / (period as f64 * SAMPLE_RATE as f64);
        // Keep phase, volume, playing state — seamless pitch change
        if self.phase >= self.wave_len as f64 {
            self.phase = 0.0; // reset if new loop is shorter
        }
    }

    fn silence(&mut self) {
        self.playing = false;
        self.volume = 0;
        self.vol_delay = 0xff;
        self.releasing = false;
        self.key_char = None;
    }

    /// Begin the release phase: volume fades out over several VBL ticks
    /// while the waveform keeps playing (smooth fade, no click).
    fn release(&mut self) {
        if !self.playing { return; }
        self.releasing = true;
    }

    fn step_envelope(&mut self, envelopes: &[[u8; ENVELOPE_BYTES]; ENVELOPE_COUNT]) {
        if self.manual { return; } // manual mode skips envelope

        // Release phase: fade volume down at a fixed rate
        if self.releasing {
            if self.volume > RELEASE_RATE {
                self.volume -= RELEASE_RATE;
            } else {
                self.silence();
            }
            return;
        }

        // Normal envelope stepping (attack/decay/sustain)
        if self.vol_delay != 0 { return; }
        let byte = envelopes[self.vol_num.min(ENVELOPE_COUNT - 1)][self.vol_list.min(ENVELOPE_BYTES - 1)];
        if byte >= 0x80 {
            self.vol_delay = 0xff; // hold (sustain)
        } else {
            self.volume = byte.min(64);
            if self.volume == 0 {
                self.silence(); // envelope decayed to zero — note is done
            } else {
                self.vol_list = self.vol_list.saturating_add(1);
            }
        }
    }

    fn mix_stereo(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        inst: &Instruments,
        left_gain: f32,
        right_gain: f32,
    ) {
        const NOISE_FLOOR: f32 = 1.0 / 65536.0;
        const DECLICK_RATE: f32 = 1.0 / 64.0;
        const LP_ALPHA: f32 = 0.487;

        let target = if self.playing && self.wave_len > 0 && self.volume > 0 {
            self.volume as f32 / 64.0 * 0.5
        } else {
            0.0
        };

        if self.declick < NOISE_FLOOR && target < NOISE_FLOOR && self.lp_state.abs() < NOISE_FLOOR {
            self.declick = 0.0;
            self.lp_state = 0.0;
            return;
        }

        let wf = &inst.waveforms[self.wave_num.min(WAVEFORM_COUNT - 1)];
        let start = self.wave_start;
        let len = self.wave_len;

        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if self.declick < target {
                self.declick = (self.declick + DECLICK_RATE).min(target);
            } else if self.declick > target {
                self.declick = (self.declick - DECLICK_RATE).max(target);
            }

            let raw = if self.playing && len > 0 && self.volume > 0 {
                let int_part = self.phase as usize;
                let i0 = start + int_part;
                let frac = (self.phase - int_part as f64) as f32;
                let i1 = start + (int_part + 1) % len;
                let s = (wf[i0] as f32 + frac * (wf[i1] as f32 - wf[i0] as f32)) / 128.0;
                self.phase += self.phase_inc;
                if self.phase >= len as f64 { self.phase -= len as f64; }
                s
            } else {
                0.0
            };

            self.lp_state = LP_ALPHA * raw + (1.0 - LP_ALPHA) * self.lp_state;
            *l += self.lp_state * self.declick * left_gain;
            *r += self.lp_state * self.declick * right_gain;
        }
    }
}

// ---------------------------------------------------------------------------
// Sine wave generation
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct SineConfig {
    harmonic2: f32,
    harmonic3: f32,
}

impl SineConfig {
    fn new() -> Self {
        SineConfig { harmonic2: 0.0, harmonic3: 0.0 }
    }
}

fn generate_sine_waveform(config: &SineConfig) -> [i8; 128] {
    let mut waveform = [0i8; 128];
    let mut raw = [0.0f64; 128];
    let mut max_abs: f64 = 0.0;
    for i in 0..128 {
        let t = i as f64 / 128.0 * std::f64::consts::TAU;
        let sample = t.sin()
            + config.harmonic2 as f64 * (2.0 * t).sin()
            + config.harmonic3 as f64 * (3.0 * t).sin();
        raw[i] = sample;
        if sample.abs() > max_abs {
            max_abs = sample.abs();
        }
    }
    if max_abs > 0.0 {
        for i in 0..128 {
            waveform[i] = (raw[i] / max_abs * 127.0).round() as i8;
        }
    }
    waveform
}

// ---------------------------------------------------------------------------
// ManualState (shared with audio callback)
// ---------------------------------------------------------------------------

struct ManualState {
    voices: [Voice; 4],
    instruments: Instruments,
    current_instrument: usize,
    sine_mode: bool,
    sine_config: SineConfig,
    /// Mutable copy of PTABLE for live editing.
    ptable: [(u16, u16); 78],
    /// Backup of original waveform when sine mode is active.
    original_waveform: Option<[i8; WAVEFORM_BYTES]>,
    /// Fractional sample accumulator for VBL-rate envelope stepping.
    samples_to_vbl: f64,
}

impl ManualState {
    fn new(instruments: Instruments) -> Self {
        let mut ptable = [(0u16, 0u16); 78];
        ptable.copy_from_slice(&PTABLE);
        ManualState {
            voices: [Voice::new(), Voice::new(), Voice::new(), Voice::new()],
            instruments,
            current_instrument: 0,
            sine_mode: false,
            sine_config: SineConfig::new(),
            ptable,
            original_waveform: None,
            samples_to_vbl: 0.0,
        }
    }

    /// Trigger a note on a voice.
    /// `latched`: if true, uses manual mode (fixed volume, no envelope).
    /// If false, uses the real ADSR envelope (attack → decay → sustain hold).
    fn trigger_voice(&mut self, voice_idx: usize, pitch: u8, latched: bool, key_char: char) {
        if voice_idx >= 4 { return; }
        let v = &mut self.voices[voice_idx];
        v.set_instrument_slot(self.current_instrument);
        v.manual = latched;
        v.key_char = Some(key_char);
        v.trigger_note(pitch as usize, &self.ptable, &self.instruments);
    }

    /// Immediate silence (for unlatching).
    fn release_voice(&mut self, voice_idx: usize) {
        if voice_idx >= 4 { return; }
        self.voices[voice_idx].silence(); // also clears key_char
    }

    /// Begin the release phase (smooth fade-out for non-latched keys).
    fn release_voice_soft(&mut self, voice_idx: usize) {
        if voice_idx >= 4 { return; }
        self.voices[voice_idx].release();
    }

    fn retrigger_voice(&mut self, voice_idx: usize, pitch: u8) {
        if voice_idx >= 4 { return; }
        self.voices[voice_idx].retrigger_pitch(pitch as usize, &self.ptable);
    }

    /// Toggle sine mode: swap current instrument's waveform with generated sine.
    fn toggle_sine(&mut self) {
        let (wave_num, _) = NEW_WAVE[self.current_instrument.min(NEW_WAVE.len() - 1)];
        let wn = wave_num as usize;

        if self.sine_mode {
            // Restore original waveform
            if let Some(orig) = self.original_waveform.take() {
                self.instruments.waveforms[wn] = orig;
            }
            self.sine_mode = false;
        } else {
            // Save original and replace with sine
            self.original_waveform = Some(self.instruments.waveforms[wn]);
            let sine = generate_sine_waveform(&self.sine_config);
            self.instruments.waveforms[wn] = sine;
            self.sine_mode = true;
        }
    }

    /// Regenerate sine waveform with updated config (while sine mode is active).
    fn update_sine(&mut self) {
        if !self.sine_mode { return; }
        let (wave_num, _) = NEW_WAVE[self.current_instrument.min(NEW_WAVE.len() - 1)];
        let wn = wave_num as usize;
        let sine = generate_sine_waveform(&self.sine_config);
        self.instruments.waveforms[wn] = sine;
    }

    /// Adjust the period for a given pitch index by delta. Returns new period.
    fn adjust_period(&mut self, pitch: usize, delta: i32) -> u16 {
        if pitch >= 78 { return 0; }
        let (period, wo) = self.ptable[pitch];
        let new_period = (period as i32 + delta).clamp(1, 65535) as u16;
        self.ptable[pitch] = (new_period, wo);

        // Update any voice currently playing this pitch
        for v in &mut self.voices {
            if v.playing {
                // Recalculate phase_inc for any voice (we can't easily track which
                // pitch a voice plays, so just update all — the caller tracks pitch)
                v.retrigger_pitch(pitch, &self.ptable);
            }
        }
        new_period
    }
}

// ---------------------------------------------------------------------------
// SDL2 audio callback
// ---------------------------------------------------------------------------

struct SynthCallback {
    state: Arc<Mutex<ManualState>>,
}

impl AudioCallback for SynthCallback {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        for s in out.iter_mut() { *s = 0; }
        let mut st = match self.state.lock() { Ok(g) => g, Err(_) => return };
        let inst = st.instruments.clone();
        let total_frames = out.len() / 2;
        let mut frame_pos = 0usize;

        while frame_pos < total_frames {
            // Step envelopes at VBL rate (~60 Hz)
            if st.samples_to_vbl <= 0.0 {
                for v in &mut st.voices {
                    v.step_envelope(&inst.envelopes);
                }
                st.samples_to_vbl += SAMPLES_PER_VBL;
                continue;
            }

            let chunk = (st.samples_to_vbl.floor() as usize)
                .min(total_frames - frame_pos)
                .max(1);

            let mut left_buf = vec![0.0f32; chunk];
            let mut right_buf = vec![0.0f32; chunk];

            // Mix all 4 voices with Paula stereo routing
            st.voices[0].mix_stereo(&mut left_buf, &mut right_buf, &inst, STEREO_PRIMARY, STEREO_BLEED);
            st.voices[3].mix_stereo(&mut left_buf, &mut right_buf, &inst, STEREO_PRIMARY, STEREO_BLEED);
            st.voices[1].mix_stereo(&mut right_buf, &mut left_buf, &inst, STEREO_PRIMARY, STEREO_BLEED);
            st.voices[2].mix_stereo(&mut right_buf, &mut left_buf, &inst, STEREO_PRIMARY, STEREO_BLEED);

            for i in 0..chunk {
                let base = (frame_pos + i) * 2;
                out[base]     = (left_buf[i].clamp(-1.0, 1.0)  * i16::MAX as f32) as i16;
                out[base + 1] = (right_buf[i].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            }

            frame_pos += chunk;
            st.samples_to_vbl -= chunk as f64;
        }
    }
}

// ---------------------------------------------------------------------------
// Key tracking state (lives on the main thread)
// ---------------------------------------------------------------------------

struct HeldKey {
    pitch: u8,
    voice_idx: usize,
    latched: bool,
}

struct KeyboardState {
    /// Map from piano key char → held key info
    held_keys: HashMap<char, HeldKey>,
    /// Next voice to assign (round-robin)
    next_voice: usize,
    /// Current base octave (0–6)
    base_octave: usize,
    /// Most recently latched key char (for arrow-key re-pitching)
    last_latched: Option<char>,
    /// Current instrument slot
    current_instrument: usize,
    /// Sine mode active
    sine_mode: bool,
    /// Sine harmonic config
    sine_config: SineConfig,
    /// Which harmonic is selected for editing (2 or 3)
    selected_harmonic: u8,
    /// PTABLE dirty flag (any modification made)
    ptable_dirty: bool,
    /// The pitch currently being tuned (most recently triggered/latched pitch)
    active_pitch: Option<u8>,
}

impl KeyboardState {
    fn new() -> Self {
        KeyboardState {
            held_keys: HashMap::new(),
            next_voice: 0,
            base_octave: 3,
            last_latched: None,
            current_instrument: 0,
            sine_mode: false,
            sine_config: SineConfig::new(),
            selected_harmonic: 2,
            ptable_dirty: false,
            active_pitch: None,
        }
    }

    fn pitch_for_key(&self, c: char) -> Option<u8> {
        key_to_semitone(c).map(|semi| {
            // Rows 1-6 of PTABLE start at A (pitch 6, 18, 30, ...).
            // Subtract 3 so the 'a' key (DAW semitone 0 = C) lands on C
            // rather than the A that begins each row.
            let pitch = self.base_octave as i32 * 12 + semi as i32 - 3;
            pitch.max(0).min(77) as u8
        })
    }

    fn find_free_voice(&mut self) -> usize {
        // Round-robin, preferring voices that aren't playing
        for i in 0..4 {
            let idx = (self.next_voice + i) % 4;
            let in_use = self.held_keys.values().any(|hk| hk.voice_idx == idx);
            if !in_use {
                self.next_voice = (idx + 1) % 4;
                return idx;
            }
        }
        // All in use — steal the next one
        let idx = self.next_voice;
        self.next_voice = (idx + 1) % 4;
        idx
    }
}

// ---------------------------------------------------------------------------
// Arrow key pitch navigation helpers
// ---------------------------------------------------------------------------

/// Whether a PTABLE pitch index is a black key (sharp/flat).
///
/// All 78 PTABLE entries map to MIDI = pitch + 27 (D#1 = MIDI 27).
/// Black keys in C-based MIDI: C#=1, D#=3, F#=6, G#=8, A#=10 (mod 12).
fn pitch_is_black(pitch: u8) -> bool {
    matches!((pitch as u32 + 27) % 12, 1 | 3 | 6 | 8 | 10)
}

/// Find the nearest black key at or above `pitch`.
fn nearest_black_above(pitch: u8) -> u8 {
    let mut p = pitch;
    while p <= 77 {
        if pitch_is_black(p) { return p; }
        p += 1;
    }
    pitch // no black key found above, stay put
}

/// Find the nearest white key at or below `pitch`.
fn nearest_white_below(pitch: u8) -> u8 {
    let mut p = pitch;
    loop {
        if !pitch_is_black(p) { return p; }
        if p == 0 { return 0; }
        p -= 1;
    }
}

// ---------------------------------------------------------------------------
// Terminal rendering
// ---------------------------------------------------------------------------

fn render(
    stdout: &mut io::Stdout,
    kbd: &KeyboardState,
    st: &ManualState,
) -> io::Result<()> {
    // Derive active key display from voice state.
    // A key is "active" if its voice is playing and not releasing.
    // A key is "latched" if its voice is in manual (latched) mode.
    let mut active_keys: HashMap<char, bool> = HashMap::new();
    for v in &st.voices {
        if let Some(kc) = v.key_char {
            if v.playing && !v.releasing {
                active_keys.insert(kc, v.manual);
            }
        }
    }
    let ptable = &st.ptable;
    let (term_w, _term_h) = terminal::size()?;
    let term_w = term_w as usize;

    execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(ClearType::All))?;

    // ── Status line 1: instrument, octave, sine ─────────────────────────────
    let inst_name = INSTRUMENT_NAMES.get(kbd.current_instrument).copied().unwrap_or("?");
    let sine_status = if kbd.sine_mode { "ON" } else { "OFF" };
    let line1 = if kbd.sine_mode {
        format!(
            " Instrument: {} ({})  |  Octave: {}  |  Sine: {}  |  H2: {:.2}  H3: {:.2}  [F{}]",
            kbd.current_instrument, inst_name, kbd.base_octave,
            sine_status, kbd.sine_config.harmonic2, kbd.sine_config.harmonic3,
            kbd.selected_harmonic
        )
    } else {
        format!(
            " Instrument: {} ({})  |  Octave: {}  |  Sine: {}",
            kbd.current_instrument, inst_name, kbd.base_octave, sine_status
        )
    };
    let line1 = pad_or_clip(&line1, term_w);
    execute!(
        stdout,
        SetForegroundColor(Color::Yellow),
        SetAttribute(Attribute::Bold),
        Print(&line1),
        SetAttribute(Attribute::Reset),
        ResetColor,
        cursor::MoveToNextLine(1),
    )?;

    // ── Status line 2: pitch/period info ────────────────────────────────────
    let line2 = if let Some(pitch) = kbd.active_pitch {
        let (period, wave_offset) = ptable[pitch as usize];
        let (orig_period, _) = PTABLE[pitch as usize];
        let wave_len = (32 - wave_offset as usize) * 2;
        let freq = AMIGA_CLOCK_NTSC as f64 / (period as f64 * wave_len as f64);
        let name = pitch_to_name(pitch);
        if period != orig_period {
            format!(
                " Pitch: {}  {}  Period: {} -> {}  Freq: {:.1} Hz  [MODIFIED]",
                pitch, name, orig_period, period, freq
            )
        } else {
            format!(
                " Pitch: {}  {}  Period: {}  Freq: {:.1} Hz",
                pitch, name, period, freq
            )
        }
    } else {
        " (no active note)".to_string()
    };
    let line2 = pad_or_clip(&line2, term_w);
    execute!(
        stdout,
        SetForegroundColor(Color::Cyan),
        Print(&line2),
        ResetColor,
        cursor::MoveToNextLine(1),
    )?;

    // ── Help line ───────────────────────────────────────────────────────────
    let help = " [1-9]/[]=instr  Z/X=oct  Tab=sine  Shift+key=latch  Arrows=bend  KP2/KP8=tune  Q=quit";
    let help = pad_or_clip(help, term_w);
    execute!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
        Print(&help),
        ResetColor,
        cursor::MoveToNextLine(1),
    )?;

    // ── Separator ───────────────────────────────────────────────────────────
    let sep = format!(" {}", "─".repeat(term_w.saturating_sub(2)));
    execute!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
        Print(pad_or_clip(&sep, term_w)),
        ResetColor,
        cursor::MoveToNextLine(1),
    )?;

    // ── Piano keyboard ──────────────────────────────────────────────────────
    // We draw a simple text keyboard with 17 semitones visible (C to E+1).
    //
    // Layout (key_width = 8):
    //   Row 0 (black notes): │       C#      D#      │       F#      G#      A#      │       C#      D#      │
    //   Row 1 (black keys):  │       W       E       │       T       Y       U       │       O       P       │
    //   Row 2 (white keys):  │ A       │ S       │ D  │ F       │ G       │ H       │ J  │ K       │ L       │ ;   │
    //   Row 3 (white extra): │         │         │    │         │         │         │    │         │         │     │
    //   Row 4 (note names):  │ C3      │ D3      │ E3 │ F3      │ G3      │ A3      │ B3 │ C4      │ D4      │ E4  │

    let white_keys: [(u8, char); 10] = [
        (0, 'A'), (2, 'S'), (4, 'D'), (5, 'F'), (7, 'G'),
        (9, 'H'), (11, 'J'), (12, 'K'), (14, 'L'), (16, ';'),
    ];
    let black_keys: [(u8, char); 7] = [
        (1, 'W'), (3, 'E'), (6, 'T'), (8, 'Y'), (10, 'U'),
        (13, 'O'), (15, 'P'),
    ];

    let key_width = 8usize;
    let total_width = white_keys.len() * key_width + 1;
    let margin = (term_w.saturating_sub(total_width)) / 2;
    let pad_left = " ".repeat(margin);

    // Precompute black key info: (boundary_pos, key_char, 2-char note name)
    // boundary_pos = wi * key_width, where wi is the index of the white key to the right.
    // All black key names are exactly 2 chars (C#, D#, F#, G#, A#).
    let bk: Vec<(usize, char, &str)> = black_keys.iter().filter_map(|&(semi, kc)| {
        let wi = white_keys.iter().position(|&(ws, _)| ws > semi)?;
        if wi == 0 { return None; }
        Some((wi * key_width, kc, NOTE_NAMES[semi as usize % 12]))
    }).collect();

    // Base buffer: borders at every white-key boundary, spaces elsewhere.
    let mut base_buf = vec![' '; total_width];
    base_buf[0] = '│';
    for i in 1..=white_keys.len() {
        let p = i * key_width;
        if p < total_width { base_buf[p] = '│'; }
    }

    // ── Black key NOTE NAMES row ─────────────────────────────────────────────
    // Note name chars at [pos, pos+1]; pos replaces the '│' border.
    {
        let mut buf = base_buf.clone();
        for &(pos, _, note) in &bk {
            let mut nc = note.chars();
            if let (Some(c0), Some(c1)) = (nc.next(), nc.next()) {
                if pos + 1 < total_width {
                    buf[pos]     = c0;
                    buf[pos + 1] = c1;
                }
            }
        }
        execute!(stdout, Print(&pad_left))?;
        let mut i = 0;
        while i < buf.len() {
            if let Some(&(_, kc, _)) = bk.iter().find(|&&(pos, _, _)| i == pos) {
                // 2-char note name starting here
                let end = (i + 2).min(buf.len());
                let s: String = buf[i..end].iter().collect();
                let lc = kc.to_ascii_lowercase();
                let is_latched = active_keys.get(&lc).copied().unwrap_or(false);
                let is_active  = active_keys.contains_key(&lc);
                if is_latched {
                    execute!(stdout,
                        SetBackgroundColor(Color::Red), SetForegroundColor(Color::White),
                        SetAttribute(Attribute::Bold), Print(&s),
                        SetAttribute(Attribute::Reset), ResetColor,
                    )?;
                } else if is_active {
                    execute!(stdout,
                        SetBackgroundColor(Color::Green), SetForegroundColor(Color::Black),
                        SetAttribute(Attribute::Bold), Print(&s),
                        SetAttribute(Attribute::Reset), ResetColor,
                    )?;
                } else {
                    execute!(stdout, SetForegroundColor(Color::DarkGrey), Print(&s), ResetColor)?;
                }
                i += 2;
            } else if buf[i] == '│' {
                execute!(stdout, SetForegroundColor(Color::DarkGrey), Print("│"), ResetColor)?;
                i += 1;
            } else {
                execute!(stdout, Print(format!("{}", buf[i])))?;
                i += 1;
            }
        }
        execute!(stdout, cursor::MoveToNextLine(1))?;
    }

    // ── Black key LABELS row ─────────────────────────────────────────────────
    // Key char at [pos]; spaces at pos-1 and pos+1.
    {
        let mut buf = base_buf.clone();
        for &(pos, kc, _) in &bk {
            if pos >= 1 && pos + 1 < total_width {
                buf[pos - 1] = ' ';
                buf[pos]     = kc;
                buf[pos + 1] = ' ';
            }
        }
        execute!(stdout, Print(&pad_left))?;
        for (i, &ch) in buf.iter().enumerate() {
            if let Some(&(_, kc, _)) = bk.iter().find(|&&(pos, kc, _)| i == pos && ch == kc) {
                let lc = kc.to_ascii_lowercase();
                let is_latched = active_keys.get(&lc).copied().unwrap_or(false);
                let is_active  = active_keys.contains_key(&lc);
                if is_latched {
                    execute!(stdout,
                        SetBackgroundColor(Color::Red), SetForegroundColor(Color::White),
                        SetAttribute(Attribute::Bold), Print(format!("{}", ch)),
                        SetAttribute(Attribute::Reset), ResetColor,
                    )?;
                } else if is_active {
                    execute!(stdout,
                        SetBackgroundColor(Color::Green), SetForegroundColor(Color::Black),
                        SetAttribute(Attribute::Bold), Print(format!("{}", ch)),
                        SetAttribute(Attribute::Reset), ResetColor,
                    )?;
                } else {
                    execute!(stdout, SetForegroundColor(Color::Grey), Print(format!("{}", ch)), ResetColor)?;
                }
            } else if ch == '│' {
                execute!(stdout, SetForegroundColor(Color::DarkGrey), Print("│"), ResetColor)?;
            } else {
                execute!(stdout, Print(format!("{}", ch)))?;
            }
        }
        execute!(stdout, cursor::MoveToNextLine(1))?;
    }

    // ── White key rows (label + extra row for height) ────────────────────────
    for row in 0..2usize {
        execute!(stdout, Print(&pad_left))?;
        for (i, &(semi, key_char)) in white_keys.iter().enumerate() {
            let lc = key_char.to_ascii_lowercase();
            let is_latched = active_keys.get(&lc).copied().unwrap_or(false);
            let is_active  = active_keys.contains_key(&lc);
            // Row 0: show key char centred; row 1: blank (adds height)
            let content = if row == 0 {
                format!(" {:^w$}", key_char, w = key_width - 2)
            } else {
                " ".repeat(key_width - 1)
            };
            if is_latched {
                execute!(stdout,
                    SetForegroundColor(Color::DarkGrey), Print("│"), ResetColor,
                    SetBackgroundColor(Color::Red), SetForegroundColor(Color::White),
                    SetAttribute(Attribute::Bold), Print(&content),
                    SetAttribute(Attribute::Reset), ResetColor,
                )?;
            } else if is_active {
                execute!(stdout,
                    SetForegroundColor(Color::DarkGrey), Print("│"), ResetColor,
                    SetBackgroundColor(Color::Green), SetForegroundColor(Color::Black),
                    SetAttribute(Attribute::Bold), Print(&content),
                    SetAttribute(Attribute::Reset), ResetColor,
                )?;
            } else {
                execute!(stdout,
                    SetForegroundColor(Color::DarkGrey), Print("│"), ResetColor,
                    Print(&content),
                )?;
            }
            if i == white_keys.len() - 1 {
                execute!(stdout, SetForegroundColor(Color::DarkGrey), Print("│"), ResetColor)?;
            }
            let _ = semi;
        }
        execute!(stdout, cursor::MoveToNextLine(1))?;
    }

    // ── Note names row ───────────────────────────────────────────────────────
    // Use pitch_to_name so the label matches the actual note played (C-correct).
    execute!(stdout, Print(&pad_left))?;
    for (i, &(semi, _key_char)) in white_keys.iter().enumerate() {
        let pitch = (kbd.base_octave as i32 * 12 + semi as i32 - 3).max(0).min(77) as u8;
        let note_label = pitch_to_name(pitch);
        execute!(stdout,
            SetForegroundColor(Color::DarkGrey), Print("│"), ResetColor,
            SetForegroundColor(Color::Cyan),
            Print(format!(" {:^w$}", note_label, w = key_width - 2)),
            ResetColor,
        )?;
        if i == white_keys.len() - 1 {
            execute!(stdout, SetForegroundColor(Color::DarkGrey), Print("│"), ResetColor)?;
        }
    }
    execute!(stdout, cursor::MoveToNextLine(1))?;

    // ── Bottom border ───────────────────────────────────────────────────────
    execute!(stdout, Print(&pad_left))?;
    let bottom = format!("└{}┘", "─".repeat(total_width - 2));
    execute!(stdout,
        SetForegroundColor(Color::DarkGrey),
        Print(&bottom),
        ResetColor,
    )?;
    execute!(stdout, cursor::MoveToNextLine(1))?;

    // ── Active notes list ───────────────────────────────────────────────────
    if !kbd.held_keys.is_empty() {
        execute!(stdout, cursor::MoveToNextLine(1))?;
        let mut notes: Vec<String> = kbd.held_keys.iter().map(|(c, hk)| {
            let name = pitch_to_name(hk.pitch);
            let latch = if hk.latched { " [L]" } else { "" };
            format!("{}:{}{}", c.to_uppercase(), name, latch)
        }).collect();
        notes.sort();
        let notes_line = format!(" Active: {}", notes.join("  "));
        let notes_line = pad_or_clip(&notes_line, term_w);
        execute!(stdout,
            SetForegroundColor(Color::Green),
            Print(&notes_line),
            ResetColor,
        )?;
    }

    stdout.flush()
}

fn pad_or_clip(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        s.chars().take(width).collect()
    } else {
        format!("{}{:pad$}", s, "", pad = width - len)
    }
}

// ---------------------------------------------------------------------------
// PTABLE export
// ---------------------------------------------------------------------------

fn print_ptable(ptable: &[(u16, u16); 78]) {
    println!();
    println!("// Modified PTABLE — paste into src/game/songs.rs");
    println!("pub const PTABLE: [(u16, u16); 78] = [");

    // Row 0 is 6 entries; rows 1-6 are 12 entries each.
    let rows: &[(usize, usize, &str)] = &[
        (0,  6,  "Row 0 – D#1–G#1    (pitch 0–5)"),
        (6,  12, "Row 1 – A1–G#2     (pitch 6–17)"),
        (18, 12, "Row 2 – A2–G#3     (pitch 18–29)"),
        (30, 12, "Row 3 – A3–G#4     (pitch 30–41, wave_offset 16)"),
        (42, 12, "Row 4 – A4–G#5     (pitch 42–53, wave_offset 24)"),
        (54, 12, "Row 5 – A5–G#6     (pitch 54–65, wave_offset 28)"),
        (66, 12, "Row 6 – A6–G#7     (pitch 66–77, wave_offset 28)"),
    ];

    for &(base, len, label) in rows {
        println!("    // {}", label);
        print!("    ");
        for i in 0..len {
            let idx = base + i;
            let (period, wo) = ptable[idx];
            let (orig_period, _) = PTABLE[idx];
            if period != orig_period {
                print!("({:>4}, {:>2}), // was {}", period, wo, orig_period);
                if i < len - 1 { print!("\n    "); }
            } else {
                print!("({:>4}, {:>2}),", period, wo);
                if i < len - 1 { print!("  "); }
            }
        }
        println!();
    }
    println!("];");
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let instruments = Instruments::load(&base.join("game/v6"))
        .expect("Could not load game/v6 — run from the project root");

    // ── SDL2 audio setup ────────────────────────────────────────────────────
    let sdl = sdl2::init()?;
    let audio_ss = sdl.audio()?;
    let desired = AudioSpecDesired {
        freq: Some(SAMPLE_RATE as i32),
        channels: Some(2),
        samples: Some(256), // smaller buffer for lower latency
    };

    let audio_state = Arc::new(Mutex::new(ManualState::new(instruments)));
    let cb_state = Arc::clone(&audio_state);

    let device = audio_ss.open_playback(None, &desired, |_spec| {
        SynthCallback { state: cb_state }
    })?;
    device.resume();

    // ── Terminal setup ──────────────────────────────────────────────────────
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        cursor::Hide,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        )
    )?;

    let mut kbd = KeyboardState::new();

    // Initial render
    {
        let st = audio_state.lock().unwrap();
        render(&mut stdout, &kbd, &st)?;
    }

    // ── Main loop ───────────────────────────────────────────────────────────
    'main: loop {
        // ── Periodic cleanup: sync held_keys with audio voice state ─────
        // When a non-latched voice finishes playing, remove the key from
        // held_keys so it can be re-triggered. Visual display is derived
        // from voice state in render(), so no separate tracking needed.
        {
            let st = audio_state.lock().unwrap();
            let stale: Vec<char> = kbd.held_keys.iter()
                .filter(|(c, hk)| {
                    if hk.latched { return false; }
                    let v = &st.voices[hk.voice_idx];
                    // Also stale if the voice was stolen by a different key
                    v.releasing || !v.playing || v.key_char != Some(**c)
                })
                .map(|(c, _)| *c)
                .collect();
            for key in stale {
                kbd.held_keys.remove(&key);
            }
            // Update active_pitch if it referred to a removed key
            if let Some(ap) = kbd.active_pitch {
                let still_active = kbd.held_keys.values().any(|hk| hk.pitch == ap);
                if !still_active {
                    kbd.active_pitch = kbd.held_keys.values().last().map(|hk| hk.pitch);
                }
            }
        }

        // Always re-render to reflect voice state changes (release fading, etc.)
        {
            let st = audio_state.lock().unwrap();
            render(&mut stdout, &kbd, &st)?;
        }

        // Poll events with a short timeout for responsive input
        if !event::poll(Duration::from_millis(16))? {
            continue;
        }

        let ev = event::read()?;
        let mut needs_render = false;

        match ev {
            Event::Key(KeyEvent { code, modifiers, kind, state, .. }) => {
                // We care about Press and Repeat events for most keys,
                // and Release events for non-latched piano keys.
                let is_press = kind == KeyEventKind::Press || kind == KeyEventKind::Repeat;
                let is_release = kind == KeyEventKind::Release;
                let has_shift = modifiers.contains(KeyModifiers::SHIFT);

                match code {
                    // ── Quit ──────────────────────────────────────────────
                    KeyCode::Esc => break 'main,
                    KeyCode::Char('q') if is_press && !has_shift => {
                        // Only quit on 'q' if it's not being used as a piano key
                        // 'q' is not mapped as a piano key, so it's safe
                        break 'main;
                    },
                    KeyCode::Char('c') if is_press && modifiers.contains(KeyModifiers::CONTROL) => {
                        break 'main;
                    },

                    // ── Octave ────────────────────────────────────────────
                    KeyCode::Char('z') if is_press && !has_shift => {
                        if kbd.base_octave > 0 {
                            kbd.base_octave -= 1;
                            needs_render = true;
                        }
                    },
                    KeyCode::Char('x') if is_press && !has_shift => {
                        if kbd.base_octave < 6 {
                            kbd.base_octave += 1;
                            needs_render = true;
                        }
                    },

                    // ── Instrument select (number keys, not numpad) ──────────────────
                    KeyCode::Char(c @ '1'..='9') if is_press && !state.contains(KeyEventState::KEYPAD) => {
                        let slot = (c as usize - '1' as usize).min(11);
                        kbd.current_instrument = slot;
                        let mut st = audio_state.lock().unwrap();
                        // If sine mode was on, restore waveform for old instrument first
                        if st.sine_mode {
                            st.toggle_sine(); // restore
                        }
                        st.current_instrument = slot;
                        kbd.sine_mode = st.sine_mode;
                        needs_render = true;
                    },

                    // ── Instrument cycle ─────────────────────────────────
                    KeyCode::Char('[') if is_press => {
                        let slot = if kbd.current_instrument == 0 { 11 } else { kbd.current_instrument - 1 };
                        kbd.current_instrument = slot;
                        let mut st = audio_state.lock().unwrap();
                        if st.sine_mode { st.toggle_sine(); }
                        st.current_instrument = slot;
                        kbd.sine_mode = st.sine_mode;
                        needs_render = true;
                    },
                    KeyCode::Char(']') if is_press => {
                        let slot = (kbd.current_instrument + 1) % 12;
                        kbd.current_instrument = slot;
                        let mut st = audio_state.lock().unwrap();
                        if st.sine_mode { st.toggle_sine(); }
                        st.current_instrument = slot;
                        kbd.sine_mode = st.sine_mode;
                        needs_render = true;
                    },

                    // ── Sine mode toggle ─────────────────────────────────
                    KeyCode::Tab if is_press => {
                        let mut st = audio_state.lock().unwrap();
                        st.toggle_sine();
                        kbd.sine_mode = st.sine_mode;
                        needs_render = true;
                    },

                    // ── Harmonic selection ────────────────────────────────
                    KeyCode::F(2) if is_press => {
                        kbd.selected_harmonic = 2;
                        needs_render = true;
                    },
                    KeyCode::F(3) if is_press => {
                        kbd.selected_harmonic = 3;
                        needs_render = true;
                    },

                    // ── Harmonic adjustment ──────────────────────────────
                    KeyCode::Char('+') | KeyCode::Char('=') if is_press && kbd.sine_mode => {
                        if kbd.selected_harmonic == 2 {
                            kbd.sine_config.harmonic2 = (kbd.sine_config.harmonic2 + 0.05).min(1.0);
                        } else {
                            kbd.sine_config.harmonic3 = (kbd.sine_config.harmonic3 + 0.05).min(1.0);
                        }
                        let mut st = audio_state.lock().unwrap();
                        st.sine_config = kbd.sine_config.clone();
                        st.update_sine();
                        needs_render = true;
                    },
                    KeyCode::Char('-') | KeyCode::Char('_') if is_press && kbd.sine_mode => {
                        if kbd.selected_harmonic == 2 {
                            kbd.sine_config.harmonic2 = (kbd.sine_config.harmonic2 - 0.05).max(0.0);
                        } else {
                            kbd.sine_config.harmonic3 = (kbd.sine_config.harmonic3 - 0.05).max(0.0);
                        }
                        let mut st = audio_state.lock().unwrap();
                        st.sine_config = kbd.sine_config.clone();
                        st.update_sine();
                        needs_render = true;
                    },

                    // ── PTABLE fine tuning (Numpad 2/8) ──────────────────
                    KeyCode::Char('2') if is_press && state.contains(KeyEventState::KEYPAD) => {
                        if let Some(pitch) = kbd.active_pitch {
                            let delta = if has_shift { -10 } else { -1 };
                            let mut st = audio_state.lock().unwrap();
                            st.adjust_period(pitch as usize, delta);
                            kbd.ptable_dirty = true;
                            needs_render = true;
                        }
                    },
                    KeyCode::Char('8') if is_press && state.contains(KeyEventState::KEYPAD) => {
                        if let Some(pitch) = kbd.active_pitch {
                            let delta = if has_shift { 10 } else { 1 };
                            let mut st = audio_state.lock().unwrap();
                            st.adjust_period(pitch as usize, delta);
                            kbd.ptable_dirty = true;
                            needs_render = true;
                        }
                    },

                    // ── Arrow keys (re-pitch latched notes) ──────────────
                    KeyCode::Right if is_press => {
                        if let Some(latch_char) = kbd.last_latched {
                            if let Some(hk) = kbd.held_keys.get_mut(&latch_char) {
                                if hk.pitch < 77 {
                                    hk.pitch += 1;
                                    let pitch = hk.pitch;
                                    let vi = hk.voice_idx;
                                    kbd.active_pitch = Some(pitch);
                                    let mut st = audio_state.lock().unwrap();
                                    st.retrigger_voice(vi, pitch);
                                    needs_render = true;
                                }
                            }
                        }
                    },
                    KeyCode::Left if is_press => {
                        if let Some(latch_char) = kbd.last_latched {
                            if let Some(hk) = kbd.held_keys.get_mut(&latch_char) {
                                if hk.pitch > 0 {
                                    hk.pitch -= 1;
                                    let pitch = hk.pitch;
                                    let vi = hk.voice_idx;
                                    kbd.active_pitch = Some(pitch);
                                    let mut st = audio_state.lock().unwrap();
                                    st.retrigger_voice(vi, pitch);
                                    needs_render = true;
                                }
                            }
                        }
                    },
                    KeyCode::Up if is_press => {
                        if let Some(latch_char) = kbd.last_latched {
                            if let Some(hk) = kbd.held_keys.get_mut(&latch_char) {
                                let new_pitch = nearest_black_above(hk.pitch.saturating_add(1));
                                if new_pitch != hk.pitch && new_pitch <= 77 {
                                    hk.pitch = new_pitch;
                                    let vi = hk.voice_idx;
                                    kbd.active_pitch = Some(new_pitch);
                                    let mut st = audio_state.lock().unwrap();
                                    st.retrigger_voice(vi, new_pitch);
                                    needs_render = true;
                                }
                            }
                        }
                    },
                    KeyCode::Down if is_press => {
                        if let Some(latch_char) = kbd.last_latched {
                            if let Some(hk) = kbd.held_keys.get_mut(&latch_char) {
                                let new_pitch = nearest_white_below(hk.pitch.saturating_sub(1));
                                if new_pitch != hk.pitch {
                                    hk.pitch = new_pitch;
                                    let vi = hk.voice_idx;
                                    kbd.active_pitch = Some(new_pitch);
                                    let mut st = audio_state.lock().unwrap();
                                    st.retrigger_voice(vi, new_pitch);
                                    needs_render = true;
                                }
                            }
                        }
                    },

                    // ── Piano keys ───────────────────────────────────────
                    // Only trigger on Press, not Repeat — we track held
                    // state ourselves, and stale Repeat events after a
                    // Release could re-trigger the note.
                    KeyCode::Char(c) if kind == KeyEventKind::Press => {
                        let lc = c.to_ascii_lowercase();
                        if let Some(pitch) = kbd.pitch_for_key(lc) {
                            if has_shift {
                                // Latch toggle
                                if let Some(hk) = kbd.held_keys.get(&lc) {
                                    // Already held — unlatch and release
                                    let vi = hk.voice_idx;
                                    let mut st = audio_state.lock().unwrap();
                                    st.release_voice(vi);
                                    kbd.held_keys.remove(&lc);
                                    if kbd.last_latched == Some(lc) {
                                        // Find another latched key, or None
                                        kbd.last_latched = kbd.held_keys.iter()
                                            .find(|(_, hk)| hk.latched)
                                            .map(|(c, _)| *c);
                                    }
                                    // Update active pitch
                                    kbd.active_pitch = kbd.last_latched
                                        .and_then(|c| kbd.held_keys.get(&c))
                                        .map(|hk| hk.pitch);
                                } else {
                                    // New latch
                                    let vi = kbd.find_free_voice();
                                    kbd.held_keys.insert(lc, HeldKey { pitch, voice_idx: vi, latched: true });
                                    kbd.last_latched = Some(lc);
                                    kbd.active_pitch = Some(pitch);
                                    let mut st = audio_state.lock().unwrap();
                                    st.trigger_voice(vi, pitch, true, lc);
                                }
                            } else {
                                // Normal press (check if latched first)
                                if let Some(hk) = kbd.held_keys.get(&lc) {
                                    if hk.latched {
                                        // Pressing a latched key unlatches it
                                        let vi = hk.voice_idx;
                                        let mut st = audio_state.lock().unwrap();
                                        st.release_voice(vi);
                                        kbd.held_keys.remove(&lc);
                                        if kbd.last_latched == Some(lc) {
                                            kbd.last_latched = kbd.held_keys.iter()
                                                .find(|(_, hk)| hk.latched)
                                                .map(|(c, _)| *c);
                                        }
                                        kbd.active_pitch = kbd.last_latched
                                            .and_then(|c| kbd.held_keys.get(&c))
                                            .map(|hk| hk.pitch);
                                    } else {
                                        // Already held (non-latched), ignore repeat
                                    }
                                } else {
                                    // New non-latched hold (uses ADSR envelope)
                                    let vi = kbd.find_free_voice();
                                    kbd.held_keys.insert(lc, HeldKey { pitch, voice_idx: vi, latched: false });
                                    kbd.active_pitch = Some(pitch);
                                    let mut st = audio_state.lock().unwrap();
                                    st.trigger_voice(vi, pitch, false, lc);
                                }
                            }
                            needs_render = true;
                        }
                    },

                    _ => {}
                }

                // Handle key release for non-latched keys:
                // Only start the audio release — visual cleanup is handled
                // by the polling code at the top of the loop.
                if is_release {
                    if let KeyCode::Char(c) = code {
                        let lc = c.to_ascii_lowercase();
                        if let Some(hk) = kbd.held_keys.get(&lc) {
                            if !hk.latched {
                                let vi = hk.voice_idx;
                                let mut st = audio_state.lock().unwrap();
                                st.release_voice_soft(vi);
                                // held_keys cleanup handled by polling;
                                // visual driven by voice state in render().
                            }
                        }
                    }
                }
            },
            Event::Resize(_, _) => {
                needs_render = true;
            },
            _ => {},
        }

        if needs_render {
            let st = audio_state.lock().unwrap();
            render(&mut stdout, &kbd, &st)?;
        }
    }

    // ── Cleanup ─────────────────────────────────────────────────────────────
    execute!(
        stdout,
        PopKeyboardEnhancementFlags,
        cursor::Show,
        terminal::LeaveAlternateScreen
    )?;
    terminal::disable_raw_mode()?;

    // Print modified PTABLE if dirty
    if kbd.ptable_dirty {
        let st = audio_state.lock().unwrap();
        print_ptable(&st.ptable);
    }

    Ok(())
}
