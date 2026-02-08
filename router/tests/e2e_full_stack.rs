//! Full stack end-to-end test
//!
//! Tests: router + adapter + client integration
//!
//! This test:
//! 1. Starts router
//! 2. Starts adapter wrapping "seq 1 10"
//! 3. Connects client (non-interactive)
//! 4. Verifies client receives all output
//! 5. Cleans up

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
    fn start(name: &str, package: &str, args: &[&str]) -> Result<Self> {
        println!("Starting {}...", name);

        let mut cmd = Command::new("cargo");
        cmd.args(&["run", "--package", package, "--quiet"]);

        if !args.is_empty() {
            cmd.arg("--");
            cmd.args(args);
        }

        let child = cmd.spawn()
            .context(format!("Failed to start {}", name))?;

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
    for _ in 0..20 {
        if std::path::Path::new("/tmp/gbe-router.sock").exists() {
            println!("✓ Router ready");
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    anyhow::bail!("Router socket not created after 2s")
}

fn discover_adapter_id() -> Result<String> {
    println!("Discovering adapter ToolId...");
    thread::sleep(Duration::from_millis(500));

    let output = Command::new("pgrep")
        .args(&["-f", "gbe-router"])
        .output()?;

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

fn connect_client(target: &str) -> Result<(RouterConnection, UnixStream)> {
    println!("\nClient connecting to router...");

    let mut router_conn = RouterConnection::connect("/tmp/gbe-router.sock")?;

    // Connect
    router_conn.send(&ControlMessage::Connect {
        capabilities: vec![],
    })?;

    let _tool_id = match router_conn.recv()? {
        ControlMessage::ConnectAck { tool_id, .. } => {
            println!("✓ Client ToolId: {}", tool_id);
            tool_id
        }
        msg => anyhow::bail!("Expected ConnectAck, got {:?}", msg),
    };

    // Subscribe
    println!("Subscribing to target: {}", target);
    router_conn.send(&ControlMessage::Subscribe {
        target: target.to_string(),
    })?;

    let data_addr = match router_conn.recv()? {
        ControlMessage::SubscribeAck {
            data_connect_address,
            ..
        } => {
            println!("✓ Data address: {}", data_connect_address);
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

    println!("Connecting to data stream...");
    let data_stream = UnixStream::connect(socket_path)?;
    println!("✓ Data stream connected");

    Ok((router_conn, data_stream))
}

fn read_data_frames(mut stream: UnixStream, expected_count: usize) -> Result<Vec<String>> {
    println!("\nReading data frames (expecting {})...", expected_count);

    let mut lines = Vec::new();
    let mut count = 0;

    loop {
        match DataFrame::read_from(&mut stream) {
            Ok(frame) => {
                if let Ok(line) = String::from_utf8(frame.payload) {
                    let trimmed = line.trim();
                    println!("  [{}] {}", frame.seq, trimmed);
                    lines.push(trimmed.to_string());
                    count += 1;

                    if count >= expected_count {
                        println!("✓ Received {} lines", count);
                        break;
                    }
                }
            }
            Err(_) => {
                println!("✓ Stream closed after {} lines", count);
                break;
            }
        }
    }

    Ok(lines)
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

#[test]
#[ignore] // Run with: cargo test --test e2e_full_stack -- --ignored --nocapture
fn test_full_stack_integration() -> Result<()> {
    println!("\n=== GBE Full Stack E2E Test ===\n");

    // Clean up any old sockets
    let _ = std::fs::remove_file("/tmp/gbe-router.sock");

    // Start router
    let _router = TestProcess::start("router", "gbe-router", &[])?;
    wait_for_router()?;

    // Start adapter with "seq 1 10"
    let _adapter = TestProcess::start("adapter", "gbe-adapter", &["seq", "1", "10"])?;
    thread::sleep(Duration::from_millis(500));
    println!("✓ Adapter started");

    // Discover adapter ToolId
    let adapter_id = discover_adapter_id()?;

    // Connect client and read data
    let (mut router_conn, data_stream) = connect_client(&adapter_id)?;
    let lines = read_data_frames(data_stream, 10)?;

    // Verify output
    println!("\nVerifying output...");
    assert_eq!(lines.len(), 10, "Should receive 10 lines");

    for (i, line) in lines.iter().enumerate() {
        let expected = (i + 1).to_string();
        assert_eq!(line, &expected, "Line {} should be {}", i, expected);
    }

    println!("✓ All lines correct");

    // Disconnect
    router_conn.send(&ControlMessage::Disconnect)?;
    println!("✓ Client disconnected");

    println!("\n=== ✓ Full Stack Test Passed ===\n");

    Ok(())
}
