use std::process::Command;

const EXE: &str = env!("CARGO_BIN_EXE_usage-watch");

fn clean_env(cmd: &mut Command) {
    for var in [
        "OPENCODE_API_KEY",
        "OPENCODE_GO_API_KEY",
        "CHATGPT_ACCESS_TOKEN",
        "CHATGPT_ACCOUNT_ID",
        "MISTRAL_API_KEY",
        "VIBE_API_KEY",
        "MISTRAL_ADMIN_COOKIE",
    ] {
        cmd.env_remove(var);
    }
}

#[test]
fn help_lists_providers_and_env_vars() {
    let mut cmd = Command::new(EXE);
    clean_env(&mut cmd);
    let out = cmd.arg("--help").output().expect("run help");
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    for needle in [
        "opencode-go",
        "chatgpt",
        "mistral",
        "vibe",
        "OPENCODE_API_KEY",
        "CHATGPT_ACCESS_TOKEN",
        "MISTRAL_API_KEY",
        "VIBE_API_KEY",
        "MISTRAL_ADMIN_COOKIE",
        "--json",
        "--provider",
    ] {
        assert!(s.contains(needle), "help output missing {needle}");
    }
}

#[test]
fn version_prints_name_and_version() {
    let mut cmd = Command::new(EXE);
    clean_env(&mut cmd);
    let out = cmd.arg("--version").output().expect("run version");
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("usage-watch"));
}

#[test]
fn no_env_vars_exits_2_with_hint() {
    let mut cmd = Command::new(EXE);
    clean_env(&mut cmd);
    let out = cmd.output().expect("run");
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("no providers configured"), "stderr: {err}");
    assert!(err.contains("CHATGPT_ACCESS_TOKEN"));
}

#[test]
fn unknown_provider_exits_2() {
    let mut cmd = Command::new(EXE);
    clean_env(&mut cmd);
    let out = cmd.args(["--provider", "nope"]).output().expect("run");
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown provider"));
}

#[test]
fn unknown_flag_exits_2() {
    let mut cmd = Command::new(EXE);
    clean_env(&mut cmd);
    let out = cmd.arg("--bogus").output().expect("run");
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown argument"));
}

#[test]
fn named_provider_without_env_reports_error_and_exits_1() {
    let mut cmd = Command::new(EXE);
    clean_env(&mut cmd);
    let out = cmd
        .args(["--provider", "opencode-go", "--json"])
        .output()
        .expect("run");
    assert_eq!(out.status.code(), Some(1));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("\"opencode-go\""), "stdout: {s}");
    assert!(s.contains("not configured"), "stdout: {s}");
}
