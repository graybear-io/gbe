use gbe_nexus::Subscription;
use std::sync::Arc;

/// A single subject route: source subject → sink transport(s).
pub struct Route {
    pub subject: String,
    pub(crate) subscription: Option<Box<dyn Subscription>>,
    pub(crate) last_message_id: Arc<tokio::sync::Mutex<Option<String>>>,
}

impl Route {
    pub fn new(subject: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            subscription: None,
            last_message_id: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Returns the last successfully forwarded message ID (cursor).
    pub async fn cursor(&self) -> Option<String> {
        self.last_message_id.lock().await.clone()
    }
}
