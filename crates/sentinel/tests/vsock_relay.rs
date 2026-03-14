use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use gbe_nexus::{
    Message, MessageHandler, StartPosition, StreamConfig, SubscribeOpts, Transport, TransportError,
};
use gbe_nexus_memory::{MemoryTransport, MemoryTransportConfig};
use gbe_sentinel::vsock::listener::{VsockBackend, VsockListener};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio_util::sync::CancellationToken;

/// Collects payloads from a subscription.
struct Collector {
    received: Arc<tokio::sync::Mutex<Vec<Bytes>>>,
}

#[async_trait]
impl MessageHandler for Collector {
    async fn handle(&self, msg: &dyn Message) -> Result<(), TransportError> {
        self.received.lock().await.push(msg.payload().clone());
        msg.ack().await?;
        Ok(())
    }
}

async fn setup_listener(
    tmp: &tempfile::TempDir,
) -> (Arc<MemoryTransport>, VsockListener, std::path::PathBuf) {
    let edge = Arc::new(MemoryTransport::new(MemoryTransportConfig::default()));
    let sock_path = tmp.path().join("vsock.sock");
    let listener = VsockListener::new(
        VsockBackend::Unix {
            path: sock_path.clone(),
        },
        edge.clone(),
        "test-host".into(),
    );
    (edge, listener, sock_path)
}

async fn ensure_streams(transport: &MemoryTransport, subjects: &[&str]) {
    for subject in subjects {
        transport
            .ensure_stream(StreamConfig {
                subject: (*subject).into(),
                max_age: Duration::from_secs(3600),
                max_bytes: None,
                max_msgs: None,
            })
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn progress_message_relayed_to_edge() {
    let tmp = tempfile::tempdir().unwrap();
    let (edge, listener, sock_path) = setup_listener(&tmp).await;

    let progress_subject = "gbe.tasks.shell.progress.task-1";
    ensure_streams(&edge, &[progress_subject]).await;

    let received = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let _sub = edge
        .subscribe(
            progress_subject,
            "test",
            Box::new(Collector {
                received: received.clone(),
            }),
            Some(SubscribeOpts {
                start_from: StartPosition::Earliest,
                ..Default::default()
            }),
        )
        .await
        .unwrap();

    let token = CancellationToken::new();
    let run_token = token.clone();
    let handle = tokio::spawn(async move { listener.run(run_token).await });

    // Wait for listener to bind
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect and send a progress message
    let mut stream = UnixStream::connect(&sock_path).await.unwrap();
    let msg = r#"{"type":"progress","id":"task-1","step":"compile","status":"running"}"#;
    stream
        .write_all(format!("{msg}\n").as_bytes())
        .await
        .unwrap();
    stream.flush().await.unwrap();

    // Wait for relay
    tokio::time::sleep(Duration::from_millis(200)).await;

    let msgs = received.lock().await;
    assert!(!msgs.is_empty(), "progress should be relayed to edge");

    let payload = String::from_utf8_lossy(&msgs[0]);
    assert!(payload.contains("compile"));

    token.cancel();
    handle.await.unwrap().unwrap();
}

#[tokio::test]
async fn result_message_relayed_and_acked() {
    let tmp = tempfile::tempdir().unwrap();
    let (edge, listener, sock_path) = setup_listener(&tmp).await;

    let terminal_subject = "gbe.tasks.shell.terminal.task-2";
    ensure_streams(&edge, &[terminal_subject]).await;

    let received = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let _sub = edge
        .subscribe(
            terminal_subject,
            "test",
            Box::new(Collector {
                received: received.clone(),
            }),
            Some(SubscribeOpts {
                start_from: StartPosition::Earliest,
                ..Default::default()
            }),
        )
        .await
        .unwrap();

    let token = CancellationToken::new();
    let run_token = token.clone();
    let handle = tokio::spawn(async move { listener.run(run_token).await });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let stream = UnixStream::connect(&sock_path).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    // Send Result message
    let msg = r#"{"type":"result","id":"task-2","output":{"data":"done"},"exit_code":0}"#;
    writer
        .write_all(format!("{msg}\n").as_bytes())
        .await
        .unwrap();
    writer.flush().await.unwrap();

    // Read ack
    let ack_line = tokio::time::timeout(Duration::from_secs(2), lines.next_line())
        .await
        .expect("ack timeout")
        .unwrap()
        .expect("connection closed before ack");

    assert!(
        ack_line.contains("\"type\":\"ack\""),
        "expected ack, got: {ack_line}"
    );
    assert!(ack_line.contains("task-2"));

    // Verify event on edge
    tokio::time::sleep(Duration::from_millis(100)).await;
    let msgs = received.lock().await;
    assert!(!msgs.is_empty(), "result should be relayed to edge");

    token.cancel();
    handle.await.unwrap().unwrap();
}

#[tokio::test]
async fn error_message_relayed_and_acked() {
    let tmp = tempfile::tempdir().unwrap();
    let (edge, listener, sock_path) = setup_listener(&tmp).await;

    let terminal_subject = "gbe.tasks.shell.terminal.task-3";
    ensure_streams(&edge, &[terminal_subject]).await;

    let token = CancellationToken::new();
    let run_token = token.clone();
    let handle = tokio::spawn(async move { listener.run(run_token).await });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let stream = UnixStream::connect(&sock_path).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    let msg = r#"{"type":"error","id":"task-3","error":"segfault","exit_code":139}"#;
    writer
        .write_all(format!("{msg}\n").as_bytes())
        .await
        .unwrap();
    writer.flush().await.unwrap();

    let ack_line = tokio::time::timeout(Duration::from_secs(2), lines.next_line())
        .await
        .expect("ack timeout")
        .unwrap()
        .expect("connection closed before ack");

    assert!(ack_line.contains("\"type\":\"ack\""));
    assert!(ack_line.contains("task-3"));

    token.cancel();
    handle.await.unwrap().unwrap();
}

#[tokio::test]
async fn progress_not_acked() {
    let tmp = tempfile::tempdir().unwrap();
    let (edge, listener, sock_path) = setup_listener(&tmp).await;

    ensure_streams(&edge, &["gbe.tasks.shell.progress.task-4"]).await;

    let token = CancellationToken::new();
    let run_token = token.clone();
    let handle = tokio::spawn(async move { listener.run(run_token).await });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let stream = UnixStream::connect(&sock_path).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    // Send progress (should not get ack)
    let msg = r#"{"type":"progress","id":"task-4","step":"s","status":"ok"}"#;
    writer
        .write_all(format!("{msg}\n").as_bytes())
        .await
        .unwrap();
    writer.flush().await.unwrap();

    // Try to read — should timeout (no ack for progress)
    let result = tokio::time::timeout(Duration::from_millis(300), lines.next_line()).await;
    assert!(result.is_err(), "progress should not receive ack");

    token.cancel();
    handle.await.unwrap().unwrap();
}
