//! Unified config loading from `~/.config/gbe/`.
//!
//! Every node reads shared transport config from `gbe.yaml` and
//! node-specific config from `<node>.yaml`. CLI args can override
//! the config directory for dev/testing.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::DeserializeOwned;

/// Shared config that every node reads — transport, common settings.
///
/// Loaded from `~/.config/gbe/gbe.yaml`.
#[derive(Debug, Clone, Deserialize)]
pub struct SharedConfig {
    pub redis_url: String,
    #[serde(default = "default_max_payload")]
    pub max_payload_size: usize,
}

fn default_max_payload() -> usize {
    1_048_576 // 1MB
}

impl Default for SharedConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://127.0.0.1:6379".to_string(),
            max_payload_size: default_max_payload(),
        }
    }
}

/// Returns the GBE config directory.
///
/// Priority:
/// 1. `GBE_CONFIG_DIR` env var
/// 2. `~/.config/gbe/` (if it exists — preferred on all platforms)
/// 3. Platform default via `dirs::config_dir()` (~/Library/Application Support on macOS)
/// 4. `/etc/gbe` as last resort
pub fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("GBE_CONFIG_DIR") {
        return PathBuf::from(dir);
    }

    // Prefer ~/.config/gbe/ on all platforms — this is where we put it.
    if let Some(home) = dirs::home_dir() {
        let dot_config = home.join(".config").join("gbe");
        if dot_config.exists() {
            return dot_config;
        }
    }

    dirs::config_dir()
        .map(|d| d.join("gbe"))
        .unwrap_or_else(|| PathBuf::from("/etc/gbe"))
}

/// Load the shared config from `gbe.yaml` in the config directory.
///
/// Returns `SharedConfig::default()` if the file doesn't exist,
/// so nodes work out of the box without config.
pub fn load_shared() -> Result<SharedConfig, ConfigError> {
    load_shared_from(&config_dir())
}

/// Load shared config from a specific directory.
pub fn load_shared_from(dir: &Path) -> Result<SharedConfig, ConfigError> {
    let path = dir.join("gbe.yaml");
    if !path.exists() {
        return Ok(SharedConfig::default());
    }
    load_yaml(&path)
}

/// Load a node-specific config from `<node>.yaml` in the config directory.
///
/// Returns an error if the file doesn't exist — node configs are required
/// (unlike the shared config which has sensible defaults).
pub fn load_node<T: DeserializeOwned>(node: &str) -> Result<T, ConfigError> {
    load_node_from(&config_dir(), node)
}

/// Load a node-specific config from a specific directory.
pub fn load_node_from<T: DeserializeOwned>(dir: &Path, node: &str) -> Result<T, ConfigError> {
    let path = dir.join(format!("{node}.yaml"));
    load_yaml(&path)
}

fn load_yaml<T: DeserializeOwned>(path: &Path) -> Result<T, ConfigError> {
    let content = std::fs::read_to_string(path).map_err(|e| ConfigError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    serde_yaml::from_str(&content).map_err(|e| ConfigError::Parse {
        path: path.to_path_buf(),
        source: e,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("reading {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("parsing {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_yaml::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn shared_defaults_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = load_shared_from(tmp.path()).unwrap();
        assert_eq!(cfg.redis_url, "redis://127.0.0.1:6379");
        assert_eq!(cfg.max_payload_size, 1_048_576);
    }

    #[test]
    fn shared_loads_from_yaml() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("gbe.yaml"),
            "redis_url: redis://10.0.0.1:6379\n",
        )
        .unwrap();
        let cfg = load_shared_from(tmp.path()).unwrap();
        assert_eq!(cfg.redis_url, "redis://10.0.0.1:6379");
        assert_eq!(cfg.max_payload_size, 1_048_576); // default
    }

    #[test]
    fn node_config_loads() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("test-node.yaml"),
            "name: hello\nvalue: 42\n",
        )
        .unwrap();

        #[derive(Deserialize)]
        struct TestConfig {
            name: String,
            value: u32,
        }

        let cfg: TestConfig = load_node_from(tmp.path(), "test-node").unwrap();
        assert_eq!(cfg.name, "hello");
        assert_eq!(cfg.value, 42);
    }

    #[test]
    fn node_config_missing_is_error() {
        let tmp = tempfile::tempdir().unwrap();
        let result = load_node_from::<serde_json::Value>(tmp.path(), "nonexistent");
        assert!(result.is_err());
    }
}
