## Canonical Scope

This document is the canonical human-readable roadmap and progress log.

- Human status index and plan narrative: this file (`PLAN.md`)
- Machine-readable task state mirror: `plan_status.toml`
- Human-readable research/reference notes: `RESEARCH.md`
- Machine-readable research index for agents: `research_index.toml`
- Build/run setup and developer environment: `README.md`
- Reverse-engineering and file format details: `DECODE.md`
- Agent constraints and execution contract: `AGENTS.md`

## Issue Tracking Provenance

GitHub Issues were enabled for this project on **2026-03-01**.

Rollup summary comments listing sub-issue breakdowns were posted on **2026-03-01** to issues #1–#7.

Status reporting conventions:
- **Issue: #<number>** — task is tracked in GitHub Issues.
- **Issue: pre-issues** — task was completed before Issues were enabled.
- **Issue: n/a** — intentionally untracked housekeeping/meta work.

For tasks marked `pre-issues`, use commit history as evidence (for example:
`git log -- <path>`), and do not backfill synthetic issue numbers.

## Issue Map (Rollups)

- `gameloop-001` → #59
- `debug-001` → #58
- `intro-001` → pre-issues
- `audio-001` → #1
- `vkbd-001` → pre-issues
- `world-001` → #3
- `player-001` → #4
- `npc-001` → #5
- `gfx-001` → #2
- `keys-001` → #6
- `persist-001` → #7

## Status Index (source of truth for humans)

Machine-readable mirror: `plan_status.toml`

### In Progress
- [audio-001] Audio system (3/5 steps complete; 2 deferred; Issue: #1)
- [gfx-001] Graphics effects (1/5 steps in progress; Issue: #2)
- [world-001] Game world & map system (0/11 steps complete; Issue: #3)
- [player-001] Player & movement (0/14 steps complete; Issue: #4)
- [npc-001] NPC system (0/7 steps complete; Issue: #5)
- [keys-001] Key bindings (0/7 steps complete; Issue: #6)
- [gameloop-001] Core game loop (2/15 steps complete; Issue: #59)
- [debug-001] Debug window enhancements (10/11 steps complete; debug-111 unblocked by persist-001; Issue: #58)

### Todo

### Done
- [persist-001] Persistence (3/3 steps complete; Issue: #7)
- [intro-001] Intro sequence end-to-end (6/6 steps complete; Issue: pre-issues)
- [vkbd-001] Virtual keyboard & sine wave tuning tool (3/3 steps complete; Issue: pre-issues)

## Plan: Intro Sequence End-to-End

**TL;DR:** Build just enough scene infrastructure to drive the full intro — from title text through story pages to copy protection and the first placard — then wire it into the existing event loop. This gives you a playable sequence and a reusable scene system for everything that follows.

### Completed

1. **Scene system** — `Scene` trait in `src/game/scene.rs` with `handle_event()` + `update(canvas, play_tex, delta_ticks, game_lib, resources) -> SceneResult`. `SceneResources` bundles `&mut [ImageTexture]`, `&HashMap<String, usize>`, and font textures. Both `find_image()` (read) and `find_image_mut()` (write, for palette re-rasterization) are available. Scenes chain via `ScenePhase` enum in `main.rs`: Intro → CopyProtect → PlacardStart → Gameplay.

2. **Palette fading** — Hybrid system in `src/game/palette_fader.rs`:
   - **`fade_page()`**: Port of the original C `fade_page()` from `fmain2.c`. Per-channel multiplicative scaling (0–100%), night-time floor limits (r≥10, g≥25, b≥60), blue tint injection for moonlight, vegetation color boost (indices 16–24), and Green Jewel light illumination (light_timer boosts red to green's level). Returns a new `Palette`.
   - **`FadeController`**: Time-based wrapper that interpolates RGB percentages over a duration. Returns `FadeResult::ColorMod(r,g,b)` for uniform fades (cheap SDL2 `set_color_mod`) or `FadeResult::PaletteUpdate(Palette)` for non-uniform fades (per-frame `ImageTexture::update()` re-rasterization). Convenience constructors: `fade_down()`, `fade_normal()`, `zoom_fade()`, `zoom_percentages()`.
   - **`PaletteFader`**: Simple linear interpolation between two palettes (retained for cases where lerp is sufficient).
   - 10 tests covering fade_page behavior, FadeController dispatch, zoom fading, and PaletteFader lerp.

3. **Viewport zoom** — `ViewportZoom` in `src/game/viewport_zoom.rs`. Port of `screen_size()` — computes centered sub-rect for zoom-in (0→160) and zoom-out (156→0). `half_width()` accessor exposes raw zoom position for zoom-position-dependent palette fading. 5 unit tests.

4. **Page flip animation** — `PageFlip` in `src/game/page_flip.rs`. Port of `flipscan()` + `page_det()` with the original 22-step lookup tables. Draws directly to the window canvas at display scale using `copy_region()` / `copy_full()` helpers with `&Texture` sources (no `ImageTexture` dependency). `SceneResources` carries a 320×200 scratch render-target texture for the old-page snapshot.

5. **IntroScene** — `IntroScene` in `src/game/intro_scene.rs`. 7-phase FSM: TitleText → TitleFadeOut → ZoomIn → ShowPage → FlipPage → ZoomOut → Done. Space to skip.
   - **TitleFadeOut**: Uses `FadeController::fade_down()` → `FadeResult::ColorMod` applied via `play_tex.set_color_mod()`.
   - **ZoomIn/ZoomOut**: Uses `FadeController::zoom_fade(introcolors, half_width)` per frame to compute a zoom-position-dependent palette with staggered channel ramp-up (red leads, green follows, blue lags — matching original `screen_size()` formula). Re-rasterizes all intro images via `resources.find_image_mut(name).update(&faded_palette, None)`. Restores full-brightness palette when zoom completes.
   - **FlipPage**: Animated strip-based page flip via `PageFlip`. On enter, snapshots `play_tex` → scratch (old page), draws new overlays onto `play_tex` (new page). Each frame, `PageFlip::update()` composites strips from scratch (old) and `play_tex` (new) directly onto the window canvas at 2× scale with 40 px vertical offset. Timing uses original NTSC 60Hz `Delay()` values directly from the `FLIP3` table.
   - Font, image, and placard rendering wire directly through `SceneResources`.

6. **delta_ticks** — `GameClock::update()` returns monotonic delta. Frame delta passed to scenes instead of hardcoded `1`.

7. **Wire image textures into IntroScene** — `SceneResources<'a, 'tex>` struct provides named image lookup via `find_image()` and `find_image_mut()`. `ZoomIn` draws `page0` to play_tex; page compositing overlays portraits at (4,24) and bios at page-specific positions (from original `copypage()` calls). Pages accumulate on play_tex matching original behavior. `image_name_map: HashMap<String, usize>` built in main.rs image loading loop.

8. **Title text rendering in IntroScene** — `titletext` placard rendered directly to 640x480 canvas (not play_tex) during TitleText phase using amber font. Y offset of 140px centers text vertically. Dark blue background (0x006) matches original `textcolors` palette. `draw_offset()` method added to Placard.

9. **Copy protection scene** — `CopyProtectScene` in `src/game/copy_protect_scene.rs`. 3 random questions from pool of 8 (without replacement). Typed input via KeyDown events, case-insensitive answer matching. `submit_pending` flag bridges handle_event/update. Topaz font on dark blue background. `passed()` method checked via `as_any()` downcast after scene completes. Skip flag for development.

10. **Character start placard** — `PlacardScene` in `src/game/placard_scene.rs`. Generic scene for any placard with swirly border. Instant border via `draw_placard_border()` using palette index 24 for both border and text color. `FontTexture::set_color_mod()` tints the amber font to the palette's color 24 (red in `pagecolors`) before drawing, then resets to white. Configurable placard name, palette, and hold duration (default 144 ticks = 2.4s). Space to skip. Uses `pagecolors` palette (matching original `map_message()` which calls `LoadRGB4(&vp_page, pagecolors, 32)`).

11. **Copy protection failure**: Wrong answer quits the game

### Future Refinements (Intro)

- ~~**Animated placard border**: Progressive border drawing over time (currently drawn all at once)~~ Done
- ~~**Audio integration**: Intro music (tracks 12–15) during TitleText phase~~ Done (music starts on TitleFadeOut, matching original `playscore()` placement)

### Decisions

- Enum-based FSM within each scene (phase enum), trait-based FSM across scenes (`Box<dyn Scene>`) — balances simplicity inside with flexibility across
- `PaletteFader` is a standalone utility, not a scene — it's composed into scenes that need it
- Page flip is a `RenderTask` (fits the existing pattern), intro phases are *not* RenderTasks (they need richer control flow)
- Copy protection is included but can be gated with a skip flag for development convenience

---

## Plan: Audio System

**Status:** In progress (steps 1–2, 4 complete; step 3 deferred)

### Overview
Parse the music file (`game/songs`), build a song list, and play tracks via SDL2 mixer. Sound effects loaded from the game data.

### Steps

1. ~~**Parse music/song data**~~ Done — `SongLibrary` in `src/game/songs.rs`. Custom 4-voice tracker format parsed from `game/songs` (28 tracks). `TrackEvent` enum models all commands from `gdriver.asm`: Note, Rest, SetInstrument, SetTempo, End (with loop flag). Lookup tables `PTABLE` (78 period/wave-offset entries) and `NOTE_DURATIONS` (64 timing values) ported verbatim. NTSC Paula clock (3,579,545 Hz). 12 unit tests including real-file parsing.

2. ~~**SDL2 mixer integration**~~ Done — `AudioSystem` in `src/game/audio.rs`. Pure-Rust, no `unsafe`, 4-voice software synthesizer porting `gdriver.asm` note-trigger + envelope logic. `Instruments` loads waveforms and ADSR envelopes from `game/v6` (envelopes at byte 2048, matching the original `Seek(+S_WAVBUF, OFFSET_CURRENT)` load sequence). `SequencerState` drives 4 `Voice`s with timeclock stepping (150 tempo, 60 Hz NTSC VBL). `SynthCallback` fires a VBL tick every 735 samples (~44100/60) and mixes voices into a **stereo i16 stream at 44100 Hz** with Amiga Paula DAC routing (voices 0+3 → left, 1+2 → right; 75%/25% primary/bleed). Per-voice rendering uses linear interpolation with correct modulo loop-wrap, a 1-pole IIR low-pass at ~4800 Hz, and a 64-sample de-click ramp to suppress note-transition pops. Intro music (tracks 12–15) plays automatically at startup. 13 tests.

3. **Sound effects** *(deferred to game implementation phase)*
   - Identify and load sound effect data from `game/` (sample bank referenced in `gdriver.asm` as `sample_mem`, 5632 bytes from `dh0:z/samples`)
   - Trigger effects from game events via a dedicated effects channel
   - **Do NOT replicate the Amiga's voice-stealing behavior**: the original `_playsample()` hijacks Paula voice 2 (silencing that music channel) because the hardware only has 4 channels total. We have no such limitation — sound effects should mix into a separate channel alongside the 4 music voices, leaving music uninterrupted.

4. ~~**Wire into scenes**~~ Done — `IntroScene::new()` takes `Option<[Arc<Track>; 4]>` and starts music (`play_score`) on the first frame of `TitleFadeOut`, matching the original `playscore()` placement (before zoom-in). Music stops (`stop_score`) in `main.rs` when `PlacardStart` completes and gameplay begins. All Amiga `Delay()` constants corrected to NTSC 60Hz native values (removed the erroneous ×1.2 PAL conversion factor that had inflated `PAGE_DISPLAY_TICKS` 420→350, `LAST_PAGE_HOLD_TICKS` 228→190, `TITLE_HOLD_TICKS` 120→100, and `PageFlip` step delays).

5. **Implement `setmood()` — in-game music context switching** *(deferred to game implementation phase)*
   - Port `setmood()` from `fmain.c`: select the active song group (tracks 0–3, 4–7, … 24–27) based on hero state and call `play_score` / `set_score` / `stop_score` accordingly.
   - Group selection logic (in priority order):
     1. Hero vitality = 0 → group 6 (death/game-over, tracks 24–27)
     2. Palace zone coordinates (`hero_x` 0x2400–0x3100, `hero_y` 0x8200–0x8a00) → group 4 (tracks 16–19)
     3. `battleflag` set → group 1 (battle, tracks 4–7)
     4. `region_num > 7` → group 5 (indoor/dungeon, tracks 20–23)
     5. `lightlevel > 120` → group 0 (outdoor daytime, tracks 0–3)
     6. Otherwise → group 2 (outdoor night, tracks 8–11)
   - **Cave instrument swap**: region 9 (caves) uses group 5 tracks but must swap instrument slot 10 in `new_wave[]` to `0x0307` before calling `play_score`; all other indoor regions use `0x0100`. This changes the timbre of voice 2 without altering any note data. Reset to `0x0100` on leaving region 9.
   - `set_score` (vs `playscore`) is used when the new score should take effect at the next loop boundary rather than immediately (avoids an abrupt cut mid-phrase).

**Known issues / fine tuning:**
- ~~Minor click between notes~~ — Fixed by a 64-sample de-click ramp in `mix_stereo` that smooths the gain envelope on note start/stop. Stereo output also added (Amiga Paula DAC routing) at the same time. Music is correct and verified against original.

---

## Plan: Virtual Keyboard & Sine Wave Tuning Tool

**Status:** Complete

**Implementation notes (divergences from original plan):**
- Period tuning uses **numpad `2`/`8`** (fine ±1) and **`Shift`+numpad** (coarse ±10) instead of F5/F6.
- Frequency formula corrected per Amiga hardware reference: `AMIGA_CLOCK / (wave_len × period)` where `wave_len = (32 - wave_offset) × 2`. Prior display used just `AMIGA_CLOCK / period` (8–64× too high).
- Pitch offset fixed: PTABLE rows start at **A**, not C. `pitch_for_key` adds +3 so that the 'A' key sounds C at the selected octave. White key note labels use the same corrected formula.

**TL;DR:** Add a new `src/bin/vkbd.rs` standalone binary (following the `music_viz` pattern) that renders a terminal piano keyboard via crossterm. The bottom ASDF row maps to white keys, the QWERTY row to black keys (standard DAW layout). A sine wave instrument mode with configurable H2/H3 harmonics lives inside the same tool for PTABLE frequency tuning. Instrument switching uses both `1`–`9` number keys and `[`/`]`. The synth is a private copy (same as `music_viz`) to avoid modifying `game::audio`.

### Steps

1. **Register the binary in `Cargo.toml`** — add `[[bin]] name = "vkbd" path = "src/bin/vkbd.rs"` alongside the `music_viz` entry.

2. **Define the keyboard → pitch mapping** in a `const` array at the top of the file. Standard two-row DAW layout:
   - White keys (ASDF row): `A`=C, `S`=D, `D`=E, `F`=F, `G`=G, `H`=A, `J`=B, `K`=C+1, `L`=D+1, `;`=E+1
   - Black keys (QWERTY row): `W`=C#, `E`=D#, `T`=F#, `Y`=G#, `U`=A#, `O`=C#+1, `P`=D#+1
   - Each char maps to a semitone offset 0–16; add `base_octave * 12` to get the absolute `PTABLE` index, clamped 0–83.

3. **Private synth copy** following the `music_viz` pattern:
   - Pull in `songs.rs` with `#[path = "../game/songs.rs"] mod songs;`
   - Copy `Instruments`, `Voice`, and a stripped-down `SynthCallback` from `music_viz`/`audio.rs`
   - **Extend `Voice` for manual sustain**: add a `manual: bool` field. When `true`, the voice ignores all event-based timing and renders continuously until `Voice::silence()` is called explicitly — no `event_stop` processing
   - Replace `SequencerState` with a simpler `struct ManualState { voices: [Voice; 4], instruments: Instruments, current_instrument: usize, sine_mode: bool, sine_config: SineConfig }` — no track/timeclock machinery
   - Expose `fn trigger_voice(voice_idx: usize, pitch: u8)` and `fn release_voice(voice_idx: usize)` on `ManualState` for direct keydown/keyup control
   - SDL2 audio callback only calls `mix_stereo()` per voice — no VBL tick or sequencer stepping

4. **Polyphony and sustain**: track `held_keys: HashMap<char, (u8, usize)>` mapping key char → `(pitch, voice_idx)`. On keydown, assign the next free voice (round-robin 0–3); on keyup, release only that voice. Up to 4 simultaneous keys sustained.
   - **Latch mode** (`Shift`+key): pressing `Shift`+piano-key latches the note — it sustains indefinitely after key release. The keyboard display marks latched keys distinctly (e.g. bold/underline). Pressing the same key again (with or without `Shift`) unlatches and silences it. Multiple keys can be latched simultaneously (up to 4 voices).
   - **Arrow-key pitch bending on latched notes**: while one or more notes are latched, arrow keys re-pitch the most-recently-latched voice:
     - `→` — move to the next higher semitone (pitch + 1, clamped to 83)
     - `←` — move to the next lower semitone (pitch − 1, clamped to 0)
     - `↑` — jump to the nearest black key (sharp/flat) above current pitch
     - `↓` — jump to the nearest white key (natural) below current pitch
     - The voice is re-triggered at the new pitch without releasing first (seamless glide). The held_keys entry and keyboard display update to reflect the new note.

5. **Sine wave instrument mode**:
   - `struct SineConfig { harmonic2: f32, harmonic3: f32 }` — additive amplitudes for harmonics relative to fundamental (each clamped 0.0–1.0)
   - `fn generate_sine_waveform(config: &SineConfig) -> [i8; 128]`: sum `sin(2πi/128)` + `h2 * sin(4πi/128)` + `h3 * sin(6πi/128)`, normalize peak to ±127
   - `Tab` toggles sine mode: swaps the current instrument's waveform slot with the generated sine (or restores the original). Update happens under the audio mutex — latency-free, no restart
   - `F2`/`F3` selects which harmonic to adjust; `+`/`-` adjusts its amplitude by ±0.05, regenerates and re-uploads the waveform immediately

6. **Live PTABLE editing**:
   - While a note is sounding (held or latched), the user can fine-tune the period value for that pitch's PTABLE entry in real time:
     - `F5`/`F6` — decrease/increase the period by 1 (fine tuning)
     - `Shift`+`F5`/`Shift`+`F6` — decrease/increase the period by 10 (coarse tuning)
   - Period changes apply to the currently sounding pitch index in a mutable copy of `PTABLE` (the original is const). The active voice's `phase_inc` is recalculated immediately so the frequency shift is audible while the note sustains.
   - The status bar shows the current pitch's PTABLE entry: `Pitch: 36  Period: 428 → 425  Freq: 8362.7 Hz`
   - A `ptable_dirty` flag tracks whether any entry has been modified.

7. **PTABLE export on exit**:
   - If `ptable_dirty` is true when the user quits (`Q`/Esc), print the full modified PTABLE to stdout as a Rust `const` array (copy-pasteable into `songs.rs`):
     ```
     // Modified PTABLE — paste into src/game/songs.rs
     pub const PTABLE: [(u16, u16); 84] = [
         (1440, 0), (1360, 0), ...
         (428, 16), (404, 16), ...  // ← modified entries marked with comment
     ];
     ```
   - Only entries that differ from the original are annotated with `// was <old_period>`.
   - This output appears after crossterm cleanup so terminal formatting is clean.

8. **Key input handling** (crossterm raw mode):
   - Piano key press → `trigger_voice(next_free, pitch)`; release → `release_voice(idx)`
   - `Shift`+piano-key → latch/unlatch note (toggle)
   - `←`/`→`/`↑`/`↓` → re-pitch most-recently-latched voice (see step 4)
   - `Z`/`X` → decrement/increment `base_octave` (0–6), re-display
   - `1`–`9` → set `current_instrument` directly; `[`/`]` → decrement/increment, both wrap around 0–11
   - `Tab` → toggle sine mode
   - `F2`/`F3` + `+`/`-` → harmonic amplitude control (only active in sine mode)
   - `F5`/`F6` → fine-tune period (±1); `Shift`+`F5`/`F6` → coarse-tune period (±10)
   - `Q`/Esc → if ptable dirty, print modified PTABLE; cleanup crossterm, quit

9. **Terminal rendering** (refresh on every state change):
   - **Two-row ASCII piano keyboard** — ~5 chars per white-key section. Pressed keys highlighted (inverted background via crossterm styling). Latched keys shown with a distinct style (e.g. bold + underline). Each key labelled with its note name (C, C#, …) below:
     ```
     |   |W |   |E |   |   |T |   |Y |   |U |   |
     | A | S | D | F | G | H | J | K | L | ; |
     | C | D | E | F | G | A | B | C | D | E |
     ```
   - **Status bar** above the keyboard:
     ```
     Instrument: 3 (Flute)  |  Octave: 3  |  Sine: ON  |  H2: 0.30  H3: 0.10
     Pitch: 36  Period: 428 → 425  Freq: 8362.7 Hz  [MODIFIED]
     [1-9]/[/]=instr  Z/X=oct  Tab=sine  Shift+key=latch  Arrows=bend  F5/F6=tune  Q=quit
     ```
   - When no note is active, the pitch/period line shows `(no active note)`.
   - Reuse `INSTRUMENT_NAMES` array from `music_viz` verbatim for the status display.

### Decisions

- Self-contained private synth (no changes to `game::audio`) — consistent with `music_viz`, zero risk to game code
- Crossterm terminal rendering — matches `music_viz` aesthetic
- Sine wave mode lives inside the keyboard tool — no separate binary needed
- Configurable H2 + H3 harmonics — richer tuning signal than pure sine alone
- Both `1`–`9` and `[`/`]` for instrument switching
- Round-robin polyphony across 4 voices — matches Paula hardware voice count
- Latch (Shift+key) rather than a global sustain toggle — allows selective per-key sustain
- Arrow keys re-pitch the most-recently-latched voice — natural for single-note tuning workflows
- PTABLE edits are exported as a Rust const on exit — zero-friction workflow for testing changes in-game

### Verification

- `cargo build --bin vkbd` compiles cleanly
- Pressing `A` plays C at the selected octave; releasing silences it
- Up to 4 keys held simultaneously each sustain independently
- `Shift`+`A` latches C — it keeps sounding after release; pressing `A` again unlatches and silences it
- While C is latched, `→` shifts to C#, `→` again to D; `↑` jumps to nearest black key, `↓` to nearest white key
- `Z`/`X` shifts the octave and note labels update
- `1`–`9` and `[`/`]` both switch instruments with audible timbre change
- `Tab` switches to sine mode; key C plays a pure sine at the correct PTABLE frequency
- `F2` + `+`/`-` adjusts H2 amplitude; tone changes in real time without clicking
- While a note sustains, `F5`/`F6` adjusts the period and the pitch shifts audibly; status bar shows old → new period and frequency
- On quit with modified PTABLE, a valid Rust `const PTABLE` array is printed to stdout with change annotations

---

## Plan: Game World & Map System

**Status:** Not started

### Overview

Load and render the game world from the `game/image` ADF disk image. This includes the tile graphics, map sector data, terrain data, scrolling viewport, and the main play UI frame. The data pipeline closely mirrors the original Amiga code in `fmain.c`, `fsubs.asm`, and `hdrive.c`.

Reference notes moved to `RESEARCH.md`:
- `Game World & Map System: Data format`
- `Game World & Map System: Constants, addresses, and implementation notes`

### Steps

1. **ADF reader module** (`src/game/adf.rs`)
   - Struct `AdfDisk` wraps `Vec<u8>` (the 880 KB `game/image` bytes)
   - `load_blocks(f_block: u32, count: u32) -> &[u8]`: returns a slice at `[f_block*512 .. (f_block+count)*512]`
   - Parse `faery.toml` (or hard-code) the block offsets for region F3 as the starting region (region_num=3, `current_loads = {0,0,0,0,1,2,0,0,0}` = sector block 0, region block 0 which are placeholder defaults — actual starting values are in `file_index[3]`)
   - Add `game/image` path to `GameLibrary` / `faery.toml`

2. **Load starting region data** (`src/game/world_data.rs`)
   - `WorldData` struct: owns `sector_mem: [u8; 32768]`, `map_mem: [u8; 4096]`, `terra_mem: [u8; 1024]`, `image_mem: Box<[u8; 81920]>`
   - `WorldData::load(adf, region_index)`: reads the 4 tilesets and associated map data for region `file_index[region_index]`
   - Port the `load_new_region()` logic faithfully including the async-equivalent sequencing (all loads are synchronous in our Rust port)

3. **Build tile atlas texture** (`src/game/tile_atlas.rs`)
   - From `image_mem`, decode all 256 tiles (4 groups × 64 tiles) into RGBA32 pixels using the region's palette
   - Pack them into an SDL2 texture atlas: 16 tiles per row × 16 rows = 256 tiles, each 16 × 32 px → atlas = 256 × 512 px
   - `TileAtlas::tile_src_rect(tile_idx: u8) -> Rect` for SDL2 `copy()` calls
   - `TileAtlas::rebuild(image_mem, palette)` for re-palette on region transition

4. **`genmini` port** (`src/game/map_view.rs`)
   - `fn genmini(img_x: u16, img_y: u16, map_mem: &[u8], sector_mem: &[u8]) -> [u16; 114]`
   - Direct port of the asm logic: `secx = (img_x >> 4).wrapping_sub(xreg)` with wrapping/clamping, `secy = (img_y >> 5) - yreg` clamped 0–31, `sec_num = map_mem[secx + secy*128]`, then `tile_idx = sector_mem[sec_num*128 + (img_y&7)*16 + (img_x&15)]`
   - Initially hard-code `xreg`/`yreg` as 0; proper region tracking comes later

5. **Map rendering** (`src/game/map_view.rs` or `gameplay_scene.rs`)
   - Given `minimap[114]`, blit 19 × 6 tiles to `play_tex` using `TileAtlas`
   - Each tile: dst rect = `Rect::new(col*16 + 16, row*32, 16, 32)` (x-offset 16 px to match original `vp_page.DxOffset`)
   - Pixel-accurate scroll: sub-tile pixel offset from `map_x % 16` / `map_y % 32` shifts all blit destinations
   - On each frame: recompute minimap from current hero position, blit tiles, draw `hiscreen` overlay

6. **HiScreen overlay and compass** (`src/game/gameplay_scene.rs`)
   - Load `game/hiscreen` as an `IffImage` (640 × 57 px, already supported by `iff_image.rs`)
   - Render it to the bottom strip of the 640 × 480 canvas below `play_tex` (at y = 480 − 57 = 423 or matching original `PAGE_HEIGHT = 143`)
   - Exact y-position: `vp_page` starts at y=0 with height 143; `vp_text` at `PAGE_HEIGHT` = 143, height 57 → canvas rows 143–200 (at 1× lores scale) or equivalent at our 2× scale
   - **Compass animation**: the compass needle in the hiscreen rotates based on `hero_dir` and `compass_anim`. Load the compass sprite frames from game data and blit the appropriate frame each tick.
   - **UI buttons**: the hiscreen contains clickable inventory/action button areas; wire these into the `KeyBindings` / `GameAction` dispatch (see Key Bindings plan)

7. **Scrolling text viewport** (`src/game/gameplay_scene.rs`)
   - The `vp_text` region (below the play field) shows scrolling narrative text for look/talk/action results
   - Port `scroll_message()` / `map_message()` from `fmain.c`: append text lines, scroll up on overflow, fade in/out via palette
   - Text is rendered with the amber font into a dedicated render target and composited onto the canvas each frame

8. **Gameplay scene stub** (`src/game/gameplay_scene.rs`)
   - Wire `WorldData`, `TileAtlas`, and tile rendering into the existing `Gameplay` phase in `main.rs` (currently renders directly in the loop)
   - Implement `Scene` trait; place hero at starting coordinates from `file_index[3]`
   - Static render first (no movement), confirm tiles appear correctly
   - **Refactor `main.rs`**: SDL context, texture atlas setup, and asset loading have grown large in `main.rs`. Once `GameplayScene` exists, extract this boilerplate into a `GameEngine` or `App` struct to keep `main()` thin.

9. **Palette for regions**
   - Each region uses a different palette (outdoor colours differ from dungeon/indoor)
   - Identify palette block numbers from ADF or embed per-region palettes in `faery.toml`
   - Hook `TileAtlas::rebuild()` into region transitions

10. **Door / portal system** (`src/game/doors.rs`)
    - Parse `doorlist[86]` — 86 doors sorted ascending by `xc1`
    - Binary search (outdoor→indoor) and linear scan (indoor→outdoor) lookups
    - Entry/exit position calculation per door type (horizontal/vertical/cave)
    - Region transition on door crossing (`secs==1` → region 8, `secs==2` → region 9)
    - DESERT gate requiring 5 Gold Statues (`stuff[STATBASE] >= 5`)
    - Locked door resolution via `doorfind()` with key type matching
    - Reference: `RESEARCH.md` → Door / Portal System

11. **Extents and encounter zones** (`src/game/extents.rs`)
    - Parse `extent_list[22]` — axis-aligned trigger rectangles
    - AABB point-in-rect hero position detection
    - Random encounter spawning dispatch (etype < 50)
    - Forced encounter loading (etype >= 50)
    - Carrier loading (etype >= 60/70) for bird, turtle, dragon
    - Special zone triggers: spider pit, necromancer, astral plane, princess
    - Reference: `RESEARCH.md` → Extents and Encounter Zones

Reference notes moved to `RESEARCH.md`:
- `Game World & Map System: Constants, addresses, and implementation notes`

---

## Plan: Player & Movement

**Status:** Not started

### Overview
Implement character movement, terrain interaction, and the basic player command system.

### Steps

1. **Player movement**
   - Mouse-driven directional movement (decode mouse position relative to player)
   - Walking animation sprites
   - Movement speed and terrain effects (blocked, slowed, sinking)

2. **Terrain system**
   - Terrain type lookup from map data
   - Research and identify the original terrain collision mask source (tile bitplane vs separate mask plane) from `original/` and game data
   - Blocked tiles, water/swamp sinking, bush slowdown
   - Path validation

3. **Player commands**
   - Look, Give, Get, Yell, Ask, etc. — port `do_command()` from `fmain.c`
   - Output strings via the scrolling text viewport (see Game World step 7)
   - Object/NPC interaction triggers

4. **Character state**
   - Three brothers (Julian, Phillip, Kevin) with sequential lives
   - Health, inventory, quest flags
   - Death → revive as next brother → placard → map repositioning

5. **Terrain collision mask research**
   - Trace the original collision/blocking path in `fmain.c`/asm helpers to confirm exactly which data controls blocked movement
   - Verify whether blocking uses one of the rendered terrain bitplanes or a dedicated mask/flag plane
   - Document findings in `DECODE.md` (or the relevant world-data section) before finalizing movement collision implementation

6. **Swan flight mechanic**
   - Implement swan flight traversal behavior gated by inventory possession of the lasso
   - If lasso is absent, swan flight cannot be initiated
   - Preserve original movement constraints/timing while in flight

7. **Raft riding mechanic**
   - Require explicit boarding by stepping onto the raft tile/object
   - While boarded, allow free traversal across water bodies consistent with original behavior
   - Handle boarding/unboarding transitions with original-faithful collision and movement rules

8. **Turtle egg rescue and shell reward**
   - Implement turtle egg rescue interaction (eggs threatened by snakes)
   - On successful rescue, grant shell inventory item
   - Shell item is required for turtle summon behavior

9. **Summoned turtle traversal + exploit guardrails**
   - Implement turtle summon from shell usage
   - Summoned turtle traversal mirrors raft-on-water behavior (rideable on bodies of water)
   - Concession fixes for known exploits:
     - Disallow repeated player attacks against the turtle while riding (prevents infinite BRV gain)
     - Prevent turtle push-through across impassable terrain and lava (no mountain/lava bypass or palace sequence skip)

10. **Hunger game mechanic**
   - Implement hunger progression and its gameplay effects with original-faithful behavior
   - Hook hunger state into the player status/update loop
   - Ensure hunger interactions are reflected through existing command/UI pathways

11. **Fatigue game mechanic + sleep boundary guardrail**
   - Implement fatigue progression and forced sleep behavior with original-faithful timing/state transitions
   - Apply explicit concession fix for known exploit:
     - Do not allow forced sleep at locked gates/doors to wake the player on the opposite side (no lock bypass via sleep transition)

12. **Safe spawn tracking and luck-gated respawn**
   - Track and update the player's last safe world position during movement
   - Define safe position as the most recent non-liquid terrain coordinate
   - On player death, if luck is sufficient, respawn at the tracked safe position

13. **Inventory system** (`src/game/inventory.rs`)
    - `stuff: [u8; 35]` per character (Julian, Phillip, Kevin)
    - Port `itrans[]` translation table (ob_id → stuff[] index) for item pickup
    - Weapon equip via USE menu → sets `anim_list[0].weapon`
    - Port `set_options()` menu enable/disable refresh from `stuff[]`
    - Item display screen rendering with `inv_list[]` layout metadata
    - Gold pickup adds to `wealth` variable directly (not stuff[])
    - Reference: `RESEARCH.md` → Inventory System

14. **Magic item system** (`src/game/magic.rs`)
    - 7 consumable magic items via MAGIC menu (stuff[9–15])
    - Blue Stone: stone ring teleport (hero_sector==144, `stone_list[11]`)
    - Green Jewel: `light_timer += 760`, illumination palette boost
    - Glass Vial: heal `rand8()+4` vitality (capped at `15 + brave/4`)
    - Crystal Orb: `secret_timer += 360`, reveal hidden passages (region 9 palette 31)
    - Bird Totem: world minimap display (`bigdraw()`), overworld only
    - Gold Ring: `freeze_timer += 100`, time stop for all non-hero figures
    - Jade Skull: mass kill (race < 7), `brave--` per kill
    - All blocked in astral plane (`extn->v3 == 9`)
    - Timer tick management in gameplay loop
    - Reference: `RESEARCH.md` → Inventory System (Magic consumables)

---

## Plan: NPC System

**Status:** Not started

### Overview
Port the NPC behavior system — goals, tactics, movement, and interaction.

### Steps

1. **NPC data loading**
   - Extract NPC table from `fmain` data segments
   - Object list structure: position, type, state, goal, tactic

2. **NPC behavior**
   - Goal-based AI (wander, guard, follow, attack)
   - Tactic execution (move toward goal, flee, patrol)
   - Interaction responses (ask, give triggers)

3. **Combat**
   - Player attack mechanics
   - NPC attack/response
   - Damage calculation, death handling

4. **Encounter spawning system** (`src/game/encounters.rs`)
   - `encounter_chart[11]` data table (hp, arms, cleverness, treasure, file)
   - `weapon_probs[]` weapon selection table (4 possible weapons per enemy type)
   - Enemy spawn logic: select type, arm with weapon, set goal/tactic
   - `mixflag & 2` type blending for encounter variation
   - Wire into extent zone triggers (from world-111)
   - Set correct sprite file via cfiles[] index
   - Reference: `RESEARCH.md` → Enemy Types (Encounter Chart)

5. **Missile / arrow combat** (`src/game/missiles.rs`)
   - `missile_list[6]` — 6 active missiles with position, type, speed, direction, archer
   - Player Bow: consumes arrows (stuff[8]), spawns missile_type=1, speed=3
   - Player Wand: missile_type=2, speed=5, no arrow consumption
   - NPC archer AI firing via SHOOT tactic
   - Movement: `speed * 2` pixels/tick, expire after 40 ticks or terrain collision (tiles 1, 15)
   - Damage: `rand8() + 4`, hit radius 6 (arrow) or 9 (bolt)
   - Reference: `RESEARCH.md` → Combat System (missile section)

6. **Enemy loot and treasure** (`src/game/treasure.rs`)
   - `treasure_probs[]` loot table (5 rows × 8 columns)
   - Body search: loot selection via `encounter_chart[race].treasure`, weapon drop, arrow bonus
   - Container opening: Chest/Urn/Sacks with `rand4()` loot rolls
   - Auto-equip logic for better weapons from drops
   - Reference: `RESEARCH.md` → Inventory System (loot sections)

7. **Shop / buy system** (`src/game/shop.rs`)
   - Shopkeeper (race 0x88) proximity detection triggers BUY menu
   - `jtrans[]` cost table: Food(3), Arrows(10), Vial(15), Mace(30), Sword(45), Bow(75), Totem(20)
   - Food purchase calls `eat(50)` directly instead of inventory storage
   - Gold deduction from `wealth`, menu options gated by available gold
   - GIVE menu for beggars: 2gp, chance to increase `kind`
   - Reference: `RESEARCH.md` → Inventory System (BUY menu)

---

## Plan: Graphics Effects

**Status:** Partially started (palette fading infrastructure complete)

### Overview
Visual effects that enhance the game atmosphere.

### Steps

1. **Day/Night cycle**
   - `GameClock` already tracks day phases (Midnight/Morning/Midday/Evening)
   - `fade_page()` with `limit=true` already supports night floor limits, blue tint injection, vegetation boost, and Green Jewel light illumination via `light_timer` — matching the original `day_fade()` from `fmain2.c`
   - `FadeController` can drive day/night transitions over time
   - Remaining: wire `day_fade()` into the gameplay loop using `lightlevel = daynight / 40` with the staggered RGB offsets (r-80, g-61, b-62) from the original

2. **Copper list parsing**
   - Amiga copper lists define per-scanline palette changes (sky gradients, water effects)
   - Parse and simulate these effects

3. **Witch effect**
   - Screen distortion effect when encountering the witch

4. **Teleport effect**
   - Visual transition when using teleport items/locations

5. **Sprite / shape file loading** (`src/game/sprites.rs`)
   - `cfiles[18]` table mapping logical sprite IDs to ADF block locations
   - Bitplane-to-RGBA32 decode for sprite frames (5 planes + 1 mask plane)
   - `setfig_table[14]` mapping 14 NPC types to sprite cfile/base frame/can_talk
   - `seq_list[]` slot management (PHIL, OBJECTS, RAFT, ENEMY, SETFIG, CARRIER, DRAGON)
   - On-demand sprite loading (enemy sprites change per encounter type)
   - Multi-width sprites (dragon 3×40, bird 4×64)
   - Reference: `RESEARCH.md` → Sprite / Shape File Layout (ADF)

---

## Plan: Key Bindings

**Status:** Not started

### Overview

Implement a rebindable key binding system based on the original game's keyboard commands. The original uses a flat `letter_list[38]` lookup table mapping ASCII keys to `(menu, choice)` pairs, plus special-cased direction and fight keys. Our port replaces this with a `GameAction` enum and a `KeyBindings` map that users can customize. Bindings are persisted via `settings.rs` to `~/.config/faery/settings.toml`.

Reference notes moved to `RESEARCH.md`:
- `Key Bindings: Original game key map`

### Implementation Steps

1. **Define `GameAction` enum**
   - Create `src/game/key_bindings.rs`
   - Enum variants for every bindable action: `MoveUp`, `MoveDown`, `MoveLeft`, `MoveRight`, `MoveUpLeft`, `MoveUpRight`, `MoveDownLeft`, `MoveDownRight`, `Fight`, `Pause`, `Inventory`, `Take`, `Look`, `UseItem`, `Give`, `Yell`, `Speak`, `Ask`, `Map`, `Find`, `Quit`, `LoadGame`, `SaveGame`, `ExitMenu`, `CastSpell1`..`CastSpell7`, `UseSlot1`..`UseSlot7`, `UseSpecial`, `BuyFood`, `BuyArrow`, `BuyVial`, `BuyMace`, `BuySword`, `BuyBow`, `BuyTotem`, `SelectKey1`..`SelectKey6`
   - Derive `Serialize`, `Deserialize`, `Hash`, `Eq`, `Clone`, `Debug`

2. **Define `KeyBindings` struct**
   - `HashMap<GameAction, Vec<Keycode>>` — each action maps to one or more physical keys
   - `fn default_bindings() -> KeyBindings` — populate with the original mapping (see `RESEARCH.md` key map), using modern keyboard equivalents:
     - Arrow keys → movement (original used joystick dirs 20-29; map to `Up`/`Down`/`Left`/`Right`)
   - Numpad `1`–`9` mapped to movement directions (original parity)
     - WASD as alternate movement keys (new convenience)
     - F-keys for magic spells
     - Letter keys for menu shortcuts
   - `fn action_for_key(keycode: Keycode) -> Option<GameAction>` — reverse lookup (build on demand or cache)
   - `fn set_binding(action: GameAction, keys: Vec<Keycode>)` — override a binding
   - `fn reset_to_defaults()` — restore original mapping

3. **Integrate into `GameSettings`**
   - Add `key_bindings: KeyBindings` field to `GameSettings`
   - `KeyBindings` implements `Serialize`/`Deserialize` so it persists to `settings.toml`
   - On load: merge saved bindings with defaults (so new actions added in updates get default keys)
   - On save: only write bindings that differ from defaults (keeps the file clean)

4. **Wire into event handling**
   - In `Scene::handle_event()`, translate `Event::KeyDown { keycode, .. }` through `KeyBindings::action_for_key()`
   - Scenes receive `GameAction` instead of raw keycodes (or both, for flexibility)
   - Direction keys: accumulate into a direction vector (original supports 8 directions via simultaneous key-down/key-up)
   - Fight key: press = start fighting, release = stop (key-down/key-up pair)

5. **Rebinding UI** (future)
   - Settings screen listing all actions with current key assignments
   - Select an action → "Press a key…" prompt → capture next keypress → update binding
   - Conflict detection: warn if key is already bound to another action
   - "Reset to Defaults" button
   - This is a later milestone; for now, users can edit `settings.toml` directly

6. **Game controller support**
   - Add a controller layer that maps physical pad inputs to existing logical `GameAction`s (no controller-only actions)
   - Keep gameplay parity with original one-button joystick by treating one face button as primary `Fight/Use` and mapping extra buttons to existing keyboard actions (menu shortcuts, pause, map, etc.)
   - Support D-pad and left stick for 8-direction movement, with configurable deadzone and digital/analog preference

Reference notes moved to `RESEARCH.md`:
- `Key Bindings: Design and compatibility notes`

---

## Plan: Persistence

**Status:** Not started

### Overview
Save and load game state.

### Steps

1. **Define save format**
   - Protobuf schema for game state (player position, inventory, quest flags, game clock, brother state)

2. **Save implementation**
   - Serialize game state to file in `~/.config/faery/` (same dir as settings)

3. **Load implementation**
   - Deserialize and restore game state
   - Validate save file integrity

---

## Plan: Core Game Loop

**Status:** Not started

### Overview

The core game loop is the central orchestration layer that ties together all gameplay subsystems into a faithful port of the original `while (!quitflag)` main loop from `fmain.c`. It introduces two key structures:

- **`GameState`** — the single source of truth for all mutable gameplay state (hero position, timers, counters, NPC list, flags). Mirrors the original's flat globals.
- **`GameplayScene`** — implements the `Scene` trait and runs the per-frame update sequence in the exact order the original game executes it.

Most subsystems (world loading, NPCs, combat, etc.) already have their own plans. This plan covers the **frame orchestration**, **game state container**, **entity list management**, **timer logic**, **death/revive cycle**, and the **stub/extension points** that later subsystems plug into.

### Design Principles

1. **Behavioral fidelity, not structural mimicry**: "faithful reproduction" means the game *plays* like the original — same timing, same observable outcomes, same feel. It does not mean copying the original's code structure. Where a cleaner or more efficient implementation achieves the same behavior, prefer it.
2. **Frame-order fidelity**: the per-frame update sequence preserves the original's execution order where it affects observable behavior. The ordering matters for things like timer checks gating later checks within the same frame — reordering those would change gameplay.
3. **Unified actor model**: all on-screen entities (hero, enemies, setfigs, NPCs, carriers, objects) share a single `Actor` struct. The original uses `struct shape` for everything in `anim_list[]`; we keep that uniformity rather than splitting into per-type structs. Behavioral differences come from the `ActorKind` enum and the goal/tactic system, not from different data layouts.
4. **Stubbed subsystems**: systems not yet implemented (combat, NPC AI, encounters, etc.) are represented by no-op stub calls or empty match arms that can be filled in without restructuring the loop.
5. **`GameState` is the save boundary**: every piece of data that `savegame()` serializes lives in `GameState`. This makes persistence a simple serialize/deserialize of `GameState`.
6. **60 Hz tick-based**: the loop processes one logical tick per frame at 60 Hz (NTSC VBlank). `delta_ticks` from the existing `GameClock` drives frame advancement. Multiple ticks per frame are processed sequentially to handle catch-up.
7. **Keep it simple**: avoid ECS frameworks, event buses, or other heavy abstractions. The game is small — direct calls and flat loops are easier to reason about and debug. Use Rust idioms (enums, traits, iterators) where they simplify the code, not as an end in themselves.

### Frame Execution Order

Each logical tick executes these steps in order, matching `fmain.c` lines 1382–3190:

```
 1. cycle++; flasher++
 2. Process input → GameAction (via KeyBindings, stubbed initially)
 3. Process menu/command dispatch (do_option stub)
 4. Handle viewstatus modes (inventory screen, map view, message overlay)
 5. Decode direction from input → oldir
 6. Check pause (skip remainder if paused)
 7. Decrement timers: light_timer, secret_timer, freeze_timer
 8. Check fiery_death zone
 9. Player death check + fairy resurrection
10. Player state machine: input → WALKING/FIGHTING/SHOOTING/STILL
11. Entity update loop (i=0..anix): movement, terrain, collision, animation
12. Sleep/bed detection
13. Door transition check
14. Update hero_x/hero_y from entity list
15. Map scroll computation
16. If stationary frame:
    a. Sleep time acceleration (daynight += 63)
    b. daynight++ → lightlevel recalc → day/night palette
    c. Day period transition events
    d. Healing tick (every 1024 ticks)
    e. find_place() → extent/location check
    f. Encounter loading + spawning
    g. NPC proximity announcements
    h. AI tactics loop
    i. Battle start/end detection
    j. Safe zone + auto-eat (every 128 ticks)
    k. Music mood update (every 8 ticks)
    l. Hunger/fatigue increment (every 128 ticks)
17. Melee combat resolution
18. Missile update
19. do_objects() — place on-screen objects in entity list
20. Bubble sort entities by Y
21. Render: tiles → sprites → HiScreen overlay → text viewport
22. Present frame (SDL2 canvas.present via VSync)
```

### Steps

1. **`GameState` struct** (`src/game/game_state.rs`)
   - Central container for all mutable gameplay state, replacing the original's scattered globals.
   - **Hero fields**: `hero_x: u16`, `hero_y: u16`, `map_x: u16`, `map_y: u16`, `hero_sector: u16`, `hero_place: u16`, `vitality: i16`, `brave: i16`, `luck: i16`, `kind: i16`, `wealth: i16`, `hunger: i16`, `fatigue: i16`, `brother: u8` (1=Julian, 2=Phillip, 3=Kevin), `riding: i16`, `flying: i16`
   - **Timers**: `light_timer: i16`, `secret_timer: i16`, `freeze_timer: i16`
   - **Cycle counters**: `daynight: u16` (0–24000 wrapping), `lightlevel: u16` (derived triangle 0–300–0), `cycle: u32`, `flasher: u32`
   - **Flags**: `battleflag: bool`, `quitflag: bool`, `witchflag: bool`, `safe_flag: bool`, `actors_on_screen: bool`, `actors_loading: bool`
   - **View state**: `viewstatus: u8` (0=normal, 1=map, 2=message, 3=fade-in, 4=inventory, 98/99=redraw), `cmode: u8` (current menu mode)
   - **Safe respawn**: `safe_x: u16`, `safe_y: u16`, `safe_r: u8`
   - **Region**: `region_num: u8`, `new_region: u8`
   - **Per-brother inventory**: `julstuff: [u8; 35]`, `philstuff: [u8; 35]`, `kevstuff: [u8; 35]`, `stuff: *const [u8; 35]` (pointer swapped on brother change; in Rust, use an index or enum to select the active array)
   - **Actor list**: `actors: [Actor; 20]`, `anix: usize` (active combat actor count), `anix2: usize` (total including objects)
   - **Encounter state**: `xtype: u16`, `encounter_type: u16`, `encounter_number: u8`
   - **Carrier/special**: `active_carrier: i16`, `actor_file: i16`, `set_file: i16`
   - **Princess/quest**: `princess: u8`, `dayperiod: u8`
   - **Music**: `current_mood: u8` (current song group)
   - `GameState::new()` initializes to Julian's starting state (matching `revive(TRUE)`)
   - `GameState::daynight_tick(&mut self)` — increment `daynight`, recompute `lightlevel` (triangle wave), return whether a day-period boundary was crossed

2. **`Actor` struct** (`src/game/actor.rs`)
   - Unified data structure for **all** on-screen entities: player, enemies, setfigs, NPCs, raft, carriers, dragons, and placed objects. The original uses a single `struct shape` for every `anim_list[]` slot; we do the same.
   - Fields: `abs_x: u16`, `abs_y: u16`, `rel_x: i16`, `rel_y: i16`, `kind: ActorKind`, `race: u8`, `state: ActorState`, `goal: Goal`, `tactic: Tactic`, `facing: u8` (0–7), `vitality: i16`, `weapon: u8`, `environ: i8`, `vel_x: i16`, `vel_y: i16`
   - `ActorKind` enum: `Player, Enemy, Object, Raft, SetFig, Carrier, Dragon` — determines which update behavior applies, not the data layout
   - `ActorState` enum: `Still, Walking, Fighting(u8)`, `Dying, Dead, Shooting(u8), Sinking, Falling, Sleeping` — the discriminant values match the original C constants where they affect save/load compatibility
   - `Goal` enum: `User, Attack1, Attack2, Archer1, Flee, Follower, Leader, Stand, Guard, ...`
   - `Tactic` enum: `Pursue, Shoot, Random, BumbleSeek, Backup, Follow, Evade, EggSeek, Frust, ...`
   - `Actor::is_active(&self) -> bool` — not Dead/removed
   - `Actor::clear(&mut self)` — reset to empty slot
   - All entity types share the same movement, collision, and rendering pipeline. Behavioral differences are handled by matching on `kind` at decision points (e.g., input source for Player vs. AI for Enemy), not by separate code paths for each type.

3. **`GameplayScene` struct** (`src/game/gameplay_scene.rs`)
   - Implements `Scene` trait. Owns `GameState`. Receives `GameClock` reference from `main.rs`.
   - `GameplayScene::new(game_state: GameState) -> Self`
   - **`handle_event(&mut self, event: &Event) -> bool`**: translates SDL events to `GameAction` via `KeyBindings` (stubbed initially: arrow keys → direction, space → pause, escape → quit). Stores pending direction and action for processing in `update()`.
   - **`update(...) -> SceneResult`**: runs the Frame Execution Order for each pending tick. If `quitflag` is set, returns `SceneResult::Done`.
   - **Rendering**: after all ticks are processed, renders the current frame:
     1. Blit tiles from `TileAtlas` via `genmini` (stubbed: solid color fill until world-001 is done)
     2. Sort and blit entity sprites (stubbed: placeholder rectangle for hero)
     3. Blit `hiscreen` overlay (stubbed: black bar)
     4. Blit text viewport (stubbed: empty)
   - The scene does NOT call `canvas.present()` — the existing `main.rs` loop handles that.

4. **Wire `GameplayScene` into `main.rs`**
   - When `ScenePhase::PlacardStart` completes, create `GameplayScene::new(GameState::new())` and set `active_scene = Some(Box::new(gameplay_scene))`.
   - Pass `GameClock` information via `delta_ticks` (already provided by the `Scene::update` signature).
   - The existing `main.rs` event pump, scene dispatch, and `canvas.present()` remain unchanged.
   - Remove the current placeholder rendering in the `else if dirty` branch — `GameplayScene` takes over all rendering during the Gameplay phase.

5. **Timer tick logic** (`GameState` methods)
   - `tick_timers(&mut self)`: decrement `light_timer`, `secret_timer`, `freeze_timer` (clamped at 0). Called at step 7 of the frame order.
   - `tick_daynight(&mut self) -> Option<DayPeriodEvent>`: increment `daynight` (skip if `freeze_timer > 0`), recompute `lightlevel`, detect `dayperiod` transitions, return event if boundary crossed. Called at step 16b.
   - `tick_healing(&mut self)`: if `daynight & 0x3FF == 0` and hero alive and vitality below max (`15 + brave/4`), increment vitality. Called at step 16d.
   - `tick_hunger_fatigue(&mut self) -> Vec<GameEvent>`: if `daynight & 127 == 0`, increment hunger/fatigue, check thresholds (messages at 35/60/90/100/130/140/160/170), apply vitality damage above thresholds, trigger forced sleep. Called at step 16l. Returns a list of events (messages, forced-sleep) for the scene to dispatch.
   - `tick_safe_zone(&mut self)`: if `daynight & 127 == 0` and no enemies on screen and alive, update `safe_x/safe_y/safe_r`. Called at step 16j.
   - All timer thresholds and modular-tick checks use the exact constants from the original.

6. **Death and revive cycle** (`GameState` methods + `GameplayScene` logic)
   - `GameState::check_death(&mut self) -> bool`: return true if hero vitality ≤ 0 and state ≠ Dead.
   - `GameState::revive(&mut self, first_time: bool)`: port of `revive()` from `fmain.c`:
     - If `first_time`: initialize Julian at starting coordinates (19036, 15755), set initial stats from `blist[0]`.
     - Otherwise: Place dead brother's body and ghost in object list. Advance `brother` (1→2→3→game over). Reset inventory to Dirk only. Set new brother's stats from `blist[brother-1]`. Position at `safe_x/safe_y`.
   - `GameplayScene` handles death by:
     1. Setting hero state to Dying → Dead (animation countdown)
     2. On Dead: if `brother < 3`, show placard ("Julian has fallen…"), then call `revive(false)`, set `viewstatus = 99` (full redraw)
     3. If `brother > 3`: set `quitflag = true` (game over)
   - The death placard reuses `PlacardScene` by temporarily swapping the active scene (or inlining a placard sub-phase within `GameplayScene`).

7. **Input direction decoding** (`GameplayScene` method)
   - `decode_direction(&self) -> Option<u8>`: convert pending input (keyboard arrows / numpad / mouse position relative to hero) to `oldir` (0–7 direction, or None for no input).
   - 8 compass directions: 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW (matching original).
   - Fight button state tracked as `fighting: bool` (key-down sets true, key-up sets false).
   - Initially wired to arrow keys + WASD. Full `KeyBindings` integration comes from keys-001.

8. **Player input → actor state** (`GameplayScene` method)
   - The player's `Actor` (slot 0) uses the same state machine as all other actors. The only difference is the *input source*: the player's direction and fight state come from user input rather than AI goals.
   - Each frame, `decode_direction()` (step 7) and the fight button state are translated into the player actor's `facing`, `goal`, and `tactic` fields — the same fields that AI populates for enemies.
   - The common actor update (step 9) then processes movement, collision, and state transitions identically for all actors.
   - State transitions follow original precedence: Dead > Sleeping > Sinking > Fighting > Walking > Still.
   - Walking: compute target position from facing and speed, call `proxcheck()` stub. If blocked, try ±1 direction deviation.
   - Fighting: advance through `trans_list[]` state transitions (animation frames driven by `rand4()`). Stubbed initially with a simple cycle.

9. **Actor update loop** (`GameplayScene` method)
   - `update_actors(&mut self)`: iterate `actors[0..anix]`, processing every actor through the same pipeline:
     1. Skip if `freeze_timer > 0` and actor is not the player (time-stop freezes all non-player actors)
     2. Resolve input: for `ActorKind::Player`, input comes from step 8; for others, from goal/tactic AI (stubbed as no-op until npc-001)
     3. Process state machine: Walking → movement + collision, Fighting → animation transitions, Dying → countdown, etc.
     4. Compute `rel_x / rel_y` (screen position): `rel_x = abs_x - map_x`, `rel_y = abs_y - map_y`
   - The loop body is the same for all actor kinds. AI-driven actors simply have their direction and actions set by the tactic system instead of user input.

10. **Map scroll and camera tracking** (`GameplayScene` method)
    - `update_camera(&mut self)`: recompute `map_x = hero_x - 144`, `map_y = hero_y - 90` (hero centered in viewport).
    - Compute scroll delta `dif_x = new_map_x - old_map_x`, `dif_y = new_map_y - old_map_y`.
    - If delta is zero → "stationary frame": run the step 16 sub-updates (day/night, encounters, hunger, etc.).
    - If delta is nonzero → "scrolling frame": skip step 16 sub-updates (matching original behavior where timers only advance on stationary frames).
    - Clamp camera to world bounds (`0..MAXCOORD - viewport_width`).

11. **`viewstatus` mode handling** (`GameplayScene` method)
    - `viewstatus` controls what's displayed:
      - `0` — normal gameplay rendering
      - `1` — world minimap (Bird Totem)
      - `2` — scrolling message overlay
      - `3` — fade-in after region transition
      - `4` — inventory screen
      - `98` — partial redraw
      - `99` — full map redraw (on region change, revive, etc.)
    - Each mode is a sub-state within `GameplayScene`. Initially only `0` and `99` are functional; others are stubbed as no-ops that immediately return to mode 0.
    - `viewstatus = 99` triggers a full `genmini()` + `map_draw()` on the next frame.

12. **Menu/command dispatch stub** (`GameplayScene` method)
    - `process_command(&mut self, action: GameAction)`: maps `GameAction` variants to the original `do_option()` logic.
    - Initially all commands print a debug message ("LOOK: not yet implemented") and return.
    - The `cmode` field tracks current menu context (Items/Magic/Talk/Buy/Game/etc.).
    - Filled in progressively as player-001 (commands), npc-001 (talk/give), and player-113 (inventory) are implemented.

13. **`setmood()` integration point** (`GameplayScene` method)
    - `update_mood(&mut self, audio: &AudioSystem)`: called every 8 ticks (step 16k).
    - Evaluates mood priority chain (death → palace → battle → indoor → day → night) and calls `audio.play_group()` if mood changed.
    - Initially stubbed to do nothing. Filled in when audio-105 is implemented.
    - The priority chain and coordinates are documented in `RESEARCH.md` → `setmood()`.

14. **Collision stub** (`src/game/collision.rs` or inline)
    - `fn proxcheck(x: u16, y: u16, entity_idx: usize, state: &GameState) -> u8`: returns 0 (passable) for now.
    - `fn px_to_im(x: u16, y: u16, state: &GameState) -> u8`: returns 0 (normal terrain) for now.
    - Filled in when world-001 (terrain data) and player-102 (terrain system) are implemented.

15. **`GameEvent` enum** (`src/game/game_event.rs`)
    - Lightweight event type for intra-frame communication (avoids coupling subsystems).
    - Variants: `Message(String)` (text viewport display), `PlaySound(u8)` (sound effect trigger), `DayPeriodChanged(DayPhase)`, `RegionTransition(u8)`, `BattleStart`, `BattleEnd`, `HeroDied`, `QuestUpdate(String)`, `ForcedSleep`
    - `GameplayScene` collects events during the tick and processes them after the update sequence (display messages, trigger audio, etc.).
    - Initially only `Message` is functional (prints to console until the text viewport is wired in).

### Dependencies

This plan depends on the existing infrastructure:
- `Scene` trait and `SceneResult` from [src/game/scene.rs](src/game/scene.rs) — already complete
- `GameClock` from [src/game/game_clock.rs](src/game/game_clock.rs) — already complete
- `AudioSystem` from [src/game/audio.rs](src/game/audio.rs) — already complete
- `PlacardScene` from [src/game/placard_scene.rs](src/game/placard_scene.rs) — already complete (reused for death/game-over placards)

This plan is a **prerequisite** for:
- world-108 (Gameplay scene stub) — superseded by step 3 of this plan
- player-101 (Player movement) — fills in step 8
- npc-001 (NPC system) — fills in step 9
- keys-104 (Wire into event handling) — fills in step 7 and step 2
- gfx-101 (Day/night gameplay wiring) — fills in step 16b/16c
- audio-105 (setmood) — fills in step 13
- persist-001 (Persistence) — serializes `GameState` from step 1

### Verification

- `cargo build` compiles cleanly with all new files
- After intro + copy protection + placard, the game transitions to `GameplayScene`
- A placeholder hero rectangle renders at starting coordinates (centered in viewport)
- Arrow keys move the hero rectangle; position updates in debug window
- Day/night timer advances (visible in debug window game clock)
- Hunger/fatigue counters increment over time (logged to console)
- Pressing Escape from gameplay quits the game
- Death (setting vitality to 0 via debug) shows the brother placard and revives as next brother
- Third death → game over

---

## Plan: Debug Window Enhancements

**Status:** Not started

### Overview

Expand the existing debug window from a read-only diagnostic display into a full development console with live game state inspection, stat/inventory editing, and cheat controls. The debug window is gated behind `--debug` and has no effect on normal gameplay.

All cheat operations modify `GameState` directly. The debug window reads a snapshot of `GameState` each frame (extending the existing `DebugState` pattern) and writes back mutations via a `DebugCommands` queue that `GameplayScene` drains at the start of each tick.

### Design

**Communication pattern**: The debug window cannot hold a mutable reference to `GameState` (it's a separate SDL2 window with its own renderer). Instead:
- **Read path**: `DebugState` (already exists) is extended with gameplay fields. `main.rs` populates it each frame from `GameState` — cheap copies of scalars and small arrays.
- **Write path**: `DebugWindow` accumulates `DebugCommand` values into a `Vec<DebugCommand>`. `main.rs` drains this vec and applies each command to `GameState` before the tick runs. This keeps the debug window decoupled from game internals.

**Tab layout**: Add new tabs to the existing tab bar. The current tabs (Info, Placards, Images, Tilemap, Map, Songs) remain unchanged.

| Tab | Key | Content |
|-----|-----|---------|
| Player | F7 | Hero stats, inventory, stat buttons, inventory buttons |
| Actors | F8 | Active actor list with expandable detail |
| Cheats | F9 | God mode, teleport, time control, hero pack, brother restart, insta-kill, autosave |

### Steps

1. **`DebugCommand` enum** (`src/game/debug_window.rs` or `src/game/debug_command.rs`)
   - Represents a mutation request from the debug window to the game state.
   - Variants:
     - `SetStat { stat: StatId, value: i16 }` — set a specific stat (vitality, brave, luck, kind, wealth, hunger, fatigue)
     - `AdjustStat { stat: StatId, delta: i16 }` — increment/decrement a stat
     - `SetInventory { index: u8, value: u8 }` — set a specific `stuff[]` slot
     - `AdjustInventory { index: u8, delta: i8 }` — increment/decrement an inventory slot
     - `TeleportSafe` — move hero to `safe_x/safe_y/safe_r`
     - `TeleportStoneRing { index: u8 }` — move hero to `stone_list[index]` (0–10)
     - `TeleportCoords { x: u16, y: u16 }` — move hero to arbitrary map coordinates
     - `ToggleMagicEffect { effect: MagicEffect }` — toggle a sticky magic effect (see step 8)
     - `HeroPack` — fill inventory with a useful adventurer loadout
     - `SetGodMode { flags: GodModeFlags }` — enable/disable god mode facets
     - `SummonSwan` — place swan carrier near hero, grant lasso if missing
     - `SetDayPhase { phase: DayPhase }` — jump `daynight` to the start of a day phase
     - `SetGameTime { hour: u8, minute: u8 }` — set `daynight` to a specific time
     - `HoldTimeOfDay { hold: bool }` — freeze `daynight` progression without pausing the game
     - `ToggleAutosave { enable: bool }` — enable/disable rolling autosave
     - `RestartAsBrother { brother: BrotherId }` — full game-state reset, start as the chosen brother
     - `InstaKill` — kill the active hero immediately, trigger the death/transition scene
   - `BrotherId` enum: `Julian, Phillip, Kevin`
   - `StatId` enum: `Vitality, Brave, Luck, Kind, Wealth, Hunger, Fatigue`
   - `MagicEffect` enum: `Light, Secret, Freeze` (maps to `light_timer`, `secret_timer`, `freeze_timer`)
   - `GodModeFlags`: bitflags struct with `NOCLIP`, `INVINCIBLE`, `ONE_HIT_KILL`, `INSANE_REACH`
   - `DebugWindow` exposes `fn drain_commands(&mut self) -> Vec<DebugCommand>`

2. **Extend `DebugState` with gameplay fields**
   - Add to the existing `DebugState<'a>` struct:
     - `hero_stats: Option<HeroStats>` — copied scalars: vitality, max_vitality, brave, luck, kind, wealth, hunger, fatigue, brother, riding, flying, hero_x, hero_y, hero_sector, hero_place, region_num
     - `inventory: Option<[u8; 35]>` — copy of the active brother's `stuff[]`
     - `actors: Option<&'a [(ActorKind, ActorState, u16, u16, i16, u8, u8)]>` — compact tuple slice of (kind, state, abs_x, abs_y, vitality, race, weapon) for the active `actors[0..anix2]`
     - `timers: Option<TimerSnapshot>` — light_timer, secret_timer, freeze_timer, daynight, lightlevel
     - `safe_pos: Option<(u16, u16, u8)>` — safe_x, safe_y, safe_r
     - `god_mode: GodModeFlags` — current god mode state
     - `time_held: bool` — whether time-of-day is frozen
     - `autosave_enabled: bool`
   - These are `Option` so the debug window gracefully handles pre-gameplay phases (when `GameState` doesn't exist yet).
   - `main.rs` populates these from `GameplayScene`'s `GameState` when in the Gameplay phase.

3. **Player tab** (F7)
   - **Stats panel** (top half):
     - Display: `VIT: 12/16  BRV: 35  LCK: 20  KND: 15  WLT: 20`
     - Display: `HGR: 45  FTG: 12  BRO: Julian  RGN: 3`
     - Display: `POS: (19036, 15755)  SEC: 144  PLACE: Village`
     - Each stat has a clickable button. Left-click emits `AdjustStat { delta: +1 }`, right-click emits `AdjustStat { delta: -1 }`. Shift+click emits `±10`.
     - Keyboard shortcuts while Player tab is focused: `V`=vitality, `B`=brave, `L`=luck, `K`=kind, `W`=wealth, `H`=hunger, `F`=fatigue — then `+`/`-` or Up/Down to adjust.
   - **Inventory panel** (bottom half):
     - Grid of all 35 `stuff[]` slots with item names and counts.
     - Each slot shows: `[idx] Name: count`
     - Item names from a `const ITEM_NAMES: [&str; 35]` lookup table.
     - Left-click emits `AdjustInventory { delta: +1 }`, right-click emits `AdjustInventory { delta: -1 }`. Value clamped 0–255.
     - Scrollable if the window is too small to show all 35 items.

4. **Actors tab** (F8)
   - List all active actors (`actors[0..anix2]`), one row per actor:
     - `[slot] Kind  State  Pos(x,y)  VIT  Race  Wpn`
     - Example: `[0] Player  Walking  (19036,15755)  12  0  Sword`
     - Example: `[3] Enemy   Fighting (19100,15700)   5  4  Mace`
   - Color coding: Player=green, Enemy=red, SetFig=yellow, Object=grey, Carrier=cyan
   - **Expandable detail**: press Enter or click on a row to expand it, showing:
     - Full fields: goal, tactic, facing, environ, vel_x, vel_y
     - For actors with inventory (e.g., dead brothers with lootable bodies): show their `stuff[]`
   - Up/Down arrows to navigate the list, Enter to expand/collapse.

5. **Cheats tab — stat and inventory shortcuts** (F9, top section)
   - **Hero Pack button**: one-click fills inventory with a useful loadout:
     - 1× each weapon (Dirk, Mace, Sword, Bow, Wand), 50 Arrows
     - 1× Golden Lasso, Sea Shell, Sun Stone
     - 3× each magic consumable (Blue Stone, Green Jewel, Glass Vial, Crystal Orb, Bird Totem, Gold Ring, Jade Skull)
     - 1× each key (Gold, Green, Blue, Red, Grey, White)
     - Sets wealth to 200
     - Does NOT grant quest items (Talisman, Rose, Gold Statues) — those should be earned
   - **Max Stats button**: sets vitality to max (15 + brave/4), zeroes hunger and fatigue

6. **Cheats tab — teleport controls** (F9, middle section)
   - **Teleport to Safe**: button emits `TeleportSafe`. Shows current safe coords.
   - **Stone Ring dropdown**: numbered list 0–10 with sector coordinates from `stone_list[]`. Click a row to teleport there. Also handles region transition to overworld if currently indoors.
   - **Coordinate entry**: text input field for `x,y` (e.g., `19036,15755`). Press Enter to teleport. Validates range 0–32767.

7. **Cheats tab — god mode** (F9, middle-lower section)
   - Four toggle buttons, each independently controllable:
     - **Noclip**: walk through walls, across water, over lava without sinking or terrain damage. Bypasses `proxcheck()` terrain collision (entity-entity collision still applies).
     - **Invincible**: hero vitality cannot decrease below 1. Damage events are ignored.
     - **One-Hit Kill**: any hero melee or missile hit sets target vitality to 0 (instant kill). Applies **only** to actors with `kind == Enemy`. SetFigs, carriers, and objects are unaffected.
     - **Insane Reach**: hero melee hit detection range extended to screen edge (~144px). Applies **only** to actors with `kind == Enemy`.
   - Each button shows its current state (ON/OFF) with color (green/grey).
   - A master **God Mode** toggle enables/disables all four at once.
   - `GodModeFlags` is checked in `GameplayScene`'s collision, damage, and combat resolution code.

8. **Cheats tab — magic effect toggles** (F9, right section)
   - Three toggle buttons: **Light**, **Secret**, **Freeze**
   - Left-click: apply the magic effect once (same as using the item: adds the standard timer increment)
   - Right-click: decrement the timer by the same amount (for fine-tuning timer values)
   - ALT+click (or keyboard ALT+L/S/F): **sticky toggle** — the timer is locked at its maximum value and does not decrement. The effect persists until toggled off. When toggled off, the timer resumes normal decrement from its current value.
   - Display shows: `Light: 760 [STICKY]` or `Secret: OFF`
   - Implementation: `GameState` gets three `bool` fields (`light_sticky`, `secret_sticky`, `freeze_sticky`). The timer decrement logic in `tick_timers()` skips decrement when the corresponding sticky flag is set.

9. **Cheats tab — swan summon** (F9)
   - **Summon Swan** button: places the bird carrier actor at hero position offset by +20px to the right (or left if rightward is blocked). Sets `active_carrier` to the swan file ID. If `stuff[5]` (Golden Lasso) is 0, automatically grants one.
   - Emits `SummonSwan` command.

10. **Cheats tab — brother restart** (F9)
    - Three buttons: **Julian**, **Phillip**, **Kevin**.
    - Click emits `RestartAsBrother { brother }`. `GameplayScene` performs a full `GameState` reset: clears all inventory, resets stats to the chosen brother's starting values, resets quest flags, clears all non-player actors, and positions the hero at the brother's starting location. No corpse or ghost is spawned — this is a clean restart, not a death transition.
    - A confirmation prompt ("Restart as Julian? This resets all progress.") is shown before emitting the command.

11. **Cheats tab — insta-kill** (F9)
    - **Kill Hero** button. Click emits `InstaKill`. Sets the active hero's vitality to 0, then invokes the normal death scene and brother transition logic (same code path as a real death: corpse placed, ghost walks to Dorian's tomb, next brother takes over — or game over if Kevin dies).
    - Useful for testing the death/revive cycle without needing to find an enemy.

12. **Cheats tab — time controls** (F9, bottom section)
    - **Day phase jump buttons**: Midnight, Morning, Midday, Evening — each emits `SetDayPhase` which sets `daynight` to the corresponding value (0, 8000, 12000, 18000). Left-click jumps forward to the next phase; right-click jumps backward to the previous phase.
    - **Set time**: text input for `HH:MM` (24hr format). Enter emits `SetGameTime`.
    - **Hold Time**: toggle button. When active, `daynight` is not incremented by the game loop (but the game otherwise runs normally — actors move, combat works, etc.). Emits `HoldTimeOfDay { hold: true/false }`.
    - Display shows current time: `Time: Day 2 14:30 [HELD]` or `Time: Day 2 14:30`.

13. **Autosave system** (Cheats tab toggle + `GameplayScene` logic)
    - **Toggle button** in the Cheats tab: `Autosave: ON/OFF`.
    - When enabled, `GameplayScene` saves the game state at a configurable interval (default: every 120 seconds of real time, or on region transition).
    - **Rolling savefiles**: maintains up to N backup slots (default N=5). Files named `autosave_0.sav` through `autosave_4.sav`. On each autosave, rotate: `4→delete`, `3→4`, `2→3`, `1→2`, `0→1`, write new `0`.
    - Save directory: `~/.config/faery/saves/` (same as manual saves from persist-001).
    - Autosave triggers are logged to the debug window's log panel.
    - **Consider promoting to a user-facing feature**: autosave with rolling backups is useful for all players, not just debugging. The implementation should be clean enough to expose via the Game menu (with a settings toggle) once persist-001 is complete. The debug toggle provides early access before the Game menu exists.
    - Depends on persist-001 (save format and serialization) for the actual file I/O. Until persist-001 is done, the toggle exists but autosave is a no-op with a log message.

### Dependencies

- `GameState` from gameloop-001 step 1 — required for all read/write operations
- `Actor` from gameloop-001 step 2 — required for the Actors tab
- `GameplayScene` from gameloop-001 step 3 — processes `DebugCommand` queue
- Death/revive cycle from gameloop-001 step 10 — required for `InstaKill` and `RestartAsBrother`
- persist-001 — required for autosave file I/O (autosave is a no-op stub until then)
- Existing `DebugWindow` infrastructure in [src/game/debug_window.rs](src/game/debug_window.rs) — extended, not replaced

### Verification

- New tabs (F7/F8/F9) render without crashing in pre-gameplay phases (show "Not in gameplay" placeholder)
- Player tab shows live stats and inventory that update each frame
- Clicking `[+]`/`[-]` on vitality changes the hero's HP visibly in-game
- Hero Pack fills inventory; items appear in the inventory screen
- God Mode Noclip: hero walks through walls
- God Mode Invincible: hero takes no damage
- God Mode One-Hit Kill: enemies die on first hit; setfigs and carriers are unaffected
- Stone Ring teleport: hero appears at the correct overworld coordinates
- Coordinate teleport: hero moves to typed location
- Hold Time: day/night cycle freezes while actors continue to move
- ALT+Light toggle: green jewel illumination persists indefinitely
- Autosave toggle logs "autosave: not yet implemented" until persist-001 is done
