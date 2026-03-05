//! Message queue: scrolling text viewport for status/event messages.

/// Maximum messages stored in the visible queue (original shows 4 lines).
pub const MSG_QUEUE_MAX: usize = 4;
/// Maximum characters per message line.
pub const MSG_LINE_MAX: usize = 40;

pub struct MessageQueue {
    lines: Vec<String>,
}

impl MessageQueue {
    pub fn new() -> Self {
        MessageQueue { lines: Vec::with_capacity(MSG_QUEUE_MAX + 1) }
    }

    /// Push a new message; oldest is dropped when queue is full.
    pub fn push(&mut self, msg: impl Into<String>) {
        let s = msg.into();
        // Truncate to MSG_LINE_MAX
        let s = if s.chars().count() > MSG_LINE_MAX {
            s.chars().take(MSG_LINE_MAX).collect()
        } else { s };
        self.lines.push(s);
        if self.lines.len() > MSG_QUEUE_MAX {
            self.lines.remove(0);
        }
    }

    /// Iterate messages oldest-first (top of scroll area).
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.lines.iter().map(|s| s.as_str())
    }

    /// Most recent message, if any.
    pub fn latest(&self) -> Option<&str> {
        self.lines.last().map(|s| s.as_str())
    }

    pub fn is_empty(&self) -> bool { self.lines.is_empty() }
    pub fn len(&self) -> usize { self.lines.len() }
    pub fn clear(&mut self) { self.lines.clear(); }
}

impl Default for MessageQueue {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_scroll() {
        let mut q = MessageQueue::new();
        for i in 0..MSG_QUEUE_MAX + 2 {
            q.push(format!("Message {}", i));
        }
        assert_eq!(q.len(), MSG_QUEUE_MAX);
        // oldest messages dropped
        assert!(!q.iter().any(|m| m.contains("Message 0")));
    }

    #[test]
    fn test_truncation() {
        let mut q = MessageQueue::new();
        q.push("a".repeat(MSG_LINE_MAX + 10));
        assert!(q.latest().unwrap().chars().count() <= MSG_LINE_MAX);
    }
}
