//! Mistral La Plateforme (API org billing) — `MISTRAL_API_KEY` against
//! `GET https://api.mistral.ai/v1/billing/subscription` and
//! `GET /v1/billing/usage?start_date=…&end_date=…` (current UTC month).

use serde::Deserialize;

use crate::http::Http;
use crate::model::{human_count, money, Metric, Report};
use crate::providers::{env_nonempty, ProviderSpec};
use crate::timeutil::{format_ymd, utc_now};

pub const ID: &str = "mistral";
pub const TITLE: &str = "Mistral La Plateforme";
pub const ENV_KEY: &str = "MISTRAL_API_KEY";
const BASE: &str = "https://api.mistral.ai/v1";

pub fn spec() -> ProviderSpec {
    ProviderSpec {
        id: ID,
        title: TITLE,
        env_hint: "MISTRAL_API_KEY (key needs billing scope)",
        is_configured: || env_nonempty(ENV_KEY).is_some(),
        fetch,
    }
}

#[derive(Debug, Deserialize)]
pub struct Subscription {
    #[serde(alias = "plan", alias = "name")]
    pub plan_name: Option<String>,
    pub monthly_budget: Option<f64>,
    pub credit_balance: Option<f64>,
    pub currency: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UsageResponse {
    #[serde(default)]
    pub data: Vec<UsageRow>,
}

#[derive(Debug, Deserialize)]
pub struct UsageRow {
    #[serde(alias = "totalCost")]
    pub total_cost: Option<f64>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

pub fn parse_subscription(body: &str) -> Result<Subscription, String> {
    serde_json::from_str(body)
        .map_err(|e| format!("invalid JSON from {BASE}/billing/subscription: {e}"))
}

pub fn parse_usage(body: &str) -> Result<UsageResponse, String> {
    serde_json::from_str(body).map_err(|e| format!("invalid JSON from {BASE}/billing/usage: {e}"))
}

fn month_range() -> (String, String) {
    let now = utc_now();
    let first = format_ymd(&crate::timeutil::Utc {
        year: now.year,
        month: now.month,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0,
    });
    (first, format_ymd(&now))
}

pub fn metrics(sub: Option<&Subscription>, usage: Option<&UsageResponse>) -> Vec<Metric> {
    let currency = sub
        .and_then(|s| s.currency.clone())
        .unwrap_or_else(|| "EUR".to_string());
    let mut out = Vec::new();
    if let Some(s) = sub {
        if let Some(credits) = s.credit_balance {
            out.push(Metric {
                name: "credits".to_string(),
                remaining: format!("{} left", money(credits, &currency)),
                used_percent: None,
                reset: None,
            });
        }
    }
    if let Some(u) = usage {
        let spend: f64 = u.data.iter().filter_map(|r| r.total_cost).sum();
        let input: u64 = u.data.iter().filter_map(|r| r.input_tokens).sum();
        let output: u64 = u.data.iter().filter_map(|r| r.output_tokens).sum();
        let budget = sub.and_then(|s| s.monthly_budget);
        let used_percent = budget.and_then(|b| {
            if b > 0.0 {
                Some(spend / b * 100.0)
            } else {
                None
            }
        });
        out.push(Metric {
            name: "monthly budget".to_string(),
            remaining: match budget {
                Some(b) => format!(
                    "{} of {} left",
                    money((b - spend).max(0.0), &currency),
                    money(b, &currency)
                ),
                None => format!("{} spent", money(spend, &currency)),
            },
            used_percent,
            reset: None,
        });
        out.push(Metric {
            name: "tokens this month".to_string(),
            remaining: format!("{} in / {} out", human_count(input), human_count(output)),
            used_percent: None,
            reset: None,
        });
    }
    out
}

fn fetch(http: &Http) -> Result<Report, String> {
    let key = env_nonempty(ENV_KEY).ok_or("no API key in environment")?;
    let auth = format!("Bearer {key}");
    let headers: Vec<(&str, &str)> = vec![
        ("Authorization", auth.as_str()),
        ("Accept", "application/json"),
    ];

    let sub = match http.get_json(&format!("{BASE}/billing/subscription"), &headers) {
        Ok(v) => Some(parse_subscription(&v.to_string())?),
        Err(e) => {
            // Some keys lack billing scope; usage may still work.
            eprintln!("warning: billing/subscription failed: {e}");
            None
        }
    };

    let (start, end) = month_range();
    let usage_url = format!("{BASE}/billing/usage?start_date={start}&end_date={end}");
    let usage = match http.get_json(&usage_url, &headers) {
        Ok(v) => Some(parse_usage(&v.to_string())?),
        Err(e) => return Err(format!("billing/usage failed: {e}")),
    };

    if sub.is_none() && usage.as_ref().is_none_or(|u| u.data.is_empty()) {
        return Err("no billing data available (key may lack billing scope)".to_string());
    }
    let plan = sub.as_ref().and_then(|s| s.plan_name.clone());
    Ok(Report::ok(
        ID,
        TITLE,
        plan,
        metrics(sub.as_ref(), usage.as_ref()),
    ))
}
