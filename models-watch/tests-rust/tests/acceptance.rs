use std::sync::atomic::{AtomicU64, Ordering};

use models_watch_tests::{given, given_broadcast, DeltaEntry};

static CAPTURE_COUNTER: AtomicU64 = AtomicU64::new(0);
use serde_json::{json, Map, Value};

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

fn api_fixture_full(
    opencode_go: &[(&str, &str)],
    opencode: Option<&[(&str, &str, Option<(i64, i64)>)]>,
) -> String {
    let og_entries: Vec<String> = opencode_go
        .iter()
        .map(|(id, name)| format!("\"{}\":{{\"id\":\"{}\",\"name\":\"{}\"}}", id, id, name))
        .collect();
    let og_block = format!(
        "{{\"id\":\"opencode-go\",\"name\":\"OpenCode Go\",\"models\":{{{}}}}}",
        og_entries.join(",")
    );

    match opencode {
        Some(models) => {
            let oc_entries: Vec<String> = models
                .iter()
                .map(|(id, name, cost)| {
                    let mut obj = json!({
                        "id": id,
                        "name": name,
                    });
                    if let Some((input, output)) = cost {
                        obj.as_object_mut().unwrap().insert(
                            "cost".to_string(),
                            json!({ "input": input, "output": output }),
                        );
                    }
                    format!("\"{}\":{}", id, obj)
                })
                .collect();
            let oc_block = format!(
                "{{\"id\":\"opencode\",\"name\":\"OpenCode Zen\",\"models\":{{{}}}}}",
                oc_entries.join(",")
            );
            format!("{{\"opencode-go\":{},\"opencode\":{}}}", og_block, oc_block)
        }
        None => {
            format!("{{\"opencode-go\":{}}}", og_block)
        }
    }
}

fn api_fixture(models: &[(&str, &str)]) -> String {
    api_fixture_full(models, Some(&[]))
}

/// Compute the synthetic snapshot that models-watch.sh stores.
fn snapshot_from_fixture(api_json: &str) -> String {
    let v: Value = serde_json::from_str(api_json).unwrap();
    let og_models = v
        .get("opencode-go")
        .and_then(|b| b.get("models"))
        .and_then(|m| m.as_object())
        .cloned()
        .unwrap_or_default();
    let oc_models = v
        .get("opencode")
        .and_then(|b| b.get("models"))
        .and_then(|m| m.as_object())
        .cloned()
        .unwrap_or_default();

    let prefixed_og: Map<String, Value> = og_models
        .into_iter()
        .map(|(id, model)| (format!("opencode-go/{id}"), model))
        .collect();

    let free_oc: Map<String, Value> = oc_models
        .into_iter()
        .filter(|(_, model)| {
            let cost = model.get("cost");
            let input = cost.and_then(|c| c.get("input")).and_then(|v| v.as_i64());
            let output = cost.and_then(|c| c.get("output")).and_then(|v| v.as_i64());
            input == Some(0) && output == Some(0)
        })
        .map(|(id, model)| (format!("opencode/{id}"), model))
        .collect();

    let mut merged = prefixed_og;
    merged.extend(free_oc);
    json!({ "models": merged }).to_string()
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
        .with_prior_snapshot(snapshot_from_fixture(&fixture))
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
        .with_prior_snapshot(snapshot_from_fixture(&prior))
        .when_run()
        .then_result()
        .should_succeed()
        .expect_delta_added(&["opencode-go/model-b"])
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
        .with_prior_snapshot(snapshot_from_fixture(&prior))
        .when_run()
        .then_result()
        .should_succeed()
        .expect_delta_removed(&["opencode-go/model-b"])
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
        .with_prior_snapshot(snapshot_from_fixture(&prior))
        .with_notify_file(notify_path.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_notify_file_contains("opencode-go/new-model");

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
        .with_prior_snapshot(snapshot_from_fixture(&prior))
        .with_notify_file(notify_path.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_delta_changed(&[("opencode-go/model-a", "Model A", "Model A Renamed")])
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
        .with_prior_snapshot(snapshot_from_fixture(&prior))
        .with_notify_file(notify_path.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_delta_added(&["opencode-go/model-b"])
        .expect_delta_changed(&[("opencode-go/model-a", "Model A", "Model A Renamed")])
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
        .with_prior_snapshot(snapshot_from_fixture(&prior))
        .with_notify_file(notify_path.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_notify_file_contains("opencode-go/model-a")
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
        .with_prior_snapshot(snapshot_from_fixture(&prior))
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
fn report_displays_provider_prefixed_entries() {
    let deltas = vec![DeltaEntry {
        timestamp: "2026-04-29T10:00:00Z".to_string(),
        added: vec!["opencode-go/model-a".to_string()],
        removed: vec!["opencode/zen-free".to_string()],
        changed: vec![(
            "opencode/zen-1".to_string(),
            "Zen Old".to_string(),
            "Zen New".to_string(),
        )],
    }];

    given()
        .with_deltas(deltas)
        .with_arg("--report")
        .when_run()
        .then_result()
        .should_succeed()
        .expect_stdout_contains("opencode-go/model-a")
        .expect_stdout_contains("opencode/zen-free")
        .expect_stdout_contains("opencode/zen-1 \"Zen Old\" → \"Zen New\"");
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

// ---------------------------------------------------------------------------
// Zen / opencode provider tests
// ---------------------------------------------------------------------------

#[test]
fn models_watch_should_include_free_zen_models_on_first_run() {
    let fixture = api_fixture_full(
        &[("model-a", "Model A")],
        Some(&[
            ("zen-free", "Zen Free", Some((0, 0))),
            ("zen-paid", "Zen Paid", Some((1, 1))),
        ]),
    );

    given()
        .with_api_fixture(&fixture)
        .when_run()
        .then_result()
        .should_succeed()
        .expect_snapshot_exists()
        .expect_delta_added(&["opencode-go/model-a", "opencode/zen-free"]);
}

#[test]
fn models_watch_should_ignore_paid_zen_models() {
    let fixture = api_fixture_full(
        &[("model-a", "Model A")],
        Some(&[("zen-paid", "Zen Paid", Some((1, 1)))]),
    );

    given()
        .with_api_fixture(&fixture)
        .when_run()
        .then_result()
        .should_succeed()
        .expect_snapshot_exists()
        .expect_delta_added(&["opencode-go/model-a"]);
}

#[test]
fn models_watch_should_report_zen_model_becoming_free_as_added() {
    let prior = api_fixture_full(
        &[("model-a", "Model A")],
        Some(&[("zen-1", "Zen One", Some((1, 1)))]),
    );
    let current = api_fixture_full(
        &[("model-a", "Model A")],
        Some(&[("zen-1", "Zen One", Some((0, 0)))]),
    );

    given()
        .with_api_fixture(&current)
        .with_prior_snapshot(snapshot_from_fixture(&prior))
        .when_run()
        .then_result()
        .should_succeed()
        .expect_delta_added(&["opencode/zen-1"]);
}

#[test]
fn models_watch_should_report_zen_model_ceasing_to_be_free_as_removed() {
    let prior = api_fixture_full(
        &[("model-a", "Model A")],
        Some(&[("zen-1", "Zen One", Some((0, 0)))]),
    );
    let current = api_fixture_full(
        &[("model-a", "Model A")],
        Some(&[("zen-1", "Zen One", Some((1, 1)))]),
    );

    given()
        .with_api_fixture(&current)
        .with_prior_snapshot(snapshot_from_fixture(&prior))
        .when_run()
        .then_result()
        .should_succeed()
        .expect_delta_removed(&["opencode/zen-1"]);
}

#[test]
fn models_watch_should_report_free_zen_name_change_as_changed() {
    let prior = api_fixture_full(
        &[("model-a", "Model A")],
        Some(&[("zen-1", "Zen Old", Some((0, 0)))]),
    );
    let current = api_fixture_full(
        &[("model-a", "Model A")],
        Some(&[("zen-1", "Zen New", Some((0, 0)))]),
    );

    given()
        .with_api_fixture(&current)
        .with_prior_snapshot(snapshot_from_fixture(&prior))
        .when_run()
        .then_result()
        .should_succeed()
        .expect_delta_changed(&[("opencode/zen-1", "Zen Old", "Zen New")]);
}

#[test]
fn models_watch_should_track_same_model_id_per_provider_separately() {
    let fixture = api_fixture_full(
        &[("shared-model", "Go Shared")],
        Some(&[("shared-model", "Zen Shared", Some((0, 0)))]),
    );

    given()
        .with_api_fixture(&fixture)
        .when_run()
        .then_result()
        .should_succeed()
        .expect_delta_added(&["opencode-go/shared-model", "opencode/shared-model"]);
}

#[test]
fn models_watch_should_exit_3_when_opencode_block_missing() {
    let fixture = api_fixture_full(&[("model-a", "Model A")], None);

    given()
        .with_api_fixture(&fixture)
        .when_run()
        .then_result()
        .should_exit_with(3);
}

// ---------------------------------------------------------------------------
// Broadcaster – capture mode
// ---------------------------------------------------------------------------


fn capture_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "models-watch-capture-{}-{}",
        std::process::id(),
        CAPTURE_COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create capture dir");
    dir
}

#[test]
fn broadcast_capture_renders_post_for_added_model() {
    let capture_dir = capture_dir();

    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
  "timestamp": "2026-07-20T10:00:00Z",
  "added": ["opencode-go/new-model"],
  "removed": [],
  "changed": []
}"#)
        .with_capture_dir(capture_dir.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_capture_count(1)
        .expect_capture_contains(1, "New: opencode-go/new-model is now available.")
        .expect_no_ledger();

    let _ = std::fs::remove_dir_all(&capture_dir);
}

#[test]
fn broadcast_capture_renders_removed_changed_added_in_order() {
    let capture_dir = capture_dir();

    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "timestamp": "2026-07-20T10:00:00Z",
          "added": ["opencode-go/model-c"],
          "removed": ["opencode-go/model-a"],
          "changed": [{"id": "opencode-go/model-b", "old_name": "Old B", "new_name": "New B"}]
        }"#)
        .with_capture_dir(capture_dir.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_capture_count(3)
        // Capture files contain JSON-escaped content; check for model IDs and action markers
        .expect_capture_contains(1, "model-a")
        .expect_capture_contains(1, "no longer available")
        .expect_capture_contains(2, "model-b")
        .expect_capture_contains(2, "Old B")
        .expect_capture_contains(2, "New B")
        .expect_capture_contains(3, "model-c")
        .expect_capture_contains(3, "now available")
        .expect_no_ledger();

    let _ = std::fs::remove_dir_all(&capture_dir);
}

#[test]
fn broadcast_capture_sorts_by_model_id_within_each_action_group() {
    let capture_dir = capture_dir();

    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "timestamp": "2026-07-20T10:00:00Z",
          "added": ["opencode-go/model-z", "opencode-go/model-a"],
          "removed": [],
          "changed": []
        }"#)
        .with_capture_dir(capture_dir.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_capture_count(2)
        .expect_capture_contains(1, "opencode-go/model-a")
        .expect_capture_contains(2, "opencode-go/model-z")
        .expect_no_ledger();

    let _ = std::fs::remove_dir_all(&capture_dir);
}

#[test]
fn broadcast_capture_unknown_flag_exits_2() {
    given_broadcast()
        .with_arg("--bogus")
        .when_run()
        .then_result()
        .should_exit_with(2)
        .expect_stdout_does_not_contain("capture"); // stderr check via stderr_contains perhaps
}

#[test]
fn broadcast_capture_no_eligible_deltas_exits_3() {
    let capture_dir = capture_dir();

    given_broadcast()
        .with_capture_dir(capture_dir.clone())
        .when_run()
        .then_result()
        .should_exit_with(3);

    let _ = std::fs::remove_dir_all(&capture_dir);
}

#[test]
fn broadcast_capture_skips_ledgered_deltas() {
    let capture_dir = capture_dir();

    let ledger = r#"{"deltas": {"change-2026-07-20T10:00:00Z.json": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}"#;

    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "timestamp": "2026-07-20T10:00:00Z",
          "added": ["opencode-go/already-posted"],
          "removed": [],
          "changed": []
        }"#)
        .with_state_ledger(ledger)
        .with_capture_dir(capture_dir.clone())
        .when_run()
        .then_result()
        .should_exit_with(3);

    let _ = std::fs::remove_dir_all(&capture_dir);
}

#[test]
fn broadcast_capture_two_deltas_process_both() {
    let capture_dir = capture_dir();

    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "timestamp": "2026-07-20T10:00:00Z",
          "added": ["opencode-go/first"],
          "removed": [],
          "changed": []
        }"#)
        .with_state_delta("change-2026-07-21T10:00:00Z.json", r#"{
          "timestamp": "2026-07-21T10:00:00Z",
          "added": ["opencode-go/second"],
          "removed": [],
          "changed": []
        }"#)
        .with_capture_dir(capture_dir.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_capture_count(2)
        .expect_capture_contains(1, "opencode-go/first")
        .expect_capture_contains(2, "opencode-go/second");

    let _ = std::fs::remove_dir_all(&capture_dir);
}

#[test]
fn broadcast_capture_processes_oldest_first() {
    let capture_dir = capture_dir();

    given_broadcast()
        .with_state_delta("change-2026-07-21T10:00:00Z.json", r#"{
          "timestamp": "2026-07-21T10:00:00Z",
          "added": ["opencode-go/second"],
          "removed": [],
          "changed": []
        }"#)
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "timestamp": "2026-07-20T10:00:00Z",
          "added": ["opencode-go/first"],
          "removed": [],
          "changed": []
        }"#)
        .with_capture_dir(capture_dir.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_capture_count(2)
        .expect_capture_contains(1, "opencode-go/first")
        .expect_capture_contains(2, "opencode-go/second");

    let _ = std::fs::remove_dir_all(&capture_dir);
}

// ---------------------------------------------------------------------------
// Broadcaster – flag validation
// ---------------------------------------------------------------------------

#[test]
fn broadcast_rejects_limit_with_capture_dir() {
    given_broadcast()
        .with_arg("--capture-dir")
        .with_arg("/tmp/foo")
        .with_arg("--limit")
        .with_arg("5")
        .when_run()
        .then_result()
        .should_exit_with(2)
        .expect_stderr_contains("mutually exclusive");
}

#[test]
fn broadcast_rejects_invalid_limit_value() {
    given_broadcast()
        .with_arg("--limit")
        .with_arg("foo")
        .when_run()
        .then_result()
        .should_exit_with(2)
        .expect_stderr_contains("positive integer");
}

#[test]
fn broadcast_rejects_missing_state_dir_value() {
    given_broadcast()
        .with_arg("--state-dir")
        .with_arg("--capture-dir")
        .with_arg("/tmp/x")
        .when_run()
        .then_result()
        .should_exit_with(2)
        .expect_stderr_contains("requires a value");
}

// ---------------------------------------------------------------------------
// Broadcaster – validation
// ---------------------------------------------------------------------------

#[test]
fn broadcast_rejects_delta_with_non_provider_id() {
    let capture_dir = capture_dir();

    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "timestamp": "2026-07-20T10:00:00Z",
          "added": ["no-prefix-model"],
          "removed": [],
          "changed": []
        }"#)
        .with_capture_dir(capture_dir.clone())
        .when_run()
        .then_result()
        .should_exit_with(1)
        .expect_stderr_contains("non-provider-prefixed");

    let _ = std::fs::remove_dir_all(&capture_dir);
}

#[test]
fn broadcast_rejects_delta_missing_timestamp() {
    let capture_dir = capture_dir();

    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "added": ["opencode-go/model"],
          "removed": [],
          "changed": []
        }"#)
        .with_capture_dir(capture_dir.clone())
        .when_run()
        .then_result()
        .should_exit_with(1)
        .expect_stderr_contains("missing string timestamp");

    let _ = std::fs::remove_dir_all(&capture_dir);
}

#[test]
fn broadcast_rejects_delta_malformed_changed_entry() {
    let capture_dir = capture_dir();

    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "timestamp": "2026-07-20T10:00:00Z",
          "added": [],
          "removed": [],
          "changed": [{"id": "opencode-go/model", "old_name": 123, "new_name": "New"}]
        }"#)
        .with_capture_dir(capture_dir.clone())
        .when_run()
        .then_result()
        .should_exit_with(1)
        .expect_stderr_contains("malformed changed");

    let _ = std::fs::remove_dir_all(&capture_dir);
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Broadcaster – live posting with file:// PDS transport
// ---------------------------------------------------------------------------

/// Write a fixture envelope for a `file://` PDS endpoint call.
fn write_pds_fixture(pds_root: &std::path::Path, endpoint: &str, number: usize, status: u16, body: &serde_json::Value) {
    let dir = pds_root.join("xrpc").join(endpoint);
    std::fs::create_dir_all(&dir).expect("create PDS fixture dir");
    let envelope = serde_json::json!({
        "status": status,
        "body": body
    });
    std::fs::write(dir.join(format!("{}.json", number)), envelope.to_string())
        .expect("write PDS fixture");
}

#[test]
fn broadcast_posts_via_file_pds_transport() {
    let capture_dir = capture_dir();
    let pds_root = capture_dir.join("pds-root");
    std::fs::create_dir_all(&pds_root).expect("create pds root");

    // Fixture: session creation succeeds
    write_pds_fixture(
        &pds_root,
        "com.atproto.server.createSession",
        1,
        200,
        &serde_json::json!({
            "accessJwt": "test-jwt",
            "did": "did:plc:testdid"
        }),
    );

    // Fixture: record creation succeeds
    write_pds_fixture(
        &pds_root,
        "com.atproto.repo.createRecord",
        1,
        200,
        &serde_json::json!({
            "uri": "at://did:plc:testdid/app.bsky.feed.post/3test",
            "cid": "bafyreibtest"
        }),
    );

    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "timestamp": "2026-07-20T10:00:00Z",
          "added": ["opencode-go/new-model"],
          "removed": [],
          "changed": []
        }"#)
        .with_env("BLUESKY_PDS", format!("file://{}", pds_root.display()))
        .with_env("BLUESKY_HANDLE", "test.bsky.social")
        .with_env("BLUESKY_APP_PASSWORD", "test-pass")
        .when_run()
        .then_result()
        .should_succeed()
        .expect_ledger_entry(
            "change-2026-07-20T10:00:00Z.json",
            "4c699019775537375bfdea9f3d8cdb7a064fdb4d898b428d19824aedb6352717",
        );

    let _ = std::fs::remove_dir_all(&capture_dir);
}

#[test]
fn broadcast_does_not_ledger_rejected_record_response() {
    let capture_dir = capture_dir();
    let pds_root = capture_dir.join("pds-root");
    std::fs::create_dir_all(&pds_root).expect("create pds root");

    write_pds_fixture(
        &pds_root,
        "com.atproto.server.createSession",
        1,
        200,
        &serde_json::json!({"accessJwt": "test-jwt", "did": "did:plc:testdid"}),
    );
    write_pds_fixture(
        &pds_root,
        "com.atproto.repo.createRecord",
        1,
        401,
        &serde_json::json!({"error": "AuthRequired", "message": "expired token"}),
    );

    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "timestamp": "2026-07-20T10:00:00Z",
          "added": ["opencode-go/new-model"], "removed": [], "changed": []
        }"#)
        .with_env("BLUESKY_PDS", format!("file://{}", pds_root.display()))
        .with_env("BLUESKY_HANDLE", "test.bsky.social")
        .with_env("BLUESKY_APP_PASSWORD", "test-pass")
        .when_run()
        .then_result()
        .should_exit_with(1)
        .expect_stderr_contains("createRecord failed")
        .expect_no_ledger();

    let _ = std::fs::remove_dir_all(&capture_dir);
}

#[test]
fn broadcast_persists_completed_delta_before_later_failure() {
    let capture_dir = capture_dir();
    let pds_root = capture_dir.join("pds-root");
    std::fs::create_dir_all(&pds_root).expect("create pds root");

    write_pds_fixture(
        &pds_root,
        "com.atproto.server.createSession",
        1,
        200,
        &serde_json::json!({"accessJwt": "test-jwt", "did": "did:plc:testdid"}),
    );
    write_pds_fixture(
        &pds_root,
        "com.atproto.repo.createRecord",
        1,
        200,
        &serde_json::json!({"uri": "at://first", "cid": "first"}),
    );
    write_pds_fixture(
        &pds_root,
        "com.atproto.repo.createRecord",
        2,
        500,
        &serde_json::json!({"error": "InternalServerError"}),
    );

    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "timestamp": "2026-07-20T10:00:00Z",
          "added": ["opencode-go/first"], "removed": [], "changed": []
        }"#)
        .with_state_delta("change-2026-07-20T11:00:00Z.json", r#"{
          "timestamp": "2026-07-20T11:00:00Z",
          "added": ["opencode-go/second"], "removed": [], "changed": []
        }"#)
        .with_env("BLUESKY_PDS", format!("file://{}", pds_root.display()))
        .with_env("BLUESKY_HANDLE", "test.bsky.social")
        .with_env("BLUESKY_APP_PASSWORD", "test-pass")
        .when_run()
        .then_result()
        .should_exit_with(1)
        .expect_ledger_has_entry("change-2026-07-20T10:00:00Z.json");

    let _ = std::fs::remove_dir_all(&capture_dir);
}

#[test]
fn broadcast_validates_all_selected_deltas_before_creating_a_session() {
    let capture_dir = capture_dir();
    let pds_root = capture_dir.join("empty-pds-root");
    std::fs::create_dir_all(&pds_root).expect("create PDS root");

    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "timestamp": "2026-07-20T10:00:00Z",
          "added": ["opencode-go/valid"], "removed": [], "changed": []
        }"#)
        .with_state_delta("change-2026-07-20T11:00:00Z.json", r#"{
          "timestamp": "2026-07-20T11:00:00Z",
          "added": ["not-provider-prefixed"], "removed": [], "changed": []
        }"#)
        .with_env("BLUESKY_PDS", format!("file://{}", pds_root.display()))
        .with_env("BLUESKY_HANDLE", "test.bsky.social")
        .with_env("BLUESKY_APP_PASSWORD", "test-pass")
        .when_run()
        .then_result()
        .should_exit_with(1)
        .expect_stderr_contains("invalid delta change-2026-07-20T11:00:00Z.json")
        .expect_no_ledger();

    let _ = std::fs::remove_dir_all(&capture_dir);
}

#[test]
fn broadcast_missing_credentials_exits_4() {
    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "timestamp": "2026-07-20T10:00:00Z",
          "added": ["opencode-go/new-model"],
          "removed": [],
          "changed": []
        }"#)
        // No BLUESKY_HANDLE or BLUESKY_APP_PASSWORD set
        .when_run()
        .then_result()
        .should_exit_with(4)
        .expect_stderr_contains("BLUESKY_HANDLE");
}

// ---------------------------------------------------------------------------
// Broadcaster – text truncation
// ---------------------------------------------------------------------------

fn truncation_delta_for_added(long_id: &str) -> String {
    format!(
        r#"{{"timestamp":"2026-07-20T10:00:00Z","added":["{}"],"removed":[],"changed":[]}}"#,
        long_id
    )
}

fn truncation_delta_for_changed(old_name: &str, new_name: &str) -> String {
    format!(
        r#"{{"timestamp":"2026-07-20T10:00:00Z","added":[],"removed":[],"changed":[{{"id":"opencode-go/m","old_name":"{}","new_name":"{}"}}]}}"#,
        old_name, new_name
    )
}

fn check_capture_text(capture_dir: &std::path::Path) -> String {
    let capture = capture_dir.join("1.json");
    let raw = std::fs::read_to_string(&capture).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    v["text"].as_str().unwrap().to_string()
}

#[test]
fn broadcast_truncates_long_model_id_in_new_post() {
    let capture_dir = capture_dir();
    let long_id = "opencode-go/".to_string() + &"x".repeat(285);

    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", truncation_delta_for_added(&long_id))
        .with_capture_dir(capture_dir.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_capture_count(1)
        .expect_stderr_contains("original")
        .expect_stderr_contains("final");

    let text = check_capture_text(&capture_dir);
    assert!(
        text.chars().count() <= 300,
        "should be <= 300 cp, got {}: {:?}",
        text.chars().count(),
        text
    );
    assert!(text.contains('…'), "should contain ellipsis: {:?}", text);
    let _ = std::fs::remove_dir_all(&capture_dir);
}

#[test]
fn broadcast_truncates_long_old_name_in_updated_post() {
    let capture_dir = capture_dir();
    let long_old = "X".repeat(275);

    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", truncation_delta_for_changed(&long_old, "N"))
        .with_capture_dir(capture_dir.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_capture_count(1)
        .expect_stderr_contains("original")
        .expect_stderr_contains("final");

    let text = check_capture_text(&capture_dir);
    assert!(
        text.chars().count() <= 300,
        "should be <= 300 cp, got {}: {:?}",
        text.chars().count(),
        text
    );
    assert!(text.contains('…'), "should contain ellipsis: {:?}", text);
    let _ = std::fs::remove_dir_all(&capture_dir);
}

#[test]
fn broadcast_truncates_long_new_name_in_updated_post() {
    let capture_dir = capture_dir();
    let long_new = "Y".repeat(280);

    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", truncation_delta_for_changed("O", &long_new))
        .with_capture_dir(capture_dir.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_capture_count(1)
        .expect_stderr_contains("original")
        .expect_stderr_contains("final");

    let text = check_capture_text(&capture_dir);
    assert!(
        text.chars().count() <= 300,
        "should be <= 300 cp, got {}: {:?}",
        text.chars().count(),
        text
    );
    assert!(text.contains('…'), "should contain ellipsis: {:?}", text);
    let _ = std::fs::remove_dir_all(&capture_dir);
}

// ---------------------------------------------------------------------------
// Broadcaster – ledger validation
// ---------------------------------------------------------------------------

#[test]
fn broadcast_rejects_malformed_ledger_overlapping_keys() {
    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "timestamp": "2026-07-20T10:00:00Z",
          "added": ["opencode-go/new-model"],
          "removed": [],
          "changed": []
        }"#)
        .with_state_ledger(r#"{"deltas":{"change-2026-07-20T10:00:00Z.json":"abc"},"skipped":{"change-2026-07-20T10:00:00Z.json":"reason"}}"#)
        .when_run()
        .then_result()
        .should_exit_with(1)
        .expect_stderr_contains("overlapping");
}

#[test]
fn broadcast_rejects_malformed_ledger_wrong_type() {
    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "timestamp": "2026-07-20T10:00:00Z",
          "added": ["opencode-go/new-model"],
          "removed": [],
          "changed": []
        }"#)
        .with_state_ledger(r#""not an object""#)
        .when_run()
        .then_result()
        .should_exit_with(1)
        .expect_stderr_contains("invalid shape");
}

#[test]
fn broadcast_rejects_malformed_ledger_with_non_string_skipped_value() {
    given_broadcast()
        .with_state_delta("change-2026-07-20T10:00:00Z.json", r#"{
          "timestamp": "2026-07-20T10:00:00Z",
          "added": ["opencode-go/new-model"],
          "removed": [],
          "changed": []
        }"#)
        .with_state_ledger(r#"{"skipped":{"change-2026-07-20T10:00:00Z.json":false}}"#)
        .with_capture_dir(capture_dir())
        .when_run()
        .then_result()
        .should_exit_with(1)
        .expect_stderr_contains("invalid shape");
}
