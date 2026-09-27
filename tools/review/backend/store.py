"""Discovery, validation, count evaluation, hashing, and results/draft I/O."""
from __future__ import annotations

import hashlib
import json
import os
import re
import tempfile
import threading
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath
from typing import Any, Optional

from pydantic import ValidationError

from models import (
    Draft, DraftBody, Item, ItemMark, Problem, ResultsFile, Round, Submission,
    TaskInfo, VerifyFile,
)

CITATION_RE = re.compile(r"^((?:src|reference)/[^:]+):(\d+)(?:-(\d+))?$")
SOURCE_ROOTS = ("src", "reference")
TASK_FILE_RE = re.compile(r"^T\d+(\.\d+)*$")
NON_CODE_RE = re.compile(r"/\*.*?\*/|//.*$|\"(?:\\.|[^\"\\])*\"|'(?:\\.|[^'\\])*'")
BLOCK_SCAN_LIMIT = 1000


def block_end(lines: list[str], start: int) -> int:
    """1-based line closing the first `{` block opened at or after `start`; `start` if none."""
    depth, opened, in_comment = 0, False, False
    for n in range(start, min(len(lines), start + BLOCK_SCAN_LIMIT) + 1):
        text = lines[n - 1]
        if in_comment:
            if "*/" not in text:
                continue
            text, in_comment = text.split("*/", 1)[1], False
        text = NON_CODE_RE.sub("", text)
        if "/*" in text:
            text, in_comment = text.split("/*", 1)[0], True
        for ch in text:
            if ch == "{":
                depth, opened = depth + 1, True
            elif ch == "}" and opened:
                depth -= 1
                if depth == 0:
                    return n
        if not opened and (n > start or text.rstrip().endswith(";")):
            return start
    return start


class NotFound(Exception):
    pass


class Rejected(Exception):
    pass


def _now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def resolve_within(base: Path, rel: str) -> Path:
    """Resolve `rel` under `base`, refusing absolute paths and escapes."""
    p = PurePosixPath(rel)
    if not rel or p.is_absolute() or ".." in p.parts:
        raise ValueError(f"path not allowed: {rel!r}")
    base = base.resolve()
    target = (base / p).resolve()
    if not target.is_relative_to(base):
        raise ValueError(f"path escapes {base.name}/: {rel!r}")
    return target


def expand(base: Path, pattern: str) -> list[Path]:
    """Files under `base` matching `pattern` (plain path or glob)."""
    resolve_within(base, pattern)
    if any(c in pattern for c in "*?["):
        base_r = base.resolve()
        return sorted(p for p in base.glob(pattern)
                      if p.is_file() and p.resolve().is_relative_to(base_r))
    p = resolve_within(base, pattern)
    return [p] if p.is_file() else []


def read_lines(path: Path) -> list[str]:
    return path.read_text(encoding="utf-8", errors="replace").splitlines()


def _write_json_atomic(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.", suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2, ensure_ascii=False)
            f.write("\n")
            f.flush()
            os.fsync(f.fileno())
        os.replace(tmp, path)
    except BaseException:
        Path(tmp).unlink(missing_ok=True)
        raise


@dataclass
class Bundle:
    tasks: dict[str, tuple[str, TaskInfo]] = field(default_factory=dict)  # id -> (resource, info)
    items: dict[str, list[tuple[str, Item]]] = field(default_factory=dict)  # task -> [(resource, item)]
    problems: list[Problem] = field(default_factory=list)

    def task_ids(self) -> list[str]:
        ids = set(self.tasks) | set(self.items)
        return sorted(ids, key=lambda t: [int(x) for x in re.findall(r"\d+", t)] + [t])

    def problems_for(self, task: str) -> list[Problem]:
        return [p for p in self.problems if p.task == task]


class Store:
    def __init__(self, repo_root: Path, assets_dir: Optional[Path] = None,
                 results_path: Optional[Path] = None, drafts_dir: Optional[Path] = None):
        self.repo_root = Path(repo_root).resolve()
        self.assets_dir = Path(assets_dir or self.repo_root / "assets").resolve()
        self.results_path = Path(results_path or self.assets_dir / "tasks" / "review_results.json")
        self.drafts_dir = Path(drafts_dir or self.repo_root / "tools" / "review" / ".drafts")
        self._lock = threading.Lock()

    # --- paths -------------------------------------------------------------

    def rel(self, path: Path) -> str:
        return path.resolve().relative_to(self.repo_root).as_posix()

    def asset_rel(self, path: Path) -> str:
        return path.resolve().relative_to(self.assets_dir).as_posix()

    def resource_dir(self, resource: str) -> Path:
        return self.assets_dir / resource if resource else self.assets_dir

    def asset_file(self, rel: str) -> Path:
        p = resolve_within(self.assets_dir, rel)
        if not p.is_file():
            raise NotFound(rel)
        return p

    def source_lines(self, rel: str, start: Optional[int] = None,
                     end: Optional[int] = None, block: bool = False) -> dict:
        root, _, rest = rel.partition("/")
        if root not in SOURCE_ROOTS:
            raise ValueError(f"source path must be under src/ or reference/: {rel!r}")
        p = resolve_within(self.repo_root / root, rest)
        if not p.is_file():
            raise NotFound(rel)
        lines = read_lines(p)
        if start is None:
            start, end = 1, end or len(lines)
        elif end is None:
            end = block_end(lines, start) if block and 1 <= start <= len(lines) else start
        if start < 1 or end < start or end > len(lines):
            raise ValueError(f"line range {start}-{end} outside 1-{len(lines)}")
        return {"path": rel, "start": start, "end": end, "lines": lines[start - 1:end]}

    # --- discovery & validation ---------------------------------------------

    def verify_files(self) -> list[Path]:
        tasks_dir = self.assets_dir / "tasks"
        return sorted(p for p in self.assets_dir.rglob("verify.json")
                      if not p.resolve().is_relative_to(tasks_dir.resolve()))

    def load(self) -> Bundle:
        b = Bundle()
        task_defs: dict[str, list[str]] = {}
        for vf in self.verify_files():
            resource = vf.parent.relative_to(self.assets_dir).as_posix()
            resource = "" if resource == "." else resource
            fname = self.rel(vf)
            try:
                raw = json.loads(vf.read_text(encoding="utf-8"))
            except (json.JSONDecodeError, UnicodeDecodeError) as e:
                b.problems.append(Problem(file=fname, message=f"invalid JSON: {e}"))
                continue
            try:
                data = VerifyFile.model_validate(raw)
            except ValidationError as e:
                for t in _raw_task_ids(raw) or [None]:
                    for err in e.errors():
                        loc = ".".join(str(x) for x in err["loc"])
                        b.problems.append(Problem(file=fname, task=t,
                                                  message=f"schema: {loc}: {err['msg']}"))
                continue
            for tid, info in data.tasks.items():
                task_defs.setdefault(tid, []).append(fname)
                b.tasks.setdefault(tid, (resource, info))
            seen: set[str] = set()
            base = self.resource_dir(resource)
            for item in data.items:
                b.items.setdefault(item.task, []).append((resource, item))

                def bad(msg: str) -> None:
                    b.problems.append(Problem(file=fname, task=item.task, item=item.id, message=msg))

                if item.id in seen:
                    bad(f"duplicate item id {item.id!r}")
                seen.add(item.id)
                for pat in item.files:
                    try:
                        if not expand(base, pat):
                            bad(f"files: {pat!r} matches no file")
                    except ValueError as e:
                        bad(f"files: {e}")
                for c in item.citations:
                    err = self.check_citation(c)
                    if err:
                        bad(f"citation {c!r}: {err}")
                for c in item.counts:
                    try:
                        resolve_within(base, c.file or c.glob)
                    except ValueError as e:
                        bad(f"count {c.label!r}: {e}")
        for tid, files in task_defs.items():
            if len(files) > 1:
                for f in files:
                    b.problems.append(Problem(file=f, task=tid,
                                              message=f"task {tid} defined in {len(files)} files: {', '.join(files)}"))
        for tid, entries in b.items.items():
            if tid not in b.tasks:
                for resource, item in entries:
                    b.problems.append(Problem(file=self.rel(self.resource_dir(resource) / "verify.json"),
                                              task=tid, item=item.id,
                                              message=f"task {tid} has no 'tasks' entry in any verify.json"))
        return b

    def check_citation(self, cite: str) -> Optional[str]:
        m = CITATION_RE.match(cite)
        if not m:
            return "expected src/|reference/<path>:<line>[-<line>]"
        path, start, end = m.group(1), int(m.group(2)), int(m.group(3) or m.group(2))
        try:
            p = resolve_within(self.repo_root, path)
        except ValueError as e:
            return str(e)
        if not p.is_file():
            return "cited file does not exist"
        n = len(read_lines(p))
        if start < 1 or end < start or end > n:
            return f"line range {start}-{end} outside 1-{n}"
        return None

    # --- counts & hashing ---------------------------------------------------

    def evaluate_count(self, resource: str, c) -> dict:
        out = {"label": c.label, "expect": c.expect, "actual": None, "ok": False, "error": None}
        base = self.resource_dir(resource)
        try:
            if c.glob is not None:
                out["actual"] = len(expand(base, c.glob))
            else:
                p = resolve_within(base, c.file)
                value: Any = json.loads(p.read_text(encoding="utf-8"))
                for seg in (c.path.split(".") if c.path else []):
                    if isinstance(value, list) and seg.lstrip("-").isdigit():
                        value = value[int(seg)]
                    elif isinstance(value, dict):
                        value = value[seg]
                    else:
                        raise KeyError(seg)
                if not isinstance(value, (list, dict, str)):
                    raise TypeError(f"value at {c.path!r} has no length ({type(value).__name__})")
                out["actual"] = len(value)
        except (OSError, ValueError, KeyError, IndexError, TypeError) as e:
            out["error"] = f"{type(e).__name__}: {e}"
        out["ok"] = out["actual"] == c.expect
        return out

    def covered_files(self, bundle: Bundle, task: str) -> list[Path]:
        paths: set[Path] = set()
        for resource, item in bundle.items.get(task, []):
            base = self.resource_dir(resource)
            pats = list(item.files) + [c.file or c.glob for c in item.counts]
            for pat in pats:
                try:
                    paths.update(p.resolve() for p in expand(base, pat))
                except ValueError:
                    pass
        return sorted(paths)

    def file_hashes(self, bundle: Bundle, task: str) -> dict[str, str]:
        return {self.rel(p): sha256(p) for p in self.covered_files(bundle, task)}

    # --- results ------------------------------------------------------------

    def results(self) -> ResultsFile:
        if not self.results_path.exists():
            return ResultsFile()
        return ResultsFile.model_validate_json(self.results_path.read_text(encoding="utf-8"))

    def rounds(self, task: Optional[str] = None) -> list[Round]:
        return [r for r in self.results().rounds if task is None or r.task == task]

    def is_stale(self, bundle: Bundle, task: str, rnd: Round) -> bool:
        keys_now = {(res, it.id) for res, it in bundle.items.get(task, [])}
        keys_then = {(m.resource, m.id) for m in rnd.items}
        return keys_now != keys_then or self.file_hashes(bundle, task) != rnd.files

    def submit(self, sub: Submission) -> Round:
        with self._lock:
            bundle = self.load()
            marks = self._check_marks(bundle, sub.task, sub.items)
            if bundle.problems_for(sub.task):
                raise Rejected(f"task {sub.task} has validation errors")
            has_problem = any(m.status == "problem" for m in marks.values())
            if sub.verdict == "ACCEPT" and has_problem:
                raise Rejected("ACCEPT is blocked while any item is marked Problem")
            if sub.verdict == "REJECT" and not has_problem and not sub.notes.strip():
                raise Rejected("REJECT needs at least one Problem item or a task-level note")
            items = [marks.get((res, it.id), ItemMark(resource=res, id=it.id))
                     for res, it in bundle.items.get(sub.task, [])]
            rnd = Round(task=sub.task, submitted_at=_now(), verdict=sub.verdict,
                        notes=sub.notes, items=items, files=self.file_hashes(bundle, sub.task))
            results = self.results()
            results.rounds.append(rnd)
            _write_json_atomic(self.results_path, results.model_dump())
            self.delete_draft(sub.task)
            return rnd

    def _check_marks(self, bundle: Bundle, task: str,
                     marks: list[ItemMark]) -> dict[tuple[str, str], ItemMark]:
        if task not in bundle.task_ids():
            raise NotFound(f"unknown task {task!r}")
        valid = {(res, it.id) for res, it in bundle.items.get(task, [])}
        out: dict[tuple[str, str], ItemMark] = {}
        for m in marks:
            key = (m.resource, m.id)
            if key not in valid:
                raise Rejected(f"item {m.resource or '.'}/{m.id} is not part of {task}")
            if key in out:
                raise Rejected(f"item {m.resource or '.'}/{m.id} marked twice")
            out[key] = m
        return out

    # --- drafts -------------------------------------------------------------

    def _draft_path(self, task: str) -> Path:
        if not TASK_FILE_RE.match(task):
            raise NotFound(f"unknown task {task!r}")
        return self.drafts_dir / f"{task}.json"

    def get_draft(self, task: str) -> Optional[Draft]:
        p = self._draft_path(task)
        return Draft.model_validate_json(p.read_text(encoding="utf-8")) if p.exists() else None

    def put_draft(self, task: str, body: DraftBody) -> Draft:
        self._check_marks(self.load(), task, body.items)
        draft = Draft(task=task, updated_at=_now(), **body.model_dump())
        _write_json_atomic(self._draft_path(task), draft.model_dump())
        return draft

    def delete_draft(self, task: str) -> None:
        self._draft_path(task).unlink(missing_ok=True)

    # --- views for the API --------------------------------------------------

    def task_summary(self, bundle: Bundle, task: str) -> dict:
        rounds = self.rounds(task)
        latest = rounds[-1] if rounds else None
        stale = bool(latest and latest.verdict == "ACCEPT" and self.is_stale(bundle, task, latest))
        has_draft = self._draft_path(task).exists()
        n_problems = len(bundle.problems_for(task))
        if n_problems:
            state = "invalid"
        elif has_draft:
            state = "draft"
        elif stale:
            state = "stale"
        elif latest:
            state = latest.verdict
        else:
            state = "not_reviewed"
        resource, info = bundle.tasks.get(task, ("", TaskInfo(title=task)))
        return {
            "task": task, "title": info.title, "state": state,
            "latest_verdict": latest.verdict if latest else None,
            "latest_at": latest.submitted_at if latest else None,
            "stale": stale, "has_draft": has_draft,
            "items": len(bundle.items.get(task, [])), "problems": n_problems,
        }

    def task_detail(self, task: str) -> dict:
        bundle = self.load()
        if task not in bundle.task_ids():
            raise NotFound(f"unknown task {task!r}")
        resource, info = bundle.tasks.get(task, ("", TaskInfo(title=task)))
        items = []
        for res, it in bundle.items.get(task, []):
            base = self.resource_dir(res)
            files: list[str] = []
            for pat in it.files:
                try:
                    files.extend(self.asset_rel(p) for p in expand(base, pat))
                except ValueError:
                    pass
            items.append({
                "resource": res, **it.model_dump(), "resolved_files": files,
                "count_results": [self.evaluate_count(res, c) for c in it.counts],
                "problems": [p.model_dump() for p in bundle.problems_for(task) if p.item == it.id
                             and p.file == self.rel(base / "verify.json")],
            })
        rounds = self.rounds(task)
        return {
            **self.task_summary(bundle, task),
            "summary": info.summary, "resource": resource, "items_detail": items,
            "task_problems": [p.model_dump() for p in bundle.problems_for(task)],
            "latest_round": rounds[-1].model_dump() if rounds else None,
            "draft": d.model_dump() if (d := self.get_draft(task)) else None,
        }


def _raw_task_ids(raw: Any) -> list[str]:
    """Best-effort task IDs from a verify.json that failed schema validation."""
    ids: set[str] = set()
    if isinstance(raw, dict):
        if isinstance(raw.get("tasks"), dict):
            ids.update(k for k in raw["tasks"] if isinstance(k, str))
        if isinstance(raw.get("items"), list):
            ids.update(i["task"] for i in raw["items"]
                       if isinstance(i, dict) and isinstance(i.get("task"), str))
    return sorted(ids)
