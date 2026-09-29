"""Self-check aid for tools/extract_tiles.py (T2.2). Not an acceptance gate."""

import json
from pathlib import Path

import numpy as np
import pytest
from PIL import Image

import extract_tiles as xt

REPO = Path(__file__).resolve().parents[2]
IMAGE = (REPO / "src" / "assets" / "image").read_bytes()
REGIONS = xt.load_file_index(REPO / "src")


def test_file_index_from_source():
    # fmain.c:616 (F1) and fmain.c:625 (F10)
    assert len(REGIONS) == 10
    assert REGIONS[0]["source"] == "src/fmain.c:616"
    assert [REGIONS[0][f"image_{g}"] for g in range(4)] == [320, 480, 520, 560]
    assert REGIONS[0]["label"] == "F1" and REGIONS[0]["name"] == "snowy region"
    assert REGIONS[9]["source"] == "src/fmain.c:625"
    assert [REGIONS[9][f"image_{g}"] for g in range(4)] == [680, 760, 800, 840]
    assert REGIONS[9]["name"] == "dungeons and caves"


def _ref_pixel(mem: bytes, T: int, R: int, x: int) -> int:
    """Pure-python decode: offset(T,P,R) = (T/64)*20480 + P*4096 + (T%64)*64 + R*2."""
    pix = 0
    for P in range(5):
        off = (T // 64) * 20480 + P * 4096 + (T % 64) * 64 + R * 2
        word = (mem[off] << 8) | mem[off + 1]
        pix |= ((word >> (15 - x)) & 1) << P
    return pix


def test_decode_matches_hand_formula():
    blocks = [REGIONS[7][f"image_{g}"] for g in range(4)]  # F8, fmain.c:623
    mem = b"".join(IMAGE[b * 512:b * 512 + 20480] for b in blocks)
    tiles = xt.decode_region(IMAGE, blocks)
    assert tiles.shape == (256, 32, 16)
    # tile 65, plane 2, row 3 -> byte 1*20480 + 2*4096 + 1*64 + 6 = 28742
    assert (65 // 64) * 20480 + 2 * 4096 + (65 % 64) * 64 + 3 * 2 == 28742
    word = (mem[28742] << 8) | mem[28743]
    for x in range(16):
        assert (tiles[65, 3, x] >> 2) & 1 == (word >> (15 - x)) & 1
    for T, R in [(0, 0), (65, 3), (127, 31), (200, 17), (255, 31)]:
        assert [int(v) for v in tiles[T, R]] == [_ref_pixel(mem, T, R, x) for x in range(16)]


def test_atlas_placement():
    tiles = xt.decode_region(IMAGE, [REGIONS[0][f"image_{g}"] for g in range(4)])
    atlas = xt.pack_atlas(tiles)
    assert atlas.shape == (512, 256)
    for T in (0, 17, 65, 255):
        y, x = (T // 16) * 32, (T % 16) * 16
        assert np.array_equal(atlas[y:y + 32, x:x + 16], tiles[T])


@pytest.fixture(scope="module")
def bundle(tmp_path_factory):
    out = tmp_path_factory.mktemp("tiles")
    xt.main(["--out-dir", str(out)])
    return out


def test_outputs_and_palette_override(bundle):
    for r in range(10):
        d = bundle / f"region_{r:02d}"
        idx = Image.open(d / "atlas_indexed.png")
        rgba = Image.open(d / "atlas_rgba.png")
        mask = Image.open(d / "atlas_highlightmask.png")
        assert idx.size == rgba.size == mask.size == (256, 512)
        a = np.array(idx)
        assert a.max() < 32
        # mask bit set exactly where index in 16..24
        m = np.array(mask.convert("L")) > 0
        assert np.array_equal(m, (a >= 16) & (a <= 24))
        # RGBA == palette-applied indices, incl. the region colour-31 override (fmain2.c:381-386)
        meta = json.loads((d / "tiles.json").read_text())
        pal = [tuple(e["rgba8"]) for e in sorted(
            json.loads((REPO / "assets/palettes/pagecolors.json").read_text()), key=lambda e: e["index"])]
        pal[31] = tuple(meta["palette"]["color_31"]["rgba8"])
        assert np.array_equal(np.array(rgba), np.array(pal, dtype=np.uint8)[a])
        assert len(meta["tiles"]) == 256 and meta["tile_size"] == [16, 32]
        assert meta["tiles"][65] == {"index": 65, "x": 16, "y": 128, "w": 16, "h": 32,
                                     "group": 1, "block": REGIONS[r]["image_1"]}
    c31 = {r: json.loads((bundle / f"region_{r:02d}/tiles.json").read_text())["palette"]["color_31"]["rgb4"]
           for r in range(10)}
    assert c31[4] == "0x0980" and c31[9] == "0x0445"
    assert all(v == "0x0bdf" for r, v in c31.items() if r not in (4, 9))
