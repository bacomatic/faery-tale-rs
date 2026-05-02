## 24. Special Effects

### Requirements

| ID | Requirement |
|----|-------------|
| R-FX-001 | Witch vision cone: a rotating filled wedge-shaped polygon (~11.25° arc) rendered in COMPLEMENT (XOR) mode around the witch position. Endpoints looked up from `witchpoints[256]` (two concentric circles, radii ~10 and ~100 pixels). `witchindex` (u8, 0–255) advances each frame by `wdir` (±1), completing a full rotation over 256 frames. Steering adjusts via cross-product sign, gated by `rand4() == 0` (1-in-4 frames). Hero hit detection: cross-product test within wedge AND distance < 100 pixels, dealing 1–2 HP damage. |
| R-FX-002 | Teleport colorplay: 32 frames of randomized 12-bit RGB values for palette entries 1–31 (preserving entry 0, the background color), creating a psychedelic flash effect lasting ≈ 0.5 seconds. |
| R-FX-003 | Columnar page reveal (`flipscan`): 22-step vertical strip animation for story page transitions — steps 0–10 sweep the right half, steps 11–21 sweep the left half, with per-step timing from the `flip3[]` delay table. Each step performs a page swap for intermediate display. |
| R-FX-004 | Victory sunrise (`win_colors`): 55-step palette fade (index 25 down to −29) using `sun_colors[53]`. Colors 0/31 always black, colors 1/28 always white, colors 2–27 swept from deep blue/black through purple/red to golden tones. Colors 29–30 use red computations. First frame holds 60 ticks (~1 second), subsequent frames 9 ticks (~150 ms), final hold 30 ticks. |
| R-FX-005 | Screen fade-down (`fade_down`): 21 steps from 100% to 0% in decrements of 5, with `Delay(1)` per step and `limit=FALSE` (no night clamping). Screen fade-up (`fade_normal`): 21 steps from 0% to 100% in increments of 5. Both fade the entire palette to/from black. |
| R-FX-006 | Flasher border blink: during dialogue mode (`viewstatus == 1`), color register 31 shall blink white↔black every 16 frames (~0.27 seconds), toggled by bit 4 of the `flasher` counter (which increments each main-loop tick). |
| R-FX-007 | Viewport zoom (`screen_size(x)`) is a single-frame compound operation combining: (a) playfield aperture resize to `(x*2) × ((x*5/8)*2)` centered in 320×200; (b) inverse HUD shrink — HUD is hidden when `x ≥ 152`; and (c) palette fade on `introcolors` with per-channel percentages `R% = y*2 − 40`, `G% = y*2 − 70`, `B% = y*2 − 100` where `y = (x*5)/8` (negative clamps to black), producing a warm red→green→blue fade-in as the aperture opens. Zoom-in animates `x = 0..160` step +4. Zoom-out animates `x = 156..0` step −4 (starts at 156 to skip a redundant no-op first frame, since 160 is already the current state). All three effects (aperture, HUD, palette) MUST be frame-synchronized. |
| R-FX-008 | Full-screen message transitions: `map_message()` fades down, clears playfield, hides status bar, enables drawing on the playfield with pen 24 in JAM1 mode, and sets `viewstatus = 2`. `message_off()` fades down, restores status bar, flips page, and sets `viewstatus = 3`, triggering `fade_normal()` on the next frame. |
| R-FX-009 | Static display reset (`stillscreen`): resets scroll offsets to (0, 0) and flips the page, used for non-scrolling display modes. |
| R-FX-010 | Placard border (`placard`): a recursive fractal line pattern drawn on the playfield using `xmod`/`ymod` offset tables (±4 pixel deltas). The pattern is mirror-symmetric about center (284, 124) with 90°/270° rotations, using 16×15 outer iterations with 5 inner passes. Color 1 for most lines, color 24 for the first inner pass. |

### User Stories

- As a player, I see the witch's spinning vision cone that damages me when caught.
- As a player, I see dramatic palette effects during teleportation, story transitions, and the victory ending.
- As a player, I see a blinking border/prompt effect during dialogue mode.
- As a player, I see smooth viewport zoom-in during the intro sequence.
- As a player, I see decorative fractal borders on story placard screens.

---

