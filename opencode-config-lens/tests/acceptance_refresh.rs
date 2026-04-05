mod support;

use opencode_config_lens::{build_rows, ReportInput, SortMode, UsageLabel, UsageSource};
use support::given_model_report;

fn rows_for(model: &str, in_cost: f64, out_cost: f64) -> Vec<opencode_config_lens::ModelRow> {
    build_rows(
        ReportInput {
            active_usage: vec![(
                model.to_string(),
                vec![UsageLabel {
                    label: "default".to_string(),
                    source: UsageSource::OpenCodeDefault,
                }],
            )],
            available_models: vec![model.to_string()],
            costs: vec![(model.to_string(), Some(in_cost), Some(out_cost))],
        },
        SortMode::ActiveFirst,
    )
}

#[test]
fn refresh_should_update_rows_and_effect_counters_on_success() {
    let initial_rows = rows_for("provider/alpha", 1.0, 2.0);
    let refreshed_rows = rows_for("provider/beta", 3.0, 4.0);

    given_model_report()
        .with_startup_rows(initial_rows)
        .with_refresh_rows(refreshed_rows)
        .when_started()
        .when_refresh_pressed(Ok(rows_for("provider/beta", 3.0, 4.0)))
        .then_state()
        .shows_ready()
        .shows_models_in_order(&["provider/beta"])
        .shows_status_contains("Loaded");
}

#[test]
fn refresh_should_keep_previous_snapshot_on_failure() {
    given_model_report()
        .with_startup_rows(rows_for("provider/alpha", 1.0, 2.0))
        .when_started()
        .when_refresh_pressed(Err("refresh failed: network".to_string()))
        .then_effects()
        .keeps_previous_snapshot()
        .ran_opencode_refresh(2)
        .fetched_costs(2);
}

#[test]
fn refresh_should_show_failure_status_when_refresh_fails() {
    given_model_report()
        .with_startup_rows(rows_for("provider/alpha", 1.0, 2.0))
        .when_started()
        .when_refresh_pressed(Err("refresh failed: timeout".to_string()))
        .then_state()
        .shows_ready()
        .shows_status_contains("failed")
        .shows_models_in_order(&["provider/alpha"]);
}

#[test]
fn refresh_should_record_startup_effects() {
    given_model_report()
        .with_startup_rows(rows_for("provider/alpha", 1.0, 2.0))
        .when_started()
        .then_effects()
        .ran_opencode_refresh(1)
        .fetched_costs(1);
}
