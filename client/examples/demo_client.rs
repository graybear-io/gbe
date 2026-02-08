//! Demo client example
//!
//! This is a non-interactive demo that connects to a running adapter
//! and displays output for testing purposes.
//!
//! Usage:
//! 1. Start router: cargo run --package gbe-router
//! 2. Start adapter: cargo run --package gbe-adapter -- seq 1 10
//! 3. Run this: cargo run --package gbe-client --example demo_client

use anyhow::{Context, Result};
use gbe_protocol::{ControlMessage, DataFrame};
use std::os::unix::net::UnixStream;
use std::process::Command;

fn main() -> Result<()> {
    println!("GBE Client Demo");
    println!("===============\n");

    // Connect to router
    let router_socket = "/tmp/gbe-router.sock";
    println!("Connecting to router at {}...", router_socket);

    let stream = UnixStream::connect(router_socket)
        .context("Failed to connect to router. Is the router running?")?;

    let mut control_reader = std::io::BufReader::new(stream.try_clone()?);
    let mut control_writer = stream;

    // Send Connect message
    let connect_msg = ControlMessage::Connect {
        capabilities: vec![],
    };
    send_message(&mut control_writer, &connect_msg)?;

    // Receive ConnectAck
    let response = recv_message(&mut control_reader)?;
    let _tool_id = match response {
        ControlMessage::ConnectAck { tool_id, .. } => {
            println!("✓ Connected! ToolId: {}", tool_id);
            tool_id
        }
        msg => anyhow::bail!("Expected ConnectAck, got {:?}", msg),
    };

    // Discover adapter ToolId
    println!("\nDiscovering adapter...");
    let adapter_id = discover_adapter_id()?;
    println!("✓ Found adapter: {}", adapter_id);

    // Subscribe to adapter
    println!("\nSubscribing to adapter...");
    let subscribe_msg = ControlMessage::Subscribe { target: adapter_id };
    send_message(&mut control_writer, &subscribe_msg)?;

    // Receive SubscribeAck
    let response = recv_message(&mut control_reader)?;
    let data_addr = match response {
        ControlMessage::SubscribeAck {
            data_connect_address,
            ..
        } => {
            println!("✓ Subscribed! Data address: {}", data_connect_address);
            data_connect_address
        }
        ControlMessage::Error { code, message } => {
            anyhow::bail!("Subscription failed: {} - {}", code, message);
        }
        msg => anyhow::bail!("Expected SubscribeAck, got {:?}", msg),
    };

    // Connect to data stream
    let socket_path = data_addr
        .strip_prefix("unix://")
        .context("Invalid data address")?;

    println!("\nConnecting to data stream...");
    let mut data_stream = UnixStream::connect(socket_path)?;
    println!("✓ Connected to data stream");

    println!("\nReceiving data:\n");

    // Read data frames
    let mut count = 0;
    loop {
        match DataFrame::read_from(&mut data_stream) {
            Ok(frame) => {
                if let Ok(line) = String::from_utf8(frame.payload) {
                    println!("  [{}] {}", frame.seq, line.trim());
                    count += 1;

                    // Exit after receiving some data (for demo)
                    if count >= 10 {
                        break;
                    }
                }
            }
            Err(e) => {
                println!("\n✓ Data stream closed: {}", e);
                break;
            }
        }
    }

    // Disconnect
    println!("\nDisconnecting...");
    let disconnect_msg = ControlMessage::Disconnect;
    send_message(&mut control_writer, &disconnect_msg)?;
    println!("✓ Disconnected");

    Ok(())
}

fn send_message(writer: &mut UnixStream, msg: &ControlMessage) -> Result<()> {
    use std::io::Write;
    let json = serde_json::to_string(msg)?;
    writeln!(writer, "{}", json)?;
    writer.flush()?;
    Ok(())
}

fn recv_message(reader: &mut std::io::BufReader<UnixStream>) -> Result<ControlMessage> {
    use std::io::BufRead;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(serde_json::from_str(line.trim())?)
}

fn discover_adapter_id() -> Result<String> {
    // Try to find router process and construct adapter ToolId
    let output = Command::new("pgrep").args(["-f", "gbe-router"]).output()?;

    if output.status.success() {
        let pid_str = String::from_utf8_lossy(&output.stdout);
        if let Some(pid_line) = pid_str.lines().next() {
            if let Ok(pid) = pid_line.trim().parse::<u32>() {
                return Ok(format!("{}-001", pid));
            }
        }
    }

    anyhow::bail!("Could not discover adapter ToolId")
}
