use std::path::PathBuf;
use std::sync::Arc;

use gbe_nexus::{EventEmitter, Transport, dedup_id};
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio_util::sync::CancellationToken;

use crate::error::SentinelError;
use crate::vsock::protocol::{OperativeMessage, SentinelMessage, parse_operative_message};

/// Backend for accepting connections. Unix sockets for macOS dev,
/// AF_VSOCK for production on Linux.
pub enum VsockBackend {
    Unix { path: PathBuf },
    // Future: Vsock { port: u32 },
}

/// Accepts connections from VMs and relays operative messages
/// to the edge transport.
pub struct VsockListener {
    backend: VsockBackend,
    edge_transport: Arc<dyn Transport>,
    host_id: String,
}

/// Events published to edge transport from vsock relay.
#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum RelayEvent {
    Progress {
        task_id: String,
        step: String,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    },
    Completed {
        task_id: String,
        output: serde_json::Value,
        exit_code: i32,
    },
    Failed {
        task_id: String,
        error: String,
        exit_code: i32,
    },
}

impl VsockListener {
    pub fn new(backend: VsockBackend, edge_transport: Arc<dyn Transport>, host_id: String) -> Self {
        Self {
            backend,
            edge_transport,
            host_id,
        }
    }

    /// Run the listener until cancelled.
    pub async fn run(&self, token: CancellationToken) -> Result<(), SentinelError> {
        match &self.backend {
            VsockBackend::Unix { path } => self.run_unix(path, token).await,
        }
    }

    async fn run_unix(
        &self,
        path: &PathBuf,
        token: CancellationToken,
    ) -> Result<(), SentinelError> {
        // Clean up stale socket
        let _ = tokio::fs::remove_file(path).await;

        let listener = UnixListener::bind(path)
            .map_err(|e| SentinelError::Vsock(format!("bind failed: {e}")))?;

        tracing::info!(path = %path.display(), "vsock listener started (unix backend)");

        loop {
            tokio::select! {
                () = token.cancelled() => {
                    tracing::info!("vsock listener shutting down");
                    return Ok(());
                }
                accept = listener.accept() => {
                    match accept {
                        Ok((stream, _addr)) => {
                            let transport = self.edge_transport.clone();
                            let host_id = self.host_id.clone();
                            let conn_token = token.child_token();
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, transport, &host_id, conn_token).await {
                                    tracing::warn!(%e, "vsock connection handler failed");
                                }
                            });
                        }
                        Err(e) => {
                            tracing::warn!(%e, "vsock accept failed");
                        }
                    }
                }
            }
        }
    }
}

async fn handle_connection(
    stream: UnixStream,
    edge_transport: Arc<dyn Transport>,
    host_id: &str,
    token: CancellationToken,
) -> Result<(), SentinelError> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    let emitter = EventEmitter::new(edge_transport, "sentinel", host_id);

    loop {
        tokio::select! {
            () = token.cancelled() => return Ok(()),
            line = lines.next_line() => {
                let line = match line {
                    Ok(Some(line)) => line,
                    Ok(None) => return Ok(()), // connection closed
                    Err(e) => {
                        tracing::debug!(%e, "vsock read error");
                        return Ok(());
                    }
                };

                let msg = parse_operative_message(line.as_bytes())?;
                let task_id = msg.task_id().to_string();

                match msg {
                    OperativeMessage::Progress { id, step, status, data } => {
                        let subject = format!("gbe.tasks.shell.progress.{id}");
                        let dedup = dedup_id("sentinel", host_id, &format!("progress-{id}"));
                        emitter
                            .emit(&subject, 1, dedup, RelayEvent::Progress {
                                task_id: id,
                                step,
                                status,
                                data,
                            })
                            .await
                            .ok(); // fire-and-forget for progress
                    }

                    OperativeMessage::Result { id, output, exit_code } => {
                        let subject = format!("gbe.tasks.shell.terminal.{id}");
                        let dedup = dedup_id("sentinel", host_id, &format!("result-{id}"));
                        emitter
                            .emit(&subject, 1, dedup, RelayEvent::Completed {
                                task_id: id.clone(),
                                output,
                                exit_code,
                            })
                            .await
                            .map_err(SentinelError::Transport)?;

                        // Ack terminal message
                        send_ack(&mut writer, &id).await?;
                    }

                    OperativeMessage::Error { id, error, exit_code } => {
                        let subject = format!("gbe.tasks.shell.terminal.{id}");
                        let dedup = dedup_id("sentinel", host_id, &format!("error-{id}"));
                        emitter
                            .emit(&subject, 1, dedup, RelayEvent::Failed {
                                task_id: id.clone(),
                                error,
                                exit_code,
                            })
                            .await
                            .map_err(SentinelError::Transport)?;

                        // Ack terminal message
                        send_ack(&mut writer, &id).await?;
                    }

                    OperativeMessage::ToolCall { id, call_id, .. } => {
                        // Stub: tool proxy not implemented yet
                        let error_result = SentinelMessage::ToolResult {
                            id,
                            call_id,
                            result: serde_json::json!({"error": "tool proxy not implemented"}),
                        };
                        let json = serde_json::to_string(&error_result)
                            .map_err(SentinelError::Json)?;
                        writer
                            .write_all(format!("{json}\n").as_bytes())
                            .await
                            .map_err(|e| SentinelError::Vsock(format!("write failed: {e}")))?;
                        writer
                            .flush()
                            .await
                            .map_err(|e| SentinelError::Vsock(format!("flush failed: {e}")))?;
                    }
                }

                let _ = task_id; // used for tracing in future
            }
        }
    }
}

async fn send_ack(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    task_id: &str,
) -> Result<(), SentinelError> {
    let ack = SentinelMessage::Ack {
        id: task_id.to_string(),
    };
    let json = serde_json::to_string(&ack).map_err(SentinelError::Json)?;
    writer
        .write_all(format!("{json}\n").as_bytes())
        .await
        .map_err(|e| SentinelError::Vsock(format!("ack write failed: {e}")))?;
    writer
        .flush()
        .await
        .map_err(|e| SentinelError::Vsock(format!("ack flush failed: {e}")))?;
    Ok(())
}
