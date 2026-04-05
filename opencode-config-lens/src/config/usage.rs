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
