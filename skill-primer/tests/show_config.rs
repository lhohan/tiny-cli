mod common;
use common::Cmd;

#[test]
fn show_config_should_list_included_directories() {
    Cmd::given()
        .command_show_config()
        .with_include_dir("my-skills")
        .when_run()
        .should_succeed()
        .expect_exists_in_config("my-skills")
        .expect_stderr_empty();
}

#[test]
fn show_config_should_mark_missing_path() {
    Cmd::given()
        .command_show_config()
        .with_include("definitely-not-here")
        .when_run()
        .should_succeed()
        .expect_missing_in_config("definitely-not-here")
        .expect_stderr_empty();
}

#[test]
fn show_config_should_distinguish_existing_from_missing() {
    Cmd::given()
        .command_show_config()
        .with_include_dir("existing")
        .with_include("missing-dir")
        .when_run()
        .should_succeed()
        .expect_exists_in_config("existing")
        .expect_missing_in_config("missing-dir")
        .expect_stderr_empty();
}

#[test]
fn show_config_should_output_nothing_without_includes() {
    Cmd::given()
        .command_show_config()
        .when_run()
        .should_succeed()
        .expect_no_output();
}

#[test]
fn show_config_should_mark_error_on_bad_path() {
    Cmd::given()
        .command_show_config()
        .with_include_dir("good")
        .with_include("/dev/null")
        .when_run()
        .should_succeed()
        .expect_exists_in_config("good")
        .expect_error_in_config("/dev/null")
        .expect_stderr_empty();
}
