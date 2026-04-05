//! Node condition — operational state that affects rite matching.

use serde::{Deserialize, Serialize};

/// A node's operational condition — affects whether a matched node actually acts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeCondition {
    /// Healthy and ready to act.
    Ready,
    /// Running but degraded — may act with reduced capability.
    Degraded,
    /// Draining — will not accept new work.
    Draining,
    /// Offline — will not respond.
    Offline,
}

impl NodeCondition {
    /// Can this node accept new work?
    pub fn can_act(self) -> bool {
        matches!(self, NodeCondition::Ready | NodeCondition::Degraded)
    }
}

impl std::fmt::Display for NodeCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeCondition::Ready => write!(f, "ready"),
            NodeCondition::Degraded => write!(f, "degraded"),
            NodeCondition::Draining => write!(f, "draining"),
            NodeCondition::Offline => write!(f, "offline"),
        }
    }
}
