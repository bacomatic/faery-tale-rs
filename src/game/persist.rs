//! Save/load game state using protobuf (prost).

use std::io::Write;
use std::path::Path;

use anyhow::Context;
use prost::Message;

use crate::game::actor::{Actor, ActorKind, ActorState};
use crate::game::game_state::{GameState, WorldObject};

/// Generated protobuf types for the save format.
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/faery.rs"));
}

pub const SAVE_MAGIC: &[u8; 4] = b"FERY";
pub const SAVE_VERSION: u32 = 1;
pub const SAVE_DIR: &str = ".config/faery/saves";

fn state_to_proto(state: &GameState) -> proto::SaveFile {
    let make_stuff = |arr: &[u8; 36]| proto::BrotherStuff {
        // ARROWBASE = 35: only save indices 0-34; slot 35 is a transient quiver
        // accumulator cleared at the start of every loot-pickup (fmain.c:3151).
        slots: arr[0..35].iter().map(|&v| v as u32).collect(),
    };

    let world_objects = state
        .world_objects
        .iter()
        .map(|wo| proto::SavedWorldObject {
            ob_id: wo.ob_id as u32,
            ob_stat: wo.ob_stat as u32,
            region: wo.region as u32,
            x: wo.x as u32,
            y: wo.y as u32,
            visible: wo.visible,
        })
        .collect();

    proto::SaveFile {
        save_version: SAVE_VERSION,

        hero_x: state.hero_x as u32,
        hero_y: state.hero_y as u32,
        hero_sector: state.hero_sector as u32,
        hero_place: state.hero_place as u32,

        vitality: state.vitality as i32,
        brave: state.brave as i32,
        luck: state.luck as i32,
        kind: state.kind as i32,
        wealth: state.wealth as i32,
        hunger: state.hunger as i32,
        fatigue: state.fatigue as i32,
        brother: state.brother as u32,
        riding: state.riding as i32,
        flying: state.flying as i32,
        swan_vx: state.swan_vx as i32,
        swan_vy: state.swan_vy as i32,

        light_timer: state.light_timer as i32,
        secret_timer: state.secret_timer as i32,
        freeze_timer: state.freeze_timer as i32,

        daynight: state.daynight as u32,
        lightlevel: state.lightlevel as u32,
        cycle: state.cycle,
        flasher: state.flasher,

        battleflag: state.battleflag,
        witchflag: state.witchflag,
        safe_flag: state.safe_flag,

        viewstatus: state.viewstatus as u32,
        cmode: state.cmode as u32,

        safe_x: state.safe_x as u32,
        safe_y: state.safe_y as u32,
        safe_r: state.safe_r as u32,

        region_num: state.region_num as u32,
        new_region: state.new_region as u32,

        julstuff: Some(make_stuff(&state.julstuff)),
        philstuff: Some(make_stuff(&state.philstuff)),
        kevstuff: Some(make_stuff(&state.kevstuff)),
        active_brother: state.active_brother as u32,

        xtype: state.xtype as u32,
        encounter_type: state.encounter_type as u32,
        encounter_number: state.encounter_number as u32,

        active_carrier: state.active_carrier as i32,
        actor_file: state.actor_file as i32,
        set_file: state.set_file as i32,
        wcarry: state.wcarry as u32,

        princess: state.princess as u32,
        dayperiod: state.dayperiod as u32,

        current_mood: state.current_mood as u32,

        actors: vec![],
        world_objects,
        cheat1: state.cheat1,
        // Quest state (Plan V) - legacy GameState doesn't track these, use defaults
        statues_collected: 0,
        writ_obtained: false,
        rose_obtained: false,
        crystal_shard_obtained: false,
        sun_stone_obtained: false,
        golden_lasso_obtained: false,
        talisman_obtained: false,
        king_bone_obtained: false,
    }
}

/// Write a save file to an explicit path. Exposed for testing.
pub fn save_to_path(state: &GameState, path: &Path) -> anyhow::Result<()> {
    let save = state_to_proto(state);
    let encoded = save.encode_to_vec();

    let mut f = std::fs::File::create(path)
        .with_context(|| format!("creating save file {}", path.display()))?;
    f.write_all(SAVE_MAGIC)?;
    f.write_all(&SAVE_VERSION.to_le_bytes())?;
    f.write_all(&encoded)?;
    Ok(())
}

/// Save `state` into slot `slot` under `~/.config/faery/saves/save{slot:02}.sav`.
pub fn save_game(state: &GameState, slot: u8) -> anyhow::Result<()> {
    let base = dirs::config_dir()
        .context("could not determine config directory")?
        .join("faery")
        .join("saves");
    std::fs::create_dir_all(&base)
        .with_context(|| format!("creating save directory {}", base.display()))?;
    let path = base.join(format!("save{slot:02}.sav"));
    save_to_path(state, &path)
}

/// Load a save file from an explicit path. Exposed for testing.
pub fn load_from_path(path: &std::path::Path) -> anyhow::Result<GameState> {
    let data = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("failed to read save file {}: {}", path.display(), e))?;

    if data.len() < 8 {
        anyhow::bail!("invalid save file: too short");
    }
    if &data[0..4] != SAVE_MAGIC.as_ref() {
        anyhow::bail!("invalid save file: bad magic");
    }
    let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if version != SAVE_VERSION {
        anyhow::bail!(
            "invalid save file: version mismatch (got {}, expected {})",
            version,
            SAVE_VERSION
        );
    }

    let sf = proto::SaveFile::decode(&data[8..])
        .map_err(|e| anyhow::anyhow!("failed to decode save file: {}", e))?;

    let mut state = GameState::new();

    state.hero_x = sf.hero_x as u16;
    state.hero_y = sf.hero_y as u16;
    state.hero_sector = sf.hero_sector as u16;
    state.hero_place = sf.hero_place as u16;

    state.vitality = sf.vitality as i16;
    state.brave = sf.brave as i16;
    state.luck = sf.luck as i16;
    state.kind = sf.kind as i16;
    state.wealth = sf.wealth as i16;
    state.hunger = sf.hunger as i16;
    state.fatigue = sf.fatigue as i16;
    state.brother = sf.brother as u8;
    state.riding = sf.riding as i16;
    state.flying = sf.flying as i16;
    state.swan_vx = sf.swan_vx as i16;
    state.swan_vy = sf.swan_vy as i16;

    state.light_timer = sf.light_timer as i16;
    state.secret_timer = sf.secret_timer as i16;
    state.freeze_timer = sf.freeze_timer as i16;

    state.daynight = sf.daynight as u16;
    state.lightlevel = sf.lightlevel as u16;
    state.cycle = sf.cycle;
    state.flasher = sf.flasher;

    state.battleflag = sf.battleflag;
    state.witchflag = sf.witchflag;
    state.safe_flag = sf.safe_flag;

    state.viewstatus = sf.viewstatus as u8;
    state.cmode = sf.cmode as u8;

    state.safe_x = sf.safe_x as u16;
    state.safe_y = sf.safe_y as u16;
    state.safe_r = sf.safe_r as u8;

    state.region_num = sf.region_num as u8;
    state.new_region = sf.new_region as u8;

    if let Some(j) = sf.julstuff {
        for (i, s) in j.slots.iter().take(35).enumerate() {
            state.julstuff[i] = *s as u8;
        }
    }
    if let Some(p) = sf.philstuff {
        for (i, s) in p.slots.iter().take(35).enumerate() {
            state.philstuff[i] = *s as u8;
        }
    }
    if let Some(k) = sf.kevstuff {
        for (i, s) in k.slots.iter().take(35).enumerate() {
            state.kevstuff[i] = *s as u8;
        }
    }
    state.active_brother = sf.active_brother as usize;

    state.xtype = sf.xtype as u16;
    state.encounter_type = sf.encounter_type as u16;
    state.encounter_number = sf.encounter_number as u8;

    state.active_carrier = sf.active_carrier as i16;
    state.actor_file = sf.actor_file as i16;
    state.set_file = sf.set_file as i16;
    state.wcarry = sf.wcarry as u8;

    state.princess = sf.princess as u8;
    state.dayperiod = sf.dayperiod as u8;

    state.current_mood = sf.current_mood as u8;

    // cheat1 persists across save/load (fmain.c:562, save-load.md §"Cheats persist")
    state.cheat1 = sf.cheat1;

    if !sf.actors.is_empty() {
        state.actors.clear();
        for sa in &sf.actors {
            let kind = match sa.kind {
                0 => ActorKind::Player,
                1 => ActorKind::Enemy,
                2 => ActorKind::Object,
                3 => ActorKind::Raft,
                4 => ActorKind::SetFig,
                5 => ActorKind::Carrier,
                _ => ActorKind::Dragon,
            };
            let actor_state = match sa.state {
                0 => ActorState::Still,
                1 => ActorState::Walking,
                2 => ActorState::Fighting(0),
                3 => ActorState::Dying,
                4 => ActorState::Dead,
                5 => ActorState::Shooting(0),
                6 => ActorState::Sinking,
                7 => ActorState::Falling,
                _ => ActorState::Sleeping,
            };
            state.actors.push(Actor {
                abs_x: sa.abs_x as u16,
                abs_y: sa.abs_y as u16,
                kind,
                race: sa.race as u8,
                state: actor_state,
                vitality: sa.vitality as i16,
                weapon: sa.weapon as u8,
                facing: crate::game::direction::Direction::from(sa.facing as u8),
                ..Actor::default()
            });
        }
    }

    // Restore per-region object tables (SPEC §24.2)
    state.world_objects.clear();
    for wo in &sf.world_objects {
        state.world_objects.push(WorldObject {
            ob_id: wo.ob_id as u8,
            ob_stat: wo.ob_stat as u8,
            region: wo.region as u8,
            x: wo.x as u16,
            y: wo.y as u16,
            visible: wo.visible,
            goal: 0, // recomputed when region is reloaded from game library
        });
    }

    // Post-load cleanup (SPEC §24.5)
    state.encounter_number = 0;
    state.actors_loading = false;
    state.encounter_type = 0;
    state.viewstatus = 99; // Force full redraw

    Ok(state)
}

/// Load `GameState` from slot `slot` under the platform config dir.
pub fn load_game(slot: u8) -> anyhow::Result<GameState> {
    let base = dirs::config_dir()
        .context("could not determine config directory")?
        .join("faery")
        .join("saves");
    let path = base.join(format!("save{slot:02}.sav"));
    load_from_path(&path)
}

// --------------------------------------------------------------------------
// ECS save/load (Plan E): serialize directly from EcsScene World + Resources
// --------------------------------------------------------------------------

/// Serialize `EcsScene` to a `proto::SaveFile`.
fn ecs_to_proto(scene: &crate::game::ecs::scene::EcsScene) -> proto::SaveFile {
    use crate::game::ecs::components::{
        BrotherKind, CarrierMount, HeroStats, Inventory, Position, SafePoint,
    };
    let w = &scene.world;
    let res = &scene.res;
    let e = res.hero_entity;

    let make_stuff = |arr: &[u8; 36]| proto::BrotherStuff {
        slots: arr[0..35].iter().map(|&v| v as u32).collect(),
    };

    // Hero components.
    let (hero_x, hero_y) = w.get::<&Position>(e)
        .map(|p| (p.x as u32, p.y as u32)).unwrap_or((0, 0));
    let brother_id = w.get::<&BrotherKind>(e).map(|b| b.id as u32).unwrap_or(0);
    let (vit, brave, luck, kind, wealth, hunger, fatigue) =
        w.get::<&HeroStats>(e).map(|s| (
            s.vitality as i32, s.brave as i32, s.luck as i32, s.kind as i32,
            s.wealth as i32, s.hunger as i32, s.fatigue as i32,
        )).unwrap_or_default();
    let (riding, flying, swan_vx, swan_vy, active_carrier, wcarry) =
        w.get::<&CarrierMount>(e).map(|c| (
            c.riding as i32, c.flying as i32,
            (c.swan_vx * 1000.0) as i32, (c.swan_vy * 1000.0) as i32,
            c.active_carrier as i32, c.wcarry as u32,
        )).unwrap_or_default();
    let (safe_x, safe_y, safe_r) = w.get::<&SafePoint>(e)
        .map(|s| (s.x as u32, s.y as u32, s.region as u32)).unwrap_or_default();

    // Hero inventory.
    let hero_inv = w.get::<&Inventory>(e)
        .map(|inv| inv.stuff).unwrap_or([0u8; 36]);

    // Inactive brother inventories live in res.brother.inactive_inventories.
    let invs = &res.brother.inactive_inventories;
    let julstuff  = make_stuff(&invs[0]);
    let philstuff = make_stuff(&invs[1]);
    let kevstuff  = make_stuff(&invs[2]);
    // Overwrite the active brother's slot with the live inventory.
    let active = res.brother.active_brother;
    let (julstuff, philstuff, kevstuff) = match active {
        0 => (make_stuff(&hero_inv), philstuff, kevstuff),
        1 => (julstuff, make_stuff(&hero_inv), kevstuff),
        _ => (julstuff, philstuff, make_stuff(&hero_inv)),
    };

    proto::SaveFile {
        save_version: SAVE_VERSION,
        hero_x,
        hero_y,
        hero_sector: 0,
        hero_place:  0,
        vitality:    vit,
        brave,
        luck,
        kind,
        wealth,
        hunger,
        fatigue,
        brother:     brother_id,
        riding,
        flying,
        swan_vx,
        swan_vy,
        light_timer:  res.clock.light_timer as i32,
        secret_timer: res.clock.secret_timer as i32,
        freeze_timer: res.clock.freeze_timer as i32,
        daynight:     res.clock.daynight as u32,
        lightlevel:   res.clock.lightlevel as u32,
        cycle:        res.clock.cycle,
        flasher:      res.clock.flasher,
        battleflag:   res.region.battleflag,
        witchflag:    res.brother.witchflag,
        safe_flag:    res.brother.safe_flag,
        viewstatus:   res.view.viewstatus as u32,
        cmode:        res.view.cmode as u32,
        safe_x,
        safe_y,
        safe_r,
        region_num:       res.region.region_num as u32,
        new_region:       res.region.new_region as u32,
        julstuff:         Some(julstuff),
        philstuff:        Some(philstuff),
        kevstuff:         Some(kevstuff),
        active_brother:   active as u32,
        xtype:            res.region.xtype as u32,
        encounter_type:   res.region.encounter_type as u32,
        encounter_number: res.region.encounter_number as u32,
        active_carrier:   active_carrier,
        actor_file:       res.region.actor_file as i32,
        set_file:         res.region.set_file as i32,
        wcarry,
        princess:         res.region.princess as u32,
        dayperiod:        res.region.dayperiod as u32,
        current_mood:     res.region.current_mood as u32,
        cheat1:           res.brother.cheat1,
        // Quest state (Plan V)
        statues_collected: res.quest.statues_collected as u32,
        writ_obtained:     res.quest.writ_obtained,
        rose_obtained:     res.quest.rose_obtained,
        crystal_shard_obtained: res.quest.crystal_shard_obtained,
        sun_stone_obtained:     res.quest.sun_stone_obtained,
        golden_lasso_obtained:  res.quest.golden_lasso_obtained,
        talisman_obtained:      res.quest.talisman_obtained,
        king_bone_obtained:     res.quest.king_bone_obtained,
        actors:           vec![],
        world_objects:    vec![],
    }
}

/// Apply a decoded `proto::SaveFile` back into an `EcsScene`.
fn proto_to_ecs(
    sf: proto::SaveFile,
    scene: &mut crate::game::ecs::scene::EcsScene,
) {
    use crate::game::ecs::components::{
        BrotherKind, CarrierMount, HeroStats, Inventory, Position, SafePoint,
    };
    let e = scene.res.hero_entity;

    // Position.
    if let Ok(mut p) = scene.world.get::<&mut Position>(e) {
        p.x = sf.hero_x as f32;
        p.y = sf.hero_y as f32;
    }
    // HeroStats.
    if let Ok(mut s) = scene.world.get::<&mut HeroStats>(e) {
        s.vitality = sf.vitality as i16;
        s.brave    = sf.brave    as i16;
        s.luck     = sf.luck     as i16;
        s.kind     = sf.kind     as i16;
        s.wealth   = sf.wealth   as i16;
        s.hunger   = sf.hunger   as i16;
        s.fatigue  = sf.fatigue  as i16;
    }
    // BrotherKind.
    if let Ok(mut b) = scene.world.get::<&mut BrotherKind>(e) {
        b.id = sf.brother as u8;
    }
    // CarrierMount.
    if let Ok(mut c) = scene.world.get::<&mut CarrierMount>(e) {
        c.riding         = sf.riding as i16;
        c.flying         = sf.flying as i16;
        c.swan_vx        = sf.swan_vx as f32 / 1000.0;
        c.swan_vy        = sf.swan_vy as f32 / 1000.0;
        c.active_carrier = sf.active_carrier as i16;
        c.wcarry         = sf.wcarry as u8;
    }
    // SafePoint.
    let new_sp = SafePoint { x: sf.safe_x as f32, y: sf.safe_y as f32, region: sf.safe_r as u8 };
    let has_sp = scene.world.get::<&SafePoint>(e).is_ok();
    if has_sp {
        if let Ok(mut sp) = scene.world.get::<&mut SafePoint>(e) {
            *sp = new_sp;
        }
    } else {
        scene.world.insert_one(e, new_sp).ok();
    }
    // Inventory — active brother.
    let active = sf.active_brother as usize;
    let src_stuff = match active {
        0 => sf.julstuff.as_ref(),
        1 => sf.philstuff.as_ref(),
        _ => sf.kevstuff.as_ref(),
    };
    if let Some(stuff) = src_stuff {
        if let Ok(mut inv) = scene.world.get::<&mut Inventory>(e) {
            for (i, &v) in stuff.slots.iter().take(35).enumerate() {
                inv.stuff[i] = v as u8;
            }
        }
    }
    // Inactive inventories.
    let restore = |arr: &mut [u8; 36], msg: Option<&proto::BrotherStuff>| {
        if let Some(s) = msg {
            for (i, &v) in s.slots.iter().take(35).enumerate() {
                arr[i] = v as u8;
            }
        }
    };
    restore(&mut scene.res.brother.inactive_inventories[0], sf.julstuff.as_ref());
    restore(&mut scene.res.brother.inactive_inventories[1], sf.philstuff.as_ref());
    restore(&mut scene.res.brother.inactive_inventories[2], sf.kevstuff.as_ref());
    // Overwrite the active brother's slot with live inventory (already set above).

    // Clock.
    scene.res.clock.light_timer  = sf.light_timer  as i16;
    scene.res.clock.secret_timer = sf.secret_timer as i16;
    scene.res.clock.freeze_timer = sf.freeze_timer as i16;
    scene.res.clock.daynight     = sf.daynight     as u16;
    scene.res.clock.lightlevel   = sf.lightlevel   as u16;
    scene.res.clock.cycle        = sf.cycle;
    scene.res.clock.flasher      = sf.flasher;

    // Region.
    scene.res.region.battleflag      = sf.battleflag;
    scene.res.region.region_num      = sf.region_num      as u8;
    scene.res.region.new_region      = sf.new_region      as u8;
    scene.res.region.xtype           = sf.xtype           as u16;
    scene.res.region.encounter_type  = sf.encounter_type  as u16;
    scene.res.region.encounter_number = sf.encounter_number as u8;
    scene.res.region.actor_file      = sf.actor_file      as i16;
    scene.res.region.set_file        = sf.set_file        as i16;
    scene.res.region.princess        = sf.princess        as u8;
    scene.res.region.dayperiod       = sf.dayperiod       as u8;
    scene.res.region.current_mood    = sf.current_mood    as u8;

    // Brother roster.
    scene.res.brother.brother        = sf.brother         as u8;
    scene.res.brother.active_brother = active;
    scene.res.brother.witchflag      = sf.witchflag;
    scene.res.brother.safe_flag      = sf.safe_flag;
    scene.res.brother.cheat1         = sf.cheat1;

    // Quest state (Plan V).
    scene.res.quest.statues_collected      = sf.statues_collected as u8;
    scene.res.quest.writ_obtained          = sf.writ_obtained;
    scene.res.quest.rose_obtained          = sf.rose_obtained;
    scene.res.quest.crystal_shard_obtained = sf.crystal_shard_obtained;
    scene.res.quest.sun_stone_obtained     = sf.sun_stone_obtained;
    scene.res.quest.golden_lasso_obtained  = sf.golden_lasso_obtained;
    scene.res.quest.talisman_obtained      = sf.talisman_obtained;
    scene.res.quest.king_bone_obtained     = sf.king_bone_obtained;

    // View.
    scene.res.view.viewstatus = sf.viewstatus as u8;
    scene.res.view.cmode      = sf.cmode      as u8;

    // Post-load cleanup (SPEC §24.5).
    scene.res.region.encounter_number = 0;
    scene.res.region.encounter_type   = 0;
    scene.res.view.viewstatus         = 99; // force full redraw
}

/// Save an `EcsScene` to an explicit path.  Exposed for testing.
pub fn ecs_save_to_path(
    scene: &crate::game::ecs::scene::EcsScene,
    path: &Path,
) -> anyhow::Result<()> {
    let save = ecs_to_proto(scene);
    let encoded = save.encode_to_vec();
    let mut f = std::fs::File::create(path)
        .with_context(|| format!("creating save file {}", path.display()))?;
    f.write_all(SAVE_MAGIC)?;
    f.write_all(&SAVE_VERSION.to_le_bytes())?;
    f.write_all(&encoded)?;
    Ok(())
}

/// Save `EcsScene` into slot `slot` under `~/.config/faery/saves/save{slot:02}.sav`.
pub fn ecs_save_game(
    scene: &crate::game::ecs::scene::EcsScene,
    slot: u8,
) -> anyhow::Result<()> {
    let base = dirs::config_dir()
        .context("could not determine config directory")?
        .join("faery")
        .join("saves");
    std::fs::create_dir_all(&base)
        .with_context(|| format!("creating save directory {}", base.display()))?;
    let path = base.join(format!("save{slot:02}.sav"));
    ecs_save_to_path(scene, &path)
}

/// Load a save file from an explicit path into an existing `EcsScene`.
/// The scene must already have a hero entity spawned; only component data is patched.
pub fn ecs_load_from_path(
    path: &Path,
    scene: &mut crate::game::ecs::scene::EcsScene,
) -> anyhow::Result<()> {
    let data = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("failed to read save file {}: {}", path.display(), e))?;
    if data.len() < 8 {
        anyhow::bail!("invalid save file: too short");
    }
    if &data[0..4] != SAVE_MAGIC.as_ref() {
        anyhow::bail!("invalid save file: bad magic");
    }
    let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if version != SAVE_VERSION {
        anyhow::bail!(
            "invalid save file: version mismatch (got {}, expected {})",
            version,
            SAVE_VERSION,
        );
    }
    let sf = proto::SaveFile::decode(&data[8..])
        .map_err(|e| anyhow::anyhow!("failed to decode save file: {}", e))?;
    proto_to_ecs(sf, scene);
    Ok(())
}

/// Load into `EcsScene` from slot `slot` under the platform config dir.
pub fn ecs_load_game(
    slot: u8,
    scene: &mut crate::game::ecs::scene::EcsScene,
) -> anyhow::Result<()> {
    let base = dirs::config_dir()
        .context("could not determine config directory")?
        .join("faery")
        .join("saves");
    let path = base.join(format!("save{slot:02}.sav"));
    ecs_load_from_path(&path, scene)
}

// --------------------------------------------------------------------------
// Transcript helpers
// --------------------------------------------------------------------------

/// Return the filesystem path for the story-transcript file of `slot`.
fn transcript_path(slot: u8) -> anyhow::Result<std::path::PathBuf> {
    let base = dirs::config_dir()
        .context("could not determine config directory")?
        .join("faery")
        .join("saves");
    Ok(base.join(format!("save{slot:02}.txt")))
}

/// Overwrite (or create) the transcript file for `slot` with `lines`.
/// Each line is written as a UTF-8 text line.
pub fn save_transcript(lines: &[String], slot: u8) -> anyhow::Result<()> {
    use std::io::Write;
    let path = transcript_path(slot)?;
    std::fs::create_dir_all(path.parent().unwrap())
        .with_context(|| format!("creating save directory for transcript"))?;
    let mut f = std::fs::File::create(&path)
        .with_context(|| format!("creating transcript {}", path.display()))?;
    for line in lines {
        writeln!(f, "{}", line)?;
    }
    Ok(())
}

/// Load the transcript for `slot`.  Returns an empty `Vec` if no file exists.
pub fn load_transcript(slot: u8) -> Vec<String> {
    transcript_path(slot)
        .ok()
        .and_then(|p| std::fs::read_to_string(&p).ok())
        .map(|data| data.lines().map(|l| l.to_string()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_save_file_header() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("save00.sav");
        let state = GameState::new();
        save_to_path(&state, &path).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], SAVE_MAGIC, "magic mismatch");
        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(version, SAVE_VERSION, "version mismatch");
    }

    #[test]
    fn test_savefile_proto_roundtrip() {
        let mut state = GameState::new();
        state.hero_x = 12345;
        state.hero_y = 54321;
        state.vitality = 42;
        state.brave = 99;
        state.julstuff[0] = 7;
        state.julstuff[34] = 15;
        state.brother = 2;
        state.daynight = 6001;

        let save = state_to_proto(&state);
        let encoded = save.encode_to_vec();
        let decoded = proto::SaveFile::decode(encoded.as_slice()).unwrap();

        assert_eq!(decoded.hero_x, 12345);
        assert_eq!(decoded.hero_y, 54321);
        assert_eq!(decoded.vitality, 42);
        assert_eq!(decoded.brave, 99);
        assert_eq!(decoded.brother, 2);
        assert_eq!(decoded.daynight, 6001);
        let jul = decoded.julstuff.unwrap();
        assert_eq!(jul.slots[0], 7);
        assert_eq!(jul.slots[34], 15);
    }

    #[test]
    fn test_load_bad_magic() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad_magic.sav");
        let mut buf: Vec<u8> = b"XXXZ".to_vec();
        buf.extend_from_slice(&SAVE_VERSION.to_le_bytes());
        std::fs::write(&path, &buf).unwrap();
        let err = load_from_path(&path)
            .err()
            .expect("expected Err for bad magic");
        assert!(err.to_string().contains("bad magic"), "got: {}", err);
    }

    #[test]
    fn test_load_wrong_version() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("wrong_version.sav");
        let mut buf: Vec<u8> = SAVE_MAGIC.to_vec();
        buf.extend_from_slice(&99u32.to_le_bytes());
        std::fs::write(&path, &buf).unwrap();
        let err = load_from_path(&path)
            .err()
            .expect("expected Err for wrong version");
        assert!(err.to_string().contains("version mismatch"), "got: {}", err);
    }

    #[test]
    fn test_load_missing_file() {
        let path = std::path::Path::new("/tmp/faery_nonexistent_save_xyzzy.sav");
        let err = load_from_path(path)
            .err()
            .expect("expected Err for missing file");
        assert!(err.to_string().contains("failed to read"), "got: {}", err);
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("save00.sav");

        let mut state = GameState::new();
        state.hero_x = 19000;
        state.hero_y = 15000;
        state.vitality = 77;
        state.brave = 50;
        state.julstuff[3] = 9;
        state.brother = 3;
        state.daynight = 12345;
        state.region_num = 4;

        save_to_path(&state, &path).unwrap();
        let loaded = load_from_path(&path).unwrap();

        assert_eq!(loaded.hero_x, state.hero_x);
        assert_eq!(loaded.hero_y, state.hero_y);
        assert_eq!(loaded.vitality, state.vitality);
        assert_eq!(loaded.brave, state.brave);
        assert_eq!(loaded.julstuff[3], state.julstuff[3]);
        assert_eq!(loaded.brother, state.brother);
        assert_eq!(loaded.daynight, state.daynight);
        assert_eq!(loaded.region_num, state.region_num);
    }

    #[test]
    fn test_postload_cleanup() {
        // T2-SAVE-POSTLOAD: verify SPEC §24.5 post-load cleanup
        let dir = tempdir().unwrap();
        let path = dir.path().join("save_postload.sav");

        let mut state = GameState::new();
        state.encounter_number = 42;
        state.actors_loading = true;
        state.encounter_type = 99;
        state.viewstatus = 3;

        save_to_path(&state, &path).unwrap();
        let loaded = load_from_path(&path).unwrap();

        // Post-load cleanup should reset these fields
        assert_eq!(
            loaded.encounter_number, 0,
            "encounter_number should be cleared"
        );
        assert_eq!(
            loaded.actors_loading, false,
            "actors_loading should be cleared"
        );
        assert_eq!(loaded.encounter_type, 0, "encounter_type should be cleared");
        assert_eq!(loaded.viewstatus, 99, "viewstatus should be set to 99");
    }

    #[test]
    fn test_world_objects_persistence() {
        // T2-SAVE-REGIONAL: verify per-region object tables are persisted
        let dir = tempdir().unwrap();
        let path = dir.path().join("save_objects.sav");

        let mut state = GameState::new();
        state.world_objects.push(WorldObject {
            ob_id: 25,
            ob_stat: 1,
            region: 2,
            x: 1000,
            y: 2000,
            visible: true,
            goal: 0,
        });
        state.world_objects.push(WorldObject {
            ob_id: 114,
            ob_stat: 5,
            region: 3,
            x: 3000,
            y: 4000,
            visible: false,
            goal: 0,
        });

        save_to_path(&state, &path).unwrap();
        let loaded = load_from_path(&path).unwrap();

        assert_eq!(loaded.world_objects.len(), 2);
        assert_eq!(loaded.world_objects[0].ob_id, 25);
        assert_eq!(loaded.world_objects[0].ob_stat, 1);
        assert_eq!(loaded.world_objects[0].region, 2);
        assert_eq!(loaded.world_objects[0].x, 1000);
        assert_eq!(loaded.world_objects[0].y, 2000);
        assert_eq!(loaded.world_objects[0].visible, true);

        assert_eq!(loaded.world_objects[1].ob_id, 114);
        assert_eq!(loaded.world_objects[1].ob_stat, 5);
        assert_eq!(loaded.world_objects[1].region, 3);
        assert_eq!(loaded.world_objects[1].x, 3000);
        assert_eq!(loaded.world_objects[1].y, 4000);
        assert_eq!(loaded.world_objects[1].visible, false);
    }

    // ── ECS round-trip tests ────────────────────────────────────────────────

    #[test]
    fn ecs_save_load_roundtrip_position() {
        use crate::game::ecs::components::Position;
        let dir = tempdir().unwrap();
        let path = dir.path().join("ecs_pos.sav");

        let scene = crate::game::ecs::scene::new_for_test();
        // Set a distinctive position.
        scene.world.get::<&mut Position>(scene.res.hero_entity)
            .map(|mut p| { p.x = 12345.0; p.y = 54321.0; }).ok();

        ecs_save_to_path(&scene, &path).unwrap();

        let mut loaded = crate::game::ecs::scene::new_for_test();
        ecs_load_from_path(&path, &mut loaded).unwrap();

        let pos = loaded.world.get::<&Position>(loaded.res.hero_entity).unwrap();
        assert_eq!(pos.x as u32, 12345);
        assert_eq!(pos.y as u32, 54321);
    }

    #[test]
    fn ecs_save_load_roundtrip_stats() {
        use crate::game::ecs::components::HeroStats;
        let dir = tempdir().unwrap();
        let path = dir.path().join("ecs_stats.sav");

        let scene = crate::game::ecs::scene::new_for_test();
        scene.world.get::<&mut HeroStats>(scene.res.hero_entity).map(|mut s| {
            s.vitality = 77; s.brave = 50; s.luck = 33; s.kind = 66; s.wealth = 99;
        }).ok();

        ecs_save_to_path(&scene, &path).unwrap();

        let mut loaded = crate::game::ecs::scene::new_for_test();
        ecs_load_from_path(&path, &mut loaded).unwrap();

        let s = loaded.world.get::<&HeroStats>(loaded.res.hero_entity).unwrap();
        assert_eq!(s.vitality, 77);
        assert_eq!(s.brave,    50);
        assert_eq!(s.luck,     33);
        assert_eq!(s.kind,     66);
        assert_eq!(s.wealth,   99);
    }

    #[test]
    fn ecs_save_load_roundtrip_clock() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ecs_clock.sav");

        let mut scene = crate::game::ecs::scene::new_for_test();
        scene.res.clock.daynight     = 12345;
        scene.res.clock.lightlevel   = 200;
        scene.res.clock.cycle        = 42;
        scene.res.clock.light_timer  = 100;
        scene.res.clock.secret_timer = 50;

        ecs_save_to_path(&scene, &path).unwrap();

        let mut loaded = crate::game::ecs::scene::new_for_test();
        ecs_load_from_path(&path, &mut loaded).unwrap();

        assert_eq!(loaded.res.clock.daynight,     12345);
        assert_eq!(loaded.res.clock.lightlevel,   200);
        assert_eq!(loaded.res.clock.cycle,        42);
        assert_eq!(loaded.res.clock.light_timer,  100);
        assert_eq!(loaded.res.clock.secret_timer, 50);
    }

    #[test]
    fn ecs_save_load_roundtrip_region() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ecs_region.sav");

        let mut scene = crate::game::ecs::scene::new_for_test();
        scene.res.region.region_num = 3;
        scene.res.region.princess   = 7;
        scene.res.brother.cheat1    = true;
        scene.res.brother.witchflag = true;

        ecs_save_to_path(&scene, &path).unwrap();

        let mut loaded = crate::game::ecs::scene::new_for_test();
        ecs_load_from_path(&path, &mut loaded).unwrap();

        assert_eq!(loaded.res.region.region_num, 3);
        assert_eq!(loaded.res.region.princess,   7);
        assert!(loaded.res.brother.cheat1);
        assert!(loaded.res.brother.witchflag);
    }

    #[test]
    fn ecs_postload_cleanup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ecs_cleanup.sav");

        let mut scene = crate::game::ecs::scene::new_for_test();
        scene.res.region.encounter_number = 42;
        scene.res.region.encounter_type   = 99;

        ecs_save_to_path(&scene, &path).unwrap();

        let mut loaded = crate::game::ecs::scene::new_for_test();
        ecs_load_from_path(&path, &mut loaded).unwrap();

        assert_eq!(loaded.res.region.encounter_number, 0,   "encounter_number cleared");
        assert_eq!(loaded.res.region.encounter_type,   0,   "encounter_type cleared");
        assert_eq!(loaded.res.view.viewstatus,         99,  "viewstatus set to 99");
    }

    #[test]
    fn ecs_bad_magic_rejected() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.sav");
        let mut buf = b"XXXX".to_vec();
        buf.extend_from_slice(&SAVE_VERSION.to_le_bytes());
        std::fs::write(&path, &buf).unwrap();

        let mut scene = crate::game::ecs::scene::new_for_test();
        let err = ecs_load_from_path(&path, &mut scene).err().unwrap();
        assert!(err.to_string().contains("bad magic"), "got: {}", err);
    }
}
