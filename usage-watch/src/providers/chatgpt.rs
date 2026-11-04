//! ChatGPT Go/Plus/Pro subscription — `GET https://chatgpt.com/backend-api/wham/usage`.
//!
//! The endpoint is unofficial (the same one the Codex CLI uses) and its shape has
//! varied over time, so parsing is tolerant: windows are collected from
//! `rate_limit.primary_window`/`secondary_window`, from a top-level
//! `primary_window`/`secondary_window`, or from a `rate_limits` array.

use serde_json::Value;

use crate::http::Http;
use crate::model::{Metric, Report};
use crate::providers::{env_any, env_nonempty, ProviderSpec};
use crate::timeutil::{epoch_to_utc, fmt_duration, format_ymd_hms};

pub const ID: &str = "chatgpt";
pub const TITLE: &str = "ChatGPT";
pub const ENV_TOKEN: &str = "CHATGPT_ACCESS_TOKEN";
pub const ENV_ACCOUNT: &str = "CHATGPT_ACCOUNT_ID";
const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";

pub fn spec() -> ProviderSpec {
    ProviderSpec {
        id: ID,
        title: TITLE,
        env_hint: "CHATGPT_ACCESS_TOKEN (+ optional CHATGPT_ACCOUNT_ID)",
        is_configured: || env_nonempty(ENV_TOKEN).is_some(),
        fetch,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub label: String,
    pub used_percent: f64,
    pub window_secs: Option<u64>,
    pub reset_after_secs: Option<u64>,
    pub reset_at_epoch: Option<u64>,
}

struct RawWindow {
    used_percent: f64,
    window_secs: Option<u64>,
    reset_after_secs: Option<u64>,
    reset_at_epoch: Option<u64>,
}

fn window_of(v: &Value) -> Option<RawWindow> {
    let used = v.get("used_percent")?.as_f64()?;
    Some(RawWindow {
        used_percent: used,
        window_secs: v.get("limit_window_seconds").and_then(Value::as_u64),
        reset_after_secs: v.get("reset_after_seconds").and_then(Value::as_u64),
        reset_at_epoch: v.get("reset_at").and_then(Value::as_u64),
    })
}

fn push_window(v: &Value, label: &str, out: &mut Vec<Window>) {
    if let Some(raw) = window_of(v) {
        out.push(Window {
            label: label.to_string(),
            used_percent: raw.used_percent,
            window_secs: raw.window_secs,
            reset_after_secs: raw.reset_after_secs,
            reset_at_epoch: raw.reset_at_epoch,
        });
    }
}

fn collect_named(v: &Value, out: &mut Vec<Window>) {
    for (key, label) in [
        ("primary_window", "primary"),
        ("secondary_window", "secondary"),
    ] {
        if let Some(w) = v.get(key) {
            push_window(w, label, out);
        }
    }
}

/// Parse a `wham/usage` response into (plan_type, windows).
pub fn parse(body: &str) -> Result<(Option<String>, Vec<Window>), String> {
    let v: Value =
        serde_json::from_str(body).map_err(|e| format!("invalid JSON from {USAGE_URL}: {e}"))?;
    let plan = v
        .get("plan_type")
        .and_then(Value::as_str)
        .map(str::to_string);
    let mut wins = Vec::new();
    if let Some(rl) = v.get("rate_limit") {
        collect_named(rl, &mut wins);
    }
    if wins.is_empty() {
        if let Some(items) = v.get("rate_limits").and_then(Value::as_array) {
            for item in items {
                let id = item
                    .get("id")
                    .and_then(Value::as_str)
                    .or_else(|| item.get("name").and_then(Value::as_str))
                    .unwrap_or("limit");
                if item.get("used_percent").is_some() {
                    push_window(item, id, &mut wins);
                }
                for (key, label) in [
                    ("primary_window", "primary"),
                    ("secondary_window", "secondary"),
                ] {
                    if let Some(w) = item.get(key) {
                        push_window(w, &format!("{id}:{label}"), &mut wins);
                    }
                }
            }
        }
    }
    if wins.is_empty() {
        collect_named(&v, &mut wins);
    }
    Ok((plan, wins))
}

pub fn metrics(wins: &[Window], now: u64) -> Vec<Metric> {
    wins.iter()
        .map(|w| {
            let name = match w.window_secs {
                Some(s) => format!("{} ({})", w.label, fmt_duration(s)),
                None => w.label.clone(),
            };
            let reset = w
                .reset_after_secs
                .map(|s| format!("in {}", fmt_duration(s)))
                .or_else(|| {
                    w.reset_at_epoch.map(|ts| {
                        let delta = ts as i64 - now as i64;
                        if delta > 0 {
                            format!("in {}", fmt_duration(delta as u64))
                        } else {
                            format!("at {}", format_ymd_hms(&epoch_to_utc(ts)))
                        }
                    })
                });
            Metric {
                name,
                remaining: format!("{:.0}% left", (100.0 - w.used_percent).max(0.0)),
                used_percent: Some(w.used_percent),
                reset,
            }
        })
        .collect()
}

fn fetch(http: &Http) -> Result<Report, String> {
    let token = env_nonempty(ENV_TOKEN).ok_or("no access token in environment")?;
    let auth = format!("Bearer {token}");
    let mut headers: Vec<(&str, &str)> = vec![
        ("Authorization", auth.as_str()),
        ("Accept", "application/json"),
    ];
    let account = env_any(&[ENV_ACCOUNT]);
    if let Some(acc) = &account {
        headers.push(("ChatGPT-Account-Id", acc.as_str()));
    }
    let (status, body) = http.get(USAGE_URL, &headers)?;
    if !(200..300).contains(&status) {
        return Err(format!(
            "usage failed: HTTP {status}: {}",
            crate::http::truncate(&body, 400)
        ));
    }
    let (plan, wins) = parse(&body)?;
    if wins.is_empty() {
        return Err(format!(
            "no rate-limit windows in response from {USAGE_URL}"
        ));
    }
    Ok(Report::ok(
        ID,
        TITLE,
        plan,
        metrics(&wins, crate::timeutil::unix_now()),
    ))
}
