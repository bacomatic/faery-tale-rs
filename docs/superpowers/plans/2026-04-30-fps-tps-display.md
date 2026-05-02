# FPS / TPS Debug Display Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Display render FPS and game simulation TPS in the debug TUI so we can confirm the game runs at ~30 Hz and diagnose movement speed issues.

**Architecture:** Add `tps: f64` to `DebugSnapshot`, track game ticks per second in `main.rs` alongside the existing frame counter, and render both values in the Geography panel of the debug TUI.

**Tech Stack:** Rust, SDL2, ratatui (debug-tui feature)

---

## File Map

| File | Change |
|------|--------|
| `src/game/debug_tui/bridge.rs` | Add `tps: f64` field to `DebugSnapshot` (line ~32) |
| `src/main.rs` | Track `game_tick_count`/`game_tps`; set `tps` in 3 snapshot sites |
| `src/game/debug_tui/view.rs` | Add FPS/TPS line to Geography panel (line ~482) |

---

### Task 1: Add `tps` field to `DebugSnapshot`

**Files:**
- Modify: `src/game/debug_tui/bridge.rs:32`

- [ ] **Step 1: Add the field**

Open `src/game/debug_tui/bridge.rs`. After line 32 (`pub fps: f64,`), add:

```rust
    pub fps: f64,
    pub tps: f64,
```

`DebugSnapshot` derives `Default` (line 30), so `tps` defaults to `0.0` automatically. No other changes needed in this file.

- [ ] **Step 2: Verify it compiles**

```bash
cargo build 2>&1 | grep -E 'error|warning.*tps'
```

Expected: compile succeeds (there may be an "unused field" warning — that's fine for now, it's used in the next task).

---

### Task 2: Track game TPS in `main.rs`

**Files:**
- Modify: `src/main.rs`

The existing FPS tracking pattern (lines ~229–253) counts render frames per second. Mirror it to count game ticks per second.

- [ ] **Step 1: Add tick counters near the FPS variables (line ~231)**

Find this block (around line 229):

```rust
    let mut game_frame_count: u64 = 0;
    let mut game_fps_time = std::time::Instant::now();
    let mut game_fps: f64 = 0.0;
```

Add two lines immediately after:

```rust
    let mut game_frame_count: u64 = 0;
    let mut game_fps_time = std::time::Instant::now();
    let mut game_fps: f64 = 0.0;
    let mut game_tick_count: u64 = 0;
    let mut game_tps: f64 = 0.0;
```

- [ ] **Step 2: Accumulate ticks each frame (line ~248)**

Find the existing frame counter increment (around line 248):

```rust
        game_frame_count += 1;
        let fps_elapsed = game_fps_time.elapsed().as_secs_f64();
        if fps_elapsed >= 1.0 {
            game_fps = game_frame_count as f64 / fps_elapsed;
            game_frame_count = 0;
            game_fps_time = std::time::Instant::now();
        }
```

Replace with:

```rust
        game_frame_count += 1;
        game_tick_count += delta_ticks as u64;
        let fps_elapsed = game_fps_time.elapsed().as_secs_f64();
        if fps_elapsed >= 1.0 {
            game_fps = game_frame_count as f64 / fps_elapsed;
            game_tps = game_tick_count as f64 / fps_elapsed;
            game_frame_count = 0;
            game_tick_count = 0;
            game_fps_time = std::time::Instant::now();
        }
```

- [ ] **Step 3: Set `tps` in all three `DebugSnapshot` construction sites**

There are three sites in `main.rs` that build a `DebugSnapshot` (around lines 529, 651, 716). Each already sets `fps: game_fps`. Add `tps: game_tps,` on the line immediately after each `fps:` line:

Site 1 (~line 530):
```rust
                let status = DebugSnapshot {
                    fps: game_fps,
                    tps: game_tps,
                    // ... rest unchanged
```

Site 2 (~line 652):
```rust
                let status = DebugSnapshot {
                    fps: game_fps,
                    tps: game_tps,
                    // ... rest unchanged
```

Site 3 (~line 717):
```rust
            let status = DebugSnapshot {
                fps: game_fps,
                tps: game_tps,
                // ... rest unchanged
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo build 2>&1 | grep error
```

Expected: no errors.

---

### Task 3: Display FPS and TPS in the debug TUI Geography panel

**Files:**
- Modify: `src/game/debug_tui/view.rs`

The Geography panel (around line 463) currently has 4 lines: Pos, Rgn/Ext, Env, Carrier. Add a 5th line for FPS/TPS.

- [ ] **Step 1: Add the FPS/TPS line to the Geography panel**

Find the `geo_text` vec (around line 463). It ends with:

```rust
                Line::from(vec![
                    styled_label("Carrier: "),
                    Span::raw(status.active_carrier_name.clone()),
                ]),
```

Add one more line after it, before the closing `];`:

```rust
                Line::from(vec![
                    styled_label("Carrier: "),
                    Span::raw(status.active_carrier_name.clone()),
                ]),
                Line::from(vec![
                    styled_label("FPS:"),
                    Span::raw(format!("{:.1}  ", status.fps)),
                    styled_label("TPS:"),
                    Span::raw(format!("{:.1}", status.tps)),
                ]),
```

- [ ] **Step 2: Build and run to verify display**

```bash
cargo build 2>&1 | grep error
cargo run -- --debug --skip-intro
```

Open the debug console. The Geography panel should now show a line like:
```
FPS:60.0  TPS:30.0
```

Confirm values look reasonable (FPS matches your monitor refresh ÷ vsync behaviour; TPS should be ~30.0).

- [ ] **Step 3: Commit**

```bash
git add src/game/debug_tui/bridge.rs src/main.rs src/game/debug_tui/view.rs
git commit -m "feat: display FPS and TPS in debug TUI Geography panel"
```

---

## Expected Outcomes

| Scenario | FPS | TPS |
|----------|-----|-----|
| 60 Hz display, normal | ~60 | ~30 |
| 144 Hz display, normal | ~144 | ~30 |
| Slow rendering (lag) | < refresh | > 30 (ticks accumulating) |

If TPS is consistently above 30, `delta_ticks` accumulates and movement is proportionally faster than intended. The fix in that case is `let delta_ticks = raw_delta.min(1);` in `main.rs`.
