mod common;
use common::Cmd;

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
        .expect_stderr_contains("warning: duplicate skill 'shared-skill' at ")
        .expect_stderr_dir("second")
        .expect_stderr_contains("/SKILL.md, keeping first");
}
