//! Save/load game state using protobuf (prost).

use std::io::Write;
use std::path::Path;

use anyhow::Context;
use prost::Message;

use crate::game::game_state::GameState;

/// Generated protobuf types for the save format.
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/faery.rs"));
}

pub const SAVE_MAGIC: &[u8; 4] = b"FERY";
pub const SAVE_VERSION: u32 = 1;
pub const SAVE_DIR: &str = ".config/faery/saves";

fn state_to_proto(state: &GameState) -> proto::SaveFile {
    let make_stuff = |arr: &[u8; 35]| proto::BrotherStuff {
        slots: arr.iter().map(|&v| v as u32).collect(),
    };

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

        princess: state.princess as u32,
        dayperiod: state.dayperiod as u32,

        current_mood: state.current_mood as u32,

        actors: vec![],
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
}
