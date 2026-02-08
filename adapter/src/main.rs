//! GBE Adapter - Universal command wrapper
//!
//! Wraps any Unix command and bridges stdin/stdout to GBE protocol.
//!
//! See: docs/design/protocol-v1.md

use anyhow::{Context, Result};
use clap::Parser;
use gbe_protocol::{ControlMessage, DataFrame};
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixListener;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use tracing::{debug, error, info};

mod router_connection;

use router_connection::RouterConnection;

/// GBE Adapter - Universal command wrapper
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Router socket path
    #[arg(short, long, default_value = "/tmp/gbe-router.sock")]
    router: String,

    /// Command to execute
    #[arg(trailing_var_arg = true, required = true)]
    command: Vec<String>,
}

fn main() -> Result<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    info!("gbe-adapter v{}", env!("CARGO_PKG_VERSION"));
    info!("Command: {:?}", args.command);

    // Connect to router
    let mut router_conn = RouterConnection::connect(&args.router)?;

    // Register with router
    info!("Connecting to router...");
    router_conn.send(&ControlMessage::Connect {
        capabilities: vec![],
    })?;

    // Get assigned ToolId and data address
    let (_tool_id, data_addr) = match router_conn.recv()? {
        ControlMessage::ConnectAck { tool_id, data_listen_address } => {
            info!("Assigned ToolId: {}", tool_id);
            info!("Data address: {}", data_listen_address);
            (tool_id, data_listen_address)
        }
        msg => {
            anyhow::bail!("Expected ConnectAck, got {:?}", msg);
        }
    };

    // Extract path from unix:// URL
    let socket_path = data_addr
        .strip_prefix("unix://")
        .context("Invalid data address format")?;

    // Bind data listener
    let _ = std::fs::remove_file(socket_path); // Clean up if exists
    let data_listener = UnixListener::bind(socket_path)
        .context("Failed to bind data listener")?;

    info!("Data listener bound at {}", socket_path);

    // Spawn command
    let program = &args.command[0];
    let cmd_args = &args.command[1..];

    info!("Spawning command: {} {:?}", program, cmd_args);

    let mut child = Command::new(program)
        .args(cmd_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn command")?;

    let stdout = child.stdout.take().context("No stdout")?;
    let stderr = child.stderr.take().context("No stderr")?;

    // Sequence counter for frames
    let seq = Arc::new(AtomicU64::new(0));

    // Accept data connections in background
    let seq_clone = seq.clone();
    let data_thread = thread::spawn(move || {
        handle_data_stream(data_listener, stdout, stderr, seq_clone)
    });

    // Wait for command to exit
    let exit_status = child.wait().context("Failed to wait for child")?;

    info!("Command exited: {:?}", exit_status);

    // Wait for data thread to finish
    data_thread.join().expect("Data thread panicked")?;

    // Disconnect from router
    router_conn.send(&ControlMessage::Disconnect)?;

    // Cleanup socket
    let _ = std::fs::remove_file(socket_path);

    info!("Adapter shutdown complete");

    Ok(())
}

/// Handle data streaming to subscribers
fn handle_data_stream(
    listener: UnixListener,
    stdout: impl std::io::Read + Send + 'static,
    stderr: impl std::io::Read + Send + 'static,
    seq: Arc<AtomicU64>,
) -> Result<()> {
    // Accept first connection (Phase 1: single subscriber)
    let (stream, _addr) = listener.accept()
        .context("Failed to accept data connection")?;

    info!("Data subscriber connected");

    let data_conn_stdout = stream.try_clone()
        .context("Failed to clone data stream")?;
    let data_conn_stderr = stream;

    // Stream stdout
    let stdout_seq = seq.clone();
    let stdout_thread = thread::spawn(move || {
        let mut conn = data_conn_stdout;
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let mut data = line.into_bytes();
                    data.push(b'\n');

                    let frame = DataFrame::new(
                        stdout_seq.fetch_add(1, Ordering::SeqCst),
                        data,
                    );

                    if let Err(e) = frame.write_to(&mut conn) {
                        error!("Failed to write stdout frame: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Error reading stdout: {}", e);
                    break;
                }
            }
        }
        debug!("Stdout streaming complete");
    });

    // Stream stderr (in same connection, interleaved)
    let stderr_seq = seq;
    let stderr_thread = thread::spawn(move || {
        let mut conn = data_conn_stderr;
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let mut data = format!("[stderr] {}", line).into_bytes();
                    data.push(b'\n');

                    let frame = DataFrame::new(
                        stderr_seq.fetch_add(1, Ordering::SeqCst),
                        data,
                    );

                    if let Err(e) = frame.write_to(&mut conn) {
                        error!("Failed to write stderr frame: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Error reading stderr: {}", e);
                    break;
                }
            }
        }
        debug!("Stderr streaming complete");
    });

    stdout_thread.join().expect("Stdout thread panicked");
    stderr_thread.join().expect("Stderr thread panicked");

    info!("Data streaming complete");

    Ok(())
}
