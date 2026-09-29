"""Tests for tools/extract_fonts.py (T2.8 self-check).

Parses the two DiskFont size files the game uses and checks the header fields
the OS reads (dfh_FileID, tf_YSize) against the sizes in the file names, that
every dereferenced pointer is a relocated hunk offset, and that the char_loc
table indexes the tf_CharData strip without overflow.
"""
import json
import sys
from pathlib import Path

import pytest

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))

import extract_fonts as ef  # noqa: E402

FONTS_DIR = TOOLS.parent / "src" / "assets" / "fonts"


@pytest.fixture(scope="module", params=[("Amber/9", 9), ("Topaz/8", 8)], ids=["amber_9", "topaz_8"])
def font(request):
    rel, size = request.param
    code, relocs = ef.parse_hunk((FONTS_DIR / rel).read_bytes())
    return code, relocs, ef.parse_font(code, relocs), size


def test_header(font):
    code, relocs, tf, size = font
    assert tf["dfh_file_id"] == 0x0F80
    assert tf["y_size"] == size
    assert tf["dfh_name"] == ""  # both files ship an all-zero dfh_Name
    assert tf["lo_char"] <= tf["hi_char"]
    assert tf["baseline"] < tf["y_size"]


def test_pointers_are_relocated(font):
    code, relocs, tf, _ = font
    for field in ("char_data", "char_loc"):
        assert 0 < tf[field] < len(code)
    assert all(0 <= r <= len(code) - 4 for r in relocs)


def test_char_loc_within_strip(font):
    code, relocs, tf, size = font
    glyphs = ef.glyph_table(code, tf)
    assert len(glyphs) == tf["hi_char"] - tf["lo_char"] + 2
    assert glyphs[-1]["code"] is None
    strip_bits = tf["modulo"] * 8
    for g in glyphs[:-1]:
        assert g["bit_offset"] + g["width"] <= strip_bits
    # Amber/9's default glyph (bit 766, width 10) runs 8 bits past its 768-bit strip rows.
    over = glyphs[-1]["bit_offset"] + glyphs[-1]["width"] - strip_bits
    assert max(over, 0) == (8 if size == 9 else 0)
    assert tf["char_data"] + tf["y_size"] * tf["modulo"] <= len(code)
    if not tf["flags"] & 0x20:  # FPF_PROPORTIONAL
        assert all(g["width"] <= tf["x_size"] for g in glyphs)


def test_extract_roundtrip(tmp_path, font):
    code, relocs, tf, size = font
    rel, name, use = next(f for f in ef.FONTS if f[1].endswith(str(size)))
    meta = ef.extract(FONTS_DIR / rel, name, use, tmp_path)
    out = tmp_path / name
    assert (out / f"{name}.json").exists() and (out / meta["atlas"]).exists()
    assert json.loads((out / f"{name}.json").read_text())["y_size"] == size
    files = [g["file"] for g in meta["glyphs"] if g["file"]]
    assert len(list((out / "glyphs").glob("*.png"))) == len(files)
    assert f"glyphs/{ord('A'):03d}.png" in files
