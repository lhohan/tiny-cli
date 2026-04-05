//! Config module
//!
//! This module handles configuration file loading and parsing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

pub mod usage;
pub use usage::collect_active_usage;

/// Error types for configuration operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    MissingConfig(PathBuf),
    Io(String),
    Parse(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingConfig(path) => {
                write!(f, "missing config file: {}", path.display())
            }
            ConfigError::Io(msg) => write!(f, "IO error: {}", msg),
            ConfigError::Parse(msg) => write!(f, "JSONC parse error: {}", msg),
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigBundle {
    pub opencode: OpenCodeConfig,
    pub weave: Option<WeaveConfig>,
}

/// Resolve the config home directory
///
/// If an override is provided, use that. Otherwise, use $HOME/.config/opencode
pub fn resolve_config_home(override_home: Option<&Path>) -> Result<PathBuf, ConfigError> {
    if let Some(path) = override_home {
        return Ok(path.to_path_buf());
    }

    let home = std::env::var_os("HOME")
        .ok_or_else(|| ConfigError::Io("HOME environment variable is not set".to_string()))?;
    Ok(PathBuf::from(home).join(".config/opencode"))
}

/// Load both opencode.jsonc and optional weave-opencode.jsonc
pub fn load_config_bundle(home_dir: &Path) -> Result<ConfigBundle, ConfigError> {
    let opencode_path = home_dir.join("opencode.jsonc");
    if !opencode_path.exists() {
        return Err(ConfigError::MissingConfig(opencode_path));
    }

    let opencode_text =
        std::fs::read_to_string(&opencode_path).map_err(|e| ConfigError::Io(e.to_string()))?;
    let opencode: OpenCodeConfig =
        parse_jsonc(&opencode_text).map_err(|e| ConfigError::Parse(e.to_string()))?;

    let weave_path = home_dir.join("weave-opencode.jsonc");
    let weave = if weave_path.exists() {
        let weave_text =
            std::fs::read_to_string(&weave_path).map_err(|e| ConfigError::Io(e.to_string()))?;
        Some(parse_jsonc(&weave_text).map_err(|e| ConfigError::Parse(e.to_string()))?)
    } else {
        None
    };

    Ok(ConfigBundle { opencode, weave })
}

/// Parse JSONC (JSON with comments) text
///
/// Strips both // comments and trailing commas before parsing
pub fn parse_jsonc<T: DeserializeOwned>(text: &str) -> serde_json::Result<T> {
    let stripped = strip_jsonc(text);
    serde_json::from_str(&stripped)
}

/// Strip JSONC comments and trailing commas
fn strip_jsonc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut in_string = false;
    let mut escape = false;

    while i < chars.len() {
        let ch = chars[i];
        let next = chars.get(i + 1).copied();

        if in_string {
            out.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            i += 1;
            continue;
        }

        if ch == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        out.push(ch);
        i += 1;
    }

    remove_trailing_commas(&out)
}

/// Remove trailing commas before closing braces/brackets
fn remove_trailing_commas(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == ',' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                i += 1;
                continue;
            }
        }

        result.push(ch);
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::fs;

    #[derive(Debug, Deserialize, PartialEq)]
    struct SampleConfig {
        name: String,
        values: Vec<u8>,
    }

    #[test]
    fn parse_jsonc_should_ignore_comments_and_trailing_commas() {
        let parsed: SampleConfig = parse_jsonc(
            r#"
            {
              // comment to ignore
              "name": "demo",
              "values": [1, 2, 3,],
            }
            "#,
        )
        .unwrap();

        assert_eq!(
            parsed,
            SampleConfig {
                name: "demo".to_string(),
                values: vec![1, 2, 3],
            }
        );
    }

    #[test]
    fn resolve_config_home_should_use_override_when_provided() {
        let path = PathBuf::from("/tmp/custom-opencode-home");
        let resolved = resolve_config_home(Some(&path)).unwrap();
        assert_eq!(resolved, path);
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

    fn make_temp_home() -> PathBuf {
        let mut base = std::env::temp_dir();
        base.push(format!(
            "opencode-config-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&base).unwrap();
        base
    }
}
