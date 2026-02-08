//! Test client for gbe-router
//!
//! Run this with: cargo run --package gbe-router --example test_client

use anyhow::Result;
use gbe_protocol::ControlMessage;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

fn main() -> Result<()> {
    println!("Connecting to router...");

    let stream = UnixStream::connect("/tmp/gbe-router.sock")?;
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);

    println!("Connected! Sending Connect message...");

    // Send Connect
    let connect = ControlMessage::Connect {
        capabilities: vec!["pty".to_string(), "color".to_string()],
    };
    let json = serde_json::to_string(&connect)?;
    writeln!(writer, "{}", json)?;
    writer.flush()?;

    // Receive ConnectAck
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let ack: ControlMessage = serde_json::from_str(line.trim())?;

    match ack {
        ControlMessage::ConnectAck { tool_id, data_listen_address } => {
            println!("✓ Received ConnectAck:");
            println!("  Tool ID: {}", tool_id);
            println!("  Data address: {}", data_listen_address);
        }
        msg => {
            println!("✗ Unexpected message: {:?}", msg);
        }
    }

    println!("\nSending Disconnect...");
    let disconnect = ControlMessage::Disconnect;
    let json = serde_json::to_string(&disconnect)?;
    writeln!(writer, "{}", json)?;
    writer.flush()?;

    println!("✓ Test complete!");

    Ok(())
}
