# Model List Paging Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents are available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add page-by-page navigation to the model list so long inventories fit the terminal without continuous scrolling.

**Architecture:** Keep the current report renderer and fullscreen ratatui shell, but add a small paging helper in the runtime layer. Paging works on discrete pages of the rendered model table, not a selection cursor, so wrapped usage text stays attached to the row that produced it and the footer stays fixed. The runtime handles `j`/`k`, resets to the first page after a successful data refresh or sort change, clamps on resize, and shows a page indicator so users know where they are.

**Tech Stack:** Rust 2021, ratatui 0.28, crossterm 0.28, existing unit tests with `TestBackend`.

---

## Context

### What the code does today
- `src/runtime.rs` renders the full model report into one `Paragraph` and lets ratatui clip whatever does not fit.
- `q`, `r`, and `s` are the only runtime keys.
- The footer already stays fixed in a separate layout region, so paging only needs to affect the main report panel.
- Runtime rendering tests already exist in `src/runtime.rs` with `TestBackend`, which makes this change easy to verify without launching a real terminal.

### What the user wants
- If the model list is longer than the screen, they want paging.
- They do not want smooth scrolling.
- Vim-style keys should work, so use `j` / `k`.

### Docs checked
- Ratatui `Paragraph` renders wrapped `Text` inside a bordered area, so the inner height needs to be budgeted carefully.
- Crossterm exposes keyboard codes through the terminal event stream.

## Design decisions

- Treat the report panel’s inner height as the paging viewport.
- Pin the table header at the top of every page.
- The header row counts against the inner height budget, so each page has `inner_height - 1` lines available for row content.
- `page X/Y` counts model pages only; the repeated header does not create an extra page.
- Page boundaries are row-atomic: a row and all of its wrapped continuation lines stay on the same page.
- If a wrapped row would start on the last visible line of a page, move the whole row to the next page instead of splitting it, even if that leaves blank space at the bottom of the current page.
- If a single wrapped row is taller than the available page content height, keep it intact on its own page and accept that the terminal may clip the bottom of that page on very small screens.
- Reset paging to the top when a refresh succeeds or sort mode changes. That avoids landing mid-list after the order changes.
- Keep paging separate from `UiState` unless the runtime needs to expose it for tests. The page offset belongs to the view layer, not the data model.
- Show a page indicator at the left edge of the status line as `page X/Y • <status>`.
- Reserve the page indicator first and truncate the status text on the right when space is tight; keep the legend on the second footer line unchanged.
- Clamp every offset with saturating math. Empty lists and tiny terminals must not panic.

## Risk and security review

- No new external inputs, subprocesses, or network paths.
- No config data leaves the existing load path.
- The only safety risk is bad bounds math in tiny terminals or empty reports; the implementation must use saturating subtraction and clamp offsets on every draw.
- Key handling should remain a whitelist, so unknown keys stay inert.

## Tasks

### Task 1: Add a paging helper with tests

**Files:**
- Create: `src/runtime/paging.rs`
- Modify: `src/runtime.rs` (module wiring only)
- Test: `src/runtime/paging.rs` unit tests

- [ ] **Step 1: Write failing tests for paging math**

Add tests that prove:
- `j` advances to the next page.
- `k` moves back to the previous page.
- Offsets clamp correctly when the report is shorter than the viewport.
- A page that exactly fits the viewport stays on one page.
- A page that overflows by one line becomes a second page.
- Empty reports and tiny viewports still produce sane page metadata.
- An empty report shows `page 1/1`.
- A smaller resize clamps the current page instead of leaving the view blank.
- Repeated `k` presses from the last page stop at page 1.
- A wrapped row that would start on the last visible line moves to the next page intact.
- A wrapped row that is taller than the page content height stays intact on a single oversized page.

- [ ] **Step 2: Run the focused tests and confirm they fail**

Run: `cargo test --lib runtime::paging`

Expected: the new paging tests fail because the helper does not exist yet.

- [ ] **Step 3: Implement the minimal paging helper**

Add a small `PageState` or equivalent with:
- current scroll offset
- viewport height
- page count / current page helpers
- methods to page up, page down, and clamp
- a helper to compute `page X/Y`

- [ ] **Step 4: Run the focused tests again**

Run: `cargo test --lib runtime::paging`

Expected: the paging tests pass.

- [ ] **Step 5: Commit the helper in isolation**

Keep this change small so later runtime wiring stays easy to review.

### Task 2: Wire paging into the runtime and footer

**Files:**
- Modify: `src/runtime.rs`
- Modify: `src/runtime/paging.rs`
- Test: `src/runtime.rs` unit tests

- [ ] **Step 1: Write failing runtime tests for page navigation**

Add rendering tests that prove:
- the first page renders the top of the list
- `j` shows later rows
- `k` returns to the top page
- the footer status line shows the current page
- `q`, `r`, and `s` still work
- shrinking the terminal keeps the current view clamped to a valid page
- a successful refresh resets to page 1
- a sort change resets to page 1
- a cramped status line keeps `page X/Y` visible while truncating the status text
- a wrapped row that would start on the last visible line stays intact on the next page
- an empty report shows `page 1/1`
- an oversized wrapped row stays intact on a single oversized page

- [ ] **Step 2: Run the focused runtime tests and confirm they fail**

Run: `cargo test --lib runtime::tests`

Expected: the paging-specific runtime tests fail because the runtime does not yet track pages.

- [ ] **Step 3: Implement the runtime wiring**

Update the event loop to:
- recognise `j` and `k`
- update paging state only on those keys
- reset paging when a refresh succeeds or sort order changes
- keep the current page when a refresh fails

Update drawing to:
- render the current page slice into the report panel
- keep the footer fixed
- show a page indicator on the status line, ahead of the status text so it survives truncation better

- [ ] **Step 4: Run the targeted runtime tests again**

Run: `cargo test --lib runtime::tests`

Expected: the runtime paging tests pass.

- [ ] **Step 5: Run the existing runtime tests**

Run: `cargo test --lib runtime::tests`

Expected: the current rendering and colour tests stay green.

### Task 3: Update user-facing controls documentation

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update the controls list**

Document the new paging keys next to `q`, `r`, and `s`.

- [ ] **Step 2: Add a brief note about paging behaviour**

State that paging is page-by-page, not smooth scrolling, and that the page indicator stays visible.

- [ ] **Step 3: Check the doc wording**

Make sure the new text is concrete and short. No marketing language.

### Task 4: Full verification pass

**Files:** none expected, unless a test exposes a boundary bug.

- [ ] **Step 1: Run the full test suite**

Run: `cargo test`

Expected: all tests pass.

- [ ] **Step 2: Run the formatting check**

Run: `cargo fmt --check`

Expected: clean formatting.

- [ ] **Step 3: Manually review the paging edge cases**

Confirm the final code handles:
- empty model lists
- one-page model lists
- many wrapped rows
- terminal resize while mid-page

## Acceptance criteria

- `j` and `k` page through the model list.
- The UI does not rely on a selection cursor.
- The footer status line shows the current page.
- `q`, `r`, and `s` still behave exactly as before.
- Empty reports and tiny terminals do not panic.
- Existing report rendering tests still pass.
