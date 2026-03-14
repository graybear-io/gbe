use std::path::PathBuf;

/// Firecracker boot configuration builder.
///
/// Produces the JSON payloads for Firecracker's API:
/// /machine-config, /boot-source, /drives/rootfs, /vsock
pub struct FirecrackerConfig {
    pub vcpus: u32,
    pub mem_mb: u32,
    pub kernel_path: PathBuf,
    pub rootfs_path: PathBuf,
    pub vsock_cid: u32,
    pub socket_path: PathBuf,
}

impl FirecrackerConfig {
    #[must_use]
    pub fn machine_config_json(&self) -> serde_json::Value {
        serde_json::json!({
            "vcpu_count": self.vcpus,
            "mem_size_mib": self.mem_mb,
        })
    }

    #[must_use]
    pub fn boot_source_json(&self) -> serde_json::Value {
        serde_json::json!({
            "kernel_image_path": self.kernel_path.to_string_lossy(),
            "boot_args": "console=ttyS0 reboot=k panic=1 pci=off",
        })
    }

    #[must_use]
    pub fn rootfs_drive_json(&self) -> serde_json::Value {
        serde_json::json!({
            "drive_id": "rootfs",
            "path_on_host": self.rootfs_path.to_string_lossy(),
            "is_root_device": true,
            "is_read_only": false,
        })
    }

    #[must_use]
    pub fn vsock_json(&self) -> serde_json::Value {
        serde_json::json!({
            "guest_cid": self.vsock_cid,
            "uds_path": self.socket_path.to_string_lossy(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> FirecrackerConfig {
        FirecrackerConfig {
            vcpus: 2,
            mem_mb: 512,
            kernel_path: PathBuf::from("/opt/vmlinux"),
            rootfs_path: PathBuf::from("/images/base.ext4"),
            vsock_cid: 3,
            socket_path: PathBuf::from("/tmp/fc.sock"),
        }
    }

    #[test]
    fn machine_config_has_correct_fields() {
        let cfg = test_config();
        let json = cfg.machine_config_json();
        assert_eq!(json["vcpu_count"], 2);
        assert_eq!(json["mem_size_mib"], 512);
    }

    #[test]
    fn boot_source_has_kernel_and_args() {
        let cfg = test_config();
        let json = cfg.boot_source_json();
        assert_eq!(json["kernel_image_path"], "/opt/vmlinux");
        assert!(
            json["boot_args"]
                .as_str()
                .unwrap()
                .contains("console=ttyS0")
        );
        assert!(json["boot_args"].as_str().unwrap().contains("pci=off"));
    }

    #[test]
    fn rootfs_drive_is_root_device() {
        let cfg = test_config();
        let json = cfg.rootfs_drive_json();
        assert_eq!(json["drive_id"], "rootfs");
        assert_eq!(json["path_on_host"], "/images/base.ext4");
        assert_eq!(json["is_root_device"], true);
        assert_eq!(json["is_read_only"], false);
    }

    #[test]
    fn vsock_has_cid_and_path() {
        let cfg = test_config();
        let json = cfg.vsock_json();
        assert_eq!(json["guest_cid"], 3);
        assert_eq!(json["uds_path"], "/tmp/fc.sock");
    }
}
