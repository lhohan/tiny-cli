mod common;
use common::Cmd;

#[test]
fn prime_should_output_skills_instructions() {
    Cmd::given()
        .args(&["prime"])
        .when_run()
        .should_succeed()
        .expect_prime_instructions();
}

#[test]
fn no_arg_should_print_help() {
    Cmd::given()
        .when_run()
        .should_succeed()
        .expect_help_printed();
}

#[test]
fn help_flag_should_print_help() {
    Cmd::given()
        .arg("help")
        .when_run()
        .should_succeed()
        .expect_help_printed();
}

#[test]
fn discovers_skills_from_include_directory() {
    Cmd::given()
        .with_include_dir()
        .with_skill(
            "example-skill",
            "Use when testing example scenarios.",
            "# Example Skill\nDo the thing.",
        )
        .when_run()
        .should_succeed()
        .expect_skill("example-skill", "Use when testing example scenarios");
}

#[test]
fn empty_include_directory_yields_zero_skills() {
    Cmd::given()
        .with_include_dir()
        .when_run()
        .should_succeed()
        .expect_available_skills();
}

#[test]
fn multiple_include_directories_combine_skills() {
    Cmd::given()
        .with_include_dir()
        .with_include_dir()
        .with_skill_in(0, "skill-a", "First skill.", "# Skill A")
        .with_skill_in(1, "skill-b", "Second skill.", "# Skill B")
        .when_run()
        .should_succeed()
        .expect_skill("skill-a", "First skill.")
        .expect_skill("skill-b", "Second skill.");
}

// ── Failure / edge cases ────────────────────────────────────

#[test]
fn bad_frontmatter_should_skip() {
    use assert_fs::fixture::{FileWriteStr, PathChild};
    let tmp = assert_fs::TempDir::new().unwrap();
    // SKILL.md with frontmatter delimiters but unparseable YAML.
    // Serde_yaml returns `Err` for bad YAML, so the skill is skipped.
    tmp.child("broken/SKILL.md")
        .write_str("---\nname: broken\ndescription: [unclosed\n---\n# Nope\n")
        .unwrap();

    Cmd::given()
        .arg("--include")
        .arg(tmp.to_str().unwrap())
        .when_run()
        .should_succeed()
        .expect_available_skills();
}

#[test]
fn skill_without_frontmatter_is_skipped() {
    use assert_fs::fixture::{FileWriteStr, PathChild};
    let tmp = assert_fs::TempDir::new().unwrap();
    // SKILL.md that exists but has no frontmatter delimiters.
    tmp.child("no-frontmatter/SKILL.md")
        .write_str("# Just a heading\n")
        .unwrap();

    Cmd::given()
        .arg("--include")
        .arg(tmp.to_str().unwrap())
        .when_run()
        .should_succeed()
        .expect_available_skills();
}

#[test]
fn empty_subdirectory_with_no_skilly_md_produces_no_skills() {
    use assert_fs::fixture::{PathChild, PathCreateDir};
    let tmp = assert_fs::TempDir::new().unwrap();
    tmp.child("empty-subdir").create_dir_all().unwrap();

    Cmd::given()
        .arg("--include")
        .arg(tmp.to_str().unwrap())
        .when_run()
        .should_succeed()
        .expect_available_skills();
}
