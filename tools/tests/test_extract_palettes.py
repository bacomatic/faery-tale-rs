"""Tests for tools/extract_palettes.py.

Confirms the palette extractor pulls the right arrays from the original C
source, converts every 12-bit colour to rgba8 by nibble replication, and emits
the per-region colour-31 overrides from fade_page().
"""
import json
import sys
from pathlib import Path

import pytest

# Ensure tools/ is on the path so we can import the modules under test.
TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))

import asset_common as ac  # noqa: E402
import extract_palettes as ep  # noqa: E402

REPO_ROOT = TOOLS.parent
SRC = REPO_ROOT / "src"

# Expected entry counts per palette, matching the original arrays.
EXPECTED_COUNTS = {
    "pagecolors": 32,
    "textcolors": 20,
    "introcolors": 32,
    "sun_colors": 53,
    "blackcolors": 32,
}


@pytest.fixture(scope="module")
def arrays():
    return ep.load_arrays(SRC)


# --- raw array extraction --------------------------------------------------

def test_all_palettes_present(arrays):
    assert set(arrays) == set(EXPECTED_COUNTS)


@pytest.mark.parametrize("name,count", EXPECTED_COUNTS.items())
def test_counts(arrays, name, count):
    assert len(arrays[name]) == count


def test_blackcolors_all_zero(arrays):
    assert all(v == 0 for v in arrays["blackcolors"])


def test_known_pagecolors_values(arrays):
    # Transcribed by hand from fmain2.c:367-371.
    pc = arrays["pagecolors"]
    assert pc[0] == 0x000
    assert pc[16] == 0x040
    assert pc[24] == 0xC00
    assert pc[31] == 0xBDF  # the default colour-31; memory region_palette note


def test_textcolors_first_values(arrays):
    # fmain.c:476-479
    assert arrays["textcolors"][:4] == [0x000, 0xFFF, 0xC00, 0xF60]


def test_introcolors_last_value(arrays):
    assert arrays["introcolors"][-1] == 0xEEE  # fmain.c:488


def test_sun_colors_endpoints(arrays):
    sc = arrays["sun_colors"]
    assert sc[0] == 0x000 and sc[-1] == 0x76F  # fmain2.c:1569-1576


# --- colour conversion -----------------------------------------------------

@pytest.mark.parametrize("value,rgba", [
    (0x000, (0, 0, 0, 255)),
    (0xFFF, (255, 255, 255, 255)),
    (0xBDF, (187, 221, 255, 255)),
    (0x980, (153, 136, 0, 255)),
    (0x445, (68, 68, 85, 255)),
])
def test_make_entry(value, rgba):
    entry = ep.make_entry(7, value)
    assert entry["index"] == 7
    assert entry["rgb4"] == f"0x0{value:03x}"
    assert tuple(entry["rgba8"]) == rgba


def test_entries_rgba_consistent_with_rgb4(arrays):
    # Every emitted rgba8 must match asset_common's conversion of rgb4.
    for values in arrays.values():
        for entry in ep.build_palette(values):
            v = int(entry["rgb4"], 16)
            assert tuple(entry["rgba8"]) == ac.rgb4_to_rgba8(v)


# --- region overrides ------------------------------------------------------

def test_region_overrides():
    ov = ep.build_region_overrides()
    assert ov["color_index"] == 31
    assert ov["default"]["rgb4"] == "0x0bdf"
    assert ov["regions"]["4"]["rgb4"] == "0x0980"
    assert ov["regions"]["9"]["rgb4"] == "0x0445"
    # Index is always 31 across overrides.
    assert ov["default"]["index"] == 31
    assert all(r["index"] == 31 for r in ov["regions"].values())


# --- end-to-end emission ---------------------------------------------------

def test_main_writes_deterministic_json(tmp_path):
    out = tmp_path / "out"
    rc = ep.main(["--src-dir", str(SRC), "--out-dir", str(out)])
    assert rc == 0

    expected_files = {f"{n}.json" for n in EXPECTED_COUNTS} | {"region_overrides.json"}
    assert {p.name for p in out.glob("*.json")} == expected_files

    page = json.loads((out / "pagecolors.json").read_text())
    assert len(page) == 32
    assert page[31] == {"index": 31, "rgb4": "0x0bdf",
                        "rgba8": [187, 221, 255, 255]}

    # Re-run must be byte-identical (deterministic output).
    first = (out / "pagecolors.json").read_bytes()
    ep.main(["--src-dir", str(SRC), "--out-dir", str(out)])
    assert (out / "pagecolors.json").read_bytes() == first
