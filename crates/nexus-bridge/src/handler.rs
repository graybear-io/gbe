use async_trait::async_trait;
use gbe_nexus::{Message, MessageHandler, PublishOpts, Transport, TransportError};
use std::sync::Arc;

/// Forwards messages from the source transport to one or more sink transports.
pub struct BridgeHandler {
    sinks: Vec<Arc<dyn Transport>>,
    target_subject: Option<String>,
    cursor: Arc<tokio::sync::Mutex<Option<String>>>,
}

impl BridgeHandler {
    pub fn new(
        sinks: Vec<Arc<dyn Transport>>,
        target_subject: Option<String>,
        cursor: Arc<tokio::sync::Mutex<Option<String>>>,
    ) -> Self {
        Self {
            sinks,
            target_subject,
            cursor,
        }
    }
}

#[async_trait]
impl MessageHandler for BridgeHandler {
    async fn handle(&self, msg: &dyn Message) -> Result<(), TransportError> {
        let envelope = msg.envelope();
        let subject = self.target_subject.as_deref().unwrap_or(&envelope.subject);
        let opts = PublishOpts {
            trace_id: envelope.trace_id.clone(),
            idempotency_key: Some(envelope.message_id.clone()),
        };

        for sink in &self.sinks {
            sink.publish(subject, msg.payload().clone(), Some(opts.clone()))
                .await?;
        }

        msg.ack().await?;

        let mut cursor = self.cursor.lock().await;
        *cursor = Some(envelope.message_id.clone());

        Ok(())
    }
}
