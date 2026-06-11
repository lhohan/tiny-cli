mod common;
use common::Cmd;

#[test]
fn config_annotates_existing_include_dir_as_found() {
    Cmd::given()
        .command_config()
        .with_include_dir("my-skills")
        .when_run()
        .should_succeed()
        .expect_annotated("my-skills", "found");
}

#[test]
fn config_annotates_missing_include_path_as_not_found() {
    Cmd::given()
        .command_config()
        .with_include("definitely-not-here")
        .when_run()
        .should_succeed()
        .expect_annotated("definitely-not-here", "not found");
}

#[test]
fn config_annotates_multiple_include_paths_independently() {
    Cmd::given()
        .command_config()
        .with_include_dir("existing")
        .with_include("missing-dir")
        .when_run()
        .should_succeed()
        .expect_annotated("existing", "found")
        .expect_annotated("missing-dir", "not found");
}

#[rstest::rstest]
#[case("~/.agents/skills")]
#[case("~/.claude/skills")]
#[case("~/.codex/skills")]
fn config_lists_the_configured_directory_patterns_in_home_dir(#[case] expected: &str) {
    Cmd::given()
        .command_config()
        .when_run()
        .should_succeed()
        .expect_output(expected);
}

/// A non-directory path (a regular file) is rejected with an error,
/// consistent with how `ls` and `prime` handle file paths.
#[test]
fn config_rejects_file_path_as_error() {
    Cmd::given()
        .command_config()
        .with_file_include("not-a-dir")
        .when_run()
        .should_fail()
        .expect_stderr_contains("is a file, not a directory");
}

#[rstest::rstest]
#[case(".agents/skills")]
#[case(".claude/skills")]
#[case(".codex/skills")]
fn config_shows_skill_directory_names_in_walked_paths(#[case] expected: &str) {
    Cmd::given()
        .command_config()
        .with_cwd("project/a/b")
        .when_run()
        .should_succeed()
        .expect_output(expected);
}
