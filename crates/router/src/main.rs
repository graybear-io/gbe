//! GBE Router - Message broker for GBE components
//!
//! Routes control messages between tools and orchestrates data channel connections.
//!
//! See: docs/design/protocol-v1.md

use anyhow::{Context, Result};
use clap::Parser;
use gbe_protocol::{ControlMessage, ToolId, ToolInfo};
use std::collections::HashMap;
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::{self, Child, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

mod connection;

use connection::Connection;

/// GBE Router - Message broker for GBE components
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Unix socket path to listen on
    #[arg(short, long, default_value = "/tmp/gbe-router.sock")]
    socket: String,
}

/// Router state shared across connections
#[derive(Clone)]
struct RouterState {
    /// Next sequence number for `ToolId` assignment
    next_seq: Arc<AtomicU64>,

    /// Active connections: `ToolId` -> Connection info
    connections: Arc<Mutex<HashMap<ToolId, ConnectionInfo>>>,

    /// Subscriptions: source `ToolId` -> list of subscriber `ToolIds`
    subscriptions: Arc<Mutex<HashMap<ToolId, Vec<ToolId>>>>,

    /// Proxies: source `ToolId` -> `ProxyInfo` (spawned when multiple subscribers)
    proxies: Arc<Mutex<HashMap<ToolId, ProxyInfo>>>,
}

/// Information about a connected tool
#[derive(Debug, Clone)]
struct ConnectionInfo {
    #[allow(dead_code)]
    tool_id: ToolId,
    data_listen_address: String,
    capabilities: Vec<String>,
    metadata: std::collections::HashMap<String, String>,
}

/// Information about a spawned proxy subprocess
#[derive(Debug)]
struct ProxyInfo {
    /// Proxy subprocess handle
    #[allow(dead_code)] // Kept for future cleanup/lifecycle management
    child: Child,
    /// Proxy listen address (where subscribers connect)
    listen_address: String,
    /// Source tool this proxy is teeing
    #[allow(dead_code)] // May be used for debugging/logging in future
    source_tool_id: ToolId,
}

impl RouterState {
    fn new() -> Self {
        Self {
            next_seq: Arc::new(AtomicU64::new(1)),
            connections: Arc::new(Mutex::new(HashMap::new())),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            proxies: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Assign a new `ToolId` in "PID-SEQ" format
    fn assign_tool_id(&self) -> ToolId {
        let pid = process::id();
        let seq = self.next_seq.fetch_add(1, Ordering::SeqCst);
        format!("{pid}-{seq:03}")
    }

    /// Generate data listen address for a tool
    fn assign_data_address(tool_id: &str) -> String {
        format!("unix:///tmp/gbe-{tool_id}.sock")
    }

    /// Register a new connection
    fn register_connection(
        &self,
        tool_id: ToolId,
        data_address: String,
        capabilities: Vec<String>,
        metadata: std::collections::HashMap<String, String>,
    ) {
        let mut conns = self.connections.lock().unwrap();
        conns.insert(
            tool_id.clone(),
            ConnectionInfo {
                tool_id,
                data_listen_address: data_address,
                capabilities,
                metadata,
            },
        );
    }

    /// Unregister a connection
    fn unregister_connection(&self, tool_id: &ToolId) {
        let mut conns = self.connections.lock().unwrap();
        conns.remove(tool_id);

        // Clean up subscriptions where this tool was a subscriber
        let mut subs = self.subscriptions.lock().unwrap();
        subs.retain(|_, subscribers| {
            subscribers.retain(|sub| sub != tool_id);
            !subscribers.is_empty()
        });

        // If this tool was a source being subscribed to, remove its subscriptions
        // This prevents new subscriptions to a dead tool
        subs.remove(tool_id);

        // Clean up associated proxy if it exists
        let mut proxies = self.proxies.lock().unwrap();
        if let Some(proxy_info) = proxies.remove(tool_id) {
            debug!("Cleaning up proxy for disconnected tool {}", tool_id);
            // Proxy process will detect upstream closure and exit on its own
            drop(proxy_info);
        }
    }

    /// Get connection info for a tool
    fn get_connection(&self, tool_id: &ToolId) -> Option<ConnectionInfo> {
        let conns = self.connections.lock().unwrap();
        conns.get(tool_id).cloned()
    }

    /// Get all connected tools (for observability)
    fn list_tools(&self) -> Vec<ToolInfo> {
        let conns = self.connections.lock().unwrap();
        conns
            .values()
            .map(|info| ToolInfo {
                tool_id: info.tool_id.clone(),
                capabilities: info.capabilities.clone(),
                metadata: info.metadata.clone(),
            })
            .collect()
    }

    /// Add a subscription: subscriber wants data from source
    fn add_subscription(&self, source: &ToolId, subscriber: ToolId) {
        let mut subs = self.subscriptions.lock().unwrap();
        subs.entry(source.clone()).or_default().push(subscriber);
    }

    /// Get subscriber count for a source
    fn subscriber_count(&self, source: &ToolId) -> usize {
        let subs = self.subscriptions.lock().unwrap();
        subs.get(source).map_or(0, std::vec::Vec::len)
    }

    /// Spawn a proxy subprocess for a source tool
    fn spawn_proxy(&self, source: &ToolId, upstream_address: &str) -> Result<String> {
        let pid = process::id();
        let proxy_seq = self.next_seq.fetch_add(1, Ordering::SeqCst);
        let proxy_listen = format!("unix:///tmp/gbe-proxy-{pid}-{proxy_seq:03}.sock");

        // Remove socket if it exists
        let proxy_socket_path = proxy_listen.strip_prefix("unix://").unwrap();
        let _ = std::fs::remove_file(proxy_socket_path);

        info!("Spawning proxy for {} at {}", source, proxy_socket_path);

        // Try to find gbe-proxy binary
        let proxy_bin = std::env::var("GBE_PROXY_BIN").unwrap_or_else(|_| {
            // Try relative path from router binary location
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("gbe-proxy")))
                .and_then(|p| p.to_str().map(String::from))
                .unwrap_or_else(|| "gbe-proxy".to_string())
        });

        let child = Command::new(&proxy_bin)
            .arg("--router")
            .arg("unix:///tmp/gbe-router.sock")
            .arg("--upstream")
            .arg(upstream_address)
            .arg("--listen")
            .arg(&proxy_listen) // Pass full unix:// address
            .arg("--mode")
            .arg("framed")
            .spawn()
            .context(format!("Failed to spawn gbe-proxy from {proxy_bin}"))?;

        info!("✓ Proxy spawned (PID: {})", child.id());

        // Wait for proxy socket to be created (with timeout)
        let socket_path = std::path::Path::new(proxy_socket_path);
        for _ in 0..50 {
            // 5 seconds max
            if socket_path.exists() {
                debug!("✓ Proxy socket ready");
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        if !socket_path.exists() {
            warn!("Proxy socket not created after 5s, may cause connection issues");
        }

        // Store proxy info
        let mut proxies = self.proxies.lock().unwrap();
        proxies.insert(
            source.clone(),
            ProxyInfo {
                child,
                listen_address: proxy_listen.clone(),
                source_tool_id: source.clone(),
            },
        );

        Ok(proxy_listen)
    }

    /// Get proxy address for a source (if proxy exists)
    fn get_proxy_address(&self, source: &ToolId) -> Option<String> {
        let proxies = self.proxies.lock().unwrap();
        proxies.get(source).map(|p| p.listen_address.clone())
    }

    /// Check if proxy exists for a source
    #[allow(dead_code)] // May be used for optimization logic in future
    fn has_proxy(&self, source: &ToolId) -> bool {
        let proxies = self.proxies.lock().unwrap();
        proxies.contains_key(source)
    }
}

fn main() -> Result<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Clean up old socket if it exists
    let _ = std::fs::remove_file(&args.socket);

    info!("Starting gbe-router v{}", env!("CARGO_PKG_VERSION"));
    info!("Listening on {}", args.socket);

    let listener = UnixListener::bind(&args.socket).context("Failed to bind Unix socket")?;

    let state = RouterState::new();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = state.clone();
                std::thread::spawn(move || {
                    if let Err(e) = handle_connection(stream, state) {
                        error!("Connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }

    Ok(())
}

/// Handle a single tool connection
#[allow(clippy::too_many_lines, clippy::needless_pass_by_value)] // moved into thread::spawn
fn handle_connection(stream: UnixStream, state: RouterState) -> Result<()> {
    let mut conn = Connection::new(stream);
    let mut tool_id: Option<ToolId> = None;

    loop {
        if let Some(msg) = conn.recv_message()? {
            debug!("Received: {:?}", msg);

            let response = match msg {
                ControlMessage::Connect {
                    capabilities,
                    metadata,
                } => {
                    let tid = state.assign_tool_id();
                    let data_addr = RouterState::assign_data_address(&tid);

                    state.register_connection(
                        tid.clone(),
                        data_addr.clone(),
                        capabilities,
                        metadata,
                    );

                    info!("Tool {} connected", tid);
                    tool_id = Some(tid.clone());

                    Some(ControlMessage::ConnectAck {
                        tool_id: tid,
                        data_listen_address: data_addr,
                    })
                }

                ControlMessage::Subscribe { target } => {
                    let subscriber = tool_id.as_ref().context("Subscribe without Connect")?;

                    if let Some(info) = state.get_connection(&target) {
                        // Add subscription first
                        state.add_subscription(&target, subscriber.clone());
                        let sub_count = state.subscriber_count(&target);

                        info!(
                            "Tool {} subscribed to {} (total subscribers: {})",
                            subscriber, target, sub_count
                        );

                        // Always use proxy for consistency (Phase 1 simplification)
                        // This ensures all subscribers can receive data reliably
                        let data_address =
                            if let Some(proxy_addr) = state.get_proxy_address(&target) {
                                info!("Using existing proxy at {}", proxy_addr);
                                proxy_addr
                            } else {
                                info!("Spawning proxy for tool {}", target);
                                match state.spawn_proxy(&target, &info.data_listen_address) {
                                    Ok(proxy_addr) => {
                                        info!("✓ Proxy ready at {}", proxy_addr);
                                        proxy_addr
                                    }
                                    Err(e) => {
                                        error!("Failed to spawn proxy: {}", e);
                                        // Fall back to direct address
                                        warn!("Falling back to direct connection");
                                        info.data_listen_address.clone()
                                    }
                                }
                            };

                        Some(ControlMessage::SubscribeAck {
                            data_connect_address: data_address,
                            capabilities: info.capabilities,
                        })
                    } else {
                        warn!("Subscribe to unknown tool: {}", target);
                        Some(ControlMessage::Error {
                            code: "NOT_FOUND".to_string(),
                            message: format!("Tool {target} not found"),
                        })
                    }
                }

                ControlMessage::Unsubscribe { target } => {
                    let subscriber = tool_id.as_ref().context("Unsubscribe without Connect")?;

                    info!("Tool {} unsubscribed from {}", subscriber, target);

                    // TODO: implement unsubscribe tracking
                    None
                }

                ControlMessage::QueryCapabilities { target } => {
                    match state.get_connection(&target) {
                        Some(info) => Some(ControlMessage::CapabilitiesResponse {
                            capabilities: info.capabilities,
                        }),
                        None => Some(ControlMessage::Error {
                            code: "NOT_FOUND".to_string(),
                            message: format!("Tool {target} not found"),
                        }),
                    }
                }

                ControlMessage::QueryTools => {
                    let tools = state.list_tools();
                    info!("Query tools: {} connected", tools.len());
                    Some(ControlMessage::ToolsResponse { tools })
                }

                ControlMessage::Disconnect => {
                    if let Some(tid) = &tool_id {
                        info!("Tool {} disconnected", tid);
                        state.unregister_connection(tid);
                    }
                    break;
                }

                // These messages are sent by router, not received
                ControlMessage::ConnectAck { .. }
                | ControlMessage::SubscribeAck { .. }
                | ControlMessage::CapabilitiesResponse { .. }
                | ControlMessage::ToolsResponse { .. }
                | ControlMessage::Error { .. }
                | ControlMessage::FlowControl { .. } => {
                    warn!("Received unexpected message type: {:?}", msg);
                    None
                }
            };

            if let Some(resp) = response {
                conn.send_message(&resp)?;
            }
        } else {
            // Connection closed
            if let Some(tid) = tool_id {
                info!("Tool {} connection closed", tid);
                state.unregister_connection(&tid);
            }
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id_format() {
        let state = RouterState::new();
        let id1 = state.assign_tool_id();
        let id2 = state.assign_tool_id();

        // Should be "PID-SEQ" format
        assert!(id1.contains('-'));
        assert!(id2.contains('-'));

        // Should be different
        assert_ne!(id1, id2);

        // Should have same PID
        let pid1: Vec<&str> = id1.split('-').collect();
        let pid2: Vec<&str> = id2.split('-').collect();
        assert_eq!(pid1[0], pid2[0]);

        // Should have sequential numbers
        let seq1: u64 = pid1[1].parse().unwrap();
        let seq2: u64 = pid2[1].parse().unwrap();
        assert_eq!(seq2, seq1 + 1);
    }

    #[test]
    fn test_data_address_format() {
        let id = "12345-001";
        let addr = RouterState::assign_data_address(id);

        assert_eq!(addr, "unix:///tmp/gbe-12345-001.sock");
    }

    #[test]
    fn test_connection_registration() {
        let state = RouterState::new();
        let tool_id = "12345-001".to_string();
        let data_addr = "unix:///tmp/gbe-12345-001.sock".to_string();
        let caps = vec!["pty".to_string()];

        // Register connection
        state.register_connection(
            tool_id.clone(),
            data_addr.clone(),
            caps.clone(),
            Default::default(),
        );

        // Should be retrievable
        let info = state.get_connection(&tool_id).unwrap();
        assert_eq!(info.data_listen_address, data_addr);
        assert_eq!(info.capabilities, caps);

        // Unregister
        state.unregister_connection(&tool_id);

        // Should be gone
        assert!(state.get_connection(&tool_id).is_none());
    }

    #[test]
    fn test_subscriptions() {
        let state = RouterState::new();
        let source = "12345-001".to_string();
        let sub1 = "12345-002".to_string();
        let sub2 = "12345-003".to_string();

        // Add subscriptions
        state.add_subscription(&source, sub1.clone());
        state.add_subscription(&source, sub2.clone());

        // Should have 2 subscribers
        assert_eq!(state.subscriber_count(&source), 2);

        // Unregister subscriber
        state.unregister_connection(&sub1);

        // Should have 1 subscriber
        assert_eq!(state.subscriber_count(&source), 1);
    }

    #[test]
    fn test_get_connection_missing() {
        let state = RouterState::new();
        assert!(state.get_connection(&"nonexistent".to_string()).is_none());
    }

    #[test]
    fn test_list_tools_empty() {
        let state = RouterState::new();
        assert!(state.list_tools().is_empty());
    }

    #[test]
    fn test_list_tools_multiple() {
        let state = RouterState::new();
        state.register_connection(
            "t1".into(),
            "addr1".into(),
            vec!["cap1".into()],
            Default::default(),
        );
        state.register_connection(
            "t2".into(),
            "addr2".into(),
            vec!["cap2".into()],
            Default::default(),
        );
        let tools = state.list_tools();
        assert_eq!(tools.len(), 2);
        let ids: Vec<&str> = tools.iter().map(|t| t.tool_id.as_str()).collect();
        assert!(ids.contains(&"t1"));
        assert!(ids.contains(&"t2"));
    }

    #[test]
    fn test_proxy_address_missing() {
        let state = RouterState::new();
        assert!(state.get_proxy_address(&"none".to_string()).is_none());
    }

    #[test]
    fn test_has_proxy_false() {
        let state = RouterState::new();
        assert!(!state.has_proxy(&"none".to_string()));
    }

    #[test]
    fn test_subscriber_count_zero() {
        let state = RouterState::new();
        assert_eq!(state.subscriber_count(&"no-source".to_string()), 0);
    }

    #[test]
    fn test_unregister_cleans_source_subscriptions() {
        let state = RouterState::new();
        let source = "src".to_string();
        let sub = "sub".to_string();
        state.register_connection(source.clone(), "addr".into(), vec![], Default::default());
        state.add_subscription(&source, sub.clone());
        assert_eq!(state.subscriber_count(&source), 1);

        // Unregister the source tool — its subscription list should be removed
        state.unregister_connection(&source);
        assert_eq!(state.subscriber_count(&source), 0);
    }

    #[test]
    fn test_unregister_removes_from_all_subscription_lists() {
        let state = RouterState::new();
        let src1 = "src1".to_string();
        let src2 = "src2".to_string();
        let sub = "sub".to_string();

        state.add_subscription(&src1, sub.clone());
        state.add_subscription(&src2, sub.clone());
        assert_eq!(state.subscriber_count(&src1), 1);
        assert_eq!(state.subscriber_count(&src2), 1);

        state.unregister_connection(&sub);
        assert_eq!(state.subscriber_count(&src1), 0);
        assert_eq!(state.subscriber_count(&src2), 0);
    }

    #[test]
    fn test_sequential_tool_ids() {
        let state = RouterState::new();
        let ids: Vec<ToolId> = (0..5).map(|_| state.assign_tool_id()).collect();
        // All unique
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), 5);
    }
}
