//! OpenCode Go subscription — `GET https://opencode.ai/zen/go/v1/usage`.
//!
//! The endpoint reports `usage.rolling`/`weekly`/`monthly` windows with a
//! `percent` (0–100) and an ISO `resetsAt`. Older responses used top-level
//! `rolling5h`/`weekly`/`monthly` windows with `usageDollars`, `limitDollars`,
//! `usagePercent`, and `resetInSec`; both shapes are accepted.

use serde_json::Value;

use crate::http::Http;
use crate::model::{Metric, Report};
use crate::providers::{env_any, ProviderSpec};
use crate::timeutil::{epoch_to_utc, fmt_duration, format_ymd_hms, parse_iso8601};

pub const ID: &str = "opencode-go";
pub const TITLE: &str = "OpenCode Go";
pub const ENV_KEYS: &[&str] = &["OPENCODE_GO_API_KEY", "OPENCODE_API_KEY"];
const USAGE_URL: &str = "https://opencode.ai/zen/go/v1/usage";

pub fn spec() -> ProviderSpec {
    ProviderSpec {
        id: ID,
        title: TITLE,
        env_hint: "OPENCODE_GO_API_KEY or OPENCODE_API_KEY",
        is_configured: || env_any(ENV_KEYS).is_some(),
        fetch,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub used_percent: f64,
    pub usage_dollars: Option<f64>,
    pub limit_dollars: Option<f64>,
    pub reset_in_sec: Option<u64>,
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GoUsage {
    pub rolling: Window,
    pub weekly: Option<Window>,
    pub monthly: Option<Window>,
    pub subscribed_at: Option<String>,
    pub renew_at: Option<String>,
}

fn window_of(v: &Value) -> Option<Window> {
    let used_percent = v
        .get("percent")
        .or_else(|| v.get("usagePercent"))
        .or_else(|| v.get("usedPercent"))
        .and_then(Value::as_f64)?;
    Some(Window {
        used_percent,
        usage_dollars: v.get("usageDollars").and_then(Value::as_f64),
        limit_dollars: v.get("limitDollars").and_then(Value::as_f64),
        reset_in_sec: v.get("resetInSec").and_then(Value::as_u64),
        resets_at: v
            .get("resetsAt")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn window_at<'a>(usage: &'a Value, root: &'a Value, key: &str, legacy: &str) -> Option<Window> {
    usage
        .get(key)
        .or_else(|| root.get(legacy))
        .and_then(window_of)
}

pub fn parse(body: &str) -> Result<GoUsage, String> {
    let root: Value =
        serde_json::from_str(body).map_err(|e| format!("invalid JSON from {USAGE_URL}: {e}"))?;
    let usage = root.get("usage").unwrap_or(&root);
    let rolling = window_at(usage, &root, "rolling", "rolling5h")
        .ok_or_else(|| format!("no rolling usage window in response from {USAGE_URL}"))?;
    Ok(GoUsage {
        rolling,
        weekly: window_at(usage, &root, "weekly", "weekly"),
        monthly: window_at(usage, &root, "monthly", "monthly"),
        subscribed_at: root
            .get("subscribedAt")
            .and_then(Value::as_str)
            .map(str::to_string),
        renew_at: usage
            .get("renewAt")
            .or_else(|| root.get("renewAt"))
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn reset_label(w: &Window, now: u64) -> Option<String> {
    if let Some(secs) = w.reset_in_sec {
        return Some(format!("in {}", fmt_duration(secs)));
    }
    let ts = parse_iso8601(w.resets_at.as_deref()?)?;
    let delta = ts as i64 - now as i64;
    if delta > 0 {
        Some(format!("in {}", fmt_duration(delta as u64)))
    } else {
        Some(format!("at {}", format_ymd_hms(&epoch_to_utc(ts))))
    }
}

pub fn metrics(u: &GoUsage, now: u64) -> Vec<Metric> {
    let mut out = Vec::new();
    for (name, window) in [
        ("5h window", Some(&u.rolling)),
        ("weekly", u.weekly.as_ref()),
        ("monthly", u.monthly.as_ref()),
    ] {
        let Some(w) = window else { continue };
        let remaining = match (w.usage_dollars, w.limit_dollars) {
            (Some(used), Some(limit)) => {
                format!("${:.2} of ${:.2} left", (limit - used).max(0.0), limit)
            }
            _ => format!("{:.0}% left", (100.0 - w.used_percent).max(0.0)),
        };
        out.push(Metric {
            name: name.to_string(),
            remaining,
            used_percent: Some(w.used_percent),
            reset: reset_label(w, now),
        });
    }
    out
}

fn fetch(http: &Http) -> Result<Report, String> {
    let key = env_any(ENV_KEYS).ok_or("no API key in environment")?;
    let auth = format!("Bearer {key}");
    let resp = http.get(
        USAGE_URL,
        &[
            ("Authorization", auth.as_str()),
            ("Accept", "application/json"),
        ],
    )?;
    if !(200..300).contains(&resp.status) {
        return Err(format!(
            "usage failed: HTTP {}: {}",
            resp.status,
            crate::http::truncate(&resp.body, 400)
        ));
    }
    let usage = parse(&resp.body)?;
    let plan = usage
        .subscribed_at
        .as_ref()
        .map(|s| format!("subscribed {s}"))
        .or_else(|| usage.renew_at.as_ref().map(|s| format!("renews {s}")));
    Ok(Report::ok(
        ID,
        TITLE,
        plan,
        metrics(&usage, crate::timeutil::unix_now()),
    ))
}
