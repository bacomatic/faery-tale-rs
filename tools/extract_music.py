#!/usr/bin/env python3
"""Music + instrument extractor for The Faery Tale Adventure (T2.6).

Inputs (original game data, read-only):

* ``songs`` -- 28 packed track event streams, each ``i32 packlen`` (big-endian,
  in words) followed by ``packlen*2`` bytes of ``(command, value)`` pairs
  (``fmain2.c:760-776``). A "song" is 4 consecutive tracks, one per Paula
  voice (``fmain.c:2953``, ``gdriver.asm:357-368``).
* ``v6`` -- 8 x 128-byte signed waveforms (``fmain.c:663,932``) followed by
  10 x 256-byte volume envelopes (``fmain.c:664,934``).

The event format, period table, duration table and instrument table are all
read from the player (``gdriver.asm``) and ``fmain.c`` at run time; nothing
is transcribed by hand. Outputs (relative to ``assets/audio``):

* ``music/track_NN.json`` x28 -- decoded events with the raw bytes alongside,
* ``music/format.json`` -- command encoding, period table, duration table,
  instrument table, timing model (all cited),
* ``music/previews/track_NN.wav`` x28 and ``previews/song_N.wav`` x7 -- rough,
  non-authoritative renders (see ``music/previews/README.md``),
* ``instruments/waveforms.json`` and ``instruments/envelopes.json``.

Usage::

    python tools/extract_music.py            # everything -> assets/audio
"""

from __future__ import annotations

import argparse
import math
import re
import struct
import sys
import wave
from pathlib import Path

import numpy as np

TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import asset_common as ac  # noqa: E402
import extract_table as et  # noqa: E402

REPO_ROOT = TOOLS_DIR.parent

NUM_TRACKS = 4 * 7                # fmain2.c:766
VOICES_PER_SONG = 4               # fmain.c:2953
NUM_WAVEFORMS, WAVEFORM_LEN = 8, 128    # fmain.c:663  S_WAVBUF = 128 * 8
NUM_ENVELOPES, ENVELOPE_LEN = 10, 256   # fmain.c:664  S_VOLBUF = 10 * 256
WAVBUF = NUM_WAVEFORMS * WAVEFORM_LEN
VOLBUF = NUM_ENVELOPES * ENVELOPE_LEN

INITIAL_TEMPO = 150               # gdriver.asm:429
NOTE_GAP = 300                    # gdriver.asm:150
CMD_REST = 128                    # gdriver.asm:127
CMD_INSTRUMENT = 129              # gdriver.asm:130
CMD_TEMPO = 0x90                  # gdriver.asm:133
CMD_END = 255                     # gdriver.asm:136

# Preview renderer (non-authoritative, see previews/README.md)
PAULA_CLOCK_NTSC = 3579545        # Hz (NTSC; the game targets NTSC machines)
VBLANK_HZ = 60                    # NTSC vertical blank
PREVIEW_RATE = 22050
PREVIEW_MAX_SECONDS = 120

# Which song plays where (song = track // 4). All from setmood() unless noted.
SONGS = [
    {"name": "day", "title": "Overworld, daytime",
     "cue": "Outdoor daytime theme: setmood() picks tracks 0-3 when the hero is alive, "
            "outside the astral box, not in battle, in an outdoor region (<= 7) and "
            "lightlevel > 120.",
     "citations": ["src/fmain.c:2948", "src/fmain.c:2951-2954"]},
    {"name": "combat", "title": "Combat",
     "cue": "Battle theme: setmood() picks tracks 4-7 while battleflag is set "
            "(started with a full reset when battle begins).",
     "citations": ["src/fmain.c:2942", "src/fmain.c:2951-2954"]},
    {"name": "night", "title": "Overworld, night",
     "cue": "Outdoor night theme: setmood() falls through to tracks 8-11 when "
            "lightlevel <= 120 outdoors.",
     "citations": ["src/fmain.c:2949", "src/fmain.c:2951-2954"]},
    {"name": "title", "title": "Title / intro",
     "cue": "Title theme: main() starts tracks 12-15 right after loading the score, "
            "before the intro pages; stopped at fmain.c:1244 before the game begins.",
     "citations": ["src/fmain.c:1182", "src/fmain.c:1244"]},
    {"name": "astral", "title": "Astral (spirit) plane",
     "cue": "Astral-plane theme: setmood() picks tracks 16-19 while the hero is inside "
            "the box (0x2400,0x8200)-(0x3100,0x8a00), the 'astral plane' extent.",
     "citations": ["src/fmain.c:2939-2941", "src/fmain.c:353"]},
    {"name": "indoors", "title": "Indoors / dungeons",
     "cue": "Indoor theme: setmood() picks tracks 20-23 when region_num > 7, i.e. region 8 "
            "(inside of buildings) or 9 (dungeons and caves); in region 9 instrument 10 "
            "is swapped to 0x0307.",
     "citations": ["src/fmain.c:2943-2947", "src/fmain.c:624-625"]},
    {"name": "death", "title": "Death",
     "cue": "Death theme: setmood() picks tracks 24-27 when the hero's vitality is 0.",
     "citations": ["src/fmain.c:2938", "src/fmain.c:2951-2954"]},
]


# --------------------------------------------------------------------------- #
# Source tables
# --------------------------------------------------------------------------- #
def read_asm_words(asm_path: Path, label: str) -> tuple[list[int], str]:
    """Return the ``dc.w`` values under a bare ``label`` line and their line range."""
    lines = asm_path.read_text(errors="replace").splitlines()
    start = next(i for i, l in enumerate(lines) if l.strip() == label)
    values: list[int] = []
    first = last = None
    for i in range(start + 1, len(lines)):
        line = lines[i]
        if not line.strip() or line[0] in "*;":
            continue  # blank or comment line (a '*' in column 1 comments the line)
        m = re.match(r"\s+dc\.w\s+(.+)", line, re.IGNORECASE)
        if not m:
            break
        values.extend(et.parse_asm_values(m.group(1)))
        first = first or i + 1
        last = i + 1
    return values, f"src/{asm_path.name}:{first}-{last}"


class Source:
    def __init__(self, src_dir: Path):
        asm = src_dir / "gdriver.asm"
        flat, self.ptable_cite = read_asm_words(asm, "ptable")
        self.ptable = [(flat[i], flat[i + 1]) for i in range(0, len(flat), 2)]
        self.notevals, self.notevals_cite = read_asm_words(asm, "notevals")
        arrays = et.extract_c_arrays(src_dir / "fmain.c")
        self.new_wave = arrays["new_wave"]["values"]
        line = arrays["new_wave"]["line"]
        self.new_wave_cite = f"src/fmain.c:{line}-{line + 3}"


def instrument_word(word: int) -> tuple[int, int]:
    """``move.w (a2,d2),wave_num(a3)`` writes both bytes: high -> wave_num, low -> vol_num.

    gdriver.asm:26-27 (wave_num at vbase+0, vol_num at vbase+1), :241, :377-380.
    """
    return (word >> 8) & 0xFF, word & 0xFF


def wave_slice(offset: int) -> tuple[int, int]:
    """Byte start and length of the waveform slice for a ptable offset (gdriver.asm:184-188)."""
    return offset * 4, (32 - offset) * 2


def note_name(freq: float) -> str:
    midi = round(69 + 12 * math.log2(freq / 440.0))
    names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
    return f"{names[midi % 12]}{midi // 12 - 1}"


def period_entry(src: Source, index: int) -> dict:
    period, offset = src.ptable[index]
    start, length = wave_slice(offset)
    freq = PAULA_CLOCK_NTSC / (period * length)
    return {"index": index, "period": period, "wave_offset": offset,
            "wave_start_byte": start, "wave_len_bytes": length,
            "pitch_hz_ntsc": round(freq, 2), "note_name": note_name(freq)}


# --------------------------------------------------------------------------- #
# songs
# --------------------------------------------------------------------------- #
def split_tracks(data: bytes) -> list[dict]:
    """Split ``songs`` into tracks; the 28 tracks must tile the file exactly."""
    tracks = []
    pos = 0
    while pos < len(data):
        (packlen,) = struct.unpack_from(">i", data, pos)
        body = data[pos + 4:pos + 4 + packlen * 2]
        if len(body) != packlen * 2:
            raise ValueError(f"track {len(tracks)} at {pos}: truncated ({len(body)} of {packlen * 2} bytes)")
        tracks.append({"offset": pos, "packlen": packlen, "body": body})
        pos += 4 + packlen * 2
    if pos != len(data) or len(tracks) != NUM_TRACKS:
        raise ValueError(f"expected {NUM_TRACKS} tracks tiling {len(data)} bytes, got {len(tracks)} ending at {pos}")
    return tracks


def decode_event(c: int, v: int, src: Source) -> dict:
    """Decode one ``(command, value)`` pair the way ``newnote`` dispatches it (gdriver.asm:116-139)."""
    ev: dict = {"raw": [c, v]}
    if not c & 0x80 or c == CMD_REST:                       # gdriver.asm:124-128
        code = v & 0x3F                                      # gdriver.asm:142-143 clear bits 7, 6
        ev.update({
            "kind": "rest" if c == CMD_REST else "note",
            "duration_code": code,
            "duration_counts": src.notevals[code],            # gdriver.asm:145-148
            "tie": bool(v & 0x40),                           # gdriver.asm:143 (cleared, unused)
            "chord": bool(v & 0x80),                         # gdriver.asm:142 (cleared, unused)
        })
        if c != CMD_REST:
            ev["note"] = c                                   # gdriver.asm:181-183 ptable[c]
            if c < len(src.ptable):
                pe = period_entry(src, c)
                ev.update({"period": pe["period"], "wave_offset": pe["wave_offset"],
                           "note_name": pe["note_name"]})
            else:
                ev["period"] = None                          # beyond ptable: would read notevals memory
    elif c == CMD_INSTRUMENT:                                # gdriver.asm:130-131, 237-242
        idx = v & 0x0F
        ev.update({"kind": "set_instrument", "instrument": idx})
        if idx < len(src.new_wave):
            wave_num, vol_num = instrument_word(src.new_wave[idx])
            ev.update({"waveform": wave_num, "envelope": vol_num})
        else:
            ev.update({"waveform": None, "envelope": None})  # beyond new_wave[12]
    elif c == CMD_TEMPO:                                     # gdriver.asm:133-134, 244-247
        ev.update({"kind": "set_tempo", "tempo": v & 0xFF})
    elif c == CMD_END:                                       # gdriver.asm:136-137, 249-258
        ev.update({"kind": "end", "loop": v != 0})
    else:
        ev["kind"] = "ignored"                               # gdriver.asm:139
    return ev


def decode_track(index: int, track: dict, src: Source) -> dict:
    body = track["body"]
    events = []
    end_at = None
    for off in range(0, len(body), 2):
        ev = decode_event(body[off], body[off + 1], src)
        ev = {"offset": off, **ev}
        events.append(ev)
        if ev["kind"] == "end" and end_at is None:
            end_at = off
    counts: dict[str, int] = {}
    for ev in events:
        counts[ev["kind"]] = counts.get(ev["kind"], 0) + 1
    song, voice = divmod(index, VOICES_PER_SONG)
    meta = SONGS[song]
    init_wave, init_env = instrument_word(src.new_wave[voice])
    loop = next((ev["loop"] for ev in events if ev["kind"] == "end"), None)
    return {
        "track": index,
        "song": song,
        "voice": voice,
        "theme": {"name": meta["name"], "title": meta["title"], "played_when": meta["cue"],
                  "citations": meta["citations"]},
        "source": {"file": "songs", "offset": track["offset"], "packlen_words": track["packlen"],
                   "bytes": track["packlen"] * 2, "citations": ["src/fmain2.c:765-772"]},
        "initial_instrument": {
            "waveform": init_wave, "envelope": init_env,
            "note": f"playscore() seeds voice {voice} from new_wave[{voice}] "
                    f"(0x{src.new_wave[voice]:04x}); high byte = waveform, low byte = envelope.",
            "citations": ["src/gdriver.asm:376-380", "src/gdriver.asm:26-27", src.new_wave_cite]},
        "loop": loop,
        "event_count": len(events),
        "counts": counts,
        "trailing_bytes_after_end": None if end_at is None else len(body) - end_at - 2,
        "format": "format.json",
        "events": events,
    }


def format_doc(src: Source) -> dict:
    """The shared decoding reference: command encoding, tables, timing model."""
    return {
        "event": {
            "layout": "2 bytes per event: command byte then value byte (gdriver.asm:120-121)",
            "commands": {
                "0x00-0x7f": "note: command = ptable index; value bits 0-5 = duration code "
                             "(notevals index), bit 6 = TIE, bit 7 = CHORD (both cleared and "
                             "otherwise unused)",
                "0x80": "rest: value = duration code as for notes; volume set to 0",
                "0x81": "set instrument: value & 0x0f indexes new_wave[]; the word is written over "
                        "wave_num/vol_num, so high byte = waveform number, low byte = envelope number",
                "0x90": "set tempo: tempo = value & 0xff (commented 'NOT SMUS STANDARD')",
                "0xff": "end of track: value 0 = stop (voice silenced), non-zero = repeat from start",
                "other": "skipped; the next event is read immediately",
            },
            "citations": ["src/gdriver.asm:116-139", "src/gdriver.asm:141-148",
                          "src/gdriver.asm:237-258", "src/gdriver.asm:26-27"],
        },
        "timing": {
            "model": "Every vertical blank timeclock += tempo. A note/rest lasts duration_counts "
                     "timeclock units; the voice is silenced NOTE_GAP counts before the next event "
                     "(no gap when the duration is shorter than the gap). Instrument/tempo/end "
                     "events take no time.",
            "initial_tempo": INITIAL_TEMPO,
            "note_gap_counts": NOTE_GAP,
            "seconds_per_count": f"1 / (tempo * vblank_hz); vblank_hz = {VBLANK_HZ} on NTSC",
            "citations": ["src/gdriver.asm:65-67", "src/gdriver.asm:149-155", "src/gdriver.asm:429"],
        },
        "duration_table": {
            "note": "notevals: 64 words indexed by duration code (8 rows x 8 columns; each column "
                    "halves the previous)",
            "values": src.notevals,
            "citations": [src.notevals_cite, "src/gdriver.asm:145-148"],
        },
        "period_table": {
            "note": "ptable: (Paula period, waveform offset) per note index. The waveform slice "
                    "played starts at offset*4 bytes and is (32-offset) words long, so higher "
                    "octaves use the shorter 32/16/8-byte cycles stored after the 64-byte one. "
                    f"pitch_hz_ntsc = {PAULA_CLOCK_NTSC} / (period * wave_len_bytes); note_name is "
                    "the nearest equal-tempered pitch (A4 = 440 Hz) of that figure.",
            "paula_clock_ntsc_hz": PAULA_CLOCK_NTSC,
            "entries": [period_entry(src, i) for i in range(len(src.ptable))],
            "citations": [src.ptable_cite, "src/gdriver.asm:181-193"],
        },
        "instrument_table": {
            "note": "new_wave[]: word per instrument number; high byte = waveform (0-7), low byte "
                    "= envelope (0-9). set_instrument masks its value to 0-15 but the table has "
                    "12 entries. setmood() rewrites entry 10 to 0x0307 in region 9 and 0x0100 "
                    "elsewhere indoors.",
            "values": [{"instrument": i, "word": f"0x{w:04x}",
                        "waveform": instrument_word(w)[0], "envelope": instrument_word(w)[1]}
                       for i, w in enumerate(src.new_wave)],
            "citations": [src.new_wave_cite, "src/gdriver.asm:237-242", "src/fmain.c:2945-2946"],
        },
        "volume": {
            "note": "The envelope's first byte is the note's starting volume; each following "
                    "vertical blank the next byte is written to AUDxVOL, a negative byte (>= 128) "
                    "holds the current volume (the read pointer does not advance).",
            "citations": ["src/gdriver.asm:98-104", "src/gdriver.asm:167-175"],
        },
    }


# --------------------------------------------------------------------------- #
# v6
# --------------------------------------------------------------------------- #
# open_all() reads S_WAVBUF, then Seek(file, S_WAVBUF, 0) -- AmigaDOS mode 0 is OFFSET_CURRENT
# (OFFSET_BEGINNING is -1, cf. hdrive.c:136) -- so the envelopes start at 2*S_WAVBUF, not S_WAVBUF
# (fmain.c:931-936). Bytes 1024-2047 and the 20 bytes after the envelopes are never read.
VOL_OFFSET = 2 * WAVBUF


def split_v6(data: bytes) -> tuple[list[list[int]], list[list[int]], dict[int, bytes]]:
    if len(data) < VOL_OFFSET + VOLBUF:
        raise ValueError(f"v6 is {len(data)} bytes, need at least {VOL_OFFSET + VOLBUF}")
    waves = [list(struct.unpack(f"{WAVEFORM_LEN}b", data[i:i + WAVEFORM_LEN]))
             for i in range(0, WAVBUF, WAVEFORM_LEN)]
    envs = [list(data[i:i + ENVELOPE_LEN])
            for i in range(VOL_OFFSET, VOL_OFFSET + VOLBUF, ENVELOPE_LEN)]
    unread = {WAVBUF: data[WAVBUF:VOL_OFFSET], VOL_OFFSET + VOLBUF: data[VOL_OFFSET + VOLBUF:]}
    return waves, envs, unread


def waveforms_doc(waves: list[list[int]], unread: dict[int, bytes]) -> dict:
    return {
        "count": len(waves),
        "length": WAVEFORM_LEN,
        "sample_format": "signed 8-bit (Paula)",
        "source": {"file": "v6", "offset": 0, "bytes": WAVBUF,
                   "citations": ["src/fmain.c:663", "src/fmain.c:931-932"]},
        "octave_slices": {
            "note": "Only part of each 128-byte waveform is looped for a given note: the ptable "
                    "offset selects start = offset*4, length = (32-offset)*2 bytes.",
            "slices": [dict(zip(("wave_offset", "start", "length"), (o, *wave_slice(o))))
                       for o in (0, 16, 24, 28)],
            "citations": ["src/gdriver.asm:181-193"],
        },
        "selection": {"note": "waveform address = wave_num * 128 + wavmem",
                      "citations": ["src/gdriver.asm:162-165"]},
        "waveforms": [{"index": i, "samples": w} for i, w in enumerate(waves)],
        "v6_unread": {
            "note": "open_all() reads S_WAVBUF, then Seek(file, S_WAVBUF, 0); AmigaDOS mode 0 is "
                    "OFFSET_CURRENT (OFFSET_BEGINNING is -1, hdrive.c:136), so the envelopes are read "
                    "from byte 2048. Bytes 1024-2047 and everything after the envelopes are never read. "
                    "Preserved here as hex; the source does not say what they are.",
            "citations": ["src/fmain.c:931-936", "src/hdrive.c:136"],
            "ranges": [{"offset": o, "length": len(b), "hex": b.hex()} for o, b in unread.items()],
        },
    }


def envelopes_doc(envs: list[list[int]]) -> dict:
    return {
        "count": len(envs),
        "length": ENVELOPE_LEN,
        "sample_format": "unsigned byte as stored; values >= 128 are negative to the player and "
                         "mean 'hold current volume'",
        "source": {"file": "v6", "offset": VOL_OFFSET, "bytes": VOLBUF,
                   "note": "Seek(file, S_WAVBUF, 0) after the first Read is OFFSET_CURRENT, "
                           "so the envelopes start at 2*S_WAVBUF = 2048.",
                   "citations": ["src/fmain.c:664", "src/fmain.c:931-936", "src/hdrive.c:136"]},
        "semantics": {
            "note": "Byte 0 is written to AUDxVOL when the note starts; then one byte per "
                    "vertical blank while the note sounds. A negative byte stops advancing "
                    "(sustain). Envelope address = vol_num * 256 + volmem.",
            "citations": ["src/gdriver.asm:167-175", "src/gdriver.asm:98-104"],
        },
        "envelopes": [{"index": i, "values": e,
                       "hold_from": next((k for k, b in enumerate(e) if b >= 128), None)}
                      for i, e in enumerate(envs)],
    }


# --------------------------------------------------------------------------- #
# Preview renderer -- a per-vertical-blank simulation of dovoice (gdriver.asm:85-200)
# --------------------------------------------------------------------------- #
class Voice:
    def __init__(self, body: bytes, voice: int, src: Source):
        self.body = body
        self.ptr = 0
        self.wave_num, self.vol_num = instrument_word(src.new_wave[voice])  # gdriver.asm:376-380
        self.vol_delay = -1                                                  # gdriver.asm:382-386
        self.event_start = self.event_stop = 0                               # gdriver.asm:389-396
        self.vol_list = 0
        self.volume = 0
        self.period = None
        self.slice = None
        self.phase = 0.0
        self.passes = 0        # times the end event was reached
        self.stopped = False   # end with value 0, or ran off the data


def step_voice(vc: Voice, timeclock: int, tempo: int, src: Source,
               waves: bytes, envs: bytes) -> int:
    """One dovoice call; returns the (possibly changed) tempo."""
    if vc.stopped:
        return tempo
    if timeclock >= vc.event_start:                                  # gdriver.asm:88-90
        for _ in range(len(vc.body)):                                # newnote (bounded: a
            if vc.ptr + 2 > len(vc.body):                            # note-less looping track
                vc.stopped, vc.volume = True, 0                      # would spin forever)
                return tempo
            c, v = vc.body[vc.ptr], vc.body[vc.ptr + 1]
            vc.ptr += 2
            if not c & 0x80 or c == CMD_REST:                        # note_comm
                dur = src.notevals[v & 0x3F]
                stop = dur - NOTE_GAP if dur >= NOTE_GAP else dur    # gdriver.asm:149-152
                vc.event_stop = vc.event_start + stop
                vc.event_start += dur
                env_base = vc.vol_num * ENVELOPE_LEN
                vc.vol_list = env_base + 1                           # gdriver.asm:173-174
                vc.vol_delay = 0                                     # gdriver.asm:175
                if c == CMD_REST:                                    # dorestnote
                    vc.volume, vc.event_stop = 0, timeclock
                elif c < len(src.ptable):
                    period, offset = src.ptable[c]
                    start, length = wave_slice(offset)
                    wbase = vc.wave_num * WAVEFORM_LEN + start
                    vc.slice = np.frombuffer(waves[wbase:wbase + length], dtype=np.int8)
                    vc.period, vc.phase = period, 0.0
                    vc.volume = envs[env_base] if env_base < len(envs) else 0
                return tempo
            if c == CMD_INSTRUMENT:
                idx = v & 0x0F
                if idx < len(src.new_wave):
                    vc.wave_num, vc.vol_num = instrument_word(src.new_wave[idx])
            elif c == CMD_TEMPO:
                tempo = v & 0xFF
            elif c == CMD_END:
                vc.passes += 1
                if v:
                    vc.ptr = 0                                       # repeat
                else:
                    vc.stopped, vc.volume = True, 0
                    return tempo
    elif timeclock >= vc.event_stop:                                 # rest_env
        vc.volume = 0
    elif vc.vol_delay == 0:                                          # gdriver.asm:98-104
        b = envs[vc.vol_list] if vc.vol_list < len(envs) else 0x80
        if b < 0x80:
            vc.volume = b
            vc.vol_list += 1
    return tempo


def render_frame(vc: Voice, n: int) -> np.ndarray:
    if vc.volume == 0 or vc.slice is None or vc.period is None:
        return np.zeros(n, dtype=np.float64)
    step = PAULA_CLOCK_NTSC / vc.period / PREVIEW_RATE    # waveform samples per output sample
    length = len(vc.slice)
    idx = (vc.phase + np.arange(n) * step) % length
    vc.phase = (vc.phase + n * step) % length
    vol = min(vc.volume, 64)                              # AUDxVOL saturates at 64
    return vc.slice[idx.astype(np.intp)].astype(np.float64) * vol


def render(bodies: list[bytes], voices: list[int], src: Source,
           waves: bytes, envs: bytes) -> np.ndarray:
    """Render tracks together (sharing the tempo) until every voice has ended or looped once."""
    vcs = [Voice(b, v, src) for b, v in zip(bodies, voices)]
    tempo = INITIAL_TEMPO
    timeclock = 0
    n = PREVIEW_RATE // VBLANK_HZ
    frames = []
    for _ in range(PREVIEW_MAX_SECONDS * VBLANK_HZ):
        timeclock += tempo                                # gdriver.asm:65-67
        mix = np.zeros(n, dtype=np.float64)
        for vc in vcs:
            tempo = step_voice(vc, timeclock, tempo, src, waves, envs)
            mix += render_frame(vc, n)
        frames.append(mix)
        if all(vc.stopped or vc.passes > 0 for vc in vcs):
            break
    out = np.concatenate(frames) * (4 / len(vcs))         # one voice: 128*64*4 = full scale
    return np.clip(out, -32768, 32767).astype("<i2")


def write_wav(path: Path, samples: np.ndarray) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(PREVIEW_RATE)
        w.writeframes(samples.tobytes())


PREVIEW_README = f"""# Music previews (non-authoritative)

Rough renders of the track event streams so a reviewer can recognise each theme by ear.
The JSON next to this directory is the deliverable; these WAVs are convenience artifacts.

- `track_NN.wav` -- one Paula voice (track NN) rendered alone.
- `song_N.wav` -- the 4 voices of song N (tracks 4N..4N+3) rendered together, as
  `playscore()` starts them (fmain.c:2953).

Renderer: a per-vertical-blank simulation of `dovoice` (gdriver.asm:85-200) -- tempo added
to the timeclock each frame, note gap of {NOTE_GAP} counts, envelope byte per frame with the
"negative = hold" rule, instrument words split into waveform/envelope, `set_tempo` shared by
all voices. Simplifications:

- NTSC timing: {VBLANK_HZ} Hz vertical blank, Paula clock {PAULA_CLOCK_NTSC} Hz.
- Output {PREVIEW_RATE} Hz mono 16-bit, nearest-sample lookup into the waveform slice
  (no Paula DMA latency, no filtering, no aliasing control).
- A new note restarts the waveform slice at phase 0 (Paula would finish the current loop first).
- Volume saturates at 64; the 68000 byte write to AUDxVOL is treated as writing the byte value.
- Each render stops once every voice has either stopped or reached its end-of-track event once
  (looping tracks play a single pass), capped at {PREVIEW_MAX_SECONDS} s.
- Single-voice renders are scaled x4 relative to the 4-voice mixes.
- Sound effects, the mute flag and the `new_wave[10]` swap for region 9 are not modelled.
"""


# --------------------------------------------------------------------------- #
def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    ac.add_io_args(parser)
    parser.add_argument("--out-dir", type=Path, default=REPO_ROOT / "assets" / "audio")
    parser.add_argument("--no-previews", action="store_true", help="skip the WAV renders")
    args = parser.parse_args(argv)

    src = Source(args.src_dir)
    songs = (args.game_dir / "songs").read_bytes()
    v6 = (args.game_dir / "v6").read_bytes()
    music_dir = args.out_dir / "music"
    inst_dir = args.out_dir / "instruments"

    tracks = split_tracks(songs)
    decoded = [decode_track(i, t, src) for i, t in enumerate(tracks)]
    for d in decoded:
        ac.write_json(music_dir / f"track_{d['track']:02d}.json", d)
    ac.write_json(music_dir / "format.json", format_doc(src))
    print(f"songs: {len(tracks)} tracks tile {len(songs)} bytes; "
          f"{sum(d['event_count'] for d in decoded)} events")

    waves, envs, unread = split_v6(v6)
    ac.write_json(inst_dir / "waveforms.json", waveforms_doc(waves, unread))
    ac.write_json(inst_dir / "envelopes.json", envelopes_doc(envs))
    print(f"v6: {len(waves)} waveforms, {len(envs)} envelopes, {sum(map(len, unread.values()))} unread bytes")

    if args.no_previews:
        return 0
    wave_bytes = v6[:WAVBUF]
    env_bytes = v6[VOL_OFFSET:VOL_OFFSET + VOLBUF]
    prev_dir = music_dir / "previews"
    for i, t in enumerate(tracks):
        write_wav(prev_dir / f"track_{i:02d}.wav",
                  render([t["body"]], [i % VOICES_PER_SONG], src, wave_bytes, env_bytes))
    for s in range(NUM_TRACKS // VOICES_PER_SONG):
        bodies = [tracks[s * VOICES_PER_SONG + v]["body"] for v in range(VOICES_PER_SONG)]
        write_wav(prev_dir / f"song_{s}.wav",
                  render(bodies, list(range(VOICES_PER_SONG)), src, wave_bytes, env_bytes))
    (prev_dir / "README.md").write_text(PREVIEW_README, encoding="utf-8")
    print(f"previews: {len(tracks)} track + {NUM_TRACKS // VOICES_PER_SONG} song WAVs")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
