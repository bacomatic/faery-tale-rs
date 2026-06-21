// Port of menus[10] data structures and logic from original/fmain.c (fmain.c:534-620, 3758-4445)

pub const LABEL1: &str = "ItemsMagicTalk Buy  Game ";
pub const LABEL2: &str = "List Take Look Use  Give ";
pub const LABEL3: &str = "Yell Say  Ask  ";
pub const LABEL4: &str = "PauseMusicSoundQuit Load ";
pub const LABEL5: &str = "Food ArrowVial Mace SwordBow  Totem";
pub const LABEL6: &str = "StoneJewelVial Orb  TotemRing Skull";
pub const LABEL7: &str = "Dirk Mace SwordBow  Wand LassoShellKey  Sun       ";
pub const LABEL8: &str = "Save Exit ";
pub const LABEL9: &str = "Gold GreenBlue Red  Grey White";
pub const LABELA: &str = "Gold Book Writ Bone ";
pub const LABELB: &str = "  A    B    C    D    E    F    G    H  ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuMode {
    Items = 0,
    Magic = 1,
    Talk = 2,
    Buy = 3,
    Game = 4,
    SaveX = 5,
    Keys = 6,
    Give = 7,
    Use = 8,
    File = 9,
}

impl From<usize> for MenuMode {
    fn from(v: usize) -> Self {
        match v {
            0 => MenuMode::Items,
            1 => MenuMode::Magic,
            2 => MenuMode::Talk,
            3 => MenuMode::Buy,
            4 => MenuMode::Game,
            5 => MenuMode::SaveX,
            6 => MenuMode::Keys,
            7 => MenuMode::Give,
            8 => MenuMode::Use,
            9 => MenuMode::File,
            _ => MenuMode::Items,
        }
    }
}

// bit0 = selected/on, bit1 = displayed/visible, bits 2-7 = action type
pub const TYPE_MASK: u8 = 0xFC;
pub const TYPE_TAB: u8 = 0; // not changeable (tab header)
pub const TYPE_TOGGLE: u8 = 4; // click flips bit0
pub const TYPE_IMMEDIATE: u8 = 8; // fire on click
pub const TYPE_RADIO: u8 = 12; // radio button
pub const FLAG_SELECTED: u8 = 1;
pub const FLAG_DISPLAYED: u8 = 2;

// textcolors palette indices for key color rendering (fmain.c:557)
pub const KEYCOLORS: [u8; 6] = [8, 6, 4, 2, 14, 1];

pub struct MenuDef {
    pub labels: &'static str,
    pub num: u8,
    pub color: u8,
    pub enabled: [u8; 12],
}

pub struct ButtonRender {
    pub display_slot: usize,
    pub menu_index: i8, // -1 = empty slot
    pub text: String,   // 5 chars (padded)
    pub fg_color: u8,   // textcolors palette index (0=black, 1=white)
    pub bg_color: u8,   // textcolors palette index
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    // Handled entirely within MenuState:
    SwitchMode(MenuMode),
    // Actions GameplayScene must handle:
    Inventory,
    Take,
    Look,
    UseMenu,
    GiveMenu,
    CastSpell(u8),
    Yell,
    Say,
    Ask,
    BuyItem(u8),
    SetWeapon(u8),
    TryKey(u8),
    GiveGold,
    GiveWrit,
    GiveBone,
    SaveGame(u8),
    LoadGame(u8),
    Quit,
    TogglePause,
    ToggleMusic,
    ToggleSound,
    RefreshMusic,
    SummonTurtle,
    UseSunstone,
    None,
}

// (key_char, menu_mode, menu_slot) — fmain.c:579-589
pub const LETTER_LIST: &[(u8, MenuMode, u8)] = &[
    (b'I', MenuMode::Items, 5),
    (b'T', MenuMode::Items, 6),
    (b'?', MenuMode::Items, 7),
    (b'U', MenuMode::Items, 8),
    (b'G', MenuMode::Items, 9),
    (b'Y', MenuMode::Talk, 5),
    (b'S', MenuMode::Talk, 6),
    (b'A', MenuMode::Talk, 7),
    (b' ', MenuMode::Game, 5),
    (b'M', MenuMode::Game, 6),
    (b'F', MenuMode::Game, 7),
    (b'Q', MenuMode::Game, 8),
    (b'L', MenuMode::Game, 9),
    (b'O', MenuMode::Buy, 5),
    (b'R', MenuMode::Buy, 6),
    (b'8', MenuMode::Buy, 7),
    (b'C', MenuMode::Buy, 8),
    (b'W', MenuMode::Buy, 9),
    (b'B', MenuMode::Buy, 10),
    (b'E', MenuMode::Buy, 11),
    (b'V', MenuMode::SaveX, 5),
    (b'X', MenuMode::SaveX, 6),
    // F1-F7 → MAGIC slots 5-11 (fmain.c:537-547, key codes 10-16)
    (10, MenuMode::Magic, 5),
    (11, MenuMode::Magic, 6),
    (12, MenuMode::Magic, 7),
    (13, MenuMode::Magic, 8),
    (14, MenuMode::Magic, 9),
    (15, MenuMode::Magic, 10),
    (16, MenuMode::Magic, 11),
    (b'1', MenuMode::Use, 0),
    (b'2', MenuMode::Use, 1),
    (b'3', MenuMode::Use, 2),
    (b'4', MenuMode::Use, 3),
    (b'5', MenuMode::Use, 4),
    (b'6', MenuMode::Use, 5),
    (b'7', MenuMode::Use, 6),
    (b'K', MenuMode::Use, 7),
];

pub struct MenuState {
    pub cmode: MenuMode,
    pub menus: [MenuDef; 10],
    pub real_options: [i8; 12], // display slot → menu index (-1 = empty)
    /// When true, the File sub-menu is being used for saving (not loading).
    save_pending: bool,
    /// Track which display slot is currently pressed (mouse held down).
    /// Used for click-and-hold behavior where action triggers on release.
    pressed_slot: Option<usize>,
    /// For toggle items, remember original selected state to restore on cancel.
    pressed_was_selected: bool,
    /// The slot where the mouse was originally pressed (valid while mouse button is held).
    /// Used to re-activate the press when moving back into this slot region.
    committed_slot: Option<usize>,
}

impl MenuState {
    /// Initialize with exact original values from fmain.c:563-573.
    pub fn new() -> Self {
        MenuState {
            cmode: MenuMode::Items,
            real_options: [-1; 12],
            save_pending: false,
            pressed_slot: None,
            pressed_was_selected: false,
            committed_slot: None,
            menus: [
                // ITEMS
                MenuDef {
                    labels: LABEL2,
                    num: 10,
                    color: 6,
                    enabled: [3, 2, 2, 2, 2, 10, 10, 10, 10, 10, 0, 0],
                },
                // MAGIC
                MenuDef {
                    labels: LABEL6,
                    num: 12,
                    color: 5,
                    enabled: [2, 3, 2, 2, 2, 8, 8, 8, 8, 8, 8, 8],
                },
                // TALK
                MenuDef {
                    labels: LABEL3,
                    num: 8,
                    color: 9,
                    enabled: [2, 2, 3, 2, 2, 10, 10, 10, 0, 0, 0, 0],
                },
                // BUY
                MenuDef {
                    labels: LABEL5,
                    num: 12,
                    color: 10,
                    enabled: [2, 2, 2, 3, 2, 10, 10, 10, 10, 10, 10, 10],
                },
                // GAME
                MenuDef {
                    labels: LABEL4,
                    num: 10,
                    color: 2,
                    enabled: [2, 2, 2, 2, 3, 6, 7, 7, 10, 10, 0, 0],
                },
                // SAVEX
                MenuDef {
                    labels: LABEL8,
                    num: 7,
                    color: 0,
                    enabled: [2, 2, 2, 2, 2, 10, 10, 0, 0, 0, 0, 0],
                },
                // KEYS
                MenuDef {
                    labels: LABEL9,
                    num: 11,
                    color: 8,
                    enabled: [2, 2, 2, 2, 2, 10, 10, 10, 10, 10, 10, 0],
                },
                // GIVE
                MenuDef {
                    labels: LABELA,
                    num: 9,
                    color: 10,
                    enabled: [2, 2, 2, 2, 2, 10, 0, 0, 0, 0, 0, 0],
                },
                // USE
                MenuDef {
                    labels: LABEL7,
                    num: 10,
                    color: 8,
                    enabled: [10, 10, 10, 10, 10, 10, 10, 10, 10, 0, 10, 10],
                },
                // FILE
                MenuDef {
                    labels: LABELB,
                    num: 10,
                    color: 5,
                    enabled: [10, 10, 10, 10, 10, 10, 10, 10, 0, 0, 0, 0],
                },
            ],
        }
    }

    /// Returns 8 if stuff[i] == 0, else 10 (fmain.c: stuff_flag helper).
    pub fn stuff_flag(stuff: &[u8], i: usize) -> u8 {
        if stuff[i] == 0 {
            8
        } else {
            10
        }
    }

    /// Update enabled flags based on player inventory and wealth (fmain.c:4419-4441).
    pub fn set_options(&mut self, stuff: &[u8], wealth: i16) {
        for i in 0..7 {
            self.menus[MenuMode::Magic as usize].enabled[i + 5] = Self::stuff_flag(stuff, i + 9);
            self.menus[MenuMode::Use as usize].enabled[i] = Self::stuff_flag(stuff, i);
        }

        let mut j: u8 = 8;
        for i in 0..6 {
            let v = Self::stuff_flag(stuff, i + 16);
            self.menus[MenuMode::Keys as usize].enabled[i + 5] = v;
            if v == 10 {
                j = 10;
            }
        }
        self.menus[MenuMode::Use as usize].enabled[7] = j;
        self.menus[MenuMode::Use as usize].enabled[8] = Self::stuff_flag(stuff, 7); // sunstone

        let j = if wealth > 2 { 10 } else { 8 };
        self.menus[MenuMode::Give as usize].enabled[5] = j; // gold
        self.menus[MenuMode::Give as usize].enabled[6] = 8; // book (always hidden)
        self.menus[MenuMode::Give as usize].enabled[7] = Self::stuff_flag(stuff, 28); // writ
        self.menus[MenuMode::Give as usize].enabled[8] = Self::stuff_flag(stuff, 29);
        // bone
    }

    /// Switch menu mode; refuses if paused (fmain.c:4409-4414).
    pub fn gomenu(&mut self, mode: MenuMode) {
        if self.is_paused() {
            return;
        }
        self.cmode = mode;
        self.real_options = [-1; 12];
    }

    /// Build render list for current menu (fmain.c:3758-3783).
    pub fn print_options(&mut self) -> Vec<ButtonRender> {
        let mut result = Vec::with_capacity(12);
        let mut j = 0usize;
        let num = self.menus[self.cmode as usize].num as usize;
        for i in 0..num {
            let x = self.menus[self.cmode as usize].enabled[i];
            if x & FLAG_DISPLAYED == 0 {
                continue;
            }
            self.real_options[j] = i as i8;
            let selected = (x & FLAG_SELECTED) != 0;
            let pressed = self.is_pressed(j);
            result.push(self.propt(j, selected, pressed));
            j += 1;
            if j > 11 {
                break;
            }
        }
        while j < 12 {
            self.real_options[j] = -1;
            result.push(ButtonRender {
                display_slot: j,
                menu_index: -1,
                text: "      ".to_string(),
                fg_color: 0,
                bg_color: 0,
            });
            j += 1;
        }
        result
    }

    /// Compute render info for one button (fmain.c:3785-3828).
    /// `selected` is the button's selected/toggled state.
    /// `pressed` is true when the user is holding the mouse down on this button.
    pub fn propt(&self, j: usize, selected: bool, pressed: bool) -> ButtonRender {
        let k = self.real_options[j] as usize; // menu index
        // When pressed (mouse held down), highlight non-toggle items visually.
        // Toggle items already show their toggled state from handle_mouse_down.
        let atype = if k < self.menus[self.cmode as usize].num as usize {
            self.menus[self.cmode as usize].enabled[k] & TYPE_MASK
        } else {
            TYPE_IMMEDIATE
        };
        let is_toggle = atype == TYPE_TOGGLE;
        let fg_color = if selected || (pressed && !is_toggle) { 1 } else { 0 };
        let bg_color = if self.cmode == MenuMode::Use {
            14
        } else if self.cmode == MenuMode::File {
            13
        } else if k < 5 {
            4
        } else if self.cmode == MenuMode::Keys {
            KEYCOLORS[k - 5]
        } else if self.cmode == MenuMode::SaveX {
            k as u8
        } else {
            self.menus[self.cmode as usize].color
        };

        let text_offset = k * 5;
        let text = if self.cmode as usize >= MenuMode::Use as usize {
            self.menus[self.cmode as usize].labels[text_offset..text_offset + 5].to_string()
        } else if k < 5 {
            LABEL1[text_offset..text_offset + 5].to_string()
        } else {
            let off = text_offset - 25; // (k - 5) * 5
            self.menus[self.cmode as usize].labels[off..off + 5].to_string()
        };

        ButtonRender {
            display_slot: j,
            menu_index: self.real_options[j],
            text,
            fg_color,
            bg_color,
        }
    }

    /// Handle a button click at the given display slot (fmain.c:1447-1474).
    /// DEPRECATED: Use handle_mouse_down + handle_mouse_up for click-and-hold behavior.
    pub fn handle_click(&mut self, display_slot: usize) -> MenuAction {
        if display_slot >= 12 {
            return MenuAction::None;
        }
        let hit = self.real_options[display_slot];
        if hit < 0 || hit as u8 >= self.menus[self.cmode as usize].num {
            return MenuAction::None;
        }
        let hit = hit as u8;
        let atype = self.menus[self.cmode as usize].enabled[hit as usize] & TYPE_MASK;
        if atype == TYPE_TOGGLE {
            self.menus[self.cmode as usize].enabled[hit as usize] ^= FLAG_SELECTED;
            self.dispatch_do_option(hit)
        } else if atype == TYPE_IMMEDIATE {
            self.dispatch_do_option(hit)
        } else if atype == TYPE_RADIO {
            self.menus[self.cmode as usize].enabled[hit as usize] |= FLAG_SELECTED;
            self.dispatch_do_option(hit)
        } else if atype == TYPE_TAB && (hit as usize) < 5 {
            if !self.is_paused() {
                self.gomenu(MenuMode::from(hit as usize));
            }
            MenuAction::SwitchMode(self.cmode)
        } else {
            MenuAction::None
        }
    }

    /// Start a mouse press on a menu item. Returns true if the slot is a valid clickable item.
    /// For toggle items, this shows the toggle state visually but doesn't commit it yet.
    pub fn handle_mouse_down(&mut self, display_slot: usize) -> bool {
        if display_slot >= 12 {
            return false;
        }
        let hit = self.real_options[display_slot];
        if hit < 0 || hit as u8 >= self.menus[self.cmode as usize].num {
            return false;
        }
        let hit = hit as u8;
        let atype = self.menus[self.cmode as usize].enabled[hit as usize] & TYPE_MASK;
        // Only track press for actionable items
        if atype == TYPE_TOGGLE || atype == TYPE_IMMEDIATE || atype == TYPE_RADIO || atype == TYPE_TAB {
            // Record where the mouse button was first pressed (held until mouse up)
            self.committed_slot = Some(display_slot);
            self.pressed_slot = Some(display_slot);
            // For toggle items, remember the original state so we can restore on cancel
            if atype == TYPE_TOGGLE {
                self.pressed_was_selected = self.menus[self.cmode as usize].enabled[hit as usize] & FLAG_SELECTED != 0;
                // Preview the toggle state visually
                self.menus[self.cmode as usize].enabled[hit as usize] ^= FLAG_SELECTED;
            }
            true
        } else {
            false
        }
    }

    /// Handle mouse moving while button is held. If moving into the committed slot,
    /// re-activate the press (highlight for non-toggle, restore toggle preview for toggle items).
    /// Returns true if the slot is now active (either re-activated or was already active).
    pub fn handle_mouse_move_while_held(&mut self, display_slot: usize) -> bool {
        // If we're already in the pressed slot, nothing to do
        if self.pressed_slot == Some(display_slot) {
            return true;
        }
        // If we moved back into the committed slot, re-activate
        if self.committed_slot == Some(display_slot) {
            let hit = self.real_options[display_slot];
            if hit >= 0 && (hit as u8) < self.menus[self.cmode as usize].num {
                let hit = hit as u8;
                let atype = self.menus[self.cmode as usize].enabled[hit as usize] & TYPE_MASK;
                if atype == TYPE_TOGGLE {
                    // Re-apply the toggle preview (it was cancelled when we moved out)
                    let currently_selected = self.menus[self.cmode as usize].enabled[hit as usize] & FLAG_SELECTED != 0;
                    if currently_selected != self.pressed_was_selected {
                        self.menus[self.cmode as usize].enabled[hit as usize] ^= FLAG_SELECTED;
                    }
                }
                self.pressed_slot = Some(display_slot);
            }
            return true;
        }
        // Not in the committed slot - cancel current press if any
        self.cancel_press();
        false
    }

    /// End a mouse press. If the mouse is over the same slot that was pressed,
    /// execute the action. Returns the action to dispatch, or None if cancelled/moved out.
    pub fn handle_mouse_up(&mut self, display_slot: usize) -> MenuAction {
        // Clear committed slot - mouse button is no longer held
        self.committed_slot = None;
        if self.pressed_slot != Some(display_slot) {
            // Mouse released over a different slot - cancel the press
            self.cancel_press();
            return MenuAction::None;
        }
        // Valid release over the pressed slot - execute the action
        self.pressed_slot = None;
        let hit = self.real_options[display_slot];
        // hit was validated in handle_mouse_down, but double-check
        if hit < 0 || hit as u8 >= self.menus[self.cmode as usize].num {
            return MenuAction::None;
        }
        let hit = hit as u8;
        let atype = self.menus[self.cmode as usize].enabled[hit as usize] & TYPE_MASK;
        if atype == TYPE_TOGGLE {
            // Toggle state was already flipped in handle_mouse_down as preview,
            // so just dispatch the action
            self.dispatch_do_option(hit)
        } else if atype == TYPE_IMMEDIATE {
            self.dispatch_do_option(hit)
        } else if atype == TYPE_RADIO {
            self.menus[self.cmode as usize].enabled[hit as usize] |= FLAG_SELECTED;
            self.dispatch_do_option(hit)
        } else if atype == TYPE_TAB && (hit as usize) < 5 {
            if !self.is_paused() {
                self.gomenu(MenuMode::from(hit as usize));
            }
            MenuAction::SwitchMode(self.cmode)
        } else {
            MenuAction::None
        }
    }

    /// Cancel the current press (e.g., mouse moved out of the item region).
    /// For toggle items, this restores the original state.
    pub fn cancel_press(&mut self) {
        if let Some(display_slot) = self.pressed_slot {
            // Restore toggle state if this was a toggle item
            let hit = self.real_options[display_slot];
            if hit >= 0 && (hit as u8) < self.menus[self.cmode as usize].num {
                let hit = hit as u8;
                let atype = self.menus[self.cmode as usize].enabled[hit as usize] & TYPE_MASK;
                if atype == TYPE_TOGGLE {
                    // Restore original selected state
                    let currently_selected = self.menus[self.cmode as usize].enabled[hit as usize] & FLAG_SELECTED != 0;
                    if currently_selected != self.pressed_was_selected {
                        self.menus[self.cmode as usize].enabled[hit as usize] ^= FLAG_SELECTED;
                    }
                }
            }
        }
        self.pressed_slot = None;
    }

    /// Check if a display slot is currently being pressed (for visual feedback).
    pub fn is_pressed(&self, display_slot: usize) -> bool {
        self.pressed_slot == Some(display_slot)
    }

    /// Handle a keyboard shortcut (fmain.c:1499-1520).
    pub fn handle_key(&mut self, key: u8) -> MenuAction {
        if self.cmode == MenuMode::Keys {
            if (b'1'..=b'6').contains(&key) {
                return self.dispatch_do_option(key - b'1' + 5);
            }
            self.gomenu(MenuMode::Items);
            return MenuAction::None;
        }
        for &(letter, menu, slot) in LETTER_LIST {
            if letter != key {
                continue;
            }
            // V/X (SaveX actions) only accessible when already in SaveX mode (fmain.c:1510)
            if menu == MenuMode::SaveX && self.cmode != MenuMode::SaveX {
                return MenuAction::None;
            }
            if self.is_paused() && key != b' ' {
                return MenuAction::None;
            }
            self.cmode = menu;
            let hit = slot;
            let atype = self.menus[self.cmode as usize].enabled[hit as usize] & TYPE_MASK;
            if atype == TYPE_TOGGLE {
                self.menus[self.cmode as usize].enabled[hit as usize] ^= FLAG_SELECTED;
            }
            return self.dispatch_do_option(hit);
        }
        MenuAction::None
    }

    /// Generate a MenuAction from cmode + hit index (fmain.c:3830-4408).
    pub fn dispatch_do_option(&mut self, hit: u8) -> MenuAction {
        match (self.cmode, hit) {
            (MenuMode::Items, 5) => MenuAction::Inventory,
            (MenuMode::Items, 6) => MenuAction::Take,
            (MenuMode::Items, 7) => MenuAction::Look,
            (MenuMode::Items, 8) => {
                self.gomenu(MenuMode::Use);
                MenuAction::None
            }
            (MenuMode::Items, 9) => {
                self.gomenu(MenuMode::Give);
                MenuAction::None
            }
            (MenuMode::Magic, 5..=11) => MenuAction::CastSpell(hit - 5),
            (MenuMode::Talk, 5) => MenuAction::Yell,
            (MenuMode::Talk, 6) => MenuAction::Say,
            (MenuMode::Talk, 7) => MenuAction::Ask,
            (MenuMode::Buy, 5..=11) => MenuAction::BuyItem(hit - 5),
            (MenuMode::Game, 5) => MenuAction::TogglePause,
            (MenuMode::Game, 6) => MenuAction::ToggleMusic,
            (MenuMode::Game, 7) => MenuAction::ToggleSound,
            (MenuMode::Game, 8) => {
                self.gomenu(MenuMode::SaveX);
                MenuAction::None
            }
            (MenuMode::Game, 9) => {
                self.gomenu(MenuMode::File);
                MenuAction::None
            }
            (MenuMode::Use, 0..=4) => {
                self.gomenu(MenuMode::Items);
                MenuAction::SetWeapon(hit + 1)
            }
            (MenuMode::Use, 5) => {
                // Lasso: no USE effect; falls through to Items (fmain.c:3466).
                self.gomenu(MenuMode::Items);
                MenuAction::None
            }
            (MenuMode::Use, 6) => {
                self.gomenu(MenuMode::Items);
                MenuAction::SummonTurtle
            }
            (MenuMode::Use, 7) => {
                self.gomenu(MenuMode::Keys);
                MenuAction::None
            }
            (MenuMode::Use, 8) => {
                self.gomenu(MenuMode::Items);
                MenuAction::UseSunstone
            }
            (MenuMode::SaveX, 5) => {
                self.save_pending = true;
                self.gomenu(MenuMode::File);
                MenuAction::None
            }
            (MenuMode::SaveX, 6) => MenuAction::Quit,
            (MenuMode::File, 0..=7) => {
                let slot = hit;
                let action = if self.save_pending {
                    MenuAction::SaveGame(slot)
                } else {
                    MenuAction::LoadGame(slot)
                };
                self.save_pending = false;
                action
            }
            (MenuMode::Keys, 5..=10) => MenuAction::TryKey(hit - 5),
            (MenuMode::Give, 5) => MenuAction::GiveGold,
            (MenuMode::Give, 7) => MenuAction::GiveWrit,
            (MenuMode::Give, 8) => MenuAction::GiveBone,
            _ => MenuAction::None,
        }
    }

    pub fn is_paused(&self) -> bool {
        self.menus[MenuMode::Game as usize].enabled[5] & FLAG_SELECTED != 0
    }

    /// Toggle the pause flag (mirrors what handle_click does for a TYPE_TOGGLE item).
    pub fn toggle_pause(&mut self) {
        self.menus[MenuMode::Game as usize].enabled[5] ^= FLAG_SELECTED;
    }

    pub fn is_music_on(&self) -> bool {
        self.menus[MenuMode::Game as usize].enabled[6] & FLAG_SELECTED != 0
    }

    pub fn is_sound_on(&self) -> bool {
        self.menus[MenuMode::Game as usize].enabled[7] & FLAG_SELECTED != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_initial_state() {
        let ms = MenuState::new();
        assert_eq!(ms.cmode, MenuMode::Items);
        // GAME enabled[6] = 7 → Music ON (bit0 set)
        assert_eq!(ms.menus[MenuMode::Game as usize].enabled[6], 7);
        // GAME enabled[5] = 6 → Pause OFF (bit0 clear)
        assert_eq!(ms.menus[MenuMode::Game as usize].enabled[5], 6);
    }

    #[test]
    fn test_stuff_flag() {
        assert_eq!(MenuState::stuff_flag(&[0], 0), 8);
        assert_eq!(MenuState::stuff_flag(&[1], 0), 10);
        assert_eq!(MenuState::stuff_flag(&[255], 0), 10);
    }

    #[test]
    fn test_set_options_magic() {
        let mut ms = MenuState::new();
        let mut stuff = vec![0u8; 32];
        stuff[9] = 1; // first magic item owned
        ms.set_options(&stuff, 0);
        assert_eq!(ms.menus[MenuMode::Magic as usize].enabled[5], 10);
    }

    #[test]
    fn test_is_paused_initially_false() {
        let ms = MenuState::new();
        assert!(!ms.is_paused());
    }

    #[test]
    fn test_is_paused_after_set() {
        let mut ms = MenuState::new();
        ms.menus[MenuMode::Game as usize].enabled[5] |= FLAG_SELECTED;
        assert!(ms.is_paused());
    }

    #[test]
    fn test_is_music_on_initially_true() {
        let ms = MenuState::new();
        // enabled[6] = 7, 7 & 1 == 1
        assert!(ms.is_music_on());
    }

    #[test]
    fn test_gomenu_switches_mode() {
        let mut ms = MenuState::new();
        ms.gomenu(MenuMode::Talk);
        assert_eq!(ms.cmode, MenuMode::Talk);
    }

    #[test]
    fn test_gomenu_refuses_when_paused() {
        let mut ms = MenuState::new();
        ms.menus[MenuMode::Game as usize].enabled[5] |= FLAG_SELECTED; // pause
        ms.gomenu(MenuMode::Talk);
        assert_eq!(ms.cmode, MenuMode::Items); // unchanged
    }

    #[test]
    fn test_print_options_items_count() {
        let mut ms = MenuState::new();
        let renders = ms.print_options();
        assert_eq!(renders.len(), 12);
        // ITEMS: enabled = [3,2,2,2,2,10,10,10,10,10,0,0] → 10 displayed (num=10, index 10,11 never reached)
        let visible = renders.iter().filter(|r| r.menu_index >= 0).count();
        assert_eq!(visible, 10);
        assert_eq!(ms.real_options[0], 0);
    }

    #[test]
    fn test_propt_tab_bg_color() {
        let mut ms = MenuState::new();
        ms.print_options(); // populate real_options
                            // Slot 0 → menu_index 0 (tab, k < 5) → bg_color = 4
        let btn = ms.propt(0, false, false);
        assert_eq!(btn.bg_color, 4);
    }

    #[test]
    fn test_propt_game_music_bg_color() {
        let mut ms = MenuState::new();
        ms.gomenu(MenuMode::Game);
        ms.print_options(); // populate real_options
                            // Find the Music button (menu_index 6)
        let music_slot = ms
            .real_options
            .iter()
            .position(|&x| x == 6)
            .expect("Music button not found");
        let btn = ms.propt(music_slot, true, false);
        // cmode == Game, k=6 >= 5, not Keys/SaveX → bg_color = menus[Game].color = 2
        assert_eq!(btn.bg_color, 2);
    }

    #[test]
    fn test_file_menu_slot_labels() {
        // T2-SAVE-SLOT-UI: verify FILE menu presents slots A-H (SPEC §24.1)
        let mut ms = MenuState::new();
        ms.gomenu(MenuMode::File);
        let renders = ms.print_options();

        // FILE menu should show 8 slots (A-H) from LABELB
        let visible_labels: Vec<String> = renders
            .iter()
            .filter(|r| r.menu_index >= 0)
            .map(|r| r.text.trim().to_string())
            .collect();

        assert_eq!(visible_labels.len(), 8, "FILE menu should show 8 slots");
        assert_eq!(visible_labels[0], "A");
        assert_eq!(visible_labels[1], "B");
        assert_eq!(visible_labels[2], "C");
        assert_eq!(visible_labels[3], "D");
        assert_eq!(visible_labels[4], "E");
        assert_eq!(visible_labels[5], "F");
        assert_eq!(visible_labels[6], "G");
        assert_eq!(visible_labels[7], "H");
    }

    #[test]
    fn test_keys_mode_number_dispatches_key_action() {
        let mut ms = MenuState::new();
        ms.gomenu(MenuMode::Keys);
        let action = ms.handle_key(b'1');
        assert!(matches!(action, MenuAction::TryKey(0)));
        assert_eq!(ms.cmode, MenuMode::Keys);
    }

    /// Clicking a weapon slot in the Use menu must produce SetWeapon(hit + 1)
    /// so that weapon codes 1..=5 map to Dirk..Wand (fmain.c:3450: weapon = hit + 1).
    #[test]
    fn test_use_weapon_dispatch_is_hit_plus_one() {
        let mut ms = MenuState::new();
        ms.gomenu(MenuMode::Use);
        // Enable all weapon slots so dispatch_do_option doesn't hit a "don't have one" guard.
        for i in 0..5 {
            ms.menus[MenuMode::Use as usize].enabled[i] = 1;
        }
        for hit in 0u8..=4 {
            ms.gomenu(MenuMode::Use);
            let action = ms.dispatch_do_option(hit);
            assert_eq!(
                action,
                MenuAction::SetWeapon(hit + 1),
                "Use hit {} should emit SetWeapon({}) but got {:?}",
                hit, hit + 1, action
            );
            assert_eq!(ms.cmode, MenuMode::Items,
                "Use hit {} should return to Items menu", hit);
        }
    }

    /// Key slot (hit 7) in the Use menu must open the Keys submenu, not return to Items.
    #[test]
    fn test_use_key_slot_opens_keys_menu() {
        let mut ms = MenuState::new();
        ms.gomenu(MenuMode::Use);
        ms.dispatch_do_option(7);
        assert_eq!(ms.cmode, MenuMode::Keys);
    }

    #[test]
    fn test_use_menu_labels_and_no_book_entry() {
        assert_eq!(LABEL7.len() % 5, 0, "LABEL7 must be 5-char chunks");
        let chunks: Vec<&str> = (0..LABEL7.len())
            .step_by(5)
            .map(|i| &LABEL7[i..i + 5])
            .collect();
        assert_eq!(chunks[0], "Dirk ");
        assert_eq!(chunks[1], "Mace ");
        assert_eq!(chunks[2], "Sword");
        assert_eq!(chunks[3], "Bow  ");
        assert_eq!(chunks[4], "Wand ");
        assert_eq!(chunks[5], "Lasso");
        assert_eq!(chunks[6], "Shell");
        assert_eq!(chunks[7], "Key  ");
        assert_eq!(chunks[8], "Sun  ");
        assert!(chunks[9].trim().is_empty(), "10th USE slot should be blank");
        assert!(!LABEL7.contains("Book"));
    }

    #[test]
    fn test_menu_colors_for_use_file_keys_savex() {
        let mut ms = MenuState::new();

        ms.gomenu(MenuMode::Use);
        ms.print_options();
        let dirk_slot = ms.real_options.iter().position(|&x| x == 0).unwrap();
        assert_eq!(ms.propt(dirk_slot, false, false).bg_color, 14);

        ms.gomenu(MenuMode::File);
        ms.print_options();
        let file_slot = ms.real_options.iter().position(|&x| x == 0).unwrap();
        assert_eq!(ms.propt(file_slot, false, false).bg_color, 13);

        ms.gomenu(MenuMode::Keys);
        ms.print_options();
        let gold_slot = ms.real_options.iter().position(|&x| x == 5).unwrap();
        assert_eq!(ms.propt(gold_slot, false, false).bg_color, KEYCOLORS[0]);

        ms.gomenu(MenuMode::SaveX);
        ms.print_options();
        let save_slot = ms.real_options.iter().position(|&x| x == 5).unwrap();
        assert_eq!(ms.propt(save_slot, false, false).bg_color, 5);
    }
}
