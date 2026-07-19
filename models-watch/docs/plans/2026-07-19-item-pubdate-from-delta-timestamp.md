# Plan: Lock item `<pubDate>` to delta detection timestamp

## Goal

Add test coverage that locks in the existing behavior: each RSS `<item>`'s `<pubDate>` must be the delta file's `.timestamp` (the UTC ISO instant when `models-watch.sh` detected the change), converted to RFC-822 format.

## Assumptions

- The current code already does the right thing: `models-feed.sh` reads `.timestamp` from each delta JSON and assigns it as `pubDate` for every item derived from that delta.
- RFC-822 conversion is handled by `rfc822_from_iso()` (BSD `date` / GNU `date` dual path) and produces strings like `Tue, 29 Apr 2026 10:00:00 +0000`.
- The test DSL already has `expect_rss_contains` for substring checks; a focused `expect_rss_pubDate` method is clearer but `expect_rss_contains` with the exact `<pubDate>...</pubDate>` string is sufficient and consistent with the existing style.
- No change to `models-feed.sh`, `models-watch.sh`, or `publish-feed.sh` is needed — only tests.

## Plan

1. **Add a dedicated RSS assertion to the test DSL** (`tests-rust/src/lib.rs`):
   - Add `expect_rss_pubDate(&self, expected: &str) -> &Self` that checks the generated feed contains a `<pubDate>expected</pubDate>` substring.
   - This is a thin wrapper over `expect_rss_contains` with a clearer name, making the test intent explicit.

2. **Add `pubDate` coverage to existing acceptance tests** (`tests-rust/tests/acceptance_rss.rs`):
   - `feed_emits_one_item_per_model` — the single delta has timestamp `2026-04-29T10:00:00Z`; assert all four items carry `<pubDate>Tue, 29 Apr 2026 10:00:00 +0000</pubDate>`.
   - `feed_should_order_items_newest_first` — the first delta has `2026-04-30T10:00:00Z` and the second `2026-04-29T10:00:00Z`; assert the newer `pubDate` appears before the older one (this already follows from item ordering, but makes the time semantics explicit).
   - `feed_should_write_rss_when_deltas_exist` — basic `pubDate` assertion for the single delta case.

3. **Run `mise run test`** (fallback: `cargo test --manifest-path tests-rust/Cargo.toml`) to verify the new assertions pass against the current implementation.

4. **Update `README.md` feed-format section** — add a line documenting that `<pubDate>` is the change-detection timestamp (RFC-822), not the feed-generation time. This keeps docs in sync with the locked behavior.

## Likely files

- `tests-rust/src/lib.rs` — add `expect_rss_pubDate` method
- `tests-rust/tests/acceptance_rss.rs` — add pubDate assertions to 3 existing tests
- `models-watch/README.md` — one-line addition to feed format docs

## Risks

- **RFC-822 weekday name depends on the actual calendar date** (`2026-04-29` → `Tue`). The tests must hardcode the correct weekday. This is deterministic and safe because the fixture timestamps are fixed.
- **BSD vs GNU `date` may produce slightly different whitespace or zone formats** in edge cases. The current `rfc822_from_iso()` forces `+0000` and uses the same `%a, %d %b %Y %H:%M:%S` format on both paths, so the output should be identical across platforms.
- **No test for `lastBuildDate`**: the channel-level `lastBuildDate` is feed-generation time (current), which is different from item-level `pubDate` (change-detection time). The user asked only about item time, so `lastBuildDate` is intentionally out of scope.

## Validation

- `bash -n models-feed.sh` — unchanged, syntax still valid.
- `mise run test` — all 31 tests (22 + 9) pass, including the new pubDate assertions.
- Manual sanity: run `models-feed.sh` against a fixture, grep `<pubDate>` in output, verify it matches the delta JSON's `.timestamp` converted to RFC-822.
