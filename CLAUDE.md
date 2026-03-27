# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project constraints

- Rust port of "The Faery Tale Adventure" (1987 Amiga game by MicroIllusions).
- **Fidelity first**: be true to the original game. Do not add enhancements or modernizations.
- Do not "fix" original behavior unless reproducing the original bug would require extra work.
- Original C/ASM sources in `original/` are reference material and may be non-buildable.
- Personal project; PRs are not accepted.

## Timing invariant

- **NTSC-only at 30 fps** (interlaced frame rate; audio VBL is 60 Hz but gameplay ticks at 30 Hz). There was no PAL release; ignore PAL/50Hz comments in `original/`.

## Build and run

```bash
# Linux deps: sudo apt install rust libsdl2-dev libsdl2-gfx-dev libsdl2-mixer-dev
# macOS deps: brew install rust sdl2 sdl2_gfx sdl2_mixer  (then set LIBRARY_PATH)

cargo build
cargo run -- --debug --skip-intro   # recommended during development
cargo test                           # run all 150+ tests
cargo test <pattern>                 # run matching tests (e.g. cargo test combat)
cargo test -- --nocapture            # show println! output
```

## Canonical sources by topic

- Build/run commands and developer setup: `README.md`
- Reverse-engineering and file formats (`songs`, `v6`, ADF layout, etc.): **always check `RESEARCH.md` before guessing at binary format details**
- Architecture deep-dives (screen layout, Amiga rendering pipeline, palette handling, etc.): **always check `RESEARCH.md` before re-deriving implementation decisions**
- Roadmap/progress and task state: `PLAN.md` + `plan_status.toml`

## Architecture

SDL2-rendered game using a **chain-of-scenes** pattern. Scenes run sequentially:
`IntroScene → CopyProtectScene → PlacardScene → GameplayScene`

Each scene implements a `Scene` trait and returns a `SceneResult` to advance the chain.

### Key modules under `src/game/`

| Module | Role |
|--------|------|
| `gameplay_scene.rs` | Main game loop; input, movement, rendering dispatch |
| `world_data.rs` | Loads ADF sector/map/terrain/image blocks for a region |
| `adf.rs` | Raw ADF disk image reader — `load_blocks(block, count) -> &[u8]` |
| `tile_atlas.rs` | Decodes 5-bitplane Amiga tiles into pixel atlas |
| `palette.rs` | `amiga_color_to_rgba()`, `Palette`, `PaletteTransition` |
| `game_library.rs` | Parses `faery.toml`; `RegionBlockConfig` has all ADF block offsets |
| `map_renderer.rs` | Renders map tiles to 320×200 offscreen SDL2 texture |
| `hiscreen.rs` | Renders the 640×57 HI bar (buttons, compass, messages) |
| `render_resources.rs` | Owns all SDL2 textures; builds font/image atlases at startup |
| `audio.rs` | Software synthesizer — 4-voice Amiga-style envelope/wave generation |
| `game_state.rs` | Player stats (vitality, hunger, fatigue), inventory, position |
| `game_clock.rs` | NTSC 30 Hz ticker; in-game wall clock (day/hour/minute) |
| `debug_console.rs` | ratatui TUI in the launch terminal (activated with `--debug`) |
| `sprites.rs` | Character/enemy sprite loading and palette remapping |

### Data flow for region loading

`faery.toml` (via `game_library.rs`) → `WorldData::load()` with explicit ADF block numbers → `tile_atlas.rs` decodes 5-bitplane image memory → SDL2 texture atlas

### Screen layout

- Logical canvas: 640×480
- Playfield viewport: 288×140 (320×200 offscreen, then scaled)
- HI bar: 640×57 at bottom

### SDL2 color_mod gotcha

Always call `font.set_color_mod(r, g, b)` before every `render_string()` call. Default/white is `set_color_mod(255, 255, 255)`. SDL2 color mod is stateful — forgetting to reset causes text to render in the previous scene's tint color.

## Planning files contract

- `PLAN.md` is the human-readable roadmap; `plan_status.toml` is the machine-readable task state.
- Update **both files together** on any task state change.
- Validate after edits: `bash scripts/plan_sync_check.sh`
- GitHub Issues are the live tracker for active rollup tasks (`*-001`).
- Sync workflow: `bash scripts/sync_plan_from_github.sh`
