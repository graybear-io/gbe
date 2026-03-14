use std::sync::Arc;

use async_trait::async_trait;
use gbe_nexus::{EventEmitter, Message, MessageHandler, Transport, TransportError, dedup_id};
use gbe_state_store::StateStore;
use serde::{Deserialize, Serialize};

use crate::claim::claim_task;
use crate::sentinel::SlotTracker;

/// Lifecycle events published to edge transport when tasks are claimed.
#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum SentinelEvent {
    TaskClaimed {
        task_id: String,
        host_id: String,
    },
    TaskStarted {
        task_id: String,
        host_id: String,
        vm_cid: u32,
    },
    TaskCompleted {
        task_id: String,
        host_id: String,
        exit_code: i32,
    },
    TaskFailed {
        task_id: String,
        host_id: String,
        error: String,
    },
}

/// Payload expected on the task queue stream.
#[derive(Debug, Deserialize)]
pub struct TaskQueuePayload {
    pub task_id: String,
    pub state_key: String,
    pub task_type: String,
    pub timeout_sec: u64,
}

/// Handles incoming task queue messages from core transport.
///
/// On receipt: extract state key, attempt CAS claim, emit lifecycle events
/// to edge transport. VM provisioning is a TODO — the handler demonstrates
/// the claim → edge publish flow.
pub struct TaskHandler {
    host_id: String,
    store: Arc<dyn StateStore>,
    slots: Arc<SlotTracker>,
    emitter: EventEmitter,
}

impl TaskHandler {
    pub fn new(
        host_id: String,
        store: Arc<dyn StateStore>,
        slots: Arc<SlotTracker>,
        edge_transport: Arc<dyn Transport>,
    ) -> Self {
        let emitter = EventEmitter::new(edge_transport, "sentinel", &host_id);
        Self {
            host_id,
            store,
            slots,
            emitter,
        }
    }

    async fn emit_lifecycle(&self, event: SentinelEvent) {
        let subject = format!("gbe.sentinel.{}.lifecycle", self.host_id);
        let event_name = match &event {
            SentinelEvent::TaskClaimed { .. } => "task_claimed",
            SentinelEvent::TaskStarted { .. } => "task_started",
            SentinelEvent::TaskCompleted { .. } => "task_completed",
            SentinelEvent::TaskFailed { .. } => "task_failed",
        };
        let dedup = dedup_id("sentinel", &self.host_id, event_name);
        if let Err(e) = self.emitter.emit(&subject, 1, dedup, event).await {
            tracing::warn!(%e, "failed to emit lifecycle event");
        }
    }
}

#[async_trait]
impl MessageHandler for TaskHandler {
    async fn handle(&self, msg: &dyn Message) -> Result<(), TransportError> {
        let payload: TaskQueuePayload = serde_json::from_slice(msg.payload())
            .map_err(|e| TransportError::Other(format!("invalid task payload: {e}")))?;

        // Check capacity
        if !self.slots.try_claim() {
            tracing::info!(task_id = %payload.task_id, "no slots available, nak-ing");
            msg.nak(None).await?;
            return Ok(());
        }

        // Attempt CAS claim
        let timeout_at = {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock before epoch")
                .as_millis() as u64;
            now + payload.timeout_sec * 1000
        };

        // Use a placeholder CID — real impl assigns from VM creation
        let vm_cid = 3_u32; // TODO: allocate from CID pool

        match claim_task(
            &self.store,
            &payload.state_key,
            &self.host_id,
            vm_cid,
            timeout_at,
        )
        .await
        {
            Ok(()) => {
                tracing::info!(task_id = %payload.task_id, "task claimed");
                msg.ack().await?;

                self.emit_lifecycle(SentinelEvent::TaskClaimed {
                    task_id: payload.task_id.clone(),
                    host_id: self.host_id.clone(),
                })
                .await;

                // TODO: provision VM, inject task, start vsock relay
                // For now, emit a started event to demonstrate the flow
                self.emit_lifecycle(SentinelEvent::TaskStarted {
                    task_id: payload.task_id,
                    host_id: self.host_id.clone(),
                    vm_cid,
                })
                .await;
            }
            Err(e) => {
                tracing::info!(task_id = %payload.task_id, %e, "claim failed");
                self.slots.release();
                msg.nak(None).await?;
            }
        }

        Ok(())
    }
}
