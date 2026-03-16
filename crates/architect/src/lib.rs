//! Architect — role definitions and standing geas for GBE nodes.
//!
//! The single source of truth for "what does it mean to be a sentinel"
//! or "what does an oracle do." Each role's standing geas defines its
//! identity, capabilities, and behavior.
//!
//! For v0, this is a library of builder functions. Each returns a standing
//! Geas that a node can use at startup to know what it is, derive its
//! capabilities, and publish its identity.

pub mod roles;

pub use roles::{oracle, overseer, sentinel, watcher};
