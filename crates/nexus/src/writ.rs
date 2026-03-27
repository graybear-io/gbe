//! Writ dispatcher — transport adapter for writ handling.
//!
//! The domain logic (CapabilityHandler trait, response builders) lives in
//! `frame::writ`. This module provides the transport glue: deserialize
//! the envelope, hand the unwrapped Writ to the handler, publish the
//! response, ack the message.

use std::sync::Arc;

use async_trait::async_trait;
use frame::NodeIdentity;
use tracing::{info, warn};

use crate::emitter::EventEmitter;
use crate::error::TransportError;
use crate::payload::DomainPayload;
use crate::transport::{Message, MessageHandler, Transport};

// Re-export frame::writ so existing consumers (`use gbe_nexus::writ`) keep working.
pub use frame::writ::{
    CapabilityHandler, RESPONSE_SUBJECT, denied, error, ok, parse_params, unsupported,
};

/// Transport-level writ dispatcher.
///
/// Wraps a `CapabilityHandler` (from frame) and handles the transport
/// boilerplate: deserialize `DomainPayload<Writ>`, delegate to handler,
/// publish `WritResponse`, ack the message.
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

        // Open the envelope — transport concern, not frame's
        let payload: DomainPayload<frame::Writ> = match DomainPayload::from_bytes(msg.payload()) {
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

        // Hand the unwrapped writ to the frame-level handler
        let response = self.handler.handle_capability(writ).await;

        // Wrap the response back up for the wire
        if let Err(e) = self
            .emitter
            .emit(RESPONSE_SUBJECT, 1, writ_id.to_string(), &response)
            .await
        {
            warn!(%e, %writ_id, "failed to publish writ response");
        }

        msg.ack().await?;
        Ok(())
    }
}
