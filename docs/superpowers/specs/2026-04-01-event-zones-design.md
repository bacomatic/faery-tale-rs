# Event Zones Design Spec

## Problem

The original game has 22 trigger zones (`extent_list[22]` in `fmain.c`) that drive
encounters, carrier spawns, peace zones, and special events. The Rust port has
stub infrastructure (`zones.rs`, `ZoneConfig`, `gameplay_scene.rs` zone detection)
but zero actual zone data. This spec defines the TOML schema, data, and code
changes needed to fully populate and wire up event zones.

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Field fidelity | Raw `etype`/`v1`/`v2`/`v3` values | Exact match to original; avoids misrepresenting fields whose meaning varies by etype |
| Region field | Dropped | Original has no region field; zones use global world coordinates |
| Label field | Added | Human-readable; matches original source comments; aids debugging |
| Data source | `faery.toml` only (no static table) | Consistent with how regions, encounters, and other config already works |
| Inline comments | Required for v1/v2/v3 per zone | Fields are cryptic without per-zone context |

## TOML Schema

Each zone entry in `faery.toml`:

```toml
[[zones]]
label = "bird extent"      # human-readable label from original comments
etype = 70                 # zone type: 0-49=random, 50+=forced, 60+=special, 70+=carrier, 80+=peace
x1 = 2118                  # bounding box in world coordinates
y1 = 27237
x2 = 2618
y2 = 27637
v1 = 0                     # base encounter count (0 for carriers)
v2 = 1                     # count spread (encounter_count = v1 + rnd(v2))
v3 = 11                    # carrier cfile: bird
```

Fields:
- `label`: `String` — descriptive name from original source comments
- `etype`: `u8` — zone type, used directly in branching logic:
  - 0–49: random encounter; value is danger modifier (`danger = 2 + etype` outdoor, `5 + etype` indoor)
  - 50: set group encounter
  - 52: astral plane (always Loraii)
  - 53: forced spider encounter
  - 60–61: special forced encounters (necromancer, turtle eggs, hidden valley)
  - 70: carrier zones (bird, turtle, dragon)
  - 80: peace/NPC zones (no random encounters)
  - 81: king's palace
  - 82: sorceress palace
  - 83: princess rescue trigger
- `x1`, `y1`, `x2`, `y2`: `u16` — axis-aligned bounding box in global world coordinates
- `v1`: `u8` — base encounter count (for forced/random) or unused (for carriers/peace)
- `v2`: `u8` — encounter count randomization range: final count = `v1 + rnd(v2)`
- `v3`: `u8` — enemy race index (etype < 70) or carrier cfile ID (etype >= 70)

## Complete Zone Data (23 entries; indices 0–22)

Transcribed from `original/fmain.c` lines 388–419.

**Important**: The original defines `EXT_COUNT = 22`. The matching loop iterates indices
0–21. Index 22 ("whole world") is **not iterated** — it acts as a **fallback/sentinel**.
When no zone 0–21 matches, the `extn` pointer naturally falls through to index 22,
making it the default zone for any position not covered by a specific zone. The
faery.toml list should preserve this ordering and the code should replicate the
fallback behavior.

**Boundary check**: The original uses **strict inequality** (`>` and `<`, exclusive
bounds), not `>=`/`<=`. The current Rust code uses inclusive bounds and must be
corrected. Zone 20 has intentionally inverted y-coordinates (`y1=18719 > y2=17484`)
which works correctly with the original's strict check: `17484 < hero_y < 18719`.

| Idx | Label | etype | x1 | y1 | x2 | y2 | v1 | v2 | v3 | Notes |
|-----|-------|-------|----|----|----|----|----|----|----|----|
| 0 | bird extent | 70 | 2118 | 27237 | 2618 | 27637 | 0 | 1 | 11 | carrier: bird cfile |
| 1 | turtle extent | 70 | 0 | 0 | 0 | 0 | 0 | 1 | 5 | carrier: turtle cfile; coords set at runtime |
| 2 | dragon extent | 70 | 6749 | 34951 | 7249 | 35351 | 0 | 1 | 10 | carrier: dragon cfile |
| 3 | spider pit | 53 | 4063 | 34819 | 4909 | 35125 | 4 | 1 | 6 | forced: 4 spiders (race 6) |
| 4 | necromancer | 60 | 9563 | 33883 | 10144 | 34462 | 1 | 1 | 9 | forced: 1 necromancer (race 9); blocks magic |
| 5 | turtle eggs | 61 | 22945 | 5597 | 23225 | 5747 | 3 | 2 | 4 | forced: 3–4 snakes (race 4) guarding eggs |
| 6 | princess extent | 83 | 10820 | 35646 | 10877 | 35670 | 1 | 1 | 0 | special: princess rescue trigger |
| 7 | graveyard ext | 48 | 19596 | 17123 | 19974 | 17401 | 8 | 8 | 2 | forced: 8–15 wraiths (race 2) |
| 8 | around city | 80 | 19400 | 17034 | 20240 | 17484 | 4 | 20 | 0 | peace: palace zone, 4–23 generic |
| 9 | astral plane | 52 | 9216 | 33280 | 12544 | 35328 | 3 | 1 | 8 | forced: 3 loraii (race 8); always loraii |
| 10 | king pax | 81 | 5272 | 33300 | 6112 | 34200 | 0 | 1 | 0 | peace: king's palace zone |
| 11 | sorceress pax | 82 | 11712 | 37350 | 12416 | 38020 | 0 | 1 | 0 | peace: sorceress palace zone |
| 12 | peace 1 - buildings | 80 | 2752 | 33300 | 8632 | 35400 | 0 | 1 | 0 | peace: buildings area |
| 13 | peace 2 - specials | 80 | 10032 | 35550 | 12976 | 40270 | 0 | 1 | 0 | peace: specials area |
| 14 | peace 3 - cabins | 80 | 4712 | 38100 | 10032 | 40350 | 0 | 1 | 0 | peace: cabins area |
| 15 | hidden valley | 60 | 21405 | 25583 | 21827 | 26028 | 1 | 1 | 7 | forced: 1 DKnight (race 7); fixed spawn at (21635,25762) |
| 16 | swamp region | 7 | 6156 | 12755 | 12316 | 15905 | 1 | 8 | 0 | random: 1–8 snakes; etype=7 overrides race 2→4 |
| 17 | spider region | 8 | 5140 | 34860 | 6260 | 37260 | 1 | 8 | 0 | random: 1–8 spiders; etype=8 forces race 6 |
| 18 | spider region 2 | 8 | 660 | 33510 | 2060 | 34560 | 1 | 8 | 0 | random: 1–8 spiders; etype=8 forces race 6 |
| 19 | village | 80 | 18687 | 15338 | 19211 | 16136 | 0 | 1 | 0 | peace: village zone |
| 20 | around village | 3 | 16953 | 18719 | 20240 | 17484 | 1 | 3 | 0 | random: 1–3 generic; NOTE: y1 > y2 intentional |
| 21 | around city 2 | 3 | 20593 | 18719 | 23113 | 22769 | 1 | 3 | 0 | random: 1–3 generic; danger=5 outdoor |
| 22 | whole world | 3 | 0 | 0 | 32767 | 40959 | 1 | 8 | 0 | FALLBACK: catch-all; not iterated; 1–8 generic |

Note: Zone 9 (astral plane) coordinates are hex in the original: `0x2400=9216`, `0x8200=33280`, `0x3100=12544`, `0x8a00=35328`.
Note: Zone 22 (whole world) uses `0x7fff=32767`, `0x9fff=40959`.

## Code Changes

### 1. `src/game/game_library.rs` — Update `ZoneConfig`

Replace current struct:

```rust
#[derive(Deserialize, Debug, Clone)]
pub struct ZoneConfig {
    pub label:  String,
    pub etype:  u8,
    pub x1:     u16,
    pub y1:     u16,
    pub x2:     u16,
    pub y2:     u16,
    pub v1:     u8,
    pub v2:     u8,
    pub v3:     u8,
}
```

Remove the old fields: `zone_type` (String), `region` (u8), `encounter_rate` (u8), `event_id` (u8).

### 2. `src/game/zones.rs` — Refactor to runtime data

- Remove the static `ZONE_TABLE` and its single placeholder entry.
- Keep `ZoneType` enum for categorization.
- Add `ZoneType::from_etype(etype: u8) -> ZoneType`:
  ```rust
  pub fn from_etype(etype: u8) -> ZoneType {
      match etype {
          0..=49  => ZoneType::Encounter,
          70..=79 => ZoneType::Carrier,
          80..=89 => ZoneType::Peace,
          _       => ZoneType::Special,  // 50-69, 83
      }
  }
  ```
- Add `ZoneType::Peace` variant for etype 80–82 (no random encounters).
- Update `Zone::contains()` to drop region check; match by x/y only.
- **Fix boundary check**: use strict inequality (`>` and `<`) to match original, not `>=`/`<=`.
- Update `zones_at()`, `in_encounter_zone()` to take `&[ZoneConfig]` parameter.
- Add fallback behavior: iterate indices 0..n-1; if no match, use last entry as default.
- Adjust tests.

### 3. `src/game/gameplay_scene.rs` — Drop region filter

The zone entry detection block currently checks `z.region == region`. Change to use
strict inequality matching coordinates only (matching original `fmain.c` line 3283):

```rust
let current_zone = self.zones.iter().position(|z|
    hx > z.x1 && hx < z.x2
    && hy > z.y1 && hy < z.y2
);
```

If no zone matches, fall back to the last entry (whole world catch-all), replicating
the original's `extn` pointer fall-through behavior.

### 4. `faery.toml` — Populate 22 zone entries

Replace the commented-out zone template with all 22 `[[zones]]` entries.
Each entry includes inline TOML comments explaining what v1/v2/v3 mean for that zone.

### 5. Debug Console — Add "Geography" section

The debug console (`src/game/debug_console.rs`) currently shows hero coordinates
and region on line 2 of the Status panel. Refactor to add a dedicated Geography
panel and surface the current zone.

#### DebugStatus changes

Add two fields to `DebugStatus`:

```rust
// Geography
pub current_zone_idx: Option<usize>,
pub current_zone_label: Option<String>,
```

#### GameplayScene accessor

Add a public method to expose zone info (fields are private):

```rust
pub fn current_zone_info(&self) -> (Option<usize>, Option<String>) {
    let label = self.last_zone
        .and_then(|i| self.zones.get(i).map(|z| z.label.clone()));
    (self.last_zone, label)
}
```

#### main.rs — Populate new fields

In the `DebugStatus` construction for gameplay, call `gs.current_zone_info()` and
populate `current_zone_idx` and `current_zone_label`.

#### Layout change

Change the status header from 2-column to 3-column horizontal split:

```
Before: Status (65%) | VFX (35%)
After:  Status (40%) | Geography (35%) | VFX (25%)
```

#### Move fields from Status to Geography

Remove from Status line 2: `Hero: (x, y)`, `Region: n`.

Geography panel content:

```
Hero: (xxxx, yyyy)
Region: n
Zone: [idx] label    (or "—" when None)
```

## Original etype Branching Reference

For implementation of the zone logic beyond this spec:

```
etype == 83              → princess rescue (check ob_list8[9].ob_stat)
etype >= 70              → carrier load (load_carrier(v3))
etype == 60 || etype == 61 → forced encounter at zone center
etype == 52              → astral plane (always encounter_type = 8)
etype >= 50              → forced encounter at hero position (every 16 ticks)
etype < 50               → random encounter (every 32 ticks, danger = base + etype)
  etype == 7             → swamp: race 2 overridden to 4 (snake)
  etype == 8             → spider zone: race forced to 6 (spider)
  etype == 49            → always wraith (race 2)
etype < 70               → deactivate carrier
```

## Scope

This spec covers:
- TOML schema and all 22 zone entries in faery.toml
- `ZoneConfig` struct update in game_library.rs
- `zones.rs` refactor (drop static table, runtime helpers)
- `gameplay_scene.rs` zone detection fix (drop region filter)
- Debug console: new Geography panel (hero coords, region, current zone)

This spec does NOT cover (future work):
- Encounter spawning logic (the spawn timer, danger calculation, `set_encounter`)
- Carrier loading logic (`load_carrier`)
- Princess rescue sequence
- Necromancer magic-blocking
- DKnight fixed-position spawn
- Combat system integration
