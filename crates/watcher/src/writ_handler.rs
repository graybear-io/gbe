//! Writ handler — watcher's capability implementations.
//!
//! Uses the common WritDispatcher from gbe-nexus. Only the
//! capability-specific logic lives here.

use async_trait::async_trait;
use frame::{NodeIdentity, WritResponse};
use gbe_nexus::writ;
use serde_json::json;

use crate::watcher::SweepReport;

/// Watcher's capability handler — plugs into WritDispatcher.
pub struct WatcherCapabilities {
    identity: NodeIdentity,
    last_sweep: tokio::sync::Mutex<Option<SweepReport>>,
}

impl WatcherCapabilities {
    pub fn new(identity: NodeIdentity) -> Self {
        Self {
            identity,
            last_sweep: tokio::sync::Mutex::new(None),
        }
    }

    /// Record a sweep result for reporting via writ.
    pub async fn record_sweep(&self, report: SweepReport) {
        *self.last_sweep.lock().await = Some(report);
    }
}

#[async_trait]
impl writ::CapabilityHandler for WatcherCapabilities {
    async fn handle_capability(&self, w: &frame::Writ) -> WritResponse {
        match w.capability.as_str() {
            "trigger-sweep" => {
                // For v0, acknowledge but don't actually trigger — watcher sweeps on its own schedule
                writ::ok(w, &self.identity, json!({
                    "note": "sweep trigger acknowledged — watcher will sweep on next cycle"
                }))
            }
            "sweep-status" => {
                let last = self.last_sweep.lock().await;
                match &*last {
                    Some(report) => writ::ok(w, &self.identity, json!({
                        "retried": report.retried,
                        "failed": report.failed,
                        "streams_trimmed": report.streams_trimmed,
                        "entries_trimmed": report.entries_trimmed,
                    })),
                    None => writ::ok(w, &self.identity, json!({
                        "note": "no sweep has run yet"
                    })),
                }
            }
            "dead-letter-status" => {
                // For v0, stub — dead letter monitoring not yet wired
                writ::ok(w, &self.identity, json!({
                    "count": 0,
                    "note": "dead letter monitoring not yet implemented"
                }))
            }
            _ => writ::unsupported(w, &self.identity),
        }
    }
}
