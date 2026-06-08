//! Canonical direction type for all facing/heading values.
//! Amiga DIR_* encoding: NW=0, N=1, NE=2, E=3, SE=4, S=5, SW=6, W=7.
//! Value 9 is the DIRECTION_STILL sentinel (Direction::None).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Direction {
    NW   = 0,
    #[default]
    N    = 1,
    NE   = 2,
    E    = 3,
    SE   = 4,
    S    = 5,
    SW   = 6,
    W    = 7,
    None = 9,
}

impl From<u8> for Direction {
    fn from(v: u8) -> Self {
        match v {
            0 => Direction::NW,
            1 => Direction::N,
            2 => Direction::NE,
            3 => Direction::E,
            4 => Direction::SE,
            5 => Direction::S,
            6 => Direction::SW,
            7 => Direction::W,
            _ => Direction::None,
        }
    }
}

impl Direction {
    /// Rotate clockwise by `steps` (positive = CW, negative = CCW).
    /// `Direction::None` is returned unchanged.
    pub fn rotate_by(self, steps: i8) -> Direction {
        if self == Direction::None {
            return Direction::None;
        }
        Direction::from(((self as i8).wrapping_add(steps)).rem_euclid(8) as u8)
    }

    /// One step clockwise.
    pub fn rotate_cw(self) -> Direction {
        self.rotate_by(1)
    }

    /// One step counter-clockwise.
    pub fn rotate_ccw(self) -> Direction {
        self.rotate_by(-1)
    }

    /// Opposite direction (180°).
    pub fn opposite(self) -> Direction {
        self.rotate_by(4)
    }

    /// True if this is a cardinal (N/S/E/W).
    pub fn is_cardinal(self) -> bool {
        matches!(self, Direction::N | Direction::S | Direction::E | Direction::W)
    }

    /// True if this is a diagonal (NW/NE/SW/SE).
    pub fn is_diagonal(self) -> bool {
        matches!(self, Direction::NW | Direction::NE | Direction::SW | Direction::SE)
    }
}

#[cfg(test)]
mod tests {
    use super::Direction;

    #[test]
    fn amiga_discriminants() {
        assert_eq!(Direction::NW as u8, 0);
        assert_eq!(Direction::N  as u8, 1);
        assert_eq!(Direction::NE as u8, 2);
        assert_eq!(Direction::E  as u8, 3);
        assert_eq!(Direction::SE as u8, 4);
        assert_eq!(Direction::S  as u8, 5);
        assert_eq!(Direction::SW as u8, 6);
        assert_eq!(Direction::W  as u8, 7);
        assert_eq!(Direction::None as u8, 9);
    }

    #[test]
    fn from_u8_all_values() {
        assert_eq!(Direction::from(0u8), Direction::NW);
        assert_eq!(Direction::from(1u8), Direction::N);
        assert_eq!(Direction::from(2u8), Direction::NE);
        assert_eq!(Direction::from(3u8), Direction::E);
        assert_eq!(Direction::from(4u8), Direction::SE);
        assert_eq!(Direction::from(5u8), Direction::S);
        assert_eq!(Direction::from(6u8), Direction::SW);
        assert_eq!(Direction::from(7u8), Direction::W);
        assert_eq!(Direction::from(8u8), Direction::None);
        assert_eq!(Direction::from(9u8), Direction::None);
        assert_eq!(Direction::from(255u8), Direction::None);
    }

    #[test]
    fn rotate_cw_full_circle() {
        let dirs = [
            Direction::NW, Direction::N, Direction::NE, Direction::E,
            Direction::SE, Direction::S, Direction::SW, Direction::W,
        ];
        for i in 0..8 {
            assert_eq!(dirs[i].rotate_cw(), dirs[(i + 1) % 8]);
        }
    }

    #[test]
    fn rotate_ccw_full_circle() {
        let dirs = [
            Direction::NW, Direction::N, Direction::NE, Direction::E,
            Direction::SE, Direction::S, Direction::SW, Direction::W,
        ];
        for i in 0..8 {
            assert_eq!(dirs[i].rotate_ccw(), dirs[(i + 7) % 8]);
        }
    }

    #[test]
    fn opposite() {
        assert_eq!(Direction::N.opposite(),  Direction::S);
        assert_eq!(Direction::S.opposite(),  Direction::N);
        assert_eq!(Direction::E.opposite(),  Direction::W);
        assert_eq!(Direction::W.opposite(),  Direction::E);
        assert_eq!(Direction::NE.opposite(), Direction::SW);
        assert_eq!(Direction::SW.opposite(), Direction::NE);
        assert_eq!(Direction::NW.opposite(), Direction::SE);
        assert_eq!(Direction::SE.opposite(), Direction::NW);
    }

    #[test]
    fn none_rotate_returns_none() {
        assert_eq!(Direction::None.rotate_cw(),   Direction::None);
        assert_eq!(Direction::None.rotate_ccw(),  Direction::None);
        assert_eq!(Direction::None.rotate_by(3),  Direction::None);
        assert_eq!(Direction::None.opposite(),    Direction::None);
    }

    #[test]
    fn rotate_by_2() {
        assert_eq!(Direction::N.rotate_by(2),  Direction::E);
        assert_eq!(Direction::N.rotate_by(-2), Direction::W);
        assert_eq!(Direction::W.rotate_by(2),  Direction::N);
    }

    #[test]
    fn is_cardinal_and_diagonal() {
        assert!(Direction::N.is_cardinal());
        assert!(Direction::S.is_cardinal());
        assert!(Direction::E.is_cardinal());
        assert!(Direction::W.is_cardinal());
        assert!(!Direction::NE.is_cardinal());

        assert!(Direction::NE.is_diagonal());
        assert!(Direction::NW.is_diagonal());
        assert!(Direction::SE.is_diagonal());
        assert!(Direction::SW.is_diagonal());
        assert!(!Direction::N.is_diagonal());
    }
}
