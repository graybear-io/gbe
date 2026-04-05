//! Frame — shared primitives for the ecumene.
//!
//! Identity, framing, capabilities, and authority flow.
//! Every node across every domain speaks frame.

pub mod authority;
pub mod capability;
pub mod condition;
pub mod config;
pub mod error;
pub mod flow;
pub mod frame;
pub mod geas;
pub mod identity;
pub mod interface;
pub mod packet;
pub mod rite;
pub mod trace;
pub mod writ;

pub use authority::{AuthorityFrame, AuthorityLevel};
pub use capability::{Capability, CapabilityParam, CapabilitySet, ParamKind};
pub use condition::NodeCondition;
pub use error::FrameError;
pub use flow::{Dispatch, Mandate, Writ, WritResponse, WritStatus, WritTarget};
pub use frame::{Frame, FrameKind};
pub use geas::{Geas, GeasLifecycle, GeasParam, GeasStep, StepBinding};
pub use identity::{NodeIdentity, NodeKind};
pub use interface::{Interface, InterfaceParseError, iface};
pub use packet::Packet;
pub use rite::{
    Rite, RiteBuilder, RiteParam, Value, ValueType, YieldField, YieldPattern, YieldShape,
    YieldValues, pattern_matches, validate_yields,
};
pub use trace::{TraceEvent, print_trace};

/// Current time as milliseconds since the Unix epoch.
#[allow(clippy::cast_possible_truncation)]
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
