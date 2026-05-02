## 8. Characters & Animation

### 8.1 Actor Record (`struct shape`)

The fundamental actor record, 22 bytes total, used for player, NPCs, and enemies:

| Offset | Size | Field | Type | Purpose |
|--------|------|-------|------|---------|
| 0 | 2 | `abs_x` | u16 | Absolute world X coordinate |
| 2 | 2 | `abs_y` | u16 | Absolute world Y coordinate |
| 4 | 2 | `rel_x` | u16 | Screen-relative X position |
| 6 | 2 | `rel_y` | u16 | Screen-relative Y position |
| 8 | 1 | `type` | u8 | Object type number |
| 9 | 1 | `race` | u8 | Race (indexes `encounter_chart[]`) |
| 10 | 1 | `index` | u8 | Current animation frame image index |
| 11 | 1 | `visible` | u8 | On-screen visibility flag |
| 12 | 1 | `weapon` | u8 | Weapon: 0=none, 1=Dirk, 2=mace, 3=sword, 4=bow, 5=wand, 8=touch |
| 13 | 1 | `environ` | i8 | Environment/terrain state |
| 14 | 1 | `goal` | u8 | Current goal mode (§11.1) |
| 15 | 1 | `tactic` | u8 | Current tactical mode (§11.2) |
| 16 | 1 | `state` | u8 | Motion/animation state (§8.2) |
| 17 | 1 | `facing` | u8 | Direction facing (0–7) |
| 18 | 2 | `vitality` | i16 | Hit points |
| 20 | 1 | `vel_x` | i8 | X velocity (ice/slippery physics) |
| 21 | 1 | `vel_y` | i8 | Y velocity (ice/slippery physics) |

#### Actor Array Layout

- `anim_list[0]` — player-controlled hero
- `anim_list[1]` — raft (always present) / party member
- `anim_list[2]` — NPC set-piece figure
- `anim_list[3–6]` — enemy actors (up to 4; `anix` tracks count, max 7)
- `anim_list[7–19]` — remaining slots for world objects and set-figures

`MAXSHAPES = 25` governs the per-page rendering queue, not the actor array size (20 entries).

### 8.2 Motion States

26 animation states stored in `shape.state`:

| Value | Name | Purpose |
|-------|------|---------|
| 0–11 | *(fighting frames)* | Combat animation sub-states; `statelist[facing*12 + state]` selects figure |
| 12 | WALKING | Normal walk cycle |
| 13 | STILL | Stationary/idle |
| 14 | DYING | Death animation in progress |
| 15 | DEAD | Fully dead |
| 16 | SINK | Sinking (quicksand/water) |
| 17 | OSCIL | Oscillation anim 1 — vestigial, never assigned |
| 18 | *(implicit)* | Oscillation anim 2 — vestigial, never assigned |
| 19 | TALKING | SETFIG-only: 15-tick image-flicker while speech text displays |
| 20 | FROZEN | Frozen in place (freeze spell) |
| 21 | FLYING | Vestigial — defined but never assigned; swan uses WALKING + `riding` |
| 22 | FALL | Falling; velocity decays 25% per tick |
| 23 | SLEEP | Sleeping |
| 24 | SHOOT1 | Bow up — aiming |
| 25 | SHOOT3 | Bow fired, arrow given velocity |

### 8.3 Direction System

8 compass directions plus 2 stop values. Direction vectors defined by `xdir[10]` / `ydir[10]`:

| Value | Direction | xdir | ydir |
|-------|-----------|------|------|
| 0 | NW | −2 | −2 |
| 1 | N | 0 | −3 |
| 2 | NE | +2 | −2 |
| 3 | E | +3 | 0 |
| 4 | SE | +2 | +2 |
| 5 | S | 0 | +3 |
| 6 | SW | −2 | +2 |
| 7 | W | −3 | 0 |
| 8 | Still | 0 | 0 |
| 9 | Still | 0 | 0 |

Cardinals have magnitude 3, diagonals 2 per axis (displacement √8 ≈ 2.83), yielding near-parity between cardinal and diagonal speed.

Walk base offsets via `diroffs[16]`:

```
diroffs[16] = {16,16,24,24,0,0,8,8,56,56,68,68,32,32,44,44}
```

Indices 0–7 select walk animation bases; indices 8–15 select fight/shoot bases.

### 8.4 `statelist[87]` — Animation Frame Lookup

Maps `(motion_state, facing, frame)` → `(figure_image, weapon_overlay_index, weapon_x_offset, weapon_y_offset)`:

```rust
struct State { figure: i8, wpn_no: i8, wpn_x: i8, wpn_y: i8 }
```

#### Walk Sequences (8 frames each)

| Index Range | Direction |
|-------------|-----------|
| 0–7 | South |
| 8–15 | West |
| 16–23 | North |
| 24–31 | East |

#### Fight Sequences (12 states each)

| Index Range | Direction |
|-------------|-----------|
| 32–43 | South |
| 44–55 | West |
| 56–67 | North |
| 68–79 | East |

Each 12-entry block: states 0–8 = weapon swing positions, state 9 = duplicate swing, states 10–11 = ranged attack frames.

#### Special States

| Index | Purpose |
|-------|---------|
| 80–82 | Death sequence (3 frames) |
| 83 | Sinking |
| 84–85 | Oscillation (2 frames) |
| 86 | Asleep |

### 8.5 Combat Animation FSA — `trans_list[9]`

Nine `struct transition` entries controlling fight swing animation:

```rust
struct Transition { newstate: [i8; 4] }
```

| Index | newstate[0] | [1] | [2] | [3] |
|-------|-------------|-----|-----|-----|
| 0 | 1 | 8 | 0 | 1 |
| 1 | 2 | 0 | 1 | 0 |
| 2 | 3 | 1 | 2 | 8 |
| 3 | 4 | 2 | 3 | 7 |
| 4 | 5 | 3 | 4 | 6 |
| 5 | 6 | 4 | 5 | 5 |
| 6 | 8 | 5 | 6 | 4 |
| 7 | 8 | 6 | 7 | 3 |
| 8 | 0 | 6 | 8 | 2 |

Forward cycle via `newstate[0]`: 0→1→2→3→4→5→6→8→0 (state 7 reached via other paths). Each tick: `trans_list[state].newstate[rand4()]`. Monsters at states 6 or 7 forced to state 8.

### 8.6 Missile System

```rust
struct Missile {
    abs_x: u16, abs_y: u16,
    missile_type: u8,    // NULL, arrow, rock, 'thing', fireball
    time_of_flight: u8,
    speed: u8,           // 0 = unshot
    direction: u8,
    archer: u8,          // ID of firing actor
}
```

6 missile slots maximum. Slots assigned round-robin via `mdex`.

### 8.7 Sprite Sheet Descriptors

Sequence type constants:

| Value | Name | Purpose |
|-------|------|---------|
| 0 | PHIL | Player character sprites |
| 1 | OBJECTS | World object sprites |
| 2 | ENEMY | Enemy sprites |
| 3 | RAFT | Raft/vehicle sprites |
| 4 | SETFIG | Set-piece figure sprites (NPCs) |
| 5 | CARRIER | Carrier animal sprites |
| 6 | DRAGON | Dragon sprites |

### 8.8 NPC Type Descriptors — `setfig_table[14]`

| Index | NPC Type | cfile_entry | image_base | can_talk |
|-------|----------|-------------|------------|----------|
| 0 | Wizard | 13 | 0 | 1 |
| 1 | Priest | 13 | 4 | 1 |
| 2 | Guard (front) | 14 | 0 | 0 |
| 3 | Guard (back) | 14 | 1 | 0 |
| 4 | Princess | 14 | 2 | 0 |
| 5 | King | 14 | 4 | 1 |
| 6 | Noble | 14 | 6 | 0 |
| 7 | Sorceress | 14 | 7 | 0 |
| 8 | Bartender | 15 | 0 | 0 |
| 9 | Witch | 16 | 0 | 0 |
| 10 | Spectre | 16 | 6 | 0 |
| 11 | Ghost | 16 | 7 | 0 |
| 12 | Ranger | 17 | 0 | 1 |
| 13 | Beggar | 17 | 4 | 1 |

`cfile_entry` selects the image file. `image_base` is the sub-image offset. `can_talk=1` enables the TALKING visual effect during speech — it does **not** gate speech dispatch. All types produce speech text regardless.

---


