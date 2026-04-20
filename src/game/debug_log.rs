//! Debug logging system for development and troubleshooting.
//!
//! Provides categorized debug logging with per-category filtering.
//! See DEBUG_SPECIFICATION.md §"Log Categories" for the full specification.

/// Categorizes debug log entries for filtering.
///
/// The ordering matches DEBUG_SPECIFICATION.md §"Log Categories".
/// The first 8 categories default to ON, the last 5 default to OFF (noisy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogCategory {
    Combat,
    Encounter,
    Quest,
    Npc,
    Door,
    Carrier,
    Magic,
    General,
    Movement,
    Ai,
    Rendering,
    Animation,
    Time,
}

impl LogCategory {
    /// All 13 log categories in specification order.
    pub const ALL: [LogCategory; 13] = [
        LogCategory::Combat,
        LogCategory::Encounter,
        LogCategory::Quest,
        LogCategory::Npc,
        LogCategory::Door,
        LogCategory::Carrier,
        LogCategory::Magic,
        LogCategory::General,
        LogCategory::Movement,
        LogCategory::Ai,
        LogCategory::Rendering,
        LogCategory::Animation,
        LogCategory::Time,
    ];

    /// Returns the uppercase label for this category as shown in DEBUG_SPECIFICATION.md.
    pub fn label(self) -> &'static str {
        match self {
            LogCategory::Combat => "COMBAT",
            LogCategory::Encounter => "ENCOUNTER",
            LogCategory::Quest => "QUEST",
            LogCategory::Npc => "NPC",
            LogCategory::Door => "DOOR",
            LogCategory::Carrier => "CARRIER",
            LogCategory::Magic => "MAGIC",
            LogCategory::General => "GENERAL",
            LogCategory::Movement => "MOVEMENT",
            LogCategory::Ai => "AI",
            LogCategory::Rendering => "RENDERING",
            LogCategory::Animation => "ANIMATION",
            LogCategory::Time => "TIME",
        }
    }

    /// Returns whether this category is enabled by default.
    ///
    /// Per DEBUG_SPECIFICATION.md:
    /// - First 8 categories (COMBAT..=GENERAL): ON by default
    /// - Last 5 categories (MOVEMENT..=TIME): OFF by default (noisy)
    pub fn default_enabled(self) -> bool {
        matches!(
            self,
            LogCategory::Combat
                | LogCategory::Encounter
                | LogCategory::Quest
                | LogCategory::Npc
                | LogCategory::Door
                | LogCategory::Carrier
                | LogCategory::Magic
                | LogCategory::General
        )
    }
}

/// A single debug log entry with category, timestamp, and message text.
#[derive(Debug, Clone)]
pub struct DebugLogEntry {
    /// The log category (for filtering).
    pub category: LogCategory,
    /// Game ticks at the time this entry was logged.
    pub timestamp_ticks: u64,
    /// The log message text.
    pub text: String,
}

/// Constructs a [`DebugLogEntry`] with the given category and formatted message.
///
/// The `timestamp_ticks` field is left at `0`; callers that care about timing
/// should overwrite it at push time (the debug console is responsible for the
/// authoritative tick stamp).
///
/// # Examples
///
/// ```ignore
/// use crate::debug_log;
/// let dmg = 7;
/// let entry = debug_log!(Combat, "hero dealt {} dmg", dmg);
/// assert_eq!(entry.text, "hero dealt 7 dmg");
/// let entry = debug_log!(General, "paused");
/// assert_eq!(entry.text, "paused");
/// ```
#[macro_export]
macro_rules! debug_log {
    ($cat:ident, $fmt:expr $(, $args:expr)* $(,)?) => {
        $crate::game::debug_log::DebugLogEntry {
            category: $crate::game::debug_log::LogCategory::$cat,
            timestamp_ticks: 0,
            text: format!($fmt $(, $args)*),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_categories_count() {
        assert_eq!(LogCategory::ALL.len(), 13, "Should have exactly 13 categories");
    }

    #[test]
    fn test_all_categories_present() {
        let all_set: std::collections::HashSet<_> = LogCategory::ALL.iter().copied().collect();
        
        // Verify each variant is in ALL
        assert!(all_set.contains(&LogCategory::Combat));
        assert!(all_set.contains(&LogCategory::Encounter));
        assert!(all_set.contains(&LogCategory::Quest));
        assert!(all_set.contains(&LogCategory::Npc));
        assert!(all_set.contains(&LogCategory::Door));
        assert!(all_set.contains(&LogCategory::Carrier));
        assert!(all_set.contains(&LogCategory::Magic));
        assert!(all_set.contains(&LogCategory::General));
        assert!(all_set.contains(&LogCategory::Movement));
        assert!(all_set.contains(&LogCategory::Ai));
        assert!(all_set.contains(&LogCategory::Rendering));
        assert!(all_set.contains(&LogCategory::Animation));
        assert!(all_set.contains(&LogCategory::Time));
        
        assert_eq!(all_set.len(), 13, "No duplicates in ALL");
    }

    #[test]
    fn test_default_enabled() {
        // First 8: ON by default
        assert!(LogCategory::Combat.default_enabled());
        assert!(LogCategory::Encounter.default_enabled());
        assert!(LogCategory::Quest.default_enabled());
        assert!(LogCategory::Npc.default_enabled());
        assert!(LogCategory::Door.default_enabled());
        assert!(LogCategory::Carrier.default_enabled());
        assert!(LogCategory::Magic.default_enabled());
        assert!(LogCategory::General.default_enabled());
        
        // Last 5: OFF by default (noisy)
        assert!(!LogCategory::Movement.default_enabled());
        assert!(!LogCategory::Ai.default_enabled());
        assert!(!LogCategory::Rendering.default_enabled());
        assert!(!LogCategory::Animation.default_enabled());
        assert!(!LogCategory::Time.default_enabled());
    }

    #[test]
    fn test_labels() {
        assert_eq!(LogCategory::Combat.label(), "COMBAT");
        assert_eq!(LogCategory::Encounter.label(), "ENCOUNTER");
        assert_eq!(LogCategory::Quest.label(), "QUEST");
        assert_eq!(LogCategory::Npc.label(), "NPC");
        assert_eq!(LogCategory::Door.label(), "DOOR");
        assert_eq!(LogCategory::Carrier.label(), "CARRIER");
        assert_eq!(LogCategory::Magic.label(), "MAGIC");
        assert_eq!(LogCategory::General.label(), "GENERAL");
        assert_eq!(LogCategory::Movement.label(), "MOVEMENT");
        assert_eq!(LogCategory::Ai.label(), "AI");
        assert_eq!(LogCategory::Rendering.label(), "RENDERING");
        assert_eq!(LogCategory::Animation.label(), "ANIMATION");
        assert_eq!(LogCategory::Time.label(), "TIME");
    }

    #[test]
    fn test_all_order_matches_spec() {
        // Verify the order matches DEBUG_SPECIFICATION.md
        assert_eq!(LogCategory::ALL[0], LogCategory::Combat);
        assert_eq!(LogCategory::ALL[1], LogCategory::Encounter);
        assert_eq!(LogCategory::ALL[2], LogCategory::Quest);
        assert_eq!(LogCategory::ALL[3], LogCategory::Npc);
        assert_eq!(LogCategory::ALL[4], LogCategory::Door);
        assert_eq!(LogCategory::ALL[5], LogCategory::Carrier);
        assert_eq!(LogCategory::ALL[6], LogCategory::Magic);
        assert_eq!(LogCategory::ALL[7], LogCategory::General);
        assert_eq!(LogCategory::ALL[8], LogCategory::Movement);
        assert_eq!(LogCategory::ALL[9], LogCategory::Ai);
        assert_eq!(LogCategory::ALL[10], LogCategory::Rendering);
        assert_eq!(LogCategory::ALL[11], LogCategory::Animation);
        assert_eq!(LogCategory::ALL[12], LogCategory::Time);
    }

    #[test]
    fn test_debug_log_macro_no_format_args() {
        let entry = crate::debug_log!(General, "paused");
        assert_eq!(entry.category, LogCategory::General);
        assert_eq!(entry.text, "paused");
        assert_eq!(entry.timestamp_ticks, 0);
    }

    #[test]
    fn test_debug_log_macro_with_format_args() {
        let dmg = 7;
        let entry = crate::debug_log!(Combat, "hero dealt {} dmg", dmg);
        assert_eq!(entry.category, LogCategory::Combat);
        assert_eq!(entry.text, "hero dealt 7 dmg");
    }

    #[test]
    fn test_debug_log_macro_multiple_format_args() {
        let entry = crate::debug_log!(Quest, "{} of {} complete", 3, 10);
        assert_eq!(entry.category, LogCategory::Quest);
        assert_eq!(entry.text, "3 of 10 complete");
    }

    #[test]
    fn test_debug_log_macro_trailing_comma() {
        let name = "goblin";
        let entry = crate::debug_log!(Encounter, "spawned {}", name,);
        assert_eq!(entry.category, LogCategory::Encounter);
        assert_eq!(entry.text, "spawned goblin");
    }
}
