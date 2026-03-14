use bytes::Bytes;
use gbe_nexus::{StreamConfig, Transport};
use gbe_nexus_memory::{MemoryTransport, MemoryTransportConfig};
use std::sync::Arc;
use std::time::Duration;

use crate::{BridgeConfig, NexusBridge};

fn memory_transport() -> Arc<MemoryTransport> {
    Arc::new(MemoryTransport::new(MemoryTransportConfig::default()))
}

#[tokio::test]
async fn bridge_forwards_messages() {
    let source = memory_transport();
    let sink = memory_transport();

    let subject = "gbe.test.bridge";

    source
        .ensure_stream(StreamConfig {
            subject: subject.into(),
            max_age: Duration::from_secs(3600),
            max_bytes: None,
            max_msgs: None,
        })
        .await
        .unwrap();

    sink.ensure_stream(StreamConfig {
        subject: subject.into(),
        max_age: Duration::from_secs(3600),
        max_bytes: None,
        max_msgs: None,
    })
    .await
    .unwrap();

    let mut bridge = NexusBridge::new(source.clone(), vec![sink.clone()], BridgeConfig::default());
    bridge.route(subject);
    bridge.start().await.unwrap();

    // Publish to source
    source
        .publish(subject, Bytes::from(r#"{"hello":"world"}"#), None)
        .await
        .unwrap();

    // Give bridge consumer loop time to forward
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify cursor advanced
    let cursors = bridge.cursors().await;
    assert_eq!(cursors.len(), 1);
    assert!(
        cursors[0].1.is_some(),
        "cursor should have advanced after forwarding"
    );

    bridge.stop().await.unwrap();
}

#[tokio::test]
async fn bridge_tracks_cursor_per_route() {
    let source = memory_transport();
    let sink = memory_transport();

    let subject_a = "gbe.test.a";
    let subject_b = "gbe.test.b";

    for subject in [subject_a, subject_b] {
        source
            .ensure_stream(StreamConfig {
                subject: subject.into(),
                max_age: Duration::from_secs(3600),
                max_bytes: None,
                max_msgs: None,
            })
            .await
            .unwrap();
        sink.ensure_stream(StreamConfig {
            subject: subject.into(),
            max_age: Duration::from_secs(3600),
            max_bytes: None,
            max_msgs: None,
        })
        .await
        .unwrap();
    }

    let mut bridge = NexusBridge::new(source.clone(), vec![sink.clone()], BridgeConfig::default());
    bridge.route(subject_a).route(subject_b);
    bridge.start().await.unwrap();

    // Publish only to subject A
    source
        .publish(subject_a, Bytes::from("a1"), None)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let cursors = bridge.cursors().await;
    assert!(cursors[0].1.is_some(), "subject A cursor should advance");
    assert!(cursors[1].1.is_none(), "subject B cursor should be None");

    // Now publish to B
    source
        .publish(subject_b, Bytes::from("b1"), None)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let cursors = bridge.cursors().await;
    assert!(cursors[0].1.is_some());
    assert!(
        cursors[1].1.is_some(),
        "subject B cursor should now advance"
    );

    bridge.stop().await.unwrap();
}

#[tokio::test]
async fn bridge_multiple_messages() {
    let source = memory_transport();
    let sink = memory_transport();

    let subject = "gbe.test.multi";

    source
        .ensure_stream(StreamConfig {
            subject: subject.into(),
            max_age: Duration::from_secs(3600),
            max_bytes: None,
            max_msgs: None,
        })
        .await
        .unwrap();
    sink.ensure_stream(StreamConfig {
        subject: subject.into(),
        max_age: Duration::from_secs(3600),
        max_bytes: None,
        max_msgs: None,
    })
    .await
    .unwrap();

    let mut bridge = NexusBridge::new(source.clone(), vec![sink.clone()], BridgeConfig::default());
    bridge.route(subject);
    bridge.start().await.unwrap();

    // Publish 5 messages
    let mut ids = Vec::new();
    for i in 0..5 {
        let id = source
            .publish(subject, Bytes::from(format!("msg-{i}")), None)
            .await
            .unwrap();
        ids.push(id);
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Cursor should be at the last message
    let cursors = bridge.cursors().await;
    assert_eq!(cursors[0].1.as_deref(), Some(ids[4].as_str()));

    bridge.stop().await.unwrap();
}
