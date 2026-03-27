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

use crate::game::debug_command::{
    BrotherId, DebugCommand, GodModeFlags, MagicEffect, StatId,
};
use crate::game::game_state::DayPhase;

// ── Status snapshot ──────────────────────────────────────────────────────────

/// Lightweight game-state snapshot for the status header.
/// Built by main.rs each frame when the console is active.
#[derive(Debug, Clone, Default)]
pub struct DebugStatus {
    pub fps: f64,
    pub game_day: u32,
    pub game_hour: u32,
    pub game_minute: u32,
    pub day_phase: DayPhase,
    pub daynight: u16,
    pub lightlevel: u16,
    pub game_ticks: u64,
    pub paused: bool,
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
    pub autosave_enabled: bool,
    pub song_group_count: usize,
    pub current_song_group: Option<usize>,
    pub cave_mode: bool,

    // VFX state
    pub vfx_jewel_active: bool,
    pub vfx_light_sticky: bool,
    pub vfx_secret_active: bool,
    pub vfx_witch_active: bool,
    pub vfx_teleport_active: bool,
    pub vfx_palette_xfade: bool,
}

// ── DebugConsole ─────────────────────────────────────────────────────────────

const MAX_LOG_LINES: usize = 1000;

pub struct DebugConsole {
    terminal: Terminal<CrosstermBackend<Stdout>>,

    // Log output
    log_lines: Vec<String>,
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

    // Latest status snapshot
    status: DebugStatus,
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
            log_lines: Vec::new(),
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
            status: DebugStatus::default(),
        })
    }

    /// Update the status snapshot shown in the header.
    pub fn update_status(&mut self, status: DebugStatus) {
        self.status = status;
    }

    /// Push a message to the scrolling log.
    pub fn log(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        // Handle embedded newlines so each line is separate
        for line in msg.lines() {
            self.log_lines.push(line.to_owned());
        }
        if self.log_lines.len() > MAX_LOG_LINES {
            let overflow = self.log_lines.len() - MAX_LOG_LINES;
            self.log_lines.drain(..overflow);
        }
        // If auto-scroll is on, keep offset at 0 (bottom)
        if self.auto_scroll {
            self.scroll_from_bottom = 0;
        }
    }

    /// Drain pending debug commands for the main loop to apply.
    pub fn drain_commands(&mut self) -> Vec<DebugCommand> {
        self.pending_commands.drain(..).collect()
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
                        self.scroll_from_bottom = self.log_lines.len().saturating_sub(1);
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
        let log_lines = &self.log_lines;
        let scroll_from_bottom = self.scroll_from_bottom;
        let input = format!("> {}", self.input_buffer);
        let status = &self.status;

        let _ = self.terminal.draw(|f| {
            let area = f.area();

            // Layout: status header (fixed 6 lines) | log (fills) | prompt (3 lines)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(6),
                    Constraint::Min(3),
                    Constraint::Length(3),
                ])
                .split(area);

            // Split status header horizontally: Status (left) | VFX (right)
            let status_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(65),
                    Constraint::Percentage(35),
                ])
                .split(chunks[0]);

            // ── Status header ──────────────────────────────────────────────
            let phase_str = format!("{:?}", status.day_phase);
            let brother_name = match status.brother {
                1 => "Julian",
                2 => "Phillip",
                3 => "Kevin",
                _ => "?",
            };
            let god_str = build_god_str(status.god_mode_flags);
            let scene_str = status.scene_name.as_deref().unwrap_or("—");
            let song_str = match status.current_song_group {
                Some(g) => format!("playing #{}", g + 1),
                None => "stopped".to_owned(),
            };
            let hold_str = if status.time_held { "HELD" } else { "free" };
            let save_str = if status.autosave_enabled { "on" } else { "off" };

            let status_text = vec![
                Line::from(vec![
                    styled_label("FPS: "),
                    Span::raw(format!("{:5.1}  ", status.fps)),
                    styled_label("Day: "),
                    Span::raw(format!("{} {:02}:{:02}  ", status.game_day, status.game_hour, status.game_minute)),
                    styled_label("Phase: "),
                    Span::raw(format!("{}  ", phase_str)),
                    styled_label("Ticks: "),
                    Span::raw(format!("{}  ", status.game_ticks)),
                    if status.paused {
                        Span::styled("[PAUSED]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    } else {
                        Span::raw("")
                    },
                ]),
                Line::from(vec![
                    styled_label("Hero: "),
                    Span::raw(format!("({:4},{:4})  ", status.hero_x, status.hero_y)),
                    styled_label("Region: "),
                    Span::raw(format!("{}  ", status.region_num)),
                    styled_label("Brother: "),
                    Span::raw(format!("{}  ", brother_name)),
                    styled_label("Scene: "),
                    Span::raw(format!("{}  ", scene_str)),
                ]),
                Line::from(vec![
                    styled_label("VIT: "),
                    Span::raw(format!("{}  ", status.vitality)),
                    styled_label("HGR: "),
                    Span::raw(format!("{}  ", status.hunger)),
                    styled_label("FTG: "),
                    Span::raw(format!("{}  ", status.fatigue)),
                    styled_label("God: "),
                    Span::raw(format!("{}  ", if god_str.is_empty() { "off" } else { &god_str })),
                    styled_label("Time: "),
                    Span::raw(format!("{}  ", hold_str)),
                    styled_label("Autosave: "),
                    Span::raw(format!("{}  ", save_str)),
                ]),
                Line::from(vec![
                    styled_label("Music: "),
                    Span::raw(format!("{}  ({} groups available)", song_str, status.song_group_count)),
                ]),
            ];

            let status_widget = Paragraph::new(status_text)
                .block(Block::default().borders(Borders::ALL).title(" Status "));
            f.render_widget(status_widget, status_chunks[0]);

            // ── VFX status ────────────────────────────────────────────
            let on_off = |v: bool| if v { "ON" } else { "off" };
            let vfx_text = vec![
                Line::from(vec![
                    styled_label("LL: "),
                    Span::raw(format!("{}  ", status.lightlevel)),
                    styled_label("DN: "),
                    Span::raw(format!("{}  ", status.daynight)),
                ]),
                Line::from(vec![
                    styled_label("Jewel: "),
                    Span::raw(format!("{}  ", on_off(status.vfx_jewel_active))),
                    styled_label("Sticky: "),
                    Span::raw(format!("{}  ", on_off(status.vfx_light_sticky))),
                    styled_label("Secret: "),
                    Span::raw(format!("{}  ", on_off(status.vfx_secret_active))),
                ]),
                Line::from(vec![
                    styled_label("Witch: "),
                    Span::raw(format!("{}  ", on_off(status.vfx_witch_active))),
                    styled_label("Teleport: "),
                    Span::raw(format!("{}  ", on_off(status.vfx_teleport_active))),
                    styled_label("Xfade: "),
                    Span::raw(format!("{}  ", on_off(status.vfx_palette_xfade))),
                ]),
            ];

            let vfx_widget = Paragraph::new(vfx_text)
                .block(Block::default().borders(Borders::ALL).title(" VFX "));
            f.render_widget(vfx_widget, status_chunks[1]);

            // ── Log ───────────────────────────────────────────────────────
            let log_height = chunks[1].height.saturating_sub(2) as usize; // subtract borders
            let total = log_lines.len();
            // Compute scroll offset (from top) for ratatui's .scroll((top, 0))
            let top_offset = if total <= log_height {
                0
            } else {
                let bottom_top = total - log_height; // scroll to show tail
                bottom_top.saturating_sub(scroll_from_bottom)
            };

            let log_text: Vec<Line> = log_lines.iter().map(|l| Line::raw(l.as_str())).collect();
            let log_widget = Paragraph::new(log_text)
                .block(Block::default().borders(Borders::ALL).title(" Log  [PgUp/PgDn/Home/End to scroll] "))
                .scroll((top_offset as u16, 0));
            f.render_widget(log_widget, chunks[1]);

            // ── Prompt ────────────────────────────────────────────────────
            let prompt_widget = Paragraph::new(input.as_str())
                .block(Block::default().borders(Borders::ALL).title(" Command "))
                .wrap(Wrap { trim: false });
            f.render_widget(prompt_widget, chunks[2]);
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
            "/kill" => self.push_cmd(DebugCommand::InstaKill),
            "/die" => {
                self.push_cmd(DebugCommand::AdjustStat { stat: StatId::Vitality, delta: -9999 });
                self.log("Player vitality set to zero.");
            }
            "/pack" => self.push_cmd(DebugCommand::HeroPack),
            "/max" => self.cmd_max_stats(),
            "/stat" => self.cmd_stat(args),
            "/inv" => self.cmd_inv(args),
            "/tp" | "/teleport" => self.cmd_tp(args),
            "/god" => self.cmd_god(args),
            "/noclip" => self.cmd_god(&["noclip"]),
            "/magic" => self.cmd_magic(args),
            "/swan" => self.push_cmd(DebugCommand::SummonSwan),
            "/time" => self.cmd_time(args),
            "/brother" => self.cmd_brother(args),
            "/save" => self.cmd_autosave(args),
            "/fx" => self.cmd_fx(args),
            "/actors" => self.push_cmd(DebugCommand::QueryActors),
            "/terrain" => self.push_cmd(DebugCommand::QueryTerrain),
            "/songs" => self.cmd_songs(args),
            "/adf" => self.cmd_adf(args),
            "/clear" | "/cls" => self.cmd_clear(),
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
                "/kill" | "kill"     => "/kill — kill all enemies currently on screen.",
                "/die"  | "die"      => "/die — set player vitality to zero (die).",
                "/pack" | "pack"     => "/pack — fill weapons, magic items, keys, and arrows.",
                "/max"  | "max"      => "/max — set all stats to maximum / hunger+fatigue to 0.",
                "/stat" | "stat"     => "/stat <name> [+|-]<value>  e.g. /stat vit 100 or /stat hunger -50\n  Names: vit, brv, lck, knd, wlt, hgr, ftg",
                "/inv"  | "inv"      => "/inv <slot 0-34> [+|-]<value>  e.g. /inv 0 1 or /inv 8 +99",
                "/tp"   | "teleport" => "/tp safe | ring <N> | <x> <y>  e.g. /tp 200 150",
                "/god"  | "god"      => "/god [noclip|invincible|ohk|reach|all|off]  — toggle god mode flag.",
                "/noclip"           => "/noclip — shortcut for /god noclip.",
                "/magic"| "magic"    => "/magic <light|secret|freeze> — toggle sticky magic effect.",
                "/swan" | "swan"     => "/swan — summon the swan.",
                "/time" | "time"     => "/time <HH:MM> | dawn | noon | dusk | midnight | hold | free\n  /time hold — freeze time.  /time free — unfreeze.",
                "/brother"          => "/brother <julian|phillip|kevin>",
                "/save" | "save"    => "/save <on|off> — enable/disable autosave.",
                "/fx"   | "fx"      => "/fx <witch|teleport|fadeout|fadein>",
                "/actors"           => "/actors — print actor list to log.",
                "/terrain"          => "/terrain — dump terra lookup chain at hero's feet (collision debug).",
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
            "  /kill          kill all enemies on screen",
            "  /die           kill the player",
            "  /pack          fill weapons, magic, keys",
            "  /max           max all stats",
            "  /stat <n> <v>  set/adjust a stat (vit/brv/lck/knd/wlt/hgr/ftg)",
            "  /inv <s> <v>   set/adjust inventory slot 0-34",
            "  /tp <x> <y>    teleport to coords (also: /tp safe, /tp ring <N>)",
            "  /god [flag]    god mode: noclip/invincible/ohk/reach/all/off",
            "  /noclip        toggle noclip shortcut",
            "  /magic <m>     sticky magic: light/secret/freeze",
            "  /swan          summon the swan",
            "  /time <t>      set time: HH:MM or dawn/noon/dusk/midnight/hold/free",
            "  /brother <b>   switch to julian/phillip/kevin",
            "  /save <on|off> toggle autosave",
            "  /fx <e>        trigger: witch/teleport/fadeout/fadein",
            "  /actors        list actors",
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
            ["safe"] | ["Safe"] => self.push_cmd(DebugCommand::TeleportSafe),
            ["ring", n] => {
                match n.parse::<u8>() {
                    Ok(idx) => self.push_cmd(DebugCommand::TeleportStoneRing { index: idx }),
                    Err(_) => self.log(format!("Bad ring index: {}", n)),
                }
            }
            [xs, ys] => {
                let x = xs.parse::<u16>();
                let y = ys.parse::<u16>();
                match (x, y) {
                    (Ok(x), Ok(y)) => self.push_cmd(DebugCommand::TeleportCoords { x, y }),
                    _ => self.log("Usage: /tp <x> <y>  (unsigned integers)"),
                }
            }
            _ => self.log("Usage: /tp safe | /tp ring <N> | /tp <x> <y>"),
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
            Some("free") | Some("unhold") => self.push_cmd(DebugCommand::HoldTimeOfDay { hold: false }),
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
                    self.log("Usage: /time <HH:MM | dawn | noon | dusk | midnight | hold | free>");
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

    fn cmd_autosave(&mut self, args: &[&str]) {
        let enable = match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("on")  | Some("1") | Some("true")  => true,
            Some("off") | Some("0") | Some("false") => false,
            _ => { self.log("Usage: /save <on|off>"); return; }
        };
        self.push_cmd(DebugCommand::ToggleAutosave { enable });
        self.log(format!("Autosave: {}", if enable { "on" } else { "off" }));
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

    fn cmd_clear(&mut self) {
        self.log_lines.clear();
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
