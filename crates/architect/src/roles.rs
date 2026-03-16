//! Standing geas for each GBE role.
//!
//! Each function returns a Geas that defines the role — its steps are
//! the standing behaviors, its params are the configuration the node
//! needs at instantiation.

use frame::{
    AuthorityLevel, Capability, CapabilityParam, Geas, GeasLifecycle, GeasParam, GeasStep,
    ParamKind,
};

/// All known role names in the constellation.
///
/// Overseer uses this to derive concrete subjects for lifecycle and writ
/// subscriptions. This is the Architect's view of what roles exist —
/// the source of truth for the subject namespace.
pub fn role_names() -> &'static [&'static str] {
    &["oracle", "sentinel", "watcher", "overseer"]
}

/// Standing geas for the Oracle role.
///
/// Oracle walks DAGs, dispatches tasks, and emits job lifecycle events.
/// It accepts writs to create and cancel jobs.
pub fn oracle() -> Geas {
    Geas {
        name: "oracle".to_string(),
        description: "DAG walker and task coordinator. Receives writs, emits mandates.".to_string(),
        version: 1,
        lifecycle: GeasLifecycle::Standing,
        authority_required: AuthorityLevel::Pilgrim,
        params: vec![],
        steps: vec![
            GeasStep {
                name: "accept-writs".to_string(),
                role: "oracle".to_string(),
                capability: "create-job".to_string(),
                input: vec![],
                depends_on: vec![],
            },
            GeasStep {
                name: "manage-jobs".to_string(),
                role: "oracle".to_string(),
                capability: "cancel-job".to_string(),
                input: vec![],
                depends_on: vec![],
            },
            GeasStep {
                name: "report-status".to_string(),
                role: "oracle".to_string(),
                capability: "job-status".to_string(),
                input: vec![],
                depends_on: vec![],
            },
        ],
    }
}

/// Standing geas for the Sentinel role.
///
/// Sentinel manages VM lifecycle on a host. It claims tasks from the bus,
/// provisions VMs, relays operative events, and publishes health beacons.
pub fn sentinel(host_id: &str, slots: u32, task_types: &[&str]) -> Geas {
    Geas {
        name: "sentinel".to_string(),
        description: format!(
            "Per-host VM lifecycle manager. host={host_id}, slots={slots}, types=[{}]",
            task_types.join(", ")
        ),
        version: 1,
        lifecycle: GeasLifecycle::Standing,
        authority_required: AuthorityLevel::Pilgrim,
        params: vec![
            GeasParam {
                name: "host_id".to_string(),
                description: "Host identifier".to_string(),
                required: true,
            },
            GeasParam {
                name: "slots".to_string(),
                description: "Number of VM slots available".to_string(),
                required: true,
            },
            GeasParam {
                name: "task_types".to_string(),
                description: "Task types this sentinel handles".to_string(),
                required: true,
            },
        ],
        steps: vec![
            GeasStep {
                name: "report-host".to_string(),
                role: "sentinel".to_string(),
                capability: "host-status".to_string(),
                input: vec![],
                depends_on: vec![],
            },
            GeasStep {
                name: "manage-vms".to_string(),
                role: "sentinel".to_string(),
                capability: "list-vms".to_string(),
                input: vec![],
                depends_on: vec![],
            },
            GeasStep {
                name: "drain".to_string(),
                role: "sentinel".to_string(),
                capability: "drain-host".to_string(),
                input: vec![],
                depends_on: vec![],
            },
        ],
    }
}

/// Standing geas for the Watcher role.
///
/// Watcher sweeps for stuck jobs, trims streams, monitors dead letter queues.
pub fn watcher() -> Geas {
    Geas {
        name: "watcher".to_string(),
        description: "Stuck job detector, stream trimmer, dead letter monitor.".to_string(),
        version: 1,
        lifecycle: GeasLifecycle::Standing,
        authority_required: AuthorityLevel::Pilgrim,
        params: vec![],
        steps: vec![
            GeasStep {
                name: "sweep".to_string(),
                role: "watcher".to_string(),
                capability: "trigger-sweep".to_string(),
                input: vec![],
                depends_on: vec![],
            },
            GeasStep {
                name: "report-sweep".to_string(),
                role: "watcher".to_string(),
                capability: "sweep-status".to_string(),
                input: vec![],
                depends_on: vec![],
            },
            GeasStep {
                name: "monitor-dead-letters".to_string(),
                role: "watcher".to_string(),
                capability: "dead-letter-status".to_string(),
                input: vec![],
                depends_on: vec![],
            },
        ],
    }
}

/// Standing geas for the Overseer role.
///
/// Overseer is the human's presence on the network. It subscribes to the bus,
/// collects capabilities from all nodes, accepts TUI connections, and
/// translates human commands into writs.
pub fn overseer() -> Geas {
    Geas {
        name: "overseer".to_string(),
        description: "Human command interface. Watches the bus, serves the TUI, issues writs."
            .to_string(),
        version: 1,
        lifecycle: GeasLifecycle::Standing,
        authority_required: AuthorityLevel::Consul,
        params: vec![],
        steps: vec![
            GeasStep {
                name: "collect-capabilities".to_string(),
                role: "overseer".to_string(),
                capability: "discover-nodes".to_string(),
                input: vec![],
                depends_on: vec![],
            },
            GeasStep {
                name: "serve-tui".to_string(),
                role: "overseer".to_string(),
                capability: "accept-connections".to_string(),
                input: vec![],
                depends_on: vec![],
            },
            GeasStep {
                name: "issue-writs".to_string(),
                role: "overseer".to_string(),
                capability: "execute-geas".to_string(),
                input: vec![],
                depends_on: vec![],
            },
        ],
    }
}

/// Derive the capabilities a role offers from its standing geas.
///
/// This extracts the capability names referenced in the geas steps
/// and builds full Capability definitions. For v0, the descriptions
/// and params are hardcoded here — eventually they'll be derived
/// from the geas step definitions themselves.
pub fn capabilities_for(geas: &Geas) -> Vec<Capability> {
    geas.steps
        .iter()
        .map(|step| Capability {
            name: step.capability.clone(),
            description: format!("{} (from {} standing geas)", step.name, geas.name),
            params: vec![],
            authority_required: geas.authority_required,
        })
        .collect()
}

/// Derive capabilities with full param definitions for known roles.
///
/// This returns the richer capability definitions (with params and
/// per-capability authority levels) for roles we've fully specified.
pub fn rich_capabilities_for(geas: &Geas, identity: frame::NodeIdentity) -> frame::CapabilitySet {
    // For known roles, delegate to the hand-crafted definitions.
    // This is the bridge between architect (role definitions) and
    // the capability system. Eventually capabilities will be fully
    // derived from the geas.
    let capabilities = match geas.name.as_str() {
        "oracle" => oracle_capabilities(),
        "sentinel" => sentinel_capabilities(),
        "watcher" => watcher_capabilities(),
        "overseer" => overseer_capabilities(),
        _ => capabilities_for(geas),
    };

    frame::CapabilitySet {
        node: identity,
        capabilities,
        version: geas.version,
    }
}

fn oracle_capabilities() -> Vec<Capability> {
    vec![
        Capability {
            name: "create-job".to_string(),
            description: "Submit a job definition for DAG execution".to_string(),
            params: vec![
                CapabilityParam {
                    name: "definition".to_string(),
                    kind: ParamKind::String,
                    required: true,
                    description: "Job definition (YAML or JSON)".to_string(),
                },
                CapabilityParam {
                    name: "org_id".to_string(),
                    kind: ParamKind::Reference,
                    required: false,
                    description: "Organization ID for event correlation".to_string(),
                },
            ],
            authority_required: AuthorityLevel::Pilgrim,
        },
        Capability {
            name: "cancel-job".to_string(),
            description: "Cancel a running job".to_string(),
            params: vec![
                CapabilityParam {
                    name: "job_id".to_string(),
                    kind: ParamKind::Reference,
                    required: true,
                    description: "ID of the job to cancel".to_string(),
                },
                CapabilityParam {
                    name: "reason".to_string(),
                    kind: ParamKind::String,
                    required: false,
                    description: "Reason for cancellation".to_string(),
                },
            ],
            authority_required: AuthorityLevel::Pilgrim,
        },
        Capability {
            name: "job-status".to_string(),
            description: "Query the current state of a job and its tasks".to_string(),
            params: vec![CapabilityParam {
                name: "job_id".to_string(),
                kind: ParamKind::Reference,
                required: true,
                description: "ID of the job to query".to_string(),
            }],
            authority_required: AuthorityLevel::Pilgrim,
        },
    ]
}

fn sentinel_capabilities() -> Vec<Capability> {
    vec![
        Capability {
            name: "host-status".to_string(),
            description: "Report slot usage and VM states for this host".to_string(),
            params: vec![],
            authority_required: AuthorityLevel::Pilgrim,
        },
        Capability {
            name: "list-vms".to_string(),
            description: "List active VMs and their task assignments".to_string(),
            params: vec![],
            authority_required: AuthorityLevel::Pilgrim,
        },
        Capability {
            name: "drain-host".to_string(),
            description: "Stop accepting new tasks, wait for running tasks to complete".to_string(),
            params: vec![],
            authority_required: AuthorityLevel::Consul,
        },
    ]
}

fn watcher_capabilities() -> Vec<Capability> {
    vec![
        Capability {
            name: "trigger-sweep".to_string(),
            description: "Force an immediate sweep for stuck jobs and stream trimming".to_string(),
            params: vec![],
            authority_required: AuthorityLevel::Pilgrim,
        },
        Capability {
            name: "sweep-status".to_string(),
            description: "Report the last sweep result and next scheduled sweep".to_string(),
            params: vec![],
            authority_required: AuthorityLevel::Pilgrim,
        },
        Capability {
            name: "dead-letter-status".to_string(),
            description: "Report the count and age of messages in dead letter queues".to_string(),
            params: vec![],
            authority_required: AuthorityLevel::Pilgrim,
        },
    ]
}

fn overseer_capabilities() -> Vec<Capability> {
    vec![
        Capability {
            name: "discover-nodes".to_string(),
            description: "List all known nodes and their capabilities".to_string(),
            params: vec![],
            authority_required: AuthorityLevel::Pilgrim,
        },
        Capability {
            name: "accept-connections".to_string(),
            description: "Accept TUI connections over unix socket".to_string(),
            params: vec![],
            authority_required: AuthorityLevel::Pilgrim,
        },
        Capability {
            name: "execute-geas".to_string(),
            description: "Translate a named geas into a writ and submit to Oracle".to_string(),
            params: vec![
                CapabilityParam {
                    name: "geas_name".to_string(),
                    kind: ParamKind::String,
                    required: true,
                    description: "Name of the geas to execute".to_string(),
                },
                CapabilityParam {
                    name: "params".to_string(),
                    kind: ParamKind::String,
                    required: false,
                    description: "JSON-encoded parameters for the geas".to_string(),
                },
            ],
            authority_required: AuthorityLevel::Pilgrim,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame::{NodeIdentity, NodeKind};

    #[test]
    fn oracle_geas_is_standing() {
        let g = oracle();
        assert_eq!(g.lifecycle, GeasLifecycle::Standing);
        assert_eq!(g.name, "oracle");
        assert_eq!(g.steps.len(), 3);
    }

    #[test]
    fn sentinel_geas_captures_config() {
        let g = sentinel("host-03", 4, &["shell", "http"]);
        assert!(g.description.contains("host-03"));
        assert!(g.description.contains("4"));
        assert_eq!(g.params.len(), 3);
    }

    #[test]
    fn watcher_geas_is_standing() {
        let g = watcher();
        assert_eq!(g.lifecycle, GeasLifecycle::Standing);
        assert_eq!(g.steps.len(), 3);
    }

    #[test]
    fn overseer_geas_requires_consul() {
        let g = overseer();
        assert_eq!(g.authority_required, AuthorityLevel::Consul);
    }

    #[test]
    fn capabilities_derived_from_geas() {
        let g = oracle();
        let caps = capabilities_for(&g);
        assert_eq!(caps.len(), 3);
        assert_eq!(caps[0].name, "create-job");
    }

    #[test]
    fn rich_capabilities_for_oracle() {
        let g = oracle();
        let identity = NodeIdentity::new("oracle", NodeKind::Service, "gbe", "orc-001");
        let set = rich_capabilities_for(&g, identity);
        assert_eq!(set.capabilities.len(), 3);
        // Rich version has params
        assert!(!set.capabilities[0].params.is_empty());
        assert_eq!(set.capabilities[0].params[0].name, "definition");
    }

    #[test]
    fn rich_capabilities_for_unknown_falls_back() {
        let mut g = oracle();
        g.name = "custom-role".to_string();
        let identity = NodeIdentity::new("custom", NodeKind::Service, "gbe", "cust-001");
        let set = rich_capabilities_for(&g, identity);
        // Falls back to derived capabilities (no params)
        assert_eq!(set.capabilities.len(), 3);
        assert!(set.capabilities[0].params.is_empty());
    }

    #[test]
    fn all_geas_serialize_to_yaml() {
        for g in [oracle(), watcher(), overseer(), sentinel("h1", 2, &["shell"])] {
            let yaml = serde_yaml::to_string(&g).unwrap();
            let back: Geas = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(back.name, g.name);
        }
    }
}
