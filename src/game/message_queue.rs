//! Message queue: scrolling text viewport for status/event messages.

use std::borrow::Cow;

/// Maximum messages stored in the visible queue (original shows 4 lines).
pub const MSG_QUEUE_MAX: usize = 4;
/// Maximum characters per message line.
pub const MSG_LINE_MAX: usize = 40;

pub struct MessageQueue {
    lines: Vec<String>,
    /// Full ordered history of every message (the story transcript).
    transcript: Vec<String>,
    /// When true, each new message is also printed to stdout.
    echo: bool,
    /// Optional `%` substitution string (current brother name).
    substitution: Option<String>,
}

impl MessageQueue {
    pub fn new() -> Self {
        MessageQueue {
            lines: Vec::with_capacity(MSG_QUEUE_MAX + 1),
            transcript: Vec::new(),
            echo: false,
            substitution: None,
        }
    }

    /// Enable or disable echoing each new message to stdout.
    pub fn set_echo(&mut self, echo: bool) {
        self.echo = echo;
    }

    /// Return the full story transcript (all messages ever pushed).
    pub fn transcript(&self) -> &[String] {
        &self.transcript
    }

    /// Set the `%` substitution string (current brother name).
    pub fn set_substitution(&mut self, name: impl Into<String>) {
        self.substitution = Some(name.into());
    }

    /// Replace the transcript with a previously saved one (call after loading a game).
    pub fn set_transcript(&mut self, saved: Vec<String>) {
        self.transcript = saved;
    }

    /// Push a new message; oldest is dropped when queue is full.
    /// The message is always appended to the transcript.
    pub fn push(&mut self, msg: impl Into<String>) {
        self.print(msg.into());
    }

    /// Push a message, word-wrapping across multiple lines if it exceeds
    /// [`MSG_LINE_MAX`].  The original Amiga `extract()` / `print()` system
    /// word-wrapped long event text the same way.
    pub fn push_wrapped(&mut self, msg: impl Into<String>) {
        let s = msg.into();
        let s = self.apply_substitution(&s);
        for segment in s.split(|c| c == '\n' || c == '\r') {
            if segment.chars().count() <= MSG_LINE_MAX {
                self.print(segment);
            } else {
                for line in wrap_words(segment, MSG_LINE_MAX) {
                    self.print(line);
                }
            }
        }
    }

    /// Print a new line, scrolling the viewport up by one line.
    pub fn print(&mut self, msg: impl AsRef<str>) {
        let s = self.apply_substitution(msg.as_ref());
        let s = truncate_line(&s);
        if self.echo {
            println!("[transcript] {}", s);
        }
        self.transcript.push(s.clone());
        self.lines.push(s);
        if self.lines.len() > MSG_QUEUE_MAX {
            self.lines.remove(0);
        }
    }

    /// Append to the current line without scrolling.
    pub fn print_cont(&mut self, msg: impl AsRef<str>) {
        if self.lines.is_empty() {
            self.print(msg);
            return;
        }
        let s = self.apply_substitution(msg.as_ref());
        let idx = self.lines.len() - 1;
        let mut updated = self.lines[idx].clone();
        updated.push_str(&s);
        updated = truncate_line(&updated);
        self.lines[idx] = updated.clone();
        if let Some(last) = self.transcript.last_mut() {
            *last = updated;
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

    fn apply_substitution<'a>(&self, msg: &'a str) -> Cow<'a, str> {
        match self.substitution.as_deref() {
            Some(sub) if msg.contains('%') => Cow::Owned(msg.replace('%', sub)),
            _ => Cow::Borrowed(msg),
        }
    }
}

/// Word-wrap `text` into lines no longer than `max` characters, breaking on
/// whitespace boundaries.  Falls back to hard truncation when a single word
/// exceeds `max`.
fn wrap_words(text: &str, max: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if line.is_empty() {
            if word.len() > max {
                lines.push(word.chars().take(max).collect());
            } else {
                line.push_str(word);
            }
        } else if line.len() + 1 + word.len() > max {
            lines.push(std::mem::take(&mut line));
            if word.len() > max {
                lines.push(word.chars().take(max).collect());
            } else {
                line.push_str(word);
            }
        } else {
            line.push(' ');
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn truncate_line(text: &str) -> String {
    if text.chars().count() > MSG_LINE_MAX {
        text.chars().take(MSG_LINE_MAX).collect()
    } else {
        text.to_string()
    }
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

    #[test]
    fn test_push_wrapped_splits_long_message() {
        let mut q = MessageQueue::new();
        q.push_wrapped(
            "Julian started the journey in his home village of Tambry."
        );
        // Should be split into two lines, both <= 40 chars
        assert_eq!(q.len(), 2);
        for msg in q.iter() {
            assert!(msg.chars().count() <= MSG_LINE_MAX,
                "line too long: {:?} ({})", msg, msg.chars().count());
        }
    }

    #[test]
    fn test_push_wrapped_short_message_stays_single() {
        let mut q = MessageQueue::new();
        q.push_wrapped("Hello world.");
        assert_eq!(q.len(), 1);
        assert_eq!(q.latest().unwrap(), "Hello world.");
    }

    #[test]
    fn test_wrap_words() {
        let lines = wrap_words("Julian started the journey in his home village of Tambry.", 40);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Julian started the journey in his home");
        assert_eq!(lines[1], "village of Tambry.");
    }

    #[test]
    fn test_print_cont_appends_without_scroll() {
        let mut q = MessageQueue::new();
        q.print("Hello");
        q.print_cont(", world");
        assert_eq!(q.len(), 1);
        assert_eq!(q.latest().unwrap(), "Hello, world");
    }

    #[test]
    fn test_print_substitutes_brother_name() {
        let mut q = MessageQueue::new();
        q.set_substitution("Phillip");
        q.print("% was here.");
        assert_eq!(q.latest().unwrap(), "Phillip was here.");
    }
}
