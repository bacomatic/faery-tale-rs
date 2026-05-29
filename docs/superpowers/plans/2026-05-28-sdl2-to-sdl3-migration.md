# SDL2 → SDL3 Migration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate the project from `sdl2` (v0.37) to `sdl3` (v0.18.x) while keeping the game fully playable at each checkpoint. The end state is the game running identically on SDL3, after which shader-based rendering becomes viable via `SDL_GPU`.

**Strategy:** Work in three phases — (1) gating SDL2 to isolate the boundary, (2) migrating graphics rendering, (3) migrating audio. Each phase ends with a working build. No gameplay logic changes are permitted at any point in this plan.

**Key invariant:** `cargo test` must pass after every task. The game must be runnable after each phase checkpoint.

---

## Background: SDL2 surface area audit

SDL2 appears in **24 source files** across these categories:

| Category | Files | Key types |
|----------|-------|-----------|
| Windowing & event loop | `main.rs` | `sdl2::init`, `Canvas<Window>`, `EventPump`, `Event` |
| Core scene interface | `scene.rs`, `render_task.rs` | `Canvas<Window>`, `Texture`, `Event` in trait signatures |
| Texture management | `render_resources.rs`, `font_texture.rs`, `image_texture.rs` | `Texture<'tex>`, `TextureCreator`, `PixelFormatEnum` |
| Rendering (scenes) | `gameplay_scene/rendering.rs`, `intro_scene.rs`, `placard_scene.rs`, `victory_scene.rs`, `copy_protect_scene.rs`, `placard.rs`, `page_flip.rs` | `Canvas`, `Texture`, `Rect`, `Color` |
| Input | `key_bindings.rs`, `gameplay_scene/input.rs`, `gameplay_scene/scene_impl.rs` | `Keycode`, `Button`, `Axis`, `Mod` |
| Audio | `audio.rs` | `AudioCallback`, `AudioDevice`, `AudioSpecDesired` |
| Small type conversions | `colors.rs`, `cursor.rs`, `viewport_zoom.rs`, `font.rs` | `Color`, `Point`, `Rect` |

**Good news from the SDL3 crate compatibility research:**
- `Canvas<Window>`, `Texture`, `TextureCreator`, `Surface`, `Rect`, `Point`, `Color`, `PixelFormat`, `Cursor`, `FullscreenType` — all **API-compatible** in `sdl3` v0.18.x. The high-level 2D renderer is a near drop-in.
- `Event` / `Keycode` / `Scancode` — compatible; `Keycode` retained in the Rust wrapper.
- `Canvas<T: RenderTarget>` generic pattern — confirmed working in `sdl3`.
- `GameController` / `Axis` / `Button` — compatible.
- **Only real delta: audio.** `AudioCallback` + `open_playback` → `AudioStream` + push model. The synthesizer code itself is unchanged.
- **SDL_mixer** — the high-level `sdl3::mixer` wrapper is not yet available. This is a non-issue: the project uses a pure-Rust software synthesizer; `mixer` was only needed for SDL2's audio device driver. SDL3 audio streams replace it entirely.

---

## Phase 0 — Preparation

### Task 0.1 — Create a feature-gated rendering stub
- [ ] In `Cargo.toml`, rename the existing `sdl2` dependency to remain but add a `default-features = false` stub for build testing without a display (CI use only). **Do not remove sdl2 yet.**
- [ ] Confirm `cargo build` and `cargo test` still pass with no changes.

### Task 0.2 — Identify and document the SDL2/SDL3 version boundary
- [ ] Add a comment block at the top of `src/game/scene.rs`, `src/main.rs`, and `src/game/audio.rs` noting the SDL version in use. Format: `// SDL-VERSION: sdl2 (migration target: sdl3)`. This makes grep-based progress tracking easy during the migration.
- [ ] Confirm `cargo test` passes.

---

## Phase 1 — Graphics Migration (SDL2 → SDL3, CPU rasterization preserved)

**Goal:** Replace `sdl2` with `sdl3` for all rendering and input. Audio stays on SDL2 until Phase 2. The game must render and accept input correctly at the end of this phase.

**Approach:** Because the SDL3 Rust crate's high-level API is nearly identical to SDL2's, most files need only `s/sdl2/sdl3/g` on imports plus a few targeted fixes. Work file-by-file from the outside in (leaf files first, core interfaces last).

### Task 1.1 — Update `Cargo.toml`
- [ ] Add `sdl3 = { version = "0.18", features = ["build-from-source"] }` as a dependency. Use `build-from-source` to avoid requiring a system SDL3 install.
- [ ] Keep `sdl2` in the dependency list for now (audio still uses it). Both can coexist during transition.
- [ ] Remove `features = ["mixer"]` from the `sdl2` entry — mixer is no longer needed once audio is ported (Phase 2). Leave it for now; remove in Task 2.1.
- [ ] Confirm `cargo build` (will fail to compile SDL2/SDL3 simultaneously — that is expected and will be resolved task by task).

### Task 1.2 — Port leaf type-conversion files
These files use only `Rect`, `Point`, or `Color` — trivial changes.

- [ ] `src/game/colors.rs` — change `use sdl2::pixels::Color` → `use sdl3::pixels::Color`. The `Color::RGB(r,g,b)` constructor is identical. Verify `From<&Color> for RGB4` and `From<&RGB4> for Color` still compile.
- [ ] `src/game/cursor.rs` — change `use sdl2::rect::Point` → `use sdl3::rect::Point`. The `Point::new(x,y)` constructor is identical.
- [ ] `src/game/viewport_zoom.rs` — change `use sdl2::rect::Rect` → `use sdl3::rect::Rect`.
- [ ] `src/game/font.rs` — change any `sdl2::rect::Rect` import → `sdl3::rect::Rect`.
- [ ] `src/game/key_bindings.rs` — change `use sdl2::controller::Button` → `use sdl3::controller::Button`; `use sdl2::keyboard::Keycode` → `use sdl3::keyboard::Keycode`.
- [ ] Run `cargo test` after all five files are updated.

### Task 1.3 — Port `render_task.rs`
- [ ] Change `use sdl2::rect::Rect` → `use sdl3::rect::Rect`; `use sdl2::render::Canvas` → `use sdl3::render::Canvas`; `use sdl2::video::Window` → `use sdl3::video::Window`.
- [ ] The `RenderTask` trait signature `fn update(&mut self, canvas: &mut Canvas<Window>, delta_ticks: i32, area: Option<Rect>)` requires no semantic changes.
- [ ] Run `cargo test`.

### Task 1.4 — Port `page_flip.rs`
- [ ] Change `sdl2` → `sdl3` in all imports (`Rect`, `Canvas`, `RenderTarget`, `Texture`).
- [ ] Verify that the generic `fn draw_page<T: RenderTarget>(canvas: &mut Canvas<T>, ...)` still compiles — `RenderTarget` trait exists in `sdl3`.
- [ ] Run `cargo test`.

### Task 1.5 — Port `font_texture.rs`
- [ ] Change all `sdl2` → `sdl3` in imports.
- [ ] `set_color_mod()` — confirm method exists on `sdl3::render::Texture` (it does).
- [ ] `texture.query()` — SDL3 renames this; use `texture.size()` or the `TextureQuery` struct equivalent. Investigate the sdl3 crate API and adapt.
- [ ] `BlendMode::Blend` — confirm identical in `sdl3::render::BlendMode`.
- [ ] The generic `render_string<T: RenderTarget>` pattern — confirm compiles.
- [ ] Run `cargo test`.

### Task 1.6 — Port `image_texture.rs`
- [ ] Change all `sdl2` → `sdl3` in imports.
- [ ] `canvas.copy()` — identical in sdl3.
- [ ] `texture.update()` — confirm API compatible; SDL3 uses `SDL_UpdateTexture` with same parameters.
- [ ] Run `cargo test`.

### Task 1.7 — Port `render_resources.rs`
- [ ] Change all `sdl2` → `sdl3` in imports (`PixelFormatEnum` → `PixelFormat` if renamed; check sdl3 crate).
- [ ] `create_texture_static()` — confirm method name in sdl3; SDL3 uses `SDL_CreateTexture` with `SDL_TEXTUREACCESS_STATIC`. Adapt if method name changed.
- [ ] `create_texture_target()` — confirm method name; adapt if changed.
- [ ] The `'tex` lifetime tied to `TextureCreator<WindowContext>` — confirm `WindowContext` still exists in `sdl3::video`. Adapt if type changed.
- [ ] `set_blend_mode()` — identical.
- [ ] Run `cargo test`.

### Task 1.8 — Port `scene.rs`
- [ ] Change `use sdl2::event::Event` → `use sdl3::event::Event`.
- [ ] Change `use sdl2::render::{Canvas, Texture}` → `use sdl3::render::{Canvas, Texture}`.
- [ ] Change `use sdl2::video::Window` → `use sdl3::video::Window`.
- [ ] The `Scene` trait's `fn handle_event(&mut self, event: &Event)` and `fn update(&mut self, canvas: &mut Canvas<Window>, play_tex: &mut Texture, ...)` signatures are unchanged.
- [ ] `SceneResources` struct fields referencing `Texture<'tex>` — unchanged.
- [ ] Run `cargo test`.

### Task 1.9 — Port scene implementations
These all implement the `Scene` trait. Once `scene.rs` is ported (Task 1.8), each file needs only import changes. Update all five in one task:

- [ ] `src/game/intro_scene.rs` — `sdl2` → `sdl3` in all imports.
- [ ] `src/game/copy_protect_scene.rs` — `sdl2` → `sdl3` in all imports.
- [ ] `src/game/placard_scene.rs` — `sdl2` → `sdl3` in all imports.
- [ ] `src/game/placard.rs` — `sdl2` → `sdl3` in all imports.
- [ ] `src/game/victory_scene.rs` — `sdl2` → `sdl3` in all imports.
- [ ] Run `cargo test`.

### Task 1.10 — Port `gameplay_scene/rendering.rs`
This file has the most SDL2 canvas calls (33 references). Update all at once:

- [ ] Change all `sdl2` → `sdl3` in imports.
- [ ] `canvas.texture_creator()` — unchanged.
- [ ] `canvas.with_texture_canvas()` — confirm available in sdl3; this wraps `SDL_SetRenderTarget`. If renamed, use the equivalent.
- [ ] `canvas.set_draw_color()`, `canvas.clear()`, `canvas.fill_rect()`, `canvas.copy()`, `canvas.draw_line()` — all unchanged in sdl3.
- [ ] `sdl2::surface::Surface::from_data()` → `sdl3::surface::Surface::from_data()` — API-compatible.
- [ ] `sdl2::pixels::Color::RGB()` → `sdl3::pixels::Color::RGB()` — identical.
- [ ] `sdl2::pixels::PixelFormatEnum::ARGB8888` → `sdl3::pixels::PixelFormat::ARGB8888` (note: enum name may drop `Enum` suffix in sdl3; check and adapt).
- [ ] Run `cargo test`.

### Task 1.11 — Port `gameplay_scene/mod.rs`, `scene_impl.rs`, `input.rs`
- [ ] `gameplay_scene/mod.rs` — `sdl2` → `sdl3` in imports (`Event`, `Keycode`, `Canvas`, `Texture`, `Window`).
- [ ] `gameplay_scene/scene_impl.rs` — `sdl2::keyboard::Mod` → `sdl3::keyboard::Mod`; `sdl2::controller::Axis` → `sdl3::controller::Axis`; `sdl2::mouse::MouseButton` → `sdl3::mouse::MouseButton`.
- [ ] `gameplay_scene/input.rs` — `sdl2::keyboard::Keycode` → `sdl3::keyboard::Keycode`.
- [ ] Run `cargo test`.

### Task 1.12 — Port `main.rs` (windowing, event loop, rendering loop)
This is the largest single-file change. Work section by section:

- [ ] **Imports** — change all `sdl2::` → `sdl3::` throughout.
- [ ] **Init** — `sdl2::init()` → `sdl3::init()` (identical pattern in sdl3 crate).
- [ ] **Video subsystem / window** — `video_subsystem.window(...)` builder pattern is identical.
- [ ] **Canvas creation** — `window.into_canvas()` is identical.
- [ ] **Logical size** — `canvas.set_logical_size(640, 480)` — confirm API in sdl3; adapt if method signature changed (SDL3 uses float logical size; the Rust wrapper may accept integers or require `f32`).
- [ ] **Texture creation** — `canvas.texture_creator()` / `create_texture_target()` — unchanged.
- [ ] **Game controller init** — `sdl_context.game_controller()` — identical pattern in sdl3.
- [ ] **Event loop** — `event_pump.poll_iter()` — identical. Match on `Event::Quit`, `Event::KeyDown`, `Event::Window` etc. — check that `WindowEvent` sub-events are available (SDL3 promotes these to top-level; the sdl3 Rust crate may use nested or flat form — adapt accordingly).
- [ ] **Keyboard events** — `Keycode::*` variants — identical names in sdl3.
- [ ] **Fullscreen toggle** — `FullscreenType::Desktop` / `FullscreenType::Off` — identical in sdl3.
- [ ] **Mouse cursor** — `Cursor::from_surface()` — identical API in sdl3.
- [ ] **Rendering loop** — `canvas.with_texture_canvas()`, `canvas.copy()`, `canvas.present()` — all identical.
- [ ] **`set_mouse()` helper** — `Surface::from_data()`, `Cursor::from_surface()` — identical.
- [ ] Remove `extern crate sdl2;` line; replace with `extern crate sdl3;` if needed (likely not needed in 2021 edition).
- [ ] Run `cargo build` and `cargo test`.
- [ ] **Checkpoint: launch the game and verify it renders and accepts input correctly.**

### Task 1.13 — Remove SDL2 from non-audio code
- [ ] Confirm that `sdl2` is now only referenced in `src/game/audio.rs`.
- [ ] Run `cargo build` with a temporary `#[allow(unused_imports)]` removed from audio.rs to confirm isolation.
- [ ] Remove the `sdl2` comments added in Task 0.2 from `scene.rs` and `main.rs`; update the `audio.rs` comment to `// SDL-VERSION: sdl2 (audio only, pending Phase 2)`.

---

## Phase 2 — Audio Migration (SDL2 → SDL3 audio streams)

**Goal:** Port the audio system from SDL2's callback model to SDL3's push-based `AudioStream`. Remove `sdl2` entirely from the dependency tree. The synthesizer logic in `audio.rs` is unchanged; only the delivery mechanism changes.

### Background: the callback → stream inversion

SDL2 model (current):
```
SDL calls SynthCallback::callback(out: &mut [i16]) on a background thread
→ synthesizer fills the buffer
→ SDL sends buffer to hardware
```

SDL3 model (target):
```
Game loop calls AudioSystem::push_frame() once per tick
→ synthesizer generates N frames of i16 PCM
→ game calls stream.put_audio_stream_data(&pcm_bytes)
→ SDL drains the stream to hardware asynchronously
```

The `SynthCallback::callback()` method body becomes a plain function. The `Arc<Mutex<SequencerState>>` can be simplified or removed entirely since the push happens on the main thread.

### Task 2.1 — Update `Cargo.toml`
- [ ] Remove `features = ["mixer"]` from the `sdl2` entry (already noted in Task 1.1).
- [ ] Confirm `sdl3` dependency is present (done in Phase 1).
- [ ] After this phase, `sdl2` will be removed entirely.

### Task 2.2 — Add SDL3 audio types alongside existing SDL2 audio
- [ ] In `audio.rs`, add the new SDL3 imports alongside existing SDL2 ones temporarily:
  ```rust
  use sdl3::audio::{AudioSpec, AudioFormat, AudioStream};
  ```
- [ ] This lets both compile during transition. Remove SDL2 audio imports at end of task.

### Task 2.3 — Extract the synthesizer core from the callback
The synthesizer logic currently lives inside `SynthCallback::callback()`. Extract it to a standalone method with no SDL2 types:

- [ ] Add a method to `SynthCallback` (or `SequencerState`):
  ```rust
  /// Generate `frames` stereo i16 output samples into `out`.
  /// Identical logic to the existing AudioCallback::callback() body.
  pub fn generate(&mut self, out: &mut [i16], frames: usize) { ... }
  ```
- [ ] The existing `AudioCallback::callback()` implementation delegates to `generate()`:
  ```rust
  fn callback(&mut self, out: &mut [i16]) {
      self.generate(out, out.len() / 2);
  }
  ```
- [ ] Run `cargo test` — all audio unit tests must still pass.

### Task 2.4 — Replace `AudioSystem::new()` with SDL3 stream
- [ ] Replace the `open_playback` call with SDL3 stream creation:
  ```rust
  let spec = AudioSpec {
      format: AudioFormat::S16,
      channels: 2,
      freq: SAMPLE_RATE,
  };
  let stream = audio_subsystem.open_audio_device_stream(
      sdl3::audio::AudioDeviceID::DEFAULT_PLAYBACK,
      &spec,
  )?;
  stream.resume()?;
  ```
- [ ] Remove `AudioDevice<SynthCallback>` field from `AudioSystem`. Replace with `AudioStream` (SDL3).
- [ ] Remove `AudioCallback` trait implementation from `SynthCallback` (no longer needed).
- [ ] Remove `AudioSpecDesired` (no longer needed).
- [ ] Update `AudioSystem` to own the `SequencerState` directly (not behind `Arc<Mutex<>>` — it's now main-thread-only).
- [ ] Run `cargo test`.

### Task 2.5 — Implement push-based audio tick
- [ ] Add `AudioSystem::tick(&mut self)` method called from the main game loop once per frame (30fps):
  ```rust
  pub fn tick(&mut self) {
      // Generate one frame's worth of samples (44100 / 30 ≈ 1470 frames = 2940 i16 values)
      const FRAMES_PER_TICK: usize = (SAMPLE_RATE as usize + 29) / 30;
      let mut buf = vec![0i16; FRAMES_PER_TICK * 2];
      self.synth.generate(&mut buf, FRAMES_PER_TICK);
      // Push to SDL3 stream as raw bytes
      let bytes: &[u8] = bytemuck::cast_slice(&buf);  // or manual cast
      self.stream.put_audio_stream_data(bytes).ok();
  }
  ```
- [ ] Call `audio.tick()` in the main loop, once per game tick (alongside scene update).
- [ ] Remove the `Arc<Mutex<SequencerState>>` shared-state pattern. `SequencerState` is now owned directly by `AudioSystem` and accessed only from the main thread.
- [ ] Update `play_score()`, `stop_score()`, `play_sfx()` etc. to operate on `&mut self` directly (no mutex needed).
- [ ] Run `cargo test`.

### Task 2.6 — Sound effects
The existing SFX system uses `SfxChannel` with nearest-neighbour resampling from 8 kHz source data. It currently mixes into the same PCM buffer as the music in the callback.

- [ ] Confirm that the `SfxChannel::mix_into()` call is included in `generate()` from Task 2.3. No change needed to SFX logic — it already runs in the same generate pass.
- [ ] Verify that `AudioSystem::play_sfx(index)` continues to work by setting `sfx.active` as before.
- [ ] **SFX gap note:** The original game has 6 SFX loaded from `game/samples`. The existing implementation in `SfxChannel` already handles all 6. No new SFX work is needed for parity. Document the 6 SFX slots in a comment for future reference:
  - Slot 0: attack hit
  - Slot 1–5: remaining effects (verify from reference docs)
- [ ] Run `cargo test`.

### Task 2.7 — Remove SDL2 entirely
- [ ] Remove `sdl2` from `Cargo.toml` completely.
- [ ] Remove all remaining `use sdl2::...` and `extern crate sdl2` references.
- [ ] Run `cargo build` — must succeed with zero SDL2 references.
- [ ] Run `cargo test` — all tests must pass.
- [ ] Remove the `// SDL-VERSION:` comment from `audio.rs`.
- [ ] **Checkpoint: launch the game. Verify rendering, input, and music all work correctly on SDL3.**

---

## Phase 3 — Verification & Cleanup

### Task 3.1 — Full gameplay verification checklist
Manually verify the following before declaring the migration complete:

- [ ] Game launches and displays intro sequence correctly.
- [ ] Keyboard input works (movement, menus, inventory).
- [ ] Game controller input works (if hardware available).
- [ ] Music plays throughout intro and gameplay.
- [ ] Sound effects play on combat hits and other triggers.
- [ ] Palette transitions render correctly (day/night fades).
- [ ] Witch effect (scanline warp) renders correctly.
- [ ] Compass renders in the HUD.
- [ ] Fullscreen toggle (F key or equivalent) works.
- [ ] Custom bow cursor displays correctly.
- [ ] Window resize / logical scaling works at 640×480.
- [ ] Game saves and loads correctly.

### Task 3.2 — Cleanup
- [ ] Remove any temporary `#[allow(...)]` attributes added during migration.
- [ ] Remove the `// SDL-VERSION:` marker comments.
- [ ] Remove any dead code or commented-out SDL2 blocks.
- [ ] Run `cargo clippy` and fix any new warnings introduced by the migration.
- [ ] Run `cargo test` — all tests pass.

### Task 3.3 — Update `Cargo.toml` and `AGENTS.md`
- [ ] Update the `[dependencies]` comment in `Cargo.toml` noting SDL3.
- [ ] Update `AGENTS.md` build/run notes if the dependency setup changed (e.g. if `build-from-source` requires CMake to be available).

### Task 3.4 — Commit
- [ ] Stage all changes: `git add -A`.
- [ ] Write commit message summarising the migration: SDL2 → SDL3, audio callback → stream, SDL_mixer removed.
- [ ] Commit.

---

## Appendix A: SDL2 → SDL3 API quick-reference for this codebase

| SDL2 | SDL3 | Notes |
|------|------|-------|
| `sdl2::init()` | `sdl3::init()` | Identical |
| `Canvas<Window>` | `Canvas<Window>` | Identical |
| `TextureCreator<WindowContext>` | `TextureCreator<WindowContext>` | Identical |
| `Texture<'tex>` | `Texture<'tex>` | Identical |
| `PixelFormatEnum::RGBA32` | `PixelFormat::RGBA32` | May drop `Enum` suffix |
| `PixelFormatEnum::ARGB8888` | `PixelFormat::ARGB8888` | May drop `Enum` suffix |
| `BlendMode::Blend` | `BlendMode::Blend` | Identical |
| `Rect::new(x,y,w,h)` | `Rect::new(x,y,w,h)` | Identical |
| `Point::new(x,y)` | `Point::new(x,y)` | Identical |
| `Color::RGB(r,g,b)` | `Color::RGB(r,g,b)` | Identical |
| `Surface::from_data(...)` | `Surface::from_data(...)` | Identical |
| `Cursor::from_surface(...)` | `Cursor::from_surface(...)` | Identical |
| `FullscreenType::Desktop` | `FullscreenType::Desktop` | Identical |
| `canvas.set_logical_size(w,h)` | `canvas.set_logical_size(w,h)` | May use `f32` in SDL3 |
| `canvas.with_texture_canvas()` | `canvas.with_texture_canvas()` | Wraps `SDL_SetRenderTarget` |
| `canvas.copy()` | `canvas.copy()` | Identical |
| `canvas.present()` | `canvas.present()` | Identical |
| `event_pump.poll_iter()` | `event_pump.poll_iter()` | Identical |
| `Event::KeyDown { keycode, .. }` | `Event::KeyDown { keycode, .. }` | Keycode retained in wrapper |
| `WindowEvent::Resized` | Check sdl3 crate — may be top-level `Event::Window { win_event }` or separate variant |
| `Keycode::Escape` etc. | `Keycode::Escape` etc. | Identical names |
| `sdl_context.game_controller()` | `sdl_context.game_controller()` | Identical |
| `Button::*`, `Axis::*` | `Button::*`, `Axis::*` | Identical names |
| `AudioCallback` trait | Removed — use push model | See Phase 2 |
| `AudioSpecDesired` | `AudioSpec { format, channels, freq }` | Simplified |
| `open_playback(...)` | `open_audio_device_stream(...)` | Push model |
| `device.resume()` | `stream.resume()` | Equivalent |

## Appendix B: Files NOT requiring changes

These files have zero SDL2 dependencies and are unaffected by the migration:

- `src/game/bitmap.rs`
- `src/game/bitblit.rs`
- `src/game/sprite_mask.rs`
- `src/game/tile_atlas.rs`
- `src/game/map_renderer.rs`
- `src/game/gfx_effects.rs`
- `src/game/palette.rs`
- `src/game/copper.rs`
- `src/game/songs.rs`
- `src/game/combat.rs`
- `src/game/npc.rs`
- `src/game/npc_ai.rs`
- `src/game/world_data.rs`
- `src/game/game_state.rs`
- `src/game/game_clock.rs`
- `src/game/game_library.rs`
- `src/game/sprites.rs`
- `src/game/iff_image.rs`
- `src/game/adf.rs`
- All `docs/`, `src/game/debug_tui/` (uses crossterm/ratatui, not SDL)

## Appendix C: What becomes possible after this migration

Once on SDL3, the following are available without any additional crates:

- **`SDL_GPU` API** — first-party GPU pipeline with shader support (SPIR-V on Vulkan/Linux). This is the path to the palette-lookup shader discussed earlier. Available via the `sdl3-sys` low-level bindings; high-level Rust wrapper in progress.
- **`raw-window-handle` feature** — `sdl3` crate implements `HasWindowHandle` + `HasDisplayHandle`, enabling `wgpu` or `ash` integration if the SDL_GPU path is not sufficient.
- **Wayland support** — SDL3 has better Wayland support out of the box.
- **HDR / high-DPI** — SDL3 renderer supports these natively.
