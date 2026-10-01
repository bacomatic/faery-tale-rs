"""Self-check aid for tools/extract_maps.py + decode_map_layers.py (T2.5). Not an acceptance gate."""

import json
import sys
from pathlib import Path

import numpy as np
import pytest

REPO = Path(__file__).resolve().parents[2]
TOOLS = REPO / "tools"
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

import decode_map_layers as dl  # noqa: E402
import extract_maps as em  # noqa: E402

SRC, GAME, ASSETS = REPO / "src", REPO / "src" / "assets", REPO / "assets"


@pytest.fixture(scope="module")
def world():
    return em.World(GAME, SRC)


@pytest.fixture(scope="module")
def spaces(world):
    return em.finish_spaces(em.segment(world))


@pytest.fixture(scope="module")
def bundle(world, spaces, tmp_path_factory):
    out = tmp_path_factory.mktemp("maps")
    index = em.build(world, spaces, em.load_master(ASSETS / "tiles" / "master"),
                     em.region_atlases(ASSETS / "tiles"), out)
    return out, index


def test_doors_and_entries(world):
    assert len(world.doors) == 86                                   # DOORCOUNT, fmain.c:231
    assert sum("fix" in d for d in world.doors) == 3                # P27 corrections
    assert world.doors[0] == {"line": 240, "name": "desert fort", "type": "HWOOD", "type_value": 1,
                              "xc1": 0x1170, "yc1": 0x5060, "xc2": 0x2870, "yc2": 0x8b60, "secs": 1}
    kinds = [e["kind"] for e in world.entries]
    assert kinds.count("door") == 84 and kinds.count("stargate") == 2 and kinds.count("quicksand") == 1
    cave = next(e for e in world.entries if e["name"] == "dragon cave")        # CAVE: xc2+24, yc2+16
    assert (cave["x"], cave["y"], cave["region"]) == (0x1980 + 24, 0x8c60 + 16, 9)
    tombs = next(e for e in world.entries if e["name"] == "tombs")             # STAIR (odd): xc2+16, yc2
    assert (tombs["x"], tombs["y"], tombs["region"]) == (0x0470 + 16, 0x8ee0, 9)
    oasis = next(e for e in world.entries if e["name"] == "oasis #1")          # DESERT (odd)
    assert (oasis["x"], oasis["y"], oasis["region"]) == (0x13a0 + 16, 0x95a0, 8)
    fwd = next(e for e in world.entries if e["name"] == "stargate forwards")   # indoor xc1: lands at xc1
    assert (fwd["x"], fwd["y"], fwd["region"]) == (0x2960 + 16, 0x8760 + 34, 8)
    qs = next(e for e in world.entries if e["kind"] == "quicksand")
    assert (qs["x"], qs["y"], qs["region"]) == (0x1080, 34950, 9)              # fmain.c:1787-1788
    assert world.quicksand_px == [24064, 9728]                                  # sector 181: region 3 (30, 6)


def test_overworld_is_eight_regions(world):
    assert world.overworld.shape == (1024, 2048)
    for r in range(8):
        y0, x0 = (r // 2) * 256, (r % 2) * 1024
        assert np.array_equal(world.overworld[y0:y0 + 256, x0:x0 + 1024], np.array(world.regions[r]["tiles"]))


def test_walkability_rules(world):
    w9 = world.walkable(9)
    terra = world.regions[9]["terra"]
    # tile 114 (SECRET door, type 15) is walkable; tile 7 (type 10, lower half) blocks exactly its masked sub-tiles
    for ty in range(256):
        for tx in range(1024):
            t = world.interior[ty, tx]
            if t in (114, 7):
                cell = w9[ty * 4:ty * 4 + 4, tx * 2:tx * 2 + 2]
                exp = np.array([[not (terra[t]["feature_type"] in em.BLOCKING_TYPES and terra[t]["subtile_mask"] & (0x80 >> (4 * c + r)))
                                 for c in range(2)] for r in range(4)])
                assert np.array_equal(cell, exp)
                break


def test_spaces(spaces):
    by_name = {sp["name"]: sp for sp in spaces}
    assert len(spaces) == 68
    kinds = [sp["kind"] for sp in spaces]
    assert kinds.count("interior") == 62 and kinds.count("dungeon") == 5 and kinds.count("astral") == 1
    assert {sp["name"] for sp in spaces if sp["kind"] == "dungeon"} == \
        {"tombs", "dragon cave", "troll cave", "maze caves", "spider pit"}
    assert sorted(e["name"] for e in by_name["spider pit"]["entries"]) == ["quicksand drop", "spider exit"]
    assert sorted(e["name"] for e in by_name["doom tower"]["entries"]) == ["doom tower", "stargate backwards"]
    assert [e["name"] for e in by_name["astral plane"]["entries"]] == ["stargate forwards"]
    assert by_name["astral plane"]["region"] == 8 and by_name["astral plane"]["bbox"] == (575, 15, 784, 80)
    assert len(by_name["desert fort"]["entries"]) == 4                 # four identical doorlist rows
    # P27: yard gates #4/#5/#9 are corrected to their own cabins (source has them in cabins 7/8/6)
    for n in range(1, 11):
        assert {e["name"] for e in by_name[f"cabin {n}"]["entries"]} == {f"cabin #{n}", f"cabin yard #{n}"}, n
    fixed = {e["name"]: e["fix"] for e in by_name["cabin 4"]["entries"] + by_name["cabin 5"]["entries"] if "fix" in e}
    assert fixed["cabin yard #4"]["source_yc2"] == 0x9a40 and fixed["cabin yard #4"]["yc2"] == 0x9c40
    assert fixed["cabin yard #5"]["source_yc2"] == 0x9a40 and fixed["cabin yard #5"]["yc2"] == 0x9840
    assert all(sp["region"] == 8 for sp in spaces if sp["kind"] == "interior")
    # reachable areas never overlap, bounding boxes may
    for a in spaces:
        for b in spaces:
            if a is not b:
                assert not (a["reach"] & b["reach"]).any()


def test_bundle_round_trips(bundle, spaces):
    out, index = bundle
    assert len(index["rows"]) == 69
    for entry in index["rows"]:
        m = dl.load_map(out / entry["dir"])
        w, h = entry["size_tiles"]
        assert m["tiles"].shape == m["master"].shape == (h, w)
        assert m["tiles"].dtype == np.uint8 and m["master"].dtype == np.uint16
        assert not (m["master"] == dl.NO_MASTER).any()
        assert (out / entry["dir"] / "preview.png").is_file()
    # a space's layer equals the interior map crop with foreign tiles blanked
    castle = next(sp for sp in spaces if sp["name"] == "main castle")
    m = dl.load_map(out / "interiors" / "main_castle")
    x0, y0, x1, y1 = castle["crop"]
    world = em.World(GAME, SRC)
    exp = world.interior[y0:y1, x0:x1].copy()
    exp[castle["foreign"]] = 0
    assert np.array_equal(m["tiles"], exp)
    assert m["meta"]["origin_world_px"] == [x0 * 16, 32768 + y0 * 32]
    # overworld region layer + master ids agree with master.json region_maps
    ow = dl.load_map(out / "overworld")
    ref = json.loads((ASSETS / "tiles" / "master" / "master.json").read_text())["region_maps"]
    for row, col in ((984, 1189), (0, 0), (600, 1500), (1023, 2047)):
        r = int(ow["region"][row, col])
        assert r == (row // 256) * 2 + col // 1024
        assert ow["master"][row, col] == ref[str(r)][int(ow["tiles"][row, col])]
