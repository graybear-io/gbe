//! Ring buffer - Fixed-size, append-only storage for streams
//!
//! Circular buffer with bounded memory for log streams and
//! continuous output.

use crate::ViewWindow;
use std::collections::VecDeque;

/// Ring buffer for streams (fixed size, append-only)
#[derive(Debug, Clone)]
pub struct RingBuffer {
    /// Maximum number of lines to store
    capacity: usize,
    /// Stored lines
    lines: VecDeque<String>,
    /// Total lines ever pushed (for tracking)
    total_pushed: usize,
}

impl RingBuffer {
    /// Create a new ring buffer with specified capacity (number of lines).
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is zero.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity != 0, "Ring buffer capacity must be > 0");

        Self {
            capacity,
            lines: VecDeque::with_capacity(capacity),
            total_pushed: 0,
        }
    }

    /// Get the capacity
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get current number of lines stored
    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Check if buffer is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Check if buffer is at capacity
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.lines.len() >= self.capacity
    }

    /// Get total lines ever pushed (including evicted ones)
    #[must_use]
    pub fn total_pushed(&self) -> usize {
        self.total_pushed
    }

    /// Push a new line (oldest line is evicted if at capacity)
    pub fn push(&mut self, line: impl Into<String>) {
        let line = line.into();

        if self.lines.len() >= self.capacity {
            self.lines.pop_front();
        }

        self.lines.push_back(line);
        self.total_pushed += 1;
    }

    /// Push multiple lines at once
    pub fn push_lines(&mut self, lines: impl IntoIterator<Item = String>) {
        for line in lines {
            self.push(line);
        }
    }

    /// Get a specific line by index (0 = oldest line in buffer)
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(std::string::String::as_str)
    }

    /// Get all lines as a vector
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        self.lines.iter().cloned().collect()
    }

    /// Get a view window of lines
    #[must_use]
    pub fn view(&self, window: ViewWindow) -> Vec<String> {
        let start = window.start_line.min(self.lines.len());
        let end = (window.start_line + window.count).min(self.lines.len());

        self.lines.range(start..end).cloned().collect()
    }

    /// Get the last N lines
    #[must_use]
    pub fn tail(&self, n: usize) -> Vec<String> {
        let start = self.lines.len().saturating_sub(n);
        self.lines.iter().skip(start).cloned().collect()
    }

    /// Get the first N lines
    #[must_use]
    pub fn head(&self, n: usize) -> Vec<String> {
        self.lines.iter().take(n).cloned().collect()
    }

    /// Clear all lines
    pub fn clear(&mut self) {
        self.lines.clear();
        self.total_pushed = 0;
    }

    /// Search for lines containing a pattern
    #[must_use]
    pub fn search(&self, pattern: &str) -> Vec<(usize, String)> {
        self.lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.contains(pattern))
            .map(|(idx, line)| (idx, line.clone()))
            .collect()
    }

    /// Get the oldest line
    #[must_use]
    pub fn oldest(&self) -> Option<&str> {
        self.lines.front().map(std::string::String::as_str)
    }

    /// Get the newest line
    #[must_use]
    pub fn newest(&self) -> Option<&str> {
        self.lines.back().map(std::string::String::as_str)
    }

    /// Resize the buffer capacity (may truncate if new capacity < current size).
    ///
    /// # Panics
    ///
    /// Panics if `new_capacity` is zero.
    pub fn resize(&mut self, new_capacity: usize) {
        assert!(new_capacity != 0, "Ring buffer capacity must be > 0");

        self.capacity = new_capacity;

        // Truncate from front if over capacity
        while self.lines.len() > self.capacity {
            self.lines.pop_front();
        }

        // Shrink allocation if needed
        self.lines.shrink_to_fit();
    }

    /// Get memory usage estimate (bytes)
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        self.lines
            .iter()
            .map(std::string::String::len)
            .sum::<usize>()
            + self.lines.capacity() * std::mem::size_of::<String>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buf = RingBuffer::new(10);
        assert_eq!(buf.capacity(), 10);
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
        assert!(!buf.is_full());
    }

    #[test]
    #[should_panic(expected = "capacity must be > 0")]
    fn test_zero_capacity_panics() {
        let _ = RingBuffer::new(0);
    }

    #[test]
    fn test_push() {
        let mut buf = RingBuffer::new(3);
        buf.push("line 1");
        buf.push("line 2");
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.lines(), vec!["line 1", "line 2"]);
    }

    #[test]
    fn test_push_eviction() {
        let mut buf = RingBuffer::new(3);
        buf.push("line 1");
        buf.push("line 2");
        buf.push("line 3");
        buf.push("line 4"); // Should evict "line 1"

        assert_eq!(buf.len(), 3);
        assert_eq!(buf.lines(), vec!["line 2", "line 3", "line 4"]);
        assert!(buf.is_full());
    }

    #[test]
    fn test_total_pushed() {
        let mut buf = RingBuffer::new(2);
        buf.push("line 1");
        buf.push("line 2");
        buf.push("line 3");
        buf.push("line 4");

        assert_eq!(buf.total_pushed(), 4);
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn test_push_lines() {
        let mut buf = RingBuffer::new(5);
        buf.push_lines(vec![
            "line 1".to_string(),
            "line 2".to_string(),
            "line 3".to_string(),
        ]);

        assert_eq!(buf.len(), 3);
        assert_eq!(buf.lines(), vec!["line 1", "line 2", "line 3"]);
    }

    #[test]
    fn test_get() {
        let mut buf = RingBuffer::new(5);
        buf.push("line 1");
        buf.push("line 2");
        buf.push("line 3");

        assert_eq!(buf.get(0), Some("line 1"));
        assert_eq!(buf.get(1), Some("line 2"));
        assert_eq!(buf.get(2), Some("line 3"));
        assert_eq!(buf.get(3), None);
    }

    #[test]
    fn test_view_window() {
        let mut buf = RingBuffer::new(10);
        for i in 1..=5 {
            buf.push(format!("line {i}"));
        }

        let view = buf.view(ViewWindow::new(1, 2));
        assert_eq!(view, vec!["line 2", "line 3"]);
    }

    #[test]
    fn test_view_window_overflow() {
        let mut buf = RingBuffer::new(10);
        buf.push("line 1");
        buf.push("line 2");

        let view = buf.view(ViewWindow::new(0, 10));
        assert_eq!(view.len(), 2);
    }

    #[test]
    fn test_tail() {
        let mut buf = RingBuffer::new(10);
        for i in 1..=5 {
            buf.push(format!("line {i}"));
        }

        let tail = buf.tail(2);
        assert_eq!(tail, vec!["line 4", "line 5"]);
    }

    #[test]
    fn test_tail_more_than_available() {
        let mut buf = RingBuffer::new(10);
        buf.push("line 1");
        buf.push("line 2");

        let tail = buf.tail(10);
        assert_eq!(tail, vec!["line 1", "line 2"]);
    }

    #[test]
    fn test_head() {
        let mut buf = RingBuffer::new(10);
        for i in 1..=5 {
            buf.push(format!("line {i}"));
        }

        let head = buf.head(2);
        assert_eq!(head, vec!["line 1", "line 2"]);
    }

    #[test]
    fn test_oldest_newest() {
        let mut buf = RingBuffer::new(3);
        buf.push("line 1");
        buf.push("line 2");
        buf.push("line 3");

        assert_eq!(buf.oldest(), Some("line 1"));
        assert_eq!(buf.newest(), Some("line 3"));

        buf.push("line 4"); // Evicts "line 1"
        assert_eq!(buf.oldest(), Some("line 2"));
        assert_eq!(buf.newest(), Some("line 4"));
    }

    #[test]
    fn test_search() {
        let mut buf = RingBuffer::new(10);
        buf.push("ERROR: something wrong");
        buf.push("INFO: all good");
        buf.push("ERROR: another issue");
        buf.push("DEBUG: trace info");

        let results = buf.search("ERROR");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 0);
        assert_eq!(results[1].0, 2);
    }

    #[test]
    fn test_clear() {
        let mut buf = RingBuffer::new(5);
        buf.push("line 1");
        buf.push("line 2");

        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.total_pushed(), 0);
    }

    #[test]
    fn test_resize_smaller() {
        let mut buf = RingBuffer::new(5);
        for i in 1..=5 {
            buf.push(format!("line {i}"));
        }

        buf.resize(3);
        assert_eq!(buf.capacity(), 3);
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.lines(), vec!["line 3", "line 4", "line 5"]);
    }

    #[test]
    fn test_resize_larger() {
        let mut buf = RingBuffer::new(3);
        buf.push("line 1");
        buf.push("line 2");

        buf.resize(10);
        assert_eq!(buf.capacity(), 10);
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.lines(), vec!["line 1", "line 2"]);
    }

    #[test]
    fn test_memory_usage() {
        let mut buf = RingBuffer::new(10);
        buf.push("test");
        buf.push("data");

        let usage = buf.memory_usage();
        assert!(usage > 0);
    }
}
