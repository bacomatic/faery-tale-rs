"""Self-checks for tools/extract_music.py (T2.6) against the in-repo originals."""
import json
import sys
import wave
from pathlib import Path

import pytest

TOOLS = Path(__file__).resolve().parents[1]
REPO = TOOLS.parent
sys.path.insert(0, str(TOOLS))

import extract_music as em  # noqa: E402

SONGS = REPO / "src" / "assets" / "songs"
V6 = REPO / "src" / "assets" / "v6"
SRC = REPO / "src"

pytestmark = pytest.mark.skipif(not SONGS.exists() or not V6.exists(),
                                reason="original game files not present")


@pytest.fixture(scope="module")
def src():
    return em.Source(SRC)


def test_source_tables(src):
    assert len(src.ptable) == 78          # 13 dc.w lines x 6 (period, offset) pairs
    assert src.ptable[0] == (1440, 0)     # gdriver.asm:204
    assert src.ptable[77] == (135, 28)    # gdriver.asm:222
    assert len(src.notevals) == 64        # gdriver.asm:228-235
    assert src.notevals[0] == 26880 and src.notevals[63] == 270
    assert len(src.new_wave) == 12        # fmain.c:669-672
    assert src.new_wave[4] == 0x0005 and src.new_wave[9] == 0x0504


def test_instrument_word():
    assert em.instrument_word(0x0504) == (5, 4)
    assert em.instrument_word(0x0100) == (1, 0)


def test_wave_slice():
    assert em.wave_slice(0) == (0, 64)
    assert em.wave_slice(16) == (64, 32)
    assert em.wave_slice(24) == (96, 16)
    assert em.wave_slice(28) == (112, 8)


def test_tracks_tile_file(src):
    data = SONGS.read_bytes()
    tracks = em.split_tracks(data)
    assert len(tracks) == 28
    assert sum(4 + t["packlen"] * 2 for t in tracks) == len(data) == 5984
    for i, t in enumerate(tracks):
        d = em.decode_track(i, t, src)
        assert d["event_count"] == t["packlen"]
        assert all(ev["raw"] == [t["body"][ev["offset"]], t["body"][ev["offset"] + 1]]
                   for ev in d["events"])


def test_v6_split():
    waves, envs, unread = em.split_v6(V6.read_bytes())
    assert len(waves) == 8 and all(len(w) == 128 for w in waves)
    assert all(-128 <= s <= 127 for w in waves for s in w)
    assert len(envs) == 10 and all(len(e) == 256 for e in envs)
    assert {o: len(b) for o, b in unread.items()} == {1024: 1024, 4608: 20}
    assert all(e[0] > 0 for e in envs[:9])  # real curves start at 2048, not 1024


def test_main_writes_bundle(tmp_path):
    assert em.main(["--out-dir", str(tmp_path), "--game-dir", str(SONGS.parent),
                    "--src-dir", str(SRC), "--no-previews"]) == 0
    assert len(list((tmp_path / "music").glob("track_*.json"))) == 28
    fmt = json.loads((tmp_path / "music" / "format.json").read_text())
    assert len(fmt["period_table"]["entries"]) == 78
    assert fmt["period_table"]["entries"][18]["note_name"] == "A2"   # period 508, 64-byte cycle
    inst = json.loads((tmp_path / "instruments" / "waveforms.json").read_text())
    assert len(inst["waveforms"]) == 8
    env = json.loads((tmp_path / "instruments" / "envelopes.json").read_text())
    assert len(env["envelopes"]) == 10


def test_render_one_track(src, tmp_path):
    tracks = em.split_tracks(SONGS.read_bytes())
    v6 = V6.read_bytes()
    pcm = em.render([t["body"] for t in tracks[12:16]], [0, 1, 2, 3], src,
                    v6[:em.WAVBUF], v6[em.VOL_OFFSET:em.VOL_OFFSET + em.VOLBUF])   # title song
    assert len(pcm) > em.PREVIEW_RATE            # at least a second of audio
    assert int(abs(pcm.astype(int)).max()) > 0   # not silent
    em.write_wav(tmp_path / "t.wav", pcm)
    with wave.open(str(tmp_path / "t.wav")) as w:
        assert (w.getnchannels(), w.getsampwidth(), w.getframerate()) == (1, 2, em.PREVIEW_RATE)
        assert w.getnframes() == len(pcm)
