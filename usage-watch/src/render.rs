//! Plain-text and JSON rendering of provider reports.

use crate::model::Report;

pub fn render_text(reports: &[Report]) -> String {
    let mut out = String::new();
    for (i, r) in reports.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match &r.plan {
            Some(p) => out.push_str(&format!("{} [{}] — plan: {}\n", r.title, r.provider, p)),
            None => out.push_str(&format!("{} [{}]\n", r.title, r.provider)),
        }
        if let Some(e) = &r.error {
            out.push_str(&format!("  error: {e}\n"));
        }
        for m in &r.metrics {
            let mut line = format!("  {:<18} {}", m.name, m.remaining);
            if let Some(u) = m.used_percent {
                line.push_str(&format!(" ({u:.1}% used)"));
            }
            if let Some(reset) = &m.reset {
                line.push_str(&format!(" — resets {reset}"));
            }
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

pub fn render_json(reports: &[Report]) -> String {
    serde_json::to_string_pretty(reports).unwrap_or_else(|_| "[]".to_string())
}
