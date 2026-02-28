/// Audio score / music-track parser for the Faery Tale Adventure `songs` file.
///
/// # File format  (ported from `fmain2.c` `read_score()` + `gdriver.asm`)
///
/// The `songs` file stores up to 28 tracks (4 voices × 7 song groups).
/// For each track:
///   * 4 bytes  – `packlen` (big-endian i32): number of 16-bit words in the track
///   * `packlen * 2` bytes – raw track bytes (sequence of 2-byte command/value pairs)
///
/// The parser stops when the 5900-byte score buffer would be exceeded, exactly
/// matching the original C code.
///
/// # Track event encoding  (from `gdriver.asm` `_vblank_server` → `newnote`)
///
/// Each event is a pair of bytes `(command, value)`:
///
/// | command byte           | meaning                          |
/// |------------------------|----------------------------------|
/// | 0 – 127                | Note: pitch index into PTABLE    |
/// | 128  (0x80)            | Rest                             |
/// | 129  (0x81)            | Set instrument (value & 0x0f)    |
/// | 144  (0x90)            | Set tempo (value = new tempo)    |
/// | 255  (0xFF)            | End track (value ≠ 0 → loop)    |
/// | other (bit 7 set)      | Ignored / skip                   |
///
/// For notes and rests the `value` byte encodes the duration as an index into
/// `NOTE_DURATIONS` (bits 7 and 6 are masked off, per the original asm).
///
/// # Playback notes
///
/// The intro music uses **tracks 12–15** (`track[12]..track[15]` in the
/// original, corresponding to `self.tracks[12..=15]`).
///
/// Each song group occupies four consecutive track slots (one per Amiga Paula
/// voice) and the offset into the table is always a multiple of four.
///
/// Amiga Paula period → frequency: `freq = AMIGA_CLOCK_NTSC / period`.

use std::path::Path;
use std::fs;

// ---------------------------------------------------------------------------
// Amiga hardware constants
// ---------------------------------------------------------------------------

/// NTSC Amiga Paula clock rate (Hz).  Used to convert period → frequency.
///
/// The Faery Tale Adventure was developed and released for NTSC machines.
/// PAL machines run Paula at 3,546,895 Hz and produce a slightly lower pitch.
pub const AMIGA_CLOCK_NTSC: u32 = 3_579_545;

/// NTSC vertical-blank interrupt rate (Hz).
///
/// The NTSC Amiga fires the VBL interrupt at 60 Hz.  The music sequencer
/// runs inside this interrupt (`_vblank_server` in `gdriver.asm`): each VBL
/// adds `tempo` to `timeclock`, so the effective clock-unit rate is
/// `tempo × VBL_RATE_HZ` units per second.  At the default tempo of 150 this
/// gives **9 000 timeclock units per second**.
///
/// PAL machines run at 50 Hz; using the wrong rate would make all music play
/// ~17% slower than intended.
pub const VBL_RATE_HZ: u32 = 60;

/// Default music tempo (matches `move.w #150,tempo(a1)` in `_init_music`).
pub const DEFAULT_TEMPO: u32 = 150;

/// Timeclock units per second at the default tempo on NTSC hardware.
///
/// `= DEFAULT_TEMPO × VBL_RATE_HZ = 150 × 60 = 9 000`
pub const TIMECLOCK_RATE: u32 = DEFAULT_TEMPO * VBL_RATE_HZ;

// ---------------------------------------------------------------------------
// Lookup tables (verbatim from gdriver.asm)
// ---------------------------------------------------------------------------

/// Note period/wave-offset table, ported from `ptable` in `gdriver.asm`.
///
/// Each entry is `(period, wave_offset)`.  The note pitch byte (0–83) indexes
/// directly into this table.  `period` is an Amiga Paula period register value;
/// `wave_offset` is an offset in 16-bit words into the instrument waveform.
///
/// Frequency ≈ `AMIGA_CLOCK_NTSC / period` Hz.
pub const PTABLE: [(u16, u16); 84] = [
    // Row 0 – octave 0 low  (pitch 0–11)
    (1440, 0),  (1356, 0),  (1280, 0),  (1208, 0),  (1140, 0),  (1076, 0),
    (1016, 0),  (960,  0),  (906,  0),  (856,  0),  (808,  0),  (762,  0),
    // Row 1 – octave 0 high (pitch 12–23)
    (1016, 0),  (960,  0),  (906,  0),  (856,  0),  (808,  0),  (762,  0),
    (720,  0),  (678,  0),  (640,  0),  (604,  0),  (570,  0),  (538,  0),
    // Row 2 – octave 1      (pitch 24–35)
    (508,  0),  (480,  0),  (453,  0),  (428,  0),  (404,  0),  (381,  0),
    (360,  0),  (339,  0),  (320,  0),  (302,  0),  (285,  0),  (269,  0),
    // Row 3 – octave 2      (pitch 36–47, wave_offset 16)
    (508, 16),  (480, 16),  (453, 16),  (428, 16),  (404, 16),  (381, 16),
    (360, 16),  (339, 16),  (320, 16),  (302, 16),  (285, 16),  (269, 16),
    // Row 4 – octave 3      (pitch 48–59, wave_offset 24)
    (508, 24),  (480, 24),  (453, 24),  (428, 24),  (404, 24),  (381, 24),
    (360, 24),  (339, 24),  (320, 24),  (302, 24),  (285, 24),  (269, 24),
    // Row 5 – octave 4      (pitch 60–71, wave_offset 28)
    (508, 28),  (480, 28),  (453, 28),  (428, 28),  (404, 28),  (381, 28),
    (360, 28),  (339, 28),  (320, 28),  (302, 28),  (285, 28),  (269, 28),
    // Row 6 – octave 5 high (pitch 72–83, wave_offset 28)
    (254, 28),  (240, 28),  (226, 28),  (214, 28),  (202, 28),  (190, 28),
    (180, 28),  (170, 28),  (160, 28),  (151, 28),  (143, 28),  (135, 28),
];

/// Duration table, ported from `notevals` in `gdriver.asm`.
///
/// Each entry is a clock-tick count for one note duration.  The duration index
/// byte (bits 6–0 of the `value` byte, after masking off bits 7 and 6) indexes
/// into this table.  The tempo register scales all values proportionally.
///
/// At the default tempo of 150 ticks/frame the values correspond roughly to
/// whole, half, quarter, eighth, sixteenth note lengths in seven time
/// signatures (see original source comments).
pub const NOTE_DURATIONS: [u16; 64] = [
    // Group 0 – 4/4
    26880, 13440, 6720, 3360, 1680, 840, 420, 210,
    // Group 1 – 6/8
    40320, 20160, 10080, 5040, 2520, 1260, 630, 315,
    // Group 2 – 3/4
    17920, 8960, 4480, 2240, 1120, 560, 280, 140,
    // Group 3 – 4/4 (duplicate, different base)
    26880, 13440, 6720, 3360, 1680, 840, 420, 210,
    // Group 4 – 7/8
    21504, 10752, 5376, 2688, 1344, 672, 336, 168,
    // Group 5 – 5/4
    32256, 16128, 8064, 4032, 2016, 1008, 504, 252,
    // Group 6 – 3/4 alt
    23040, 11520, 5760, 2880, 1440, 720, 360, 180,
    // Group 7 – 9/8
    34560, 17280, 8640, 4320, 2160, 1080, 540, 270,
];

// ---------------------------------------------------------------------------
// Track event model
// ---------------------------------------------------------------------------

/// One decoded event from a track stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackEvent {
    /// Play a pitched note.
    ///
    /// * `pitch`        — index into [`PTABLE`] (0–83)
    /// * `duration_idx` — index into [`NOTE_DURATIONS`] (0–63, bits 7/6 are masked)
    Note { pitch: u8, duration_idx: u8 },

    /// Silence for a duration.
    ///
    /// * `duration_idx` — index into [`NOTE_DURATIONS`]
    Rest { duration_idx: u8 },

    /// Change the waveform for this voice.
    ///
    /// * `slot` — instrument slot (0–15); looked up via the instrument handle
    ///   table to get the `wave_num` stored in the voice data.
    SetInstrument { slot: u8 },

    /// Change the global tempo.
    ///
    /// * `value` — new value written directly to the `tempo` register (default 150)
    SetTempo { value: u8 },

    /// End of track.
    ///
    /// * `looping` — if `true`, the track wraps back to the beginning
    End { looping: bool },

    /// Unrecognised command (command byte has bit 7 set but matches no known code).
    Unknown { command: u8, value: u8 },
}

/// A parsed music track — a sequence of [`TrackEvent`]s decoded from the raw
/// two-byte-pair stream.
pub type Track = Vec<TrackEvent>;

// ---------------------------------------------------------------------------
// SongLibrary
// ---------------------------------------------------------------------------

/// Parsed contents of the `songs` file.
///
/// Holds up to 28 tracks split across seven four-voice song groups.  Indexed
/// as `tracks[0..28]`, matching the original C `track[]` array.
///
/// Tracks 12–15 are the intro music (voiced by the four Amiga Paula channels).
#[derive(Debug)]
pub struct SongLibrary {
    /// All decoded tracks in file order.  Length ≤ 28.
    pub tracks: Vec<Track>,
}

impl SongLibrary {
    /// Number of song groups (4 tracks each).
    pub const GROUPS: usize = 7;
    /// Tracks per group (one per Amiga Paula voice).
    pub const VOICES: usize = 4;
    /// Maximum total tracks.
    pub const MAX_TRACKS: usize = Self::GROUPS * Self::VOICES;
    /// Maximum raw score data in bytes (matches `SCORE_SZ` in `fmain.c`).
    const SCORE_SZ: usize = 5900;

    /// Index of the first intro music track (`track[12]` in original).
    pub const INTRO_TRACK_BASE: usize = 12;

    /// Load and decode the `songs` file at `path`.
    ///
    /// Returns `None` if the file cannot be read.
    pub fn load(path: &Path) -> Option<Self> {
        let data = fs::read(path).ok()?;
        Some(Self::parse(&data))
    }

    /// Parse raw `songs` file bytes into a [`SongLibrary`].
    pub fn parse(data: &[u8]) -> Self {
        let mut tracks = Vec::new();
        let mut offset = 0usize;
        let mut sc_load = 0usize; // bytes consumed into the 5900-byte score buffer

        for _ in 0..Self::MAX_TRACKS {
            if offset + 4 > data.len() {
                break;
            }
            // Read 4-byte big-endian packlen
            let packlen = i32::from_be_bytes(
                data[offset..offset + 4].try_into().unwrap(),
            );
            offset += 4;

            if packlen <= 0 {
                break;
            }

            let byte_count = (packlen as usize) * 2;

            // Original: if (packlen * 2 + sc_load) > 5900 break
            if byte_count + sc_load > Self::SCORE_SZ {
                break;
            }

            if offset + byte_count > data.len() {
                break;
            }

            let track_bytes = &data[offset..offset + byte_count];
            offset += byte_count;
            sc_load += byte_count;

            tracks.push(Self::decode_track(track_bytes));
        }

        SongLibrary { tracks }
    }

    /// Decode a raw (command, value) byte stream into a [`Track`].
    ///
    /// Public so that unit tests and tooling can exercise the decoder directly.
    pub fn parse_track_bytes(bytes: &[u8]) -> Track {
        Self::decode_track(bytes)
    }

    fn decode_track(bytes: &[u8]) -> Track {
        let mut events = Vec::new();
        let mut i = 0;

        while i + 1 < bytes.len() {
            let cmd = bytes[i];
            let val = bytes[i + 1];
            i += 2;

            if cmd < 128 {
                // Pitched note: bits 7 and 6 of value masked per gdriver.asm
                events.push(TrackEvent::Note {
                    pitch: cmd,
                    duration_idx: val & 0x3f,
                });
            } else {
                match cmd {
                    128 => {
                        // Rest
                        events.push(TrackEvent::Rest {
                            duration_idx: val & 0x3f,
                        });
                    }
                    129 => {
                        // Set instrument
                        events.push(TrackEvent::SetInstrument {
                            slot: val & 0x0f,
                        });
                    }
                    0x90 => {
                        // Set tempo
                        events.push(TrackEvent::SetTempo { value: val });
                    }
                    0xff => {
                        // End of track
                        events.push(TrackEvent::End {
                            looping: val != 0,
                        });
                        break; // nothing meaningful after end
                    }
                    _ => {
                        // Unrecognised — original asm skips these
                        events.push(TrackEvent::Unknown { command: cmd, value: val });
                    }
                }
            }
        }

        events
    }

    /// Return the four tracks for the requested song group.
    ///
    /// Groups are numbered 0–6.  Group 3 (`base = track[12]`) is the intro.
    /// Returns `None` if any of the four tracks for the group are absent.
    pub fn group(&self, group: usize) -> Option<[&Track; 4]> {
        let base = group * Self::VOICES;
        if base + 3 >= self.tracks.len() {
            return None;
        }
        Some([
            &self.tracks[base],
            &self.tracks[base + 1],
            &self.tracks[base + 2],
            &self.tracks[base + 3],
        ])
    }

    /// Return the four intro music tracks (`track[12..=15]`).
    pub fn intro_tracks(&self) -> Option<[&Track; 4]> {
        self.group(Self::INTRO_TRACK_BASE / Self::VOICES)
    }

    /// Compute the approximate playback frequency (Hz) for a pitch index.
    ///
    /// Returns `None` if the pitch index is out of range.
    pub fn pitch_freq(pitch: usize) -> Option<f32> {
        PTABLE.get(pitch).map(|(period, _)| {
            AMIGA_CLOCK_NTSC as f32 / *period as f32
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse the real `game/songs` file (present in the repository).
    fn load_songs() -> SongLibrary {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("game/songs");
        SongLibrary::load(&path).expect("game/songs must be readable")
    }

    #[test]
    fn test_songs_track_count() {
        let lib = load_songs();
        // Original reads up to 28 tracks; the file should have all of them.
        assert!(
            lib.tracks.len() > 0,
            "expected at least one track, got {}",
            lib.tracks.len()
        );
        println!("songs: {} tracks loaded", lib.tracks.len());
    }

    #[test]
    fn test_intro_tracks_present() {
        let lib = load_songs();
        let intro = lib.intro_tracks();
        assert!(
            intro.is_some(),
            "intro tracks (indices 12–15) should be present; only {} tracks loaded",
            lib.tracks.len()
        );
    }

    #[test]
    fn test_track_has_end_event() {
        let lib = load_songs();
        // Every non-empty track should contain an End event (or loop).
        for (idx, track) in lib.tracks.iter().enumerate() {
            let has_end = track.iter().any(|e| matches!(e, TrackEvent::End { .. }));
            assert!(has_end, "track {} has no End event", idx);
        }
    }

    #[test]
    fn test_intro_track_starts_with_note_or_instrument() {
        let lib = load_songs();
        if let Some(intro) = lib.intro_tracks() {
            for (voice, track) in intro.iter().enumerate() {
                let first = track.first();
                assert!(
                    matches!(
                        first,
                        Some(TrackEvent::Note { .. })
                            | Some(TrackEvent::Rest { .. })
                            | Some(TrackEvent::SetInstrument { .. })
                            | Some(TrackEvent::SetTempo { .. })
                    ),
                    "intro voice {} starts with unexpected event {:?}",
                    voice,
                    first
                );
            }
        }
    }

    #[test]
    fn test_note_durations_table_size() {
        assert_eq!(NOTE_DURATIONS.len(), 64);
    }

    #[test]
    fn test_ptable_size() {
        assert_eq!(PTABLE.len(), 84);
    }

    #[test]
    fn test_pitch_freq_range() {
        // Pitch 30 → period 360 (row 2, index 6 within the row: 360,0)
        // freq = 3579545 / 360 ≈ 9943 Hz
        let freq = SongLibrary::pitch_freq(30).unwrap();
        let expected = AMIGA_CLOCK_NTSC as f32 / 360.0;
        assert!((freq - expected).abs() < 1.0, "freq mismatch: {}", freq);
    }

    #[test]
    fn test_decode_simple_track() {
        // Manually construct a minimal two-event track: note(4, 3) then end(loop)
        let bytes: &[u8] = &[0x04, 0x03, 0xFF, 0x01];
        let t = SongLibrary::parse_track_bytes(bytes);
        assert_eq!(
            t,
            vec![
                TrackEvent::Note { pitch: 4, duration_idx: 3 },
                TrackEvent::End { looping: true },
            ]
        );
    }

    #[test]
    fn test_decode_rest_and_endtrack() {
        let bytes: &[u8] = &[0x80, 0x05, 0xFF, 0x00];
        let t = SongLibrary::parse_track_bytes(bytes);
        assert_eq!(
            t,
            vec![
                TrackEvent::Rest { duration_idx: 5 },
                TrackEvent::End { looping: false },
            ]
        );
    }

    #[test]
    fn test_decode_set_instrument() {
        // command=129 (0x81), value=0x1f → slot = 0x1f & 0x0f = 0x0f = 15
        let bytes: &[u8] = &[0x81, 0x1f, 0xFF, 0x00];
        let t = SongLibrary::parse_track_bytes(bytes);
        assert_eq!(
            t[0],
            TrackEvent::SetInstrument { slot: 15 }
        );
    }

    #[test]
    fn test_decode_set_tempo() {
        let bytes: &[u8] = &[0x90, 0x64, 0xFF, 0x00];
        let t = SongLibrary::parse_track_bytes(bytes);
        assert_eq!(t[0], TrackEvent::SetTempo { value: 100 });
    }

    #[test]
    fn test_duration_index_bits_masked() {
        // Bits 7 and 6 of value byte must be masked off
        // value = 0xff → 0xff & 0x3f = 0x3f = 63
        let bytes: &[u8] = &[0x00, 0xff, 0xFF, 0x00];
        let t = SongLibrary::parse_track_bytes(bytes);
        assert_eq!(t[0], TrackEvent::Note { pitch: 0, duration_idx: 63 });
    }
}
