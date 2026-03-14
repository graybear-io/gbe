use std::path::{Path, PathBuf};

use crate::error::SentinelError;

/// Copy-on-Write rootfs overlay management.
///
/// Each VM gets a copy-on-write snapshot of the base image:
/// 1. Create sparse copy (or device-mapper snapshot)
/// 2. VM writes land in overlay, base stays clean
/// 3. On teardown, delete overlay — instant cleanup
pub struct OverlayManager {
    pub overlay_dir: PathBuf,
}

impl OverlayManager {
    #[must_use]
    pub fn new(overlay_dir: PathBuf) -> Self {
        Self { overlay_dir }
    }

    /// # Errors
    ///
    /// Returns `SentinelError::Vm` on overlay creation failure.
    pub async fn create(&self, _base_image: &Path, _vm_id: &str) -> Result<PathBuf, SentinelError> {
        // TODO: create CoW snapshot of base image
        // Options: cp --reflink=auto, qemu-img create -b, device-mapper
        Err(SentinelError::Vm("overlay create not implemented".into()))
    }

    /// # Errors
    ///
    /// Returns `SentinelError::Vm` on cleanup failure.
    pub async fn destroy(&self, _overlay_path: &Path) -> Result<(), SentinelError> {
        // TODO: remove overlay file
        Ok(())
    }
}
