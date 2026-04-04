use opencode_model_report::v2::{
    build_rows, ModelRow, ReportInput, SortMode, UsageLabel, UsageSource,
};

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
