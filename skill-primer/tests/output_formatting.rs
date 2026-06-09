mod common;
use common::Cmd;
use assert_fs::fixture::{FileWriteStr, PathChild};

#[test]
fn prime_should_escape_xml_in_skill_name_and_description() {
    Cmd::given()
        .with_skill("<script>alert(1)</script>", "A & B <test>", "# Body")
        .when_run()
        .should_succeed()
        .expect_output("&lt;script&gt;alert(1)&lt;/script&gt;")
        .expect_output("A &amp; B &lt;test&gt;");
}

#[test]
fn prime_should_not_treat_nested_skill_md_as_separate_skill() {
    let tmp = assert_fs::TempDir::new().unwrap();

    tmp.child("foo/SKILL.md")
        .write_str("---\nname: foo\ndescription: Foo skill\n---\n# Foo")
        .unwrap();
    tmp.child("foo/assets/SKILL.md")
        .write_str("---\nname: nested-asset\ndescription: Asset doc\n---\n# Asset")
        .unwrap();

    Cmd::given()
        .arg("--include")
        .arg(tmp.to_str().unwrap())
        .when_run()
        .should_succeed()
        .expect_skill("foo", "Foo skill")
        .expect_output_count("<name>foo</name>", 1)
        .expect_out_does_not_contain("nested-asset");
}
