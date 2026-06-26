#!/usr/bin/env python3
"""Extract the hardcoded colour tables from the original Faery Tale C source.

The 1987 source defines several 12-bit Amiga colour tables as plain C arrays:

* ``pagecolors``  -- the main in-game (page) palette (``fmain2.c``)
* ``textcolors``  -- the message/text palette (``fmain.c``)
* ``introcolors`` -- the intro sequence palette (``fmain.c``)
* ``sun_colors``  -- the day/night sun colour ramp (``fmain2.c``)
* ``blackcolors`` -- the all-zero fade-to-black palette (``fmain.c``)

In addition, ``fade_page`` (``fmain2.c``) patches ``pagecolors[31]`` per region
before every fade. Only colour index 31 varies by region:

* region 4 (desert)   -> ``0x0980``
* region 9 (dungeons) -> ``0x0445``
* all other regions   -> ``0x0bdf`` (the default already in ``pagecolors[31]``)

This extractor is a *byte-exact* converter: it pulls the raw C arrays (reusing
the generic extractor in ``extract_table.py``) and emits, for every entry,
``{index, amiga12, rgba8}`` where ``amiga12`` is the canonical ``0x0RGB`` string
and ``rgba8`` is produced by ``asset_common.amiga12_to_rgba8`` (nibble
replication). No gameplay, engine, or creative changes.

Usage::

    python tools/extract_palettes.py
    python tools/extract_palettes.py --src-dir src/ --out-dir assets/palettes
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# Make the sibling tools modules importable regardless of CWD.
TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import asset_common as ac  # noqa: E402
from extract_table import parse_c_initializer  # noqa: E402

REPO_ROOT = TOOLS_DIR.parent

# Palette name -> source file it is defined in (relative to --src-dir).
# pagecolors / sun_colors live in fmain2.c; the rest in fmain.c. We scan both
# files and look the name up, so this map only documents intent.
PALETTE_NAMES = ["pagecolors", "textcolors", "introcolors", "sun_colors", "blackcolors"]
SOURCE_FILES = ["fmain.c", "fmain2.c"]

# Region colour-31 overrides, from fade_page() in fmain2.c. The non-default
# regions patch pagecolors[31] before fading; everyone else uses the default.
# (region 9 also has a transient secret_timer value 0x00f0, which is a runtime
# effect, not a stored palette override, so it is intentionally excluded.)
COLOR31_INDEX = 31
REGION_OVERRIDES = {4: 0x0980, 9: 0x0445}
COLOR31_DEFAULT = 0x0BDF


def amiga12_str(v: int) -> str:
    """Render a 12-bit Amiga colour as the canonical ``0x0RGB`` string."""
    return f"0x0{v & 0xFFF:03x}"


def make_entry(index: int, value: int) -> dict:
    """Build a ``{index, amiga12, rgba8}`` record for one colour."""
    return {
        "index": index,
        "amiga12": amiga12_str(value),
        "rgba8": list(ac.amiga12_to_rgba8(value)),
    }


def _find_c_array(content: str, name: str) -> list[int] | None:
    """Locate ``<type> name[] = { ... }`` in *content* and parse it to ints.

    Tolerant of the brace appearing on the line *after* the ``=`` (which the
    line-by-line scanner in ``extract_table`` misses). The braced body is parsed
    with ``extract_table.parse_c_initializer`` so the literal handling stays
    identical to the generic extractor.
    """
    decl = re.compile(r'\b' + re.escape(name) + r'\s*\[\s*\d*\s*\]\s*=\s*\{')
    m = decl.search(content)
    if not m:
        return None
    brace_start = m.end() - 1  # position of the '{'
    depth = 1
    pos = brace_start + 1
    while pos < len(content) and depth > 0:
        if content[pos] == '{':
            depth += 1
        elif content[pos] == '}':
            depth -= 1
        pos += 1
    inner = content[brace_start + 1:pos - 1]
    inner = re.sub(r'/\*.*?\*/', '', inner, flags=re.DOTALL)
    inner = re.sub(r'//.*$', '', inner, flags=re.MULTILINE)
    return parse_c_initializer('{' + inner + '}')


def load_arrays(src_dir: Path) -> dict[str, list[int]]:
    """Return ``name -> [int, ...]`` for every palette array, scanning sources."""
    contents = []
    for fname in SOURCE_FILES:
        path = src_dir / fname
        if path.is_file():
            contents.append(path.read_text(errors="replace"))

    arrays: dict[str, list[int]] = {}
    for name in PALETTE_NAMES:
        values = None
        for content in contents:
            values = _find_c_array(content, name)
            if values is not None:
                break
        if values is None:
            raise SystemExit(f"palette array {name!r} not found under {src_dir}")
        if not all(isinstance(v, int) for v in values):
            raise SystemExit(f"palette array {name!r} has non-integer entries")
        arrays[name] = [v & 0xFFF for v in values]
    return arrays


def build_palette(values: list[int]) -> list[dict]:
    return [make_entry(i, v) for i, v in enumerate(values)]


def build_region_overrides() -> dict:
    """Build the colour-31 region-override record."""
    return {
        "color_index": COLOR31_INDEX,
        "default": make_entry(COLOR31_INDEX, COLOR31_DEFAULT),
        "regions": {
            str(region): make_entry(COLOR31_INDEX, value)
            for region, value in sorted(REGION_OVERRIDES.items())
        },
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--src-dir", type=Path, default=REPO_ROOT / "src",
                        help="Original C source directory (default: src/).")
    parser.add_argument("--out-dir", type=Path, default=REPO_ROOT / "assets" / "palettes",
                        help="Output directory (default: assets/palettes).")
    args = parser.parse_args(argv)

    arrays = load_arrays(args.src_dir)
    out_dir = args.out_dir

    for name, values in arrays.items():
        palette = build_palette(values)
        ac.write_json(out_dir / f"{name}.json", palette)
        print(f"{name}.json: {len(palette)} entries")

    overrides = build_region_overrides()
    ac.write_json(out_dir / "region_overrides.json", overrides)
    print(f"region_overrides.json: default + {len(overrides['regions'])} region overrides")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
