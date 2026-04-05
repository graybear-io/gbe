use frame::AuthorityLevel;
use frame::Interface;
use frame::Rite;
use serde::{Deserialize, Serialize};

use crate::node::CrossingMode;

/// An imprint — a standing geas bound to a node.
///
/// The imprint defines what a node *is*. Its interfaces are derived from
/// the rites in the imprint. A sentinel is a sentinel because it carries
/// the sentinel imprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Imprint {
    /// Name of this imprint (e.g., "sentinel", "oracle", "watcher").
    pub name: String,

    /// The rites this imprint carries. Each rite's `needs` become
    /// the node's published interfaces.
    pub rites: Vec<String>,

    /// Authority level granted by this imprint.
    pub authority: AuthorityLevel,
}

/// Derive a node's interfaces from its imprint and the rite registry.
///
/// The node publishes interfaces that correspond to what its standing rites need —
/// because if the node can *perform* a rite, it *has* the interfaces that rite requires.
pub fn derive_interfaces(imprint: &Imprint, rites: &[(&str, &Rite)]) -> Vec<Interface> {
    let mut interfaces = Vec::new();

    for rite_name in &imprint.rites {
        if let Some((_, rite)) = rites.iter().find(|(name, _)| *name == rite_name.as_str()) {
            for need in &rite.needs {
                if !interfaces.contains(need) {
                    interfaces.push(need.clone());
                }
            }
        }
    }

    interfaces
}

/// Derive a barrier's outer interfaces from:
/// 1. Its own imprint (what the barrier does as a node)
/// 2. The rites it can forward or translate for inner nodes
///
/// For forwarded rites, the inner rite's needs become outer interfaces.
/// For translated rites, the outer rite's needs become outer interfaces.
/// For absorbed rites, nothing additional is published (the barrier's own
/// imprint already covers those).
pub fn derive_barrier_outer_interfaces(
    barrier_imprint: &Imprint,
    crossing_rules: &[crate::node::CrossingRule],
    rites: &[(&str, &Rite)],
) -> Vec<Interface> {
    // Start with the barrier's own interfaces.
    let mut interfaces = derive_interfaces(barrier_imprint, rites);

    for rule in crossing_rules {
        match &rule.mode {
            CrossingMode::Forward => {
                // The outer rite's needs become outer interfaces — the barrier
                // advertises that it can accept this rite on behalf of inner nodes.
                if let Some((_, rite)) = rites.iter().find(|(name, _)| *name == rule.outer_rite) {
                    for need in &rite.needs {
                        if !interfaces.contains(need) {
                            interfaces.push(need.clone());
                        }
                    }
                }
            }
            CrossingMode::Translate { inner_rites: _ } => {
                // The outer rite's needs become outer interfaces — the barrier
                // advertises that it can accept this outer rite, even though
                // it will translate to different inner rites.
                if let Some((_, rite)) = rites.iter().find(|(name, _)| *name == rule.outer_rite) {
                    for need in &rite.needs {
                        if !interfaces.contains(need) {
                            interfaces.push(need.clone());
                        }
                    }
                }
            }
            CrossingMode::Absorb => {
                // Nothing additional — the barrier's own imprint covers absorbed rites.
            }
        }
    }

    interfaces
}
