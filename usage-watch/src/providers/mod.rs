//! Subscription providers. Each provider reads its credentials from environment
//! variables and produces a [`Report`](crate::model::Report).

pub mod chatgpt;
pub mod mistral;
pub mod opencode;
pub mod vibe;

use crate::http::Http;
use crate::model::Report;

pub struct ProviderSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub env_hint: &'static str,
    pub is_configured: fn() -> bool,
    pub fetch: fn(&Http) -> Result<Report, String>,
}

impl Clone for ProviderSpec {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for ProviderSpec {}

pub fn all() -> Vec<ProviderSpec> {
    vec![
        opencode::spec(),
        chatgpt::spec(),
        mistral::spec(),
        vibe::spec(),
    ]
}

pub fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub fn env_any(names: &[&str]) -> Option<String> {
    names.iter().find_map(|n| env_nonempty(n))
}

/// Normalise a pasted `Cookie:` header into a header value.
///
/// People copy the whole header line out of DevTools, so the value often
/// starts with a literal `Cookie:` label. Sending that as the value of a
/// `Cookie` header produces `Cookie: Cookie: …`, which servers reject by
/// bouncing to the login page. Fold newlines too, so a multi-line paste
/// cannot split the header.
pub fn cookie_header(raw: &str) -> String {
    let folded = raw.replace(['\r', '\n'], " ");
    let trimmed = folded.trim();
    let value = match trimmed.get(..7) {
        Some(prefix) if prefix.eq_ignore_ascii_case("cookie:") => &trimmed[7..],
        _ => trimmed,
    };
    value.trim().to_string()
}

/// Read one cookie value out of a full `Cookie:` header.
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
