use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::de::DeserializeOwned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigBundle {
    pub opencode: OpenCodeConfig,
    pub weave: Option<WeaveConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct OpenCodeConfig {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub small_model: Option<String>,
    #[serde(default)]
    pub agent: HashMap<String, AgentConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct AgentConfig {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct WeaveConfig {
    #[serde(default)]
    pub agents: HashMap<String, AgentConfig>,
    #[serde(default, rename = "custom_agents")]
    pub custom_agents: HashMap<String, AgentConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    MissingConfig(PathBuf),
    Io(String),
    Parse(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingConfig(path) => {
                write!(f, "missing config file: {}", path.display())
            }
            ConfigError::Io(msg) => write!(f, "IO error: {}", msg),
            ConfigError::Parse(msg) => write!(f, "JSONC parse error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    Config(ConfigError),
    OpenCodeNotFound,
    RefreshFailed { stderr: String, code: i32 },
    CurlNotFound,
    FetchFailed(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Config(err) => write!(f, "{}", err),
            LoadError::OpenCodeNotFound => write!(f, "opencode command not found"),
            LoadError::RefreshFailed { stderr, code } => {
                write!(
                    f,
                    "failed to refresh OpenCode models (exit {}): {}",
                    code, stderr
                )
            }
            LoadError::CurlNotFound => write!(f, "curl command not found"),
            LoadError::FetchFailed(msg) => write!(f, "failed to fetch model costs: {}", msg),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<ConfigError> for LoadError {
    fn from(value: ConfigError) -> Self {
        LoadError::Config(value)
    }
}

impl LoadError {
    pub fn exit_code(&self) -> i32 {
        match self {
            LoadError::RefreshFailed { code, .. } => *code,
            _ => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    ActiveFirst,
    CostAsc,
    CostDesc,
    ModelName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageSource {
    OpenCodeDefault,
    OpenCodeCustom,
    Weave,
    WeaveCustom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageLabel {
    pub label: String,
    pub source: UsageSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelRow {
    pub model: String,
    pub provider: String,
    pub model_name: String,
    pub active: bool,
    pub input_cost: Option<f64>,
    pub output_cost: Option<f64>,
    pub usage: Vec<UsageLabel>,
}

impl ModelRow {
    pub fn total_cost(&self) -> Option<f64> {
        Some(self.input_cost? + self.output_cost?)
    }
}

#[derive(Debug, Default, Clone)]
pub struct ReportInput {
    pub active_usage: Vec<(String, Vec<UsageLabel>)>,
    pub available_models: Vec<String>,
    pub costs: Vec<(String, Option<f64>, Option<f64>)>,
}

pub fn build_rows(input: ReportInput, sort_mode: SortMode) -> Vec<ModelRow> {
    let mut usage_by_model: HashMap<String, Vec<UsageLabel>> = HashMap::new();
    for (model, mut usages) in input.active_usage {
        usages.sort_by(|a, b| {
            a.label
                .cmp(&b.label)
                .then_with(|| source_rank(a.source).cmp(&source_rank(b.source)))
        });
        usage_by_model.insert(model, usages);
    }

    let costs: HashMap<String, (Option<f64>, Option<f64>)> = input
        .costs
        .into_iter()
        .map(|(model, input_cost, output_cost)| (model, (input_cost, output_cost)))
        .collect();
    let active_models: HashSet<String> = usage_by_model.keys().cloned().collect();

    let mut rows: Vec<ModelRow> = input
        .available_models
        .into_iter()
        .map(|model| {
            let usage = usage_by_model.remove(&model).unwrap_or_default();
            let (input_cost, output_cost) = costs.get(&model).copied().unwrap_or((None, None));
            let (provider, model_name) = split_model_id(&model);
            ModelRow {
                active: active_models.contains(&model),
                model,
                provider,
                model_name,
                input_cost,
                output_cost,
                usage,
            }
        })
        .collect();

    rows.sort_by(|a, b| compare_rows(a, b, sort_mode));
    rows
}

pub fn compare_model_names(a: &str, b: &str) -> Ordering {
    a.cmp(b)
}

pub fn parse_jsonc<T: DeserializeOwned>(text: &str) -> serde_json::Result<T> {
    let stripped = strip_jsonc(text);
    serde_json::from_str(&stripped)
}

pub fn resolve_config_home(override_home: Option<&Path>) -> Result<PathBuf, ConfigError> {
    if let Some(path) = override_home {
        return Ok(path.to_path_buf());
    }

    let home = std::env::var_os("HOME")
        .ok_or_else(|| ConfigError::Io("HOME environment variable is not set".to_string()))?;
    Ok(PathBuf::from(home).join(".config/opencode"))
}

pub fn load_config_bundle(_home_dir: &Path) -> Result<ConfigBundle, ConfigError> {
    let opencode_path = _home_dir.join("opencode.jsonc");
    if !opencode_path.exists() {
        return Err(ConfigError::MissingConfig(opencode_path));
    }

    let opencode_text =
        std::fs::read_to_string(&opencode_path).map_err(|e| ConfigError::Io(e.to_string()))?;
    let opencode: OpenCodeConfig =
        parse_jsonc(&opencode_text).map_err(|e| ConfigError::Parse(e.to_string()))?;

    let weave_path = _home_dir.join("weave-opencode.jsonc");
    let weave = if weave_path.exists() {
        let weave_text =
            std::fs::read_to_string(&weave_path).map_err(|e| ConfigError::Io(e.to_string()))?;
        Some(parse_jsonc(&weave_text).map_err(|e| ConfigError::Parse(e.to_string()))?)
    } else {
        None
    };

    Ok(ConfigBundle { opencode, weave })
}

pub fn render_report_rows(_rows: &[ModelRow]) -> Vec<String> {
    let rows = _rows;
    let provider_width = std::iter::once("PROVIDER".len())
        .chain(rows.iter().map(|row| row.provider.len()))
        .max()
        .unwrap_or(0);
    let model_width = std::iter::once("MODEL".len())
        .chain(rows.iter().map(|row| row.model_name.len()))
        .max()
        .unwrap_or(0);
    let active_width = "ACTIVE".len();
    let in_width = std::iter::once("IN".len())
        .chain(rows.iter().map(|row| format_cost(row.input_cost).len()))
        .max()
        .unwrap_or(0);
    let out_width = std::iter::once("OUT".len())
        .chain(rows.iter().map(|row| format_cost(row.output_cost).len()))
        .max()
        .unwrap_or(0);
    let prefix_width =
        provider_width + 2 + model_width + 2 + active_width + 2 + in_width + 2 + out_width + 2;

    let mut lines = Vec::new();
    lines.push(format!(
        "{}  {}  {}  {}  {}  USAGE",
        ljust("PROVIDER", provider_width),
        ljust("MODEL", model_width),
        ljust("ACTIVE", active_width),
        rjust("IN", in_width),
        rjust("OUT", out_width)
    ));

    for row in rows {
        let usage_text = row
            .usage
            .iter()
            .map(|usage| usage.label.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let usage_lines = wrap_usage(&usage_text, 50);
        let first_usage = usage_lines.first().map(String::as_str).unwrap_or("");

        lines.push(format!(
            "{}  {}  {}  {}  {}  {}",
            ljust(&row.provider, provider_width),
            ljust(&row.model_name, model_width),
            ljust(if row.active { "yes" } else { "no" }, active_width),
            rjust(&format_cost(row.input_cost), in_width),
            rjust(&format_cost(row.output_cost), out_width),
            first_usage
        ));

        for continuation in usage_lines.iter().skip(1) {
            lines.push(format!("{}{}", " ".repeat(prefix_width), continuation));
        }
    }

    lines
}

pub fn load_report_rows(home_dir: &Path) -> Result<Vec<ModelRow>, LoadError> {
    let bundle = load_config_bundle(home_dir)?;
    let active_usage = collect_active_usage(&bundle);
    let available_models = fetch_available_models()?;
    let costs = fetch_costs()?;

    Ok(build_rows(
        ReportInput {
            active_usage,
            available_models,
            costs,
        },
        SortMode::ActiveFirst,
    ))
}

pub mod runtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiKey {
    Quit,
    Refresh,
    CycleSort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiAction {
    None,
    Quit,
    Refresh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    Loading,
    Ready,
    Refreshing,
}

#[derive(Debug, Clone)]
pub struct UiState {
    pub sort_mode: SortMode,
    pub mode: UiMode,
    pub status: String,
    pub snapshot: Vec<ModelRow>,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            sort_mode: SortMode::ActiveFirst,
            mode: UiMode::Loading,
            status: "Loading model data...".to_string(),
            snapshot: Vec::new(),
        }
    }

    pub fn handle_key(&mut self, key: UiKey) -> UiAction {
        match key {
            UiKey::Quit => UiAction::Quit,
            UiKey::Refresh => {
                self.set_refreshing();
                UiAction::Refresh
            }
            UiKey::CycleSort => {
                self.sort_mode = match self.sort_mode {
                    SortMode::ActiveFirst => SortMode::CostAsc,
                    SortMode::CostAsc => SortMode::CostDesc,
                    SortMode::CostDesc => SortMode::ModelName,
                    SortMode::ModelName => SortMode::ActiveFirst,
                };
                UiAction::None
            }
        }
    }

    pub fn apply_snapshot(&mut self, rows: Vec<ModelRow>) {
        self.snapshot = rows;
        self.mode = UiMode::Ready;
        self.status = "Loaded model data".to_string();
    }

    pub fn apply_refresh_error(&mut self, message: String) {
        self.mode = UiMode::Ready;
        self.status = message;
    }

    pub fn set_refreshing(&mut self) {
        self.mode = UiMode::Refreshing;
        self.status = "Refreshing model data...".to_string();
    }

    pub fn visible_rows(&self) -> Vec<ModelRow> {
        let mut rows = self.snapshot.clone();
        rows.sort_by(|a, b| compare_rows(a, b, self.sort_mode));
        rows
    }
}

pub fn extract_available_models(stdout: &str) -> Vec<String> {
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

    models
}

pub fn parse_costs_from_api_json(
    _text: &str,
) -> serde_json::Result<HashMap<String, (Option<f64>, Option<f64>)>> {
    #[derive(Debug, serde::Deserialize)]
    struct Provider {
        #[serde(default)]
        models: HashMap<String, serde_json::Value>,
    }

    let api: HashMap<String, Provider> = serde_json::from_str(_text)?;
    let mut costs = HashMap::new();

    for (provider_id, provider) in api {
        for (model_id, model_data) in provider.models {
            let key = format!("{}/{}", provider_id, model_id);
            let cost = model_data
                .get("cost")
                .and_then(|value| serde_json::from_value::<CostWire>(value.clone()).ok())
                .map(|cost| (cost.input, cost.output))
                .unwrap_or((None, None));
            costs.insert(key, cost);
        }
    }

    Ok(costs)
}

fn source_rank(source: UsageSource) -> u8 {
    match source {
        UsageSource::OpenCodeDefault => 0,
        UsageSource::OpenCodeCustom => 1,
        UsageSource::Weave => 2,
        UsageSource::WeaveCustom => 3,
    }
}

fn compare_rows(a: &ModelRow, b: &ModelRow, mode: SortMode) -> Ordering {
    let active_cmp = b.active.cmp(&a.active);
    let cost_cmp = compare_costs(a.total_cost(), b.total_cost());
    let name_cmp = compare_model_names(&a.model, &b.model);

    match mode {
        SortMode::ActiveFirst => active_cmp.then(cost_cmp).then(name_cmp),
        SortMode::CostAsc => cost_cmp.then(name_cmp),
        SortMode::CostDesc => compare_costs_desc(a.total_cost(), b.total_cost()).then(name_cmp),
        SortMode::ModelName => name_cmp,
    }
}

pub fn split_model_id(model: &str) -> (String, String) {
    match model.split_once('/') {
        Some((provider, model_name)) => (provider.to_string(), model_name.to_string()),
        None => (String::new(), model.to_string()),
    }
}

fn compare_costs(a: Option<f64>, b: Option<f64>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

fn compare_costs_desc(a: Option<f64>, b: Option<f64>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => b.partial_cmp(&a).unwrap_or(Ordering::Equal),
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

fn collect_active_usage(bundle: &ConfigBundle) -> Vec<(String, Vec<UsageLabel>)> {
    let mut active: HashMap<String, Vec<UsageLabel>> = HashMap::new();

    if let Some(model) = bundle.opencode.model.as_deref() {
        record_usage(
            &mut active,
            model,
            "default".to_string(),
            UsageSource::OpenCodeDefault,
        );
    }
    if let Some(model) = bundle.opencode.small_model.as_deref() {
        record_usage(
            &mut active,
            model,
            "small_model".to_string(),
            UsageSource::OpenCodeDefault,
        );
    }

    for (name, cfg) in &bundle.opencode.agent {
        if let Some(model) = cfg.model.as_deref() {
            record_usage(
                &mut active,
                model,
                name.to_string(),
                UsageSource::OpenCodeCustom,
            );
        }
    }

    if let Some(weave) = bundle.weave.as_ref() {
        for (name, cfg) in &weave.agents {
            if let Some(model) = cfg.model.as_deref() {
                record_usage(
                    &mut active,
                    model,
                    weave_usage_label(name, cfg),
                    UsageSource::Weave,
                );
            }
        }
        for (name, cfg) in &weave.custom_agents {
            if let Some(model) = cfg.model.as_deref() {
                record_usage(
                    &mut active,
                    model,
                    weave_usage_label(name, cfg),
                    UsageSource::WeaveCustom,
                );
            }
        }
    }

    active.into_iter().collect()
}

fn record_usage(
    active: &mut HashMap<String, Vec<UsageLabel>>,
    model: &str,
    label: String,
    source: UsageSource,
) {
    active
        .entry(model.to_string())
        .or_default()
        .push(UsageLabel { label, source });
}

fn weave_usage_label(name: &str, cfg: &AgentConfig) -> String {
    cfg.display_name.as_deref().unwrap_or(name).to_string()
}

fn fetch_available_models() -> Result<Vec<String>, LoadError> {
    let output = Command::new("opencode")
        .args(["models", "--refresh"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                LoadError::OpenCodeNotFound
            } else {
                LoadError::RefreshFailed {
                    stderr: err.to_string(),
                    code: 4,
                }
            }
        })?;

    if !output.status.success() {
        return Err(LoadError::RefreshFailed {
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            code: output.status.code().unwrap_or(4),
        });
    }

    Ok(extract_available_models(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn fetch_costs() -> Result<Vec<(String, Option<f64>, Option<f64>)>, LoadError> {
    let output = Command::new("curl")
        .args(["-fsSL", "https://models.dev/api.json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                LoadError::CurlNotFound
            } else {
                LoadError::FetchFailed(err.to_string())
            }
        })?;

    if !output.status.success() {
        return Err(LoadError::FetchFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let costs = parse_costs_from_api_json(&String::from_utf8_lossy(&output.stdout))
        .map_err(|err| LoadError::FetchFailed(err.to_string()))?;

    Ok(costs
        .into_iter()
        .map(|(model, (input_cost, output_cost))| (model, input_cost, output_cost))
        .collect())
}

fn format_cost(value: Option<f64>) -> String {
    match value {
        None => "n/a".to_string(),
        Some(v) if v == 0.0 => "0".to_string(),
        Some(v) if v.fract() == 0.0 => format!("{:.0}", v),
        Some(v) => format!("{:0.10}", v)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string(),
    }
}

fn ljust(text: &str, width: usize) -> String {
    if text.len() >= width {
        text.to_string()
    } else {
        format!("{}{}", text, " ".repeat(width - text.len()))
    }
}

fn rjust(text: &str, width: usize) -> String {
    if text.len() >= width {
        text.to_string()
    } else {
        format!("{}{}", " ".repeat(width - text.len()), text)
    }
}

fn wrap_usage(text: &str, width: usize) -> Vec<String> {
    if text.len() <= width {
        return vec![text.to_string()];
    }

    let parts: Vec<&str> = text.split(',').map(|part| part.trim()).collect();
    let mut lines = Vec::new();
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

fn strip_jsonc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut in_string = false;
    let mut escape = false;

    while i < chars.len() {
        let ch = chars[i];
        let next = chars.get(i + 1).copied();

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

        if ch == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        out.push(ch);
        i += 1;
    }

    remove_trailing_commas(&out)
}

fn remove_trailing_commas(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == ',' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                i += 1;
                continue;
            }
        }

        result.push(ch);
        i += 1;
    }

    result
}

fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '[' {
            i += 2;
            while i < chars.len() && chars[i] != 'm' && !chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

#[derive(Debug, serde::Deserialize)]
struct CostWire {
    #[serde(default)]
    input: Option<f64>,
    #[serde(default)]
    output: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::{
        build_rows, collect_active_usage, extract_available_models, load_config_bundle,
        parse_costs_from_api_json, parse_jsonc, render_report_rows, resolve_config_home,
        AgentConfig, ConfigBundle, ModelRow, OpenCodeConfig, ReportInput, SortMode, UiAction,
        UiKey, UiMode, UiState, UsageLabel, UsageSource, WeaveConfig,
    };
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    #[derive(Debug, Deserialize, PartialEq)]
    struct SampleConfig {
        name: String,
        values: Vec<u8>,
    }

    #[test]
    fn jsonc_should_ignore_comments_and_trailing_commas() {
        let parsed: SampleConfig = parse_jsonc(
            r#"
            {
              // comment to ignore
              "name": "demo",
              "values": [1, 2, 3,],
            }
            "#,
        )
        .unwrap();

        assert_eq!(
            parsed,
            SampleConfig {
                name: "demo".to_string(),
                values: vec![1, 2, 3],
            }
        );
    }

    #[test]
    fn refresh_output_should_ignore_non_model_lines_and_duplicates() {
        let models = extract_available_models(
            "\u{1b}[32mprovider/alpha\u{1b}[0m\nModels cache refreshed\nnot-a-model\nprovider/beta\nprovider/alpha\n",
        );

        assert_eq!(models, vec!["provider/alpha", "provider/beta"]);
    }

    #[test]
    fn costs_json_should_map_provider_model_keys_to_input_and_output() {
        let costs = parse_costs_from_api_json(
            r#"
            {
              "provider": {
                "models": {
                  "alpha": { "cost": { "input": 1.25, "output": 2.5 } },
                  "beta": { "cost": { "input": 3.0 } },
                  "gamma": {}
                }
              }
            }
            "#,
        )
        .unwrap();

        assert_eq!(costs.get("provider/alpha"), Some(&(Some(1.25), Some(2.5))));
        assert_eq!(costs.get("provider/beta"), Some(&(Some(3.0), None)));
        assert_eq!(costs.get("provider/gamma"), Some(&(None, None)));
    }

    #[test]
    fn config_home_should_use_override_path_when_provided() {
        let path = PathBuf::from("/tmp/custom-opencode-home");
        let resolved = resolve_config_home(Some(&path)).unwrap();
        assert_eq!(resolved, path);
    }

    #[test]
    fn config_bundle_should_load_required_and_optional_files() {
        let home = make_temp_home();
        fs::write(
            home.join("opencode.jsonc"),
            r#"
            {
              // default model
              "model": "provider/alpha",
              "small_model": "provider/beta",
              "agent": {
                "coder": { "model": "provider/gamma", },
              },
            }
            "#,
        )
        .unwrap();
        fs::write(
            home.join("weave-opencode.jsonc"),
            r#"
            {
              "agents": {
                "reviewer": { "model": "provider/delta" }
              },
              "custom_agents": {
                "ops": { "model": "provider/epsilon" }
              }
            }
            "#,
        )
        .unwrap();

        let bundle = load_config_bundle(&home).unwrap();
        assert_eq!(bundle.opencode.model.as_deref(), Some("provider/alpha"));
        assert_eq!(
            bundle.opencode.small_model.as_deref(),
            Some("provider/beta")
        );
        assert_eq!(
            bundle
                .opencode
                .agent
                .get("coder")
                .and_then(|a| a.model.as_deref()),
            Some("provider/gamma")
        );
        let weave = bundle.weave.expect("weave config");
        assert_eq!(
            weave
                .agents
                .get("reviewer")
                .and_then(|a| a.model.as_deref()),
            Some("provider/delta")
        );
        assert_eq!(
            weave
                .custom_agents
                .get("ops")
                .and_then(|a| a.model.as_deref()),
            Some("provider/epsilon")
        );
    }

    #[test]
    fn report_rows_should_render_unified_table_with_wrapped_usage() {
        let rows = build_rows(
            ReportInput {
                active_usage: vec![(
                    "provider/alpha".to_string(),
                    vec![
                        UsageLabel {
                            label: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                            source: UsageSource::OpenCodeDefault,
                        },
                        UsageLabel {
                            label: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
                            source: UsageSource::Weave,
                        },
                    ],
                )],
                available_models: vec!["provider/alpha".to_string(), "provider/beta".to_string()],
                costs: vec![
                    ("provider/alpha".to_string(), Some(1.0), Some(2.0)),
                    ("provider/beta".to_string(), Some(3.0), Some(4.0)),
                ],
            },
            SortMode::ModelName,
        );

        let lines = render_report_rows(&rows);
        assert_eq!(lines[0], "PROVIDER  MODEL  ACTIVE  IN  OUT  USAGE");
        assert!(lines.iter().any(|line| line.contains("provider  alpha")));
        assert!(lines.iter().any(|line| line.contains("yes")));
        assert!(lines.iter().any(|line| line.contains("no")));
        assert!(lines
            .iter()
            .any(|line| line.contains("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")));
        assert!(lines.len() > 3);
        assert!(lines[2].starts_with(&" ".repeat(34)));
    }

    #[test]
    fn ui_state_should_cycle_sort_modes_when_pressing_s() {
        let mut state = UiState::new();
        assert_eq!(state.mode, UiMode::Loading);

        let sequence = [
            SortMode::CostAsc,
            SortMode::CostDesc,
            SortMode::ModelName,
            SortMode::ActiveFirst,
        ];

        for expected in sequence {
            let action = state.handle_key(UiKey::CycleSort);
            assert_eq!(action, UiAction::None);
            assert_eq!(state.sort_mode, expected);
        }
    }

    #[test]
    fn ui_state_should_keep_rows_when_refresh_fails() {
        let mut state = UiState::new();
        state.apply_snapshot(vec![ModelRow {
            model: "provider/alpha".to_string(),
            provider: "provider".to_string(),
            model_name: "alpha".to_string(),
            active: true,
            input_cost: Some(1.0),
            output_cost: Some(2.0),
            usage: vec![],
        }]);
        state.set_refreshing();
        state.apply_refresh_error("refresh failed".to_string());

        assert_eq!(state.mode, UiMode::Ready);
        assert_eq!(state.status, "refresh failed");
        assert_eq!(state.visible_rows().len(), 1);
    }

    #[test]
    fn active_usage_should_distinguish_opencode_default_custom_and_weave_sources() {
        let bundle = ConfigBundle {
            opencode: OpenCodeConfig {
                model: Some("provider/alpha".to_string()),
                small_model: Some("provider/beta".to_string()),
                agent: [(
                    "builder".to_string(),
                    super::AgentConfig {
                        model: Some("provider/gamma".to_string()),
                        display_name: None,
                    },
                )]
                .into_iter()
                .collect(),
            },
            weave: Some(super::WeaveConfig {
                agents: [(
                    "reviewer".to_string(),
                    super::AgentConfig {
                        model: Some("provider/delta".to_string()),
                        display_name: None,
                    },
                )]
                .into_iter()
                .collect(),
                custom_agents: [(
                    "ops".to_string(),
                    super::AgentConfig {
                        model: Some("provider/epsilon".to_string()),
                        display_name: None,
                    },
                )]
                .into_iter()
                .collect(),
            }),
        };

        let usage = collect_active_usage(&bundle);
        let by_model: std::collections::HashMap<_, _> = usage.into_iter().collect();

        assert_eq!(
            by_model.get("provider/alpha").unwrap()[0].source,
            UsageSource::OpenCodeDefault
        );
        assert_eq!(
            by_model.get("provider/beta").unwrap()[0].source,
            UsageSource::OpenCodeDefault
        );
        assert_eq!(
            by_model.get("provider/gamma").unwrap()[0].source,
            UsageSource::OpenCodeCustom
        );
        assert_eq!(
            by_model.get("provider/delta").unwrap()[0].source,
            UsageSource::Weave
        );
        assert_eq!(
            by_model.get("provider/epsilon").unwrap()[0].source,
            UsageSource::WeaveCustom
        );
    }

    #[test]
    fn active_usage_should_distinguish_weave_agents_from_custom_agents() {
        let bundle = ConfigBundle {
            opencode: OpenCodeConfig {
                model: None,
                small_model: None,
                agent: HashMap::new(),
            },
            weave: Some(WeaveConfig {
                agents: [(
                    "reviewer".to_string(),
                    AgentConfig {
                        model: Some("provider/delta".to_string()),
                        display_name: None,
                    },
                )]
                .into_iter()
                .collect(),
                custom_agents: [(
                    "ops".to_string(),
                    AgentConfig {
                        model: Some("provider/epsilon".to_string()),
                        display_name: None,
                    },
                )]
                .into_iter()
                .collect(),
            }),
        };

        let usage = collect_active_usage(&bundle);
        let by_model: std::collections::HashMap<_, _> = usage.into_iter().collect();

        assert_eq!(
            by_model.get("provider/delta").unwrap()[0].source,
            UsageSource::Weave
        );
        assert_eq!(
            by_model.get("provider/epsilon").unwrap()[0].source,
            UsageSource::WeaveCustom
        );
    }

    #[test]
    fn active_usage_should_prefer_weave_display_names_when_configured() {
        let home = make_temp_home();
        fs::write(
            home.join("opencode.jsonc"),
            r#"{
                "agent": {}
            }"#,
        )
        .unwrap();
        fs::write(
            home.join("weave-opencode.jsonc"),
            r#"{
                "agents": {
                    "reviewer": {
                        "model": "provider/delta",
                        "display_name": "Review Bot"
                    }
                },
                "custom_agents": {
                    "ops": {
                        "model": "provider/epsilon",
                        "display_name": "Ops Bot"
                    }
                }
            }"#,
        )
        .unwrap();

        let bundle = load_config_bundle(&home).unwrap();
        let usage = collect_active_usage(&bundle);
        let by_model: std::collections::HashMap<_, _> = usage.into_iter().collect();

        assert_eq!(
            by_model.get("provider/delta").unwrap()[0].label,
            "Review Bot"
        );
        assert_eq!(
            by_model.get("provider/epsilon").unwrap()[0].label,
            "Ops Bot"
        );
    }

    fn make_temp_home() -> PathBuf {
        let mut base = std::env::temp_dir();
        base.push(format!(
            "opencode-model-report-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&base).unwrap();
        base
    }
}
