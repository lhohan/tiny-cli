mod common;
use common::Cmd;

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
fn no_subcommand_with_path_should_error_and_show_help() {
    Cmd::given()
        .with_path(".codex/skills")
        .when_run()
        .should_fail()
        .expect_stderr_contains("subcommand is required");
}

#[test]
fn version_flag_prints_version() {
    Cmd::given()
        .arg("--version")
        .when_run()
        .should_succeed()
        .expect_output(env!("CARGO_PKG_VERSION"));
}

#[test]
fn warnings_flags_should_be_mutually_exclusive() {
    Cmd::given()
        .args(&["--warnings", "--no-warnings", "prime"])
        .when_run()
        .should_fail()
        .expect_stderr_contains("cannot be used with");
}
