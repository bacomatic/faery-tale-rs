use bitflags::bitflags;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatId {
    Vitality,
    Brave,
    Luck,
    Kind,
    Wealth,
    Hunger,
    Fatigue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrotherId {
    Julian,
    Phillip,
    Kevin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagicEffect {
    Light,
    Secret,
    Freeze,
}

pub const DEFAULT_TICK_RATE_HZ: u32 = 15;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct GodModeFlags: u8 {
        const NOCLIP       = 0b0001;
        const INVINCIBLE   = 0b0010;
        const ONE_HIT_KILL = 0b0100;
        const INSANE_REACH = 0b1000;
    }
}

#[derive(Debug, Clone)]
pub enum DebugCommand {
    SetStat {
        stat: StatId,
        value: i16,
    },
    AdjustStat {
        stat: StatId,
        delta: i16,
    },
    SetInventory {
        index: u8,
        value: u8,
    },
    AdjustInventory {
        index: u8,
        delta: i8,
    },
    TeleportSafe,
    TeleportStoneRing {
        index: u8,
    },
    TeleportCoords {
        x: u16,
        y: u16,
    },
    ToggleMagicEffect {
        effect: MagicEffect,
    },
    HeroPack,
    SetGodMode {
        flags: GodModeFlags,
    },
    SummonSwan,
    /// daynight value: 0=Midnight, 6000=Morning, 12000=Midday, 18000=Evening
    SetDayPhase {
        phase: u16,
    },
    SetGameTime {
        hour: u8,
        minute: u8,
    },
    HoldTimeOfDay {
        hold: bool,
    },
    RestartAsBrother {
        brother: BrotherId,
    },
    InstaKill,
    TriggerWitchEffect,
    TriggerTeleportEffect,
    TriggerPaletteTransition {
        to_black: bool,
    },
    /// Request actor list; gameplay scene will push to log buffer.
    QueryActors,
    /// Request song list; gameplay scene will push to log buffer.
    QuerySongs,
    /// Dump `count` ADF blocks starting at `block` as hex rows to the debug log.
    DumpAdfBlock {
        block: u32,
        count: u32,
    },
    /// Dump full terra lookup chain at hero's current position (both foot probes).
    QueryTerrain,
    /// Force a regional encounter (4 enemies, mixflag blending).
    SpawnEncounterRandom,
    /// Spawn one named enemy type adjacent to the hero.
    SpawnEncounterType(u8),
    /// Deactivate all NPCs in the current npc_table.
    ClearEncounters,
    /// Scatter items in a ring around the player.
    /// item_id: None = random from safe pool (no talisman); Some(id) = specific item.
    ScatterItems {
        count: usize,
        item_id: Option<usize>,
    },
    /// Kill a single actor slot (1..=19). Slot 0 is hero; use /die instead.
    KillActorSlot {
        slot: u8,
    },
    /// Set the `cheat1` debug-keys mode flag.
    SetCheat1 {
        enabled: bool,
    },
    /// Teleport hero to the named extent's walkable center.
    TeleportNamedLocation {
        name: String,
    },
    /// Dump door state to the debug log.
    QueryDoors,
    /// Dump the extent zone under the hero's feet to the debug log.
    QueryExtent,
    /// Set the game tick rate in Hz. Startup defaults to 15; 30 = normal, 60 = double.
    SetTickRate {
        hz: u32,
    },
}
