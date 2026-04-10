//! Config module
//!
//! This module handles configuration file loading and parsing.
//!
//! ## Module Structure
//!
//! - `types`: Config-owned types for usage tracking (no report dependencies)
//! - `loader`: File loading and JSONC parsing utilities
//! - `opencode`: OpenCode configuration ingestion (mandatory)
//! - `weave`: Weave configuration ingestion (optional)
//! - `usage`: Aggregates usage from all sources

use std::collections::HashMap;
use std::path::Path;

pub mod loader;
pub mod opencode;
pub mod types;
pub mod usage;
pub mod weave;

pub use loader::{load_config_files, parse_jsonc, resolve_config_home};
pub use types::{ConfigSourceFamily, ConfigUsageLabel, UsageClass};
pub use usage::collect_active_usage;

/// Error types for configuration operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    MissingConfig(std::path::PathBuf),
    Io(String),
    Parse {
        path: std::path::PathBuf,
        message: String,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingConfig(path) => {
                write!(f, "missing config file: {}", path.display())
            }
            ConfigError::Io(msg) => write!(f, "IO error: {}", msg),
            ConfigError::Parse { path, message } => {
                write!(f, "JSONC parse error in {}: {}", path.display(), message)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// OpenCode configuration from opencode.jsonc
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct OpenCodeConfig {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub small_model: Option<String>,
    #[serde(default)]
    pub agent: HashMap<String, AgentConfig>,
}

/// Agent configuration within opencode.jsonc
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct AgentConfig {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
}

/// Weave configuration from weave-opencode.jsonc
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct WeaveConfig {
    #[serde(default)]
    pub agents: HashMap<String, AgentConfig>,
    #[serde(default, rename = "custom_agents")]
    pub custom_agents: HashMap<String, AgentConfig>,
}

/// Bundle containing both opencode and optional weave config
///
/// This is the primary input to usage collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigBundle {
    pub opencode: OpenCodeConfig,
    pub weave: Option<WeaveConfig>,
}

/// Load a config bundle from the specified home directory
///
/// This is the main entry point for loading configuration.
/// It reads opencode.jsonc (required) and weave-opencode.jsonc (optional).
pub fn load_config_bundle(home_dir: &Path) -> Result<ConfigBundle, ConfigError> {
    let (opencode, weave) = loader::load_config_files(home_dir)?;
    Ok(ConfigBundle { opencode, weave })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn load_config_bundle_should_error_when_opencode_missing() {
        let home = make_temp_home();
        // Only create weave config, not opencode
        fs::write(home.join("weave-opencode.jsonc"), r#"{"agents": {}}"#).unwrap();

        let result = load_config_bundle(&home);
        assert!(
            matches!(result, Err(ConfigError::MissingConfig(_))),
            "should error when opencode.jsonc is missing"
        );
    }

    #[test]
    fn load_config_bundle_should_succeed_when_weave_missing() {
        let home = make_temp_home();
        fs::write(
            home.join("opencode.jsonc"),
            r#"{"model": "provider/alpha"}"#,
        )
        .unwrap();
        // Do not create weave-opencode.jsonc

        let bundle = load_config_bundle(&home).unwrap();
        assert_eq!(bundle.opencode.model.as_deref(), Some("provider/alpha"));
        assert!(
            bundle.weave.is_none(),
            "weave should be None when file missing"
        );
    }

    #[test]
    fn load_config_bundle_should_load_required_and_optional_files() {
        let home = make_temp_home();
        fs::write(
            home.join("opencode.jsonc"),
            r#"
            {
              // default model
              "model": "provider/alpha",
              "small_model": "provider/beta",
              "agent": {
                "coder": { "model": "provider/gamma", },
              },
            }
            "#,
        )
        .unwrap();
        fs::write(
            home.join("weave-opencode.jsonc"),
            r#"
            {
              "agents": {
                "reviewer": { "model": "provider/delta" }
              },
              "custom_agents": {
                "ops": { "model": "provider/epsilon" }
              }
            }
            "#,
        )
        .unwrap();

        let bundle = load_config_bundle(&home).unwrap();
        assert_eq!(bundle.opencode.model.as_deref(), Some("provider/alpha"));
        assert_eq!(
            bundle.opencode.small_model.as_deref(),
            Some("provider/beta")
        );
        let weave = bundle.weave.expect("weave config");
        assert_eq!(
            weave
                .agents
                .get("reviewer")
                .and_then(|a| a.model.as_deref()),
            Some("provider/delta")
        );
    }

    fn make_temp_home() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let mut base = std::env::temp_dir();
        base.push(format!(
            "opencode-config-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&base).unwrap();
        base
    }
}
