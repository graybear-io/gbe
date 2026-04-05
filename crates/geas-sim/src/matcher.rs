use frame::{AuthorityLevel, Interface, NodeCondition, Rite, TraceEvent};

use crate::node::SimNode;

/// Result of matching a rite against a set of nodes.
#[derive(Debug)]
pub struct MatchResult {
    pub matched_nodes: Vec<String>,
    pub unmet_needs: Vec<Interface>,
    pub events: Vec<TraceEvent>,
}

/// Match a rite's interface needs against a set of nodes.
///
/// A node matches when:
/// 1. It publishes interfaces that satisfy ALL of the rite's needs (prefix match).
/// 2. Its authority level meets or exceeds the rite's requirement.
/// 3. Its condition allows it to act (Ready or Degraded).
///
/// Nodes that are Draining or Offline are reported as condition-blocked.
/// Nodes that match some but not all needs are reported as partial matches.
pub fn match_rite(rite: &Rite, nodes: &[SimNode]) -> MatchResult {
    let mut matched = Vec::new();
    let mut events = Vec::new();

    events.push(TraceEvent::RiteEvaluated {
        rite: rite.name.clone(),
    });

    for node in nodes {
        // Check authority first.
        if !authority_sufficient(node.authority, rite.requires) {
            events.push(TraceEvent::AuthorityInsufficient {
                rite: rite.name.clone(),
                required: format!("{:?}", rite.requires),
                node: node.name.clone(),
                had: format!("{:?}", node.authority),
            });
            continue;
        }

        // Check which interface needs are satisfied.
        let mut met = Vec::new();
        let mut unmet = Vec::new();

        for need in &rite.needs {
            if node
                .interfaces
                .iter()
                .any(|published| need.satisfied_by(published))
            {
                met.push(need.clone());
            } else {
                unmet.push(need.clone());
            }
        }

        if !unmet.is_empty() {
            // Near-miss: report when exactly one need is unmet AND at least
            // one need IS met. A node that meets nothing isn't a near-miss,
            // it's just a different kind of node.
            //
            // Direction: this threshold will eventually become adaptive
            // rather than fixed, as partial matches carry signal about
            // system composition and alignment.
            if unmet.len() == 1 && !met.is_empty() {
                events.push(TraceEvent::PartialMatch {
                    rite: rite.name.clone(),
                    node: node.name.clone(),
                    met,
                    unmet,
                });
            }
            continue;
        }

        // All interfaces match — check condition.
        match node.condition {
            NodeCondition::Ready | NodeCondition::Degraded => {
                matched.push(node.name.clone());
            }
            condition => {
                events.push(TraceEvent::ConditionBlocked {
                    rite: rite.name.clone(),
                    node: node.name.clone(),
                    condition,
                });
            }
        }
    }

    if matched.is_empty() {
        // Figure out which needs went unmet across all nodes.
        let globally_unmet: Vec<Interface> = rite
            .needs
            .iter()
            .filter(|need| {
                !nodes.iter().any(|n| {
                    n.interfaces
                        .iter()
                        .any(|pub_iface| need.satisfied_by(pub_iface))
                })
            })
            .cloned()
            .collect();

        events.push(TraceEvent::NoMatch {
            rite: rite.name.clone(),
            unmet: globally_unmet.clone(),
        });

        MatchResult {
            matched_nodes: matched,
            unmet_needs: globally_unmet,
            events,
        }
    } else {
        events.push(TraceEvent::NodesMatched {
            rite: rite.name.clone(),
            nodes: matched.clone(),
        });

        MatchResult {
            matched_nodes: matched,
            unmet_needs: vec![],
            events,
        }
    }
}

fn authority_sufficient(have: AuthorityLevel, need: AuthorityLevel) -> bool {
    have >= need
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame::{iface, YieldShape};

    fn make_node(name: &str, interfaces: &[&str], authority: AuthorityLevel) -> SimNode {
        SimNode {
            name: name.to_string(),
            interfaces: interfaces.iter().map(|s| iface(s)).collect(),
            authority,
            domain: "gbe".to_string(),
            imprint_name: None,
            condition: NodeCondition::Ready,
        }
    }

    fn make_rite(name: &str, needs: &[&str], requires: AuthorityLevel) -> Rite {
        Rite {
            name: name.to_string(),
            needs: needs.iter().map(|s| iface(s)).collect(),
            requires,
            params: vec![],
            yields: YieldShape::empty(),
        }
    }

    #[test]
    fn single_node_exact_match() {
        let nodes = vec![make_node(
            "sentinel-01",
            &["target::host", "resource::health-probe"],
            AuthorityLevel::Consul,
        )];
        let rite = make_rite(
            "check-health",
            &["target::host", "resource::health-probe"],
            AuthorityLevel::Pilgrim,
        );

        let result = match_rite(&rite, &nodes);
        assert_eq!(result.matched_nodes, vec!["sentinel-01"]);
        assert!(result.unmet_needs.is_empty());
    }

    #[test]
    fn prefix_match_more_specific_node() {
        let nodes = vec![make_node(
            "sentinel-07",
            &["target::host::sentinel-07", "resource::health-probe"],
            AuthorityLevel::Consul,
        )];
        let rite = make_rite(
            "check-health",
            &["target::host", "resource::health-probe"],
            AuthorityLevel::Pilgrim,
        );

        let result = match_rite(&rite, &nodes);
        assert_eq!(result.matched_nodes, vec!["sentinel-07"]);
    }

    #[test]
    fn fan_out_multiple_nodes_match() {
        let nodes = vec![
            make_node(
                "sentinel-01",
                &["target::host::s01", "resource::health-probe"],
                AuthorityLevel::Consul,
            ),
            make_node(
                "sentinel-02",
                &["target::host::s02", "resource::health-probe"],
                AuthorityLevel::Consul,
            ),
            make_node("watcher-01", &["resource::sweep"], AuthorityLevel::Pilgrim),
        ];
        let rite = make_rite(
            "check-health",
            &["target::host", "resource::health-probe"],
            AuthorityLevel::Pilgrim,
        );

        let result = match_rite(&rite, &nodes);
        assert_eq!(result.matched_nodes, vec!["sentinel-01", "sentinel-02"]);
    }

    #[test]
    fn no_match_missing_interface() {
        let nodes = vec![make_node(
            "sentinel-01",
            &["target::host"],
            AuthorityLevel::Consul,
        )];
        let rite = make_rite(
            "check-health",
            &["target::host", "resource::health-probe"],
            AuthorityLevel::Pilgrim,
        );

        let result = match_rite(&rite, &nodes);
        assert!(result.matched_nodes.is_empty());
        assert_eq!(result.unmet_needs.len(), 1);
        assert_eq!(result.unmet_needs[0].to_string(), "resource::health-probe");
    }

    #[test]
    fn authority_blocks_match() {
        let nodes = vec![make_node(
            "sentinel-01",
            &["target::host", "resource::health-probe"],
            AuthorityLevel::Pilgrim,
        )];
        let rite = make_rite("admin-action", &["target::host"], AuthorityLevel::Consul);

        let result = match_rite(&rite, &nodes);
        assert!(result.matched_nodes.is_empty());
    }

    #[test]
    fn condition_blocks_match() {
        let mut node = make_node(
            "sentinel-01",
            &["target::host", "resource::health-probe"],
            AuthorityLevel::Consul,
        );
        node.condition = NodeCondition::Draining;

        let rite = make_rite(
            "check-health",
            &["target::host", "resource::health-probe"],
            AuthorityLevel::Pilgrim,
        );

        let result = match_rite(&rite, &[node]);
        assert!(result.matched_nodes.is_empty());
        let has_blocked = result
            .events
            .iter()
            .any(|e| matches!(e, TraceEvent::ConditionBlocked { .. }));
        assert!(has_blocked);
    }

    #[test]
    fn degraded_still_matches() {
        let mut node = make_node(
            "sentinel-01",
            &["target::host", "resource::health-probe"],
            AuthorityLevel::Consul,
        );
        node.condition = NodeCondition::Degraded;

        let rite = make_rite(
            "check-health",
            &["target::host", "resource::health-probe"],
            AuthorityLevel::Pilgrim,
        );

        let result = match_rite(&rite, &[node]);
        assert_eq!(result.matched_nodes, vec!["sentinel-01"]);
    }

    #[test]
    fn partial_match_reported() {
        let nodes = vec![make_node(
            "oracle",
            &["resource::job-router", "resource::dag-planner"],
            AuthorityLevel::Consul,
        )];
        let rite = make_rite(
            "check-health",
            &["target::host", "resource::dag-planner"],
            AuthorityLevel::Pilgrim,
        );

        let result = match_rite(&rite, &nodes);
        assert!(result.matched_nodes.is_empty());
        let has_partial = result
            .events
            .iter()
            .any(|e| matches!(e, TraceEvent::PartialMatch { .. }));
        assert!(has_partial);
    }
}
