"""Tests for the review app backend (tools/review/backend).

Covers verify.json validation, count kinds, append-only results, staleness,
submit rules, drafts, and path-traversal guards, against a synthetic repo.
"""
import json
import sys
from pathlib import Path

import pytest

BACKEND = Path(__file__).resolve().parents[1] / "review" / "backend"
if str(BACKEND) not in sys.path:
    sys.path.insert(0, str(BACKEND))

from fastapi.testclient import TestClient  # noqa: E402

import models  # noqa: E402
from app import create_app  # noqa: E402
from store import Rejected, Store  # noqa: E402


def write(path: Path, data) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(data if isinstance(data, str) else json.dumps(data))
    return path


def item(id_, task="T1.2", files=("a.json",), **kw):
    return {"id": id_, "task": task, "title": id_, "view": "table",
            "files": list(files), "look_for": "x", **kw}


@pytest.fixture
def repo(tmp_path):
    write(tmp_path / "src" / "foo.c", "\n".join(f"line {i}" for i in range(1, 11)) + "\n")
    write(tmp_path / "reference" / "doc.md", "one\ntwo\n")
    write(tmp_path / "secret.txt", "nope")
    t = tmp_path / "assets" / "tables"
    write(t / "a.json", {"fields": ["f"], "rows": [{"f": 1}, {"f": 2}, {"f": 3}]})
    write(t / "b.json", [1, 2])
    write(t / "sub" / "x.png", "png")
    write(t / "sub" / "y.png", "png")
    write(t / "verify.json", {
        "schema_version": 1,
        "tasks": {"T1.2": {"title": "Tables"}, "T1.3": {"title": "Items"}},
        "items": [
            item("a", citations=["src/foo.c:2-4"],
                 counts=[{"label": "rows", "file": "a.json", "path": "rows", "expect": 3}]),
            item("pngs", files=["sub/*.png"],
                 counts=[{"label": "frames", "glob": "sub/*.png", "expect": 2}]),
            item("b", task="T1.3", files=["b.json"]),
        ],
    })
    return tmp_path


@pytest.fixture
def store(repo):
    return Store(repo, drafts_dir=repo / "drafts")


@pytest.fixture
def client(store):
    return TestClient(create_app(store, dist=None))


def set_verify(repo, data, sub="tables"):
    write(repo / "assets" / sub / "verify.json", data)


def messages(store):
    return [p.message for p in store.load().problems]


# --- validation --------------------------------------------------------------

def test_valid_bundle_has_no_problems(store):
    b = store.load()
    assert b.problems == []
    assert b.task_ids() == ["T1.2", "T1.3"]
    assert [i.id for _, i in b.items["T1.2"]] == ["a", "pngs"]


def test_schema_version_and_view_enforced(repo, store):
    set_verify(repo, {"schema_version": 2, "tasks": {"T1.2": {"title": "t"}},
                      "items": [item("a", view="nope")]})
    b = store.load()
    assert b.problems and all(p.task == "T1.2" for p in b.problems)
    assert any("schema_version" in p.message for p in b.problems)
    assert any("view" in p.message for p in b.problems)


def test_invalid_json_reported(repo, store):
    set_verify(repo, "{not json")
    assert any("invalid JSON" in m for m in messages(store))


@pytest.mark.parametrize("files,needle", [
    (["missing.json"], "matches no file"),
    (["sub/*.gif"], "matches no file"),
    (["../../secret.txt"], "not allowed"),
    (["/etc/passwd"], "not allowed"),
])
def test_files_must_exist_and_stay_inside(repo, store, files, needle):
    set_verify(repo, {"schema_version": 1, "tasks": {"T1.2": {"title": "t"}},
                      "items": [item("a", files=files)]})
    assert any(needle in m for m in messages(store))


@pytest.mark.parametrize("cite,needle", [
    ("foo.c:1", "expected"),
    ("src/foo.c", "expected"),
    ("src/nope.c:1", "does not exist"),
    ("src/foo.c:9-12", "outside"),
    ("src/foo.c:5-3", "outside"),
    ("src/../secret.txt:1", "not allowed"),
    ("assets/tables/a.json:1", "expected"),
])
def test_citation_checks(repo, store, cite, needle):
    set_verify(repo, {"schema_version": 1, "tasks": {"T1.2": {"title": "t"}},
                      "items": [item("a", citations=[cite])]})
    assert any(needle in m for m in messages(store))


def test_reference_citation_ok(repo, store):
    set_verify(repo, {"schema_version": 1, "tasks": {"T1.2": {"title": "t"}},
                      "items": [item("a", citations=["reference/doc.md:2"])]})
    assert messages(store) == []


def test_undefined_task_duplicate_task_and_duplicate_item(repo, store):
    set_verify(repo, {"schema_version": 1, "tasks": {"T1.2": {"title": "t"}},
                      "items": [item("a"), item("a"), item("z", task="T9.9")]})
    write(repo / "assets" / "palettes" / "p.json", [])
    set_verify(repo, {"schema_version": 1, "tasks": {"T1.2": {"title": "again"}},
                      "items": [item("p", files=["p.json"])]}, sub="palettes")
    m = messages(store)
    assert any("duplicate item id 'a'" in x for x in m)
    assert any("T9.9 has no 'tasks' entry" in x for x in m)
    assert sum("defined in 2 files" in x for x in m) == 2


def test_count_needs_exactly_one_kind(repo, store):
    set_verify(repo, {"schema_version": 1, "tasks": {"T1.2": {"title": "t"}},
                      "items": [item("a", counts=[{"label": "x", "expect": 1}])]})
    assert any("exactly one" in m for m in messages(store))


def test_root_verify_discovered_and_tasks_dir_ignored(repo, store):
    write(repo / "assets" / "FORMATS.md", "# f")
    set_verify(repo, {"schema_version": 1, "tasks": {"T3.2": {"title": "Formats"}},
                      "items": [item("formats", task="T3.2", files=["FORMATS.md"])]}, sub="")
    write(repo / "assets" / "tasks" / "verify.json", "{bad")
    b = store.load()
    assert "T3.2" in b.task_ids() and b.problems == []
    assert b.items["T3.2"][0][0] == ""


def test_schema_file_in_sync():
    assert json.loads(models.SCHEMA_PATH.read_text()) == models.verify_schema()


# --- counts ------------------------------------------------------------------

@pytest.mark.parametrize("count,actual", [
    ({"file": "a.json", "path": "rows"}, 3),
    ({"file": "a.json", "path": ""}, 2),
    ({"file": "b.json"}, 2),
    ({"file": "a.json", "path": "rows.0"}, 1),
    ({"glob": "sub/*.png"}, 2),
    ({"glob": "**/*.png"}, 2),
])
def test_count_kinds(store, count, actual):
    c = models.Count(label="n", expect=actual, **count)
    r = store.evaluate_count("tables", c)
    assert (r["actual"], r["ok"], r["error"]) == (actual, True, None)


@pytest.mark.parametrize("count", [
    {"file": "a.json", "path": "nope"},
    {"file": "a.json", "path": "rows.0.f"},
    {"file": "missing.json"},
])
def test_count_errors(store, count):
    r = store.evaluate_count("tables", models.Count(label="n", expect=1, **count))
    assert r["actual"] is None and r["ok"] is False and r["error"]


def test_count_mismatch_flagged(store):
    r = store.evaluate_count("tables", models.Count(label="n", glob="sub/*.png", expect=5))
    assert r == {"label": "n", "expect": 5, "actual": 2, "ok": False, "error": None}


# --- submit rules, append-only, staleness ------------------------------------

def mark(id_, status=None, note="", resource="tables"):
    return models.ItemMark(resource=resource, id=id_, status=status, note=note)


def submit(store, verdict, items=(), notes="", task="T1.2"):
    return store.submit(models.Submission(task=task, verdict=verdict, notes=notes, items=list(items)))


def test_accept_blocked_by_problem(store):
    with pytest.raises(Rejected, match="ACCEPT is blocked"):
        submit(store, "ACCEPT", [mark("a", "problem")])


def test_reject_needs_problem_or_note(store):
    with pytest.raises(Rejected, match="REJECT needs"):
        submit(store, "REJECT", [mark("a", "ok")])
    submit(store, "REJECT", notes="wrong colours")
    submit(store, "REJECT", [mark("a", "problem", "row 2 off")])
    assert [r.verdict for r in store.rounds("T1.2")] == ["REJECT", "REJECT"]


def test_unmarked_items_allowed_and_recorded(store):
    rnd = submit(store, "ACCEPT", [mark("a", "ok")])
    assert [(m.id, m.status) for m in rnd.items] == [("a", "ok"), ("pngs", None)]
    assert set(rnd.files) == {"assets/tables/a.json", "assets/tables/sub/x.png",
                              "assets/tables/sub/y.png"}


def test_foreign_or_duplicate_marks_rejected(store):
    with pytest.raises(Rejected, match="not part of"):
        submit(store, "ACCEPT", [mark("b")])
    with pytest.raises(Rejected, match="marked twice"):
        submit(store, "ACCEPT", [mark("a"), mark("a")])


def test_invalid_task_cannot_be_submitted(repo, store):
    set_verify(repo, {"schema_version": 1, "tasks": {"T1.2": {"title": "t"}},
                      "items": [item("a", files=["missing.json"])]})
    with pytest.raises(Rejected, match="validation errors"):
        submit(store, "ACCEPT")


def test_results_append_only(store):
    submit(store, "REJECT", notes="first")
    first = json.loads(store.results_path.read_text())["rounds"][0]
    submit(store, "ACCEPT")
    submit(store, "ACCEPT", task="T1.3", items=[mark("b", "ok")])
    data = json.loads(store.results_path.read_text())
    assert data["schema_version"] == 1
    assert data["rounds"][0] == first
    assert [(r["task"], r["verdict"]) for r in data["rounds"]] == [
        ("T1.2", "REJECT"), ("T1.2", "ACCEPT"), ("T1.3", "ACCEPT")]
    assert not list(store.results_path.parent.glob(".*.tmp"))


def state(store, task="T1.2"):
    return store.task_summary(store.load(), task)


def test_states_and_staleness_on_file_change(repo, store):
    assert state(store)["state"] == "not_reviewed"
    submit(store, "ACCEPT")
    assert state(store)["state"] == "ACCEPT"
    write(repo / "assets" / "tables" / "sub" / "x.png", "changed")
    s = state(store)
    assert s["state"] == "stale" and s["stale"] and s["latest_verdict"] == "ACCEPT"


def test_staleness_on_new_matching_file(repo, store):
    submit(store, "ACCEPT")
    write(repo / "assets" / "tables" / "sub" / "z.png", "png")
    assert state(store)["state"] == "stale"


def test_staleness_on_item_set_change(repo, store):
    submit(store, "ACCEPT")
    data = json.loads((repo / "assets" / "tables" / "verify.json").read_text())
    data["items"].append(item("b2", files=["b.json"]))
    set_verify(repo, data)
    assert state(store)["state"] == "stale"


def carried(store, task="T1.2"):
    return {i["id"]: i["carried_ok"] for i in store.task_detail(task)["items_detail"]}


def test_carried_ok_needs_ok_mark_and_unchanged_files(repo, store):
    assert carried(store) == {"a": False, "pngs": False}
    submit(store, "REJECT", [mark("a", "ok"), mark("pngs", "problem")])
    assert carried(store) == {"a": True, "pngs": False}
    write(repo / "assets" / "tables" / "a.json", {"rows": [1, 2, 3], "changed": 1})
    assert carried(store) == {"a": False, "pngs": False}


def test_carried_ok_cleared_by_new_matching_file(repo, store):
    submit(store, "ACCEPT", [mark("a", "ok"), mark("pngs", "ok")])
    assert carried(store) == {"a": True, "pngs": True}
    write(repo / "assets" / "tables" / "sub" / "z.png", "png")
    assert carried(store) == {"a": True, "pngs": False}


def test_reject_never_stale(repo, store):
    submit(store, "REJECT", notes="bad")
    write(repo / "assets" / "tables" / "a.json", [])
    assert state(store)["state"] == "REJECT"


def test_invalid_state(repo, store):
    set_verify(repo, {"schema_version": 1, "tasks": {"T1.2": {"title": "t"}},
                      "items": [item("a", files=["missing.json"])]})
    assert state(store)["state"] == "invalid"


# --- API: drafts, submit, traversal -------------------------------------------

def test_api_tasks_and_detail(client):
    tasks = client.get("/api/tasks").json()
    assert [t["task"] for t in tasks] == ["T1.2", "T1.3"]
    d = client.get("/api/tasks/T1.2").json()
    a = d["items_detail"][0]
    assert a["resolved_files"] == ["tables/a.json"]
    assert a["count_results"][0]["actual"] == 3
    assert client.get("/api/tasks/T7.7").status_code == 404


def test_api_draft_roundtrip_and_cleared_on_submit(client, store):
    assert client.get("/api/drafts/T1.2").status_code == 404
    body = {"notes": "wip", "items": [{"resource": "tables", "id": "a", "status": "ok"}]}
    r = client.put("/api/drafts/T1.2", json=body)
    assert r.status_code == 200 and r.json()["updated_at"]
    assert client.get("/api/drafts/T1.2").json()["notes"] == "wip"
    assert client.get("/api/tasks").json()[0]["state"] == "draft"
    bad = {"items": [{"resource": "tables", "id": "b"}]}
    assert client.put("/api/drafts/T1.2", json=bad).status_code == 409
    assert client.put("/api/drafts/..%2Fx", json=body).status_code == 404
    r = client.post("/api/reviews", json={"task": "T1.2", "verdict": "ACCEPT", **body})
    assert r.status_code == 201
    assert client.get("/api/drafts/T1.2").status_code == 404
    assert client.get("/api/reviews", params={"task": "T1.2"}).json()[0]["verdict"] == "ACCEPT"


def test_api_submit_rules_return_409(client):
    r = client.post("/api/reviews", json={"task": "T1.2", "verdict": "REJECT"})
    assert r.status_code == 409


def test_api_files_served_and_traversal_blocked(client, store):
    for rel in ["../secret.txt", "tables/../../secret.txt", "/etc/passwd"]:
        with pytest.raises(ValueError):
            store.asset_file(rel)
    r = client.get("/files/tables/b.json")
    assert r.json() == [1, 2] and r.headers["cache-control"] == "no-cache"
    for p in ["/files/../secret.txt", "/files/%2e%2e/secret.txt",
              "/files/tables/%2e%2e/%2e%2e/secret.txt", "/files/tables/nope.json",
              "/files/%2Fetc%2Fpasswd"]:
        r = client.get(p)
        assert r.status_code == 404, p
        assert r.text != "nope"


ARR_C = """\
int x = 1;
char probs[] = {
\t0, 0,\t/* no { treasure */
\t'}', 9,\t// } stray
\t1, 2
};
struct s tbl[] =
{
\t{1, 2},
\t{3, 4}
};
char tail[] = { 1,
/* } still
   in comment { */
2 };
"""


@pytest.mark.parametrize("start,end", [
    (2, 6),    # multi-line array; braces in comments / char literal ignored
    (7, 11),   # opening brace on the next line; nested braces
    (12, 15),  # multi-line block comment containing braces
    (1, 1),    # no block: stays a single line
    (6, 6),    # closing line alone: no new block opens
])
def test_source_block_expands_to_closing_brace(repo, client, start, end):
    write(repo / "src" / "arr.c", ARR_C)
    r = client.get("/api/source", params={"path": "src/arr.c", "start": start, "block": True}).json()
    assert (r["start"], r["end"]) == (start, end)


def test_source_block_ignored_with_explicit_end(repo, client):
    write(repo / "src" / "arr.c", ARR_C)
    r = client.get("/api/source", params={"path": "src/arr.c", "start": 2, "end": 3, "block": True}).json()
    assert (r["start"], r["end"]) == (2, 3)


def test_api_source(client):
    r = client.get("/api/source", params={"path": "src/foo.c", "start": 2, "end": 3})
    assert r.json()["lines"] == ["line 2", "line 3"]
    assert client.get("/api/source", params={"path": "src/foo.c", "start": 5}).json()["lines"] == ["line 5"]
    assert len(client.get("/api/source", params={"path": "reference/doc.md"}).json()["lines"]) == 2
    for path in ["secret.txt", "src/../secret.txt", "assets/tables/a.json", "/etc/passwd", "src"]:
        assert client.get("/api/source", params={"path": path}).status_code == 400, path
    assert client.get("/api/source", params={"path": "src/foo.c", "start": 11}).status_code == 400
    assert client.get("/api/source", params={"path": "src/nope.c"}).status_code == 404
