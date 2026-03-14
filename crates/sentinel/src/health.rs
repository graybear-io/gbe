use std::sync::Arc;
use std::time::Duration;

use gbe_nexus::{EventEmitter, Transport, dedup_id};
use serde::Serialize;
use tokio_util::sync::CancellationToken;

use crate::error::SentinelError;
use crate::sentinel::SlotTracker;

#[derive(Debug, Serialize)]
struct Beacon {
    host_id: String,
    available_slots: u32,
    total_slots: u32,
}

/// Publishes periodic heartbeat beacons to the edge transport.
///
/// Subject: `gbe.sentinel.{host_id}.health`
pub struct HealthPublisher {
    emitter: EventEmitter,
    host_id: String,
    slots: Arc<SlotTracker>,
    interval: Duration,
}

impl HealthPublisher {
    pub fn new(
        edge_transport: Arc<dyn Transport>,
        host_id: String,
        slots: Arc<SlotTracker>,
        interval: Duration,
    ) -> Self {
        let emitter = EventEmitter::new(edge_transport, "sentinel", &host_id);
        Self {
            emitter,
            host_id,
            slots,
            interval,
        }
    }

    /// Run the beacon loop until cancelled.
    pub async fn run(&self, token: CancellationToken) -> Result<(), SentinelError> {
        let subject = format!("gbe.sentinel.{}.health", self.host_id);

        loop {
            tokio::select! {
                () = token.cancelled() => {
                    tracing::info!("health publisher shutting down");
                    return Ok(());
                }
                () = tokio::time::sleep(self.interval) => {
                    let beacon = Beacon {
                        host_id: self.host_id.clone(),
                        available_slots: self.slots.available(),
                        total_slots: self.slots.total(),
                    };
                    let dedup = dedup_id("sentinel", &self.host_id, "beacon");
                    if let Err(e) = self.emitter.emit(&subject, 1, dedup, beacon).await {
                        tracing::warn!(%e, "failed to publish beacon");
                    }
                }
            }
        }
    }
}
