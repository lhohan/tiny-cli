mod common;
use assert_fs::fixture::{FileWriteStr, PathChild};
use common::Cmd;
use indoc::indoc;

#[test]
fn prime_should_discover_skills_when_include_provided() {
    Cmd::given()
        .with_include_dir("skills-dir")
        .with_skill(
            "example-skill",
            "Use when testing example scenarios.",
            "# Example Skill\nDo the thing.",
        )
        .when_run()
        .should_succeed()
        .expect_prime_instructions()
        .expect_available_skills()
        .expect_skill("example-skill", "Use when testing example scenarios");
}

#[test]
fn prime_should_not_discover_skills_when_include_empty() {
    Cmd::given()
        .with_empty_include_dir()
        .when_run()
        .should_succeed()
        .expect_prime_instructions()
        .expect_no_skills_detected()
        .expect_out_does_not_contain("<skill>");
}

#[test]
fn prime_should_merge_skills_from_multiple_includes() {
    Cmd::given()
        .with_include_dir("first")
        .with_skill("skill-a", "First skill.", "# Skill A")
        .with_include_dir("second")
        .with_skill("skill-b", "Second skill.", "# Skill B")
        .when_run()
        .should_succeed()
        .expect_prime_instructions()
        .expect_available_skills()
        .expect_skill("skill-a", "First skill.")
        .expect_skill("skill-b", "Second skill.");
}

#[test]
fn prime_should_skip_skill_with_bad_frontmatter() {
    Cmd::given()
        .with_skill_raw(
            "skill_dir",
            "---\nname: broken\ndescription: [unclosed\n---\n# Nope\n",
        )
        .when_run()
        .should_succeed()
        .expect_prime_instructions()
        .expect_no_skills_detected()
        .expect_stderr_contains("warning: SKILL.md has invalid or missing frontmatter");
}

#[test]
fn prime_should_deduplicate_skills_by_name_from_multiple_includes() {
    Cmd::given()
        .with_include_dir("first")
        .with_skill("shared-skill", "Shared description", "# Shared")
        .with_include_dir("second")
        .with_skill("shared-skill", "Shared description", "# Shared")
        .when_run()
        .should_succeed()
        .expect_prime_instructions()
        .expect_available_skills()
        .expect_skill("shared-skill", "Shared description")
        .expect_output_count("<name>shared-skill</name>", 1)
        .expect_stderr_contains("warning: duplicate skill 'shared-skill' at ")
        .expect_stderr_dir("second")
        .expect_stderr_contains("/SKILL.md, keeping first");
}

#[test]
fn unreadable_subdirectory_should_warn() {
    let tmp = assert_fs::TempDir::new().unwrap();

    // A readable skill directory
    let good_dir = tmp.child("good-skill");
    std::fs::create_dir(&good_dir).unwrap();
    good_dir
        .child("SKILL.md")
        .write_str(indoc! {"
            ---
            name: good-skill
            description: Should be found
            ---
            # Body
        "})
        .unwrap();

    // A subdirectory with no read/execute permission
    let bad_dir = tmp.child("hidden-dir");
    std::fs::create_dir(&bad_dir).unwrap();
    deny_all_permissions(&bad_dir);

    // Run the tool against the parent directory — expect a warning
    Cmd::given()
        .with_include(tmp.to_str().unwrap())
        .when_run()
        .should_succeed()
        .expect_skill("good-skill", "Should be found")
        .expect_stderr_contains("warning: unable to read directory");

    // Restore before TempDir drop so cleanup doesn't fail
    restore_permissions(&bad_dir);
}

#[cfg(unix)]
fn deny_all_permissions(path: &std::path::Path) {
    let mut perms = std::fs::metadata(path).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o000);
    std::fs::set_permissions(path, perms).unwrap();
}

#[cfg(unix)]
fn restore_permissions(path: &std::path::Path) {
    let mut perms = std::fs::metadata(path).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(path, perms).ok();
}

#[cfg(not(unix))]
fn deny_all_permissions(path: &std::path::Path) {
    let mut perms = std::fs::metadata(path).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(path, perms).unwrap();
}

#[cfg(not(unix))]
fn restore_permissions(path: &std::path::Path) {
    let mut perms = std::fs::metadata(path).unwrap().permissions();
    perms.set_readonly(false);
    std::fs::set_permissions(path, perms).ok();
}
