use crate::error::SentinelError;

/// Network setup for VMs.
///
/// Phase 1: tap device + iptables NAT (outbound internet access)
/// Phase 2: vsock-only with sentinel CONNECT proxy
/// Phase 3: vsock-only with tool proxy (no network in guest)
pub struct NetworkSetup;

impl NetworkSetup {
    /// # Errors
    ///
    /// Returns `SentinelError::Vm` on tap device creation failure.
    pub async fn create_tap(&self, _vm_id: &str) -> Result<TapDevice, SentinelError> {
        // TODO: create tap device, assign IP, add to bridge, configure iptables
        Err(SentinelError::Vm("tap create not implemented".into()))
    }

    /// # Errors
    ///
    /// Returns `SentinelError::Vm` on cleanup failure.
    pub async fn destroy_tap(&self, _tap: &TapDevice) -> Result<(), SentinelError> {
        // TODO: remove tap device, clean iptables rules
        Ok(())
    }
}

pub struct TapDevice {
    pub name: String,
    pub ip: String,
}
