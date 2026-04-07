use std::collections::HashMap;

use super::{AgentConfig, ConfigBundle};
use crate::report::{UsageLabel, UsageSource};

pub fn collect_active_usage(bundle: &ConfigBundle) -> Vec<(String, Vec<UsageLabel>)> {
    let mut active: HashMap<String, Vec<UsageLabel>> = HashMap::new();

    if let Some(model) = bundle.opencode.model.as_deref() {
        record_usage(
            &mut active,
            model,
            "default".to_string(),
            UsageSource::OpenCodeDefault,
        );
    }
    if let Some(model) = bundle.opencode.small_model.as_deref() {
        record_usage(
            &mut active,
            model,
            "small_model".to_string(),
            UsageSource::OpenCodeDefault,
        );
    }

    for (name, cfg) in &bundle.opencode.agent {
        if let Some(model) = cfg.model.as_deref() {
            record_usage(
                &mut active,
                model,
                name.to_string(),
                UsageSource::OpenCodeCustom,
            );
        }
    }

    if let Some(weave) = bundle.weave.as_ref() {
        for (name, cfg) in &weave.agents {
            if let Some(model) = cfg.model.as_deref() {
                record_usage(
                    &mut active,
                    model,
                    weave_usage_label(name, cfg),
                    UsageSource::Weave,
                );
            }
        }
        for (name, cfg) in &weave.custom_agents {
            if let Some(model) = cfg.model.as_deref() {
                record_usage(
                    &mut active,
                    model,
                    weave_usage_label(name, cfg),
                    UsageSource::WeaveCustom,
                );
            }
        }
    }

    active.into_iter().collect()
}

fn record_usage(
    active: &mut HashMap<String, Vec<UsageLabel>>,
    model: &str,
    label: String,
    source: UsageSource,
) {
    active
        .entry(model.to_string())
        .or_default()
        .push(UsageLabel { label, source });
}

fn weave_usage_label(name: &str, cfg: &AgentConfig) -> String {
    cfg.display_name.as_deref().unwrap_or(name).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn active_usage_should_map_opencode_general_config_to_default_usage_class() {
        let bundle = ConfigBundle {
            opencode: super::super::OpenCodeConfig {
                model: Some("provider/alpha".to_string()),
                small_model: Some("provider/beta".to_string()),
                agent: HashMap::new(),
            },
            weave: None,
        };

        let usage = collect_active_usage(&bundle);
        let by_model: HashMap<_, _> = usage.into_iter().collect();

        assert_eq!(
            by_model.get("provider/alpha").unwrap()[0].source,
            crate::report::UsageSource::OpenCodeDefault
        );
        assert_eq!(
            by_model.get("provider/beta").unwrap()[0].source,
            crate::report::UsageSource::OpenCodeDefault
        );
    }

    #[test]
    fn active_usage_should_map_opencode_agents_to_custom_usage_class() {
        let bundle = ConfigBundle {
            opencode: super::super::OpenCodeConfig {
                model: None,
                small_model: None,
                agent: [(
                    "coder".to_string(),
                    super::super::AgentConfig {
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

        assert_eq!(
            by_model.get("provider/gamma").unwrap()[0].source,
            crate::report::UsageSource::OpenCodeCustom
        );
        assert_eq!(by_model.get("provider/gamma").unwrap()[0].label, "coder");
    }

    #[test]
    fn active_usage_should_yield_no_weave_labels_when_weave_absent() {
        let bundle = ConfigBundle {
            opencode: super::super::OpenCodeConfig {
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
                    !matches!(
                        label.source,
                        crate::report::UsageSource::Weave | crate::report::UsageSource::WeaveCustom
                    ),
                    "should not have weave labels when weave is absent"
                );
            }
        }
    }

    #[test]
    fn active_usage_should_distinguish_weave_agents_from_custom_agents() {
        let bundle = ConfigBundle {
            opencode: super::super::OpenCodeConfig {
                model: None,
                small_model: None,
                agent: HashMap::new(),
            },
            weave: Some(super::super::WeaveConfig {
                agents: [(
                    "reviewer".to_string(),
                    super::super::AgentConfig {
                        model: Some("provider/delta".to_string()),
                        display_name: None,
                    },
                )]
                .into_iter()
                .collect(),
                custom_agents: [(
                    "ops".to_string(),
                    super::super::AgentConfig {
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

        assert_eq!(
            by_model.get("provider/delta").unwrap()[0].source,
            crate::report::UsageSource::Weave
        );
        assert_eq!(
            by_model.get("provider/epsilon").unwrap()[0].source,
            crate::report::UsageSource::WeaveCustom
        );
    }

    #[test]
    fn active_usage_should_use_display_names_for_weave_labels() {
        let bundle = ConfigBundle {
            opencode: super::super::OpenCodeConfig {
                model: None,
                small_model: None,
                agent: HashMap::new(),
            },
            weave: Some(super::super::WeaveConfig {
                agents: [(
                    "reviewer".to_string(),
                    super::super::AgentConfig {
                        model: Some("provider/delta".to_string()),
                        display_name: Some("Review Bot".to_string()),
                    },
                )]
                .into_iter()
                .collect(),
                custom_agents: [(
                    "ops".to_string(),
                    super::super::AgentConfig {
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
            opencode: super::super::OpenCodeConfig {
                model: None,
                small_model: None,
                agent: HashMap::new(),
            },
            weave: Some(super::super::WeaveConfig {
                agents: [(
                    "reviewer".to_string(),
                    super::super::AgentConfig {
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
            opencode: super::super::OpenCodeConfig {
                model: Some("provider/alpha".to_string()),
                small_model: Some("provider/beta".to_string()),
                agent: [(
                    "builder".to_string(),
                    super::super::AgentConfig {
                        model: Some("provider/gamma".to_string()),
                        display_name: None,
                    },
                )]
                .into_iter()
                .collect(),
            },
            weave: Some(super::super::WeaveConfig {
                agents: [(
                    "reviewer".to_string(),
                    super::super::AgentConfig {
                        model: Some("provider/delta".to_string()),
                        display_name: None,
                    },
                )]
                .into_iter()
                .collect(),
                custom_agents: [(
                    "ops".to_string(),
                    super::super::AgentConfig {
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

        assert_eq!(
            by_model.get("provider/alpha").unwrap()[0].source,
            crate::report::UsageSource::OpenCodeDefault
        );
        assert_eq!(
            by_model.get("provider/beta").unwrap()[0].source,
            crate::report::UsageSource::OpenCodeDefault
        );
        assert_eq!(
            by_model.get("provider/gamma").unwrap()[0].source,
            crate::report::UsageSource::OpenCodeCustom
        );
        assert_eq!(
            by_model.get("provider/delta").unwrap()[0].source,
            crate::report::UsageSource::Weave
        );
        assert_eq!(
            by_model.get("provider/epsilon").unwrap()[0].source,
            crate::report::UsageSource::WeaveCustom
        );
    }
}
