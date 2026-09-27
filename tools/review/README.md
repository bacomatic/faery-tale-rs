# Review app

Local web app for human review of the `assets/` bundle. Design and schema: [`PLAN.md`](PLAN.md).

## Review session
```
(cd tools/review/frontend && npm install && npm run build)   # Node via nvm (v24)
.toolenv/bin/python -m uvicorn app:app --app-dir tools/review/backend --host 127.0.0.1 --port 8765
```
Open <http://127.0.0.1:8765>. The backend serves the built `frontend/dist/` at `/`.

## UI development
Run the backend as above, then `npm run dev` in `tools/review/frontend/` (Vite proxies `/api` and
`/files` to `:8765`). `npm run typecheck` runs `tsc` only.

## Writes
- `assets/tasks/review_results.json` — append-only review rounds (on submit).
- `tools/review/.drafts/<task>.json` — in-progress marks (gitignored; deleted on submit).

## Author aids
- `backend/verify.schema.json` — JSON Schema for `verify.json`. Regenerate after editing
  `backend/models.py`: `.toolenv/bin/python tools/review/backend/models.py`.
- `GET /api/validation` lists every problem across all `verify.json` files.

## Tests
```
.toolenv/bin/python -m pytest tools/tests/test_review_backend.py
```
