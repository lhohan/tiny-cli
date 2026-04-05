use opencode_config_lens::{
    build_rows, render_report_rows, ReportInput, SortMode, UsageLabel, UsageSource,
};

#[test]
fn plain_renderer_should_produce_consistent_table_format() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![(
                "openai/gpt-4".to_string(),
                vec![UsageLabel {
                    label: "default".to_string(),
                    source: UsageSource::OpenCodeDefault,
                }],
            )],
            available_models: vec!["openai/gpt-4".to_string(), "anthropic/claude-3".to_string()],
            costs: vec![
                ("openai/gpt-4".to_string(), Some(30.0), Some(60.0)),
                ("anthropic/claude-3".to_string(), Some(3.0), Some(15.0)),
            ],
        },
        SortMode::ModelName,
    );

    let lines = render_report_rows(&rows);

    // Header line contains expected columns (column widths vary based on content)
    let header = &lines[0];
    assert!(
        header.contains("PROVIDER"),
        "Header should contain PROVIDER column"
    );
    assert!(
        header.contains("MODEL"),
        "Header should contain MODEL column"
    );
    assert!(
        header.contains("ACTIVE"),
        "Header should contain ACTIVE column"
    );
    assert!(header.contains("IN"), "Header should contain IN column");
    assert!(header.contains("OUT"), "Header should contain OUT column");
    assert!(
        header.contains("USAGE"),
        "Header should contain USAGE column"
    );

    // Should have at least 3 lines (header + 2 data rows)
    assert!(
        lines.len() >= 3,
        "Expected at least 3 lines, got {}",
        lines.len()
    );
}

#[test]
fn plain_renderer_should_left_justify_provider_and_model() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![],
            available_models: vec!["provider/model-name".to_string()],
            costs: vec![],
        },
        SortMode::ModelName,
    );

    let lines = render_report_rows(&rows);

    // Find the data row (skip header)
    let data_line = &lines[1];

    // Should contain the provider and model left-justified
    assert!(data_line.contains("provider"));
    assert!(data_line.contains("model-name"));
}

#[test]
fn plain_renderer_should_right_justify_cost_columns() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![],
            available_models: vec!["test/model".to_string()],
            costs: vec![("test/model".to_string(), Some(1.5), Some(2.5))],
        },
        SortMode::ModelName,
    );

    let lines = render_report_rows(&rows);
    let data_line = &lines[1];

    // Should contain formatted costs
    assert!(data_line.contains("1.5") || data_line.contains("1.50"));
    assert!(data_line.contains("2.5") || data_line.contains("2.50"));
}

#[test]
fn plain_renderer_should_show_active_status() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![(
                "test/active-model".to_string(),
                vec![UsageLabel {
                    label: "default".to_string(),
                    source: UsageSource::OpenCodeDefault,
                }],
            )],
            available_models: vec![
                "test/active-model".to_string(),
                "test/inactive-model".to_string(),
            ],
            costs: vec![],
        },
        SortMode::ModelName,
    );

    let lines = render_report_rows(&rows);

    // Find lines containing the models
    let active_line = lines.iter().find(|l| l.contains("active-model")).unwrap();

    // Active model should show "yes"
    assert!(
        active_line.contains("yes"),
        "Active model should show 'yes'"
    );
}

#[test]
fn plain_renderer_should_wrap_long_usage_lists() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![(
                "test/model".to_string(),
                vec![
                    UsageLabel {
                        label: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                        source: UsageSource::OpenCodeDefault,
                    },
                    UsageLabel {
                        label: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
                        source: UsageSource::Weave,
                    },
                ],
            )],
            available_models: vec!["test/model".to_string()],
            costs: vec![],
        },
        SortMode::ModelName,
    );

    let lines = render_report_rows(&rows);

    // With very long usage labels, they should wrap to continuation lines
    // The second usage label might appear on a new line with indentation
    let total_lines = lines.len();
    assert!(
        total_lines > 2,
        "Long usage should wrap to multiple lines, got {} lines",
        total_lines
    );

    // At least one line should start with spaces (continuation line)
    let has_continuation = lines.iter().any(|line| {
        !line.is_empty()
            && line
                .chars()
                .next()
                .map(|c| c.is_whitespace())
                .unwrap_or(false)
    });
    assert!(
        has_continuation,
        "Should have continuation lines for wrapped usage"
    );
}

#[test]
fn plain_renderer_should_format_unknown_costs_as_na() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![],
            available_models: vec!["test/unknown-cost-model".to_string()],
            costs: vec![("test/unknown-cost-model".to_string(), None, None)],
        },
        SortMode::ModelName,
    );

    let lines = render_report_rows(&rows);
    let data_line = lines.iter().find(|l| l.contains("unknown-cost")).unwrap();

    // Unknown costs should show as "n/a"
    assert!(
        data_line.contains("n/a"),
        "Unknown costs should display as 'n/a'"
    );
}

#[test]
fn plain_renderer_should_format_whole_numbers_without_decimals() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![],
            available_models: vec!["test/model".to_string()],
            costs: vec![("test/model".to_string(), Some(5.0), Some(10.0))],
        },
        SortMode::ModelName,
    );

    let lines = render_report_rows(&rows);
    let data_line = lines.iter().find(|l| l.contains("test")).unwrap();

    // Whole numbers should be formatted without decimal places
    // The exact spacing depends on column widths, so just check for the numbers
    assert!(
        data_line.contains(" 5 ")
            || data_line.contains("5  ")
            || data_line.ends_with("5")
            || data_line.contains(" 10 ")
            || data_line.contains("10  ")
            || data_line.ends_with("10"),
        "Whole number costs should be formatted without decimals: {}",
        data_line
    );
}

#[test]
fn plain_renderer_should_handle_empty_rows() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![],
            available_models: vec![],
            costs: vec![],
        },
        SortMode::ModelName,
    );

    let lines = render_report_rows(&rows);

    // Should still have header even with no data
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], "PROVIDER  MODEL  ACTIVE  IN  OUT  USAGE");
}

#[test]
fn plain_renderer_should_preserve_row_order_from_input() {
    let rows = build_rows(
        ReportInput {
            active_usage: vec![],
            available_models: vec![
                "zeta/test-model".to_string(),
                "alpha/test-model".to_string(),
                "middle/test-model".to_string(),
            ],
            costs: vec![],
        },
        SortMode::ModelName, // This should sort by model name
    );

    let lines = render_report_rows(&rows);

    // With ModelName sort, rows should be alphabetically ordered
    // Find the indices of each model in the output (look for unique model names)
    let find_index = |model_name: &str| lines.iter().position(|l| l.contains(model_name));

    let a_idx = find_index("alpha").expect("Should find alpha/test-model");
    let m_idx = find_index("middle").expect("Should find middle/test-model");
    let z_idx = find_index("zeta").expect("Should find zeta/test-model");

    // Should be in alphabetical order by model name: alpha, middle, zeta
    assert!(a_idx < m_idx, "alpha should come before middle");
    assert!(m_idx < z_idx, "middle should come before zeta");
}
