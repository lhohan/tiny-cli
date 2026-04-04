use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use clap::Parser;
use owo_colors::{OwoColorize, Style};
use serde::Deserialize;

/// Report OpenCode model usage and costs
#[derive(Parser, Debug)]
#[command(name = "opencode-model-report")]
#[command(about = "Report OpenCode model usage and costs")]
#[command(version)]
struct Cli {
    /// Disable colored output
    #[arg(long, env = "NO_COLOR")]
    no_color: bool,
}

/// Error types for the application
#[derive(Debug)]
enum AppError {
    ConfigNotFound(PathBuf),
    HomeDirNotFound,
    #[allow(dead_code)]
    JsoncParse(String),
    JsonParse(serde_json::Error),
    Io(io::Error),
    SubprocessFailed(String, i32),
    CurlNotFound,
    OpenCodeNotFound,
    FetchFailed(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::ConfigNotFound(path) => write!(f, "missing config file: {}", path.display()),
            AppError::HomeDirNotFound => write!(f, "HOME environment variable is not set"),
            AppError::JsoncParse(msg) => write!(f, "failed to parse JSONC: {}", msg),
            AppError::JsonParse(e) => write!(f, "JSON parse error: {}", e),
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::SubprocessFailed(msg, code) => {
                write!(f, "subprocess failed (exit {}): {}", code, msg)
            }
            AppError::CurlNotFound => write!(f, "curl command not found"),
            AppError::OpenCodeNotFound => write!(f, "opencode command not found"),
            AppError::FetchFailed(msg) => write!(f, "failed to fetch: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl From<io::Error> for AppError {
    fn from(e: io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::JsonParse(e)
    }
}

/// Type alias for Result with our error type
type Result<T> = std::result::Result<T, AppError>;

/// Cost information for a model
#[derive(Debug, Default, Deserialize)]
struct Cost {
    #[serde(default)]
    input: Option<f64>,
    #[serde(default)]
    output: Option<f64>,
}

/// Provider data from models.dev API
#[derive(Debug, Deserialize)]
struct Provider {
    #[serde(default)]
    models: HashMap<String, serde_json::Value>,
}

/// Agent configuration from opencode.jsonc
#[derive(Debug, Deserialize)]
struct AgentConfig {
    #[serde(default)]
    model: Option<String>,
}

/// Main opencode configuration
#[derive(Debug, Deserialize)]
struct OpenCodeConfig {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    small_model: Option<String>,
    #[serde(default)]
    agent: HashMap<String, AgentConfig>,
}

/// Weave agent configuration
#[derive(Debug, Deserialize)]
struct WeaveAgent {
    #[serde(default)]
    model: Option<String>,
}

/// Weave configuration
#[derive(Debug, Deserialize)]
struct WeaveConfig {
    #[serde(default)]
    agents: HashMap<String, WeaveAgent>,
    #[serde(default, rename = "custom_agents")]
    custom_agents: HashMap<String, WeaveAgent>,
}

/// Strip ANSI escape sequences from text
fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '[' {
            // Skip ANSI sequence
            i += 2;
            while i < chars.len() && chars[i] != 'm' && !chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            if i < chars.len() {
                i += 1; // Skip the final letter
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Strip JSONC comments and trailing commas
fn strip_jsonc(text: &str) -> Result<String> {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut in_string = false;
    let mut escape = false;

    while i < chars.len() {
        let ch = chars[i];
        let nxt = chars.get(i + 1).copied();

        if in_string {
            out.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            i += 1;
            continue;
        }

        // Skip single-line comments
        if ch == '/' && nxt == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        out.push(ch);
        i += 1;
    }

    // Remove trailing commas before } or ]
    let mut stripped = out;
    loop {
        let updated = remove_trailing_commas(&stripped);
        if updated == stripped {
            return Ok(stripped);
        }
        stripped = updated;
    }
}

/// Remove trailing commas before closing braces/brackets
fn remove_trailing_commas(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == ',' {
            // Look ahead for } or ] with optional whitespace
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                // Skip the comma
                i += 1;
                continue;
            }
        }

        result.push(ch);
        i += 1;
    }

    result
}

/// Load and parse a JSONC file
fn load_jsonc<T: for<'de> Deserialize<'de>>(path: &PathBuf) -> Result<T> {
    let text = std::fs::read_to_string(path)?;
    let stripped = strip_jsonc(&text)?;
    let value = serde_json::from_str(&stripped)?;
    Ok(value)
}

/// Add a model usage entry
fn add_usage(active: &mut BTreeMap<String, Vec<String>>, model: Option<&str>, usage: &str) {
    let Some(model) = model else { return };
    if model.is_empty() {
        return;
    }

    let entry = active.entry(model.to_string()).or_default();
    if !entry.contains(&usage.to_string()) {
        entry.push(usage.to_string());
    }
}

/// Format a cost value for display.
fn format_cost(value: Option<f64>) -> String {
    match value {
        None => "n/a".to_string(),
        Some(v) if v == 0.0 => "0".to_string(),
        Some(v) if v.fract() == 0.0 => format!("{:.0}", v),
        Some(v) => {
            // Trim trailing zeros after decimal for compact output.
            let s = format!("{:.10}", v);
            let trimmed = s.trim_end_matches('0').trim_end_matches('.');
            trimmed.to_string()
        }
    }
}

/// Get total cost for sorting (input + output, defaults to infinity if unknown)
fn get_total_cost(costs: &HashMap<String, Cost>, model: &str) -> f64 {
    let cost = costs.get(model);
    match cost {
        Some(c) if c.input.is_some() && c.output.is_some() => c.input.unwrap() + c.output.unwrap(),
        _ => f64::INFINITY,
    }
}

/// Wrap text to fit within width, breaking at commas when possible
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if text.len() <= width {
        return vec![text.to_string()];
    }

    let parts: Vec<&str> = text.split(',').map(|s| s.trim()).collect();
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for part in parts {
        if current.is_empty() {
            current = part.to_string();
        } else if current.len() + 2 + part.len() <= width {
            current.push_str(", ");
            current.push_str(part);
        } else {
            lines.push(current);
            current = part.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

/// Run opencode models --refresh and parse the output
fn fetch_available_models() -> Result<Vec<String>> {
    let output = Command::new("opencode")
        .args(["models", "--refresh"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                AppError::OpenCodeNotFound
            } else {
                AppError::Io(e)
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::SubprocessFailed(
            stderr.to_string(),
            output.status.code().unwrap_or(4),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut seen = HashSet::new();
    let mut models = Vec::new();

    for raw_line in stdout.lines() {
        let line = strip_ansi(raw_line).trim().to_string();
        if line.is_empty() || line == "Models cache refreshed" {
            continue;
        }
        if line.contains(' ') || !line.contains('/') {
            continue;
        }
        if seen.insert(line.clone()) {
            models.push(line);
        }
    }

    Ok(models)
}

/// Fetch cost data from models.dev API
fn fetch_costs() -> Result<HashMap<String, Cost>> {
    let output = Command::new("curl")
        .args(["-fsSL", "https://models.dev/api.json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                AppError::CurlNotFound
            } else {
                AppError::Io(e)
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::FetchFailed(stderr.to_string()));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let api: HashMap<String, Provider> = serde_json::from_str(&text)?;

    let mut costs = HashMap::new();
    for (provider_id, provider) in api {
        for (model_id, model_data) in provider.models {
            let key = format!("{}/{}", provider_id, model_id);
            let cost: Cost = if let Some(cost_val) = model_data.get("cost") {
                serde_json::from_value(cost_val.clone()).unwrap_or_default()
            } else {
                Cost::default()
            };
            costs.insert(key, cost);
        }
    }

    Ok(costs)
}

/// Print an error message and exit with code
fn die(message: &str, code: i32) -> ! {
    eprintln!("ERROR: {}", message);
    std::process::exit(code);
}

/// Left-justify a string to a given width
fn ljust(s: &str, width: usize) -> String {
    if s.len() >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - s.len()))
    }
}

/// Right-justify a string to a given width
fn rjust(s: &str, width: usize) -> String {
    if s.len() >= width {
        s.to_string()
    } else {
        format!("{}{}", " ".repeat(width - s.len()), s)
    }
}

/// Find the Opencode config directory in the user's home directory.
fn opencode_config_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or(AppError::HomeDirNotFound)?;
    Ok(PathBuf::from(home).join(".config/opencode"))
}

/// Main entry point
fn run(cli: &Cli) -> Result<()> {
    // Determine if we should use colors
    let supports_color = !cli.no_color && io::stdout().is_terminal();

    let config_dir = opencode_config_dir()?;
    let opencode_cfg = config_dir.join("opencode.jsonc");
    let weave_cfg = config_dir.join("weave-opencode.jsonc");

    // Verify config files exist
    for cfg in [&opencode_cfg, &weave_cfg] {
        if !cfg.exists() {
            return Err(AppError::ConfigNotFound(cfg.clone()));
        }
    }

    // Track active model usage (BTreeMap for consistent ordering within cost groups)
    let mut active: BTreeMap<String, Vec<String>> = BTreeMap::new();

    // Load opencode config
    let opencode: OpenCodeConfig = load_jsonc(&opencode_cfg)?;
    add_usage(&mut active, opencode.model.as_deref(), "default");
    add_usage(&mut active, opencode.small_model.as_deref(), "small_model");

    for (name, cfg) in opencode.agent {
        add_usage(&mut active, cfg.model.as_deref(), &name);
    }

    // Load weave config
    let weave: WeaveConfig = load_jsonc(&weave_cfg)?;
    for (name, cfg) in weave.agents {
        add_usage(&mut active, cfg.model.as_deref(), &name);
    }
    for (name, cfg) in weave.custom_agents {
        add_usage(&mut active, cfg.model.as_deref(), &name);
    }

    // Fetch available models
    let available_models = fetch_available_models()?;

    // Compute allowed models (available but not active)
    let active_set: HashSet<&String> = active.keys().collect();
    let allowed: Vec<&String> = available_models
        .iter()
        .filter(|m| !active_set.contains(m))
        .collect();

    // Fetch cost data
    let costs = fetch_costs()?;

    // Collect all models for width calculation
    let all_models: Vec<&String> = active.keys().chain(allowed.iter().copied()).collect();

    // Calculate column widths
    let model_header = "MODEL";
    let in_header = "IN";
    let out_header = "OUT";
    let usage_header = "USAGE";
    let max_usage_width = 50;

    let model_width = std::iter::once(model_header.len())
        .chain(all_models.iter().map(|m| m.len()))
        .max()
        .unwrap_or(0);

    let in_width = std::iter::once(in_header.len())
        .chain(
            all_models
                .iter()
                .map(|m| format_cost(costs.get(*m).and_then(|c| c.input)).len()),
        )
        .max()
        .unwrap_or(0);

    let out_width = std::iter::once(out_header.len())
        .chain(
            all_models
                .iter()
                .map(|m| format_cost(costs.get(*m).and_then(|c| c.output)).len()),
        )
        .max()
        .unwrap_or(0);

    let prefix_width = model_width + 2 + in_width + 2 + out_width + 2;

    // Sort active models by usage count (descending), then by total cost (ascending),
    // then by model name for deterministic output.
    let mut sorted_active: Vec<(&String, &Vec<String>)> = active.iter().collect();
    sorted_active.sort_by(|a, b| {
        let usage_cmp = b.1.len().cmp(&a.1.len());
        if usage_cmp != std::cmp::Ordering::Equal {
            return usage_cmp;
        }

        let cost_a = get_total_cost(&costs, a.0);
        let cost_b = get_total_cost(&costs, b.0);
        let cost_cmp = cost_a
            .partial_cmp(&cost_b)
            .unwrap_or(std::cmp::Ordering::Equal);
        if cost_cmp != std::cmp::Ordering::Equal {
            return cost_cmp;
        }

        a.0.cmp(b.0)
    });

    // Print ACTIVE section
    if supports_color {
        println!("{}", "ACTIVE".style(Style::new().bold().green()));
    } else {
        println!("ACTIVE");
    }
    println!(
        "{}  {}  {}  {}",
        ljust(model_header, model_width),
        rjust(in_header, in_width),
        rjust(out_header, out_width),
        usage_header
    );
    println!(
        "{}",
        "-".repeat(model_width + 2 + in_width + 2 + out_width + 2 + max_usage_width)
    );

    for (model, usages) in sorted_active {
        let cost = costs.get(model);
        let usage_text = usages.join(", ");
        let usage_lines = wrap_text(&usage_text, max_usage_width);

        // First line with model and costs
        println!(
            "{}  {}  {}  {}",
            ljust(model, model_width),
            rjust(&format_cost(cost.and_then(|c| c.input)), in_width),
            rjust(&format_cost(cost.and_then(|c| c.output)), out_width),
            usage_lines.first().map(String::as_str).unwrap_or("")
        );

        // Continuation lines if usage wrapped
        for line in &usage_lines[1..] {
            println!("{}{}", " ".repeat(prefix_width), line);
        }
    }

    // Sort allowed models by total cost
    let mut sorted_allowed: Vec<&&String> = allowed.iter().collect();
    sorted_allowed.sort_by(|a, b| {
        let cost_a = get_total_cost(&costs, a);
        let cost_b = get_total_cost(&costs, b);
        cost_a
            .partial_cmp(&cost_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Print ALLOWED section
    println!();
    if supports_color {
        println!("{}", "ALLOWED".style(Style::new().bold().yellow()));
    } else {
        println!("ALLOWED");
    }
    println!(
        "{}  {}  {}",
        ljust(model_header, model_width),
        rjust(in_header, in_width),
        rjust(out_header, out_width)
    );
    println!("{}", "-".repeat(model_width + 2 + in_width + 2 + out_width));

    for model in sorted_allowed {
        let cost = costs.get(*model);
        println!(
            "{}  {}  {}",
            ljust(model, model_width),
            rjust(&format_cost(cost.and_then(|c| c.input)), in_width),
            rjust(&format_cost(cost.and_then(|c| c.output)), out_width)
        );
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(&cli) {
        match e {
            AppError::SubprocessFailed(stderr, code) => {
                eprintln!("ERROR: failed to refresh OpenCode models");
                let stderr = stderr.trim();
                if !stderr.is_empty() {
                    eprintln!("{}", stderr);
                }
                std::process::exit(if code == 0 { 4 } else { code });
            }
            other => die(&other.to_string(), 3),
        }
    }
}
