//! Hierarchical interface paths — the matching surface between rites and nodes.
//!
//! A rite declares what interfaces it `needs`; a node publishes what interfaces
//! it has. Matching is prefix-based: the need `target::host` is satisfied by
//! a node publishing `target::host::sentinel-07`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A hierarchical interface path like `target::host` or `resource::health-probe`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Interface {
    segments: Vec<String>,
}

impl Interface {
    pub fn new(segments: Vec<String>) -> Self {
        Self { segments }
    }

    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Does this interface (as a need) match against a published interface?
    ///
    /// Prefix matching: `target::host` matches `target::host::sentinel-07`
    /// because the need is a prefix of the published interface. Exact match
    /// also works: `target::host` matches `target::host`.
    ///
    /// The published interface must be at least as specific as the need.
    pub fn satisfied_by(&self, published: &Interface) -> bool {
        if self.segments.len() > published.segments.len() {
            return false;
        }
        self.segments
            .iter()
            .zip(published.segments.iter())
            .all(|(need, have)| need == have)
    }

    /// Stack a more specific segment onto this interface, producing a narrower version.
    pub fn narrow(&self, segment: impl Into<String>) -> Interface {
        let mut segments = self.segments.clone();
        segments.push(segment.into());
        Interface { segments }
    }
}

impl fmt::Display for Interface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.segments.join("::"))
    }
}

impl FromStr for Interface {
    type Err = InterfaceParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let segments: Vec<String> = s.split("::").map(|s| s.trim().to_string()).collect();
        if segments.is_empty() || segments.iter().any(|s| s.is_empty()) {
            return Err(InterfaceParseError(s.to_string()));
        }
        Ok(Interface { segments })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid interface path: {0:?}")]
pub struct InterfaceParseError(String);

/// Convenience: parse an interface from a string literal.
pub fn iface(s: &str) -> Interface {
    s.parse().expect("invalid interface literal")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        let need = iface("target::host");
        let published = iface("target::host");
        assert!(need.satisfied_by(&published));
    }

    #[test]
    fn prefix_match() {
        let need = iface("target::host");
        let published = iface("target::host::sentinel-07");
        assert!(need.satisfied_by(&published));
    }

    #[test]
    fn no_match_different_path() {
        let need = iface("target::host");
        let published = iface("resource::health-probe");
        assert!(!need.satisfied_by(&published));
    }

    #[test]
    fn no_match_more_specific_need() {
        let need = iface("target::host::sentinel-07");
        let published = iface("target::host");
        assert!(!need.satisfied_by(&published));
    }

    #[test]
    fn narrow() {
        let base = iface("target::host");
        let narrowed = base.narrow("sentinel-07");
        assert_eq!(narrowed.to_string(), "target::host::sentinel-07");
    }

    #[test]
    fn display_and_parse_roundtrip() {
        let iface = Interface::new(vec!["target".into(), "host".into(), "sentinel-07".into()]);
        let s = iface.to_string();
        assert_eq!(s, "target::host::sentinel-07");
        let back: Interface = s.parse().unwrap();
        assert_eq!(iface, back);
    }
}
