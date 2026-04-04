use opencode_model_report::v2::{
    build_rows, ModelRow, ReportInput, SortMode, UsageLabel, UsageSource,
};

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
