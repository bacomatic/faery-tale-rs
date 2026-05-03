//! Terminal music visualizer for the Faery Tale Adventure.
//!
//! Plays a song group through SDL2 audio while rendering a scrolling note
//! view in the terminal.
//!
//! Usage:
//!   cargo run --bin music_viz [-- <group>]   (group 0-6, default 3 = intro)
//!
//! Controls:
//!   Space         — pause / resume
//!   + / -         — increase / decrease tempo by 10
//!   Enter         — step one VBL tick while paused
//!   1 / 2 / 3 / 4 — toggle track on/off
//!   ← / →         — previous / next song group
//!   Q / Esc       — quit

// ---------------------------------------------------------------------------
// Bring in the songs module directly (it has no intra-crate deps).
// ---------------------------------------------------------------------------
#[allow(dead_code, unused_imports)]
#[path = "../game/songs.rs"]
mod songs;

use songs::{
    SongLibrary, Track, TrackEvent, AMIGA_CLOCK_NTSC, DEFAULT_TEMPO, NOTE_DURATIONS, PTABLE,
    VBL_RATE_HZ,
};

// ---------------------------------------------------------------------------
// Re-implement the minimal audio types we need (Voice, etc. are private
// in game::audio, so we keep a self-contained copy here).
// ---------------------------------------------------------------------------

use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};

use sdl2::audio::{AudioCallback, AudioSpecDesired};

// ---------------------------------------------------------------------------
// Audio constants (mirrors audio.rs)
// ---------------------------------------------------------------------------

pub const SAMPLE_RATE: u32 = 44_100;
const VBL_RATE: u32 = VBL_RATE_HZ;
const SAMPLES_PER_VBL: f64 = SAMPLE_RATE as f64 / VBL_RATE as f64;

/// Absolute minimum ticks-per-row (= smallest entry in NOTE_DURATIONS).
const MIN_TICKS_PER_ROW: u32 = 140;

const WAVEFORM_COUNT: usize = 8;
const WAVEFORM_BYTES: usize = 128;
const ENVELOPE_COUNT: usize = 10;
const ENVELOPE_BYTES: usize = 256;
const NOTE_GAP: u32 = 300;
const STEREO_PRIMARY: f32 = 0.75;
const STEREO_BLEED: f32 = 0.25;

/// NEW_WAVE table: (wave_num, vol_num) for each instrument slot.
const NEW_WAVE: [(u8, u8); 12] = [
    (0, 0),
    (0, 0),
    (0, 0),
    (0, 0),
    (0, 5),
    (2, 2),
    (1, 1),
    (1, 3),
    (0, 4),
    (5, 4),
    (1, 0),
    (5, 0),
];

/// Human-readable instrument names for the visualizer header.
/// Indexed by NEW_WAVE slot (0-11): (wave_num, vol_num) → character.
/// Slots 0-3 all use wave=0, env=0 (attack vol 62, slow decay) — not silent.
const INSTRUMENT_NAMES: [&str; 12] = [
    "Piano",      // slot 0  (wave 0, env 0 — decay from vol 62)
    "Piano",      // slot 1  (wave 0, env 0)
    "Piano",      // slot 2  (wave 0, env 0)
    "Piano",      // slot 3  (wave 0, env 0)
    "Strings",    // slot 4  (wave 0, env 5)
    "Brass",      // slot 5  (wave 2, env 2)
    "Harpsichrd", // slot 6  (wave 1, env 1 — fast decay)
    "Woodwind",   // slot 7  (wave 1, env 3 — long sustain)
    "Flute",      // slot 8  (wave 0, env 4 — attack shape)
    "Organ",      // slot 9  (wave 5, env 4)
    "Pluck",      // slot 10 (wave 1, env 0)
    "Pad",        // slot 11 (wave 5, env 0)
];

// ---------------------------------------------------------------------------
// Instruments (mirrors the public struct from game::audio)
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
        // Volume data starts at byte 2048 (after a seek of S_WAVBUF past the wave read).
        let vol_base = WAVEFORM_COUNT * WAVEFORM_BYTES * 2;
        for (e, env) in envelopes.iter_mut().enumerate() {
            let base = vol_base + e * ENVELOPE_BYTES;
            if base + ENVELOPE_BYTES <= data.len() {
                env.copy_from_slice(&data[base..base + ENVELOPE_BYTES]);
            }
        }
        Instruments {
            waveforms,
            envelopes,
        }
    }
}

// ---------------------------------------------------------------------------
// Per-voice synthesizer (mirrors game::audio::Voice)
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
    // Sequencer fields
    event_start: u32,
    event_stop: u32,
    trak_ptr: Option<usize>,
    trak_beg: Option<usize>,
}

impl Voice {
    fn new() -> Self {
        Voice {
            wave_num: 0,
            vol_num: 0,
            vol_list: 0,
            vol_delay: 0xff,
            volume: 0,
            wave_start: 0,
            wave_len: 64,
            phase: 0.0,
            phase_inc: 0.0,
            playing: false,
            lp_state: 0.0,
            declick: 0.0,
            event_start: 0,
            event_stop: 0,
            trak_ptr: None,
            trak_beg: None,
        }
    }

    fn set_instrument_slot(&mut self, slot: usize) {
        let (wave, vol) = NEW_WAVE[slot.min(NEW_WAVE.len() - 1)];
        self.wave_num = wave as usize;
        self.vol_num = vol as usize;
    }

    fn trigger_note(&mut self, pitch: usize, inst: &Instruments) {
        if pitch >= PTABLE.len() {
            return;
        }
        let (period, wave_offset) = PTABLE[pitch];
        if period == 0 {
            return;
        }
        self.wave_start = (wave_offset as usize) * 4;
        self.wave_len = ((32 - wave_offset as usize) * 2).min(WAVEFORM_BYTES - self.wave_start);
        if self.wave_len == 0 {
            self.playing = false;
            return;
        }
        self.phase = 0.0;
        self.phase_inc = AMIGA_CLOCK_NTSC as f64 / (period as f64 * SAMPLE_RATE as f64);
        let vol_num = self.vol_num.min(ENVELOPE_COUNT - 1);
        let first = inst.envelopes[vol_num][0];
        if first < 0x80 {
            self.volume = first.min(64);
            self.vol_list = 1;
        } else {
            self.vol_list = 0;
        }
        self.vol_delay = 0;
        self.playing = true;
    }

    fn silence(&mut self) {
        self.playing = false;
        self.volume = 0;
        self.vol_delay = 0xff;
    }

    fn step_envelope(&mut self, envelopes: &[[u8; ENVELOPE_BYTES]; ENVELOPE_COUNT]) {
        if self.vol_delay != 0 {
            return;
        }
        let byte =
            envelopes[self.vol_num.min(ENVELOPE_COUNT - 1)][self.vol_list.min(ENVELOPE_BYTES - 1)];
        if byte >= 0x80 {
            self.vol_delay = 0xff;
        } else {
            self.volume = byte.min(64);
            self.vol_list = self.vol_list.saturating_add(1);
        }
    }

    fn mix_stereo(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        inst: &Instruments,
        left_gain: f32,
        right_gain: f32,
        muted: bool,
    ) {
        const NOISE_FLOOR: f32 = 1.0 / 65536.0;
        const DECLICK_RATE: f32 = 1.0 / 64.0;
        const LP_ALPHA: f32 = 0.487;

        let target = if self.playing && self.wave_len > 0 && self.volume > 0 && !muted {
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
                if self.phase >= len as f64 {
                    self.phase -= len as f64;
                }
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
// Audio sequencer (shared with SDL2 callback via Arc<Mutex>)
// ---------------------------------------------------------------------------

struct SeqState {
    voices: [Voice; 4],
    timeclock: u32,
    tempo: u32,
    nosound: bool,
    tracks: [Option<Arc<Track>>; 4],
    samples_to_vbl: f64,
    muted: [bool; 4],
}

impl SeqState {
    fn new() -> Self {
        SeqState {
            voices: [Voice::new(), Voice::new(), Voice::new(), Voice::new()],
            timeclock: 0,
            tempo: DEFAULT_TEMPO,
            nosound: true,
            tracks: [None, None, None, None],
            samples_to_vbl: 0.0,
            muted: [false; 4],
        }
    }

    fn play_score(&mut self, group_tracks: [Arc<Track>; 4], inst: &Instruments) {
        self.timeclock = 0;
        self.tempo = DEFAULT_TEMPO;
        for (i, v) in self.voices.iter_mut().enumerate() {
            *v = Voice::new();
            v.set_instrument_slot(i);
            let vol_num = v.vol_num.min(ENVELOPE_COUNT - 1);
            let first = inst.envelopes[vol_num][0];
            if first < 0x80 {
                v.volume = first.min(64);
                v.vol_list = 1;
            } else {
                v.vol_list = 0;
            }
            self.tracks[i] = Some(Arc::clone(&group_tracks[i]));
            v.trak_ptr = Some(0);
            v.trak_beg = Some(0);
            v.event_start = 0;
            v.event_stop = 0;
        }
        self.nosound = false;
        self.samples_to_vbl = 0.0;
    }

    fn vbl_tick(&mut self, inst: &Instruments) {
        if self.nosound {
            return;
        }
        self.timeclock = self.timeclock.wrapping_add(self.tempo);
        for vi in 0..4 {
            self.tick_voice(vi, inst);
        }
    }

    fn tick_voice(&mut self, vi: usize, inst: &Instruments) {
        let tc = self.timeclock;
        self.voices[vi].step_envelope(&inst.envelopes);
        if self.voices[vi].trak_ptr.is_none() {
            return;
        }
        let track = match &self.tracks[vi] {
            Some(t) => Arc::clone(t),
            None => return,
        };

        let diff = tc.wrapping_sub(self.voices[vi].event_start);
        if diff >= 0x8000_0000 {
            let stop_diff = tc.wrapping_sub(self.voices[vi].event_stop);
            if stop_diff < 0x8000_0000 {
                self.voices[vi].playing = false;
                self.voices[vi].volume = 0;
            }
            return;
        }

        let mut ptr = self.voices[vi].trak_ptr.unwrap();
        loop {
            let event = match track.get(ptr) {
                Some(e) => e,
                None => break,
            };
            ptr += 1;
            match event {
                TrackEvent::Note {
                    pitch,
                    duration_idx,
                } => {
                    let dur = NOTE_DURATIONS[(*duration_idx as usize).min(63)] as u32;
                    let sustain = if dur >= NOTE_GAP { dur - NOTE_GAP } else { dur };
                    let es = self.voices[vi].event_start;
                    self.voices[vi].event_stop = es.wrapping_add(sustain);
                    self.voices[vi].event_start = es.wrapping_add(dur);
                    self.voices[vi].trak_ptr = Some(ptr);
                    self.voices[vi].trigger_note(*pitch as usize, inst);
                    return;
                }
                TrackEvent::Rest { duration_idx } => {
                    let dur = NOTE_DURATIONS[(*duration_idx as usize).min(63)] as u32;
                    let es = self.voices[vi].event_start;
                    self.voices[vi].event_stop = es;
                    self.voices[vi].event_start = es.wrapping_add(dur);
                    self.voices[vi].trak_ptr = Some(ptr);
                    self.voices[vi].silence();
                    return;
                }
                TrackEvent::SetInstrument { slot } => {
                    self.voices[vi].set_instrument_slot(*slot as usize);
                }
                TrackEvent::SetTempo { value } => {
                    self.tempo = *value as u32;
                }
                TrackEvent::End { looping } => {
                    if *looping {
                        ptr = self.voices[vi].trak_beg.unwrap_or(0);
                    } else {
                        self.voices[vi].trak_ptr = None;
                        self.voices[vi].silence();
                        return;
                    }
                }
                TrackEvent::Unknown { .. } => {}
            }
        }
        self.voices[vi].trak_ptr = Some(ptr);
    }
}

// ---------------------------------------------------------------------------
// SDL2 audio callback
// ---------------------------------------------------------------------------

struct SynthCallback {
    state: Arc<Mutex<SeqState>>,
    instruments: Instruments,
}

impl AudioCallback for SynthCallback {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        for s in out.iter_mut() {
            *s = 0;
        }
        let mut st = match self.state.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let inst = &self.instruments;
        let total_frames = out.len() / 2;
        let mut frame_pos = 0usize;

        while frame_pos < total_frames {
            if st.samples_to_vbl <= 0.0 {
                st.vbl_tick(inst);
                st.samples_to_vbl += SAMPLES_PER_VBL;
                continue;
            }
            let chunk = (st.samples_to_vbl.floor() as usize)
                .min(total_frames - frame_pos)
                .max(1);
            let mut left_buf = vec![0.0f32; chunk];
            let mut right_buf = vec![0.0f32; chunk];
            let muted = st.muted;
            // Paula: ch0, ch3 → Left; ch1, ch2 → Right
            st.voices[0].mix_stereo(
                &mut left_buf,
                &mut right_buf,
                inst,
                STEREO_PRIMARY,
                STEREO_BLEED,
                muted[0],
            );
            st.voices[3].mix_stereo(
                &mut left_buf,
                &mut right_buf,
                inst,
                STEREO_PRIMARY,
                STEREO_BLEED,
                muted[3],
            );
            st.voices[1].mix_stereo(
                &mut right_buf,
                &mut left_buf,
                inst,
                STEREO_PRIMARY,
                STEREO_BLEED,
                muted[1],
            );
            st.voices[2].mix_stereo(
                &mut right_buf,
                &mut left_buf,
                inst,
                STEREO_PRIMARY,
                STEREO_BLEED,
                muted[2],
            );
            for i in 0..chunk {
                let base = (frame_pos + i) * 2;
                out[base] = (left_buf[i].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                out[base + 1] = (right_buf[i].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            }
            frame_pos += chunk;
            st.samples_to_vbl -= chunk as f64;
        }
    }
}

// ---------------------------------------------------------------------------
// Note name helper
// ---------------------------------------------------------------------------

/// Convert a pitch PTABLE index → note name + octave (e.g. "C#4").
///
/// Derives note from the Paula period via frequency → MIDI note number.
fn pitch_name(pitch: u8) -> String {
    if pitch as usize >= PTABLE.len() {
        return "?".to_string();
    }
    let (period, _) = PTABLE[pitch as usize];
    if period == 0 {
        return "---".to_string();
    }
    let freq = AMIGA_CLOCK_NTSC as f64 / period as f64;
    // MIDI 69 = A4 = 440 Hz
    let midi = 69.0 + 12.0 * (freq / 440.0).log2();
    let midi_i = midi.round() as i32;
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let name = NAMES[((midi_i % 12 + 12) % 12) as usize];
    let octave = midi_i / 12 - 1;
    format!("{}{}", name, octave)
}

// ---------------------------------------------------------------------------
// Visualizer track state (piano-roll, pre-scanned)
// ---------------------------------------------------------------------------

/// One note/rest event with absolute tick position (relative to song start, pre-loop).
struct ScheduledEvent {
    tick_start: u32, // timeclock value when this event begins
    tick_dur: u32,   // duration in timeclock units
    label: String,   // note name like "C#4", empty for rests
    is_rest: bool,
}

struct TrackVis {
    events: Vec<ScheduledEvent>, // sorted by tick_start
    loop_len: u32,               // total ticks in one loop; 0 = no loop
    min_note_dur: u32,           // shortest note/rest seen (u32::MAX if empty)
    instrument: u8,
    enabled: bool,
}

impl TrackVis {
    fn new(track: &Track) -> Self {
        let mut events: Vec<ScheduledEvent> = Vec::new();
        let mut tick: u32 = 0;
        let mut instrument: u8 = 0;
        let mut loop_len: u32 = 0;
        let mut ptr: usize = 0;

        loop {
            let event = match track.get(ptr) {
                Some(e) => e,
                None => break,
            };
            ptr += 1;
            match event {
                TrackEvent::Note {
                    pitch,
                    duration_idx,
                } => {
                    let dur = NOTE_DURATIONS[(*duration_idx as usize).min(63)] as u32;
                    events.push(ScheduledEvent {
                        tick_start: tick,
                        tick_dur: dur,
                        label: pitch_name(*pitch),
                        is_rest: false,
                    });
                    tick += dur;
                }
                TrackEvent::Rest { duration_idx } => {
                    let dur = NOTE_DURATIONS[(*duration_idx as usize).min(63)] as u32;
                    events.push(ScheduledEvent {
                        tick_start: tick,
                        tick_dur: dur,
                        label: String::new(),
                        is_rest: true,
                    });
                    tick += dur;
                }
                TrackEvent::SetInstrument { slot } => {
                    instrument = *slot;
                }
                TrackEvent::SetTempo { .. } => {}
                TrackEvent::End { looping } => {
                    if *looping {
                        loop_len = tick;
                    }
                    break;
                }
                TrackEvent::Unknown { .. } => {}
            }
        }

        let min_note_dur = events
            .iter()
            .map(|e| e.tick_dur)
            .filter(|&d| d > 0)
            .min()
            .unwrap_or(u32::MAX);

        TrackVis {
            events,
            loop_len,
            min_note_dur,
            instrument,
            enabled: true,
        }
    }

    /// Look up the event covering `row_tick` (the first tick of a display row).
    /// Returns (event, is_start_row) where is_start_row is true when the event
    /// begins within this row's tick span [row_tick, row_tick+ticks_per_row).
    fn event_at(&self, row_tick: u32, ticks_per_row: u32) -> Option<(&ScheduledEvent, bool)> {
        if self.events.is_empty() {
            return None;
        }
        let eff = if self.loop_len > 0 {
            row_tick % self.loop_len
        } else {
            row_tick
        };
        let row_end = eff.saturating_add(ticks_per_row);

        // First check: does an event START inside this row?
        let start_idx = self.events.partition_point(|e| e.tick_start < eff);
        if let Some(ev) = self.events.get(start_idx) {
            if ev.tick_start < row_end {
                return Some((ev, true));
            }
        }

        // Second check: does an event that started BEFORE this row span into it?
        if start_idx > 0 {
            let ev = &self.events[start_idx - 1];
            if ev.tick_start.saturating_add(ev.tick_dur) > eff {
                return Some((ev, false));
            }
        }

        None
    }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

struct App {
    songs: SongLibrary,
    instruments: Instruments,
    current_group: usize,
    tracks: [TrackVis; 4],
    ticks_per_row: u32,
    paused: bool,
    /// Link to the audio callback state — timeclock is read from here for sync.
    audio: Arc<Mutex<SeqState>>,
}

impl App {
    fn new(
        songs: SongLibrary,
        instruments: Instruments,
        group: usize,
        audio: Arc<Mutex<SeqState>>,
    ) -> Self {
        let tracks = Self::make_tracks(&songs, group);
        let ticks_per_row = Self::auto_scale(&tracks);
        App {
            songs,
            instruments,
            current_group: group,
            tracks,
            ticks_per_row,
            paused: false,
            audio,
        }
    }

    fn make_tracks(songs: &SongLibrary, group: usize) -> [TrackVis; 4] {
        let g = songs.group(group).expect("invalid group");
        [
            TrackVis::new(&g[0]),
            TrackVis::new(&g[1]),
            TrackVis::new(&g[2]),
            TrackVis::new(&g[3]),
        ]
    }

    /// Choose an initial ticks_per_row based on the shortest note in the song.
    /// We snap to the nearest power-of-2 multiple of MIN_TICKS_PER_ROW that is
    /// >= the shortest note duration, so note boundaries always align to rows.
    fn auto_scale(tracks: &[TrackVis; 4]) -> u32 {
        let min_dur = tracks
            .iter()
            .map(|t| t.min_note_dur)
            .filter(|&d| d < u32::MAX)
            .min()
            .unwrap_or(MIN_TICKS_PER_ROW);
        // Find smallest power-of-2 multiple of MIN_TICKS_PER_ROW that covers min_dur.
        let mut tpr = MIN_TICKS_PER_ROW;
        while tpr < min_dur {
            tpr *= 2;
        }
        tpr
    }

    fn switch_group(&mut self, group: usize) {
        if self.songs.group(group).is_none() {
            return;
        }
        self.current_group = group;
        self.tracks = Self::make_tracks(&self.songs, group);
        self.ticks_per_row = Self::auto_scale(&self.tracks);

        let group_tracks = self.songs.group(group).unwrap();
        let arcs: [Arc<Track>; 4] = [
            Arc::new(group_tracks[0].clone()),
            Arc::new(group_tracks[1].clone()),
            Arc::new(group_tracks[2].clone()),
            Arc::new(group_tracks[3].clone()),
        ];
        let mut st = self.audio.lock().unwrap();
        st.play_score(arcs, &self.instruments);
        st.muted = [false; 4];
        st.nosound = self.paused;
        for (i, t) in self.tracks.iter().enumerate() {
            st.muted[i] = !t.enabled;
        }
    }

    fn toggle_track(&mut self, idx: usize) {
        self.tracks[idx].enabled = !self.tracks[idx].enabled;
        let mut st = self.audio.lock().unwrap();
        st.muted[idx] = !self.tracks[idx].enabled;
    }

    fn set_tempo(&mut self, tempo: u32) {
        let mut st = self.audio.lock().unwrap();
        st.tempo = tempo;
    }

    /// Double or halve the ticks-per-row, clamped to [MIN_TICKS_PER_ROW, 2^16 * MIN].
    fn zoom_out(&mut self) {
        self.ticks_per_row = (self.ticks_per_row * 2).min(MIN_TICKS_PER_ROW << 16);
    }
    fn zoom_in(&mut self) {
        self.ticks_per_row = (self.ticks_per_row / 2).max(MIN_TICKS_PER_ROW);
    }

    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        let mut st = self.audio.lock().unwrap();
        st.nosound = paused;
    }

    /// Step one VBL tick while paused (advances the audio sequencer; the
    /// visualizer reads timeclock from the audio state so it follows automatically).
    fn step_tick(&mut self) {
        let mut st = self.audio.lock().unwrap();
        let was = st.nosound;
        st.nosound = false;
        st.vbl_tick(&self.instruments.clone());
        st.nosound = was;
    }

    /// Read (timeclock, tempo) from the audio thread for display / piano-roll.
    fn audio_pos(&self) -> (u32, u32) {
        let st = self.audio.lock().unwrap();
        (st.timeclock, st.tempo)
    }
}

// ---------------------------------------------------------------------------
// Terminal rendering
// ---------------------------------------------------------------------------

const MAX_COL_W: usize = 12; // max column width — keeps the roll narrow

/// Current instrument name for a track (from the last seen SetInstrument event).
fn instrument_name(slot: u8) -> &'static str {
    INSTRUMENT_NAMES
        .get(slot as usize)
        .copied()
        .unwrap_or("Unknown")
}

fn render(stdout: &mut io::Stdout, app: &App, timeclock: u32, tempo: u32) -> io::Result<()> {
    let ticks_per_row = app.ticks_per_row;
    let (term_w, term_h) = terminal::size()?;
    let term_w = term_w as usize;
    let term_h = term_h as usize;
    // rows available for the piano-roll grid
    let grid_rows = term_h.saturating_sub(5); // header + track-hdr + sep + sep + footer
    let center_row = grid_rows / 2;
    // Row index (in ticks_per_row units) at the playhead
    let now_row_idx: i64 = (timeclock / ticks_per_row) as i64;

    execute!(
        stdout,
        cursor::MoveTo(0, 0),
        terminal::Clear(ClearType::All)
    )?;

    // ── Header ──────────────────────────────────────────────────────────────
    let status = if app.paused { "PAUSED " } else { "PLAYING" };
    let tpr_label = ticks_per_row / MIN_TICKS_PER_ROW;
    let header = format!(
        " Group {}   Tempo {:>3}   Timeclock {:>9}   Scale x{:<4}  {}",
        app.current_group, tempo, timeclock, tpr_label, status
    );
    let header = pad_or_clip(&header, term_w);
    execute!(
        stdout,
        SetForegroundColor(Color::Yellow),
        SetAttribute(Attribute::Bold),
        Print(&header),
        SetAttribute(Attribute::Reset),
        ResetColor,
        cursor::MoveToNextLine(1),
    )?;

    // ── Track headers ────────────────────────────────────────────────────────
    // Column width is capped so the display stays narrow; the whole roll is
    // centred within the terminal.
    let n_tracks = app.tracks.len().max(1);
    let roll_col_w = ((term_w.saturating_sub(2)) / n_tracks).min(MAX_COL_W);
    let roll_w = 2 + roll_col_w * n_tracks; // total chars used by the roll
    let margin = (term_w.saturating_sub(roll_w)) / 2;
    let pad_left = " ".repeat(margin);
    let mut hdr_line = format!("{pad_left}  "); // margin + 2-char gutter
    for (i, t) in app.tracks.iter().enumerate() {
        let mute_marker = if t.enabled { ' ' } else { 'M' };
        let inst = instrument_name(t.instrument);
        let cell = format!("[{}{mute_marker}] {}", i + 1, inst);
        hdr_line.push_str(&pad_or_clip(&cell, roll_col_w));
    }
    let hdr_line = pad_or_clip(&hdr_line, term_w);
    execute!(
        stdout,
        SetForegroundColor(Color::Cyan),
        SetAttribute(Attribute::Bold),
        Print(&hdr_line),
        SetAttribute(Attribute::Reset),
        ResetColor,
        cursor::MoveToNextLine(1),
    )?;

    // ── Top separator ────────────────────────────────────────────────────────
    let sep = format!("{}{}", pad_left, "─".repeat(roll_w));
    execute!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
        Print(&sep),
        ResetColor,
        cursor::MoveToNextLine(1),
    )?;

    // ── Piano-roll grid ──────────────────────────────────────────────────────
    // display_row 0 = top (future); center_row = now (playhead); bottom = past.
    // row_tick for display_row d = (now_row_idx + center_row - d) * ticks_per_row
    for display_row in 0..grid_rows {
        let is_center = display_row == center_row;
        let row_offset = center_row as i64 - display_row as i64; // +ve = future, -ve = past
        let row_idx = now_row_idx + row_offset;
        let row_tick = if row_idx < 0 {
            // Before song start — blank row
            let mut line = String::from(&pad_left);
            if is_center {
                line.push_str("▶ ");
                line.push_str(&" ".repeat(roll_col_w * n_tracks));
            } else {
                line.push_str(&" ".repeat(2 + roll_col_w * n_tracks));
            }
            execute!(
                stdout,
                Print(pad_or_clip(&line, term_w)),
                cursor::MoveToNextLine(1)
            )?;
            continue;
        } else {
            (row_idx as u32) * ticks_per_row
        };

        let mut line = String::from(&pad_left);

        // 2-char gutter: playhead marker
        if is_center {
            line.push_str("▶ ");
        } else {
            line.push_str("  ");
        }

        let col_w = roll_col_w;

        for t in &app.tracks {
            let cell = match t.event_at(row_tick, ticks_per_row) {
                Some((ev, is_start)) if !ev.is_rest => {
                    if is_start {
                        format!(" {:<width$}", ev.label, width = col_w.saturating_sub(1))
                    } else {
                        format!(" │{:<width$}", "", width = col_w.saturating_sub(2))
                    }
                }
                Some((_, is_start)) => {
                    if is_start {
                        format!(" ·{:<width$}", "", width = col_w.saturating_sub(2))
                    } else {
                        " ".repeat(col_w)
                    }
                }
                None => " ".repeat(col_w),
            };
            line.push_str(&pad_or_clip(&cell, col_w));
        }

        let line = pad_or_clip(&line, term_w);

        // Colour: center = bright green; near-future = white; far-future/past = dim
        if is_center {
            execute!(
                stdout,
                SetForegroundColor(Color::Green),
                SetAttribute(Attribute::Bold),
                Print(&line),
                SetAttribute(Attribute::Reset),
                ResetColor,
                cursor::MoveToNextLine(1),
            )?;
        } else {
            // Distance from center in rows
            let dist = (display_row as i64 - center_row as i64).unsigned_abs() as usize;
            let is_future = display_row < center_row;
            let color = if dist <= 2 {
                if is_future {
                    Color::White
                } else {
                    Color::Grey
                }
            } else if dist <= 8 {
                Color::Grey
            } else {
                Color::DarkGrey
            };
            execute!(
                stdout,
                SetForegroundColor(color),
                SetAttribute(Attribute::Reset),
                Print(&line),
                ResetColor,
                cursor::MoveToNextLine(1),
            )?;
        }
    }

    // ── Bottom separator ─────────────────────────────────────────────────────
    execute!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
        Print(&sep),
        ResetColor,
        cursor::MoveToNextLine(1),
    )?;

    // ── Footer ───────────────────────────────────────────────────────────────
    let footer = " Space=pause  +/-=tempo  Enter=step  1-4=track  ←/→=group  [/]=zoom  Q=quit";
    let footer = pad_or_clip(footer, term_w);
    execute!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
        Print(&footer),
        ResetColor,
    )?;

    stdout.flush()
}

/// Pad a string with spaces or clip it to exactly `width` terminal columns.
fn pad_or_clip(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        s.chars().take(width).collect()
    } else {
        format!("{}{:pad$}", s, "", pad = width - len)
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let songs = SongLibrary::load(&base.join("game/songs"))
        .expect("Could not load game/songs — run from the project root");
    let instruments = Instruments::load(&base.join("game/v6"))
        .expect("Could not load game/v6 — run from the project root");

    // Optional CLI argument: song group (0-6, default 3 = intro)
    let args: Vec<String> = std::env::args().collect();
    let initial_group = args
        .get(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(3);

    // ── SDL2 audio setup ────────────────────────────────────────────────────
    let sdl = sdl2::init()?;
    let audio_ss = sdl.audio()?;
    let desired = AudioSpecDesired {
        freq: Some(SAMPLE_RATE as i32),
        channels: Some(2),
        samples: Some(512),
    };

    let audio_state = Arc::new(Mutex::new(SeqState::new()));
    let cb_state = Arc::clone(&audio_state);
    let cb_inst = instruments.clone();

    let device = audio_ss.open_playback(None, &desired, |_spec| SynthCallback {
        state: cb_state,
        instruments: cb_inst,
    })?;
    device.resume();

    // Start playback for the initial group
    {
        let group_tracks = songs
            .group(initial_group)
            .unwrap_or_else(|| songs.group(0).expect("no groups"));
        let arcs: [Arc<Track>; 4] = [
            Arc::new(group_tracks[0].clone()),
            Arc::new(group_tracks[1].clone()),
            Arc::new(group_tracks[2].clone()),
            Arc::new(group_tracks[3].clone()),
        ];
        audio_state.lock().unwrap().play_score(arcs, &instruments);
    }

    // ── Terminal setup ──────────────────────────────────────────────────────
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    // ── Application state ───────────────────────────────────────────────────
    let mut app = App::new(songs, instruments, initial_group, Arc::clone(&audio_state));

    // Loop
    'main: loop {
        // ── Input ────────────────────────────────────────────────────────────
        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            {
                match code {
                    KeyCode::Char('q') | KeyCode::Esc => break 'main,
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => break 'main,

                    KeyCode::Char(' ') => {
                        app.set_paused(!app.paused);
                    }
                    KeyCode::Enter => {
                        if app.paused {
                            app.step_tick();
                        }
                    }

                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        let cur = app.audio_pos().1;
                        app.set_tempo((cur + 10).min(500));
                    }
                    KeyCode::Char('-') | KeyCode::Char('_') => {
                        let cur = app.audio_pos().1;
                        app.set_tempo(cur.saturating_sub(10).max(10));
                    }

                    KeyCode::Char('1') => {
                        app.toggle_track(0);
                    }
                    KeyCode::Char('2') => {
                        app.toggle_track(1);
                    }
                    KeyCode::Char('3') => {
                        app.toggle_track(2);
                    }
                    KeyCode::Char('4') => {
                        app.toggle_track(3);
                    }

                    KeyCode::Char('[') | KeyCode::Char(',') => {
                        app.zoom_in();
                    }
                    KeyCode::Char(']') | KeyCode::Char('.') => {
                        app.zoom_out();
                    }

                    KeyCode::Left => {
                        if app.current_group > 0 {
                            app.switch_group(app.current_group - 1);
                        }
                    }
                    KeyCode::Right => {
                        let next = app.current_group + 1;
                        if next < SongLibrary::GROUPS {
                            app.switch_group(next);
                        }
                    }
                    _ => {}
                }
            }
        }

        // ── Render ───────────────────────────────────────────────────────────
        let (timeclock, tempo) = app.audio_pos();
        render(&mut stdout, &app, timeclock, tempo)?;

        std::thread::sleep(Duration::from_millis(16)); // ~60 fps cap
    }

    // Cleanup
    execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    Ok(())
}
