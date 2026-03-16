// -- Lifecycle payloads --
// Published to gbe.events.lifecycle.{component}.* subjects.
// Wrap in DomainPayload<T> from gbe-nexus before publishing.

use frame::NodeIdentity;

/// Component has connected to transport and is ready.
/// Subject: `gbe.events.lifecycle.{component}.started`
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComponentStarted {
    pub node: NodeIdentity,
    pub started_at: u64,
    pub version: Option<String>,
}

/// Component is shutting down gracefully.
/// Subject: `gbe.events.lifecycle.{component}.stopped`
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComponentStopped {
    pub node: NodeIdentity,
    pub stopped_at: u64,
    pub reason: String,
}

/// Periodic liveness signal.
/// Subject: `gbe.events.lifecycle.{component}.heartbeat`
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Heartbeat {
    pub node: NodeIdentity,
    pub timestamp: u64,
    pub uptime_secs: u64,
}

/// Component is alive but unhealthy.
/// Subject: `gbe.events.lifecycle.{component}.degraded`
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComponentDegraded {
    pub node: NodeIdentity,
    pub degraded_at: u64,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame::NodeKind;

    fn test_identity(name: &str, instance: &str) -> NodeIdentity {
        NodeIdentity::new(name, NodeKind::Service, "gbe", instance)
    }

    #[test]
    fn component_started_round_trip() {
        let payload = ComponentStarted {
            node: test_identity("operative", "op-abc123"),
            started_at: 1_707_934_567_000,
            version: Some("0.1.0".to_string()),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let back: ComponentStarted = serde_json::from_str(&json).unwrap();
        assert_eq!(back.node.name, "operative");
        assert_eq!(back.node.instance, "op-abc123");
    }

    #[test]
    fn heartbeat_round_trip() {
        let payload = Heartbeat {
            node: test_identity("oracle", "orc-def456"),
            timestamp: 1_707_934_600_000,
            uptime_secs: 3600,
        };
        let json = serde_json::to_string(&payload).unwrap();
        let back: Heartbeat = serde_json::from_str(&json).unwrap();
        assert_eq!(back.uptime_secs, 3600);
    }

    #[test]
    fn component_degraded_round_trip() {
        let payload = ComponentDegraded {
            node: test_identity("sentinel", "snt-ghi789"),
            degraded_at: 1_707_935_000_000,
            reason: "redis connection pool exhausted".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let back: ComponentDegraded = serde_json::from_str(&json).unwrap();
        assert_eq!(back.reason, "redis connection pool exhausted");
    }

    #[test]
    fn component_stopped_round_trip() {
        let payload = ComponentStopped {
            node: test_identity("watcher", "wtc-jkl012"),
            stopped_at: 1_707_936_000_000,
            reason: "SIGTERM".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let back: ComponentStopped = serde_json::from_str(&json).unwrap();
        assert_eq!(back.reason, "SIGTERM");
    }
}
