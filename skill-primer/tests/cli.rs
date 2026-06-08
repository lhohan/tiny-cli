mod common;
use common::Cmd;

#[test]
fn prime_should_output_skills_instructions() {
    Cmd::given()
        .args(&["prime"])
        .when_run()
        .should_succeed()
        .expect_skills_header()
        .expect_instructions()
        .expect_available_skills();
}

#[test]
fn no_arg_should_print_help() {
    Cmd::given().when_run().should_succeed().expect_help();
}

#[test]
fn help_flag_should_print_help() {
    Cmd::given()
        .arg("help")
        .when_run()
        .should_succeed()
        .expect_help();
}
