//! Writ dispatcher — common infrastructure for handling writs.
//!
//! Every role that receives writs has the same envelope:
//! deserialize, log, dispatch to capability handler, publish response, ack.
//! This module extracts that pattern so roles only implement the
//! capability-specific logic.

use std::sync::Arc;

use async_trait::async_trait;
use frame::{NodeIdentity, WritResponse, WritStatus};
use tracing::{info, warn};

use crate::emitter::EventEmitter;
use crate::error::TransportError;
use crate::payload::DomainPayload;
use crate::transport::{Message, MessageHandler, Transport};

/// Subject where all WritResponses are published.
pub const RESPONSE_SUBJECT: &str = "gbe.writs.responses";

/// Role-specific capability handler.
///
/// Implement this for each role (sentinel, oracle, watcher).
/// The dispatcher handles deserialization, logging, response publishing, and ack.
#[async_trait]
pub trait CapabilityHandler: Send + Sync {
    /// Handle a writ targeting a specific capability.
    /// Return a WritResponse — the dispatcher publishes it.
    async fn handle_capability(&self, writ: &frame::Writ) -> WritResponse;
}

/// Common writ handler that implements MessageHandler.
///
/// Wraps a CapabilityHandler and handles the boilerplate:
/// parse DomainPayload<Writ>, delegate to handler, publish WritResponse, ack.
pub struct WritDispatcher {
    emitter: EventEmitter,
    handler: Box<dyn CapabilityHandler>,
}

impl WritDispatcher {
    pub fn new(
        identity: NodeIdentity,
        transport: Arc<dyn Transport>,
        handler: Box<dyn CapabilityHandler>,
    ) -> Self {
        Self {
            emitter: EventEmitter::new(transport, identity),
            handler,
        }
    }
}

#[async_trait]
impl MessageHandler for WritDispatcher {
    async fn handle(&self, msg: &dyn Message) -> Result<(), TransportError> {
        let envelope = msg.envelope();

        // Deserialize the writ
        let payload: DomainPayload<frame::Writ> =
            match DomainPayload::from_bytes(msg.payload()) {
                Ok(p) => p,
                Err(e) => {
                    warn!(
                        subject = %envelope.subject,
                        error = %e,
                        "failed to deserialize writ, dropping"
                    );
                    msg.ack().await?;
                    return Ok(());
                }
            };

        let writ = &payload.data;
        let writ_id = writ.packet.id;

        info!(
            %writ_id,
            capability = %writ.capability,
            subject = %envelope.subject,
            "received writ"
        );

        // Delegate to the role-specific handler
        let response = self.handler.handle_capability(writ).await;

        // Publish response
        if let Err(e) = self.emitter
            .emit(
                RESPONSE_SUBJECT,
                1,
                writ_id.to_string(),
                &response,
            )
            .await
        {
            warn!(%e, %writ_id, "failed to publish writ response");
        }

        msg.ack().await?;
        Ok(())
    }
}

/// Build a WritResponse for the common case of an unsupported capability.
pub fn unsupported(writ: &frame::Writ, responder: &NodeIdentity) -> WritResponse {
    WritResponse {
        writ_id: writ.packet.id,
        responder: responder.clone(),
        status: WritStatus::Unsupported,
        data: serde_json::json!({
            "error": format!("{} does not support capability: {}", responder.name, writ.capability)
        }),
    }
}

/// Build a WritResponse for the common case of insufficient authority.
pub fn denied(writ: &frame::Writ, responder: &NodeIdentity, required: &str) -> WritResponse {
    WritResponse {
        writ_id: writ.packet.id,
        responder: responder.clone(),
        status: WritStatus::Denied,
        data: serde_json::json!({
            "error": format!("{} requires {} authority", writ.capability, required)
        }),
    }
}

/// Build an Ok WritResponse with data.
pub fn ok(writ: &frame::Writ, responder: &NodeIdentity, data: serde_json::Value) -> WritResponse {
    WritResponse {
        writ_id: writ.packet.id,
        responder: responder.clone(),
        status: WritStatus::Ok,
        data,
    }
}

/// Parse the writ's packet payload as a JSON params map.
pub fn parse_params(
    writ: &frame::Writ,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    serde_json::from_slice(&writ.packet.payload)
        .map_err(|e| format!("failed to parse writ params: {e}"))
}

/// Build an Error WritResponse.
pub fn error(writ: &frame::Writ, responder: &NodeIdentity, msg: &str) -> WritResponse {
    WritResponse {
        writ_id: writ.packet.id,
        responder: responder.clone(),
        status: WritStatus::Error,
        data: serde_json::json!({ "error": msg }),
    }
}
