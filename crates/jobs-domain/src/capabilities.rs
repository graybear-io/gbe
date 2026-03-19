//! Capability definitions for GBE services.
//!
//! Each service declares what writs it can accept. These are published
//! on the bus so overseer and other nodes can discover what's available.

use frame::{AuthorityLevel, Capability, CapabilityParam, CapabilitySet, NodeIdentity, ParamKind};

/// Build the capability set for Oracle.
///
/// Oracle accepts writs to create, cancel, and query jobs.
pub fn oracle(identity: NodeIdentity) -> CapabilitySet {
    CapabilitySet {
        node: identity,
        version: 1,
        capabilities: vec![
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
        ],
    }
}

/// Build the capability set for Sentinel.
///
/// Sentinel manages VM lifecycle on a host. Its capabilities are
/// scoped to the host it runs on.
pub fn sentinel(identity: NodeIdentity) -> CapabilitySet {
    CapabilitySet {
        node: identity,
        version: 1,
        capabilities: vec![
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
                description: "Stop accepting new tasks, wait for running tasks to complete"
                    .to_string(),
                params: vec![],
                authority_required: AuthorityLevel::Consul,
            },
        ],
    }
}

/// Build the capability set for Watcher.
///
/// Watcher sweeps for stuck jobs, trims streams, and archives.
pub fn watcher(identity: NodeIdentity) -> CapabilitySet {
    CapabilitySet {
        node: identity,
        version: 1,
        capabilities: vec![
            Capability {
                name: "trigger-sweep".to_string(),
                description: "Force an immediate sweep for stuck jobs and stream trimming"
                    .to_string(),
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
                description: "Report the count and age of messages in dead letter queues"
                    .to_string(),
                params: vec![],
                authority_required: AuthorityLevel::Pilgrim,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame::NodeKind;

    fn test_identity(name: &str) -> NodeIdentity {
        NodeIdentity::new(name, NodeKind::Service, "gbe", "test-001")
    }

    #[test]
    fn oracle_capabilities_round_trip() {
        let caps = oracle(test_identity("oracle"));
        assert_eq!(caps.capabilities.len(), 3);
        assert_eq!(caps.capabilities[0].name, "create-job");
        assert_eq!(caps.capabilities[1].name, "cancel-job");
        assert_eq!(caps.capabilities[2].name, "job-status");

        let json = serde_json::to_string(&caps).unwrap();
        let back: CapabilitySet = serde_json::from_str(&json).unwrap();
        assert_eq!(back.capabilities.len(), 3);
    }

    #[test]
    fn sentinel_capabilities_round_trip() {
        let caps = sentinel(test_identity("sentinel"));
        assert_eq!(caps.capabilities.len(), 3);
        assert_eq!(caps.capabilities[0].name, "host-status");

        // drain-host requires consul
        assert_eq!(
            caps.capabilities[2].authority_required,
            AuthorityLevel::Consul
        );

        let json = serde_json::to_string(&caps).unwrap();
        let back: CapabilitySet = serde_json::from_str(&json).unwrap();
        assert_eq!(back.capabilities.len(), 3);
    }

    #[test]
    fn watcher_capabilities_round_trip() {
        let caps = watcher(test_identity("watcher"));
        assert_eq!(caps.capabilities.len(), 3);
        assert_eq!(caps.capabilities[0].name, "trigger-sweep");

        let json = serde_json::to_string(&caps).unwrap();
        let back: CapabilitySet = serde_json::from_str(&json).unwrap();
        assert_eq!(back.capabilities.len(), 3);
    }
}
