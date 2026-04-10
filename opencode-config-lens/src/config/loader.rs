//! Configuration file loader
//!
//! This module handles reading and parsing configuration files from disk.
//! It is responsible for:
//! - Resolving config home directories
//! - Reading opencode.jsonc (required)
//! - Reading weave-opencode.jsonc (optional)
//! - Parsing JSONC (JSON with comments)

use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use super::{ConfigError, OpenCodeConfig, WeaveConfig};

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
pub fn load_config_files(
    home_dir: &Path,
) -> Result<(OpenCodeConfig, Option<WeaveConfig>), ConfigError> {
    let opencode_path = home_dir.join("opencode.jsonc");
    if !opencode_path.exists() {
        return Err(ConfigError::MissingConfig(opencode_path));
    }

    let opencode_text =
        std::fs::read_to_string(&opencode_path).map_err(|e| ConfigError::Io(e.to_string()))?;
    let opencode: OpenCodeConfig = parse_jsonc(&opencode_text).map_err(|e| ConfigError::Parse {
        path: opencode_path.clone(),
        message: e.to_string(),
    })?;

    let weave_path = home_dir.join("weave-opencode.jsonc");
    let weave = if weave_path.exists() {
        let weave_text =
            std::fs::read_to_string(&weave_path).map_err(|e| ConfigError::Io(e.to_string()))?;
        Some(parse_jsonc(&weave_text).map_err(|e| ConfigError::Parse {
            path: weave_path.clone(),
            message: e.to_string(),
        })?)
    } else {
        None
    };

    Ok((opencode, weave))
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

    fn make_temp_home() -> PathBuf {
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

    #[test]
    fn load_config_files_should_error_when_opencode_missing() {
        let home = make_temp_home();
        // Only create weave config, not opencode
        fs::write(home.join("weave-opencode.jsonc"), r#"{"agents": {}}"#).unwrap();

        let result = load_config_files(&home);
        assert!(
            matches!(result, Err(ConfigError::MissingConfig(_))),
            "should error when opencode.jsonc is missing"
        );
    }

    #[test]
    fn load_config_files_should_succeed_when_weave_missing() {
        let home = make_temp_home();
        fs::write(
            home.join("opencode.jsonc"),
            r#"{"model": "provider/alpha"}"#,
        )
        .unwrap();
        // Do not create weave-opencode.jsonc

        let (opencode, weave) = load_config_files(&home).unwrap();
        assert_eq!(opencode.model.as_deref(), Some("provider/alpha"));
        assert!(weave.is_none(), "weave should be None when file missing");
    }

    #[test]
    fn load_config_files_should_load_required_and_optional_files() {
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

        let (opencode, weave) = load_config_files(&home).unwrap();
        assert_eq!(opencode.model.as_deref(), Some("provider/alpha"));
        assert_eq!(opencode.small_model.as_deref(), Some("provider/beta"));
        let weave = weave.expect("weave config");
        assert_eq!(
            weave
                .agents
                .get("reviewer")
                .and_then(|a| a.model.as_deref()),
            Some("provider/delta")
        );
    }

    #[test]
    fn load_config_files_should_include_opencode_filename_in_parse_errors() {
        let home = make_temp_home();
        fs::write(
            home.join("opencode.jsonc"),
            r#"{"model": "provider/alpha", "model": "provider/beta"}"#,
        )
        .unwrap();

        let result = load_config_files(&home).unwrap_err();

        assert!(
            result.to_string().contains("opencode.jsonc"),
            "expected error to include source file name, got: {}",
            result
        );
    }

    #[test]
    fn load_config_files_should_include_weave_filename_in_parse_errors() {
        let home = make_temp_home();
        fs::write(
            home.join("opencode.jsonc"),
            r#"{"model": "provider/alpha"}"#,
        )
        .unwrap();
        fs::write(
            home.join("weave-opencode.jsonc"),
            r#"{"agents": {"reviewer": {"model": "provider/beta"}}"#,
        )
        .unwrap();

        let result = load_config_files(&home).unwrap_err();

        assert!(
            result.to_string().contains("weave-opencode.jsonc"),
            "expected error to include source file name, got: {}",
            result
        );
    }
}
