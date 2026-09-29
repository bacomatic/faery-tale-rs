"""Self-check aid for tools/extract_sprites.py (T2.1). Not an acceptance gate."""

import json
from pathlib import Path

import numpy as np
import pytest
from PIL import Image

import extract_sprites as es

REPO = Path(__file__).resolve().parents[2]
SRC = es.Source(REPO / "src")


@pytest.fixture(scope="module")
def bundle(tmp_path_factory):
    out = tmp_path_factory.mktemp("sprites")
    es.main(["--out-dir", str(out)])
    return out


def _modes(atlas):
    return {m["id"]: m for m in atlas["modes"]}


def _origins(atlas, steps):
    return [atlas["frames"][s["frame"]]["origin"]["frame"] for s in steps]


def test_tables_parsed_from_source():
    assert len(SRC.cfiles) == 18
    assert SRC.cfiles[0]["source"] == "src/fmain2.c:644"
    assert SRC.cfiles[12]["file_id"] == SRC.cfiles[0]["file_id"]
    assert [r["name"] for r in SRC.races][:3] == ["Ogre", "Orcs", "Wraith"]
    assert SRC.setfigs[3]["name"] == "guard_back" and SRC.setfigs[3]["source"] == "src/fmain.c:28"
    assert SRC.race_weapons(0) == [1, 2] and SRC.race_weapons(7) == [3]
    assert SRC.race_weapons(9) == [5] and SRC.race_weapons(2) == [0]
    assert SRC.race_weapons(1) == [2, 3, 4]


def test_overlay_rules():
    # walk S step 0 (statelist[0] = {0,11,-2,11}), facing S
    assert es.overlay(SRC, 0, 5, 1, False) == {"frame": 75, "rows": [0, 16], "dx": -2, "dy": 11, "behind": False}
    assert es.overlay(SRC, 0, 5, 4, False) == {"frame": 81, "rows": [0, 16], "dx": 1, "dy": 8, "behind": False}
    assert es.overlay(SRC, 24, 2, 5, False)["dy"] == SRC.statelist[24][3] - 6
    assert [f for f in range(8) if es.overlay(SRC, 0, f, 3, False)["behind"]] == [0, 1, 6, 7]
    assert [f for f in range(8) if es.overlay(SRC, 0, f, 4, False)["behind"]] == [0, 1, 2, 3]


@pytest.mark.parametrize("inum,expected", [
    (0x91, (17, [8, 16])), (8, (8, [0, 8])), (0x1B, (0x1B, [0, 8])), (30, (30, [0, 16])),
])
def test_objects_rows(inum, expected):
    assert es.objects_rows(inum) == expected


def test_resolved_animations(bundle):
    j = json.loads((bundle / "heroes/julian/julian.json").read_text())
    m = _modes(j)
    assert _origins(j, m["walk"]["facings"]["S"]) == list(range(8))
    assert [(j["frames"][s["frame"]]["origin"]["cfile"], s["ticks"]) for s in m["fall"]["steps"]] \
        == [(9, 5)] * 3 + [(3, 5)] * 3
    ogre = json.loads((bundle / "enemies/ogre/ogre.json").read_text())
    assert _origins(ogre, _modes(ogre)["walk"]["facings"]["S"]) == [0, 0, 2, 2, 4, 4, 6, 6]
    snake = json.loads((bundle / "enemies/snake/snake.json").read_text())
    assert _origins(snake, _modes(snake)["walk"]["facings"]["S"]) == [36, 37]
    loraii = json.loads((bundle / "enemies/loraii/loraii.json").read_text())
    assert _origins(loraii, _modes(loraii)["form1"]["steps"]) == [40, 42, 44, 45]


def _load(bundle, rel):
    return json.loads((bundle / rel).read_text())


def test_review_fixes_actors(bundle):
    assert not (bundle / "enemies/salamander").exists()
    assert not (bundle / "npcs/guard_back").exists()

    j = _load(bundle, "heroes/julian/julian.json")
    frus = _modes(j)["frustrated"]["facings"]["S"]
    assert {s["state_index"] for s in frus[:-1]} == {84, 85} and frus[-1]["state_index"] == 40
    assert not any("no setter" in n["text"] for n in j["notes"])
    assert set(_modes(j)["fight"]["weapons"]) == {0, 1, 2, 3}

    necro = _load(bundle, "enemies/necromancer/necromancer.json")
    nm = _modes(necro)
    assert "fight" not in nm and nm["cast_wand"]["weapons"] == [5]
    assert _origins(necro, nm["dying"]["facings"]["S"]) == [47, 63]

    wood = _load(bundle, "enemies/woodcutter/woodcutter.json")
    assert "fight" not in _modes(wood)
    assert {f["origin"]["frame"] for f in wood["frames"]} <= set(range(0, 31, 2)) | {38, 46, 62}

    orcs = _load(bundle, "enemies/orcs/orcs.json")
    assert _modes(orcs)["fight"]["weapons"] == [2, 3]
    wraith = _load(bundle, "enemies/wraith/wraith.json")
    assert _modes(wraith)["fight"]["weapons"] == [0]

    guard = _load(bundle, "npcs/guard/guard.json")
    gm = _modes(guard)
    assert list(gm) == ["still_front", "still_back"]
    assert [_origins(guard, gm[k]["steps"]) for k in gm] == [[0], [1]]
    for name in ("princess", "king", "noble", "sorceress"):
        assert not {"dying", "dead"} & set(_modes(_load(bundle, f"npcs/{name}/{name}.json")))

    bart = _load(bundle, "npcs/bartender/bartender.json")
    bm = _modes(bart)
    assert bart["frame_count"] == 8
    assert _origins(bart, bm["dying"]["steps"]) == [6] and _origins(bart, bm["dead"]["steps"]) == [7]

    bird = _load(bundle, "carriers/bird/bird.json")
    g = bird["frames"][_modes(bird)["grounded"]["steps"][0]["frame"]]["origin"]
    assert (g["cfile"], g["frame"]) == (4, 1)
    raft = _load(bundle, "carriers/raft/raft.json")
    assert raft["frame_count"] == 1 and list(_modes(raft)) == ["still"]


def test_objects_items_split(bundle):
    o = _load(bundle, "objects/objects.json")
    items = {i["name"]: i for i in o["items"]}
    assert items["Sword"]["rows"] == [0, 8] and items["Herb"]["rows"] == [8, 16]
    assert items["Sword"]["frame"] == items["Herb"]["frame"] == 8
    assert items["Jade Skull"]["rows"] == [0, 16] and items["Blue Key"]["rows"] == [0, 8]
    sheet = np.array(Image.open(bundle / "objects/objects_sheet.png"))
    for it in o["items"]:
        png = np.array(Image.open(bundle / "objects" / it["file"]))
        r0, r1 = it["rows"]
        f = it["frame"]
        x, y = (f % 16) * 16, (f // 16) * 16
        assert np.array_equal(png, sheet[y + r0:y + r1, x:x + 16])
    assert len(list((bundle / "objects/items").glob("*.png"))) == len(o["items"])


def _overlay_frames(bundle, weapon):
    used = set()
    for p in bundle.glob("*/*/*.json"):
        for m in json.loads(p.read_text()).get("modes", []):
            for seq in (m["facings"].values() if m["directional"] else [m["steps"]]):
                for st in seq:
                    o = (st.get("overlays") or {}).get(str(weapon))
                    if o:
                        used.add(o["frame"])
    return used


def _check_set(bundle, rel, expect_frames):
    s = _load(bundle, rel)
    objs = np.array(Image.open(bundle / "objects/objects_sheet.png"))
    sheet = np.array(Image.open(bundle / Path(rel).parent / s["sheet"]["file"]))
    assert [f["origin"]["frame"] for f in s["frames"]] == expect_frames
    for f in s["frames"]:
        x, y, w, h = f["rect"]
        o = f["origin"]["frame"]
        src = objs[(o // 16) * 16:(o // 16) * 16 + 16, (o % 16) * 16:(o % 16) * 16 + 16].copy()
        r0, r1 = f["rows"]
        src[:r0] = 31
        src[r1:] = 31
        assert np.array_equal(sheet[y:y + h, x:x + w], src)
    return s


def test_weapon_sheets(bundle):
    for wid, name in [(1, "dirk"), (2, "mace"), (3, "sword"), (5, "wand")]:
        s = _check_set(bundle, f"objects/weapons/{name}/{name}.json", sorted(_overlay_frames(bundle, wid)))
        assert all(f["used_by"] for f in s["frames"])
    _check_set(bundle, "objects/weapons/bow/bow.json", [30] + list(range(80, 88)))


def test_effect_sheets(bundle):
    _check_set(bundle, "objects/effects/arrow/arrow.json", list(range(8)))
    _check_set(bundle, "objects/effects/fireball/fireball.json", list(range(0x59, 0x61)) + [0x58])
    b = _check_set(bundle, "objects/effects/bubbles/bubbles.json", [97, 98])
    assert all(f["rows"] == [0, 8] for f in b["frames"])


def test_turtle_eggs_item(bundle):
    o = _load(bundle, "objects/objects.json")
    it = next(i for i in o["items"] if 102 in i.get("ob_id", []))
    assert it["name"] == "Turtle eggs" and it["file"] == "items/turtle_eggs.png"
    assert not (bundle / "objects/items/turtle.png").exists()


def test_raw_sheets(bundle):
    raw = _load(bundle, "raw/raw.json")
    assert [s["cfile"] for s in raw["sheets"]] == [i for i in range(18) if i != 12]
    assert len(list((bundle / "raw").glob("*.png"))) == 17
    image = (REPO / "src/assets/image").read_bytes()
    for s in raw["sheets"]:
        frames = es.decode_frames(image, SRC.cfiles[s["cfile"]])
        sheet = np.array(Image.open(bundle / "raw" / s["file"]))
        w, h = s["frame_size"]
        assert s["frame_count"] == len(frames) and (w, h) == (frames.shape[2], frames.shape[1])
        for i, fr in enumerate(frames):
            x, y = (i % s["columns"]) * w, (i // s["columns"]) * h
            assert np.array_equal(sheet[y:y + h, x:x + w], fr)


def test_every_set_is_consistent(bundle):
    atlases = list(bundle.glob("*/*/*.json"))
    assert len(atlases) == 30
    for p in atlases:
        a = json.loads(p.read_text())
        sheet = np.array(Image.open(p.parent / a["sheet"]["file"]))
        w, h = a["frame_size"]
        assert len(a["frames"]) == a["frame_count"] == len(list(p.parent.glob("frame_*.png")))
        for f in a["frames"]:
            x, y, fw, fh = f["rect"]
            assert (fw, fh) == (w, h) and x + fw <= sheet.shape[1] and y + fh <= sheet.shape[0]
        for m in a["modes"]:
            seqs = m["facings"].values() if m["directional"] else [m["steps"]]
            for steps in seqs:
                assert steps and all(0 <= s["frame"] < a["frame_count"] for s in steps)
        hl = np.array(Image.open(p.parent / a["sheet"]["highlight_mask"]).convert("L")) > 0
        sil = np.array(Image.open(p.parent / a["sheet"]["silhouette_mask"]).convert("L")) > 0
        assert np.array_equal(hl, (sheet >= 16) & (sheet <= 24))
        assert np.array_equal(sil, sheet != 31)
        assert Image.open(p.parent / a["frames"][0]["file"]).mode == "P"
