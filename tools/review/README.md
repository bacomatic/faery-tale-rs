# Review app

Local web app for human review of the `assets/` bundle. Design and schema: [`PLAN.md`](PLAN.md).

## Review session
```
mise run setup          # once: .venv + npm deps
mise run review:build   # build frontend/dist/
mise run review         # serve on :8765
```
Open <http://127.0.0.1:8765>. The backend serves the built `frontend/dist/` at `/`.

## Who runs which server
- **`:8765` (`mise run review`) belongs to the human.** Agents never start, stop or probe it. When a
  backend change needs a restart, the agent says so and leaves the restart to the human.
- **`:8766` (`mise run review:test`) is the agent's test instance** — `--reload` picks up backend
  edits, so it never needs restarting. The agent kills it before finishing the task it was started for.
- Both serve `frontend/dist/` from disk, so a `review:build` shows up on both at the next page load.

## UI development
Run `mise run review`, then `mise run review:dev` (Vite proxies `/api` and `/files` to `:8765`).
`mise exec -- npm --prefix tools/review/frontend run typecheck` runs `tsc` only.

## Writes
- `assets/tasks/review_results.json` — append-only review rounds (on submit).
- `tools/review/.drafts/<task>.json` — in-progress marks (gitignored; deleted on submit).

## Author aids
- `backend/verify.schema.json` — JSON Schema for `verify.json`. Regenerate after editing
  `backend/models.py`: `mise exec -- python tools/review/backend/models.py`.
- `GET /api/validation` lists every problem across all `verify.json` files.

## Tests
```
mise exec -- python -m pytest tools/tests/test_review_backend.py
```
