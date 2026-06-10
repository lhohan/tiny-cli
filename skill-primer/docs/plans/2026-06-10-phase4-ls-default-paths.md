# Plan: Phase 4 — `ls` with Default Paths

## Goal

Implement `resolve_skill_paths` and wire it into `generate_ls_output` so `skills-primer ls` (without `--include`) discovers skills by walking upward from CWD through project directories and home directory candidates, using `detect_repo_root` as the walk stop boundary.

## Assumptions

- **Filter before `collect_skills`**: Default-path non-existent directories are silently skipped by filtering `resolve_skill_paths` results to only `is_dir()` before passing to `collect_skills`. Explicit `--include` paths flow unchanged (non-existent dirs still warn).
- **Walk stop when HOME unset and no repo**: The walk reaches the filesystem root. Harmless — checking `/` for `.agents/skills` won't find anything in practice.
- **CWD canonicalization**: The walk canonicalizes CWD at the start. All candidate paths are absolute, so the `canonical_key` fallback for non-existent paths produces a correct absolute string without extra work.
- **All 5 Phase 3 unit tests become redundant** after Phase 4 integration tests pass and are deleted. `trailing_whitespace_stripped` is already deleted (tested stdlib `str::trim()`).
- **Phase 5 `--cwd` wiring**: The global `--cwd` flag is wired in Phase 5, not Phase 4. Phase 4 uses the process CWD or `current_dir()` on the test command.
- **`show-config` remains stub**: Phase 4 does not rewire `generate_show_config_response`. That's Phase 5's scope. The function signature already takes `cwd` but ignores it — left as-is.

## Implementation (TDD Order)

### Step 1: Implement `resolve_skill_paths` and `canonical_key`

Add two functions in `src/lib.rs` after `detect_repo_root`:

- `canonical_key(path: &Path) -> PathBuf` — private. If path exists, returns `fs::canonicalize` result (falls back to absolute string on error). If not, returns normalized absolute string.
- `resolve_skill_paths(include_dirs: &[PathBuf], cwd: &Path) -> Vec<PathBuf>` — public.
  - If `include_dirs` is non-empty: returns them verbatim, in order, no dedup.
  - If empty:
    1. Determine `stop_at = detect_repo_root(cwd)` or `HOME` or `None`.
    2. Canonicalize `cwd`. Loop: for each `[.agents/skills, .claude/skills, .codex/skills]` at current level, insert if canonical key not in `seen` set.
    3. If `current` canonical-equals `stop_at`, break (after checking dirs at stop level).
    4. Move to parent; break if no parent (filesystem root).
    5. Append HOME candidates (if `HOME` is set), dedup by canonical key.

### Step 2: Wire `generate_ls_output`

Modify `generate_ls_output` in `src/lib.rs`:

```rust
let resolved = resolve_skill_paths(include_dirs, cwd);
let scan_dirs: Vec<PathBuf> = if include_dirs.is_empty() {
    resolved.into_iter().filter(|p| p.is_dir()).collect()
} else {
    resolved
};
let (all_skills, stderr) = collect_skills(&scan_dirs)?;
```

The file-path pre-check on `include_dirs` stays (only runs for explicit `--include`).

### Step 3: Extend test DSL

In `tests/common.rs`:

- Add `cwd: Option<PathBuf>` field to `CmdSetup` (init to `None` in `Cmd::given()`).
- Add `pub fn with_cwd_dir(mut self, path: &Path) -> Self`.
- Wire in `when_run()`: `if let Some(ref cwd) = self.cwd { cmd.current_dir(cwd); }`.

### Step 4: TDD — 8 integration tests in `tests/list_skills.rs`

Tests are listed in TDD order. Each must fail before its driving code is written, then pass.

#### Test 1: `walks_upward_to_repo_root_git` (Driving)

**Drives**: `resolve_skill_paths` walk loop, repo root stop, `canonical_key` dedup — the minimal viable default-path resolution.

`git init` in temp dir, create skill at repo root `.agents/skills/foo/SKILL.md`, run `ls` from a subdirectory. Assert stdout contains `[foo`, exit 0, stderr empty.

#### Test 2: `outside_repo_walks_to_home` (Driving)

**Drives**: HOME as fallback `stop_at` when `detect_repo_root` returns `None`.

No repo. Set `HOME` to a temp dir containing `.agents/skills/home-skill/SKILL.md`. CWD is a subdirectory of HOME. Assert skill is found.

#### Test 3: `home_dir_candidates_appended_after_project_paths` (Driving)

**Drives**: HOME candidates appended *after* the walk loop, not just stopping at HOME.

Git repo with skill `project-skill` at repo root. Separate HOME temp dir with skill `home-skill`. Run from repo subdir. Assert `project-skill` appears before `home-skill` in stdout.

Without the post-walk HOME append, `project-skill` is found via walk loop but `home-skill` is missed (walk stops at repo root, never reaches HOME). This test forces the HOME candidate append after the walk.

#### Test 4: `stops_at_repo_root_does_not_walk_above` (Confirmation)

**Confirms**: Walk stops at repo root boundary (test 1 already proves this, but this adds the negative assertion).

`git init` in `tmp/repo/`. Skill at `tmp/repo/.agents/skills/inside/` AND `tmp/.agents/skills/above/`. Run from `tmp/repo/subdir/`. Assert `inside` found, `above` not found.

#### Test 5: `deduplicates_canonically_identical_paths` (Confirmation)

**Confirms**: `canonical_key` handles symlinks for dedup (already implemented for test 1).

Git repo. `symlink .claude/skills → .agents/skills`. Skill `only-once` in `.agents/skills/`. Assert `only-once` appears exactly once in stdout.

#### Test 6: `duplicate_skill_names_across_walk_and_home_first_wins` (Confirmation)

**Confirms**: Existing `collect_skills` dedup handles cross-source duplicate names.

Git repo + HOME both have `conflict-name` skill. Project discovered first (walk loop before HOME append). Assert stderr warning, stdout contains name once, project description not home description.

#### Test 7: `walks_upward_to_repo_root_jj` (Confirmation)

**Confirms**: `detect_repo_root` tries `jj root` first (no new code required; regression for Phase 3).

`jj git init`, skill at repo root, run from subdir. Skip if `jj` not on PATH. Assert skill found.

#### Test 8: `jj_fails_falls_back_to_git` (Confirmation)

**Confirms**: `detect_repo_root` falls back to git when `jj root` fails (regression for Phase 3 `jj_fails_git_fallback`).

Pure `git init` repo (not a jj workspace). When `jj` is on PATH, `jj root` fails here. Assert skill found via git fallback.

### Step 5: Delete Phase 3 unit tests

All 5 Phase 3 unit tests in `src/lib.rs` `#[cfg(test)]` block are now redundant:

| Phase 3 test | Covered by |
|---|---|
| `detects_git_repo_from_subdirectory` | Tests 1, 4 |
| `outside_any_repo_returns_none` | Test 2 |
| `jj_preferred_over_git` | Test 7 |
| `jj_fails_git_fallback` | Test 8 |
| `trailing_whitespace_stripped` | Already deleted |

## Likely Files

| File | Action |
|------|--------|
| `src/lib.rs` | Add `canonical_key` (private), `resolve_skill_paths` (pub); modify `generate_ls_output`; delete Phase 3 unit tests |
| `tests/common.rs` | Add `cwd` field, `with_cwd_dir()` method, wire in `when_run()` |
| `tests/list_skills.rs` | Add 8 integration tests in TDD order |

## Risks

- **`canonicalize` on macOS temp dirs**: `assert_fs::TempDir` places dirs in `/var/folders/...` which may have symlinks. `canonicalize` resolves correctly, but stop-at comparison must also canonicalize the stop point.
- **Symlink loop**: `canonicalize` returns an error — `canonical_key` fallback handles it.
- **`jj git init` availability**: Tests 7 and 8 skip gracefully when `jj` is not on PATH.
- **Ordering assertion fragility**: Test 3 asserts relative ordering of project vs home skills. If path resolution order changes, this test needs updating. Ordering is specified: project walk paths before home candidates.

## Validation

```sh
cargo test                           # 29 existing + 8 new, then 24 existing + 8 new after Phase 3 deletion
cargo clippy -- -D warnings
cargo fmt --check
mise verify
skills-primer ls                     # in skills-primer repo: discovers project + home skills
skills-primer ls --include ~/.agents/skills  # explicit include still works
```
