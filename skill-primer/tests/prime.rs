mod common;
use common::Cmd;

mod basics {
    use super::*;

    #[test]
    fn prime_should_output_skills_instructions() {
        Cmd::given()
            .args(&["prime"])
            .when_run()
            .should_succeed()
            .expect_prime_instructions();
    }
}

mod discovery {
    use super::*;
    use assert_fs::fixture::{FileWriteStr, PathChild};
    use indoc::indoc;

    #[test]
    fn prime_should_discover_skills_from_default_path() {
        Cmd::given()
            .command_prime()
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
    fn prime_should_not_discover_skills_when_default_path_empty() {
        Cmd::given()
            .command_prime()
            .when_run()
            .should_succeed()
            .expect_prime_instructions()
            .expect_no_skills_detected()
            .expect_out_does_not_contain("<skill>");
    }

    #[test]
    fn prime_should_discover_skills_from_custom_path() {
        Cmd::given()
            .command_prime()
            .with_path(".codex/skills")
            .with_skill("skill-b", "Second skill.", "# Skill B")
            .when_run()
            .should_succeed()
            .expect_prime_instructions()
            .expect_available_skills()
            .expect_skill("skill-b", "Second skill.");
    }

    #[test]
    fn prime_should_skip_skill_with_bad_frontmatter() {
        Cmd::given()
            .command_prime()
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
    fn prime_should_deduplicate_skills_by_name_from_walked_paths() {
        Cmd::given()
            .command_prime()
            .with_subdir_skill(
                "project",
                "shared-skill",
                "Project description",
                "# Project",
            )
            .with_home_skill("shared-skill", "Home description", "# Home")
            .with_cwd("project/a")
            .when_run()
            .should_succeed()
            .expect_prime_instructions()
            .expect_available_skills()
            .expect_skill("shared-skill", "Project description")
            .expect_output_count("<name>shared-skill</name>", 1)
            .expect_stderr_contains("warning: duplicate skill 'shared-skill' at ")
            .expect_stderr_contains("/SKILL.md, keeping first");
    }

    #[test]
    fn prime_should_not_treat_nested_skill_md_as_separate_skill() {
        let tmp = assert_fs::TempDir::new().unwrap();

        tmp.child("foo/SKILL.md")
            .write_str("---\nname: foo\ndescription: Foo skill\n---\n# Foo")
            .unwrap();
        tmp.child("foo/assets/SKILL.md")
            .write_str("---\nname: nested-asset\ndescription: Asset doc\n---\n# Asset")
            .unwrap();

        Cmd::given()
            .arg("prime")
            .arg("--path")
            .arg(".")
            .with_cwd_dir(tmp.path())
            .with_env("HOME", tmp.to_str().unwrap())
            .when_run()
            .should_succeed()
            .expect_skill("foo", "Foo skill")
            .expect_output_count("<name>foo</name>", 1)
            .expect_out_does_not_contain("nested-asset");
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
            .command_prime()
            .with_path(".")
            .with_cwd_dir(tmp.path())
            .with_env("HOME", tmp.to_str().unwrap())
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
}

mod name {
    use super::*;
    use indoc::indoc;
    use rstest::rstest;

    #[rstest]
    #[case::contains_uppercase_letters("My-Skill", "invalid name")]
    #[case::contains_consecutive_hyphens("pdf--name", "consecutive hyphens")]
    #[case::starts_with_hyphen("-pdf", "starts with hyphen")]
    #[case::ends_with_hyphen("pdf-", "ends with hyphen")]
    #[case::contains_non_alphanumeric_chars("café", "invalid character")]
    #[case::exceeds_64_chars("a".repeat(65), "exceeds 64 characters")]
    fn prime_should_emit_warning_when_skill_name(
        #[case] name: String,
        #[case] expected_warning: &str,
    ) {
        Cmd::given()
            .command_prime()
            .with_skill_raw(
                "my-skill",
                &format!(
                    indoc! {"
                        ---
                        name: {}
                        description: A test skill
                        ---
                        # Body
                    "},
                    name
                ),
            )
            .when_run()
            .should_succeed()
            .expect_skill(&name, "A test skill")
            .expect_stderr_contains("warning")
            .expect_stderr_contains(expected_warning);
    }

    #[rstest]
    #[case::name_contains_only_lowercase_alphanumeric_chars_and_hyphens("my-skill")]
    #[case::name_contains_single_char("a")]
    #[case::name_contains_exactly_64_chars("a".repeat(64))]
    fn skill_should_be_valid_when(#[case] name: String) {
        Cmd::given()
            .command_prime()
            .with_skill_raw(
                "my-skill",
                &format!(
                    indoc! {"
                        ---
                        name: {}
                        description: A test skill
                        ---
                        # Body
                    "},
                    name
                ),
            )
            .when_run()
            .should_succeed()
            .expect_skill(&name, "A test skill")
            .expect_out_does_not_contain("warning");
    }
}

mod path_validation {
    use super::*;
    use rstest::rstest;

    #[test]
    fn prime_should_fail_when_path_is_a_file() {
        Cmd::given()
            .with_file_path("not-a-dir.txt")
            .command_prime()
            .when_run()
            .should_fail()
            .expect_stderr_contains("resolves to a file");
    }

    #[test]
    fn prime_should_fail_when_path_has_no_value() {
        Cmd::given()
            .arg("prime")
            .arg("--path")
            .when_run()
            .should_fail()
            .expect_stderr_contains("a value is required");
    }

    #[rstest]
    #[case::path_is_empty("", false, "a value is required")]
    #[case::path_is_absolute("/nonexistent/path/that/does/not/exist", false, "must be relative")]
    fn path_validation(
        #[case] path: &str,
        #[case] expect_success: bool,
        #[case] expected_stderr: &str,
    ) {
        let result = Cmd::given().arg("prime").arg("--path").arg(path).when_run();

        let result = if expect_success {
            result.should_succeed()
        } else {
            result.should_fail()
        };
        result.expect_stderr_contains(expected_stderr);
    }
}

mod output {
    use super::*;

    #[test]
    fn prime_should_escape_xml_in_skill_name_and_description() {
        Cmd::given()
            .arg("prime")
            .with_skill("<script>alert(1)</script>", "A & B <test>", "# Body")
            .when_run()
            .should_succeed()
            .expect_output("&lt;script&gt;alert(1)&lt;/script&gt;")
            .expect_output("A &amp; B &lt;test&gt;");
    }
}

mod default_walk {
    use super::*;

    #[test]
    fn prime_should_discover_skill_in_home_without_path_flag() {
        Cmd::given()
            .command_prime()
            .with_home_skill("home-skill", "A skill found via walk", "# Body")
            .with_cwd("work")
            .when_run()
            .should_succeed()
            .expect_skill("home-skill", "A skill found via walk");
    }

    #[test]
    fn prime_should_detect_no_skills_in_empty_default_paths() {
        Cmd::given()
            .command_prime()
            .with_cwd("work")
            .when_run()
            .should_succeed()
            .expect_no_skills_detected();
    }

    #[test]
    fn prime_should_discover_skill_in_subdirectory_via_walk() {
        Cmd::given()
            .command_prime()
            .with_subdir_skill("project", "deep-skill", "Found via walk", "# Body")
            .with_cwd("project/a/b/c")
            .when_run()
            .should_succeed()
            .expect_skill("deep-skill", "Found via walk");
    }
}
