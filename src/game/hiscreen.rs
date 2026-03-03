//! HiScreen overlay: status bar above the map showing vitality, gold, compass.

use crate::game::game_state::GameState;

/// Cardinal direction index (0=N, 2=E, 4=S, 6=W, odd=diagonals, from original).
/// Player faces the direction they last moved.
pub fn facing_char(dir: u8) -> char {
    // Compass needle character for each of 8 directions
    match dir & 7 {
        0 => '↑', // N
        1 => '↗', // NE
        2 => '→', // E
        3 => '↘', // SE
        4 => '↓', // S
        5 => '↙', // SW
        6 => '←', // W
        7 => '↖', // NW
        _ => '?',
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
    use crate::game::game_state::GameState;

    #[test]
    fn test_facing_char_north() {
        assert_eq!(facing_char(0), '↑');
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
