# Plan: Publish a public RSS feed for models-watch

## Goal

Provide a public **RSS 2.0 feed** of model changes detected by `models-watch`. A dedicated, independent script reads the existing `state/change-*.json` deltas and rebuilds the feed; a companion deploy step publishes it to a **`pages` branch** on Codeberg via a Jujutsu bookmark. `models-watch.sh` itself is unchanged.

## Assumptions

- **Separate module:** A new script (`models-feed.sh`) in the `models-watch/` directory. It reads `state/change-*.json` (relative to the script dir, same convention as `models-watch.sh`), never fetches the network, never writes deltas — pure read-transform. The two scripts run sequentially in cron (the watcher logs changes; the feed script publishes them).
- **Idempotent full rebuild each run:** Every invocation rebuilds the entire feed from on-disk deltas. No append, no partial states. Safe to run any time.
- **Window:** Last **100** deltas, newest first.
- **Granularity:** One `<item>` **per delta (change event)** — the natural shape for a changelog feed. Title summarises counts; description lists affected models.
- **Deploy target:** A `pages` branch in this repo (Forgejo/Codeberg Pages convention). The feed file is written to a configured path; a deploy step commits it to the `pages` branch (Jujutsu bookmark, pushed to Codeberg). The Codeberg Pages URL then serves the feed.
- **Deploy is out-of-script:** The feed script writes a file. A separate tiny step (jj new commit on `pages` bookmark, push) handles publishing. Keeps the feed script single-responsibility, matching the repo's "one self-contained tool per directory" ethos.
- **JJ availability:** The repo may use Jujutsu (per top-level AGENTS.md). Deploy guidance will use `jj` bookmarks; fall back to `git` if explicitly requested.

## Open questions

- None material to the script itself. The `pages`-branch deploy wiring (commit message, push target) is documented as advisory guidance, not implemented by the script.

## Plan

1. **New file `models-feed.sh`** (`#!/usr/bin/env bash`, `set -euo pipefail`) — self-contained, mirroring models-watch.sh conventions:
   - Resolves `STATE_DIR` relative to its own location (`SCRIPT_DIR` pattern), defaulting to the sibling `state/` directory so it reads the same deltas `models-watch.sh` writes. Override via `MODELS_WATCH_STATE_DIR`.
   - Output path via `--output <file>` / `MODELS_WATCH_FEED_FILE` env; default `state/feed.rss`.
   - Globs `state/change-*.json`, sorts lexicographically, takes the **last 100** (matches the established sort pattern in `models-watch.sh`'s `do_report`), reverses to newest-first.
   - Builds an **RFC-822 date** via dual-path `date` (BSD `/bin/date -j -f …` / GNU `date -Ru`), same outer shape as `utc_to_local`.
   - Emits RSS 2.0: channel header (`title="models-watch"`, `link="https://models.dev"`, `description`, `language=en`, `lastBuildDate`=now RFC-822) plus one `<item>` per delta: `title` summarising added/removed/changed counts, `pubDate` RFC-822, `guid isPermaLink="false"`=`models-watch-<timestamp>`, `description` listing IDs.
   - Uses `jq` with `@html` for content escaping (safe against `<`/`&`/`>`).
   - Writes atomically: build to a temp file, `mv` over the target — prevents half-written feeds under cron interruption.
   - Exit 0 on success; exit 3 if `state/` has no deltas (mirror models-watch's "missing block -> exit 3" signal intent: "nothing to publish").
   - Usage: `./models-feed.sh [--output <file>] [--state-dir <dir>]`. Unknown flag → exit 2 (CLI contract consistency).
2. **Acceptance tests in `tests-rust/`** — extend the DSL (`tests-rust/src/lib.rs`) minimally:
   - Add a second builder entry point (e.g. `given_feed()`) targeting `models-feed.sh` (or a generic `with_script(name)` selector on `AppSpec`), reusing the existing temp-tool-dir + fixture/delta seeding infrastructure.
   - New tests in `tests-rust/tests/acceptance_rss.rs` (or appended to `acceptance.rs`):
     - `feed_should_write_rss_when_deltas_exist` — seed deltas, run, assert feed exists, well-formed RSS, expected `<item>`s, expected `<guid>`, RFC-822 `pubDate`.
     - `feed_should_order_items_newest_first` — multiple deltas, assert newest is first.
     - `feed_should_limit_to_last_100` — seed 102 deltas, assert exactly 100 `<item>`s.
     - `feed_should_escape_special_chars` — model name with `<`, `&`, assert valid XML.
     - `feed_should_exit_3_when_no_deltas` — empty `state/`, exit 3.
     - `feed_should_write_to_custom_output_path` — `--output <path>` honoured.
     - `feed_should_not_fetch_network` — run without `MODELS_WATCH_API_URL`, assert no network call (the script has no fetch path, so this is structural).
3. **Update `README.md`:** document `models-feed.sh`, its flags/env, the feed format (RSS 2.0, **per-delta items**, last-100 window), and a **"Publishing to Codeberg Pages"** section: output to a tracked path, then commit + push to the `pages` branch (`jj bookmark create pages`; push). Show the combined cron entry chaining `models-watch.sh && models-feed.sh <output> && <deploy step>`.
4. **Add a `.mise.toml` convenience task** `feed` that runs `./models-feed.sh`.
5. **Verify** with `bash -n models-feed.sh`, `bash -n models-watch.sh` (unchanged), and `mise run test` (full suite green; existing tests untouched).

## Likely files

- `models-feed.sh` — new, self-contained RSS generator (the work).
- `tests-rust/tests/acceptance_rss.rs` — new test scenarios for the feed script.
- `tests-rust/src/lib.rs` — DSL additions for the second script (`given_feed` / `with_script` + RSS assertions: `expect_rss_file`, `expect_rss_item_count`, `expect_rss_contains`).
- `README.md` — document the new script, feed format, publishing flow.
- `.mise.toml` — optional `feed` task (advisory).
- *(Advisory, not committed by this plan)* `public/feed.xml` or chosen tracked output path on the `pages` branch.

## Risks

- **RSS validity from hand-rolled XML in bash.** Mitigate by `jq @html` escaping and assertion-based tests; optionally `xmllint --noout` locally when available (don't hard-require it).
- **Date format portability.** RFC-822 must work on macOS BSD `date` and GNU `date`. The dual-path pattern is already proven in `utc_to_local`; reuse it verbatim in shape.
- **State-dir coupling.** `models-feed.sh` reads from `state/` — it must find the same state dir `models-watch.sh` writes. Defaulting to the sibling `state/` (both scripts live in `models-watch/`) keeps them aligned; document `MODELS_WATCH_STATE_DIR` for unusual setups.
- **Deploy correctness.** If the tracked output path drifts from the `pages`-branch layout, the public feed won't update. Mitigated by README guidance and the explicit `--output` contract; the script itself is agnostic.
- **First-run noise** (one big "all models added" item) is consistent with `--report` and the delta files' own first-run behaviour — expected, unchanged.

## Validation

- `bash -n models-feed.sh` and `bash -n models-watch.sh` (syntax; the latter unchanged).
- `mise run test` (full suite green; all existing tests untouched, new RSS tests pass).
- New RSS tests assert: feed exists after a change, item count == `min(100, total deltas)`, newest item first, valid RSS 2.0 structure, RFC-822 dates, HTML-escaped special chars, `--output` path honoured, exit 3 on empty state, no network fetch.
- Manual sanity: run `./models-watch.sh` against a fixture then `./models-feed.sh --output /tmp/feed.rss`; `cat /tmp/feed.rss`; validate in an RSS reader / `xmllint --noout` if available.