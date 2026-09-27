# Review app plan — browser-based human-in-the-loop asset verification

A local web app that presents each task's extracted assets to the human Reviewer and records
itemized verdicts. It replaces the scattered `VERIFY.md` files: verification instructions ship
as **data** (`verify.json`) and the app renders them. The app is separate from the MkDocs docs
site (`site/PLAN.md`), which keeps its own plan.

## Decisions (locked with the user, 2026-09-26)
- **Separate app** (not part of the MkDocs site), under **`tools/review/`**.
- **Stack:** FastAPI backend (runs in `.toolenv`; `fastapi`/`uvicorn`/pydantic already in
  `tools/requirements.txt`) + **React + Vite + TypeScript** frontend. Node via nvm (v24).
  Frontend is **built on demand**; `dist/` and `node_modules/` are gitignored.
- **Instructions ship as data:** one **`verify.json` per resource subdir**, discovered recursively
  (`assets/**/verify.json`, e.g. `audio/verify.json` and `audio/sfx/verify.json`). Items are tagged
  with their task ID, so one file can hold items from several tasks. `files` are
  relative to the `verify.json`'s own directory and may not escape it. Items for bundle-root files
  (`FORMATS.md`/`formats/`, `manifest.json`, the T4.2 final walkthrough) live in the root
  `assets/verify.json`; discovery includes it. `assets/tasks/` is not scanned.
  `verify.json` files **ship** and are **listed in the manifest** (T4.1). There is **no top-level
  `assets/VERIFY.md`**; the app's task list replaces it (T4.2).
- **`verify.json` may be hand-authored or emitted by an extractor.** Either way, the app validates it
  against a shared schema and flags problems.
- **Previews still ship** under `assets/<subdir>/previews/` for non-perceivable data (music, world
  maps). Palettes have none: the app's swatch view replaces them.
  The app also **renders data natively** (swatches, tables, text) and shows shipped previews where
  they exist.
- **Review unit:** one page per task with **itemized** checks. Each item gets **OK / Problem + an
  optional note**, and the task gets **one overall ACCEPT/REJECT** + notes.
- **Recording:** the local server writes an **append-only history** to
  **`assets/tasks/review_results.json`** (results file only; `STATUS.md` is updated manually).
  This is a deliberate exception to the "write only under `tools/`" rule in
  `docs/tools-conventions.md`.
- **Counts:** `verify.json` states expected counts. The app computes the actual counts from the shipped
  data, shows **actual vs expected**, and highlights mismatches. The Reviewer still decides.
- **Drafts:** item marks/notes are saved **server-side** per task, in gitignored
  `tools/review/.drafts/`, until the verdict is submitted.
- **Submit rules:** items may be left unmarked. **ACCEPT is blocked while any item is Problem.**
  **REJECT requires at least one Problem item or a task-level note.**
- **Staleness:** each submitted round records **SHA-256 of the files it covered**. The task list flags
  an ACCEPT as **stale** if any of those files changed or the task's item set changed.

## Layout (target)
```
tools/review/
  PLAN.md                 # this file
  README.md               # how to run (dev + review session)
  backend/
    app.py                # FastAPI routes; serves frontend dist/ in review mode
    models.py             # pydantic: VerifyFile, Item, Count, Round, ResultsFile, Draft
    store.py              # discovery, validation, count evaluation, hashing, results/drafts I/O
    verify.schema.json    # JSON Schema exported from models.py (reference for authors)
  frontend/               # Vite + React + TS
    package.json  vite.config.ts  tsconfig.json  index.html
    src/
      api.ts  types.ts    # typed API client; types mirror models.py
      pages/TaskList.tsx  pages/TaskPage.tsx
      components/ItemCard.tsx  VerdictBar.tsx  CitationLink.tsx  CountBadge.tsx
      viewers/            # one per `view` kind (see below)
tools/tests/test_review_*.py   # backend tests, per repo convention
```
Run (details go in README):
```
# review session
(cd tools/review/frontend && npm install && npm run build)
.toolenv/bin/python -m uvicorn app:app --app-dir tools/review/backend --host 127.0.0.1 --port 8765
# UI development: backend as above + `npm run dev` (Vite proxies /api and /files to :8765)
```

## `verify.json` schema (v1)
```jsonc
{
  "schema_version": 1,
  "tasks": {                                  // task-level text; each task ID defined in exactly one file
    "T1.2": { "title": "Gameplay tables", "summary": "What this task shipped and how to review it." }
  },
  "items": [
    {
      "id": "statelist",                      // unique within this file
      "task": "T1.2",
      "title": "statelist — animation state table",
      "view": "table",                        // viewer kind (see Viewers)
      "files": ["statelist.json"],            // relative to this subdir; globs allowed
      "look_for": "87 rows × 4 fields; first row …",   // what to see/hear, derived from source/reference
      "citations": ["src/<file>.c:<start>-<end>"],   // repo-relative path:line[-line]
      "counts": [                             // optional; the app computes actual values
        { "label": "rows", "file": "statelist.json", "path": "rows", "expect": 87 },
        { "label": "frames", "glob": "<actor>/frame_*.png", "expect": "<N from CFILES>" }
      ]
    }
  ]
}
```
- **Count kinds:** `file` + dotted `path` → length of the JSON value there (`""` = root;
  numeric segments index lists); `glob` → number of matching files. `expect` is a non-negative
  integer (the `<N …>` above is a placeholder).
- **Citations** must be under `src/` or `reference/` (so the app can open them); the line range must
  exist in the file.
- **Task IDs** match `T<n>(.<n>)*`; item IDs are `[A-Za-z0-9][A-Za-z0-9_.-]*`.
- **Validation** (shown in the UI and via `GET /api/validation`): schema conformance; every `files`
  entry/glob matches at least one existing file; citation syntax is valid and the cited file exists; each item
  `task` has a `tasks` entry somewhere; no duplicate task definitions; unique item IDs per file.
  A task with validation errors can't be submitted.

## Viewers (`view` kinds)
- **v1:** `palette` (swatch grid from `rgba8`, index + `rgb4` labels; shipped swatch PNG shown if listed),
  `table` (HTML table from `fields`/`rows` or `values`), `text` (indexed string list; `%`
  highlighted; strings containing `%` also show `original ---> sample`, with `%` replaced by the
  selected brother's name, as `extract()` does (`src/fmain2.c:527-530`, names from `src/fmain.c:604`)),
  `image` (pixelated, zoomable), `json` (pretty-printed, collapsible).
- **`$` placeholder — removed 2026-09-26, may come back.** `src/narr.asm:4` documents
  `;$ = who we are speaking to??`, but `extract()` (`src/fmain2.c:514-548`) never substitutes `$`, and
  as of this check no text string in `src/` contains it (`$22` in `fsubs.asm:236` is a hex literal).
  What it was meant to mean is unconfirmed, so the viewer treats `$` as plain text. If T1.4 or a
  later task turns up a string that uses `$`, bring back the highlighting in `StringsViewer.tsx`. Show
  a sample value only if the source defines what `$` expands to.
- **Later, as their tasks land:** `image-grid` (sprite frames / tile atlases / glyphs), `audio`
  (`<audio>` for SFX + music previews), `markdown` (FORMATS/README docs), `code` (GLSL).
- **Citations:** clicking a citation opens the cited lines from `src/` or `reference/`, read-only,
  via `GET /api/source`. `verify.json` citations show exactly the range written, so authors must cite
  a whole data structure through its closing `};`, or deliberately cite just part of it. Table
  metadata (`source` + `line`) records only the declaration line, so it uses `block=true`: the
  backend extends it to the line that closes that `{…}` block, ignoring braces in comments and
  string/char literals.

## Backend API
| Method & path | Purpose |
|---|---|
| `GET /api/tasks` | All tasks discovered from `assets/**/verify.json`, each with a state: not reviewed / draft / ACCEPT / REJECT / stale ACCEPT / invalid |
| `GET /api/tasks/{task}` | The task's items (across subdirs), computed counts, validation errors, latest round |
| `GET /api/validation` | All validation problems across the bundle |
| `GET /api/source?path=&start=&end=&block=` | Read-only line range from `src/` or `reference/`; `block=true` without `end` extends to the closing `}` |
| `GET /files/{path}` | Read-only static files from `assets/` |
| `GET/PUT/DELETE /api/drafts/{task}` | Server-side draft of item marks/notes |
| `POST /api/reviews` | Append a round (verdict, notes, per-item status/notes, file hashes); clears the draft |
| `GET /api/reviews?task=` | Round history |

- **Safety:** binds to `127.0.0.1`. `assets/`, `src/` and `reference/` are served read-only, with
  path-traversal guards. The only writes are to the results file (atomic append: write temp, then rename)
  and to the drafts.
- **Errors:** 400 bad path/range, 404 unknown task/file, 409 submit-rule or item-mismatch rejection.
- **Task state precedence:** invalid > draft > stale > ACCEPT/REJECT > not_reviewed; `latest_verdict`,
  `stale` and `has_draft` are also returned separately.
- **Results file:**
  `{"schema_version":1,"rounds":[{"task","submitted_at","verdict","notes","items":[{"resource","id","status","note"}],"files":{"assets/...":"sha256"}}]}`.

## Phases (each ends in a working state)
0. **Convention switch (docs only).** ✅ Done 2026-09-26. Replace the `VERIFY.md` convention with `verify.json` +
   the review app in `assets/plan.md`, `assets/tasks/_SHARED.md`, `assets/tasks/README.md`, every task file's
   Outputs/Manual-verification sections, and T1.5 Part B. Also update T4.2, which plans a top-level
   `assets/VERIFY.md`.
1. **Backend core.** ✅ Done 2026-09-26 (`backend/`, `tools/tests/test_review_backend.py`). Models, schema export, discovery, validation, count evaluation, hashing,
   results append, drafts, read-only file/source routes. Tests: validation cases, count kinds,
   append-only, stale detection, path traversal.
2. **Frontend skeleton.** ✅ Done 2026-09-26 (hash routing, plain CSS, no deps beyond React; files
   dispatch by extension to `json`/`image`/text viewers until Phase 3; drafts autosave after 500 ms and
   are deleted when emptied). Task list (states + validation badges), task page with items using the
   `json`/`image` viewers, OK/Problem + note, draft autosave, overall verdict submit, history panel.
3. **Native viewers.** ✅ Done 2026-09-26. `palette`, `table`, `text`, count badges, citation source popover.
   JSON files use the item's `view`, with a rendered/raw JSON toggle; any shape a viewer can't
   recognize falls back to the JSON tree. `palette` also finds nested entries (`region_overrides`).
   `table` handles `{fields, rows}` and `{values}` (with a per-row width selector), and links
   `source`+`line` as a citation. `text` accepts a string array/object, or an object with one
   string array.
4. **Dogfood on T1.1–T1.3.** T1.5 authors `palettes/verify.json` + `tables/verify.json`; the human
   reviews all three in the app; rounds land in `review_results.json`.
5. **Later viewers** as Wave 2/3 tasks land (`image-grid`, `audio`, `markdown`, `code`).

