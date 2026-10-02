#!/usr/bin/env python3
"""Extract the narrative text from ``src/narr.asm`` (The Faery Tale Adventure, 1987).

``narr.asm`` holds every in-game message as ``dc.b`` directives. This tool parses the
assembly directly (quoted literals, numeric bytes, ``equ`` symbols, ``;`` comments) and
emits one JSON file per structure under ``assets/text/``:

* ``event_msg.json``    -- ``_event_msg``  (39 null-terminated strings)
* ``place_msg.json``    -- ``_place_msg``  (27; entries 0-1 are empty, ``dc.b 0`` slots)
* ``inside_msg.json``   -- ``_inside_msg`` (23; entries 0-1 are empty)
* ``speeches.json``     -- ``_speeches``   (61; entry 52 is an empty ``dc.b 0`` slot)
* ``question.json``     -- ``q1``..``q8`` in ``qq`` table order (8 riddle prompts) with the
  answers from ``char *answers[]`` (``fmain2.c:1306-1307``), same index
* ``placard_text.json`` -- the 20 ``mst`` entries, decoded as ``_ssp`` does
  (``fsubs.asm:497-536``): byte 0 ends the entry, byte 128 (``XY``) is followed by
  ``x/2`` and ``y`` (a ``Move``), anything else is a text run up to the next 0 / >=128 byte.
* ``place_tbl.json`` / ``inside_tbl.json`` -- ``{lo, hi, msg_index, comment}`` rows
  (29 / 37), source order preserved (first match wins).

Text bytes are kept verbatim: ``%`` (character name) stays as-is, byte 13 becomes ``\\r``.
No content is changed or reflowed.

Usage::

    python tools/extract_text.py
    python tools/extract_text.py --src-dir src/ --out-dir assets/text
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import asset_common as ac  # noqa: E402

REPO_ROOT = TOOLS_DIR.parent
SOURCE_FILE = "narr.asm"

STRING_BLOCKS = {
    "_event_msg": "event_msg",
    "_place_msg": "place_msg",
    "_inside_msg": "inside_msg",
    "_speeches": "speeches",
}
TABLE_BLOCKS = {"_place_tbl": "place_tbl", "_inside_tbl": "inside_tbl"}
BLOCK_END_DIRECTIVES = {"cseg", "dseg", "end"}


# --------------------------------------------------------------------------- #
# Line model
# --------------------------------------------------------------------------- #
@dataclass
class Line:
    no: int                      # 1-based source line
    label: str | None            # symbol at column 0, if any
    directive: str | None        # dc.b / dc.l / dc.w / equ / cseg / ...
    operands: str                # raw operand text, comment stripped
    comment: str                 # text after ';' (outside quotes), stripped
    is_comment_only: bool = False


def split_comment(text: str) -> tuple[str, str]:
    """Split at the first ``;`` that is outside a quoted literal."""
    quote = None
    for i, ch in enumerate(text):
        if quote:
            if ch == quote:
                quote = None
        elif ch in "'\"":
            quote = ch
        elif ch == ";":
            return text[:i], text[i + 1:].strip()
    return text, ""


def parse_line(no: int, raw: str) -> Line:
    code, comment = split_comment(raw.rstrip("\n"))
    if not code.strip():
        return Line(no, None, None, "", comment, is_comment_only=bool(comment))
    label = None
    if not code[0].isspace():
        label, *rest = code.split(None, 1)
        code = rest[0] if rest else ""
    parts = code.strip().split(None, 1)
    directive = parts[0].lower() if parts else None
    operands = parts[1].strip() if len(parts) > 1 else ""
    return Line(no, label, directive, operands, comment)


def parse_lines(text: str) -> list[Line]:
    return [parse_line(i, raw) for i, raw in enumerate(text.splitlines(), 1)]


# --------------------------------------------------------------------------- #
# Operand evaluation
# --------------------------------------------------------------------------- #
def split_operands(text: str) -> list[str]:
    """Split a ``dc.b`` operand list on commas outside quoted literals."""
    out, cur, quote = [], [], None
    for ch in text:
        if quote:
            cur.append(ch)
            if ch == quote:
                quote = None
        elif ch in "'\"":
            quote = ch
            cur.append(ch)
        elif ch == ",":
            out.append("".join(cur).strip())
            cur = []
        else:
            cur.append(ch)
    if cur or text.endswith(","):
        out.append("".join(cur).strip())
    return out


def eval_expr(expr: str, symbols: dict[str, int]) -> int:
    """Evaluate the integer expressions narr.asm uses: decimal literals, symbols, ``a/b``."""
    expr = expr.strip()
    if "/" in expr:
        a, b = expr.split("/", 1)
        return eval_expr(a, symbols) // eval_expr(b, symbols)
    if expr.startswith("$"):
        return int(expr[1:], 16)
    if expr.isdigit():
        return int(expr, 10)
    if expr in symbols:
        return symbols[expr]
    raise ValueError(f"cannot evaluate operand {expr!r}")


def dcb_bytes(operands: str, symbols: dict[str, int]) -> bytes:
    """Bytes emitted by one ``dc.b`` operand list."""
    out = bytearray()
    for op in split_operands(operands):
        if len(op) >= 2 and op[0] in "'\"" and op[-1] == op[0]:
            out += op[1:-1].encode("latin-1")
        else:
            out.append(eval_expr(op, symbols) & 0xFF)
    return bytes(out)


def collect_symbols(lines: list[Line]) -> dict[str, int]:
    return {ln.label: eval_expr(ln.operands, {}) for ln in lines
            if ln.directive == "equ" and ln.label}


# --------------------------------------------------------------------------- #
# Block extraction
# --------------------------------------------------------------------------- #
def block_after(lines: list[Line], label: str) -> tuple[int, list[Line]]:
    """``dc.*`` lines following *label* up to the next label or segment directive."""
    start = next(i for i, ln in enumerate(lines) if ln.label == label)
    body = []
    for ln in lines[start + 1:]:
        if ln.label is not None or ln.directive in BLOCK_END_DIRECTIVES:
            break
        if ln.directive:
            body.append(ln)
    return lines[start].no, body


def strings_from_block(body: list[Line], symbols: dict[str, int]) -> list[str]:
    """Reassemble null-terminated strings from consecutive ``dc.b`` fragments.

    A ``0`` byte closes the pending string; a ``0`` with nothing pending is a complete
    empty entry (the ``dc.b 0`` placeholder slots).
    """
    strings, pending = [], bytearray()
    for ln in body:
        if ln.directive != "dc.b":
            raise ValueError(f"narr.asm:{ln.no}: unexpected {ln.directive} in string block")
        for b in dcb_bytes(ln.operands, symbols):
            if b == 0:
                strings.append(pending.decode("latin-1"))
                pending = bytearray()
            else:
                pending.append(b)
    if pending:
        raise ValueError("string block ends with an unterminated string")
    return strings


def table_from_block(body: list[Line], symbols: dict[str, int]) -> list[dict]:
    rows = []
    for ln in body:
        vals = dcb_bytes(ln.operands, symbols)
        if len(vals) != 3:
            raise ValueError(f"narr.asm:{ln.no}: table row has {len(vals)} bytes, expected 3")
        rows.append({"lo": vals[0], "hi": vals[1], "msg_index": vals[2],
                     "comment": ln.comment, "line": ln.no})
    return rows


def pointer_table(lines: list[Line], label: str) -> list[str]:
    """Names referenced by a ``dc.l name-base, ...`` offset table (``qq`` / ``mst``)."""
    _, body = block_after(lines, label)
    first = next(ln for ln in lines if ln.label == label)
    names = []
    for ln in [first] + body:
        if ln.directive != "dc.l":
            raise ValueError(f"narr.asm:{ln.no}: expected dc.l in pointer table {label}")
        for op in split_operands(ln.operands):
            name, _, base = op.partition("-")
            if base.strip() != label:
                raise ValueError(f"narr.asm:{ln.no}: offset {op!r} is not relative to {label}")
            names.append(name.strip())
    return names


def read_answers(path: Path) -> list[str]:
    """The riddle answers: ``char *answers[] = { "LIGHT", ... };`` (src/fmain2.c:1306-1307)."""
    m = re.search(r"char\s*\*\s*answers\s*\[\s*\]\s*=\s*\{(.*?)\}", path.read_text(errors="replace"),
                  re.DOTALL)
    if not m:
        raise ValueError(f"answers[] not found in {path}")
    return re.findall(r'"([^"]*)"', m.group(1))


def labelled_bytes(lines: list[Line], label: str, symbols: dict[str, int]) -> tuple[int, bytes]:
    """All ``dc.b`` bytes belonging to *label* (same line and following unlabelled lines)."""
    first = next(ln for ln in lines if ln.label == label)
    _, body = block_after(lines, label)
    out = bytearray()
    for ln in ([first] if first.directive == "dc.b" else []) + body:
        if ln.directive != "dc.b":  # e.g. the `dc.w 0` pad after q8 (narr.asm:82)
            break
        out += dcb_bytes(ln.operands, symbols)
    return first.no, bytes(out)


# --------------------------------------------------------------------------- #
# Placard decoding (mirrors _ssp, fsubs.asm:497-536)
# --------------------------------------------------------------------------- #
XY_MARKER = 128
ETX = 0


def decode_ssp(data: bytes) -> list[dict]:
    """Decode one ``_ssp`` byte stream into positioned text segments.

    Each segment is ``{x_half, x, y, text}``. ``x_half``/``x``/``y`` are ``None`` for a text
    run that has no ``XY`` marker in front of it (the stream continues from the current
    pen position -- these entries are printed right after a ``name()`` call).
    """
    segments: list[dict] = []
    i = 0
    while i < len(data):
        b = data[i]
        if b == ETX:
            i += 1
            break
        if b == XY_MARKER:
            x_half, y = data[i + 1], data[i + 2]
            segments.append({"x_half": x_half, "x": x_half * 2, "y": y, "text": ""})
            i += 3
            continue
        j = i
        while j < len(data) and data[j] != ETX and data[j] < 128:
            j += 1
        text = data[i:j].decode("latin-1")
        if segments and segments[-1]["text"] == "" and segments[-1]["x_half"] is not None:
            segments[-1]["text"] = text
        else:
            segments.append({"x_half": None, "x": None, "y": None, "text": text})
        i = j
    else:
        raise ValueError("placard stream has no ETX terminator")
    if i != len(data):
        raise ValueError(f"{len(data) - i} stray byte(s) after ETX")
    return segments


# --------------------------------------------------------------------------- #
# Driver
# --------------------------------------------------------------------------- #
@dataclass
class Extraction:
    strings: dict[str, dict] = field(default_factory=dict)
    tables: dict[str, dict] = field(default_factory=dict)
    question: dict = field(default_factory=dict)
    placard_text: dict = field(default_factory=dict)


def extract(src_dir: Path) -> Extraction:
    path = src_dir / SOURCE_FILE
    lines = parse_lines(path.read_text(encoding="latin-1"))
    symbols = collect_symbols(lines)
    source = f"src/{SOURCE_FILE}"
    result = Extraction()

    for label, name in STRING_BLOCKS.items():
        line_no, body = block_after(lines, label)
        strings = strings_from_block(body, symbols)
        result.strings[name] = {
            "name": label, "source": source, "line": line_no,
            "lines": f"{line_no}-{body[-1].no}", "strings": strings,
        }

    for label, name in TABLE_BLOCKS.items():
        line_no, body = block_after(lines, label)
        rows = table_from_block(body, symbols)
        result.tables[name] = {
            "name": label, "source": source, "line": line_no,
            "lines": f"{line_no}-{body[-1].no}",
            "fields": ["lo", "hi", "msg_index", "comment", "line"],
            "rows": rows,
            "semantics": ("Rows are scanned in order; the first row with lo <= code <= hi "
                          "selects msg_index into the matching *_msg list (0 = no message)."),
        }

    q_names = pointer_table(lines, "qq")
    answers = read_answers(src_dir / "fmain2.c")
    if len(answers) != len(q_names):
        raise ValueError(f"{len(answers)} answers for {len(q_names)} questions")
    rows, q_lines = [], []
    for idx, (qn, answer) in enumerate(zip(q_names, answers)):
        line_no, data = labelled_bytes(lines, qn, symbols)
        if not data.endswith(b"\0") or b"\0" in data[:-1]:
            raise ValueError(f"{qn}: expected exactly one null-terminated string")
        rows.append({"index": idx, "label": qn, "line": line_no,
                     "question": data[:-1].decode("latin-1"), "answer": answer})
        q_lines.append(line_no)
    result.question = {
        "name": "_question", "source": source, "line": q_lines[0],
        "lines": f"{q_lines[0]}-{q_lines[-1]}",
        "answers_source": "src/fmain2.c:1306-1307",
        "fields": ["index", "label", "line", "question", "answer"],
        "rows": rows,
        "semantics": ("copy_protect_junk picks j = rand8() and shows question(j), then compares "
                      "the typed text against answers[j] (src/fmain2.c:1316-1331); the manual's "
                      "riddle answers, uppercase."),
    }

    m_names = pointer_table(lines, "mst")
    entries = []
    for idx, mn in enumerate(m_names):
        line_no, data = labelled_bytes(lines, mn, symbols)
        entries.append({"index": idx, "label": mn, "line": line_no,
                        "segments": decode_ssp(data)})
    result.placard_text = {
        "name": "_placard_text", "source": source,
        "format": ("Decoded as _ssp (src/fsubs.asm:497-536): byte 128 = XY marker followed by "
                   "x/2 and y (pixel x = x_half * 2; the source writes e.g. 21/2, which assembles to "
                   "10, so x = 20), text runs end at byte 0 or any byte >= 128, "
                   "byte 0 (ETX) ends the entry. Segments without a position continue from the "
                   "current pen position (the caller prints the brother's name in between, e.g. "
                   "src/fmain2.c:1588)."),
        "entries": entries,
    }
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--src-dir", type=Path, default=REPO_ROOT / "src",
                        help="Original source directory (default: src/).")
    parser.add_argument("--out-dir", type=Path, default=REPO_ROOT / "assets" / "text",
                        help="Output directory (default: assets/text).")
    args = parser.parse_args(argv)

    ex = extract(args.src_dir)
    for name, obj in ex.strings.items():
        ac.write_json(args.out_dir / f"{name}.json", obj)
        print(f"{name}.json: {len(obj['strings'])} strings")
    for name, obj in ex.tables.items():
        ac.write_json(args.out_dir / f"{name}.json", obj)
        print(f"{name}.json: {len(obj['rows'])} rows")
    ac.write_json(args.out_dir / "question.json", ex.question)
    print(f"question.json: {len(ex.question['rows'])} rows")
    ac.write_json(args.out_dir / "placard_text.json", ex.placard_text)
    print(f"placard_text.json: {len(ex.placard_text['entries'])} entries")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
