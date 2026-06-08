mod common;
use common::Cmd;
use indoc::indoc;
use assert_fs::fixture::{PathChild, FileWriteStr};

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

#[test]
fn discovers_skills_from_include_directory() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let skill_file = tmp.child("example-skill/SKILL.md");
    skill_file.write_str(indoc! {r#"
        ---
        name: example-skill
        description: Use when testing example scenarios.
        ---
        # Example Skill
        Do the thing.
    "#}).unwrap();

    Cmd::given()
        .arg("--include").arg(tmp.to_str().unwrap())
        .when_run()
        .should_succeed()
        .expect_output("example-skill")
        .expect_output("Use when testing example scenarios");
}

#[test]
fn empty_include_directory_yields_zero_skills() {
    let tmp = assert_fs::TempDir::new().unwrap();

    Cmd::given()
        .arg("--include").arg(tmp.to_str().unwrap())
        .when_run()
        .should_succeed()
        .expect_output("<available_skills>")
        .expect_output("</available_skills>");
    // No skill names should appear between the tags.
}

#[test]
fn multiple_include_directories_combine_skills() {
    let tmp1 = assert_fs::TempDir::new().unwrap();
    let skill_file1 = tmp1.child("skill-a/SKILL.md");
    skill_file1.write_str(indoc! {r#"
        ---
        name: skill-a
        description: First skill.
        ---
        # Skill A
    "#}).unwrap();

    let tmp2 = assert_fs::TempDir::new().unwrap();
    let skill_file2 = tmp2.child("skill-b/SKILL.md");
    skill_file2.write_str(indoc! {r#"
        ---
        name: skill-b
        description: Second skill.
        ---
        # Skill B
    "#}).unwrap();

    Cmd::given()
        .arg("--include").arg(tmp1.to_str().unwrap())
        .arg("--include").arg(tmp2.to_str().unwrap())
        .when_run()
        .should_succeed()
        .expect_output("skill-a")
        .expect_output("First skill.")
        .expect_output("skill-b")
        .expect_output("Second skill.");
}
