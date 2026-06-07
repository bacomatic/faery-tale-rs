# Faery Tale Adventure - Controls & Keybinds

## Basic Controls

### Movement
- **Numeric Keypad 0-9**: Directional movement (8-way)
- **Mouse**: Move cursor to status bar area for directional control
- **Joystick**: Standard 8-way directional input

### Actions
- **Space**: Fire/attack (works even when paused)
- **0 (held)**: Toggle fight mode
- **Left Mouse Click**: Select menu options in status bar

### Menu Navigation
- **1-6 Keys**: Quick select items 1-6 (in KEYS menu)
- **Mouse Click**: Click menu slots in status bar
- **Any Key**: Dismiss full-screen views/transient messages

## Cheat Commands (requires cheat mode enabled)

Cheat mode is enabled by setting the `cheat1` flag (requires save file editing or patching).

| Key | Effect |
|-----|--------|
| **B** | Summon Swan near hero; grants Golden Lasso if Swan already active |
| **.** | Add 3 random gold items (random stuff[] entry 0-30) |
| **R** | Rescue hero (teleport to safety) |
| **=** | Show debug coordinates |
| **Ctrl+S** | Show debug location info |
| **Key 18** | Advance time by 1000 (day/night cycle) |
| **Key 19** | Show debug location info |
| **Keys 1-4** | Teleport hero (±150 Y / ±280 X) |

## Menu System

### Status Bar Menu
- 12 slots accessible via mouse clicks (codes 0x61-0x6C)
- Left button press generates slot selection
- Left button release confirms selection

### Menu Modes
- **CMODE_GAME**: Main game menu
- **CMODE_ITEMS**: Inventory management
- **CMODE_KEYS**: Key items quick access

## Technical Notes

- All key codes have bit 7 (0x80) set for key-up events
- Movement keys are codes 20-29 (keypad)
- Menu slots are synthetic codes 0x61-0x6C from mouse clicks
- Input is processed through a 128-byte circular buffer
- Mouse movement is clamped to status bar area (X: 5-315, Y: 147-195)