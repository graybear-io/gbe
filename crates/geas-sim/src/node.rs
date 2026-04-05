use frame::{AuthorityLevel, Interface, NodeCondition};
use serde::{Deserialize, Serialize};

/// A simulated node on the network.
///
/// Nodes publish interfaces — that's how rites find them. A node doesn't
/// need to know about rites or geas. It just has a shape, and rites that
/// need that shape will match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimNode {
    pub name: String,
    pub interfaces: Vec<Interface>,
    pub authority: AuthorityLevel,
    pub domain: String,
    /// What imprint this node carries (if any). For tracing.
    #[serde(default)]
    pub imprint_name: Option<String>,
    /// Current condition of the node.
    #[serde(default = "default_condition")]
    pub condition: NodeCondition,
}

fn default_condition() -> NodeCondition {
    NodeCondition::Ready
}

/// How a barrier handles a rite that matches on the outer side.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrossingMode {
    /// Barrier handles the rite itself. Nothing goes inward.
    /// The barrier has the interfaces and acts on them directly.
    Absorb,

    /// Barrier forwards the rite to inner nodes unchanged.
    /// Used when the rite has sufficient authority and the inner
    /// nodes understand it directly.
    Forward,

    /// Barrier translates the outer rite into a different inner rite.
    /// Sentinel receives "deploy-image", constructs "prepare-filesystem"
    /// + "start-process" for its operatives.
    Translate {
        /// The inner rite(s) the barrier emits in response.
        inner_rites: Vec<String>,
    },
}

/// A rite crossing rule — maps an outer rite to a crossing behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossingRule {
    /// The outer rite name this rule applies to.
    pub outer_rite: String,
    /// How to handle this rite at the barrier.
    pub mode: CrossingMode,
}

/// A simulated barrier between domains.
///
/// The barrier node sits on two sides. Its outer interfaces are *derived*
/// from two sources:
/// 1. Its own imprint (what the barrier does as a node)
/// 2. The rites it can offer on behalf of inner nodes (its crossing rules)
///
/// The barrier doesn't blindly forward — it interprets. Each rite that
/// arrives on the outside is handled according to its crossing rules:
/// absorb, forward, or translate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Barrier {
    pub name: String,
    pub outer_domain: String,
    pub inner_domain: String,
    /// The barrier's own authority level.
    pub authority: AuthorityLevel,
    /// How this barrier handles rites that match on the outer side.
    /// If a rite matches but has no crossing rule, the barrier absorbs it
    /// (handles it itself, nothing goes inward).
    pub crossing_rules: Vec<CrossingRule>,
    /// Condition of the barrier node itself.
    #[serde(default = "default_condition")]
    pub condition: NodeCondition,
}

impl Barrier {
    /// Find the crossing rule for a given outer rite.
    /// If no rule exists, the default is Absorb.
    pub fn rule_for(&self, rite_name: &str) -> CrossingMode {
        self.crossing_rules
            .iter()
            .find(|r| r.outer_rite == rite_name)
            .map(|r| r.mode.clone())
            .unwrap_or(CrossingMode::Absorb)
    }
}
