use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use gbe_nexus::{
    Message, MessageHandler, StartPosition, StreamConfig, SubscribeOpts, Transport, TransportError,
};
use gbe_nexus_memory::{MemoryTransport, MemoryTransportConfig};
use gbe_sentinel::{Sentinel, SentinelConfig};
use gbe_state_store::{Record, ScanFilter, StateStore, StateStoreError};
use tokio_util::sync::CancellationToken;

/// In-memory StateStore that always succeeds CAS claims.
struct MockStore;

#[async_trait]
impl StateStore for MockStore {
    async fn get(&self, _key: &str) -> Result<Option<Record>, StateStoreError> {
        Ok(None)
    }
    async fn put(
        &self,
        _key: &str,
        _record: Record,
        _ttl: Option<Duration>,
    ) -> Result<(), StateStoreError> {
        Ok(())
    }
    async fn delete(&self, _key: &str) -> Result<(), StateStoreError> {
        Ok(())
    }
    async fn get_field(&self, _key: &str, _field: &str) -> Result<Option<Bytes>, StateStoreError> {
        Ok(None)
    }
    async fn set_field(
        &self,
        _key: &str,
        _field: &str,
        _value: Bytes,
    ) -> Result<(), StateStoreError> {
        Ok(())
    }
    async fn set_fields(
        &self,
        _key: &str,
        _fields: HashMap<String, Bytes>,
    ) -> Result<(), StateStoreError> {
        Ok(())
    }
    async fn compare_and_swap(
        &self,
        _key: &str,
        _field: &str,
        _expected: Bytes,
        _new: Bytes,
    ) -> Result<bool, StateStoreError> {
        Ok(true) // always succeed
    }
    async fn scan(
        &self,
        _prefix: &str,
        _filter: Option<ScanFilter>,
    ) -> Result<Vec<(String, Record)>, StateStoreError> {
        Ok(vec![])
    }
    async fn ping(&self) -> Result<bool, StateStoreError> {
        Ok(true)
    }
    async fn close(&self) -> Result<(), StateStoreError> {
        Ok(())
    }
}

fn test_config(tmp: &std::path::Path) -> SentinelConfig {
    use std::fs;
    let image_dir = tmp.join("images");
    let overlay_dir = tmp.join("overlays");
    fs::create_dir_all(&image_dir).unwrap();
    fs::create_dir_all(&overlay_dir).unwrap();
    let kernel = tmp.join("vmlinux");
    let fc_bin = tmp.join("firecracker");
    fs::write(&kernel, b"").unwrap();
    fs::write(&fc_bin, b"").unwrap();

    SentinelConfig {
        host_id: "test-host".into(),
        slots: 4,
        image_dir,
        kernel_path: kernel,
        overlay_dir,
        firecracker_bin: fc_bin,
        profiles: HashMap::new(),
        task_types: vec!["shell".into()],
        heartbeat_interval_secs: 1,
    }
}

/// Collects messages received on a subscription into a shared vec.
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

#[tokio::test]
async fn sentinel_lifecycle_events_bridge_to_core() {
    let tmp = tempfile::tempdir().unwrap();
    let config = test_config(tmp.path());

    let core = Arc::new(MemoryTransport::new(MemoryTransportConfig::default()));
    let edge = Arc::new(MemoryTransport::new(MemoryTransportConfig::default()));
    let store: Arc<dyn StateStore> = Arc::new(MockStore);

    // Set up a collector on core transport to catch bridged lifecycle events
    let lifecycle_subject = "gbe.sentinel.test-host.lifecycle";
    core.ensure_stream(StreamConfig {
        subject: lifecycle_subject.into(),
        max_age: Duration::from_secs(3600),
        max_bytes: None,
        max_msgs: None,
    })
    .await
    .unwrap();

    let received = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let collector = Collector {
        received: received.clone(),
    };
    let _sub = core
        .subscribe(
            lifecycle_subject,
            "test-collector",
            Box::new(collector),
            Some(SubscribeOpts {
                start_from: StartPosition::Earliest,
                ..Default::default()
            }),
        )
        .await
        .unwrap();

    // Create sentinel with bridge
    let mut sentinel = Sentinel::new(config, core.clone(), edge.clone(), store)
        .await
        .unwrap();
    sentinel.with_bridge(&[lifecycle_subject]);

    // Run sentinel in background, cancel after a short delay
    let token = CancellationToken::new();
    let run_token = token.clone();
    let handle = tokio::spawn(async move { sentinel.run(run_token).await });

    // Let it run long enough for startup + at least one beacon
    tokio::time::sleep(Duration::from_millis(500)).await;
    token.cancel();

    let result = handle.await.unwrap();
    assert!(result.is_ok(), "sentinel run failed: {result:?}");

    // Check that lifecycle events arrived on core transport
    tokio::time::sleep(Duration::from_millis(100)).await;
    let msgs = received.lock().await;
    assert!(
        !msgs.is_empty(),
        "expected lifecycle events bridged to core, got none"
    );

    // First event should be sentinel_started
    let first = String::from_utf8_lossy(&msgs[0]);
    assert!(
        first.contains("started") || first.contains("sentinel"),
        "first lifecycle event should be sentinel started, got: {first}"
    );
}

#[tokio::test]
async fn sentinel_task_claim_emits_to_edge() {
    let tmp = tempfile::tempdir().unwrap();
    let config = test_config(tmp.path());

    let core = Arc::new(MemoryTransport::new(MemoryTransportConfig::default()));
    let edge = Arc::new(MemoryTransport::new(MemoryTransportConfig::default()));
    let store: Arc<dyn StateStore> = Arc::new(MockStore);

    let queue_subject = "gbe.tasks.shell.queue";
    let lifecycle_subject = "gbe.sentinel.test-host.lifecycle";

    // Ensure queue exists on core
    core.ensure_stream(StreamConfig {
        subject: queue_subject.into(),
        max_age: Duration::from_secs(3600),
        max_bytes: None,
        max_msgs: None,
    })
    .await
    .unwrap();

    // Collect lifecycle events on edge
    edge.ensure_stream(StreamConfig {
        subject: lifecycle_subject.into(),
        max_age: Duration::from_secs(3600),
        max_bytes: None,
        max_msgs: None,
    })
    .await
    .unwrap();

    let received = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let collector = Collector {
        received: received.clone(),
    };
    let _sub = edge
        .subscribe(
            lifecycle_subject,
            "test-lifecycle",
            Box::new(collector),
            Some(SubscribeOpts {
                start_from: StartPosition::Earliest,
                ..Default::default()
            }),
        )
        .await
        .unwrap();

    let mut sentinel = Sentinel::new(config, core.clone(), edge.clone(), store)
        .await
        .unwrap();

    let token = CancellationToken::new();
    let run_token = token.clone();
    let handle = tokio::spawn(async move { sentinel.run(run_token).await });

    // Wait for sentinel to subscribe
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Publish a task to the queue on core
    let task_payload = serde_json::json!({
        "task_id": "task-001",
        "state_key": "job:1:task:a",
        "task_type": "shell",
        "timeout_sec": 300
    });
    core.publish(
        queue_subject,
        Bytes::from(serde_json::to_vec(&task_payload).unwrap()),
        None,
    )
    .await
    .unwrap();

    // Wait for claim + events
    tokio::time::sleep(Duration::from_millis(500)).await;
    token.cancel();
    handle.await.unwrap().unwrap();

    // Check lifecycle events on edge
    tokio::time::sleep(Duration::from_millis(100)).await;
    let msgs = received.lock().await;

    // Should have: sentinel_started, task_claimed, task_started (at minimum)
    assert!(
        msgs.len() >= 2,
        "expected at least sentinel_started + task_claimed, got {} events",
        msgs.len()
    );
}
