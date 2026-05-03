//! Game event dispatch, mood calculation, and stat field helpers.

use super::*;

impl GameplayScene {
    pub fn handle_game_event(&mut self, event: crate::game::game_event::GameEvent) {
        use crate::game::game_event::GameEvent;
        match event {
            GameEvent::Message { text } => {
                self.messages.push(text);
            }
            _ => {}
        }
    }

    pub(crate) fn setmood(&self) -> u8 {
        let s = &self.state;
        // Priority 1: Death
        if s.vitality <= 0 {
            return 6;
        }
        // Priority 2: Zone (astral plane bounds)
        if s.hero_x >= 0x2400 && s.hero_x <= 0x3100 && s.hero_y >= 0x8200 && s.hero_y <= 0x8a00 {
            return 4;
        }
        // Priority 3: Battle
        if s.battleflag {
            return 1;
        }
        // Priority 4: Dungeon (underground)
        if s.region_num > 7 {
            return 5;
        }
        // Priority 5 & 6: Day/Night based on lightlevel
        if s.lightlevel > 120 {
            0 // Day
        } else {
            2 // Night
        }
    }

    pub(crate) fn stat_field_mut(state: &mut GameState, stat: StatId) -> &mut i16 {
        match stat {
            StatId::Vitality => &mut state.vitality,
            StatId::Brave => &mut state.brave,
            StatId::Luck => &mut state.luck,
            StatId::Kind => &mut state.kind,
            StatId::Wealth => &mut state.wealth,
            StatId::Hunger => &mut state.hunger,
            StatId::Fatigue => &mut state.fatigue,
        }
    }
}
