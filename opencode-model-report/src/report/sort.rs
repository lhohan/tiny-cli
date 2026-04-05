//! Sorting logic for model rows
//!
//! This module provides deterministic sorting comparators for ModelRow.

use std::cmp::Ordering;

use super::model::{ModelRow, SortMode, UsageSource};

/// Compare two rows according to the specified sort mode
pub fn compare_rows(a: &ModelRow, b: &ModelRow, mode: SortMode) -> Ordering {
    let active_cmp = b.active.cmp(&a.active);
    let cost_cmp = compare_costs(a.total_cost(), b.total_cost());
    let name_cmp = compare_model_names(&a.model, &b.model);

    match mode {
        SortMode::ActiveFirst => active_cmp.then(cost_cmp).then(name_cmp),
        SortMode::CostAsc => cost_cmp.then(name_cmp),
        SortMode::CostDesc => compare_costs_desc(a.total_cost(), b.total_cost()).then(name_cmp),
        SortMode::ModelName => name_cmp,
    }
}

/// Compare model names alphabetically
pub fn compare_model_names(a: &str, b: &str) -> Ordering {
    a.cmp(b)
}

/// Compare costs: known costs come before unknown
pub fn compare_costs(a: Option<f64>, b: Option<f64>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

/// Compare costs in descending order
pub fn compare_costs_desc(a: Option<f64>, b: Option<f64>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => b.partial_cmp(&a).unwrap_or(Ordering::Equal),
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

/// Get a ranking for usage source (for sorting labels consistently)
pub fn source_rank(source: UsageSource) -> u8 {
    match source {
        UsageSource::OpenCodeDefault => 0,
        UsageSource::OpenCodeCustom => 1,
        UsageSource::Weave => 2,
        UsageSource::WeaveCustom => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_costs_should_sort_known_before_unknown() {
        assert_eq!(compare_costs(Some(1.0), None), Ordering::Less);
        assert_eq!(compare_costs(None, Some(1.0)), Ordering::Greater);
        assert_eq!(compare_costs(None, None), Ordering::Equal);
    }

    #[test]
    fn compare_costs_should_sort_by_value() {
        assert_eq!(compare_costs(Some(1.0), Some(2.0)), Ordering::Less);
        assert_eq!(compare_costs(Some(2.0), Some(1.0)), Ordering::Greater);
        assert_eq!(compare_costs(Some(1.0), Some(1.0)), Ordering::Equal);
    }

    #[test]
    fn compare_costs_desc_should_reverse_order() {
        assert_eq!(compare_costs_desc(Some(1.0), Some(2.0)), Ordering::Greater);
        assert_eq!(compare_costs_desc(Some(2.0), Some(1.0)), Ordering::Less);
    }

    #[test]
    fn source_rank_should_be_consistent() {
        assert!(source_rank(UsageSource::OpenCodeDefault) < source_rank(UsageSource::WeaveCustom));
    }
}
