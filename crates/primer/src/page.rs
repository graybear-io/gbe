//! Page — a collection of cells with an index.
//!
//! Pages are the unit of transport through a feed. A matter compiler
//! bundles related cells into a page, attaches an index for ractor
//! routing, and emits it.

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::cell::Cell;

/// A collection of related cells with self-describing metadata.
///
/// The index is what the ractor reads to decide routing.
/// The cells are what the mediatronic reads to paint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// Unique identifier for this page.
    pub id: Ulid,

    /// The cells that make up this page's content.
    pub cells: Vec<Cell>,

    /// Self-describing metadata for routing and layout decisions.
    pub index: PageIndex,
}

/// A page's self-description. Read by ractors for routing and
/// by mediatronics for layout constraint checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageIndex {
    /// What kind of content this page represents.
    /// Used for filtering — e.g. "writ", "lifecycle", "gather-result".
    pub content_type: String,

    /// The source this page was compiled from (subject pattern).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Which imprint compiled this page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiled_by: Option<String>,

    /// When this page was created (unix milliseconds).
    /// Set by the matter compiler at compilation time.
    #[serde(default)]
    pub timestamp: u64,

    /// Minimum display width (columns) for this page to be useful.
    #[serde(default)]
    pub min_width: u16,

    /// How often this page updates, in milliseconds.
    /// Zero means it's a one-shot page that won't change.
    #[serde(default)]
    pub refresh_ms: u64,
}

impl Page {
    /// Create a new page with the given cells and content type.
    pub fn new(cells: Vec<Cell>, content_type: impl Into<String>) -> Self {
        Self {
            id: Ulid::new(),
            cells,
            index: PageIndex {
                content_type: content_type.into(),
                source: None,
                compiled_by: None,
                timestamp: 0,
                min_width: 0,
                refresh_ms: 0,
            },
        }
    }

    /// Set the source subject pattern.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.index.source = Some(source.into());
        self
    }

    /// Set the page timestamp (unix milliseconds).
    pub fn with_timestamp(mut self, ts: u64) -> Self {
        self.index.timestamp = ts;
        self
    }

    /// Set the minimum display width.
    pub fn with_min_width(mut self, width: u16) -> Self {
        self.index.min_width = width;
        self
    }

    /// Set the refresh interval.
    pub fn with_refresh_ms(mut self, ms: u64) -> Self {
        self.index.refresh_ms = ms;
        self
    }

    /// Find a cell by id.
    pub fn cell(&self, id: ulid::Ulid) -> Option<&Cell> {
        self.cells.iter().find(|c| c.id == id)
    }

    /// All cells with a given role.
    pub fn cells_by_role(&self, role: crate::cell::Role) -> Vec<&Cell> {
        self.cells.iter().filter(|c| c.role == role).collect()
    }
}
