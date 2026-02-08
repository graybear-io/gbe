//! Automated end-to-end test
//!
//! Usage: cargo run --package gbe-adapter --example automated_e2e
//!
//! This will:
//! 1. Start router in background
//! 2. Start adapter wrapping "seq 1 5" in background
//! 3. Subscribe and read output
//! 4. Verify we received all 5 lines
//! 5. Cleanup

use anyhow::{Context, Result};
use gbe_protocol::{ControlMessage, DataFrame};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

struct RouterProcess {
    child: Child,
}

impl RouterProcess {
    fn start() -> Result<Self> {
        // Clean up old socket
        let _ = std::fs::remove_file("/tmp/gbe-router.sock");

        println!("Starting router...");
        let child = Command::new("cargo")
            .args(&["run", "--package", "gbe-router", "--quiet"])
            .spawn()
            .context("Failed to start router")?;

        // Wait for router to start
        thread::sleep(Duration::from_millis(500));

        // Check if socket exists
        for _ in 0..10 {
            if std::path::Path::new("/tmp/gbe-router.sock").exists() {
                println!("✓ Router started");
                return Ok(Self { child });
            }
            thread::sleep(Duration::from_millis(100));
        }

        anyhow::bail!("Router socket not created");
    }
}

impl Drop for RouterProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file("/tmp/gbe-router.sock");
    }
}

struct AdapterProcess {
    child: Child,
}

impl AdapterProcess {
    fn start() -> Result<Self> {
        println!("\nStarting adapter (seq 1 5)...");

        let child = Command::new("cargo")
            .args(&[
                "run",
                "--package",
                "gbe-adapter",
                "--quiet",
                "--",
                "seq",
                "1",
                "5",
            ])
            .spawn()
            .context("Failed to start adapter")?;

        // Wait for adapter to connect
        thread::sleep(Duration::from_millis(500));

        println!("✓ Adapter started");

        Ok(Self { child })
    }

    fn discover_tool_id() -> Result<String> {
        // Connect to router and list tools
        // For Phase 1, we just use the first tool (router PID + 001)

        // Try to read router PID from socket path or process list
        // Simple heuristic: recent router process
        let output = Command::new("pgrep").args(&["-f", "gbe-router"]).output()?;

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
}

impl Drop for AdapterProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn main() -> Result<()> {
    println!("=== Automated End-to-End Test ===\n");

    // Start router
    let _router = RouterProcess::start()?;

    // Start adapter
    let _adapter = AdapterProcess::start()?;

    // Give everything a moment to settle
    thread::sleep(Duration::from_millis(500));

    // Discover adapter's ToolId
    let adapter_tool_id =
        AdapterProcess::discover_tool_id().context("Failed to discover adapter ToolId")?;

    println!("✓ Discovered adapter ToolId: {}", adapter_tool_id);

    // Connect to router as subscriber
    println!("\nConnecting to router as subscriber...");
    let router_stream =
        UnixStream::connect("/tmp/gbe-router.sock").context("Failed to connect to router")?;

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
    let _ack: ControlMessage = serde_json::from_str(line.trim())?;
    println!("✓ Subscriber connected");

    // Subscribe to adapter
    println!("\nSubscribing to adapter {}...", adapter_tool_id);
    let subscribe = ControlMessage::Subscribe {
        target: adapter_tool_id.clone(),
    };
    let json = serde_json::to_string(&subscribe)?;
    writeln!(router_writer, "{}", json)?;
    router_writer.flush()?;

    // Receive SubscribeAck
    line.clear();
    router_reader.read_line(&mut line)?;
    let sub_ack: ControlMessage = serde_json::from_str(line.trim())?;

    let data_addr = match sub_ack {
        ControlMessage::SubscribeAck {
            data_connect_address,
            ..
        } => {
            println!("✓ Subscription successful");
            data_connect_address
        }
        ControlMessage::Error { code, message } => {
            anyhow::bail!("Subscription failed: {} - {}", code, message);
        }
        msg => anyhow::bail!("Expected SubscribeAck, got {:?}", msg),
    };

    // Connect to data channel
    let socket_path = data_addr
        .strip_prefix("unix://")
        .context("Invalid data address")?;

    println!("\nConnecting to data channel...");
    let mut data_stream =
        UnixStream::connect(socket_path).context("Failed to connect to data channel")?;
    println!("✓ Data channel connected");

    // Read data frames
    println!("\nReceiving output:\n---");

    let mut frames = Vec::new();
    let mut lines = Vec::new();

    // Read with timeout
    data_stream.set_read_timeout(Some(Duration::from_secs(2)))?;

    loop {
        match DataFrame::read_from(&mut data_stream) {
            Ok(frame) => {
                let payload = String::from_utf8_lossy(&frame.payload);
                print!("{}", payload);
                lines.push(payload.trim().to_string());
                frames.push(frame);
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("UnexpectedEof")
                    || err_str.contains("EOF")
                    || err_str.contains("timed out")
                    || err_str.contains("Unexpected end")
                    || err_str.contains("failed to fill whole buffer")
                {
                    break;
                }
                anyhow::bail!("Error reading frame: {}", e);
            }
        }
    }

    println!("---");

    // Verify results
    println!("\n=== Results ===");
    println!("✓ Received {} frames", frames.len());

    let expected = vec!["1", "2", "3", "4", "5"];
    if lines == expected {
        println!("✓ Output matches expected: {:?}", expected);
        println!("\n🎉 END-TO-END TEST PASSED!");
    } else {
        println!("✗ Output mismatch");
        println!("  Expected: {:?}", expected);
        println!("  Received: {:?}", lines);
        anyhow::bail!("Test failed");
    }

    Ok(())
}
