//! End-to-end tests for Pi integration via subprocess
//!
//! These tests verify that Pi can be invoked as a subprocess with simple prompts.
//! They are skipped at build time if the `pi` binary is not on `$PATH`.

use assert_fs::TempDir;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::thread;
use std::time::Duration;
use wait_timeout::ChildExt;

/// Setup phase - entry point for end-to-end Pi tests
pub struct AgentWithoutSkills;

/// Action phase - holds the command configuration
pub struct PiCmdSetup {
    args: Vec<String>,
    stdin_input: Option<String>,
    cwd: Option<PathBuf>,
    env_vars: Vec<(String, String)>,
    timeout: Option<Duration>,
}

/// Result phase - wraps the command output
pub struct PiCmdResult {
    stdout: String,
    stderr: String,
    exit_code: i32,
    timed_out: bool,
}

impl AgentWithoutSkills {
    pub fn given() -> PiCmdSetup {
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
            env_vars: vec![],
            timeout: None,
        }
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

    /// Execute the pi command
    pub fn when_run(self) -> PiCmdResult {
        let mut cmd = Command::new("pi");
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
                    Ok(None) => {
                        // Timeout - kill the process
                        let _ = child.kill();
                        let status = child.wait().unwrap_or(ExitStatus::default());
                        // Try to get whatever output we have
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
                    Err(e) => {
                        eprintln!("Error waiting for child: {}", e);
                        let _ = child.kill();
                        let status = child.wait().unwrap_or(ExitStatus::default());
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
            }
            None => child.wait_with_output().expect("Failed to wait for pi"),
        }
    }
}

impl PiCmdResult {
    /// Assert the command exited successfully (exit code 0)
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

// ============================================================================
// Tests
// ============================================================================

/// Fixture: create a temporary PI_CODING_AGENT_DIR inside the project's
/// `target/` directory. This avoids macOS TCC restrictions that block
/// access to `~/.pi/` and similar system locations.
fn pi_agent_dir() -> TempDir {
    let project_target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    std::fs::create_dir_all(&project_target).unwrap();
    TempDir::new_in(&project_target).unwrap()
}

#[cfg_attr(
    not(has_test_agent),
    ignore = "test agents's command not found - install a test agent to run this test"
)]
#[test]
fn pi_has_skills_available_from_user_directory() {
    let agent_dir = pi_agent_dir();

    AgentWithoutSkills::given()
        .env("PI_CODING_AGENT_DIR", agent_dir.path().to_str().unwrap())
        .with_timeout(Duration::from_secs(20))
        .prompt("Do you have any skills available? Answer yes or no.")
        .when_run()
        .should_succeed()
        .expect_output("no");
}
