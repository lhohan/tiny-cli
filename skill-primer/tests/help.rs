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
fn no_subcommand_with_include_should_error_and_show_help() {
    Cmd::given()
        .with_empty_include_dir()
        .when_run()
        .should_fail()
        .expect_stderr_contains("subcommand is required");
}
