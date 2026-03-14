use crate::error::SentinelError;

/// Accepts vsock connections from VMs, demultiplexes by CID.
///
/// Each VM connects on a unique CID. The listener routes incoming
/// messages to the appropriate VM handler based on CID.
pub struct VsockListener;

impl VsockListener {
    /// # Errors
    ///
    /// Returns `SentinelError` on bind or accept failure.
    pub async fn accept_loop(&self) -> Result<(), SentinelError> {
        // TODO: bind vsock listener, accept connections, demux by CID
        // Each connection gets a tokio task for reading OperativeMessages
        Ok(())
    }
}
