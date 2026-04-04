# Product Requirements Document (PRD)

## Product

`opencode-model-report` — CLI report for model usage and model cost visibility.

## Purpose

Provide a fast, deterministic command-line report that shows:

1. which models are actively configured and where they are used,
2. which additional models are available but currently unused,
3. model input/output cost metadata when available.

## Target Users

- Maintainers of OpenCode/Weave configuration
- Users comparing model cost and usage concentration
- Contributors auditing model selection in agent configurations

## User Outcomes

- Identify heavily used models quickly
- Spot unused but available alternatives
- Make model selection decisions with cost context

## In Scope

- Configuration parsing from JSONC
- Model inventory refresh via `opencode models --refresh`
- Cost data fetch from `https://models.dev/api.json`
- Two-section terminal report (`ACTIVE`, `ALLOWED`)
- Deterministic sorting and formatting
- Explicit error and exit behavior

## Out of Scope

- Interactive UI/TUI
- Automatic model changes
- Persisting report output to storage by default

## Functional Requirements

### FR-1: Config Inputs

The CLI must load model references from:

- `packages/opencode/.config/opencode/opencode.jsonc`
- `packages/opencode/.config/opencode/weave-opencode.jsonc`

The parser must support:

- single-line `//` comments,
- trailing commas before `}` and `]`.

### FR-2: Active Usage Collection

The CLI must aggregate active model usage from:

- top-level `model`
- top-level `small_model`
- `agent.*.model`
- `agents.*.model`
- `custom_agents.*.model`

Duplicate usage labels for the same model must not be repeated.

### FR-3: Available Models Refresh

The CLI must run:

`opencode models --refresh`

and derive model IDs from stdout while ignoring non-model lines and ANSI sequences.

### FR-4: Cost Data

The CLI must fetch:

`https://models.dev/api.json`

using `curl -fsSL`, map provider/model keys to `input` and `output` costs, and render unknown costs as `n/a`.

### FR-5: Report Sections

The CLI must print:

1. `ACTIVE` table with columns: `MODEL`, `IN`, `OUT`, `USAGE`
2. `ALLOWED` table with columns: `MODEL`, `IN`, `OUT`

### FR-6: Formatting

- Dynamic width alignment for `MODEL`, `IN`, `OUT`
- `USAGE` wrapping at 50 chars, comma-aware
- Continuation-line indentation aligned to `USAGE` column

### FR-7: Sorting

- `ACTIVE`: usage count descending (most-used first)
  - tie-breaker 1: total cost ascending
  - tie-breaker 2: model name ascending
- `ALLOWED`: total cost ascending
- Unknown total cost sorts last

### FR-8: Color Behavior

- Color is only for human-facing section headers
- `--no-color` disables color output
- Data fields remain plain text values

### FR-9: Exit and Error Behavior

- Missing config / parse / general failure: exit code `3`
- `opencode` command missing: error + exit `3`
- `opencode models --refresh` fails:
  - print refresh failure message
  - print subprocess stderr if present
  - exit with subprocess exit code (fallback `4`)
- `curl` missing or fetch failure: error + exit `3`

### FR-10: Standalone Execution Requirement (from dot-eize)

The product must support execution outside repository context via explicit configuration inputs.

Required behavior:

- Support explicit config-path CLI options
  - `--opencode-config <path>`
  - `--weave-config <path>`
- Support optional env-var fallbacks for those config paths
- Define deterministic precedence:
  1. CLI flags
  2. env vars
  3. auto-discovery

## Non-Functional Requirements

- Stable Rust toolchain
- Minimal dependency set
- Predictable terminal output suitable for automation parsing
- Clear, actionable error messages

## CLI Contract

- `--help`
- `--version`
- `--no-color`

## Acceptance Criteria

1. Report consistently renders `ACTIVE` and `ALLOWED` sections with required columns.
2. Sorting matches FR-7 exactly.
3. Exit behavior matches FR-9 exactly.
4. Output formatting (widths/wrapping/indentation) matches FR-6.
5. FR-10 portability requirement is implemented and verified.

## Rollout Notes

- Keep current auto-discovery behavior for in-repo usage.
- Add FR-10 with backward-compatible defaults so existing usage does not break.
- Validate with side-by-side sample runs (in-repo and out-of-repo invocation).
