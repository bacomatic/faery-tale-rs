## Plan: Intro Sequence End-to-End

**TL;DR:** Build just enough scene infrastructure to drive the full intro — from title text through story pages to copy protection and the first placard — then wire it into the existing event loop. This gives you a playable sequence and a reusable scene system for everything that follows.

### Completed

1. **Scene system** — `Scene` trait in `src/game/scene.rs` with `handle_event()` + `update(canvas, play_tex, delta_ticks, game_lib, resources) -> SceneResult`. `SceneResources` bundles `&mut [ImageTexture]`, `&HashMap<String, usize>`, and font textures. Both `find_image()` (read) and `find_image_mut()` (write, for palette re-rasterization) are available. Scenes chain via `ScenePhase` enum in `main.rs`: Intro → CopyProtect → PlacardStart → Gameplay.

2. **Palette fading** — Hybrid system in `src/game/palette_fader.rs`:
   - **`fade_page()`**: Port of the original C `fade_page()` from `fmain2.c`. Per-channel multiplicative scaling (0–100%), night-time floor limits (r≥10, g≥25, b≥60), blue tint injection for moonlight, vegetation color boost (indices 16–24), and torch/spell illumination (light_timer boosts red to green's level). Returns a new `Palette`.
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

2. ~~**SDL2 mixer integration**~~ Done — `AudioSystem` in `src/game/audio.rs`. Pure-Rust, no `unsafe`, 4-voice software synthesizer porting `gdriver.asm` note-trigger + envelope logic. `Instruments` loads waveforms and ADSR envelopes from `game/v6` (envelopes at byte 2048, matching the original `Seek(+S_WAVBUF, OFFSET_CURRENT)` load sequence). `SequencerState` drives 4 `Voice`s with timeclock stepping (150 tempo, 60 Hz NTSC VBL). `SynthCallback` fires a VBL tick every 735 samples (~44100/60) and mixes voices into a 44100 Hz f32 mono stream. Per-voice rendering uses linear interpolation with correct modulo loop-wrap (avoids click on every waveform cycle) and a 1-pole IIR low-pass at ~4800 Hz approximating the A500 hardware RC filter. Voices mixed at ¼ scale to match four-channel headroom. Intro music (tracks 12–15) plays automatically at startup. 13 tests.

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
- Minor click between notes. Likely a phase discontinuity when `trigger_note()` resets `phase` to 0.0 mid-cycle without crossfading to the new waveform, or a misaligned VBL boundary when the sequencer fires inside a partially-rendered chunk. Music is otherwise correct and verified against original. Address in a future fine-tuning pass.

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

### Data Format Reference

All game world data lives in `game/image`, an Amiga 880KB floppy disk image accessed as a flat file. `load_track_range(block, count, buf)` reads `count × 512` bytes from offset `block × 512`.

**Memory regions loaded from `game/image`:**

| Buffer | ADF source | Size | Description |
|---|---|---|---|
| `sector_mem` | `nd->sector`, 64 blocks | 32 768 B | 128 sectors × 256 bytes; tile indices |
| `map_mem` | `nd->region`, 8 blocks | 4 096 B | Region map; maps `(secx, secy)` → sector number |
| `terra_mem[0..512]` | `TERRA_BLOCK + nd->terra1`, 1 block | 512 B | Terrain type table, layer 1 |
| `terra_mem[512..1024]` | `TERRA_BLOCK + nd->terra2`, 1 block | 512 B | Terrain type table, layer 2 |
| `image_mem` | 4 tilesets × (5 planes × 8 blocks) | 81 920 B | Tile graphics; 4 groups × 64 tiles × 5 bitplanes × 64 B |

**Tile format:**
- 16 × 32 pixels, 5 bitplanes (→ 32 colours from a 32-entry palette)
- 64 bytes per plane per tile (2 bytes/row × 32 rows)
- `image_mem` layout: plane stride = `IPLAN_SZ` = 16 384 B; tileset stride within each plane = `QPLAN_SZ` = 4 096 B
- Tile `n` in group `g`, plane `p`: byte offset = `p × IPLAN_SZ + g × QPLAN_SZ + n × 64`

**Viewport and coordinate system:**
- World: up to `MAXCOORD = 32 768` units each axis (pixel coordinates)
- Image units: 16 px wide × 32 px tall per tile
- Sectors: 16 × 8 image units = 256 × 256 pixels (128 sectors per axis via `map_mem` grid)
- Screen playfield: 288 × 140 lores px, x-offset 16 px from left edge
- Viewport tile grid: `minimap[114]` = 19 columns × 6 rows (each short = tile index)
- `map_x = hero_x − 144`, `map_y = hero_y − 90` (hero centred)
- `img_x = map_x >> 4`, `img_y = map_y >> 5`

**Region table (`file_index[10]`):** Ten outdoor/indoor regions (F1–F10); each `struct need` holds: `image[4]` (4 tileset ADF block numbers), `terra1`, `terra2`, `sector`, `region`, `setchar`. Region 0–7 = outdoor, 8–9 = indoor/dungeon. `region_num` selects the active region.

**`genmini(img_x, img_y)`** (asm in `fsubs.asm`): Fills `minimap[114]` by walking the 19×6 tile grid, looking up each tile's sector via `map_mem`, then its byte value from `sector_mem`. Used as the tile index for rendering.

**`map_draw()`** (asm in `fsubs.asm`): Blits all 114 minimap tiles to the 5-plane screen bitplanes, column by column (19 strips × 6 tiles each). Tile `image_mem` offset = `plane_base + char_idx × 64`.

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

6. **HiScreen overlay** (`src/game/gameplay_scene.rs`)
   - Load `game/hiscreen` as an `IffImage` (640 × 57 px, already supported by `iff_image.rs`)
   - Render it to the bottom strip of the 640 × 480 canvas below `play_tex` (at y = 480 − 57 = 423 or matching original `PAGE_HEIGHT = 143`)
   - Exact y-position: `vp_page` starts at y=0 with height 143; `vp_text` at `PAGE_HEIGHT` = 143, height 57 → canvas rows 143–200 (at 1× lores scale) or equivalent at our 2× scale

7. **Gameplay scene stub** (`src/game/gameplay_scene.rs`)
   - Wire `WorldData`, `TileAtlas`, and tile rendering into the existing `Gameplay` phase in `main.rs` (currently renders directly in the loop)
   - Implement `Scene` trait; place hero at starting coordinates from `file_index[3]`
   - Static render first (no movement), confirm tiles appear correctly

8. **Palette for regions**
   - Each region uses a different palette (outdoor colours differ from dungeon/indoor)
   - Identify palette block numbers from ADF or embed per-region palettes in `faery.toml`
   - Hook `TileAtlas::rebuild()` into region transitions

### Key constants and block addresses (region F4, region_num=3 is starting region)

```
file_index[3] = {320, 360, 400, 440, 2, 3, 32, 168, 21}
  image[0]=320, image[1]=360, image[2]=400, image[3]=440
  terra1=2, terra2=3, sector=32, region=168, setchar=21
TERRA_BLOCK = 149
```

So starting map data (region 3):
- sector_mem: ADF offset 32×512 = 16 384
- map_mem: ADF offset 168×512 = 86 016
- terra_mem[0]: ADF offset (149+2)×512 = 77 312
- terra_mem[512]: ADF offset (149+3)×512 = 77 824
- image[0] plane 0: ADF offset 320×512 = 163 840

### Notes

- The original renders via 5-plane Amiga bitplanes; we use RGBA32 SDL2 textures. The decode step (planar → packed pixel) is the same as used for IFF images in `bitmap.rs`.
- `minimap` in the original is a `short[114]` (tile index as signed 16-bit); in our port use `u8[114]` since tile indices fit in one byte (0–255 from sector_mem).
- Scroll: original uses Amiga copper-list `RxOffset`/`RyOffset` for sub-tile pixel scrolling; we achieve the same by shifting blit destination rects.
- `xreg` and `yreg` track which 64×32 sector block is currently loaded into sector_mem. Initially 0. Updated by `load_new_region()` as hero moves.

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
   - Blocked tiles, water/swamp sinking, bush slowdown
   - Path validation

3. **Player commands**
   - Look, Give, Get, Yell, Ask, etc.
   - Text output to scroll viewport
   - Object/NPC interaction triggers

4. **Character state**
   - Three brothers (Julian, Phillip, Kevin) with sequential lives
   - Health, inventory, quest flags
   - Death → revive as next brother → placard → map repositioning

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

---

## Plan: Graphics Effects

**Status:** Partially started (palette fading infrastructure complete)

### Overview
Visual effects that enhance the game atmosphere.

### Steps

1. **Day/Night cycle**
   - `GameClock` already tracks day phases (Midnight/Morning/Midday/Evening)
   - `fade_page()` with `limit=true` already supports night floor limits, blue tint injection, vegetation boost, and light_timer torch illumination — matching the original `day_fade()` from `fmain2.c`
   - `FadeController` can drive day/night transitions over time
   - Remaining: wire `day_fade()` into the gameplay loop using `lightlevel = daynight / 40` with the staggered RGB offsets (r-80, g-61, b-62) from the original

2. **Copper list parsing**
   - Amiga copper lists define per-scanline palette changes (sky gradients, water effects)
   - Parse and simulate these effects

3. **Witch effect**
   - Screen distortion effect when encountering the witch

4. **Teleport effect**
   - Visual transition when using teleport items/locations

---

## Plan: Key Bindings

**Status:** Not started

### Overview

Implement a rebindable key binding system based on the original game's keyboard commands. The original uses a flat `letter_list[38]` lookup table mapping ASCII keys to `(menu, choice)` pairs, plus special-cased direction and fight keys. Our port replaces this with a `GameAction` enum and a `KeyBindings` map that users can customize. Bindings are persisted via `settings.rs` to `~/.config/faery/settings.toml`.

### Original Game Key Map

From `fmain.c` `letter_list[]` and the main game loop:

| Key (original) | Menu   | Action             |
|-----------------|--------|--------------------|
| Arrow keys      | —      | Movement (8 dirs)  |
| `0`             | —      | Fight / Attack     |
| `I`             | ITEMS  | List inventory     |
| `T`             | ITEMS  | Take / Pick up     |
| `?`             | ITEMS  | Look / Examine     |
| `U`             | ITEMS  | Use item           |
| `G`             | ITEMS  | Give item          |
| `Y`             | TALK   | Yell               |
| `S`             | TALK   | Say / Speak        |
| `A`             | TALK   | Ask                |
| `Space`         | GAME   | Pause toggle       |
| `M`             | GAME   | Map view           |
| `F`             | GAME   | Find (compass)     |
| `Q`             | GAME   | Quit               |
| `L`             | GAME   | Load game          |
| `O`             | BUY    | Food               |
| `R`             | BUY    | Arrow              |
| `8`             | BUY    | Vial               |
| `C`             | BUY    | Mace               |
| `W`             | BUY    | Sword              |
| `B`             | BUY    | Bow                |
| `E`             | BUY    | Totem              |
| `V`             | SAVEX  | Save game          |
| `X`             | SAVEX  | Exit / Load        |
| `F1`–`F7`      | MAGIC  | Cast spell 1–7     |
| `1`–`7`        | USE    | Use item in slot   |
| `K`             | USE    | Use special (key?) |
| `1`–`6` (KEYS) | KEYS   | Select key color   |

### Implementation Steps

1. **Define `GameAction` enum**
   - Create `src/game/key_bindings.rs`
   - Enum variants for every bindable action: `MoveUp`, `MoveDown`, `MoveLeft`, `MoveRight`, `MoveUpLeft`, `MoveUpRight`, `MoveDownLeft`, `MoveDownRight`, `Fight`, `Pause`, `Inventory`, `Take`, `Look`, `UseItem`, `Give`, `Yell`, `Speak`, `Ask`, `Map`, `Find`, `Quit`, `LoadGame`, `SaveGame`, `ExitMenu`, `CastSpell1`..`CastSpell7`, `UseSlot1`..`UseSlot7`, `UseSpecial`, `BuyFood`, `BuyArrow`, `BuyVial`, `BuyMace`, `BuySword`, `BuyBow`, `BuyTotem`, `SelectKey1`..`SelectKey6`
   - Derive `Serialize`, `Deserialize`, `Hash`, `Eq`, `Clone`, `Debug`

2. **Define `KeyBindings` struct**
   - `HashMap<GameAction, Vec<Keycode>>` — each action maps to one or more physical keys
   - `fn default_bindings() -> KeyBindings` — populate with the original mapping (see table above), using modern keyboard equivalents:
     - Arrow keys → movement (original used joystick dirs 20-29; map to `Up`/`Down`/`Left`/`Right`)
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

### Design Notes

- The original game's `letter_list[]` is a flat array scanned linearly on each keypress — we replace this with a `HashMap` reverse index for O(1) lookup
- Direction keys need special handling: the original tracks key-down/key-up separately (`keydir` set on press, cleared on release), so we need to track held-key state
- The KEYS menu (`SelectKey1`..`SelectKey6`) is only active when `cmode == KEYS` in the original; our implementation can context-gate these actions
- Buy menu keys are only relevant when a shop interface is open — scene-level filtering handles this
- Cheat keys from the original (B, '.', R, '=', arrows-teleport) are intentionally excluded from the rebindable system and handled separately as debug/cheat commands

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