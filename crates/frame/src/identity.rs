//! Node identity — every node on the network has one.
//!
//! A human at a TUI, a service on the bus, a bridge to Discord —
//! all nodes, all identified the same way.

use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Unique identity for any node on the network.
///
/// The `id` is stable across restarts — generated on first run and persisted.
/// The `name` is human-readable and used in logs, frames, and capability sets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeIdentity {
    /// Stable identifier, persisted across restarts.
    pub id: Ulid,

    /// Human-readable name: "gbe-oracle", "bear", "overseer-core".
    pub name: String,

    /// What kind of node this is.
    pub kind: NodeKind,

    /// Which domain this node belongs to: "gbe", "allthing", "akasha".
    pub domain: String,

    /// Instance discriminator — hostname, pod name, or similar.
    /// Distinguishes multiple instances of the same node.
    pub instance: String,
}

/// The kind of node. Not a hierarchy — just a descriptor so other nodes
/// know what they're talking to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// A human behind a TUI or other interface.
    Human,

    /// A long-running service (oracle, sentinel, watcher, overseer-core).
    Service,

    /// A bridge to an external system (Discord, XMPP, email).
    Bridge,

    /// An autonomous agent (LLM-powered or otherwise).
    Agent,
}

impl NodeIdentity {
    /// Create a new identity with a fresh ULID.
    pub fn new(
        name: impl Into<String>,
        kind: NodeKind,
        domain: impl Into<String>,
        instance: impl Into<String>,
    ) -> Self {
        Self {
            id: Ulid::new(),
            name: name.into(),
            kind,
            domain: domain.into(),
            instance: instance.into(),
        }
    }

    /// Reconstruct an identity with a known id (loaded from persisted config).
    pub fn with_id(
        id: Ulid,
        name: impl Into<String>,
        kind: NodeKind,
        domain: impl Into<String>,
        instance: impl Into<String>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            domain: domain.into(),
            instance: instance.into(),
        }
    }
}

impl std::fmt::Display for NodeIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.name, self.instance)
    }
}

impl std::fmt::Display for NodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeKind::Human => write!(f, "human"),
            NodeKind::Service => write!(f, "service"),
            NodeKind::Bridge => write!(f, "bridge"),
            NodeKind::Agent => write!(f, "agent"),
        }
    }
}
