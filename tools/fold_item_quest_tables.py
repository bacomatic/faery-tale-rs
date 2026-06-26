#!/usr/bin/env python3
"""T1.3 — fold the item-effects and quest-data extractors into assets/tables/.

This is a thin adapter. It does **not** reimplement any extraction logic: it
imports the existing ``extract_item_effects`` and ``extract_quest_data``
modules, runs their data builders, and re-emits the results as deterministic
JSON under ``assets/tables/`` via :func:`asset_common.write_json` (sorted keys,
fixed indent, single trailing newline).

Two files are produced (names chosen to avoid colliding with the eleven T1.2
table files already in ``assets/tables/``):

* ``item_effects.json`` — per-item (``stuff[N]``) read/write cross-reference
  map, built from :func:`extract_item_effects.scan_file` over the C sources.
* ``quest_data.json``  — the full quest/narrative database from
  :func:`extract_quest_data.build_quest_db`, with the non-deterministic
  ``metadata.generated`` timestamp removed so re-runs are byte-identical.

Usage::

    python tools/fold_item_quest_tables.py
    python tools/fold_item_quest_tables.py --src-dir src/ --out-dir assets/tables
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

TOOLS_DIR = Path(__file__).resolve().parent
REPO_ROOT = TOOLS_DIR.parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import asset_common as ac  # noqa: E402
import extract_item_effects as eie  # noqa: E402
import extract_quest_data as eqd  # noqa: E402


def build_item_effects(src_dir: Path) -> dict:
    """Build the item-effects cross-reference map.

    Reuses ``extract_item_effects.scan_file`` (no logic changes) over every
    ``*.c`` file in *src_dir*, then folds the per-item summary that the
    extractor's ``main()`` computes into a JSON-serialisable structure (sets
    become sorted lists).
    """
    sources = sorted(p for p in src_dir.glob("*.c"))

    all_refs: dict[int, list] = {}
    for filepath in sources:
        basename = os.path.basename(filepath)
        for idx, line_num, write, line_text, region in eie.scan_file(str(filepath)):
            all_refs.setdefault(idx, []).append(
                (basename, line_num, write, line_text, region)
            )

    items: dict[str, dict] = {}
    for idx in sorted(all_refs):
        writes = [
            {"file": f, "line": ln, "region": reg, "text": txt}
            for f, ln, w, txt, reg in all_refs[idx]
            if w
        ]
        reads = [
            {"file": f, "line": ln, "region": reg, "text": txt}
            for f, ln, w, txt, reg in all_refs[idx]
            if not w
        ]
        write_regions = sorted({r["region"] for r in writes})
        read_regions = sorted({r["region"] for r in reads})
        cross_cutting = sorted(set(read_regions) - set(write_regions))
        items[str(idx)] = {
            "name": eie.ITEM_NAMES.get(idx, f"unknown_{idx}"),
            "writes": writes,
            "reads": reads,
            "write_regions": write_regions,
            "read_regions": read_regions,
            "cross_cutting": cross_cutting,
        }

    return {
        "source_files": [os.path.basename(str(p)) for p in sources],
        "items": items,
    }


# Tables that ship byte-exact from T1.2 (assets/tables/<name>.json) and must
# not be duplicated here: extract_quest_data embeds its own hand-transcribed
# copies, which risk drift from the authoritative source. (In particular its
# encounter_chart copy contradicts src/fmain.c:52-63 / the T1.2 export.) Drop
# them so the bundle has a single source of truth.
_REDUNDANT_T12_TABLES = ("encounter_chart", "setfig_table")


def build_quest_data() -> dict:
    """Return the quest DB, made deterministic and de-duplicated.

    Removes the non-deterministic ``metadata.generated`` timestamp and drops
    table copies that T1.2 already ships byte-exact (see
    ``_REDUNDANT_T12_TABLES``); consumers must use the authoritative
    ``assets/tables/<name>.json`` for those.
    """
    db = eqd.build_quest_db()
    meta = db.get("metadata")
    if isinstance(meta, dict):
        meta.pop("generated", None)
    for key in _REDUNDANT_T12_TABLES:
        db.pop(key, None)
    return db


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--src-dir",
        default=REPO_ROOT / "src",
        type=Path,
        help="Original C source directory (default: <repo>/src).",
    )
    parser.add_argument(
        "--out-dir",
        default=REPO_ROOT / "assets" / "tables",
        type=Path,
        help="Output directory (default: assets/tables).",
    )
    args = parser.parse_args(argv)

    item_effects = build_item_effects(args.src_dir)
    quest_data = build_quest_data()

    ac.write_json(args.out_dir / "item_effects.json", item_effects)
    ac.write_json(args.out_dir / "quest_data.json", quest_data)

    print(
        f"Wrote item_effects.json ({len(item_effects['items'])} items) and "
        f"quest_data.json ({len(quest_data)} top-level keys) to {args.out_dir}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
