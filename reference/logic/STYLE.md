# Logic Documentation Style Guide (Normative)

This document is the normative grammar for pseudo-code blocks under `docs/logic/`. Every ` ```pseudo ` fenced block must conform. The linter (`tools/lint_logic.py`) enforces this grammar.

## Rationale

The logic tier exists so that the full documentation set — logic + [ARCHITECTURE](../ARCHITECTURE.md) + [RESEARCH](../RESEARCH.md) + [STORYLINE](../STORYLINE.md) + `world_db.json` + `quest_db.json` — is sufficient to re-implement *The Faery Tale Adventure* without consulting the 1987 source. The target audience is both human porters and AI coding agents: the format is readable as Markdown, and each fenced block is machine-parseable by the linter.

The fidelity target is **behavioral, not implementation**: same inputs produce the same observable gameplay. The grammar specifies *what* happens (rolls, thresholds, state transitions, ordering) and leaves primitives (RNG algorithm, fixed-point layout, integer widths when not observable) to the porter. Consequences:

- Integer widths are declared only when overflow or wrap is observable.
- Randomness is expressed via `rand(lo, hi)` / `chance(n, d)`; any uniform PRNG with adequate period is acceptable.
- Save-file byte layout **is** bit-exact — save compatibility is observable.
- Per-frame ordering **is** exact when it changes outcomes.
- Graphics, audio, and input-device APIs stay as prose in RESEARCH.md — any port will swap them for SDL or an equivalent.

The strict, normalized grammar below makes every file parseable by the same tool, makes cross-references mechanically checkable, and makes the pseudo-code a contract rather than a suggestion.

## 1. Per-file structure

Every logic doc MUST begin with:

```markdown
# <Subsystem Name> — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, fmain2.c
> Cross-refs: [RESEARCH §N](../RESEARCH.md#section-anchor)

## Overview

(1–2 paragraphs of prose.)

## Symbols

(Locals declared in this file; globals go in SYMBOLS.md.)
```

Then one H2 per documented function.

## 2. Function header (mandatory, outside the fence)

```markdown
## function_name

Source: `fmain.c:820-905`
Called by: `handle_menu`
Calls: `draw_menu`, `play_click`, `TABLE:menu_options`

```pseudo
def function_name(key: KeyCode, state: MenuState) -> MenuAction:
    """One-line purpose."""
    ...
```
```

- `Source:` — one or more backticked `file:line` or `file:start-end` citations, comma-separated.
- `Called by:` — function names that call this one, or `entry point` if none. Comma-separated.
- `Calls:` — function names, primitives, or `TABLE:name` references used in the body. Comma-separated, or `none`.
- Return annotation is required. Use `None` for void.
- A one-line docstring is required as the first statement.

## 3. Allowed statements

| Construct | Form |
|---|---|
| Assignment | `x = expr`, compound ops OK |
| Conditional | `if / elif / else` |
| Match | `match x: case LITERAL:` |
| Loop | `for x in iterable:`, `while cond:` with `break`/`continue` |
| Call | `name(args...)` |
| Return | `return expr` / `return` |

## 4. Forbidden constructs

- `try`, `raise`, `with` — use explicit error-state returns.
- Comprehensions (list/set/dict/generator) — be explicit with loops.
- `lambda`, closures.
- `class` — data shapes go in SYMBOLS.md.
- `import`, `from ... import`.
- `global`, `nonlocal` — globals are referenced by name and must be registered in SYMBOLS.md.

## 5. Primitives (the pseudo-code stdlib)

Usable without declaration:

| Primitive | Semantics |
|---|---|
| `rand(lo, hi)` | Uniform int in `[lo, hi]` inclusive |
| `chance(n, d)` | True with probability `n/d` |
| `min(a, b)`, `max(a, b)` | Standard |
| `clamp(x, lo, hi)` | `max(lo, min(x, hi))` |
| `abs(x)`, `sign(x)` | Standard |
| `bit(n)` | `1 << n` — for flag bit positions |
| `wrap_u8(x)`, `wrap_i16(x)`, `wrap_u16(x)` | Explicit wrap; use only when observable |
| `now_ticks()` | Monotonic game tick counter |
| `speak(N)` | Display narr.asm message N |
| `play_sound(id)`, `play_music(id)` | Audio triggers |

## 6. Data types & naming

- **Enums** are UPPER_SNAKE constants in SYMBOLS.md: `DIR_N`, `GOAL_WANDER`, `STATE_WALKING`.
- **Structs** are dataclass-style declarations in SYMBOLS.md: `Shape`, `Missile`, `SaveRecord`.
- **Table refs** use the form `TABLE:name` and must appear in SYMBOLS.md's table registry.
- Field access uses `.`: `actor.vitality`.
- Bitfield flags: named bit constants registered in SYMBOLS.md.

## 7. Numeric literals

Literal integers other than `{-1, 0, 1, 2}` must either:
- Be a named constant declared in SYMBOLS.md, OR
- Carry an inline comment on the same line explaining the meaning, e.g.:

```pseudo
if actor.vitality < 25:                # fmain.c:1842 — low-HP flee threshold
    actor.goal = GOAL_FLEE
```

## 8. Inline citations

Any line whose behavior isn't obvious from the function header's source range SHOULD carry an inline `# file.ext:NNN` comment.

## 9. State machines

State-machine functions are written as `match` on the current state variable with one `case` per state. Transitions are explicit assignments (`actor.goal = GOAL_FLEE`). A companion Mermaid `stateDiagram-v2` block MAY follow the function; when present, every `STATE_*` assignment in the pseudo-code MUST appear as a node in the diagram.

## 10. Tick ordering

Where per-frame ordering matters, `docs/logic/game-loop.md` declares the canonical phase sequence. Other docs reference phase numbers (e.g., "runs in phase 3").

## 11. File-level checklist (enforced by linter)

- [ ] File starts with `# <Title> — Logic Spec`.
- [ ] Header block with `> Fidelity:` and `> Cross-refs:` lines.
- [ ] `## Overview` and `## Symbols` sections present.
- [ ] Every `## <Name>` (other than `Overview`/`Symbols`/`Notes`/`Mermaid`) is a function entry with the full header + pseudo block.
- [ ] Every function appears in `docs/logic/README.md` index.
