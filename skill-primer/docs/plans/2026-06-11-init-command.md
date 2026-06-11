# Plan: `init` subcommand

## Goal

Add the `init` subcommand, the last major PRD feature. `init` injects (or updates) a small static runner block into `AGENTS.md` that tells coding agents to run `skills-primer prime`. This makes the tool self-bootstrapping: an agent reads AGENTS.md, sees the block, runs `skills-primer prime` to get the full skill catalog at runtime.

The block is a static constant — no skill scanning, no discovery — keeping `init` fast, idempotent, and decoupled from the discovery pipeline. The runner instruction uses `skills-primer prime` without `--include` flags, relying on the tool's built-in default path resolution (shared across all subcommands since Phase 5 of the ls-command plan).

## Key Decisions

| Decision | Value |
|----------|-------|
| Marker format | `<!-- skill-primer:start -->` / `<!-- skill-primer:end -->` (PRD style) |
| Old marker handling | No backward compatibility — old `<!-- skills-primer BEGIN-->` / `<!-- skills-primer END-->` markers will be treated as absent, causing block to be appended |
| Marker behavior | Replace content between existing markers; append full block if markers absent |
| Block content | Static constant — runner instructions only, no skill catalog |
| Block command | `skills-primer prime` (no `--include` — uses default path resolution) |
| Target file | `AGENTS.md` in effective CWD |
| Stdout output | `"AGENTS.md updated with skill-primer block."` |
| Exit code | 0 on success, 1 on I/O error (unwritable file, etc.) |
| `--include` interaction | `init` does not accept `--include` — no scanning involved |
| `--cwd` interaction | Not yet implemented; `init` uses the real CWD. When `--cwd` is added later, `init` will respect it via the same dispatch path as other commands |
| Idempotency | Running `init` twice produces identical AGENTS.md |

## Runner Block Content

The injected block (between markers):

```markdown
## Agent skills integration

Run `skills-primer prime` to integrate agent skills in this project. Follow those instructions before proceeding with any task.

IMPORTANT: report available skills NOW in format:

```
*Available skills:* skill-a, skill-b, etc.
```
```

Full block with markers:

```markdown
<!-- skill-primer:start -->
## Agent skills integration

Run `skills-primer prime` to integrate agent skills in this project. Follow those instructions before proceeding with any task.

IMPORTANT: report available skills NOW in format:

```
*Available skills:* skill-a, skill-b, etc.
```
<!-- skill-primer:end -->
```

## Implementation Logic (library)

```
run_init(cwd):
  1. Compute path <- cwd / "AGENTS.md"
  2. Try to read existing content (empty string if file absent)
  3. If markers both present (start before end):
       Keep content before MARKER_START line (inclusive)
       Keep content from MARKER_END line onward (inclusive)
       Replace everything between with BLOCK_CONTENT
     Else:
       Append full marker-delimited block at end
  4. Write back to AGENTS.md
  5. Return InitOutput { message: "AGENTS.md updated with skill-primer block." }
```

Marker detection uses string find (`str::find`), not regex. Only exact matches on their own lines count (trimmed comparison). The start marker must appear before the end marker.

## Library Contract

```rust
pub struct InitOutput {
    pub message: String,
}

pub fn run_init(cwd: &Path) -> Result<InitOutput, Vec<String>>;
```

- `Ok(InitOutput)` — AGENTS.md updated or created successfully.
- `Err(Vec<String>)` — I/O error (unwritable file, permission denied, etc.).

## Implementation Phases (TDD Order)

### Phase 1: Library — block constant and marker logic

Add to `src/lib.rs`:
- `const BLOCK_CONTENT: &str` — the runner instructions (without markers)
- `const MARKER_START: &str = "<!-- skill-primer:start -->"`
- `const MARKER_END: &str = "<!-- skill-primer:end -->"`
- `pub struct InitOutput { pub message: String }`
- `pub fn run_init(cwd: &Path) -> Result<InitOutput, Vec<String>>`
- Private helper: `fn apply_block(existing: &str) -> String` — replace-between-markers or append

**Note on typo fix**: The existing `AGENTS.md` in the repo contains a typo ("for to" instead of "to"). The `BLOCK_CONTENT` constant should use the corrected version: "Run `skills-primer prime` to integrate agent skills"

**Unit tests (in `lib.rs` `#[cfg(test)]`):**
- [ ] Existing content with markers -> replaces content between, preserves surrounding text
- [ ] Existing content without markers -> appends block at end
- [ ] Empty file -> appends block
- [ ] Only start marker (no end) -> appends block (malformed)
- [ ] Only end marker (no start) -> appends block (malformed)
- [ ] Markers in wrong order (end before start) -> appends block
- [ ] Idempotent: applying twice produces same output as once
- [ ] Preserves trailing newline of original
- [ ] Preserves content after end marker
- [ ] CRLF line endings -> still matches (trimmed comparisons)
- [ ] Markers with leading/trailing whitespace on line -> still matches (trimmed comparisons)
- [ ] Markers as only content in file -> replace correctly

### Phase 2: CLI wiring

Modify `src/main.rs`:
- Add `Init` variant to `Command` enum
- Add `handle_init(cwd: &Path)` function — calls `run_init`, prints message or errors
- Wire dispatch: `(Some(Command::Init), _)` -> `handle_init`
- `help` output must list `init` alongside `prime`, `ls`, `config`

### Phase 3: Integration tests

Modify `tests/init.rs` (new file) using the existing DSL:

- [ ] **Init creates AGENTS.md when none exists** — `Cmd::given().command_init().when_run()` -> success, file created with correct content
- [ ] **Init replaces content between markers** — pre-write AGENTS.md with markers and old content -> init -> content between markers replaced, surrounding text preserved
- [ ] **Init appends block when no markers** — pre-write AGENTS.md without markers -> init -> markers + block appended
- [ ] **Init is idempotent** — run init twice -> AGENTS.md unchanged after second run, stdout message repeated
- [ ] **Init fails on unwritable AGENTS.md** — make AGENTS.md read-only -> init -> error, non-zero exit

Extend `tests/common.rs`:
- Add `command_init()` method to `CmdSetup`
- Add `home_path()` accessor to `CmdResult` for reading files from the home fixture

**Test DSL pattern:**

```rust
// No AGENTS.md exists
Cmd::given()
    .command_init()
    .when_run()
    .should_succeed()
    .expect_output("AGENTS.md updated with skill-primer block.");
// Then read AGENTS.md from home/CWD to verify content
```

### Phase 4: Self-update and docs

- Run `init` on the repo to update its own `AGENTS.md` — replaces any existing block with the new marker format
- Update `README.md` — replace `cargo run` example with `skills-primer prime`, list all subcommands including `init`
- Update `AGENTS.md` block to use `skills-primer prime` instead of `skills-primer --include ~/.agents/skills prime` and fix the typo ("for to" -> "to")

## Files

| File | Action |
|------|--------|
| `src/lib.rs` | Add `BLOCK_CONTENT`, `MARKER_START`, `MARKER_END`, `InitOutput`, `run_init`, `apply_block`, unit tests |
| `src/main.rs` | Add `Init` variant, `handle_init`, dispatch |
| `tests/common.rs` | Add `command_init()`, `home_path()` accessor |
| `tests/init.rs` | New: integration tests |
| `tests/help.rs` | Update `expect_help_printed` to include `init` |
| `README.md` | Update examples to reflect current subcommands |
| `AGENTS.md` | Replace old markers/block with new format (Phase 4 dogfooding) |

## Risks

- **Marker substring in body**: If `BLOCK_CONTENT` happens to contain `<!-- skill-primer:start -->` or `<!-- skill-primer:end -->` as literal text, replacement logic would break. Mitigation: the block is a static constant under project control. This risk is theoretical.
- **Concurrent writes**: No file locking. Two agents running `init` simultaneously could produce interleaved output. Mitigation: this is a CLI tool run manually or once per repo setup — concurrent use is unlikely.
- **AGENTS.md in non-UTF-8 encoding**: `std::fs::read_to_string` and `write` use UTF-8. If AGENTS.md is in another encoding, init will fail. Mitigation: AGENTS.md files are always UTF-8 in practice.
- **Old marker format**: Repositories with the old `<!-- skills-primer BEGIN-->` / `<!-- skills-primer END-->` markers will have the block appended rather than replaced. Mitigation: this is intentional per project decision — no backward compatibility required.

## Validation

```sh
mise verify                              # clippy -D warnings, fmt --check, nextest
cargo run -- init                        # manual smoke: creates AGENTS.md in CWD
cargo run -- init                        # second run: idempotent, file unchanged
cargo run -- help                        # lists all four subcommands
cargo run -- prime                       # verify existing commands still work
```

## Total Tests

| Phase | Count | Type |
|-------|-------|------|
| 1 | 12 | unit |
| 3 | 5 | integration |
| **Total** | **17** | |
