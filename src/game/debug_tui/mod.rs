//! Debug TUI module — split into `bridge` (always compiled), `view` +
//! `commands` (feature-gated rendering/dispatch), and `stub` (no-op when
//! the `debug-tui` feature is disabled).
//!
//! See `DEBUG_SPECIFICATION.md` §Architecture.

pub mod bridge;
pub use bridge::*;

#[cfg(feature = "debug-tui")]
mod commands;
#[cfg(feature = "debug-tui")]
mod view;
#[cfg(feature = "debug-tui")]
pub use view::DebugConsole;

#[cfg(not(feature = "debug-tui"))]
mod stub;
#[cfg(not(feature = "debug-tui"))]
pub use stub::DebugConsole;
