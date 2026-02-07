//! GBE Protocol definitions
//!
//! This crate defines the message types and serialization formats
//! for communication between GBE components.
//!
//! See: docs/design/protocol-v1.md for full specification

/// Tool identifier (PID+sequence format)
pub type ToolId = String;

/// Control channel messages (JSON serialization)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ControlMessage {
    /// Tool connects to router
    Connect { capabilities: Vec<String> },

    /// Router acknowledges connection
    ConnectAck {
        tool_id: ToolId,
        data_listen_address: String,
    },

    /// Tool disconnects
    Disconnect,

    /// Subscribe to another tool's data stream
    Subscribe { target: ToolId },

    /// Router acknowledges subscription with connection details
    SubscribeAck {
        data_connect_address: String,
        capabilities: Vec<String>,
    },

    /// Unsubscribe from tool's data stream
    Unsubscribe { target: ToolId },

    /// Flow control notification (from proxy)
    FlowControl { source: ToolId, status: String },

    /// Query tool capabilities
    QueryCapabilities { target: ToolId },

    /// Response with capabilities
    CapabilitiesResponse { capabilities: Vec<String> },

    /// Error message
    Error { code: String, message: String },
}

/// Data channel frame (binary serialization)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataFrame {
    /// Sequence number for ordering
    pub seq: u64,
    /// Payload data (line or chunk)
    pub payload: Vec<u8>,
}

impl DataFrame {
    /// Create a new data frame
    pub fn new(seq: u64, payload: Vec<u8>) -> Self {
        Self { seq, payload }
    }

    /// Serialize to binary (length-prefixed)
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserialize from binary
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_message_serialization() {
        let msg = ControlMessage::Connect {
            capabilities: vec!["pty".to_string(), "color".to_string()],
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ControlMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ControlMessage::Connect { capabilities } => {
                assert_eq!(capabilities.len(), 2);
                assert_eq!(capabilities[0], "pty");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_data_frame_serialization() {
        let frame = DataFrame::new(42, b"hello world".to_vec());

        let bytes = frame.to_bytes().unwrap();
        let parsed = DataFrame::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.seq, 42);
        assert_eq!(parsed.payload, b"hello world");
    }
}
