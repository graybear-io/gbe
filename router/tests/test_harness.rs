//! Test harness for GBE integration tests
//!
//! Provides isolated test environments with automatic cleanup and unique resource paths.
//!
//! # Example
//!
//! ```no_run
//! use test_harness::TestEnv;
//!
//! let env = TestEnv::new()?;
//! let router = env.start_router()?;
//! let adapter = env.start_adapter(&["seq", "1", "10"])?;
//! let client = env.connect_client()?;
//!
//! // Test interactions...
//!
//! // Automatic cleanup on drop
//! ```

use anyhow::{Context, Result};
use gbe_protocol::ControlMessage;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::Duration;

/// Global counter for unique test IDs
static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Test environment providing isolated resources and automatic cleanup
pub struct TestEnv {
    /// Unique ID for this test environment
    #[allow(dead_code)]
    pub id: u32,
    /// Socket path for router control channel
    pub router_socket: String,
    /// Running router process (if started)
    router: Option<Child>,
    /// Cleanup handler
    _cleanup: Cleanup,
}

impl TestEnv {
    /// Create a new isolated test environment
    ///
    /// Each environment gets unique socket paths to avoid collisions when tests run in parallel.
    pub fn new() -> Result<Self> {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let router_socket = format!("/tmp/gbe-test-{}-{}.sock", pid, id);

        // Clean up any existing socket
        let _ = std::fs::remove_file(&router_socket);

        Ok(Self {
            id,
            router_socket: router_socket.clone(),
            router: None,
            _cleanup: Cleanup::new(router_socket),
        })
    }

    /// Start the GBE router process
    ///
    /// Returns a RouterHandle for interacting with the router.
    pub fn start_router(&mut self) -> Result<RouterHandle> {
        let router_bin = get_binary_path("gbe-router")?;

        let child = Command::new(&router_bin)
            .args(["--socket", &self.router_socket])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .context(format!("Failed to start router from {}", router_bin))?;

        let pid = child.id();
        self.router = Some(child);

        // Wait for socket to be created
        self.wait_for_socket(&self.router_socket)?;

        Ok(RouterHandle {
            socket_path: self.router_socket.clone(),
            pid,
        })
    }

    /// Start an adapter wrapping the given command
    ///
    /// The adapter will connect to the router and run the command.
    #[allow(dead_code)]
    pub fn start_adapter(&self, command_args: &[&str]) -> Result<AdapterHandle> {
        let adapter_bin = get_binary_path("gbe-adapter")?;

        let child = Command::new(&adapter_bin)
            .args(command_args)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .context(format!("Failed to start adapter from {}", adapter_bin))?;

        let pid = child.id();

        // Give adapter time to connect
        thread::sleep(Duration::from_millis(200));

        Ok(AdapterHandle { child, pid })
    }

    /// Connect a test client to the router
    pub fn connect_client(&self) -> Result<ClientConnection> {
        let stream = UnixStream::connect(&self.router_socket).context(format!(
            "Failed to connect to router at {}",
            self.router_socket
        ))?;

        let writer = stream.try_clone()?;
        let reader = BufReader::new(stream);

        Ok(ClientConnection { reader, writer })
    }

    /// Wait for a socket file to be created
    fn wait_for_socket(&self, socket_path: &str) -> Result<()> {
        for i in 0..50 {
            if std::path::Path::new(socket_path).exists() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(10));
            if i == 49 {
                anyhow::bail!("Socket {} not created after 500ms", socket_path);
            }
        }
        Ok(())
    }
}

/// Handle to a running router process
#[allow(dead_code)]
pub struct RouterHandle {
    pub socket_path: String,
    pub pid: u32,
}

/// Handle to a running adapter process
#[allow(dead_code)]
pub struct AdapterHandle {
    pub child: Child,
    pub pid: u32,
}

impl Drop for AdapterHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Client connection to router control channel
pub struct ClientConnection {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl ClientConnection {
    /// Send a control message to the router
    pub fn send(&mut self, msg: &ControlMessage) -> Result<()> {
        let json = serde_json::to_string(msg)?;
        writeln!(self.writer, "{}", json)?;
        self.writer.flush()?;
        Ok(())
    }

    /// Receive a control message from the router
    pub fn recv(&mut self) -> Result<ControlMessage> {
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        Ok(serde_json::from_str(line.trim())?)
    }
}

/// Cleanup handler for test resources
struct Cleanup {
    socket_path: String,
}

impl Cleanup {
    fn new(socket_path: String) -> Self {
        Self { socket_path }
    }
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        // Clean up socket
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        // Kill router if running
        if let Some(mut router) = self.router.take() {
            let _ = router.kill();
            let _ = router.wait();
        }
    }
}

/// Get the path to a pre-built binary
fn get_binary_path(name: &str) -> Result<String> {
    // Try CARGO_BIN_EXE_* env var first (when available)
    let env_var = format!("CARGO_BIN_EXE_{}", name.replace('-', "_"));
    if let Ok(path) = std::env::var(&env_var) {
        return Ok(path);
    }

    // Fall back to workspace target directory
    let mut path = std::env::current_dir()?;
    path.push("../target/debug");
    path.push(name);

    path.canonicalize()
        .context(format!("Binary {} not found at {:?}", name, path))?
        .to_str()
        .map(|s| s.to_string())
        .context("Invalid path encoding")
}
