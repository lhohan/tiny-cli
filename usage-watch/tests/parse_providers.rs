use usage_watch::providers::{chatgpt, cookie_header, mistral, opencode, vibe};
use usage_watch::timeutil::{epoch_to_utc, fmt_duration, format_ymd_hms, parse_iso8601};

const OPENCODE_FIXTURE: &str = r#"{
  "usage": {
    "rolling": {"percent": 74.0, "resetsAt": "2026-11-03T12:00:00Z"},
    "weekly": {"percent": 27.0, "resetsAt": "2026-11-07T00:00:00Z"},
    "monthly": {"percent": 25.0, "resetsAt": "2026-11-19T07:00:00Z"}
  },
  "renewAt": "2026-12-03T00:00:00Z"
}"#;

const OPENCODE_LEGACY: &str = r#"{
  "rolling5h": {"usageDollars": 2.34, "limitDollars": 12, "usagePercent": 19.5, "resetInSec": 7200},
  "weekly": {"usageDollars": 8.91, "limitDollars": 30, "usagePercent": 29.7, "resetInSec": 345600},
  "monthly": {"usageDollars": 15.00, "limitDollars": 60, "usagePercent": 25.0, "resetInSec": 1414800},
  "subscribedAt": "2026-05-22T14:30:00Z"
}"#;

#[test]
fn opencode_parse_current_shape() {
    let u = opencode::parse(OPENCODE_FIXTURE).expect("parse");
    assert_eq!(u.rolling.used_percent, 74.0);
    assert_eq!(u.weekly.as_ref().map(|w| w.used_percent), Some(27.0));
    assert_eq!(u.renew_at.as_deref(), Some("2026-12-03T00:00:00Z"));
    let now = parse_iso8601("2026-11-03T10:00:00Z").expect("now");
    let m = opencode::metrics(&u, now);
    assert_eq!(m.len(), 3);
    assert_eq!(m[0].name, "5h window");
    assert_eq!(m[0].remaining, "26% left");
    assert_eq!(m[0].used_percent, Some(74.0));
    assert_eq!(m[0].reset.as_deref(), Some("in 2h00m"));
    assert_eq!(m[1].remaining, "73% left");
    assert_eq!(m[1].reset.as_deref(), Some("in 3d14h"));
    assert_eq!(m[2].name, "monthly");
    assert_eq!(m[2].reset.as_deref(), Some("in 15d21h"));
}

#[test]
fn opencode_parse_legacy_shape() {
    let u = opencode::parse(OPENCODE_LEGACY).expect("parse");
    assert_eq!(u.subscribed_at.as_deref(), Some("2026-05-22T14:30:00Z"));
    let m = opencode::metrics(&u, 0);
    assert_eq!(m.len(), 3);
    assert_eq!(m[0].remaining, "$9.66 of $12.00 left");
    assert_eq!(m[0].reset.as_deref(), Some("in 2h00m"));
    assert_eq!(m[1].remaining, "$21.09 of $30.00 left");
    assert_eq!(m[1].reset.as_deref(), Some("in 4d00h"));
    assert_eq!(m[2].remaining, "$45.00 of $60.00 left");
    assert_eq!(m[2].reset.as_deref(), Some("in 16d09h"));
}

const WHAM_RATE_LIMIT: &str = r#"{
  "plan_type": "plus",
  "rate_limit": {
    "allowed": true,
    "limit_reached": false,
    "primary_window": {"used_percent": 74, "limit_window_seconds": 18000, "reset_after_seconds": 6198, "reset_at": 1789801452},
    "secondary_window": {"used_percent": 27, "limit_window_seconds": 604800, "reset_after_seconds": 11991, "reset_at": 1789807245}
  }
}"#;

const WHAM_RATE_LIMITS_ARRAY: &str = r#"{
  "plan_type": "go",
  "rate_limits": [
    {"id": "primary", "name": "Weekly", "used_percent": 6, "limit_window_seconds": 604800, "reset_after_seconds": 3600},
    {"id": "feature:codex_spark", "name": "GPT-5.3-Codex-Spark", "used_percent": 0, "reset_after_seconds": 7200}
  ]
}"#;

const WHAM_TOP_LEVEL: &str = r#"{
  "plan_type": "pro",
  "primary_window": {"used_percent": 10, "limit_window_seconds": 18000, "reset_at": 1789801452},
  "secondary_window": null
}"#;

#[test]
fn chatgpt_parse_rate_limit_shape() {
    let (plan, wins) = chatgpt::parse(WHAM_RATE_LIMIT).expect("parse");
    assert_eq!(plan.as_deref(), Some("plus"));
    assert_eq!(wins.len(), 2);
    assert_eq!(wins[0].label, "primary");
    assert_eq!(wins[0].used_percent, 74.0);
    let m = chatgpt::metrics(&wins, 1789801452 - 6198);
    assert_eq!(m[0].remaining, "26% left");
    assert_eq!(m[0].used_percent, Some(74.0));
    assert_eq!(m[0].reset.as_deref(), Some("in 1h43m"));
    assert_eq!(m[1].name, "secondary (7d00h)");
    assert_eq!(m[1].remaining, "73% left");
}

#[test]
fn chatgpt_parse_rate_limits_array_shape() {
    let (plan, wins) = chatgpt::parse(WHAM_RATE_LIMITS_ARRAY).expect("parse");
    assert_eq!(plan.as_deref(), Some("go"));
    assert_eq!(wins.len(), 2);
    assert_eq!(wins[0].label, "primary");
    assert_eq!(wins[1].label, "feature:codex_spark");
    assert_eq!(wins[1].used_percent, 0.0);
}

#[test]
fn chatgpt_parse_top_level_windows_and_null_window() {
    let (plan, wins) = chatgpt::parse(WHAM_TOP_LEVEL).expect("parse");
    assert_eq!(plan.as_deref(), Some("pro"));
    assert_eq!(wins.len(), 1);
    assert_eq!(wins[0].label, "primary");
}

#[test]
fn chatgpt_reset_from_epoch_when_no_reset_after() {
    let (plan, wins) = chatgpt::parse(WHAM_TOP_LEVEL).expect("parse");
    assert_eq!(plan.as_deref(), Some("pro"));
    let now = 1789801452 - 3600;
    let m = chatgpt::metrics(&wins, now);
    assert_eq!(m[0].reset.as_deref(), Some("in 1h00m"));
    let m_late = chatgpt::metrics(&wins, 1789801452 + 10);
    assert!(m_late[0].reset.as_deref().unwrap().starts_with("at "));
}

const MISTRAL_SUB: &str = r#"{"plan_name": "INDIVIDUAL", "payg_enabled": false, "vibe_payg_enabled": false, "enterprise": false}"#;
const MISTRAL_USAGE: &str = r#"{
  "currency": "EUR",
  "completion": {"models": {"mistral-large-3": {"input": [{"value": 1000000}], "output": [{"value": 500000}]}}},
  "chat": {"models": {"mistral-small-4": {"input": [{"value": 200000}], "output": [{"value": 100000}]}}},
  "vibe_code": {"completion": {"models": {"codestral": {"input": [{"value": 1000}], "output": [{"value": 500}]}}}},
  "audio": {"models": {"voxtral": {"input": [{"value": 999999}]}}}
}"#;
const MISTRAL_CREDITS: &str =
    r#"{"wallet_amount": 12.34, "credit_notes_amount": 1.0, "currency": "EUR"}"#;

#[test]
fn mistral_parse_and_metrics() {
    let sub = mistral::parse_subscription(MISTRAL_SUB).expect("parse sub");
    assert_eq!(sub.plan_name.as_deref(), Some("INDIVIDUAL"));
    assert_eq!(sub.payg_enabled, Some(false));
    let usage = mistral::parse_usage(MISTRAL_USAGE).expect("parse usage");
    assert_eq!(usage.input_tokens, 1_201_000);
    assert_eq!(usage.output_tokens, 600_500);
    let credits = mistral::parse_credits(MISTRAL_CREDITS).expect("parse credits");
    let m = mistral::metrics(Some(&usage), Some(&credits), None);
    let tokens = m
        .iter()
        .find(|m| m.name == "tokens this month")
        .expect("tokens");
    assert_eq!(tokens.remaining, "1.20M in / 600.5K out");
    let credits = m.iter().find(|m| m.name == "credits").expect("credits");
    assert_eq!(credits.remaining, "€13.34 left");
}

#[test]
fn mistral_metrics_without_cookie() {
    let m = mistral::metrics(None, None, None);
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].name, "usage");
    assert!(m[0].remaining.contains("MISTRAL_ADMIN_COOKIE"));
}

const VIBE_WHOAMI: &str =
    r#"{"plan_type": "CHAT", "plan_name": "PRO", "vibe_base": "https://chat.mistral.ai"}"#;
const VIBE_USAGE: &str = r#"[{"result": {"data": {"json": {"usage_percentage": 58.6, "reset_at": "2026-12-01T00:00:00Z"}}}}]"#;
const VIBE_CREDITS: &str = r#"{"wallet_amount": 25.5, "credit_notes_amount": 1.5, "ongoing_usage_balance": 0, "currency": "EUR"}"#;

#[test]
fn vibe_parse_whoami() {
    let w = vibe::parse_whoami(VIBE_WHOAMI).expect("parse");
    assert_eq!(w.plan_name.as_deref(), Some("PRO"));
    assert_eq!(w.plan_type.as_deref(), Some("CHAT"));
}

#[test]
fn vibe_parse_usage_envelope() {
    let (pct, reset) = vibe::parse_vibe_usage(VIBE_USAGE).expect("parse");
    assert_eq!(pct, 58.6);
    assert_eq!(reset.as_deref(), Some("2026-12-01T00:00:00Z"));
}

#[test]
fn vibe_parse_credits() {
    let c = vibe::parse_credits(VIBE_CREDITS).expect("parse");
    assert_eq!(c.wallet_amount, 25.5);
    assert_eq!(c.credit_notes_amount, Some(1.5));
    assert_eq!(c.currency.as_deref(), Some("EUR"));
}

#[test]
fn vibe_cookie_helpers() {
    let cookie = "ory_session_abc=xyz; csrftoken=tok123; other=nope";
    assert_eq!(
        vibe::cookie_value(cookie, "csrftoken").as_deref(),
        Some("tok123")
    );
    assert_eq!(vibe::cookie_value(cookie, "missing"), None);
    assert_eq!(
        vibe::console_cookie(cookie),
        "ory_session_abc=xyz; csrftoken=tok123"
    );
}

#[test]
fn cookie_header_strips_pasted_label_and_newlines() {
    assert_eq!(
        cookie_header("Cookie: ory_session_abc=xyz; csrftoken=tok123"),
        "ory_session_abc=xyz; csrftoken=tok123"
    );
    assert_eq!(cookie_header("cookie: a=b"), "a=b");
    assert_eq!(cookie_header("  a=b;\ncsrftoken=c  "), "a=b; csrftoken=c");
    assert_eq!(cookie_header("a=b"), "a=b");
    // A cookie actually named `cookie` must survive untouched.
    assert_eq!(cookie_header("cookie=1; a=b"), "cookie=1; a=b");
}

#[test]
fn timeutil_epoch_zero() {
    let u = epoch_to_utc(0);
    assert_eq!(format_ymd_hms(&u), "1970-01-01 00:00:00 UTC");
}

#[test]
fn timeutil_iso8601_roundtrip() {
    let ts = parse_iso8601("2026-11-02T18:30:52Z").expect("parse Z");
    assert_eq!(format_ymd_hms(&epoch_to_utc(ts)), "2026-11-02 18:30:52 UTC");
    let with_offset = parse_iso8601("2026-11-02T20:30:52+02:00").expect("parse offset");
    assert_eq!(with_offset, ts);
    let frac = parse_iso8601("2026-11-02T18:30:52.500Z").expect("parse frac");
    assert_eq!(frac, ts);
    assert_eq!(parse_iso8601("not a date"), None);
}

#[test]
fn timeutil_fmt_duration() {
    assert_eq!(fmt_duration(1414800), "16d09h");
    assert_eq!(fmt_duration(7200), "2h00m");
    assert_eq!(fmt_duration(90), "1m30s");
    assert_eq!(fmt_duration(30), "30s");
}
