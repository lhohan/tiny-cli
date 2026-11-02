//! usage-watch library
//!
//! Shows remaining usage for AI subscriptions (OpenCode Go, ChatGPT Go/Plus,
//! Mistral La Plateforme, Mistral Vibe) using credentials from environment
//! variables.
//!
//! ## Module Structure
//!
//! - `args`: command-line parsing
//! - `http`: minimal HTTP GET client
//! - `timeutil`: UTC calendar math without external crates
//! - `model`: report/metric types shared by all providers
//! - `providers`: one module per subscription provider
//! - `render`: plain-text and JSON output

pub mod args;
pub mod http;
pub mod model;
pub mod providers;
pub mod render;
pub mod timeutil;
