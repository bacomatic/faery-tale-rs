# ECS Migration Plan E: Save/Load Proto v2

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the v1 proto schema and `persist.rs` with a v2 schema that reads/writes directly from the ECS `World` and `Resources`. V1 save files are explicitly rejected. The round-trip (save → load) must produce an identical world state.

**Architecture:** The proto schema is redesigned for component layout. `persist.rs` is rewritten to iterate ECS entities and resources rather than `GameState` fields. The `build.rs` protobuf code-generation step is unchanged. No migration shim — v1 files get a clear rejection message.

**Prerequisites:** Plans A–D complete. `EcsScene` is the active gameplay path.

**Tech Stack:** Rust 2021, `prost = "0.13"`, `prost-build = "0.13"`.

---

## File map

| File | Change |
|---|---|
| `proto/faery_save.proto` | **Rewrite** — v2 schema |
| `src/game/persist.rs` | **Rewrite** — serialize/deserialize ECS World + Resources |

---

## Task 1: Rewrite the proto schema

**Files:**
- Modify: `proto/faery_save.proto`

- [ ] **Step 1: Replace `proto/faery_save.proto` with v2 schema**

```protobuf
syntax = "proto3";
package faery;

// ── v2 schema — incompatible with v1. Version field checked on load. ─────────
// V1 saves are rejected with a user-facing error.

// ── Hero ─────────────────────────────────────────────────────────────────────

message HeroComponents {
    // Position (f32 stored as fixed-precision int: value × 1000, rounded)
    int32 x = 1;
    int32 y = 2;
    // Facing: Direction as u8 (NW=0, N=1, NE=2, E=3, SE=4, S=5, SW=6, W=7)
    uint32 facing = 3;
    // BrotherKind: 0=Julian, 1=Phillip, 2=Kevin
    uint32 brother_id = 4;

    // HeroStats
    int32  vitality = 10;
    int32  brave    = 11;
    int32  luck     = 12;
    int32  kind_stat = 13;   // 'kind' is a proto3 keyword; using kind_stat
    int32  wealth   = 14;
    int32  hunger   = 15;
    int32  fatigue  = 16;
    int32  gold     = 17;

    // Inventory: slots 0-34 (slot 35 is transient quiver, not saved)
    repeated uint32 stuff = 20;   // exactly 35 entries

    // CarrierMount
    int32  riding         = 30;
    int32  flying         = 31;
    int32  swan_vx        = 32;   // × 1000
    int32  swan_vy        = 33;   // × 1000
    int32  active_carrier = 34;
    bool   on_raft        = 35;
    int32  raftprox       = 36;
    uint32 wcarry         = 37;

    // SafePoint
    int32  safe_x      = 40;    // × 1000
    int32  safe_y      = 41;    // × 1000
    uint32 safe_region = 42;
}

// ── Bones (dead brother inventory) ───────────────────────────────────────────

message BonesEntity {
    int32  x          = 1;     // × 1000
    int32  y          = 2;     // × 1000
    uint32 brother_id = 3;
    uint32 region     = 4;
    repeated uint32 stuff = 5; // exactly 35 entries
}

// ── World object delta ────────────────────────────────────────────────────────
// Only objects whose ob_stat differs from the GameLibrary default are stored.

message WorldObjectDelta {
    uint32 index   = 1;   // index into GameLibrary.objects
    uint32 ob_stat = 2;   // current ob_stat (0=taken, 1=present, 5=hidden)
    bool   visible = 3;
}

// ── Inactive brother inventories ─────────────────────────────────────────────

message BrotherInventory {
    uint32 brother_id = 1;
    repeated uint32 stuff = 2; // exactly 35 entries
}

// ── Carrier entity (if active) ───────────────────────────────────────────────

message CarrierEntity {
    int32 x    = 1;  // × 1000
    int32 y    = 2;  // × 1000
    int32 kind = 3;
}

// ── Clock state ───────────────────────────────────────────────────────────────

message ClockState {
    uint32 daynight      = 1;
    uint32 game_days     = 2;
    uint32 tick_counter  = 3;
    int32  light_timer   = 4;
    int32  secret_timer  = 5;
    int32  freeze_timer  = 6;
    bool   light_sticky  = 7;
    bool   secret_sticky = 8;
    bool   freeze_sticky = 9;
}

// ── Region / quest / misc ─────────────────────────────────────────────────────

message RegionProgress {
    uint32 region_num    = 1;
    uint32 princess      = 2;
    uint32 dayperiod     = 3;
    uint32 current_mood  = 4;
    bool   cheat1        = 5;
    uint32 viewstatus    = 6;
    bool   witchflag     = 7;
    bool   safe_flag     = 8;

    // Inactive brother inventories (active brother's inventory is in HeroComponents)
    repeated BrotherInventory inactive_inventories = 10;

    // Carrier entity (absent if no carrier active)
    optional CarrierEntity carrier = 11;
}

// ── Top-level save file ───────────────────────────────────────────────────────

message SaveFile {
    // Must be 2 — any other value is rejected on load.
    uint32 save_version = 1;

    HeroComponents            hero         = 2;
    repeated BonesEntity      bones        = 3;
    repeated WorldObjectDelta world_objects = 4;
    ClockState                clock        = 5;
    RegionProgress            progress     = 6;
}
```

- [ ] **Step 2: Verify protobuf generation**

```bash
cd /home/ddehaven/projects/faery-tale-rs
cargo build 2>&1 | grep -E "^error|proto"
```
Expected: no errors. The generated Rust types will be in `OUT_DIR/faery.rs`.

- [ ] **Step 3: Commit proto schema**

```bash
git add proto/faery_save.proto
git commit -m "feat(save): v2 proto schema — component-oriented, drops v1 compatibility"
```

---

## Task 2: Rewrite `persist.rs`

**Files:**
- Modify: `src/game/persist.rs`

- [ ] **Step 1: Write failing round-trip test**

Add to `persist.rs` before implementing the new functions:

```rust
#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::spawn_hero;
    use crate::game::ecs::components::{HeroStats, Inventory, Position};
    use super::{save_to_bytes, load_from_bytes};

    fn make_world_and_resources() -> (World, Resources) {
        let mut world = World::new();
        let stats = HeroStats { vitality: 75, brave: 12, luck: 8, kind: 6,
                                wealth: 10, hunger: 3, fatigue: 1, gold: 50 };
        let mut inv = Inventory::empty();
        inv.stuff[0] = 2;
        inv.stuff[5] = 1;
        let hero = spawn_hero(&mut world, 1234.5, 5678.0, 0, stats, inv);
        let mut res = Resources::new(hero);
        res.clock.daynight = 12000;
        res.clock.tick_counter = 999;
        res.region.region_num = 3;
        (world, res)
    }

    #[test]
    fn round_trip_hero_position() {
        let (world, res) = make_world_and_resources();
        let bytes = save_to_bytes(&world, &res).expect("save should succeed");

        let mut world2 = World::new();
        let mut res2_placeholder = {
            let ph = world2.spawn((crate::game::ecs::components::Hero,));
            Resources::new(ph)
        };
        load_from_bytes(&bytes, &mut world2, &mut res2_placeholder).expect("load should succeed");

        let pos = world2.get::<&Position>(res2_placeholder.hero_entity).unwrap();
        // Position stored as ×1000 int — allow 0.001 tolerance
        assert!((pos.x - 1234.5).abs() < 0.002, "x mismatch: {}", pos.x);
        assert!((pos.y - 5678.0).abs() < 0.002, "y mismatch: {}", pos.y);
    }

    #[test]
    fn round_trip_hero_stats() {
        let (world, res) = make_world_and_resources();
        let bytes = save_to_bytes(&world, &res).unwrap();

        let mut world2 = World::new();
        let mut res2 = { let ph = world2.spawn((crate::game::ecs::components::Hero,)); Resources::new(ph) };
        load_from_bytes(&bytes, &mut world2, &mut res2).unwrap();

        let stats = world2.get::<&HeroStats>(res2.hero_entity).unwrap();
        assert_eq!(stats.vitality, 75);
        assert_eq!(stats.gold, 50);
    }

    #[test]
    fn round_trip_clock() {
        let (world, res) = make_world_and_resources();
        let bytes = save_to_bytes(&world, &res).unwrap();

        let mut world2 = World::new();
        let mut res2 = { let ph = world2.spawn((crate::game::ecs::components::Hero,)); Resources::new(ph) };
        load_from_bytes(&bytes, &mut world2, &mut res2).unwrap();

        assert_eq!(res2.clock.daynight, 12000);
        assert_eq!(res2.clock.tick_counter, 999);
    }

    #[test]
    fn round_trip_inventory() {
        let (world, res) = make_world_and_resources();
        let bytes = save_to_bytes(&world, &res).unwrap();

        let mut world2 = World::new();
        let mut res2 = { let ph = world2.spawn((crate::game::ecs::components::Hero,)); Resources::new(ph) };
        load_from_bytes(&bytes, &mut world2, &mut res2).unwrap();

        let inv = world2.get::<&Inventory>(res2.hero_entity).unwrap();
        assert_eq!(inv.stuff[0], 2);
        assert_eq!(inv.stuff[5], 1);
        assert_eq!(inv.stuff[10], 0);
    }

    #[test]
    fn v1_save_is_rejected() {
        // A save file with version=1 in the first proto field should be rejected.
        use prost::Message;
        let v1 = super::proto::SaveFile {
            save_version: 1,
            ..Default::default()
        };
        let mut buf = super::SAVE_MAGIC.to_vec();
        buf.extend_from_slice(&1u32.to_le_bytes()); // version in header
        let mut proto_bytes = Vec::new();
        v1.encode(&mut proto_bytes).unwrap();
        buf.extend_from_slice(&proto_bytes);

        let mut world = World::new();
        let ph = world.spawn((crate::game::ecs::components::Hero,));
        let mut res = Resources::new(ph);
        let result = load_from_bytes(&buf, &mut world, &mut res);
        assert!(result.is_err(), "v1 save should be rejected");
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cargo test persist 2>&1 | grep -E "error\[|FAILED|not found"
```
Expected: compile errors (`save_to_bytes`, `load_from_bytes` not yet defined).

- [ ] **Step 3: Implement the new `persist.rs`**

Replace the entire file:

```rust
//! Save/load game state — ECS v2 format.
//! Uses protobuf (prost). Save files begin with SAVE_MAGIC + version u32 LE.
//! V1 files (version != 2) are rejected with a clear error.
//!
//! See docs/spec/save-load.md for behavioral spec.
//! See docs/superpowers/plans/2025-01-01-ecs-plan-e-saveload.md for design.

use std::io::Write;
use std::path::Path;

use anyhow::{bail, Context};
use prost::Message;

use hecs::World;
use crate::game::ecs::resources::Resources;
use crate::game::ecs::components::*;
use crate::game::ecs::spawn::*;
use crate::game::direction::Direction;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/faery.rs"));
}

pub const SAVE_MAGIC: &[u8; 4] = b"FERY";
pub const SAVE_VERSION: u32 = 2;
pub const SAVE_DIR: &str = ".config/faery/saves";

// ── Position encoding ─────────────────────────────────────────────────────────
// f32 positions are stored as i32 × 1000 to avoid floating-point proto fields.

fn encode_pos(v: f32) -> i32 { (v * 1000.0).round() as i32 }
fn decode_pos(v: i32) -> f32 { v as f32 / 1000.0 }

fn encode_stuff(stuff: &[u8; 36]) -> Vec<u32> {
    stuff[0..35].iter().map(|&v| v as u32).collect()
}

fn decode_stuff(slots: &[u32]) -> [u8; 36] {
    let mut arr = [0u8; 36];
    for (i, &v) in slots.iter().take(35).enumerate() {
        arr[i] = v.min(255) as u8;
    }
    arr
}

// ── Serialization ─────────────────────────────────────────────────────────────

/// Serialize the current ECS world and resources into raw bytes (magic + version + proto).
pub fn save_to_bytes(world: &World, res: &Resources) -> anyhow::Result<Vec<u8>> {
    let save = world_to_proto(world, res)?;
    let mut buf = Vec::new();
    buf.extend_from_slice(SAVE_MAGIC);
    buf.extend_from_slice(&SAVE_VERSION.to_le_bytes());
    save.encode(&mut buf).context("proto encode failed")?;
    Ok(buf)
}

/// Save to a numbered slot file (~/.config/faery/saves/save{slot:02}.sav).
pub fn save(world: &World, res: &Resources, slot: u8) -> anyhow::Result<()> {
    let path = save_path(slot)?;
    std::fs::create_dir_all(path.parent().unwrap())?;
    let bytes = save_to_bytes(world, res)?;
    let mut f = std::fs::File::create(&path)?;
    f.write_all(&bytes)?;
    Ok(())
}

fn world_to_proto(world: &World, res: &Resources) -> anyhow::Result<proto::SaveFile> {
    // Hero components.
    let hero = {
        let pos = world.get::<&Position>(res.hero_entity)
            .context("hero has no Position")?;
        let facing = world.get::<&Facing>(res.hero_entity)
            .map(|f| f.dir as u8 as u32)
            .unwrap_or(Direction::N as u8 as u32);
        let bk = world.get::<&BrotherKind>(res.hero_entity)
            .map(|b| b.id as u32)
            .unwrap_or(0);
        let stats = world.get::<&HeroStats>(res.hero_entity)
            .context("hero has no HeroStats")?;
        let inv = world.get::<&Inventory>(res.hero_entity)
            .context("hero has no Inventory")?;
        let cm = world.get::<&CarrierMount>(res.hero_entity)
            .map(|c| *c)
            .unwrap_or_default();
        let sp = world.get::<&SafePoint>(res.hero_entity).ok();

        proto::HeroComponents {
            x: encode_pos(pos.x),
            y: encode_pos(pos.y),
            facing,
            brother_id: bk,
            vitality: stats.vitality as i32,
            brave:    stats.brave    as i32,
            luck:     stats.luck     as i32,
            kind_stat: stats.kind    as i32,
            wealth:   stats.wealth   as i32,
            hunger:   stats.hunger   as i32,
            fatigue:  stats.fatigue  as i32,
            gold:     stats.gold,
            stuff:    encode_stuff(&inv.stuff),
            riding:   cm.riding as i32,
            flying:   cm.flying as i32,
            swan_vx:  encode_pos(cm.swan_vx),
            swan_vy:  encode_pos(cm.swan_vy),
            active_carrier: cm.active_carrier as i32,
            on_raft:  cm.on_raft,
            raftprox: cm.raftprox as i32,
            wcarry:   cm.wcarry as u32,
            safe_x:   sp.map(|s| encode_pos(s.x)).unwrap_or(0),
            safe_y:   sp.map(|s| encode_pos(s.y)).unwrap_or(0),
            safe_region: sp.map(|s| s.region as u32).unwrap_or(0),
        }
    };

    // Bones entities.
    let bones: Vec<proto::BonesEntity> = world
        .query::<(&Position, &BrotherKind, &Inventory)>()
        .with::<&Bones>()
        .iter()
        .map(|(_, (pos, bk, inv))| proto::BonesEntity {
            x:         encode_pos(pos.x),
            y:         encode_pos(pos.y),
            brother_id: bk.id as u32,
            region:    res.region.region_num as u32,
            stuff:     encode_stuff(&inv.stuff),
        })
        .collect();

    // World object deltas (only items that differ from GameLibrary defaults).
    // In v2 we serialize all visible/taken ground items to keep it simple.
    let world_objects: Vec<proto::WorldObjectDelta> = world
        .query::<(&Position, &WorldObj)>()
        .iter()
        .enumerate()
        .map(|(idx, (_, (_, obj)))| proto::WorldObjectDelta {
            index:   idx as u32,
            ob_stat: obj.ob_stat as u32,
            visible: obj.visible,
        })
        .collect();

    // Clock state.
    let clock = proto::ClockState {
        daynight:      res.clock.daynight as u32,
        game_days:     res.clock.game_days,
        tick_counter:  res.clock.tick_counter,
        light_timer:   res.clock.light_timer as i32,
        secret_timer:  res.clock.secret_timer as i32,
        freeze_timer:  res.clock.freeze_timer as i32,
        light_sticky:  res.clock.light_sticky,
        secret_sticky: res.clock.secret_sticky,
        freeze_sticky: res.clock.freeze_sticky,
    };

    // Carrier entity.
    let carrier_proto = res.carrier_entity.and_then(|ce| {
        world.get::<&Position>(ce).ok().zip(
            world.get::<&CarrierKind>(ce).ok()
        ).map(|(pos, ck)| proto::CarrierEntity {
            x:    encode_pos(pos.x),
            y:    encode_pos(pos.y),
            kind: ck.kind as i32,
        })
    });

    // Inactive brother inventories.
    let inactive: Vec<proto::BrotherInventory> = (0u8..3)
        .filter(|&id| id != res.brother.active_brother as u8)
        .map(|id| proto::BrotherInventory {
            brother_id: id as u32,
            stuff:      encode_stuff(&res.brother.inactive_inventories[id as usize]),
        })
        .collect();

    // Region / quest progress.
    let progress = proto::RegionProgress {
        region_num:   res.region.region_num as u32,
        princess:     res.region.princess as u32,
        dayperiod:    res.region.dayperiod as u32,
        current_mood: res.region.current_mood as u32,
        cheat1:       res.brother.cheat1,
        viewstatus:   res.view.viewstatus as u32,
        witchflag:    res.brother.witchflag,
        safe_flag:    res.brother.safe_flag,
        inactive_inventories: inactive,
        carrier:      carrier_proto,
    };

    Ok(proto::SaveFile {
        save_version: SAVE_VERSION,
        hero:         Some(hero),
        bones,
        world_objects,
        clock:        Some(clock),
        progress:     Some(progress),
    })
}

// ── Deserialization ───────────────────────────────────────────────────────────

/// Deserialize from raw bytes into an existing (cleared) World and Resources.
/// Clears the world before loading.
pub fn load_from_bytes(
    bytes: &[u8],
    world: &mut World,
    res: &mut Resources,
) -> anyhow::Result<()> {
    if bytes.len() < 8 {
        bail!("Save file too short");
    }
    if &bytes[0..4] != SAVE_MAGIC {
        bail!("Not a faery save file");
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != SAVE_VERSION {
        bail!(
            "Unsupported save file version {}. This game requires version {}. \
             Please start a new game.",
            version, SAVE_VERSION
        );
    }

    let save = proto::SaveFile::decode(&bytes[8..]).context("proto decode failed")?;
    proto_to_world(save, world, res)
}

/// Load from a numbered slot file.
pub fn load(slot: u8, world: &mut World, res: &mut Resources) -> anyhow::Result<()> {
    let path = save_path(slot)?;
    let bytes = std::fs::read(&path)
        .with_context(|| format!("cannot read save file {:?}", path))?;
    load_from_bytes(&bytes, world, res)
}

fn proto_to_world(
    save: proto::SaveFile,
    world: &mut World,
    res: &mut Resources,
) -> anyhow::Result<()> {
    // Clear existing entities.
    let entities: Vec<_> = world.iter().map(|(e, _)| e).collect();
    for e in entities { world.despawn(e).ok(); }

    let h = save.hero.context("missing hero in save")?;
    let clock = save.clock.context("missing clock in save")?;
    let progress = save.progress.context("missing progress in save")?;

    // Restore clock.
    res.clock.daynight      = h.vitality as u16; // temporary — overwritten below
    res.clock.daynight      = clock.daynight as u16;
    res.clock.game_days     = clock.game_days;
    res.clock.tick_counter  = clock.tick_counter;
    res.clock.light_timer   = clock.light_timer as i16;
    res.clock.secret_timer  = clock.secret_timer as i16;
    res.clock.freeze_timer  = clock.freeze_timer as i16;
    res.clock.light_sticky  = clock.light_sticky;
    res.clock.secret_sticky = clock.secret_sticky;
    res.clock.freeze_sticky = clock.freeze_sticky;

    // Restore region / quest.
    res.region.region_num  = progress.region_num as u8;
    res.region.new_region  = progress.region_num as u8;
    res.region.princess    = progress.princess as u8;
    res.region.dayperiod   = progress.dayperiod as u8;
    res.region.current_mood = progress.current_mood as u8;
    res.brother.cheat1     = progress.cheat1;
    res.view.viewstatus    = 99; // force full redraw
    res.brother.witchflag  = progress.witchflag;
    res.brother.safe_flag  = progress.safe_flag;

    // Inactive brother inventories.
    for inv in &progress.inactive_inventories {
        let id = inv.brother_id as usize;
        if id < 3 {
            res.brother.inactive_inventories[id] = decode_stuff(&inv.stuff);
        }
    }

    // Spawn hero.
    let stats = HeroStats {
        vitality: h.vitality as i16,
        brave:    h.brave    as i16,
        luck:     h.luck     as i16,
        kind:     h.kind_stat as i16,
        wealth:   h.wealth   as i16,
        hunger:   h.hunger   as i16,
        fatigue:  h.fatigue  as i16,
        gold:     h.gold,
    };
    let mut inv = Inventory::empty();
    inv.stuff = decode_stuff(&h.stuff);
    let hero = spawn_hero(
        world,
        decode_pos(h.x),
        decode_pos(h.y),
        h.brother_id as u8,
        stats,
        inv,
    );
    res.hero_entity = hero;
    res.brother.active_brother = h.brother_id as usize;

    // Restore facing.
    if let Ok(mut facing) = world.get_mut::<Facing>(hero) {
        facing.dir = Direction::from(h.facing as u8);
    }

    // Restore carrier mount.
    if let Ok(mut cm) = world.get_mut::<CarrierMount>(hero) {
        cm.riding         = h.riding as i16;
        cm.flying         = h.flying as i16;
        cm.swan_vx        = decode_pos(h.swan_vx);
        cm.swan_vy        = decode_pos(h.swan_vy);
        cm.active_carrier = h.active_carrier as i16;
        cm.on_raft        = h.on_raft;
        cm.raftprox       = h.raftprox as i16;
        cm.wcarry         = h.wcarry as u8;
    }

    // Restore safe point.
    if h.safe_region != 0 || h.safe_x != 0 || h.safe_y != 0 {
        world.insert_one(hero, SafePoint {
            x:      decode_pos(h.safe_x),
            y:      decode_pos(h.safe_y),
            region: h.safe_region as u8,
        }).ok();
    }

    // Spawn Bones entities.
    for b in save.bones {
        spawn_bones(
            world,
            decode_pos(b.x),
            decode_pos(b.y),
            b.region as u8,
            b.brother_id as u8,
            decode_stuff(&b.stuff),
        );
    }

    // Carrier entity.
    if let Some(c) = progress.carrier {
        let ce = spawn_carrier(world, decode_pos(c.x), decode_pos(c.y), c.kind as i16, 0);
        res.carrier_entity = Some(ce);
    }

    // Note: world objects (GroundItem / SetFig) are NOT spawned here.
    // They are spawned by RegionSystem when the region loads.
    // WorldObjectDelta patches are applied by RegionSystem after spawn.
    // Store the deltas for RegionSystem to consume.
    res.pending_world_object_deltas = save.world_objects
        .into_iter()
        .map(|d| crate::game::ecs::resources::WorldObjectDelta {
            index:   d.index as usize,
            ob_stat: d.ob_stat as u8,
            visible: d.visible,
        })
        .collect();

    Ok(())
}

fn save_path(slot: u8) -> anyhow::Result<std::path::PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(Path::new(&home)
        .join(SAVE_DIR)
        .join(format!("save{:02}.sav", slot)))
}
```

- [ ] **Step 4: Add `pending_world_object_deltas` and `WorldObjectDelta` to `Resources`**

In `src/game/ecs/resources.rs`, add:

```rust
/// Pending world object state patches from a loaded save.
/// Consumed by RegionSystem after ground items are spawned.
#[derive(Debug, Clone)]
pub struct WorldObjectDelta {
    pub index:   usize,
    pub ob_stat: u8,
    pub visible: bool,
}
```

And to the `Resources` struct:
```rust
pub pending_world_object_deltas: Vec<WorldObjectDelta>,
```

Initialize it in `Resources::new()`:
```rust
pending_world_object_deltas: Vec::new(),
```

- [ ] **Step 5: Run tests**

```bash
cargo test persist 2>&1 | grep -E "test.*ok|FAILED|^error"
```
Expected: all 5 round-trip tests pass.

- [ ] **Step 6: Run full suite**

```bash
cargo test 2>&1 | grep "^test result"
```

- [ ] **Step 7: Commit**

```bash
git add proto/faery_save.proto src/game/persist.rs src/game/ecs/resources.rs
git commit -m "feat(save): rewrite persist.rs for ECS v2 proto schema with round-trip tests"
```

---

## Completion check

```bash
cargo test persist 2>&1 | grep -E "test.*ok|FAILED"
cargo test 2>&1 | grep "^test result"
```

5 round-trip tests pass. V1 rejection test passes. Full suite green.
