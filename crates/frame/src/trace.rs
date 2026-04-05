//! Trace events — the shared vocabulary between simulation and reality.
//!
//! Both the geas-sim engine and the real system emit these events.
//! The sim returns them as `Vec<TraceEvent>` from `submit()`.
//! The real system publishes them on the bus.
//! An observer can diff predicted vs actual for the same geas.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::condition::NodeCondition;
use crate::interface::Interface;
use crate::rite::YieldValues;

/// A trace event recording what happened during geas execution.
///
/// Emitted by both the simulation engine and the real system.
/// The trace vocabulary IS the interface between sim and reality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraceEvent {
    /// A geas was submitted to the network.
    GeasSubmitted { name: String },

    /// A rite was evaluated for matching.
    RiteEvaluated { rite: String },

    /// Nodes matched a rite's interface needs.
    NodesMatched { rite: String, nodes: Vec<String> },

    /// No nodes matched a rite — unmet needs.
    NoMatch { rite: String, unmet: Vec<Interface> },

    /// A barrier absorbed a rite — handled it itself, nothing goes inward.
    BarrierAbsorbed { barrier: String, rite: String },

    /// A barrier forwarded a rite to the inner domain unchanged.
    BarrierForwarded {
        barrier: String,
        rite: String,
        inner_domain: String,
    },

    /// A barrier translated a rite into different inner rites.
    BarrierTranslated {
        barrier: String,
        outer_rite: String,
        inner_rites: Vec<String>,
        inner_domain: String,
    },

    /// Inner nodes matched behind the barrier (after forward or translate).
    InnerNodesMatched {
        barrier: String,
        rite: String,
        nodes: Vec<String>,
    },

    /// A rite completed on a node.
    RiteCompleted { rite: String, matched_node: String },

    /// Authority was insufficient for a rite.
    AuthorityInsufficient {
        rite: String,
        required: String,
        node: String,
        had: String,
    },

    /// A rite's yields don't satisfy the next rite's expected input shape.
    YieldChainBreak {
        from_rite: String,
        to_rite: String,
        missing_fields: Vec<String>,
    },

    /// A node matched on interfaces but its condition prevents action.
    ConditionBlocked {
        rite: String,
        node: String,
        condition: NodeCondition,
    },

    /// A rite was not found in the registry.
    RiteNotFound { rite: String },

    /// An imprint was applied to a node, deriving its interfaces.
    Imprinted {
        node: String,
        imprint: String,
        derived_interfaces: Vec<Interface>,
    },

    /// A barrier is not ready — inner nodes are unreachable.
    BarrierConditionBlocked {
        barrier: String,
        condition: NodeCondition,
    },

    /// Partial match — node satisfies some but not all needs.
    PartialMatch {
        rite: String,
        node: String,
        met: Vec<Interface>,
        unmet: Vec<Interface>,
    },

    /// A rite produced yield values.
    RiteYielded {
        rite: String,
        node: String,
        values: YieldValues,
    },

    /// A branch was evaluated against yield values.
    BranchEvaluated {
        arm_index: usize,
        matched: bool,
        pattern_desc: String,
    },

    /// A branch arm was taken.
    BranchTaken {
        arm_index: usize,
        pattern_desc: String,
    },

    /// No branch arm matched the yield values.
    BranchNoMatch { values_desc: String },

    /// Entering a sub-geas.
    SubGeasEnter { name: String },

    /// Exiting a sub-geas.
    SubGeasExit { name: String },

    /// Yield values don't conform to the rite's declared shape.
    YieldShapeViolation {
        rite: String,
        node: String,
        violations: Vec<String>,
    },

    /// Multiple nodes yielded different values during fan-out.
    FanOutDivergence {
        rite: String,
        nodes: Vec<String>,
        note: String,
    },

    /// A sub-geas was not found in the registry.
    SubGeasNotFound { name: String },
}

impl fmt::Display for TraceEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraceEvent::GeasSubmitted { name } => write!(f, "  geas submitted: {name}"),
            TraceEvent::RiteEvaluated { rite } => write!(f, "    rite: {rite}"),
            TraceEvent::NodesMatched { rite, nodes } => {
                write!(f, "    {rite} -> matched: [{}]", nodes.join(", "))
            }
            TraceEvent::NoMatch { rite, unmet } => {
                let unmet: Vec<String> = unmet.iter().map(|i| i.to_string()).collect();
                write!(f, "    {rite} -> NO MATCH (unmet: [{}])", unmet.join(", "))
            }
            TraceEvent::BarrierAbsorbed { barrier, rite } => {
                write!(f, "    barrier {barrier} ABSORBED {rite} (handled locally)")
            }
            TraceEvent::BarrierForwarded {
                barrier,
                rite,
                inner_domain,
            } => write!(
                f,
                "    barrier {barrier} FORWARDED {rite} -> domain:{inner_domain}"
            ),
            TraceEvent::BarrierTranslated {
                barrier,
                outer_rite,
                inner_rites,
                inner_domain,
            } => write!(
                f,
                "    barrier {barrier} TRANSLATED {outer_rite} -> [{}] for domain:{inner_domain}",
                inner_rites.join(", ")
            ),
            TraceEvent::InnerNodesMatched {
                barrier,
                rite,
                nodes,
            } => write!(
                f,
                "      {barrier}/{rite} -> inner matched: [{}]",
                nodes.join(", ")
            ),
            TraceEvent::RiteCompleted { rite, matched_node } => {
                write!(f, "    {rite} -> completed on {matched_node}")
            }
            TraceEvent::AuthorityInsufficient {
                rite,
                required,
                node,
                had,
            } => write!(
                f,
                "    {rite} -> {node} authority insufficient (need {required}, have {had})"
            ),
            TraceEvent::YieldChainBreak {
                from_rite,
                to_rite,
                missing_fields,
            } => write!(
                f,
                "    CHAIN BREAK: {from_rite} -> {to_rite} (missing yields: [{}])",
                missing_fields.join(", ")
            ),
            TraceEvent::ConditionBlocked {
                rite,
                node,
                condition,
            } => write!(
                f,
                "    {rite} -> {node} matched but BLOCKED (condition: {condition})"
            ),
            TraceEvent::RiteNotFound { rite } => {
                write!(f, "    {rite} -> NOT FOUND in rite registry")
            }
            TraceEvent::Imprinted {
                node,
                imprint,
                derived_interfaces,
            } => {
                let ifaces: Vec<String> =
                    derived_interfaces.iter().map(|i| i.to_string()).collect();
                write!(
                    f,
                    "  imprint {imprint} -> {node} (interfaces: [{}])",
                    ifaces.join(", ")
                )
            }
            TraceEvent::BarrierConditionBlocked { barrier, condition } => {
                write!(f, "    barrier {barrier} BLOCKED (condition: {condition})")
            }
            TraceEvent::PartialMatch {
                rite,
                node,
                met,
                unmet,
            } => {
                let met: Vec<String> = met.iter().map(|i| i.to_string()).collect();
                let unmet: Vec<String> = unmet.iter().map(|i| i.to_string()).collect();
                write!(
                    f,
                    "    {rite} -> {node} PARTIAL (met: [{}], unmet: [{}])",
                    met.join(", "),
                    unmet.join(", ")
                )
            }
            TraceEvent::RiteYielded { rite, node, values } => {
                let vals: Vec<String> = values.iter().map(|(k, v)| format!("{k}: {v}")).collect();
                write!(f, "    {rite} on {node} yielded {{{}}}", vals.join(", "))
            }
            TraceEvent::BranchEvaluated {
                arm_index,
                matched,
                pattern_desc,
            } => {
                let mark = if *matched { "MATCH" } else { "skip" };
                write!(f, "      arm[{arm_index}] {pattern_desc} -> {mark}")
            }
            TraceEvent::BranchTaken {
                arm_index,
                pattern_desc,
            } => write!(f, "    branch -> arm[{arm_index}] ({pattern_desc})"),
            TraceEvent::BranchNoMatch { values_desc } => {
                write!(f, "    branch -> NO ARM MATCHED (values: {values_desc})")
            }
            TraceEvent::SubGeasEnter { name } => write!(f, "  >> entering sub-geas: {name}"),
            TraceEvent::SubGeasExit { name } => write!(f, "  << exiting sub-geas: {name}"),
            TraceEvent::YieldShapeViolation {
                rite,
                node,
                violations,
            } => write!(
                f,
                "    WARNING: {rite} on {node} yield shape violation: [{}]",
                violations.join("; ")
            ),
            TraceEvent::FanOutDivergence { rite, nodes, note } => write!(
                f,
                "    WARNING: {rite} fan-out divergence across [{}]: {note}",
                nodes.join(", ")
            ),
            TraceEvent::SubGeasNotFound { name } => write!(f, "    sub-geas NOT FOUND: {name}"),
        }
    }
}

/// Print a trace to stdout.
pub fn print_trace(events: &[TraceEvent]) {
    for event in events {
        println!("{event}");
    }
}
