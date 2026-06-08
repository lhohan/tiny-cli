use assert_fs::fixture::{FileWriteStr, PathChild};

/// Setup phase — entry point. Zero-sized marker.
pub struct Cmd;

/// Action phase — holds args and managed fixtures before execution.
pub struct CmdSetup {
    args: Vec<String>,
    _include_dirs: Vec<assert_fs::TempDir>,
}

/// Assert phase — wraps assert_cmd result and keeps fixtures alive.
pub struct CmdResult {
    result: assert_cmd::assert::Assert,
    _include_dirs: Vec<assert_fs::TempDir>,
}

impl Cmd {
    pub fn given() -> CmdSetup {
        CmdSetup {
            args: vec![],
            _include_dirs: vec![],
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

    /// Create a temporary include directory and add `--include <path>`.
    /// The directory is retained until the assertion phase completes.
    pub fn with_include_dir(mut self) -> Self {
        let tmp = assert_fs::TempDir::new().unwrap();
        let path = tmp.to_str().unwrap().to_string();
        self._include_dirs.push(tmp);
        self.args.push("--include".to_string());
        self.args.push(path);
        self
    }

    /// Add a skill fixture inside the managed include directory at `index`.
    /// Creates `{include_dir[index]}/{name}/SKILL.md` with frontmatter and body.
    ///
    /// # Panics
    /// Panics if `index` is out of range (no `with_include_dir()` call at that
    /// position).
    pub fn with_skill_in(self, index: usize, name: &str, description: &str, body: &str) -> Self {
        let dir = self
            ._include_dirs
            .get(index)
            .unwrap_or_else(|| panic!("with_skill_in({index}): no include dir at index {index}; call with_include_dir() first"));
        let skill_file = dir.child(format!("{name}/SKILL.md"));
        let content = format!("---\nname: {name}\ndescription: {description}\n---\n{body}");
        skill_file.write_str(&content).unwrap();
        self
    }

    /// Add a skill fixture inside the **first** managed include directory.
    ///
    /// Convenience wrapper around `with_skill_in(0, …)`.
    ///
    /// # Panics
    /// Panics if no `with_include_dir()` has been called.
    pub fn with_skill(self, name: &str, description: &str, body: &str) -> Self {
        self.with_skill_in(0, name, description, body)
    }

    /// Build and execute the binary.
    pub fn when_run(self) -> CmdResult {
        let mut cmd = assert_cmd::Command::cargo_bin("skills-primer").unwrap();
        cmd.args(&self.args);
        CmdResult {
            result: cmd.assert(),
            _include_dirs: self._include_dirs,
        }
    }
}

impl CmdResult {
    /// Assert the command exited successfully (exit code 0).
    pub fn should_succeed(self) -> Self {
        CmdResult {
            result: self.result.success(),
            _include_dirs: self._include_dirs,
        }
    }

    /// Assert the command exited with a non-zero exit code.
    pub fn should_fail(self) -> Self {
        CmdResult {
            result: self.result.failure(),
            _include_dirs: self._include_dirs,
        }
    }

    /// Assert stdout contains the given text.
    pub fn expect_output(self, text: &str) -> Self {
        CmdResult {
            result: self.result.stdout(predicates::str::contains(text)),
            _include_dirs: self._include_dirs,
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

    /// Assert the help text is displayed (Usage, Commands, prime listing).
    pub fn expect_help_printed(self) -> Self {
        self.expect_output("Usage:")
            .expect_output("Commands:")
            .expect_output("prime")
    }

    /// Composite assertion for the full `prime` subcommand output.
    pub fn expect_prime_instructions(self) -> Self {
        self.expect_skills_header()
            .expect_instructions()
            .expect_available_skills()
    }
}
