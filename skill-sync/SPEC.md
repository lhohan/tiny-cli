# Config-Driven Local Skill Sync for `tiny-cli`

## Summary

Build a small Bash-based tool in a dedicated `skill-sync/` directory that syncs selected local skills into this repo under `.agents/skills`, using configured source roots from a file next to the script. The workflow is repo-local, repeatable, and includes discovery (`--list-all`), machine output (`--json`), and no-op preview (`--dry-run`).

## Key Changes

- Add `skill-sync/sync-skills.sh` as the only user-facing command.
- Add `skill-sync/sync-skills.conf` as a simple line-based config file:
  - one source root per line,
  - `#` comments and blank lines ignored,
  - `~` expanded to the user home directory,
  - missing, unreadable, or non-directory source roots produce a warning and are skipped.
- Keep `.agents/skills.selected` as the tracked selection file:
  - one skill name per line,
  - comments and blank lines ignored.
- Implement sync behavior in Bash:
  - default mode is sync when no mode flag is provided,
  - read all configured source roots,
  - discover skill directories by immediate child directory name,
  - treat a directory as a valid skill only if it contains `SKILL.md`,
  - for each selected skill:
    - if found in no source root, print a warning and continue,
    - if found in more than one source root, fail with non-zero exit,
    - if found uniquely, stage it for copy into `.agents/skills/<name>`,
  - replace each resolved destination skill atomically by copying into a temporary sibling directory and renaming into place only after a successful copy and validation,
  - remove any temporary or backup directory after a successful replacement,
  - strict mirror pruning removes destination skill directories that are not named in `.agents/skills.selected`,
  - selected skills that are missing from all source roots are left untouched in `.agents/skills`; they warn but are not deleted.
- Implement listing behavior:
  - `--list-all` enumerates all discovered skills across configured roots,
  - default output is human-readable and includes `name`, `source` or `sources`, and `status`,
  - `status` is at least `selected`, `unselected`, or `conflict`,
  - unique rows expose a single `source`,
  - conflict rows expose `sources` as the full list of matching source roots,
  - human-readable conflict rows print all matching source roots,
  - `--list-all --json` emits the same underlying data as structured JSON,
  - internal implementation should build one canonical dataset first, then render human or JSON output from it.
- Implement dry-run behavior:
  - `--dry-run` with sync prints planned `copy`, `update`, `delete`, `warn`, and `conflict` actions without changing files,
  - `--dry-run` with `--list-all` is allowed and simply avoids any side effects.
- Update docs:
  - document the two config files,
  - document command examples,
  - document warning-only behavior for missing selected skills,
  - document warning-and-skip behavior for invalid source roots.

## CLI Contract

- Command:
  - `skill-sync/sync-skills.sh`
- Supported flags:
  - `--sync`
  - `--list-all`
  - `--json`
  - `--dry-run`
  - `--help`
- Flag rules:
  - no mode flag means `--sync`,
  - `--json` is only valid with `--list-all`,
  - `--sync` and `--list-all` are mutually exclusive,
  - invalid combinations exit non-zero with usage text.
- Exit codes:
  - `0`: success, including warning-only runs,
  - `2`: usage or validation error,
  - `3`: configuration or environment prerequisite error,
  - `4`: runtime failure after a valid invocation, including duplicate-skill conflicts.
- Output behavior:
  - missing selected skill: warning on stderr, overall success unless another fatal error occurs,
  - missing, unreadable, or non-directory source root: warning on stderr, skipped,
  - duplicate skill name across configured roots: fatal error on stderr, exit `4`,
  - invalid selected skill directory missing `SKILL.md`: fatal error, non-zero exit,
  - human-readable list mode should be stable and easy to scan in a terminal,
  - JSON output should be valid without extra log noise on stdout.

## Test Plan

Add a focused shell test suite for the script.

Cover:

1. `--list-all` prints expected human-readable rows with source and status.
2. `--list-all` prints all matching source roots for conflict rows.
3. `--list-all --json` returns valid JSON describing the same discovered skills, with `source` for unique rows and `sources` for conflict rows.
4. Missing selected skill prints a warning and exits with code `0`.
5. Missing selected skill does not delete an existing copy in `.agents/skills`.
6. Missing, unreadable, or non-directory source root warns and is skipped.
7. Duplicate skill found in multiple source roots exits with code `4`.
8. Invalid selected skill directory without `SKILL.md` exits with code `4`.
9. Invalid flag combinations fail with code `2`.
10. `--dry-run` reports intended sync and prune actions without mutating `.agents/skills`.
11. Sync copies valid selected skills into `.agents/skills` and replaces existing versions atomically.
12. Strict mirror pruning removes previously synced skills that are no longer named in `.agents/skills.selected`.

Use temporary directories in tests so source roots and destination can be isolated and deterministic.

## Assumptions

- Implementation language is Bash for v1.
- Rust is deferred unless the tool grows beyond shell-script complexity.
- Destination remains fixed at `.agents/skills`.
- Install mode is copy, not symlink.
