//! Packet — the fundamental unit. A payload with frames.
//!
//! Everything in the ecosystem is a packet. The difference between
//! "chat" and "notification" and "task result" is the framing,
//! not the type.

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::frame::Frame;

/// The fundamental unit of the ecosystem.
///
/// A packet is a payload plus its accumulated frame history.
/// Frames stack as the packet moves through the network.
/// The payload is immutable — only frames are added.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    /// Unique identifier for this packet.
    pub id: Ulid,

    /// The payload — opaque bytes. Interpretation is determined
    /// by the frames (origin frame says what it is, domain frames
    /// say how to render it).
    #[serde(with = "bytes_base64")]
    pub payload: Bytes,

    /// Accumulated frame stack. Ordered — first frame is the origin,
    /// subsequent frames are added as the packet moves.
    pub frames: Vec<Frame>,
}

impl Packet {
    /// Create a new packet with an origin frame.
    pub fn new(payload: Bytes, origin: Frame) -> Self {
        Self {
            id: Ulid::new(),
            payload,
            frames: vec![origin],
        }
    }

    /// Add a frame to the stack.
    pub fn push_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }

    /// The origin frame, if present.
    pub fn origin(&self) -> Option<&Frame> {
        self.frames.first()
    }
}

/// Serde helper: serialize Bytes as base64.
mod bytes_base64 {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use bytes::Bytes;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &Bytes, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Bytes, D::Error> {
        let s = String::deserialize(d)?;
        STANDARD
            .decode(&s)
            .map(Bytes::from)
            .map_err(serde::de::Error::custom)
    }
}
