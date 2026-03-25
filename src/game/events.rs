//! Event message and speech helpers, backed by `faery.toml [narr]`.
//!
//! `event_msg(n)` in the original displayed `_event_msg[n]` with `%` replaced
//! by the hero's name.  `speak(n)` displayed `_speeches[n]` the same way.

use crate::game::game_library::NarrConfig;

/// Return event message `id` with `%` replaced by `name`.
/// Silently returns an empty string for out-of-range IDs.
pub fn event_msg(narr: &NarrConfig, id: usize, name: &str) -> String {
    narr.event_msg
        .get(id)
        .map(|tmpl| tmpl.replace('%', name))
        .unwrap_or_default()
}

/// Return speech message `id` with `%` replaced by `name`.
/// Silently returns an empty string for out-of-range IDs.
pub fn speak(narr: &NarrConfig, id: usize, name: &str) -> String {
    narr.speeches
        .get(id)
        .map(|tmpl| tmpl.replace('%', name))
        .unwrap_or_default()
}
