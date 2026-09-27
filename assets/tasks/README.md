# Task index — Asset Extraction

Decomposition of [`../plan.md`](../plan.md). Each task is a self-contained file. **The deliverable
is the extracted `assets/`; agents extract, and scripts are written only to decode binary or very
large inputs** (see `_SHARED.md`). Each task is done by an **Implementer** agent and accepted by
a **separate Reviewer** who inspects the emitted assets (human-in-the-loop verification).

| Task | Title | Depends on |
|---|---|---|
| [T0.1](T0.1-scaffolding.md) | Repo scaffolding & shared helpers | — |
| [T0.2](T0.2-carray-baseline.md) | C-array extraction baseline | — |
| [T1.1](T1.1-palettes.md) | Palettes extractor | T0.1, T0.2 |
| [T1.2](T1.2-tables.md) | ~~Gameplay tables~~ **dropped**: constants in `reference/`, art bindings folded into T2.1 | — |
| T1.3 | ~~Item/quest data~~ **dropped**: behavior spec'd in `reference/logic/` (`magic.md`, `menu-system.md`, `brother-succession.md`) | — |
| [T1.4](T1.4-text.md) | Narrative text extractor | T0.1 |
| [T1.5](T1.5-retrofit-done.md) | Retrofit done tasks to current acceptance model | T1.1 |
| [T2.1](T2.1-sprites.md) | Sprites extractor extension | T0.1, T1.1 |
| [T2.2](T2.2-tiles.md) | Background tile atlas extractor | T0.1, T1.1 |
| [T2.3](T2.3-masks.md) | Shadow/collision masks extractor | T0.1 |
| [T2.4](T2.4-screens.md) | IFF/ILBM screens extractor | T0.1 |
| [T2.5](T2.5-world.md) | World data extension | T0.1 |
| [T2.6](T2.6-music.md) | Music + instruments extractor | T0.1 |
| [T2.7](T2.7-sfx.md) | SFX extractor | T0.1 |
| [T2.8](T2.8-fonts.md) | Fonts extractor | T0.1 |
| [T3.1](T3.1-shaders.md) | Reference shaders + light-level renders | T2.1, T2.2 |
| [T3.2](T3.2-formats.md) | Format spec | Wave 1 + Wave 2 |
| [T4.1](T4.1-manifest.md) | Bundle index (manifest) | Wave 1 + Wave 2 |
| [T4.2](T4.2-verification.md) | Bundle acceptance (human) | T4.1 |

## Waves (parallel within a wave)
```
Wave 0:  T0.1  T0.2
Wave 1:  T1.1  T1.4  T1.5
Wave 2:  T2.1  T2.2  T2.3  T2.4  T2.5  T2.6  T2.7  T2.8
Wave 3:  T3.1  T3.2
Wave 4:  T4.1  T4.2
```

## Running a task (and resuming in a future session)

**Live progress lives in [`STATUS.md`](STATUS.md).** Always read it first to see what is
done, what passed verification, and what is next. Update it as tasks change state.

Each task has two roles (see `_SHARED.md` → Roles):

1. **Implementer** — a fresh general-purpose subagent. Prompt it to read `_SHARED.md` + its one
   task file, do the "Implementation" section, **produce the assets** (plus previews where the
   task calls for them), and author the task's `verify.json` items. The "Implementer self-check" is an
   optional aid. No commits.
2. **Reviewer** — **the human**, verifying the final assets **by perception only**: open the
   images, play the audio, glance at tables/JSON in the **review app** (`tools/review/`, items from
   `verify.json`), then give ACCEPT/REJECT with concrete notes. The app appends the verdict to
   `assets/tasks/review_results.json`. No re-extraction, no hand-decoding, no verification code. Anything perception
   misses will be caught during port implementation and revisited.

A task is **done** only when the Reviewer accepts the assets. On REJECT, the findings go back to
a (fresh) Implementer. Record every transition in `STATUS.md`.

To resume: read `STATUS.md`, pick the next `TODO` task whose dependencies are all `DONE`, and
dispatch the Implementer→Reviewer pair. Honor the env notes in `_SHARED.md`
(mise-managed `.venv`, `uv` for installs).
