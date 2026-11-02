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
