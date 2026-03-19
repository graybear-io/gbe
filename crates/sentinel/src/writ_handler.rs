//! Writ handler — sentinel's capability implementations.
//!
//! Uses the common WritDispatcher from gbe-nexus. Only the
//! capability-specific logic lives here.

use std::sync::Arc;

use async_trait::async_trait;
use frame::{NodeIdentity, WritResponse};
use gbe_nexus::writ;
use serde_json::json;

use crate::sentinel::SlotTracker;

/// Sentinel's capability handler — plugs into WritDispatcher.
pub struct SentinelCapabilities {
    slots: Arc<SlotTracker>,
    identity: NodeIdentity,
}

impl SentinelCapabilities {
    pub fn new(slots: Arc<SlotTracker>, identity: NodeIdentity) -> Self {
        Self { slots, identity }
    }
}

#[async_trait]
impl writ::CapabilityHandler for SentinelCapabilities {
    async fn handle_capability(&self, w: &frame::Writ) -> WritResponse {
        match w.capability.as_str() {
            "host-status" => {
                let total = self.slots.total();
                let available = self.slots.available();
                let used = total - available;
                writ::ok(
                    w,
                    &self.identity,
                    json!({
                        "host_id": self.identity.instance,
                        "slots_total": total,
                        "slots_used": used,
                        "slots_available": available,
                    }),
                )
            }
            "list-vms" => writ::ok(
                w,
                &self.identity,
                json!({
                    "vms": [],
                    "note": "VM tracking not yet implemented — use host-status for slot info"
                }),
            ),
            "drain-host" => {
                if w.authority.level < frame::AuthorityLevel::Consul {
                    writ::denied(w, &self.identity, "Consul")
                } else {
                    writ::ok(
                        w,
                        &self.identity,
                        json!({
                            "note": "drain acknowledged — actual drain not yet implemented"
                        }),
                    )
                }
            }
            _ => writ::unsupported(w, &self.identity),
        }
    }
}
