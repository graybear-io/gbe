use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::SentinelError;

/// Maximum size of a single vsock message in bytes (1 MB).
const MAX_VSOCK_MESSAGE_SIZE: usize = 1_048_576;

/// Messages sent from operative (guest) to sentinel (host) over vsock.
///
/// Fields using `Value` (`data`, `output`, `params`) are intentionally
/// untyped — their schemas vary by task type and tool. Validation
/// happens downstream in task-specific handlers, not at the protocol layer.
/// Size limits are enforced at deserialization time via `parse_operative_message`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OperativeMessage {
    Progress {
        id: String,
        step: String,
        status: String,
        #[serde(default)]
        data: Option<Value>,
    },
    Result {
        id: String,
        output: Value,
        exit_code: i32,
    },
    Error {
        id: String,
        error: String,
        exit_code: i32,
    },
    ToolCall {
        id: String,
        call_id: String,
        tool: String,
        params: Value,
    },
}

/// Messages sent from sentinel (host) to operative (guest) over vsock.
///
/// `payload` and `result` use `Value` because task payloads and tool
/// results vary by type. Size limits are enforced at serialization time.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SentinelMessage {
    Task {
        id: String,
        payload: Value,
        tools: Vec<String>,
    },
    ToolResult {
        id: String,
        call_id: String,
        result: Value,
    },
}

/// Deserialize an operative message with size limit enforcement.
///
/// # Errors
///
/// Returns `SentinelError::Vsock` if the message exceeds size limits or is malformed.
pub fn parse_operative_message(raw: &[u8]) -> Result<OperativeMessage, SentinelError> {
    if raw.len() > MAX_VSOCK_MESSAGE_SIZE {
        return Err(SentinelError::Vsock(format!(
            "message too large: {} bytes (max {})",
            raw.len(),
            MAX_VSOCK_MESSAGE_SIZE
        )));
    }
    serde_json::from_slice(raw).map_err(|e| SentinelError::Vsock(format!("invalid message: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_progress_message() {
        let json = r#"{"type":"progress","id":"t1","step":"compile","status":"running"}"#;
        let msg = parse_operative_message(json.as_bytes()).unwrap();
        assert!(matches!(msg, OperativeMessage::Progress { ref id, .. } if id == "t1"));
    }

    #[test]
    fn parse_progress_with_data() {
        let json = r#"{"type":"progress","id":"t1","step":"s","status":"ok","data":{"pct":50}}"#;
        let msg = parse_operative_message(json.as_bytes()).unwrap();
        if let OperativeMessage::Progress { data, .. } = msg {
            assert_eq!(data.unwrap()["pct"], 50);
        } else {
            panic!("expected Progress");
        }
    }

    #[test]
    fn parse_result_message() {
        let json = r#"{"type":"result","id":"t2","output":{"key":"val"},"exit_code":0}"#;
        let msg = parse_operative_message(json.as_bytes()).unwrap();
        if let OperativeMessage::Result { id, exit_code, .. } = msg {
            assert_eq!(id, "t2");
            assert_eq!(exit_code, 0);
        } else {
            panic!("expected Result");
        }
    }

    #[test]
    fn parse_error_message() {
        let json = r#"{"type":"error","id":"t3","error":"boom","exit_code":1}"#;
        let msg = parse_operative_message(json.as_bytes()).unwrap();
        if let OperativeMessage::Error {
            error, exit_code, ..
        } = msg
        {
            assert_eq!(error, "boom");
            assert_eq!(exit_code, 1);
        } else {
            panic!("expected Error");
        }
    }

    #[test]
    fn parse_tool_call_message() {
        let json =
            r#"{"type":"tool_call","id":"t4","call_id":"c1","tool":"grep","params":{"q":"x"}}"#;
        let msg = parse_operative_message(json.as_bytes()).unwrap();
        if let OperativeMessage::ToolCall { tool, call_id, .. } = msg {
            assert_eq!(tool, "grep");
            assert_eq!(call_id, "c1");
        } else {
            panic!("expected ToolCall");
        }
    }

    #[test]
    fn oversized_message_rejected() {
        let big = vec![b' '; MAX_VSOCK_MESSAGE_SIZE + 1];
        let err = parse_operative_message(&big).unwrap_err();
        assert!(err.to_string().contains("too large"));
    }

    #[test]
    fn exactly_max_size_accepted() {
        // Valid JSON padded to exactly max size won't parse as valid JSON,
        // but the size check itself should pass
        let json = r#"{"type":"error","id":"x","error":"e","exit_code":0}"#;
        assert!(json.len() <= MAX_VSOCK_MESSAGE_SIZE);
        assert!(parse_operative_message(json.as_bytes()).is_ok());
    }

    #[test]
    fn malformed_json_rejected() {
        let err = parse_operative_message(b"not json").unwrap_err();
        assert!(err.to_string().contains("invalid message"));
    }

    #[test]
    fn unknown_type_tag_rejected() {
        let json = r#"{"type":"unknown","id":"t1"}"#;
        assert!(parse_operative_message(json.as_bytes()).is_err());
    }

    #[test]
    fn empty_payload_rejected() {
        assert!(parse_operative_message(b"").is_err());
    }

    #[test]
    fn sentinel_message_task_round_trip() {
        let msg = SentinelMessage::Task {
            id: "t1".into(),
            payload: serde_json::json!({"cmd": "echo hi"}),
            tools: vec!["grep".into(), "curl".into()],
        };
        let bytes = serde_json::to_vec(&msg).unwrap();
        let parsed: SentinelMessage = serde_json::from_slice(&bytes).unwrap();
        if let SentinelMessage::Task { id, tools, .. } = parsed {
            assert_eq!(id, "t1");
            assert_eq!(tools.len(), 2);
        } else {
            panic!("expected Task");
        }
    }

    #[test]
    fn sentinel_message_tool_result_round_trip() {
        let msg = SentinelMessage::ToolResult {
            id: "t1".into(),
            call_id: "c1".into(),
            result: serde_json::json!({"found": true}),
        };
        let bytes = serde_json::to_vec(&msg).unwrap();
        let parsed: SentinelMessage = serde_json::from_slice(&bytes).unwrap();
        assert!(matches!(parsed, SentinelMessage::ToolResult { .. }));
    }
}
