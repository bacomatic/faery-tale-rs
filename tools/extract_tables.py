#!/usr/bin/env python3
"""Extract the gameplay lookup tables from the original Faery Tale C source.

This is a *byte-exact* converter (T1.2). It pulls each named C array from the
1987 source via the generic extractor in ``extract_table.py`` (brace-aware,
N-D, newline-tolerant) and emits one deterministic JSON per table under
``assets/tables/``. Field names and short semantics are attached from
``reference/RESEARCH-data-structures.md``. No gameplay/engine/creative changes.

Symbolic initializers are resolved to their compile-time integer values so the
output is pure data:

* ``TRUE`` -> 1, ``FALSE`` -> 0, ``NULL`` -> 0 (Amiga ``exec/types.h`` values),
  used by ``encounter_chart``.
* ``enum obytes`` (``fmain2.c``) object-id constants, used by ``rand_treasure``.

String fields (the ``inv_list`` item names) have their surrounding C quotes
stripped.

Tables exported (shape / count):
  statelist (87x4), encounter_chart (11x6), inv_list (36), weapon_probs (32),
  treasure_probs (40), rand_treasure (16), diroffs (16), fallstates (24),
  setfig_table (14x3), file_index (10x9), trans_list (9x4).

Usage::

    python tools/extract_tables.py
    python tools/extract_tables.py --src-dir src/ --out-dir assets/tables
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# Make sibling tools modules importable regardless of CWD.
TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import asset_common as ac  # noqa: E402
import extract_table as et  # noqa: E402

REPO_ROOT = TOOLS_DIR.parent

# exec/types.h boolean/null constants (not #defined in the game source itself).
BASE_SYMBOLS = {"TRUE": 1, "FALSE": 0, "NULL": 0}


def parse_enum(content: str, enum_name: str) -> dict[str, int]:
    """Parse a C ``enum <name> { ... }`` into a ``symbol -> value`` map.

    Honours explicit ``NAME = value`` resets and auto-increments otherwise,
    exactly like the C compiler. Used for ``enum obytes`` (object ids).
    """
    m = re.search(r'\benum\s+' + re.escape(enum_name) + r'\s*\{([^}]*)\}', content)
    if not m:
        raise SystemExit(f"enum {enum_name!r} not found")
    body = re.sub(r'/\*.*?\*/', '', m.group(1), flags=re.DOTALL)
    symbols: dict[str, int] = {}
    nxt = 0
    for item in body.split(','):
        item = item.strip()
        if not item:
            continue
        if '=' in item:
            name, val = item.split('=', 1)
            nxt = et.parse_c_value(val.strip())
            symbols[name.strip()] = nxt
        else:
            symbols[item] = nxt
        nxt += 1
    return symbols


def resolve(value, symbols: dict[str, int]):
    """Recursively resolve symbolic tokens to ints / strip string quotes."""
    if isinstance(value, list):
        return [resolve(v, symbols) for v in value]
    if isinstance(value, int):
        return value
    if isinstance(value, str):
        s = value.strip()
        if len(s) >= 2 and s[0] == '"' and s[-1] == '"':
            return s[1:-1]  # C string literal -> bare text
        if s in symbols:
            return symbols[s]
        raise SystemExit(f"unresolved symbol/token: {value!r}")
    raise SystemExit(f"unexpected value type: {value!r}")


def reshape(flat: list[int], ncols: int) -> list[list[int]]:
    """Split a flat list into rows of *ncols* (the C initializer was flat)."""
    if len(flat) % ncols:
        raise SystemExit(f"cannot reshape {len(flat)} values into rows of {ncols}")
    return [flat[i:i + ncols] for i in range(0, len(flat), ncols)]


# --------------------------------------------------------------------------- #
# Table specifications
# --------------------------------------------------------------------------- #
# Each spec: source file, optional reshape width, field names (for struct/2-D
# tables), and a short semantics blurb from the reference doc. ``fields=None``
# marks a flat 1-D table emitted as a plain ``values`` list.

STRUCT_FIELDS = {
    "statelist": ["figure", "wpn_no", "wpn_x", "wpn_y"],
    "encounter_chart": ["hitpoints", "agressive", "arms", "cleverness",
                         "treasure", "file_id"],
    "inv_list": ["image_number", "xoff", "yoff", "ydelta", "img_off",
                 "img_height", "maxshown", "name"],
    "setfig_table": ["cfile_entry", "image_base", "can_talk"],
    "file_index": ["image_0", "image_1", "image_2", "image_3", "terra1",
                   "terra2", "sector", "region", "setchar"],
    "trans_list": ["newstate_0", "newstate_1", "newstate_2", "newstate_3"],
}

SEMANTICS = {
    "statelist": "87 struct state entries (fmain.c:143-205). Maps "
                 "(motion_state, facing, frame) -> (figure image, weapon "
                 "overlay index, weapon x/y offset). 0-7 south walk, 8-15 west "
                 "walk, 16-23 north walk, 24-31 east walk, 32-79 fight blocks "
                 "(12 states x 4 facings), 80-82 death, 83 sink, 84-85 "
                 "oscillation, 86 asleep.",
    "encounter_chart": "11 struct encounter entries (fmain.c:54-64). Per-race "
                       "combat stats; indexed by character.race. agressive "
                       "(sic) is vestigial; arms indexes weapon_probs, treasure "
                       "indexes treasure_probs, cleverness picks ATTACK1/2.",
    "inv_list": "36 struct inv_item entries (fmain.c:380-418). Inventory item "
                "display descriptors; maxshown doubles as gold value for coins. "
                "name is the display string.",
    "weapon_probs": "8 groups of 4 (fmain2.c:860-868). Weapon-at-spawn lookup: "
                    "weapon_probs[arms*4 + rnd(4)]. 0=none 1=dirk 2=mace "
                    "3=sword 4=bow 5=wand 8=touch.",
    "treasure_probs": "5 groups of 8 (fmain2.c:852-858). Loot-on-search lookup: "
                      "treasure_probs[treasure*8 + rnd(8)]. Values are object ids.",
    "rand_treasure": "16 enum-obytes object ids (fmain2.c:987-991). Random "
                     "treasure scattered into a region: rand_treasure[rnd(15)] "
                     "(fmain2.c:1236). Resolved from enum obytes.",
    "diroffs": "16-entry walk/fight base selector (fmain.c:1010). 0-7 pick walk "
               "bases, 8-15 pick fight/shoot bases for statelist.",
    "fallstates": "24 UBYTE (fmain2.c:871-874), logically 4 rows x 6. Death/fall "
                  "animation state targets; fallstates[j].",
    "setfig_table": "14 NPC type descriptors (fmain.c:24-37). cfile_entry = "
                    "image file (seq_list index), image_base = sub-image offset, "
                    "can_talk = enables TALKING visual effect.",
    "file_index": "10 struct need entries (fmain.c:615-625), one per region "
                  "F1-F10. Asset-loading descriptor: image[4] file indices, "
                  "terra1/terra2 terrain files, sector, region map, setchar.",
    "trans_list": "9 struct transition entries (fmain.c:138-146). Fight-swing "
                  "animation transitions; next state = "
                  "trans_list[state].newstate[rnd(4)].",
}

# label -> (source filename, reshape-width-or-None)
SOURCES = {
    "statelist": ("fmain.c", None),
    "encounter_chart": ("fmain.c", None),
    "inv_list": ("fmain.c", None),
    "weapon_probs": ("fmain2.c", None),
    "treasure_probs": ("fmain2.c", None),
    "rand_treasure": ("fmain2.c", None),
    "diroffs": ("fmain.c", None),
    "fallstates": ("fmain2.c", None),
    "setfig_table": ("fmain.c", 3),   # flat initializer -> 14x3
    "file_index": ("fmain.c", None),
    "trans_list": ("fmain.c", None),
}


def build_table(label: str, src_dir: Path, enum_obytes: dict[str, int]) -> dict:
    fname, width = SOURCES[label]
    rel_src = f"src/{fname}"
    tables = et.extract_tables(str(src_dir / fname))
    if label not in tables:
        raise SystemExit(f"array {label!r} not found in {rel_src}")
    raw = tables[label]
    symbols = {**BASE_SYMBOLS, **enum_obytes}
    values = resolve(raw["values"], symbols)

    if width is not None:
        values = reshape(values, width)

    record: dict = {
        "name": label,
        "source": rel_src,
        "line": raw["line"],
        "semantics": SEMANTICS[label],
    }

    fields = STRUCT_FIELDS.get(label)
    if fields is not None:
        if not all(isinstance(r, list) and len(r) == len(fields) for r in values):
            raise SystemExit(f"{label}: rows do not match {len(fields)} fields")
        record["shape"] = [len(values), len(fields)]
        record["fields"] = fields
        record["rows"] = [dict(zip(fields, row)) for row in values]
    else:
        record["shape"] = [len(values)]
        record["values"] = values

    return record


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--src-dir", type=Path, default=REPO_ROOT / "src",
                        help="Original C source directory (default: src/).")
    parser.add_argument("--out-dir", type=Path,
                        default=REPO_ROOT / "assets" / "tables",
                        help="Output directory (default: assets/tables).")
    args = parser.parse_args(argv)

    enum_obytes = parse_enum((args.src_dir / "fmain2.c").read_text(errors="replace"),
                             "obytes")

    for label in SOURCES:
        record = build_table(label, args.src_dir, enum_obytes)
        ac.write_json(args.out_dir / f"{label}.json", record)
        print(f"{label}.json: shape={record['shape']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
