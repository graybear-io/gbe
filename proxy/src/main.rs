//! GBE Proxy - Data channel tee for multiple subscribers
//!
//! Enables multiple tools to subscribe to the same data source by duplicating
//! data frames from one upstream to multiple downstream connections.
//!
//! Usage:
//!   gbe-proxy --router unix:///tmp/gbe-router.sock \
//!             --upstream unix:///tmp/gbe-12345-001.sock \
//!             --listen unix:///tmp/gbe-proxy-56789.sock \
//!             --mode framed

use anyhow::{Context, Result};
use clap::Parser;
use gbe_protocol::DataFrame;
use std::collections::HashMap;
use std::io::{BufReader, BufWriter, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// GBE Proxy - Tee data streams to multiple subscribers
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Router control socket address
    #[arg(long)]
    router: String,

    /// Upstream data socket to connect to
    #[arg(long)]
    upstream: String,

    /// Listen address for downstream connections
    #[arg(long)]
    listen: String,

    /// Data mode: framed (default) or raw
    #[arg(long, default_value = "framed")]
    mode: String,
}

/// Proxy state shared across threads
#[derive(Clone)]
struct ProxyState {
    /// Active downstream connections (ID -> stream writer)
    downstreams: Arc<Mutex<HashMap<u64, BufWriter<UnixStream>>>>,

    /// Next downstream connection ID
    next_id: Arc<Mutex<u64>>,

    /// Router control connection for FlowControl messages
    router_control: Arc<Mutex<Option<BufWriter<UnixStream>>>>,
}

impl ProxyState {
    fn new() -> Self {
        Self {
            downstreams: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
            router_control: Arc::new(Mutex::new(None)),
        }
    }

    /// Register a new downstream connection
    fn add_downstream(&self, stream: UnixStream) -> u64 {
        let mut id_lock = self.next_id.lock().unwrap();
        let id = *id_lock;
        *id_lock += 1;
        drop(id_lock);

        let writer = BufWriter::new(stream);
        let mut downstreams = self.downstreams.lock().unwrap();
        downstreams.insert(id, writer);
        info!("Downstream {} connected (total: {})", id, downstreams.len());

        id
    }

    /// Remove a downstream connection
    #[allow(dead_code)]
    fn remove_downstream(&self, id: u64) {
        let mut downstreams = self.downstreams.lock().unwrap();
        downstreams.remove(&id);
        info!(
            "Downstream {} disconnected (remaining: {})",
            id,
            downstreams.len()
        );
    }

    /// Get count of active downstreams
    fn downstream_count(&self) -> usize {
        self.downstreams.lock().unwrap().len()
    }

    /// Duplicate a frame to all downstream connections
    fn broadcast_frame(&self, frame: &DataFrame) -> Result<()> {
        let mut downstreams = self.downstreams.lock().unwrap();
        let mut failed_ids = Vec::new();

        for (id, writer) in downstreams.iter_mut() {
            match frame.write_to(writer) {
                Ok(_) => {
                    if let Err(e) = writer.flush() {
                        warn!("Failed to flush to downstream {}: {}", id, e);
                        failed_ids.push(*id);
                    }
                }
                Err(e) => {
                    warn!("Failed to write to downstream {}: {}", id, e);
                    failed_ids.push(*id);
                }
            }
        }

        // Remove failed connections
        for id in failed_ids {
            downstreams.remove(&id);
            info!("Removed failed downstream {}", id);
        }

        Ok(())
    }

    /// Send FlowControl message to router (not implemented in Phase 1)
    #[allow(dead_code)]
    fn send_flow_control(&self, status: &str) {
        let router = self.router_control.lock().unwrap();
        if let Some(ref _writer) = *router {
            debug!("FlowControl status: {} (logging only in Phase 1)", status);
            // Phase 1: Log only
            // Future: Send actual FlowControl message to router
        }
    }
}

fn main() -> Result<()> {
    // Parse arguments
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    info!("Starting gbe-proxy v{}", env!("CARGO_PKG_VERSION"));
    info!("Mode: {}", args.mode);
    info!("Upstream: {}", args.upstream);
    info!("Listen: {}", args.listen);

    // Validate mode
    if args.mode != "framed" && args.mode != "raw" {
        anyhow::bail!("Invalid mode: {} (must be 'framed' or 'raw')", args.mode);
    }

    // Initialize proxy state
    let state = ProxyState::new();

    // Extract socket path from upstream address
    let upstream_path = args
        .upstream
        .strip_prefix("unix://")
        .context("Upstream address must be unix://path")?;

    // Extract listen path from listen address
    let listen_path = args
        .listen
        .strip_prefix("unix://")
        .context("Listen address must be unix://path")?;

    // Clean up old listen socket if it exists
    let _ = std::fs::remove_file(listen_path);

    // Bind listener for downstream connections
    let listener = UnixListener::bind(listen_path).context("Failed to bind listener")?;
    info!("Listening for downstream connections at {}", args.listen);

    // Connect to upstream data source
    info!("Connecting to upstream...");
    let upstream_stream =
        UnixStream::connect(upstream_path).context("Failed to connect to upstream")?;
    upstream_stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    info!("Connected to upstream");

    // Spawn thread to accept downstream connections
    let accept_state = state.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let id = accept_state.add_downstream(stream);
                    debug!("Accepted downstream connection {}", id);
                }
                Err(e) => {
                    error!("Error accepting downstream connection: {}", e);
                }
            }
        }
    });

    // Main loop: read from upstream and broadcast to downstreams
    info!("Starting data relay loop");
    let mut reader = BufReader::new(upstream_stream);

    loop {
        // Check if we still have downstream connections
        if state.downstream_count() == 0 {
            warn!("No downstream connections, waiting...");
            thread::sleep(Duration::from_millis(500));
            continue;
        }

        // Read frame from upstream
        match DataFrame::read_from(&mut reader) {
            Ok(frame) => {
                debug!(
                    "Received frame seq={} len={}",
                    frame.seq,
                    frame.payload.len()
                );

                // Broadcast to all downstreams
                if let Err(e) = state.broadcast_frame(&frame) {
                    error!("Error broadcasting frame: {}", e);
                }
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("UnexpectedEof")
                    || err_str.contains("EOF")
                    || err_str.contains("timed out")
                {
                    info!("Upstream closed or timed out");
                    break;
                }
                error!("Error reading from upstream: {}", e);
                break;
            }
        }
    }

    info!("Proxy shutting down");
    Ok(())
}
