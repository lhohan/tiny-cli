//! Report adapter for config-to-report conversion
//!
//! This module provides the explicit conversion boundary between config ingestion
//! and report assembly. It maps config-owned usage records into report-owned
//! labels, styles, and order keys.

use crate::config::types::{ConfigSourceFamily, ConfigUsageLabel, UsageClass};
use crate::report::model::{UsageLabel, UsageSource};

/// Convert a config usage label to a report usage label
///
/// This is the single point where config source semantics are translated
/// into report presentation categories.
pub fn to_report_usage_label(config_label: &ConfigUsageLabel) -> UsageLabel {
    UsageLabel {
        label: config_label.label.clone(),
        source: to_report_usage_source(config_label.family, config_label.class),
    }
}

/// Convert config family/class to report usage source
///
/// Maps:
/// - OpenCode + Default -> OpenCodeDefault
/// - OpenCode + Custom -> OpenCodeCustom
/// - Weave + Default -> Weave
/// - Weave + Custom -> WeaveCustom
fn to_report_usage_source(family: ConfigSourceFamily, class: UsageClass) -> UsageSource {
    match (family, class) {
        (ConfigSourceFamily::OpenCode, UsageClass::Default) => UsageSource::OpenCodeDefault,
        (ConfigSourceFamily::OpenCode, UsageClass::Custom) => UsageSource::OpenCodeCustom,
        (ConfigSourceFamily::Weave, UsageClass::Default) => UsageSource::Weave,
        (ConfigSourceFamily::Weave, UsageClass::Custom) => UsageSource::WeaveCustom,
    }
}

/// Convert active usage from config representation to report representation
///
/// This is the main adapter entry point for transforming config ingestion output
/// into report input.
pub fn to_report_active_usage(
    config_usage: Vec<(String, Vec<ConfigUsageLabel>)>,
) -> Vec<(String, Vec<UsageLabel>)> {
    config_usage
        .into_iter()
        .map(|(model, labels)| {
            let report_labels: Vec<UsageLabel> = labels.iter().map(to_report_usage_label).collect();
            (model, report_labels)
        })
        .collect()
}

/// Get the source rank for sorting from config types
///
/// Returns the same ordering as report::sort::source_rank but works
/// directly with config types.
pub fn config_source_rank(family: ConfigSourceFamily, class: UsageClass) -> u8 {
    match (family, class) {
        (ConfigSourceFamily::OpenCode, UsageClass::Default) => 0,
        (ConfigSourceFamily::OpenCode, UsageClass::Custom) => 1,
        (ConfigSourceFamily::Weave, UsageClass::Default) => 2,
        (ConfigSourceFamily::Weave, UsageClass::Custom) => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_should_map_opencode_default_to_opencode_default() {
        let config_label = ConfigUsageLabel {
            label: "test".to_string(),
            family: ConfigSourceFamily::OpenCode,
            class: UsageClass::Default,
        };

        let report_label = to_report_usage_label(&config_label);

        assert_eq!(report_label.label, "test");
        assert_eq!(report_label.source, UsageSource::OpenCodeDefault);
    }

    #[test]
    fn adapter_should_map_opencode_custom_to_opencode_custom() {
        let config_label = ConfigUsageLabel {
            label: "agent".to_string(),
            family: ConfigSourceFamily::OpenCode,
            class: UsageClass::Custom,
        };

        let report_label = to_report_usage_label(&config_label);

        assert_eq!(report_label.source, UsageSource::OpenCodeCustom);
    }

    #[test]
    fn adapter_should_map_weave_default_to_weave() {
        let config_label = ConfigUsageLabel {
            label: "reviewer".to_string(),
            family: ConfigSourceFamily::Weave,
            class: UsageClass::Default,
        };

        let report_label = to_report_usage_label(&config_label);

        assert_eq!(report_label.source, UsageSource::Weave);
    }

    #[test]
    fn adapter_should_map_weave_custom_to_weave_custom() {
        let config_label = ConfigUsageLabel {
            label: "ops".to_string(),
            family: ConfigSourceFamily::Weave,
            class: UsageClass::Custom,
        };

        let report_label = to_report_usage_label(&config_label);

        assert_eq!(report_label.source, UsageSource::WeaveCustom);
    }

    #[test]
    fn adapter_should_convert_usage_vec() {
        let config_usage = vec![(
            "provider/alpha".to_string(),
            vec![
                ConfigUsageLabel {
                    label: "default".to_string(),
                    family: ConfigSourceFamily::OpenCode,
                    class: UsageClass::Default,
                },
                ConfigUsageLabel {
                    label: "coder".to_string(),
                    family: ConfigSourceFamily::OpenCode,
                    class: UsageClass::Custom,
                },
            ],
        )];

        let report_usage = to_report_active_usage(config_usage);

        assert_eq!(report_usage.len(), 1);
        assert_eq!(report_usage[0].0, "provider/alpha");
        assert_eq!(report_usage[0].1.len(), 2);
        assert_eq!(report_usage[0].1[0].source, UsageSource::OpenCodeDefault);
        assert_eq!(report_usage[0].1[1].source, UsageSource::OpenCodeCustom);
    }

    #[test]
    fn config_source_rank_should_match_report_rank() {
        // These should match the ordering in report::sort::source_rank
        assert_eq!(
            config_source_rank(ConfigSourceFamily::OpenCode, UsageClass::Default),
            0
        );
        assert_eq!(
            config_source_rank(ConfigSourceFamily::OpenCode, UsageClass::Custom),
            1
        );
        assert_eq!(
            config_source_rank(ConfigSourceFamily::Weave, UsageClass::Default),
            2
        );
        assert_eq!(
            config_source_rank(ConfigSourceFamily::Weave, UsageClass::Custom),
            3
        );
    }
}
