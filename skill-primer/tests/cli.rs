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
        .expect_skill("example-skill", "Use when testing example scenarios");
}

#[test]
fn prime_should_not_discover_skills_when_include_empty() {
    Cmd::given()
        .with_empty_include_dir()
        .when_run()
        .should_succeed()
        .expect_available_skills()
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
        .expect_available_skills()
        .expect_stderr_contains("warning: SKILL.md has invalid or missing frontmatter");
}

#[test]
fn prime_should_escape_xml_in_skill_name_and_description() {
    Cmd::given()
        .with_skill("<script>alert(1)</script>", "A & B <test>", "# Body")
        .when_run()
        .should_succeed()
        .expect_output("&lt;script&gt;alert(1)&lt;/script&gt;")
        .expect_output("A &amp; B &lt;test&gt;");
}

#[test]
fn prime_should_not_treat_nested_skill_md_as_separate_skill() {
    use assert_fs::fixture::{FileWriteStr, PathChild};
    let tmp = assert_fs::TempDir::new().unwrap();

    // Top-level skill
    tmp.child("foo/SKILL.md")
        .write_str("---\nname: foo\ndescription: Foo skill\n---\n# Foo")
        .unwrap();
    // Nested SKILL.md inside the skill directory — should be ignored
    tmp.child("foo/assets/SKILL.md")
        .write_str("---\nname: nested-asset\ndescription: Asset doc\n---\n# Asset")
        .unwrap();

    Cmd::given()
        .arg("--include")
        .arg(tmp.to_str().unwrap())
        .when_run()
        .should_succeed()
        .expect_skill("foo", "Foo skill")
        .expect_output_count("<name>foo</name>", 1)
        .expect_out_does_not_contain("nested-asset");
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
        .expect_skill("shared-skill", "Shared description")
        .expect_output_count("<name>shared-skill</name>", 1)
        .expect_stderr_contains("warning: skipping duplicate skill 'shared-skill'")
        .expect_stderr_dir("second")
        .expect_stderr_contains("already included from earlier include directory");
}

#[test]
fn prime_should_fail_when_include_is_a_file() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file_path = tmp.path().join("not-a-dir.txt");
    std::fs::write(&file_path, "this is a file, not a directory").unwrap();

    Cmd::given()
        .arg("--include")
        .arg(file_path.to_str().unwrap())
        .when_run()
        .should_fail()
        .expect_stderr_contains("is a file");
}

#[test]
fn prime_should_fail_when_include_path_is_empty() {
    Cmd::given()
        .arg("--include")
        .arg("")
        .when_run()
        .should_fail()
        .expect_stderr_contains("a value is required");
}
#[test]

fn prime_should_fail_when_include_path_has_no_value() {
    Cmd::given()
        .arg("--include")
        .when_run()
        .should_fail()
        .expect_stderr_contains("a value is required");
}

#[test]
fn prime_should_warn_on_nonexistent_include_directory() {
    Cmd::given()
        .arg("--include")
        .arg("/nonexistent/path/that/does/not/exist")
        .when_run()
        .should_succeed()
        .expect_stderr_contains("warning: include directory not found");
}
