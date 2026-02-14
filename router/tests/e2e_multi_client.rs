//! Multi-client end-to-end test
//!
//! Tests: router + adapter + proxy + multiple clients
//!
//! This test verifies the proxy/tee functionality:
//! 1. Starts router
//! 2. Starts adapter wrapping "seq 1 5"
//! 3. Connects two clients to same adapter
//! 4. Verifies proxy is spawned (when needed)
//! 5. Verifies both clients receive same data
//! 6. Cleans up

use anyhow::{Context, Result};
use gbe_protocol::{ControlMessage, DataFrame};
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

struct TestProcess {
    child: Child,
    name: String,
}

impl TestProcess {
    fn start(name: &str, bin_path: &str, args: &[&str]) -> Result<Self> {
        println!("Starting {} from {}...", name, bin_path);

        let mut cmd = Command::new(bin_path);
        cmd.args(args);

        // Inherit stdout/stderr for debugging
        cmd.stdout(std::process::Stdio::inherit());
        cmd.stderr(std::process::Stdio::inherit());

        let child = cmd.spawn().context(format!("Failed to start {}", name))?;

        Ok(Self {
            child,
            name: name.to_string(),
        })
    }
}

impl Drop for TestProcess {
    fn drop(&mut self) {
        println!("Stopping {}...", self.name);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn wait_for_router() -> Result<()> {
    println!("Waiting for router...");
    for i in 0..50 {
        if std::path::Path::new("/tmp/gbe-router.sock").exists() {
            println!("✓ Router ready (after {}ms)", i * 100);
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    anyhow::bail!("Router socket not created after 5s")
}

fn discover_adapter_id() -> Result<String> {
    println!("Discovering adapter ToolId...");
    thread::sleep(Duration::from_millis(500));

    let output = Command::new("pgrep").args(["-f", "gbe-router"]).output()?;

    if output.status.success() {
        let pid_str = String::from_utf8_lossy(&output.stdout);
        if let Some(pid_line) = pid_str.lines().next() {
            if let Ok(pid) = pid_line.trim().parse::<u32>() {
                let tool_id = format!("{}-001", pid);
                println!("✓ Adapter ToolId: {}", tool_id);
                return Ok(tool_id);
            }
        }
    }

    anyhow::bail!("Could not discover adapter ToolId")
}

struct RouterConnection {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl RouterConnection {
    fn connect(socket: &str) -> Result<Self> {
        let stream = UnixStream::connect(socket)?;
        let writer = stream.try_clone()?;
        let reader = BufReader::new(stream);
        Ok(Self { reader, writer })
    }

    fn send(&mut self, msg: &ControlMessage) -> Result<()> {
        use std::io::Write;
        let json = serde_json::to_string(msg)?;
        writeln!(self.writer, "{}", json)?;
        self.writer.flush()?;
        Ok(())
    }

    fn recv(&mut self) -> Result<ControlMessage> {
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        Ok(serde_json::from_str(line.trim())?)
    }
}

fn connect_client(name: &str, target: &str) -> Result<(RouterConnection, UnixStream)> {
    println!("\n{} connecting to router...", name);

    let mut router_conn = RouterConnection::connect("/tmp/gbe-router.sock")?;

    // Connect
    router_conn.send(&ControlMessage::Connect {
        capabilities: vec![],
    })?;

    let _tool_id = match router_conn.recv()? {
        ControlMessage::ConnectAck { tool_id, .. } => {
            println!("✓ {} ToolId: {}", name, tool_id);
            tool_id
        }
        msg => anyhow::bail!("Expected ConnectAck, got {:?}", msg),
    };

    // Subscribe
    println!("{} subscribing to target: {}", name, target);
    router_conn.send(&ControlMessage::Subscribe {
        target: target.to_string(),
    })?;

    let data_addr = match router_conn.recv()? {
        ControlMessage::SubscribeAck {
            data_connect_address,
            ..
        } => {
            println!("✓ {} data address: {}", name, data_connect_address);
            data_connect_address
        }
        ControlMessage::Error { code, message } => {
            anyhow::bail!("Subscribe failed: {} - {}", code, message);
        }
        msg => anyhow::bail!("Expected SubscribeAck, got {:?}", msg),
    };

    // Connect to data stream
    let socket_path = data_addr
        .strip_prefix("unix://")
        .context("Invalid data address")?;

    println!("{} connecting to data stream...", name);
    let data_stream = UnixStream::connect(socket_path)?;
    println!("✓ {} data stream connected", name);

    Ok((router_conn, data_stream))
}

fn read_data_frames(
    name: &str,
    mut stream: UnixStream,
    expected_count: usize,
) -> Result<Vec<String>> {
    println!(
        "\n{} reading data frames (expecting {})...",
        name, expected_count
    );

    let mut lines = Vec::new();
    let mut count = 0;

    // Set timeout to avoid hanging
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;

    loop {
        match DataFrame::read_from(&mut stream) {
            Ok(frame) => {
                if let Ok(line) = String::from_utf8(frame.payload) {
                    let trimmed = line.trim();
                    println!("  {} [{}] {}", name, frame.seq, trimmed);
                    lines.push(trimmed.to_string());
                    count += 1;

                    if count >= expected_count {
                        println!("✓ {} received {} lines", name, count);
                        break;
                    }
                }
            }
            Err(e) => {
                let err_str = e.to_string();
                // Expected errors when stream ends or times out
                if err_str.contains("UnexpectedEof")
                    || err_str.contains("EOF")
                    || err_str.contains("timed out")
                    || err_str.contains("Unexpected end")
                    || err_str.contains("failed to fill whole buffer")
                {
                    println!("✓ {} stream closed after {} lines", name, count);
                    break;
                }
                return Err(e.into());
            }
        }
    }

    Ok(lines)
}

#[test]
#[ignore] // Requires pre-built binaries; runs in CI via `just test`
fn test_multi_client_proxy() -> Result<()> {
    println!("\n=== GBE Multi-Client E2E Test ===\n");

    // Clean up any old sockets
    let _ = std::fs::remove_file("/tmp/gbe-router.sock");

    // Get pre-built binary paths
    let router_bin = std::env::var("CARGO_BIN_EXE_gbe-router")
        .unwrap_or_else(|_| "../target/debug/gbe-router".to_string());
    let adapter_bin = std::env::var("CARGO_BIN_EXE_gbe-adapter")
        .unwrap_or_else(|_| "../target/debug/gbe-adapter".to_string());

    println!("Using router binary: {}", router_bin);
    println!("Using adapter binary: {}", adapter_bin);

    // Start router
    let router = TestProcess::start("router", &router_bin, &[])?;
    println!("Router started (PID: {})", router.child.id());
    wait_for_router()?;

    // Start adapter with a longer-running command (100 lines with delay)
    // This ensures adapter stays alive while both clients connect
    let adapter = TestProcess::start(
        "adapter",
        &adapter_bin,
        &[
            "sh",
            "-c",
            "for i in $(seq 1 100); do echo $i; sleep 0.01; done",
        ],
    )?;
    thread::sleep(Duration::from_millis(500));
    println!("✓ Adapter started (PID: {})", adapter.child.id());

    // Discover adapter ToolId
    let adapter_id = discover_adapter_id()?;

    // Connect BOTH clients BEFORE reading (while adapter is alive)
    // This tests the real-world scenario: multiple viewers of a running tool
    println!("\n--- Connecting both clients (parallel subscription) ---");

    let (_router_conn1, data_stream1) = connect_client("Client1", &adapter_id)?;
    let (_router_conn2, data_stream2) = connect_client("Client2", &adapter_id)?;

    // Check if proxy was spawned
    thread::sleep(Duration::from_millis(200));
    let proxy_check = Command::new("pgrep").args(["-f", "gbe-proxy"]).output()?;

    if proxy_check.status.success() {
        println!("\n✓ Proxy process detected (multi-subscriber tee active)");
    } else {
        println!("\n⚠️  No proxy process found");
        println!("   Note: Router may use direct routing for 2 clients in Phase 1");
    }

    // NOW start reading from both clients concurrently
    // Both clients are subscribed to the LIVE adapter
    // Read first 10 lines to verify they both get the same data
    println!("\n--- Reading data from both clients ---");

    let handle1 = thread::spawn(move || read_data_frames("Client1", data_stream1, 10));

    let handle2 = thread::spawn(move || read_data_frames("Client2", data_stream2, 10));

    // Wait for both clients to finish reading
    let lines1 = handle1.join().expect("Client1 thread panicked")?;
    let lines2 = handle2.join().expect("Client2 thread panicked")?;

    // Verify results
    println!("\n=== Verifying Results ===");

    let expected = vec!["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"];

    println!("Client1 received: {:?}", lines1);
    println!("Client2 received: {:?}", lines2);

    // Both clients should receive the same data
    assert_eq!(
        lines1.len(),
        expected.len(),
        "Client1 should receive {} lines",
        expected.len()
    );
    assert_eq!(
        lines2.len(),
        expected.len(),
        "Client2 should receive {} lines",
        expected.len()
    );

    // Verify content matches expected
    for i in 0..expected.len() {
        assert_eq!(
            lines1[i], expected[i],
            "Client1 line {} should be {}",
            i, expected[i]
        );
        assert_eq!(
            lines2[i], expected[i],
            "Client2 line {} should be {}",
            i, expected[i]
        );
    }

    // Both clients should receive identical data
    assert_eq!(lines1, lines2, "Both clients should receive identical data");

    println!("✓ Both clients received identical correct data");

    println!("\n=== ✓ Multi-Client Test Passed ===\n");

    Ok(())
}

#[test]
#[ignore] // Requires pre-built binaries; runs in CI via `just test`
fn test_subscribe_to_dead_tool() -> Result<()> {
    println!("\n=== GBE Subscribe to Dead Tool Test ===\n");

    // Clean up any old sockets
    let _ = std::fs::remove_file("/tmp/gbe-router.sock");

    // Get pre-built binary paths
    let router_bin = std::env::var("CARGO_BIN_EXE_gbe-router")
        .unwrap_or_else(|_| "../target/debug/gbe-router".to_string());
    let adapter_bin = std::env::var("CARGO_BIN_EXE_gbe-adapter")
        .unwrap_or_else(|_| "../target/debug/gbe-adapter".to_string());

    println!("Using router binary: {}", router_bin);
    println!("Using adapter binary: {}", adapter_bin);

    // Start router
    let router = TestProcess::start("router", &router_bin, &[])?;
    println!("Router started (PID: {})", router.child.id());
    wait_for_router()?;

    // Start adapter with a short-lived command
    let adapter = TestProcess::start(
        "adapter",
        &adapter_bin,
        &["sh", "-c", "echo done"],
    )?;
    thread::sleep(Duration::from_millis(500));
    println!("✓ Adapter started (PID: {})", adapter.child.id());

    // Discover adapter ToolId
    let adapter_id = discover_adapter_id()?;
    println!("Adapter ToolId: {}", adapter_id);

    // Wait for adapter to complete and disconnect
    println!("\nWaiting for adapter to complete...");
    thread::sleep(Duration::from_secs(2));
    println!("✓ Adapter should have exited by now");

    // Try to subscribe to the dead tool
    println!("\n--- Attempting to subscribe to dead tool ---");

    let result = connect_client("Client", &adapter_id);

    match result {
        Err(e) => {
            let err_msg = e.to_string();
            if err_msg.contains("NOT_FOUND") || err_msg.contains("not found") {
                println!("✓ Expected error: {}", err_msg);
                println!("✓ Router correctly rejects subscription to dead tool");
            } else {
                anyhow::bail!("Unexpected error: {}", err_msg);
            }
        }
        Ok(_) => {
            anyhow::bail!(
                "Expected subscription to fail for dead tool, but it succeeded"
            );
        }
    }

    println!("\n=== ✓ Subscribe to Dead Tool Test Passed ===\n");

    Ok(())
}
