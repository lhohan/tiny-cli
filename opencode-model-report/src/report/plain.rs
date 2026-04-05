//! Plain text renderer for tests
//!
//! This module provides a stable, test-facing plain-text renderer.
//! It is intentionally kept simple and deterministic for testing purposes.

use super::model::ModelRow;
use super::text::{format_cost, ljust, rjust, wrap_usage};

/// Render model rows as plain text lines
///
/// This is the stable test-facing contract for report rendering.
/// The output format is intentionally kept simple and consistent.
///
/// # Arguments
/// * `rows` - The model rows to render
///
/// # Returns
/// A vector of strings representing the formatted report lines
pub fn render_report_rows(rows: &[ModelRow]) -> Vec<String> {
    let provider_width = std::iter::once("PROVIDER".len())
        .chain(rows.iter().map(|row| row.provider.len()))
        .max()
        .unwrap_or(0);
    let model_width = std::iter::once("MODEL".len())
        .chain(rows.iter().map(|row| row.model_name.len()))
        .max()
        .unwrap_or(0);
    let active_width = "ACTIVE".len();
    let in_width = std::iter::once("IN".len())
        .chain(rows.iter().map(|row| format_cost(row.input_cost).len()))
        .max()
        .unwrap_or(0);
    let out_width = std::iter::once("OUT".len())
        .chain(rows.iter().map(|row| format_cost(row.output_cost).len()))
        .max()
        .unwrap_or(0);
    let prefix_width =
        provider_width + 2 + model_width + 2 + active_width + 2 + in_width + 2 + out_width + 2;

    let mut lines = Vec::new();
    lines.push(format!(
        "{}  {}  {}  {}  {}  USAGE",
        ljust("PROVIDER", provider_width),
        ljust("MODEL", model_width),
        ljust("ACTIVE", active_width),
        rjust("IN", in_width),
        rjust("OUT", out_width)
    ));

    for row in rows {
        let usage_text = row
            .usage
            .iter()
            .map(|usage| usage.label.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let usage_lines = wrap_usage(&usage_text, 50);
        let first_usage = usage_lines.first().map(String::as_str).unwrap_or("");

        lines.push(format!(
            "{}  {}  {}  {}  {}  {}",
            ljust(&row.provider, provider_width),
            ljust(&row.model_name, model_width),
            ljust(if row.active { "yes" } else { "no" }, active_width),
            rjust(&format_cost(row.input_cost), in_width),
            rjust(&format_cost(row.output_cost), out_width),
            first_usage
        ));

        for continuation in usage_lines.iter().skip(1) {
            lines.push(format!("{}{}", " ".repeat(prefix_width), continuation));
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::model::{ModelRow, UsageLabel, UsageSource};

    fn test_row() -> ModelRow {
        ModelRow {
            model: "openai/gpt-4".to_string(),
            provider: "openai".to_string(),
            model_name: "gpt-4".to_string(),
            active: true,
            input_cost: Some(30.0),
            output_cost: Some(60.0),
            usage: vec![UsageLabel {
                label: "default".to_string(),
                source: UsageSource::OpenCodeDefault,
            }],
        }
    }

    #[test]
    fn render_should_include_header() {
        let lines = render_report_rows(&[]);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("PROVIDER"));
        assert!(lines[0].contains("MODEL"));
    }

    #[test]
    fn render_should_show_active_status() {
        let mut row = test_row();
        row.active = true;
        let lines = render_report_rows(&[row]);
        assert!(lines[1].contains("yes"));
    }

    #[test]
    fn render_should_show_inactive_status() {
        let mut row = test_row();
        row.active = false;
        let lines = render_report_rows(&[row]);
        assert!(lines[1].contains("no"));
    }
}
