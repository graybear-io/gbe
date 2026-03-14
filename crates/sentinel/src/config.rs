use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::SentinelError;

#[derive(Debug, Clone, Deserialize)]
pub struct SentinelConfig {
    pub host_id: String,
    pub slots: u32,
    pub image_dir: PathBuf,
    pub kernel_path: PathBuf,
    pub overlay_dir: PathBuf,
    pub firecracker_bin: PathBuf,
    pub profiles: HashMap<String, VmProfile>,
    pub task_types: Vec<String>,
    #[serde(default = "default_heartbeat")]
    pub heartbeat_interval_secs: u64,
}

impl SentinelConfig {
    /// Validate all paths and identifiers after deserialization.
    ///
    /// # Errors
    ///
    /// Returns `SentinelError::Config` if any path or identifier is invalid.
    pub fn validate(&self) -> Result<(), SentinelError> {
        Self::validate_host_id(&self.host_id)?;
        Self::require_dir(&self.image_dir, "image_dir")?;
        Self::require_file(&self.kernel_path, "kernel_path")?;
        Self::require_dir(&self.overlay_dir, "overlay_dir")?;
        Self::require_file(&self.firecracker_bin, "firecracker_bin")?;
        Self::reject_traversal(&self.image_dir, "image_dir")?;
        Self::reject_traversal(&self.kernel_path, "kernel_path")?;
        Self::reject_traversal(&self.overlay_dir, "overlay_dir")?;
        Self::reject_traversal(&self.firecracker_bin, "firecracker_bin")?;
        Ok(())
    }

    fn validate_host_id(id: &str) -> Result<(), SentinelError> {
        if id.is_empty() || id.len() > 128 {
            return Err(SentinelError::Config(format!(
                "host_id: must be 1-128 characters, got {}",
                id.len()
            )));
        }
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(SentinelError::Config(
                "host_id: must contain only alphanumeric, hyphen, underscore, or dot".to_string(),
            ));
        }
        Ok(())
    }

    fn require_dir(path: &Path, field: &str) -> Result<(), SentinelError> {
        if !path.exists() {
            return Err(SentinelError::Config(format!(
                "{field}: path does not exist: {}",
                path.display()
            )));
        }
        if !path.is_dir() {
            return Err(SentinelError::Config(format!(
                "{field}: expected directory, got file: {}",
                path.display()
            )));
        }
        Ok(())
    }

    fn require_file(path: &Path, field: &str) -> Result<(), SentinelError> {
        if !path.exists() {
            return Err(SentinelError::Config(format!(
                "{field}: path does not exist: {}",
                path.display()
            )));
        }
        if !path.is_file() {
            return Err(SentinelError::Config(format!(
                "{field}: expected file, got directory: {}",
                path.display()
            )));
        }
        Ok(())
    }

    fn reject_traversal(path: &Path, field: &str) -> Result<(), SentinelError> {
        let canonical = path
            .canonicalize()
            .map_err(|e| SentinelError::Config(format!("{field}: cannot resolve path: {e}")))?;
        if canonical
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(SentinelError::Config(format!(
                "{field}: path contains traversal: {}",
                path.display()
            )));
        }
        Ok(())
    }
}

fn default_heartbeat() -> u64 {
    10
}

#[derive(Debug, Clone, Deserialize)]
pub struct VmProfile {
    pub vcpus: u32,
    pub mem_mb: u32,
    pub rootfs: String,
    #[serde(default = "default_timeout")]
    pub timeout_sec: u64,
    #[serde(default)]
    pub network: NetworkMode,
    pub network_policy: Option<NetworkPolicy>,
    pub tool_policy: Option<ToolPolicy>,
}

fn default_timeout() -> u64 {
    300
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkMode {
    #[default]
    Nat,
    Proxy,
    None,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkPolicy {
    pub mode: String,
    #[serde(default)]
    pub allow: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolPolicy {
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    pub rate_limit: Option<RateLimit>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimit {
    pub calls_per_minute: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn valid_config(tmp: &std::path::Path) -> SentinelConfig {
        let image_dir = tmp.join("images");
        let overlay_dir = tmp.join("overlays");
        fs::create_dir_all(&image_dir).unwrap();
        fs::create_dir_all(&overlay_dir).unwrap();
        let kernel = tmp.join("vmlinux");
        let fc_bin = tmp.join("firecracker");
        fs::write(&kernel, b"").unwrap();
        fs::write(&fc_bin, b"").unwrap();

        SentinelConfig {
            host_id: "host-01".into(),
            slots: 4,
            image_dir,
            kernel_path: kernel,
            overlay_dir,
            firecracker_bin: fc_bin,
            profiles: HashMap::new(),
            task_types: vec!["shell".into()],
            heartbeat_interval_secs: 10,
        }
    }

    #[test]
    fn valid_config_passes() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = valid_config(tmp.path());
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn empty_host_id_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cfg = valid_config(tmp.path());
        cfg.host_id = String::new();
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("host_id"));
    }

    #[test]
    fn long_host_id_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cfg = valid_config(tmp.path());
        cfg.host_id = "a".repeat(129);
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("host_id"));
    }

    #[test]
    fn host_id_with_special_chars_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cfg = valid_config(tmp.path());
        cfg.host_id = "host/evil".into();
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("host_id"));
    }

    #[test]
    fn host_id_allows_hyphens_underscores_dots() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cfg = valid_config(tmp.path());
        cfg.host_id = "host-01_az.local".into();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn missing_image_dir_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cfg = valid_config(tmp.path());
        cfg.image_dir = tmp.path().join("nonexistent");
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("image_dir"));
    }

    #[test]
    fn file_as_dir_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cfg = valid_config(tmp.path());
        let file = tmp.path().join("not-a-dir");
        fs::write(&file, b"").unwrap();
        cfg.image_dir = file;
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("expected directory"));
    }

    #[test]
    fn missing_kernel_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cfg = valid_config(tmp.path());
        cfg.kernel_path = tmp.path().join("missing-kernel");
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("kernel_path"));
    }

    #[test]
    fn dir_as_file_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cfg = valid_config(tmp.path());
        cfg.kernel_path = tmp.path().join("images");
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("expected file"));
    }

    #[test]
    fn deserialization_defaults() {
        let json = r#"{
            "host_id": "h1",
            "slots": 2,
            "image_dir": "/tmp",
            "kernel_path": "/tmp/k",
            "overlay_dir": "/tmp",
            "firecracker_bin": "/tmp/fc",
            "profiles": {},
            "task_types": ["shell"]
        }"#;
        let cfg: SentinelConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.heartbeat_interval_secs, 10);
    }

    #[test]
    fn vm_profile_defaults() {
        let json = r#"{"vcpus": 2, "mem_mb": 256, "rootfs": "base.ext4"}"#;
        let p: VmProfile = serde_json::from_str(json).unwrap();
        assert_eq!(p.timeout_sec, 300);
        assert!(matches!(p.network, NetworkMode::Nat));
    }

    #[test]
    fn network_mode_variants() {
        let json_nat = r#"{"vcpus":1,"mem_mb":128,"rootfs":"r","network":"nat"}"#;
        let json_proxy = r#"{"vcpus":1,"mem_mb":128,"rootfs":"r","network":"proxy"}"#;
        let json_none = r#"{"vcpus":1,"mem_mb":128,"rootfs":"r","network":"none"}"#;
        assert!(matches!(
            serde_json::from_str::<VmProfile>(json_nat).unwrap().network,
            NetworkMode::Nat
        ));
        assert!(matches!(
            serde_json::from_str::<VmProfile>(json_proxy)
                .unwrap()
                .network,
            NetworkMode::Proxy
        ));
        assert!(matches!(
            serde_json::from_str::<VmProfile>(json_none)
                .unwrap()
                .network,
            NetworkMode::None
        ));
    }
}
