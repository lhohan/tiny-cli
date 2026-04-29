//! Test DSL for models-watch acceptance tests.
//!
//! Provides a fluent API for building test scenarios that exercise
//! `models-watch.sh` as a black-box command.

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;

/// Fluent DSL entry point.
pub fn given() -> AppSpec {
    AppSpec::new()
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

/// Setup phase: configure the test environment.
pub struct AppSpec {
    tool_dir: PathBuf,
    api_fixture: Option<String>,
    prior_snapshot: Option<String>,
    notify_file: Option<PathBuf>,
}

impl AppSpec {
    fn new() -> Self {
        Self {
            tool_dir: make_temp_tool_dir(),
            api_fixture: None,
            prior_snapshot: None,
            notify_file: None,
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

    /// Build the execution context and return it.
    pub fn when_run(self) -> ExecutionContext {
        // Write the API fixture to a known path so the script can "curl" it.
        let api_fixture_path = self.tool_dir.join("test_api.json");
        if let Some(ref content) = self.api_fixture {
            std::fs::write(&api_fixture_path, content).expect("write api fixture");
        }

        // Write prior snapshot if provided.
        if let Some(ref snapshot) = self.prior_snapshot {
            let state_dir = self.tool_dir.join("state");
            std::fs::create_dir_all(&state_dir).expect("create state dir");
            std::fs::write(state_dir.join("latest.json"), snapshot).expect("write prior snapshot");
        }

        let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("parent of tests-rust")
            .join("models-watch.sh");

        ExecutionContext {
            tool_dir: self.tool_dir,
            script_path,
            api_fixture_path,
            notify_file: self.notify_file,
        }
    }
}

/// Action phase: the script has been invoked.
pub struct ExecutionContext {
    tool_dir: PathBuf,
    script_path: PathBuf,
    api_fixture_path: PathBuf,
    notify_file: Option<PathBuf>,
}

impl ExecutionContext {
    /// Run the script and return assertion helpers.
    pub fn then_result(self) -> AppResult {
        // Build the command using the stubbed tool directory.
        // The script uses the directory it lives in as its state root.
        // We symlink or copy the script to the temp tool dir so state is local.
        let script_in_tool = self.tool_dir.join("models-watch.sh");
        std::fs::copy(&self.script_path, &script_in_tool).expect("copy script to tool dir");

        let mut cmd = Command::new("bash");
        cmd.arg(&script_in_tool);

        // Stub the API URL with the fixture path.
        cmd.env("MODELS_WATCH_API_URL", format!("file://{}", self.api_fixture_path.display()));

        // Suppress osascript pop-ups during tests.
        cmd.env("MODELS_WATCH_NO_OSASCRIPT", "1");

        if let Some(ref notify_file) = self.notify_file {
            cmd.arg("--notify-file").arg(notify_file);
        }

        let output = match cmd.output() {
            Ok(o) => o,
            Err(err) => fail(format!("failed to run models-watch.sh: {err}")),
        };

        AppResult {
            tool_dir: self.tool_dir,
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
