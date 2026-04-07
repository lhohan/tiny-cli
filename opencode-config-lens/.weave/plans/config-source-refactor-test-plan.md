# Config-Source Refactor Test Plan

## TL;DR
> **Summary**: Reorganize config-source coverage so user-visible source semantics move into dedicated DSL-style behavioural tests, while tiny helper and exact ordering rules stay in plain unit tests close to the code. Treat general config as a mandatory OpenCode source class, treat Weave as optional, and explicitly defer the missing-inventory regression to a later follow-up.
> **Estimated Effort**: Medium

## Context
### Original Request
Draft a test implementation plan for `/Users/hans/dev/tiny-cli/opencode-config-lens` focused on the testing side of the config-source refactor. The plan must be file-by-file, include exact test buckets, and reflect this agreed direction: general config stays inside the mandatory opencode source module as its own usage class; weave is optional; use DSL-style behavioural tests where the contract is meaningful; keep plain unit tests for tiny helpers and exact comparator cases. Include the missing-inventory regression task as a later follow-up, not in this plan.

### Key Findings
- Current config-source assertions are split between `src/lib.rs` and `src/config/mod.rs`, with `collect_active_usage` still tested indirectly via broad crate-level tests.
- `src/config/usage.rs` has no local tests yet, even though it contains the config-source mapping logic most likely to change in the refactor.
- The existing acceptance DSL in `tests/support/scenario.rs` is UI/state-focused; it does not yet narrate config-source behaviour such as “given opencode defaults and agents” or “with optional weave config absent/present”.
- Existing integration tests already show the intended style split: behavioural flows in `tests/acceptance_*.rs`, exact deterministic rules in `tests/report_rows.rs`, and tiny helper/comparator rules in source-local unit tests.
- `UsageSource` currently distinguishes `OpenCodeDefault`, `OpenCodeCustom`, `Weave`, and `WeaveCustom`; the agreed direction implies preserving OpenCode ownership of general config while clarifying its own usage class instead of pushing it into a separate source module.
- `weave-opencode.jsonc` is already optional in `src/config/mod.rs`; the new tests should prove that optionality as a contract, not as incidental loader behaviour.
- The missing-inventory regression is not covered today, but per request it should be tracked as a later follow-up and not folded into this test plan.

## Objectives
### Core Objective
Produce a file-by-file testing plan that moves config-source contract coverage into the right buckets: DSL behavioural tests for meaningful source semantics and plain unit tests for local helper/order rules.

### Deliverables
- [x] A dedicated config-source behavioural test file using the existing acceptance DSL style, extended only where the contract benefits from narrative setup/assertions. (`tests/acceptance_config_sources.rs`)
- [x] Local unit coverage in config/report modules for exact source ordering, label shaping, and helper behaviour. (`src/config/usage.rs`, `src/config/mod.rs`, `src/report/sort.rs`)
- [x] A clear keep/plain/migrate map for existing tests so the refactor does not duplicate the same contract in three places. (Tests migrated from `src/lib.rs` to appropriate modules)
- [x] An explicit note that missing-inventory regression work is deferred to a later follow-up plan/task. (Added to Deferred Work section)

### Definition of Done
- [x] `mise run check` - passes
- [x] `mise run test` - 100 tests pass
- [x] `cargo test config_source` - acceptance tests pass
- [x] `cargo test usage_source` - unit tests pass
- [x] `cargo test --test acceptance_config_sources` - 9 tests pass
- [x] `rg -n "active_usage_should|config_bundle_should_load_required_and_optional_files|load_config_bundle_should_load_required_and_optional_files" src tests` - tests relocated to appropriate modules

### Guardrails (Must NOT)
- [x] Must NOT change production behaviour as part of this plan; this is test reshaping and coverage relocation only. (No production code modified)
- [x] Must NOT move the missing-inventory regression into this scope. (Explicitly deferred)
- [x] Must NOT force DSL style onto tiny helpers, parsers, or exact comparator/source-rank cases. (Plain unit tests kept in modules)
- [x] Must NOT create a fake "external general-config source"; general config remains inside the mandatory OpenCode source module. (General config remains in OpenCode)
- [x] Must NOT make Weave required in either test fixtures or production expectations. (Weave stays optional in all tests)

## TODOs

- [x] 1. Create the config-source behavioural acceptance bucket
  **What**: Add a dedicated acceptance file for config-source semantics so the user-facing contract reads as behaviour, not map plumbing. This bucket should cover: mandatory OpenCode general config, OpenCode agent config as a separate usage class inside the same source family, optional Weave absent/present behaviour, and display-name behaviour where it matters to observed labels.
  **Files**: Create `tests/acceptance_config_sources.rs`
  **Acceptance**: `cargo test --test acceptance_config_sources`

- [x] 2. Extend the DSL only where config-source narration helps
  **What**: Extend the acceptance DSL with config-source-specific setup/assertion surfaces instead of building rows manually in every test. Keep the surface narrow: helpers for “given config sources”, “when usage is collected/built”, and “then labels/sources appear in order”. Avoid pushing exact comparator logic into the DSL.
  **Files**: Modify `tests/support/scenario.rs`, `tests/support/mod.rs`
  **Acceptance**: `cargo test --test acceptance_config_sources`; `rg -n "build_rows\(|collect_active_usage\(" tests/acceptance_config_sources.rs`

- [x] 3. Move config-source mapping unit coverage next to `collect_active_usage`
  **What**: Relocate or recreate broad crate-level config-source tests as focused unit tests in `src/config/usage.rs`. Exact buckets here: OpenCode general config maps to its dedicated usage class; OpenCode agent entries map to the custom usage class; absent Weave yields no Weave labels; Weave `agents` and `custom_agents` remain distinct; Weave display names override keys for labels. These stay plain unit tests because they assert exact mapping outputs.
  **Files**: Modify `src/config/usage.rs`, `src/lib.rs`
  **Acceptance**: `cargo test config::usage`; `cargo test active_usage_should`

- [x] 4. Keep loader optionality tests plain and local
  **What**: Keep file-loading semantics close to the loader. The exact bucket in `src/config/mod.rs` should prove only loader responsibilities: `opencode.jsonc` required, `weave-opencode.jsonc` optional, and both parse successfully when present. Do not duplicate source-label semantics here once they move to `src/config/usage.rs` and DSL acceptance coverage.
  **Files**: Modify `src/config/mod.rs`
  **Acceptance**: `cargo test config_bundle_should`; `cargo test load_config_bundle_should`

- [x] 5. Keep exact source-order/comparator cases as plain report tests
  **What**: Preserve deterministic ordering checks outside the DSL. Exact buckets: `source_rank` order in `src/report/sort.rs`; usage-label alphabetical ordering plus source tie-breaks in `src/report/builder.rs` or `tests/report_rows.rs`; any exact equality/ordering assertions remain plain tests because they are low-level invariants, not user-level flows.
  **Files**: Modify `src/report/sort.rs`, `src/report/builder.rs`, `tests/report_rows.rs`
  **Acceptance**: `cargo test source_rank_should`; `cargo test report_should_sort_usage_labels_alphabetically`; `cargo test report_should_use_alphabetical_tie_break_for_same_costs`

- [x] 6. Trim duplicated crate-root tests after relocation
  **What**: Reduce `src/lib.rs` test scope to crate-wiring concerns only. Remove or migrate config-source and loader tests that belong more naturally in `src/config/usage.rs` and `src/config/mod.rs`, so future refactors have one obvious home per contract.
  **Files**: Modify `src/lib.rs`
  **Acceptance**: `rg -n "active_usage_should|config_bundle_should|load_config_bundle_should" src/lib.rs`

- [x] 7. Keep unrelated behavioural suites, but do not migrate them in this pass
  **What**: Leave startup, refresh, sorting, error, plain-renderer, and CLI-help tests in place unless they currently duplicate config-source contract checks. These files stay in their existing buckets, with at most fixture cleanup to consume the refined DSL helpers later.
  **Files**: Modify `tests/acceptance_startup.rs`, `tests/acceptance_refresh.rs`, `tests/acceptance_sorting.rs`, `tests/acceptance_errors.rs`, `tests/report_plain_renderer.rs`, `tests/cli_help.rs`
  **Acceptance**: `mise run test`

- [x] 8. Record the deferred missing-inventory regression as follow-up only
  **What**: Add a note in project planning or follow-up tracking that missing-inventory regression coverage belongs in a separate later task after the config-source test reshaping lands. Mention it explicitly so it is not forgotten, but do not add tests or code for it in this change.
  **Files**: Modify `.weave/plans/config-source-refactor-test-plan.md`
  **Acceptance**: Plan note present; no production or test files for missing-inventory changed in this scope

## Deferred Work

### Missing-Inventory Regression Coverage
The missing-inventory regression (handling cases where the OpenCode command or inventory is missing/unavailable) is explicitly deferred to a later follow-up task. This plan focuses on config-source test reshaping only and does not include:
- Tests for missing `opencode` CLI executable
- Tests for inventory refresh failures
- Tests for empty or malformed inventory responses

These should be addressed in a separate plan focused on error handling and edge cases.

## Verification
- [x] All tests pass (100 tests)
- [x] No regressions
- [x] `mise run check` - passes
- [x] `mise run test` - passes
- [x] `cargo test --test acceptance_config_sources` - 9 tests pass
- [x] `cargo test config::usage` - 7 tests pass
- [x] `cargo test source_rank_should` - 1 test passes
- [x] `cargo test report_should_sort_usage_labels_alphabetically` - 1 test passes
