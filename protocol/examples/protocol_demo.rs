//! Protocol demonstration showing all message types and usage patterns

use gbe_protocol::{ControlMessage, DataFrame};

fn main() {
    println!("=== GBE Protocol Examples ===\n");

    // Control Messages
    println!("## Control Messages (JSON)\n");

    // 1. Connect
    let connect = ControlMessage::Connect {
        capabilities: vec!["pty".to_string(), "color".to_string()],
    };
    println!("Connect:\n{}\n", serde_json::to_string_pretty(&connect).unwrap());

    // 2. ConnectAck
    let connect_ack = ControlMessage::ConnectAck {
        tool_id: "12345-001".to_string(),
        data_listen_address: "unix:///tmp/gbe-12345-001.sock".to_string(),
    };
    println!("ConnectAck:\n{}\n", serde_json::to_string_pretty(&connect_ack).unwrap());

    // 3. Subscribe
    let subscribe = ControlMessage::Subscribe {
        target: "12345-002".to_string(),
    };
    println!("Subscribe:\n{}\n", serde_json::to_string_pretty(&subscribe).unwrap());

    // 4. SubscribeAck
    let subscribe_ack = ControlMessage::SubscribeAck {
        data_connect_address: "unix:///tmp/gbe-12345-002.sock".to_string(),
        capabilities: vec!["raw".to_string()],
    };
    println!("SubscribeAck:\n{}\n", serde_json::to_string_pretty(&subscribe_ack).unwrap());

    // 5. FlowControl
    let flow_control = ControlMessage::FlowControl {
        source: "12345-001".to_string(),
        status: "backpressure".to_string(),
    };
    println!("FlowControl:\n{}\n", serde_json::to_string_pretty(&flow_control).unwrap());

    // 6. QueryCapabilities
    let query = ControlMessage::QueryCapabilities {
        target: "12345-002".to_string(),
    };
    println!("QueryCapabilities:\n{}\n", serde_json::to_string_pretty(&query).unwrap());

    // 7. CapabilitiesResponse
    let capabilities = ControlMessage::CapabilitiesResponse {
        capabilities: vec!["pty".to_string(), "color".to_string()],
    };
    println!("CapabilitiesResponse:\n{}\n", serde_json::to_string_pretty(&capabilities).unwrap());

    // 8. Error
    let error = ControlMessage::Error {
        code: "NOT_FOUND".to_string(),
        message: "Tool not found".to_string(),
    };
    println!("Error:\n{}\n", serde_json::to_string_pretty(&error).unwrap());

    // 9. Disconnect
    let disconnect = ControlMessage::Disconnect;
    println!("Disconnect:\n{}\n", serde_json::to_string_pretty(&disconnect).unwrap());

    // Data Frames
    println!("\n## Data Frames (Binary)\n");

    let frame = DataFrame::new(42, b"hello world".to_vec());
    let bytes = frame.to_bytes();

    println!("DataFrame:");
    println!("  seq: {}", frame.seq);
    println!("  payload: {:?}", String::from_utf8_lossy(&frame.payload));
    println!("  wire format ({} bytes): {:?}", bytes.len(), bytes);

    // Show wire format breakdown
    println!("\nWire format breakdown:");
    println!("  [0..4]   length:  {:?} = {} bytes", &bytes[0..4], u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
    println!("  [4..12]  seq:     {:?} = {}", &bytes[4..12], u64::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11]]));
    println!("  [12..]   payload: {:?}", &bytes[12..]);

    // Round-trip test
    let parsed = DataFrame::from_bytes(&bytes).unwrap();
    println!("\nRound-trip successful: {}", parsed == frame);
}
