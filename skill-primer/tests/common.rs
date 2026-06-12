#![allow(dead_code)]

use predicates::prelude::PredicateBooleanExt;
use std::collections::HashMap;

/// Setup phase — entry point. Zero-sized marker.
pub struct Cmd;

/// Action phase — holds args and managed fixtures before execution.
pub struct CmdSetup {
    args: Vec<String>,
    _temp_dirs: Vec<(String, assert_fs::TempDir)>,
    _file_fixtures: Vec<(String, String, assert_fs::TempDir)>,
    _path_names: HashMap<String, String>,
    skill_path: String,
    cwd: Option<std::path::PathBuf>,
    _home_dirs: Vec<assert_fs::TempDir>,
    _env_vars: Vec<(String, String)>,
}

/// Assert phase — wraps assert_cmd result and keeps fixtures alive.
pub struct CmdResult {
    result: assert_cmd::assert::Assert,
    _temp_dirs: Vec<(String, assert_fs::TempDir)>,
    _file_fixtures: Vec<(String, String, assert_fs::TempDir)>,
    _path_names: HashMap<String, String>,
    _home_dirs: Vec<assert_fs::TempDir>,
}

impl Cmd {
    pub fn given() -> CmdSetup {
        let home = assert_fs::TempDir::new().unwrap();
        let home_str = home.path().to_string_lossy().to_string();
        CmdSetup {
            args: vec![],
            _temp_dirs: vec![],
            _file_fixtures: vec![],
            _path_names: HashMap::new(),
            skill_path: ".agents/skills".to_string(),
            cwd: Some(home.path().to_path_buf()),
            _home_dirs: vec![home],
            _env_vars: vec![("HOME".to_string(), home_str)],
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

    /// Enable warnings for commands that suppress them by default.
    pub fn with_warnings(mut self) -> Self {
        self.args.push("--warnings".to_string());
        self
    }

    /// Disable warnings for commands that show them by default.
    pub fn without_warnings(mut self) -> Self {
        self.args.push("--no-warnings".to_string());
        self
    }

    /// Set the relative skill path passed via `--path`.
    pub fn with_path(mut self, path: &str) -> Self {
        self.args.push("--path".to_string());
        self.args.push(path.to_string());
        self.skill_path = path.to_string();
        self
    }

    pub fn with_empty_skill_path(self) -> Self {
        self.with_path(".empty/skills")
    }

    /// Add a skill fixture to the configured relative skill path under HOME.
    pub fn with_skill(self, name: &str, description: &str, body: &str) -> Self {
        let root = self._home_dirs.last().unwrap().path().to_path_buf();
        let skill_file = root.join(&self.skill_path).join(name).join("SKILL.md");
        let content = format!("---\nname: {name}\ndescription: {description}\n---\n{body}");
        std::fs::create_dir_all(skill_file.parent().unwrap()).unwrap();
        std::fs::write(skill_file, &content).unwrap();
        self
    }

    /// Add a raw skill fixture to the configured relative skill path under HOME.
    pub fn with_skill_raw(self, dir_name: &str, content: &str) -> Self {
        let root = self._home_dirs.last().unwrap().path().to_path_buf();
        let skill_file = root.join(&self.skill_path).join(dir_name).join("SKILL.md");
        std::fs::create_dir_all(skill_file.parent().unwrap()).unwrap();
        std::fs::write(skill_file, content).unwrap();
        self
    }

    /// Set the subcommand to `prime`.
    pub fn command_prime(mut self) -> Self {
        self.args.push("prime".to_string());
        self
    }

    /// Set the subcommand to `config`.
    pub fn command_config(mut self) -> Self {
        self.args.push("config".to_string());
        self
    }

    /// Set the subcommand to `ls`.
    pub fn command_ls(mut self) -> Self {
        self.args.push("ls".to_string());
        self
    }

    /// Set the working directory for the command.
    pub fn with_cwd_dir(mut self, path: &std::path::Path) -> Self {
        self.cwd = Some(path.to_path_buf());
        self
    }

    /// Set an environment variable for the command.
    pub fn with_env(mut self, key: &str, val: &str) -> Self {
        self._env_vars.push((key.to_string(), val.to_string()));
        self
    }

    /// Create a skill under the implicit home directory's `.agents/skills/{name}/`.
    /// No `--path` flags are emitted.
    pub fn with_home_skill(self, name: &str, description: &str, body: &str) -> Self {
        let root = self._home_dirs.last().unwrap().path().to_path_buf();
        let skill_dir = root.join(".agents/skills").join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        let content = format!("---\nname: {name}\ndescription: {description}\n---\n{body}");
        std::fs::write(skill_dir.join("SKILL.md"), &content).unwrap();
        self
    }

    /// Create a skill under `{home}/{relative}/.agents/skills/{name}/`.
    /// No `--path` flags are emitted.
    pub fn with_subdir_skill(
        self,
        relative: &str,
        name: &str,
        description: &str,
        body: &str,
    ) -> Self {
        let root = self._home_dirs.last().unwrap().path().to_path_buf();
        let skill_dir = root.join(relative).join(".agents/skills").join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        let content = format!("---\nname: {name}\ndescription: {description}\n---\n{body}");
        std::fs::write(skill_dir.join("SKILL.md"), &content).unwrap();
        self
    }

    /// Set CWD to a subdirectory of the implicit home directory.
    /// The subdirectory is created if it does not exist.
    pub fn with_cwd(mut self, relative: &str) -> Self {
        let root = self._home_dirs.last().unwrap().path().to_path_buf();
        let subdir = root.join(relative);
        std::fs::create_dir_all(&subdir).unwrap();
        self.cwd = Some(subdir);
        self
    }

    /// Create a file at a relative path within the configured skill path.
    pub fn with_file_at(self, relative_path: &str, content: &str) -> Self {
        let root = self._home_dirs.last().unwrap().path().to_path_buf();
        let file = root.join(&self.skill_path).join(relative_path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, content).unwrap();
        self
    }

    /// Create a file under HOME and add `--path <path-to-file>`.
    /// The file is treated as a non-directory path, used to test error handling.
    pub fn with_file_path(mut self, name: &str) -> Self {
        let root = self._home_dirs.last().unwrap().path().to_path_buf();
        let file_path = root.join(name);
        std::fs::write(&file_path, "this is a file, not a directory").unwrap();
        self.args.push("--path".to_string());
        self.args.push(name.to_string());
        self._path_names
            .insert(name.to_string(), file_path.to_str().unwrap().to_string());
        self
    }

    /// Execute against the binary.
    pub fn when_run(self) -> CmdResult {
        let mut cmd = assert_cmd::Command::cargo_bin("skills-primer").unwrap();
        cmd.args(&self.args);
        if let Some(ref cwd) = self.cwd {
            cmd.current_dir(cwd);
        }
        for (key, val) in &self._env_vars {
            cmd.env(key, val);
        }
        CmdResult {
            result: cmd.assert(),
            _temp_dirs: self._temp_dirs,
            _file_fixtures: self._file_fixtures,
            _path_names: self._path_names,
            _home_dirs: self._home_dirs,
        }
    }
}

impl CmdResult {
    /// Assert the command exited successfully (exit code 0).
    pub fn should_succeed(self) -> Self {
        CmdResult {
            result: self.result.success(),
            _temp_dirs: self._temp_dirs,
            _file_fixtures: self._file_fixtures,
            _path_names: self._path_names,
            _home_dirs: self._home_dirs,
        }
    }

    /// Assert the command exited with a non-zero exit code.
    pub fn should_fail(self) -> Self {
        CmdResult {
            result: self.result.failure(),
            _temp_dirs: self._temp_dirs,
            _file_fixtures: self._file_fixtures,
            _path_names: self._path_names,
            _home_dirs: self._home_dirs,
        }
    }

    /// Assert stdout contains the given text.
    pub fn expect_output(self, text: &str) -> Self {
        CmdResult {
            result: self.result.stdout(predicates::str::contains(text)),
            _temp_dirs: self._temp_dirs,
            _file_fixtures: self._file_fixtures,
            _path_names: self._path_names,
            _home_dirs: self._home_dirs,
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
            _temp_dirs: self._temp_dirs,
            _file_fixtures: self._file_fixtures,
            _path_names: self._path_names,
            _home_dirs: self._home_dirs,
        }
    }

    /// Assert `first` appears before `second` in stdout.
    pub fn expect_output_order(self, first: &str, second: &str) -> Self {
        let stdout = String::from_utf8_lossy(&self.result.get_output().stdout);
        let first_pos = stdout.find(first);
        let second_pos = stdout.find(second);
        assert!(
            first_pos.is_some(),
            "expected stdout to contain {:?}",
            first
        );
        assert!(
            second_pos.is_some(),
            "expected stdout to contain {:?}",
            second
        );
        assert!(
            first_pos.unwrap() < second_pos.unwrap(),
            "expected {:?} to appear before {:?} in stdout, but found the reverse\nstdout: {}",
            first,
            second,
            stdout
        );
        self
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
            _temp_dirs: self._temp_dirs,
            _file_fixtures: self._file_fixtures,
            _path_names: self._path_names,
            _home_dirs: self._home_dirs,
        }
    }

    /// Assert stderr does NOT contain the given text.
    pub fn expect_stderr_does_not_contain(self, text: &str) -> Self {
        CmdResult {
            result: self.result.stderr(predicates::str::contains(text).not()),
            _temp_dirs: self._temp_dirs,
            _file_fixtures: self._file_fixtures,
            _path_names: self._path_names,
            _home_dirs: self._home_dirs,
        }
    }

    /// Assert stderr contains the path of the named temporary directory.
    pub fn expect_stderr_dir(self, name: &str) -> Self {
        let path = self
            ._temp_dirs
            .iter()
            .find(|(n, _)| n == name)
            .unwrap()
            .1
            .to_str()
            .unwrap();
        CmdResult {
            result: self.result.stderr(predicates::str::contains(path)),
            _temp_dirs: self._temp_dirs,
            _file_fixtures: self._file_fixtures,
            _path_names: self._path_names,
            _home_dirs: self._home_dirs,
        }
    }

    /// Assert the help text is displayed (Usage, Commands, prime listing).
    pub fn expect_help_printed(self) -> Self {
        self.expect_output("Usage:")
            .expect_output("Commands:")
            .expect_output("prime ")
            .expect_output("ls ")
            .expect_output("config ")
            .expect_output("--warnings")
            .expect_output("--no-warnings")
            .expect_output(env!("CARGO_PKG_VERSION"))
    }

    /// Composite assertion for the full `prime` subcommand output.
    pub fn expect_prime_instructions(self) -> Self {
        self.expect_skills_header().expect_instructions()
    }

    /// Assert no skills were detected.
    pub fn expect_no_skills_detected(self) -> Self {
        self.expect_output("No skills detected")
            .expect_out_does_not_contain("<available_skills>")
    }

    // ── Config assertions ──────────────────────────────────

    /// Resolve a fixture name to its absolute path, falling back to the raw
    /// string if the name is not a known fixture.
    fn resolve_path(&self, name_or_path: &str) -> String {
        self._path_names
            .get(name_or_path)
            .cloned()
            .unwrap_or_else(|| name_or_path.to_string())
    }

    /// Assert a path appears annotated with `(found)` or `(not found)`.
    /// The `name_or_path` is resolved against known fixture names first,
    /// then used as a literal path string.
    pub fn expect_annotated(self, name_or_path: &str, annotation: &str) -> Self {
        let path = self.resolve_path(name_or_path);
        self.expect_output(&format!("{} ({})", path, annotation))
    }
}
