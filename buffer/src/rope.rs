//! Rope buffer - Seekable, mutable storage for file editing
//!
//! Simplified rope implementation using a vector of strings.
//! For production, consider xi-rope or ropey crates.

use crate::{BufferError, Position, Range, ViewWindow};

/// Rope buffer for file editing (seekable, mutable)
#[derive(Debug, Clone)]
pub struct RopeBuffer {
    /// Content stored as a single string for simplicity
    /// TODO: Use proper rope data structure for large files
    content: String,
}

impl RopeBuffer {
    /// Create a new empty rope buffer
    pub fn new() -> Self {
        Self {
            content: String::new(),
        }
    }

    /// Create a rope buffer from initial content
    pub fn with_content(content: &str) -> Self {
        Self {
            content: content.to_string(),
        }
    }

    /// Get the total length in bytes
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Insert text at a position
    pub fn insert(&mut self, pos: Position, text: &str) -> Result<(), BufferError> {
        if pos > self.content.len() {
            return Err(BufferError::OutOfBounds(pos));
        }

        self.content.insert_str(pos, text);
        Ok(())
    }

    /// Delete a range of text
    pub fn delete(&mut self, range: Range) -> Result<(), BufferError> {
        if range.start > range.end {
            return Err(BufferError::InvalidRange(range));
        }
        if range.end > self.content.len() {
            return Err(BufferError::OutOfBounds(range.end));
        }

        self.content.drain(range);
        Ok(())
    }

    /// Replace a range with new text
    pub fn replace(&mut self, range: Range, text: &str) -> Result<(), BufferError> {
        self.delete(range.clone())?;
        self.insert(range.start, text)?;
        Ok(())
    }

    /// Get a slice of content
    pub fn slice(&self, range: Range) -> Result<&str, BufferError> {
        if range.start > range.end {
            return Err(BufferError::InvalidRange(range.clone()));
        }
        if range.end > self.content.len() {
            return Err(BufferError::OutOfBounds(range.end));
        }

        Ok(&self.content[range])
    }

    /// Get entire content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get a view window (line-based)
    pub fn view(&self, window: ViewWindow) -> Vec<String> {
        let lines: Vec<&str> = self.content.lines().collect();

        let start = window.start_line.min(lines.len());
        let end = (window.start_line + window.count).min(lines.len());

        lines[start..end].iter().map(|s| s.to_string()).collect()
    }

    /// Get line count
    pub fn line_count(&self) -> usize {
        if self.content.is_empty() {
            0
        } else {
            self.content.lines().count()
        }
    }

    /// Get a specific line (0-indexed)
    pub fn line(&self, line_num: usize) -> Option<String> {
        self.content.lines().nth(line_num).map(|s| s.to_string())
    }

    /// Find the byte position of a line
    pub fn line_to_byte(&self, line_num: usize) -> Option<Position> {
        let mut pos = 0;
        for (idx, line) in self.content.lines().enumerate() {
            if idx == line_num {
                return Some(pos);
            }
            pos += line.len() + 1; // +1 for newline
        }
        None
    }

    /// Find the line number of a byte position
    pub fn byte_to_line(&self, pos: Position) -> Option<usize> {
        if pos > self.content.len() {
            return None;
        }

        let mut byte_count = 0;
        for (line_num, line) in self.content.lines().enumerate() {
            if byte_count + line.len() >= pos {
                return Some(line_num);
            }
            byte_count += line.len() + 1; // +1 for newline
        }

        Some(self.line_count().saturating_sub(1))
    }

    /// Clear all content
    pub fn clear(&mut self) {
        self.content.clear();
    }
}

impl Default for RopeBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buf = RopeBuffer::new();
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_with_content() {
        let buf = RopeBuffer::with_content("hello world");
        assert_eq!(buf.len(), 11);
        assert_eq!(buf.content(), "hello world");
    }

    #[test]
    fn test_insert() {
        let mut buf = RopeBuffer::new();
        buf.insert(0, "hello").unwrap();
        buf.insert(5, " world").unwrap();
        assert_eq!(buf.content(), "hello world");
    }

    #[test]
    fn test_insert_middle() {
        let mut buf = RopeBuffer::with_content("hello world");
        buf.insert(5, " awesome").unwrap();
        assert_eq!(buf.content(), "hello awesome world");
    }

    #[test]
    fn test_insert_out_of_bounds() {
        let mut buf = RopeBuffer::with_content("hello");
        let result = buf.insert(10, "x");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete() {
        let mut buf = RopeBuffer::with_content("hello world");
        buf.delete(5..11).unwrap();
        assert_eq!(buf.content(), "hello");
    }

    #[test]
    fn test_delete_middle() {
        let mut buf = RopeBuffer::with_content("hello awesome world");
        buf.delete(5..13).unwrap(); // Delete " awesome" (not including final space)
        assert_eq!(buf.content(), "hello world");
    }

    #[test]
    fn test_delete_invalid_range() {
        let mut buf = RopeBuffer::with_content("hello");
        let result = buf.delete(3..2);
        assert!(result.is_err());
    }

    #[test]
    fn test_replace() {
        let mut buf = RopeBuffer::with_content("hello world");
        buf.replace(6..11, "rust").unwrap();
        assert_eq!(buf.content(), "hello rust");
    }

    #[test]
    fn test_slice() {
        let buf = RopeBuffer::with_content("hello world");
        let slice = buf.slice(0..5).unwrap();
        assert_eq!(slice, "hello");
    }

    #[test]
    fn test_view_window() {
        let buf = RopeBuffer::with_content("line 1\nline 2\nline 3\nline 4");
        let view = buf.view(ViewWindow::new(1, 2));
        assert_eq!(view, vec!["line 2", "line 3"]);
    }

    #[test]
    fn test_view_window_overflow() {
        let buf = RopeBuffer::with_content("line 1\nline 2");
        let view = buf.view(ViewWindow::new(0, 10));
        assert_eq!(view.len(), 2);
    }

    #[test]
    fn test_line_count() {
        let buf = RopeBuffer::with_content("line 1\nline 2\nline 3");
        assert_eq!(buf.line_count(), 3);
    }

    #[test]
    fn test_line_count_empty() {
        let buf = RopeBuffer::new();
        assert_eq!(buf.line_count(), 0);
    }

    #[test]
    fn test_get_line() {
        let buf = RopeBuffer::with_content("line 1\nline 2\nline 3");
        assert_eq!(buf.line(0).unwrap(), "line 1");
        assert_eq!(buf.line(1).unwrap(), "line 2");
        assert_eq!(buf.line(2).unwrap(), "line 3");
        assert!(buf.line(3).is_none());
    }

    #[test]
    fn test_line_to_byte() {
        let buf = RopeBuffer::with_content("line 1\nline 2\nline 3");
        assert_eq!(buf.line_to_byte(0).unwrap(), 0);
        assert_eq!(buf.line_to_byte(1).unwrap(), 7); // "line 1\n"
        assert_eq!(buf.line_to_byte(2).unwrap(), 14); // "line 1\nline 2\n"
    }

    #[test]
    fn test_byte_to_line() {
        let buf = RopeBuffer::with_content("line 1\nline 2\nline 3");
        assert_eq!(buf.byte_to_line(0).unwrap(), 0);
        assert_eq!(buf.byte_to_line(7).unwrap(), 1);
        assert_eq!(buf.byte_to_line(14).unwrap(), 2);
    }

    #[test]
    fn test_clear() {
        let mut buf = RopeBuffer::with_content("hello world");
        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }
}
