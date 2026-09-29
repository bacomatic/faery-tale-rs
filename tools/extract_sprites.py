#!/usr/bin/env python3
"""Sprite extractor for The Faery Tale Adventure (T2.1).

The game packs several actors into one ``cfiles[]`` sheet (two enemy races on
alternating frames, NPCs sharing a sheet, hero fall frames borrowed from an
enemy sheet). This script decodes those sheets from the ADF image and writes
one **repacked sprite set per actor** into ``assets/sprites/<group>/<actor>/``:

* ``frame_NNN.png`` -- indexed PNGs (palette ``assets/palettes/pagecolors.json``,
  index 31 transparent),
* ``<actor>_sheet.png`` -- the actor's frames, renumbered 0..N-1,
* ``<actor>_highlightmask.png`` -- 1-bit, set where the index is 16..24,
* ``<actor>_silhouettemask.png`` -- 1-bit, set where the index is not 31
  (the game's ``make_mask``: NOT of the AND of all 5 planes, ``fsubs.asm:1619-1653``),
* ``<actor>.json`` -- frame rects with each frame's origin (cfile + frame), and
  **resolved** animation modes: per facing, the frame sequence and, for each
  weapon the actor can hold, the weapon-overlay frame, rows, offset and draw order.

The OBJECTS sheet (items, weapon overlays, effects) is written whole to
``assets/sprites/objects/``; actor overlays point into it.

Inputs are the original sources only: ``cfiles`` (``fmain2.c:643-665``),
``statelist``/``diroffs``/``trans_list``/``setfig_table``/``encounter_chart``
(``fmain.c``), ``bow_x``/``bow_y``/``fallstates``/``weapon_probs`` (``fmain2.c``).
The render rules that turn those tables into frames are applied below, each
with its source line. ``cfiles[12]`` is never loaded (file_id 1376 is Julian's
block, ``fmain2.c:644,658``; encounter file_ids are 6-9, ``fmain.c:53-63``). Race 5
(Salamander) is never spawned: ``encounter_type`` is only ever 0-3, 4, 6, 2, 8 or an
extent ``v3`` (``fmain.c:2086-2090, 2696, 2704``; extent v3 values ``fmain.c:339-369``), and
mixing skips type 4 (``fmain.c:2752-2754``), so it gets no set.

Usage::

    python tools/extract_sprites.py               # everything -> assets/sprites
    python tools/extract_sprites.py --actor ogre
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

import numpy as np

TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import asset_common as ac  # noqa: E402
import extract_table as et  # noqa: E402
import extract_tables as xt  # noqa: E402

REPO_ROOT = TOOLS_DIR.parent
BLOCK_SIZE = 512
NUM_PLANES = 5
SHEET_COLS = 16
CFILE_FIELDS = ["width", "height", "count", "numblocks", "seq_num", "file_id"]

# cfiles[] indices, named after the entry comments at fmain2.c:644-664.
SHEET_NAMES = {
    0: "julian", 1: "phillip", 2: "kevin", 3: "objects", 4: "raft", 5: "turtle",
    6: "ogre file", 7: "ghost file", 8: "dknight file", 9: "necromancer file",
    10: "dragon", 11: "bird", 13: "wizard/priest", 14: "royal set", 15: "bartender",
    16: "witch", 17: "ranger/begger",
}
OBJECTS, RAFT, TURTLE, DRAGON, BIRD = 3, 4, 5, 10, 11
HEROES = [(0, "julian", 1), (1, "phillip", 2), (2, "kevin", 3)]  # cfile, name, brother (fmain2.c:676)

DIRS = ["NW", "N", "NE", "E", "SE", "S", "SW", "W"]  # xdir/ydir, fsubs.asm:1276-1277
WEAPONS = {0: "none", 1: "dirk", 2: "mace", 3: "sword", 4: "bow", 5: "wand"}
SALAMANDER, LORAII, NECROMANCER, WOODCUTTER = 5, 8, 9, 10
PAX_SETFIGS = range(2, 8)  # guards, princess, king, noble, sorceress
HAND_K = {1: 64, 2: 32, 3: 48, 4: 0}  # fmain.c:2440-2443
OBJECTS_SHEET = "sprites/objects/objects_sheet.png"


# --------------------------------------------------------------------------- #
# Source parsing
# --------------------------------------------------------------------------- #
def _lines(path: Path) -> list[str]:
    return path.read_text(errors="replace").splitlines()


def _brace_row_lines(lines: list[str], start: int, count: int) -> list[int]:
    """1-based lines of the first *count* ``{...}`` rows after line *start*."""
    out = []
    for n in range(start + 1, len(lines) + 1):
        if re.match(r"\s*\{[^{}]*\}", lines[n - 1]):
            out.append(n)
            if len(out) == count:
                return out
    raise SystemExit(f"found {len(out)} of {count} rows after line {start}")


def load_table(src_dir: Path, fname: str, name: str, symbols: dict | None = None):
    table = et.extract_tables(str(src_dir / fname))[name]
    return xt.resolve(table["values"], {**xt.BASE_SYMBOLS, **(symbols or {})}), table["line"]


def load_define(src_dir: Path, fname: str, name: str) -> tuple[int, int]:
    for n, line in enumerate(_lines(src_dir / fname), 1):
        m = re.match(rf"\s*#define\s+{name}\s+(\S+)", line)
        if m:
            return et.parse_c_value(m.group(1)), n
    raise SystemExit(f"#define {name} not found in {fname}")


def load_palette(path: Path) -> list[tuple[int, int, int, int]]:
    entries = json.loads(path.read_text())
    return [tuple(e["rgba8"]) for e in sorted(entries, key=lambda e: e["index"])]


class Source:
    """Every table the sprite bindings need, parsed from src/."""

    def __init__(self, src_dir: Path):
        self.dir = src_dir
        fmain = _lines(src_dir / "fmain.c")

        seq = xt.parse_enum((src_dir / "ftale.h").read_text(errors="replace"), "sequences")
        seq_names = {v: k for k, v in seq.items()}
        rows, start = load_table(src_dir, "fmain2.c", "cfiles", seq)
        lines = _brace_row_lines(_lines(src_dir / "fmain2.c"), start, len(rows))
        self.cfiles = []
        for idx, (row, line) in enumerate(zip(rows, lines)):
            cf = dict(zip(CFILE_FIELDS, row))
            cf.update(index=idx, seq_name=seq_names[cf["seq_num"]], source=f"src/fmain2.c:{line}")
            self.cfiles.append(cf)

        self.statelist, _ = load_table(src_dir, "fmain.c", "statelist")
        self.diroffs, _ = load_table(src_dir, "fmain.c", "diroffs")
        self.trans, _ = load_table(src_dir, "fmain.c", "trans_list")
        self.bow_x, _ = load_table(src_dir, "fmain2.c", "bow_x")
        self.bow_y, _ = load_table(src_dir, "fmain2.c", "bow_y")
        self.fall, _ = load_table(src_dir, "fmain2.c", "fallstates")
        self.weapon_probs, _ = load_table(src_dir, "fmain2.c", "weapon_probs")

        # encounter_chart rows + their "/* N - Name */" comments (fmain.c:53-63)
        enc, enc_start = load_table(src_dir, "fmain.c", "encounter_chart")
        enc_lines = _brace_row_lines(fmain, enc_start, len(enc))
        self.races = []
        for r, (row, line) in enumerate(zip(enc, enc_lines)):
            name = re.search(rf"/\*\s*{r}\s*-\s*(\w+)", fmain[line - 1]).group(1)
            self.races.append({"race": r, "name": name, "arms": row[2], "file_id": row[5],
                               "source": f"src/fmain.c:{line}"})

        # setfig_table: flat triples, one per line, named by the '"name" = N' comment
        flat, sf_start = load_table(src_dir, "fmain.c", "setfig_table")
        self.setfigs = []
        n = sf_start
        for i in range(len(flat) // 3):
            while not re.search(rf'"\w+"\s*=\s*{i}\b', fmain[n - 1]):
                n += 1
            m = re.search(rf'"(\w+)"\s*=\s*{i}\b\s*(\((\w+)\))?', fmain[n - 1])
            name = m.group(1) + (f"_{m.group(3)}" if m.group(3) else "")
            self.setfigs.append({"id": i, "name": name, "cfile": flat[3 * i],
                                 "image_base": flat[3 * i + 1], "can_talk": flat[3 * i + 2],
                                 "source": f"src/fmain.c:{n}"})

    def race_weapons(self, race: int) -> list[int]:
        """Weapons a spawned race can hold: weapon_probs[arms*4 + 0..3] (fmain.c:2757-2758).

        0 and 8 draw no overlay (weapon > 0 && weapon < 8, fmain.c:2400) and are reported as 0."""
        arms = self.races[race]["arms"]
        return sorted({w if 1 <= w <= 5 else 0 for w in self.weapon_probs[arms * 4:arms * 4 + 4]})


# --------------------------------------------------------------------------- #
# Pixel decoding
# --------------------------------------------------------------------------- #
def decode_frames(image: bytes, cf: dict) -> np.ndarray:
    """Return a (count, height, width_px) uint8 array of palette indices."""
    ww, h, count = cf["width"], cf["height"], cf["count"]
    total = count * NUM_PLANES * h * ww * 2
    raw = np.frombuffer(image, dtype=np.uint8, count=total, offset=cf["file_id"] * BLOCK_SIZE)
    bits = np.unpackbits(raw.reshape(count, NUM_PLANES, h, ww * 2), axis=3)
    idx = np.zeros((count, h, ww * 16), dtype=np.uint8)
    for plane in range(NUM_PLANES):
        idx |= bits[:, plane] << plane
    return idx


def pack_sheet(frames: list[np.ndarray], cell: tuple[int, int]) -> np.ndarray:
    w, h = cell
    cols = min(SHEET_COLS, len(frames))
    rows = (len(frames) + cols - 1) // cols
    sheet = np.full((rows * h, cols * w), ac.TRANSPARENT_INDEX, dtype=np.uint8)
    for i, fr in enumerate(frames):
        r, c = divmod(i, cols)
        sheet[r * h:r * h + fr.shape[0], c * w:c * w + fr.shape[1]] = fr
    return sheet


# --------------------------------------------------------------------------- #
# Render rules
# --------------------------------------------------------------------------- #
def objects_rows(inum: int) -> tuple[int, list[int]]:
    """OBJECTS inum -> (frame, [first_row, end_row)) per fmain.c:2477-2479, 2524."""
    if inum & 0x80:
        return inum & 0x7F, [8, 16]
    if inum == 0x1B or 8 <= inum <= 12 or inum in (25, 26) or 0x10 < inum < 0x18:
        return inum, [0, 8]
    return inum, [0, 16]


def overlay(src: Source, state_idx: int, facing: int, weapon: int, shooting: bool) -> dict:
    """Weapon pass for one frame (fmain.c:2400-2447)."""
    if weapon == 4 and state_idx < 32:                       # bow at the side, 2422-2434
        q = state_idx // 8
        obj = 30 if q & 1 else (0x53 if q & 2 else 0x51)
        dx, dy = src.bow_x[state_idx], src.bow_y[state_idx]
    else:
        _, wpn_no, dx, dy = src.statelist[state_idx]         # 2425-2426
        if weapon == 5:                                      # 2435-2437
            obj = 103 + facing
            if facing == 2:
                dy -= 6
        else:                                                # 2439-2444
            obj = wpn_no + HAND_K[weapon]
    frame, rows = objects_rows(obj)
    behind = bool((facing - 2) & 4)                          # 2402-2403
    if weapon == 4 and not shooting:                         # 2404-2406
        behind = (facing & 4) == 0
    return {"frame": frame, "rows": rows, "dx": dx, "dy": dy, "behind": behind}


def parity(figure: int, race: int | None) -> int:
    """Non-hero ENEMY frames forced odd/even by race (fmain.c:2460)."""
    if race is None:
        return figure
    return figure | 1 if race & 1 else figure & ~1


# --------------------------------------------------------------------------- #
# Actor model
# --------------------------------------------------------------------------- #
class Actor:
    def __init__(self, src: Source, name: str, group: str, cfile: int, title: str):
        self.src, self.name, self.group, self.cfile, self.title = src, name, group, cfile, title
        cf = src.cfiles[cfile]
        self.cell = (cf["width"] * 16, cf["height"])
        self.weapons: list[int] = []
        self.modes: list[dict] = []
        self.notes: list[dict] = []
        self.info: dict = {}
        self._refs: set[tuple[int, int]] = set()

    def ref(self, cfile: int, frame: int) -> tuple[int, int] | None:
        if not 0 <= frame < self.src.cfiles[cfile]["count"]:
            return None
        self._refs.add((cfile, frame))
        return (cfile, frame)

    def note(self, text: str, source: str) -> None:
        self.notes.append({"text": text, "source": source})

    def step(self, frame: int, cfile: int | None = None, ticks: int | None = None, **extra) -> dict:
        key = self.ref(self.cfile if cfile is None else cfile, frame)
        if key is None:
            raise IndexError(frame)
        st = {"frame": key}
        if ticks:
            st["ticks"] = ticks
        st.update(extra)
        return st

    def mode(self, mid: str, label: str, source: str, playback: str = "loop",
             facings: dict | None = None, steps: list | None = None, **extra) -> None:
        m = {"id": mid, "label": label, "source": source, "playback": playback,
             "directional": facings is not None}
        m.update(extra)
        if facings is not None:
            m["facings"] = facings
        else:
            m["steps"] = steps
        self.modes.append(m)

    def ordered_refs(self) -> list[tuple[int, int]]:
        return sorted(self._refs, key=lambda k: (k[0] != self.cfile, k))


# --------------------------------------------------------------------------- #
# Humanoids (PHIL and ENEMY sheets)
# --------------------------------------------------------------------------- #
def hstep(a: Actor, state_idx: int, facing: int, race: int | None, weapons: list[int],
          shooting: bool = False, ticks: int | None = None, overlays: bool = True,
          figure: int | None = None, cfile: int | None = None) -> dict:
    fig = a.src.statelist[state_idx][0] if figure is None else figure
    st = a.step(parity(fig, race), cfile=cfile, ticks=ticks, state_index=state_idx)
    if overlays:
        ov = {str(w): overlay(a.src, state_idx, facing, w, shooting) for w in weapons if w}
        if ov:
            st["overlays"] = ov
    return st


def dying_order(f: int) -> tuple[int, int]:
    """First/second DYING index by facing (fmain.c:1719-1724)."""
    return (80, 81) if f == 0 or f > 4 else (81, 80)


def humanoid_modes(a: Actor, race: int | None) -> None:
    src, W = a.src, a.weapons
    walk = lambda f: src.diroffs[f]
    fight = lambda f: src.diroffs[f + 8]
    # bow/wand holders never enter FIGHTING: hero fmain.c:1431-1436, enemies fmain.c:2164-2166
    melee = [w for w in W if not w & 4]
    per = lambda fn: {DIRS[f]: fn(f) for f in range(8)}

    a.mode("still", "Still", "src/fmain.c:1661-1663", weapons=W,
           facings=per(lambda f: [hstep(a, walk(f) + 1, f, race, W)]))

    n_steps = 1 if race == 2 else 8  # wraith: step never advances, fmain.c:1632
    a.mode("walk", "Walk", "src/fmain.c:1629-1632", weapons=W,
           facings=per(lambda f: [hstep(a, walk(f) + k, f, race, W) for k in range(n_steps)]))

    remap = {7: 8} if race is None else {6: 8, 7: 8}  # fmain.c:1713 (enemies sit in slots > 2)
    if melee and race != WOODCUTTER:
        a.mode("fight", "Fight (melee)", "src/fmain.c:1710-1714", playback="transitions",
               transitions=src.trans, weapons=melee,
               note="Steps are indexed by fight state 0-8; next state = transitions[state][rand4()]. "
                    "State 7 (and 6 for enemies) is drawn as state 8.",
               facings=per(lambda f: [hstep(a, fight(f) + remap.get(s, s), f, race, melee)
                                      for s in range(9)]))
    if 4 in W:
        aim = 10 if race is None else 11
        a.mode("shoot_bow", "Shoot (bow)", "src/fmain.c:1670-1684", weapons=[4],
               note="Aim (SHOOT1) is held while firing; release (SHOOT3) lasts one tick, then STILL.",
               facings=per(lambda f: [hstep(a, fight(f) + aim, f, race, [4], shooting=True),
                                      hstep(a, fight(f) + 11, f, race, [4], shooting=True)]))
    if 5 in W:
        a.mode("cast_wand", "Cast (wand)", "src/fmain.c:1671-1673", weapons=[5],
               facings=per(lambda f: [hstep(a, fight(f), f, race, [5], shooting=True)]))

    if race == 7:  # DKnight: vitality 0 forces index 1, fmain.c:1819-1822, 2773
        a.mode("dying", "Dying / dead", "src/fmain.c:1819-1822", playback="once", weapons=W,
               facings=per(lambda f: [hstep(a, 1, f, race, W, ticks=7),
                                      hstep(a, 1, f, race, W, overlays=False)]))
    elif race == NECROMANCER:
        def nsteps(f):
            first, second = dying_order(f)
            return [hstep(a, first, f, race, W, ticks=3), hstep(a, second, f, race, W, ticks=4)]
        a.mode("dying", "Dying", "src/fmain.c:1747-1755", playback="once", weapons=W,
               note="When tactic reaches 0 the Necromancer becomes race 10 (Woodcutter), STILL, weapon 0, "
                    "so DEAD (statelist 82) is never drawn; continue with the woodcutter set.",
               facings=per(nsteps))
    else:
        def dsteps(f):
            first, second = dying_order(f)
            return [hstep(a, first, f, race, W, ticks=3), hstep(a, second, f, race, W, ticks=4),
                    hstep(a, 82, f, race, W, overlays=False)]
        a.mode("dying", "Dying / dead", "src/fmain.c:1719-1727", playback="once", weapons=W,
               note="tactic starts at 7 (fmain.c:2773): first frame for tactic 7-5, second for 4-1, "
                    "then DEAD (no weapon drawn, fmain.c:2400-2401).",
               facings=per(dsteps))


def build_hero(src: Source, cfile: int, name: str, brother: int) -> Actor:
    a = Actor(src, name, "heroes", cfile, f"{name.title()} (brother {brother})")
    a.weapons = sorted(WEAPONS)
    a.info = {"brother": brother, "brother_source": "src/fmain2.c:676"}
    humanoid_modes(a, None)
    W = a.weapons
    shake = lambda f: [hstep(a, 84 + k, f, None, W, ticks=2) for _ in range(5) for k in (0, 1)]
    a.mode("frustrated", "Frustrated (blocked)", "src/fmain.c:1654-1658", playback="once", weapons=W,
           note="Hero blocked in every direction: the frame is held for frustflag 1-20 "
                "(dex = an->index, fmain.c:1477), then faces the player shaking his head "
                "(statelist 84/85 by (cycle >> 1) & 1) for 21-40, then shows statelist 40.",
           facings={DIRS[f]: shake(f) + [hstep(a, 40, f, None, W)] for f in range(8)})
    a.mode("sink", "Sinking", "src/fmain.c:1575-1576", steps=[hstep(a, 83, 0, None, [])])
    a.mode("sleep", "Asleep", "src/fmain.c:1731", steps=[hstep(a, 86, 0, None, [])])

    # Fall (xtype 52 only, fmain.c:1766-1771): step = tactic/5, tactic++ before the draw, and the
    # draw uses ENEMY while tactic < 16 else OBJECTS (fmain.c:1732-1736, 2456-2457). ENEMY here is
    # encounter_chart[8].file_id, loaded at xtype 52 (fmain.c:2695-2697, 2724-2728).
    enemy_cfile = src.races[8]["file_id"]
    runs: list[list] = []  # [cfile, frame, ticks]
    for t in range(30):
        key = [enemy_cfile if t + 1 < 16 else OBJECTS, src.fall[brother * 6 + t // 5]]
        if runs and runs[-1][:2] == key:
            runs[-1][2] += 1
        else:
            runs.append(key + [1])
    a.mode("fall", "Fall (pit)", "src/fmain.c:1732-1736", playback="once",
           note="Frames come from the Loraii/necromancer ENEMY sheet, then the OBJECTS sheet.",
           steps=[a.step(fr, cfile=cf, ticks=n) for cf, fr, n in runs])
    a.note("fallstates row for this brother", f"src/fmain2.c:{872 + brother}")
    a.note("statelist 84-85 are the head-shake frames, reached through frustflag (see Frustrated)",
           "src/fmain.c:1656-1658")
    a.note("with the bow, DYING draws OBJECTS frame 0 (an arrow): states 80-81 are not < 32, so "
           "the hand-weapon rule applies with k = 0 and statelist wpn_no 0. Kept as the source "
           "draws it; whether the bow was intended is unknown (reference/PROBLEMS.md P25)",
           "src/fmain.c:2422-2444")
    return a


def build_enemy(src: Source, race: int) -> Actor:
    r = src.races[race]
    a = Actor(src, r["name"].lower(), "enemies", r["file_id"], f"{r['name']} (race {race})")
    a.weapons = src.race_weapons(race)
    a.info = {"race": race, "race_source": r["source"], "weapons_source": "src/fmain.c:2757-2758"}
    if race == 4:
        build_snake(a)
    elif race == 8:
        build_loraii(a)
    else:
        humanoid_modes(a, race)
        a.note(f"{'odd' if race & 1 else 'even'} race: every body frame is forced "
               f"{'odd' if race & 1 else 'even'}", "src/fmain.c:2460")
    if race == 2:
        a.note("walk step never advances", "src/fmain.c:1632")
    if race == NECROMANCER:
        a.note("wand only (weapon_probs row 5), so no melee fight", "src/fmain.c:2164-2166")
    if race == WOODCUTTER:
        a.note("what the Necromancer becomes (weapon 0); a hostile actor with weapon < 1 turns "
               "CONFUSED / RANDOM, so no fight mode", "src/fmain.c:2151-2152")
    if 0 in a.weapons and race not in (LORAII, WOODCUTTER):
        a.note("weapon 0 or 8 draws no overlay", "src/fmain.c:2400")
    return a


def build_snake(a: Actor) -> None:
    src = a.src
    fig = lambda i: src.statelist[i][0] + 0x24                # fmain.c:2459
    per = lambda fn: {DIRS[f]: fn(f) for f in range(8)}
    two = lambda f, t: [a.step(fig(src.diroffs[f] + k), ticks=t, state_index=src.diroffs[f] + k)
                        for k in (0, 1)]
    a.mode("still", "Still", "src/fmain.c:1804-1805", facings=per(lambda f: two(f, 2)))
    a.mode("walk", "Walk", "src/fmain.c:1804-1805", facings=per(lambda f: two(f, 2)))
    a.mode("fight", "Fight", "src/fmain.c:1804", facings=per(lambda f: two(f, 1)))

    def dsteps(f):
        first, second = dying_order(f)
        return [hstep(a, first, f, None, [], ticks=3), hstep(a, second, f, None, [], ticks=4),
                hstep(a, 82, f, None, [])]
    a.mode("dying", "Dying / dead", "src/fmain.c:1719-1727", playback="once", facings=per(dsteps))
    a.note("no parity; alive frames are statelist figure + 0x24, dying frames are not offset",
           "src/fmain.c:2459-2460")


def build_loraii(a: Actor) -> None:
    # fmain.c:1806-1817: no statelist, no parity (2450); frame depends on slot % 3 and cycle & 3.
    cyc = [c * 2 - 1 if c * 2 > 4 else c * 2 for c in range(4)]
    for slot, label, base in [(0, "slot % 3 = 0", None), (1, "slot % 3 = 1", 0x28), (2, "slot % 3 = 2", 0x30)]:
        steps = [a.step(0x25)] if base is None else [a.step(base + d) for d in cyc]
        a.mode(f"form{slot}", f"Alive ({label})", "src/fmain.c:1809-1816", steps=steps)
    a.mode("dying", "Dying", "src/fmain.c:1808", steps=[a.step(0x3F)])
    a.note("DEAD: removed from view (abs_x = 0)", "src/fmain.c:1807")


# --------------------------------------------------------------------------- #
# NPCs, carriers
# --------------------------------------------------------------------------- #
PAX_NOTE = ("no dying/dead (review): this NPC stands in the king's or sorceress's pax extent "
            "(fmain.c:354-355) or the princess extent (fmain.c:345), where xtype > 80 turns the hero's attack into SHOOT1 and blocks "
            "shooting (fmain.c:1669)", "src/fmain.c:1412-1416")


def build_guard(src: Source, front: dict, back: dict) -> Actor:
    a = Actor(src, "guard", "npcs", front["cfile"], f"guard (setfigs {front['id']}, {back['id']})")
    a.info = {"setfig": [front["id"], back["id"]],
              "image_base": [front["image_base"], back["image_base"]],
              "can_talk": front["can_talk"], "setfig_source": "src/fmain.c:27-28"}
    a.mode("still_front", "Standing (front)", front["source"], steps=[a.step(front["image_base"])])
    a.mode("still_back", "Standing (back)", back["source"], steps=[a.step(back["image_base"])])
    a.note("one guard in two standing positions (setfig 2 faces the viewer, setfig 3 faces away)",
           "src/fmain.c:27-28")
    a.note(*PAX_NOTE)
    return a


def build_bartender(src: Source, sf: dict) -> Actor:
    a = Actor(src, sf["name"], "npcs", sf["cfile"], f"bartender (setfig {sf['id']})")
    a.info = {"setfig": sf["id"], "image_base": sf["image_base"], "can_talk": sf["can_talk"],
              "setfig_source": sf["source"]}
    a.mode("still", "Still", "src/fmain2.c:1272", steps=[a.step(0)])
    a.mode("dying", "Dying", "src/fmain.c:1551", steps=[a.step(6)])
    a.mode("dead", "Dead", "src/fmain.c:1552", steps=[a.step(7)])
    a.mode("other", "Other frames", "src/fmain2.c:662", steps=[a.step(f) for f in (1, 2, 3, 4, 5)])
    a.note("Dying/Dead are frames 6/7 (intended). The source's image_base + 2 / + 3 = frames 2/3 "
           "is a bug (reference/PROBLEMS.md P25)", "src/fmain.c:1551-1552")
    return a


def build_npc(src: Source, sf: dict) -> Actor:
    a = Actor(src, sf["name"], "npcs", sf["cfile"], f"{sf['name'].replace('_', ' ')} (setfig {sf['id']})")
    a.info = {"setfig": sf["id"], "image_base": sf["image_base"], "can_talk": sf["can_talk"],
              "setfig_source": sf["source"]}
    base, i = sf["image_base"], sf["id"]
    if i == 9:  # witch, fmain.c:1551-1554
        a.mode("still", "Still (turns to hero)", "src/fmain.c:1553-1554",
               facings={DIRS[f]: [a.step(f // 2)] for f in range(8)})
        base += 2
    else:
        a.mode("still", "Still", "src/fmain2.c:1272", steps=[a.step(base)])
    if sf["can_talk"]:
        a.mode("talking", "Talking", "src/fmain.c:1555-1557", playback="random",
               note="image_base + rand2() each tick for 15 ticks (tactic, fmain.c:3375-3377)",
               steps=[a.step(base), a.step(base + 1)])
    if i in (10, 11):
        a.note("never takes damage (race 0x8a / 0x8b), so no dying/dead frames", "src/fmain2.c:235")
        return a
    if i == 4:
        a.note("royal set frame 3 is a second princess pose (image_base + 1, the talking slot). "
               "It is unused in the game: can_talk is 0, and TALKING is set only when can_talk is "
               "non-zero (fmain.c:3375-3377). Not extracted; see assets/sprites/raw/", "src/fmain.c:29")
    if i in PAX_SETFIGS:
        a.note(*PAX_NOTE)
        return a
    for mid, label, off, line in [("dying", "Dying", 2, 1551), ("dead", "Dead", 3, 1552)]:
        try:
            a.mode(mid, label, f"src/fmain.c:{line}", steps=[a.step(base + off)])
        except IndexError:
            a.note(f"{mid} frame {base + off} is beyond the sheet's "
                   f"{src.cfiles[sf['cfile']]['count']} frames (reference/PROBLEMS.md P25)",
                   f"src/fmain.c:{line}")
    return a


def build_carriers(src: Source) -> list[Actor]:
    per = lambda fn: {DIRS[f]: fn(f) for f in range(8)}
    turtle = Actor(src, "turtle", "carriers", TURTLE, "Turtle")
    turtle.mode("swim", "Swim", "src/fmain.c:1539",
                facings=per(lambda f: [turtle.step(2 * f), turtle.step(2 * f + 1)]))
    turtle.mode("ridden_idle", "Ridden, hero not walking", "src/fmain.c:1513-1518",
                facings=per(lambda f: [turtle.step(2 * f)]))
    bird = Actor(src, "bird", "carriers", BIRD, "Swan (bird)")
    bird.mode("fly", "Fly", "src/fmain.c:1497-1508", facings=per(lambda f: [bird.step(f)]))
    bird.mode("grounded", "Landed (not ridden)", "src/fmain.c:2463-2464", steps=[bird.step(1, cfile=RAFT)])
    bird.note("while riding == 0 the swan is drawn from the RAFT sheet, frame 1 (32x32, padded "
              "top-left into the 64x64 cell)", "src/fmain.c:2463-2464")
    raft = Actor(src, "raft", "carriers", RAFT, "Raft")
    raft.mode("still", "Raft", "src/fmain.c:2822", steps=[raft.step(0)])
    dragon = Actor(src, "dragon", "carriers", DRAGON, "Dragon")
    dragon.mode("idle", "Idle", "src/fmain.c:1482", steps=[dragon.step(0)])
    dragon.mode("firing", "Firing", "src/fmain.c:1485-1487", playback="random",
                steps=[dragon.step(1), dragon.step(2)])
    dragon.mode("dying", "Dying", "src/fmain.c:1483", steps=[dragon.step(3)])
    dragon.mode("dead", "Dead", "src/fmain.c:1484", steps=[dragon.step(4)])
    return [turtle, bird, raft, dragon]


def build_actors(src: Source) -> list[Actor]:
    actors = [build_hero(src, cf, name, b) for cf, name, b in HEROES]
    actors += [build_enemy(src, r["race"]) for r in src.races if r["race"] != SALAMANDER]
    for sf in src.setfigs:
        if sf["id"] == 2:
            actors.append(build_guard(src, sf, src.setfigs[3]))
        elif sf["id"] == 8:
            actors.append(build_bartender(src, sf))
        elif sf["id"] != 3:
            actors.append(build_npc(src, sf))
    actors += build_carriers(src)
    return actors


# --------------------------------------------------------------------------- #
# OBJECTS sheet (whole)
# --------------------------------------------------------------------------- #
def objects_tables(src_dir: Path) -> dict:
    inv, inv_start = load_table(src_dir, "fmain.c", "inv_list")
    inv_lines = _brace_row_lines(_lines(src_dir / "fmain.c"), inv_start, len(inv))
    goldbase, gb_line = load_define(src_dir, "fmain.c", "GOLDBASE")
    icons = []
    for j, (row, line) in enumerate(zip(inv, inv_lines)):
        image_number, xoff, yoff, ydelta, img_off, img_height, maxshown, name = row
        entry = {"stuff_index": j, "name": name, "source": f"src/fmain.c:{line}"}
        if j < goldbase:
            entry["icon"] = {"frame": image_number, "rows": [img_off, img_off + img_height]}
            entry["items_page"] = {"x": xoff + 20, "y": yoff, "ydelta": ydelta, "maxshown": maxshown}
        else:
            entry["icon"] = None
            entry["maxshown"] = maxshown
        icons.append(entry)

    fmain2 = (src_dir / "fmain2.c").read_text(errors="replace")
    obytes = xt.parse_enum(fmain2, "obytes")
    enum_line = next(n for n, l in enumerate(fmain2.splitlines(), 1) if "enum obytes" in l)
    itrans, it_line = load_table(src_dir, "fmain2.c", "itrans", obytes)
    by_id: dict[int, dict] = {}
    for name, ob_id in obytes.items():
        by_id.setdefault(ob_id, {})["enum"] = name
    for k in range(0, len(itrans) - 1, 2):
        ob_id, stuff = itrans[k], itrans[k + 1]
        if ob_id == 0 and stuff == 0:
            break
        by_id.setdefault(ob_id, {})["item"] = inv[stuff][7]
    world = []
    for ob_id in sorted(by_id):
        frame, rows = objects_rows(ob_id)
        world.append({"ob_id": ob_id, "enum": by_id[ob_id].get("enum"),
                      "item": by_id[ob_id].get("item"), "frame": frame, "rows": rows})
    return {
        "items": item_assets(icons, world),
        "inventory_icons": {
            "source": f"src/fmain.c:{inv_start}, {gb_line}, 3128-3139",
            "note": "Items page draws rows [rows[0], rows[1]) of the icon frame, 16 px wide, "
                    "for stuff_index < GOLDBASE; entries at or above GOLDBASE have no icon.",
            "items": icons,
        },
        "world_object_ids": {
            "source": f"src/fmain2.c:{enum_line}, {it_line}, 1287; src/fmain.c:2477-2479, 2524",
            "note": "A ground object's frame is its ob_id; bit 7 selects rows 8-15 of frame "
                    "ob_id & 0x7f. Names: enum obytes and itrans -> inv_list.",
            "objects": world,
        },
    }


# enum obytes TURTLE (102) is the turtle eggs: the snake EGG_SEEK tactic keys off it (fmain2.c:1284,
# fmain.c:2150) and it cannot be picked up ("don't take turtle eggs", fmain.c:3170-3171).
OB_NAMES = {102: "Turtle eggs"}


def item_assets(icons: list[dict], world: list[dict]) -> list[dict]:
    """One asset per item: frames whose rows 8-15 hold a second item are split in half.

    Every inventory icon and world object is mapped onto its frame's half (or the whole
    frame when nothing lives in rows 8-15); the source rows each one draws are kept."""
    refs = [(e["icon"]["frame"], e["icon"]["rows"], e["name"], "stuff_index", e["stuff_index"])
            for e in icons if e["icon"]]
    refs += [(o["frame"], o["rows"],
              OB_NAMES.get(o["ob_id"]) or o["item"] or o["enum"].replace("_", " ").title(),
              "ob_id", o["ob_id"]) for o in world]
    split = {f for f, rows, *_ in refs if rows[0] >= 8}
    assets: dict[tuple, dict] = {}
    for frame, rows, name, kind, key in refs:
        half = ([8, 16] if rows[0] >= 8 else [0, 8]) if frame in split else [0, 16]
        a = assets.setdefault((frame, *half), {"name": name, "frame": frame, "rows": half})
        a.setdefault(kind, []).append(key)
        a.setdefault("icon_rows" if kind == "stuff_index" else "world_rows", rows)
    out = sorted(assets.values(), key=lambda a: (a["frame"], a["rows"]))
    for a in out:
        a["file"] = "items/" + re.sub(r"[^a-z0-9]+", "_", a["name"].lower()).strip("_") + ".png"
    return out


# --------------------------------------------------------------------------- #
# Output
# --------------------------------------------------------------------------- #
def write_set(out: Path, name: str, frames: list[np.ndarray], cell, palette) -> dict:
    sheet = pack_sheet(frames, cell)
    for i, fr in enumerate(frames):
        pad = np.full((cell[1], cell[0]), ac.TRANSPARENT_INDEX, dtype=np.uint8)
        pad[:fr.shape[0], :fr.shape[1]] = fr
        ac.write_indexed_png(out / f"frame_{i:03d}.png", pad, palette)
    ac.write_indexed_png(out / f"{name}_sheet.png", sheet, palette)
    ac.write_highlight_mask(out / f"{name}_highlightmask.png", sheet)
    ac.write_bit_mask(out / f"{name}_silhouettemask.png", sheet != ac.TRANSPARENT_INDEX)
    cols = min(SHEET_COLS, len(frames))
    return {
        "frame_size": list(cell),
        "frame_count": len(frames),
        "transparent_index": ac.TRANSPARENT_INDEX,
        "palette": "palettes/pagecolors.json",
        "sheet": {"file": f"{name}_sheet.png", "columns": cols,
                  "highlight_mask": f"{name}_highlightmask.png",
                  "silhouette_mask": f"{name}_silhouettemask.png",
                  "silhouette_source": "src/fsubs.asm:1619-1653"},
    }


def _resolve_frames(node, index: dict):
    if isinstance(node, dict):
        return {k: (index[v] if k == "frame" and isinstance(v, tuple) else _resolve_frames(v, index))
                for k, v in node.items()}
    if isinstance(node, list):
        return [_resolve_frames(v, index) for v in node]
    return node


def emit_actor(a: Actor, sheets: dict, out_dir: Path, palette) -> Path:
    refs = a.ordered_refs()
    index = {k: i for i, k in enumerate(refs)}
    out = out_dir / a.group / a.name
    atlas = {"actor": a.name, "title": a.title, "group": a.group,
             "cfile": {"index": a.cfile, "sheet": SHEET_NAMES[a.cfile],
                       "source": a.src.cfiles[a.cfile]["source"]}}
    atlas.update(a.info)
    atlas.update(write_set(out, a.name, [sheets[cf][fr] for cf, fr in refs], a.cell, palette))
    cols = atlas["sheet"]["columns"]
    w, h = a.cell
    atlas["frames"] = []
    for i, (cf, fr) in enumerate(refs):
        origin = {"cfile": cf, "sheet": SHEET_NAMES[cf], "frame": fr}
        if sheets[cf].shape[1:] != (h, w):
            origin["padded_from"] = [sheets[cf].shape[2], sheets[cf].shape[1]]
        atlas["frames"].append({"index": i, "file": f"frame_{i:03d}.png",
                                "rect": [(i % cols) * w, (i // cols) * h, w, h], "origin": origin})
    if a.weapons:
        atlas["weapons"] = [{"id": wid, "name": WEAPONS[wid]} for wid in a.weapons]
        atlas["overlay_sheet"] = {"file": OBJECTS_SHEET, "frame_size": [16, 16], "columns": SHEET_COLS,
                                  "source": "src/fmain.c:2400-2447"}
    atlas["modes"] = _resolve_frames(a.modes, index)
    if a.notes:
        atlas["notes"] = a.notes
    ac.write_json(out / f"{a.name}.json", atlas)
    return out


def emit_objects(src: Source, sheets: dict, out_dir: Path, palette) -> None:
    cf = src.cfiles[OBJECTS]
    out = out_dir / "objects"
    frames = list(sheets[OBJECTS])
    atlas = {"actor": "objects", "title": "Objects (items, weapon overlays, effects)",
             "group": "objects", "cfile": {"index": OBJECTS, "sheet": SHEET_NAMES[OBJECTS],
                                           "source": cf["source"]}}
    atlas.update(write_set(out, "objects", frames, (16, 16), palette))
    atlas["frames"] = [{"index": i, "file": f"frame_{i:03d}.png",
                        "rect": [(i % SHEET_COLS) * 16, (i // SHEET_COLS) * 16, 16, 16]}
                       for i in range(len(frames))]
    sheet_bytes = cf["count"] * NUM_PLANES * cf["height"] * cf["width"] * 2
    if cf["numblocks"] * BLOCK_SIZE < sheet_bytes:
        atlas["cfile"]["load_shortfall"] = {
            "sheet_bytes": sheet_bytes, "loaded_bytes": cf["numblocks"] * BLOCK_SIZE,
            "note": "read_shapes loads numblocks blocks (fmain2.c:697-698); bytes past "
                    "loaded_bytes are on disk but never loaded by the game."}
    atlas.update(objects_tables(src.dir))
    for it in atlas["items"]:
        r0, r1 = it["rows"]
        ac.write_indexed_png(out / it["file"], frames[it["frame"]][r0:r1], palette)
    atlas["items_note"] = ("items/: one PNG per item. Frames that hold a second item in rows 8-15 "
                           "(ob_id | 0x80, fmain.c:2524) are cut in half; icon_rows / world_rows are "
                           "the rows the items page and the ground draw use.")
    path = out / "objects.json"
    if path.exists():
        old = json.loads(path.read_text())
        if "bindings" in old:
            atlas["bindings"] = old["bindings"]
    ac.write_json(path, atlas)


EFFECTS = [
    ("arrow", "Arrow (missile type 1)", "src/fmain.c:2319",
     "index = missile direction (0-7); missile_type = weapon - 3, so the bow fires type 1 "
     "(fmain.c:1697)", [(d, f"flight {DIRS[d]}") for d in range(8)]),
    ("fireball", "Fireball (missile type 2)", "src/fmain.c:2321-2322",
     "0x59 + direction in flight (wand, fmain.c:1697; dragon, fmain.c:1490). A hit sets type 3 "
     "(fmain.c:2295), drawn as 0x58; the same frame is the lava death (fmain.c:2454)",
     [(0x59 + d, f"flight {DIRS[d]}") for d in range(8)] + [(0x58, "impact")]),
    ("bubbles", "Drowning bubbles", "src/fmain.c:2492-2497",
     "environ > 29: 97 + ((cycle + i) & 1), drawn 8 rows high (ystop = ystart + 7)",
     [(97, "bubbles A"), (98, "bubbles B")]),
]


def _emit_object_set(out: Path, name: str, title: str, source: str, note: str,
                     entries: list[tuple[int, list[int], list[str]]], sheets: dict, palette) -> None:
    """entries: (OBJECTS frame, drawn rows, used_by). Rows the game does not draw are blanked."""
    frames = []
    for frame, (r0, r1), _ in entries:
        fr = sheets[OBJECTS][frame].copy()
        fr[:r0] = ac.TRANSPARENT_INDEX
        fr[r1:] = ac.TRANSPARENT_INDEX
        frames.append(fr)
    atlas = {"name": name, "title": title, "source": source, "note": note}
    atlas.update(write_set(out, name, frames, (16, 16), palette))
    cols = atlas["sheet"]["columns"]
    atlas["frames"] = [{"index": i, "file": f"frame_{i:03d}.png",
                        "rect": [(i % cols) * 16, (i // cols) * 16, 16, 16],
                        "origin": {"cfile": OBJECTS, "sheet": SHEET_NAMES[OBJECTS], "frame": frame},
                        "rows": rows, "used_by": used}
                       for i, (frame, rows, used) in enumerate(entries)]
    ac.write_json(out / f"{name}.json", atlas)


def emit_object_sets(sheets: dict, actors: list[Actor], out_dir: Path, palette) -> None:
    """Weapon sheets: every OBJECTS frame an actor set draws for that weapon. Effect sheets: EFFECTS."""
    for wid in range(1, 6):
        used: dict[tuple[int, tuple], set[str]] = {}
        for a in actors:
            for m in a.modes:
                for seq in (m["facings"].values() if m["directional"] else [m["steps"]]):
                    for st in seq:
                        o = (st.get("overlays") or {}).get(str(wid))
                        if wid == 4 and st.get("state_index", 0) >= 32 and m["id"] != "shoot_bow":
                            continue  # not bow art: hand-weapon rule in dying/frustrated (P25)
                        if o:
                            label = f"{m['id']} (state {st['state_index']})" if "state_index" in st else m["id"]
                            used.setdefault((o["frame"], tuple(o["rows"])), set()).add(label)
        entries = [(f, list(r), sorted(u)) for (f, r), u in sorted(used.items())]
        _emit_object_set(out_dir / "objects" / "weapons" / WEAPONS[wid], WEAPONS[wid],
                         f"{WEAPONS[wid].title()} overlay frames", "src/fmain.c:2400-2447",
                         "Every OBJECTS frame the actor sets draw for this weapon; used_by lists "
                         "mode (statelist index).", entries, sheets, palette)
    for name, title, source, note, frames in EFFECTS:
        entries = [(f, [0, 8] if name == "bubbles" else objects_rows(f)[1], [label]) for f, label in frames]
        _emit_object_set(out_dir / "objects" / "effects" / name, name, title, source, note,
                         entries, sheets, palette)


def emit_raw(src: Source, sheets: dict, out_dir: Path, palette) -> None:
    """One sheet per cfile, frames in original order, for reference."""
    out = out_dir / "raw"
    entries = []
    for i, frames in sheets.items():
        cf = src.cfiles[i]
        h, w = frames.shape[1:]
        name = f"cfile_{i:02d}_{re.sub(r'[^a-z0-9]+', '_', SHEET_NAMES[i])}.png"
        ac.write_indexed_png(out / name, pack_sheet(list(frames), (w, h)), palette)
        e = {"cfile": i, "name": SHEET_NAMES[i], "file": name, "frame_size": [w, h],
             "frame_count": len(frames), "columns": min(SHEET_COLS, len(frames)),
             "file_id": cf["file_id"], "numblocks": cf["numblocks"], "source": cf["source"]}
        loaded = cf["numblocks"] * BLOCK_SIZE // (NUM_PLANES * h * w // 8)
        if loaded < len(frames):
            e["loaded_frames"] = loaded
        entries.append(e)
    ac.write_json(out / "raw.json", {
        "note": "Each cfile decoded as stored: frame n of a sheet is at file_id * 512 + n * 5 planes "
                "* height * width * 2 bytes (fmain.c:2540, fmain2.c:690-698). cfile 12 is skipped "
                "(its file_id is Julian's block). loaded_frames: read_shapes loads only numblocks "
                "blocks, so later frames are never in memory.",
        "sheets": entries})


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    ac.add_io_args(parser)
    parser.add_argument("--out-dir", type=Path, default=REPO_ROOT / "assets" / "sprites")
    parser.add_argument("--palette", type=Path,
                        default=REPO_ROOT / "assets" / "palettes" / "pagecolors.json")
    parser.add_argument("--actor", default="all", help="actor name, 'objects', 'raw', or 'all'")
    args = parser.parse_args(argv)

    image = (args.game_dir / "image").read_bytes()
    palette = load_palette(args.palette)
    src = Source(args.src_dir)
    sheets = {i: decode_frames(image, src.cfiles[i]) for i in SHEET_NAMES}

    if args.actor in ("all", "raw"):
        emit_raw(src, sheets, args.out_dir, palette)
        print(f"raw: {len(sheets)} sheets")
    actors = build_actors(src)
    if args.actor in ("all", "objects"):
        emit_objects(src, sheets, args.out_dir, palette)
        emit_object_sets(sheets, actors, args.out_dir, palette)
        print(f"objects: {src.cfiles[OBJECTS]['count']} frames, weapon and effect sheets")
    for a in actors:
        if args.actor in ("all", a.name):
            emit_actor(a, sheets, args.out_dir, palette)
            print(f"{a.group}/{a.name}: {len(a.ordered_refs())} frames, {len(a.modes)} modes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
