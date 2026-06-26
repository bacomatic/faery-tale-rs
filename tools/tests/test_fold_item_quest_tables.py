"""T1.3 — tests for the item/quest fold-in into assets/tables/.

Checks that the produced files are valid JSON, deterministic, and do not
collide with the eleven T1.2 table file names.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

TOOLS_DIR = Path(__file__).resolve().parent.parent
REPO_ROOT = TOOLS_DIR.parent
TABLES_DIR = REPO_ROOT / "assets" / "tables"
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import fold_item_quest_tables as fold  # noqa: E402

# The eleven table files produced by T1.2 — the new files must not collide.
T1_2_TABLES = {
    "statelist",
    "encounter_chart",
    "inv_list",
    "weapon_probs",
    "treasure_probs",
    "rand_treasure",
    "diroffs",
    "fallstates",
    "setfig_table",
    "file_index",
    "trans_list",
}

NEW_FILES = ("item_effects.json", "quest_data.json")


@pytest.fixture(scope="module")
def produced(tmp_path_factory) -> Path:
    """Run the fold tool into a temp dir and return that dir."""
    out = tmp_path_factory.mktemp("tables")
    rc = fold.main(["--src-dir", str(REPO_ROOT / "src"), "--out-dir", str(out)])
    assert rc == 0
    return out


@pytest.mark.parametrize("name", NEW_FILES)
def test_valid_json(produced: Path, name: str) -> None:
    obj = json.loads((produced / name).read_text(encoding="utf-8"))
    assert isinstance(obj, dict)


def test_no_collision_with_t1_2(produced: Path) -> None:
    for name in NEW_FILES:
        stem = Path(name).stem
        assert stem not in T1_2_TABLES, f"{stem} collides with a T1.2 table"


def test_deterministic(produced: Path, tmp_path: Path) -> None:
    rc = fold.main(["--src-dir", str(REPO_ROOT / "src"), "--out-dir", str(tmp_path)])
    assert rc == 0
    for name in NEW_FILES:
        assert (produced / name).read_bytes() == (tmp_path / name).read_bytes()


def test_item_effects_shape(produced: Path) -> None:
    ie = json.loads((produced / "item_effects.json").read_text(encoding="utf-8"))
    assert set(ie) == {"items", "source_files"}
    assert ie["source_files"]  # non-empty list of scanned C files
    assert ie["items"], "expected at least one referenced stuff[] item"


def test_quest_data_deterministic_no_timestamp(produced: Path) -> None:
    qd = json.loads((produced / "quest_data.json").read_text(encoding="utf-8"))
    # The non-deterministic generation timestamp must be stripped.
    assert "generated" not in qd.get("metadata", {})
    assert qd["speeches"] and qd["quest_items"]
    # Tables that ship byte-exact from T1.2 must not be duplicated here
    # (extract_quest_data's hand-transcribed encounter_chart drifts from source).
    for redundant in fold._REDUNDANT_T12_TABLES:
        assert redundant not in qd, f"{redundant} duplicates a T1.2 table"


def test_committed_files_match_fresh_run(produced: Path) -> None:
    """The files checked into assets/tables/ match a fresh extraction."""
    for name in NEW_FILES:
        committed = TABLES_DIR / name
        if committed.exists():
            assert committed.read_bytes() == (produced / name).read_bytes()
