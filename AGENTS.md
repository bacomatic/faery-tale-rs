# AGENTS.md

This file is the agent contract for this repository. Keep it stable and concise.

## Behavioral Guidelines

### 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

### 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

### 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

### 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

## Project constraints

- Rust port of "The Faery Tale Adventure" (1987 Amiga game by MicroIllusions).
- **Fidelity first**: be true to the original game. Do not add enhancements or modernizations.
- Fidelity means matching the documented original behavior and player experience, even if the implementation structure differs.
- Do not intentionally "fix" bugs in the original game unless reproducing them would require extra work.
- Some bugs will be fixed and quality of life improvements will be added where it makes sense, and **always** at the direction of the user.
- **This project is specification-driven** - rely on the reference docs hosted on the `research` branch (see "Reference documentation" below) and the implementation specifications and requirements in `docs/`.
- Fix warnings when they happen, do not consider code to be clean or a task complete if it's emitting warnings.
- Do not use worktrees, only use feature branches when necessary. This is a single developer project and worktrees cause issues when running the game.

## Timing invariant (critical)

- **NTSC-only at 30 fps for presentation**; the audio VBL interrupt is 60 Hz.
- **Animation and AI advance on a 15 Hz gameplay tick** (one gameplay tick every two presented frames).

## Agent working rules

- **use context-mode to read large documents or perform semantic searches** Reference docs on the `research` branch are pre-indexed in context-mode — use `ctx_search` first, then fall back to `ctx_fetch_and_index` for content not yet in the index. `reference/README.md` on that branch is the entry point for the reference documentation.
- **Always follow `docs/GUIDELINES.md`** when writing, reviewing, or refactoring Rust code in this repository. If there is any conflict, stop and ask for clarification. **NEVER MAKE ASSUMPTIONS**
- Avoid unrelated refactors while touching gameplay-critical code.
- **Use test-driven development for substantial work**: start by writing tests that fail based on the relevant `docs/spec/` and `docs/reqs/` subsystem files, then implement the code against those tests. This is applicable to both new features and bug fixes.
- **Do not modify tests just to make them pass** unless there is a strong project reason, such as a change in the specification or requirements.
- Validate changed behavior with targeted commands/tests when feasible.
- **Do not invent player-facing strings.** Any message shown to the user must come from one of two authoritative sources: (1) `faery.toml` (`[narr]` tables such as `event_msg`, `speeches`, `place_msg`, `inside_msg`) via `crate::game::events`, or (2) the hardcoded string literals exhaustively enumerated in `reference/logic/dialog_system.md` on the research branch ("Hardcoded scroll messages — complete reference"). No other source of scroll-area text is permitted — never hardcode new narrative prose in Rust code. See `docs/spec/intro-narrative.md` §23.6 and `docs/reqs/intro-narrative.md` R-INTRO-012/013/014.

## Spec & requirements file map

Specifications for this implementation are in docs/spec/ and requirements are in docs/reqs/. Use the README.md in each directory for more information.

The specification for the debug TUI is in docs/DEBUG_SPECIFICATION.md.

## Reference documentation

Refer to docs/REFERENCE.md for information about the reference documentation.

## Canonical sources by topic

- Build/run commands and developer setup: `README.md` (repo root).
- Current Rust implementation details: source under `src/` (especially `src/main.rs` and `src/game/`).

## Game mechanics research order

When investigating any game mechanic (combat, movement, AI, timings, formulas, etc.):
1. **First**: the research is the authoritative source of truth for verified mechanics.
2. Search `reference/ARCHITECTURE.md` for subsystem structure/data flow and `reference/STORYLINE.md` for quest/scenario flow.
3. Use the relevant `docs/spec/` subsystem file (local) to resolve implementation details and keep the port internally consistent.
4. Do not create competing source-of-truth documents unless the user explicitly requests it.
5. If any information is missing, unclear, ambiguous or contradictory, stop immediately and ask for clarification.

## Commit Rules

- Do not commit without user consent. Always ask, never assume, even if permission has been given for other changes in a session.
- Follow the conventional commit format: `<type>(<scope>): <subject>`
- Use the following types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`
- Keep the subject line under 50 characters
- Use the imperative mood in the subject line
- Add a blank line before the body
- The body must list the changes made in the commit using **short and concise bullet points**
- Prefer short summarization over long rambling descriptions
- When creating a commit to fix a bug, add `Closes: #<issue>` on its own line at the end of the commit message (e.g. `Closes: #111`).
- **NEVER add Co-authored-by or "generated by" attributions** to commit messages
