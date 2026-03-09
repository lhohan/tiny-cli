# skill-sync

Config-driven local skill sync tool for tiny-cli.

## Overview

Syncs selected local skills into `.agents/skills` based on configured source roots. The workflow is repo-local, repeatable, and includes discovery, machine-readable output, and no-op preview modes.

## Configuration Files

### `sync-skills.conf`

Source root configuration file. Each line specifies a directory to search for skills:

- One source root per line
- Lines starting with `#` are comments
- Blank lines are ignored
- `~` is expanded to the user's home directory
- Missing, unreadable, or non-directory source roots produce a warning and are skipped

Example:
```
# Local skills directory
~/.config/opencode/skills
~/.agents/skills
```

### `.agents/skills.selected`

The tracked selection file specifying which skills to sync:

- One skill name per line (matching directory name)
- Lines starting with `#` are comments
- Blank lines are ignored
- Skills not found in any source root print a warning but do not cause failure
- Skills found in multiple source roots cause a fatal error

Example:
```
# Essential skills
debugging
testing
writing
```

## Usage

```bash
./skill-sync/sync-skills.sh [OPTIONS]
```

### Options

- `--sync` – Sync mode (default when no mode flag provided)
- `--list-all` – List all discovered skills across configured roots
- `--json` – Output as JSON (only valid with `--list-all`)
- `--dry-run` – Show planned actions without making changes
- `--help` – Show help message

### Examples

Sync selected skills:
```bash
./skill-sync/sync-skills.sh
# or explicitly
./skill-sync/sync-skills.sh --sync
```

List all discovered skills:
```bash
./skill-sync/sync-skills.sh --list-all
```

List skills as JSON:
```bash
./skill-sync/sync-skills.sh --list-all --json
```

Preview sync actions:
```bash
./skill-sync/sync-skills.sh --dry-run
```

## Sync Behavior

- Reads all configured source roots
- Discovers skill directories by immediate child directory name
- Treats a directory as a valid skill only if it contains `SKILL.md`
- For each selected skill:
  - If found in no source root, prints a warning and continues
  - If found in more than one source root, fails with non-zero exit
  - If found uniquely, stages it for copy into `.agents/skills/<name>`
- Replaces each resolved destination skill atomically by copying into a temporary sibling directory and renaming into place only after a successful copy
- Removes any temporary or backup directory after a successful replacement
- Strict mirror pruning removes destination skill directories that are not named in `.agents/skills.selected`
- Selected skills that are missing from all source roots are left untouched in `.agents/skills`; they warn but are not deleted

## Warning Behavior

- **Missing selected skill**: Warning on stderr, overall success unless another fatal error occurs
- **Missing/unreadable/non-directory source root**: Warning on stderr, skipped
- **Duplicate skill name across configured roots**: Fatal error on stderr, exit 4
- **Invalid selected skill directory missing SKILL.md**: Fatal error, non-zero exit

## Exit Codes

- `0` – Success, including warning-only runs
- `2` – Usage or validation error
- `3` – Configuration or environment prerequisite error
- `4` – Runtime failure after a valid invocation, including duplicate-skill conflicts

## Testing

Run the test suite:

```bash
./skill-sync/test-sync-skills.sh
```

Tests cover:
1. `--list-all` prints expected human-readable rows with source and status
2. `--list-all` prints all matching source roots for conflict rows
3. `--list-all --json` returns valid JSON describing discovered skills
4. Missing selected skill prints a warning and exits with code 0
5. Missing selected skill does not delete an existing copy in `.agents/skills`
6. Missing, unreadable, or non-directory source root warns and is skipped
7. Duplicate skill found in multiple source roots exits with code 4
8. Invalid selected skill directory without SKILL.md exits with code 4
9. Invalid flag combinations fail with code 2
10. `--dry-run` reports intended sync and prune actions without mutating `.agents/skills`
11. Sync copies valid selected skills into `.agents/skills` atomically
12. Strict mirror pruning removes previously synced skills that are no longer named in `.agents/skills.selected`
