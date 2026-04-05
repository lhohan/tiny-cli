use opencode_config_lens::{build_rows, ModelRow, ReportInput, SortMode, UsageLabel, UsageSource};

#[test]
fn report_should_sort_unknown_costs_last_when_using_cost_asc() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![],
            available_models: vec![
                "provider/unknown".to_string(),
                "provider/expensive".to_string(),
                "provider/cheap".to_string(),
            ],
            costs: vec![
                ("provider/expensive".to_string(), Some(10.0), Some(10.0)),
                ("provider/cheap".to_string(), Some(1.0), Some(1.0)),
                ("provider/unknown".to_string(), None, None),
            ],
        },
        SortMode::CostAsc,
    );

    assert_eq!(
        rows.into_iter().map(|row| row.model).collect::<Vec<_>>(),
        vec!["provider/cheap", "provider/expensive", "provider/unknown"]
    );
}

#[test]
fn report_should_use_alphabetical_tie_break_for_same_costs() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![],
            available_models: vec![
                "provider/zebra".to_string(),
                "provider/alpha".to_string(),
                "provider/middle".to_string(),
            ],
            costs: vec![
                // All have same cost, should be sorted alphabetically
                ("provider/zebra".to_string(), Some(5.0), Some(5.0)),
                ("provider/alpha".to_string(), Some(5.0), Some(5.0)),
                ("provider/middle".to_string(), Some(5.0), Some(5.0)),
            ],
        },
        SortMode::CostAsc,
    );

    let models: Vec<_> = rows.into_iter().map(|row| row.model).collect();
    assert_eq!(
        models,
        vec!["provider/alpha", "provider/middle", "provider/zebra"]
    );
}

#[test]
fn report_should_preserve_duplicate_usage_labels() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![(
                "provider/alpha".to_string(),
                vec![
                    UsageLabel {
                        label: "agent1".to_string(),
                        source: UsageSource::OpenCodeCustom,
                    },
                    UsageLabel {
                        label: "agent2".to_string(),
                        source: UsageSource::OpenCodeCustom,
                    },
                    UsageLabel {
                        label: "agent1".to_string(), // Duplicate label
                        source: UsageSource::Weave,
                    },
                ],
            )],
            available_models: vec!["provider/alpha".to_string()],
            costs: vec![],
        },
        SortMode::ModelName,
    );

    let alpha_row = rows.iter().find(|r| r.model == "provider/alpha").unwrap();
    assert_eq!(
        alpha_row.usage.len(),
        3,
        "Should preserve all usage labels including duplicates"
    );
}

#[test]
fn report_should_sort_usage_labels_alphabetically() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![(
                "provider/alpha".to_string(),
                vec![
                    UsageLabel {
                        label: "zebra".to_string(),
                        source: UsageSource::OpenCodeCustom,
                    },
                    UsageLabel {
                        label: "alpha".to_string(),
                        source: UsageSource::OpenCodeCustom,
                    },
                    UsageLabel {
                        label: "middle".to_string(),
                        source: UsageSource::Weave,
                    },
                ],
            )],
            available_models: vec!["provider/alpha".to_string()],
            costs: vec![],
        },
        SortMode::ModelName,
    );

    let alpha_row = rows.iter().find(|r| r.model == "provider/alpha").unwrap();
    let labels: Vec<_> = alpha_row.usage.iter().map(|u| u.label.as_str()).collect();
    // Should be sorted alphabetically by label
    assert_eq!(labels, vec!["alpha", "middle", "zebra"]);
}

#[test]
fn report_should_split_model_id_into_provider_and_model_name() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![],
            available_models: vec!["provider/sub/model".to_string()],
            costs: vec![],
        },
        SortMode::ModelName,
    );

    let row = &rows[0];
    assert_eq!(row.provider, "provider");
    assert_eq!(row.model_name, "sub/model");
}

#[test]
fn report_should_sort_active_models_first_when_using_active_first() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![
                (
                    "provider/alpha".to_string(),
                    vec![UsageLabel {
                        label: "default".to_string(),
                        source: UsageSource::OpenCodeDefault,
                    }],
                ),
                (
                    "provider/beta".to_string(),
                    vec![UsageLabel {
                        label: "agent-a".to_string(),
                        source: UsageSource::Weave,
                    }],
                ),
            ],
            available_models: vec![
                "provider/beta".to_string(),
                "provider/alpha".to_string(),
                "provider/zeta".to_string(),
            ],
            costs: vec![
                ("provider/alpha".to_string(), Some(1.0), Some(2.0)),
                ("provider/beta".to_string(), Some(0.5), Some(0.5)),
                ("provider/zeta".to_string(), Some(10.0), Some(10.0)),
            ],
        },
        SortMode::ActiveFirst,
    );

    assert_eq!(
        rows.into_iter().map(|row| row.model).collect::<Vec<_>>(),
        vec!["provider/beta", "provider/alpha", "provider/zeta"]
    );
}

#[test]
fn report_should_mark_inactive_models_without_usage() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![(
                "provider/alpha".to_string(),
                vec![UsageLabel {
                    label: "default".to_string(),
                    source: UsageSource::OpenCodeDefault,
                }],
            )],
            available_models: vec!["provider/alpha".to_string(), "provider/beta".to_string()],
            costs: vec![],
        },
        SortMode::ModelName,
    );

    let beta = rows
        .into_iter()
        .find(|row: &ModelRow| row.model == "provider/beta")
        .unwrap();
    assert!(!beta.active);
    assert!(beta.usage.is_empty());
}

#[test]
fn report_should_sort_unknown_costs_last_when_using_cost_desc() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![],
            available_models: vec![
                "provider/unknown".to_string(),
                "provider/expensive".to_string(),
                "provider/cheap".to_string(),
            ],
            costs: vec![
                ("provider/expensive".to_string(), Some(10.0), Some(10.0)),
                ("provider/cheap".to_string(), Some(1.0), Some(1.0)),
                ("provider/unknown".to_string(), None, None),
            ],
        },
        SortMode::CostDesc,
    );

    assert_eq!(
        rows.into_iter().map(|row| row.model).collect::<Vec<_>>(),
        vec!["provider/expensive", "provider/cheap", "provider/unknown"]
    );
}
