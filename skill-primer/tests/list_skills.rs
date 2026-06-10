mod common;
use assert_fs::fixture::PathChild;
use common::Cmd;

// ── Basic discovery ───────────────────────────────────────

#[test]
fn lists_single_skill() {
    Cmd::given()
        .with_skill("hello-skill", "A friendly skill", "Some body text")
        .command_ls()
        .when_run()
        .should_succeed()
        .expect_output("[hello-skill")
        .expect_output("SKILL.md");
}

#[test]
fn empty_include_dir_shows_no_skills_found() {
    Cmd::given()
        .with_empty_include_dir()
        .command_ls()
        .when_run()
        .should_succeed()
        .expect_output("No skills found.");
}

#[test]
fn multiple_includes_preserve_discovery_order() {
    Cmd::given()
        .with_skill("alpha", "First", "body")
        .with_include_dir("second")
        .with_skill("beta", "Second", "body")
        .command_ls()
        .when_run()
        .should_succeed()
        .expect_output("alpha")
        .expect_output("beta");
}

// ── Name formatting ───────────────────────────────────────

#[test]
fn short_names_padded_to_align_paths() {
    Cmd::given()
        .with_skill("short", "desc", "body")
        .with_skill("this-name-is-way-too-long-for-24", "long desc", "body")
        .command_ls()
        .when_run()
        .should_succeed()
        .expect_output("[short                   ]")
        .expect_output("[this-name-is-way-too-...]");
}

#[test]
fn exact_24_char_name_not_truncated() {
    Cmd::given()
        .with_skill("abcdefghijklmnopqrstuvwx", "exactly 24", "body")
        .command_ls()
        .when_run()
        .should_succeed()
        .expect_output("[abcdefghijklmnopqrstuvwx]");
}

// ── Duplicate handling ────────────────────────────────────

#[test]
fn duplicate_skill_names_first_wins_stderr_warning() {
    // Use two separate include dirs so both contain a skill with the same name.
    Cmd::given()
        .with_skill("dup-skill", "First occurrence", "body A")
        .with_include_dir("second")
        .with_skill("dup-skill", "Second occurrence", "body B")
        .command_ls()
        .when_run()
        .should_succeed()
        .expect_output("[dup-skill")
        .expect_stderr_contains("warning: duplicate skill 'dup-skill'")
        .expect_stderr_contains("keeping first");
}

// ── Edge cases in skill directories ───────────────────────

#[test]
fn nested_skill_md_inside_skill_subdirectory_ignored() {
    Cmd::given()
        .with_skill("myskill", "Top-level skill", "body")
        .with_file_at(
            "myskill/nested/SKILL.md",
            "---\nname: nested\ndescription: hidden\n---\n# body",
        )
        .command_ls()
        .when_run()
        .should_succeed()
        .expect_output("[myskill")
        .expect_out_does_not_contain("nested");
}

#[test]
fn bad_frontmatter_stderr_warning_skill_excluded() {
    Cmd::given()
        .with_skill_raw(
            "bad-skill",
            "---\nname: bad-skill\ndescription: [unclosed\n---\n# body",
        )
        .command_ls()
        .when_run()
        .should_succeed()
        .expect_output("No skills found.")
        .expect_stderr_contains("warning: SKILL.md has invalid or missing frontmatter");
}

#[test]
fn invalid_name_stderr_warning_skill_still_listed() {
    Cmd::given()
        .with_skill_raw(
            "Bad Name!",
            "---\nname: Bad Name!\ndescription: has spaces and caps\n---\n# body",
        )
        .command_ls()
        .when_run()
        .should_succeed()
        .expect_output("[Bad Name!")
        .expect_stderr_contains("warning: skill 'Bad Name!' has invalid name");
}

// ── Error conditions ──────────────────────────────────────

#[test]
fn include_path_is_file_error_exit_nonzero() {
    Cmd::given()
        .with_file_include("not-a-dir")
        .command_ls()
        .when_run()
        .should_fail()
        .expect_stderr_contains("error: include path")
        .expect_stderr_contains("is a file, not a directory");
}

// Empty-path test lives in lib.rs as a unit test — clap intercepts
// `--include ""` as a missing required value before it reaches our code.

// ── Unreadable subdirectory ───────────────────────────────

#[test]
fn unreadable_subdirectory_stderr_warning_sibling_skills_found() {
    // Create a temp dir with a skill and a locked subdir
    let tmp = assert_fs::TempDir::new().unwrap();
    let skill_dir = tmp.child("good-skill");
    std::fs::create_dir(&skill_dir).unwrap();
    let skill_md = skill_dir.child("SKILL.md");
    std::fs::write(
        &skill_md,
        "---\nname: good-skill\ndescription: still found\n---\n# body",
    )
    .unwrap();

    // Create a locked subdir that can't be listed
    let locked = tmp.child("locked-dir");
    std::fs::create_dir(&locked).unwrap();
    let mut perms = std::fs::metadata(&locked).unwrap().permissions();
    perms.set_readonly(true);
    // On Unix, remove user read/execute to make it unlistable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o000);
    }
    std::fs::set_permissions(&locked, perms).unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("skills-primer").unwrap();
    cmd.args(["--include", tmp.to_str().unwrap(), "ls"]);

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "expected exit 0, got {:?}",
        output.status
    );
    assert!(
        stdout.contains("good-skill"),
        "stdout should contain good-skill, got: {}",
        stdout
    );
    assert!(
        stderr.contains("warning: unable to read directory"),
        "stderr should contain warning about locked dir, got: {}",
        stderr
    );
}
