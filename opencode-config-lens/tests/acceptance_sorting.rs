mod support;

use opencode_config_lens::{build_rows, ReportInput, SortMode, UsageLabel, UsageSource};
use support::given_model_report;

fn sort_fixture_rows() -> Vec<opencode_config_lens::ModelRow> {
    build_rows(
        ReportInput {
            active_usage: vec![(
                "p/active".to_string(),
                vec![UsageLabel {
                    label: "default".to_string(),
                    source: UsageSource::OpenCodeDefault,
                }],
            )],
            available_models: vec![
                "p/unknown".to_string(),
                "p/expensive".to_string(),
                "p/cheap".to_string(),
                "p/active".to_string(),
            ],
            costs: vec![
                ("p/expensive".to_string(), Some(10.0), Some(10.0)),
                ("p/cheap".to_string(), Some(1.0), Some(1.0)),
                ("p/active".to_string(), Some(2.0), Some(2.0)),
                ("p/unknown".to_string(), None, None),
            ],
        },
        SortMode::ActiveFirst,
    )
}

#[test]
fn sorting_should_cycle_modes_via_s_key() {
    given_model_report()
        .with_startup_rows(sort_fixture_rows())
        .when_started()
        .then_state()
        .shows_sort_mode(SortMode::ActiveFirst);

    given_model_report()
        .with_startup_rows(sort_fixture_rows())
        .when_started()
        .when_sort_pressed()
        .then_state()
        .shows_sort_mode(SortMode::CostAsc);

    given_model_report()
        .with_startup_rows(sort_fixture_rows())
        .when_started()
        .when_sort_pressed()
        .when_sort_pressed()
        .then_state()
        .shows_sort_mode(SortMode::CostDesc);
}

#[test]
fn sorting_should_place_unknown_costs_last_for_cost_asc() {
    given_model_report()
        .with_startup_rows(sort_fixture_rows())
        .when_started()
        .when_sort_pressed()
        .then_state()
        .shows_models_in_order(&["p/cheap", "p/active", "p/expensive", "p/unknown"]);
}

#[test]
fn sorting_should_place_unknown_costs_last_for_cost_desc() {
    given_model_report()
        .with_startup_rows(sort_fixture_rows())
        .when_started()
        .when_sort_pressed()
        .when_sort_pressed()
        .then_state()
        .shows_models_in_order(&["p/expensive", "p/active", "p/cheap", "p/unknown"]);
}

#[test]
fn sorting_should_be_alphabetical_in_model_name_mode() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![],
            available_models: vec![
                "p/zebra".to_string(),
                "p/alpha".to_string(),
                "p/middle".to_string(),
            ],
            costs: vec![],
        },
        SortMode::ActiveFirst,
    );

    given_model_report()
        .with_startup_rows(rows)
        .when_started()
        .when_sort_pressed()
        .when_sort_pressed()
        .when_sort_pressed()
        .then_state()
        .shows_sort_mode(SortMode::ModelName)
        .shows_models_in_order(&["p/alpha", "p/middle", "p/zebra"]);
}

#[test]
fn sorting_should_remain_deterministic_with_equal_costs() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![],
            available_models: vec!["p/model-b".to_string(), "p/model-a".to_string()],
            costs: vec![
                ("p/model-b".to_string(), Some(1.0), Some(1.0)),
                ("p/model-a".to_string(), Some(1.0), Some(1.0)),
            ],
        },
        SortMode::ActiveFirst,
    );

    given_model_report()
        .with_startup_rows(rows)
        .when_started()
        .when_sort_pressed()
        .then_state()
        .shows_models_in_order(&["p/model-a", "p/model-b"]);
}
