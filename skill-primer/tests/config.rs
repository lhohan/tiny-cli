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

/// With a non-existent --include path, the output annotates it (not found).
#[test]
fn config_with_missing_include_path_marks_it_not_found() {
    Cmd::given()
        .command_config()
        .with_include("definitely-not-here")
        .when_run()
        .should_succeed()
        .expect_output("Include paths:")
        .expect_annotated("definitely-not-here", "not found")
        .expect_output("Default paths are overridden by --include.");
}

/// When mixing a real dir and a missing path, each gets the correct annotation
/// and the found one appears before the missing one in output order.
#[test]
fn config_mixes_found_and_missing_annotations_in_order() {
    Cmd::given()
        .command_config()
        .with_include_dir("existing")
        .with_include("missing-dir")
        .when_run()
        .should_succeed()
        .expect_annotated("existing", "found")
        .expect_annotated("missing-dir", "not found")
        .expect_output_order("(found)", "(not found)")
        .expect_output_count("Default paths are overridden by --include.", 1);
}

/// Without --include, both section headers appear and the configured
/// directories section lists the three HOME patterns.
#[test]
fn config_without_includes_shows_configured_and_project_sections() {
    Cmd::given()
        .command_config()
        .with_cwd("project/sub")
        .when_run()
        .should_succeed()
        .expect_output("Configured directories:")
        .expect_output("Project directories:")
        .expect_output("~/.agents/skills")
        .expect_output("~/.claude/skills")
        .expect_output("~/.codex/skills");
}

/// A non-directory path like /dev/null is annotated (not found), not treated
/// as an error. The real dir still gets (found).
#[test]
fn config_treats_non_directory_include_as_not_found() {
    Cmd::given()
        .command_config()
        .with_include_dir("good")
        .with_include("/dev/null")
        .when_run()
        .should_succeed()
        .expect_annotated("good", "found")
        .expect_annotated("/dev/null", "not found");
}

/// When HOME is unset (empty string), the configured directories are listed
/// with a note that HOME is not set.
#[test]
fn config_shows_skipped_when_home_is_unset() {
    Cmd::given()
        .command_config()
        .with_env("HOME", "")
        .when_run()
        .should_succeed()
        .expect_output("Configured directories:")
        .expect_output("(skipped - HOME not set)");
}

/// When CWD is deep in a subdirectory, project directories show walked paths.
#[test]
fn config_shows_walked_paths_from_deep_cwd() {
    Cmd::given()
        .command_config()
        .with_cwd("project/a/b")
        .when_run()
        .should_succeed()
        .expect_output("Configured directories:")
        .expect_output("Project directories:")
        .expect_output(".agents/skills")
        .expect_output(".claude/skills")
        .expect_output(".codex/skills");
}
