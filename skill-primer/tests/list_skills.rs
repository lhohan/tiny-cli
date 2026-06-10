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

#[test]
fn walks_upward_to_repo_root_git() {
    Cmd::given()
        .command_ls()
        .with_git_repo()
        .with_repo_skill("found-it", "A skill", "body")
        .with_repo_subdir("a/b/c")
        .when_run()
        .should_succeed()
        .expect_output("[found-it");
}

#[test]
fn outside_repo_walks_to_home() {
    Cmd::given()
        .command_ls()
        .with_home()
        .with_repo_skill("home-skill", "From home", "body")
        .when_run()
        .should_succeed()
        .expect_output("[home-skill");
}

#[test]
fn home_dir_candidates_appended_after_project_paths() {
    Cmd::given()
        .command_ls()
        .with_home()
        .with_repo_skill("home-skill", "From home", "body")
        .with_git_repo()
        .with_repo_skill("project-skill", "In project", "body")
        .with_repo_subdir("a/b")
        .when_run()
        .should_succeed()
        .expect_output_order("[project-skill", "[home-skill");
}

#[test]
fn stops_at_repo_root_does_not_walk_above() {
    let tmp = assert_fs::TempDir::new().unwrap();

    // Skill above the repo root (should not be found).
    std::fs::create_dir_all(tmp.path().join(".agents/skills/above")).unwrap();
    std::fs::write(
        tmp.path().join(".agents/skills/above/SKILL.md"),
        "---\nname: above\ndescription: outside\n---\nbody",
    )
    .unwrap();

    // Git repo inside tmp.
    let repo = tmp.child("repo");
    std::fs::create_dir(repo.path()).unwrap();
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    // Skill inside repo (should be found).
    std::fs::create_dir_all(repo.path().join(".agents/skills/inside")).unwrap();
    std::fs::write(
        repo.path().join(".agents/skills/inside/SKILL.md"),
        "---\nname: inside\ndescription: found\n---\nbody",
    )
    .unwrap();

    // Run from a subdirectory of the repo.
    let subdir = repo.child("x/y/z");
    std::fs::create_dir_all(subdir.path()).unwrap();

    let home_tmp = assert_fs::TempDir::new().unwrap();

    Cmd::given()
        .command_ls()
        .with_cwd_dir(subdir.path())
        .with_env("HOME", home_tmp.path().to_str().unwrap())
        .when_run()
        .should_succeed()
        .expect_output("[inside")
        .expect_out_does_not_contain("[above");
}

#[test]
fn deduplicates_canonically_identical_paths() {
    let tmp = assert_fs::TempDir::new().unwrap();
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    // Create a skill at .agents/skills/my-skill.
    std::fs::create_dir_all(tmp.path().join(".agents/skills/my-skill")).unwrap();
    std::fs::write(
        tmp.path().join(".agents/skills/my-skill/SKILL.md"),
        "---\nname: my-skill\ndescription: a skill\n---\nbody",
    )
    .unwrap();

    // Symlink .claude/skills -> .agents/skills.
    std::fs::create_dir(tmp.path().join(".claude")).unwrap();
    std::os::unix::fs::symlink(
        tmp.path().join(".agents/skills"),
        tmp.path().join(".claude/skills"),
    )
    .unwrap();

    let home_tmp = assert_fs::TempDir::new().unwrap();

    Cmd::given()
        .command_ls()
        .with_cwd_dir(tmp.path())
        .with_env("HOME", home_tmp.path().to_str().unwrap())
        .when_run()
        .should_succeed()
        .expect_output_count("[my-skill", 1);
}

#[test]
fn duplicate_skill_names_across_walk_and_home_first_wins() {
    Cmd::given()
        .command_ls()
        .with_home()
        .with_repo_skill("conflict", "From home", "body")
        .with_git_repo()
        .with_repo_skill("conflict", "From project", "body")
        .when_run()
        .should_succeed()
        .expect_output_count("[conflict", 1)
        .expect_stderr_contains("duplicate skill 'conflict'");
}

#[test]
fn walks_upward_to_repo_root_jj() {
    if !has_command("jj") {
        return;
    }

    let tmp = assert_fs::TempDir::new().unwrap();
    let init = std::process::Command::new("jj")
        .args(["git", "init", "--git-repo=."])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    if !init.status.success() {
        eprintln!("SKIP: jj git init failed");
        return;
    }

    std::fs::create_dir_all(tmp.path().join(".agents/skills/jj-skill")).unwrap();
    std::fs::write(
        tmp.path().join(".agents/skills/jj-skill/SKILL.md"),
        "---\nname: jj-skill\ndescription: from jj repo\n---\nbody",
    )
    .unwrap();

    let subdir = tmp.child("a/b");
    std::fs::create_dir_all(subdir.path()).unwrap();

    Cmd::given()
        .command_ls()
        .with_cwd_dir(subdir.path())
        .when_run()
        .should_succeed()
        .expect_output("[jj-skill");
}

#[test]
fn jj_fails_falls_back_to_git() {
    if !has_command("jj") || !has_command("git") {
        return;
    }

    let tmp = assert_fs::TempDir::new().unwrap();
    // Pure git repo — NOT a jj workspace.
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    // jj root should fail here — verify the fallback works.
    std::fs::create_dir_all(tmp.path().join(".agents/skills/git-skill")).unwrap();
    std::fs::write(
        tmp.path().join(".agents/skills/git-skill/SKILL.md"),
        "---\nname: git-skill\ndescription: via git fallback\n---\nbody",
    )
    .unwrap();

    let subdir = tmp.child("x/y");
    std::fs::create_dir_all(subdir.path()).unwrap();

    Cmd::given()
        .command_ls()
        .with_cwd_dir(subdir.path())
        .when_run()
        .should_succeed()
        .expect_output("[git-skill");
}

fn has_command(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .output()
        .is_ok()
}
