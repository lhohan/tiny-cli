use models_watch_tests::{given, DeltaEntry};

/// Build a minimal api.json fixture containing only `opencode-go` models.
fn api_fixture(models: &[(&str, &str)]) -> String {
    let model_entries: Vec<String> = models
        .iter()
        .map(|(id, name)| {
            format!(
                "\"{}\":{{\"id\":\"{}\",\"name\":\"{}\"}}",
                id, id, name
            )
        })
        .collect();
    let models_block = model_entries.join(",");
    let open = "{\"opencode-go\":{\"id\":\"opencode-go\",\"name\":\"OpenCode Go\",\"models\":{";
    let close = "}}}";
    format!("{}{}{}", open, models_block, close)
}

/// Extract the opencode-go provider block from a raw api.json fixture.
/// This matches what models-watch.sh stores in state/latest.json.
fn opencode_go_block(api_json: &str) -> String {
    // The fixture is: {"opencode-go":{...}}
    // Extract the inner { ... } block.
    let prefix = "{\"opencode-go\":";
    let stripped = api_json.strip_prefix(prefix).expect("fixture has opencode-go key");
    // Strip trailing }
    let inner = stripped.strip_suffix("}").expect("fixture has closing brace");
    inner.to_string()
}

// ---------------------------------------------------------------------------
// Walking skeleton: first run with no prior state
// ---------------------------------------------------------------------------

#[test]
fn models_watch_should_write_first_snapshot_when_no_prior_state() {
    let fixture = api_fixture(&[("model-a", "Model A"), ("model-b", "Model B")]);

    given()
        .with_api_fixture(&fixture)
        .when_run()
        .then_result()
        .should_succeed()
        .expect_snapshot_exists()
        .expect_delta_file();
}

// ---------------------------------------------------------------------------
// No change detected
// ---------------------------------------------------------------------------

#[test]
fn models_watch_should_write_no_delta_when_no_change() {
    let fixture = api_fixture(&[("model-a", "Model A")]);

    given()
        .with_api_fixture(&fixture)
        .with_prior_snapshot(opencode_go_block(&fixture)) // same snapshot, extracted
        .when_run()
        .then_result()
        .should_succeed()
        .expect_no_delta_file();
}

// ---------------------------------------------------------------------------
// Models added
// ---------------------------------------------------------------------------

#[test]
fn models_watch_should_write_delta_when_models_added() {
    let prior = api_fixture(&[("model-a", "Model A")]);
    let current = api_fixture(&[("model-a", "Model A"), ("model-b", "Model B")]);

    given()
        .with_api_fixture(&current)
        .with_prior_snapshot(opencode_go_block(&prior))
        .when_run()
        .then_result()
        .should_succeed()
        .expect_delta_added(&["model-b"])
        .expect_snapshot_exists();
}

// ---------------------------------------------------------------------------
// Models removed
// ---------------------------------------------------------------------------

#[test]
fn models_watch_should_write_delta_when_models_removed() {
    let prior = api_fixture(&[("model-a", "Model A"), ("model-b", "Model B")]);
    let current = api_fixture(&[("model-a", "Model A")]);

    given()
        .with_api_fixture(&current)
        .with_prior_snapshot(opencode_go_block(&prior))
        .when_run()
        .then_result()
        .should_succeed()
        .expect_delta_removed(&["model-b"])
        .expect_snapshot_exists();
}

// ---------------------------------------------------------------------------
// Notification via --notify-file
// ---------------------------------------------------------------------------

#[test]
fn models_watch_should_notify_via_notify_file_when_change_detected() {
    let prior = api_fixture(&[("model-a", "Model A")]);
    let current = api_fixture(&[("model-a", "Model A"), ("new-model", "New Model")]);

    let notify_path = std::env::temp_dir().join(format!(
        "models-watch-notify-{}.txt",
        std::process::id()
    ));

    // Clean up from prior runs
    let _ = std::fs::remove_file(&notify_path);

    given()
        .with_api_fixture(&current)
        .with_prior_snapshot(opencode_go_block(&prior))
        .with_notify_file(notify_path.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_notify_file_contains("new-model");

    // Clean up
    let _ = std::fs::remove_file(&notify_path);
}

// ---------------------------------------------------------------------------
// osascript path (no --notify-file)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Model name/description changes
// ---------------------------------------------------------------------------

#[test]
fn models_watch_should_write_delta_when_model_name_changes() {
    let prior = api_fixture(&[("model-a", "Model A")]);
    let current = api_fixture(&[("model-a", "Model A Renamed")]);

    let notify_path = std::env::temp_dir().join(format!(
        "models-watch-notify-{}-name-change.txt",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&notify_path);

    given()
        .with_api_fixture(&current)
        .with_prior_snapshot(opencode_go_block(&prior))
        .with_notify_file(notify_path.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_delta_changed(&[("model-a", "Model A", "Model A Renamed")])
        .expect_snapshot_exists();

    let _ = std::fs::remove_file(&notify_path);
}

#[test]
fn models_watch_should_report_added_and_changed_together() {
    let prior = api_fixture(&[("model-a", "Model A")]);
    let current = api_fixture(&[("model-a", "Model A Renamed"), ("model-b", "Model B")]);

    let notify_path = std::env::temp_dir().join(format!(
        "models-watch-notify-{}-add-and-change.txt",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&notify_path);

    given()
        .with_api_fixture(&current)
        .with_prior_snapshot(opencode_go_block(&prior))
        .with_notify_file(notify_path.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_delta_added(&["model-b"])
        .expect_delta_changed(&[("model-a", "Model A", "Model A Renamed")])
        .expect_snapshot_exists();

    let _ = std::fs::remove_file(&notify_path);
}

#[test]
fn models_watch_should_notify_changed_models_via_notify_file() {
    let prior = api_fixture(&[("model-a", "Model A")]);
    let current = api_fixture(&[("model-a", "Model A Renamed")]);

    let notify_path = std::env::temp_dir().join(format!(
        "models-watch-notify-{}-changed.txt",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&notify_path);

    given()
        .with_api_fixture(&current)
        .with_prior_snapshot(opencode_go_block(&prior))
        .with_notify_file(notify_path.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_notify_file_contains("Model A")
        .expect_notify_file_contains("Model A Renamed");

    let _ = std::fs::remove_file(&notify_path);
}

// ---------------------------------------------------------------------------
// osascript path (no --notify-file)
// ---------------------------------------------------------------------------

#[test]
fn models_watch_should_complete_without_notify_file_flag() {
    let prior = api_fixture(&[("model-a", "Model A")]);
    let current = api_fixture(&[("model-a", "Model A"), ("model-b", "Model B")]);

    given()
        .with_api_fixture(&current)
        .with_prior_snapshot(opencode_go_block(&prior))
        .when_run()
        .then_result()
        .should_succeed();
}

// ---------------------------------------------------------------------------
// --report flag
// ---------------------------------------------------------------------------

#[test]
fn report_prints_no_changes_message_when_no_deltas() {
    given()
        .with_arg("--report")
        .when_run()
        .then_result()
        .should_succeed()
        .expect_stdout_contains("No changes recorded yet.");
}

#[test]
fn report_shows_all_deltas_when_fewer_than_ten() {
    let deltas = vec![
        DeltaEntry {
            timestamp: "2026-04-29T10:00:00Z".to_string(),
            added: vec!["alpha".to_string()],
            removed: vec![],
            changed: vec![],
        },
        DeltaEntry {
            timestamp: "2026-04-29T11:00:00Z".to_string(),
            added: vec!["bravo".to_string()],
            removed: vec![],
            changed: vec![],
        },
        DeltaEntry {
            timestamp: "2026-04-29T12:00:00Z".to_string(),
            added: vec!["charlie".to_string()],
            removed: vec![],
            changed: vec![],
        },
    ];

    given()
        .with_deltas(deltas)
        .with_arg("--report")
        .when_run()
        .then_result()
        .should_succeed()
        .expect_stdout_contains("alpha")
        .expect_stdout_contains("bravo")
        .expect_stdout_contains("charlie");
}

#[test]
fn report_shows_last_10_when_more_than_ten() {
    let deltas: Vec<DeltaEntry> = (0..12)
        .map(|i| {
            let ts = format!("2026-04-{:02}T{:02}:00:00Z", 29, i);
            let model = format!("model-{:02}", i);
            DeltaEntry {
                timestamp: ts,
                added: vec![model.clone()],
                removed: vec![],
                changed: vec![],
            }
        })
        .collect();

    let result = given()
        .with_deltas(deltas)
        .with_arg("--report")
        .when_run()
        .then_result();
    result.should_succeed();

    // First 2 should NOT appear (indices 00-01)
    result
        .expect_stdout_does_not_contain("model-00")
        .expect_stdout_does_not_contain("model-01");

    // Last 10 SHOULD appear (indices 02-11)
    for i in 2..=11 {
        let model = format!("model-{:02}", i);
        result.expect_stdout_contains(&model);
    }
}

#[test]
fn report_shows_deltas_in_chronological_order() {
    // Deltas in reverse order; they should print oldest first.
    let deltas = vec![
        DeltaEntry {
            timestamp: "2026-04-29T12:00:00Z".to_string(),
            added: vec!["third".to_string()],
            removed: vec![],
            changed: vec![],
        },
        DeltaEntry {
            timestamp: "2026-04-29T10:00:00Z".to_string(),
            added: vec!["first".to_string()],
            removed: vec![],
            changed: vec![],
        },
        DeltaEntry {
            timestamp: "2026-04-29T11:00:00Z".to_string(),
            added: vec!["second".to_string()],
            removed: vec![],
            changed: vec![],
        },
    ];

    let result = given()
        .with_deltas(deltas)
        .with_arg("--report")
        .when_run()
        .then_result();
    result.should_succeed();

    // Assert order by checking positions via the getter
    let output = result.stdout();
    let first_pos = output.find("first").unwrap_or(usize::MAX);
    let second_pos = output.find("second").unwrap_or(usize::MAX);
    let third_pos = output.find("third").unwrap_or(usize::MAX);

    assert!(
        first_pos < second_pos,
        "'first' should appear before 'second'"
    );
    assert!(
        second_pos < third_pos,
        "'second' should appear before 'third'"
    );
}

#[test]
fn report_does_not_fetch_api() {
    // Runs --report with deltas but NO MODELS_WATCH_API_URL set.
    // The script uses its default URL (which requires network),
    // but --report exits before any fetch attempt.
    let deltas = vec![DeltaEntry {
        timestamp: "2026-04-29T10:00:00Z".to_string(),
        added: vec!["offline-model".to_string()],
        removed: vec![],
        changed: vec![],
    }];

    given()
        .with_deltas(deltas)
        .with_arg("--report")
        .without_api_env()
        .when_run()
        .then_result()
        .should_succeed()
        .expect_stdout_contains("offline-model");
}
