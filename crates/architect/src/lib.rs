//! Architect — role definitions and standing geas for ecosystem nodes.
//!
//! The single source of truth for "what does it mean to be a sentinel"
//! or "what does a thalamus do." Each role's standing geas defines its
//! identity, capabilities, and behavior.
//!
//! The architect is ecosystem-wide, organized by domain (gbe, allthing, akasha).
//! For v0, this is a library of builder functions. Each returns a standing
//! Geas that a node can use at startup to know what it is, derive its
//! capabilities, and publish its identity.

pub mod roles;

pub use roles::{
    akasha_roles, allthing_roles, gbe_roles, herald, oracle, overseer, role_names, sentinel,
    thalamus, watcher,
};
