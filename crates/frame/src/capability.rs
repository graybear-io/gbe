//! Capability declarations — what a node can do, in machine-readable form.
//!
//! Every node publishes its capabilities so other nodes (and humans)
//! know what writs it can accept.

use serde::{Deserialize, Serialize};

use crate::authority::AuthorityLevel;
use crate::identity::NodeIdentity;

/// A single capability a node offers.
///
/// Capabilities are the contract between the human line and the machine line.
/// A writ targets a specific capability by name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// Unique name for this capability: "create-job", "cancel-job", "query-status".
    pub name: String,

    /// What this capability does, in human-readable terms.
    pub description: String,

    /// Parameters this capability accepts.
    pub params: Vec<CapabilityParam>,

    /// Minimum authority level required to invoke this capability.
    pub authority_required: AuthorityLevel,
}

/// A parameter accepted by a capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityParam {
    /// Parameter name.
    pub name: String,

    /// What kind of value this parameter expects.
    pub kind: ParamKind,

    /// Whether this parameter is required.
    pub required: bool,

    /// Human-readable description.
    pub description: String,
}

/// The type of a capability parameter.
///
/// Kept simple for v0 — string descriptions are enough for a typed DSL.
/// Richer schema (JSON Schema or similar) can come when an LLM agent
/// needs to reason about parameter shapes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamKind {
    String,
    Integer,
    Boolean,
    /// A reference to another entity (job_id, task_id, node_id).
    Reference,
}

/// The full set of capabilities a node publishes.
///
/// Published on the bus at startup and on heartbeat so other nodes
/// can discover what's available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySet {
    /// The node publishing these capabilities.
    pub node: NodeIdentity,

    /// The capabilities this node offers.
    pub capabilities: Vec<Capability>,

    /// Schema version for this capability set.
    pub version: u32,
}
