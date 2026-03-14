use gbe_nexus::{StartPosition, SubscribeOpts, Transport};
use std::sync::Arc;
use std::time::Duration;

use crate::handler::BridgeHandler;
use crate::route::Route;

/// Configuration for a `NexusBridge`.
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// Consumer group name used on the source transport.
    pub group: String,
    /// Subscribe options (batch size, ack timeout, etc).
    pub subscribe_opts: SubscribeOpts,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            group: "bridge".into(),
            subscribe_opts: SubscribeOpts {
                batch_size: 1,
                max_inflight: 10,
                ack_timeout: Duration::from_secs(30),
                start_from: StartPosition::Earliest,
            },
        }
    }
}

/// Bridges messages from a source transport to one or more sink transports.
///
/// Each route maps a subject on the source to the same (or different) subject
/// on the sinks. The bridge tracks a cursor per route so it can resume from
/// the last forwarded message after a restart (when backed by a durable source).
pub struct NexusBridge {
    source: Arc<dyn Transport>,
    sinks: Vec<Arc<dyn Transport>>,
    config: BridgeConfig,
    routes: Vec<Route>,
}

impl NexusBridge {
    pub fn new(
        source: Arc<dyn Transport>,
        sinks: Vec<Arc<dyn Transport>>,
        config: BridgeConfig,
    ) -> Self {
        Self {
            source,
            sinks,
            config,
            routes: Vec::new(),
        }
    }

    /// Add a route: forward `subject` from source to all sinks (same subject name).
    pub fn route(&mut self, subject: impl Into<String>) -> &mut Self {
        self.routes.push(Route::new(subject));
        self
    }

    /// Start all routes. Subscribes on the source transport for each route.
    pub async fn start(&mut self) -> Result<(), gbe_nexus::TransportError> {
        for route in &mut self.routes {
            let handler = BridgeHandler::new(
                self.sinks.clone(),
                None, // same subject
                route.last_message_id.clone(),
            );

            let subscription = self
                .source
                .subscribe(
                    &route.subject,
                    &self.config.group,
                    Box::new(handler),
                    Some(self.config.subscribe_opts.clone()),
                )
                .await?;

            route.subscription = Some(subscription);
        }

        Ok(())
    }

    /// Stop all routes.
    pub async fn stop(&mut self) -> Result<(), gbe_nexus::TransportError> {
        for route in &mut self.routes {
            if let Some(sub) = route.subscription.take() {
                sub.unsubscribe().await?;
            }
        }
        Ok(())
    }

    /// Returns a snapshot of all route cursors (subject → last message ID).
    pub async fn cursors(&self) -> Vec<(String, Option<String>)> {
        let mut result = Vec::with_capacity(self.routes.len());
        for route in &self.routes {
            let cursor = route.cursor().await;
            result.push((route.subject.clone(), cursor));
        }
        result
    }
}
