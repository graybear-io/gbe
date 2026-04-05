//! Writ handling — capability dispatch and response builders.
//!
//! This module provides the domain logic for handling writs:
//! the `CapabilityHandler` trait that roles implement, and
//! response builder functions. Transport concerns (deserialization,
//! wire format, message ack) belong in the transport layer, not here.

use std::future::Future;
use std::pin::Pin;

use crate::flow::{Writ, WritResponse, WritStatus};
use crate::identity::NodeIdentity;

/// Subject where all WritResponses are published.
pub const RESPONSE_SUBJECT: &str = "writs.responses";

/// A boxed future that returns a WritResponse.
pub type WritFuture<'a> = Pin<Box<dyn Future<Output = WritResponse> + Send + 'a>>;

/// Role-specific capability handler.
///
/// Implement this for each role (sentinel, oracle, watcher, thalamus, etc.).
/// The transport layer handles deserialization, logging, response publishing,
/// and ack — this trait is purely the capability-specific decision.
///
/// Returns a boxed future so the trait is dyn-compatible — transport layers
/// need `Box<dyn CapabilityHandler>` for runtime dispatch.
pub trait CapabilityHandler: Send + Sync {
    /// Handle a writ targeting a specific capability.
    /// Returns a WritResponse for the transport layer to publish.
    fn handle_capability<'a>(&'a self, writ: &'a Writ) -> WritFuture<'a>;
}

impl<T: CapabilityHandler> CapabilityHandler for std::sync::Arc<T> {
    fn handle_capability<'a>(&'a self, writ: &'a Writ) -> WritFuture<'a> {
        (**self).handle_capability(writ)
    }
}

/// Build a WritResponse for a successful capability execution.
pub fn ok(writ: &Writ, responder: &NodeIdentity, data: serde_json::Value) -> WritResponse {
    WritResponse {
        writ_id: writ.packet.id,
        responder: responder.clone(),
        status: WritStatus::Ok,
        data,
    }
}

/// Build a WritResponse for a failed capability execution.
pub fn error(writ: &Writ, responder: &NodeIdentity, msg: &str) -> WritResponse {
    WritResponse {
        writ_id: writ.packet.id,
        responder: responder.clone(),
        status: WritStatus::Error,
        data: serde_json::json!({ "error": msg }),
    }
}

/// Build a WritResponse for an unsupported capability.
pub fn unsupported(writ: &Writ, responder: &NodeIdentity) -> WritResponse {
    WritResponse {
        writ_id: writ.packet.id,
        responder: responder.clone(),
        status: WritStatus::Unsupported,
        data: serde_json::json!({
            "error": format!("{} does not support capability: {}", responder.name, writ.capability)
        }),
    }
}

/// Build a WritResponse for insufficient authority.
pub fn denied(writ: &Writ, responder: &NodeIdentity, required: &str) -> WritResponse {
    WritResponse {
        writ_id: writ.packet.id,
        responder: responder.clone(),
        status: WritStatus::Denied,
        data: serde_json::json!({
            "error": format!("{} requires {} authority", writ.capability, required)
        }),
    }
}

/// Parse the writ's packet payload as a JSON params map.
pub fn parse_params(
    writ: &Writ,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    serde_json::from_slice(&writ.packet.payload)
        .map_err(|e| format!("failed to parse writ params: {e}"))
}
