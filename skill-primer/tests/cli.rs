mod common;
use common::Cmd;
use rstest::rstest;

#[rstest]
#[case(vec![])]
#[case(vec!["prime"])]
fn no_arg_or_prime_should_output_skills_instructions(#[case] args: Vec<&str>) {
    let mut cmd = Cmd::given();
    if !args.is_empty() {
        cmd = cmd.args(&args);
    }
    cmd.when_run()
        .should_succeed()
        .expect_skills_header()
        .expect_instructions()
        .expect_available_skills();
}

#[test]
fn no_arg_and_prime_should_yield_same_result() {
    let no_arg_result = Cmd::given().when_run().should_succeed();
    let prime_result = Cmd::given().arg("prime").when_run().should_succeed();
    let no_arg_stdout = no_arg_result.stdout_str();
    let prime_stdout = prime_result.stdout_str();
    assert_eq!(no_arg_stdout, prime_stdout);
}
