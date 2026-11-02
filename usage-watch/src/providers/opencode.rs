//! OpenCode Go subscription — `GET https://opencode.ai/zen/go/v1/usage`.

use serde::Deserialize;

use crate::http::Http;
use crate::model::{Metric, Report};
use crate::providers::{env_any, ProviderSpec};
use crate::timeutil::fmt_duration;

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

#[derive(Debug, Deserialize)]
pub struct GoUsage {
    #[serde(rename = "rolling5h")]
    pub rolling5h: Window,
    pub weekly: Window,
    pub monthly: Window,
    #[serde(rename = "subscribedAt")]
    pub subscribed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Window {
    #[serde(rename = "usageDollars")]
    pub usage: f64,
    #[serde(rename = "limitDollars")]
    pub limit: f64,
    #[serde(rename = "usagePercent")]
    pub used_percent: f64,
    #[serde(rename = "resetInSec")]
    pub reset_in_sec: u64,
}

pub fn parse(body: &str) -> Result<GoUsage, String> {
    serde_json::from_str(body).map_err(|e| format!("invalid JSON from {USAGE_URL}: {e}"))
}

pub fn metrics(u: &GoUsage) -> Vec<Metric> {
    let mut out = Vec::new();
    for (name, w) in [
        ("5h window", &u.rolling5h),
        ("weekly", &u.weekly),
        ("monthly", &u.monthly),
    ] {
        out.push(Metric {
            name: name.to_string(),
            remaining: format!(
                "${:.2} of ${:.2} left",
                (w.limit - w.usage).max(0.0),
                w.limit
            ),
            used_percent: Some(w.used_percent),
            reset: Some(format!("in {}", fmt_duration(w.reset_in_sec))),
        });
    }
    out
}

fn fetch(http: &Http) -> Result<Report, String> {
    let key = env_any(ENV_KEYS).ok_or("no API key in environment")?;
    let auth = format!("Bearer {key}");
    let (_, body) = http.get(
        USAGE_URL,
        &[
            ("Authorization", auth.as_str()),
            ("Accept", "application/json"),
        ],
    )?;
    let usage = parse(&body)?;
    let plan = usage
        .subscribed_at
        .as_ref()
        .map(|s| format!("subscribed {s}"));
    Ok(Report::ok(ID, TITLE, plan, metrics(&usage)))
}
