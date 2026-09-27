# docs/ — Agent Contract Documentation

Task-specific guidance for agents working in this repository. The always-on contract lives in [`../AGENTS.md`](../AGENTS.md); this directory holds the detailed docs it links to (loaded on demand).

## Contents

| Document | Read it when… |
|---|---|
| [project-overview.md](project-overview.md) | You need orientation — what the repo is, layout, read-only rules, source-file map, technical context, spatial database. |
| [research-workflow.md](research-workflow.md) | You're orchestrating or participating in reverse-engineering work — agent model, roles, wave workflow, single-topic rule. |
| [documentation-conventions.md](documentation-conventions.md) | You're editing anything under `reference/` — citation format, section numbering, cross-refs, anti-drift rules, verification. |
| [tools-conventions.md](tools-conventions.md) | You're creating or editing scripts under `tools/`. |

## Agent role definitions

Detailed role prompts in [`agents/`](agents/): [scanner](agents/scanner.md) · [discovery](agents/discovery.md) · [researcher](agents/researcher.md) · [experimenter](agents/experimenter.md).

## Workflow procedures

Step-by-step orchestration procedures in [`workflows/`](workflows/): [reverse-engineer](workflows/reverse-engineer.md) · [run-experiment](workflows/run-experiment.md) · [update-doc](workflows/update-doc.md) · [verify-mechanic](workflows/verify-mechanic.md).

## Reference documentation

The research deliverables (ARCHITECTURE, RESEARCH, STORYLINE, logic specs, world_db) are indexed separately in [`../reference/README.md`](../reference/README.md).

## Asset-extraction pipeline

The `assets/tasks/` pipeline is self-contained with its own conventions — see `../assets/tasks/README.md`, `_SHARED.md`, and `STATUS.md`.
