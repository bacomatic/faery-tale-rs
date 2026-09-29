#!/usr/bin/env python3
"""Shadow/collision mask extractor for The Faery Tale Adventure (T2.3).

The game keeps 192 pre-built terrain occlusion masks (``shadow_mem``, 12,288 B =
``SHADOW_SZ = 8192+4096``, ``fmain.c:642``) that are stamped into the compositing
mask buffer so foreground terrain (trees, walls) hides sprites. They are the three
files ``mask``/``mask2``/``mask3`` in the ADF (``mtrack.c:48-50``: blocks 896, 904,
912, 8 blocks each) and are loaded in one go with
``load_track_range(896,24,shadow_mem,0)`` (``fmain.c:1222``).

Layout (``_maskit``, ``fsubs.asm:1047-1083``): entry ``n`` starts at ``n * 64``
bytes (``lsl.l #6,d0``, ``fsubs.asm:1060``) and is 32 rows of one 16-bit word
each, copied row by row into a bitplane (``fsubs.asm:1069-1080``: 8 x 4 words).
The word is written into ``bmask_mem`` (a plane of ``bm_text``, ``fmain.c:877``)
that the blitter then reads as channel A/C (``fsubs.asm:1920-1928``), so the bit
order is the Amiga bitplane order: MSB = leftmost pixel. ``mask_blit`` computes
``bmask = sprite_mask AND NOT bmask`` (miniterm $0B50, ``fsubs.asm:1934``), so a
**set bit means the terrain is in front and the sprite pixel is hidden**.

The entry number is byte 0 of the drawn tile's 4-byte ``terra_mem`` record:
``cm = minimap[..] * 4; maskit(xm, ym, blitwide, terra_mem[cm])``
(``fmain.c:2577-2578, 2595``); ``terra_mem`` is ``unsigned char`` (``fmain.c:652-658``).

Outputs (``assets/masks/``): ``mask_NNN.png`` (1-bit greyscale, set = white/opaque,
clear = transparent, via ``asset_common.write_bit_mask``), ``masks_sheet.png``
(all 192 in index order, 16 per row) and ``masks.json`` (bit layout).

Usage::

    python tools/extract_masks.py            # -> assets/masks
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

import numpy as np

TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import asset_common as ac  # noqa: E402

REPO_ROOT = TOOLS_DIR.parent
BLOCK_SIZE = 512
ENTRY_BYTES = 64      # fsubs.asm:1060  (char number * 64 bytes per char)
WIDTH, HEIGHT = 16, 32  # one word per row (fsubs.asm:1071), 8 x 4 rows (fsubs.asm:1069-1080)
SHEET_COLS = 16


def mask_blocks(src_dir: Path) -> tuple[int, int, list[dict]]:
    """(first_block, block_count, files) of the contiguous mask* diskmap entries (mtrack.c:48-50)."""
    files = []
    for n, line in enumerate((src_dir / "mtrack.c").read_text(errors="replace").splitlines(), 1):
        m = re.match(r'\s*\{\s*"(mask\d*)",\s*(\d+),\s*(\d+)\s*\}', line)
        if m:
            files.append({"name": m.group(1), "block_start": int(m.group(2)),
                          "block_count": int(m.group(3)), "source": f"src/mtrack.c:{n}"})
    first = files[0]["block_start"]
    for f, g in zip(files, files[1:]):
        if g["block_start"] != f["block_start"] + f["block_count"]:
            raise SystemExit("mask files are not contiguous in mtrack.c diskmap")
    return first, sum(f["block_count"] for f in files), files


def decode_masks(image: bytes, first_block: int, block_count: int) -> np.ndarray:
    """Return a (count, 32, 16) uint8 array of bits, MSB of each row word = leftmost pixel."""
    raw = np.frombuffer(image, dtype=np.uint8, count=block_count * BLOCK_SIZE,
                        offset=first_block * BLOCK_SIZE)
    return np.unpackbits(raw.reshape(-1, HEIGHT, WIDTH // 8), axis=2)


def encode_masks(bits: np.ndarray) -> bytes:
    """Inverse of decode_masks (round-trip check)."""
    return np.packbits(bits, axis=2).tobytes()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    ac.add_io_args(parser)
    parser.add_argument("--out-dir", type=Path, default=REPO_ROOT / "assets" / "masks")
    args = parser.parse_args(argv)

    first, count, files = mask_blocks(args.src_dir)
    image = (args.game_dir / "image").read_bytes()
    masks = decode_masks(image, first, count)
    n = len(masks)

    for i, m in enumerate(masks):
        ac.write_bit_mask(args.out_dir / f"mask_{i:03d}.png", m)

    rows = (n + SHEET_COLS - 1) // SHEET_COLS
    sheet = np.zeros((rows * HEIGHT, SHEET_COLS * WIDTH), dtype=np.uint8)
    for i, m in enumerate(masks):
        r, c = divmod(i, SHEET_COLS)
        sheet[r * HEIGHT:(r + 1) * HEIGHT, c * WIDTH:(c + 1) * WIDTH] = m
    ac.write_bit_mask(args.out_dir / "masks_sheet.png", sheet)

    ac.write_json(args.out_dir / "masks.json", {
        "entry_count": n,
        "entry_bytes": ENTRY_BYTES,
        "width": WIDTH,
        "height": HEIGHT,
        "bits_per_pixel": 1,
        "row_stride_bytes": WIDTH // 8,
        "bit_order": "big-endian 16-bit word per row, MSB = leftmost pixel (Amiga bitplane)",
        "set_bit_means": "terrain in front: the sprite pixel under it is hidden "
                         "(bmask = sprite_mask AND NOT bmask, blitter miniterm $0B50)",
        "png": "1-bit greyscale; set bit = white/opaque, clear bit = transparent (tRNS 0)",
        "sheet": {"file": "masks_sheet.png", "columns": SHEET_COLS,
                  "note": "entry i at column i % 16, row i // 16"},
        "disk": {"image": "src/assets/image", "first_block": first, "block_count": count,
                 "block_size": BLOCK_SIZE, "files": files,
                 "load": "load_track_range(896,24,shadow_mem,0)",
                 "total_bytes": count * BLOCK_SIZE, "total_bytes_define": "SHADOW_SZ 8192+4096"},
        "indexing": {
            "entry_address": "shadow_mem + n * 64",
            "entry_source": "byte 0 of the drawn tile's 4-byte terra_mem record: "
                            "cm = minimap[(xm + xbw) * 6 + ym + ym1] * 4; maskit(xm, ym, blitwide, "
                            "terra_mem[cm]); terra_mem is unsigned char, so n is the stored byte "
                            "0..255; SHADOW_SZ holds entries 0..191 (12288 / 64)",
            "applied_when": "terra_mem[cm + 1] & 15 (occlusion code 0..7) permits it",
            "rows_copied": "32 (8 iterations x 4 words), one word per bitplane row of blitwide words",
        },
        "sources": {
            "SHADOW_SZ": "src/fmain.c:642",
            "shadow_mem alloc": "src/fmain.c:924",
            "load": "src/fmain.c:1222",
            "diskmap mask/mask2/mask3": "src/mtrack.c:48-50",
            "maskit": "src/fsubs.asm:1047-1083",
            "entry * 64": "src/fsubs.asm:1060-1061",
            "row copy loop": "src/fsubs.asm:1069-1080",
            "dest cell offset y*32*mod + x*2": "src/fsubs.asm:1063-1067",
            "bmask_mem is a bitplane": "src/fmain.c:877",
            "caller / index from terra_mem": "src/fmain.c:2577-2578, 2595",
            "occlusion code gate": "src/fmain.c:2579-2594",
            "occlusion code comment": "src/fmain.c:689-691",
            "terra_mem unsigned char": "src/fmain.c:652-658",
            "mask_blit D = A AND NOT C": "src/fsubs.asm:1920-1935",
        },
    })
    print(f"masks: {n} entries of {WIDTH}x{HEIGHT} from blocks {first}-{first + count - 1} -> {args.out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
