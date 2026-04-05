mod support;

use opencode_model_report::{build_rows, ReportInput, SortMode};
use support::given_model_report;
use support::scenario::fail;

#[test]
fn startup_should_load_rows_and_mark_ready() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![(
                "provider/alpha".to_string(),
                vec![opencode_model_report::UsageLabel {
                    label: "default".to_string(),
                    source: opencode_model_report::UsageSource::OpenCodeDefault,
                }],
            )],
            available_models: vec!["provider/alpha".to_string()],
            costs: vec![("provider/alpha".to_string(), Some(1.0), Some(2.0))],
        },
        SortMode::ActiveFirst,
    );

    given_model_report()
        .with_startup_rows(rows)
        .when_started()
        .then_state()
        .shows_ready()
        .shows_status_contains("Loaded")
        .shows_models_in_order(&["provider/alpha"]);
}

#[test]
fn startup_should_surface_error_and_exit_code_when_load_fails() {
    given_model_report()
        .with_startup_failure("missing config file")
        .when_started()
        .then_exit()
        .exits_with_code(3)
        .stderr_contains("missing config");
}

#[test]
fn startup_should_show_cli_help() {
    let exe = env!("CARGO_BIN_EXE_opencode-model-report");
    let output = match std::process::Command::new(exe).arg("--help").output() {
        Ok(value) => value,
        Err(err) => fail(format!("failed to run --help: {err}")),
    };

    if !output.status.success() {
        fail("--help should succeed");
    }

    let help_text = String::from_utf8_lossy(&output.stdout);
    if !help_text.contains("opencode-model-report") {
        fail("--help should mention program name");
    }
    if !help_text.contains("--home-dir") {
        fail("--help should include --home-dir");
    }
}
