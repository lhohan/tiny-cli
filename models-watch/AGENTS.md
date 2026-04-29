# AGENTS.md

models-watch is a small Bash CLI that detects changes in the `opencode-go` models published by `models.dev` and records deltas under `state/`.

## Project Overview

- Runtime entry point: `models-watch.sh`
- Runtime state: `state/latest.json`, `state/change-<timestamp>.json`
- Acceptance tests: `tests-rust/`
- Human-facing behaviour and usage: `README.md`

## Verification Commands

Run these before claiming work is done:

```bash
bash -n models-watch.sh
mise run test
```

Fallback if `mise` is unavailable:

```bash
cargo test --manifest-path tests-rust/Cargo.toml
```

## Helpful Tools (CLI + MCP)

- `jq` — inspect fixture, snapshot, and delta JSON
- `rg` — search the shell script and Rust test DSL quickly
- `mise run test` — canonical acceptance-test entry point
- No repo-specific MCP servers are documented here

## Feature Workflow

1. Read `README.md` and `models-watch.sh` before changing behaviour.
2. Prefer black-box changes: update or add acceptance tests in `tests-rust/tests/acceptance.rs` when behaviour changes.
3. Keep `state/` relative to the script directory; do not introduce caller-cwd assumptions.
4. Preserve the CLI contract: `--notify-file <path>` for testable notifications, unknown flags exit `2`, missing `opencode-go` block exits `3`.
5. Re-run verification commands after every non-trivial change.
6. Update `README.md` when flags, state files, or user-visible behaviour change.

## Gotchas Codex

- Do not make tests depend on the live network. Use `MODELS_WATCH_API_URL=file://...` fixtures. Added: 2026-04-29
- Do not trigger macOS popups in automated runs. Use `--notify-file` or set `MODELS_WATCH_NO_OSASCRIPT=1`. Added: 2026-04-29
- The script writes state relative to its own location, not the caller's cwd. Tests copy the script into a temp tool dir to keep state isolated. Added: 2026-04-29

## Detailed Guidelines

- Usage, behaviour, and state format: `README.md`
- Test harness and fluent DSL: `tests-rust/src/lib.rs`
- Acceptance scenarios: `tests-rust/tests/acceptance.rs`
