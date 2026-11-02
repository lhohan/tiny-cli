use serde::Serialize;

/// One line of usage information, e.g. "weekly: $21.09 of $30.00 left".
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Metric {
    pub name: String,
    pub remaining: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset: Option<String>,
}

/// Everything the CLI knows about one subscription provider.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub provider: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    pub metrics: Vec<Metric>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Report {
    pub fn ok(provider: &str, title: &str, plan: Option<String>, metrics: Vec<Metric>) -> Self {
        Report {
            provider: provider.to_string(),
            title: title.to_string(),
            plan,
            metrics,
            error: None,
        }
    }

    pub fn err(provider: &str, title: &str, error: String) -> Self {
        Report {
            provider: provider.to_string(),
            title: title.to_string(),
            plan: None,
            metrics: Vec::new(),
            error: Some(error),
        }
    }
}

/// Format an amount with its currency symbol ("€12.34", "$9.66", "12.34 CHF").
pub fn money(value: f64, currency: &str) -> String {
    let sym = match currency {
        "EUR" => "€",
        "USD" => "$",
        "GBP" => "£",
        _ => "",
    };
    if sym.is_empty() {
        format!("{value:.2} {currency}")
    } else {
        format!("{sym}{value:.2}")
    }
}

/// Compact token counts ("1.23M", "45.6K", "980").
pub fn human_count(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.2}B", n as f64 / 1e9)
    } else if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1e6)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1e3)
    } else {
        n.to_string()
    }
}
