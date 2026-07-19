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
        .expect_rss_item_count(1)
        .expect_rss_pubDate("Wed, 29 Apr 2026 10:00:00 +0000");
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

    // pubDate from newer delta (2026-04-30T10:00:00Z) must appear before older
    let newer_pub = "Thu, 30 Apr 2026 10:00:00 +0000";
    let older_pub = "Wed, 29 Apr 2026 10:00:00 +0000";
    assert!(
        feed.contains(newer_pub),
        "RSS feed should contain pubDate for the newer delta"
    );
    assert!(
        feed.contains(older_pub),
        "RSS feed should contain pubDate for the older delta"
    );
    assert!(
        feed.find(newer_pub) < feed.find(older_pub),
        "newer pubDate should appear before older pubDate in the feed"
    );
}

// ---------------------------------------------------------------------------
// Window: limit to last 100 items
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

// ---------------------------------------------------------------------------
// Per-model granularity: one item per (action × model)
// ---------------------------------------------------------------------------

#[test]
fn feed_emits_one_item_per_model() {
    let deltas = vec![DeltaEntry {
        timestamp: "2026-04-29T10:00:00Z".to_string(),
        added: vec!["alpha".to_string(), "beta".to_string()],
        removed: vec!["gamma".to_string()],
        changed: vec![(
            "delta".to_string(),
            "Old Name".to_string(),
            "New Name".to_string(),
        )],
    }];

    let result = given_feed()
        .with_deltas(deltas)
        .when_run()
        .then_result();
    result
        .should_succeed()
        .expect_rss_file()
        .expect_rss_item_count(4)
        .expect_rss_contains("alpha is now available.")
        .expect_rss_contains("beta is now available.")
        .expect_rss_contains("gamma is no longer available.")
        .expect_rss_contains("delta: \"Old Name\" → \"New Name\"")
        .expect_rss_pubDate("Wed, 29 Apr 2026 10:00:00 +0000");
}

// ---------------------------------------------------------------------------
// Per-model guids: unique per action + model
// ---------------------------------------------------------------------------

#[test]
fn feed_guids_are_unique_per_model() {
    let deltas = vec![DeltaEntry {
        timestamp: "2026-04-29T10:00:00Z".to_string(),
        added: vec!["model-a".to_string(), "model-b".to_string()],
        removed: vec![],
        changed: vec![],
    }];

    let result = given_feed()
        .with_deltas(deltas)
        .when_run()
        .then_result();
    result.should_succeed().expect_rss_file();

    let feed = result.read_rss_feed();
    // Two distinct guids with different model IDs
    assert!(
        feed.contains("models-watch-2026-04-29T10:00:00Z-new-model-a"),
        "missing guid for model-a"
    );
    assert!(
        feed.contains("models-watch-2026-04-29T10:00:00Z-new-model-b"),
        "missing guid for model-b"
    );
    // Each guid appears exactly once
    assert_eq!(
        feed.matches("models-watch-").count(),
        2,
        "expected exactly 2 guids"
    );
}

// ---------------------------------------------------------------------------
// 100-item cap cuts mid-delta when a single delta has multiple models
// ---------------------------------------------------------------------------

#[test]
fn feed_should_cap_at_100_items_mid_delta() {
    // Newest delta has 3 added models. Then 49 deltas with 2 models each.
    // Processed newest-first: 3 + (49 × 2) = 101 items, capped at 100.
    // The oldest delta (model-00) contributes 2 models, but only 1 fits.
    let mut deltas = Vec::new();

    // Newest: 3 models
    deltas.push(DeltaEntry {
        timestamp: "2026-07-30T00:00:00Z".to_string(),
        added: vec![
            "burst-a".to_string(),
            "burst-b".to_string(),
            "burst-c".to_string(),
        ],
        removed: vec![],
        changed: vec![],
    });

    // Older: 49 deltas with 2 models each (model-48..model-00 from newest to oldest)
    for i in 0..49 {
        let ts = format!("2026-07-{:02}T{:02}:00:00Z", (i / 24) + 1, i % 24);
        deltas.push(DeltaEntry {
            timestamp: ts,
            added: vec![format!("model-{:02}-a", i), format!("model-{:02}-b", i)],
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
    // The oldest non-burst delta is model-00 (i=0, smallest timestamp).
    // Its first model is the 100th item; its second model is cut.
    assert!(
        feed.contains("model-00-a"),
        "model-00-a should appear (the 100th item)"
    );
    assert!(
        !feed.contains("model-00-b"),
        "model-00-b should be cut (the 101st item)"
    );
}
