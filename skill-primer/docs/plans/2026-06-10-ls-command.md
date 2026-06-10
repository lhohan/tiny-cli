# Plan: `ls` command + Default Path Resolution

## Goal

Add the `ls` subcommand (list available skills) and implement default skill-path resolution (repo root detection, upward walk, home directories) shared across `prime` and `ls`. `show-config` is simplified to display resolved paths rather than scanned paths.

## Key Decisions

### Command Dispatch

No subcommand + `--include` now prints an error message to stderr plus help output to stdout and exits with a non-zero exit code, instead of silently defaulting to `prime`. All subcommands (`ls`, `prime`, `show-config`) must be explicit.

### Global `--cwd` Flag

A new global `--cwd <path>` flag sets the effective working directory for path resolution. If omitted, the real process current directory is used. Tilde (`~`) is expanded in both `--cwd` and `--include` arguments in the CLI layer before passing to the library. The CLI layer canonicalizes `--cwd` to an absolute path before passing it to the library. `--cwd` affects only the project walk starting directory; home directory candidates always use the real effective home directory (via `dirs::home_dir()` or `HOME`), independent of `--cwd`.

### `show-config` Semantics

`show-config` displays **resolved** paths, not scanned paths. Each path is either a home directory candidate pattern or a resolved walk directory from the effective CWD, and is annotated with `(found)` or `(not found)` based on filesystem presence.

- **Without `--include`:** Shows the three hardcoded home directory patterns (`~/.agents/skills`, `~/.claude/skills`, `~/.codex/skills`) and the resolved project walk directories from the effective CWD. Each path is annotated with `(found)` or `(not found)`.
- **With `--include`:** Shows only the include paths (annotated with `(found)` or `(not found)`), plus a note: *"Default paths are overridden by --include."*

### Path Deduplication

- `--include` paths: preserve exact order and repetition as specified by the user.
- Default paths (project walk + home dirs): deduplicate canonically identical paths before scanning. The first occurrence in discovery order wins. Canonicalization uses `std::fs::canonicalize` when the path exists; for non-existent paths, deduplication falls back to string comparison of the expanded absolute path.

### Skill Name Deduplication

Across all resolved paths (both `--include` and default), duplicate skill names use first-wins. A warning is emitted to stderr for each duplicate.

### Repo Detection

No `RepoDetector` trait. A simple function `detect_repo_root(cwd: &Path) -> Option<PathBuf>` calls `jj root` first, then falls back to `git rev-parse --show-toplevel`. All repo-detection tests are integration tests that create real temporary Git/JJ repositories.

## Deviations from Original PLAN

| Decision | Original plan | This plan |
|----------|--------------|-----------|
| Subcommand name | `list-skills` | `ls` |
| No subcommand + `--include` | Default to `prime` | Error + help |
| `show-config` | Shows scan results with `exists`/`missing`/`error` | Shows resolved paths with `(found)`/`(not found)` |
| `--cwd` | N/A | Global flag for effective working directory |
| Repo detection | `RepoDetector` trait with unit doubles | Simple function, integration tests only |
| Implementation order | Phase 3 → Phase 4 → Phase 5 | Outside-in TDD: `ls` stub → `ls` + `--include` → default paths → rewire `prime`/`show-config` |

## Assumptions

- `--include` overrides all default paths for every command.
- Tilde (`~`) is expanded in the CLI layer before path values reach the library.
- Home directory candidates: `~/.agents/skills`, `~/.claude/skills`, `~/.codex/skills`.
- Project walk directories: `.agents/skills`, `.claude/skills`, `.codex/skills` at each level from CWD upward.
- Walk stops at repo root (detected via `jj root` or `git rev-parse --show-toplevel`). If outside any repo, walks up to `HOME`.
- Skill name dedup: first wins by discovery order.
- `HOME` env var unset: home directory candidates are skipped silently.
- `ls` no-skills output: `"No skills found."` to stdout, exit 0.
- Library error contract: all `Err` returns from library functions result in exit code 1. All `Ok` returns result in exit code 0.

## Implementation Phases (TDD Order)

### Phase 0: `ls` stub ✅ (complete)

Add `Ls` variant to `Command` enum, dispatch in `main`, return hardcoded output. Proves CLI wiring.

Added `Ls` to `Command` enum, `LsOutput` / `generate_ls_output` stub in `lib.rs`, `handle_ls` in `main.rs`. Breaking change: `--include` without subcommand now errors (was: defaulted to `prime`). Fixed all existing tests to use `.command_prime()`.

**Tests (integration):**
- [x] `ls` subcommand recognized, returns hardcoded output, exit 0.
- [x] `help` output lists `ls` alongside `prime` and `show-config`.
- [x] No subcommand + `--include <dir>` → stderr error message, help to stdout, non-zero exit.

### Phase 1: `format_skill_name` (unit)

Pure function: takes a skill name, returns a 24-character string (padded or truncated with `...`).

**Tests (unit, in `lib.rs`):**
- Short name padded to 24 chars with spaces.
- Exact 24-char name preserved.
- Name > 24 chars: first 21 chars + `...`.
- Multi-byte unicode handled char-by-char, not byte-by-byte.

### Phase 2: `ls` with explicit `--include` (integration)

Wire `generate_ls_output` with explicit `--include`. Reuses existing `scan_skill_directory`. Formats with `format_skill_name`.

**Tests (integration):**
- Single skill discovered and listed with correct format.
- Empty include dir → `"No skills found."`.
- Multiple includes preserve discovery order.
- Short names padded to align paths.
- Names > 24 chars truncated with `...`.
- Exact 24-char name not truncated.
- Duplicate skill names: first kept, stderr warning emitted.
- Nested `SKILL.md` inside skill subdirectory ignored.
- Bad frontmatter → stderr warning, skill excluded.
- Invalid skill name → stderr warning, skill still listed.
- Unreadable subdirectory → stderr warning, sibling skills still found.
- Include path is a file → error, non-zero exit.
- Include path is empty → error, non-zero exit.

### Phase 3: `detect_repo_root` (integration)

Simple function calling `jj root` then `git rev-parse --show-toplevel`. Tested via CLI integration tests in real temp repositories.

**Tests (integration):**
- Real Git repo detected from subdirectory.
- Outside any repo → `None`.
- JJ preferred over Git: creates a repo with `jj git init`, verifies the resolved root matches `jj root` output (skip test if no `jj` on PATH).
- JJ fails, Git fallback: `jj root` unavailable or fails, falls back to `git rev-parse --show-toplevel` (skip if no `git`).
- Trailing whitespace in command output handled.

### Phase 4: `ls` with default paths (integration)

Implement `resolve_skill_paths` and drive it through `ls` integration tests against real temp directories.

**Tests (integration):**
- Walks upward from CWD to repo root, collecting `.agents/skills`, `.claude/skills`, `.codex/skills` at each level.
- Stops at repo root, does not walk above it.
- Outside any repo, walks to `HOME`.
- Home dir candidates appended after project paths.
- `--include` override suppresses default paths entirely.
- Canonically identical paths deduplicated (uses `canonicalize` for existing paths, falls back to string comparison for non-existent paths).
- Duplicate skill names across project walk and home dirs: first wins.

### Phase 5: Rewire `prime` and `show-config`

All three commands share `resolve_skill_paths`. `prime` and `ls` scan resolved paths for skills. `show-config` displays the resolved paths without scanning for skill content.

Refactor `generate_prime_output` to call `resolve_skill_paths` and scan the resolved paths, replacing its current direct `include_dirs` scanning loop. `generate_show_config_output` likewise uses `resolve_skill_paths` to determine which paths to display.

**Tests (integration):**
- `prime` without `--include` in non-repo dir → `"No skills found."` or `<available_skills>`.
- `prime` with `--include` unchanged (existing tests pass).
- `prime` without `--include` in a Git repo containing `.agents/skills/foo/SKILL.md` → discovers `foo` via the upward walk (verify `<available_skills>` contains it). Run from a repo subdirectory (not the root) to prove the walk works.
- `show-config` without `--include` in non-repo dir → shows home directory patterns with `(not found)`.
- `show-config` with `--include` → shows include paths only, with `(found)`/`(not found)`, plus override note.
- `show-config` in Git repo with default paths → shows home dirs + resolved project walk dirs.

## Output Format (`ls`)

```
[name-column-24-chars] /absolute/path/to/SKILL.md
```

- Name column: 24 characters inside brackets.
- Short names: right-padded with spaces.
- Long names: first 21 chars + `...`.
- Character-based truncation, not byte-based.
- No skills found: single line `No skills found.` to stdout.

## Output Format (`show-config`)

**Without `--include`:**
```
Configured directories:
  ~/.agents/skills (found)
  ~/.claude/skills (not found)
  ~/.codex/skills (not found)

Project directories:
  /home/user/project/.agents/skills (found)
  /home/user/project/.claude/skills (not found)
```

**With `--include`:**
```
Include paths:
  /path/to/my-skills (found)
  /path/to/other-skills (not found)

Default paths are overridden by --include.
```

## Library Contract

```rust
pub struct LsOutput {
    pub lines: Vec<String>,
    pub stderr: Vec<String>,
}

pub fn generate_ls_output(
    include_dirs: &[PathBuf],
    cwd: &Path,
) -> Result<LsOutput, Vec<String>>;

pub fn generate_prime_output(
    include_dirs: &[PathBuf],
    cwd: &Path,
) -> Result<PrimeResponse, Vec<String>>;

pub fn generate_show_config_output(
    include_dirs: &[PathBuf],
    cwd: &Path,
) -> ShowConfigOutput;

pub struct ShowConfigOutput {
    pub lines: Vec<String>,
}

/// Resolve the effective working directory.
/// Expands `~` to the home directory.
fn resolve_cwd(cwd: &Path) -> PathBuf;

/// Detect repository root from a starting directory.
/// Tries `jj root`, then `git rev-parse --show-toplevel`.
fn detect_repo_root(cwd: &Path) -> Option<PathBuf>;

/// Resolve all candidate skill paths.
/// - When `include_dirs` is non-empty: returns them verbatim in order
///   (no deduplication). Default path resolution is skipped entirely.
/// - When `include_dirs` is empty: walks from CWD to repo root (or HOME),
///   collecting `.agents/skills`, `.claude/skills`, `.codex/skills` at each
///   level, then appends home directory candidates. Default paths are
///   deduplicated by canonical path (with string-comparison fallback for
///   non-existent paths); the first occurrence in discovery order wins.
pub fn resolve_skill_paths(
    include_dirs: &[PathBuf],
    cwd: &Path,
) -> Vec<PathBuf>;
```

## Files

| File | Action |
|------|--------|
| `src/lib.rs` | Add `format_skill_name`, `detect_repo_root`, `resolve_skill_paths`, `generate_ls_output`, `generate_show_config_output`; refactor `generate_prime_output` to call `resolve_skill_paths` instead of scanning `include_dirs` directly; update `generate_prime_output` to take `cwd` |
| `src/main.rs` | Add `Ls` variant to `Command` enum; add global `--cwd` flag; update dispatch logic (no subcommand + `--include` → error); add `handle_ls`, `handle_prime`, `handle_show_config` |
| `tests/common.rs` | Add `command_ls()`, `command_prime()`, `with_cwd_dir()`, `expect_skills_output_detected()` |
| `tests/list_skills.rs` | New: `ls` integration tests |
| `tests/cli_basics.rs` | Add `ls` help test; update `prime` test to use `expect_skills_output_detected`; add no-subcommand + `--include` error test |
| `tests/show_config.rs` | Rewrite all: default paths, include paths, in-repo project paths, override note |
| `README.md` | Replace `list-skills` with `ls` in examples |

## Validation

```sh
mise verify        # clippy -D warnings, cargo fmt --check, nextest tests
skills-primer ls --include ~/.agents/skills   # manual smoke test
skills-primer ls --cwd /tmp/foo               # manual smoke test with cwd
skills-primer ls                              # in a repo: discovers project + home skills
skills-primer help                            # lists ls, prime, show-config
skills-primer show-config                     # lists configured paths
skills-primer show-config --include /tmp/foo  # lists include path + override note
```

## Total Tests

| Phase | Count | Type |
|-------|-------|------|
| 0 | 3 | integration |
| 1 | 4 | unit |
| 2 | 13 | integration |
| 3 | 5 | integration |
| 4 | 7 | integration |
| 5 | 6 | integration |
| **Total** | **38** | |
