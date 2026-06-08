/// Setup phase — owns nothing special for now.
pub struct Cmd;

/// Action phase — holds args before execution.
pub struct CmdBuilder {
    args: Vec<String>,
}

/// Assert phase — wraps assert_cmd result.
pub struct CmdResult {
    result: assert_cmd::assert::Assert,
}

impl Cmd {
    pub fn given() -> CmdBuilder {
        CmdBuilder { args: vec![] }
    }
}

impl CmdBuilder {
    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }
    pub fn args(mut self, args: &[&str]) -> Self {
        self.args.extend(args.iter().map(|s| s.to_string()));
        self
    }
    pub fn when_run(self) -> CmdResult {
        let mut cmd = assert_cmd::Command::cargo_bin("skills-primer").unwrap();
        cmd.args(&self.args);
        CmdResult {
            result: cmd.assert(),
        }
    }
}

impl CmdResult {
    pub fn should_succeed(self) -> Self {
        CmdResult {
            result: self.result.success(),
        }
    }
    pub fn expect_output(self, text: &str) -> Self {
        CmdResult {
            result: self.result.stdout(predicates::str::contains(text)),
        }
    }
    pub fn expect_skills_header(self) -> Self {
        self.expect_output("## Skills")
    }
    pub fn expect_instructions(self) -> Self {
        self.expect_output("This repository may contain agent skills")
    }
    pub fn expect_available_skills(self) -> Self {
        self.expect_output("<available_skills>")
            .expect_output("</available_skills>")
    }
    pub fn expect_help(self) -> Self {
        self.expect_output("Usage:")
            .expect_output("Commands:")
            .expect_output("prime")
    }

}
