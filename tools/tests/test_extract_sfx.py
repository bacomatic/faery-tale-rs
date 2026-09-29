"""Tests for tools/extract_sfx.py.

Checks the length-prefix walk over ADF blocks 920-930 (fmain.c:1033-1041),
that the shipped WAVs carry the framed PCM byte-exact (signed -> unsigned by
XOR 0x80) at the documented nominal rate, and that every trigger expression in
extract_sfx.TRIGGERS appears verbatim at its cited source line.
"""
import json
import re
import sys
import wave
from pathlib import Path

import pytest

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))

import extract_sfx as es  # noqa: E402

REPO_ROOT = TOOLS.parent
SRC = REPO_ROOT / "src"
OUT = REPO_ROOT / "assets" / "audio" / "sfx"

# (pcm offset in the 5632-byte buffer, byte length), read off the BE prefixes.
EXPECTED = [(4, 1012), (1020, 684), (1708, 786), (2498, 1604), (4106, 952), (5062, 550)]


@pytest.fixture(scope="module")
def samples():
    image = (SRC / "assets" / "image").read_bytes()
    return es.read_samples(image)


def test_prefix_walk(samples):
    out, remaining = samples
    assert [(s["offset"], s["length"]) for s in out] == EXPECTED
    assert remaining == 5632 - (5062 + 550) == 20
    for s in out:
        assert len(s["pcm"]) == s["length"]
        assert s["length"] % 2 == 0  # sample_size/2 words loses nothing (fmain.c:3618)


def test_wavs_are_byte_exact(samples):
    out, _ = samples
    for s in out:
        n = s["index"]
        with wave.open(str(OUT / f"sfx_{n}.wav"), "rb") as w:
            assert (w.getnchannels(), w.getsampwidth()) == (1, 1)
            assert w.getframerate() == round(es.NTSC_PAULA_CLOCK / es.NOMINAL_BASE[n])
            data = w.readframes(w.getnframes())
        assert bytes(b ^ 0x80 for b in data) == s["pcm"]


def test_sfx_json_matches_buffer(samples):
    out, remaining = samples
    doc = json.loads((OUT / "sfx.json").read_text())
    assert doc["source"]["bytes_after_last_sample"] == remaining
    assert len(doc["effects"]) == 6
    for e, s in zip(doc["effects"], out):
        assert (e["buffer_offset"], e["byte_length"]) == (s["offset"], s["length"])
        assert e["triggers"] == es.TRIGGERS[e["effect"]]


def test_trigger_expressions_are_at_cited_lines():
    for n, trigs in es.TRIGGERS.items():
        for t in trigs:
            fname, line = re.match(r"src/(\S+):(\d+)$", t["source"]).groups()
            text = (SRC / fname).read_text(errors="replace").splitlines()[int(line) - 1]
            assert f"effect({n},{t['expr']})" in text, (t["source"], text)
            assert t["expr"].replace(" ", "").startswith(f"{t['base']}+{t['rng']}")
