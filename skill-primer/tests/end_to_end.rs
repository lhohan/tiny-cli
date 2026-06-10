//! End-to-end tests for Pi integration via subprocess
//!
//! These tests verify that Pi can be invoked as a subprocess with simple prompts.

use std::process::{Command, Stdio};

/// Setup phase - entry point for end-to-end Pi tests
pub struct PiCmd;

/// Action phase - holds the command configuration
pub struct PiCmdSetup {
    args: Vec<String>,
    stdin_input: Option<String>,
}

/// Result phase - wraps the command output
pub struct PiCmdResult {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

impl PiCmd {
    pub fn given() -> PiCmdSetup {
        PiCmdSetup {
            args: vec![
                "--mode".to_string(),
                "print".to_string(),
                "--no-session".to_string(),
            ],
            stdin_input: None,
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

    /// Execute the pi command
    pub fn when_run(self) -> PiCmdResult {
        let mut cmd = Command::new("pi");
        cmd.args(&self.args);

        let output = if let Some(input) = self.stdin_input {
            cmd.stdin(Stdio::piped());
            let mut child = cmd.spawn().expect("Failed to spawn pi");

            let mut stdin = child.stdin.take().unwrap();
            std::io::Write::write_all(&mut stdin, input.as_bytes()).unwrap();
            std::io::Write::flush(&mut stdin).unwrap();

            child.wait_with_output().expect("Failed to wait for pi")
        } else {
            cmd.output().expect("Failed to execute pi")
        };

        PiCmdResult {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
        }
    }
}

impl PiCmdResult {
    /// Assert the command exited successfully (exit code 0)
    pub fn should_succeed(self) -> Self {
        assert_eq!(
            self.exit_code, 0,
            "Expected exit code 0, got {}.\nStderr: {}",
            self.exit_code, self.stderr
        );
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
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn pi_responds_to_simple_subprocess_prompt() {
    // This test demonstrates the simplest way to use Pi as a library:
    // spawn it as a subprocess with a prompt argument in print mode
    PiCmd::given()
        .prompt("List all Rust files in the current directory")
        .when_run()
        .should_succeed()
        .expect_output("end_to_end.rs");
}
