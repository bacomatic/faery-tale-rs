//! Save/load game state using protobuf (prost).

/// Generated protobuf types for the save format.
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/faery.rs"));
}

pub const SAVE_MAGIC: &[u8; 4] = b"FERY";
pub const SAVE_VERSION: u32 = 1;
pub const SAVE_DIR: &str = ".config/faery/saves";
