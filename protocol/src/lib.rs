//! GBE Protocol definitions
//!
//! This crate defines the message types and serialization formats
//! for communication between GBE components.
//!
//! # Architecture
//!
//! GBE uses a dual-channel architecture:
//! - **Control channel**: JSON messages via router (coordination)
//! - **Data channel**: Binary frames point-to-point (streaming)
//!
//! # Examples
//!
//! ## Control Messages
//!
//! ```
//! use gbe_protocol::ControlMessage;
//!
//! // Tool connects to router
//! let connect = ControlMessage::Connect {
//!     capabilities: vec!["pty".to_string()],
//! };
//! let json = serde_json::to_string(&connect).unwrap();
//!
//! // Router responds with assigned ID
//! let ack = ControlMessage::ConnectAck {
//!     tool_id: "12345-001".to_string(),
//!     data_listen_address: "unix:///tmp/gbe-12345-001.sock".to_string(),
//! };
//! ```
//!
//! ## Data Frames
//!
//! ```
//! use gbe_protocol::DataFrame;
//!
//! // Create and serialize a data frame
//! let frame = DataFrame::new(1, b"hello world".to_vec());
//! let bytes = frame.to_bytes();
//!
//! // Deserialize from wire format
//! let parsed = DataFrame::from_bytes(&bytes).unwrap();
//! assert_eq!(parsed.seq, 1);
//! assert_eq!(parsed.payload, b"hello world");
//! ```
//!
//! ## Streaming Data
//!
//! ```
//! use gbe_protocol::DataFrame;
//! use std::io::Cursor;
//!
//! // Write frames to a stream
//! let mut buffer = Vec::new();
//! let frame1 = DataFrame::new(0, b"line 1".to_vec());
//! let frame2 = DataFrame::new(1, b"line 2".to_vec());
//!
//! frame1.write_to(&mut buffer).unwrap();
//! frame2.write_to(&mut buffer).unwrap();
//!
//! // Read frames from stream
//! let mut cursor = Cursor::new(buffer);
//! let parsed1 = DataFrame::read_from(&mut cursor).unwrap();
//! let parsed2 = DataFrame::read_from(&mut cursor).unwrap();
//! ```
//!
//! See: docs/design/protocol-v1.md for full specification

use std::io::{self, Read, Write};

/// Tool identifier (PID+sequence format: "12345-001")
pub type ToolId = String;

/// Protocol errors
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid frame: {0}")]
    InvalidFrame(String),
}

/// Control channel messages (JSON serialization)
///
/// All control messages flow through the router in JSON format.
/// Use `serde_json` to serialize/deserialize.
///
/// # Example
///
/// ```
/// use gbe_protocol::ControlMessage;
///
/// let msg = ControlMessage::Connect {
///     capabilities: vec!["pty".to_string()],
/// };
///
/// // Serialize to JSON
/// let json = serde_json::to_string(&msg).unwrap();
///
/// // Deserialize from JSON
/// let parsed: ControlMessage = serde_json::from_str(&json).unwrap();
/// ```
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

impl ControlMessage {
    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, ProtocolError> {
        Ok(serde_json::to_string(self)?)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, ProtocolError> {
        Ok(serde_json::from_str(json)?)
    }

    /// Serialize to JSON bytes
    pub fn to_json_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(serde_json::to_vec(self)?)
    }

    /// Deserialize from JSON bytes
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        Ok(serde_json::from_slice(bytes)?)
    }
}

/// Data channel frame (binary serialization)
///
/// Wire format: [u32: length][u64: seq][bytes: payload]
/// - length: 4 bytes, big-endian (payload length, excluding header)
/// - seq: 8 bytes, big-endian (sequence number)
/// - payload: variable length (raw bytes)
#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Serialize to wire format: [u32: length][u64: seq][bytes: payload]
    pub fn to_bytes(&self) -> Vec<u8> {
        let len = self.payload.len() as u32;
        let mut bytes = Vec::with_capacity(4 + 8 + self.payload.len());

        // Write length (u32, big-endian)
        bytes.extend_from_slice(&len.to_be_bytes());

        // Write sequence (u64, big-endian)
        bytes.extend_from_slice(&self.seq.to_be_bytes());

        // Write payload
        bytes.extend_from_slice(&self.payload);

        bytes
    }

    /// Write frame to a writer
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), ProtocolError> {
        let bytes = self.to_bytes();
        writer.write_all(&bytes)?;
        Ok(())
    }

    /// Read frame from a reader
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self, ProtocolError> {
        // Read length (u32)
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Read sequence (u64)
        let mut seq_buf = [0u8; 8];
        reader.read_exact(&mut seq_buf)?;
        let seq = u64::from_be_bytes(seq_buf);

        // Read payload
        let mut payload = vec![0u8; len];
        reader.read_exact(&mut payload)?;

        Ok(Self { seq, payload })
    }

    /// Deserialize from bytes (for testing/convenience)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() < 12 {
            return Err(ProtocolError::InvalidFrame(format!(
                "Frame too short: {} bytes",
                bytes.len()
            )));
        }

        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let seq = u64::from_be_bytes([
            bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11],
        ]);

        if bytes.len() != 12 + len {
            return Err(ProtocolError::InvalidFrame(format!(
                "Expected {} bytes, got {}",
                12 + len,
                bytes.len()
            )));
        }

        let payload = bytes[12..].to_vec();

        Ok(Self { seq, payload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_message_connect() {
        let msg = ControlMessage::Connect {
            capabilities: vec!["pty".to_string(), "color".to_string()],
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ControlMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ControlMessage::Connect { capabilities } => {
                assert_eq!(capabilities.len(), 2);
                assert_eq!(capabilities[0], "pty");
                assert_eq!(capabilities[1], "color");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_control_message_connect_ack() {
        let msg = ControlMessage::ConnectAck {
            tool_id: "12345-001".to_string(),
            data_listen_address: "unix:///tmp/gbe-12345-001.sock".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ControlMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ControlMessage::ConnectAck {
                tool_id,
                data_listen_address,
            } => {
                assert_eq!(tool_id, "12345-001");
                assert_eq!(data_listen_address, "unix:///tmp/gbe-12345-001.sock");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_control_message_subscribe() {
        let msg = ControlMessage::Subscribe {
            target: "12345-002".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ControlMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ControlMessage::Subscribe { target } => {
                assert_eq!(target, "12345-002");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_control_message_error() {
        let msg = ControlMessage::Error {
            code: "NOT_FOUND".to_string(),
            message: "Tool not found".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ControlMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ControlMessage::Error { code, message } => {
                assert_eq!(code, "NOT_FOUND");
                assert_eq!(message, "Tool not found");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_data_frame_wire_format() {
        let frame = DataFrame::new(42, b"hello world".to_vec());

        // Test to_bytes and from_bytes
        let bytes = frame.to_bytes();
        let parsed = DataFrame::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.seq, 42);
        assert_eq!(parsed.payload, b"hello world");
        assert_eq!(parsed, frame);
    }

    #[test]
    fn test_data_frame_wire_format_structure() {
        let payload = b"test";
        let frame = DataFrame::new(100, payload.to_vec());

        let bytes = frame.to_bytes();

        // Verify wire format: [u32: length][u64: seq][bytes: payload]
        assert_eq!(bytes.len(), 4 + 8 + 4); // header + payload

        // Check length field (u32 big-endian)
        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(len, 4);

        // Check sequence field (u64 big-endian)
        let seq = u64::from_be_bytes([
            bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11],
        ]);
        assert_eq!(seq, 100);

        // Check payload
        assert_eq!(&bytes[12..], b"test");
    }

    #[test]
    fn test_data_frame_empty_payload() {
        let frame = DataFrame::new(0, vec![]);

        let bytes = frame.to_bytes();
        let parsed = DataFrame::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.seq, 0);
        assert_eq!(parsed.payload.len(), 0);
        assert_eq!(parsed, frame);
    }

    #[test]
    fn test_data_frame_large_payload() {
        let payload = vec![0xAB; 10000];
        let frame = DataFrame::new(999, payload.clone());

        let bytes = frame.to_bytes();
        let parsed = DataFrame::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.seq, 999);
        assert_eq!(parsed.payload, payload);
    }

    #[test]
    fn test_data_frame_invalid_short() {
        let bytes = vec![0u8; 5]; // Too short
        let result = DataFrame::from_bytes(&bytes);

        assert!(result.is_err());
        match result {
            Err(ProtocolError::InvalidFrame(msg)) => {
                assert!(msg.contains("too short"));
            }
            _ => panic!("Expected InvalidFrame error"),
        }
    }

    #[test]
    fn test_data_frame_invalid_length_mismatch() {
        let mut bytes = vec![0u8; 12];

        // Set length to 100 but don't provide payload
        bytes[0..4].copy_from_slice(&100u32.to_be_bytes());

        let result = DataFrame::from_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_data_frame_read_write() {
        use std::io::Cursor;

        let frame = DataFrame::new(42, b"test data".to_vec());

        // Write to buffer
        let mut buffer = Vec::new();
        frame.write_to(&mut buffer).unwrap();

        // Read from buffer
        let mut cursor = Cursor::new(buffer);
        let parsed = DataFrame::read_from(&mut cursor).unwrap();

        assert_eq!(parsed, frame);
    }
}
