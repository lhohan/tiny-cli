//! Weave configuration ingestion
//!
//! This module handles ingestion of weave-opencode.jsonc configuration.
/// It produces usages only when the optional file exists for:
/// - agents (UsageClass::Default)
/// - custom_agents (UsageClass::Custom)
///
/// Display names are used for labels when available, falling back to the key.
use std::collections::HashMap;

use super::types::{ConfigSourceFamily, ConfigUsageLabel, UsageClass};
use super::WeaveConfig;

/// Collect usage from Weave configuration
///
/// Returns a map of model IDs to their usage labels from Weave config.
/// This includes both agents and custom_agents with display name support.
pub fn collect_weave_usage(config: &WeaveConfig) -> HashMap<String, Vec<ConfigUsageLabel>> {
    let mut active: HashMap<String, Vec<ConfigUsageLabel>> = HashMap::new();

    // Regular agents -> Default usage class
    for (name, agent_config) in &config.agents {
        if let Some(model) = agent_config.model.as_deref() {
            let label = agent_config.display_name.as_deref().unwrap_or(name);
            record_usage(&mut active, model, label, UsageClass::Default);
        }
    }

    // Custom agents -> Custom usage class
    for (name, agent_config) in &config.custom_agents {
        if let Some(model) = agent_config.model.as_deref() {
            let label = agent_config.display_name.as_deref().unwrap_or(name);
            record_usage(&mut active, model, label, UsageClass::Custom);
        }
    }

    active
}

fn record_usage(
    active: &mut HashMap<String, Vec<ConfigUsageLabel>>,
    model: &str,
    label: &str,
    class: UsageClass,
) {
    active
        .entry(model.to_string())
        .or_default()
        .push(ConfigUsageLabel {
            label: label.to_string(),
            family: ConfigSourceFamily::Weave,
            class,
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weave_usage_should_map_agents_to_default_class() {
        let config = WeaveConfig {
            agents: [(
                "reviewer".to_string(),
                super::super::AgentConfig {
                    model: Some("provider/beta".to_string()),
                    display_name: None,
                },
            )]
            .into_iter()
            .collect(),
            custom_agents: HashMap::new(),
        };

        let usage = collect_weave_usage(&config);

        assert!(usage.contains_key("provider/beta"));
        assert_eq!(
            usage.get("provider/beta").unwrap()[0].class,
            UsageClass::Default
        );
        assert_eq!(
            usage.get("provider/beta").unwrap()[0].family,
            ConfigSourceFamily::Weave
        );
    }

    #[test]
    fn weave_usage_should_map_custom_agents_to_custom_class() {
        let config = WeaveConfig {
            agents: HashMap::new(),
            custom_agents: [(
                "ops".to_string(),
                super::super::AgentConfig {
                    model: Some("provider/gamma".to_string()),
                    display_name: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        let usage = collect_weave_usage(&config);

        assert!(usage.contains_key("provider/gamma"));
        assert_eq!(
            usage.get("provider/gamma").unwrap()[0].class,
            UsageClass::Custom
        );
    }

    #[test]
    fn weave_usage_should_use_display_names_when_available() {
        let config = WeaveConfig {
            agents: [(
                "reviewer".to_string(),
                super::super::AgentConfig {
                    model: Some("provider/beta".to_string()),
                    display_name: Some("Review Bot".to_string()),
                },
            )]
            .into_iter()
            .collect(),
            custom_agents: [(
                "ops".to_string(),
                super::super::AgentConfig {
                    model: Some("provider/gamma".to_string()),
                    display_name: Some("Ops Assistant".to_string()),
                },
            )]
            .into_iter()
            .collect(),
        };

        let usage = collect_weave_usage(&config);

        assert_eq!(usage.get("provider/beta").unwrap()[0].label, "Review Bot");
        assert_eq!(
            usage.get("provider/gamma").unwrap()[0].label,
            "Ops Assistant"
        );
    }

    #[test]
    fn weave_usage_should_fallback_to_key_when_display_name_missing() {
        let config = WeaveConfig {
            agents: [(
                "reviewer".to_string(),
                super::super::AgentConfig {
                    model: Some("provider/beta".to_string()),
                    display_name: None,
                },
            )]
            .into_iter()
            .collect(),
            custom_agents: HashMap::new(),
        };

        let usage = collect_weave_usage(&config);

        assert_eq!(usage.get("provider/beta").unwrap()[0].label, "reviewer");
    }

    #[test]
    fn weave_usage_should_distinguish_agents_from_custom_agents() {
        let config = WeaveConfig {
            agents: [(
                "reviewer".to_string(),
                super::super::AgentConfig {
                    model: Some("provider/beta".to_string()),
                    display_name: None,
                },
            )]
            .into_iter()
            .collect(),
            custom_agents: [(
                "ops".to_string(),
                super::super::AgentConfig {
                    model: Some("provider/gamma".to_string()),
                    display_name: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        let usage = collect_weave_usage(&config);

        assert_eq!(
            usage.get("provider/beta").unwrap()[0].class,
            UsageClass::Default
        );
        assert_eq!(
            usage.get("provider/gamma").unwrap()[0].class,
            UsageClass::Custom
        );
    }
}
