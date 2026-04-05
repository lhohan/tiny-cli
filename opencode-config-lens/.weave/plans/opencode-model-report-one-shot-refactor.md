# OpenCode Config Lens One-Shot Refactor

## TL;DR
> **Summary**: Refactor the project into small, testable Rust modules that separate config/data loading, row assembly, TUI state/runtime/view code, and shared text formatting helpers while preserving the current fullscreen ratatui behaviour and retaining the tiny plain-text renderer for tests.
> **Estimated Effort**: Large

## Context
### Original Request
Create a complete implementation plan for a one-shot refactor of `/Users/hans/dev/tiny-cli/opencode-model-report` that preserves functionality, keeps the tiny plain-text renderer for tests, adds fluent acceptance DSL coverage, strengthens effects/behaviour testing, and safely reaches the PRD target state with continuous green tests.

### Key Findings
- The current codebase is already a fullscreen ratatui app and the test suite is green (`cargo test`).
- `src/main.rs` is dead legacy code that duplicates earlier CLI/report logic and should be deleted only after behaviour is characterized elsewhere.
- `src/lib.rs` is a kitchen sink: config types, parsers, process/network effects, row assembly, formatting helpers, UI state, and many tests all live together.
- `src/runtime.rs` mixes terminal lifecycle, worker-thread orchestration, keyboard loop, layout composition, table rendering, wrapping, and styling.
- Several helpers are duplicated across the codebase: JSONC stripping, ANSI stripping, cost formatting, alignment, wrapping.
- Existing tests mostly cover pure logic and small rendering/style helpers; there is no fluent acceptance DSL and no strong effect-observation layer around startup, refresh, or failure paths.
- The plain-text renderer already exists as `render_report_rows`; it is currently embedded in `src/lib.rs` and should be preserved as a stable test-facing contract.

### Target Module Tree
```text
src/
  bin/
    ocl.rs                          # CLI entrypoint only
  lib.rs                            # thin re-exports only
  cli.rs                            # clap args and CLI-to-app config mapping
  app/
    mod.rs                          # public app API
    controller.rs                   # startup + manual refresh orchestration
    state.rs                        # UiState, UiMode, UiAction, sort cycling
    ports.rs                        # traits for config/inventory/cost effects
    errors.rs                       # ConfigError, LoadError, RuntimeError
  config/
    mod.rs                          # config loading API
    types.rs                        # OpenCodeConfig, WeaveConfig, AgentConfig, ConfigBundle
    jsonc.rs                        # JSONC parsing and trailing-comma stripping
    loader.rs                       # resolve_config_home + file loading
    usage.rs                        # collect_active_usage and usage-label mapping
  data/
    mod.rs                          # external data loading API
    inventory.rs                    # opencode models --refresh invocation + parsing
    pricing.rs                      # curl/models.dev invocation + API parsing
    process.rs                      # small subprocess runner seam if needed
  report/
    mod.rs                          # report API
    model.rs                        # ModelRow, UsageLabel, UsageSource, stable row identity
    sort.rs                         # SortMode + deterministic comparators
    builder.rs                      # build_rows / report assembly
    text.rs                         # format_cost, ljust, rjust, wrapping helpers
    plain.rs                        # tiny plain-text renderer retained for tests
  tui/
    mod.rs                          # runtime entrypoint
    runtime.rs                      # terminal setup/cleanup and event loop shell
    events.rs                       # key mapping and worker messages
    view.rs                         # layout composition
    widgets.rs                      # report table/footer/header builders
    styles.rs                       # ratatui styles only
tests/
  support/
    mod.rs
    scenario.rs                     # fluent acceptance DSL + fake ports/harness
  acceptance_startup.rs             # loading, initial success/failure, fullscreen contract
  acceptance_refresh.rs             # manual refresh success/failure, snapshot retention
  acceptance_sorting.rs             # sort cycling and deterministic order
  acceptance_errors.rs              # CLI/error exit semantics and messages
  report_plain_renderer.rs          # plain-text renderer contract tests
  report_rows.rs                    # row assembly/sort tests
  cli_help.rs                       # help contract
```

### Responsibility Boundaries
- `config/*` owns reading and interpreting config files only.
- `data/*` owns external effects (`opencode`, `curl`) and parsing their outputs only.
- `report/*` owns stable domain rows, sorting, formatting, and the plain-text test renderer only.
- `app/*` owns state transitions and refresh orchestration only.
- `tui/*` owns terminal runtime and ratatui presentation only; it must consume prepared rows/state and never parse configs or call subprocesses directly.
- `tests/support/scenario.rs` owns fake effects and fluent test narration only; production code must not depend on it.

## Objectives
### Core Objective
Refactor the project into clear modules with explicit seams between effects, report assembly, app state, and rendering so another agent can safely implement PRD-complete behaviour with TDD and no functional regressions.

### Deliverables
- [ ] Thin-crate module layout with dead `src/main.rs` removed and all current behaviour preserved.
- [ ] Tiny plain-text renderer retained as a dedicated `report/plain` test contract.
- [ ] Fluent acceptance DSL covering startup, refresh, sorting, rendering, and error behaviour.
- [ ] Stronger behaviour/effects tests proving subprocess, fetch, status, and snapshot-retention semantics.
- [ ] Explicit runtime/view/style boundaries in the ratatui layer.

### Definition of Done
- [ ] `cargo test` passes.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo test --test acceptance_startup --test acceptance_refresh --test acceptance_sorting --test acceptance_errors` passes.
- [ ] `cargo test --test report_plain_renderer --test report_rows --test cli_help` passes.
- [ ] `rg -n "\b(assert|assert_eq|assert_ne|panic)!\s*\(" tests/acceptance_* tests/support` returns no matches.
- [ ] `src/main.rs` is removed and `cargo run -- --help` still shows the expected CLI contract.
- [ ] Manual refresh failure keeps the previous snapshot and surfaces the error in status tests.
- [ ] Unknown-cost rows sort last in both cost-based modes in unit and acceptance coverage.

### Guardrails (Must NOT)
- [ ] Must NOT change the user-visible key bindings: `q`, `r`, `s`.
- [ ] Must NOT drop the fullscreen ratatui runtime.
- [ ] Must NOT remove the tiny plain-text renderer used for tests.
- [ ] Must NOT introduce auto-refresh loops or persistent storage.
- [ ] Must NOT let TUI code call config parsing, `opencode`, or `curl` directly.
- [ ] Must NOT perform large behavioural rewrites without first adding characterization coverage.

## TODOs

- [ ] 1. Phase 0 — Lock current behaviour with characterization tests
  **What**: Before moving code, add missing tests for PRD-critical behaviour that is currently implicit: deterministic sort tie-breaks, duplicate usage-label preservation, refresh error exit-code rules, loading-state/footer contracts, and the plain-text renderer output. Use TDD for each contract: write a failing test, implement only enough extraction glue to keep behaviour, then rerun the full suite.
  **Files**: Modify `tests/report_rows.rs`, `tests/cli_help.rs`, `src/lib.rs`, `src/runtime.rs`; create `tests/report_plain_renderer.rs`, `tests/acceptance_startup.rs`.
  **Acceptance**: `cargo test`; `cargo test --test report_plain_renderer --test report_rows --test cli_help --test acceptance_startup`.

- [ ] 2. Phase 1 — Introduce the fluent acceptance DSL harness first
  **What**: Create a minimal fluent DSL around fake ports so the rest of the refactor can be driven by behaviour tests instead of raw assertions. Start with one happy-path startup scenario and one refresh-failure scenario, then migrate new acceptance coverage to the DSL as the default style.
  **Files**: Create `tests/support/mod.rs`, `tests/support/scenario.rs`; modify `tests/acceptance_startup.rs`, `tests/acceptance_refresh.rs`.
  **Acceptance**: `cargo test --test acceptance_startup --test acceptance_refresh`; `rg -n "\b(assert|assert_eq|assert_ne|panic)!\s*\(" tests/acceptance_* tests/support` returns no matches in DSL-based tests.

- [ ] 3. Phase 2 — Extract report-domain and shared text helpers from `src/lib.rs`
  **What**: Move pure report code first because it is easiest to isolate safely. Extract row/domain types, sorting, row building, cost formatting, justification, wrapping, model splitting, and the tiny plain-text renderer into `report/*`. Keep public re-exports in `src/lib.rs` so the CLI binary and existing tests stay stable while the internal structure changes. Drive each move with unit tests and the new plain-renderer contract tests.
  **Files**: Modify `src/lib.rs`; create `src/report/mod.rs`, `src/report/model.rs`, `src/report/sort.rs`, `src/report/builder.rs`, `src/report/text.rs`, `src/report/plain.rs`; modify `tests/report_rows.rs`, `tests/report_plain_renderer.rs`.
  **Acceptance**: `cargo test --test report_rows --test report_plain_renderer`; `cargo test`.

- [ ] 4. Phase 3 — Extract config parsing/loading/usage collection into `config/*`
  **What**: Move config types, JSONC parsing, home-dir resolution, file loading, optional weave handling, and usage-label collection into focused modules. Preserve exact current semantics, then add missing tests for repeated usage labels, alphabetical usage sorting before display, required `opencode.jsonc`, optional `weave-opencode.jsonc`, and display-name precedence. Do not change runtime code yet beyond calling the new APIs.
  **Files**: Modify `src/lib.rs`; create `src/config/mod.rs`, `src/config/types.rs`, `src/config/jsonc.rs`, `src/config/loader.rs`, `src/config/usage.rs`; modify `tests/report_rows.rs`; add module-local tests under the new config files as needed.
  **Acceptance**: `cargo test config_`; `cargo test`.

- [ ] 5. Phase 4 — Extract external data effects into `data/*` with test seams
  **What**: Isolate `opencode models --refresh` and `curl -fsSL https://models.dev/api.json` behind small ports so behaviour tests can observe effects without spawning real subprocesses. Keep parsing separate from execution. Preserve exact error mapping: missing command => exit `3`, startup refresh failure => subprocess code or `4`, curl failure => exit `3`, post-launch refresh failure => status update without snapshot loss.
  **Files**: Modify `src/lib.rs`; create `src/data/mod.rs`, `src/data/inventory.rs`, `src/data/pricing.rs`, `src/data/process.rs`, `src/app/ports.rs`, `src/app/errors.rs`; modify `tests/acceptance_startup.rs`, `tests/acceptance_refresh.rs`, `tests/acceptance_errors.rs`.
  **Acceptance**: `cargo test --test acceptance_errors --test acceptance_refresh --test acceptance_startup`; `cargo test`.

- [ ] 6. Phase 5 — Introduce `app/controller` to own startup and manual refresh orchestration
  **What**: Move `load_report_rows`, worker result handling, refresh semantics, and non-render state transitions out of `src/lib.rs`/`src/runtime.rs` into an application controller that depends on ports instead of concrete effects. `UiState` and sort cycling should live in `app/state.rs`; startup/refresh orchestration should live in `app/controller.rs`. Add behaviour tests for: initial loading state, success path, manual refresh success, manual refresh failure preserving current rows, and deterministic visible-row ordering after sort changes.
  **Files**: Modify `src/lib.rs`, `src/runtime.rs`; create `src/app/mod.rs`, `src/app/controller.rs`, `src/app/state.rs`; modify `tests/acceptance_startup.rs`, `tests/acceptance_refresh.rs`, `tests/acceptance_sorting.rs`.
  **Acceptance**: `cargo test --test acceptance_startup --test acceptance_refresh --test acceptance_sorting`; `cargo test`.

- [ ] 7. Phase 6 — Split ratatui runtime, view composition, widgets, and styles
  **What**: Refactor the TUI layer last, once state and ports are stable. Keep `tui/runtime.rs` responsible only for terminal setup, cleanup, event polling, resize redraws, and dispatch to the controller. Move layout composition to `tui/view.rs`, table/header/footer widget building to `tui/widgets.rs`, and palette/style functions to `tui/styles.rs`. Add focused rendering tests using ratatui `TestBackend`/buffer assertions for footer legend visibility, loading view, active/inactive row accents, and wrapped usage rendering.
  **Files**: Modify `src/runtime.rs`, `src/bin/ocl.rs`, `src/lib.rs`; create `src/tui/mod.rs`, `src/tui/runtime.rs`, `src/tui/events.rs`, `src/tui/view.rs`, `src/tui/widgets.rs`, `src/tui/styles.rs`; modify `tests/acceptance_startup.rs`, `tests/acceptance_sorting.rs`; add module-local tests under `src/tui/*.rs`.
  **Acceptance**: `cargo test tui_`; `cargo test --test acceptance_startup --test acceptance_sorting`; `cargo test`.

- [ ] 8. Phase 7 — Delete legacy `src/main.rs` and make `src/lib.rs` a thin facade
  **What**: Once all behaviours are covered and the binary uses only the new modules, delete the dead duplicate `src/main.rs`. Reduce `src/lib.rs` to re-exports and crate wiring only. This is the cleanup phase, not a logic-change phase; if any test starts failing here, back out and move the remaining logic into a named module instead of reworking behaviour.
  **Files**: Delete `src/main.rs`; modify `src/lib.rs`, `src/bin/ocl.rs`.
  **Acceptance**: `cargo test`; `cargo run -- --help`; `cargo test --test cli_help`.

- [ ] 9. Phase 8 — Finish DSL migration and tighten behavioural coverage
  **What**: Migrate remaining high-value user-facing tests to the fluent DSL so startup, refresh, sorting, resize redraw, and error contracts read as behaviour specs rather than low-level mechanics. Keep low-level unit tests for pure helpers, but use the DSL for end-to-end app behaviour and observed effects. Ensure every PRD behaviour that matters to users has either a dedicated acceptance test or a focused unit test with a clear ownership boundary.
  **Files**: Modify `tests/acceptance_startup.rs`, `tests/acceptance_refresh.rs`, `tests/acceptance_sorting.rs`, `tests/acceptance_errors.rs`, `tests/support/scenario.rs`; optionally trim obsolete assertions in `src/lib.rs`/`src/runtime.rs` tests after equivalents exist in better locations.
  **Acceptance**: `cargo test --test acceptance_startup --test acceptance_refresh --test acceptance_sorting --test acceptance_errors`; `rg -n "\b(assert|assert_eq|assert_ne|panic)!\s*\(" tests/acceptance_* tests/support` returns no matches.

- [ ] 10. Phase 9 — Final verification against the PRD and regression sweep
  **What**: Run the full verification stack, inspect coverage gaps against PRD FR-1 through FR-10, and patch only missing tests or boundary wiring. Confirm deterministic sorting, manual refresh behaviour, footer legend visibility, fullscreen runtime path, `--home-dir`, startup/refresh error rules, and preservation of the plain-text renderer. Document any intentionally deferred polish separately instead of sneaking it into the refactor.
  **Files**: Modify only the minimal files still missing coverage or crate wiring; likely `tests/*`, `src/lib.rs`, `src/tui/*`, `src/app/*`.
  **Acceptance**: `cargo fmt --check`; `cargo test`; `cargo test --test acceptance_startup --test acceptance_refresh --test acceptance_sorting --test acceptance_errors --test report_plain_renderer --test report_rows --test cli_help`.

## Fluent Acceptance DSL Design
- [ ] Entry point: `given_model_report() -> GivenScenario` in `tests/support/scenario.rs`.
- [ ] Setup phase type: `GivenScenario` owns temp config home, fake inventory output, fake pricing payload, fake subprocess/fetch outcomes, terminal size, and initial snapshot fixtures; all setup methods return `Self`.
- [ ] Action phase type: `WhenScenario` is returned by explicit boundaries such as `.when_started()`, `.when_refresh_pressed()`, `.when_sort_pressed()`, `.when_quit_pressed()`, `.when_resized(width, height)`.
- [ ] Assertion phase types: `.then_frame() -> FrameThen`, `.then_effects() -> EffectsThen`, `.then_exit() -> ExitThen`, `.then_state() -> StateThen`.
- [ ] Type-transition rule: setup methods are unavailable after `when_*`, and assertions are unavailable before `when_*`; assertions do not expose more setup methods.
- [ ] Minimal domain assertions: `shows_loading()`, `shows_models_in_order([...])`, `shows_status("...")`, `shows_legend()`, `shows_sort_mode("...")`, `keeps_previous_snapshot()`, `ran_opencode_refresh(times)`, `fetched_costs(times)`, `exits_with_code(code)`, `stderr_contains("...")`.
- [ ] Effects model: the DSL should use fake ports plus an effect recorder so tests can assert behaviour without shelling out.
- [ ] Rendering model: frame assertions should render through ratatui `TestBackend` and inspect buffer text/styles; plain renderer assertions should use `report/plain.rs` directly.

## Risk Controls
- [ ] Characterize first, then move code. No extraction without a protecting test.
- [ ] Keep `src/lib.rs` re-exports stable until the last cleanup phase so integration tests and the binary do not churn unnecessarily.
- [ ] Introduce ports before changing runtime flow so external effects become observable in tests.
- [ ] Preserve the plain-text renderer throughout the refactor; treat it as a compatibility surface for tests.
- [ ] Delete `src/main.rs` only after equivalent behaviour is covered elsewhere and the binary no longer references any legacy path.
- [ ] Use module-local unit tests for pure helpers and DSL-based acceptance tests for cross-module behaviour.
- [ ] After each phase, run the listed verification commands before starting the next phase; do not batch multiple risky moves under one green run.

## Verification
- [ ] All tests pass
- [ ] No regressions
- [ ] `cargo fmt --check`
- [ ] `cargo test`
- [ ] `cargo test --test acceptance_startup --test acceptance_refresh --test acceptance_sorting --test acceptance_errors`
- [ ] `cargo test --test report_plain_renderer --test report_rows --test cli_help`
- [ ] `cargo run -- --help`
