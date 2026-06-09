mod common;
use common::Cmd;
use rstest::rstest;

#[test]
fn prime_should_fail_when_include_is_a_file() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let file_path = tmp.path().join("not-a-dir.txt");
    std::fs::write(&file_path, "this is a file, not a directory").unwrap();

    Cmd::given()
        .arg("--include")
        .arg(file_path.to_str().unwrap())
        .when_run()
        .should_fail()
        .expect_stderr_contains("is a file");
}

#[test]
fn prime_should_fail_when_include_path_has_no_value() {
    Cmd::given()
        .arg("--include")
        .when_run()
        .should_fail()
        .expect_stderr_contains("a value is required");
}

#[rstest]
#[case::path_is_empty("", false, "a value is required")]
#[case::path_does_not_exist(
    "/nonexistent/path/that/does/not/exist",
    true,
    "warning: include directory not found",
)]
fn include_path_validation(
    #[case] path: &str,
    #[case] expect_success: bool,
    #[case] expected_stderr: &str,
) {
    let result = Cmd::given()
        .arg("--include")
        .arg(path)
        .when_run();

    let result = if expect_success {
        result.should_succeed()
    } else {
        result.should_fail()
    };
    result.expect_stderr_contains(expected_stderr);
}
