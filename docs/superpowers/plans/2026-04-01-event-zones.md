# Event Zones Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Populate all 23 event zones from the original game's `extent_list[]` in
`faery.toml`, update the `ZoneConfig` struct and `zones.rs` helpers to match the
original fields, fix the zone detection logic in `gameplay_scene.rs`, and add a
Geography debug panel.

**Architecture:** `faery.toml` is the single source of zone data (23 `[[zones]]`
entries). `game_library.rs` deserializes them into `Vec<ZoneConfig>`. `zones.rs`
provides runtime helpers operating on `&[ZoneConfig]` (no static table). The debug
console gains a Geography panel showing hero position, region, and current zone.

**Tech Stack:** Rust, serde/toml, ratatui

**Spec:** `docs/superpowers/specs/2026-04-01-event-zones-design.md`

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `src/game/game_library.rs` | Modify (lines 81–92) | `ZoneConfig` struct: replace old fields with `label`/`etype`/`v1`/`v2`/`v3` |
| `src/game/zones.rs` | Rewrite | Drop static `ZONE_TABLE`; add `ZoneType::Peace`, `from_etype()`, coordinate-only helpers on `&[ZoneConfig]` |
| `faery.toml` | Modify (lines 824–833) | Replace commented-out zone template with 23 `[[zones]]` entries |
| `src/game/gameplay_scene.rs` | Modify (lines 248–250, 3508–3531) | Update zone detection: strict inequality, drop region filter, use new field names |
| `src/game/debug_console.rs` | Modify (lines 43–75, 313–428) | Add Geography panel, move hero coords & region from Status |
| `src/main.rs` | Modify (lines 438–468) | Populate new `DebugStatus` geography fields |

---

### Task 1: Update `ZoneConfig` struct

**Files:**
- Modify: `src/game/game_library.rs:81-92`

- [ ] **Step 1: Replace `ZoneConfig` struct**

In `src/game/game_library.rs`, replace lines 81–92:

```rust
#[derive(Deserialize, Debug, Clone)]
pub struct ZoneConfig {
    pub zone_type:      String,
    pub x1:             u16,
    pub y1:             u16,
    pub x2:             u16,
    pub y2:             u16,
    pub region:         u8,
    pub encounter_rate: u8,
    #[serde(default)]
    pub event_id:       u8,
}
```

with:

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

- [ ] **Step 2: Verify it compiles (expect errors in consumers)**

Run: `cargo build 2>&1 | head -30`

Expected: Compile errors in `gameplay_scene.rs` referencing old fields (`zone_type`,
`region`, `encounter_rate`, `event_id`). This is expected — we'll fix those in
Task 4.

- [ ] **Step 3: Commit**

```bash
git add src/game/game_library.rs
git commit -m "refactor: update ZoneConfig to match original extent fields

Replace zone_type/region/encounter_rate/event_id with
label/etype/v1/v2/v3 matching the original extent struct from fmain.c.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 2: Rewrite `zones.rs`

**Files:**
- Rewrite: `src/game/zones.rs`

- [ ] **Step 1: Write the new `zones.rs`**

Replace the entire contents of `src/game/zones.rs` with:

```rust
//! Encounter zones and extents: 23 trigger rectangles from the original.
//! Each zone can trigger encounters, carrier spawns, or special events.
//! Zone data lives in faery.toml; this module provides runtime helpers.

use crate::game::game_library::ZoneConfig;

/// Total zone entries in the original (indices 0–21 iterated; index 22 is fallback).
pub const EXT_COUNT: usize = 22;

/// Zone categories derived from etype at runtime.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoneType {
    /// Random encounter zone (etype 0–49). etype value is the danger modifier.
    Encounter,
    /// Forced/special encounter (etype 50–69): spiders, necromancer, astral, etc.
    Special,
    /// Carrier spawn point (etype 70–79): bird, turtle, dragon.
    Carrier,
    /// Peace/NPC zone (etype 80–89): palaces, villages, buildings — no random encounters.
    Peace,
}

impl ZoneType {
    /// Derive zone category from the raw etype value.
    pub fn from_etype(etype: u8) -> ZoneType {
        match etype {
            0..=49  => ZoneType::Encounter,
            50..=69 => ZoneType::Special,
            70..=79 => ZoneType::Carrier,
            _       => ZoneType::Peace,  // 80+
        }
    }
}

/// Check if a point is inside a zone using strict inequality (matching original).
/// The original uses `hero_x > x1 && hero_x < x2 && hero_y > y1 && hero_y < y2`.
pub fn zone_contains(z: &ZoneConfig, x: u16, y: u16) -> bool {
    x > z.x1 && x < z.x2 && y > z.y1 && y < z.y2
}

/// Find the first matching zone (indices 0..EXT_COUNT), or fall back to the last
/// entry (the "whole world" sentinel) if no specific zone matches.
/// Returns the index into the zones slice.
///
/// Mirrors fmain.c lines 3281–3287: iterate extent_list[0..EXT_COUNT], and if
/// nothing matches the extn pointer naturally falls through to extent_list[22].
pub fn find_zone(zones: &[ZoneConfig], x: u16, y: u16) -> Option<usize> {
    let scan_count = zones.len().min(EXT_COUNT);
    for i in 0..scan_count {
        if zone_contains(&zones[i], x, y) {
            return Some(i);
        }
    }
    // Fall through to sentinel (last entry) if it exists beyond the scan range.
    if zones.len() > EXT_COUNT {
        let sentinel = zones.len() - 1;
        if zone_contains(&zones[sentinel], x, y) {
            return Some(sentinel);
        }
    }
    None
}

/// Check if the position is in any random-encounter zone (etype < 50).
pub fn in_encounter_zone(zones: &[ZoneConfig], x: u16, y: u16) -> bool {
    if let Some(idx) = find_zone(zones, x, y) {
        ZoneType::from_etype(zones[idx].etype) == ZoneType::Encounter
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_zone(label: &str, etype: u8, x1: u16, y1: u16, x2: u16, y2: u16) -> ZoneConfig {
        ZoneConfig { label: label.to_string(), etype, x1, y1, x2, y2, v1: 0, v2: 0, v3: 0 }
    }

    #[test]
    fn zone_type_from_etype() {
        assert_eq!(ZoneType::from_etype(0), ZoneType::Encounter);
        assert_eq!(ZoneType::from_etype(3), ZoneType::Encounter);
        assert_eq!(ZoneType::from_etype(49), ZoneType::Encounter);
        assert_eq!(ZoneType::from_etype(50), ZoneType::Special);
        assert_eq!(ZoneType::from_etype(52), ZoneType::Special);
        assert_eq!(ZoneType::from_etype(60), ZoneType::Special);
        assert_eq!(ZoneType::from_etype(69), ZoneType::Special);
        assert_eq!(ZoneType::from_etype(70), ZoneType::Carrier);
        assert_eq!(ZoneType::from_etype(79), ZoneType::Carrier);
        assert_eq!(ZoneType::from_etype(80), ZoneType::Peace);
        assert_eq!(ZoneType::from_etype(83), ZoneType::Peace);
    }

    #[test]
    fn zone_contains_strict_inequality() {
        let z = make_zone("test", 3, 10, 20, 100, 200);
        // Interior point
        assert!(zone_contains(&z, 50, 100));
        // On boundary — strict inequality means these are NOT inside
        assert!(!zone_contains(&z, 10, 100)); // x == x1
        assert!(!zone_contains(&z, 100, 100)); // x == x2
        assert!(!zone_contains(&z, 50, 20)); // y == y1
        assert!(!zone_contains(&z, 50, 200)); // y == y2
        // Outside
        assert!(!zone_contains(&z, 5, 100));
        assert!(!zone_contains(&z, 50, 300));
    }

    #[test]
    fn zone_contains_inverted_y_coords() {
        // Zone 20 (around village) has y1=18719 > y2=17484.
        // With strict inequality, no point can satisfy y > 18719 && y < 17484.
        // This zone effectively never matches via coordinate check alone,
        // which matches original behavior (it's a metadata-only zone).
        let z = make_zone("around village", 3, 16953, 18719, 20240, 17484);
        assert!(!zone_contains(&z, 19000, 18000));
        assert!(!zone_contains(&z, 19000, 17000));
    }

    #[test]
    fn find_zone_returns_first_match() {
        let zones: Vec<ZoneConfig> = (0..23).map(|i| {
            if i == 3 {
                make_zone("spider pit", 53, 4063, 34819, 4909, 35125)
            } else if i == 22 {
                make_zone("whole world", 3, 0, 0, 32767, 40959)
            } else {
                make_zone("empty", 80, 0, 0, 0, 0)
            }
        }).collect();

        // Point inside spider pit
        assert_eq!(find_zone(&zones, 4500, 35000), Some(3));
        // Point not in any specific zone — falls through to sentinel
        assert_eq!(find_zone(&zones, 15000, 20000), Some(22));
    }

    #[test]
    fn find_zone_empty_list() {
        let zones: Vec<ZoneConfig> = vec![];
        assert_eq!(find_zone(&zones, 100, 100), None);
    }

    #[test]
    fn in_encounter_zone_checks_etype() {
        let zones: Vec<ZoneConfig> = (0..23).map(|i| {
            if i == 16 {
                // swamp region: etype=7 (< 50 = encounter zone)
                make_zone("swamp region", 7, 6156, 12755, 12316, 15905)
            } else if i == 12 {
                // peace zone: etype=80 (>= 80 = peace)
                make_zone("peace 1", 80, 2752, 33300, 8632, 35400)
            } else if i == 22 {
                // whole world fallback: etype=3 (< 50 = encounter)
                make_zone("whole world", 3, 0, 0, 32767, 40959)
            } else {
                make_zone("empty", 80, 0, 0, 0, 0)
            }
        }).collect();

        // In swamp region (etype=7 → Encounter)
        assert!(in_encounter_zone(&zones, 8000, 14000));
        // In peace zone (etype=80 → Peace)
        assert!(!in_encounter_zone(&zones, 5000, 34000));
        // Fallback to whole world (etype=3 → Encounter)
        assert!(in_encounter_zone(&zones, 15000, 20000));
    }
}
```

- [ ] **Step 2: Verify tests pass**

Run: `cargo test zone -- --nocapture`

Expected: All 7 new tests pass:
- `zone_type_from_etype`
- `zone_contains_strict_inequality`
- `zone_contains_inverted_y_coords`
- `find_zone_returns_first_match`
- `find_zone_empty_list`
- `in_encounter_zone_checks_etype`

- [ ] **Step 3: Commit**

```bash
git add src/game/zones.rs
git commit -m "refactor: rewrite zones.rs with runtime helpers for ZoneConfig

Drop static ZONE_TABLE. Add ZoneType::Peace, from_etype(), find_zone()
with sentinel fallback, strict inequality matching the original.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 3: Populate `faery.toml` with 23 zone entries

**Files:**
- Modify: `faery.toml:824-833`

- [ ] **Step 1: Replace the commented-out zone template**

In `faery.toml`, replace lines 824–833 (the commented-out zone template block):

```toml
# Zone table schema — entries will go here once ADF analysis is complete.
# Each zone is a rectangular trigger area in a region.
# [[zones]]
# zone_type = "None"     # one of: "None", "Encounter", "Carrier", "Special"
# x1 = 0
# y1 = 0
# x2 = 0
# y2 = 0
# region = 255           # 255 = any region
# encounter_rate = 0     # 0-255
```

with the full 23 zone entries (all from `original/fmain.c` lines 388–419):

```toml
# ── Extent / zone table ──────────────────────────────────────────────────────
# 23 entries transcribed from original/fmain.c extent_list[].
# Fields: label, etype (0-83), x1/y1/x2/y2 (world coords), v1/v2/v3 (raw).
# The original loop iterates indices 0–21 (EXT_COUNT=22).
# Index 22 ("whole world") is a fallback sentinel: the extn pointer falls
# through to it when no zone 0–21 matches.
#
# etype ranges:
#   0–49  = random encounter (value is danger modifier)
#   50–69 = forced/special encounter (spiders, necromancer, astral, etc.)
#   70–79 = carrier (bird, turtle, dragon)
#   80+   = peace/NPC zone (palaces, villages, buildings)

[[zones]]
label = "bird extent"
etype = 70                 # carrier
x1 = 2118
y1 = 27237
x2 = 2618
y2 = 27637
v1 = 0                    # not used for carriers
v2 = 1                    # not used for carriers
v3 = 11                   # carrier cfile: bird

[[zones]]
label = "turtle extent"
etype = 70                 # carrier
x1 = 0                    # coords set at runtime
y1 = 0
x2 = 0
y2 = 0
v1 = 0
v2 = 1
v3 = 5                    # carrier cfile: turtle

[[zones]]
label = "dragon extent"
etype = 70                 # carrier
x1 = 6749
y1 = 34951
x2 = 7249
y2 = 35351
v1 = 0
v2 = 1
v3 = 10                   # carrier cfile: dragon

[[zones]]
label = "spider pit"
etype = 53                 # forced: spider encounter
x1 = 4063
y1 = 34819
x2 = 4909
y2 = 35125
v1 = 4                    # base count: 4 spiders
v2 = 1                    # spread: fixed count (4+rnd(1))
v3 = 6                    # race: spider

[[zones]]
label = "necromancer"
etype = 60                 # forced: special encounter
x1 = 9563
y1 = 33883
x2 = 10144
y2 = 34462
v1 = 1                    # base count: 1 necromancer
v2 = 1                    # spread: fixed
v3 = 9                    # race: necromancer (blocks magic in combat)

[[zones]]
label = "turtle eggs"
etype = 61                 # forced: guardian encounter
x1 = 22945
y1 = 5597
x2 = 23225
y2 = 5747
v1 = 3                    # base count: 3 snakes
v2 = 2                    # spread: 3–4 snakes (3+rnd(2))
v3 = 4                    # race: snake

[[zones]]
label = "princess extent"
etype = 83                 # special: princess rescue trigger
x1 = 10820
y1 = 35646
x2 = 10877
y2 = 35670
v1 = 1
v2 = 1
v3 = 0                    # no encounter race

[[zones]]
label = "graveyard ext"
etype = 48                 # forced: set-group encounter
x1 = 19596
y1 = 17123
x2 = 19974
y2 = 17401
v1 = 8                    # base count: 8 wraiths
v2 = 8                    # spread: 8–15 wraiths (8+rnd(8))
v3 = 2                    # race: wraith

[[zones]]
label = "around city"
etype = 80                 # peace: palace zone
x1 = 19400
y1 = 17034
x2 = 20240
y2 = 17484
v1 = 4                    # base count
v2 = 20                   # spread: 4–23 generic (4+rnd(20))
v3 = 0                    # race: generic

[[zones]]
label = "astral plane"
etype = 52                 # forced: astral encounter (always Loraii)
x1 = 9216                 # 0x2400
y1 = 33280                # 0x8200
x2 = 12544                # 0x3100
y2 = 35328                # 0x8a00
v1 = 3                    # base count: 3 loraii
v2 = 1                    # spread: fixed
v3 = 8                    # race: loraii

[[zones]]
label = "king pax"
etype = 81                 # peace: king's palace
x1 = 5272
y1 = 33300
x2 = 6112
y2 = 34200
v1 = 0
v2 = 1
v3 = 0

[[zones]]
label = "sorceress pax"
etype = 82                 # peace: sorceress palace
x1 = 11712
y1 = 37350
x2 = 12416
y2 = 38020
v1 = 0
v2 = 1
v3 = 0

[[zones]]
label = "peace 1 - buildings"
etype = 80                 # peace: buildings area
x1 = 2752
y1 = 33300
x2 = 8632
y2 = 35400
v1 = 0
v2 = 1
v3 = 0

[[zones]]
label = "peace 2 - specials"
etype = 80                 # peace: specials area
x1 = 10032
y1 = 35550
x2 = 12976
y2 = 40270
v1 = 0
v2 = 1
v3 = 0

[[zones]]
label = "peace 3 - cabins"
etype = 80                 # peace: cabins area
x1 = 4712
y1 = 38100
x2 = 10032
y2 = 40350
v1 = 0
v2 = 1
v3 = 0

[[zones]]
label = "hidden valley"
etype = 60                 # forced: special encounter
x1 = 21405
y1 = 25583
x2 = 21827
y2 = 26028
v1 = 1                    # base count: 1 DKnight
v2 = 1                    # spread: fixed
v3 = 7                    # race: DKnight (fixed spawn at 21635,25762)

[[zones]]
label = "swamp region"
etype = 7                  # random: danger modifier 7 (high)
x1 = 6156
y1 = 12755
x2 = 12316
y2 = 15905
v1 = 1                    # base count: 1
v2 = 8                    # spread: 1–8 (1+rnd(8))
v3 = 0                    # race: random; etype=7 overrides race 2→4 (snake)

[[zones]]
label = "spider region"
etype = 8                  # random: danger modifier 8; forces race 6 (spider)
x1 = 5140
y1 = 34860
x2 = 6260
y2 = 37260
v1 = 1                    # base count: 1
v2 = 8                    # spread: 1–8
v3 = 0                    # race: forced to 6 (spider) by etype=8

[[zones]]
label = "spider region 2"
etype = 8                  # random: danger modifier 8; forces race 6 (spider)
x1 = 660
y1 = 33510
x2 = 2060
y2 = 34560
v1 = 1
v2 = 8
v3 = 0

[[zones]]
label = "village"
etype = 80                 # peace: village zone
x1 = 18687
y1 = 15338
x2 = 19211
y2 = 16136
v1 = 0
v2 = 1
v3 = 0

[[zones]]
label = "around village"
etype = 3                  # random: danger modifier 3
x1 = 16953
y1 = 18719                # NOTE: y1 > y2 is intentional in the original
x2 = 20240
y2 = 17484
v1 = 1                    # base count: 1
v2 = 3                    # spread: 1–3 (1+rnd(3))
v3 = 0                    # race: generic (0–3 random)

[[zones]]
label = "around city 2"
etype = 3                  # random: danger modifier 3
x1 = 20593
y1 = 18719
x2 = 23113
y2 = 22769
v1 = 1                    # base count: 1
v2 = 3                    # spread: 1–3
v3 = 0                    # race: generic

[[zones]]
label = "whole world"
etype = 3                  # random: danger modifier 3 (lowest outdoor danger=5)
x1 = 0                    # 0x0000
y1 = 0                    # 0x0000
x2 = 32767                # 0x7fff
y2 = 40959                # 0x9fff
v1 = 1                    # base count: 1
v2 = 8                    # spread: 1–8 (1+rnd(8))
v3 = 0                    # race: generic
# FALLBACK SENTINEL: not iterated in the main loop (EXT_COUNT=22);
# the extn pointer falls through to this entry when no zone 0–21 matches.
```

- [ ] **Step 2: Verify TOML parses (count zones)**

Run: `python3 -c "import tomllib; d=tomllib.load(open('faery.toml','rb')); print(len(d['zones']), 'zones loaded'); assert len(d['zones'])==23"`

Expected: `23 zones loaded`

- [ ] **Step 3: Commit**

```bash
git add faery.toml
git commit -m "data: populate 23 event zones in faery.toml

All entries transcribed from original/fmain.c extent_list[].
Includes inline comments explaining v1/v2/v3 semantics per zone.
Index 22 (whole world) is the fallback sentinel.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 4: Update `gameplay_scene.rs` zone detection

**Files:**
- Modify: `src/game/gameplay_scene.rs:3508-3531`

This task fixes all compile errors from Task 1 (old field names) and updates the
zone detection to match the original's coordinate-only check with strict inequality.

- [ ] **Step 1: Update the encounter zone check (line 3508–3510)**

Replace:

```rust
        // Encounter zone check (world-111)
        self.in_encounter_zone = crate::game::zones::in_encounter_zone(
            self.state.region_num, self.state.hero_x, self.state.hero_y);
```

with:

```rust
        // Encounter zone check (world-111)
        self.in_encounter_zone = crate::game::zones::in_encounter_zone(
            &self.zones, self.state.hero_x, self.state.hero_y);
```

- [ ] **Step 2: Update the event zone entry check (lines 3512–3531)**

Replace:

```rust
        // Event zone entry check (#107)
        {
            let hx = self.state.hero_x;
            let hy = self.state.hero_y;
            let region = self.state.region_num;
            let current_zone = self.zones.iter().position(|z|
                z.region == region
                    && hx >= z.x1 && hx <= z.x2
                    && hy >= z.y1 && hy <= z.y2
            );
            if current_zone != self.last_zone {
                if let Some(idx) = current_zone {
                    let event_id = self.zones[idx].event_id as usize;
                    let bname = brother_name(&self.state);
                    let msg = crate::game::events::event_msg(&self.narr, event_id, bname);
                    if !msg.is_empty() { self.messages.push(msg); }
                }
                self.last_zone = current_zone;
            }
        }
```

with:

```rust
        // Event zone entry check (#107)
        {
            let hx = self.state.hero_x;
            let hy = self.state.hero_y;
            let current_zone = crate::game::zones::find_zone(&self.zones, hx, hy);
            if current_zone != self.last_zone {
                self.last_zone = current_zone;
            }
        }
```

Note: The old code triggered `event_msg` on zone entry. With the new schema there
is no `event_id` field — event messages will be driven by etype-specific logic in
future work. For now, zone detection tracks the current zone index only.

- [ ] **Step 3: Build and run full test suite**

Run: `cargo build && cargo test`

Expected: Build succeeds. All 18 tests pass (12 existing + 6 new from zones.rs).

- [ ] **Step 4: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix: update zone detection to use new ZoneConfig fields

Use find_zone() with strict inequality and coordinate-only matching.
Drop region filter and old event_id dispatch (replaced by etype logic
in future work).

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 5: Add Geography debug panel

**Files:**
- Modify: `src/game/debug_console.rs:43-75` (DebugStatus struct)
- Modify: `src/game/debug_console.rs:313-428` (render function)
- Modify: `src/game/gameplay_scene.rs` (add accessor)
- Modify: `src/main.rs:438-468` (populate DebugStatus)

- [ ] **Step 1: Add zone fields to `DebugStatus`**

In `src/game/debug_console.rs`, insert before the `// VFX state` comment (line 67):

```rust
    // Geography
    pub current_zone_idx: Option<usize>,
    pub current_zone_label: Option<String>,
```

- [ ] **Step 2: Add `current_zone_info()` accessor to GameplayScene**

In `src/game/gameplay_scene.rs`, after the `is_palette_xfade_active()` method
(around line 400), add:

```rust
    /// Current zone index and label for the debug console.
    pub fn current_zone_info(&self) -> (Option<usize>, Option<String>) {
        let label = self.last_zone
            .and_then(|i| self.zones.get(i).map(|z| z.label.clone()));
        (self.last_zone, label)
    }
```

- [ ] **Step 3: Populate geography fields in `main.rs`**

In `src/main.rs`, in the gameplay `DebugStatus` construction (around line 461,
after the `cave_mode` line), add:

```rust
                    current_zone_idx: {
                        let (idx, _) = gs.current_zone_info();
                        idx
                    },
                    current_zone_label: {
                        let (_, label) = gs.current_zone_info();
                        label
                    },
```

- [ ] **Step 4: Change the header layout to 3-column**

In `src/game/debug_console.rs`, replace the 2-column horizontal layout
(around lines 323–330):

```rust
            // Split status header horizontally: Status (left) | VFX (right)
            let status_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(65),
                    Constraint::Percentage(35),
                ])
                .split(chunks[0]);
```

with:

```rust
            // Split status header: Status (left) | Geography (center) | VFX (right)
            let status_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Percentage(35),
                    Constraint::Percentage(25),
                ])
                .split(chunks[0]);
```

- [ ] **Step 5: Move Hero/Region from Status to Geography panel**

In `src/game/debug_console.rs`, replace Status line 2 (around lines 365–374):

```rust
                Line::from(vec![
                    styled_label("Hero: "),
                    Span::raw(format!("({:4},{:4})  ", status.hero_x, status.hero_y)),
                    styled_label("Region: "),
                    Span::raw(format!("{}  ", status.region_num)),
                    styled_label("Brother: "),
                    Span::raw(format!("{}  ", brother_name)),
                    styled_label("Scene: "),
                    Span::raw(format!("{}  ", scene_str)),
                ]),
```

with:

```rust
                Line::from(vec![
                    styled_label("Brother: "),
                    Span::raw(format!("{}  ", brother_name)),
                    styled_label("Scene: "),
                    Span::raw(format!("{}  ", scene_str)),
                ]),
```

- [ ] **Step 6: Add the Geography panel rendering**

In `src/game/debug_console.rs`, insert after the Status widget render
(`f.render_widget(status_widget, status_chunks[0]);`, around line 397) and
before the VFX status comment:

```rust
            // ── Geography ─────────────────────────────────────────────
            let zone_str = match (status.current_zone_idx, &status.current_zone_label) {
                (Some(idx), Some(label)) => format!("[{}] {}", idx, label),
                _ => "—".to_string(),
            };
            let geo_text = vec![
                Line::from(vec![
                    styled_label("Hero: "),
                    Span::raw(format!("({:5},{:5})", status.hero_x, status.hero_y)),
                ]),
                Line::from(vec![
                    styled_label("Region: "),
                    Span::raw(format!("{}", status.region_num)),
                ]),
                Line::from(vec![
                    styled_label("Zone: "),
                    Span::raw(zone_str),
                ]),
            ];

            let geo_widget = Paragraph::new(geo_text)
                .block(Block::default().borders(Borders::ALL).title(" Geography "));
            f.render_widget(geo_widget, status_chunks[1]);
```

- [ ] **Step 7: Update VFX widget to use `status_chunks[2]`**

In `src/game/debug_console.rs`, change the VFX render line from:

```rust
            f.render_widget(vfx_widget, status_chunks[1]);
```

to:

```rust
            f.render_widget(vfx_widget, status_chunks[2]);
```

- [ ] **Step 8: Build and test**

Run: `cargo build && cargo test`

Expected: Build succeeds. All 18 tests pass.

- [ ] **Step 9: Commit**

```bash
git add src/game/debug_console.rs src/game/gameplay_scene.rs src/main.rs
git commit -m "feat: add Geography debug panel with zone display

Move hero coords and region from Status to new Geography panel.
Display current zone index and label. 3-column header layout:
Status (40%) | Geography (35%) | VFX (25%).

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 6: Final verification

- [ ] **Step 1: Full build and test**

Run: `cargo build && cargo test`

Expected: Build succeeds. All 18 tests pass.

- [ ] **Step 2: Verify zone count in faery.toml**

Run: `python3 -c "import tomllib; d=tomllib.load(open('faery.toml','rb')); zs=d['zones']; print(f'{len(zs)} zones'); assert len(zs)==23; assert zs[0]['label']=='bird extent'; assert zs[22]['label']=='whole world'; print('OK')"`

Expected: `23 zones` then `OK`

- [ ] **Step 3: Verify zone labels match original comments**

Run: `python3 -c "import tomllib; d=tomllib.load(open('faery.toml','rb')); [print(f'{i:2}: {z[\"label\"]:25s} etype={z[\"etype\"]:2}  ({z[\"x1\"]},{z[\"y1\"]})–({z[\"x2\"]},{z[\"y2\"]})') for i,z in enumerate(d['zones'])]"`

Expected: All 23 zones listed with correct labels and coordinates matching the spec.
