"""Tests for tools/asset_common.py shared asset helpers."""
import json
import struct
import sys
import zlib
from pathlib import Path

import pytest

# Ensure tools/ is on the path so we can import asset_common.
TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))

import asset_common as ac  # noqa: E402

PIL = pytest.importorskip("PIL")
from PIL import Image  # noqa: E402


# --------------------------------------------------------------------------- #
# amiga12_to_rgba8
# --------------------------------------------------------------------------- #
@pytest.mark.parametrize(
    "v,expected",
    [
        (0x0000, (0, 0, 0, 255)),
        (0x0FFF, (255, 255, 255, 255)),
        (0x0F0F, (255, 0, 255, 255)),
        (0x0123, (0x11, 0x22, 0x33, 255)),
        (0x0ABC, (0xAA, 0xBB, 0xCC, 255)),
        (0x0001, (0, 0, 0x11, 255)),
        # high nibble ignored (only low 12 bits used)
        (0xF123, (0x11, 0x22, 0x33, 255)),
    ],
)
def test_amiga12_to_rgba8(v, expected):
    assert ac.amiga12_to_rgba8(v) == expected


# --------------------------------------------------------------------------- #
# Indexed PNG round-trip
# --------------------------------------------------------------------------- #
def _ramp_palette():
    # 32 distinct, deterministic palette entries.
    return [(i * 8, (i * 5) & 0xFF, (255 - i * 7) & 0xFF) for i in range(32)]


def test_write_indexed_png_roundtrip(tmp_path):
    palette = _ramp_palette()
    idx = [
        [0, 15, 16],
        [24, 25, 31],
    ]
    out = tmp_path / "indexed.png"
    ac.write_indexed_png(out, idx, palette)

    img = Image.open(out)
    assert img.mode == "P"
    assert img.size == (3, 2)
    back = list(img.getdata())
    assert back == [0, 15, 16, 24, 25, 31]

    # Palette colours preserved exactly.
    pal = img.getpalette()
    for i, (r, g, b) in enumerate(palette):
        assert pal[i * 3 : i * 3 + 3] == [r, g, b]

    # Index 31 is transparent; a non-31 index is opaque. Pillow reports the
    # palette transparency either as a single index int or as an alpha table.
    trns = img.info.get("transparency")
    if isinstance(trns, int):
        assert trns == 31
    else:
        assert trns[31] == 0
        assert trns[0] == 255


def test_write_indexed_png_deterministic(tmp_path):
    palette = _ramp_palette()
    idx = [[0, 16, 31], [24, 15, 25]]
    a = tmp_path / "a.png"
    b = tmp_path / "b.png"
    ac.write_indexed_png(a, idx, palette)
    ac.write_indexed_png(b, idx, palette)
    assert a.read_bytes() == b.read_bytes()


# --------------------------------------------------------------------------- #
# RGBA PNG
# --------------------------------------------------------------------------- #
def test_write_rgba_png(tmp_path):
    rgba = [
        [(255, 0, 0, 255), (0, 255, 0, 128)],
        [(0, 0, 255, 0), (10, 20, 30, 40)],
    ]
    out = tmp_path / "rgba.png"
    ac.write_rgba_png(out, rgba)

    img = Image.open(out)
    assert img.mode == "RGBA"
    assert img.size == (2, 2)
    assert list(img.getdata()) == [
        (255, 0, 0, 255),
        (0, 255, 0, 128),
        (0, 0, 255, 0),
        (10, 20, 30, 40),
    ]


# --------------------------------------------------------------------------- #
# Highlight mask
# --------------------------------------------------------------------------- #
def test_write_highlight_mask_bits(tmp_path):
    # Row covering every relevant boundary value.
    idx = [
        [0, 15, 16, 17, 24, 25, 31, 23],
    ]
    out = tmp_path / "mask.png"
    ac.write_highlight_mask(out, idx)

    img = Image.open(out)
    assert img.mode == "1"
    assert img.size == (8, 1)

    bits = list(img.getdata())
    # Pillow mode "1" reports 0 / 255.
    norm = [1 if v else 0 for v in bits]
    expected = [
        0,  # 0  -> no
        0,  # 15 -> no (below range)
        1,  # 16 -> yes
        1,  # 17 -> yes
        1,  # 24 -> yes (top of range)
        0,  # 25 -> no (above range)
        0,  # 31 -> no (transparent index)
        1,  # 23 -> yes
    ]
    assert norm == expected

    # Bit set only for indices in 16..24.
    for i, idx_val in enumerate(idx[0]):
        want = 1 if 16 <= idx_val <= 24 else 0
        assert norm[i] == want

    # Greyscale value 0 (everything non-highlight, incl. index 31) transparent.
    assert img.info.get("transparency") == 0


def test_write_highlight_mask_padding(tmp_path):
    # Width not a multiple of 8 -> last byte padded with zero bits.
    idx = [[16, 16, 16]]  # 3 wide
    out = tmp_path / "mask3.png"
    ac.write_highlight_mask(out, idx)
    img = Image.open(out)
    assert img.size == (3, 1)
    norm = [1 if v else 0 for v in img.getdata()]
    assert norm == [1, 1, 1]


# --------------------------------------------------------------------------- #
# JSON
# --------------------------------------------------------------------------- #
def test_write_json_deterministic(tmp_path):
    obj = {"b": 2, "a": 1, "nested": {"z": 26, "y": 25}}
    out = tmp_path / "x.json"
    ac.write_json(out, obj)
    text = out.read_text(encoding="utf-8")

    # Sorted keys, trailing newline, re-parses equal.
    assert text.endswith("\n")
    assert text.index('"a"') < text.index('"b"') < text.index('"nested"')
    assert text.index('"y"') < text.index('"z"')
    assert json.loads(text) == obj

    # Idempotent: re-write yields identical bytes.
    out2 = tmp_path / "x2.json"
    ac.write_json(out2, obj)
    assert out.read_bytes() == out2.read_bytes()


# --------------------------------------------------------------------------- #
# arg helpers
# --------------------------------------------------------------------------- #
def test_add_io_args_defaults():
    parser = ac.build_arg_parser("test")
    args = parser.parse_args([])
    assert str(args.game_dir) == ac.DEFAULT_GAME_DIR
    assert str(args.src_dir) == ac.DEFAULT_SRC_DIR.rstrip("/") or str(
        args.src_dir
    ) == ac.DEFAULT_SRC_DIR

    args2 = parser.parse_args(["--game-dir", "/somewhere", "--src-dir", "orig"])
    assert str(args2.game_dir) == "/somewhere"
    assert str(args2.src_dir) == "orig"


def test_help_mentions_image_alternative():
    parser = ac.build_arg_parser("test")
    help_text = parser.format_help()
    assert "--image" in help_text


# --------------------------------------------------------------------------- #
# Sanity: emitted PNGs are valid (signature + parseable chunks)
# --------------------------------------------------------------------------- #
def test_png_signature_and_chunks(tmp_path):
    out = tmp_path / "sig.png"
    ac.write_indexed_png(out, [[0, 31]], _ramp_palette())
    data = out.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    # Walk chunks and verify CRCs.
    pos = 8
    tags = []
    while pos < len(data):
        length = struct.unpack(">I", data[pos : pos + 4])[0]
        tag = data[pos + 4 : pos + 8]
        chunk_data = data[pos + 8 : pos + 8 + length]
        crc = struct.unpack(">I", data[pos + 8 + length : pos + 12 + length])[0]
        assert crc == (zlib.crc32(tag + chunk_data) & 0xFFFFFFFF)
        tags.append(tag)
        pos += 12 + length
    assert tags[0] == b"IHDR"
    assert tags[-1] == b"IEND"
    assert b"PLTE" in tags
