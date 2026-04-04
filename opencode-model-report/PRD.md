# Product Requirements Document (PRD)

## Product

`opencode-model-report` — fullscreen ratatui TUI for model usage and cost visibility.

## Purpose

Provide a fast, deterministic terminal UI that shows which models are active, which are only configured, and what they cost.

## Target Users

- Maintainers of OpenCode/Weave configuration
- Users comparing model cost and usage concentration
- Contributors auditing model selection in agent configurations

## User Outcomes

- Identify active models quickly
- Compare configured models in one view
- Switch sort modes without leaving the TUI
- Refresh data manually
- Use alternate config-home locations for testing
- Get a polished, attractive default presentation without extra configuration

## In Scope

- Configuration parsing from JSONC
- Config-home override for testing and alternate locations
- Model inventory refresh via `opencode models --refresh`
- Cost data fetch from `https://models.dev/api.json`
- Fullscreen ratatui TUI
- Unified model list with active state
- Manual refresh and sort switching
- Deterministic sorting, wrapping, and error handling

## Out of Scope

- Automatic refresh loops
- Automatic model changes
- Persistent storage of report output

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

### FR-5: Unified Model List

The TUI must display one list of models, not separate sections.

Columns:

1. `MODEL`
2. `ACTIVE`
3. `IN`
4. `OUT`
5. `USAGE`

`ACTIVE` must render as a boolean value (`yes` / `no`).

Rows come from the refreshed available model inventory.

- `ACTIVE=yes` for models referenced in config
- `ACTIVE=no` for other available models

`USAGE` is empty for inactive rows.

### FR-6: Layout and Interaction

- The TUI must be fullscreen and redraw on resize.
- The TUI must show a loading state while the initial refresh and cost fetch complete.
- The TUI must support manual refresh only; it must not auto-refresh.
- The TUI must not require a selection cursor in v1.
- The TUI must provide keyboard controls only.

Required keys:

- `q` quit
- `r` refresh
- `s` cycle sort mode

### FR-7: Formatting

- Dynamic width alignment for `MODEL`, `ACTIVE`, `IN`, `OUT`
- `USAGE` wrapping at 50 chars, comma-aware
- Continuation-line indentation aligned to the `USAGE` column
- Usage labels joined with `, `
- Usage labels remain in alphabetical order

### FR-8: Sorting

Default sort mode: `active-first`.

Supported sort modes in v1:

1. `active-first`
2. `cost-asc`
3. `cost-desc`
4. `model-name`

Sort mode behaviour:

- `active-first`: active rows first, then total cost ascending, then model name ascending
- `cost-asc`: total cost ascending, then model name ascending
- `cost-desc`: total cost descending, then model name ascending
- `model-name`: model name ascending

Unknown total cost sorts last in cost-based modes.

### FR-9: Color Behavior

- Table headers use bold styling only
- Usage labels are coloured by config source
- `--no-color` disables colour output
- Colour output is emitted only when stdout is a terminal

### FR-10: Error Behavior

- Missing required config / parse / general failure: exit code `3`
- `opencode` command missing: error + exit `3`
- Initial `opencode models --refresh` failure: print refresh failure message, print subprocess stderr if present, exit with subprocess exit code (fallback `4`)
- `curl` missing or fetch failure during initial load: error + exit `3`
- Refresh failure after launch: keep the current snapshot and show the error in the TUI status area

### FR-11: Future Reordering Support

The data model must preserve stable row identities so future manual reordering or additional sort/view modes can be added without rewriting the model layer.

## Non-Functional Requirements

- Stable Rust toolchain
- Minimal dependency set
- Predictable terminal rendering suitable for human use
- Clear, actionable error messages
- Stable, deterministic row ordering for every supported sort mode
- Polished default visuals with sensible spacing, readable colours, and good out-of-the-box presentation
- Clear separation between data reading and UI rendering modules

## Architecture Notes

- Keep data loading, refresh, and model assembly in separate reader modules.
- Keep terminal rendering and keyboard handling in separate UI modules.
- The UI should consume a prepared model list and not reach into config or network code directly.
- The data layer should not depend on terminal rendering details.

## CLI Contract

- `--help`
- `--version`
- `--no-color`
- `--home-dir <path>`

## Acceptance Criteria

1. The TUI opens fullscreen and shows one unified model list.
2. `q`, `r`, and `s` behave as specified.
3. All v1 sort modes work and switch in the TUI.
4. Formatting matches the alignment and wrapping requirements.
5. `--home-dir` works with alternate config-home locations.
6. Startup and refresh error behaviour matches FR-10.

## Rollout Notes

- Keep current home-dir auto-discovery for default use.
- Keep `weave-opencode.jsonc` optional for backward compatibility.
- Validate with sample runs using both default and alternate config-home directories.
