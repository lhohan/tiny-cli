//! Config-owned types for usage tracking
//!
//! This module defines types that describe config source families and usage classes
//! without depending on report presentation types.

/// Source family for a usage record
///
/// This identifies which configuration source the usage originated from,
/// without encoding presentation concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigSourceFamily {
    /// OpenCode configuration (opencode.jsonc)
    OpenCode,
    /// Weave configuration (weave-opencode.jsonc)
    Weave,
}

/// Usage class within a source family
///
/// This identifies the type of usage within a given source family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UsageClass {
    /// Default/general configuration (model, small_model)
    Default,
    /// Agent configuration (custom agents)
    Custom,
}

/// A usage label produced by config ingestion
///
/// This is the config-owned representation of where a model is used.
/// Report code will convert this into presentation-specific types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigUsageLabel {
    /// The display label (e.g., "default", "agent-name")
    pub label: String,
    /// The source family (OpenCode or Weave)
    pub family: ConfigSourceFamily,
    /// The usage class within the family
    pub class: UsageClass,
}

/// A usage record for a model
///
/// This is the output of config ingestion: a model ID and its associated
/// usage labels from all configuration sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelUsage {
    /// The model identifier (e.g., "openai/gpt-4")
    pub model: String,
    /// Usage labels for this model
    pub labels: Vec<ConfigUsageLabel>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_source_family_should_support_equality() {
        assert_eq!(ConfigSourceFamily::OpenCode, ConfigSourceFamily::OpenCode);
        assert_ne!(ConfigSourceFamily::OpenCode, ConfigSourceFamily::Weave);
    }

    #[test]
    fn usage_class_should_support_equality() {
        assert_eq!(UsageClass::Default, UsageClass::Default);
        assert_ne!(UsageClass::Default, UsageClass::Custom);
    }

    #[test]
    fn config_usage_label_should_hold_data() {
        let label = ConfigUsageLabel {
            label: "test".to_string(),
            family: ConfigSourceFamily::OpenCode,
            class: UsageClass::Default,
        };
        assert_eq!(label.label, "test");
        assert_eq!(label.family, ConfigSourceFamily::OpenCode);
        assert_eq!(label.class, UsageClass::Default);
    }
}
