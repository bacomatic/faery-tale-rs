# Remove Legacy Planning System

**Date:** 2026-03-27
**Status:** Draft

## Problem

The project's original planning infrastructure — `PLAN.md`, `plan_status.toml`,
and associated sync scripts — tracked 126 tasks across 16 rollup issues. All
tasks are now complete and all GitHub rollup issues are closed. The system is
dead weight: 115 KB of static data, 5 shell scripts, git hook integrations, and
contract sections in AGENTS.md / CLAUDE.md / README.md that impose maintenance
burden with no ongoing value.

Forward-looking design work already lives in `docs/superpowers/specs/` and
`docs/superpowers/plans/`, making the old system redundant.

## Approach

Delete outright. Git history preserves the full record. No archiving needed.

## Files to Delete

| File | Size | Purpose (now obsolete) |
|------|------|----------------------|
| `PLAN.md` | 79 KB | Human-readable roadmap (100% done) |
| `plan_status.toml` | 36 KB | Machine-readable task state (126/126 done) |
| `scripts/plan_sync_check.sh` | 192 lines | Consistency validator |
| `scripts/sync_plan_from_github.sh` | 13 lines | Orchestrator |
| `scripts/sync_rollup_issue_states.sh` | 150 lines | GitHub → TOML sync |
| `scripts/refresh_issue_map.sh` | 117 lines | PLAN.md issue map generator |

## Files to Update

### `scripts/check_docs_links.sh`

Remove `PLAN.md`, `plan_status.toml`, and `scripts/plan_sync_check.sh` from the
`required_files` array. Keep all other entries.

### `.githooks/pre-push`

Remove the two lines that run `refresh_issue_map.sh` and `plan_sync_check.sh`.
If the hook becomes empty (no remaining commands), delete the file.

### `.githooks/pre-commit`

Remove the line that runs `plan_sync_check.sh`. Keep `check_docs_links.sh` and
the `faery.toml` validation.

### `Makefile`

Remove the `plan-check` and `sync-issues` targets. Remove them from the
`.PHONY` line. Keep `docs-check`.

### `AGENTS.md`

- **"Canonical sources by topic"**: Replace the `PLAN.md + plan_status.toml`
  bullet with a pointer to `docs/superpowers/specs/` for design specs and plans.
- **Delete entire sections**: "Planning files contract", "State update do/don't",
  "Issue tracking memory".

### `CLAUDE.md`

- **"Canonical sources by topic"**: Same change as AGENTS.md — replace PLAN.md
  reference with `docs/superpowers/specs/` pointer.
- **Delete entire section**: "Planning files contract".

### `README.md`

- **"Canonical Sources" (line 15)**: Replace PLAN.md/plan_status.toml bullet
  with `docs/superpowers/specs/` reference.
- **"Git hooks" section (lines 78–105)**: Rewrite to reflect simplified hooks
  (remove plan sync references, keep hook setup instructions).
- **"Common shortcuts" section (lines 107–113)**: Remove `make plan-check` and
  `make sync-issues` entries. Keep `make docs-check`.

## Files Kept Unchanged

- `research_index.toml` — serves a different purpose (agent research lookup)
- `docs/superpowers/` — forward-looking specs and plans (unrelated system)
- GitHub Issues — all 16 rollup issues already closed, no action needed

## Validation

After all changes:
1. `bash scripts/check_docs_links.sh` passes
2. `make docs-check` passes
3. `.githooks/pre-commit` runs cleanly
4. `.githooks/pre-push` runs cleanly (or is deleted if empty)
5. No remaining references to `PLAN.md` or `plan_status.toml` in tracked files
   (verify with `git grep -l 'PLAN\.md\|plan_status'`)
