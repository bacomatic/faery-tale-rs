# AGENTS.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

Rust port of "The Faery Tale Adventure" (1987 Amiga game by MicroIllusions). This is a personal learning project — PRs are not accepted. The goal is faithful recreation of the original game mechanics and behavior, not modernization. Original C/ASM source lives in `original/` for reference.

Key directive: **be true to the original game** — no enhancements or bug fixes unless the original bug would require extra work to reproduce.

## Build & Run

```bash
# Linux prerequisites (apt-based)
sudo apt install rust libsdl2-dev libsdl2-gfx-dev libsdl2-mixer-dev

# Build and run
cargo build
cargo run

# Run with debug window (separate SDL2 window showing clock, palettes, etc.)
cargo run -- --debug

# Run tests
cargo test

# Run a single test
cargo test test_name
# e.g. cargo test test_rgb4_conversion
```

No linter or formatter is configured. No CI pipeline.

## Architecture

### Entry Point & Game Loop (`src/main.rs`)

The main loop is a single `'running` loop that:
1. Updates `GameClock` to get `delta_ticks`
2. Pumps SDL2 events, dispatching to the active scene first, then debug window, then fallback key handlers
3. If an active scene exists, calls `scene.update()` which handles both logic and rendering
4. When a scene returns `SceneResult::Done`, chains to the next scene via the `ScenePhase` enum (Intro → CopyProtect → PlacardStart → Gameplay)
5. If no scene is active (gameplay phase, not yet implemented), renders directly

SDL2 canvas uses logical size 640×480 to preserve 4:3 aspect ratio. The `play_tex` is a 320×200 render target matching the original Amiga lores resolution. All Amiga-resolution rendering goes to `play_tex`, which is upscaled to the canvas.

### Module Structure (`src/game/`)

All game modules live under `src/game/` with `mod.rs` as the public module list. `#![allow(dead_code)]` is set at the module level since much infrastructure is built ahead of use.

**Scene system** (`scene.rs`): `Scene` trait with `handle_event()`, `update()`, `on_exit()`, `as_any()`. Scenes receive the canvas, play texture, delta ticks, game library, and `SceneResources` (mutable access to image/font textures). Concrete scenes:
- `intro_scene.rs` — 7-phase FSM: TitleText → TitleFadeOut → ZoomIn → ShowPage → FlipPage → ZoomOut → Done
- `copy_protect_scene.rs` — 3 random questions, typed input, `as_any()` downcast to check `passed()`
- `placard_scene.rs` — generic bordered text scene with configurable palette and hold duration

**Asset pipeline** (`game_library.rs`): `GameLibrary` is deserialized from `faery.toml` at startup. It holds all palettes, placards, fonts, images, cursors, and copy protection data. File-based assets (fonts, IFF images) are loaded eagerly on startup. Assets are looked up by string name.

**Amiga graphics stack**:
- `colors.rs` — `RGB4` (12-bit Amiga color) and `Palette` types, with conversions to/from SDL2 `Color` and RGBA32 lookup tables
- `bitmap.rs` — Amiga-style planar bitmap (1–5 bitplanes). Handles interleaved (IFF ILBM) and contiguous plane layouts. Converts to RGBA32 pixel buffers using palette LUTs
- `iff_image.rs` — IFF ILBM parser: FORM/BMHD/CMAP/BODY chunks, ByteRun1 decompression
- `image_texture.rs` — `ImageTexture` wraps an `IffImage` + shared SDL2 texture atlas (via `Rc<RefCell<Texture>>`). `update()` re-rasterizes pixels when palette changes (for fade effects)
- `font.rs` — Amiga DiskFont loader (`.font` files → hunk files). Parses char_loc, char_space, char_kern tables
- `font_texture.rs` — `FontTexture` renders DiskFont glyphs into a shared texture atlas. Supports `set_color_mod()` for tinting

**Texture atlas pattern**: Both images and fonts use a shared backing `Texture` (via `Weak<RefCell<Texture>>`) with per-item `Rect` bounds within the atlas. This avoids per-image texture creation overhead while allowing individual palette updates.

**Visual effects**:
- `palette_fader.rs` — Port of original `fade_page()` with per-channel multiplicative scaling, night floor limits, blue tint, vegetation boost, torch illumination. `FadeController` interpolates over time, returning either `ColorMod` (cheap SDL2 tint) or `PaletteUpdate` (full re-rasterization)
- `viewport_zoom.rs` — Port of `screen_size()`, computes centered sub-rect for zoom animations
- `page_flip.rs` — Port of `flipscan()`/`page_det()` with original 22-step lookup tables

**Other modules**:
- `game_clock.rs` — 60Hz tick-based clock. Game day = 24,000 ticks (~6m40s real time). Tracks day phases (Midnight/Morning/Midday/Evening) matching original `daynight` variable
- `settings.rs` — Persists to `~/.config/faery/settings.toml` via serde/TOML
- `hunk.rs` — Amiga HUNK executable loader (for extracting embedded game data from `game/fmain`)
- `render_task.rs` — `RenderTask` trait for periodic rendering (used by placard border animation)
- `debug_window.rs` — Optional second SDL2 window (`--debug` flag) showing game clock, palette viewer, placard/image browser
- `placard.rs` — Text rendering with swirly border animation (segment offset tables from original)

### Data Files

- `faery.toml` — Master game data: palettes (12-bit Amiga RGB4 values), placard text, font/image paths, cursor bitmaps, copy protection Q&A
- `game/` — Original Amiga game assets: IFF images, fonts, song data, `fmain` executable (contains embedded map/NPC/object data)
- `original/` — Original C/ASM source code (MIT licensed by David "Talin" Joiner). Reference only, not compiled

### Patterns & Conventions

- Enum-based FSM *within* scenes (phase enums), trait-based FSM *across* scenes (`Box<dyn Scene>`)
- Colors use Amiga 12-bit RGB4 format (0xRGB) throughout; conversion to 24-bit happens at the rendering boundary
- The original game's variable names and magic numbers are preserved in comments (e.g. `daynight`, `lightlevel`, `letter_list[]`)
- Time values reference the original's 50Hz Amiga tick rate, converted to 60Hz for the port (multiply by 1.2)
- `println!()` is used for warnings/debug output (no logging framework)
- All byte-level operations for binary file parsing go through `byteops.rs` helpers (`read_u8`, `read_u16`, `read_u32`, `read_string`)

## Current State & Roadmap

The intro sequence (title → story pages → copy protection → character placard) is fully implemented. See `PLAN.md` for detailed status and future plans covering: audio system, game world/map, player/movement, NPC system, graphics effects, key bindings, and persistence.
