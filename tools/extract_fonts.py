#!/usr/bin/env python3
"""Amiga DiskFont extractor for The Faery Tale Adventure (T2.8).

The game uses two bitmap fonts:

* ``fonts/Amber/9`` -- loaded with ``LoadSeg`` and used as a ``TextFont`` via
  ``font = (struct DiskFontHeader *)((seg<<2)+8); afont = &(font->dfh_TF)``
  (``fmain.c:774-775,782``); selected with ``SetFont(rp,afont)`` for the
  scrolling message area and placards (``fmain.c:1168,2860``, ``fmain2.c:1550,1586``).
* ``topaz.font`` size 8 -- the ROM font, ``OpenFont(&topaz_ta)`` (``fmain.c:650,778``),
  selected for the status bar and map text (``fmain.c:779-781,2881``, ``fmain2.c:1481``).
  ``src/assets/fonts/Topaz/8`` is the on-disk copy of that font.

Both files are AmigaDOS hunk executables (HUNK_HEADER / HUNK_CODE / HUNK_RELOC32 /
HUNK_END). The code hunk holds a 4-byte code stub (the ``+8`` in the game's cast:
BPTR->APTR is +4, stub is +4), then a ``DiskFontHeader`` (``libraries/diskfont.h``) whose ``dfh_TF`` is the ``TextFont``
(``graphics/text.h``). Pointers inside the hunk (``tf_CharData``, ``tf_CharLoc``,
``tf_CharSpace``, ``tf_CharKern``, the ``ln_Name`` fields) are stored as offsets
from the hunk data start and listed in the HUNK_RELOC32 table; this script checks
every pointer it dereferences against that table instead of assuming.

Outputs, per size file, under ``assets/fonts/<font>_<size>/``:

* ``glyphs/<code>.png`` -- one 1-bit PNG per character code ``lo_char..hi_char``
  (decimal, zero-padded to 3 digits) plus ``default.png`` for the extra last
  ``tf_CharLoc`` entry the OS draws for out-of-range codes,
* ``<font>_<size>_atlas.png`` -- the raw ``tf_CharData`` strip
  (``tf_Modulo*8`` x ``tf_YSize``), which the ``char_loc`` bit offsets index,
* ``<font>_<size>.json`` -- the ``TextFont`` metrics and per-character table.

Usage::

    python tools/extract_fonts.py            # -> assets/fonts/
"""

from __future__ import annotations

import struct
import sys
from pathlib import Path

TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import asset_common as ac  # noqa: E402

REPO_ROOT = TOOLS_DIR.parent

HUNK_HEADER, HUNK_CODE, HUNK_RELOC32, HUNK_END = 0x3F3, 0x3E9, 0x3EC, 0x3F2
DFH_ID = 0x0F80  # dfh_FileID, libraries/diskfont.h

# (file under --game-dir/fonts, output name, source citation for the game's use)
FONTS = [
    ("Amber/9", "amber_9", {
        "loaded_by": "LoadSeg(\"fonts/Amber/9\"); afont = &(font->dfh_TF)",
        "loaded_at": ["src/fmain.c:774-775", "src/fmain.c:782"],
        "used_at": ["src/fmain.c:1168", "src/fmain.c:2860", "src/fmain2.c:1550", "src/fmain2.c:1586"],
        "role": "in-game scrolling messages and placard/story text",
    }),
    ("Topaz/8", "topaz_8", {
        "loaded_by": "OpenFont(&topaz_ta) with topaz_ta = {\"topaz.font\", 8, 0, FPF_ROMFONT}",
        "loaded_at": ["src/fmain.c:650", "src/fmain.c:778"],
        "used_at": ["src/fmain.c:779-781", "src/fmain.c:2881", "src/fmain2.c:1481"],
        "role": "status bar labels/menus and map-mode text; the disk file stands in for the ROM font",
    }),
]

STYLE_BITS = {0x01: "FSF_UNDERLINED", 0x02: "FSF_BOLD", 0x04: "FSF_ITALIC", 0x08: "FSF_EXTENDED"}
FLAG_BITS = {0x01: "FPF_ROMFONT", 0x02: "FPF_DISKFONT", 0x04: "FPF_REVPATH", 0x08: "FPF_TALLDOT",
             0x10: "FPF_WIDEDOT", 0x20: "FPF_PROPORTIONAL", 0x40: "FPF_DESIGNED", 0x80: "FPF_REMOVED"}


def _bits(value: int, names: dict[int, str]) -> list[str]:
    return [n for b, n in names.items() if value & b]


# --------------------------------------------------------------------------- #
# Hunk file
# --------------------------------------------------------------------------- #
def parse_hunk(data: bytes) -> tuple[bytes, list[int]]:
    """Return (code hunk bytes, sorted HUNK_RELOC32 offsets) of a one-hunk executable."""
    u32 = lambda o: struct.unpack_from(">I", data, o)[0]
    pos = 0
    if u32(pos) != HUNK_HEADER:
        raise SystemExit("not a hunk file")
    pos += 4
    while u32(pos):  # resident library names (none expected)
        pos += 4 + u32(pos) * 4
    pos += 4
    table_size, first, last = u32(pos), u32(pos + 4), u32(pos + 8)
    if (table_size, first, last) != (1, 0, 0):
        raise SystemExit(f"expected exactly one hunk, got table {table_size} {first}..{last}")
    pos += 12 + 4 * table_size  # hunk sizes

    code: bytes | None = None
    relocs: list[int] = []
    while pos < len(data):
        hid = u32(pos) & 0x3FFFFFFF
        pos += 4
        if hid == HUNK_CODE:
            n = u32(pos) * 4
            code = data[pos + 4:pos + 4 + n]
            pos += 4 + n
        elif hid == HUNK_RELOC32:
            while (count := u32(pos)):
                hunk = u32(pos + 4)
                if hunk != 0:
                    raise SystemExit(f"reloc targets hunk {hunk}")
                relocs += [u32(pos + 8 + 4 * i) for i in range(count)]
                pos += 8 + 4 * count
            pos += 4
        elif hid == HUNK_END:
            break
        else:
            raise SystemExit(f"unexpected hunk 0x{hid:X} at {pos - 4}")
    if code is None:
        raise SystemExit("no HUNK_CODE")
    return code, sorted(relocs)


# --------------------------------------------------------------------------- #
# DiskFontHeader / TextFont
# --------------------------------------------------------------------------- #
# Offsets from the start of the code hunk data: a 4-byte code stub, then
# struct DiskFontHeader { struct Node dfh_DF (14); UWORD dfh_FileID; UWORD dfh_Revision;
#   LONG dfh_Segment; char dfh_Name[32]; struct TextFont dfh_TF; }   -- libraries/diskfont.h
DFH = 4
DFH_FILEID, DFH_REVISION, DFH_NAME, DFH_TF = DFH + 14, DFH + 16, DFH + 22, DFH + 54
# struct TextFont { struct Message tf_Message (20); UWORD tf_YSize; UBYTE tf_Style; UBYTE tf_Flags;
#   UWORD tf_XSize; UWORD tf_Baseline; UWORD tf_BoldSmear; UWORD tf_Accessors; UBYTE tf_LoChar;
#   UBYTE tf_HiChar; APTR tf_CharData; UWORD tf_Modulo; APTR tf_CharLoc; APTR tf_CharSpace;
#   APTR tf_CharKern; }                                              -- graphics/text.h
TF_FIELDS = ">HBBHHHHBBIHIII"  # from tf_YSize on (offset 20 in the struct)
TF_NAMES = ["y_size", "style", "flags", "x_size", "baseline", "bold_smear", "accessors",
            "lo_char", "hi_char", "char_data", "modulo", "char_loc", "char_space", "char_kern"]


def parse_font(code: bytes, relocs: list[int]) -> dict:
    file_id, revision = struct.unpack_from(">HH", code, DFH_FILEID)
    if file_id != DFH_ID:
        raise SystemExit(f"dfh_FileID 0x{file_id:04X} != 0x{DFH_ID:04X}")
    name = code[DFH_NAME:DFH_NAME + 32].split(b"\0", 1)[0].decode("ascii")
    tf = dict(zip(TF_NAMES, struct.unpack_from(TF_FIELDS, code, DFH_TF + 20)))

    # Every pointer we dereference must be a relocated offset into this hunk.
    for field in ("char_data", "char_loc", "char_space", "char_kern"):
        at = DFH_TF + 20 + struct.calcsize(TF_FIELDS[:1 + TF_NAMES.index(field)])
        if tf[field] and at not in relocs:
            raise SystemExit(f"tf_{field} at hunk offset {at} is not in HUNK_RELOC32")
        if not tf[field] and at in relocs:
            raise SystemExit(f"tf_{field} is NULL but relocated")
    tf.update(dfh_name=name, dfh_file_id=file_id, dfh_revision=revision, relocs=relocs)
    return tf


def glyph_table(code: bytes, tf: dict) -> list[dict]:
    n = tf["hi_char"] - tf["lo_char"] + 2  # +1 for the default glyph, graphics/text.h
    locs = struct.unpack_from(f">{2 * n}H", code, tf["char_loc"])
    spaces = struct.unpack_from(f">{n}h", code, tf["char_space"]) if tf["char_space"] else None
    kerns = struct.unpack_from(f">{n}h", code, tf["char_kern"]) if tf["char_kern"] else None
    out = []
    for i in range(n):
        code_pt = tf["lo_char"] + i
        g = {"code": code_pt if i < n - 1 else None,
             "char": chr(code_pt) if i < n - 1 and 32 <= code_pt < 127 else None,
             "bit_offset": locs[2 * i], "width": locs[2 * i + 1]}
        if spaces:
            g["space"] = spaces[i]
        if kerns:
            g["kern"] = kerns[i]
        out.append(g)
    return out


def char_data(code: bytes, tf: dict, width: int | None = None) -> list[list[int]]:
    """The tf_CharData strip as rows of bits (tf_Modulo*8 wide by default, tf_YSize high).

    `width` may exceed tf_Modulo*8: the bits then come from the bytes that follow the row in
    memory (the next row / whatever lies after the strip), exactly as the blitter reads them.
    """
    width = tf["modulo"] * 8 if width is None else width
    nbytes = (width + 7) // 8
    rows = []
    for y in range(tf["y_size"]):
        row = code[tf["char_data"] + y * tf["modulo"]:][:nbytes]
        rows.append([(b >> (7 - i)) & 1 for b in row for i in range(8)][:width])
    return rows


# --------------------------------------------------------------------------- #
# Main
# --------------------------------------------------------------------------- #
def extract(font_file: Path, name: str, use: dict, out_root: Path) -> dict:
    code, relocs = parse_hunk(font_file.read_bytes())
    tf = parse_font(code, relocs)
    glyphs = glyph_table(code, tf)
    strip = char_data(code, tf)
    strip_bits = len(strip[0])
    # Glyph reads may run past the strip (Amber/9's default glyph does); read linearly like the blitter.
    wide = char_data(code, tf, max(strip_bits, *(g["bit_offset"] + g["width"] for g in glyphs)))
    out_dir = out_root / name
    src_rel = font_file.as_posix()

    for g in glyphs:
        fname = f"{g['code']:03d}.png" if g["code"] is not None else "default.png"
        x0, w = g["bit_offset"], g["width"]
        g["file"] = f"glyphs/{fname}"
        if x0 + w > strip_bits:
            g["overflows_strip_by"] = x0 + w - strip_bits
        if w:  # zero-width entries have no pixels to draw
            ac.write_bit_mask(out_dir / g["file"], [row[x0:x0 + w] for row in wide])
        else:
            g["file"] = None
    ac.write_bit_mask(out_dir / f"{name}_atlas.png", strip)

    meta = {
        "name": name,
        "source_file": src_rel,
        "dfh_name": tf["dfh_name"],
        "dfh_file_id": tf["dfh_file_id"],
        "dfh_revision": tf["dfh_revision"],
        "y_size": tf["y_size"], "baseline": tf["baseline"], "x_size": tf["x_size"],
        "bold_smear": tf["bold_smear"], "modulo": tf["modulo"],
        "lo_char": tf["lo_char"], "hi_char": tf["hi_char"],
        "style": tf["style"], "style_names": _bits(tf["style"], STYLE_BITS),
        "flags": tf["flags"], "flags_names": _bits(tf["flags"], FLAG_BITS),
        "proportional": bool(tf["flags"] & 0x20),
        "has_char_space": bool(tf["char_space"]), "has_char_kern": bool(tf["char_kern"]),
        "atlas": f"{name}_atlas.png",
        "atlas_size": [tf["modulo"] * 8, tf["y_size"]],
        "glyph_count": len(glyphs),
        "glyphs": glyphs,
        "layout": {
            "container": "AmigaDOS hunk executable: HUNK_HEADER, one HUNK_CODE, HUNK_RELOC32, HUNK_END",
            "code_hunk_bytes": len(code),
            "code_stub_hex": code[:DFH].hex(),
            "disk_font_header_at": DFH,
            "text_font_at": DFH_TF,
            "tf_char_data_at": tf["char_data"], "tf_char_loc_at": tf["char_loc"],
            "tf_char_space_at": tf["char_space"] or None, "tf_char_kern_at": tf["char_kern"] or None,
            "reloc32_offsets": relocs,
            "note": "Offsets are from the start of the code hunk data. The game reads the header at "
                    "(seg<<2)+8, i.e. 4 bytes past the hunk data start (src/fmain.c:775); those 4 bytes "
                    "are code_stub_hex. "
                    "Pointer fields are relocatable offsets listed in reloc32_offsets; the OS adds the load "
                    "address. Glyph pixels: bit (bit_offset + x) of row y in the tf_CharData strip, "
                    "rows tf_Modulo bytes apart; the last char_loc entry (code null) is the default glyph "
                    "for out-of-range codes. Glyph files are named by decimal code, e.g. glyphs/065.png = 'A'. "
                    "A glyph with overflows_strip_by reads that many bits past the end of each strip row "
                    "(i.e. from the next row's first bytes, and past the strip on the last row), as the "
                    "blitter does.",
            "structs": ["libraries/diskfont.h: struct DiskFontHeader", "graphics/text.h: struct TextFont"],
        },
        "game_use": use,
    }
    ac.write_json(out_dir / f"{name}.json", meta)
    return meta


def verify_items(meta: dict) -> list[dict]:
    """T2.8 review items for one font: the atlas, then A, 0, first and last glyph."""
    name, use = meta["name"], meta["game_use"]
    cites = use["loaded_at"] + use["used_at"]
    by_code = {g["code"]: g for g in meta["glyphs"] if g["code"] is not None}
    face = ("the game's in-game message/placard face" if name == "amber_9"
            else "the familiar Amiga system font (topaz-8)")
    n_files = sum(1 for g in meta["glyphs"] if g["file"])
    items = [{
        "id": f"{name}-atlas", "task": "T2.8", "view": "image",
        "title": f"{meta['dfh_name']} {meta['y_size']} — glyph atlas ({meta['atlas_size'][0]}x{meta['atlas_size'][1]})",
        "files": [f"{name}/{meta['atlas']}", f"{name}/{name}.json"],
        "look_for": (f"The raw tf_CharData strip: every character {meta['lo_char']}..{meta['hi_char']} "
                     f"(ASCII {chr(meta['lo_char'])!r}..{chr(meta['hi_char'])!r}) in code order, left to right, "
                     f"{meta['y_size']} px tall, then the default glyph; {face}. Nothing clipped or scrambled. "
                     f"JSON: y_size {meta['y_size']}, baseline {meta['baseline']}, x_size {meta['x_size']}, "
                     f"modulo {meta['modulo']}, {meta['glyph_count']} char_loc entries."),
        "citations": cites,
        "counts": [
            {"label": "glyph PNGs", "glob": f"{name}/glyphs/*.png", "expect": n_files},
            {"label": "char_loc entries", "file": f"{name}/{name}.json", "path": "glyphs",
             "expect": meta["glyph_count"]},
        ],
    }]
    picks = [(ord("A"), "capital letter A"), (ord("0"), "digit zero"),
             (meta["lo_char"], f"first character, code {meta['lo_char']}"),
             (meta["hi_char"], f"last character, code {meta['hi_char']}")]
    for code_pt, what in dict(picks).items():  # dict() drops a duplicate code
        g = by_code.get(code_pt)
        if not g or not g["file"]:
            continue
        items.append({
            "id": f"{name}-glyph-{code_pt:03d}", "task": "T2.8", "view": "image",
            "title": f"{meta['dfh_name']} {meta['y_size']} — glyph {code_pt} {g['char']!r}",
            "files": [f"{name}/{g['file']}"],
            "look_for": f"The {what}: {g['char']!r} ({g['width']}x{meta['y_size']} px).",
            "citations": cites,
        })
    return items


def main(argv: list[str] | None = None) -> int:
    ap = ac.build_arg_parser(__doc__.split("\n\n")[0])
    ap.add_argument("--out-dir", type=Path, default=REPO_ROOT / "assets" / "fonts")
    args = ap.parse_args(argv)
    items = []
    for rel, name, use in FONTS:
        meta = extract(args.game_dir / "fonts" / rel, name, use, args.out_dir)
        items += verify_items(meta)
        print(f"{name}: {meta['dfh_name']} y_size={meta['y_size']} baseline={meta['baseline']} "
              f"x_size={meta['x_size']} modulo={meta['modulo']} chars {meta['lo_char']}..{meta['hi_char']} "
              f"flags={meta['flags_names']} glyphs={meta['glyph_count']}")
    ac.write_json(args.out_dir / "verify.json", {
        "schema_version": 1,
        "tasks": {"T2.8": {
            "title": "Fonts",
            "summary": "The two DiskFont size files the game uses: Amber/9 (LoadSeg, in-game text) and "
                       "Topaz/8 (stand-in for the ROM topaz-8 the game opens with OpenFont). Per font: the "
                       "raw glyph atlas, one PNG per character, and the TextFont metrics JSON. Check each "
                       "atlas reads as a full character set and the listed glyphs show the expected character."}},
        "items": items,
    })
    return 0


if __name__ == "__main__":
    sys.exit(main())
