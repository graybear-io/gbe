//! GBE Buffer - Storage layer for rope and ring buffers
//!
//! Provides two buffer types:
//! - **RopeBuffer**: Seekable, mutable storage for file editing
//! - **RingBuffer**: Fixed-size, append-only storage for streams
//!
//! # Examples
//!
//! ## RopeBuffer (for files)
//!
//! ```
//! use gbe_buffer::RopeBuffer;
//!
//! let mut buf = RopeBuffer::new();
//! buf.insert(0, "hello world");
//! buf.insert(5, " awesome");  // "hello awesome world"
//! buf.delete(6..13);          // "hello world"
//! ```
//!
//! ## RingBuffer (for streams)
//!
//! ```
//! use gbe_buffer::{RingBuffer, ViewWindow};
//!
//! let mut buf = RingBuffer::new(1024);  // 1024 lines capacity
//! buf.push("line 1\n");
//! buf.push("line 2\n");
//! let lines = buf.view(ViewWindow::new(0, 2));  // Get first 2 lines
//! ```

mod ring;
mod rope;

pub use ring::RingBuffer;
pub use rope::RopeBuffer;

/// Position in buffer (byte offset)
pub type Position = usize;

/// Range in buffer [start, end)
pub type Range = std::ops::Range<usize>;

/// View window request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewWindow {
    /// Starting line number (0-indexed)
    pub start_line: usize,
    /// Number of lines to retrieve
    pub count: usize,
}

impl ViewWindow {
    /// Create a new view window
    pub fn new(start_line: usize, count: usize) -> Self {
        Self { start_line, count }
    }
}

/// Buffer errors
#[derive(Debug, thiserror::Error)]
pub enum BufferError {
    #[error("Position out of bounds: {0}")]
    OutOfBounds(Position),

    #[error("Invalid range: {0:?}")]
    InvalidRange(Range),

    #[error("Buffer full: capacity {0}")]
    BufferFull(usize),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}
