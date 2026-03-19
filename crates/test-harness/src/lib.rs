//! Integration test scaffolding for GBE writ flow.
//!
//! Provides transport-level primitives for testing writs end-to-end:
//! response collection, send-and-wait, capability discovery.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use frame::{CapabilitySet, WritResponse};
use gbe_nexus::{DomainPayload, Message, MessageHandler, TransportError};
use tokio::sync::Mutex;
use tracing::info;
use ulid::Ulid;

// ---------------------------------------------------------------------------
// Response collection
// ---------------------------------------------------------------------------

/// Collects WritResponses from the bus for later inspection.
pub struct ResponseCollector {
    responses: Arc<Mutex<Vec<WritResponse>>>,
}

impl ResponseCollector {
    pub fn new(responses: Arc<Mutex<Vec<WritResponse>>>) -> Self {
        Self { responses }
    }
}

#[async_trait]
impl MessageHandler for ResponseCollector {
    async fn handle(&self, msg: &dyn Message) -> Result<(), TransportError> {
        let payload: DomainPayload<WritResponse> = DomainPayload::from_bytes(msg.payload())
            .map_err(|e| TransportError::Other(format!("parse response: {e}")))?;
        let response = payload.data;
        info!(
            writ_id = %response.writ_id,
            status = ?response.status,
            "response received"
        );
        self.responses.lock().await.push(response);
        msg.ack().await?;
        Ok(())
    }
}

/// Wait for a response matching a specific writ_id, with timeout.
pub async fn wait_for_response(
    responses: &Arc<Mutex<Vec<WritResponse>>>,
    writ_id: Ulid,
    timeout: Duration,
) -> Option<WritResponse> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        {
            let resps = responses.lock().await;
            if let Some(r) = resps.iter().find(|r| r.writ_id == writ_id) {
                return Some(r.clone());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

// ---------------------------------------------------------------------------
// Capability discovery
// ---------------------------------------------------------------------------

/// Collects CapabilitySets from lifecycle announcements.
pub struct CapabilityCollector {
    capabilities: Arc<Mutex<Vec<CapabilitySet>>>,
}

impl CapabilityCollector {
    pub fn new(capabilities: Arc<Mutex<Vec<CapabilitySet>>>) -> Self {
        Self { capabilities }
    }
}

#[async_trait]
impl MessageHandler for CapabilityCollector {
    async fn handle(&self, msg: &dyn Message) -> Result<(), TransportError> {
        if let Ok(payload) = DomainPayload::<CapabilitySet>::from_bytes(msg.payload()) {
            info!(
                role = %payload.data.node.name,
                caps = payload.data.capabilities.len(),
                "capability announcement received"
            );
            self.capabilities.lock().await.push(payload.data);
        }
        msg.ack().await?;
        Ok(())
    }
}

/// Wait until capability announcements have been received for all expected roles.
pub async fn wait_for_capabilities(
    capabilities: &Arc<Mutex<Vec<CapabilitySet>>>,
    expected_roles: &[&str],
    timeout: Duration,
) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        {
            let caps = capabilities.lock().await;
            let seen: Vec<&str> = caps.iter().map(|c| c.node.name.as_str()).collect();
            if expected_roles.iter().all(|r| seen.contains(r)) {
                return true;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

// ---------------------------------------------------------------------------
// Writ send helpers
// ---------------------------------------------------------------------------

/// Send a pre-built Writ and wait for its response.
pub async fn send_writ(
    emitter: &gbe_nexus::EventEmitter,
    writ: &frame::Writ,
    subject: &str,
    responses: &Arc<Mutex<Vec<WritResponse>>>,
    timeout: Duration,
) -> anyhow::Result<WritResponse> {
    let writ_id = writ.packet.id;

    emitter
        .emit(subject, 1, writ_id.to_string(), writ)
        .await
        .map_err(|e| anyhow::anyhow!("publish failed: {e}"))?;

    wait_for_response(responses, writ_id, timeout)
        .await
        .ok_or_else(|| anyhow::anyhow!("no response within {timeout:?}"))
}
