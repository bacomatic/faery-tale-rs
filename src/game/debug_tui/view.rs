//! Debug TUI view layer: the `DebugConsole` struct, input polling, and
//! ratatui rendering. Feature-gated behind `debug-tui`.
//!
//! The impl for `DebugConsole` is split across this file and `commands.rs`
//! (command dispatch + individual `/cmd` handlers). Both files live in the
//! same `debug_tui` module, so private fields are visible to each other.

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
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Terminal,
};

use super::bridge::*;
use super::commands::{filter_log_entries, filter_modal_lines, format_log_entry};

pub(super) const MAX_LOG_LINES: usize = 1000;

pub struct DebugConsole {
    pub(super) terminal: Terminal<CrosstermBackend<Stdout>>,

    // Log output
    pub(super) log_entries: Vec<DebugLogEntry>,
    /// If the user hasn't manually scrolled, we auto-scroll to the bottom.
    pub(super) auto_scroll: bool,
    /// Scroll offset from the bottom (0 = show tail).
    pub(super) scroll_from_bottom: usize,

    // Command prompt
    pub(super) input_buffer: String,
    pub(super) command_history: Vec<String>,
    pub(super) history_index: Option<usize>,

    // Queued items for the main loop to consume
    pub(super) pending_commands: Vec<DebugCommand>,
    pub(super) song_group_requested: Option<usize>,
    pub(super) stop_requested: bool,
    pub(super) cave_mode_requested: Option<bool>,
    pub(super) quit_requested: bool,
    /// Pause/resume request: Some(true) = pause, Some(false) = resume, None = none.
    pub(super) pause_request: Option<bool>,
    /// Step request: number of ticks to advance while paused. Consumed by the clock.
    pub(super) step_request: u32,

    /// Actor Watch panel display mode (DBG-LAYOUT-07). false = collapsed (default).
    pub(super) watch_expanded: bool,

    /// Active log categories used to filter the log panel render (DBG-LOG-05).
    /// Seeded from `LogCategory::default_enabled()` per DEBUG_SPEC §Log Categories.
    pub(super) active_categories: std::collections::HashSet<LogCategory>,

    /// DBG-LOG-08: When `Some(cursor)`, the `/filter` interactive modal is
    /// open with the highlight on `LogCategory::ALL[cursor]`. All other key
    /// input is swallowed until the user presses Enter/Esc to close.
    pub(super) filter_interactive: Option<usize>,

    // Latest status snapshot
    pub(super) status: DebugSnapshot,
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
            filter_interactive: None,
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

        // DBG-LOG-08: when the interactive /filter modal is open, route all
        // key events to it and swallow everything else.
        if self.filter_interactive.is_some() {
            if let Event::Key(ke) = &ev {
                if ke.kind == KeyEventKind::Press {
                    self.handle_filter_interactive_key(ke.code, ke.modifiers);
                }
            }
            return true;
        }

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

    /// DBG-LOG-08: handle a key press while the interactive `/filter` modal is open.
    fn handle_filter_interactive_key(&mut self, code: KeyCode, mods: KeyModifiers) {
        let Some(cursor) = self.filter_interactive else { return };
        let n = LogCategory::ALL.len();
        match code {
            KeyCode::Esc | KeyCode::Enter => {
                self.filter_interactive = None;
                self.log("Filter: closed interactive mode.");
            }
            KeyCode::Down => {
                self.filter_interactive = Some((cursor + 1) % n);
            }
            KeyCode::Up => {
                self.filter_interactive = Some((cursor + n - 1) % n);
            }
            KeyCode::Tab => {
                if mods.contains(KeyModifiers::SHIFT) {
                    self.filter_interactive = Some((cursor + n - 1) % n);
                } else {
                    self.filter_interactive = Some((cursor + 1) % n);
                }
            }
            KeyCode::BackTab => {
                self.filter_interactive = Some((cursor + n - 1) % n);
            }
            KeyCode::Char(' ') => {
                let cat = LogCategory::ALL[cursor];
                if self.active_categories.contains(&cat) {
                    self.active_categories.remove(&cat);
                } else {
                    self.active_categories.insert(cat);
                }
            }
            _ => {}
        }
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

            // Top row: four panels per DEBUG_SPEC §Top Row Panel Contents.
            let status_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(26),
                    Constraint::Percentage(26),
                    Constraint::Percentage(24),
                    Constraint::Percentage(24),
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

            // ── Flags (right-center) ──────────────────────────────────────
            let vfx_text = vec![
                Line::from(vec![
                    if status.is_paused {
                        Span::styled("[PAUSED]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    } else {
                        Span::raw("")
                    },
                    if status.vfx_witch_active {
                        Span::styled(" Witch", Style::default().fg(Color::Magenta))
                    } else { Span::raw("") },
                    if status.vfx_teleport_active {
                        Span::styled(" TP", Style::default().fg(Color::Green))
                    } else { Span::raw("") },
                    if status.vfx_secret_active {
                        Span::styled(" Secret", Style::default().fg(Color::Green))
                    } else { Span::raw("") },
                ]),
                Line::from(vec![
                    styled_label("Cave:"),
                    Span::raw(if status.cave_mode { "on" } else { "off" }),
                    Span::raw("  "),
                    styled_label("Princess:"),
                    Span::raw(if status.princess_captive { "captive" } else { "freed" }),
                ]),
                Line::from(vec![
                    styled_label("Writ:"),
                    Span::raw(if status.has_writ { "✓" } else { "—" }),
                    Span::raw("  "),
                    styled_label("Talisman:"),
                    Span::raw(if status.has_talisman { "✓" } else { "—" }),
                    Span::raw("  "),
                    styled_label("Statues:"),
                    Span::raw(format!("{}", status.statues_collected)),
                ]),
                Line::from(vec![
                    styled_label("Palette:"),
                    Span::raw(if status.vfx_palette_xfade { "xfade" } else { "—" }),
                    Span::raw("  "),
                    styled_label("Jewel:"),
                    Span::raw(if status.vfx_jewel_active { "on" } else { "—" }),
                ]),
            ];
            let vfx_widget = Paragraph::new(vfx_text)
                .block(Block::default().borders(Borders::ALL).title(" Flags "));
            f.render_widget(vfx_widget, status_chunks[2]);

            // ── Time & Performance (rightmost) ────────────────────────────────
            let time_text = vec![
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
                    styled_label("FPS:"),
                    Span::raw(format!("{:.1}  ", status.fps)),
                    styled_label("TPS:"),
                    Span::raw(format!("{:.1}", status.tps)),
                ]),
            ];
            let time_widget = Paragraph::new(time_text)
                .block(Block::default().borders(Borders::ALL).title(" Time "));
            f.render_widget(time_widget, status_chunks[3]);

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
                let mut rows: Vec<Line> = Vec::with_capacity(5);
                let non_player: Vec<_> = status.actors.iter()
                    .filter(|a| a.actor_type != 0)
                    .collect();
                for i in 0..5 {
                    let line = if let Some(a) = non_player.get(i) {
                        let kind = actor_kind_name(a.actor_type);
                        let race = race_label(a.race);
                        let state = actor_state_name(a.state);
                        if a.actor_type == 4 {
                            // SETFIG: no goal/tactic
                            format!(
                                "#{} {} {} {} HP:{}  ({},{})",
                                a.slot, kind, race, state, a.vitality, a.abs_x, a.abs_y,
                            )
                        } else {
                            format!(
                                "#{} {} {} {} HP:{} goal:{} tac:{}  ({},{})",
                                a.slot,
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
                    } else {
                        format!("— ({})", i + 1)
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
                .wrap(Wrap { trim: false })
                .scroll((top_offset as u16, 0));
            f.render_widget(log_widget, chunks[2]);

            // ── Prompt ────────────────────────────────────────────────────
            let prompt_widget = Paragraph::new(input.as_str())
                .block(Block::default().borders(Borders::ALL).title(" Command "))
                .wrap(Wrap { trim: false });
            f.render_widget(prompt_widget, chunks[3]);

            // ── DBG-LOG-08: interactive /filter modal overlay ──────────────
            if let Some(cursor) = self.filter_interactive {
                let lines = filter_modal_lines(cursor, &self.active_categories);
                let text: Vec<Line> = lines.into_iter().map(Line::raw).collect();
                let n = text.len() as u16;
                let w: u16 = 40;
                let h: u16 = n + 3;
                let x = area.x + area.width.saturating_sub(w) / 2;
                let y = area.y + area.height.saturating_sub(h) / 2;
                let popup = ratatui::layout::Rect { x, y, width: w.min(area.width), height: h.min(area.height) };
                f.render_widget(Clear, popup);
                let modal = Paragraph::new(text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" /filter — Space toggle  Tab/↑↓ move  Enter/Esc close "),
                    );
                f.render_widget(modal, popup);
            }
        });
    }
}

impl Drop for DebugConsole {
    fn drop(&mut self) {
        // Restore terminal unconditionally; ignore errors during teardown.
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
    }
}

// ── Styling helpers used by render() ─────────────────────────────────────────

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

