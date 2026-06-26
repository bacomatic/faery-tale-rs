# Task index — Asset Pipeline

Decomposition of [`../plan.md`](../plan.md). Each task is a self-contained file. Assign one
**Implementer** agent and one **separate Verifier** agent per task (see `_SHARED.md`).

| Task | Title | Depends on |
|---|---|---|
| [T0.1](T0.1-scaffolding.md) | Repo scaffolding & shared helpers | — |
| [T0.2](T0.2-carray-baseline.md) | C-array extraction baseline | — |
| [T1.1](T1.1-palettes.md) | Palettes extractor | T0.1, T0.2 |
| [T1.2](T1.2-tables.md) | Gameplay tables extractor | T0.2 |
| [T1.3](T1.3-item-quest.md) | Item/quest data fold-in | T0.1 |
| [T1.4](T1.4-text.md) | Narrative text extractor | T0.1 |
| [T2.1](T2.1-sprites.md) | Sprites extractor extension | T0.1 |
| [T2.2](T2.2-tiles.md) | Background tile atlas extractor | T0.1 |
| [T2.3](T2.3-masks.md) | Shadow/collision masks extractor | T0.1 |
| [T2.4](T2.4-screens.md) | IFF/ILBM screens extractor | T0.1 |
| [T2.5](T2.5-world.md) | World data extension | T0.1 |
| [T2.6](T2.6-music.md) | Music + instruments extractor | T0.1 |
| [T2.7](T2.7-sfx.md) | SFX extractor | T0.1 |
| [T2.8](T2.8-fonts.md) | Fonts extractor | T0.1 |
| [T3.1](T3.1-shaders.md) | Reference shaders | T2.1, T2.2 |
| [T3.2](T3.2-formats.md) | Format spec | Wave 1 + Wave 2 |
| [T4.1](T4.1-manifest.md) | Manifest + driver | Wave 1 + Wave 2 |
| [T4.2](T4.2-verification.md) | Verification harness | T4.1 |

## Waves (parallel within a wave)
```
Wave 0:  T0.1  T0.2
Wave 1:  T1.1  T1.2  T1.3  T1.4
Wave 2:  T2.1  T2.2  T2.3  T2.4  T2.5  T2.6  T2.7  T2.8
Wave 3:  T3.1  T3.2
Wave 4:  T4.1  T4.2
```

## Running a task (and resuming in a future session)

**Live progress lives in [`STATUS.md`](STATUS.md).** Always read it first to see what is
done, what passed verification, and what is next. Update it as tasks change state.

Each task is executed by **two different subagents** (see `_SHARED.md` → Roles):

1. **Implementer** — give it a fresh general-purpose subagent. Prompt it to read
   `_SHARED.md` + its one task file, do only the "Implementation" section, run the
   "Implementer self-check", and report files created + test results. No commits.
2. **Verifier** — a *separate* fresh subagent. Prompt it to read `_SHARED.md` + the same
   task file, perform the task's "Verification (DIFFERENT agent)" section **without trusting
   the implementer or reusing its tests**, and return a PASS/FAIL verdict with concrete values.

A task is **done** only when the Verifier returns PASS. On FAIL, the Verifier's findings go
back to a (fresh) Implementer. Record every transition in `STATUS.md`.

To resume: read `STATUS.md`, pick the next `TODO` task whose dependencies are all `DONE`,
and dispatch the Implementer→Verifier pair. Honor the env notes in `_SHARED.md`
(`.toolenv` venv, `uv` for installs).
