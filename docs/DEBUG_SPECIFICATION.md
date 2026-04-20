# Debug TUI Specification

A terminal-based debugging interface for the Faery Tale Adventure Rust port, activated via `--debug` CLI flag.

## Overview

When the game is launched with `--debug`, the launching terminal becomes a ratatui-based TUI displaying live game state, a filterable debug log, and an interactive command prompt. The SDL game window opens separately alongside it. The TUI communicates with the game loop via in-process `mpsc` channels.

## Architecture

Three modules under `src/game/debug_tui/`:

### bridge.rs

The interface between the game loop and the TUI thread. Defines:

- **`DebugSnapshot`** — a `Clone` struct containing all displayable game state, built by the game loop and sent over a channel.
- **`DebugCommand`** — an enum of all commands the TUI can send back to the game.
- **`DebugBridge`** — holds channel endpoints: `Sender<DebugSnapshot>` (game→TUI) and `Receiver<DebugCommand>` (TUI→game). Exposes `snapshot()` to build and send, and `poll_commands()` to drain incoming commands.
- **`DebugLogEntry`** — struct with timestamp, category, severity, and message text.
- **`LogCategory`** — enum for log filtering.

The game loop's contract: call `bridge.snapshot(game_state)` once per frame to build and send a snapshot, and call `bridge.poll_commands()` to drain and apply incoming commands.

### commands.rs

Parses command strings into `DebugCommand` variants. Independently testable without a terminal. Handles all inspection, mutation, actor watch, log filter, and flow control commands.

### view.rs

Owns the ratatui `Terminal`, renders the layout, and handles keyboard input. Runs on a dedicated thread with its own event loop.

## DebugSnapshot Data Model

All fields use owned, `Clone`-friendly types. Derived values (max vitality, time period, extent index) are computed on the game side before sending.

```rust
DebugSnapshot {
    // Hero panel (General Information)
    hero_name: String,              // "Julian", "Phillip", or "Kevin"
    brother_index: u8,              // 0, 1, or 2
    scene_name: String,             // current scene type ("Gameplay", "Intro", "Placard", etc.)
    vitality: i8,
    max_vitality: i8,               // 15 + brave/4
    brave: u8,
    luck: u8,
    kind: u8,
    wealth: u16,
    hunger: u8,
    fatigue: u8,
    weapon: u8,                     // 0=none, 1=dirk, 2=mace, 3=sword, 4=bow, 5=wand
    hero_state: u8,                 // WALKING, FIGHTING, DEAD, etc.
    hero_facing: u8,                // 0–7 compass direction

    // Geography panel
    map_x: u16,                     // 0–32767 world pixel coordinate
    map_y: u16,                     // 0–32767 world pixel coordinate
    region_num: u8,                 // 0–9
    region_loading: bool,           // MAP_FLUX flag
    extent_index: Option<u8>,       // which extent zone hero is in (0–22), None if none
    extent_type: Option<u8>,        // etype of matched extent
    environ: i8,                    // terrain depth (-2=flying, 0=normal, 1-15=water, >15=fire)
    carrier_state: u8,              // 0=none, 1=raft, 5=turtle, 11=bird

    // Visual effects panel
    daynight: u16,                  // 0–23999 tick counter
    lightlevel: u16,                // 0–300 triangle wave
    time_period: String,            // "Night", "Morning", "Midday", "Evening"
    light_timer: u16,               // Green Jewel spell remaining
    secret_timer: u16,              // Bird Totem spell remaining
    freeze_timer: u16,              // Gold Ring time-stop remaining
    is_paused: bool,                // game loop paused by debug command

    // Actor slots (for `/actors` command and `/watch` feature)
    actors: Vec<ActorSnapshot>,     // up to 20 active slots

    // Quest state (for `/quest` command)
    princess_captive: bool,
    princess_rescues: u16,
    statues_collected: u8,
    has_writ: bool,
    has_talisman: bool,

    // Encounter state
    encounter_number: u8,           // pending spawns
    encounter_type: u8,             // current encounter race
    active_enemy_count: u8,         // anix

    // Log entries added since last snapshot
    new_log_entries: Vec<DebugLogEntry>,
}

ActorSnapshot {
    slot: u8,                       // 0–19
    actor_type: u8,                 // PHIL/ENEMY/RAFT/SETFIG/CARRIER/etc.
    state: u8,
    facing: u8,
    abs_x: u16,
    abs_y: u16,
    vitality: i8,
    weapon: u8,
    race: u8,
    goal: u8,
    tactic: u8,
    environ: i8,
    visible: bool,
}
```

## TUI Layout

Minimum terminal size: 80×24. Uses ratatui constraint-based layout.

**Collapsed (default):**

```
┌─ Hero Stats ──────────┬─ Geography ────────────┬─ Visual Effects ────────┐
│ Julian    HP: 12/15   │ Pos: 5230, 12400       │ Time: 8342  Morning    │
│ B:22 L:18 K:8  W:150  │ Rgn: 3  Ext: 7 (rand)  │ Light: 208             │
│ Hgr:45 Fat:20 Wpn:Swd │ Env: 0 (normal)        │ Jewel:0 Totem:0 Frz:0 │
│ State: WALKING  F:NE  │ Carrier: none          │ [PAUSED]               │
├─ Actors [▶]  Raft:(5230,12400)  Msls:0 Items:2 ────────────────────────┤
├─ Debug Log ─────────────────────────────────────────────────────────────┤
│ [12:08] ENCOUNTER  Spawned 3x Ogre at (5200,12380) in extent 7        │
│ ...                                                                     │
├─ Command ───────────────────────────────────────────────────────────────┤
│ > _                                                                     │
└─────────────────────────────────────────────────────────────────────────┘
```

**Expanded (Ctrl+W toggles):**

```
┌─ Hero Stats ──────────┬─ Geography ────────────┬─ Visual Effects ────────┐
│ Julian    HP: 12/15   │ Pos: 5230, 12400       │ Time: 8342  Morning    │
│ B:22 L:18 K:8  W:150  │ Rgn: 3  Ext: 7 (rand)  │ Light: 208             │
│ Hgr:45 Fat:20 Wpn:Swd │ Env: 0 (normal)        │ Jewel:0 Totem:0 Frz:0 │
│ State: WALKING  F:NE  │ Carrier: none          │ [PAUSED]               │
├─ Actors [▼]  Raft:(5230,12400)  Msls:0 Items:2 ────────────────────────┤
│ #2 SETFIG  Wizard  TALKING  HP:—  (5100,12200)                         │
│ #3 ENEMY   Ogre    FIGHT    HP:12  goal:SEEK  tac:PURSUE  (5180,12350) │
│ #4 ENEMY   Orc     WALK     HP:8   goal:SEEK  tac:RANDOM  (5300,12500) │
│ #5 —                                                                    │
│ #6 —                                                                    │
├─ Debug Log ─────────────────────────────────────────────────────────────┤
│ [12:08] ENCOUNTER  Spawned 3x Ogre at (5200,12380) in extent 7        │
│ ...                                                                     │
├─ Command ───────────────────────────────────────────────────────────────┤
│ > _                                                                     │
└─────────────────────────────────────────────────────────────────────────┘
```

### Layout Rules

- **Top row:** 3 columns, equal width (33% each). Fixed height: 4 lines.
- **Actor watch:** Always visible between the top row and the debug log. Has two display modes:
  - **Collapsed (default):** 1 line showing a summary header. Contains: expand indicator `[▶]`, raft coordinates, missile count, and ground item count. Slot 0 (hero) is always omitted — hero state is already in the top row.
  - **Expanded:** 1 summary line (with `[▼]` indicator) plus up to 5 detail rows (slots 2–6). Ctrl+W toggles between collapsed and expanded.
- **Debug log:** Fills all remaining vertical space. Scrollable via PgUp/PgDn. Auto-scrolls to bottom on new entries.
- **Command prompt:** Fixed height: 1 line. Always visible at bottom. Prefix `> `.
- **Pause indicator:** `[PAUSED]` shown in the Visual Effects panel when game is paused.

### Actor Watch Panel Contents

**Summary line (always shown):**

The summary line is the panel's title bar. It contains:

- **Expand/collapse indicator:** `[▶]` when collapsed, `[▼]` when expanded.
- **Raft (slot 1):** `Raft:(x,y)` when the raft actor is active (type=RAFT, visible), or `Raft:—` when inactive.
- **Missiles & items (slots 7–19):** Two separate counts: `Msls:N` for projectile actors and `Items:N` for ground object actors. Counts only visible, active entries.

**Expanded detail rows (shown when expanded):**

One row per slot from 2 through 6, always in slot order:

- **Slot 2 — Setfig/Witch:** `#2 SETFIG <race_name> <state> HP:<vit> (<x>,<y>)` when active, or `#2 —` when empty/inactive.
- **Slots 3–6 — Enemies/Carriers:** `#N <type> <race_name> <state> HP:<vit> goal:<goal> tac:<tactic> (<x>,<y>)` when active, or `#N —` when empty. Type is `ENEMY`, `CARRIER`, etc. Carrier slots show carrier-specific labels (turtle, bird, dragon) as the race name.

Empty slots always display as `#N —` to maintain stable row positions. This prevents the layout from jumping when actors spawn or despawn.

### Top Row Panel Contents

**Hero Stats (left):**
- Hero name and HP/max HP
- Brave, Luck, Kind, Wealth (abbreviated)
- Hunger, Fatigue, equipped Weapon name
- Actor state name, Facing direction

**Geography (center):**
- World coordinates (map_x, map_y)
- Region number, Extent index and type (if matched)
- Environ value with label (normal/water/fire/flying)
- Carrier state

**Visual Effects (right):**
- Daynight tick counter and time period name
- Light level (0–300)
- Spell timers: Green Jewel, Bird Totem, Gold Ring (0 when inactive)
- `[PAUSED]` indicator when applicable

## Command System

All commands are typed at the `> ` prompt and executed on Enter. Commands are case-insensitive, whitespace-trimmed, and **must be prefixed with `/`** (e.g. `/stats`, `/tp 200 150`). Any input that does not begin with `/` is ignored with an "Unknown command" message so the prompt cannot be mistaken for a chat line.

### Inspection Commands

| Command | Output |
|---------|--------|
| `/stats` | Full hero stat dump to log (all fields from Hero panel + computed values) |
| `/actors` | Table of all active actor slots: slot, type, race, state, goal, tactic, HP, position |
| `/quest` | Quest progress: princess state, statue count, writ, talisman, key inventory |
| `/inventory` | Full stuff[35] array grouped by category (weapons, magic, keys, consumables) |
| `/timers` | All active timers: daynight, light_timer, secret_timer, freeze_timer, hunger, fatigue |
| `/doors` | Door state: locked door list, keys held, recently activated door |
| `/extent` | Current extent zone: matched index, etype, parameters (v1/v2/v3) |
| `/terrain` | Dump terrain lookup chain under the hero's feet (collision debug) |
| `/adf <block> [count]` | Hex-dump one or more ADF data blocks to the log |
| `/help` | List all commands with brief descriptions |
| `/help <cmd>` | Detailed help for one command with usage examples (prefix optional in the argument) |

### Mutation Commands

| Command | Effect | Validation |
|---------|--------|------------|
| `/stat <name> <n>` | Set or adjust a hero stat. Names: `vit`, `brv`, `lck`, `knd`, `wlt`, `hgr`, `ftg`. Values accept `+N` / `-N` for relative adjustments. | Each stat clamped to its documented range (vitality to 0–max, others to 0–255 or u16 for wealth) |
| `/inv <slot> <n>` | Set or adjust inventory slot 0–34. `+N`/`-N` for relative. | Slot range 0–34; value clamped to slot's max |
| `/give <item>` | Add item to inventory by name | See Item Name Reference |
| `/take <item>` | Remove item from inventory by name | Same names as `/give` |
| `/tp <x> <y>` | Teleport hero to world coordinates | 0–32767 each |
| `/tp safe` | Teleport hero to nearest safe (non-water, non-fire) tile | — |
| `/tp ring <N>` | Teleport hero to the ring of stones in region N | N = 0–9 |
| `/region <n>` | Change to region | 0–9 |
| `/max` | Max all stats (vitality, brave, luck, kind, wealth; zero hunger/fatigue) | Shortcut |
| `/heal` | Set vitality to max_vitality; zero hunger and fatigue | Shortcut |
| `/die` | Set hero vitality to 0 (triggers death) | — |
| `/kill` | Kill every active hostile NPC on screen in one pass. Only enemy combatants (Enemy and Dragon actors) are affected; setfigs, carriers, swans, horses, and other non-combatant NPCs are left alone. | — |
| `/kill <slot>` | Set one actor's vitality to 0 | Slot 1–19 (not hero) |
| `/brother <name>` | Switch active brother | `julian` / `phillip` / `kevin` (or 0 / 1 / 2) |
| `/cheat` | Toggle `cheat1` debug-key mode on/off | Reports new state; see SPEC §25.9 |
| `/cheat on` / `/cheat off` | Explicitly set `cheat1` state | — |
| `/pack` | Batch equip (see below) | — |

#### /pack Command

Grants a full test loadout in one command:

| Inventory Slot | Item | Value |
|----------------|------|-------|
| stuff[0] | Dirk | 1 |
| stuff[1] | Mace | 1 |
| stuff[2] | Sword | 1 |
| stuff[3] | Bow | 1 |
| stuff[4] | Magic Wand | 1 |
| stuff[5] | Golden Lasso | 1 |
| stuff[6] | Sea Shell | 1 |
| stuff[7] | Sun Stone | 1 |
| stuff[8] | Arrows | 255 (full quiver) |
| stuff[9] | Blue Stone | 3 |
| stuff[10] | Green Jewel | 3 |
| stuff[11] | Glass Vial | 3 |
| stuff[12] | Crystal Orb | 3 |
| stuff[13] | Bird Totem | 3 |
| stuff[14] | Gold Ring | 3 |
| stuff[15] | Jade Skull | 3 |
| stuff[16] | Gold Key | 5 |
| stuff[17] | Green Key | 5 |
| stuff[18] | Blue Key | 5 |
| stuff[19] | Red Key | 5 |
| stuff[20] | Grey Key | 5 |
| stuff[21] | White Key | 5 |
| stuff[23] | Rose | 1 |
| stuff[24] | Apple | 255 |

Also calls `/heal` (max vitality, zero hunger/fatigue). Each item granted is logged individually.

### Magic, Carrier, and Time Commands

These are modern debug affordances not present in the original game; they expose internal state directly so the TUI can exercise subsystems without scripted setup.

| Command | Effect |
|---------|--------|
| `/god` | Toggle all god-mode flags on/off |
| `/god <flag>` | Toggle a specific flag: `noclip`, `invincible`, `ohk` (one-hit kill), `reach` (infinite weapon reach), `all`, `off` |
| `/noclip` | Shortcut for `/god noclip` |
| `/magic <effect>` | Sticky-enable a magic effect that otherwise times out: `light` (Green Jewel), `secret` (Bird Totem), `freeze` (Gold Ring). Toggles off when applied a second time. |
| `/swan` | Summon the swan carrier at the hero's position |
| `/time <HH:MM>` | Jump the daynight clock to a specific time |
| `/time <period>` | Jump to a named time period: `dawn`, `noon`, `dusk`, `midnight` |
| `/time hold` | Freeze the daynight clock (time-of-day stops advancing; gameplay continues) |
| `/time free` | Resume the daynight clock |
| `/save <on\|off>` | Enable or disable autosave |
| `/fx <effect>` | Trigger a one-shot visual effect: `witch`, `teleport`, `fadeout`, `fadein` |

Note: `/time hold` only freezes the day-night tick counter. It does **not** pause the game loop or actor updates — for that, see the Flow Control commands below.

### Encounter & Item Spawning Commands

Helpers for testing combat, AI, and item behavior without walking to a spawn zone.

| Command | Effect |
|---------|--------|
| `/encounter` | Force a regional encounter of four enemies of the region's default type |
| `/encounter <type>` | Spawn one enemy of the named type: `orc`, `ghost`, `skeleton`, `wraith`, `dragon`, `snake`, `swan`, `horse` |
| `/encounter clear` | Deactivate all active NPCs (enemies, setfigs, carriers) |
| `/items` | Scatter all 30 safe items around the hero (excludes talisman) |
| `/items <count>` | Scatter N random items (no talisman) |
| `/items <name\|id>` | Drop one item by name or inventory index (0–30). `/items talisman` is the only way to drop the talisman (which ends the game on pickup). |
| `/items <count> <name>` | Drop N of a named item |

### Music Commands

| Command | Effect |
|---------|--------|
| `/songs` | List available song groups |
| `/songs play <N>` | Play song index N |
| `/songs stop` | Stop music |
| `/songs cave <on\|off>` | Toggle cave music overlay |

### Actor Watch Commands

| Command | Effect |
|---------|--------|
| `/watch` | Toggle the actor watch panel between collapsed and expanded (same as Ctrl+W) |

The actor watch panel is always visible. In collapsed mode it shows a one-line summary of raft position, missile count, and item count. In expanded mode it additionally shows detail rows for slots 2–6 (setfig and enemies/carriers). Updates at the same 5 Hz rate as the status panels.

### Log Filter Commands

| Command | Effect |
|---------|--------|
| `/filter` | Open interactive filter: shows all categories with [ON]/[OFF], Tab cycles, Space toggles, Enter confirms |
| `/filter +cat -cat` | Inline toggle: enable/disable specific categories (e.g. `/filter +combat -ai`) |
| `/filter reset` | Reset to defaults (noisy categories off) |
| `/filter all` | Enable all categories |
| `/filter none` | Disable all categories |

### Flow Control Commands

| Command | Effect | Shortcut |
|---------|--------|----------|
| `/pause` | Freeze game loop (all actor and physics updates; daynight clock continues unless also held with `/time hold`) | Ctrl+P |
| `/resume` | Unfreeze game loop | Ctrl+P |
| `/step` | Advance exactly 1 frame (while paused) | — |
| `/step <n>` | Advance n frames (while paused) | — |
| `/clear` (aliases: `/cls`) | Clear the log buffer | — |

## Log Categories

| Category | Default | Covers |
|----------|---------|--------|
| COMBAT | ON | Hits, misses, damage, push-back, death |
| ENCOUNTER | ON | Spawn rolls, danger level, placement |
| QUEST | ON | Flag changes, item pickups, quest triggers |
| NPC | ON | Dialogue, set figure spawns, speech index |
| DOOR | ON | Activations, lock checks, key usage |
| CARRIER | ON | Mount, dismount, carrier spawns |
| MAGIC | ON | Spell activations, timer starts/expirations |
| GENERAL | ON | Region loads, scene changes, save/load |
| MOVEMENT | OFF | Per-frame hero position changes, collision checks |
| AI | OFF | Goal/tactic changes, set_course decisions |
| RENDERING | OFF | Palette fades, sprite z-sort, frame counts |
| ANIMATION | OFF | State transitions, frame index changes |
| TIME | OFF | Daynight tick, hunger/fatigue increments |

Categories default OFF are high-frequency ("noisy") and would overwhelm the log during normal gameplay.

## Keyboard Shortcuts

Always active regardless of command prompt focus:

| Shortcut | Action |
|----------|--------|
| Ctrl+P | Toggle pause/resume |
| Ctrl+W | Toggle actor watch panel expanded/collapsed |
| PgUp/PgDn | Scroll debug log |
| Tab | Cycle through filter categories (in interactive filter mode) |
| Space | Toggle selected category (in interactive filter mode) |
| Enter | Execute command / confirm filter selection |
| Ctrl+C | Quit TUI and game |

All other input goes to the command prompt.

## Threading & Data Flow

### Startup

1. Main parses `--debug` from CLI args.
2. Creates channel pair: bounded `mpsc::channel::<DebugSnapshot>(2)` and `mpsc::channel::<DebugCommand>()`.
3. Spawns TUI thread via `std::thread::spawn`, passing snapshot receiver and command sender.
4. TUI thread enters raw mode, initializes ratatui `Terminal`, starts its event loop.
5. Game loop holds the channel endpoints wrapped in `Option<DebugBridge>` — `None` when `--debug` is not passed.

### Per-Frame Game Side

1. If bridge is active: build `DebugSnapshot` from current game state.
2. Send snapshot via `try_send`; if TUI is behind, drop the frame (bounded channel of 2).
3. Drain all pending `DebugCommand`s and apply them (set stats, teleport, pause, etc.).
4. If pause is active: enter a tight poll loop — keep draining commands and sending snapshots but skip game logic until `/resume` or `/step`.

### TUI Thread Event Loop

- Poll terminal events at 20ms intervals via crossterm.
- On key event: route to command prompt input buffer or handle shortcuts.
- On Enter: parse command string via `commands.rs`, send `DebugCommand` over channel.
- Check for new `DebugSnapshot` via non-blocking `try_recv`, take latest, discard intermediate.
- Every 200ms (~5 Hz): render full TUI frame from latest snapshot, log buffer, and actor watch state.

### Log Entry Flow

1. Game code calls `debug_log!(category, "message {}", args)` — a macro that is a no-op when `--debug` is not active.
2. The macro appends a `DebugLogEntry` to a thread-local buffer.
3. During snapshot building, the buffer is drained into `snapshot.new_log_entries`.
4. TUI thread appends received entries to its ring buffer (capped at 10,000 entries) and applies category filter for display.

### Shutdown

- Game drops the snapshot sender on exit.
- TUI thread detects channel close, restores terminal to normal mode, exits cleanly.

## Feature Gating

- `Cargo.toml` adds a `debug-tui` feature with `ratatui` and `crossterm` as optional dependencies.
- All TUI code is behind `#[cfg(feature = "debug-tui")]`.
- The `--debug` CLI flag is always accepted. If the feature is not compiled in, a warning is printed and the game continues without the TUI.

## Dependencies

| Crate | Purpose | Feature-gated |
|-------|---------|---------------|
| `ratatui` | Terminal UI framework | Yes (`debug-tui`) |
| `crossterm` | Terminal backend for ratatui | Yes (`debug-tui`) |

No other new dependencies required. Channel communication uses `std::sync::mpsc`.

## Item Name Reference

Valid item names for the `/give`, `/take`, and `/items` commands, mapped to inventory indices:

| Name | stuff[] Index | Notes |
|------|---------------|-------|
| `dirk` | 0 | |
| `mace` | 1 | |
| `sword` | 2 | |
| `bow` | 3 | |
| `wand` | 4 | Magic Wand |
| `lasso` | 5 | Golden Lasso |
| `shell` | 6 | Sea Shell |
| `sunstone` | 7 | Sun Stone |
| `arrows` | 8 | Sets count (default 255) |
| `blue_stone` | 9 | |
| `green_jewel` | 10 | |
| `glass_vial` | 11 | |
| `crystal_orb` | 12 | |
| `bird_totem` | 13 | |
| `gold_ring` | 14 | |
| `jade_skull` | 15 | |
| `gold_key` | 16 | |
| `green_key` | 17 | |
| `blue_key` | 18 | |
| `red_key` | 19 | |
| `grey_key` | 20 | |
| `white_key` | 21 | |
| `talisman` | 22 | Win item — grants immediately |
| `rose` | 23 | |
| `apple` | 24 | |
| `statue` | 25 | Gold Statue |
| `writ` | 28 | King's document |
| `bone` | 29 | For Spectre trade |
