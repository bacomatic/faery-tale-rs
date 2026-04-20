//! Debug console: TUI-based developer console using ratatui + crossterm.
//!
//! When `--debug` is passed, the terminal enters alternate-screen mode and
//! shows a three-region layout:
//!
//!  ┌─ Status ────────────────────────────────────────────────────────────┐
//!  │ FPS  Day/Hour/Phase  Hero @(x,y)  Region  Brother  God  Paused      │
//!  ├─ Log ───────────────────────────────────────────────────────────────┤
//!  │ scrollable output — game logs + command output                       │
//!  │                                                                      │
//!  ├─────────────────────────────────────────────────────────────────────┤
//!  │ > command prompt                                                     │
//!  └─────────────────────────────────────────────────────────────────────┘
//!
//! Commands use a `/name [args]` syntax.  Type `/help` for the full list.

use std::io::{self, Stdout};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};

use crate::game::actor::{Actor, ActorKind, ActorState, Goal, Tactic};
use crate::game::debug_command::{
    BrotherId, DebugCommand, GodModeFlags, MagicEffect, StatId,
};
use crate::game::debug_log::{DebugLogEntry, LogCategory};
use crate::game::game_state::DayPhase;

// ── Status snapshot ──────────────────────────────────────────────────────────

/// Lightweight game-state snapshot for the status header.
/// Built by main.rs each frame when the console is active.
#[derive(Debug, Clone, Default)]
pub struct DebugSnapshot {
    pub fps: f64,
    pub game_day: u32,
    pub game_hour: u32,
    pub game_minute: u32,
    pub day_phase: DayPhase,
    pub daynight: u16,
    pub lightlevel: u16,
    pub game_ticks: u64,
    pub paused: bool,
    pub is_paused: bool,
    pub scene_name: Option<String>,
    pub hero_x: u16,
    pub hero_y: u16,
    pub brother: u8,
    pub region_num: u8,
    pub vitality: i16,
    pub hunger: i16,
    pub fatigue: i16,
    pub god_mode_flags: u8,
    pub time_held: bool,
    pub song_group_count: usize,
    pub current_song_group: Option<usize>,
    pub cave_mode: bool,

    // Geography
    pub current_zone_idx: Option<usize>,
    pub current_zone_label: Option<String>,

    // VFX state
    pub vfx_jewel_active: bool,
    pub vfx_light_sticky: bool,
    pub vfx_secret_active: bool,
    pub vfx_witch_active: bool,
    pub vfx_teleport_active: bool,
    pub vfx_palette_xfade: bool,

    // Time-of-day period derived from day_phase.
    pub time_period: String,

    // Quest state (for `/quest` command — DEBUG_SPEC §DebugSnapshot Data Model).
    pub princess_captive: bool,
    pub princess_rescues: u16,
    pub statues_collected: u8,
    pub has_writ: bool,
    pub has_talisman: bool,

    // Encounter state (for `/enc` commands).
    pub encounter_number: u8,
    pub encounter_type: u8,
    pub active_enemy_count: u8,

    // Full inventory array for `/inventory` command.
    pub stuff: Vec<u8>,
    // cheat1 flag (DEBUG_SPEC §Mutation /cheat).
    pub cheat1: bool,
    // Wealth (gold) — used by /give/take and /inventory dumps.
    pub wealth: u16,
    // Brave — used to compute heal cap `15 + brave/4` for /heal.
    pub brave: u16,

    // Actor slots (for `/actors` command and `/watch` feature). Up to 20 active slots.
    pub actors: Vec<ActorSnapshot>,

    // ── Hero top-row extras (DBG-LAYOUT-01) ────────────────────────────
    /// `15 + brave/4` — current cap for hero HP.
    pub max_vitality: i16,
    pub luck: i16,
    pub kind: i16,
    /// Weapon slot currently equipped on the hero (`actors[0].weapon`).
    pub hero_weapon: u8,
    /// Human-readable name of the hero's weapon (Dirk/Mace/Sword/Bow/Wand/…).
    pub hero_weapon_name: String,
    /// Hero ActorState encoded (see actor_state_u8).
    pub hero_state_u8: u8,
    /// Human-readable hero state (WALKING, FIGHT, …).
    pub hero_state_name: String,
    /// Hero facing direction 0..=7 (0=N, clockwise).
    pub hero_facing: u8,
    /// Hero environ value (−3..=2); see SPEC §9.5.
    pub hero_environ: i8,
    /// Carrier index currently ridden (0 none / 1 raft / 2 turtle / 3 swan / 4 dragon).
    pub active_carrier: i16,
    /// Active carrier human-readable label.
    pub active_carrier_name: String,
    /// Light-timer tick count (Green Jewel spell).
    pub jewel_timer: u16,
    /// Totem (Bird Totem / map) active indicator tick count.
    pub totem_timer: u16,
    /// Freeze (Gold Ring) timer tick count.
    pub freeze_timer: u16,

    // ── Actor Watch (DBG-LAYOUT-06) ────────────────────────────────────
    /// Raft (slot 1) world coords when active+visible, otherwise `None`.
    pub raft_xy: Option<(u16, u16)>,
    /// Count of active projectiles (missile list), spec §Actor Watch.
    pub missile_count: u8,
    /// Count of visible ground-item actors (slots 7..=19).
    pub item_count: u8,
}

/// Per-actor snapshot for the Actor Watch panel and `/actors` dump.
/// See DEBUG_SPECIFICATION.md §DebugSnapshot Data Model for field semantics.
#[derive(Debug, Clone, Default)]
pub struct ActorSnapshot {
    pub slot: u8,
    pub actor_type: u8,
    pub state: u8,
    pub facing: u8,
    pub abs_x: u16,
    pub abs_y: u16,
    pub vitality: i8,
    pub weapon: u8,
    pub race: u8,
    pub goal: u8,
    pub tactic: u8,
    pub environ: i8,
    pub visible: bool,
}

impl ActorSnapshot {
    pub fn from_actor(slot: u8, a: &Actor) -> Self {
        Self {
            slot,
            actor_type: actor_kind_u8(&a.kind),
            state: actor_state_u8(&a.state),
            facing: a.facing,
            abs_x: a.abs_x,
            abs_y: a.abs_y,
            vitality: a.vitality.clamp(i8::MIN as i16, i8::MAX as i16) as i8,
            weapon: a.weapon,
            race: a.race,
            goal: goal_u8(&a.goal),
            tactic: tactic_u8(&a.tactic),
            environ: a.environ,
            visible: true,
        }
    }
}

fn actor_kind_u8(k: &ActorKind) -> u8 {
    match k {
        ActorKind::Player => 0,
        ActorKind::Enemy => 1,
        ActorKind::Object => 2,
        ActorKind::Raft => 3,
        ActorKind::SetFig => 4,
        ActorKind::Carrier => 5,
        ActorKind::Dragon => 6,
    }
}

fn actor_state_u8(s: &ActorState) -> u8 {
    match s {
        ActorState::Still => 0,
        ActorState::Walking => 1,
        ActorState::Fighting(_) => 2,
        ActorState::Dying => 3,
        ActorState::Dead => 4,
        ActorState::Shooting(_) => 5,
        ActorState::Sinking => 6,
        ActorState::Falling => 7,
        ActorState::Sleeping => 8,
    }
}

fn goal_u8(g: &Goal) -> u8 {
    match g {
        Goal::User => 0,
        Goal::Attack1 => 1,
        Goal::Attack2 => 2,
        Goal::Archer1 => 3,
        Goal::Archer2 => 4,
        Goal::Flee => 5,
        Goal::Follower => 6,
        Goal::Leader => 7,
        Goal::Stand => 8,
        Goal::Guard => 9,
        Goal::Confused => 10,
        Goal::None => 255,
    }
}

fn tactic_u8(t: &Tactic) -> u8 {
    match t {
        Tactic::Pursue => 0,
        Tactic::Shoot => 1,
        Tactic::Random => 2,
        Tactic::BumbleSeek => 3,
        Tactic::Backup => 4,
        Tactic::Follow => 5,
        Tactic::Evade => 6,
        Tactic::EggSeek => 7,
        Tactic::Frust => 8,
        Tactic::None => 255,
    }
}

/// Human-readable label for a `DayPhase` variant — used to populate
/// `DebugSnapshot::time_period` (spec §DebugSnapshot Data Model).
pub fn day_phase_label(phase: DayPhase) -> String {
    match phase {
        DayPhase::Midnight => "Night".to_string(),
        DayPhase::Morning => "Morning".to_string(),
        DayPhase::Midday => "Midday".to_string(),
        DayPhase::Evening => "Evening".to_string(),
    }
}

/// Hero weapon slot → short display name (DBG-LAYOUT-01).
pub fn weapon_short_name(weapon: u8) -> &'static str {
    match weapon {
        0 => "—",
        1 => "Dirk",
        2 => "Mace",
        3 => "Sword",
        4 => "Bow",
        5 => "Wand",
        _ => "?",
    }
}

/// ActorState discriminant → compact display name (DBG-LAYOUT-01).
pub fn actor_state_name(state: u8) -> &'static str {
    match state {
        0 => "STILL",
        1 => "WALK",
        2 => "FIGHT",
        3 => "DYING",
        4 => "DEAD",
        5 => "SHOOT",
        6 => "SINK",
        7 => "FALL",
        8 => "SLEEP",
        _ => "?",
    }
}

/// Facing direction 0..=7 → 8-point compass label.
pub fn facing_name(facing: u8) -> &'static str {
    match facing & 7 {
        0 => "N",
        1 => "NE",
        2 => "E",
        3 => "SE",
        4 => "S",
        5 => "SW",
        6 => "W",
        7 => "NW",
        _ => "?",
    }
}

/// Human-readable environ label (SPEC §9.5).
pub fn environ_label(env: i8) -> &'static str {
    match env {
        -3 => "reverse",
        -2 => "swamp",
        -1 => "slippery",
        0 => "normal",
        1 => "wading",
        2 => "fire",
        _ => "?",
    }
}

/// Carrier slot → human-readable label.
pub fn carrier_name(carrier: i16) -> &'static str {
    match carrier {
        0 => "none",
        1 => "raft",
        2 => "turtle",
        3 => "swan",
        4 => "dragon",
        _ => "?",
    }
}

/// Short label for `ActorKind` codes (matches `actor_kind_u8`).
pub fn actor_kind_name(kind: u8) -> &'static str {
    match kind {
        0 => "PLAYER",
        1 => "ENEMY",
        2 => "OBJECT",
        3 => "RAFT",
        4 => "SETFIG",
        5 => "CARRIER",
        6 => "DRAGON",
        _ => "?",
    }
}

/// Short label for `Goal` codes (matches `goal_u8`).
pub fn goal_name(g: u8) -> &'static str {
    match g {
        0 => "USER",
        1 => "ATK1",
        2 => "ATK2",
        3 => "ARC1",
        4 => "ARC2",
        5 => "FLEE",
        6 => "FOLL",
        7 => "LEAD",
        8 => "STAND",
        9 => "GUARD",
        10 => "CONF",
        255 => "—",
        _ => "?",
    }
}

/// Short label for `Tactic` codes (matches `tactic_u8`).
pub fn tactic_name(t: u8) -> &'static str {
    match t {
        0 => "PURSUE",
        1 => "SHOOT",
        2 => "RANDOM",
        3 => "BUMBLE",
        4 => "BACKUP",
        5 => "FOLLOW",
        6 => "EVADE",
        7 => "EGG",
        8 => "FRUST",
        255 => "—",
        _ => "?",
    }
}

/// Short race/NPC-type label. Tries NPC type byte first, then known race constants; otherwise
/// falls back to a hex representation so the panel remains informative.
pub fn race_label(race: u8) -> String {
    use crate::game::npc::{
        NPC_TYPE_DKNIGHT, NPC_TYPE_DRAGON, NPC_TYPE_GHOST, NPC_TYPE_HORSE, NPC_TYPE_HUMAN,
        NPC_TYPE_LORAII, NPC_TYPE_NECROMANCER, NPC_TYPE_ORC, NPC_TYPE_RAFT, NPC_TYPE_SKELETON,
        NPC_TYPE_SNAKE, NPC_TYPE_SPIDER, NPC_TYPE_SWAN, NPC_TYPE_WRAITH, RACE_BEGGAR, RACE_GHOST,
        RACE_NECROMANCER, RACE_SHOPKEEPER, RACE_SPECTRE, RACE_WITCH, RACE_WOODCUTTER,
    };
    match race {
        0 => "Normal".into(),
        x if x == NPC_TYPE_HUMAN => "Human".into(),
        x if x == NPC_TYPE_SWAN => "Swan".into(),
        x if x == NPC_TYPE_HORSE => "Horse".into(),
        x if x == NPC_TYPE_DRAGON => "Dragon".into(),
        x if x == NPC_TYPE_GHOST => "Ghost".into(),
        x if x == NPC_TYPE_ORC => "Orc".into(),
        x if x == NPC_TYPE_WRAITH => "Wraith".into(),
        x if x == NPC_TYPE_SKELETON => "Skel".into(),
        x if x == NPC_TYPE_RAFT => "Raft".into(),
        x if x == NPC_TYPE_SNAKE => "Snake".into(),
        x if x == NPC_TYPE_SPIDER => "Spider".into(),
        x if x == NPC_TYPE_DKNIGHT => "DKnight".into(),
        x if x == NPC_TYPE_LORAII => "Loraii".into(),
        x if x == NPC_TYPE_NECROMANCER => "Necro".into(),
        x if x == RACE_WOODCUTTER => "Woodctr".into(),
        x if x == RACE_SHOPKEEPER => "Shop".into(),
        x if x == RACE_BEGGAR => "Beggar".into(),
        x if x == RACE_WITCH => "Witch".into(),
        x if x == RACE_SPECTRE => "Spectre".into(),
        x if x == RACE_GHOST => "Ghost".into(),
        x if x == RACE_NECROMANCER => "Necro".into(),
        _ => format!("r:0x{:02X}", race),
    }
}

// ── DebugConsole ─────────────────────────────────────────────────────────────

const MAX_LOG_LINES: usize = 1000;

pub struct DebugConsole {
    terminal: Terminal<CrosstermBackend<Stdout>>,

    // Log output
    log_entries: Vec<DebugLogEntry>,
    /// If the user hasn't manually scrolled, we auto-scroll to the bottom.
    auto_scroll: bool,
    /// Scroll offset from the bottom (0 = show tail).
    scroll_from_bottom: usize,

    // Command prompt
    input_buffer: String,
    command_history: Vec<String>,
    history_index: Option<usize>,

    // Queued items for the main loop to consume
    pending_commands: Vec<DebugCommand>,
    song_group_requested: Option<usize>,
    stop_requested: bool,
    cave_mode_requested: Option<bool>,
    quit_requested: bool,
    /// Pause/resume request: Some(true) = pause, Some(false) = resume, None = none.
    pause_request: Option<bool>,
    /// Step request: number of ticks to advance while paused. Consumed by the clock.
    step_request: u32,

    /// Actor Watch panel display mode (DBG-LAYOUT-07). false = collapsed (default).
    watch_expanded: bool,

    /// Active log categories used to filter the log panel render (DBG-LOG-05).
    /// Seeded from `LogCategory::default_enabled()` per DEBUG_SPEC §Log Categories.
    active_categories: std::collections::HashSet<LogCategory>,

    // Latest status snapshot
    status: DebugSnapshot,
}

impl DebugConsole {
    pub fn new() -> Result<Self, io::Error> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        // Install a panic hook that restores the terminal before printing the
        // panic message.  Without this, panics are silently swallowed by the
        // alternate screen and the process exits with code 101 and no output.
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen);
            default_hook(info);
        }));
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            log_entries: Vec::new(),
            auto_scroll: true,
            scroll_from_bottom: 0,
            input_buffer: String::new(),
            command_history: Vec::new(),
            history_index: None,
            pending_commands: Vec::new(),
            song_group_requested: None,
            stop_requested: false,
            cave_mode_requested: None,
            quit_requested: false,
            pause_request: None,
            step_request: 0,
            watch_expanded: false,
            active_categories: LogCategory::ALL
                .iter()
                .copied()
                .filter(|c| c.default_enabled())
                .collect(),
            status: DebugSnapshot::default(),
        })
    }

    /// Update the status snapshot shown in the header.
    pub fn update_status(&mut self, status: DebugSnapshot) {
        self.status = status;
    }

    /// Push a categorized entry to the scrolling log. Primary API for gameplay-emitted logs.
    ///
    /// Splits embedded newlines into multiple entries (preserving category/timestamp),
    /// enforces `MAX_LOG_LINES`, and preserves auto-scroll behavior.
    pub fn log_entry(&mut self, entry: DebugLogEntry) {
        for line in entry.text.split('\n') {
            self.log_entries.push(DebugLogEntry {
                category: entry.category,
                timestamp_ticks: entry.timestamp_ticks,
                text: line.to_owned(),
            });
        }
        if self.log_entries.len() > MAX_LOG_LINES {
            let overflow = self.log_entries.len() - MAX_LOG_LINES;
            self.log_entries.drain(..overflow);
        }
        if self.auto_scroll {
            self.scroll_from_bottom = 0;
        }
    }

    /// Alias for [`log_entry`]; the intent is that the main loop drains
    /// gameplay-emitted entries into the console via this method.
    pub fn ingest(&mut self, entry: DebugLogEntry) {
        self.log_entry(entry);
    }

    /// Push a plain-text message to the scrolling log. Compatibility wrapper
    /// for call sites that produce user-facing command feedback; messages are
    /// tagged as [`LogCategory::General`] with `timestamp_ticks = 0`.
    pub fn log(&mut self, msg: impl Into<String>) {
        self.log_entry(DebugLogEntry {
            category: LogCategory::General,
            timestamp_ticks: 0,
            text: msg.into(),
        });
    }

    /// Drain pending debug commands for the main loop to apply.
    pub fn drain_commands(&mut self) -> Vec<DebugCommand> {
        self.pending_commands.drain(..).collect()
    }

    /// Returns and clears any queued pause/resume request.
    /// `Some(true)` = pause, `Some(false)` = resume, `None` = no change.
    pub fn take_pause_request(&mut self) -> Option<bool> {
        self.pause_request.take()
    }

    /// Returns and clears the queued step budget (ticks to advance while paused).
    pub fn take_step_request(&mut self) -> u32 {
        let n = self.step_request;
        self.step_request = 0;
        n
    }

    /// Returns and clears any song group play request.
    pub fn take_song_request(&mut self) -> Option<usize> {
        self.song_group_requested.take()
    }

    /// Returns and clears any stop-music request.
    pub fn take_stop_request(&mut self) -> bool {
        let v = self.stop_requested;
        self.stop_requested = false;
        v
    }

    /// Returns and clears any cave-mode toggle request.
    pub fn take_cave_mode_request(&mut self) -> Option<bool> {
        self.cave_mode_requested.take()
    }

    /// Returns true if the user requested quit via Ctrl+C / Ctrl+Q in the console.
    pub fn take_quit_request(&mut self) -> bool {
        let v = self.quit_requested;
        self.quit_requested = false;
        v
    }

    // ── Input polling ─────────────────────────────────────────────────────────

    /// Non-blocking input poll. Returns true if an event was processed.
    /// Call once per main-loop iteration.
    pub fn poll_input(&mut self) -> bool {
        if !event::poll(std::time::Duration::ZERO).unwrap_or(false) {
            return false;
        }
        let Ok(ev) = event::read() else { return false };

        match ev {
            Event::Key(ke) if ke.kind == KeyEventKind::Press => {
                match ke.code {
                    // Quit
                    KeyCode::Char('c') if ke.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.quit_requested = true;
                    }
                    KeyCode::Char('q') if ke.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.quit_requested = true;
                    }

                    // Toggle pause/resume (Ctrl+P — DEBUG_SPEC §Keyboard Shortcuts)
                    KeyCode::Char('p') if ke.modifiers.contains(KeyModifiers::CONTROL) => {
                        if self.status.is_paused {
                            self.pause_request = Some(false);
                            self.log("Game resumed (Ctrl+P).");
                        } else {
                            self.pause_request = Some(true);
                            self.log("Game paused (Ctrl+P). /resume to continue, /step [n] to advance frames.");
                        }
                    }

                    // Toggle actor watch expanded/collapsed (Ctrl+W — DEBUG_SPEC §Keyboard Shortcuts)
                    KeyCode::Char('w') if ke.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.watch_expanded = !self.watch_expanded;
                    }

                    // Submit command
                    KeyCode::Enter => {
                        let raw = self.input_buffer.trim().to_owned();
                        self.input_buffer.clear();
                        self.history_index = None;
                        if !raw.is_empty() {
                            // Add to history (avoid duplicate of last entry)
                            if self.command_history.last().map(|s| s.as_str()) != Some(&raw) {
                                self.command_history.push(raw.clone());
                            }
                            self.log(format!("> {}", raw));
                            self.execute_command(&raw);
                        }
                        // Return to bottom on enter
                        self.auto_scroll = true;
                        self.scroll_from_bottom = 0;
                    }

                    // Backspace
                    KeyCode::Backspace => {
                        self.input_buffer.pop();
                        self.history_index = None;
                    }

                    // History: Up
                    KeyCode::Up => {
                        let len = self.command_history.len();
                        if len == 0 { return true; }
                        let idx = match self.history_index {
                            None => len - 1,
                            Some(i) => i.saturating_sub(1),
                        };
                        self.history_index = Some(idx);
                        self.input_buffer = self.command_history[idx].clone();
                    }

                    // History: Down
                    KeyCode::Down => {
                        match self.history_index {
                            None => {}
                            Some(i) => {
                                if i + 1 < self.command_history.len() {
                                    let next = i + 1;
                                    self.history_index = Some(next);
                                    self.input_buffer = self.command_history[next].clone();
                                } else {
                                    self.history_index = None;
                                    self.input_buffer.clear();
                                }
                            }
                        }
                    }

                    // Log scrolling
                    KeyCode::PageUp => {
                        self.auto_scroll = false;
                        self.scroll_from_bottom = self.scroll_from_bottom.saturating_add(10);
                    }
                    KeyCode::PageDown => {
                        if self.scroll_from_bottom <= 10 {
                            self.scroll_from_bottom = 0;
                            self.auto_scroll = true;
                        } else {
                            self.scroll_from_bottom -= 10;
                        }
                    }
                    KeyCode::Home => {
                        self.auto_scroll = false;
                        self.scroll_from_bottom = self.log_entries.len().saturating_sub(1);
                    }
                    KeyCode::End => {
                        self.scroll_from_bottom = 0;
                        self.auto_scroll = true;
                    }

                    // Printable characters
                    KeyCode::Char(c) => {
                        self.input_buffer.push(c);
                        self.history_index = None;
                    }

                    _ => {}
                }
            }
            _ => {}
        }
        true
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    pub fn render(&mut self) {
        // DBG-LOG-05: filter log by active categories before rendering.
        let filtered_entries: Vec<&DebugLogEntry> =
            filter_log_entries(&self.log_entries, &self.active_categories);
        let scroll_from_bottom = self.scroll_from_bottom;
        let input = format!("> {}", self.input_buffer);
        let status = &self.status;

        let _ = self.terminal.draw(|f| {
            let area = f.area();

            // Layout: status header (6) | actor-watch (1 collapsed / 6 expanded) | log (fills) | prompt (3)
            let watch_height: u16 = if self.watch_expanded { 6 } else { 1 };
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(6),
                    Constraint::Length(watch_height),
                    Constraint::Min(3),
                    Constraint::Length(3),
                ])
                .split(area);

            // Top row: three equal panels per DEBUG_SPEC §Top Row Panel Contents.
            let status_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(34),
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                ])
                .split(chunks[0]);

            // ── Hero Stats (left) ──────────────────────────────────────────
            let brother_name = match status.brother {
                1 => "Julian",
                2 => "Phillip",
                3 => "Kevin",
                _ => "?",
            };
            let god_str = build_god_str(status.god_mode_flags);
            let hero_stats_text = vec![
                Line::from(vec![
                    Span::styled(brother_name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw("  "),
                    styled_label("HP: "),
                    Span::raw(format!("{}/{}", status.vitality, status.max_vitality)),
                ]),
                Line::from(vec![
                    styled_label("B:"),
                    Span::raw(format!("{} ", status.brave)),
                    styled_label("L:"),
                    Span::raw(format!("{} ", status.luck)),
                    styled_label("K:"),
                    Span::raw(format!("{}  ", status.kind)),
                    styled_label("W:"),
                    Span::raw(format!("{}", status.wealth)),
                ]),
                Line::from(vec![
                    styled_label("Hgr:"),
                    Span::raw(format!("{} ", status.hunger)),
                    styled_label("Fat:"),
                    Span::raw(format!("{}  ", status.fatigue)),
                    styled_label("Wpn:"),
                    Span::raw(status.hero_weapon_name.clone()),
                ]),
                Line::from(vec![
                    styled_label("State: "),
                    Span::raw(format!("{}  ", status.hero_state_name)),
                    styled_label("F:"),
                    Span::raw(facing_name(status.hero_facing)),
                    if !god_str.is_empty() {
                        Span::styled(format!("  God:{}", god_str), Style::default().fg(Color::Yellow))
                    } else {
                        Span::raw("")
                    },
                ]),
            ];
            let hero_widget = Paragraph::new(hero_stats_text)
                .block(Block::default().borders(Borders::ALL).title(" Hero Stats "));
            f.render_widget(hero_widget, status_chunks[0]);

            // ── Geography (center) ─────────────────────────────────────────
            let zone_str = match (status.current_zone_idx, &status.current_zone_label) {
                (Some(idx), Some(label)) => format!("{} ({})", idx, label),
                (Some(idx), None) => format!("{}", idx),
                _ => "—".to_string(),
            };
            let env_str = format!("{} ({})", status.hero_environ, environ_label(status.hero_environ));
            let geo_text = vec![
                Line::from(vec![
                    styled_label("Pos: "),
                    Span::raw(format!("{}, {}", status.hero_x, status.hero_y)),
                ]),
                Line::from(vec![
                    styled_label("Rgn: "),
                    Span::raw(format!("{}   ", status.region_num)),
                    styled_label("Ext: "),
                    Span::raw(zone_str),
                ]),
                Line::from(vec![
                    styled_label("Env: "),
                    Span::raw(env_str),
                ]),
                Line::from(vec![
                    styled_label("Carrier: "),
                    Span::raw(status.active_carrier_name.clone()),
                ]),
            ];
            let geo_widget = Paragraph::new(geo_text)
                .block(Block::default().borders(Borders::ALL).title(" Geography "));
            f.render_widget(geo_widget, status_chunks[1]);

            // ── Visual Effects (right) ─────────────────────────────────────
            let vfx_text = vec![
                Line::from(vec![
                    styled_label("Time: "),
                    Span::raw(format!("{}  ", status.daynight)),
                    Span::styled(
                        status.time_period.clone(),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(vec![
                    styled_label("Light: "),
                    Span::raw(format!("{}", status.lightlevel)),
                    if status.vfx_light_sticky {
                        Span::styled(" *sticky", Style::default().fg(Color::Yellow))
                    } else {
                        Span::raw("")
                    },
                ]),
                Line::from(vec![
                    styled_label("Jewel:"),
                    Span::raw(format!("{} ", status.jewel_timer)),
                    styled_label("Totem:"),
                    Span::raw(format!("{} ", status.totem_timer)),
                    styled_label("Frz:"),
                    Span::raw(format!("{}", status.freeze_timer)),
                ]),
                Line::from(vec![
                    if status.is_paused {
                        Span::styled(
                            "[PAUSED]",
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::raw("")
                    },
                    if status.vfx_witch_active {
                        Span::styled(" Witch", Style::default().fg(Color::Magenta))
                    } else {
                        Span::raw("")
                    },
                    if status.vfx_teleport_active {
                        Span::styled(" TP", Style::default().fg(Color::Green))
                    } else {
                        Span::raw("")
                    },
                    if status.vfx_secret_active {
                        Span::styled(" Secret", Style::default().fg(Color::Green))
                    } else {
                        Span::raw("")
                    },
                ]),
            ];
            let vfx_widget = Paragraph::new(vfx_text)
                .block(Block::default().borders(Borders::ALL).title(" Visual Effects "));
            f.render_widget(vfx_widget, status_chunks[2]);

            // ── Actor Watch (DBG-LAYOUT-06 collapsed / DBG-LAYOUT-07 expanded) ─
            let raft_str = match status.raft_xy {
                Some((x, y)) => format!("Raft:({},{})", x, y),
                None => "Raft:—".to_string(),
            };
            let indicator = if self.watch_expanded { "[▼]" } else { "[▶]" };
            let watch_title = format!(
                " Actors {}  {}  Msls:{} Items:{} ",
                indicator, raft_str, status.missile_count, status.item_count,
            );
            if self.watch_expanded {
                // One row per slot 2..=6 (always rendered; empty slots show `#N —`).
                let mut rows: Vec<Line> = Vec::with_capacity(5);
                for slot in 2u8..=6 {
                    let entry = status.actors.iter().find(|a| a.slot == slot);
                    let line = match entry {
                        Some(a) if a.visible && a.actor_type != 0 => {
                            let kind = actor_kind_name(a.actor_type);
                            let race = race_label(a.race);
                            let state = actor_state_name(a.state);
                            if a.actor_type == 4 {
                                // SETFIG: no goal/tactic
                                format!(
                                    "#{} {} {} {} HP:{} ({},{})",
                                    slot, kind, race, state, a.vitality, a.abs_x, a.abs_y,
                                )
                            } else {
                                format!(
                                    "#{} {} {} {} HP:{} goal:{} tac:{} ({},{})",
                                    slot,
                                    kind,
                                    race,
                                    state,
                                    a.vitality,
                                    goal_name(a.goal),
                                    tactic_name(a.tactic),
                                    a.abs_x,
                                    a.abs_y,
                                )
                            }
                        }
                        _ => format!("#{} —", slot),
                    };
                    rows.push(Line::raw(line));
                }
                let watch_widget = Paragraph::new(rows).block(
                    Block::default()
                        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                        .title(watch_title),
                );
                f.render_widget(watch_widget, chunks[1]);
            } else {
                let watch_widget = Paragraph::new("")
                    .block(Block::default().borders(Borders::TOP).title(watch_title));
                f.render_widget(watch_widget, chunks[1]);
            }

            // ── Log ───────────────────────────────────────────────────────
            let log_height = chunks[2].height.saturating_sub(2) as usize; // subtract borders
            let total = filtered_entries.len();
            // Compute scroll offset (from top) for ratatui's .scroll((top, 0))
            let top_offset = if total <= log_height {
                0
            } else {
                let bottom_top = total - log_height; // scroll to show tail
                bottom_top.saturating_sub(scroll_from_bottom)
            };

            let log_text: Vec<Line> = filtered_entries
                .iter()
                .map(|e| Line::raw(format_log_entry(e)))
                .collect();
            let log_widget = Paragraph::new(log_text)
                .block(Block::default().borders(Borders::ALL).title(" Log  [PgUp/PgDn/Home/End to scroll] "))
                .scroll((top_offset as u16, 0));
            f.render_widget(log_widget, chunks[2]);

            // ── Prompt ────────────────────────────────────────────────────
            let prompt_widget = Paragraph::new(input.as_str())
                .block(Block::default().borders(Borders::ALL).title(" Command "))
                .wrap(Wrap { trim: false });
            f.render_widget(prompt_widget, chunks[3]);
        });
    }

    // ── Command dispatch ─────────────────────────────────────────────────────

    fn execute_command(&mut self, raw: &str) {
        let parts: Vec<&str> = raw.split_whitespace().collect();
        if parts.is_empty() { return; }
        let cmd = parts[0].to_ascii_lowercase();
        let args = &parts[1..];

        match cmd.as_str() {
            "/help" | "/h" | "/?" => self.cmd_help(args),
            "/kill" => self.cmd_kill(args),
            "/die" => {
                self.push_cmd(DebugCommand::AdjustStat { stat: StatId::Vitality, delta: -9999 });
                self.log("Player vitality set to zero.");
            }
            "/pack" => self.push_cmd(DebugCommand::HeroPack),
            "/max" => self.cmd_max_stats(),
            "/heal" => self.cmd_heal(),
            "/stat" => self.cmd_stat(args),
            "/stats" => self.cmd_stats(),
            "/quest" => self.cmd_quest(),
            "/inventory" | "/inventorylist" => self.cmd_inventory(),
            "/inv" => self.cmd_inv(args),
            "/give" => self.cmd_give(args),
            "/take" => self.cmd_take(args),
            "/cheat" => self.cmd_cheat(args),
            "/tp" | "/teleport" => self.cmd_tp(args),
            "/god" => self.cmd_god(args),
            "/noclip" => self.cmd_god(&["noclip"]),
            "/magic" => self.cmd_magic(args),
            "/swan" => self.push_cmd(DebugCommand::SummonSwan),
            "/time" => self.cmd_time(args),
            "/brother" => self.cmd_brother(args),
            "/fx" => self.cmd_fx(args),
            "/actors" => self.push_cmd(DebugCommand::QueryActors),
            "/terrain" => self.push_cmd(DebugCommand::QueryTerrain),
            "/doors" => self.push_cmd(DebugCommand::QueryDoors),
            "/extent" => self.push_cmd(DebugCommand::QueryExtent),
            "/encounter" => self.cmd_encounter(args),
            "/items" => self.cmd_items(args),
            "/songs" => self.cmd_songs(args),
            "/adf" => self.cmd_adf(args),
            "/clear" | "/cls" => self.cmd_clear(),
            "/pause" => {
                self.pause_request = Some(true);
                self.log("Game paused. /resume to continue, /step [n] to advance frames.");
            }
            "/resume" | "/unpause" => {
                self.pause_request = Some(false);
                self.log("Game resumed.");
            }
            "/step" => {
                let n: u32 = args.first().and_then(|s| s.parse().ok()).unwrap_or(1);
                let n = n.max(1);
                self.step_request = self.step_request.saturating_add(n);
                self.pause_request = Some(true);
                self.log(format!("Stepping {} tick(s).", n));
            }
            "/watch" => {
                self.watch_expanded = !self.watch_expanded;
                self.log(format!(
                    "Actor watch {}.",
                    if self.watch_expanded { "expanded" } else { "collapsed" }
                ));
            }
            _ => {
                self.log(format!("Unknown command: {}  (type /help for list)", cmd));
            }
        }
    }

    fn push_cmd(&mut self, cmd: DebugCommand) {
        self.pending_commands.push(cmd);
    }

    // ── Individual commands ───────────────────────────────────────────────────

    fn cmd_help(&mut self, args: &[&str]) {
        if let Some(&topic) = args.first() {
            let msg = match topic.to_ascii_lowercase().as_str() {
                "/kill" | "kill"     => "/kill — kill all hostile enemies on screen.\n  /kill <slot>  kill one actor slot (1-19).",
                "/die"  | "die"      => "/die — set player vitality to zero (die).",
                "/pack" | "pack"     => "/pack — fill weapons, magic items, keys, and arrows.",
                "/max"  | "max"      => "/max — set all stats to maximum / hunger+fatigue to 0.",
                "/heal" | "heal"     => "/heal — vitality to 15 + brave/4, hunger=0, fatigue=0.",
                "/stat" | "stat"     => "/stat <name> [+|-]<value>  e.g. /stat vit 100 or /stat hunger -50\n  Names: vit, brv, lck, knd, wlt, hgr, ftg",
                "/stats"            => "/stats — full hero stat dump to log.",
                "/quest"            => "/quest — quest progress (princess, statues, writ, talisman).",
                "/inventory"        => "/inventory — full stuff[] dump grouped by category.",
                "/inv"  | "inv"      => "/inv <slot 0-34> [+|-]<value>  e.g. /inv 0 1 or /inv 8 +99",
                "/give" | "give"     => "/give <item>  add 1 x item by name or stuff index (see /items).",
                "/take" | "take"     => "/take <item>  remove 1 x item by name or stuff index.",
                "/cheat"| "cheat"    => "/cheat          toggle cheat1 debug-key mode\n  /cheat on|off  set explicitly.",
                "/tp"   | "teleport" => "/tp safe | ring <N> | <x> <y> | <location>\n  e.g. /tp 200 150   /tp tavern   /tp ring 0",
                "/god"  | "god"      => "/god [noclip|invincible|ohk|reach|all|off]  — toggle god mode flag.",
                "/noclip"           => "/noclip — shortcut for /god noclip.",
                "/magic"| "magic"    => "/magic <light|secret|freeze> — toggle sticky magic effect.",
                "/swan" | "swan"     => "/swan — summon the swan.",
                "/time" | "time"     => "/time <HH:MM> | dawn | noon | dusk | midnight | hold | resume\n  /time hold — freeze time.  /time resume — unfreeze.",
                "/brother"          => "/brother <julian|phillip|kevin>",
                "/fx"   | "fx"      => "/fx <witch|teleport|fadeout|fadein>",
                "/actors"           => "/actors — print actor list to log.",
                "/terrain"          => "/terrain — dump terra lookup chain at hero's feet (collision debug).",
                "/doors"            => "/doors — list doors in current region + key inventory.",
                "/extent"           => "/extent — dump extent zone under the hero's feet.",
                "/encounter"        => "/encounter — force regional encounter (4 enemies).\n  /encounter <type>  spawn one enemy: orc ghost skeleton wraith dragon snake swan horse\n  /encounter clear   deactivate all active NPCs",
                "/items"            => "/items — scatter items around player.\n  /items             all 30 safe items\n  /items <count>     N random items (no talisman)\n  /items <name|id>   drop one item by name or index 0-30\n  /items <n> <name>  drop N of a named item\n  Note: talisman (triggers end-of-game) only drops with: /items talisman",
                "/songs"| "songs"   => "/songs — list song groups.  /songs play <N>  /songs stop  /songs cave <on|off>",
                "/adf"  | "adf"     => "/adf <block> [count] — hex dump ADF block(s) to log.",
                "/clear"| "cls"     => "/clear — clear the log.",
                _ => "No help for that topic.",
            };
            for line in msg.lines() {
                self.log(line);
            }
            return;
        }

        let lines = [
            "— Commands ———————————————————————————",
            "  /kill          kill enemies on screen  (/kill <slot> = one actor)",
            "  /die           kill the player",
            "  /pack          fill weapons, magic, keys",
            "  /max           max all stats",
            "  /heal          heal vitality + clear hunger/fatigue",
            "  /stat <n> <v>  set/adjust a stat (vit/brv/lck/knd/wlt/hgr/ftg)",
            "  /stats         dump all hero stats",
            "  /quest         dump quest progress",
            "  /inventory     dump full stuff[] array",
            "  /inv <s> <v>   set/adjust inventory slot 0-34",
            "  /give <item>   add 1 x item (name or index)",
            "  /take <item>   remove 1 x item (name or index)",
            "  /cheat [on|off] toggle / set cheat1 mode",
            "  /tp <x> <y>    teleport (also: /tp safe | ring <N> | <location>)",
            "  /god [flag]    god mode: noclip/invincible/ohk/reach/all/off",
            "  /noclip        toggle noclip shortcut",
            "  /magic <m>     sticky magic: light/secret/freeze",
            "  /swan          summon the swan",
            "  /time <t>      set time: HH:MM or dawn/noon/dusk/midnight/hold/resume",
            "  /brother <b>   switch to julian/phillip/kevin",
            "  /fx <e>        trigger: witch/teleport/fadeout/fadein",
            "  /actors        list actors",
            "  /terrain       dump terrain at current position",
            "  /doors         dump door + key state",
            "  /extent        dump extent zone under the hero",
            "  /encounter [t] force encounter / spawn type / clear",
            "  /items [n] [name]  scatter items around player (no talisman unless named)",
            "  /songs [cmd]   music: play <N> / stop / cave <on|off>",
            "  /adf <b> [n]   hex dump n ADF block(s) starting at b",
            "  /clear         clear this log",
            "  /help [cmd]    show help",
            "——————————————————————————————————————",
            "PgUp/PgDn/Home/End — scroll log   Up/Down — command history",
        ];
        for l in &lines { self.log(*l); }
    }

    fn cmd_max_stats(&mut self) {
        use StatId::*;
        for (s, v) in &[
            (Vitality, 999i16), (Brave, 255), (Luck, 255),
            (Kind, 255), (Wealth, 9999), (Hunger, 0), (Fatigue, 0),
        ] {
            self.push_cmd(DebugCommand::SetStat { stat: *s, value: *v });
        }
        self.log("Max stats applied.");
    }

    /// `/heal` — vitality = `15 + brave/4`, hunger = 0, fatigue = 0.
    fn cmd_heal(&mut self) {
        let cap = 15i16.saturating_add((self.status.brave as i16) / 4);
        self.push_cmd(DebugCommand::SetStat { stat: StatId::Vitality, value: cap });
        self.push_cmd(DebugCommand::SetStat { stat: StatId::Hunger, value: 0 });
        self.push_cmd(DebugCommand::SetStat { stat: StatId::Fatigue, value: 0 });
        self.log(format!("Healed: vitality={} (cap 15 + brave/4), hunger=0, fatigue=0.", cap));
    }

    /// `/stats` — full hero stat dump.
    fn cmd_stats(&mut self) {
        let s = &self.status;
        let heal_cap = 15i16.saturating_add((s.brave as i16) / 4);
        let lines = [
            format!("── Hero Stats ──"),
            format!("  Vitality: {} / {}   Brave: {}   Luck: —   Kind: —",
                s.vitality, heal_cap, s.brave),
            format!("  Wealth: {}g   Hunger: {}   Fatigue: {}",
                s.wealth, s.hunger, s.fatigue),
            format!("  Position: ({}, {})   Region: {}   Brother: {}",
                s.hero_x, s.hero_y, s.region_num, s.brother),
            format!("  God mode: {:#06b}   Cheat1: {}   Paused: {}",
                s.god_mode_flags, s.cheat1, s.is_paused),
        ];
        for l in &lines { self.log(l.clone()); }
    }

    /// `/quest` — quest progress.
    fn cmd_quest(&mut self) {
        let s = &self.status;
        // Count keys in slots 16-21 (Gold/Silver/Ruby/Skull/Iron/Crystal keys per item ref).
        let key_count: u16 = s.stuff.iter().skip(16).take(6).map(|&v| v as u16).sum();
        let lines = [
            format!("── Quest Progress ──"),
            format!("  Princess captive: {}   Rescues: {}",
                s.princess_captive, s.princess_rescues),
            format!("  Gold statues: {} / 5   Writ of Safe Conduct: {}",
                s.statues_collected, if s.has_writ { "yes" } else { "no" }),
            format!("  TALISMAN: {}   Keys held (16-21): {}",
                if s.has_talisman { "YES (win condition!)" } else { "no" }, key_count),
        ];
        for l in &lines { self.log(l.clone()); }
    }

    /// `/inventory` — full stuff[] array grouped by category.
    fn cmd_inventory(&mut self) {
        let s = &self.status.stuff;
        if s.is_empty() {
            self.log("Inventory not available (not in gameplay).");
            return;
        }
        let get = |i: usize| -> u8 { s.get(i).copied().unwrap_or(0) };
        let lines = [
            format!("── Inventory (stuff[{}]) ──", s.len()),
            format!("  Weapons : dirk={} mace={} sword={} bow={} wand={} lasso={} shell={} [7]={}",
                get(0), get(1), get(2), get(3), get(4), get(5), get(6), get(7)),
            format!("  Arrows  : {}", get(8)),
            format!("  Magic   : vial={} jewel={} totem={} flute={} ring={} skull={} staff={}",
                get(9), get(10), get(11), get(12), get(13), get(14), get(15)),
            format!("  Keys    : gold={} silver={} ruby={} skull={} iron={} crystal={}",
                get(16), get(17), get(18), get(19), get(20), get(21)),
            format!("  Quest   : talisman={} writ={} statues={}",
                get(22), get(28), get(25)),
            format!("  Consume : food={} fruit={}", get(23), get(24)),
            format!("  Gold    : {}", self.status.wealth),
        ];
        for l in &lines { self.log(l.clone()); }
    }

    /// `/kill` — no args: kill all enemies on screen. With arg: kill single slot.
    fn cmd_kill(&mut self, args: &[&str]) {
        if args.is_empty() {
            self.push_cmd(DebugCommand::InstaKill);
        } else {
            match args[0].parse::<u8>() {
                Ok(slot) if slot >= 1 && slot <= 19 => {
                    self.push_cmd(DebugCommand::KillActorSlot { slot });
                }
                Ok(_) => self.log("/kill: slot must be 1-19"),
                Err(_) => self.log(format!("/kill: bad slot '{}'", args[0])),
            }
        }
    }

    /// `/give <item>` — resolve name via debug_items map, then SetInventory +1.
    fn cmd_give(&mut self, args: &[&str]) {
        let Some(raw) = args.first() else {
            self.log("Usage: /give <item>  (name or stuff index)");
            return;
        };
        match crate::game::debug_items::lookup_by_name(raw)
            .or_else(|| raw.parse::<u8>().ok().and_then(crate::game::debug_items::lookup_by_id))
        {
            Some(entry) => {
                self.push_cmd(DebugCommand::AdjustInventory {
                    index: entry.stuff_index as u8, delta: 1,
                });
                self.log(format!("Gave 1 x {} (stuff[{}]).", entry.name, entry.stuff_index));
            }
            None => self.log(format!("/give: unknown item '{}'", raw)),
        }
    }

    /// `/take <item>` — resolve name via debug_items map, then AdjustInventory -1.
    fn cmd_take(&mut self, args: &[&str]) {
        let Some(raw) = args.first() else {
            self.log("Usage: /take <item>  (name or stuff index)");
            return;
        };
        match crate::game::debug_items::lookup_by_name(raw)
            .or_else(|| raw.parse::<u8>().ok().and_then(crate::game::debug_items::lookup_by_id))
        {
            Some(entry) => {
                self.push_cmd(DebugCommand::AdjustInventory {
                    index: entry.stuff_index as u8, delta: -1,
                });
                self.log(format!("Took 1 x {} (stuff[{}]).", entry.name, entry.stuff_index));
            }
            None => self.log(format!("/take: unknown item '{}'", raw)),
        }
    }

    /// `/cheat` — toggle. `/cheat on` / `/cheat off` — explicit set.
    fn cmd_cheat(&mut self, args: &[&str]) {
        let enabled = match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            None | Some("") => !self.status.cheat1,
            Some("on")      => true,
            Some("off")     => false,
            Some(other) => {
                self.log(format!("/cheat: unknown arg '{}' (use on/off or no arg to toggle)", other));
                return;
            }
        };
        self.push_cmd(DebugCommand::SetCheat1 { enabled });
        self.log(format!("cheat1 -> {}", if enabled { "ON" } else { "OFF" }));
    }

    fn cmd_stat(&mut self, args: &[&str]) {
        if args.len() < 2 {
            self.log("Usage: /stat <name> [+|-]<value>  (vit brv lck knd wlt hgr ftg)");
            return;
        }
        let stat = match args[0].to_ascii_lowercase().as_str() {
            "vit" | "vitality"  => StatId::Vitality,
            "brv" | "brave"     => StatId::Brave,
            "lck" | "luck"      => StatId::Luck,
            "knd" | "kind"      => StatId::Kind,
            "wlt" | "wealth"    => StatId::Wealth,
            "hgr" | "hunger"    => StatId::Hunger,
            "ftg" | "fatigue"   => StatId::Fatigue,
            other => {
                self.log(format!("Unknown stat: {}  (use vit brv lck knd wlt hgr ftg)", other));
                return;
            }
        };
        let raw_val = args[1];
        let is_delta = raw_val.starts_with('+') || raw_val.starts_with('-');
        if is_delta {
            match raw_val.parse::<i16>() {
                Ok(delta) => self.push_cmd(DebugCommand::AdjustStat { stat, delta }),
                Err(_) => self.log(format!("Bad value: {}", raw_val)),
            }
        } else {
            match raw_val.parse::<i16>() {
                Ok(val) => self.push_cmd(DebugCommand::SetStat { stat, value: val }),
                Err(_) => self.log(format!("Bad value: {}", raw_val)),
            }
        }
    }

    fn cmd_inv(&mut self, args: &[&str]) {
        if args.len() < 2 {
            self.log("Usage: /inv <slot 0-34> [+|-]<value>");
            return;
        }
        let slot: u8 = match args[0].parse() {
            Ok(s) if s < 35 => s,
            _ => { self.log("Slot must be 0-34."); return; }
        };
        let raw_val = args[1];
        let is_delta = raw_val.starts_with('+') || raw_val.starts_with('-');
        if is_delta {
            match raw_val.parse::<i8>() {
                Ok(delta) => self.push_cmd(DebugCommand::AdjustInventory { index: slot, delta }),
                Err(_) => self.log(format!("Bad value: {}", raw_val)),
            }
        } else {
            match raw_val.parse::<u8>() {
                Ok(val) => self.push_cmd(DebugCommand::SetInventory { index: slot, value: val }),
                Err(_) => self.log(format!("Bad value: {}", raw_val)),
            }
        }
    }

    fn cmd_tp(&mut self, args: &[&str]) {
        match args {
            [] => self.log("Usage: /tp safe | ring <N> | <x> <y> | <location>"),
            ["safe"] | ["Safe"] => self.push_cmd(DebugCommand::TeleportSafe),
            ["ring", n] => {
                match n.parse::<u8>() {
                    Ok(idx) => self.push_cmd(DebugCommand::TeleportStoneRing { index: idx }),
                    Err(_) => self.log(format!("Bad ring index: {}", n)),
                }
            }
            [xs, ys] if xs.chars().next().map_or(false, |c| c.is_ascii_digit())
                     && ys.chars().next().map_or(false, |c| c.is_ascii_digit()) => {
                let x = xs.parse::<u16>();
                let y = ys.parse::<u16>();
                match (x, y) {
                    (Ok(x), Ok(y)) => self.push_cmd(DebugCommand::TeleportCoords { x, y }),
                    _ => self.log("Usage: /tp <x> <y>  (unsigned integers)"),
                }
            }
            _ => {
                // Treat remaining forms as a named location (may be multi-word).
                let name = args.join(" ");
                self.push_cmd(DebugCommand::TeleportNamedLocation { name: name.clone() });
                self.log(format!("Teleport request: '{}'", name));
            }
        }
    }

    fn cmd_god(&mut self, args: &[&str]) {
        let current = GodModeFlags::from_bits_truncate(self.status.god_mode_flags);
        let new_flags = match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("noclip")     => current ^ GodModeFlags::NOCLIP,
            Some("invincible") => current ^ GodModeFlags::INVINCIBLE,
            Some("ohk") | Some("onehit") => current ^ GodModeFlags::ONE_HIT_KILL,
            Some("reach") | Some("insane") => current ^ GodModeFlags::INSANE_REACH,
            Some("all") | Some("on") => GodModeFlags::all(),
            Some("off") | Some("none") => GodModeFlags::empty(),
            None | Some("") => {
                // print current state
                let s = build_god_str(self.status.god_mode_flags);
                self.log(format!("God mode: {}", if s.is_empty() { "off" } else { &s }));
                return;
            }
            Some(other) => {
                self.log(format!("Unknown flag: {}  (noclip/invincible/ohk/reach/all/off)", other));
                return;
            }
        };
        self.push_cmd(DebugCommand::SetGodMode { flags: new_flags });
        let s = build_god_str(new_flags.bits());
        self.log(format!("God mode: {}", if s.is_empty() { "off" } else { &s }));
        // Keep our local copy in sync so toggle works correctly within the same session
        self.status.god_mode_flags = new_flags.bits();
    }

    fn cmd_magic(&mut self, args: &[&str]) {
        let effect = match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("light")   => MagicEffect::Light,
            Some("secret")  => MagicEffect::Secret,
            Some("freeze")  => MagicEffect::Freeze,
            _ => { self.log("Usage: /magic <light|secret|freeze>"); return; }
        };
        self.push_cmd(DebugCommand::ToggleMagicEffect { effect });
    }

    fn cmd_time(&mut self, args: &[&str]) {
        match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("hold")      => self.push_cmd(DebugCommand::HoldTimeOfDay { hold: true }),
            Some("resume") | Some("free") | Some("unhold") => self.push_cmd(DebugCommand::HoldTimeOfDay { hold: false }),
            Some("midnight")  => self.push_cmd(DebugCommand::SetDayPhase { phase: 0 }),
            Some("dawn")      => self.push_cmd(DebugCommand::SetDayPhase { phase: 6000 }),
            Some("noon")      => self.push_cmd(DebugCommand::SetDayPhase { phase: 12000 }),
            Some("dusk")      => self.push_cmd(DebugCommand::SetDayPhase { phase: 18000 }),
            Some(hhmm) => {
                // Try HH:MM parse
                let parts: Vec<&str> = hhmm.split(':').collect();
                if parts.len() == 2 {
                    let h = parts[0].parse::<u8>();
                    let m = parts[1].parse::<u8>();
                    match (h, m) {
                        (Ok(hour), Ok(minute)) if hour < 24 && minute < 60 => {
                            self.push_cmd(DebugCommand::SetGameTime { hour, minute });
                        }
                        _ => self.log("Usage: /time HH:MM  (e.g. /time 08:30)"),
                    }
                } else {
                    self.log("Usage: /time <HH:MM | dawn | noon | dusk | midnight | hold | resume>");
                }
            }
            None => {
                self.log(format!(
                    "Game time: day {} {:02}:{:02}  phase={:?}  {}",
                    self.status.game_day,
                    self.status.game_hour,
                    self.status.game_minute,
                    self.status.day_phase,
                    if self.status.time_held { "[HELD]" } else { "" }
                ));
            }
        }
    }

    fn cmd_brother(&mut self, args: &[&str]) {
        let brother = match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("julian")  => BrotherId::Julian,
            Some("phillip") => BrotherId::Phillip,
            Some("kevin")   => BrotherId::Kevin,
            _ => { self.log("Usage: /brother <julian|phillip|kevin>"); return; }
        };
        self.push_cmd(DebugCommand::RestartAsBrother { brother });
    }

    fn cmd_fx(&mut self, args: &[&str]) {
        match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("witch")    => self.push_cmd(DebugCommand::TriggerWitchEffect),
            Some("teleport") => self.push_cmd(DebugCommand::TriggerTeleportEffect),
            Some("fadeout")  => self.push_cmd(DebugCommand::TriggerPaletteTransition { to_black: true }),
            Some("fadein")   => self.push_cmd(DebugCommand::TriggerPaletteTransition { to_black: false }),
            _ => self.log("Usage: /fx <witch|teleport|fadeout|fadein>"),
        }
    }

    fn cmd_songs(&mut self, args: &[&str]) {
        match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("play") => {
                match args.get(1).and_then(|s| s.parse::<usize>().ok()) {
                    Some(n) if n >= 1 => {
                        self.song_group_requested = Some(n - 1);
                        self.log(format!("Playing song group {}.", n));
                    }
                    _ => self.log("Usage: /songs play <N>  (1-based group number)"),
                }
            }
            Some("stop") => {
                self.stop_requested = true;
                self.log("Music stopped.");
            }
            Some("cave") => {
                match args.get(1).map(|s| s.to_ascii_lowercase()).as_deref() {
                    Some("on") => {
                        self.cave_mode_requested = Some(true);
                        self.log("Cave instrument mode ON (slot 10 → wave=3, vol=7).");
                    }
                    Some("off") => {
                        self.cave_mode_requested = Some(false);
                        self.log("Cave instrument mode OFF (slot 10 → default).");
                    }
                    _ => self.log("Usage: /songs cave <on|off>"),
                }
            }
            _ => {
                // Print song info from the status snapshot
                let count = self.status.song_group_count;
                if count == 0 {
                    self.log("No songs loaded.");
                } else {
                    self.log(format!("{} song groups available.", count));
                    let cur = self.status.current_song_group;
                    for i in 0..count {
                        let marker = if cur == Some(i) { " ◄ playing" } else { "" };
                        self.log(format!("  /songs play {}  — group {}{}", i + 1, i + 1, marker));
                    }
                    let cave_label = if self.status.cave_mode { "ON" } else { "OFF" };
                    self.log(format!("Cave mode: {}", cave_label));
                    self.log("/songs stop  — stop music");
                    self.log("/songs cave <on|off>  — cave instrument override");
                }
            }
        }
    }

    fn cmd_adf(&mut self, args: &[&str]) {
        let (block, count) = match args {
            [b] => match b.parse::<u32>() {
                Ok(b) => (b, 1u32),
                Err(_) => { self.log("Usage: /adf <block> [count]"); return; }
            },
            [b, c] => match (b.parse::<u32>(), c.parse::<u32>()) {
                (Ok(b), Ok(c)) if c >= 1 => (b, c),
                _ => { self.log("Usage: /adf <block> [count]  (count must be >= 1)"); return; }
            },
            _ => { self.log("Usage: /adf <block> [count]"); return; }
        };
        self.push_cmd(DebugCommand::DumpAdfBlock { block, count });
    }

    fn cmd_encounter(&mut self, args: &[&str]) {
        use crate::game::npc::*;
        match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            None => self.push_cmd(DebugCommand::SpawnEncounterRandom),
            Some("clear") => self.push_cmd(DebugCommand::ClearEncounters),
            Some(name) => {
                let npc_type = match name {
                    "orc"      => Some(NPC_TYPE_ORC),
                    "ghost"    => Some(NPC_TYPE_GHOST),
                    "skeleton" => Some(NPC_TYPE_SKELETON),
                    "wraith"   => Some(NPC_TYPE_WRAITH),
                    "dragon"   => Some(NPC_TYPE_DRAGON),
                    "snake"    => Some(NPC_TYPE_SKELETON), // snake → cfile 7 (same group)
                    "swan"     => Some(NPC_TYPE_SWAN),
                    "horse"    => Some(NPC_TYPE_HORSE),
                    _ => None,
                };
                match npc_type {
                    Some(t) => self.push_cmd(DebugCommand::SpawnEncounterType(t)),
                    None => self.log(format!(
                        "Unknown enemy type: {}.  Valid: orc ghost skeleton wraith dragon snake swan horse",
                        name
                    )),
                }
            }
        }
    }

    fn cmd_items(&mut self, args: &[&str]) {
        match args {
            [] => {
                // All 30 safe items (no talisman)
                self.push_cmd(DebugCommand::ScatterItems { count: 30, item_id: None });
            }
            [arg] => {
                if let Ok(n) = arg.parse::<usize>() {
                    self.push_cmd(DebugCommand::ScatterItems { count: n, item_id: None });
                } else {
                    match crate::game::sprites::item_name_to_id(arg) {
                        Some(id) => self.push_cmd(DebugCommand::ScatterItems { count: 1, item_id: Some(id) }),
                        None => self.log(format!("Unknown item: {}.  Use a name or index 0-30.", arg)),
                    }
                }
            }
            [count_str, name] => {
                match count_str.parse::<usize>() {
                    Err(_) => self.log(format!(
                        "Invalid count '{}'. Usage: /items [count] [name|index]", count_str
                    )),
                    Ok(n) => match crate::game::sprites::item_name_to_id(name) {
                        Some(id) => self.push_cmd(DebugCommand::ScatterItems { count: n, item_id: Some(id) }),
                        None => self.log(format!("Unknown item: {}.  Use a name or index 0-30.", name)),
                    },
                }
            }
            _ => self.log("Usage: /items [count] [name|index]  e.g. /items 5 sword".to_string()),
        }
    }

    fn cmd_clear(&mut self) {
        self.log_entries.clear();
        self.scroll_from_bottom = 0;
    }
}

impl Drop for DebugConsole {
    fn drop(&mut self) {
        // Restore terminal unconditionally; ignore errors during teardown.
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn styled_label(s: &'static str) -> Span<'static> {
    Span::styled(s, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
}

fn build_god_str(flags: u8) -> String {
    let f = GodModeFlags::from_bits_truncate(flags);
    let mut parts = Vec::new();
    if f.contains(GodModeFlags::NOCLIP)       { parts.push("NOCLIP"); }
    if f.contains(GodModeFlags::INVINCIBLE)   { parts.push("INVINCIBLE"); }
    if f.contains(GodModeFlags::ONE_HIT_KILL) { parts.push("ONE_HIT_KILL"); }
    if f.contains(GodModeFlags::INSANE_REACH) { parts.push("INSANE_REACH"); }
    parts.join("+")
}

/// Format a single log entry for rendering: `"[CATEGORY] text"`, with an
/// optional `"[{tick}] "` prefix when `timestamp_ticks` is non-zero.
/// Filter log entries by the given active-category set (DBG-LOG-05).
///
/// Returns references to entries whose `category` is present in `active`.
fn filter_log_entries<'a>(
    entries: &'a [DebugLogEntry],
    active: &std::collections::HashSet<LogCategory>,
) -> Vec<&'a DebugLogEntry> {
    entries.iter().filter(|e| active.contains(&e.category)).collect()
}

fn format_log_entry(entry: &DebugLogEntry) -> String {
    if entry.timestamp_ticks != 0 {
        format!("[{}] [{}] {}", entry.timestamp_ticks, entry.category.label(), entry.text)
    } else {
        format!("[{}] {}", entry.category.label(), entry.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helpers --------------------------------------------------------------
    //
    // `DebugConsole::new` installs a crossterm alternate screen and raw mode,
    // which is unsuitable for unit tests.  These tests construct a minimal
    // console-like harness that exercises the log storage/format helpers
    // directly.

    fn make_entry(cat: LogCategory, ticks: u64, text: &str) -> DebugLogEntry {
        DebugLogEntry { category: cat, timestamp_ticks: ticks, text: text.to_owned() }
    }

    /// Mimics `DebugConsole::log_entry` for testing without allocating a
    /// terminal.  Kept in sync with the real implementation above.
    fn push(entries: &mut Vec<DebugLogEntry>, entry: DebugLogEntry) {
        for line in entry.text.split('\n') {
            entries.push(DebugLogEntry {
                category: entry.category,
                timestamp_ticks: entry.timestamp_ticks,
                text: line.to_owned(),
            });
        }
        if entries.len() > MAX_LOG_LINES {
            let overflow = entries.len() - MAX_LOG_LINES;
            entries.drain(..overflow);
        }
    }

    #[test]
    fn log_entry_appends_and_respects_max_lines() {
        let mut entries: Vec<DebugLogEntry> = Vec::new();
        for i in 0..(MAX_LOG_LINES + 25) {
            push(&mut entries, make_entry(LogCategory::General, 0, &format!("msg {}", i)));
        }
        assert_eq!(entries.len(), MAX_LOG_LINES);
        // Oldest 25 entries should have been drained from the front.
        assert_eq!(entries[0].text, "msg 25");
        assert_eq!(entries.last().unwrap().text, format!("msg {}", MAX_LOG_LINES + 24));
    }

    #[test]
    fn log_entry_splits_on_newlines_preserving_category() {
        let mut entries: Vec<DebugLogEntry> = Vec::new();
        push(&mut entries, make_entry(LogCategory::Combat, 42, "line a\nline b\nline c"));
        assert_eq!(entries.len(), 3);
        for e in &entries {
            assert_eq!(e.category, LogCategory::Combat);
            assert_eq!(e.timestamp_ticks, 42);
        }
        assert_eq!(entries[0].text, "line a");
        assert_eq!(entries[1].text, "line b");
        assert_eq!(entries[2].text, "line c");
    }

    #[test]
    fn format_log_entry_general_no_tick() {
        let e = make_entry(LogCategory::General, 0, "hello");
        assert_eq!(format_log_entry(&e), "[GENERAL] hello");
    }

    #[test]
    fn format_log_entry_with_nonzero_tick_prefixes_tick() {
        let e = make_entry(LogCategory::Combat, 1234, "hero swings");
        assert_eq!(format_log_entry(&e), "[1234] [COMBAT] hero swings");
    }

    #[test]
    fn filter_keeps_only_active_categories() {
        let entries = vec![
            make_entry(LogCategory::Combat, 0, "hit"),
            make_entry(LogCategory::Movement, 0, "step"),
            make_entry(LogCategory::Quest, 0, "flag"),
            make_entry(LogCategory::Ai, 0, "think"),
        ];
        let active: std::collections::HashSet<LogCategory> =
            [LogCategory::Combat, LogCategory::Quest].iter().copied().collect();
        let kept = filter_log_entries(&entries, &active);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].text, "hit");
        assert_eq!(kept[1].text, "flag");
    }

    #[test]
    fn filter_empty_active_set_hides_everything() {
        let entries = vec![
            make_entry(LogCategory::Combat, 0, "a"),
            make_entry(LogCategory::General, 0, "b"),
        ];
        let active: std::collections::HashSet<LogCategory> = Default::default();
        let kept = filter_log_entries(&entries, &active);
        assert!(kept.is_empty());
    }

    #[test]
    fn filter_default_active_set_hides_noisy_categories() {
        // DBG-LOG-05: default seed excludes the 5 noisy categories.
        let active: std::collections::HashSet<LogCategory> = LogCategory::ALL
            .iter()
            .copied()
            .filter(|c| c.default_enabled())
            .collect();
        let entries = vec![
            make_entry(LogCategory::Combat, 0, "shown"),
            make_entry(LogCategory::Movement, 0, "hidden"),
            make_entry(LogCategory::Ai, 0, "hidden"),
            make_entry(LogCategory::Rendering, 0, "hidden"),
            make_entry(LogCategory::Animation, 0, "hidden"),
            make_entry(LogCategory::Time, 0, "hidden"),
            make_entry(LogCategory::General, 0, "shown"),
        ];
        let kept = filter_log_entries(&entries, &active);
        assert_eq!(kept.len(), 2);
        assert!(kept.iter().all(|e| e.text == "shown"));
    }
}
