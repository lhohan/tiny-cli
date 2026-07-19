//! Test DSL for models-watch acceptance tests.
//!
//! Provides a fluent API for building test scenarios that exercise
//! `models-watch.sh` as a black-box command.

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;

/// Fluent DSL entry point for models-watch.sh.
pub fn given() -> AppSpec {
    AppSpec::new()
}

/// Fluent DSL entry point for models-feed.sh.
pub fn given_feed() -> AppSpec {
    AppSpec::new().with_script("models-feed.sh")
}

/// Panic with a test failure message.
pub fn fail(message: impl Into<String>) -> ! {
    std::panic::panic_any(message.into());
}

/// Creates a unique temp directory that acts as the tool directory for a test run.
pub fn make_temp_tool_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let mut base = std::env::temp_dir();
    base.push(format!(
        "models-watch-test-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&base).expect("create temp tool dir");
    base
}

// ---------------------------------------------------------------------------
// Phase types: AppSpec -> ExecutionContext -> AppResult
// ---------------------------------------------------------------------------

/// A delta file entry for the `--report` test helper.
pub struct DeltaEntry {
    pub timestamp: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<(String, String, String)>, // (id, old_name, new_name)
}

/// Setup phase: configure the test environment.
pub struct AppSpec {
    tool_dir: PathBuf,
    script_name: String,
    api_fixture: Option<String>,
    prior_snapshot: Option<String>,
    notify_file: Option<PathBuf>,
    output_path: Option<PathBuf>,
    args: Vec<String>,
    skip_api_env: bool,
}

impl AppSpec {
    fn new() -> Self {
        Self {
            tool_dir: make_temp_tool_dir(),
            script_name: "models-watch.sh".to_string(),
            api_fixture: None,
            prior_snapshot: None,
            notify_file: None,
            output_path: None,
            args: Vec::new(),
            skip_api_env: false,
        }
    }

    /// Set the `models.dev/api.json` fixture content that curl will return.
    pub fn with_api_fixture(mut self, json: impl Into<String>) -> Self {
        self.api_fixture = Some(json.into());
        self
    }

    /// Place a prior `state/latest.json` snapshot in the tool directory.
    pub fn with_prior_snapshot(mut self, raw_api_json: impl Into<String>) -> Self {
        self.prior_snapshot = Some(raw_api_json.into());
        self
    }

    /// Enable `--notify-file` mode; the notification message will be written to `notify_file`.
    pub fn with_notify_file(mut self, path: PathBuf) -> Self {
        self.notify_file = Some(path);
        self
    }

    /// Set the script name (e.g. "models-feed.sh") to run instead of models-watch.sh.
    pub fn with_script(mut self, name: &str) -> Self {
        self.script_name = name.to_string();
        self
    }

    /// Set the `--output` path for scripts that support it (e.g. models-feed.sh).
    pub fn with_output(mut self, path: PathBuf) -> Self {
        self.output_path = Some(path);
        self
    }

    /// Do not set `MODELS_WATCH_API_URL` when running the script.
    /// The script falls back to the default URL, which requires network —
    /// used only when `--report` exits before the fetch.
    pub fn without_api_env(mut self) -> Self {
        self.skip_api_env = true;
        self
    }

    /// Add a command-line argument (e.g. `--report`).
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Write delta files into `state/` for `--report` tests. Each delta is written
    /// as `state/change-<timestamp>.json`.
    pub fn with_deltas(self, deltas: Vec<DeltaEntry>) -> Self {
        let state_dir = self.tool_dir.join("state");
        std::fs::create_dir_all(&state_dir).expect("create state dir for deltas");
        for entry in &deltas {
            let changed_arr: Vec<serde_json::Value> = entry
                .changed
                .iter()
                .map(|(id, old_name, new_name)| {
                    serde_json::json!({"id": id, "old_name": old_name, "new_name": new_name})
                })
                .collect();
            let delta = serde_json::json!({
                "timestamp": entry.timestamp,
                "added": entry.added,
                "removed": entry.removed,
                "changed": changed_arr,
            });
            let filename = format!("change-{}.json", entry.timestamp);
            std::fs::write(state_dir.join(&filename), delta.to_string())
                .expect("write delta file");
        }
        self
    }

    /// Build the execution context and return it.
    pub fn when_run(self) -> ExecutionContext {
        let api_fixture_path = self.tool_dir.join("test_api.json");

        // Write the API fixture only if one was provided and we're not skipping API env.
        if !self.skip_api_env {
            if let Some(ref content) = self.api_fixture {
                std::fs::write(&api_fixture_path, content).expect("write api fixture");
            }
        }

        // Write prior snapshot if provided.
        if let Some(ref snapshot) = self.prior_snapshot {
            let state_dir = self.tool_dir.join("state");
            std::fs::create_dir_all(&state_dir).expect("create state dir");
            std::fs::write(state_dir.join("latest.json"), snapshot).expect("write prior snapshot");
        }

        let script_name = self.script_name.clone();

        ExecutionContext {
            tool_dir: self.tool_dir,
            script_name,
            api_fixture_path,
            notify_file: self.notify_file,
            output_path: self.output_path,
            args: self.args,
            skip_api_env: self.skip_api_env,
        }
    }
}

/// Action phase: the script has been invoked.
pub struct ExecutionContext {
    tool_dir: PathBuf,
    script_name: String,
    api_fixture_path: PathBuf,
    notify_file: Option<PathBuf>,
    output_path: Option<PathBuf>,
    args: Vec<String>,
    skip_api_env: bool,
}

impl ExecutionContext {
    /// Run the script and return assertion helpers.
    pub fn then_result(self) -> AppResult {
        // Build the command using the stubbed tool directory.
        // The script uses the directory it lives in as its state root.
        // We copy the script to the temp tool dir so state is local.
        let script_in_tool = self.tool_dir.join(&self.script_name);
        let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("parent of tests-rust")
            .join(&self.script_name);
        std::fs::copy(&script_path, &script_in_tool).expect("copy script to tool dir");

        let mut cmd = Command::new("bash");
        cmd.arg(&script_in_tool);

        if !self.skip_api_env {
            // Stub the API URL with the fixture path.
            cmd.env("MODELS_WATCH_API_URL", format!("file://{}", self.api_fixture_path.display()));
        }

        // Suppress osascript pop-ups during tests.
        cmd.env("MODELS_WATCH_NO_OSASCRIPT", "1");

        if let Some(ref notify_file) = self.notify_file {
            cmd.arg("--notify-file").arg(notify_file);
        }
        if let Some(ref output_path) = self.output_path {
            cmd.arg("--output").arg(output_path);
        }
        for arg in &self.args {
            cmd.arg(arg);
        }

        let script_display = &self.script_name;
        let output = match cmd.output() {
            Ok(o) => o,
            Err(err) => fail(format!("failed to run {script_display}: {err}")),
        };

        // Determine where the feed script writes by default.
        let feed_path = self
            .output_path
            .unwrap_or_else(|| self.tool_dir.join("state").join("feed.rss"));

        AppResult {
            tool_dir: self.tool_dir,
            feed_path: Some(feed_path),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            notify_file: self.notify_file,
        }
    }
}

/// Assertion phase: inspect results.
pub struct AppResult {
    tool_dir: PathBuf,
    feed_path: Option<PathBuf>,
    exit_code: i32,
    stdout: String,
    stderr: String,
    notify_file: Option<PathBuf>,
}

impl AppResult {
    pub fn should_succeed(&self) -> &Self {
        if self.exit_code != 0 {
            fail(format!(
                "expected exit code 0, got {}\nstderr: {}",
                self.exit_code, self.stderr
            ));
        }
        self
    }

    pub fn should_exit_with(&self, code: i32) -> &Self {
        if self.exit_code != code {
            fail(format!(
                "expected exit code {}, got {}\nstderr: {}",
                code, self.exit_code, self.stderr
            ));
        }
        self
    }

    pub fn expect_no_delta_file(&self) -> &Self {
        let state_dir = self.tool_dir.join("state");
        if state_dir.exists() {
            let mut has_delta = false;
            if let Ok(entries) = std::fs::read_dir(&state_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with("change-") && name_str.ends_with(".json") {
                        has_delta = true;
                        break;
                    }
                }
            }
            if has_delta {
                fail("expected no delta file, but found one");
            }
        }
        self
    }

    pub fn expect_delta_file(&self) -> &Self {
        let state_dir = self.tool_dir.join("state");
        let mut found = false;
        if let Ok(entries) = std::fs::read_dir(&state_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("change-") && name_str.ends_with(".json") {
                    found = true;
                    break;
                }
            }
        }
        if !found {
            fail("expected a delta file (change-<timestamp>.json), but none found");
        }
        self
    }

    pub fn expect_delta_added(&self, expected: &[&str]) -> &Self {
        self.expect_delta_file();
        let delta = self.read_latest_delta();
        let added: Vec<String> = delta["added"]
            .as_array()
            .unwrap_or_else(|| fail("delta 'added' field is not an array"))
            .iter()
            .map(|v: &serde_json::Value| v.as_str().unwrap_or("").to_string())
            .collect();
        let expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
        if added != expected {
            fail(format!(
                "expected delta.added to be {:?}, got {:?}",
                expected, added
            ));
        }
        self
    }

    pub fn expect_delta_removed(&self, expected: &[&str]) -> &Self {
        self.expect_delta_file();
        let delta = self.read_latest_delta();
        let removed: Vec<String> = delta["removed"]
            .as_array()
            .unwrap_or_else(|| fail("delta 'removed' field is not an array"))
            .iter()
            .map(|v: &serde_json::Value| v.as_str().unwrap_or("").to_string())
            .collect();
        let expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
        if removed != expected {
            fail(format!(
                "expected delta.removed to be {:?}, got {:?}",
                expected, removed
            ));
        }
        self
    }

    pub fn expect_delta_changed(&self, expected: &[(&str, &str, &str)]) -> &Self {
        self.expect_delta_file();
        let delta = self.read_latest_delta();
        let changed: Vec<(String, String, String)> = delta["changed"]
            .as_array()
            .unwrap_or_else(|| fail("delta 'changed' field is not an array"))
            .iter()
            .map(|v| (
                v["id"].as_str().unwrap_or("").to_string(),
                v["old_name"].as_str().unwrap_or("").to_string(),
                v["new_name"].as_str().unwrap_or("").to_string(),
            ))
            .collect();
        let expected: Vec<(String, String, String)> = expected
            .iter()
            .map(|(id, old, new)| (id.to_string(), old.to_string(), new.to_string()))
            .collect();
        if changed != expected {
            fail(format!(
                "expected delta.changed to be {:?}, got {:?}",
                expected, changed
            ));
        }
        self
    }

    pub fn expect_snapshot_exists(&self) -> &Self {
        let latest = self.tool_dir.join("state").join("latest.json");
        if !latest.exists() {
            fail("expected state/latest.json to exist, but it does not");
        }
        self
    }

    /// Return the full stdout content for direct inspection.
    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    pub fn expect_stdout_contains(&self, needle: &str) -> &Self {
        if !self.stdout.contains(needle) {
            fail(format!(
                "expected stdout to contain '{}', but it did not.\n--- stdout ---\n{}",
                needle, self.stdout
            ));
        }
        self
    }

    pub fn expect_stdout_does_not_contain(&self, needle: &str) -> &Self {
        if self.stdout.contains(needle) {
            fail(format!(
                "expected stdout to NOT contain '{}', but it did.\n--- stdout ---\n{}",
                needle, self.stdout
            ));
        }
        self
    }

    pub fn expect_stdout_line_count(&self, count: usize) -> &Self {
        let actual_lines: Vec<&str> = self.stdout.lines().collect();
        if actual_lines.len() != count {
            fail(format!(
                "expected {} lines in stdout, got {}.\n--- stdout ---\n{}",
                count,
                actual_lines.len(),
                self.stdout
            ));
        }
        self
    }

    pub fn expect_notify_file_contains(&self, expected: &str) -> &Self {
        match &self.notify_file {
            Some(path) => {
                let content = std::fs::read_to_string(path)
                    .unwrap_or_else(|e| fail(format!("cannot read notify file: {e}")));
                if !content.contains(expected) {
                    fail(format!(
                        "expected notify file to contain '{}', got '{}'",
                        expected, content
                    ));
                }
            }
            None => fail("--notify-file was not set, cannot check notify content"),
        }
        self
    }

    // -- RSS feed assertions --

    /// Assert that the feed output file exists.
    pub fn expect_rss_file(&self) -> &Self {
        match &self.feed_path {
            Some(path) => {
                if !path.exists() {
                    fail(format!(
                        "expected RSS feed file at {}, but it does not exist",
                        path.display()
                    ));
                }
            }
            None => fail("no feed_path set, cannot check RSS file"),
        }
        self
    }

    /// Assert the RSS feed contains exactly `expected` `<item>` elements.
    pub fn expect_rss_item_count(&self, expected: usize) -> &Self {
        let feed = self.read_rss_feed();
        let count = feed.matches("<item>").count();
        if count != expected {
            fail(format!(
                "expected {} RSS items, found {}",
                expected, count
            ));
        }
        self
    }

    /// Assert the RSS feed contains the given substring.
    pub fn expect_rss_contains(&self, needle: &str) -> &Self {
        let feed = self.read_rss_feed();
        if !feed.contains(needle) {
            fail(format!(
                "expected RSS feed to contain '{}', but it did not.\n--- feed ---\n{}",
                needle, feed
            ));
        }
        self
    }

    /// Assert the RSS feed contains a `<pubDate>` element with the given RFC-822 date string.
    pub fn expect_rss_pubDate(&self, expected: &str) -> &Self {
        self.expect_rss_contains(&format!("<pubDate>{}</pubDate>", expected))
    }

    /// Read the entire RSS feed file content.
    pub fn read_rss_feed(&self) -> String {
        let path = self
            .feed_path
            .as_ref()
            .expect("no feed_path set, cannot read RSS");
        std::fs::read_to_string(path)
            .unwrap_or_else(|e| fail(format!("cannot read RSS feed file: {e}")))
    }

    // -- helpers --

    fn read_latest_delta(&self) -> serde_json::Value {
        let state_dir = self.tool_dir.join("state");
        let mut found = None;
        if let Ok(entries) = std::fs::read_dir(&state_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("change-") && name_str.ends_with(".json") {
                    found = Some(entry.path());
                    break;
                }
            }
        }
        let path = found.unwrap_or_else(|| fail("no delta file found"));
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| fail(format!("cannot read delta file {}: {e}", path.display())));
        serde_json::from_str(&raw)
            .unwrap_or_else(|e| fail(format!("invalid JSON in delta file: {e}")))
    }
}
