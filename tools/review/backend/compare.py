"""Explain a stale ACCEPT: which covered files changed, and did their *content* change.

A round stores the sha256 of every covered file at submit time. When a task shows as stale we
list the files whose hash differs, and on request fetch the accepted version from git history
(the blob whose sha256 matches the recorded one) and compare it semantically with the working
copy: PNGs by decoded pixels (and palette), JSON by structure, anything else by bytes.
"""
from __future__ import annotations

import hashlib
import io
import json
import subprocess
from pathlib import Path
from typing import Any, Optional

MAX_JSON_CHANGES = 60
# A changed file counts as *trivial* -- OK-able straight from the diff view -- when its decoded
# content is identical or a JSON diff touches at most this many paths. Anything bigger, any pixel
# change, an added/removed file or an unrecoverable accepted version needs a normal review.
TRIVIAL_MAX_CHANGES = 10


def stale_changes(then: dict[str, str], now: dict[str, str],
                  items_then: set[tuple[str, str]], items_now: set[tuple[str, str]]) -> dict:
    files = ([{"file": f, "status": "changed"} for f in sorted(then) if f in now and then[f] != now[f]]
             + [{"file": f, "status": "added"} for f in sorted(now) if f not in then]
             + [{"file": f, "status": "removed"} for f in sorted(then) if f not in now])
    return {"files": files,
            "items_added": sorted(f"{r}/{i}" for r, i in items_now - items_then),
            "items_removed": sorted(f"{r}/{i}" for r, i in items_then - items_now)}


def _git(repo: Path, *args: str) -> bytes:
    return subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True).stdout


def accepted_blob(repo: Path, rel: str, sha: str, limit: int = 50) -> tuple[Optional[bytes], Optional[str]]:
    """(bytes, commit) of the most recent historical version of `rel` whose sha256 is `sha`."""
    try:
        revs = _git(repo, "log", f"--max-count={limit}", "--format=%H", "--", rel).decode().split()
    except subprocess.CalledProcessError:
        return None, None
    for rev in revs:
        try:
            data = _git(repo, "show", f"{rev}:{rel}")
        except subprocess.CalledProcessError:
            continue
        if hashlib.sha256(data).hexdigest() == sha:
            return data, rev[:12]
    return None, None


def _json_diff(a: Any, b: Any, path: str, out: list[dict]) -> None:
    if len(out) >= MAX_JSON_CHANGES:
        return
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b), key=str):
            p = f"{path}.{k}" if path else str(k)
            if k not in a:
                out.append({"path": p, "change": "added", "now": _short(b[k])})
            elif k not in b:
                out.append({"path": p, "change": "removed", "then": _short(a[k])})
            else:
                _json_diff(a[k], b[k], p, out)
    elif isinstance(a, list) and isinstance(b, list):
        for i, (x, y) in enumerate(zip(a, b)):
            _json_diff(x, y, f"{path}[{i}]", out)
        for i in range(min(len(a), len(b)), max(len(a), len(b))):   # every extra element is one change
            if len(out) >= MAX_JSON_CHANGES:
                return
            if i < len(b):
                out.append({"path": f"{path}[{i}]", "change": "added", "now": _short(b[i])})
            else:
                out.append({"path": f"{path}[{i}]", "change": "removed", "then": _short(a[i])})
    elif a != b:
        out.append({"path": path, "change": "value", "then": _short(a), "now": _short(b)})


def _short(v: Any, n: int = 120) -> Any:
    if isinstance(v, (dict, list)):
        s = json.dumps(v, separators=(",", ":"))
        return s if len(s) <= n else s[:n] + "…"
    if isinstance(v, str) and len(v) > n:
        return v[:n] + "…"
    return v


def _image_compare(then: bytes, now: bytes) -> dict:
    from PIL import Image
    import numpy as np
    a, b = Image.open(io.BytesIO(then)), Image.open(io.BytesIO(now))
    res: dict[str, Any] = {"kind": "image", "then": {"size": list(a.size), "mode": a.mode},
                           "now": {"size": list(b.size), "mode": b.mode}}
    same_pixels = a.size == b.size and a.mode == b.mode and np.array_equal(np.array(a), np.array(b))
    if a.mode == "P" and b.mode == "P":
        res["palette_identical"] = a.getpalette() == b.getpalette() and a.info.get("transparency") == b.info.get("transparency")
        same_pixels = same_pixels and res["palette_identical"]
    if not same_pixels and a.size == b.size and a.mode == b.mode:
        diff = np.array(a) != np.array(b)
        res["pixels_differing"] = int(diff.reshape(diff.shape[0], diff.shape[1], -1).any(axis=2).sum())
    res["identical"] = res["trivial"] = bool(same_pixels)
    res["summary"] = ("pixels identical (re-encoded only)" if same_pixels else
                      f"{res.get('pixels_differing', '?')} pixels differ" if a.size == b.size and a.mode == b.mode else
                      f"size/mode changed {a.size}/{a.mode} -> {b.size}/{b.mode}")
    return res


def _json_compare(then: bytes, now: bytes) -> dict:
    a, b = json.loads(then), json.loads(now)
    changes: list[dict] = []
    _json_diff(a, b, "", changes)
    n = len(changes)
    return {"kind": "json", "identical": n == 0, "changes": changes,
            "truncated": n >= MAX_JSON_CHANGES, "trivial": n <= TRIVIAL_MAX_CHANGES,
            "summary": "structurally identical (formatting only)" if n == 0 else
                       f"{'≥' if n >= MAX_JSON_CHANGES else ''}{n} JSON change{'s' if n != 1 else ''}"}


def compare_bytes(rel: str, then: bytes, now: bytes) -> dict:
    """Semantic comparison of two versions of one file."""
    suffix = Path(rel).suffix.lower()
    try:
        if suffix == ".png":
            return _image_compare(then, now)
        if suffix == ".json":
            return _json_compare(then, now)
    except Exception as e:  # decode failure falls back to bytes
        note = f"{type(e).__name__}: {e}"
    else:
        note = None
    res = {"kind": "bytes", "identical": then == now, "trivial": then == now,
           "then": {"bytes": len(then)}, "now": {"bytes": len(now)},
           "summary": "bytes identical" if then == now else f"bytes differ ({len(then)} -> {len(now)})"}
    if note:
        res["note"] = note
    return res
