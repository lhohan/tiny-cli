mod common;
use common::Cmd;
use indoc::indoc;
use rstest::rstest;

#[rstest]
#[case::contains_uppercase_letters("My-Skill", "invalid name")]
#[case::contains_consecutive_hyphens("pdf--name", "consecutive hyphens")]
#[case::starts_with_hyphen("-pdf", "starts with hyphen")]
#[case::ends_with_hyphen("pdf-", "ends with hyphen")]
#[case::contains_non_alphanumeric_chars("café", "invalid character")]
#[case::exceeds_64_chars("a".repeat(65), "exceeds 64 characters")]
fn prime_should_emit_warning_when_skill_name(#[case] name: String, #[case] expected_warning: &str) {
    Cmd::given()
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
