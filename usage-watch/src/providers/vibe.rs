//! Mistral Vibe (Le Chat consumer plan) — plan info from
//! `GET https://console.mistral.ai/api/vibe/whoami` with the Vibe API key;
//! monthly Vibe plan usage and credit balance need a browser session, supplied
//! as a full `Cookie:` header from `admin.mistral.ai` in `MISTRAL_ADMIN_COOKIE`:
//! - `GET https://console.mistral.ai/api-ui/trpc/billing.vibeUsage?batch=1&input=…`
//! - `GET https://admin.mistral.ai/api/billing/credits`

use serde::Deserialize;
use serde_json::Value;

use crate::http::Http;
use crate::model::{money, Metric, Report};
use crate::providers::{env_any, env_nonempty, ProviderSpec};
use crate::timeutil::{epoch_to_utc, format_ymd_hms, parse_iso8601};

pub const ID: &str = "vibe";
pub const TITLE: &str = "Mistral Vibe";
pub const ENV_KEYS: &[&str] = &["VIBE_API_KEY", "MISTRAL_API_KEY"];
pub const ENV_COOKIE: &str = "MISTRAL_ADMIN_COOKIE";
const WHOAMI_URL: &str = "https://console.mistral.ai/api/vibe/whoami";
const VIBE_USAGE_URL: &str = "https://console.mistral.ai/api-ui/trpc/billing.vibeUsage?batch=1&input=%7B%220%22%3A%7B%22json%22%3Anull%2C%22meta%22%3A%7B%22values%22%3A%5B%22undefined%22%5D%2C%22v%22%3A1%7D%7D%7D";
const CREDITS_URL: &str = "https://admin.mistral.ai/api/billing/credits";

pub fn spec() -> ProviderSpec {
    ProviderSpec {
        id: ID,
        title: TITLE,
        env_hint: "VIBE_API_KEY or MISTRAL_API_KEY (+ optional MISTRAL_ADMIN_COOKIE)",
        is_configured: || env_any(ENV_KEYS).is_some(),
        fetch,
    }
}

#[derive(Debug, Deserialize)]
pub struct Whoami {
    #[serde(alias = "planType")]
    pub plan_type: Option<String>,
    #[serde(alias = "planName")]
    pub plan_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Credits {
    pub wallet_amount: f64,
    pub credit_notes_amount: Option<f64>,
    pub currency: Option<String>,
}

pub fn parse_whoami(body: &str) -> Result<Whoami, String> {
    serde_json::from_str(body).map_err(|e| format!("invalid JSON from {WHOAMI_URL}: {e}"))
}

/// Parse the tRPC envelope: `[{"result":{"data":{"json":{"usage_percentage":…,"reset_at":…}}}}]`.
pub fn parse_vibe_usage(body: &str) -> Result<(f64, Option<String>), String> {
    let v: Value = serde_json::from_str(body)
        .map_err(|e| format!("invalid JSON from vibeUsage endpoint: {e}"))?;
    let j = &v
        .get(0)
        .and_then(|r| r.get("result"))
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("json"));
    let j = j.ok_or("unexpected vibeUsage response shape")?;
    let pct = j
        .get("usage_percentage")
        .and_then(Value::as_f64)
        .ok_or("missing usage_percentage")?;
    let reset = j
        .get("reset_at")
        .and_then(Value::as_str)
        .map(str::to_string);
    Ok((pct, reset))
}

pub fn parse_credits(body: &str) -> Result<Credits, String> {
    serde_json::from_str(body).map_err(|e| format!("invalid JSON from {CREDITS_URL}: {e}"))
}

pub fn cookie_value(cookie_header: &str, name: &str) -> Option<String> {
    cookie_header.split(';').find_map(|pair| {
        let mut it = pair.trim().splitn(2, '=');
        let k = it.next()?.trim();
        let v = it.next()?.trim();
        if k == name {
            Some(v.to_string())
        } else {
            None
        }
    })
}

/// Keep only the cookies console.mistral.ai needs: csrftoken + ory_session_*.
pub fn console_cookie(cookie_header: &str) -> String {
    cookie_header
        .split(';')
        .map(str::trim)
        .filter(|pair| {
            let k = pair.split('=').next().unwrap_or("").trim();
            k == "csrftoken" || k.starts_with("ory_session")
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn fetch(http: &Http) -> Result<Report, String> {
    let key = env_any(ENV_KEYS).ok_or("no API key in environment")?;
    let auth = format!("Bearer {key}");
    let whoami_body = http.get(
        WHOAMI_URL,
        &[
            ("Authorization", auth.as_str()),
            ("Accept", "application/json"),
        ],
    )?;
    let whoami = parse_whoami(&whoami_body.1)?;
    let plan = match (whoami.plan_name, whoami.plan_type) {
        (Some(n), Some(t)) => Some(format!("{n} ({t})")),
        (Some(n), None) => Some(n),
        (None, Some(t)) => Some(t),
        (None, None) => None,
    };

    let mut metrics = Vec::new();
    match env_nonempty(ENV_COOKIE) {
        None => metrics.push(Metric {
            name: "monthly plan".to_string(),
            remaining: format!("set {ENV_COOKIE} (Cookie header from admin.mistral.ai) to fetch"),
            used_percent: None,
            reset: None,
        }),
        Some(cookie) => {
            let csrf = cookie_value(&cookie, "csrftoken");
            if let Some(csrf) = &csrf {
                let console = console_cookie(&cookie);
                let headers = [
                    ("Accept", "*/*"),
                    ("Cookie", console.as_str()),
                    ("X-CSRFToken", csrf.as_str()),
                ];
                match http.get_json(VIBE_USAGE_URL, &headers) {
                    Ok(v) => match parse_vibe_usage(&v.to_string()) {
                        Ok((pct, reset_at)) => metrics.push(Metric {
                            name: "monthly plan".to_string(),
                            remaining: format!("{:.1}% left", (100.0 - pct).max(0.0)),
                            used_percent: Some(pct),
                            reset: reset_at.and_then(|r| {
                                parse_iso8601(&r)
                                    .map(|ts| format!("at {}", format_ymd_hms(&epoch_to_utc(ts))))
                            }),
                        }),
                        Err(e) => metrics.push(Metric {
                            name: "monthly plan".to_string(),
                            remaining: format!("error: {e}"),
                            used_percent: None,
                            reset: None,
                        }),
                    },
                    Err(e) => metrics.push(Metric {
                        name: "monthly plan".to_string(),
                        remaining: format!("error: {e}"),
                        used_percent: None,
                        reset: None,
                    }),
                }
                let headers = [
                    ("Accept", "*/*"),
                    ("Cookie", cookie.as_str()),
                    ("Referer", "https://admin.mistral.ai/organization/billing"),
                    ("Origin", "https://admin.mistral.ai"),
                    ("X-CSRFTOKEN", csrf.as_str()),
                ];
                match http.get_json(CREDITS_URL, &headers) {
                    Ok(v) => match parse_credits(&v.to_string()) {
                        Ok(c) => {
                            let currency = c.currency.unwrap_or_else(|| "EUR".to_string());
                            let total = c.wallet_amount + c.credit_notes_amount.unwrap_or(0.0);
                            metrics.push(Metric {
                                name: "credits".to_string(),
                                remaining: format!("{} left", money(total, &currency)),
                                used_percent: None,
                                reset: None,
                            });
                        }
                        Err(e) => metrics.push(Metric {
                            name: "credits".to_string(),
                            remaining: format!("error: {e}"),
                            used_percent: None,
                            reset: None,
                        }),
                    },
                    Err(e) => metrics.push(Metric {
                        name: "credits".to_string(),
                        remaining: format!("error: {e}"),
                        used_percent: None,
                        reset: None,
                    }),
                }
            } else {
                metrics.push(Metric {
                    name: "monthly plan".to_string(),
                    remaining: format!("{ENV_COOKIE} has no csrftoken cookie"),
                    used_percent: None,
                    reset: None,
                });
            }
        }
    }
    Ok(Report::ok(ID, TITLE, plan, metrics))
}
