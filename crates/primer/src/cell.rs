//! Cell — the atomic unit of presentation.
//!
//! A cell carries typed content with semantic metadata. It doesn't know
//! how it will be rendered — that's the mediatronic's job. It knows
//! what it *is* (role), what it *contains* (typed content), and what
//! it's *related to* (links).

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::content::TypedContent;

/// The atomic unit of the primer domain.
///
/// Cells are produced by matter compiler imprints (shape match → field
/// projection → cell) and consumed by mediatronics (role → layout position).
/// Navigation through cells uses the link graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    /// Unique identifier. Used for linking between cells.
    pub id: Ulid,

    /// Semantic role — determines visual treatment and navigation behavior.
    pub role: Role,

    /// Structured content. The consumer renders this according to its capabilities.
    pub content: TypedContent,

    /// Links to other cells. The link graph is the navigation structure.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<CellLink>,

    /// Display priority. Higher = more important. Mediatronics use this
    /// when they can't show everything (small screens hide low-priority cells).
    #[serde(default)]
    pub priority: i32,
}

/// Semantic role of a cell. Determines visual treatment in the
/// mediatronic's layout imprint and navigation ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Primary identifier — the thing you see first.
    Heading,
    /// Compact overview — key facts at a glance.
    Summary,
    /// Full content — expandable, scrollable.
    Detail,
    /// Compact state indicator.
    Status,
    /// Something the user can interact with.
    Action,
}

/// A directional link between cells. Links are navigation affordances —
/// they define how a user moves through the cell graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellLink {
    /// The target cell.
    pub target: Ulid,
    /// The kind of relationship.
    pub kind: LinkKind,
}

/// The relationship between linked cells. Each kind maps to a
/// navigation axis:
///
/// - `DetailOf` / `SummaryOf` — semantic drill (enter/escape)
/// - `PeerOf` — lateral movement within a group
/// - `Sequence` — ordered traversal (next/prev within a list)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    /// "I am a deeper view of the target." Drill down.
    DetailOf,
    /// "I am a condensed view of the target." Drill up.
    SummaryOf,
    /// "I am a sibling of the target." Lateral.
    PeerOf,
    /// "I follow the target in an ordered group." Next in sequence.
    Sequence,
}

impl Cell {
    /// Create a new cell with the given role and content.
    pub fn new(role: Role, content: TypedContent) -> Self {
        Self {
            id: Ulid::new(),
            role,
            content,
            links: Vec::new(),
            priority: 0,
        }
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Add a link to another cell.
    pub fn link_to(mut self, target: Ulid, kind: LinkKind) -> Self {
        self.links.push(CellLink { target, kind });
        self
    }
}
