"""Self-check aid for tools/decode_map_data.py load_regions (T2.5 world model). Not an acceptance gate."""

import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
TOOLS = REPO_ROOT / "tools"
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

import decode_map_data as dm  # noqa: E402

IMAGE = REPO_ROOT / "src" / "assets" / "image"


@pytest.fixture(scope="module")
def regions():
    return dm.load_regions(str(REPO_ROOT / "src" / "assets"), str(REPO_ROOT / "src"))


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


def test_tambry_at_hero_start(regions):
    """Hero start (19036, 15755) in region 3 (fmain.c:2853) lands on a 'village of
    Tambry' sector (narr.asm place table, sectors 64-69)."""
    region = regions[3]
    col = (19036 >> 8) - region["xreg"]
    row = (15755 >> 8) - region["yreg"]
    assert (col, row) == (10, 29)
    sector = region["grid"][row][col]
    assert dm.lookup_place_name(sector, False) == "village of Tambry"


def test_region_layout(regions):
    assert [r["index"] for r in regions] == list(range(10))
    for i, region in enumerate(regions):
        assert region["source"] == f"src/fmain.c:{616 + i}"
        assert len(region["grid"]) == 32 and all(len(r) == 64 for r in region["grid"])
        assert len(region["pool"]) == 256 and all(len(s) == 8 and all(len(r) == 16 for r in s) for s in region["pool"])
        assert len(region["tiles"]) == 256 and all(len(r) == 1024 for r in region["tiles"])
        assert len(region["terra"]) == 256
        assert region["type"] == ("outdoor" if i < 8 else "indoor")
        assert (region["xreg"], region["yreg"]) == dm.region_params(i)
    assert regions[0]["file_index"]["sector"] == 32 and regions[8]["file_index"]["sector"] == 96
    # tile at map (row, col) = pool[grid[row//8][col//16]][row%8][col%16] (fsubs.asm:565-604)
    r = regions[5]
    assert r["tiles"][100][300] == r["pool"][r["grid"][12][18]][4][12]


def test_regions_8_and_9_share_map_and_sectors(regions):
    """file_index rows 8 and 9 name the same sector/region blocks (fmain.c:624-625)."""
    assert regions[8]["grid"] == regions[9]["grid"] and regions[8]["pool"] is regions[9]["pool"]
    assert regions[8]["terra_blocks"] != regions[9]["terra_blocks"]
    assert regions[8]["terra_blocks"][1] == regions[9]["terra_blocks"][1]   # inside+astral shared
