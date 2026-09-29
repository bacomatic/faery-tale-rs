#!/usr/bin/env python3
"""IFF/ILBM screen extractor for The Faery Tale Adventure (T2.4).

Decodes the nine ``FORM ILBM`` brushes in ``src/assets`` (``page0``, ``p1a``..``p3b``,
``winpic``, ``hiscreen``) to RGBA PNGs in ``assets/screens/`` plus ``screens.json``
(BMHD metadata, where the game shows each brush, which palette is on the viewport).

The decoder follows the game's ``unpackbrush`` (``iffsubs.c:139-189``): ``BMHD`` is read,
``CMAP``/``GRAB``/``CAMG``/``CRNG`` are skipped, ``BODY`` is unpacked one row per plane
with ``_unpack_line`` (``fsubs.asm:1226-1271``, ByteRun1 when ``bmhd.compression`` is set),
row bytes ``((width+15)/8) & ~1`` (``iffsubs.c:164``). Differences from the game, on purpose:

* the game never reads ``CMAP`` (``iffsubs.c:157-159``); its palettes come from
  ``LoadRGB4`` tables in the C source. The PNGs use the palette the game has loaded when
  the screen is shown at full size (see ``SHOWN`` below); the file's own CMAP is recorded
  in ``screens.json`` and compared against it.
* the game unpacks ``bitmap->Depth`` planes, not ``bmhd.nPlanes`` (``iffsubs.c:177-181``);
  here ``nPlanes`` is used. Both agree for all nine files (5 planes into the 5-plane
  ``pageb``/page bitmaps, ``fmain.c:8,830-831``; 4 planes into the 4-plane ``bm_text``,
  ``fmain.c:828``).
* ``masking`` is ignored by the game (``unpackbrush`` never reads it); output is opaque.

Usage::

    python tools/extract_screens.py            # -> assets/screens/
"""

from __future__ import annotations

import argparse
import json
import struct
import sys
from pathlib import Path

import numpy as np

TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import asset_common as ac  # noqa: E402

REPO_ROOT = TOOLS_DIR.parent

# BitMapHeader per the IFF spec (big-endian, 20 bytes). The game's struct (iffsubs.c:28-38)
# declares xAspect/yAspect as short, so it is 22 bytes; it reads blocklength (20) bytes into
# it and only ever uses width, height and compression (iffsubs.c:164,175,176).
BMHD_FIELDS = ["width", "height", "x", "y", "n_planes", "masking", "compression", "pad1",
               "transparent_color", "x_aspect", "y_aspect", "page_width", "page_height"]
BMHD_STRUCT = struct.Struct(">HHhhBBBBHBBhh")

# unpackbrush(filename, bitmap, x, y): x is a *byte* offset into the row
# (bitoffset = x + BytesPerRow*y, iffsubs.c:141), so the pixel x is 8*x.
INTRO = {"name": "introcolors", "file": "palettes/introcolors.json",
         "source": "src/fmain.c:484-488",
         "loaded_by": "screen_size() -> fade_page(y*2-40, y*2-70, y*2-100, 0, introcolors) "
                      "(src/fmain.c:2930); at full size (x=160, src/fmain.c:1199) r,g,b clamp "
                      "to 100 and fade_page yields introcolors unchanged (src/fmain2.c:388-390, "
                      "403-418)"}
WIN = {"name": "win_colors first frame (i=25)", "file": "palettes/sun_colors.json",
       "source": "src/fmain2.c:1614-1631",
       "loaded_by": "win_colors(): fader[0]=fader[31]=0, fader[1]=fader[28]=0xfff, "
                    "fader[2..27]=sun_colors[25+j], fader[29]=0x800, fader[30]=0x400; the "
                    "i=25 frame is held for Delay(60) (src/fmain2.c:1631), then the sunset "
                    "walks i down to -29 and the page goes black (src/fmain2.c:1635)"}
TEXT = {"name": "textcolors", "file": "palettes/textcolors.json",
        "source": "src/fmain.c:476-479",
        "loaded_by": "LoadRGB4(&vp_text, textcolors, 20) (src/fmain.c:1257); vp_text shows "
                     "bm_text, 4 planes (src/fmain.c:828), so entries 0-15 are visible"}

SHOWN = {
    "page0": {"call": "src/fmain.c:1194", "dest": "pageb", "byte_x": 0, "y": 0, "palette": INTRO,
              "role": "title page; blitted to both page bitmaps, then the playfield zooms open "
                      "(src/fmain.c:1195-1199)"},
    "p1a": {"call": "src/fmain.c:1203", "via": "copypage(br1, br2, x, y) (src/fmain2.c:781-790)",
            "dest": "pageb", "byte_x": 4, "y": 24, "palette": INTRO,
            "role": "story page 1, left leaf (br1 is always drawn at byte 4 = pixel 32, "
                    "src/fmain2.c:785)"},
    "p1b": {"call": "src/fmain.c:1203", "via": "copypage(br1, br2, x, y) (src/fmain2.c:781-790)",
            "dest": "pageb", "byte_x": 21, "y": 29, "palette": INTRO,
            "role": "story page 1, right leaf (br2 at byte 21 = pixel 168, src/fmain2.c:786)"},
    "p2a": {"call": "src/fmain.c:1204", "via": "copypage(br1, br2, x, y) (src/fmain2.c:781-790)",
            "dest": "pageb", "byte_x": 4, "y": 24, "palette": INTRO,
            "role": "story page 2, left leaf"},
    "p2b": {"call": "src/fmain.c:1204", "via": "copypage(br1, br2, x, y) (src/fmain2.c:781-790)",
            "dest": "pageb", "byte_x": 20, "y": 29, "palette": INTRO,
            "role": "story page 2, right leaf (byte 20 = pixel 160)"},
    "p3a": {"call": "src/fmain.c:1205", "via": "copypage(br1, br2, x, y) (src/fmain2.c:781-790)",
            "dest": "pageb", "byte_x": 4, "y": 24, "palette": INTRO,
            "role": "story page 3, left leaf"},
    "p3b": {"call": "src/fmain.c:1205", "via": "copypage(br1, br2, x, y) (src/fmain2.c:781-790)",
            "dest": "pageb", "byte_x": 20, "y": 33, "palette": INTRO,
            "role": "story page 3, right leaf (byte 20 = pixel 160)"},
    "winpic": {"call": "src/fmain2.c:1609", "dest": "bm_draw (drawing page bitmap)", "byte_x": 0,
               "y": 0, "palette": WIN,
               "role": "victory picture, shown by win_colors() after the win placard "
                       "(src/fmain2.c:1605-1636; called when stuff[22] is held at the end, "
                       "src/fmain.c:3244-3246)"},
    "hiscreen": {"call": "src/fmain.c:1227", "dest": "bm_text (640x57, 4 planes, src/fmain.c:828)",
                 "byte_x": 0, "y": 0, "palette": TEXT,
                 "role": "status bar / HUD frame shown in vp_text below the playfield "
                         "(src/fmain.c:1250-1251)"},
}
SCREENS = list(SHOWN)


# --------------------------------------------------------------------------- #
# IFF parsing
# --------------------------------------------------------------------------- #
def parse_ilbm(data: bytes) -> dict[str, bytes]:
    """Chunks of a FORM ILBM as {tag: payload}, in file order (first occurrence wins)."""
    if data[:4] != b"FORM" or data[8:12] != b"ILBM":
        raise SystemExit("not a FORM ILBM")
    end = 8 + struct.unpack(">I", data[4:8])[0]
    chunks: dict[str, bytes] = {}
    pos = 12
    while pos < end:
        tag = data[pos:pos + 4].decode("ascii")
        n = struct.unpack(">I", data[pos + 4:pos + 8])[0]
        chunks.setdefault(tag, data[pos + 8:pos + 8 + n])
        pos += 8 + n + (n & 1)  # chunks are word-aligned
    return chunks


def unpack_byterun1(src: bytes, n_out: int) -> tuple[bytes, int]:
    """ByteRun1 (_unpack_line, fsubs.asm:1245-1270): returns (bytes, count of -128 NOPs).

    The game's decoder has no -128 case (fsubs.asm:1260 '/* branch overflow?? */' would
    repeat 129 bytes); the count lets the caller check the files never exercise it.
    """
    out = bytearray()
    nops = 0
    i = 0
    while len(out) < n_out:
        c = src[i]
        i += 1
        if c < 128:
            out += src[i:i + c + 1]
            i += c + 1
        elif c == 128:
            nops += 1
        else:
            out += src[i:i + 1] * (257 - c)
            i += 1
    if len(out) != n_out:
        raise SystemExit(f"ByteRun1 overran: {len(out)} > {n_out}")
    return bytes(out), nops


def decode_body(bmhd: dict, body: bytes) -> tuple[np.ndarray, int]:
    """(height, width) palette indices from a BODY chunk, plus the -128 NOP count."""
    w, h, planes = bmhd["width"], bmhd["height"], bmhd["n_planes"]
    row_bytes = ((w + 15) // 8) & ~1                      # iffsubs.c:164
    rows_per_line = planes + (1 if bmhd["masking"] == 1 else 0)  # mskHasMask adds a plane
    total = h * rows_per_line * row_bytes
    if bmhd["compression"] == 0:
        raw, nops = body[:total], 0
    elif bmhd["compression"] == 1:
        raw, nops = unpack_byterun1(body, total)
    else:
        raise SystemExit(f"unknown compression {bmhd['compression']}")
    lines = np.frombuffer(raw, dtype=np.uint8).reshape(h, rows_per_line, row_bytes)[:, :planes]
    bits = np.unpackbits(lines, axis=2)[:, :, :w]         # (h, planes, w)
    idx = np.zeros((h, w), dtype=np.uint8)
    for p in range(planes):
        idx |= bits[:, p] << p
    return idx, nops


# --------------------------------------------------------------------------- #
# Palettes
# --------------------------------------------------------------------------- #
def load_rgb4(path: Path) -> list[int]:
    entries = json.loads(path.read_text())
    return [int(e["rgb4"], 16) for e in sorted(entries, key=lambda e: e["index"])]


def game_palette(spec: dict, palettes_dir: Path) -> list[int]:
    rgb4 = load_rgb4(palettes_dir / Path(spec["file"]).name)
    if spec is not WIN:
        return rgb4
    sun = rgb4                                             # fmain2.c:1614-1630 with i = 25
    fader = [0] * 32
    fader[1] = fader[28] = 0xFFF
    for j in range(2, 28):
        fader[j] = sun[25 + j]
    fader[29], fader[30] = 0x800, 0x400
    return fader


def cmap_rgb4(cmap: bytes) -> list[int]:
    """CMAP triplets (8 bits/channel) as 0x0RGB; the files only carry 4-bit values (low nibbles 0)."""
    if any(b & 0x0F for b in cmap):
        raise SystemExit("CMAP has non-zero low nibbles; not a plain 12-bit palette")
    return [(cmap[i] >> 4) << 8 | (cmap[i + 1] >> 4) << 4 | (cmap[i + 2] >> 4)
            for i in range(0, len(cmap), 3)]


# --------------------------------------------------------------------------- #
# Main
# --------------------------------------------------------------------------- #
def extract(name: str, game_dir: Path, palettes_dir: Path, out_dir: Path) -> dict:
    chunks = parse_ilbm((game_dir / name).read_bytes())
    bmhd = dict(zip(BMHD_FIELDS, BMHD_STRUCT.unpack(chunks["BMHD"][:BMHD_STRUCT.size])))
    idx, nops = decode_body(bmhd, chunks["BODY"])
    shown = SHOWN[name]
    palette = game_palette(shown["palette"], palettes_dir)
    lut = np.array([ac.rgb4_to_rgba8(c) for c in palette], dtype=np.uint8)
    ac.write_rgba_png(out_dir / f"{name}.png", lut[idx])

    cmap = cmap_rgb4(chunks["CMAP"])
    n = 1 << bmhd["n_planes"]
    diffs = [{"index": i, "cmap": f"0x{cmap[i]:04x}", "game": f"0x{palette[i]:04x}"}
             for i in range(min(n, len(cmap))) if cmap[i] != palette[i]]
    entry = {
        "name": name, "file": f"{name}.png", "bmhd": bmhd,
        "chunks": list(chunks),
        "cmap_rgb4": [f"0x{c:04x}" for c in cmap],
        "shown": {k: v for k, v in shown.items() if k != "palette"},
        "palette": {**shown["palette"], "rgb4": [f"0x{c:04x}" for c in palette[:n]],
                    "cmap_differs_at": diffs},
    }
    entry["shown"]["pixel_x"] = 8 * shown["byte_x"]
    if nops:
        entry["byterun1_nop_bytes"] = nops
    return entry


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    ac.add_io_args(parser)
    parser.add_argument("--out-dir", type=Path, default=REPO_ROOT / "assets" / "screens")
    parser.add_argument("--palettes-dir", type=Path, default=REPO_ROOT / "assets" / "palettes",
                        help="directory with introcolors/textcolors/sun_colors.json (T1.1)")
    args = parser.parse_args(argv)

    screens = [extract(n, args.game_dir, args.palettes_dir, args.out_dir) for n in SCREENS]
    ac.write_json(args.out_dir / "screens.json", {
        "loader": {
            "function": "unpackbrush(filename, bitmap, x, y)", "source": "src/iffsubs.c:139-189",
            "notes": [
                "BMHD is read into bmhd (src/iffsubs.c:155-156); CMAP, GRAB, CAMG, CRNG are "
                "skipped (src/iffsubs.c:157-159), so the file palettes are never used by the game.",
                "BODY: row bytes = ((width+15)/8) & 0xfffe (src/iffsubs.c:164); one _unpack_line "
                "per plane per scanline for bitmap->Depth planes (src/iffsubs.c:176-182); "
                "compress = bmhd.compression selects ByteRun1 or raw copy (src/iffsubs.c:175, "
                "src/fsubs.asm:1230-1270). masking is never read; brushes are drawn opaque.",
                "x is a byte offset (bitoffset = x + BytesPerRow*y, src/iffsubs.c:141): pixel_x = 8*x.",
                "PNG colours: the LoadRGB4 palette on the viewport when the screen is shown "
                "(see each entry's palette.loaded_by), rgb4 -> rgba8 by nibble replication. "
                "cmap_rgb4 is the file's own CMAP (8-bit triplets, low nibbles all zero, shown as "
                "0x0RGB); palette.cmap_differs_at lists entries where it differs from the game's.",
            ],
        },
        "screens": screens,
    })
    for s in screens:
        b = s["bmhd"]
        print(f"{s['name']:9} {b['width']}x{b['height']} x{b['n_planes']} comp={b['compression']} "
              f"mask={b['masking']} cmap={len(s['cmap_rgb4'])} palette={s['palette']['name']} "
              f"cmap_diffs={len(s['palette']['cmap_differs_at'])}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
