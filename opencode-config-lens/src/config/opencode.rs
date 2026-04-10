//! OpenCode configuration ingestion
//!
//! This module handles ingestion of opencode.jsonc configuration.
//! It always produces usages for:
//! - model (general config - UsageClass::Default)
//! - small_model (general config - UsageClass::Default)
//! - agent entries (custom config - UsageClass::Custom)

use std::collections::HashMap;

use super::types::{ConfigSourceFamily, ConfigUsageLabel, UsageClass};
use super::OpenCodeConfig;

/// Collect usage from OpenCode configuration
///
/// Returns a map of model IDs to their usage labels from OpenCode config.
/// This includes both general config (model, small_model) and agent configs.
pub fn collect_opencode_usage(config: &OpenCodeConfig) -> HashMap<String, Vec<ConfigUsageLabel>> {
    let mut active: HashMap<String, Vec<ConfigUsageLabel>> = HashMap::new();

    // General config (model, small_model) -> Default usage class
    if let Some(model) = config.model.as_deref() {
        record_usage(&mut active, model, "default", UsageClass::Default);
    }
    if let Some(model) = config.small_model.as_deref() {
        record_usage(&mut active, model, "small_model", UsageClass::Default);
    }

    // Agent configs -> Custom usage class
    for (name, agent_config) in &config.agent {
        if let Some(model) = agent_config.model.as_deref() {
            record_usage(&mut active, model, name, UsageClass::Custom);
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
            family: ConfigSourceFamily::OpenCode,
            class,
        });
}

#[cfg(test)]
mod tests {
    use super::super::AgentConfig;
    use super::*;

    #[test]
    fn opencode_usage_should_map_general_config_to_default_class() {
        let config = OpenCodeConfig {
            model: Some("provider/alpha".to_string()),
            small_model: Some("provider/beta".to_string()),
            agent: HashMap::new(),
        };

        let usage = collect_opencode_usage(&config);

        assert!(usage.contains_key("provider/alpha"));
        assert!(usage.contains_key("provider/beta"));
        assert_eq!(
            usage.get("provider/alpha").unwrap()[0].class,
            UsageClass::Default
        );
        assert_eq!(
            usage.get("provider/beta").unwrap()[0].class,
            UsageClass::Default
        );
        assert_eq!(
            usage.get("provider/alpha").unwrap()[0].family,
            ConfigSourceFamily::OpenCode
        );
    }

    #[test]
    fn opencode_usage_should_map_agents_to_custom_class() {
        let config = OpenCodeConfig {
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
        };

        let usage = collect_opencode_usage(&config);

        assert!(usage.contains_key("provider/gamma"));
        assert_eq!(
            usage.get("provider/gamma").unwrap()[0].class,
            UsageClass::Custom
        );
        assert_eq!(usage.get("provider/gamma").unwrap()[0].label, "coder");
    }

    #[test]
    fn opencode_usage_should_include_both_general_and_agent_configs() {
        let config = OpenCodeConfig {
            model: Some("provider/alpha".to_string()),
            small_model: None,
            agent: [(
                "helper".to_string(),
                AgentConfig {
                    model: Some("provider/alpha".to_string()),
                    display_name: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        let usage = collect_opencode_usage(&config);

        let labels = usage.get("provider/alpha").unwrap();
        assert_eq!(labels.len(), 2);
        assert!(labels.iter().any(|l| l.class == UsageClass::Default));
        assert!(labels.iter().any(|l| l.class == UsageClass::Custom));
    }
}
