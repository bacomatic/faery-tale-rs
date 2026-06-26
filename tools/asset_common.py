"""Shared helpers for Faery Tale asset extractors.

All conversions here are *pixel-/byte-exact*. No gameplay, engine, rendering,
or creative changes. See ``assets/tasks/_SHARED.md`` for the conventions:

* Amiga 12-bit colour ``0x0RGB`` -> ``rgba8`` by nibble replication (``0xF -> 0xFF``).
* Transparency convention: sprite/tile palette index **31** is transparent.
* Highlight mask: 1 bit/pixel, set where the source palette index is in
  **16..24** (inclusive); pixels with no highlight (which includes the
  transparent index 31) are transparent in the mask.

PNG output is produced with the standard library only (``zlib`` + manual chunk
encoding) so that re-runs are byte-for-byte identical and do not depend on any
third-party encoder's metadata.
"""

from __future__ import annotations

import argparse
import json
import struct
import zlib
from pathlib import Path
from typing import Any, Iterable, Sequence

# Default locations (never hardcode the sibling path elsewhere -- use these).
DEFAULT_GAME_DIR = "../faery-tale-rs/game"
DEFAULT_SRC_DIR = "src/"

# Conventions
TRANSPARENT_INDEX = 31
HIGHLIGHT_LO = 16
HIGHLIGHT_HI = 24  # inclusive


# --------------------------------------------------------------------------- #
# Argument helpers
# --------------------------------------------------------------------------- #
def add_io_args(parser: argparse.ArgumentParser) -> argparse.ArgumentParser:
    """Add the standard ``--game-dir`` / ``--src-dir`` options to *parser*.

    ``--game-dir`` defaults to the sibling checkout's ``game/`` directory. The
    repo ships a ``game`` symlink pointing there; if that symlink/dir is not
    available, point ``--game-dir`` at any directory containing an extracted
    disk image (the ``--image`` alternative).
    """
    parser.add_argument(
        "--game-dir",
        default=DEFAULT_GAME_DIR,
        type=Path,
        help=(
            "Directory containing the original game files "
            f"(default: {DEFAULT_GAME_DIR}). The repo's 'game' symlink points "
            "here. If unavailable, pass a directory holding an extracted disk "
            "image instead (the --image alternative)."
        ),
    )
    parser.add_argument(
        "--src-dir",
        default=DEFAULT_SRC_DIR,
        type=Path,
        help=f"Original C source directory (default: {DEFAULT_SRC_DIR}).",
    )
    return parser


def build_arg_parser(description: str = "") -> argparse.ArgumentParser:
    """Convenience: a fresh parser pre-populated with the standard IO args."""
    parser = argparse.ArgumentParser(description=description)
    add_io_args(parser)
    return parser


# --------------------------------------------------------------------------- #
# Colour conversion
# --------------------------------------------------------------------------- #
def amiga12_to_rgba8(v: int) -> tuple[int, int, int, int]:
    """Convert a 12-bit Amiga colour ``0x0RGB`` to ``(r, g, b, a)`` bytes.

    Each 4-bit channel is nibble-replicated (``0xF -> 0xFF``, ``0x1 -> 0x11``).
    Alpha is always 255 (opaque); transparency is handled per-pixel via the
    palette index, not the colour value.

    >>> amiga12_to_rgba8(0x0FFF)
    (255, 255, 255, 255)
    >>> amiga12_to_rgba8(0x0123)
    (17, 34, 51, 255)
    >>> amiga12_to_rgba8(0x0F0F)
    (255, 0, 255, 255)
    """
    v &= 0xFFF
    r = (v >> 8) & 0xF
    g = (v >> 4) & 0xF
    b = v & 0xF
    return (r * 0x11, g * 0x11, b * 0x11, 255)


# --------------------------------------------------------------------------- #
# Low-level PNG encoding (stdlib only, deterministic)
# --------------------------------------------------------------------------- #
_PNG_SIG = b"\x89PNG\r\n\x1a\n"


def _chunk(tag: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + tag
        + data
        + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    )


def _write_png(path, *, width: int, height: int, bit_depth: int,
               color_type: int, idat: bytes, plte: bytes | None = None,
               trns: bytes | None = None) -> None:
    ihdr = struct.pack(
        ">IIBBBBB", width, height, bit_depth, color_type, 0, 0, 0
    )
    out = bytearray(_PNG_SIG)
    out += _chunk(b"IHDR", ihdr)
    if plte is not None:
        out += _chunk(b"PLTE", plte)
    if trns is not None:
        out += _chunk(b"tRNS", trns)
    out += _chunk(b"IDAT", zlib.compress(idat, 9))
    out += _chunk(b"IEND", b"")
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_bytes(bytes(out))


def _rows(arr: Sequence) -> list:
    """Normalise a 2D array-like (list of rows, numpy array, ...) to a list."""
    rows = list(arr)
    if not rows:
        raise ValueError("image array must have at least one row")
    return rows


# --------------------------------------------------------------------------- #
# Public PNG writers
# --------------------------------------------------------------------------- #
def write_indexed_png(path, idx_array: Sequence[Sequence[int]],
                      palette: Sequence[Sequence[int]]) -> None:
    """Write an 8-bit indexed (palette) PNG.

    *idx_array* is a 2D array of palette indices (0..len(palette)-1; the game
    uses 0..31). *palette* is a sequence of ``(r, g, b)`` (or ``(r, g, b, a)``)
    tuples. Index values are preserved exactly (PNG colour type 3), so the
    image round-trips losslessly back to the same indices.

    Palette index ``31`` is written as transparent (tRNS), per the project's
    transparency convention.
    """
    rows = _rows(idx_array)
    height = len(rows)
    width = len(rows[0])

    raw = bytearray()
    for row in rows:
        if len(row) != width:
            raise ValueError("all rows must have the same width")
        raw.append(0)  # filter type: None
        raw.extend(int(p) & 0xFF for p in row)

    plte = bytearray()
    for entry in palette:
        plte.extend((int(entry[0]) & 0xFF, int(entry[1]) & 0xFF,
                     int(entry[2]) & 0xFF))

    # Transparency: index 31 transparent; everything else opaque.
    n = len(palette)
    alpha = [0xFF] * n
    if n > TRANSPARENT_INDEX:
        alpha[TRANSPARENT_INDEX] = 0x00
    while alpha and alpha[-1] == 0xFF:  # PNG allows omitting trailing opaque
        alpha.pop()
    trns = bytes(alpha) if alpha else None

    _write_png(path, width=width, height=height, bit_depth=8, color_type=3,
               idat=bytes(raw), plte=bytes(plte), trns=trns)


def write_rgba_png(path, rgba_array: Sequence[Sequence[Sequence[int]]]) -> None:
    """Write a 32-bit RGBA (truecolour + alpha) PNG.

    *rgba_array* is a 2D array whose elements are ``(r, g, b, a)`` byte tuples.
    """
    rows = _rows(rgba_array)
    height = len(rows)
    width = len(rows[0])

    raw = bytearray()
    for row in rows:
        if len(row) != width:
            raise ValueError("all rows must have the same width")
        raw.append(0)  # filter type: None
        for px in row:
            raw.extend((int(px[0]) & 0xFF, int(px[1]) & 0xFF,
                        int(px[2]) & 0xFF, int(px[3]) & 0xFF))

    _write_png(path, width=width, height=height, bit_depth=8, color_type=6,
               idat=bytes(raw))


def write_highlight_mask(path, idx_array: Sequence[Sequence[int]]) -> None:
    """Write a 1-bit highlight mask PNG.

    A pixel bit is set (white, opaque) where the source palette index is in
    ``16..24`` (inclusive). All other pixels -- including the transparent index
    ``31`` -- are 0 and are marked transparent via tRNS. The result is a 1-bit
    greyscale image where the only visible pixels are the highlight ramp.
    """
    rows = _rows(idx_array)
    height = len(rows)
    width = len(rows[0])

    raw = bytearray()
    for row in rows:
        if len(row) != width:
            raise ValueError("all rows must have the same width")
        raw.append(0)  # filter type: None
        acc = 0
        nbits = 0
        for idx in row:
            bit = 1 if HIGHLIGHT_LO <= int(idx) <= HIGHLIGHT_HI else 0
            acc = (acc << 1) | bit
            nbits += 1
            if nbits == 8:
                raw.append(acc)
                acc = 0
                nbits = 0
        if nbits:
            acc <<= (8 - nbits)  # pad remaining bits with 0
            raw.append(acc)

    # Greyscale tRNS: sample value 0 is transparent (2-byte big-endian sample).
    trns = struct.pack(">H", 0)
    _write_png(path, width=width, height=height, bit_depth=1, color_type=0,
               idat=bytes(raw), trns=trns)


# --------------------------------------------------------------------------- #
# JSON
# --------------------------------------------------------------------------- #
def write_json(path, obj: Any) -> None:
    """Write *obj* as deterministic JSON.

    Keys are sorted, indentation is fixed, and a single trailing newline is
    appended. Re-running with equal input yields byte-identical output.
    """
    text = json.dumps(
        obj,
        sort_keys=True,
        indent=2,
        ensure_ascii=False,
        separators=(",", ": "),
    )
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_text(text + "\n", encoding="utf-8")
