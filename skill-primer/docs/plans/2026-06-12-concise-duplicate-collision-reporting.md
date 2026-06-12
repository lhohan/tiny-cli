# Concise Duplicate Collision Reporting

## Goal

Replace verbose one-line-per-duplicate warnings with a concise grouped `[Skill conflicts]` collision report on stderr that shows both the kept and skipped paths, their sources, and the auto-resolution choice.

## Assumptions

1. HOME is always the final walk target — the only two sources are "project" (any directory below HOME) and "home" (the HOME directory itself).
2. Collisions only occur between different walk levels (project vs home, or two project sub-levels), never within a single directory.
3. The `--warnings`/`--no-warnings` gating continues to apply to this output since it goes to stderr.
4. The first-encountered-wins resolution strategy remains unchanged; "auto" always reflects automatic first-match precedence (no explicit user precedence exists yet).
5. `~` path shortening only applies to paths under the HOME directory; all other paths display in full.

## Plan

### Step 1: Augment collision tracking in `collect_skills`

Replace the immediate `stderr.push(...)` on duplicate detection with a collision accumulator. For each duplicate:

- Record the skill name, the kept path (already inserted into `seen_names`), and the skipped path.
- Also record whether the kept path is under HOME (needed for `(project)` / `~` annotation).

Pass the HOME path into `collect_skills` (or `collect_all_skills`) so collision formatting can determine `~` eligibility and source annotation.

### Step 2: Emit grouped collision report

After all skill directories are processed, if collisions exist:

1. Emit `[Skill conflicts]` header line.
2. For each collision, emit two lines:
   - `{name} collision: ✓ auto [(project)] {kept_path_display}`
   - `                    ✗ {skipped_path_display} (skipped)` (indented to align with `✓` from line above)

Where:

- `(project)` appears only when the kept path is not under HOME.
- `kept_path_display` / `skipped_path_display` replace the HOME prefix with `~`.
- The indentation on the `✗` line aligns the `✗` under the `✓` above.

### Step 3: Wire HOME path through

`collect_all_skills` calls `find_candidate_skill_paths` (which already determines HOME internally). Return HOME from `find_candidate_skill_paths` or determine it in `collect_all_skills` and thread it into `collect_skills`.

### Step 4: Update tests

- **`tests/prime.rs`**: Update `prime_should_deduplicate_skills_by_name_from_walked_paths` to expect the new `[Skill conflicts]` format instead of `warning: duplicate skill '...' at ..., keeping first`.
- **`tests/ls.rs`**: Update `ls_should_keep_first_duplicate_skill_name` and `ls_should_keep_first_duplicate_across_directory_levels` similarly.
- Add tests for:
  - `~` path display when HOME is the skipped source.
  - `(project)` annotation on kept paths below HOME.
  - No `(project)` annotation when the kept path is from HOME.
  - Collision section only appears when warnings are shown.
  - Multiple collisions grouped under a single `[Skill conflicts]` header.

## Likely files

| File | Change |
|---|---|
| `src/lib.rs` | Modify `collect_skills` to accumulate collisions; add formatting function; thread HOME path through `collect_all_skills`/`find_candidate_skill_paths` |
| `tests/prime.rs` | Update duplicate assertions; add collision format tests |
| `tests/ls.rs` | Update duplicate assertions; add collision format tests |

## Risks

- **Risk**: The tilde substitution needs to handle edge cases where HOME is `/` (unlikely but theoretically possible). *Mitigation*: Only substitute when the path starts with HOME + `/`.
- **Risk**: Alignment of the `✗` line with the `✓` depends on constant-width indentation. The `✓ auto [(project)] ` prefix is fixed-width, so this is predictable. *Mitigation*: Compute the prefix length at format time rather than hardcoding it, in case the annotation text changes later.
- **Low risk**: The "auto" label has no semantic counterpart today (there's no manual override). It's purely informational and harmless.

## Validation

After implementation:

1. Run `mise verify` — confirms clippy, formatting, and all tests pass.
2. Manually test with overlapping skills in project and home to visually inspect the stderr output.
3. Confirm `--no-warnings` suppresses the collision section.
4. Confirm `--warnings` on `prime` shows the collision section.
