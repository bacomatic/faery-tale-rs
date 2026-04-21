//! Stub `DebugConsole` compiled when the `debug-tui` feature is disabled.
//!
//! Mirrors the public API of the real `DebugConsole` so `main.rs` can remain
//! feature-agnostic. `new()` emits a one-time warning and returns `Err`, so
//! the main loop's normal "no console available" branch takes over; all
//! other methods are no-ops.

use std::io;

use super::bridge::{DebugCommand, DebugLogEntry, DebugSnapshot};

pub struct DebugConsole {
    _priv: (),
}

impl DebugConsole {
    pub fn new() -> Result<Self, io::Error> {
        eprintln!(
            "warning: --debug requested but binary built without `debug-tui` feature; \
             no debug console will open. Rebuild with default features (or \
             `--features debug-tui`) to enable."
        );
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "debug-tui feature disabled",
        ))
    }

    pub fn update_status(&mut self, _status: DebugSnapshot) {}
    pub fn log_entry(&mut self, _entry: DebugLogEntry) {}
    pub fn ingest(&mut self, _entry: DebugLogEntry) {}
    pub fn log(&mut self, _msg: impl Into<String>) {}
    pub fn drain_commands(&mut self) -> Vec<DebugCommand> { Vec::new() }
    pub fn take_pause_request(&mut self) -> Option<bool> { None }
    pub fn take_step_request(&mut self) -> u32 { 0 }
    pub fn take_song_request(&mut self) -> Option<usize> { None }
    pub fn take_stop_request(&mut self) -> bool { false }
    pub fn take_cave_mode_request(&mut self) -> Option<bool> { None }
    pub fn take_quit_request(&mut self) -> bool { false }
    pub fn poll_input(&mut self) -> bool { false }
    pub fn render(&mut self) {}
}
