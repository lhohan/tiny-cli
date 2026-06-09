# Plan: Skill Discovery Commands and Default Path Resolution

## Summary

Refactor `skills-primer` into a thin CLI over a shared library, then add two human-facing inspection commands alongside `prime`:

- `prime`: emit the full agent-facing instructions and XML catalog
- `show-config`: show the ordered skill search paths for the current invocation with status
- `list-skills`: show the discovered skills in precedence order in a compact human-readable format

All three commands use the same discovery pipeline. If any `--include` paths are provided, they override all default discovery paths for every command.

## Locked Decisions

| Decision | Value |
|----------|-------|
| CLI shape | Explicit subcommands only: `prime`, `show-config`, `list-skills` |
| `--include` | Overrides all defaults for every command |
| Repo root detection | `jj root` first, `git rev-parse --show-toplevel` fallback |
| No repo found | Walk upward from CWD to home directory, then stop |
| Candidate project dirs | `.agents/skills`, `.claude/skills` at each walked level |
| Candidate home dirs | `~/.agents/skills`, `~/.claude/skills`, `~/.codex/skills` |
| Symlinks | Follow |
| Path dedup | Deduplicate paths before scanning, keep first occurrence |
| Directory traversal | Sort directory entries lexicographically before recursion |
| Skill dedup | First skill name wins by discovery order |
| Duplicate warning | `warning: duplicate skill 'foo' at /path/to/SKILL.md, keeping first` |
| XML ownership | XML escaping and XML assembly live in `lib.rs` |
| Warning transport | Library returns structured warnings; `main.rs` prints them to `stderr` |
| `show-config` path states | `exists`, `missing`, `error` |
| `show-config` path-state warnings | No `stderr` warnings for `missing` or `error` rows; stdout status is the contract |
| `list-skills` format | `[{name-column}] /path/to/SKILL.md` with width 24 and character-safe truncation |
| Root-detection testing | Use a small root-detection helper trait for unit tests; integration tests cover real Git repos and skip JJ-specific tests when `jj` is unavailable |

## Command Contracts

### `prime`

- Requires explicit subcommand use.
- Emits the existing instruction block plus `<available_skills>` XML.
- Uses the shared path-resolution and discovery pipeline.
- Deduplicates skills by name, first match wins.
- Emits warnings to `stderr` using locked exact strings.

### `show-config`

- Prints the full ordered search set for the current invocation, including nonexistent paths.
- When `--include` is present, shows only included paths.
- Output is one line per candidate path:

```text
exists  /path
missing /path
error   /path
```

- `exists`: path exists and is a directory
- `missing`: path does not exist
- `error`: path status could not be determined cleanly, such as permission or filesystem error
- For path-state problems, stdout status is sufficient. `show-config` does not also emit `stderr` warnings for `missing` or `error` rows.

### `list-skills`

- Uses the same resolved search paths and same precedence rules as `prime`.
- Prints discovered skills in discovery order after first-win deduplication.
- Output is one line per skill:

```text
[use-jujutsu              ] /Users/hans/.agents/skills/use-jujutsu/SKILL.md
[very-long-skill-name-t...] /Users/hans/.agents/skills/very-long-skill-name-that-keeps-going/SKILL.md
```

- Name column width is 24 characters inside brackets.
- If the name is longer than 24 characters, print the first 21 characters plus `...`.
- If the name is shorter than 24 characters, right-pad with spaces so paths align.
- Truncation is character-based, not byte-based.

## Interfaces

The library interface is part of the contract. `main.rs` should call explicit library entry points rather than reconstructing behavior itself.

```rust
pub enum CliError {
    EmptyIncludePath,
    IncludePathIsFile { path: PathBuf },
}

pub enum PathStatus {
    Exists,
    Missing,
    Error,
}

pub struct SearchPathRow {
    pub path: PathBuf,
    pub status: PathStatus,
}

pub struct DiscoveredSkill {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
}

pub enum DiscoveryWarning {
    DuplicateSkill { name: String, losing_path: PathBuf },
    InvalidFrontmatter { path: PathBuf },
    UnreadableSkillFile { path: PathBuf },
    InvalidSkillName { name: String, path: PathBuf, reason: String },
    MissingIncludeDirectory { path: PathBuf },
    UnreadableSearchDirectory { path: PathBuf },
}

pub struct DiscoveryResult<T> {
    pub value: T,
    pub warnings: Vec<DiscoveryWarning>,
}

pub fn resolve_search_paths(
    include_dirs: &[PathBuf],
    cwd: &Path,
) -> Result<DiscoveryResult<Vec<SearchPathRow>>, CliError>;

pub fn discover_skills(
    search_paths: &[SearchPathRow],
) -> DiscoveryResult<Vec<DiscoveredSkill>>;

pub fn generate_prime_output(
    skills: &[DiscoveredSkill],
) -> String;
```

Notes:

- Exact type names may vary slightly during implementation, but the boundary must stay equivalent to this shape.
- `main.rs` is responsible for:
  - calling `std::env::current_dir()`
  - mapping `CliError` to exact error text and exit codes
  - printing `show-config` rows
  - printing `list-skills` rows
  - printing `prime` output
  - rendering `DiscoveryWarning` variants as exact `stderr` strings
- `lib.rs` is responsible for:
  - path resolution
  - directory traversal
  - scan ordering
  - skill parsing and validation
  - duplicate detection
  - XML escaping and XML assembly

## Error and Warning Contract

The library returns structured warnings with command data. `main.rs` renders exact `stderr` strings.

### Hard errors

- `error: include path cannot be empty`
- `error: include path '/path/to/file' is a file, not a directory`

These exit non-zero for every command.

### Warnings

- `warning: duplicate skill 'foo' at /path/to/SKILL.md, keeping first`
- `warning: SKILL.md has invalid or missing frontmatter: /path/to/SKILL.md`
- `warning: unable to read SKILL.md: /path/to/SKILL.md`
- `warning: skill 'foo' has invalid name: <reason> (/path/to/SKILL.md)`
- `warning: include directory not found: /path/to/dir`
- `warning: unable to access search directory: /path/to/dir`

### Missing and unreadable include directories

- Empty include path: error, exit non-zero
- Include path is a file: error, exit non-zero
- Missing include directory:
  - `show-config`: show `missing`, do not fail, no stderr warning
  - `prime`: warn and skip, do not fail
  - `list-skills`: warn and skip, do not fail
- Unreadable or stat-error directory:
  - `show-config`: show `error`, do not fail, no stderr warning
  - `prime`: warn and skip, do not fail
  - `list-skills`: warn and skip, do not fail

## Shared Discovery Model

### Path resolution

- If any `--include` paths are provided, use only those paths.
- Otherwise:
  - detect repo root with `jj root`
  - if that fails, try `git rev-parse --show-toplevel`
  - if no repo root is found, walk upward from CWD to home directory
- At each walked level, consider:
  - `.agents/skills`
  - `.claude/skills`
- After project-relative paths, append:
  - `~/.agents/skills`
  - `~/.claude/skills`
  - `~/.codex/skills`
- Follow symlinks.
- Deduplicate the ordered candidate path list before scanning, keeping the first occurrence.
- Use canonicalized path identity where available so the same directory is not scanned twice under different spellings.

### Directory traversal and scan order

- Scan paths in resolved order.
- Within each scanned directory tree, sort `read_dir` entries lexicographically by full path string before recursing.
- If a directory contains `SKILL.md`, treat that directory as a skill directory and do not recurse further beneath it.
- Preserve accepted-skill order exactly as discovered after path ordering and sorted traversal.
- Deduplicate by skill name, first match wins.
- `list-skills` and `prime` must reflect the same kept set in the same order.

### Root detection testability

- Extract repo-root detection behind a small helper trait or helper function boundary so command execution results can be unit-tested without spawning `jj` or `git`.
- Integration tests should:
  - create real temporary Git repos for Git-root behavior
  - run JJ-specific tests only when `jj` is available in the environment
- If `jj` is unavailable, JJ-specific tests should skip cleanly rather than fail.

## Implementation Phases

### Phase 1: Refactor to `lib.rs`

Goal: move discovery logic out of `main.rs` and establish a library-owned output pipeline.

- Create `src/lib.rs`
- Move scan types and helpers into the library:
  - `Skill`
  - `SkillFrontmatter`
  - `ScanResult`
  - `ScanWarning`
  - `scan_skill_directory`
  - `parse_skill_frontmatter`
  - `validate_skill_name`
- Move XML escaping into the library or a library-owned helper module.
- Introduce the library interface from the `Interfaces` section.
- Keep `src/main.rs` thin:
  - parse CLI args
  - dispatch subcommands
  - call library entry points
  - print stdout payloads
  - render warnings to `stderr`
  - exit non-zero on hard include errors

### Phase 2: Add `show-config`

Goal: add the first human-facing inspection command with include-only behavior first.

- Add `show-config` to the clap subcommand enum.
- Implement output for explicit include paths only.
- Render status lines with `exists`, `missing`, and `error`.
- Do not emit path-state stderr warnings for `show-config`.
- Extend the CLI test DSL with:
  - working-directory control
  - line-based stdout assertions
  - precise stderr assertions

### Phase 3: Add default path resolution

Goal: teach `show-config` the full default search model.

- Implement repo-root detection with `jj` then `git`.
- Implement upward walk from CWD to repo root or home directory.
- Add static home-directory candidates.
- Add path deduplication with first occurrence preserved.
- Add deterministic directory ordering.
- Verify search-path ordering and status rendering through integration tests and root-detection unit tests.

### Phase 4: Add `list-skills`

Goal: expose discovered skills without the `prime` XML wrapper.

- Reuse the resolved path list and scan logic.
- Apply first-win deduplication by skill name.
- Render aligned `[name] path` output with width 24 and character-safe truncation.
- Preserve discovery order in output.

### Phase 5: Rewire `prime`

Goal: make `prime` use the shared pipeline without changing its instruction block.

- Keep the existing human instructions.
- Generate XML in the library.
- Reuse the same path-resolution, scan, dedup, and warning pipeline as `list-skills`.
- Update duplicate warnings to reference the losing skill file path.

### Phase 6: Update docs and command expectations

- Update `README.md` examples to use explicit subcommands.
- Update `AGENTS.md` examples to use explicit subcommands.
- Remove assumptions in tests that `--include` without a subcommand runs `prime`.

## Tests

### Refactor safety

- Existing `prime` and scan tests continue to pass after the `lib.rs` split, except tests intentionally changed for explicit subcommands.

### CLI acceptance

- bare invocation prints help
- help output lists `prime`, `show-config`, and `list-skills`
- `skills-primer --include /tmp/foo` no longer runs `prime`
- explicit `prime`, `show-config`, and `list-skills` subcommands all parse correctly

### `show-config`

- explicit include paths are shown in order
- include override suppresses defaults
- default project paths are discovered from CWD and parent directories
- repo-root stopping works for both Git and Jujutsu repos
- home paths appear after project paths
- nonexistent paths render as `missing`
- permission or stat failures render as `error`
- duplicate paths are collapsed with first occurrence preserved
- path-state rows do not also emit `stderr` warnings

### `list-skills`

- discovered skills are printed in discovery order
- directory traversal order is deterministic under sorted recursion
- duplicate skill names keep the first result only
- name column is padded to width 24
- long names are truncated character-safely with `...`
- printed paths start at the same column across rows
- include override works the same way as `prime` and `show-config`

### `prime`

- default path discovery finds project and home skills
- include override uses only included paths
- duplicate-skill warning references the losing skill file path
- XML output still escapes content correctly
- warnings are emitted with exact locked text

### Root detection

- unit tests cover `jj` success, `jj` failure with `git` success, and both commands failing
- Git integration tests use a real temporary Git repo
- JJ integration tests skip cleanly when `jj` is unavailable

## Files

### Create

- `src/lib.rs`: shared discovery, path resolution, XML generation, warning collection

### Modify

- `src/main.rs`: CLI parsing, explicit subcommand dispatch, stdout rendering, `stderr` warning rendering
- `Cargo.toml`: add library target if needed
- `tests/common.rs`: add working-directory and line-based assertion helpers
- command-specific integration tests for `show-config`, `list-skills`, and updated `prime`
- root-detection unit tests
- `README.md`: explicit subcommand examples
- `AGENTS.md`: explicit subcommand examples

## Assumptions

- No new dependency is required unless home-directory resolution needs a small crate for cross-platform correctness.
- Discovery order is the public precedence model and should not be obscured by sorting beyond the locked deterministic directory-entry sort.
- `show-config` is intentionally narrow: it reports skill search configuration for the current invocation, not general application configuration.
