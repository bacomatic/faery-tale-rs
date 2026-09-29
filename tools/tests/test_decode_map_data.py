"""Self-check aid for tools/decode_map_data.py --assets (T2.5). Not an acceptance gate."""

import json
import sys
from pathlib import Path

import pytest
from PIL import Image

REPO_ROOT = Path(__file__).resolve().parents[2]
TOOLS = REPO_ROOT / "tools"
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

import decode_map_data as dm  # noqa: E402

IMAGE = REPO_ROOT / "src" / "assets" / "image"


@pytest.fixture(scope="module")
def bundle(tmp_path_factory):
    out = tmp_path_factory.mktemp("world")
    dm.cmd_assets(str(REPO_ROOT / "src" / "assets"), str(REPO_ROOT / "src"), str(out),
                  str(REPO_ROOT / "assets" / "palettes" / "pagecolors.json"))
    return out


def test_file_index_parsed_from_source():
    rows, line = dm.load_file_index(str(REPO_ROOT / "src"))
    assert len(rows) == 10 and line == 615
    assert rows[0] == [320, 480, 520, 560, 0, 1, 32, 160, 22]   # fmain.c:616
    assert rows[9] == [680, 760, 800, 840, 10, 9, 96, 192, 0]   # fmain.c:625


def test_subtile_bits():
    # fsubs.asm:548-560: 0x80, >>4 if x&8, >>1 if y&8, >>2 if y&16
    assert dm.subtile_bit(0, 0) == 0x80
    assert dm.subtile_bit(1, 0) == 0x08
    assert dm.subtile_bit(0, 1) == 0x40
    assert dm.subtile_bit(0, 2) == 0x20
    assert dm.subtile_bit(0, 3) == 0x10
    assert dm.subtile_bit(1, 3) == 0x01
    assert sorted(dm.subtile_bit(c, r) for c in range(2) for r in range(4)) == [1 << b for b in range(8)]


def test_terra_nibbles_round_trip():
    """feature_type / mask_mode are the two nibbles of byte 1 (fsubs.asm:613-614, fmain.c:2579)."""
    with open(IMAGE, "rb") as f:
        terra_mem = dm.read_blocks(f, dm.TERRA_BLOCK + 0, 1) + dm.read_blocks(f, dm.TERRA_BLOCK + 1, 1)
    entries = dm.decode_terra_mem(terra_mem)
    assert len(entries) == 256
    for e in entries:
        raw = terra_mem[e["tile"] * 4:e["tile"] * 4 + 4]
        assert e["maptag"] == raw[0]
        assert e["feature_type"] == raw[1] >> 4
        assert e["mask_mode"] == raw[1] & 15
        assert (e["feature_type"] << 4) | e["mask_mode"] == raw[1]
        assert e["subtile_mask"] == raw[2]
        assert e["big_color"] == raw[3]


def test_tambry_at_hero_start(bundle):
    """Hero start (19036, 15755) in region 3 (fmain.c:2853) lands on a 'village of
    Tambry' sector (narr.asm place table, sectors 64-69)."""
    region = json.loads((bundle / "region_3.json").read_text())
    col = (19036 >> 8) - region["xreg"]
    row = (15755 >> 8) - region["yreg"]
    assert (col, row) == (10, 29)
    sector = region["region_map"]["grid"][row][col]
    assert dm.lookup_place_name(sector, False) == "village of Tambry"


def test_bundle_layout(bundle):
    assert sorted(p.name for p in bundle.glob("region_*.json")) == [f"region_{i}.json" for i in range(10)]
    assert (bundle / "sectors_outdoor.json").is_file() and (bundle / "sectors_indoor.json").is_file()
    assert len(list((bundle / "previews").glob("region_*_map.png"))) == 10
    assert len(list((bundle / "previews").glob("region_*_collision.png"))) == 10
    assert len(list((bundle / "previews").glob("region_*_maskmode.png"))) == 10
    assert (bundle / "previews" / "legend.json").is_file()

    pool = json.loads((bundle / "sectors_outdoor.json").read_text())
    assert pool["block"] == 32
    assert len(pool["sectors"]) == 256
    assert all(len(s) == 8 and all(len(r) == 16 for r in s) for s in pool["sectors"])

    for i in range(10):
        region = json.loads((bundle / f"region_{i}.json").read_text())
        assert region["index"] == i
        assert region["file_index"]["source"] == f"src/fmain.c:{616 + i}"
        grid = region["region_map"]["grid"]
        assert len(grid) == 32 and all(len(r) == 64 for r in grid)
        assert len(region["terra"]["entries"]) == 256
        assert region["sectors_file"] == ("sectors_outdoor.json" if i < 8 else "sectors_indoor.json")
        assert (region["xreg"], region["yreg"]) == dm.region_params(i)

    assert Image.open(bundle / "previews" / "region_0_map.png").size == (1024, 256)
    assert Image.open(bundle / "previews" / "region_0_collision.png").size == (2048, 1024)
    assert Image.open(bundle / "previews" / "region_0_maskmode.png").size == (1024, 256)


def test_regions_8_and_9_share_map_and_sectors(bundle):
    """file_index rows 8 and 9 name the same sector/region blocks (fmain.c:624-625)."""
    r8 = json.loads((bundle / "region_8.json").read_text())
    r9 = json.loads((bundle / "region_9.json").read_text())
    assert r8["region_map"]["grid"] == r9["region_map"]["grid"]
    assert r8["terra"]["blocks"] != r9["terra"]["blocks"]
