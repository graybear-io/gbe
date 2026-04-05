//! Built-in imprints for common ecosystem data shapes.
//!
//! These are hand-written v0 imprints. Future: generated from sigils.
//! Each imprint matches a specific data shape on the bus and compiles
//! it into cells/pages.

pub mod capabilities;
pub mod collect;
pub mod envelope;
pub mod gather;
pub mod generic;
pub mod packet;
pub mod triage;
pub mod writ;
pub mod writ_response;

use crate::imprint::ImprintRegistry;

/// Create a registry pre-loaded with all built-in imprints,
/// ordered by specificity (most specific first).
pub fn default_registry() -> ImprintRegistry {
    let mut registry = ImprintRegistry::new();

    // Specific imprints (specificity 20).
    registry.register(Box::new(writ::WritImprint));
    registry.register(Box::new(writ_response::WritResponseImprint));
    registry.register(Box::new(packet::PacketImprint));
    registry.register(Box::new(capabilities::CapabilitiesImprint));
    registry.register(Box::new(gather::GatherCompletedImprint));
    registry.register(Box::new(gather::GatherFailedImprint));
    registry.register(Box::new(triage::TriageCompletedImprint));

    // Domain-aware imprints (specificity 15).
    registry.register(Box::new(collect::CollectImprint));

    // Envelope-aware generic (specificity 5).
    registry.register(Box::new(envelope::EnvelopeImprint));

    // Catch-all (specificity 0).
    registry.register(Box::new(generic::GenericImprint));

    registry
}
