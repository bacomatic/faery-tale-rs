//! HiScreen overlay: status bar above the map showing vitality, gold, compass.

use crate::game::direction::Direction;
use crate::game::game_state::GameState;

/// Compass needle character for each of 8 facing directions (Amiga DIR_* order).
pub fn facing_char(dir: Direction) -> char {
    match dir {
        Direction::NW   => '↖',
        Direction::N    => '↑',
        Direction::NE   => '↗',
        Direction::E    => '→',
        Direction::SE   => '↘',
        Direction::S    => '↓',
        Direction::SW   => '↙',
        Direction::W    => '←',
        Direction::None => '?',
    }
}

/// Format the HiScreen status line as ASCII for debug output.
/// Format: "VIT:xxx GOLD:xxx DIR:↑ REGION:xx LIGHT:xxx"
pub fn format_hiscreen(state: &GameState) -> String {
    format!(
        "VIT:{:3} GOLD:{:4} DIR:{} REGION:{:2} LIGHT:{:3}",
        state.vitality,
        state.gold,
        facing_char(state.facing),
        state.region_num,
        state.lightlevel,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::direction::Direction;
    use crate::game::game_state::GameState;

    #[test]
    fn test_facing_char_north() {
        assert_eq!(facing_char(Direction::N), '↑');
    }

    #[test]
    fn test_format_hiscreen() {
        let state = GameState::new();
        let s = format_hiscreen(&state);
        assert!(s.contains("VIT:"));
        assert!(s.contains("GOLD:"));
        assert!(s.contains("DIR:"));
    }
}
