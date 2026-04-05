//! Authority flow — dispatch, writ, mandate.
//!
//! These are not packet types. They are packets with specific framing
//! patterns that carry authority context.

use serde::{Deserialize, Serialize};

use crate::authority::AuthorityFrame;
use crate::identity::NodeIdentity;
use crate::packet::Packet;

/// Human-to-human communication. No authority gradient.
///
/// A dispatch is a packet where both origin and target are human nodes.
/// Peer communication — a message between equals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dispatch {
    /// The underlying packet.
    pub packet: Packet,

    /// Who sent it.
    pub from: NodeIdentity,

    /// Who it's for. None means broadcast (fatline).
    pub to: Option<NodeIdentity>,
}

/// Human-to-machine directive. Carries the authority of the issuing human.
///
/// A writ is the structured output of a human's intent — either typed
/// directly (v0 DSL) or parsed by an agent node (v1 LLM). It targets
/// a specific capability on a specific node.
///
/// Oracle receives writs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Writ {
    /// The underlying packet.
    pub packet: Packet,

    /// The authority this writ carries.
    pub authority: AuthorityFrame,

    /// The target capability: "create-job", "cancel-job", etc.
    pub capability: String,

    /// The target node (or domain, if any node with this capability can handle it).
    pub target: WritTarget,
}

/// Where a writ is directed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WritTarget {
    /// A specific node by identity.
    Node(NodeIdentity),

    /// Any node in a domain that offers the named capability.
    Domain(String),
}

/// Response to a writ. Published by the node that handled the writ.
///
/// The response carries the writ_id for correlation and a status
/// indicating whether the capability was executed successfully.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritResponse {
    /// The writ this is responding to.
    pub writ_id: ulid::Ulid,

    /// Who is responding.
    pub responder: NodeIdentity,

    /// Whether the capability executed successfully.
    pub status: WritStatus,

    /// Response payload — capability-specific data.
    pub data: serde_json::Value,
}

/// Status of a writ execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WritStatus {
    /// Capability executed successfully.
    Ok,
    /// Capability execution failed.
    Error,
    /// Capability not found or not supported by this node.
    Unsupported,
    /// Insufficient authority to execute this capability.
    Denied,
}

/// Machine-to-machine directive. Emitted by Oracle after validating a writ.
///
/// A mandate inherits the authority of the writ that spawned it.
/// It carries the actual work to be done — DAGs, tasks, standing orders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mandate {
    /// The underlying packet.
    pub packet: Packet,

    /// Authority inherited from the originating writ.
    pub authority: AuthorityFrame,

    /// Reference to the writ that spawned this mandate.
    pub writ_id: ulid::Ulid,
}
