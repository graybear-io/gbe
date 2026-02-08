//! End-to-end test: Router + Adapter + Subscriber
//!
//! Run in 3 terminals:
//! 1. cargo run --package gbe-router
//! 2. cargo run --package gbe-adapter -- seq 1 5
//! 3. cargo run --package gbe-adapter --example e2e_test

use anyhow::{Context, Result};
use gbe_protocol::{ControlMessage, DataFrame};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

fn main() -> Result<()> {
    println!("=== End-to-End Test: Adapter Subscriber ===\n");

    // Connect to router
    println!("1. Connecting to router...");
    let router_stream = UnixStream::connect("/tmp/gbe-router.sock")
        .context("Failed to connect to router")?;

    let mut router_writer = router_stream.try_clone()?;
    let mut router_reader = BufReader::new(router_stream);

    // Send Connect
    let connect = ControlMessage::Connect {
        capabilities: vec![],
    };
    let json = serde_json::to_string(&connect)?;
    writeln!(router_writer, "{}", json)?;
    router_writer.flush()?;

    // Receive ConnectAck
    let mut line = String::new();
    router_reader.read_line(&mut line)?;
    let ack: ControlMessage = serde_json::from_str(line.trim())?;

    let _my_tool_id = match ack {
        ControlMessage::ConnectAck { tool_id, .. } => {
            println!("   ✓ My ToolId: {}", tool_id);
            tool_id
        }
        msg => anyhow::bail!("Expected ConnectAck, got {:?}", msg),
    };

    // Ask user for target ToolId
    println!("\n2. Enter the ToolId of the adapter to subscribe to:");
    println!("   (Check adapter output for 'Assigned ToolId: XXXXX-XXX')");
    print!("   ToolId: ");
    std::io::stdout().flush()?;

    let mut target_id = String::new();
    std::io::stdin().read_line(&mut target_id)?;
    let target_id = target_id.trim().to_string();

    // Subscribe to target
    println!("\n3. Subscribing to {}...", target_id);
    let subscribe = ControlMessage::Subscribe {
        target: target_id.clone(),
    };
    let json = serde_json::to_string(&subscribe)?;
    writeln!(router_writer, "{}", json)?;
    router_writer.flush()?;

    // Receive SubscribeAck
    line.clear();
    router_reader.read_line(&mut line)?;
    let sub_ack: ControlMessage = serde_json::from_str(line.trim())?;

    let data_addr = match sub_ack {
        ControlMessage::SubscribeAck { data_connect_address, .. } => {
            println!("   ✓ Data address: {}", data_connect_address);
            data_connect_address
        }
        ControlMessage::Error { code, message } => {
            anyhow::bail!("Subscription failed: {} - {}", code, message);
        }
        msg => anyhow::bail!("Expected SubscribeAck, got {:?}", msg),
    };

    // Connect to data channel
    let socket_path = data_addr.strip_prefix("unix://")
        .context("Invalid data address")?;

    println!("\n4. Connecting to data channel: {}...", socket_path);
    let mut data_stream = UnixStream::connect(socket_path)
        .context("Failed to connect to data channel")?;

    println!("   ✓ Connected!\n");

    // Read and display data frames
    println!("5. Receiving data frames:\n");
    println!("---");

    let mut frame_count = 0;
    loop {
        match DataFrame::read_from(&mut data_stream) {
            Ok(frame) => {
                frame_count += 1;
                let payload = String::from_utf8_lossy(&frame.payload);
                print!("{}", payload); // Already has newline
            }
            Err(e) => {
                if e.to_string().contains("UnexpectedEof") || e.to_string().contains("EOF") {
                    break;
                }
                eprintln!("\nError reading frame: {}", e);
                break;
            }
        }
    }

    println!("---");
    println!("\n✓ Received {} frames", frame_count);
    println!("✓ End-to-end test complete!");

    Ok(())
}
