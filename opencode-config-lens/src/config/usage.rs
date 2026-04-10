//! Config usage collection
//!
//! This module aggregates usage from all configuration sources into a
//! config-owned representation. It combines OpenCode (mandatory) and
//! Weave (optional) usage into a unified model-to-labels mapping.

use std::collections::HashMap;

use super::types::ConfigUsageLabel;
use super::{opencode, weave, ConfigBundle};

/// Collect active usage from all configuration sources
///
/// This function aggregates usage labels from:
/// - OpenCode configuration (always present)
/// - Weave configuration (only if present)
///
/// Returns a vector of (model_id, usage_labels) pairs.
pub fn collect_active_usage(bundle: &ConfigBundle) -> Vec<(String, Vec<ConfigUsageLabel>)> {
    let mut active: HashMap<String, Vec<ConfigUsageLabel>> = HashMap::new();

    // Collect OpenCode usage (mandatory)
    let opencode_usage = opencode::collect_opencode_usage(&bundle.opencode);
    for (model, labels) in opencode_usage {
        active.entry(model).or_default().extend(labels);
    }

    // Collect Weave usage (optional)
    if let Some(weave_config) = bundle.weave.as_ref() {
        let weave_usage = weave::collect_weave_usage(weave_config);
        for (model, labels) in weave_usage {
            active.entry(model).or_default().extend(labels);
        }
    }

    active.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::super::types::{ConfigSourceFamily, UsageClass};
    use super::super::{AgentConfig, OpenCodeConfig, WeaveConfig};
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn active_usage_should_map_opencode_general_config_to_default_usage_class() {
        let bundle = ConfigBundle {
            opencode: OpenCodeConfig {
                model: Some("provider/alpha".to_string()),
                small_model: Some("provider/beta".to_string()),
                agent: HashMap::new(),
            },
            weave: None,
        };

        let usage = collect_active_usage(&bundle);
        let by_model: HashMap<_, _> = usage.into_iter().collect();

        assert!(by_model.contains_key("provider/alpha"));
        assert!(by_model.contains_key("provider/beta"));
        assert_eq!(
            by_model.get("provider/alpha").unwrap()[0].family,
            ConfigSourceFamily::OpenCode
        );
        assert_eq!(
            by_model.get("provider/alpha").unwrap()[0].class,
            UsageClass::Default
        );
        assert_eq!(
            by_model.get("provider/beta").unwrap()[0].class,
            UsageClass::Default
        );
    }

    #[test]
    fn active_usage_should_map_opencode_agents_to_custom_usage_class() {
        let bundle = ConfigBundle {
            opencode: OpenCodeConfig {
                model: None,
                small_model: None,
                agent: [(
                    "coder".to_string(),
                    AgentConfig {
                        model: Some("provider/gamma".to_string()),
                        display_name: None,
                    },
                )]
                .into_iter()
                .collect(),
            },
            weave: None,
        };

        let usage = collect_active_usage(&bundle);
        let by_model: HashMap<_, _> = usage.into_iter().collect();

        assert!(by_model.contains_key("provider/gamma"));
        assert_eq!(
            by_model.get("provider/gamma").unwrap()[0].family,
            ConfigSourceFamily::OpenCode
        );
        assert_eq!(
            by_model.get("provider/gamma").unwrap()[0].class,
            UsageClass::Custom
        );
        assert_eq!(by_model.get("provider/gamma").unwrap()[0].label, "coder");
    }

    #[test]
    fn active_usage_should_yield_no_weave_labels_when_weave_absent() {
        let bundle = ConfigBundle {
            opencode: OpenCodeConfig {
                model: Some("provider/alpha".to_string()),
                small_model: None,
                agent: HashMap::new(),
            },
            weave: None,
        };

        let usage = collect_active_usage(&bundle);

        for (_, labels) in &usage {
            for label in labels {
                assert!(
                    !matches!(label.family, ConfigSourceFamily::Weave),
                    "should not have weave labels when weave is absent"
                );
            }
        }
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
        let by_model: HashMap<_, _> = usage.into_iter().collect();

        assert!(by_model.contains_key("provider/delta"));
        assert!(by_model.contains_key("provider/epsilon"));
        assert_eq!(
            by_model.get("provider/delta").unwrap()[0].class,
            UsageClass::Default
        );
        assert_eq!(
            by_model.get("provider/epsilon").unwrap()[0].class,
            UsageClass::Custom
        );
    }

    #[test]
    fn active_usage_should_use_display_names_for_weave_labels() {
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
                        display_name: Some("Review Bot".to_string()),
                    },
                )]
                .into_iter()
                .collect(),
                custom_agents: [(
                    "ops".to_string(),
                    AgentConfig {
                        model: Some("provider/epsilon".to_string()),
                        display_name: Some("Ops Assistant".to_string()),
                    },
                )]
                .into_iter()
                .collect(),
            }),
        };

        let usage = collect_active_usage(&bundle);
        let by_model: HashMap<_, _> = usage.into_iter().collect();

        assert_eq!(
            by_model.get("provider/delta").unwrap()[0].label,
            "Review Bot"
        );
        assert_eq!(
            by_model.get("provider/epsilon").unwrap()[0].label,
            "Ops Assistant"
        );
    }

    #[test]
    fn active_usage_should_fallback_to_key_when_display_name_missing() {
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
                custom_agents: HashMap::new(),
            }),
        };

        let usage = collect_active_usage(&bundle);
        let by_model: HashMap<_, _> = usage.into_iter().collect();

        assert_eq!(by_model.get("provider/delta").unwrap()[0].label, "reviewer");
    }

    #[test]
    fn active_usage_should_distinguish_all_source_types() {
        let bundle = ConfigBundle {
            opencode: OpenCodeConfig {
                model: Some("provider/alpha".to_string()),
                small_model: Some("provider/beta".to_string()),
                agent: [(
                    "builder".to_string(),
                    AgentConfig {
                        model: Some("provider/gamma".to_string()),
                        display_name: None,
                    },
                )]
                .into_iter()
                .collect(),
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
        let by_model: HashMap<_, _> = usage.into_iter().collect();

        // OpenCode general config
        assert_eq!(
            by_model.get("provider/alpha").unwrap()[0],
            ConfigUsageLabel {
                label: "default".to_string(),
                family: ConfigSourceFamily::OpenCode,
                class: UsageClass::Default,
            }
        );
        // OpenCode small_model
        assert_eq!(
            by_model.get("provider/beta").unwrap()[0],
            ConfigUsageLabel {
                label: "small_model".to_string(),
                family: ConfigSourceFamily::OpenCode,
                class: UsageClass::Default,
            }
        );
        // OpenCode agent
        assert_eq!(
            by_model.get("provider/gamma").unwrap()[0],
            ConfigUsageLabel {
                label: "builder".to_string(),
                family: ConfigSourceFamily::OpenCode,
                class: UsageClass::Custom,
            }
        );
        // Weave agent
        assert_eq!(
            by_model.get("provider/delta").unwrap()[0],
            ConfigUsageLabel {
                label: "reviewer".to_string(),
                family: ConfigSourceFamily::Weave,
                class: UsageClass::Default,
            }
        );
        // Weave custom_agent
        assert_eq!(
            by_model.get("provider/epsilon").unwrap()[0],
            ConfigUsageLabel {
                label: "ops".to_string(),
                family: ConfigSourceFamily::Weave,
                class: UsageClass::Custom,
            }
        );
    }
}
