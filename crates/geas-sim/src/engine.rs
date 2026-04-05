use std::collections::HashMap;

use frame::{
    NodeCondition, Rite, TraceEvent, YieldValues, pattern_matches, validate_yields,
};

use crate::geas::{self, ChainStep, Geas, RiteRef};
use crate::imprint::{self, Imprint};
use crate::matcher;
use crate::node::{Barrier, CrossingMode, SimNode};

/// Key for mock yield lookup: (node_name, rite_name).
type MockKey = (String, String);

/// The simulation engine. Holds a simulated network and a rite registry.
pub struct SimEngine {
    nodes: Vec<SimNode>,
    barriers: Vec<Barrier>,
    rites: HashMap<String, Rite>,
    imprints: HashMap<String, Imprint>,
    /// Registered geas that can be invoked as sub-geas.
    geas_registry: HashMap<String, Geas>,
    /// Mock yields: when a specific node completes a specific rite,
    /// these are the values it produces. Used for branch evaluation.
    mock_yields: HashMap<MockKey, YieldValues>,
    /// Default yields per rite (when no node-specific mock exists).
    default_yields: HashMap<String, YieldValues>,
}

impl Default for SimEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SimEngine {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            barriers: Vec::new(),
            rites: HashMap::new(),
            imprints: HashMap::new(),
            geas_registry: HashMap::new(),
            mock_yields: HashMap::new(),
            default_yields: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: SimNode) {
        self.nodes.push(node);
    }

    pub fn add_barrier(&mut self, barrier: Barrier) {
        self.barriers.push(barrier);
    }

    pub fn register_rite(&mut self, rite: Rite) {
        self.rites.insert(rite.name.clone(), rite);
    }

    pub fn register_imprint(&mut self, imprint: Imprint) {
        self.imprints.insert(imprint.name.clone(), imprint);
    }

    /// Register a geas that can be invoked as a sub-geas from other geas.
    pub fn register_geas(&mut self, geas: Geas) {
        self.geas_registry.insert(geas.name.clone(), geas);
    }

    /// Configure what a specific node yields when it completes a rite.
    pub fn mock_yield(&mut self, node: &str, rite: &str, values: YieldValues) {
        self.mock_yields
            .insert((node.to_string(), rite.to_string()), values);
    }

    /// Configure default yield values for a rite (any node).
    pub fn default_yield(&mut self, rite: &str, values: YieldValues) {
        self.default_yields.insert(rite.to_string(), values);
    }

    /// Look up yield values for a node completing a rite.
    fn get_yields(&self, node: &str, rite: &str) -> Option<&YieldValues> {
        self.mock_yields
            .get(&(node.to_string(), rite.to_string()))
            .or_else(|| self.default_yields.get(rite))
    }

    /// Imprint a bare node — derive its interfaces from a standing geas.
    pub fn imprint_node(&mut self, node_name: &str, imprint_name: &str) -> Vec<TraceEvent> {
        let mut trace = Vec::new();

        let Some(imprint) = self.imprints.get(imprint_name).cloned() else {
            trace.push(TraceEvent::RiteNotFound {
                rite: format!("imprint:{imprint_name}"),
            });
            return trace;
        };

        let rite_refs: Vec<(&str, &Rite)> =
            self.rites.iter().map(|(k, v)| (k.as_str(), v)).collect();

        let derived = imprint::derive_interfaces(&imprint, &rite_refs);

        trace.push(TraceEvent::Imprinted {
            node: node_name.to_string(),
            imprint: imprint_name.to_string(),
            derived_interfaces: derived.clone(),
        });

        if let Some(node) = self.nodes.iter_mut().find(|n| n.name == node_name) {
            node.interfaces = derived;
            node.authority = imprint.authority;
            node.imprint_name = Some(imprint_name.to_string());
        }

        self.refresh_barrier_interfaces(node_name);

        trace
    }

    fn refresh_barrier_interfaces(&mut self, barrier_name: &str) {
        let barrier = match self.barriers.iter().find(|b| b.name == barrier_name) {
            Some(b) => b.clone(),
            None => return,
        };

        let node_imprint = self
            .nodes
            .iter()
            .find(|n| n.name == barrier_name)
            .and_then(|n| n.imprint_name.as_ref())
            .and_then(|name| self.imprints.get(name))
            .cloned();

        if let Some(imprint) = node_imprint {
            let rite_refs: Vec<(&str, &Rite)> =
                self.rites.iter().map(|(k, v)| (k.as_str(), v)).collect();

            let _derived = imprint::derive_barrier_outer_interfaces(
                &imprint,
                &barrier.crossing_rules,
                &rite_refs,
            );
        }
    }

    fn barrier_as_node(&self, barrier: &Barrier) -> SimNode {
        let node_imprint = self
            .nodes
            .iter()
            .find(|n| n.name == barrier.name)
            .and_then(|n| n.imprint_name.as_ref())
            .and_then(|name| self.imprints.get(name));

        let rite_refs: Vec<(&str, &Rite)> =
            self.rites.iter().map(|(k, v)| (k.as_str(), v)).collect();

        let mut interfaces = if let Some(imprint) = node_imprint {
            imprint::derive_interfaces(imprint, &rite_refs)
        } else {
            self.nodes
                .iter()
                .find(|n| n.name == barrier.name)
                .map(|n| n.interfaces.clone())
                .unwrap_or_default()
        };

        for rule in &barrier.crossing_rules {
            match &rule.mode {
                CrossingMode::Forward | CrossingMode::Translate { .. } => {
                    if let Some((_, rite)) =
                        rite_refs.iter().find(|(name, _)| *name == rule.outer_rite)
                    {
                        for need in &rite.needs {
                            if !interfaces.contains(need) {
                                interfaces.push(need.clone());
                            }
                        }
                    }
                }
                CrossingMode::Absorb => {}
            }
        }

        SimNode {
            name: barrier.name.clone(),
            interfaces,
            authority: barrier.authority,
            domain: barrier.outer_domain.clone(),
            imprint_name: None,
            condition: barrier.condition,
        }
    }

    /// Update a node's condition.
    pub fn set_condition(&mut self, node_name: &str, condition: NodeCondition) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.name == node_name) {
            node.condition = condition;
        }
        if let Some(barrier) = self.barriers.iter_mut().find(|b| b.name == node_name) {
            barrier.condition = condition;
        }
    }

    /// Submit a geas and trace what happens.
    pub fn submit(&self, geas: &Geas) -> Vec<TraceEvent> {
        let mut trace = Vec::new();

        trace.push(TraceEvent::GeasSubmitted {
            name: geas.name.clone(),
        });

        self.execute_chain(&geas.chain, &mut trace, None);

        trace
    }

    /// Execute a chain of steps, tracking the last rite's yields for branching.
    fn execute_chain(
        &self,
        chain: &[ChainStep],
        trace: &mut Vec<TraceEvent>,
        initial_yields: Option<(&str, &YieldValues)>,
    ) {
        let mut prev_rite_name: Option<String> = None;
        let mut last_yields: Option<(String, YieldValues)> =
            initial_yields.map(|(name, vals)| (name.to_string(), vals.clone()));

        for step in chain {
            match step {
                ChainStep::Rite(rite_ref) => {
                    let result = self.execute_rite(rite_ref, prev_rite_name.as_deref(), trace);
                    match result {
                        RiteResult::Completed { rite_name, yields } => {
                            last_yields = yields.map(|y| (rite_name.clone(), y));
                            prev_rite_name = Some(rite_name);
                        }
                        RiteResult::ChainBreak | RiteResult::NotFound => {
                            return; // Hard stop.
                        }
                    }
                }
                ChainStep::Branch { arms } => {
                    let taken = self.evaluate_branch(arms, last_yields.as_ref(), trace);
                    if let Some(sub_chain) = taken {
                        self.execute_chain(
                            sub_chain,
                            trace,
                            last_yields.as_ref().map(|(n, v)| (n.as_str(), v)),
                        );
                    }
                    // Branches are terminal in their parent chain.
                    return;
                }
                ChainStep::SubGeas { geas_name } => {
                    let Some(sub_geas) = self.geas_registry.get(geas_name).cloned() else {
                        trace.push(TraceEvent::SubGeasNotFound {
                            name: geas_name.clone(),
                        });
                        return;
                    };

                    trace.push(TraceEvent::SubGeasEnter {
                        name: geas_name.clone(),
                    });

                    // Execute the sub-geas's chain. It runs in the same
                    // network context — same nodes, same rites.
                    // Pass in the current yields so the sub-geas can
                    // chain from the parent's last rite.
                    self.execute_chain(
                        &sub_geas.chain,
                        trace,
                        last_yields.as_ref().map(|(n, v)| (n.as_str(), v)),
                    );

                    trace.push(TraceEvent::SubGeasExit {
                        name: geas_name.clone(),
                    });

                    // The sub-geas's final yields become this step's yields.
                    // For now, we don't propagate yields back from sub-geas
                    // execution (would need execute_chain to return yields).
                    // This is a known simplification — the trace shows what
                    // happened inside.
                    prev_rite_name = Some(geas_name.clone());
                    last_yields = None;
                }
            }
        }
    }

    /// Execute a single rite reference. Returns what happened.
    fn execute_rite(
        &self,
        rite_ref: &RiteRef,
        prev_rite_name: Option<&str>,
        trace: &mut Vec<TraceEvent>,
    ) -> RiteResult {
        let Some(rite) = self.rites.get(&rite_ref.rite_name) else {
            trace.push(TraceEvent::RiteNotFound {
                rite: rite_ref.rite_name.clone(),
            });
            return RiteResult::NotFound;
        };

        // Validate yield chain.
        if let Some(prev_name) = prev_rite_name
            && self.check_yield_chain(prev_name, rite, rite_ref, trace)
        {
            return RiteResult::ChainBreak;
        }

        // Build candidates.
        let barrier_names: Vec<&str> = self.barriers.iter().map(|b| b.name.as_str()).collect();
        let mut candidate_nodes: Vec<SimNode> = self
            .outer_nodes()
            .into_iter()
            .filter(|n| !barrier_names.contains(&n.name.as_str()))
            .collect();
        for barrier in &self.barriers {
            candidate_nodes.push(self.barrier_as_node(barrier));
        }

        let result = matcher::match_rite(rite, &candidate_nodes);
        trace.extend(result.events);

        // Collect all yields from matched nodes for divergence detection.
        let mut all_yields: Vec<(String, YieldValues)> = Vec::new();

        for matched_name in &result.matched_nodes {
            if let Some(barrier) = self.barriers.iter().find(|b| b.name == *matched_name) {
                if !matches!(
                    barrier.condition,
                    NodeCondition::Ready | NodeCondition::Degraded
                ) {
                    trace.push(TraceEvent::BarrierConditionBlocked {
                        barrier: barrier.name.clone(),
                        condition: barrier.condition,
                    });
                    continue;
                }

                let mode = barrier.rule_for(&rite.name);
                self.handle_crossing(barrier, rite, &mode, trace);
            } else {
                trace.push(TraceEvent::RiteCompleted {
                    rite: rite.name.clone(),
                    matched_node: matched_name.clone(),
                });

                // Emit yield values if configured, validating against shape.
                if let Some(values) = self.get_yields(matched_name, &rite.name) {
                    // Validate mock yields against the rite's declared shape.
                    let violations = validate_yields(values, &rite.yields);
                    if !violations.is_empty() {
                        trace.push(TraceEvent::YieldShapeViolation {
                            rite: rite.name.clone(),
                            node: matched_name.clone(),
                            violations,
                        });
                    }

                    trace.push(TraceEvent::RiteYielded {
                        rite: rite.name.clone(),
                        node: matched_name.clone(),
                        values: values.clone(),
                    });
                    all_yields.push((matched_name.clone(), values.clone()));
                }
            }
        }

        // Detect fan-out divergence: if multiple nodes yielded different values,
        // the downstream branch will only see one of them. This is a real problem
        // the sim should surface.
        if all_yields.len() > 1 {
            let first = &all_yields[0].1;
            let divergent: Vec<String> = all_yields
                .iter()
                .filter(|(_, vals)| vals != first)
                .map(|(name, _)| name.clone())
                .collect();

            if !divergent.is_empty() {
                let mut all_nodes: Vec<String> =
                    all_yields.iter().map(|(n, _)| n.clone()).collect();
                all_nodes.sort();
                trace.push(TraceEvent::FanOutDivergence {
                    rite: rite.name.clone(),
                    nodes: all_nodes,
                    note:
                        "nodes yielded different values — branch will only see first node's yields"
                            .to_string(),
                });
            }
        }

        let first_yields = all_yields.into_iter().next().map(|(_, v)| v);

        RiteResult::Completed {
            rite_name: rite.name.clone(),
            yields: first_yields,
        }
    }

    /// Evaluate branch arms against yield values. Returns the sub-chain to execute.
    fn evaluate_branch<'a>(
        &self,
        arms: &'a [geas::BranchArm],
        last_yields: Option<&(String, YieldValues)>,
        trace: &mut Vec<TraceEvent>,
    ) -> Option<&'a [ChainStep]> {
        let values = last_yields.map(|(_, v)| v);
        let empty = vec![];
        let vals = values.unwrap_or(&empty);

        for (i, arm) in arms.iter().enumerate() {
            let pattern_desc = match &arm.pattern {
                frame::YieldPattern::Wildcard => "_".to_string(),
                frame::YieldPattern::Fields(fields) => {
                    let parts: Vec<String> =
                        fields.iter().map(|(k, v)| format!("{k}: {v}")).collect();
                    format!("{{{}}}", parts.join(", "))
                }
            };

            let matched = pattern_matches(&arm.pattern, vals);

            trace.push(TraceEvent::BranchEvaluated {
                arm_index: i,
                matched,
                pattern_desc: pattern_desc.clone(),
            });

            if matched {
                trace.push(TraceEvent::BranchTaken {
                    arm_index: i,
                    pattern_desc,
                });
                return Some(&arm.steps);
            }
        }

        let values_desc: String = vals
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>()
            .join(", ");
        trace.push(TraceEvent::BranchNoMatch { values_desc });
        None
    }

    /// Handle a barrier crossing based on the crossing mode.
    fn handle_crossing(
        &self,
        barrier: &Barrier,
        outer_rite: &Rite,
        mode: &CrossingMode,
        trace: &mut Vec<TraceEvent>,
    ) {
        let inner_nodes: Vec<SimNode> = self
            .nodes
            .iter()
            .filter(|n| n.domain == barrier.inner_domain)
            .cloned()
            .collect();

        match mode {
            CrossingMode::Absorb => {
                trace.push(TraceEvent::BarrierAbsorbed {
                    barrier: barrier.name.clone(),
                    rite: outer_rite.name.clone(),
                });
            }
            CrossingMode::Forward => {
                trace.push(TraceEvent::BarrierForwarded {
                    barrier: barrier.name.clone(),
                    rite: outer_rite.name.clone(),
                    inner_domain: barrier.inner_domain.clone(),
                });

                let inner_result = matcher::match_rite(outer_rite, &inner_nodes);
                if !inner_result.matched_nodes.is_empty() {
                    trace.push(TraceEvent::InnerNodesMatched {
                        barrier: barrier.name.clone(),
                        rite: outer_rite.name.clone(),
                        nodes: inner_result.matched_nodes.clone(),
                    });
                }
                for event in inner_result.events {
                    trace.push(event);
                }
            }
            CrossingMode::Translate { inner_rites } => {
                trace.push(TraceEvent::BarrierTranslated {
                    barrier: barrier.name.clone(),
                    outer_rite: outer_rite.name.clone(),
                    inner_rites: inner_rites.clone(),
                    inner_domain: barrier.inner_domain.clone(),
                });

                for inner_rite_name in inner_rites {
                    if let Some(inner_rite) = self.rites.get(inner_rite_name) {
                        let inner_result = matcher::match_rite(inner_rite, &inner_nodes);
                        if !inner_result.matched_nodes.is_empty() {
                            trace.push(TraceEvent::InnerNodesMatched {
                                barrier: barrier.name.clone(),
                                rite: inner_rite_name.clone(),
                                nodes: inner_result.matched_nodes.clone(),
                            });
                        }
                        for event in inner_result.events {
                            trace.push(event);
                        }
                    } else {
                        trace.push(TraceEvent::RiteNotFound {
                            rite: inner_rite_name.clone(),
                        });
                    }
                }
            }
        }
    }

    /// Validate yield chain between consecutive rites.
    /// Returns `true` if a break was found (halt execution).
    fn check_yield_chain(
        &self,
        prev_name: &str,
        rite: &Rite,
        rite_ref: &RiteRef,
        trace: &mut Vec<TraceEvent>,
    ) -> bool {
        let Some(prev_rite) = self.rites.get(prev_name) else {
            return false;
        };

        let missing: Vec<String> = rite
            .params
            .iter()
            .filter(|param| {
                let bound_from_yield = rite_ref.bindings.iter().any(
                    |b| matches!(b, geas::Binding::Yield { target, .. } if target == &param.name),
                );
                let bound_from_param = rite_ref.bindings.iter().any(
                    |b| matches!(b, geas::Binding::Param { target, .. } if target == &param.name),
                );

                if bound_from_param || !bound_from_yield {
                    return false;
                }

                rite_ref.bindings.iter().any(|b| {
                    if let geas::Binding::Yield {
                        rite: src_rite,
                        field,
                        target,
                    } = b
                        && target == &param.name
                        && src_rite == prev_name
                    {
                        return !prev_rite.yields.fields.iter().any(|f| f.name == *field);
                    }
                    false
                })
            })
            .map(|p| p.name.clone())
            .collect();

        if !missing.is_empty() {
            trace.push(TraceEvent::YieldChainBreak {
                from_rite: prev_name.to_string(),
                to_rite: rite.name.clone(),
                missing_fields: missing,
            });
            return true;
        }

        false
    }

    fn outer_nodes(&self) -> Vec<SimNode> {
        let inner_domains: Vec<&str> = self
            .barriers
            .iter()
            .map(|b| b.inner_domain.as_str())
            .collect();

        self.nodes
            .iter()
            .filter(|n| !inner_domains.contains(&n.domain.as_str()))
            .cloned()
            .collect()
    }
}

/// What happened when a rite was executed.
enum RiteResult {
    Completed {
        rite_name: String,
        yields: Option<YieldValues>,
    },
    ChainBreak,
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geas::RiteRef;
    use crate::node::CrossingRule;
    use frame::{
        AuthorityLevel, NodeCondition, Rite, Value, YieldPattern, YieldShape, iface,
    };

    #[test]
    fn simple_geas_traces_match() {
        let mut engine = SimEngine::new();

        engine.add_node(SimNode {
            name: "sentinel-01".into(),
            interfaces: vec![iface("target::host::s01"), iface("resource::health-probe")],
            authority: AuthorityLevel::Consul,
            domain: "gbe".into(),
            imprint_name: None,
            condition: NodeCondition::Ready,
        });

        engine.register_rite(Rite {
            name: "check-health".into(),
            needs: vec![iface("target::host"), iface("resource::health-probe")],
            requires: AuthorityLevel::Pilgrim,
            params: vec![],
            yields: YieldShape::empty(),
        });

        let geas = Geas {
            name: "health-sweep".into(),
            requires: AuthorityLevel::Pilgrim,
            params: vec![],
            chain: vec![ChainStep::Rite(RiteRef {
                rite_name: "check-health".into(),
                bindings: vec![],
            })],
        };

        let trace = engine.submit(&geas);
        let has_match = trace
            .iter()
            .any(|e| matches!(e, TraceEvent::NodesMatched { .. }));
        assert!(has_match);
    }

    #[test]
    fn barrier_absorbs_by_default() {
        let mut engine = SimEngine::new();

        engine.add_node(SimNode {
            name: "operative-01".into(),
            interfaces: vec![iface("target::task"), iface("resource::execute")],
            authority: AuthorityLevel::Pilgrim,
            domain: "vm".into(),
            imprint_name: None,
            condition: NodeCondition::Ready,
        });

        engine.add_node(SimNode {
            name: "sentinel-01".into(),
            interfaces: vec![iface("target::task"), iface("resource::execute")],
            authority: AuthorityLevel::Consul,
            domain: "gbe".into(),
            imprint_name: None,
            condition: NodeCondition::Ready,
        });
        engine.add_barrier(Barrier {
            name: "sentinel-01".into(),
            outer_domain: "gbe".into(),
            inner_domain: "vm".into(),
            authority: AuthorityLevel::Consul,
            crossing_rules: vec![],
            condition: NodeCondition::Ready,
        });

        engine.register_rite(Rite {
            name: "run-task".into(),
            needs: vec![iface("target::task"), iface("resource::execute")],
            requires: AuthorityLevel::Pilgrim,
            params: vec![],
            yields: YieldShape::empty(),
        });

        let geas = Geas::builder("test").rite("run-task").build();
        let trace = engine.submit(&geas);

        let has_absorbed = trace
            .iter()
            .any(|e| matches!(e, TraceEvent::BarrierAbsorbed { .. }));
        assert!(has_absorbed);
    }

    #[test]
    fn barrier_forwards_with_rule() {
        let mut engine = SimEngine::new();

        engine.add_node(SimNode {
            name: "operative-01".into(),
            interfaces: vec![iface("target::task"), iface("resource::execute")],
            authority: AuthorityLevel::Pilgrim,
            domain: "vm".into(),
            imprint_name: None,
            condition: NodeCondition::Ready,
        });

        engine.add_node(SimNode {
            name: "sentinel-01".into(),
            interfaces: vec![],
            authority: AuthorityLevel::Consul,
            domain: "gbe".into(),
            imprint_name: None,
            condition: NodeCondition::Ready,
        });

        engine.register_rite(Rite {
            name: "run-task".into(),
            needs: vec![iface("target::task"), iface("resource::execute")],
            requires: AuthorityLevel::Pilgrim,
            params: vec![],
            yields: YieldShape::empty(),
        });

        engine.add_barrier(Barrier {
            name: "sentinel-01".into(),
            outer_domain: "gbe".into(),
            inner_domain: "vm".into(),
            authority: AuthorityLevel::Consul,
            crossing_rules: vec![CrossingRule {
                outer_rite: "run-task".into(),
                mode: CrossingMode::Forward,
            }],
            condition: NodeCondition::Ready,
        });

        let geas = Geas::builder("test").rite("run-task").build();
        let trace = engine.submit(&geas);

        let has_forwarded = trace
            .iter()
            .any(|e| matches!(e, TraceEvent::BarrierForwarded { .. }));
        let has_inner = trace
            .iter()
            .any(|e| matches!(e, TraceEvent::InnerNodesMatched { .. }));
        assert!(has_forwarded);
        assert!(has_inner);
    }

    #[test]
    fn barrier_translates_with_rule() {
        let mut engine = SimEngine::new();

        engine.add_node(SimNode {
            name: "operative-01".into(),
            interfaces: vec![iface("target::filesystem"), iface("resource::prepare")],
            authority: AuthorityLevel::Pilgrim,
            domain: "vm".into(),
            imprint_name: None,
            condition: NodeCondition::Ready,
        });

        engine.add_node(SimNode {
            name: "sentinel-01".into(),
            interfaces: vec![],
            authority: AuthorityLevel::Consul,
            domain: "gbe".into(),
            imprint_name: None,
            condition: NodeCondition::Ready,
        });

        engine.register_rite(Rite {
            name: "deploy-image".into(),
            needs: vec![iface("target::host"), iface("resource::deploy")],
            requires: AuthorityLevel::Consul,
            params: vec![],
            yields: YieldShape::empty(),
        });

        engine.register_rite(Rite {
            name: "prepare-filesystem".into(),
            needs: vec![iface("target::filesystem"), iface("resource::prepare")],
            requires: AuthorityLevel::Pilgrim,
            params: vec![],
            yields: YieldShape::empty(),
        });

        engine.add_barrier(Barrier {
            name: "sentinel-01".into(),
            outer_domain: "gbe".into(),
            inner_domain: "vm".into(),
            authority: AuthorityLevel::Consul,
            crossing_rules: vec![CrossingRule {
                outer_rite: "deploy-image".into(),
                mode: CrossingMode::Translate {
                    inner_rites: vec!["prepare-filesystem".into()],
                },
            }],
            condition: NodeCondition::Ready,
        });

        let geas = Geas::builder("test")
            .requires(frame::AuthorityLevel::Consul)
            .rite("deploy-image")
            .build();
        let trace = engine.submit(&geas);

        let has_translated = trace
            .iter()
            .any(|e| matches!(e, TraceEvent::BarrierTranslated { .. }));
        let has_inner = trace.iter().any(
            |e| matches!(e, TraceEvent::InnerNodesMatched { rite, .. } if rite == "prepare-filesystem"),
        );
        assert!(has_translated);
        assert!(has_inner);
    }

    #[test]
    fn imprint_derives_interfaces() {
        let mut engine = SimEngine::new();

        engine.add_node(SimNode {
            name: "bare-node".into(),
            interfaces: vec![],
            authority: AuthorityLevel::Pilgrim,
            domain: "gbe".into(),
            imprint_name: None,
            condition: NodeCondition::Ready,
        });

        engine.register_rite(Rite {
            name: "check-health".into(),
            needs: vec![iface("target::host"), iface("resource::health-probe")],
            requires: AuthorityLevel::Pilgrim,
            params: vec![],
            yields: YieldShape::empty(),
        });

        engine.register_imprint(Imprint {
            name: "sentinel".into(),
            rites: vec!["check-health".into()],
            authority: AuthorityLevel::Consul,
        });

        let trace = engine.imprint_node("bare-node", "sentinel");
        assert!(
            trace
                .iter()
                .any(|e| matches!(e, TraceEvent::Imprinted { .. }))
        );

        let geas = Geas::builder("test").rite("check-health").build();
        let trace = engine.submit(&geas);
        let has_match = trace.iter().any(
            |e| matches!(e, TraceEvent::NodesMatched { nodes, .. } if nodes.contains(&"bare-node".to_string())),
        );
        assert!(has_match);
    }

    #[test]
    fn branch_on_yield_values() {
        let mut engine = SimEngine::new();

        engine.add_node(SimNode {
            name: "sentinel-01".into(),
            interfaces: vec![iface("target::host"), iface("resource::health-probe")],
            authority: AuthorityLevel::Consul,
            domain: "gbe".into(),
            imprint_name: None,
            condition: NodeCondition::Ready,
        });
        engine.add_node(SimNode {
            name: "notifier".into(),
            interfaces: vec![iface("resource::notification")],
            authority: AuthorityLevel::Pilgrim,
            domain: "gbe".into(),
            imprint_name: None,
            condition: NodeCondition::Ready,
        });

        engine.register_rite(
            Rite::builder("check-health")
                .needs("target::host")
                .needs("resource::health-probe")
                .requires(AuthorityLevel::Pilgrim)
                .yields_field("healthy", frame::ValueType::Bool)
                .build(),
        );
        engine.register_rite(
            Rite::builder("notify-unhealthy")
                .needs("resource::notification")
                .requires(AuthorityLevel::Pilgrim)
                .build(),
        );

        // Sentinel yields healthy: false.
        engine.mock_yield(
            "sentinel-01",
            "check-health",
            vec![("healthy".into(), Value::Bool(false))],
        );

        let geas = Geas {
            name: "health-branch".into(),
            requires: AuthorityLevel::Pilgrim,
            params: vec![],
            chain: vec![
                ChainStep::Rite(RiteRef {
                    rite_name: "check-health".into(),
                    bindings: vec![],
                }),
                ChainStep::Branch {
                    arms: vec![
                        geas::BranchArm {
                            pattern: YieldPattern::Fields(vec![(
                                "healthy".into(),
                                Value::Bool(true),
                            )]),
                            steps: vec![], // healthy — do nothing.
                        },
                        geas::BranchArm {
                            pattern: YieldPattern::Wildcard,
                            steps: vec![ChainStep::Rite(RiteRef {
                                rite_name: "notify-unhealthy".into(),
                                bindings: vec![],
                            })],
                        },
                    ],
                },
            ],
        };

        let trace = engine.submit(&geas);
        let has_branch_taken = trace
            .iter()
            .any(|e| matches!(e, TraceEvent::BranchTaken { arm_index: 1, .. }));
        let has_notify = trace.iter().any(
            |e| matches!(e, TraceEvent::RiteCompleted { rite, .. } if rite == "notify-unhealthy"),
        );
        assert!(has_branch_taken, "wildcard arm should be taken");
        assert!(has_notify, "notify-unhealthy should execute");
    }
}
