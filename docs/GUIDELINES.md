# Rust Guidelines for Coding Agents

> **Source:** Microsoft, *Pragmatic Rust Guidelines* — AI agents reference
> Full upstream text: <https://microsoft.github.io/rust-guidelines/agents/all.txt>
>
> This file is a **repo-local adoption and condensed summary** for day-to-day work in this codebase.

## Mandatory directive

All coding agents working in this repository **must follow these guidelines** when designing, editing, reviewing, or refactoring Rust code.

When guidance conflicts, use this precedence order:

1. `AGENTS.md`
2. The project reference docs in `reference/`
3. This file
4. General upstream Rust defaults

---

## Core rules

### 1. Prefer idiomatic Rust

- Use standard Rust naming and API conventions.
- Favor clear, concrete types and strong domain types over loosely documented primitives.
- Keep public interfaces simple and unsurprising.

### 2. Document behavior clearly

- Add meaningful docs for modules and important public items.
- Keep the first doc sentence short and scannable.
- Include examples where they materially help usage.
- Document `# Errors`, `# Panics`, and `# Safety` sections when applicable.

### 3. Avoid `unsafe` unless truly necessary

- `unsafe` must have a real justification: FFI, a proven performance need, or a well-designed low-level abstraction.
- Every `unsafe` use must include plain-language safety reasoning.
- Unsound code is never acceptable.

### 4. Use the right error model

- Return `Result` for recoverable failures.
- Reserve panics for programming bugs and violated invariants, not ordinary runtime conditions.
- Keep application-level error handling consistent rather than mixing many styles.

### 5. Use test-driven development

- New feature work should begin with tests that **fail first**.
- Base those tests on the repository specification and requirements documents.
- Implement production code against those tests until the behavior passes.
- Do not rewrite or weaken tests merely to obtain a passing result unless the specification or requirements have genuinely changed.

### 6. Design for testability and verification

- Make behavior easy to exercise with unit tests.
- Prefer mockable boundaries around I/O, clocks, randomness, and other external effects when appropriate.
- Verify changes with the normal Rust toolchain (`cargo test`, `cargo fmt`, `cargo clippy`) when relevant.

### 7. Keep APIs ergonomic

- Prefer inherent methods for core functionality.
- Use builders for complex initialization.
- Accept `impl AsRef<T>` or range traits where that improves ergonomics.
- Avoid exposing implementation-detail wrappers like `Arc<Mutex<T>>` in public APIs unless the wrapper is fundamental to the abstraction.

### 8. Profile before optimizing

- Measure hot paths before making performance changes.
- Prefer throughput-oriented, cache-friendly designs in hot code.
- Add cooperative yield points in long-running async work when needed.

### 9. Use structured, safe logging

- Prefer structured events over ad-hoc formatted strings.
- Name important events consistently.
- Never log secrets or sensitive user data in plain text.

### 10. Keep names and constants intentional

- Avoid vague type names such as `Manager`, `Service`, or `Factory` unless they are genuinely descriptive.
- Replace unexplained magic numbers with named constants or comments explaining why the value matters.

---

## Upstream references worth following

The Microsoft guidance explicitly builds on the broader Rust ecosystem norms. Agents should also align with:

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html)
- [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
- [Rust Reference: Undefined Behavior](https://doc.rust-lang.org/reference/behavior-considered-undefined.html)

---

## Practical use in this repository

For this project in particular:

- favor fidelity and documented behavior over clever rewrites,
- prefer minimal, surgical changes,
- keep magic values explained when they encode original game behavior,
- avoid speculative abstractions that make gameplay code harder to compare against the specification,
- and validate behavior with focused tests or checks whenever practical.
