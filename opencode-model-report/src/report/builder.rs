//! Report builder
//!
//! This module provides functions for building ModelRow instances from input data.

use std::collections::HashMap;

use super::model::{ModelRow, ReportInput, SortMode, UsageLabel};
use super::sort::compare_rows;
use super::sort::source_rank;
use super::text::split_model_id;

/// Build model rows from report input data
///
/// This function:
/// 1. Collects usage labels by model
/// 2. Matches costs to models
/// 3. Creates ModelRow instances for all available models
/// 4. Sorts them according to the specified sort mode
pub fn build_rows(input: ReportInput, sort_mode: SortMode) -> Vec<ModelRow> {
    let mut usage_by_model: HashMap<String, Vec<UsageLabel>> = HashMap::new();
    for (model, mut usages) in input.active_usage {
        usages.sort_by(|a, b| {
            a.label
                .cmp(&b.label)
                .then_with(|| source_rank(a.source).cmp(&source_rank(b.source)))
        });
        usage_by_model.insert(model, usages);
    }

    let costs: HashMap<String, (Option<f64>, Option<f64>)> = input
        .costs
        .into_iter()
        .map(|(model, input_cost, output_cost)| (model, (input_cost, output_cost)))
        .collect();
    let active_models: std::collections::HashSet<String> = usage_by_model.keys().cloned().collect();

    let mut rows: Vec<ModelRow> = input
        .available_models
        .into_iter()
        .map(|model| {
            let usage = usage_by_model.remove(&model).unwrap_or_default();
            let (input_cost, output_cost) = costs.get(&model).copied().unwrap_or((None, None));
            let (provider, model_name) = split_model_id(&model);
            ModelRow {
                active: active_models.contains(&model),
                model,
                provider,
                model_name,
                input_cost,
                output_cost,
                usage,
            }
        })
        .collect();

    rows.sort_by(|a, b| compare_rows(a, b, sort_mode));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::model::{UsageLabel, UsageSource};

    #[test]
    fn build_rows_should_create_rows_for_all_models() {
        let input = ReportInput {
            active_usage: vec![],
            available_models: vec!["p/alpha".to_string(), "p/beta".to_string()],
            costs: vec![],
        };

        let rows = build_rows(input, SortMode::ModelName);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn build_rows_should_mark_active_models() {
        let input = ReportInput {
            active_usage: vec![(
                "p/alpha".to_string(),
                vec![UsageLabel {
                    label: "default".to_string(),
                    source: UsageSource::OpenCodeDefault,
                }],
            )],
            available_models: vec!["p/alpha".to_string(), "p/beta".to_string()],
            costs: vec![],
        };

        let rows = build_rows(input, SortMode::ModelName);
        let alpha = rows.iter().find(|r| r.model == "p/alpha").unwrap();
        let beta = rows.iter().find(|r| r.model == "p/beta").unwrap();

        assert!(alpha.active);
        assert!(!beta.active);
    }

    #[test]
    fn build_rows_should_attach_costs() {
        let input = ReportInput {
            active_usage: vec![],
            available_models: vec!["p/alpha".to_string()],
            costs: vec![("p/alpha".to_string(), Some(1.0), Some(2.0))],
        };

        let rows = build_rows(input, SortMode::ModelName);
        let alpha = &rows[0];

        assert_eq!(alpha.input_cost, Some(1.0));
        assert_eq!(alpha.output_cost, Some(2.0));
    }
}
