//! Mistral La Plateforme billing.
//!
//! The plan comes from `GET https://api.mistral.ai/v1/billing/subscription`
//! with `MISTRAL_API_KEY`. Monthly usage and credits are only exposed to the
//! admin web session, so they need a full `Cookie:` header from
//! `admin.mistral.ai` in `MISTRAL_ADMIN_COOKIE`:
//! - `GET https://admin.mistral.ai/api/billing/v2/usage?month=…&year=…`
//! - `GET https://admin.mistral.ai/api/billing/credits`

use serde::Deserialize;
use serde_json::Value;

use crate::http::Http;
use crate::model::{human_count, money, Metric, Report};
use crate::providers::{cookie_value, env_nonempty, ProviderSpec};
use crate::timeutil::utc_now;

pub const ID: &str = "mistral";
pub const TITLE: &str = "Mistral La Plateforme";
pub const ENV_KEY: &str = "MISTRAL_API_KEY";
pub const ENV_COOKIE: &str = "MISTRAL_ADMIN_COOKIE";
const API_BASE: &str = "https://api.mistral.ai/v1";
const ADMIN_BASE: &str = "https://admin.mistral.ai";

pub fn spec() -> ProviderSpec {
    ProviderSpec {
        id: ID,
        title: TITLE,
        env_hint: "MISTRAL_API_KEY (+ optional MISTRAL_ADMIN_COOKIE for usage)",
        is_configured: || env_nonempty(ENV_KEY).is_some(),
        fetch,
    }
}

#[derive(Debug, Deserialize)]
pub struct Subscription {
    pub plan_name: Option<String>,
    pub payg_enabled: Option<bool>,
    pub vibe_payg_enabled: Option<bool>,
    pub enterprise: Option<bool>,
}

#[derive(Debug, Default, PartialEq)]
pub struct UsageTotals {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub currency: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Credits {
    pub wallet_amount: f64,
    pub credit_notes_amount: Option<f64>,
    pub currency: Option<String>,
}

pub fn parse_subscription(body: &str) -> Result<Subscription, String> {
    serde_json::from_str(body)
        .map_err(|e| format!("invalid JSON from {API_BASE}/billing/subscription: {e}"))
}

pub fn parse_credits(body: &str) -> Result<Credits, String> {
    serde_json::from_str(body)
        .map_err(|e| format!("invalid JSON from {ADMIN_BASE}/api/billing/credits: {e}"))
}

fn sum_entries(v: Option<&Value>) -> u64 {
    v.and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|e| e.get("value").and_then(Value::as_u64))
                .sum()
        })
        .unwrap_or(0)
}

fn add_models(category: Option<&Value>, totals: &mut UsageTotals) {
    let Some(models) = category
        .and_then(|c| c.get("models"))
        .and_then(Value::as_object)
    else {
        return;
    };
    for model in models.values() {
        totals.input_tokens += sum_entries(model.get("input"));
        totals.output_tokens += sum_entries(model.get("output"));
        totals.cached_tokens += sum_entries(model.get("cached"));
    }
}

/// Parse `billing/v2/usage`, summing token categories only (audio seconds,
/// pages, and storage are not tokens).
pub fn parse_usage(body: &str) -> Result<UsageTotals, String> {
    let v: Value = serde_json::from_str(body)
        .map_err(|e| format!("invalid JSON from {ADMIN_BASE}/api/billing/v2/usage: {e}"))?;
    let mut totals = UsageTotals {
        currency: v
            .get("currency")
            .and_then(Value::as_str)
            .map(str::to_string),
        ..Default::default()
    };
    add_models(v.get("completion"), &mut totals);
    add_models(v.get("chat"), &mut totals);
    add_models(
        v.get("vibe_code").and_then(|c| c.get("completion")),
        &mut totals,
    );
    Ok(totals)
}

pub fn metrics(
    usage: Option<&UsageTotals>,
    credits: Option<&Credits>,
    usage_error: Option<&str>,
) -> Vec<Metric> {
    let mut out = Vec::new();
    if let Some(u) = usage {
        out.push(Metric {
            name: "tokens this month".to_string(),
            remaining: format!(
                "{} in / {} out",
                human_count(u.input_tokens),
                human_count(u.output_tokens)
            ),
            used_percent: None,
            reset: None,
        });
    }
    if let Some(c) = credits {
        let currency = c
            .currency
            .clone()
            .or_else(|| usage.and_then(|u| u.currency.clone()))
            .unwrap_or_else(|| "EUR".to_string());
        let total = c.wallet_amount + c.credit_notes_amount.unwrap_or(0.0);
        out.push(Metric {
            name: "credits".to_string(),
            remaining: format!("{} left", money(total, &currency)),
            used_percent: None,
            reset: None,
        });
    }
    if let Some(e) = usage_error {
        out.push(Metric {
            name: "usage".to_string(),
            remaining: format!("error: {e}"),
            used_percent: None,
            reset: None,
        });
    }
    if out.is_empty() {
        out.push(Metric {
            name: "usage".to_string(),
            remaining: format!("set {ENV_COOKIE} (Cookie header from admin.mistral.ai) to fetch"),
            used_percent: None,
            reset: None,
        });
    }
    out
}

fn admin_headers<'a>(
    cookie: &'a str,
    csrf: Option<&'a str>,
    referer: &'a str,
) -> Vec<(&'a str, &'a str)> {
    let mut headers = vec![
        ("Accept", "*/*"),
        ("Cookie", cookie),
        ("Referer", referer),
        ("Origin", ADMIN_BASE),
    ];
    if let Some(csrf) = csrf {
        headers.push(("X-CSRFTOKEN", csrf));
    }
    headers
}

fn fetch(http: &Http) -> Result<Report, String> {
    let key = env_nonempty(ENV_KEY).ok_or("no API key in environment")?;
    let auth = format!("Bearer {key}");
    let sub_body = http
        .get_json(
            &format!("{API_BASE}/billing/subscription"),
            &[
                ("Authorization", auth.as_str()),
                ("Accept", "application/json"),
            ],
        )
        .map_err(|e| format!("billing/subscription failed: {e}"))?;
    let sub = parse_subscription(&sub_body.to_string())?;

    let mut usage = None;
    let mut credits = None;
    let mut usage_error = None;
    if let Some(cookie) = env_nonempty(ENV_COOKIE) {
        let csrf = cookie_value(&cookie, "csrftoken");
        let now = utc_now();
        let usage_url = format!(
            "{ADMIN_BASE}/api/billing/v2/usage?month={}&year={}",
            now.month, now.year
        );
        let headers = admin_headers(
            &cookie,
            csrf.as_deref(),
            "https://admin.mistral.ai/organization/usage",
        );
        match http.get_json(&usage_url, &headers) {
            Ok(v) => match parse_usage(&v.to_string()) {
                Ok(u) => usage = Some(u),
                Err(e) => usage_error = Some(e),
            },
            Err(e) => usage_error = Some(format!("billing/v2/usage failed: {e}")),
        }
        let headers = admin_headers(
            &cookie,
            csrf.as_deref(),
            "https://admin.mistral.ai/organization/billing",
        );
        if let Ok(v) = http.get_json(&format!("{ADMIN_BASE}/api/billing/credits"), &headers) {
            credits = parse_credits(&v.to_string()).ok();
        }
    }

    Ok(Report::ok(
        ID,
        TITLE,
        sub.plan_name.clone(),
        metrics(usage.as_ref(), credits.as_ref(), usage_error.as_deref()),
    ))
}
