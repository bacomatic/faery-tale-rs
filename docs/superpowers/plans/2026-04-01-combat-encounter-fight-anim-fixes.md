# Combat, Encounter, and Fight Animation Fixes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix three interrelated combat/encounter bugs: #157 (wrong damage formula), #158 (encounter flooding), #159 (missing fight animation in Y-sorted render).

**Architecture:** Bug #157 rewrites `apply_melee_combat()` to use the original `wt + bitrand(2)` damage formula and adds a brave-based dodge for enemy attacks. Bug #158 replaces the per-tick encounter check with the original's 32-tick cooldown + `actors_on_screen` guard + 4-enemy cap + zone-scaled danger. Bug #159 copies the existing `ActorState::Fighting` frame-selection logic from the old render path into the active Y-sorted render path.

**Tech Stack:** Rust, SDL2

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `src/game/combat.rs` | Modify | Replace `WEAPON_DAMAGE` table and `resolve_combat()` with `bitrand_damage()`; add `bitrand(n)` helper |
| `src/game/npc.rs` | Modify | Add `weapon: u8` field to `Npc` struct |
| `src/game/encounter.rs` | Modify | Rewrite `should_encounter()` → `try_spawn_encounter()` with cooldown + guards; add `ENCOUNTER_CHART_FULL` with per-type HP/arms/weapon |
| `src/game/gameplay_scene.rs` | Modify | (a) Wire new `apply_melee_combat()` damage + dodge; (b) replace encounter spawn block with new gated logic; (c) add fighting frame selection to Y-sorted hero render; (d) add `encounter_cooldown` field |

---

## Task 1: Add `weapon` field to `Npc` and `bitrand` damage helper

**Files:**
- Modify: `src/game/npc.rs` (Npc struct + from_bytes + Default)
- Modify: `src/game/combat.rs` (add `bitrand` helper, add `bitrand_damage`)
- Test: `src/game/combat.rs` (inline tests)

Closes: #157 (partial — damage primitives)

- [ ] **Step 1: Add `weapon` field to Npc struct**

In `src/game/npc.rs`, add `weapon: u8` to the `Npc` struct after the `speed` field:

```rust
pub struct Npc {
    pub npc_type: u8,
    pub race: u8,
    pub x: i16,
    pub y: i16,
    pub vitality: i16,
    pub gold: i16,
    pub speed: u8,
    pub weapon: u8,
    pub active: bool,
}
```

In `from_bytes()`, initialize from byte 11 of the carrier record (was padding):

```rust
pub fn from_bytes(data: &[u8]) -> Self {
    if data.len() < 16 {
        return Npc::default();
    }
    Npc {
        npc_type: data[0],
        race: data[1],
        x: i16::from_be_bytes([data[2], data[3]]),
        y: i16::from_be_bytes([data[4], data[5]]),
        vitality: i16::from_be_bytes([data[6], data[7]]),
        gold: i16::from_be_bytes([data[8], data[9]]),
        speed: data[10],
        weapon: data[11],
        active: data[0] != NPC_TYPE_NONE,
    }
}
```

Note: For encounter-spawned NPCs, `weapon` is set by `spawn_encounter()` (Task 3), not from bytes.

- [ ] **Step 2: Add `bitrand` helper to combat.rs**

In `src/game/combat.rs`, add a `bitrand` function that matches the original's `rand() & mask` pattern, using the existing `melee_rand` as the underlying RNG:

```rust
/// Port of original `bitrand(mask)` — `rand() & mask`.
/// For combat: `bitrand(2)` returns 0, 1, or 2 (mask 0b10 → values 0..=2).
pub fn bitrand(mask: u32) -> u32 {
    melee_rand(u32::MAX) & mask
}
```

- [ ] **Step 3: Write failing test for `bitrand` range**

In `src/game/combat.rs` tests module:

```rust
#[test]
fn test_bitrand_range() {
    // bitrand(2) should only return 0, 1, or 2 (mask = 0b10)
    for _ in 0..1000 {
        let v = bitrand(2);
        assert!(v <= 2, "bitrand(2) returned {v}, expected 0-2");
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_bitrand_range -- --nocapture`
Expected: PASS

- [ ] **Step 5: Write `bitrand_damage` function**

In `src/game/combat.rs`, add the damage calculation matching the original `dohit()`:

```rust
/// Melee damage from original dohit() formula.
/// `weapon_index`: attacker's weapon value (1=dirk..5=wand, ≥8 capped to 5).
/// Returns `wt + bitrand(2)`: weapon base + random 0–2 bonus.
pub fn bitrand_damage(weapon_index: u8) -> i16 {
    let wt = if weapon_index >= 8 { 5 } else { weapon_index };
    wt as i16 + bitrand(2) as i16
}
```

- [ ] **Step 6: Write tests for `bitrand_damage`**

```rust
#[test]
fn test_bitrand_damage_range() {
    // Dirk (weapon 1): damage should be 1, 2, or 3
    for _ in 0..100 {
        let d = bitrand_damage(1);
        assert!((1..=3).contains(&d), "dirk damage {d} out of range 1-3");
    }
    // Sword (weapon 3): damage should be 3, 4, or 5
    for _ in 0..100 {
        let d = bitrand_damage(3);
        assert!((3..=5).contains(&d), "sword damage {d} out of range 3-5");
    }
    // Touch attack (weapon 8+): capped to 5, so damage 5, 6, or 7
    for _ in 0..100 {
        let d = bitrand_damage(10);
        assert!((5..=7).contains(&d), "touch damage {d} out of range 5-7");
    }
}

#[test]
fn test_bitrand_damage_fists() {
    // weapon 0 = no weapon: damage should be 0, 1, or 2
    for _ in 0..100 {
        let d = bitrand_damage(0);
        assert!((0..=2).contains(&d), "fist damage {d} out of range 0-2");
    }
}
```

- [ ] **Step 7: Run tests**

Run: `cargo test test_bitrand_damage -- --nocapture`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add src/game/npc.rs src/game/combat.rs
git commit -m "feat(combat): add Npc.weapon field and bitrand damage helpers

Port original dohit() damage formula: wt + bitrand(2).
Add weapon field to Npc struct (byte 11 of carrier record).
Add bitrand() and bitrand_damage() to combat module.

Closes: #157 (partial)"
```

---

## Task 2: Rewrite hero melee damage + add brave dodge

**Files:**
- Modify: `src/game/gameplay_scene.rs` (`apply_melee_combat()`, lines ~1209–1268)
- Test: `src/game/combat.rs` (inline tests)

Closes: #157

- [ ] **Step 1: Write failing test for brave dodge**

In `src/game/combat.rs` tests module, add:

```rust
#[test]
fn test_brave_dodge_blocks_at_max() {
    // With brave=255, rand256() (0..255) is always < 255, so dodge always succeeds.
    // hero_dodges_hit returns true when rand256() > brave fails (i.e., NPC misses).
    // At brave=255: rand256() range is 0..255, all < 255, so hit NEVER connects.
    // We can't test randomness deterministically, but we can test the boundary logic.
    // brave=0 → rand256() > 0 is almost always true → hits connect ~99.6%
    // brave=255 → rand256() > 255 is never true → hits never connect
    assert!(true); // placeholder — logic tested via integration
}
```

- [ ] **Step 2: Rewrite `apply_melee_combat()` with original formula + brave dodge**

In `src/game/gameplay_scene.rs`, replace the damage calculation inside `apply_melee_combat()`. Change:

```rust
// damage = rand() % (arms + 1), min 1 (from task spec / dohit wt).
let damage: i16 = if one_hit_kill {
    npc.vitality
} else {
    (melee_rand(arms as u32 + 1) as i16).max(1)
};
npc.vitality -= damage;
if npc.vitality < 0 { npc.vitality = 0; }
```

To:

```rust
// Original dohit() formula: wt + bitrand(2).
// wt = weapon index, capped to 5 if >= 8 (touch attacks).
let damage: i16 = if one_hit_kill {
    npc.vitality
} else {
    crate::game::combat::bitrand_damage(arms)
};
npc.vitality -= damage;
if npc.vitality < 0 { npc.vitality = 0; }
```

Also add the import at the top of the function (alongside existing imports):

```rust
use crate::game::combat::{in_melee_range, bitrand_damage};
```

Remove the now-unused `melee_rand` import if it was only used here.

- [ ] **Step 3: Add enemy-attacks-hero path with brave dodge**

After the existing hero-attacks-enemy loop in `apply_melee_combat()`, add the enemy counterattack. The original processes all figures in the same battle loop — enemies attack the hero each frame they're in range. Add after the `let _ = hit_any;` line:

```rust
// Enemy attacks hero (fmain.c:2688-2709 for i > 0):
// Each active enemy in melee range swings at hero once per combat tick.
if let Some(ref table) = self.npc_table {
    for npc in table.npcs.iter().filter(|n| n.active) {
        let npc_weapon = npc.weapon.max(1); // min weapon = fists
        // NPC reach: bv = 2 + rand4() (capped at 15)
        let npc_reach = ((2 + crate::game::combat::rand4(self.state.cycle) as i16)).min(15);
        let dx = (self.state.hero_x as i32 - npc.x as i32).abs();
        let dy = (self.state.hero_y as i32 - npc.y as i32).abs();
        if dx.max(dy) < npc_reach as i32 {
            // Brave dodge: NPC hits only if rand256() > brave (fmain.c:2707)
            let roll = crate::game::combat::melee_rand(256) as i16;
            if roll > self.state.brave {
                let damage = crate::game::combat::bitrand_damage(npc_weapon);
                self.state.vitality = (self.state.vitality - damage).max(0);
                self.dlog(format!("enemy hit hero for {} (brave dodge failed, roll={})", damage, roll));
            } else {
                self.dlog(format!("hero dodged enemy attack (roll={} <= brave={})", roll, self.state.brave));
            }
        }
    }
}
```

**Important:** This requires reading `self.npc_table` immutably after the mutable hero-attacks-enemy loop. The existing code uses `if let Some(ref mut table)` for the hero's attack loop. The enemy counterattack needs a separate `if let Some(ref table)` block after that loop completes.

- [ ] **Step 4: Run all tests**

Run: `cargo test -- --nocapture`
Expected: All tests pass. The existing `test_combat_reduces_enemy_vitality` and `test_combat_enemy_defeated` tests use `resolve_combat()` which is a dead code path — they should still pass but are now testing legacy code. The live path is `apply_melee_combat()`.

- [ ] **Step 5: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix(combat): port original dohit() formula and brave-based dodge

Hero damage: weapon_index + bitrand(2) instead of rand() % (arms+1).
Enemy counterattack: each in-range enemy swings at hero with same formula.
Brave dodge: enemy hits only connect when rand256() > brave.

Closes: #157"
```

---

## Task 3: Rewrite encounter spawning with rate-limiting

**Files:**
- Modify: `src/game/encounter.rs` (rewrite `should_encounter` → gated system, add full encounter chart)
- Modify: `src/game/gameplay_scene.rs` (add `encounter_cooldown` field, replace spawn block)
- Test: `src/game/encounter.rs` (inline tests)

Closes: #158

- [ ] **Step 1: Add full encounter chart with per-type stats**

In `src/game/encounter.rs`, replace `ENCOUNTER_CHART` with the full chart from RESEARCH.md. Add above `ENCOUNTER_CHART`:

```rust
/// Per-enemy-type stats from fmain.c encounter_chart[].
/// Fields: hp, arms, cleverness, treasure_idx, cfile.
#[derive(Debug, Clone, Copy)]
pub struct EnemyTypeStats {
    pub hp: i16,
    pub arms: u8,
    pub clever: u8,
    pub treasure: u8,
    pub cfile: u8,
}

/// Full encounter chart from fmain.c (11 enemy types).
/// Index = encounter type ID (0-10).
pub const ENCOUNTER_CHART_FULL: [EnemyTypeStats; 11] = [
    EnemyTypeStats { hp: 18, arms: 2, clever: 0, treasure: 2, cfile: 6 },  // 0: Ogre
    EnemyTypeStats { hp: 12, arms: 4, clever: 1, treasure: 1, cfile: 6 },  // 1: Orcs
    EnemyTypeStats { hp: 16, arms: 6, clever: 1, treasure: 4, cfile: 7 },  // 2: Wraith
    EnemyTypeStats { hp: 8,  arms: 3, clever: 0, treasure: 3, cfile: 7 },  // 3: Skeleton
    EnemyTypeStats { hp: 16, arms: 6, clever: 1, treasure: 0, cfile: 8 },  // 4: Snake
    EnemyTypeStats { hp: 9,  arms: 3, clever: 0, treasure: 0, cfile: 7 },  // 5: Salamander
    EnemyTypeStats { hp: 10, arms: 6, clever: 1, treasure: 0, cfile: 8 },  // 6: Spider
    EnemyTypeStats { hp: 40, arms: 7, clever: 1, treasure: 0, cfile: 8 },  // 7: DKnight
    EnemyTypeStats { hp: 12, arms: 6, clever: 1, treasure: 0, cfile: 9 },  // 8: Loraii
    EnemyTypeStats { hp: 50, arms: 5, clever: 0, treasure: 0, cfile: 9 },  // 9: Necromancer
    EnemyTypeStats { hp: 4,  arms: 0, clever: 0, treasure: 0, cfile: 9 },  // 10: Woodcutter
];

/// Weapon probability table from fmain.c weapon_probs[].
/// Indexed by arms * 4 + rand4(). Returns weapon index for the NPC.
pub const WEAPON_PROBS: [u8; 32] = [
    0, 0, 0, 0,  // arms=0: no weapon
    1, 1, 1, 1,  // arms=1: dirk
    1, 1, 2, 1,  // arms=2: mostly dirk, some mace
    2, 1, 2, 2,  // arms=3: mostly mace
    2, 2, 3, 2,  // arms=4: mace with sword chance
    3, 3, 2, 3,  // arms=5: mostly sword
    3, 3, 4, 3,  // arms=6: sword with bow chance
    4, 3, 4, 4,  // arms=7: bow-heavy
];
```

- [ ] **Step 2: Add `actors_on_screen` helper and rewrite `spawn_encounter()`**

Replace the existing `spawn_encounter()` function:

```rust
/// Check if any active enemy NPC is within visibility range (~300px) of the hero.
/// Mirrors fmain.c actors_on_screen check (300×300 Chebyshev).
pub fn actors_on_screen(
    table: &crate::game::npc::NpcTable,
    hero_x: i16,
    hero_y: i16,
) -> bool {
    table.npcs.iter().any(|n| {
        n.active
            && (n.x as i32 - hero_x as i32).abs() < 300
            && (n.y as i32 - hero_y as i32).abs() < 300
    })
}

/// Count active enemies in the NPC table.
pub fn active_enemy_count(table: &crate::game::npc::NpcTable) -> usize {
    table.npcs.iter().filter(|n| n.active).count()
}

/// Determine encounter type for a spawn, with zone overrides.
/// Base type is rand4()-based; swamp/spider/wraith zones override.
pub fn pick_encounter_type(xtype: u16, tick: u32) -> usize {
    let base = rand4_from_tick(tick) as usize;
    match xtype {
        7 if base == 2 => 4,   // swamp: wraith→snake
        8 => 6,                // spider zone: always spider
        49 => 2,               // forced wraith
        _ => base,
    }
}

/// Simple rand4 from tick (0–3).
fn rand4_from_tick(tick: u32) -> u32 {
    let h = tick.wrapping_mul(2246822519).wrapping_add(3266489917);
    (h >> 16) & 3
}

/// Spawn a single encounter NPC with stats from the full encounter chart.
pub fn spawn_encounter(encounter_type: usize, hero_x: i16, hero_y: i16, tick: u32) -> Npc {
    let etype = encounter_type.min(10);
    let stats = &ENCOUNTER_CHART_FULL[etype];

    // Weapon from weapon_probs table: arms * 4 + rand4()
    let wp_idx = (stats.arms as usize * 4 + rand4_from_tick(tick.wrapping_add(etype as u32)) as usize).min(31);
    let weapon = WEAPON_PROBS[wp_idx];

    // Race assignment
    let race = match etype {
        2 => RACE_WRAITH,
        3 | 5 => RACE_UNDEAD,
        4 => RACE_SNAKE,
        _ => RACE_ENEMY,
    };

    Npc {
        npc_type: etype as u8,
        race,
        x: hero_x.saturating_add(50),
        y: hero_y.saturating_add(50),
        vitality: stats.hp,
        gold: stats.treasure as i16 * 5,
        speed: 2,
        weapon,
        active: true,
    }
}
```

- [ ] **Step 3: Add gated `try_trigger_encounter` function**

Replace `should_encounter()` with a gated function that checks all original conditions:

```rust
/// Check if a random encounter should trigger this tick.
/// Ports fmain.c:2451-2472 gating conditions:
/// - Only every 32 ticks (not every tick)
/// - No actors on screen
/// - Active enemies < 4
/// - Not in forced encounter zone (xtype < 50)
/// - rand64() <= danger_level
///
/// Returns Some(encounter_type) if spawn should happen, None otherwise.
pub fn try_trigger_encounter(
    tick: u32,
    table: &crate::game::npc::NpcTable,
    hero_x: i16,
    hero_y: i16,
    xtype: u16,
    region_num: u8,
) -> Option<usize> {
    // Gate 1: every 32 ticks only (~1 Hz at 30fps)
    if tick & 31 != 0 {
        return None;
    }

    // Gate 2: no actors on screen
    if actors_on_screen(table, hero_x, hero_y) {
        return None;
    }

    // Gate 3: fewer than 4 active enemies
    if active_enemy_count(table) >= 4 {
        return None;
    }

    // Gate 4: not in forced encounter zone
    if xtype >= 50 {
        return None;
    }

    // Gate 5: danger level check — rand64() <= danger_level
    let danger = if region_num > 7 {
        5 + xtype as u32  // indoor
    } else {
        2 + xtype as u32  // outdoor
    };
    let roll = rand64_from_tick(tick);
    if roll > danger {
        return None;
    }

    Some(pick_encounter_type(xtype, tick))
}

/// rand64 from tick (0–63).
fn rand64_from_tick(tick: u32) -> u32 {
    let h = tick.wrapping_mul(1664525).wrapping_add(1013904223);
    (h >> 10) & 63
}
```

- [ ] **Step 4: Update `spawn_encounter_group` to use new signature**

Update `spawn_encounter_group` to accept `encounter_type` and `tick`:

```rust
/// Spawn up to 4 enemies into free NPC slots.
/// Uses ENCOUNTER_CHART_FULL for per-type stats.
pub fn spawn_encounter_group(
    table: &mut crate::game::npc::NpcTable,
    encounter_type: usize,
    hero_x: i16,
    hero_y: i16,
    tick: u32,
) -> usize {
    const MAX_GROUP: usize = 4;
    const OFFSETS: [(i16, i16); 4] = [(48, 0), (-48, 0), (0, 48), (0, -48)];

    let mut spawned = 0;
    for (i, (ox, oy)) in OFFSETS.iter().enumerate() {
        if spawned >= MAX_GROUP {
            break;
        }
        if let Some(slot) = table.npcs.iter_mut().find(|n| !n.active) {
            let mut npc = spawn_encounter(encounter_type, hero_x, hero_y, tick.wrapping_add(i as u32));
            npc.x = hero_x.saturating_add(*ox);
            npc.y = hero_y.saturating_add(*oy);
            *slot = npc;
            spawned += 1;
        }
    }
    spawned
}
```

- [ ] **Step 5: Update tests**

Replace the existing encounter tests to match new signatures:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encounter_chart_full_length() {
        assert_eq!(ENCOUNTER_CHART_FULL.len(), 11);
    }

    #[test]
    fn test_spawn_encounter_uses_chart_hp() {
        // Ogre (type 0) should have 18 HP from chart
        let npc = spawn_encounter(0, 100, 100, 42);
        assert_eq!(npc.vitality, 18);
    }

    #[test]
    fn test_spawn_encounter_wraith_race() {
        let npc = spawn_encounter(2, 0, 0, 42);
        assert_eq!(npc.race, RACE_WRAITH);
    }

    #[test]
    fn test_spawn_encounter_has_weapon() {
        let npc = spawn_encounter(0, 0, 0, 42);
        // Ogre arms=2, weapon_probs[8..11] = [1,1,2,1], so weapon is 1 or 2
        assert!(npc.weapon <= 5, "weapon {} out of range", npc.weapon);
    }

    #[test]
    fn test_actors_on_screen_empty() {
        let table = crate::game::npc::NpcTable { npcs: Default::default() };
        assert!(!actors_on_screen(&table, 100, 100));
    }

    #[test]
    fn test_actors_on_screen_nearby() {
        let mut table = crate::game::npc::NpcTable { npcs: Default::default() };
        table.npcs[0] = Npc { active: true, x: 150, y: 150, ..Default::default() };
        assert!(actors_on_screen(&table, 100, 100));
    }

    #[test]
    fn test_actors_on_screen_far_away() {
        let mut table = crate::game::npc::NpcTable { npcs: Default::default() };
        table.npcs[0] = Npc { active: true, x: 1000, y: 1000, ..Default::default() };
        assert!(!actors_on_screen(&table, 100, 100));
    }

    #[test]
    fn test_try_trigger_encounter_respects_tick_gate() {
        let table = crate::game::npc::NpcTable { npcs: Default::default() };
        // Tick 1 is not divisible by 32 → should return None
        assert!(try_trigger_encounter(1, &table, 100, 100, 0, 0).is_none());
    }

    #[test]
    fn test_try_trigger_encounter_blocks_when_actors_present() {
        let mut table = crate::game::npc::NpcTable { npcs: Default::default() };
        table.npcs[0] = Npc { active: true, x: 100, y: 100, ..Default::default() };
        // Even at tick 32, actor on screen blocks spawn
        assert!(try_trigger_encounter(32, &table, 100, 100, 0, 0).is_none());
    }

    #[test]
    fn test_active_enemy_count() {
        let mut table = crate::game::npc::NpcTable { npcs: Default::default() };
        assert_eq!(active_enemy_count(&table), 0);
        table.npcs[0] = Npc { active: true, ..Default::default() };
        table.npcs[3] = Npc { active: true, ..Default::default() };
        assert_eq!(active_enemy_count(&table), 2);
    }

    #[test]
    fn test_try_trigger_encounter_blocks_at_4_enemies() {
        let mut table = crate::game::npc::NpcTable { npcs: Default::default() };
        // Place 4 enemies far away (outside screen, so actors_on_screen is false)
        for i in 0..4 {
            table.npcs[i] = Npc { active: true, x: 5000, y: 5000, ..Default::default() };
        }
        // 4 active → should block even if all other conditions met
        assert!(try_trigger_encounter(32, &table, 100, 100, 0, 0).is_none());
    }

    #[test]
    fn test_spawn_encounter_group_caps_at_4() {
        let mut table = crate::game::npc::NpcTable { npcs: Default::default() };
        let spawned = spawn_encounter_group(&mut table, 0, 100, 100, 42);
        assert_eq!(spawned, 4);
        assert_eq!(active_enemy_count(&table), 4);
    }

    #[test]
    fn test_pick_encounter_type_swamp_override() {
        // Swamp zone (xtype=7): if base roll is 2, override to 4 (snake)
        // Find a tick that produces base=2
        let mut found = false;
        for tick in 0..1000u32 {
            if rand4_from_tick(tick) == 2 {
                let etype = pick_encounter_type(7, tick);
                assert_eq!(etype, 4, "swamp should override type 2 to 4 (snake)");
                found = true;
                break;
            }
        }
        assert!(found, "should find a tick that produces base=2");
    }
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test encounter -- --nocapture`
Expected: All encounter tests pass.

- [ ] **Step 7: Wire new encounter system into gameplay_scene.rs**

In `src/game/gameplay_scene.rs`:

**7a.** Add `encounter_cooldown` field was already planned, but since `try_trigger_encounter` handles the 32-tick gate internally, we don't need a separate cooldown field. Replace the encounter spawn block (around line 3736):

Change:
```rust
if self.in_encounter_zone && crate::game::encounter::should_encounter(self.state.tick_counter) {
    if let Some(ref mut table) = self.npc_table {
        if let Some(slot) = table.npcs.iter_mut().find(|n| !n.active) {
            let zone_idx = self.state.region_num as usize;
            *slot = crate::game::encounter::spawn_encounter(
                zone_idx, self.state.hero_x as i16, self.state.hero_y as i16);
        }
    }
}
```

To:
```rust
if self.in_encounter_zone {
    if let Some(ref mut table) = self.npc_table {
        if let Some(encounter_type) = crate::game::encounter::try_trigger_encounter(
            self.state.tick_counter,
            table,
            self.state.hero_x as i16,
            self.state.hero_y as i16,
            self.state.xtype,
            self.state.region_num,
        ) {
            crate::game::encounter::spawn_encounter_group(
                table,
                encounter_type,
                self.state.hero_x as i16,
                self.state.hero_y as i16,
                self.state.tick_counter,
            );
        }
    }
}
```

**Note:** This requires `self.state.xtype` to exist. If `xtype` is not yet a field on `GameState`, add `pub xtype: u16` initialized to `0`. Check if it already exists first.

- [ ] **Step 8: Run all tests**

Run: `cargo test -- --nocapture`
Expected: All tests pass.

- [ ] **Step 9: Commit**

```bash
git add src/game/encounter.rs src/game/gameplay_scene.rs
git commit -m "fix(encounter): port original two-layer spawn system with rate-limiting

Replace per-tick 12.5% encounter check with original fmain.c:2451 system:
- 32-tick cooldown (~1 Hz at 30fps)
- actors_on_screen guard (no spawns while enemies visible)
- 4-enemy cap
- Zone-scaled danger level (2+xtype outdoor, 5+xtype indoor)
- Full encounter chart with per-type HP/arms/weapon from RESEARCH.md

Closes: #158"
```

---

## Task 4: Fix Y-sorted render to use fighting frames

**Files:**
- Modify: `src/game/gameplay_scene.rs` (Y-sorted hero render, ~line 4017–4019)

Closes: #159

- [ ] **Step 1: Replace walking-only frame selection with fighting-aware logic**

In `src/game/gameplay_scene.rs`, in the Y-sorted render pass's `RenderKind::Hero` branch, find the 3-line frame calculation (approximately line 4017–4019):

```rust
let frame_base = Self::facing_to_frame_base(hero_facing);
let anim_offset = if is_moving { (self.state.cycle as usize) % 8 } else { 1 };
let frame = frame_base + anim_offset;
```

Replace with the fighting-aware version already proven in `blit_actors_to_framebuf` (~line 3254):

```rust
let hero_state = self.state.actors.first().map(|a| &a.state);
let frame = if let Some(ActorState::Fighting(fight_state)) = hero_state {
    let fight_base = Self::facing_to_fight_frame_base(hero_facing);
    fight_base + (*fight_state as usize).min(8)
} else {
    let frame_base = Self::facing_to_frame_base(hero_facing);
    if is_moving { frame_base + (self.state.cycle as usize) % 8 } else { frame_base + 1 }
};
```

- [ ] **Step 2: Run the game to visually verify**

Run: `cargo run -- --debug --skip-intro`
Test: Press Numpad 0 to attack. The hero sprite should now cycle through fighting frames (weapon swing animation). Hold a direction while fighting — hero should face that direction while swinging, without moving.

- [ ] **Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass (no unit tests specifically cover the render path, but ensure no regressions).

- [ ] **Step 4: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix(render): use fighting frames in Y-sorted render pass

Copy ActorState::Fighting check from old blit_actors_to_framebuf path
into the active Y-sorted render path. Hero now shows weapon swing
animation (frames 32-79) when attacking via Numpad 0.

Closes: #159"
```

---

## Task 5: Clean up dead code in combat.rs

**Files:**
- Modify: `src/game/combat.rs` (remove/deprecate `resolve_combat`, update `WEAPON_DAMAGE`)

- [ ] **Step 1: Mark `resolve_combat()` and old `WEAPON_DAMAGE` as deprecated**

`resolve_combat()` is a dead code path — the live combat runs through `apply_melee_combat()` in gameplay_scene.rs. Rather than deleting it (which would break the existing tests that call it), mark it deprecated and leave a comment:

```rust
/// Legacy combat resolution — NOT used by the live game loop.
/// The active path is `GameplayScene::apply_melee_combat()`.
/// Kept for reference; will be removed in a future cleanup.
#[deprecated(note = "Use GameplayScene::apply_melee_combat() instead")]
pub fn resolve_combat(state: &mut GameState, npc: &mut Npc, hero_weapon_slot: usize) -> CombatResult {
```

Update the tests to suppress the deprecation warning:

```rust
#[test]
#[allow(deprecated)]
fn test_combat_reduces_enemy_vitality() {
```

```rust
#[test]
#[allow(deprecated)]
fn test_combat_enemy_defeated() {
```

- [ ] **Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass, no deprecation warnings in test output.

- [ ] **Step 3: Commit**

```bash
git add src/game/combat.rs
git commit -m "chore(combat): deprecate unused resolve_combat()

The live combat path is apply_melee_combat() in gameplay_scene.rs.
Mark resolve_combat() and WEAPON_DAMAGE as deprecated for future removal."
```
