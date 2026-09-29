"""Self-check aid for tools/extract_screens.py (T2.4). Not an acceptance gate."""

import json
from pathlib import Path

import pytest
from PIL import Image

import extract_screens as xs

REPO = Path(__file__).resolve().parents[2]

# (width, height, n_planes, compression, masking) read from each file's BMHD.
EXPECTED = {
    "page0": (320, 200, 5, 1, 2), "p1a": (112, 147, 5, 1, 2), "p1b": (128, 127, 5, 1, 2),
    "p2a": (112, 147, 5, 1, 2), "p2b": (136, 76, 5, 1, 2), "p3a": (112, 147, 5, 1, 2),
    "p3b": (136, 78, 5, 1, 2), "winpic": (320, 200, 5, 1, 2), "hiscreen": (640, 57, 4, 1, 2),
}


@pytest.fixture(scope="module")
def bundle(tmp_path_factory):
    out = tmp_path_factory.mktemp("screens")
    xs.main(["--out-dir", str(out)])
    return out, json.loads((out / "screens.json").read_text())


def test_byterun1():
    # literal 3 bytes, repeat 0x42 four times, -128 NOP, literal 1 byte
    packed = bytes([2, 1, 2, 3, 0xFD, 0x42, 0x80, 0, 9])
    assert xs.unpack_byterun1(packed, 8) == (bytes([1, 2, 3, 0x42, 0x42, 0x42, 0x42, 9]), 1)


def test_bmhd_and_png_dimensions(bundle):
    out, meta = bundle
    by_name = {s["name"]: s for s in meta["screens"]}
    assert list(by_name) == xs.SCREENS and len(by_name) == 9
    for name, (w, h, planes, comp, mask) in EXPECTED.items():
        b = by_name[name]["bmhd"]
        assert (b["width"], b["height"], b["n_planes"], b["compression"], b["masking"]) == (w, h, planes, comp, mask)
        assert b["x_aspect"] == 10 and b["y_aspect"] == 11
        img = Image.open(out / f"{name}.png")
        assert img.size == (w, h) and img.mode == "RGBA"
        assert "byterun1_nop_bytes" not in by_name[name]
        assert len(by_name[name]["cmap_rgb4"]) == 1 << planes


def test_game_palettes_vs_cmap(bundle):
    _, meta = bundle
    by_name = {s["name"]: s for s in meta["screens"]}
    for name in ("page0", "p1a", "p1b", "p2a", "p2b", "p3a", "p3b"):
        assert by_name[name]["palette"]["name"] == "introcolors"
        assert by_name[name]["palette"]["cmap_differs_at"] == []
    win = by_name["winpic"]["palette"]
    assert [d["index"] for d in win["cmap_differs_at"]] == [1, 28]
    assert win["rgb4"][2] == "0x076f" and win["rgb4"][27] == "0x076f" and win["rgb4"][29] == "0x0800"
    assert by_name["hiscreen"]["palette"]["name"] == "textcolors"
    assert len(by_name["hiscreen"]["palette"]["rgb4"]) == 16


def test_placement(bundle):
    _, meta = bundle
    by_name = {s["name"]: s for s in meta["screens"]}
    assert by_name["p1a"]["shown"]["pixel_x"] == 32 and by_name["p1b"]["shown"]["pixel_x"] == 168
    assert by_name["p2b"]["shown"]["pixel_x"] == 160 and by_name["p3b"]["shown"]["y"] == 33
