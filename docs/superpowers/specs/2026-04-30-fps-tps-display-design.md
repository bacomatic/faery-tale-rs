# FPS / TPS Debug Display

**Goal:** Surface render FPS and game tick rate (TPS) in the debug TUI so we can confirm the game is running at the expected 30 Hz simulation rate and diagnose if movement is too fast due to tick accumulation.

## Background

The game clock is time-based at 30 Hz (NANOS_PER_TICK = 33_333_334 ns). `delta_ticks` — the number of 30 Hz ticks elapsed per render frame — gates all movement and game logic via `for _ in 0..delta_ticks`. If TPS drifts above 30, movement speed is proportionally too high.

`fps` (render frames/sec) is already tracked in `main.rs` and present in `DebugSnapshot`, but never rendered in the TUI. `tps` (game simulation ticks/sec) does not exist yet.

## Changes

### `bridge.rs` — DebugSnapshot
Add `tps: f64` field alongside existing `fps: f64`. Default = 0.0.

### `main.rs` — tick rate tracking
Mirror the existing frame-count pattern:
```
game_tick_count += delta_ticks as u64;   // incremented after the for-loop
// every second:
game_tps = game_tick_count as f64 / elapsed;
game_tick_count = 0;
```
Set `tps: game_tps` in all three `DebugSnapshot` construction sites (lines ~530, ~652, ~717).

### `view.rs` — render in Geography panel
Add one line to the Geography panel (currently 4 lines, has room):
```
FPS: 60.1  TPS: 30.0
```
Uses existing `styled_label` / `Span::raw` pattern.

## Success Criteria
- Debug TUI shows both FPS and TPS values, updating every second.
- At steady state on a 60 Hz display: FPS ≈ 60, TPS ≈ 30.
- If TPS > 30, the loop is running too many ticks (fix: cap delta_ticks).
- If TPS ≈ 30 but movement still feels fast, speed values need tuning.

## Files
- `src/main.rs`
- `src/game/debug_tui/bridge.rs`
- `src/game/debug_tui/view.rs`
