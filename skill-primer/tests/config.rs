mod common;
use common::Cmd;

#[test]
fn config_shows_default_skill_path() {
    Cmd::given()
        .command_config()
        .when_run()
        .should_succeed()
        .expect_output("Skill path:")
        .expect_output(".agents/skills");
}

#[test]
fn config_shows_custom_skill_path() {
    Cmd::given()
        .with_path(".codex/skills")
        .command_config()
        .when_run()
        .should_succeed()
        .expect_output("Skill path:")
        .expect_output(".codex/skills");
}

#[test]
fn config_annotates_existing_resolved_directory_as_found() {
    Cmd::given()
        .with_skill("existing-skill", "desc", "body")
        .command_config()
        .when_run()
        .should_succeed()
        .expect_output("Candidate directories:")
        .expect_output(".agents/skills (found)");
}

#[test]
fn config_does_not_list_agent_specific_paths_by_default() {
    Cmd::given()
        .command_config()
        .when_run()
        .should_succeed()
        .expect_out_does_not_contain(".claude/skills")
        .expect_out_does_not_contain(".codex/skills");
}

/// A non-directory path (a regular file) is rejected with an error,
/// consistent with how `ls` and `prime` handle file paths.
#[test]
fn config_rejects_file_path_as_error() {
    Cmd::given()
        .command_config()
        .with_file_path("not-a-dir")
        .when_run()
        .should_fail()
        .expect_stderr_contains("resolves to a file");
}

#[test]
fn config_shows_selected_path_in_walked_paths() {
    Cmd::given()
        .with_path(".codex/skills")
        .command_config()
        .with_cwd("project/a/b")
        .when_run()
        .should_succeed()
        .expect_output(".codex/skills")
        .expect_out_does_not_contain(".agents/skills");
}
