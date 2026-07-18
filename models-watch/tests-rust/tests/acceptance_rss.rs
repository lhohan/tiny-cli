use models_watch_tests::{given_feed, DeltaEntry};

// ---------------------------------------------------------------------------
// Walking skeleton: feed should write RSS when deltas exist
// ---------------------------------------------------------------------------

#[test]
fn feed_should_write_rss_when_deltas_exist() {
    let deltas = vec![DeltaEntry {
        timestamp: "2026-04-29T10:00:00Z".to_string(),
        added: vec!["opencode-go/model-a".to_string()],
        removed: vec![],
        changed: vec![],
    }];

    given_feed()
        .with_deltas(deltas)
        .when_run()
        .then_result()
        .should_succeed()
        .expect_rss_file()
        .expect_rss_item_count(1);
}

// ---------------------------------------------------------------------------
// Ordering: newest delta first in the feed
// ---------------------------------------------------------------------------

#[test]
fn feed_should_order_items_newest_first() {
    let deltas = vec![
        DeltaEntry {
            timestamp: "2026-04-29T10:00:00Z".to_string(),
            added: vec!["oldest-model".to_string()],
            removed: vec![],
            changed: vec![],
        },
        DeltaEntry {
            timestamp: "2026-04-30T10:00:00Z".to_string(),
            added: vec!["newest-model".to_string()],
            removed: vec![],
            changed: vec![],
        },
    ];

    let result = given_feed()
        .with_deltas(deltas)
        .when_run()
        .then_result();
    result.should_succeed().expect_rss_file().expect_rss_item_count(2);

    let feed = result.read_rss_feed();
    let first_pos = feed.find("newest-model").unwrap_or(usize::MAX);
    let second_pos = feed.find("oldest-model").unwrap_or(usize::MAX);

    assert!(
        first_pos < second_pos,
        "'newest-model' should appear before 'oldest-model' in the feed"
    );
}

// ---------------------------------------------------------------------------
// Window: limit to last 100 deltas
// ---------------------------------------------------------------------------

#[test]
fn feed_should_limit_to_last_100() {
    let mut deltas = Vec::new();
    for i in 0..102 {
        let ts = format!("2026-06-{:02}T{:02}:00:00Z", (i / 24) + 1, i % 24);
        deltas.push(DeltaEntry {
            timestamp: ts,
            added: vec![format!("model-{:03}", i)],
            removed: vec![],
            changed: vec![],
        });
    }

    let result = given_feed()
        .with_deltas(deltas)
        .when_run()
        .then_result();
    result.should_succeed().expect_rss_item_count(100);

    let feed = result.read_rss_feed();
    // First 2 models should NOT appear
    assert!(!feed.contains("model-000"), "model-000 should not appear");
    assert!(!feed.contains("model-001"), "model-001 should not appear");
    // model-002 should appear (the 3rd, first within the last-100 window)
    assert!(feed.contains("model-002"), "model-002 should appear");
}

// ---------------------------------------------------------------------------
// XML escaping: special chars in model data
// ---------------------------------------------------------------------------

#[test]
fn feed_should_escape_special_chars() {
    let deltas = vec![DeltaEntry {
        timestamp: "2026-04-29T10:00:00Z".to_string(),
        added: vec!["opencode-go/model<angry>".to_string()],
        removed: vec![],
        changed: vec![],
    }];

    given_feed()
        .with_deltas(deltas)
        .when_run()
        .then_result()
        .should_succeed()
        .expect_rss_file()
        .expect_rss_item_count(1)
        .expect_rss_contains("opencode-go/model<angry>");
}

// ---------------------------------------------------------------------------
// Exit 3 when no deltas exist
// ---------------------------------------------------------------------------

#[test]
fn feed_should_exit_3_when_no_deltas() {
    given_feed()
        .when_run()
        .then_result()
        .should_exit_with(3);
}

// ---------------------------------------------------------------------------
// Custom --output path
// ---------------------------------------------------------------------------

#[test]
fn feed_should_write_to_custom_output_path() {
    let deltas = vec![DeltaEntry {
        timestamp: "2026-04-29T10:00:00Z".to_string(),
        added: vec!["opencode-go/model-a".to_string()],
        removed: vec![],
        changed: vec![],
    }];

    let out_path = std::env::temp_dir().join(format!(
        "models-feed-test-{}.xml",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&out_path);

    given_feed()
        .with_deltas(deltas)
        .with_output(out_path.clone())
        .when_run()
        .then_result()
        .should_succeed()
        .expect_rss_file()
        .expect_rss_item_count(1);

    assert!(out_path.exists(), "feed file should exist at custom path");
    let _ = std::fs::remove_file(&out_path);
}
