# Config Ingestion Module Refactor

## TL;DR
> **Summary**: Refactor config ingestion around explicit source modules so mandatory OpenCode ingestion owns both general config and agent config, optional Weave ingestion stays isolated, and report/runtime code consumes report-owned presentation data through an adapter instead of importing config semantics directly.
> **Estimated Effort**: Medium

## Context
### Original Request
Draft a module-refactor implementation plan for `/Users/hans/dev/tiny-cli/opencode-config-lens` that depends on the test-plan contract. The plan should reorganize config ingestion so OpenCode is the mandatory source module, Weave is optional, and the general config stays inside the OpenCode module as a distinct usage class. The plan must be file-by-file, reference the new modular boundaries, and explain what to modify in report/runtime code so config code no longer leaks report-specific enums. Exclude the missing-inventory regression bug except as an out-of-scope note.

### Key Findings
- `src/config/usage.rs` currently imports `crate::report::{UsageLabel, UsageSource}`, so config ingestion depends on report presentation types.
- `src/report/model.rs` owns `UsageSource`, but its variants encode config-source semantics (`OpenCodeDefault`, `OpenCodeCustom`, `Weave`, `WeaveCustom`) rather than a pure report concern.
- `src/runtime.rs` hard-codes legend labels and colours by matching directly on `crate::UsageSource`, so runtime also knows config-source categories instead of consuming a report-owned legend/presentation contract.
- The existing test contract already lives in `.weave/plans/config-source-refactor-test-plan.md`; this refactor should follow that contract rather than redefining source semantics in a second place.
- `src/config/mod.rs` already treats `opencode.jsonc` as required and `weave-opencode.jsonc` as optional; the refactor is about boundaries and ownership, not changing file-loading behaviour.
- Crate-root tests in `src/lib.rs` still carry config-source behaviour that belongs closer to config/report modules after the split.
- Missing-inventory regression work is explicitly out of scope for this plan.

## Objectives
### Core Objective
Create clear module boundaries where config ingestion produces config-owned source records, report code translates those records into report-owned labels/presentation, and runtime consumes only report-facing display metadata.

### Deliverables
- [ ] A file-by-file refactor plan that introduces mandatory OpenCode ingestion and optional Weave ingestion as separate config modules.
- [ ] A boundary plan showing how general config remains inside the OpenCode module as its own usage class instead of becoming a fake standalone source.
- [ ] An adapter plan for report/runtime so config stops importing `report::UsageSource` and other report-specific enums.
- [ ] Sequencing notes that explicitly depend on the config-source test contract landing first or evolving in lockstep.

### Definition of Done
- [ ] `mise run check`
- [ ] `mise run test`
- [ ] `cargo test config::usage`
- [ ] `cargo test source_rank_should`
- [ ] `cargo test --test acceptance_config_sources`
- [ ] `rg -n "crate::report::\{UsageLabel, UsageSource\}|UsageSource::OpenCode|UsageSource::Weave" src`

### Guardrails (Must NOT)
- [ ] Must NOT change the source contract agreed in `.weave/plans/config-source-refactor-test-plan.md`.
- [ ] Must NOT invent a separate top-level “general config” source outside OpenCode.
- [ ] Must NOT make Weave mandatory in loader logic, adapters, or tests.
- [ ] Must NOT let config modules depend on report/runtime-specific enums after the refactor.
- [ ] Must NOT fold missing-inventory regression work into this scope.

## TODOs

- [x] 1. Lock the source contract before moving modules
  **What**: Treat `.weave/plans/config-source-refactor-test-plan.md` as authoritative and do not change the agreed source contract in this refactor. Preserve the existing behavioural/unit buckets while changing ownership boundaries only.
  **Files**: Modify `.weave/plans/config-source-refactor-test-plan.md`, `tests/acceptance_config_sources.rs`, `tests/support/scenario.rs`, `tests/support/mod.rs`, `src/config/mod.rs`, `src/config/usage.rs`, `src/report/sort.rs`, `tests/report_rows.rs`, `src/lib.rs`
  **Acceptance**: `cargo test --test acceptance_config_sources`; `cargo test config::usage`; `cargo test source_rank_should`

- [x] 2. Split config ingestion into explicit modules and config-owned usage records
  **What**: Reshape `config/*` around explicit boundaries: a loader that only reads/parses files, an OpenCode ingestion module that always emits usages for `model`, `small_model`, and `agent`, a Weave ingestion module that emits usages only when the optional file exists, and a config-owned usage model that describes source family and usage class without importing report types. Keep OpenCode general config distinct as its own usage class inside the OpenCode family.
  **Files**: Modify `src/config/mod.rs`, `src/lib.rs`; create `src/config/types.rs`, `src/config/loader.rs`, `src/config/opencode.rs`, `src/config/weave.rs`, `src/config/usage.rs`
  **Acceptance**: `cargo test config::`; `cargo test config::usage`; `rg -n "crate::report::\{UsageLabel, UsageSource\}" src/config`

- [x] 3. Add a dedicated report adapter for presentation enums and sorting inputs
  **What**: Insert an explicit conversion boundary between config ingestion and report assembly. Report code should accept config-owned usage records and map them into report-owned labels, styles, and order keys inside a dedicated report adapter module, not inside config. This is the only place where config source classes are translated into report-facing presentation categories.
  **Files**: Modify `src/report/mod.rs`, `src/report/model.rs`, `src/report/builder.rs`, `src/report/sort.rs`, `src/lib.rs`; create `src/report/usage.rs`
  **Acceptance**: `cargo test report_should_sort_usage_labels_alphabetically`; `cargo test source_rank_should`; `rg -n "config::.*UsageSource|crate::report::UsageSource" src/config`

- [x] 4. Rework runtime/footer rendering to consume report-owned legend metadata
  **What**: Move legend text/category ownership out of `src/runtime.rs` matches on config-derived enum variants. Runtime should render a report-provided legend contract (for example, legend entries or report usage styles) so it no longer knows whether a label came from OpenCode general config, OpenCode agents, or optional Weave parsing internals beyond report-facing display categories.
  **Files**: Modify `src/runtime.rs`, `src/report/usage.rs`, `src/report/model.rs`
  **Acceptance**: `cargo test footer_legend`; `cargo test usage_style`; `rg -n "OpenCode agents|Weave custom_agents|UsageSource::" src/runtime.rs`

- [x] 5. Thin the crate root so orchestration stays, ownership moves
  **What**: Keep `src/lib.rs` as crate wiring and public API only. Re-export the new config/report modules as needed, update `load_report_rows` to flow through the new ingestion-to-report adapter path, and migrate broad config-source tests out of crate-root once lower-level modules own them.
  **Files**: Modify `src/lib.rs`
  **Acceptance**: `cargo test`; `rg -n "active_usage_should|config_bundle_should|load_config_bundle_should" src/lib.rs`

- [x] 6. Verify migration safety and document out-of-scope items
  **What**: Run the full verification stack after the boundary split, confirm no source semantics changed relative to the test plan, and note explicitly that missing-inventory regression work remains a later follow-up rather than part of this module refactor. This note is documentation-only.
  **Files**: Modify `.weave/plans/config-ingestion-module-refactor.md`
  **Acceptance**: `mise run check`; `mise run test`; plan note present about missing-inventory being out of scope

## Out of Scope

### Missing-Inventory Regression Coverage
As specified in the requirements, the missing-inventory regression (handling cases where the OpenCode command or inventory is missing/unavailable) is explicitly out of scope for this refactor. This work is deferred to a later follow-up task and should include:
- Tests for missing `opencode` CLI executable
- Tests for inventory refresh failures  
- Tests for empty or malformed inventory responses

These edge cases are not covered by this module refactor and remain as future work.

## Verification
- [x] All tests pass
- [x] No regressions
- [x] `mise run check`
- [x] `mise run test`
- [x] `cargo test --test acceptance_config_sources`
- [x] `cargo test config::usage`
- [x] `cargo test source_rank_should`
- [ ] `rg -n "crate::report::\{UsageLabel, UsageSource\}|UsageSource::OpenCode|UsageSource::Weave" src`
