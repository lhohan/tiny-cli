//! End-to-end tests for Pi integration via subprocess
//!
//! These tests verify that Pi can be invoked as a subprocess with simple prompts.
//! They are skipped at build time if the `pi` binary is not on `$PATH`.

use assert_fs::TempDir;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::Duration;
use wait_timeout::ChildExt;

#[cfg_attr(
    not(has_test_agent),
    ignore = "test agents's command not found - install a test agent to run this test"
)]
#[test]
fn agent_without_skills_should_not_find_skills_when_not_primed() {
    AgentWithoutSkills::with_system_prompt("".to_string())
        .when_run_with("Do you have any skills available? Answer with SKILLS_NOT_AVAILABLE")
        .should_succeed()
        .expect_output("SKILLS_NOT_AVAILABLE");
}

#[cfg_attr(
    not(has_test_agent),
    ignore = "test agents's command not found - install a test agent to run this test"
)]
#[test]
fn agent_without_skills_should_not_find_skills_when_primed() {
    let skills_system_prompt =
        skills_primer::generate_prime_output(&[PathBuf::from("tests/fixtures/")])
            .expect("prime output should succeed")
            .instructions;

    AgentWithoutSkills::with_system_prompt(skills_system_prompt)
        .when_run_with("Load test-skill")
        .should_succeed()
        .expect_output("Loaded primed skill: [test-skill]")
        .expect_output("Loaded primed skill: [nested-invocation-test-skill]");
}

/// Setup phase - entry point for end-to-end Pi tests
pub struct AgentWithoutSkills;

/// Action phase - holds the command configuration for running pi
pub struct PiCmdSetup {
    args: Vec<String>,
    stdin_input: Option<String>,
    cwd: Option<PathBuf>,
    env_vars: Vec<(String, String)>,
    timeout: Option<Duration>,
    system_prompt: Option<String>,
    _tmp_dir: Option<TempDir>,
    _home_dir: Option<TempDir>,
}

/// Result phase - wraps the command output
pub struct PiCmdResult {
    stdout: String,
    stderr: String,
    exit_code: i32,
    timed_out: bool,
}

impl AgentWithoutSkills {
    /// Combine `given()` and `system_prompt()` into a single construction.
    pub fn with_system_prompt(skills_prompt: String) -> PiCmdSetup {
        Self::given().system_prompt(skills_prompt)
    }

    pub fn given() -> PiCmdSetup {
        let tmp = Self::temp_dir_in_target();
        let path = tmp.path().to_str().unwrap().to_string();
        let home = Self::temp_dir_in_target();
        let home_path = home.path().to_str().unwrap().to_string();
        let mut env_vars = vec![
            ("PI_CODING_AGENT_DIR".to_string(), path),
            ("HOME".to_string(), home_path),
        ];
        if let Ok(key) = std::env::var("MISTRAL_API_KEY") {
            env_vars.push(("MISTRAL_API_KEY".to_string(), key));
        }
        if let Ok(key) = std::env::var("OPENCODE_ZEROSTACK_API_KEY") {
            env_vars.push(("OPENCODE_ZEROSTACK_API_KEY".to_string(), key));
        }

        PiCmdSetup {
            args: vec![
                "--print".to_string(),
                "--no-session".to_string(),
                "--no-skills".to_string(),
                "--model".to_string(),
                "mistral/mistral-medium-3.5".to_string(),
            ],
            stdin_input: None,
            cwd: None,
            env_vars,
            timeout: Some(Duration::from_secs(60)),
            system_prompt: None,
            _tmp_dir: Some(tmp),
            _home_dir: Some(home),
        }
    }

    /// Fixture: create a temporary directory inside the project's `target/`
    /// directory. This avoids macOS TCC restrictions that block access to
    /// `~/.pi/` and similar system locations.
    fn temp_dir_in_target() -> TempDir {
        let project_target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
        std::fs::create_dir_all(&project_target).unwrap();
        TempDir::new_in(&project_target).unwrap()
    }
}

impl PiCmdSetup {
    /// Add a raw argument to the pi command
    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    /// Add multiple raw arguments
    pub fn args(mut self, args: &[&str]) -> Self {
        self.args.extend(args.iter().map(|s| s.to_string()));
        self
    }

    /// Set the prompt message (as a positional argument for print mode)
    pub fn prompt(mut self, message: &str) -> Self {
        self.args.push(message.to_string());
        self
    }

    /// Set the working directory for the pi command
    pub fn in_dir(mut self, path: &str) -> Self {
        self.cwd = Some(PathBuf::from(path));
        self
    }

    /// Set an environment variable for the pi command
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env_vars.push((key.to_string(), value.to_string()));
        self
    }

    /// Set a timeout for the pi command
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Set the system prompt from generated prime output.
    pub fn system_prompt(mut self, skills_prompt: String) -> Self {
        self.system_prompt = Some(skills_prompt);
        self
    }

    /// Execute the pi command
    /// Set the prompt and execute in one call
    pub fn when_run_with(self, prompt: &str) -> PiCmdResult {
        self.prompt(prompt).when_run()
    }

    /// Execute the pi command
    pub fn when_run(self) -> PiCmdResult {
        let mut cmd = Command::new("pi");
        if let Some(system_prompt) = &self.system_prompt {
            cmd.arg("--system-prompt").arg(system_prompt);
        }
        cmd.args(&self.args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(ref dir) = self.cwd {
            cmd.current_dir(dir);
        }

        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        let output = if let Some(input) = self.stdin_input {
            cmd.stdin(Stdio::piped());
            let mut child = cmd.spawn().expect("Failed to spawn pi");

            let mut stdin = child.stdin.take().unwrap();
            std::io::Write::write_all(&mut stdin, input.as_bytes()).unwrap();
            std::io::Write::flush(&mut stdin).unwrap();
            drop(stdin);

            Self::wait_with_timeout(child, self.timeout)
        } else {
            let child = cmd.spawn().expect("Failed to spawn pi");
            Self::wait_with_timeout(child, self.timeout)
        };

        PiCmdResult {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
            timed_out: self.timeout.is_some() && output.status.code().is_none(),
        }
    }

    /// Wait for a child process with an optional timeout
    fn wait_with_timeout(mut child: std::process::Child, timeout: Option<Duration>) -> Output {
        match timeout {
            Some(duration) => {
                // Spawn threads to read stdout and stderr
                let stdout_handle = child.stdout.take().map(|s| {
                    thread::spawn(move || {
                        let mut s = s;
                        let mut buf = vec![];
                        let _ = Read::read_to_end(&mut s, &mut buf);
                        buf
                    })
                });

                let stderr_handle = child.stderr.take().map(|s| {
                    thread::spawn(move || {
                        let mut s = s;
                        let mut buf = vec![];
                        let _ = Read::read_to_end(&mut s, &mut buf);
                        buf
                    })
                });

                // Wait for the process with timeout
                let wait_result = child.wait_timeout(duration);

                match wait_result {
                    Ok(Some(status)) => {
                        // Process completed within timeout - collect output
                        let stdout_buf = stdout_handle
                            .map(|h| h.join().unwrap_or_default())
                            .unwrap_or_default();
                        let stderr_buf = stderr_handle
                            .map(|h| h.join().unwrap_or_default())
                            .unwrap_or_default();
                        Output {
                            status,
                            stdout: stdout_buf,
                            stderr: stderr_buf,
                        }
                    }
                    Ok(None) => Self::collect_after_kill(&mut child, stdout_handle, stderr_handle),
                    Err(e) => {
                        eprintln!("Error waiting for child: {}", e);
                        Self::collect_after_kill(&mut child, stdout_handle, stderr_handle)
                    }
                }
            }
            None => child.wait_with_output().expect("Failed to wait for pi"),
        }
    }

    /// Kill the child process and collect whatever output was produced
    fn collect_after_kill(
        child: &mut std::process::Child,
        stdout_handle: Option<thread::JoinHandle<Vec<u8>>>,
        stderr_handle: Option<thread::JoinHandle<Vec<u8>>>,
    ) -> Output {
        let _ = child.kill();
        let status = child.wait().unwrap_or_default();
        let stdout_buf = stdout_handle
            .and_then(|h| h.join().ok())
            .unwrap_or_default();
        let stderr_buf = stderr_handle
            .and_then(|h| h.join().ok())
            .unwrap_or_default();
        Output {
            status,
            stdout: stdout_buf,
            stderr: stderr_buf,
        }
    }
}

impl PiCmdResult {
    /// Assert the command exited successfully (exit code 0) and no prime warnings
    pub fn should_succeed(self) -> Self {
        let msg = if self.timed_out {
            format!(
                "Process timed out and did not complete within the configured timeout.\nStderr: {}",
                self.stderr
            )
        } else {
            format!(
                "Expected exit code 0, got {}.\nStderr: {}",
                self.exit_code, self.stderr
            )
        };
        assert_eq!(self.exit_code, 0, "{}", msg);
        self
    }

    /// Assert stdout contains the given text
    pub fn expect_output(self, text: &str) -> Self {
        assert!(
            self.stdout.contains(text),
            "Expected stdout to contain {:?}, but it was:\n{}",
            text,
            self.stdout
        );
        self
    }

    /// Assert stdout does NOT contain the given text
    pub fn expect_output_not_contain(self, text: &str) -> Self {
        assert!(
            !self.stdout.contains(text),
            "Expected stdout NOT to contain {:?}, but it was:\n{}",
            text,
            self.stdout
        );
        self
    }

    /// Assert stderr contains the given text
    pub fn expect_stderr_contains(self, text: &str) -> Self {
        assert!(
            self.stderr.contains(text),
            "Expected stderr to contain {:?}, but it was:\n{}",
            text,
            self.stderr
        );
        self
    }

    /// Chainable connector for fluent assertions - allows `and()` between assertions
    /// for more readable test expressions
    pub fn and(self) -> Self {
        self
    }
}
