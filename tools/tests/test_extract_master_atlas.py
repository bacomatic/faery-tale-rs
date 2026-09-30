"""Self-check for tools/extract_master_atlas.py (T2.2.1)."""
import json
import sys
from pathlib import Path

import numpy as np
import pytest
from PIL import Image

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))
import extract_master_atlas as ema  # noqa: E402

ASSETS = TOOLS.parent / "assets"
MASTER = ASSETS / "tiles" / "master"
GREEN, GREY = np.array([0, 255, 0, 255]), np.array([68, 68, 85, 255])


@pytest.fixture(scope="module")
def built():
    return ema.build(ASSETS / "tiles", ASSETS / "world", ASSETS / "palettes", ASSETS.parent / "src")


def master_tile(atlas, m):
    return atlas[m["y"]:m["y"] + 32, m["x"]:m["x"] + 16]


def test_every_used_pair_round_trips(built):
    atlas, _, shadow, meta = built
    for region in range(ema.NUM_REGIONS):
        art = ema.region_tiles(ASSETS / "tiles", region)
        used, _ = ema.used_tiles(ASSETS / "world", region, meta["usage"]["indoor_entries"])
        ref = meta["region_maps"][str(region)]
        assert len(ref) == 256
        assert {t for t, m in enumerate(ref) if m is not None} == set(used.tolist())
        shadow_bits, recs = ema.region_shadow(ASSETS / "tiles", region)
        for t in used.tolist():  # verbatim source indices, and the same shadow mask / mode
            m = meta["tiles"][ref[t]]
            assert np.array_equal(master_tile(atlas, m), art[t])
            assert (m["mask"], m["mask_mode"]) == (recs[t]["mask"], recs[t]["mask_mode"])
            assert np.array_equal(master_tile(shadow, m), shadow_bits[t])


def test_masters_are_unique(built):
    atlas, _, _, meta = built
    keys = {(master_tile(atlas, m).tobytes(), m["color_31"]["rgb4"] if m["color_31"] else None,
             m["mask"], m["mask_mode"]) for m in meta["tiles"]}
    assert len(keys) == meta["count"] == len(meta["tiles"])
    assert meta["used_pairs"] == sum(len(m["sources"]) for m in meta["tiles"]) == 1874
    assert atlas.max() < 32


def test_rgba_uses_each_tiles_colour_31(built):
    atlas, rgba, _, meta = built
    for m in meta["tiles"]:
        if m["uses_index_31"]:
            sl = master_tile(atlas, m) == 31
            block = rgba[m["y"]:m["y"] + 32, m["x"]:m["x"] + 16]
            assert (block[sl] == np.array(m["color_31"]["rgba8"])).all()
    c31 = {m["color_31"]["rgb4"] for m in meta["tiles"] if m["uses_index_31"]}
    assert c31 == {"0x0bdf", "0x0980", "0x0445", "0x00f0"}   # fmain2.c:381-386


def test_indoor_islands(built):
    _, _, _, meta = built
    entries = meta["usage"]["indoor_entries"]
    assert len(entries) == 87                      # DOORCOUNT 86 (fmain.c:231) + necromancer xfer
    # crystal palace doors load region 8 (secs=1, fmain.c:1926); the stargate lands in region 8 by formula
    assert {e["region"] for e in entries if "crystal palace" in e["via"]} == {8}
    assert {e["region"] for e in entries if "stargate" in e["via"]} == {8}
    assert meta["regions"]["9"]["islands_shown_in_region"] == 3
    assert meta["region_maps"]["9"][241] is None and meta["region_maps"]["8"][241] is not None


def test_secret_variants(built):
    atlas, rgba, _, meta = built
    variants = meta["index_31"]["secret_variants"]
    # only the two cave-group tiles are ever drawn with region 9's palette while containing index 31
    assert [v["sources"] for v in variants] == [[{"region": 9, "tile": 114}], [{"region": 9, "tile": 115}]]
    for v in variants:
        orig, var = meta["tiles"][v["tile"]], meta["tiles"][v["variant"]]
        assert orig["secret_variant"] == var["index"] and var["variant_of"] == orig["index"]
        assert np.array_equal(master_tile(atlas, orig), master_tile(atlas, var))   # identical indices
        hit = master_tile(atlas, var) == 31
        assert hit.any()
        assert (rgba[var["y"]:var["y"] + 32, var["x"]:var["x"] + 16][hit] == GREEN).all()
        assert (rgba[orig["y"]:orig["y"] + 32, orig["x"]:orig["x"] + 16][hit] == GREY).all()
        assert var["sources"] == [] and all(m != var["index"] for r in meta["region_maps"].values() for m in r)


def test_shipped_files_match_build(built):
    atlas, rgba, shadow, meta = built
    assert np.array_equal(np.array(Image.open(MASTER / "atlas_shadowmask.png").convert("L")) > 0, shadow > 0)
    idx = Image.open(MASTER / "atlas_indexed.png")
    assert idx.mode == "P" and len(idx.getpalette()) // 3 == 32 and "transparency" not in idx.info
    assert np.array_equal(np.array(idx), atlas)
    assert np.array_equal(np.array(Image.open(MASTER / "atlas_rgba.png")), rgba)
    assert json.loads((MASTER / "master.json").read_text())["count"] == meta["count"]
    assert sorted(p.name for p in MASTER.iterdir()) == \
        ["atlas_highlightmask.png", "atlas_indexed.png", "atlas_rgba.png", "atlas_shadowmask.png", "master.json"]
