//! Report domain model types
//!
//! This module contains the core data types for the model report:
//! - ModelRow: A row in the report representing a model
//! - UsageLabel: Labels indicating where a model is used
//! - UsageSource: The origin of a usage label
//! - SortMode: Different sorting strategies for the report
//! - ReportInput: Input data for building a report

/// Sorting mode for the report
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    /// Active models first, then by cost, then by name
    #[default]
    ActiveFirst,
    /// Sort by total cost ascending
    CostAsc,
    /// Sort by total cost descending
    CostDesc,
    /// Sort by model name alphabetically
    ModelName,
}

/// Source of a usage label
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UsageSource {
    /// Default model or small_model from opencode.jsonc
    OpenCodeDefault,
    /// Agent configuration from opencode.jsonc
    OpenCodeCustom,
    /// Agent from weave-opencode.jsonc agents section
    Weave,
    /// Agent from weave-opencode.jsonc custom_agents section
    WeaveCustom,
}

impl UsageSource {
    /// Get the display text for the legend
    pub fn legend_text(&self) -> &'static str {
        match self {
            UsageSource::OpenCodeDefault => "OpenCode",
            UsageSource::OpenCodeCustom => "OpenCode agents",
            UsageSource::Weave => "Weave agents",
            UsageSource::WeaveCustom => "Weave custom_agents",
        }
    }

    /// Get all legend entries in display order
    pub fn all_legend_entries() -> &'static [UsageSource] {
        &[
            UsageSource::OpenCodeDefault,
            UsageSource::OpenCodeCustom,
            UsageSource::Weave,
            UsageSource::WeaveCustom,
        ]
    }
}

/// A label indicating where a model is used
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageLabel {
    /// The display label (e.g., "default", "agent-name")
    pub label: String,
    /// Where this usage label originated from
    pub source: UsageSource,
}

/// A row in the model report
#[derive(Debug, Clone, PartialEq)]
pub struct ModelRow {
    /// Full model identifier (e.g., "openai/gpt-4")
    pub model: String,
    /// Provider portion of the model ID
    pub provider: String,
    /// Model name portion (after the slash)
    pub model_name: String,
    /// Whether this model is actively used in any config
    pub active: bool,
    /// Input cost per 1M tokens
    pub input_cost: Option<f64>,
    /// Output cost per 1M tokens
    pub output_cost: Option<f64>,
    /// Usage labels indicating where this model is configured
    pub usage: Vec<UsageLabel>,
}

impl ModelRow {
    /// Calculate the total cost (input + output) if both are known
    pub fn total_cost(&self) -> Option<f64> {
        Some(self.input_cost? + self.output_cost?)
    }
}

/// Input data for building a report
#[derive(Debug, Default, Clone)]
pub struct ReportInput {
    /// Active usage: model ID -> list of usage labels
    pub active_usage: Vec<(String, Vec<UsageLabel>)>,
    /// Available models from opencode inventory
    pub available_models: Vec<String>,
    /// Cost data: model ID -> (input_cost, output_cost)
    pub costs: Vec<(String, Option<f64>, Option<f64>)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_row_should_calculate_total_cost() {
        let row = ModelRow {
            model: "test/model".to_string(),
            provider: "test".to_string(),
            model_name: "model".to_string(),
            active: true,
            input_cost: Some(1.0),
            output_cost: Some(2.0),
            usage: vec![],
        };
        assert_eq!(row.total_cost(), Some(3.0));
    }

    #[test]
    fn model_row_should_return_none_for_unknown_total_cost() {
        let row = ModelRow {
            model: "test/model".to_string(),
            provider: "test".to_string(),
            model_name: "model".to_string(),
            active: true,
            input_cost: None,
            output_cost: Some(2.0),
            usage: vec![],
        };
        assert_eq!(row.total_cost(), None);
    }
}
