"""Tests for the generic C-array extractor (tools/extract_table.py).

Covers the T0.2 baseline: pulling a named C array as raw nested ints with no
field semantics, for 1-D and N-D shapes and hex/decimal/char literals.
"""
import json
from pathlib import Path

import pytest

import extract_table as et

REPO_ROOT = Path(__file__).resolve().parents[2]
SRC = REPO_ROOT / "src"
FIXTURES = Path(__file__).parent / "fixtures"


# --- scalar literal parsing ------------------------------------------------

@pytest.mark.parametrize("token,expected", [
    ("16", 16),
    ("-3", -3),
    ("0x20", 0x20),
    ("0X3a", 0x3a),
    ("-0x10", -0x10),
    ("'A'", 65),
    ("'\\n'", 10),
    ("'\\0'", 0),
    ("'\\x1b'", 0x1b),
    ("'\\033'", 0o33),
])
def test_parse_c_value(token, expected):
    assert et.parse_c_value(token) == expected


def test_parse_c_value_non_numeric_falls_back_to_string():
    assert et.parse_c_value("SOME_MACRO") == "SOME_MACRO"


# --- nested initializer parsing -------------------------------------------

def test_parse_flat_1d():
    assert et.parse_c_initializer("{1, 2, 3}") == [1, 2, 3]


def test_parse_nested_2d():
    assert et.parse_c_initializer("{ {1,2}, {3,4}, {5,6} }") == [[1, 2], [3, 4], [5, 6]]


def test_parse_nested_3d():
    assert et.parse_c_initializer("{ {{1,2},{3,4}}, {{5,6},{7,8}} }") == \
        [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]


def test_parse_mixed_literals():
    assert et.parse_c_initializer("{0x10, 16, -2, 'A'}") == [16, 16, -2, 65]


# --- real extraction from source ------------------------------------------

DIROFFS_EXPECTED = [16, 16, 24, 24, 0, 0, 8, 8, 56, 56, 68, 68, 32, 32, 44, 44]


def test_extract_diroffs_from_source():
    tables = et.extract_tables(str(SRC / "fmain.c"))
    assert "diroffs" in tables
    t = tables["diroffs"]
    assert t["values"] == DIROFFS_EXPECTED
    assert len(t["values"]) == 16
    assert t["shape"] == (16,)


def test_extract_fallstates_shape():
    # fallstates is a flat UBYTE[] of 24 entries (logically 4 rows x 6).
    tables = et.extract_tables(str(SRC / "fmain2.c"))
    assert "fallstates" in tables
    vals = tables["fallstates"]["values"]
    assert len(vals) == 24
    assert all(isinstance(v, int) for v in vals)
    assert vals[:6] == [0, 0, 0, 0, 0, 0]
    assert vals[6:12] == [0x20, 0x22, 0x3a, 0x6f, 0x70, 0x71]


def test_diroffs_fixture_matches_source():
    fixture = json.loads((FIXTURES / "diroffs.json").read_text())
    assert fixture["diroffs"]["values"] == DIROFFS_EXPECTED
    assert fixture["diroffs"]["shape"] == [16]
    assert fixture["diroffs"]["source"] == "src/fmain.c"
