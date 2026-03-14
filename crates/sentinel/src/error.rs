use std::io;

/// Sentinel error type.
///
/// **Convention**: Use `Display` (`{}`) for external-facing messages (bus
/// publications, API responses). Use `Debug` (`{:?}`) only in internal
/// logs at debug/trace level. `Display` output (from thiserror `#[error]`)
/// is designed to be safe for external consumption. `Debug` output may
/// include filesystem paths and internal identifiers.
#[derive(Debug, thiserror::Error)]
pub enum SentinelError {
    #[error("transport error: {0}")]
    Transport(#[from] gbe_nexus::TransportError),

    #[error("state store error: {0}")]
    StateStore(#[from] gbe_state_store::StateStoreError),

    #[error("vm error: {0}")]
    Vm(String),

    #[error("vsock error: {0}")]
    Vsock(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("claim failed for task {task_id}: {reason}")]
    ClaimFailed { task_id: String, reason: String },

    #[error("timeout: task {0} exceeded deadline")]
    Timeout(String),

    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
