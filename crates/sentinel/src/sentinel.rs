use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use gbe_nexus::{EventEmitter, StreamConfig, Transport, dedup_id};
use gbe_nexus_bridge::{BridgeConfig, NexusBridge};
use gbe_state_store::StateStore;
use serde::Serialize;
use tokio_util::sync::CancellationToken;

use gbe_nexus::SubscribeOpts;
use gbe_nexus::writ::WritDispatcher;

use crate::config::SentinelConfig;
use crate::error::SentinelError;
use crate::handler::TaskHandler;
use crate::health::HealthPublisher;
use crate::vsock::listener::{VsockBackend, VsockListener};
use crate::writ_handler::SentinelCapabilities;

/// Tracks VM slot usage with atomic operations. Safe to share across
/// concurrent task handlers without external locking.
pub struct SlotTracker {
    total: u32,
    used: AtomicU32,
}

impl SlotTracker {
    #[must_use]
    pub fn new(total: u32) -> Self {
        Self {
            total,
            used: AtomicU32::new(0),
        }
    }

    pub fn total(&self) -> u32 {
        self.total
    }

    pub fn available(&self) -> u32 {
        self.total.saturating_sub(self.used.load(Ordering::Acquire))
    }

    /// Try to claim a slot. Returns true if successful.
    pub fn try_claim(&self) -> bool {
        self.used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                if current < self.total {
                    Some(current + 1)
                } else {
                    None
                }
            })
            .is_ok()
    }

    /// Release a previously claimed slot.
    pub fn release(&self) {
        self.used.fetch_sub(1, Ordering::Release);
    }
}

/// Lifecycle events emitted by sentinel itself.
#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum SentinelLifecycle {
    Started { host_id: String, slots: u32 },
    Stopping { host_id: String },
}

pub struct Sentinel {
    pub(crate) config: SentinelConfig,
    /// Core transport (Redis/NATS) — connects to the system-wide bus.
    pub(crate) core_transport: Arc<dyn Transport>,
    /// Edge transport (in-process) — operatives publish here via vsock relay.
    /// Sentinel's own lifecycle events also publish here. The bridge forwards
    /// selected subjects from edge → core.
    pub(crate) edge_transport: Arc<dyn Transport>,
    pub(crate) store: Arc<dyn StateStore>,
    pub(crate) slots: Arc<SlotTracker>,
    pub(crate) bridge: Option<NexusBridge>,
    pub(crate) vsock_backend: Option<VsockBackend>,
}

impl Sentinel {
    /// # Errors
    ///
    /// Returns `SentinelError::Config` if config validation fails.
    pub async fn new(
        config: SentinelConfig,
        core_transport: Arc<dyn Transport>,
        edge_transport: Arc<dyn Transport>,
        store: Arc<dyn StateStore>,
    ) -> Result<Self, SentinelError> {
        config.validate()?;
        let slots = Arc::new(SlotTracker::new(config.slots));
        Ok(Self {
            config,
            core_transport,
            edge_transport,
            store,
            slots,
            bridge: None,
            vsock_backend: None,
        })
    }

    /// Configure the vsock backend for accepting operative connections.
    pub fn with_vsock(&mut self, backend: VsockBackend) -> &mut Self {
        self.vsock_backend = Some(backend);
        self
    }

    /// Configure the bridge with subjects to forward from edge → core.
    pub fn with_bridge(&mut self, subjects: &[&str]) -> &mut Self {
        let mut bridge = NexusBridge::new(
            self.edge_transport.clone(),
            vec![self.core_transport.clone()],
            BridgeConfig::default(),
        );
        for subject in subjects {
            bridge.route(*subject);
        }
        self.bridge = Some(bridge);
        self
    }

    /// Run the sentinel main loop.
    ///
    /// 1. Ensure streams exist on edge transport
    /// 2. Start bridge (edge → core forwarding)
    /// 3. Subscribe to task queues on core transport
    /// 4. Start health beacon on edge transport
    /// 5. Wait for cancellation
    /// 6. Graceful shutdown
    ///
    /// # Errors
    ///
    /// Returns `SentinelError` on transport or state store failures.
    pub async fn run(&mut self, token: CancellationToken) -> Result<(), SentinelError> {
        let host_id = &self.config.host_id;
        let lifecycle_subject = format!("gbe.sentinel.{host_id}.lifecycle");
        let health_subject = format!("gbe.sentinel.{host_id}.health");

        // Ensure edge streams exist
        for subject in [&lifecycle_subject, &health_subject] {
            self.edge_transport
                .ensure_stream(StreamConfig {
                    subject: subject.clone(),
                    max_age: Duration::from_secs(3600),
                    max_bytes: None,
                    max_msgs: None,
                })
                .await?;
        }

        // Start bridge
        if let Some(bridge) = &mut self.bridge {
            bridge.start().await.map_err(SentinelError::Transport)?;
        }

        // Emit sentinel started → edge transport (bridge forwards to core)
        let emitter = EventEmitter::new(
            self.edge_transport.clone(),
            gbe_nexus::NodeIdentity::new("sentinel", gbe_nexus::NodeKind::Service, "gbe", host_id),
        );
        emitter
            .emit(
                &lifecycle_subject,
                1,
                dedup_id("sentinel", host_id, "started"),
                SentinelLifecycle::Started {
                    host_id: host_id.clone(),
                    slots: self.config.slots,
                },
            )
            .await?;

        // Emit capabilities
        let task_type_strs: Vec<&str> = self.config.task_types.iter().map(|s| s.as_str()).collect();
        let geas = gbe_architect::sentinel(host_id, self.config.slots, &task_type_strs);
        let caps = gbe_architect::roles::rich_capabilities_for(&geas, emitter.identity().clone());
        if let Err(e) = emitter.emit_capabilities(&caps).await {
            tracing::warn!(%e, "failed to emit CapabilitySet");
        }

        tracing::info!(host_id, slots = self.config.slots, "sentinel started");

        // Subscribe to writs targeting sentinel and ensure response stream
        let writ_subject = gbe_jobs_domain::subjects::writs::role("sentinel");
        for subject in [writ_subject.as_str(), gbe_nexus::writ::RESPONSE_SUBJECT] {
            self.core_transport
                .ensure_stream(StreamConfig {
                    subject: subject.to_string(),
                    max_age: Duration::from_secs(86400),
                    max_bytes: None,
                    max_msgs: None,
                })
                .await?;
        }

        let capabilities = SentinelCapabilities::new(
            self.slots.clone(),
            emitter.identity().clone(),
        );
        let dispatcher = WritDispatcher::new(
            emitter.identity().clone(),
            self.core_transport.clone(),
            Box::new(capabilities),
        );
        let _writ_sub = self
            .core_transport
            .subscribe(
                &writ_subject,
                &format!("sentinel-{host_id}-writs"),
                Box::new(dispatcher),
                Some(SubscribeOpts {
                    batch_size: 1,
                    max_inflight: 10,
                    ack_timeout: Duration::from_secs(30),
                    start_from: gbe_nexus::StartPosition::Latest,
                }),
            )
            .await?;
        tracing::info!("subscribed to writ subject: {writ_subject}");

        // Subscribe to task queues on core transport
        let mut subscriptions = Vec::new();
        for task_type in &self.config.task_types {
            let queue_subject = format!("gbe.tasks.{task_type}.queue");

            // Ensure queue stream exists on core
            self.core_transport
                .ensure_stream(StreamConfig {
                    subject: queue_subject.clone(),
                    max_age: Duration::from_secs(86400),
                    max_bytes: None,
                    max_msgs: None,
                })
                .await?;

            // Ensure progress/terminal streams on edge
            let progress_subject = format!("gbe.tasks.{task_type}.progress");
            let terminal_subject = format!("gbe.tasks.{task_type}.terminal");
            for subject in [&progress_subject, &terminal_subject] {
                self.edge_transport
                    .ensure_stream(StreamConfig {
                        subject: subject.clone(),
                        max_age: Duration::from_secs(3600),
                        max_bytes: None,
                        max_msgs: None,
                    })
                    .await?;
            }

            let handler = TaskHandler::new(
                host_id.clone(),
                self.store.clone(),
                self.slots.clone(),
                self.edge_transport.clone(),
            );

            let sub = self
                .core_transport
                .subscribe(
                    &queue_subject,
                    &format!("sentinel-{host_id}"),
                    Box::new(handler),
                    None,
                )
                .await?;

            tracing::info!(task_type, "subscribed to task queue");
            subscriptions.push(sub);
        }

        // Start health beacon
        let health = HealthPublisher::new(
            self.edge_transport.clone(),
            host_id.clone(),
            self.slots.clone(),
            Duration::from_secs(self.config.heartbeat_interval_secs),
        );
        let health_token = token.child_token();
        let health_handle = tokio::spawn(async move {
            if let Err(e) = health.run(health_token).await {
                tracing::error!(%e, "health publisher failed");
            }
        });

        // Start vsock listener for VM connections
        let vsock_handle = if let Some(backend) = self.vsock_backend.take() {
            let listener =
                VsockListener::new(backend, self.edge_transport.clone(), host_id.clone());
            let vsock_token = token.child_token();
            Some(tokio::spawn(async move {
                if let Err(e) = listener.run(vsock_token).await {
                    tracing::error!(%e, "vsock listener failed");
                }
            }))
        } else {
            None
        };

        // Wait for cancellation
        token.cancelled().await;
        tracing::info!("sentinel shutting down");

        // Emit stopping event
        emitter
            .emit(
                &lifecycle_subject,
                1,
                dedup_id("sentinel", host_id, "stopping"),
                SentinelLifecycle::Stopping {
                    host_id: host_id.clone(),
                },
            )
            .await
            .ok(); // best-effort on shutdown

        // Unsubscribe from task queues
        for sub in subscriptions {
            sub.unsubscribe().await.ok();
        }

        // Stop health beacon and vsock listener
        health_handle.abort();
        if let Some(handle) = vsock_handle {
            handle.abort();
        }

        // Stop bridge
        if let Some(bridge) = &mut self.bridge {
            bridge.stop().await.map_err(SentinelError::Transport)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_has_full_capacity() {
        let t = SlotTracker::new(4);
        assert_eq!(t.available(), 4);
    }

    #[test]
    fn claim_reduces_available() {
        let t = SlotTracker::new(2);
        assert!(t.try_claim());
        assert_eq!(t.available(), 1);
    }

    #[test]
    fn claim_at_capacity_fails() {
        let t = SlotTracker::new(1);
        assert!(t.try_claim());
        assert!(!t.try_claim());
        assert_eq!(t.available(), 0);
    }

    #[test]
    fn release_restores_capacity() {
        let t = SlotTracker::new(1);
        assert!(t.try_claim());
        t.release();
        assert_eq!(t.available(), 1);
        assert!(t.try_claim());
    }

    #[test]
    fn zero_slots_never_claims() {
        let t = SlotTracker::new(0);
        assert_eq!(t.available(), 0);
        assert!(!t.try_claim());
    }

    #[test]
    fn concurrent_claims_respect_limit() {
        let tracker = Arc::new(SlotTracker::new(3));
        let mut handles = vec![];

        for _ in 0..10 {
            let t = Arc::clone(&tracker);
            handles.push(std::thread::spawn(move || t.try_claim()));
        }

        let successes: usize = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|&claimed| claimed)
            .count();
        assert_eq!(successes, 3);
        assert_eq!(tracker.available(), 0);
    }
}
