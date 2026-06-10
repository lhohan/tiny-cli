#![allow(dead_code)]

use assert_fs::fixture::{FileWriteStr, PathChild};
use predicates::prelude::PredicateBooleanExt;
use std::collections::HashMap;

/// Setup phase — entry point. Zero-sized marker.
pub struct Cmd;

/// Action phase — holds args and managed fixtures before execution.
pub struct CmdSetup {
    args: Vec<String>,
    _include_dirs: Vec<(String, assert_fs::TempDir)>,
    _file_fixtures: Vec<(String, String, assert_fs::TempDir)>,
    _path_names: HashMap<String, String>,
}

/// Assert phase — wraps assert_cmd result and keeps fixtures alive.
pub struct CmdResult {
    result: assert_cmd::assert::Assert,
    _include_dirs: Vec<(String, assert_fs::TempDir)>,
    _file_fixtures: Vec<(String, String, assert_fs::TempDir)>,
    _path_names: HashMap<String, String>,
}

impl Cmd {
    pub fn given() -> CmdSetup {
        CmdSetup {
            args: vec![],
            _include_dirs: vec![],
            _file_fixtures: vec![],
            _path_names: HashMap::new(),
        }
    }
}

impl CmdSetup {
    /// Add a raw argument.
    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    /// Add multiple raw arguments.
    pub fn args(mut self, args: &[&str]) -> Self {
        self.args.extend(args.iter().map(|s| s.to_string()));
        self
    }

    /// Create a named temporary include directory and add `--include <path>`.
    /// The name maps to the temp dir path for later config assertions.
    pub fn with_include_dir(mut self, name: &str) -> Self {
        let tmp = assert_fs::TempDir::new().unwrap();
        let path = tmp.to_str().unwrap().to_string();
        self._include_dirs.push((name.to_string(), tmp));
        self._path_names.insert(name.to_string(), path.clone());
        self.args.push("--include".to_string());
        self.args.push(path);
        self
    }

    pub fn with_empty_include_dir(self) -> Self {
        self.with_include_dir("empty")
    }

    /// Add a skill fixture to the most recently created include directory.
    /// If no include directory has been created yet, one is auto-provisioned.
    /// Creates `{include_dir}/{name}/SKILL.md` with frontmatter and body.
    pub fn with_skill(mut self, name: &str, description: &str, body: &str) -> Self {
        if self._include_dirs.is_empty() {
            let tmp = assert_fs::TempDir::new().unwrap();
            let path = tmp.to_str().unwrap().to_string();
            self._include_dirs.push(("auto".to_string(), tmp));
            self.args.push("--include".to_string());
            self.args.push(path.clone());
            self._path_names.insert(name.to_string(), path);
        } else {
            let path = self
                ._include_dirs
                .last()
                .unwrap()
                .1
                .to_str()
                .unwrap()
                .to_string();
            self._path_names.insert(name.to_string(), path);
        }
        let dir = &self._include_dirs.last().unwrap().1;
        let skill_file = dir.child(format!("{name}/SKILL.md"));
        let content = format!("---\nname: {name}\ndescription: {description}\n---\n{body}");
        skill_file.write_str(&content).unwrap();
        self
    }

    /// Add a skill fixture to the most recently created include directory.
    /// If no include directory has been created yet, one is auto-provisioned.
    /// Creates `{include_dir}/{name}/SKILL.md` with frontmatter and body.
    pub fn with_skill_raw(mut self, dir_name: &str, content: &str) -> Self {
        if self._include_dirs.is_empty() {
            let tmp = assert_fs::TempDir::new().unwrap();
            let path = tmp.to_str().unwrap().to_string();
            self._include_dirs.push(("auto".to_string(), tmp));
            self.args.push("--include".to_string());
            self.args.push(path);
        }
        let dir = &self._include_dirs.last().unwrap().1;
        let skill_file = dir.child(format!("{dir_name}/SKILL.md"));
        skill_file.write_str(content).unwrap();
        self
    }

    /// Set the subcommand to `show-config`.
    pub fn command_show_config(mut self) -> Self {
        self.args.push("show-config".to_string());
        self
    }

    /// Add `--include <path>` using the literal path string (no temp dir created).
    pub fn with_include(mut self, path: &str) -> Self {
        self.args.push("--include".to_string());
        self.args.push(path.to_string());
        self
    }

    /// Create a temporary file and add `--include <path-to-file>`.
    /// The file is treated as a non-directory path, used to test error handling.
    pub fn with_file_include(mut self, name: &str) -> Self {
        let tmp = assert_fs::TempDir::new().unwrap();
        let file_path = tmp.path().join(name);
        std::fs::write(&file_path, "this is a file, not a directory").unwrap();
        let path_str = file_path.to_str().unwrap().to_string();
        self.args.push("--include".to_string());
        self.args.push(path_str.clone());
        self._file_fixtures.push((name.to_string(), path_str, tmp));
        self._path_names
            .insert(name.to_string(), file_path.to_str().unwrap().to_string());
        self
    }

    /// Execute against the binary.
    pub fn when_run(self) -> CmdResult {
        let mut cmd = assert_cmd::Command::cargo_bin("skills-primer").unwrap();
        cmd.args(&self.args);
        CmdResult {
            result: cmd.assert(),
            _include_dirs: self._include_dirs,
            _file_fixtures: self._file_fixtures,
            _path_names: self._path_names,
        }
    }
}

impl CmdResult {
    /// Assert the command exited successfully (exit code 0).
    pub fn should_succeed(self) -> Self {
        CmdResult {
            result: self.result.success(),
            _include_dirs: self._include_dirs,
            _file_fixtures: self._file_fixtures,
            _path_names: self._path_names,
        }
    }

    /// Assert the command exited with a non-zero exit code.
    pub fn should_fail(self) -> Self {
        CmdResult {
            result: self.result.failure(),
            _include_dirs: self._include_dirs,
            _file_fixtures: self._file_fixtures,
            _path_names: self._path_names,
        }
    }

    /// Assert stdout contains the given text.
    pub fn expect_output(self, text: &str) -> Self {
        CmdResult {
            result: self.result.stdout(predicates::str::contains(text)),
            _include_dirs: self._include_dirs,
            _file_fixtures: self._file_fixtures,
            _path_names: self._path_names,
        }
    }

    // ── Domain-specific assertions ──────────────────────────

    /// Assert the "## Skills" header is present.
    pub fn expect_skills_header(self) -> Self {
        self.expect_output("## Skills")
    }

    /// Assert the agent-skills instruction block is present.
    pub fn expect_instructions(self) -> Self {
        self.expect_output("This repository may contain agent skills")
    }

    /// Assert the `<available_skills>...</available_skills>` wrapper is present.
    pub fn expect_available_skills(self) -> Self {
        self.expect_output("<available_skills>")
            .expect_output("</available_skills>")
    }

    /// Assert a specific skill name and description appear in the listing.
    pub fn expect_skill(self, name: &str, description: &str) -> Self {
        self.expect_output(name).expect_output(description)
    }

    /// Assert stdout does NOT contain the given text.
    pub fn expect_out_does_not_contain(self, text: &str) -> Self {
        CmdResult {
            result: self.result.stdout(predicates::str::contains(text).not()),
            _include_dirs: self._include_dirs,
            _file_fixtures: self._file_fixtures,
            _path_names: self._path_names,
        }
    }

    /// Assert stdout contains the given text exactly `count` times.
    pub fn expect_output_count(self, text: &str, count: usize) -> Self {
        let stdout = String::from_utf8_lossy(&self.result.get_output().stdout);
        let actual = stdout.matches(text).count();
        assert_eq!(
            actual, count,
            "expected {:?} to appear {} times in stdout, but found {}",
            text, count, actual
        );
        self
    }

    /// Assert stderr contains the given text.
    pub fn expect_stderr_contains(self, text: &str) -> Self {
        CmdResult {
            result: self.result.stderr(predicates::str::contains(text)),
            _include_dirs: self._include_dirs,
            _file_fixtures: self._file_fixtures,
            _path_names: self._path_names,
        }
    }

    /// Assert stderr contains the path of the named include directory.
    pub fn expect_stderr_dir(self, name: &str) -> Self {
        let path = self
            ._include_dirs
            .iter()
            .find(|(n, _)| n == name)
            .unwrap()
            .1
            .to_str()
            .unwrap();
        CmdResult {
            result: self.result.stderr(predicates::str::contains(path)),
            _include_dirs: self._include_dirs,
            _file_fixtures: self._file_fixtures,
            _path_names: self._path_names,
        }
    }

    /// Assert the help text is displayed (Usage, Commands, prime listing).
    pub fn expect_help_printed(self) -> Self {
        self.expect_output("Usage:")
            .expect_output("Commands:")
            .expect_output("prime")
            .expect_output("show-config")
    }

    /// Composite assertion for the full `prime` subcommand output.
    pub fn expect_prime_instructions(self) -> Self {
        self.expect_skills_header().expect_instructions()
    }

    /// Assert no skills were detected (empty include dirs).
    pub fn expect_no_skills_detected(self) -> Self {
        self.expect_output("No skills detected")
            .expect_out_does_not_contain("<available_skills>")
    }

    // ── Config-related assertions ───────────────────────────

    /// Resolve a fixture name to its absolute path via `_path_names`,
    /// falling back to the raw string if not found.
    fn resolve_path(&self, name_or_path: &str) -> String {
        self._path_names
            .get(name_or_path)
            .cloned()
            .unwrap_or_else(|| name_or_path.to_string())
    }

    /// Assert a path appears with `exists` status in stdout.
    pub fn expect_exists_in_config(self, name: &str) -> Self {
        let path = self.resolve_path(name);
        self.expect_output("exists").expect_output(&path)
    }

    /// Assert a path appears with `missing` status in stdout.
    pub fn expect_missing_in_config(self, path: &str) -> Self {
        self.expect_output("missing").expect_output(path)
    }

    /// Assert a path appears with `error` status in stdout.
    pub fn expect_error_in_config(self, name: &str) -> Self {
        let path = self.resolve_path(name);
        self.expect_output("error").expect_output(&path)
    }

    /// Assert stdout is completely empty.
    pub fn expect_no_output(self) -> Self {
        let stdout = &self.result.get_output().stdout;
        assert!(
            stdout.is_empty(),
            "expected empty stdout, got: {:?}",
            String::from_utf8_lossy(stdout),
        );
        self
    }

    /// Assert stderr is completely empty.
    pub fn expect_stderr_empty(self) -> Self {
        let stderr = &self.result.get_output().stderr;
        assert!(
            stderr.is_empty(),
            "expected empty stderr, got: {:?}",
            String::from_utf8_lossy(stderr),
        );
        self
    }
}
