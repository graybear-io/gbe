//! Frame — metadata added at any hop. Frames stack.
//!
//! The payload never changes. The frames are the history.

use serde::{Deserialize, Serialize};

use crate::authority::AuthorityFrame;
use crate::identity::NodeIdentity;

/// Metadata added to a packet at a hop. Frames stack — the packet
/// accumulates its history as it moves through the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    /// What kind of hop this frame represents.
    pub kind: FrameKind,

    /// The node that added this frame.
    pub node: NodeIdentity,

    /// When this frame was added (unix milliseconds).
    pub timestamp: u64,

    /// Additional metadata specific to this frame kind.
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

/// What kind of hop a frame represents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameKind {
    /// Where the packet was created.
    Origin,

    /// A transport hop — what Envelope currently represents.
    /// Carries subject, trace_id, and transport-specific metadata.
    Transport,

    /// A barrier crossing between domains. A sheath.
    /// Carries origin domain, destination domain, crossing context.
    Barrier {
        from_domain: String,
        to_domain: String,
    },

    /// Authority context — writ issuance, mandate delegation.
    Authority(AuthorityFrame),

    /// An indexing event (akasha touched this packet).
    Index,
}
