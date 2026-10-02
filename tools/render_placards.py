#!/usr/bin/env python3
"""Render the placard cards from ``assets/text/placard_text.json`` with the Amber/9 font.

Non-authoritative preview artifacts (see ``assets/tasks/_SHARED.md`` -> Previews). Each PNG is
one card as the game composes it on the 320x200 ``rp_map`` page:

* text is drawn by ``_ssp`` (``src/fsubs.asm:497-536``): ``XY`` = ``Move(x_half*2, y)`` to the
  baseline, then ``Text`` runs in the current pen; ``name()`` splices the brother's name at the
  current pen position (``src/fmain2.c:592-593``, ``datanames`` at ``src/fmain.c:604``);
* the font is ``afont`` = Amber/9 (``src/fmain.c:2860``, ``src/fmain2.c:1586``), drawn like
  the Amiga ``Text()`` does with a proportional font: pen x += ``kern``, blit the glyph with its
  top at ``baseline_y - tf_Baseline``, pen x += ``space``;
* the border is ``_placard`` (``src/fsubs.asm:387-475``) in its final state (every stroke ends
  in pen 24); the copy-protection lead-in (``src/fmain.c:1229-1235``) has no border, uses
  pen 1 on a ``0x006`` background, and is drawn in ``tfont`` = topaz-8: ``rp_map`` gets
  ``tfont`` at ``src/fmain.c:781`` and nothing sets ``afont`` on it before line 1235.

Card compositions follow the call sites: ``src/fmain.c:2859-2879`` (intro / death / set-out /
game over), ``src/fmain2.c:1586-1591`` (rescues, departure), ``src/fmain2.c:1607`` (victory),
``src/fmain.c:1235`` (copy protection).

Usage::

    python tools/render_placards.py            # writes assets/text/previews/
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from PIL import Image

TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import asset_common as ac  # noqa: E402

REPO_ROOT = TOOLS_DIR.parent
WIDTH, HEIGHT = 320, 200

# _placard stroke tables, fsubs.asm:382-385 (MOD = 4)
XMOD = [-4, -4, -4, 0, 0, 0, 4, 4, 0, -4, 0, 4, 4, 0, 0, 0]
YMOD = [0, 0, 0, 4, 4, 4, 0, 0, -4, 0, -4, 0, 0, 4, 4, 4]

# (file stem, [entry index | ("name",)], has_border, pen, background rgb4, font)
NAME = ("name",)
CARDS = [
    ("01_intro_julian",      [0],                        True,  24, 0x000, "amber_9"),  # fmain.c:2862
    ("02_death_julian",      [1],                        True,  24, 0x000, "amber_9"),  # fmain.c:2866
    ("03_setout_phillip",    [2],                        True,  24, 0x000, "amber_9"),  # fmain.c:2877 (border from the previous card)
    ("04_death_phillip",     [3],                        True,  24, 0x000, "amber_9"),  # fmain.c:2867
    ("05_setout_kevin",      [4],                        True,  24, 0x000, "amber_9"),  # fmain.c:2877
    ("06_game_over",         [5],                        True,  24, 0x000, "amber_9"),  # fmain.c:2868
    ("07_victory",           [6, NAME, 7],               True,  24, 0x000, "amber_9"),  # fmain2.c:1607
    ("08_rescue_katra",      [8, NAME, 9, NAME, 10],     True,  24, 0x000, "amber_9"),  # fmain2.c:1588, princess 0
    ("09_rescue_karla",      [11, NAME, 12, NAME, 13],   True,  24, 0x000, "amber_9"),  # princess 1
    ("10_rescue_kandy",      [14, NAME, 15, NAME, 16],   True,  24, 0x000, "amber_9"),  # princess 2
    ("11_departure",         [17, NAME, 18],             True,  24, 0x000, "amber_9"),  # fmain2.c:1591 (border from the rescue card)
    ("12_copy_protection",   [19],                       False, 1,  0x006, "topaz_8"),  # fmain.c:781, 1226-1235
]


class Font:
    def __init__(self, font_dir: Path):
        self.meta = json.loads((font_dir / f"{font_dir.name}.json").read_text())
        self.baseline = self.meta["baseline"]
        self.lo, self.hi = self.meta["lo_char"], self.meta["hi_char"]
        self.glyphs = {}
        for g in self.meta["glyphs"]:
            img = Image.open(font_dir / g["file"]).convert("L") if g["file"] else None
            self.glyphs[g["code"]] = (g, img)   # code None = default glyph

    def glyph(self, ch: str):
        code = ord(ch)
        return self.glyphs[code if self.lo <= code <= self.hi else None]

    def draw_text(self, px, x: int, y: int, text: str, pen) -> int:
        """Amiga Text(): per char pen x += kern, blit at (x, y - baseline), pen x += space.

        Monospace fonts (topaz) have no kern/space tables: kern 0, advance tf_XSize.
        """
        for ch in text:
            g, img = self.glyph(ch)
            gx, gy = x + g.get("kern", 0), y - self.baseline
            if img is not None:
                w, h = img.size
                data = img.load()
                for yy in range(h):
                    for xx in range(g["width"]):
                        if data[xx, yy] and 0 <= gx + xx < WIDTH and 0 <= gy + yy < HEIGHT:
                            px[gx + xx, gy + yy] = pen
            x += g.get("space", self.meta["x_size"])
        return x


def draw_line(px, x0: int, y0: int, x1: int, y1: int, pen) -> None:
    """Bresenham, both endpoints inclusive (Amiga Draw), clipped to the canvas."""
    dx, dy = abs(x1 - x0), -abs(y1 - y0)
    sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
    err = dx + dy
    while True:
        if 0 <= x0 < WIDTH and 0 <= y0 < HEIGHT:
            px[x0, y0] = pen
        if x0 == x1 and y0 == y1:
            return
        e2 = 2 * err
        if e2 >= dy:
            err += dy
            x0 += sx
        if e2 <= dx:
            err += dx
            y0 += sy


def draw_border(px, pen) -> None:
    """Final frame of _placard (fsubs.asm:387-475): all strokes end in pen 24."""
    xorg, yorg = 12, 0                                   # fsubs.asm:392-393
    for i in range(16, -1, -1):                          # fsubs.asm:395, 472
        for j in range(16):                              # fsubs.asm:397, 469
            dx, dy = xorg + XMOD[j], yorg + YMOD[j]      # fsubs.asm:400-411
            if i > 9:                                    # fsubs.asm:420-422
                draw_line(px, xorg, yorg, dx, dy, pen)                           # :424-429
                draw_line(px, 284 - xorg, 124 - yorg, 284 - dx, 124 - dy, pen)   # :431-440
            draw_line(px, 16 + yorg, 12 - xorg, 16 + dy, 12 - dx, pen)           # :442-451
            draw_line(px, 268 - yorg, 112 + xorg, 268 - dy, 112 + dx, pen)       # :453-462
            xorg, yorg = dx, dy                          # fsubs.asm:466-467


def render_card(entries: list[dict], font: Font, parts: list, border: bool, pen: int,
                bg: int, palette: list[tuple], name: str) -> Image.Image:
    img = Image.new("RGB", (WIDTH, HEIGHT), palette[0][:3])
    px = img.load()
    colour = palette[pen][:3]
    x = y = 0
    for part in parts:
        if part == NAME:                                 # name(): print_cont at the current pen
            x = font.draw_text(px, x, y, name, colour)
            continue
        for seg in entries[part]["segments"]:
            if seg["x_half"] is not None:                # XY -> Move(x_half*2, y)
                x, y = seg["x"], seg["y"]
            x = font.draw_text(px, x, y, seg["text"], colour)
    if border:
        draw_border(px, palette[24][:3])
    return img


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--assets-dir", type=Path, default=REPO_ROOT / "assets")
    parser.add_argument("--name", default="Julian",
                        help="brother name spliced in by name() (datanames, src/fmain.c:604)")
    args = parser.parse_args(argv)
    assets = args.assets_dir

    entries = json.loads((assets / "text" / "placard_text.json").read_text())["entries"]
    fonts = {name: Font(assets / "fonts" / name) for name in ("amber_9", "topaz_8")}
    pagecolors = json.loads((assets / "palettes" / "pagecolors.json").read_text())
    base = [tuple(e["rgba8"]) for e in pagecolors]
    out_dir = assets / "text" / "previews"
    out_dir.mkdir(parents=True, exist_ok=True)

    for stem, parts, border, pen, bg, font in CARDS:
        palette = list(base)
        palette[0] = ac.rgb4_to_rgba8(bg)
        if pen == 1:
            palette[1] = ac.rgb4_to_rgba8(0xFFF)         # SetRGB4(&vp_page,1,15,15,15), fmain.c:1229
        img = render_card(entries, fonts[font], parts, border, pen, bg, palette, args.name)
        path = out_dir / f"placard_{stem}.png"
        img.save(path, optimize=True)
        print(path.relative_to(REPO_ROOT))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
