# Plan: First Acceptance Tests (End-to-End)

## Next Task: `--include` Flag with Fixture Directory

**Files:**
- Modify: `Cargo.toml` — add runtime dependency for frontmatter parsing
- Modify: `src/main.rs` — add `--include` CLI arg, SKILL.md scanner, frontmatter parser, dynamic output
- Create: `tests/fixtures/example-skill/SKILL.md` — test fixture
- Modify: `tests/cli.rs` — add `--include` tests, extend builder as needed

**Purpose:** Drive the core feature: scan a skill directory, parse SKILL.md frontmatter, print a dynamic catalog. Includes acceptance tests.

**Expected Outcome:** `cargo test` passes. A test creates a temp dir with a real SKILL.md fixture, passes `--include <dir>`, and asserts the output contains the skill name and description. The hardcoded skill list is replaced with dynamic scanning.

**Details:**

1. Runtime dependency: add `serde_yaml` (parses YAML frontmatter between `---` delimiters per the Agent Skills spec).

2. Production code in `src/main.rs`:
   - Parse `--include` (repeatable) from `std::env::args()` — no clap dependency for now, keep it simple.
   - For each `--include` directory, walk it recursively, find all `SKILL.md` files.
   - For each `SKILL.md`, read it, parse frontmatter delimited by `---`, extract `name` and `description` fields.
   - Collect into a `Vec<Skill>`.
   - Generate the same output format as the current hardcoded version, but dynamically.
   - Remove all hardcoded skill entries.
   - If no `--include` is provided, either print a catalog with zero skills or the fallback message (PRD says `skill-primer` with no args prints the catalog — if no skills found, print empty catalog).

3. Test fixture `tests/fixtures/example-skill/SKILL.md`:
   ```yaml
   ---
   name: example-skill
   description: Use when testing example scenarios.
   ---
   # Example Skill
   Do the thing.
   ```

4. New test in `tests/cli.rs`:
   ```rust
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
   ```

5. Empty directory test (edge case):
   ```rust
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
   ```

**Alternatives considered:**
- **YAML vs. manual parsing**: YAML frontmatter is the Agent Skills spec convention (between `---` delimiters). Using `serde_yaml` for robust parsing. The spec notes lenient handling for malformed YAML (e.g., unquoted colons in values).
- **clap vs. manual arg parsing**: Manual `std::env::args()` parsing (chosen) avoids a heavy dependency for what is currently one flag. If more flags appear (`init`, `--trust-project`, `--version`), switch to `clap` in a later task.
- **Recursive walk vs. immediate children only**: PRD suggests scanning directories — skills may be nested one level deep. A recursive walk (chosen via `walkdir` or manual `fs::read_dir`) is more robust and handles arbitrary nesting. However, using only `std::fs` keeps dependencies minimal. I'll use `walkdir` if the recursion gets complex, otherwise stick to a simple `fs::read_dir` loop.

---

## Dependencies

- Task 2 requires `serde_yaml` runtime dependency.

## Risks / Open Questions

- **Fixture path resolution**: The test binary runs from the workspace root. The `tests/fixtures/` directory needs to be referenced correctly. `assert_fs` avoids this by creating temp dirs, but if we want a checked-in fixture, we need to resolve relative to `CARGO_MANIFEST_DIR`.
- **Hardcoded skills removal**: The current binary prints hardcoded skill entries with absolute paths specific to the user's machine. Task 2 removes these entirely. The tests must cover these scenarios before removal.
- **No `init` command yet**: The `init` subcommand (Task 3+) is out of scope. Task 2 focuses only on default/print output. The `init` command will come in a later plan.

---

## Summary

| Task | What | Touches | Depends On |
|---|---|---|---|
| 2 | `--include` flag + dynamic catalog | Cargo.toml, src/main.rs, tests/cli.rs, tests/fixtures/ | Nothing |
