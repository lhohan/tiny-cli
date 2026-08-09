# Plan: Fix the 4 break-review findings in the rich post renderer

## Goal

Close the four findings from the adversarial review of `models-watch`'s rich Bluesky post renderer (`render_rich_added` in `models-broadcast.sh`):

1. **Missing ladder step 6** — truncate the ID within the rich title+ID pair before falling back to legacy (the plan/doc spec calls for it; the code jumps straight to legacy).
2. **Fragile `overshoot` scoping** — `local overshoot` is declared inside a conditional block but read in a later block; hoist it to function scope.
3. **Locale-dependent measurement** — `wc -m` counts bytes under `LANG=C`, inflating multibyte characters; the rich path (and the shared `truncate_value` helper it calls) should measure code points with jq's `length`, which is locale-independent.
4. **Weak facet assertion** — the rich-post test verifies byte offsets arithmetically but never checks the sliced bytes equal the URL; add a substring-identity assertion.

## Assumptions

- `truncate_value` is included in the jq-measurement change even though it is shared with legacy paths. It is behavior-identical for ASCII inputs (all existing truncation tests use ASCII) and strictly more correct under `LANG=C`; its jq slicing is already codepoint-based. Legacy `render_post` direct `wc -m` measurements stay untouched (pre-existing behavior, out of scope).
- After step 6 is added, the legacy fallback (step 7) becomes unreachable by construction (title-min + ID-min always fit within 300). It is kept as a defensive tail, not deleted.
- `README.md` and `docs/plans/2026-08-09-rich-bluesky-posts.md` already document the ladder as name → ID → legacy, so this change brings the code in line with the docs; no documentation edits are required.

## Plan (test-first, per repo conventions)

1. **RED — add the step-6 test** in `tests-rust/tests/acceptance.rs`:

   - `broadcast_capture_rich_ladder_truncates_long_id`: delta with `added: ["opencode-go/" + 290×"X"]` and a matching `models` entry whose `name` is `"M"`. Assert:
     - capture count 1;
     - `text.chars().count() <= 300`;
     - text contains `ID: opencode-go/X` **and** `…` (rich ID line, truncated);
     - text does **not** contain `is now available.` (rules out the legacy fallback — the key discriminator, since both paths emit `…` and log `TRUNC: model_id`);
     - stderr contains `TRUNC: model_id`.
   - The delta must be an inline literal with the full `models` metadata (cost/limit/caps fields, mirroring `rich_go_delta`'s shape); `rich_go_delta` itself cannot be reused because it hardcodes the `opencode-go/m` ID key.
   - Strengthen `broadcast_capture_renders_rich_post_for_free_zen_model` with a **supplementary** substring-identity assertion: `assert_eq!(&text.as_bytes()[byteStart..byteEnd], url.as_bytes())`. Keep the existing arithmetic byte-offset assertions; the new check proves the computed offsets actually point to the URL substring. Passes immediately; it pins correct behavior.
   - Expected RED signal: the step-6 test fails because today's code renders the legacy one-liner (`… is now available.`) for this input.

2. **Implement in `models-broadcast.sh` (`render_rich_added`)**:

   - **Step 6 (ID truncation)**: insert a new block between the name-truncation block and the legacy fallback:
     ```bash
     if (( total_cp > POST_MAX )); then
         overshoot=$(( total_cp - POST_MAX ))
         local id_cp max_id short_id
         id_cp="$(jq -nr --arg t "$model_id" '$t | length')"
         max_id=$(( id_cp - overshoot - 1 ))
         if (( max_id < 1 )); then max_id=1; fi
         short_id="$(truncate_value "$model_id" "$max_id" "model_id")"
         sections="$(jq -c --arg id "ID: ${short_id}" \
             'map(if .key == "id" then .text = $id else . end)' <<< "$sections")"
         text="$(jq -r 'map(.text) | join("\n")' <<< "$sections")"
         total_cp="$(jq -nr --arg t "$text" '$t | length')"
     fi
     ```
     The existing legacy fallback block (with its own legacy-text ID truncation) stays below, unchanged in structure, as the defensive step 7 — only its two `wc -m` measurements are converted in the next bullet. Update the `render_rich_added` comment to note that the legacy tail is unreachable after step 6 and is retained deliberately as a defensive guard.
   - **Hoist `overshoot`**: at the top of `render_rich_added`, directly after the existing parameter declarations, add `local overshoot=0`:
     ```bash
     render_rich_added() {
         local delta_file="$1"
         local model_id="$2"
         local delta_name="$3"
         local overshoot=0
     ```
     Then drop the `local` keyword from the assignment inside the name-truncation block (`local overshoot=$(( total_cp - POST_MAX ))` → `overshoot=$(( total_cp - POST_MAX ))`). The legacy tail already reads `overshoot` without declaring it and now uses the hoisted variable.
   - **jq codepoint measurement**: replace every `wc -m`-based measurement inside `render_rich_added` (initial `total_cp`, the ladder-loop re-measure, `title_name_len`, and the legacy-tail `cp_count`/`mid_len`) with `$(jq -nr --arg t "<value>" '$t | length')`. No `tr -d '[:space:]'` is needed — jq's `length` returns a bare number.

     Apply the same change in `truncate_value`:
     ```bash
     # Before:
     local cp_count
     cp_count=$(printf '%s' "$value" | wc -m)

     # After:
     local cp_count
     cp_count=$(jq -nr --arg t "$value" '$t | length')
     ```
     `truncate_value`'s slicing body already uses jq `.[0:$n]` which operates on code points, so only the measurement gate changes — the function becomes internally consistent.

3. **Verify ordering invariants**: the ladder arithmetic (`max_id = id_cp - overshoot - 1`) mirrors the existing name-truncation math and guarantees exactly-300-or-less after each step; no constants are hand-maintained.

## Likely files

- `models-watch/models-broadcast.sh` — `render_rich_added` (step 6 insert, hoist, jq measurements) and `truncate_value` (measurement).
- `models-watch/tests-rust/tests/acceptance.rs` — one new test (step-6 ID truncation) + one strengthened facet assertion.

## Risks

- **`truncate_value` blast radius**: shared with legacy `render_post`; mitigated by the existing ASCII truncation tests (`broadcast_truncates_long_*`) which pin identical behavior under both measurement methods.
- **Legacy `render_post` `wc -m` remains locale-dependent**: the legacy path still counts bytes under `LANG=C`. All current inputs are ASCII (byte count = code point count) and the path renders fixed-format strings (`New: <id> is now available.`), so this has no observable effect today. It is acknowledged tech debt, out of scope for this fix.
- **jq `length` semantics**: counts Unicode code points in jq ≥ 1.5; repo and CI runners have jq ≥ 1.6 — no version risk.
- **Step-6/legacy discrimination in tests**: both paths emit `…` and `TRUNC: model_id`; the test asserts on the *absence* of `is now available.` to prove the rich path was taken.
- **Dead code**: the legacy fallback tail becomes unreachable after step 6; retained deliberately as a guard, documented in the function comment.
- **Finding 3 has no testable RED signal in current environments — no regression test for it**: a byte-vs-codepoint test cannot be made to fail. Empirically verified on this repo's dev machine: BSD `wc -m` counts code points even under `LANG=C` (`Luna Café` → 9 under `LANG=C`, `C.UTF-8`, and `en_US.UTF-8`). GNU `wc -m` counts code points under any multibyte locale, which is what the ubuntu-latest runner ships (`C.UTF-8` on recent images, `en_US.UTF-8` historically); it counts bytes only under a non-UTF-8 locale (`LANG=C`, or an unset `LANG` on a C-default system). The bug is real but unobservable in dev and CI today, so no test can red on it. The fix (jq `length`) is still correct as locale-hardening, and every measurement line it touches is exercised by the step-6 test, the three legacy truncation tests, and the exact-text rich-post assertions.

## Validation

```bash
bash -n models-watch.sh && bash -n models-broadcast.sh
mise run test   # fallback: cargo test --manifest-path tests-rust/Cargo.toml
```

Expected: new step-6 test passes; all existing tests (65) stay green — including the legacy-hash pin (`4c6990…`) and the three pre-existing truncation tests that exercise `truncate_value`.
