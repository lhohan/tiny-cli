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
| Skill dedup | First skill name wins by discovery order |
| Duplicate warning | `warning: duplicate skill 'foo' at /path/to/SKILL.md, keeping first` |
| XML ownership | XML escaping and XML assembly live in `lib.rs` |
| Warning transport | Library returns structured warnings; `main.rs` prints them to `stderr` |
| `show-config` status words | `exists`, `missing`, `error` |
| `list-skills` format | `[{name-column}] /path/to/SKILL.md` with width 24 and character-safe truncation |

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
  - `show-config`: show `missing`, do not fail
  - `prime`: warn and skip, do not fail
  - `list-skills`: warn and skip, do not fail
- Unreadable or stat-error directory:
  - `show-config`: show `error`
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

### Discovery and precedence

- Scan paths in resolved order.
- Preserve discovery order for kept skills.
- Deduplicate by skill name, first match wins.
- `list-skills` and `prime` must reflect the same kept set in the same order.

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
- Introduce library return types that can carry both command data and warnings.
- Keep `src/main.rs` thin:
  - parse CLI args
  - dispatch subcommands
  - print stdout payloads
  - render warnings to `stderr`
  - exit non-zero on hard include errors

### Phase 2: Add `show-config`

Goal: add the first human-facing inspection command with include-only behavior first.

- Add `show-config` to the clap subcommand enum.
- Implement output for explicit include paths only.
- Render status lines with `exists`, `missing`, and `error`.
- Extend the CLI test DSL with:
  - working-directory control
  - line-based stdout assertions
  - stderr assertions for exact warning text

### Phase 3: Add default path resolution

Goal: teach `show-config` the full default search model.

- Implement repo-root detection with `jj` then `git`.
- Implement upward walk from CWD to repo root or home directory.
- Add static home-directory candidates.
- Add path deduplication with first occurrence preserved.
- Verify search-path ordering and status rendering through integration tests.

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

### `show-config`

- explicit include paths are shown in order
- include override suppresses defaults
- default project paths are discovered from CWD and parent directories
- repo-root stopping works for both Git and Jujutsu repos
- home paths appear after project paths
- nonexistent paths render as `missing`
- permission or stat failures render as `error`
- duplicate paths are collapsed with first occurrence preserved

### `list-skills`

- discovered skills are printed in discovery order
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

## Files

### Create

- `src/lib.rs`: shared discovery, path resolution, XML generation, warning collection

### Modify

- `src/main.rs`: CLI parsing, explicit subcommand dispatch, stdout rendering, `stderr` warning rendering
- `Cargo.toml`: add library target if needed
- `tests/common.rs`: add working-directory and line-based assertion helpers
- command-specific integration tests for `show-config`, `list-skills`, and updated `prime`
- `README.md`: explicit subcommand examples
- `AGENTS.md`: explicit subcommand examples

## Assumptions

- No new dependency is required unless home-directory resolution needs a small crate for cross-platform correctness.
- Discovery order is the public precedence model and should not be obscured by sorting.
- `show-config` is intentionally narrow: it reports skill search configuration for the current invocation, not general application configuration.
