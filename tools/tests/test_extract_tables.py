"""Shape/count + spot-value tests for the T1.2 gameplay-table extractor.

Builds every table in-process from the C source (so the test does not depend on
the JSON having been regenerated) and asserts the dimensions from the task spec,
plus a few hand-transcribed first/last rows against the original source.
"""
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
TOOLS = REPO_ROOT / "tools"
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

import extract_tables as xt  # noqa: E402

SRC = REPO_ROOT / "src"


@pytest.fixture(scope="module")
def tables():
    enum_obytes = xt.parse_enum((SRC / "fmain2.c").read_text(errors="replace"),
                                "obytes")
    return {label: xt.build_table(label, SRC, enum_obytes) for label in xt.SOURCES}


# Spec dimensions: label -> expected shape list.
EXPECTED_SHAPE = {
    "statelist": [87, 4],
    "encounter_chart": [11, 6],
    "inv_list": [36, 8],
    "weapon_probs": [32],
    "treasure_probs": [40],
    "rand_treasure": [16],
    "diroffs": [16],
    "fallstates": [24],
    "setfig_table": [14, 3],
    "file_index": [10, 9],
    "trans_list": [9, 4],
}


def test_all_tables_present(tables):
    assert set(tables) == set(EXPECTED_SHAPE)


@pytest.mark.parametrize("label,shape", sorted(EXPECTED_SHAPE.items()))
def test_shape(tables, label, shape):
    assert tables[label]["shape"] == shape


@pytest.mark.parametrize("label,shape", sorted(EXPECTED_SHAPE.items()))
def test_row_or_value_count(tables, label, shape):
    rec = tables[label]
    if "rows" in rec:
        assert len(rec["rows"]) == shape[0]
        assert all(len(r) == shape[1] for r in rec["rows"])
        assert rec["fields"] == xt.STRUCT_FIELDS[label]
    else:
        assert len(rec["values"]) == shape[0]
        assert all(isinstance(v, int) for v in rec["values"])


# --- hand-transcribed first/last rows from the C source --------------------

def test_statelist_endpoints(tables):
    rows = tables["statelist"]["rows"]
    # fmain.c:156 first entry / fmain.c:225 last entry ("asleep")
    assert rows[0] == {"figure": 0, "wpn_no": 11, "wpn_x": -2, "wpn_y": 11}
    assert rows[-1] == {"figure": 66, "wpn_no": 10, "wpn_x": 5, "wpn_y": 11}


def test_file_index_endpoints(tables):
    rows = tables["file_index"]["rows"]
    # fmain.c:616 F1 / fmain.c:625 F10
    assert rows[0] == {"image_0": 320, "image_1": 480, "image_2": 520,
                       "image_3": 560, "terra1": 0, "terra2": 1, "sector": 32,
                       "region": 160, "setchar": 22}
    assert rows[-1] == {"image_0": 680, "image_1": 760, "image_2": 800,
                        "image_3": 840, "terra1": 10, "terra2": 9, "sector": 96,
                        "region": 192, "setchar": 0}


def test_setfig_table_endpoints(tables):
    rows = tables["setfig_table"]["rows"]
    # fmain.c:24 wizard / fmain.c:37 beggar
    assert rows[0] == {"cfile_entry": 13, "image_base": 0, "can_talk": 1}
    assert rows[-1] == {"cfile_entry": 17, "image_base": 4, "can_talk": 1}


def test_encounter_chart_symbols_resolved(tables):
    rows = tables["encounter_chart"]["rows"]
    # TRUE -> 1 (Ogre), NULL -> 0 (Woodcutter, index 10).
    assert rows[0]["agressive"] == 1
    assert rows[10]["agressive"] == 0
    assert all(isinstance(v, int) for r in rows for v in r.values())


def test_rand_treasure_enum_resolved(tables):
    # SACKS=16, CHEST=15, MONEY=13, GOLD_KEY=25, QUIVER=11, GREY_KEY=26,
    # RED_KEY=242, B_TOTEM=23, VIAL=22, WHITE_KEY=154.
    assert tables["rand_treasure"]["values"] == [
        16, 16, 16, 16, 15, 13, 25, 11, 26, 26, 26, 242, 23, 22, 154, 15,
    ]


def test_inv_list_name_is_stripped_string(tables):
    rows = tables["inv_list"]["rows"]
    assert rows[0]["name"] == "Dirk"
    assert rows[-1]["name"] == "quiver of arrows"


def test_diroffs_values(tables):
    assert tables["diroffs"]["values"] == \
        [16, 16, 24, 24, 0, 0, 8, 8, 56, 56, 68, 68, 32, 32, 44, 44]


def test_weapon_treasure_group_layout(tables):
    wp = tables["weapon_probs"]["values"]
    assert wp[:4] == [0, 0, 0, 0]
    assert wp[24:28] == [8, 8, 8, 8]  # group 6: touch attack
    tp = tables["treasure_probs"]["values"]
    assert tp[8:16] == [9, 11, 13, 31, 31, 17, 17, 32]  # group 1
