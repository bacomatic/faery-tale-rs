/// Time-of-day phase — used by the debug snapshot and clock system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DayPhase {
    #[default]
    Midnight = 0,
    Morning = 4,
    Midday = 6,
    Evening = 9,
}
