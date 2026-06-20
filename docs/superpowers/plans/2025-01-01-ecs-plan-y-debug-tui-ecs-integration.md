---
title: "Plan Y — Debug TUI ECS Integration"
plan: Y
status: draft
depends_on: [X]
touches: [src/main.rs, src/game/debug_tui/bridge.rs, src/game/debug_tui/view.rs, src/game/debug_tui/stub.rs, src/game/ecs/resources.rs, src/game/ecs/systems]
---

# ECS Migration Plan Y: Debug TUI ECS Integration

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate the manual `DebugSnapshot` construction in `main.rs` and move the debug TUI data bridge into the ECS. Either remove the `DebugSnapshot` entirely or reduce it to a thin, read-only resource that the ECS populates and the console consumes. The aim is to stop copying data every frame when nothing has changed.

**Architecture:** After the rest of the ECS migration is complete, the debug TUI should read from the ECS world rather than a separately hand-built snapshot. The cleanest path is:

1. Add a `DebugSnapshot` resource to the ECS world (or, if the TUI layer stays external, an ECS system that builds it).
2. Run the build system only when the debug console is active and only when the underlying data is dirty.
3. `main.rs` reads the resource each frame and passes it to the console. The console no longer cares where the data came from.

**Prerequisites:** Plan X (Parity and Cleanup) complete. All game state that the debug TUI reads must be available through ECS components or resources.

**Tech Stack:** Rust 2021, hecs ECS, crossterm/ratatui TUI.

---

## File map

| File | Change |
|------|--------|
| `src/game/ecs/resources.rs` | Add `DebugSnapshot` as an ECS resource; add dirty flags or split high/medium/low frequency update resources. |
| `src/game/ecs/systems/debug_snapshot.rs` | New system that populates the snapshot from ECS queries. |
| `src/game/ecs/systems/mod.rs` | Register the debug snapshot system. |
| `src/game/debug_tui/bridge.rs` | Keep `DebugSnapshot` as the data contract; maybe simplify it by removing fields that are no longer needed. |
| `src/game/debug_tui/view.rs` | Update `DebugConsole` to read the snapshot from a resource or from `main.rs` (implementation choice). |
| `src/game/debug_tui/stub.rs` | Keep stub API unchanged. |
| `src/main.rs` | Remove hand-rolled snapshot building; replace with `build_ecs_snapshot_system` or a read from `ecs.res.debug_snapshot`. |

---

## Task 1: Decide the integration pattern

**Files:**
- Read: `src/main.rs` (debug snapshot build block)
- Read: `src/game/debug_tui/bridge.rs`
- Read: `src/game/ecs/resources.rs`

- [ ] **Step 1: Choose between two patterns**

  Option A — **ECS resource snapshot (recommended):**
  - Store a `DebugSnapshot` in `Resources`.
  - Add a `build_debug_snapshot` system that runs every frame but only rebuilds when dirty.
  - `main.rs` reads `ecs.res.debug_snapshot` and passes it to the console.
  - This keeps the current `DebugSnapshot` contract and minimizes TUI changes.

  Option B — **Tiered ECS resources:**
  - Replace `DebugSnapshot` with three resources: `DebugHighFreq`, `DebugMedium`, `DebugOnDemand`.
  - Systems update each resource at different cadences or on dirty flags.
  - The console holds cached copies and updates only the tiers that changed.
  - This is more efficient but changes the TUI contract more.

  Pick the option that matches the state of the ECS at the time of implementation.

- [ ] **Step 2: Define dirty conditions**

  For each data tier, decide what makes it dirty:
  - High freq: `game_ticks`, `paused`, `fps`, `tps` — always dirty every frame.
  - Medium: hero position, stats, inventory, actors, narrative — dirty when those components/resources change.
  - On demand: quest state, region, zone — dirty when the user opens a panel or when those resources change.

- [ ] **Step 3: Decide whether to keep the DebugSnapshot struct**

  If the TUI view already renders from `DebugSnapshot`, it may be cheapest to keep the struct as the single source for the view and just have an ECS system produce it. The long-term goal is to eliminate *manual* copying from `main.rs`, not necessarily to delete the struct.

---

## Task 2: Add DebugSnapshot resource and system

**Files:**
- Modify: `src/game/ecs/resources.rs`
- Create: `src/game/ecs/systems/debug_snapshot.rs`
- Modify: `src/game/ecs/systems/mod.rs`

- [ ] **Step 1: Add the resource**

  ```rust
  pub struct DebugSnapshotResource {
      pub snapshot: DebugSnapshot,
      pub dirty: DebugDirtyFlags,
  }
  ```

  Or add the fields directly to `Resources`:
  ```rust
  pub debug_snapshot: DebugSnapshot,
  pub debug_dirty: DebugDirtyFlags,
  ```

- [ ] **Step 2: Add dirty flags**

  ```rust
  #[derive(Debug, Default, Clone, Copy)]
  pub struct DebugDirtyFlags {
      pub high_freq: bool,
      pub medium: bool,
      pub on_demand: bool,
  }
  ```

  Systems that change the underlying data should set the relevant flag.

- [ ] **Step 3: Implement the build system**

  ```rust
  pub fn build_debug_snapshot(world: &mut World, res: &mut Resources) {
      if res.debug_console.is_none() {
          // No debug console active; skip all work.
          return;
      }
      if res.debug_dirty.high_freq {
          // Update high-frequency fields only.
      }
      if res.debug_dirty.medium {
          // Update medium-frequency fields.
      }
      if res.debug_dirty.on_demand {
          // Update on-demand fields.
      }
      res.debug_dirty = DebugDirtyFlags::default();
  }
  ```

  Move the existing logic from `main.rs` into this system.

- [ ] **Step 4: Register the system**

  Add the system to the ECS system schedule in `src/game/ecs/systems/mod.rs` or wherever systems are registered. Run it after gameplay systems but before rendering.

---

## Task 3: Update systems that write debug-related data

**Files:**
- Modify: relevant systems under `src/game/ecs/systems/`

- [ ] **Step 1: Identify all data sources**

  Find every system that changes data consumed by `DebugSnapshot`:
  - Hero position: `movement` system
  - Hero stats: `combat`, `item`, `carrier` systems
  - Quest state: `quest` or `narrative` systems
  - Narrative queue: `narrative` system
  - Clock/timers: `clock` system

- [ ] **Step 2: Set dirty flags when data changes**

  When a system mutates a component or resource that the snapshot reads, set the corresponding dirty flag. Prefer coarse flags over fine-grained ones to keep the change small.

  Example:
  ```rust
  res.debug_dirty.medium = true;
  ```

---

## Task 4: Simplify main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Remove the manual snapshot build block**

  Replace the large block that constructs `DebugSnapshot` from ECS queries with a single read:
  ```rust
  if let Some(snapshot) = ecs.res.debug_snapshot.take() {
      dc.update_status(snapshot);
  }
  ```

  Or, if the snapshot is cloned instead of taken:
  ```rust
  dc.update_status(ecs.res.debug_snapshot.clone());
  ```

- [ ] **Step 2: Keep the command/log plumbing**

  `main.rs` still drains commands from the console and applies them through `ecs::debug_commands::handle`. It also drains `diag_log` messages back to the console. These stay because they are not data-shipping, they are request/response plumbing.

- [ ] **Step 3: Ensure the stub path still compiles**

  The non-`debug-tui` build should still work. The stub does not need the snapshot resource.

---

## Task 5: Verify and optimize

**Files:**
- All touched files.

- [ ] **Step 1: Build with no warnings**

  ```bash
  cargo check
  cargo test
  ```

- [ ] **Step 2: Measure whether copying is reduced**

  If the snapshot is still cloned every frame, the only win is that the build moved into ECS. To reduce copying, either:
  - Keep the snapshot in `Resources` and have the console take ownership when it renders.
  - Split the snapshot into tiered resources and update only dirty tiers.

- [ ] **Step 3: Test with and without `--debug`**

  Ensure no regressions when the console is not active. Ensure the `debug-tui` feature still builds.

---

## Acceptance criteria

- [ ] `main.rs` no longer hand-builds `DebugSnapshot` from ECS queries.
- [ ] The debug TUI still shows all the same data it did before Plan Y.
- [ ] The snapshot is either an ECS resource or built by an ECS system.
- [ ] Dirty flags prevent unnecessary rebuilds when the console is active.
- [ ] `cargo check` and `cargo test` pass with no warnings.
- [ ] The non-`debug-tui` stub build still compiles and runs.
