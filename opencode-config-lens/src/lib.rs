//! OpenCode Config Lens Library
//!
//! This library provides functionality for inspecting OpenCode model configuration and usage.
//!
//! ## Module Structure
//!
//! - `config`: Configuration file loading and parsing
//! - `report`: Domain model, sorting, formatting, and rendering
//! - `runtime`: TUI runtime and terminal handling
//!
//! ## Re-exports
//!
//! Key types are re-exported at the crate root for convenience.

use std::path::Path;

// Re-export config module types
pub mod config;
pub use config::{
    collect_active_usage, load_config_bundle, parse_jsonc, resolve_config_home, AgentConfig,
    ConfigBundle, ConfigError, OpenCodeConfig, WeaveConfig,
};

pub mod data;
pub use data::{
    extract_available_models, fetch_available_models, fetch_costs, parse_costs_from_api_json,
};

// Re-export report module types
pub mod report;
pub use report::{
    build_rows, format_cost, ljust, render_report_rows, rjust, split_model_id, strip_ansi,
    wrap_usage, ModelRow, ReportInput, SortMode, UsageLabel, UsageSource,
};

// Re-export app module types
pub mod app;
pub use app::{UiAction, UiKey, UiMode, UiState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    Config(ConfigError),
    OpenCodeNotFound,
    RefreshFailed { stderr: String, code: i32 },
    CurlNotFound,
    FetchFailed(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Config(err) => write!(f, "{}", err),
            LoadError::OpenCodeNotFound => write!(f, "opencode command not found"),
            LoadError::RefreshFailed { stderr, code } => {
                write!(
                    f,
                    "failed to refresh OpenCode models (exit {}): {}",
                    code, stderr
                )
            }
            LoadError::CurlNotFound => write!(f, "curl command not found"),
            LoadError::FetchFailed(msg) => write!(f, "failed to fetch model costs: {}", msg),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<ConfigError> for LoadError {
    fn from(value: ConfigError) -> Self {
        LoadError::Config(value)
    }
}

impl LoadError {
    pub fn exit_code(&self) -> i32 {
        match self {
            LoadError::RefreshFailed { code, .. } => *code,
            _ => 3,
        }
    }
}

// Config functions re-exported from config module

// render_report_rows already re-exported above via report module

pub fn load_report_rows(home_dir: &Path) -> Result<Vec<ModelRow>, LoadError> {
    let bundle = load_config_bundle(home_dir)?;
    let active_usage = collect_active_usage(&bundle);
    let available_models = fetch_available_models()?;
    let costs = fetch_costs()?;

    Ok(build_rows(
        ReportInput {
            active_usage,
            available_models,
            costs,
        },
        SortMode::ActiveFirst,
    ))
}

pub mod runtime;

// Re-export helper functions for backward compatibility
// Note: split_model_id already re-exported from report module above
pub use report::sort::{
    compare_costs, compare_costs_desc, compare_model_names, compare_rows, source_rank,
};

#[cfg(test)]
mod tests {
    use super::{
        build_rows, collect_active_usage, extract_available_models, load_config_bundle,
        parse_costs_from_api_json, parse_jsonc, render_report_rows, resolve_config_home,
        AgentConfig, ConfigBundle, ModelRow, OpenCodeConfig, ReportInput, SortMode, UiAction,
        UiKey, UiMode, UiState, UsageLabel, UsageSource, WeaveConfig,
    };
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    #[derive(Debug, Deserialize, PartialEq)]
    struct SampleConfig {
        name: String,
        values: Vec<u8>,
    }

    #[test]
    fn jsonc_should_ignore_comments_and_trailing_commas() {
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
    fn refresh_output_should_ignore_non_model_lines_and_duplicates() {
        let models = extract_available_models(
            "\u{1b}[32mprovider/alpha\u{1b}[0m\nModels cache refreshed\nnot-a-model\nprovider/beta\nprovider/alpha\n",
        );

        assert_eq!(models, vec!["provider/alpha", "provider/beta"]);
    }

    #[test]
    fn costs_json_should_map_provider_model_keys_to_input_and_output() {
        let costs = parse_costs_from_api_json(
            r#"
            {
              "provider": {
                "models": {
                  "alpha": { "cost": { "input": 1.25, "output": 2.5 } },
                  "beta": { "cost": { "input": 3.0 } },
                  "gamma": {}
                }
              }
            }
            "#,
        )
        .unwrap();

        assert_eq!(costs.get("provider/alpha"), Some(&(Some(1.25), Some(2.5))));
        assert_eq!(costs.get("provider/beta"), Some(&(Some(3.0), None)));
        assert_eq!(costs.get("provider/gamma"), Some(&(None, None)));
    }

    #[test]
    fn config_home_should_use_override_path_when_provided() {
        let path = PathBuf::from("/tmp/custom-opencode-home");
        let resolved = resolve_config_home(Some(&path)).unwrap();
        assert_eq!(resolved, path);
    }

    #[test]
    fn config_bundle_should_load_required_and_optional_files() {
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
        assert_eq!(
            bundle
                .opencode
                .agent
                .get("coder")
                .and_then(|a| a.model.as_deref()),
            Some("provider/gamma")
        );
        let weave = bundle.weave.expect("weave config");
        assert_eq!(
            weave
                .agents
                .get("reviewer")
                .and_then(|a| a.model.as_deref()),
            Some("provider/delta")
        );
        assert_eq!(
            weave
                .custom_agents
                .get("ops")
                .and_then(|a| a.model.as_deref()),
            Some("provider/epsilon")
        );
    }

    #[test]
    fn report_rows_should_render_unified_table_with_wrapped_usage() {
        let rows = build_rows(
            ReportInput {
                active_usage: vec![(
                    "provider/alpha".to_string(),
                    vec![
                        UsageLabel {
                            label: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                            source: UsageSource::OpenCodeDefault,
                        },
                        UsageLabel {
                            label: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
                            source: UsageSource::Weave,
                        },
                    ],
                )],
                available_models: vec!["provider/alpha".to_string(), "provider/beta".to_string()],
                costs: vec![
                    ("provider/alpha".to_string(), Some(1.0), Some(2.0)),
                    ("provider/beta".to_string(), Some(3.0), Some(4.0)),
                ],
            },
            SortMode::ModelName,
        );

        let lines = render_report_rows(&rows);
        assert_eq!(lines[0], "PROVIDER  MODEL  ACTIVE  IN  OUT  USAGE");
        assert!(lines.iter().any(|line| line.contains("provider  alpha")));
        assert!(lines.iter().any(|line| line.contains("yes")));
        assert!(lines.iter().any(|line| line.contains("no")));
        assert!(lines
            .iter()
            .any(|line| line.contains("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")));
        assert!(lines.len() > 3);
        assert!(lines[2].starts_with(&" ".repeat(34)));
    }

    #[test]
    fn ui_state_should_cycle_sort_modes_when_pressing_s() {
        let mut state = UiState::new();
        assert_eq!(state.mode, UiMode::Loading);

        let sequence = [
            SortMode::CostAsc,
            SortMode::CostDesc,
            SortMode::ModelName,
            SortMode::ActiveFirst,
        ];

        for expected in sequence {
            let action = state.handle_key(UiKey::CycleSort);
            assert_eq!(action, UiAction::None);
            assert_eq!(state.sort_mode, expected);
        }
    }

    #[test]
    fn ui_state_should_keep_rows_when_refresh_fails() {
        let mut state = UiState::new();
        state.apply_snapshot(vec![ModelRow {
            model: "provider/alpha".to_string(),
            provider: "provider".to_string(),
            model_name: "alpha".to_string(),
            active: true,
            input_cost: Some(1.0),
            output_cost: Some(2.0),
            usage: vec![],
        }]);
        state.set_refreshing();
        state.apply_refresh_error("refresh failed".to_string());

        assert_eq!(state.mode, UiMode::Ready);
        assert_eq!(state.status, "refresh failed");
        assert_eq!(state.visible_rows().len(), 1);
    }

    #[test]
    fn active_usage_should_distinguish_opencode_default_custom_and_weave_sources() {
        let bundle = ConfigBundle {
            opencode: OpenCodeConfig {
                model: Some("provider/alpha".to_string()),
                small_model: Some("provider/beta".to_string()),
                agent: [(
                    "builder".to_string(),
                    super::AgentConfig {
                        model: Some("provider/gamma".to_string()),
                        display_name: None,
                    },
                )]
                .into_iter()
                .collect(),
            },
            weave: Some(super::WeaveConfig {
                agents: [(
                    "reviewer".to_string(),
                    super::AgentConfig {
                        model: Some("provider/delta".to_string()),
                        display_name: None,
                    },
                )]
                .into_iter()
                .collect(),
                custom_agents: [(
                    "ops".to_string(),
                    super::AgentConfig {
                        model: Some("provider/epsilon".to_string()),
                        display_name: None,
                    },
                )]
                .into_iter()
                .collect(),
            }),
        };

        let usage = collect_active_usage(&bundle);
        let by_model: std::collections::HashMap<_, _> = usage.into_iter().collect();

        assert_eq!(
            by_model.get("provider/alpha").unwrap()[0].source,
            UsageSource::OpenCodeDefault
        );
        assert_eq!(
            by_model.get("provider/beta").unwrap()[0].source,
            UsageSource::OpenCodeDefault
        );
        assert_eq!(
            by_model.get("provider/gamma").unwrap()[0].source,
            UsageSource::OpenCodeCustom
        );
        assert_eq!(
            by_model.get("provider/delta").unwrap()[0].source,
            UsageSource::Weave
        );
        assert_eq!(
            by_model.get("provider/epsilon").unwrap()[0].source,
            UsageSource::WeaveCustom
        );
    }

    #[test]
    fn active_usage_should_distinguish_weave_agents_from_custom_agents() {
        let bundle = ConfigBundle {
            opencode: OpenCodeConfig {
                model: None,
                small_model: None,
                agent: HashMap::new(),
            },
            weave: Some(WeaveConfig {
                agents: [(
                    "reviewer".to_string(),
                    AgentConfig {
                        model: Some("provider/delta".to_string()),
                        display_name: None,
                    },
                )]
                .into_iter()
                .collect(),
                custom_agents: [(
                    "ops".to_string(),
                    AgentConfig {
                        model: Some("provider/epsilon".to_string()),
                        display_name: None,
                    },
                )]
                .into_iter()
                .collect(),
            }),
        };

        let usage = collect_active_usage(&bundle);
        let by_model: std::collections::HashMap<_, _> = usage.into_iter().collect();

        assert_eq!(
            by_model.get("provider/delta").unwrap()[0].source,
            UsageSource::Weave
        );
        assert_eq!(
            by_model.get("provider/epsilon").unwrap()[0].source,
            UsageSource::WeaveCustom
        );
    }

    #[test]
    fn active_usage_should_prefer_weave_display_names_when_configured() {
        let home = make_temp_home();
        fs::write(
            home.join("opencode.jsonc"),
            r#"{
                "agent": {}
            }"#,
        )
        .unwrap();
        fs::write(
            home.join("weave-opencode.jsonc"),
            r#"{
                "agents": {
                    "reviewer": {
                        "model": "provider/delta",
                        "display_name": "Review Bot"
                    }
                },
                "custom_agents": {
                    "ops": {
                        "model": "provider/epsilon",
                        "display_name": "Ops Bot"
                    }
                }
            }"#,
        )
        .unwrap();

        let bundle = load_config_bundle(&home).unwrap();
        let usage = collect_active_usage(&bundle);
        let by_model: std::collections::HashMap<_, _> = usage.into_iter().collect();

        assert_eq!(
            by_model.get("provider/delta").unwrap()[0].label,
            "Review Bot"
        );
        assert_eq!(
            by_model.get("provider/epsilon").unwrap()[0].label,
            "Ops Bot"
        );
    }

    fn make_temp_home() -> PathBuf {
        let mut base = std::env::temp_dir();
        base.push(format!(
            "opencode-config-lens-test-{}-{}",
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
