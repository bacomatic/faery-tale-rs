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

4. **Page flip animation** — `PageFlip` in `src/game/page_flip.rs`. Port of `flipscan()` + `page_det()` with the original 22-step lookup tables. `draw_region()` added to `ImageTexture`.

5. **IntroScene** — `IntroScene` in `src/game/intro_scene.rs`. 7-phase FSM: TitleText → TitleFadeOut → ZoomIn → ShowPage → FlipPage → ZoomOut → Done. Space to skip.
   - **TitleFadeOut**: Uses `FadeController::fade_down()` → `FadeResult::ColorMod` applied via `play_tex.set_color_mod()`.
   - **ZoomIn/ZoomOut**: Uses `FadeController::zoom_fade(introcolors, half_width)` per frame to compute a zoom-position-dependent palette with staggered channel ramp-up (red leads, green follows, blue lags — matching original `screen_size()` formula). Re-rasterizes all intro images via `resources.find_image_mut(name).update(&faded_palette, None)`. Restores full-brightness palette when zoom completes.
   - Font, image, and placard rendering wire directly through `SceneResources`.

6. **delta_ticks** — `GameClock::update()` returns monotonic delta. Frame delta passed to scenes instead of hardcoded `1`.

7. **Wire image textures into IntroScene** — `SceneResources<'a, 'tex>` struct provides named image lookup via `find_image()` and `find_image_mut()`. `ZoomIn` draws `page0` to play_tex; page compositing overlays portraits at (4,24) and bios at page-specific positions (from original `copypage()` calls). Pages accumulate on play_tex matching original behavior. `image_name_map: HashMap<String, usize>` built in main.rs image loading loop.

8. **Title text rendering in IntroScene** — `titletext` placard rendered directly to 640x480 canvas (not play_tex) during TitleText phase using amber font. Y offset of 140px centers text vertically. Dark blue background (0x006) matches original `textcolors` palette. `draw_offset()` method added to Placard.

9. **Copy protection scene** — `CopyProtectScene` in `src/game/copy_protect_scene.rs`. 3 random questions from pool of 8 (without replacement). Typed input via KeyDown events, case-insensitive answer matching. `submit_pending` flag bridges handle_event/update. Topaz font on dark blue background. `passed()` method checked via `as_any()` downcast after scene completes. Skip flag for development.

10. **Character start placard** — `PlacardScene` in `src/game/placard_scene.rs`. Generic scene for any placard with swirly border. Instant border via `draw_placard_border()` using palette index 24 for both border and text color. `FontTexture::set_color_mod()` tints the amber font to the palette's color 24 (red in `pagecolors`) before drawing, then resets to white. Configurable placard name, palette, and hold duration (default 144 ticks = 2.4s). Space to skip. Uses `pagecolors` palette (matching original `map_message()` which calls `LoadRGB4(&vp_page, pagecolors, 32)`).

11. **Copy protection failure**: Wrong answer quits the game

### Future Refinements (Intro)

- **Animated page flip**: Strip-based column-by-column animation using PageFlip + two scratch textures (currently instant transition)
- **Animated placard border**: Progressive border drawing over time (currently drawn all at once)
- **Audio integration**: Intro music (tracks 12-15) during TitleText phase

### Decisions

- Enum-based FSM within each scene (phase enum), trait-based FSM across scenes (`Box<dyn Scene>`) — balances simplicity inside with flexibility across
- `PaletteFader` is a standalone utility, not a scene — it's composed into scenes that need it
- Page flip is a `RenderTask` (fits the existing pattern), intro phases are *not* RenderTasks (they need richer control flow)
- Copy protection is included but can be gated with a skip flag for development convenience
- Audio is stubbed (no-op calls) — song playback is a separate workstream

---

## Plan: Audio System

**Status:** Not started

### Overview
Parse the music file (`game/songs`), build a song list, and play tracks via SDL2 mixer. Sound effects loaded from the game data.

### Steps

1. **Parse music/song data**
   - Investigate `game/songs` file format (likely Amiga MOD or custom tracker format)
   - Build a song list with track indices matching the original (`track[12]`..`track[15]` for intro music)

2. **SDL2 mixer integration**
   - The `mixer` feature is already enabled in `Cargo.toml` but unused
   - Initialize mixer, set up audio channels
   - Play/stop/fade songs by index

3. **Sound effects**
   - Identify and load sound effect data
   - Trigger effects from game events

4. **Wire into scenes**
   - IntroScene: play intro music (tracks 12-15) during TitleText phase
   - Game loop: ambient/combat music, day/night transitions trigger different tracks

---

## Plan: Game World & Map System

**Status:** Not started

### Overview
Load and render the game world from the `fmain` binary. This includes map tiles, terrain data, scrolling viewport, and the main play UI.

### Steps

1. **Decode `fmain` binary**
   - The Amiga executable (`game/fmain`) contains embedded game data: maps, object tables, NPC data, item lists
   - Hunk loader exists (`src/game/hunk.rs`) — use it to extract data segments
   - Map segment offsets are partially decoded; continue reverse-engineering

2. **Tileset loading**
   - Extract tile graphics from the game data
   - Build tile atlas texture (similar to image atlas pattern)

3. **Map loading and rendering**
   - Parse map data into a 2D tile grid
   - Render visible tiles to the offscreen texture
   - Implement smooth pixel-level scrolling (original uses `RxOffset`/`RyOffset`)

4. **Main viewport UI**
   - Play field (scrolling map view)
   - Scroll text area (message output)
   - UI buttons
   - Compass

5. **HiScreen overlay**
   - Load and display `hiscreen` IFF image as the UI frame
   - Place viewport within the frame bounds

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