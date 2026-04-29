use models_watch_tests::given;

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
