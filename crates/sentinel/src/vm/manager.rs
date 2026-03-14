use crate::error::SentinelError;

/// Firecracker API client — communicates over Unix socket.
///
/// Handles: PUT /machine-config, PUT /boot-source, PUT /drives/*, PUT /vsock,
/// PUT /actions (`InstanceStart`), etc.
#[allow(dead_code)]
pub struct VmManager {
    pub(crate) firecracker_bin: std::path::PathBuf,
}

impl VmManager {
    #[must_use]
    pub fn new(firecracker_bin: std::path::PathBuf) -> Self {
        Self { firecracker_bin }
    }

    /// # Errors
    ///
    /// Returns `SentinelError::Vm` on spawn or configuration failure.
    pub async fn create_vm(
        &self,
        _config: &super::config::FirecrackerConfig,
    ) -> Result<VmHandle, SentinelError> {
        // TODO: spawn firecracker process, configure via API, start instance
        Err(SentinelError::Vm("not implemented".into()))
    }

    /// # Errors
    ///
    /// Returns `SentinelError::Vm` on cleanup failure.
    pub async fn destroy_vm(&self, _handle: &VmHandle) -> Result<(), SentinelError> {
        // TODO: kill firecracker process, cleanup socket
        Ok(())
    }
}

pub struct VmHandle {
    pub cid: u32,
    pub pid: u32,
    pub socket_path: std::path::PathBuf,
}
