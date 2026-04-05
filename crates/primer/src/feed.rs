//! Feed — an ordered sequence of pages with navigation state.
//!
//! The feed is the first-class navigable object. Sequence navigation
//! (next/prev, top/bottom) traverses the feed. The ractor manages
//! what pages enter the feed (filtering, throttling). The mediatronic
//! reads the feed to know what to paint.

use crate::cell::Role;
use crate::page::Page;

/// An ordered buffer of pages with a navigation cursor.
///
/// Supports sequence navigation (next/prev/top/bottom) and
/// provides the selected page for spatial/semantic navigation
/// within its cell graph.
pub struct Feed {
    /// The ordered page buffer.
    pages: Vec<Page>,

    /// Current cursor position. `None` if the feed is empty.
    cursor: Option<usize>,

    /// Maximum number of pages to retain. Oldest pages are
    /// dropped when the limit is exceeded. Zero means no limit.
    capacity: usize,

    /// Whether the cursor should follow new pages (auto-scroll).
    /// Disabled when the user navigates away from the tail.
    follow: bool,
}

/// Result of a navigation action — tells the mediatronic what changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavResult {
    /// Cursor moved to a new position.
    Moved,
    /// Already at the boundary (top or bottom). No movement.
    AtBoundary,
    /// Feed is empty. Nothing to navigate.
    Empty,
}

impl Feed {
    /// Create a new feed with the given capacity.
    /// A capacity of zero means unlimited.
    pub fn new(capacity: usize) -> Self {
        Self {
            pages: Vec::new(),
            cursor: None,
            capacity,
            follow: true,
        }
    }

    /// Push a new page into the feed.
    ///
    /// If the feed is at capacity, the oldest page is dropped
    /// and the cursor is adjusted. If `follow` is true, the
    /// cursor moves to the new page.
    pub fn push(&mut self, page: Page) {
        self.pages.push(page);

        // Enforce capacity.
        if self.capacity > 0 && self.pages.len() > self.capacity {
            self.pages.remove(0);
            // Adjust cursor for the removed element.
            if let Some(ref mut c) = self.cursor {
                *c = c.saturating_sub(1);
            }
        }

        if self.follow || self.cursor.is_none() {
            self.cursor = Some(self.pages.len() - 1);
        }
    }

    /// The currently selected page, if any.
    pub fn current(&self) -> Option<&Page> {
        self.cursor.and_then(|i| self.pages.get(i))
    }

    /// The current cursor position.
    pub fn cursor_position(&self) -> Option<usize> {
        self.cursor
    }

    /// Total number of pages in the feed.
    pub fn len(&self) -> usize {
        self.pages.len()
    }

    /// Whether the feed is empty.
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }

    /// All pages in the feed, ordered.
    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    /// Whether the cursor is following new pages.
    pub fn is_following(&self) -> bool {
        self.follow
    }

    // -- Sequence navigation --

    /// Move cursor to the next page.
    pub fn next(&mut self) -> NavResult {
        match self.cursor {
            None => NavResult::Empty,
            Some(i) if i + 1 >= self.pages.len() => NavResult::AtBoundary,
            Some(i) => {
                self.cursor = Some(i + 1);
                self.follow = i + 1 == self.pages.len() - 1;
                NavResult::Moved
            }
        }
    }

    /// Move cursor to the previous page.
    pub fn prev(&mut self) -> NavResult {
        match self.cursor {
            None => NavResult::Empty,
            Some(0) => NavResult::AtBoundary,
            Some(i) => {
                self.cursor = Some(i - 1);
                self.follow = false;
                NavResult::Moved
            }
        }
    }

    /// Jump to the first page.
    pub fn top(&mut self) -> NavResult {
        if self.pages.is_empty() {
            return NavResult::Empty;
        }
        if self.cursor == Some(0) {
            return NavResult::AtBoundary;
        }
        self.cursor = Some(0);
        self.follow = false;
        NavResult::Moved
    }

    /// Jump to the last page and re-enable follow.
    pub fn bottom(&mut self) -> NavResult {
        if self.pages.is_empty() {
            return NavResult::Empty;
        }
        let last = self.pages.len() - 1;
        if self.cursor == Some(last) {
            return NavResult::AtBoundary;
        }
        self.cursor = Some(last);
        self.follow = true;
        NavResult::Moved
    }

    /// Re-enable follow mode (cursor tracks new pages).
    pub fn resume_follow(&mut self) {
        self.follow = true;
        if !self.pages.is_empty() {
            self.cursor = Some(self.pages.len() - 1);
        }
    }

    /// Pause follow mode (cursor stays put).
    pub fn pause_follow(&mut self) {
        self.follow = false;
    }

    // -- Filtering (ractor-like, for v0 without a real ractor) --

    /// Return an iterator over pages matching a content type filter.
    pub fn filter_by_content_type<'a>(&'a self, content_type: &'a str) -> impl Iterator<Item = (usize, &'a Page)> {
        self.pages
            .iter()
            .enumerate()
            .filter(move |(_, p)| p.index.content_type == content_type)
    }

    /// Summary: count of pages per content type.
    pub fn content_type_counts(&self) -> std::collections::HashMap<&str, usize> {
        let mut counts = std::collections::HashMap::new();
        for page in &self.pages {
            *counts.entry(page.index.content_type.as_str()).or_insert(0) += 1;
        }
        counts
    }
}

impl std::ops::Index<usize> for Feed {
    type Output = Page;

    fn index(&self, index: usize) -> &Self::Output {
        &self.pages[index]
    }
}

/// Navigate within a page's cell graph. This is spatial and semantic
/// navigation — movement within a single page, not across the feed.
pub struct CellNavigator<'a> {
    page: &'a Page,
    focused: Option<usize>,
}

impl<'a> CellNavigator<'a> {
    /// Create a navigator for a page, focused on the first heading cell
    /// (or the first cell if no heading exists).
    pub fn new(page: &'a Page) -> Self {
        let focused = page
            .cells
            .iter()
            .position(|c| c.role == Role::Heading)
            .or(if page.cells.is_empty() { None } else { Some(0) });

        Self { page, focused }
    }

    /// The currently focused cell.
    pub fn focused(&self) -> Option<&crate::cell::Cell> {
        self.focused.and_then(|i| self.page.cells.get(i))
    }

    /// Move to the next cell by spatial order (role priority: heading → summary → detail → status → action).
    pub fn next_spatial(&mut self) -> NavResult {
        match self.focused {
            None => NavResult::Empty,
            Some(i) if i + 1 >= self.page.cells.len() => NavResult::AtBoundary,
            Some(i) => {
                self.focused = Some(i + 1);
                NavResult::Moved
            }
        }
    }

    /// Move to the previous cell by spatial order.
    pub fn prev_spatial(&mut self) -> NavResult {
        match self.focused {
            None => NavResult::Empty,
            Some(0) => NavResult::AtBoundary,
            Some(i) => {
                self.focused = Some(i - 1);
                NavResult::Moved
            }
        }
    }

    /// Drill into the focused cell — follow the first `DetailOf` link
    /// from any cell that links to this one as its summary.
    pub fn drill_down(&mut self) -> NavResult {
        let Some(focused) = self.focused() else {
            return NavResult::Empty;
        };
        let focused_id = focused.id;

        // Find a cell whose link says "I am detail-of the focused cell."
        let detail_idx = self.page.cells.iter().position(|c| {
            c.links.iter().any(|link| {
                link.target == focused_id && link.kind == crate::cell::LinkKind::DetailOf
            })
        });

        match detail_idx {
            Some(idx) => {
                self.focused = Some(idx);
                NavResult::Moved
            }
            None => NavResult::AtBoundary,
        }
    }

    /// Drill up from the focused cell — follow its `DetailOf` link
    /// back to the cell it's a detail of.
    pub fn drill_up(&mut self) -> NavResult {
        let Some(focused) = self.focused() else {
            return NavResult::Empty;
        };

        // Find the DetailOf link on the focused cell.
        let summary_id = focused.links.iter().find_map(|link| {
            if link.kind == crate::cell::LinkKind::DetailOf {
                Some(link.target)
            } else {
                None
            }
        });

        match summary_id {
            Some(target) => {
                let idx = self.page.cells.iter().position(|c| c.id == target);
                match idx {
                    Some(i) => {
                        self.focused = Some(i);
                        NavResult::Moved
                    }
                    None => NavResult::AtBoundary,
                }
            }
            None => NavResult::AtBoundary,
        }
    }
}
