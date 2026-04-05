//! Authority — carried in frames, not enforced by a separate system.
//!
//! A writ carries the authority of the human who issued it.
//! A mandate inherits that authority. The chain is traceable
//! through the frame stack.

use serde::{Deserialize, Serialize};

use crate::identity::NodeIdentity;

/// Authority context carried in a frame.
///
/// Authority is metadata, not a gate. It stacks like any other frame.
/// The packet's authority history is part of the packet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityFrame {
    /// The authority level granted.
    pub level: AuthorityLevel,

    /// The node that issued or delegated this authority.
    pub issuer: NodeIdentity,

    /// What this authority covers — a domain, a resource, or "*" for unrestricted.
    pub scope: String,
}

/// Authority levels. Not a hierarchy of importance — a description of
/// what kind of access the issuer has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityLevel {
    /// Guest or limited access.
    Pilgrim,

    /// Full administrative authority.
    Consul,
}
