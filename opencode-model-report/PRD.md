# Product Requirements Document (PRD)

## Product

`opencode-model-report` — CLI report for model usage and model cost visibility.

## Purpose

Provide a fast, deterministic command-line report that shows:

1. which models are actively used and where they are used,
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
- Config-home override for testing and alternate locations
- Model inventory refresh via `opencode models --refresh`
- Cost data fetch from `https://models.dev/api.json`
- Two-section terminal report (`Used`, `Configured`)
- Deterministic sorting and formatting
- Explicit error and exit behavior

## Out of Scope

- Interactive UI/TUI
- Automatic model changes
- Persisting report output to storage by default

## Functional Requirements

### FR-1: Config Inputs

The CLI must load model references from the selected config-home directory.

- Default config-home: `$HOME/.config/opencode`
- Override flag: `--home-dir <path>`

Under the selected config-home directory:

- `opencode.jsonc` is required
- `weave-opencode.jsonc` is optional

The parser must support:

- single-line `//` comments
- trailing commas before `}` and `]`

### FR-2: Active Usage Collection

The CLI must aggregate active model usage from:

- top-level `model`
- top-level `small_model`
- `agent.*.model`
- `agents.*.model`
- `custom_agents.*.model`

Usage labels must map as follows:

- top-level `model` -> `default`
- top-level `small_model` -> `small_model`
- `agent.*.model` -> agent key
- `agents.*.model` -> agent key
- `custom_agents.*.model` -> custom agent key

Usage labels must be:

- preserved even if repeated
- sorted alphabetically before display

Usage labels must render in different colours by config source when colour is enabled.

- OpenCode-derived labels use one colour
- Weave-derived labels use another colour

### FR-3: Available Models Refresh

The CLI must run:

`opencode models --refresh`

and derive model IDs from stdout while ignoring non-model lines and ANSI sequences.

Accepted model lines are exact `provider/model` tokens. Duplicate model IDs from refresh output are ignored after first occurrence.

### FR-4: Cost Data

The CLI must fetch:

`https://models.dev/api.json`

using `curl -fsSL`, map provider/model keys to `input` and `output` costs, and render unknown costs as `n/a`.

Cost values are expressed per 1M tokens.

If either input or output cost is missing, the total cost is treated as unknown for sorting.

### FR-5: Report Sections

The CLI must print:

1. `Used` table with columns: `MODEL`, `IN`, `OUT`, `USAGE`
2. `Configured` table with columns: `MODEL`, `IN`, `OUT`

Section headers must use bold styling only.

### FR-6: Formatting

- Dynamic width alignment for `MODEL`, `IN`, `OUT`
- `USAGE` wrapping at 50 chars, comma-aware
- Continuation-line indentation aligned to the `USAGE` column
- Usage labels joined with `, `

### FR-7: Sorting

- `Used`: usage count descending, tie-breaker 1: total cost ascending, tie-breaker 2: model name ascending
- `Configured`: total cost descending, tie-breaker 1: model name ascending
- Unknown total cost sorts last

### FR-8: Color Behavior

- Section headers use bold styling only
- Usage labels are coloured by config source
- `--no-color` disables colour output
- Colour output is emitted only when stdout is a terminal

### FR-9: Exit and Error Behavior

- Missing required config / parse / general failure: exit code `3`
- `opencode` command missing: error + exit `3`
- `opencode models --refresh` fails:
  - print refresh failure message
  - print subprocess stderr if present
  - exit with subprocess exit code (fallback `4`)
- `curl` missing or fetch failure: error + exit `3`

### FR-10: Alternate Config-Home Override

The product must support execution against an alternate config-home directory for testing and non-default setups.

Required behaviour:

- Support `--home-dir <path>`
- Apply the override before loading `opencode.jsonc` and `weave-opencode.jsonc`
- Keep default auto-discovery behaviour when the flag is omitted

## Non-Functional Requirements

- Stable Rust toolchain
- Minimal dependency set
- Predictable terminal output suitable for automation parsing
- Clear, actionable error messages

## CLI Contract

- `--help`
- `--version`
- `--no-color`
- `--home-dir <path>`

## Acceptance Criteria

1. Report consistently renders `Used` and `Configured` sections with required columns.
2. Sorting matches FR-7 exactly.
3. Exit behavior matches FR-9 exactly.
4. Output formatting (widths/wrapping/indentation) matches FR-6.
5. Alternate config-home support is implemented and verified.

## Rollout Notes

- Keep current home-dir auto-discovery for in-repo usage.
- Keep `weave-opencode.jsonc` optional for backward compatibility.
- Validate with side-by-side sample runs using default and alternate config-home directories.
