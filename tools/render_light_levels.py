#!/usr/bin/env python3
"""Render the light-level reference images in ``assets/shaders/previews/``.

Non-authoritative preview artifacts (``assets/tasks/_SHARED.md`` -> Previews) that double as
golden images for the port: every subject (region tile atlas, actor sheet) is shown as the game
palette-fades it, computed with the verified Python port of ``fade_page()``
(``experiment/shaders/fade_page.py``, ``src/fmain2.c:377-419``) -- never with the GLSL.

* Light levels follow ``day_fade()`` (``src/fmain2.c:1653-1660``): outdoors
  ``fade_page(lightlevel-80, lightlevel-61, lightlevel-62, TRUE, pagecolors)``. The canonical
  levels are ``fade_page.CANONICAL_LEVELS`` minus 86 (identical to 0: every channel is pinned to
  its night floor below lightlevel 87).
* The Green Jewel (``light_timer``, ``src/fmain.c:3306``) adds 200 to the red weight and lifts
  each colour's red nibble to its green nibble (``src/fmain2.c:1655``, ``:407``); rendered at
  deep night, where it is visible.
* Indoors (regions 8 and 9) ``day_fade`` always passes ``(100, 100, 100)`` (``src/fmain2.c:1659``),
  so those atlases are rendered at full brightness only.
* Colour 31 is the per-region value ``fade_page`` installs (``src/fmain2.c:381-386``); on actor
  sheets index 31 is transparent (``asset_common.TRANSPARENT_INDEX``).
* Only actors that appear outdoors are rendered (T3.1 review, 2026-10-02): Loraii, the
  necromancer and the woodcutter live in the astral plane, the dragon in his cave, and every NPC
  except the beggar, ranger, spectre and ghost is indoors -- none of them is ever palette-faded.

Each subject also gets a labelled ``*_strip.png`` with every render side by side, and
``previews.json`` indexes everything for the review app's shader viewer (bank layers, compare).

Usage::

    python tools/render_light_levels.py          # writes assets/shaders/previews/
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont

TOOLS_DIR = Path(__file__).resolve().parent
REPO_ROOT = TOOLS_DIR.parent
for p in (TOOLS_DIR, REPO_ROOT / "experiment" / "shaders"):
    if str(p) not in sys.path:
        sys.path.insert(0, str(p))

import asset_common as ac  # noqa: E402
import fade_page as fp  # noqa: E402

LEVELS = [lv for lv in fp.CANONICAL_LEVELS if lv != 86]
INDOOR_REGIONS = (8, 9)  # src/fmain2.c:1657-1659
ACTOR_GROUPS = ("heroes", "enemies", "npcs", "carriers")
INDOOR_ONLY_ACTORS = {  # never outdoors, so never faded (T3.1 review decision)
    "loraii", "necromancer", "woodcutter", "dragon",
    "wizard", "priest", "guard", "princess", "king", "noble", "sorceress", "bartender", "witch",
}


def palette_rgba(level: int, region: int, jewel: bool, transparent31: bool) -> np.ndarray:
    """32x4 uint8 LUT for one light level, from the fade_page port."""
    pal = fp.fade_palette_at(level, region_num=region, torch=jewel)
    lut = np.array([ac.rgb4_to_rgba8(c) for c in pal], dtype=np.uint8)
    if transparent31:
        lut[ac.TRANSPARENT_INDEX, 3] = 0
    return lut


def load_indices(path: Path) -> np.ndarray:
    im = Image.open(path)
    if im.mode != "P":
        raise ValueError(f"{path}: expected an indexed PNG, got mode {im.mode}")
    return np.array(im, dtype=np.uint8)


def render(idx: np.ndarray, lut: np.ndarray) -> Image.Image:
    return Image.fromarray(lut[idx], "RGBA")


def strip(renders: list[tuple[str, Image.Image]]) -> Image.Image:
    """Lay the labelled renders out side by side (tall subjects) or stacked (wide subjects)."""
    font = ImageFont.load_default()
    label_h = 12
    w, h = renders[0][1].size
    horizontal = h >= w
    pad = 4
    if horizontal:
        out = Image.new("RGBA", ((w + pad) * len(renders) - pad, h + label_h), (40, 40, 40, 255))
    else:
        out = Image.new("RGBA", (w, (h + label_h + pad) * len(renders) - pad), (40, 40, 40, 255))
    draw = ImageDraw.Draw(out)
    for n, (label, im) in enumerate(renders):
        x, y = ((w + pad) * n, 0) if horizontal else (0, (h + label_h + pad) * n)
        draw.text((x + 1, y), label, fill=(255, 255, 255, 255), font=font)
        out.alpha_composite(im, (x, y + label_h))
    return out


def subjects(assets: Path) -> list[dict]:
    subs = []
    for r in range(10):
        d = assets / "tiles" / f"region_{r:02d}"
        subs.append({
            "id": f"region_{r:02d}", "kind": "tiles", "region": r,
            "label": f"region {r}",
            "indexed": f"tiles/region_{r:02d}/atlas_indexed.png",
            "highlight_mask": f"tiles/region_{r:02d}/atlas_highlightmask.png",
            "levels": [180] if r in INDOOR_REGIONS else LEVELS,
            "jewel": r not in INDOOR_REGIONS,
            "transparent31": False, "path": d / "atlas_indexed.png",
        })
    for g in ACTOR_GROUPS:
        for d in sorted(p for p in (assets / "sprites" / g).iterdir() if p.is_dir()):
            if d.name in INDOOR_ONLY_ACTORS:
                continue
            subs.append({
                "id": d.name, "kind": "actor", "region": 0, "label": d.name,
                "indexed": f"sprites/{g}/{d.name}/{d.name}_sheet.png",
                "highlight_mask": f"sprites/{g}/{d.name}/{d.name}_highlightmask.png",
                "levels": LEVELS, "jewel": True,
                "transparent31": True, "path": d / f"{d.name}_sheet.png",
            })
    return subs


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--assets", type=Path, default=REPO_ROOT / "assets")
    args = ap.parse_args()
    out = args.assets / "shaders" / "previews"
    out.mkdir(parents=True, exist_ok=True)

    index = []
    total = 0
    for s in subjects(args.assets):
        idx = load_indices(s["path"])
        renders: list[tuple[str, Image.Image]] = []
        files = {}
        for lv in s["levels"]:
            im = render(idx, palette_rgba(lv, s["region"], False, s["transparent31"]))
            name = f"{s['id']}_light{lv:03d}.png"
            im.save(out / name, optimize=True)
            files[str(lv)] = name
            renders.append((f"light {lv}", im))
        if s["jewel"]:
            im = render(idx, palette_rgba(0, s["region"], True, s["transparent31"]))
            name = f"{s['id']}_jewel_light000.png"
            im.save(out / name, optimize=True)
            files["jewel"] = name
            renders.append(("green jewel, light 0", im))
        strip(renders).save(out / f"{s['id']}_strip.png", optimize=True)
        total += len(renders) + 1
        index.append({
            "id": s["id"], "kind": s["kind"], "region": s["region"],
            "source": {"indexed": s["indexed"], "highlight_mask": s["highlight_mask"]},
            "levels": s["levels"], "files": files, "strip": f"{s['id']}_strip.png",
        })
        print(f"{s['id']:<12} {len(renders)} renders")

    ac.write_json(out / "previews.json", {
        "source": "src/fmain2.c:377-419, src/fmain2.c:1653-1660",
        "renderer": "experiment/shaders/fade_page.py (bit-exact port); GLSL is NOT used here",
        "levels": LEVELS,
        "jewel_level": 0,
        "indoor_regions": list(INDOOR_REGIONS),
        "subjects": index,
    })
    print(f"{total} PNGs + previews.json -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
