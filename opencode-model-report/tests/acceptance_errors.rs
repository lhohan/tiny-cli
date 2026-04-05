mod support;

use opencode_model_report::{LoadError, ModelRow};
use support::given_model_report;
use support::scenario::fail;

#[test]
fn errors_should_map_refresh_failure_exit_code_from_subprocess() {
    let err = LoadError::RefreshFailed {
        stderr: "boom".to_string(),
        code: 17,
    };
    if err.exit_code() != 17 {
        fail("refresh failure should preserve subprocess exit code");
    }
}

#[test]
fn errors_should_map_non_refresh_errors_to_exit_code_3() {
    let err = LoadError::FetchFailed("network".to_string());
    if err.exit_code() != 3 {
        fail("non-refresh failures should map to exit code 3");
    }
}

#[test]
fn errors_should_surface_startup_failure_in_exit_phase() {
    given_model_report()
        .with_startup_failure("missing config file")
        .when_started()
        .then_exit()
        .exits_with_code(3)
        .stderr_contains("missing config");
}

#[test]
fn errors_should_keep_snapshot_when_refresh_fails() {
    given_model_report()
        .with_startup_rows(vec![ModelRow {
            model: "provider/alpha".to_string(),
            provider: "provider".to_string(),
            model_name: "alpha".to_string(),
            active: true,
            input_cost: Some(1.0),
            output_cost: Some(2.0),
            usage: vec![],
        }])
        .when_started()
        .when_refresh_pressed(Err("refresh failed: timeout".to_string()))
        .then_effects()
        .keeps_previous_snapshot();
}

#[test]
fn errors_should_include_refresh_failure_text() {
    let message = format!(
        "{}",
        LoadError::RefreshFailed {
            stderr: "broken pipe".to_string(),
            code: 4,
        }
    );
    if !message.contains("failed to refresh OpenCode models") {
        fail("refresh failure display text should be actionable");
    }
    if !message.contains("broken pipe") {
        fail("refresh failure display text should include stderr");
    }
}

#[test]
fn errors_should_show_help_for_user_recovery() {
    let exe = env!("CARGO_BIN_EXE_opencode-model-report");
    let output = match std::process::Command::new(exe).arg("--help").output() {
        Ok(value) => value,
        Err(err) => fail(format!("failed to run --help: {err}")),
    };

    if !output.status.success() {
        fail("--help should succeed");
    }

    let help = String::from_utf8_lossy(&output.stdout);
    if !help.contains("--home-dir") {
        fail("--help should include --home-dir option");
    }
}
