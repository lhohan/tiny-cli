mod common;
use assert_fs::fixture::PathChild;
use common::Cmd;

mod basic_discovery {
    use super::*;

    #[test]
    fn ls_should_list_single_skill() {
        Cmd::given()
            .with_skill("hello-skill", "A friendly skill", "Some body text")
            .command_ls()
            .when_run()
            .should_succeed()
            .expect_output("[hello-skill")
            .expect_output("SKILL.md");
    }

    #[test]
    fn ls_should_report_no_skills_found() {
        Cmd::given()
            .command_ls()
            .when_run()
            .should_succeed()
            .expect_output("No skills found.");
    }

    #[test]
    fn ls_should_use_custom_relative_path() {
        Cmd::given()
            .with_path(".codex/skills")
            .with_skill("codex-skill", "Custom path", "body")
            .command_ls()
            .when_run()
            .should_succeed()
            .expect_output("codex-skill");
    }
}

mod name_formatting {
    use super::*;

    #[test]
    fn ls_should_pad_short_names_to_align_paths() {
        Cmd::given()
            .with_skill("short", "desc", "body")
            .with_skill("this-name-is-way-too-long-for-24", "long desc", "body")
            .command_ls()
            .when_run()
            .should_succeed()
            .expect_output("[short                   ]")
            .expect_output("[this-name-is-way-too-...]");
    }

    #[test]
    fn ls_should_not_truncate_exact_24_char_name() {
        Cmd::given()
            .with_skill("abcdefghijklmnopqrstuvwx", "exactly 24", "body")
            .command_ls()
            .when_run()
            .should_succeed()
            .expect_output("[abcdefghijklmnopqrstuvwx]");
    }
}

mod error_conditions {
    use super::*;

    #[test]
    fn ls_should_fail_when_path_is_file() {
        Cmd::given()
            .with_file_path("not-a-dir")
            .command_ls()
            .when_run()
            .should_fail()
            .expect_stderr_contains("error: --path")
            .expect_stderr_contains("resolves to a file");
    }

    #[test]
    fn ls_should_fail_when_path_is_absolute() {
        Cmd::given()
            .arg("--path")
            .arg("/tmp/skills")
            .command_ls()
            .when_run()
            .should_fail()
            .expect_stderr_contains("must be relative");
    }

    #[test]
    fn ls_should_fail_when_path_is_repeated() {
        Cmd::given()
            .arg("--path")
            .arg(".agents/skills")
            .arg("--path")
            .arg(".codex/skills")
            .command_ls()
            .when_run()
            .should_fail()
            .expect_stderr_contains("--path can only be specified once");
    }

    // Empty-path test lives in lib.rs as a unit test because clap intercepts
    // `--path ""` as a missing required value before it reaches our code.
}

mod edge_cases {
    use super::*;

    #[test]
    fn ls_should_ignore_nested_skill_md() {
        Cmd::given()
            .with_skill("myskill", "Top-level skill", "body")
            .with_file_at(
                "myskill/nested/SKILL.md",
                "---\nname: nested\ndescription: hidden\n---\n# body",
            )
            .command_ls()
            .when_run()
            .should_succeed()
            .expect_output("[myskill")
            .expect_out_does_not_contain("nested");
    }

    #[test]
    fn ls_should_exclude_skill_with_invalid_frontmatter() {
        Cmd::given()
            .with_skill_raw(
                "bad-skill",
                "---\nname: bad-skill\ndescription: [unclosed\n---\n# body",
            )
            .command_ls()
            .when_run()
            .should_succeed()
            .expect_output("No skills found.")
            .expect_stderr_contains("warning: SKILL.md has invalid or missing frontmatter");
    }

    #[test]
    fn ls_should_suppress_warnings_when_disabled() {
        Cmd::given()
            .without_warnings()
            .with_skill_raw(
                "bad-skill",
                "---\nname: bad-skill\ndescription: [unclosed\n---\n# body",
            )
            .command_ls()
            .when_run()
            .should_succeed()
            .expect_output("No skills found.")
            .expect_stderr_does_not_contain("warning:");
    }

    #[test]
    fn ls_should_list_skill_with_invalid_name() {
        Cmd::given()
            .with_skill_raw(
                "Bad Name!",
                "---\nname: Bad Name!\ndescription: has spaces and caps\n---\n# body",
            )
            .command_ls()
            .when_run()
            .should_succeed()
            .expect_output("[Bad Name!")
            .expect_stderr_contains("warning: skill 'Bad Name!' has invalid name");
    }

    #[test]
    fn ls_should_find_sibling_skills_when_subdirectory_unreadable() {
        // Create a temp dir with a skill and a locked subdir
        let tmp = assert_fs::TempDir::new().unwrap();
        let skill_dir = tmp.child("good-skill");
        std::fs::create_dir(&skill_dir).unwrap();
        let skill_md = skill_dir.child("SKILL.md");
        std::fs::write(
            &skill_md,
            "---\nname: good-skill\ndescription: still found\n---\n# body",
        )
        .unwrap();

        // Create a locked subdir that can't be listed
        let locked = tmp.child("locked-dir");
        std::fs::create_dir(&locked).unwrap();
        let mut perms = std::fs::metadata(&locked).unwrap().permissions();
        perms.set_readonly(true);
        // On Unix, remove user read/execute to make it unlistable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o000);
        }
        std::fs::set_permissions(&locked, perms).unwrap();

        let mut cmd = assert_cmd::Command::cargo_bin("skills-primer").unwrap();
        cmd.current_dir(tmp.path());
        cmd.env("HOME", tmp.path());
        cmd.args(["--path", ".", "ls"]);

        let output = cmd.output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "expected exit 0, got {:?}",
            output.status
        );
        assert!(
            stdout.contains("good-skill"),
            "stdout should contain good-skill, got: {}",
            stdout
        );
        assert!(
            stderr.contains("warning: unable to read directory"),
            "stderr should contain warning about locked dir, got: {}",
            stderr
        );
    }
}

mod duplicate_handling {
    use super::*;

    #[test]
    fn ls_should_keep_first_duplicate_skill_name() {
        Cmd::given()
            .with_subdir_skill("project", "dup-skill", "First occurrence", "body A")
            .with_home_skill("dup-skill", "Second occurrence", "body B")
            .with_cwd("project/a")
            .command_ls()
            .when_run()
            .should_succeed()
            .expect_output("[dup-skill")
            .expect_stderr_contains("warning: duplicate skill 'dup-skill'")
            .expect_stderr_contains("keeping first");
    }
}

mod default_walk {
    use super::*;

    #[test]
    fn ls_should_find_skill_at_project_level() {
        Cmd::given()
            .command_ls()
            .with_subdir_skill("project", "found-it", "A skill", "body")
            .with_cwd("project/a/b/c")
            .when_run()
            .should_succeed()
            .expect_output("[found-it");
    }

    #[test]
    fn ls_should_find_skill_at_home_level() {
        Cmd::given()
            .command_ls()
            .with_home_skill("home-skill", "From home", "body")
            .with_cwd("work")
            .when_run()
            .should_succeed()
            .expect_output("[home-skill");
    }

    #[test]
    fn ls_should_find_home_skills_after_project_skills() {
        Cmd::given()
            .command_ls()
            .with_subdir_skill("project", "project-skill", "In project", "body")
            .with_home_skill("home-skill", "From home", "body")
            .with_cwd("project/a/b")
            .when_run()
            .should_succeed()
            .expect_output_order("[project-skill", "[home-skill");
    }

    #[test]
    fn ls_should_find_skills_at_every_level_up_to_home() {
        Cmd::given()
            .command_ls()
            .with_subdir_skill("project/a/b", "deep-skill", "Deep inside", "body")
            .with_subdir_skill("project", "project-skill", "In project", "body")
            .with_home_skill("home-skill", "From home", "body")
            .with_cwd("project/a/b")
            .when_run()
            .should_succeed()
            .expect_output("[deep-skill")
            .expect_output("[project-skill")
            .expect_output("[home-skill")
            .expect_output_order("[deep-skill", "[project-skill")
            .expect_output_order("[project-skill", "[home-skill");
    }

    #[test]
    fn ls_should_deduplicate_symlinked_directories() {
        let home = assert_fs::TempDir::new().unwrap();
        std::fs::create_dir_all(home.path().join(".agents/skills/my-skill")).unwrap();
        std::fs::write(
            home.path().join(".agents/skills/my-skill/SKILL.md"),
            "---\nname: my-skill\ndescription: a skill\n---\nbody",
        )
        .unwrap();
        std::fs::create_dir_all(home.path().join("project/.agents")).unwrap();
        std::os::unix::fs::symlink(
            home.path().join(".agents/skills"),
            home.path().join("project/.agents/skills"),
        )
        .unwrap();

        Cmd::given()
            .command_ls()
            .with_cwd_dir(&home.path().join("project"))
            .with_env("HOME", home.path().to_str().unwrap())
            .when_run()
            .should_succeed()
            .expect_output_count("[my-skill", 1);
    }

    #[test]
    fn ls_should_keep_first_duplicate_across_directory_levels() {
        Cmd::given()
            .command_ls()
            .with_home_skill("conflict", "From home", "body")
            .with_subdir_skill("project", "conflict", "From project", "body")
            .with_cwd("project/a/b")
            .when_run()
            .should_succeed()
            .expect_output_count("[conflict", 1)
            .expect_stderr_contains("duplicate skill 'conflict'");
    }

    #[test]
    fn ls_should_find_home_skills_when_cwd_outside_home() {
        let home = assert_fs::TempDir::new().unwrap();
        std::fs::create_dir_all(home.path().join(".agents/skills/home-skill")).unwrap();
        std::fs::write(
            home.path().join(".agents/skills/home-skill/SKILL.md"),
            "---\nname: home-skill\ndescription: From home\n---\nbody",
        )
        .unwrap();

        let outside = assert_fs::TempDir::new().unwrap();
        std::fs::create_dir_all(outside.path().join("nested")).unwrap();

        Cmd::given()
            .command_ls()
            .with_cwd_dir(outside.path().join("nested").as_path())
            .with_env("HOME", home.path().to_str().unwrap())
            .when_run()
            .should_succeed()
            .expect_output("[home-skill");
    }
}
