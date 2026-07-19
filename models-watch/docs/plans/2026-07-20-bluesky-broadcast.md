# Plan: Broadcast recent model changes from GitHub Actions

## Summary

Keep the authoritative watcher state in this repository. Two Bash CLIs run
locally and in GitHub Actions:

```text
models-watch.sh      fetches models.dev and updates state/latest.json + deltas
models-broadcast.sh  posts eligible deltas and updates state/posted.json
```

GitHub Actions supplies scheduling, secrets, and commits. It does not contain
the detection or posting logic. Local runs update the same tracked files; the
operator reviews and commits them with Jujutsu.

The broadcaster uses the standard Bluesky `createSession` and `createRecord`
endpoints. It makes one attempt for each request: any transport or non-2xx
failure stops the run without recording the incomplete delta as posted.

## Repository state and bootstrap

- Stop ignoring the entire `state/` directory. Ignore only `state/feed.rss`
  and `tests-rust/target/`.
- Track `state/latest.json`, all `state/change-<timestamp>.json` files, and
  `state/posted.json`.
- The existing history has 21 delta files. Only the ten newest files, from
  `change-2026-06-04T18:00:01Z.json` through
  `change-2026-07-18T11:00:02Z.json`, are eligible for initial broadcast;
  every identifier in those files is provider-prefixed.
- Record the eleven older files as intentionally skipped because they predate
  provider-prefixed IDs. Do not publish or migrate them.

`state/posted.json` has this shape:

```json
{
  "deltas": {
    "change-2026-07-18T11:00:02Z.json": "<sha256>"
  },
  "skipped": {
    "change-2026-06-03T18:00:01Z.json": "pre-provider-prefix history"
  }
}
```

`deltas` maps fully posted delta basenames to the SHA-256 of the compact JSON
array of final rendered post texts, encoded as UTF-8 with no trailing newline.
`skipped` is an auditable terminal state distinct from posting. A filename may
not appear in both maps. A missing ledger is treated as empty; a present ledger
with the wrong shape, invalid hashes, or overlapping maps is a non-zero error
before any request is made.

Bootstrap by committing the ignore-rule change, existing snapshot/deltas, and
an initial ledger containing the eleven skipped entries. Preview the ten
eligible deltas with `--capture-dir`, then manually dispatch the broadcaster
on `main` once to publish all 16 resulting posts. The script waits one second
between successful posts and commits the resulting ledger in one update.

## `models-broadcast.sh`

```text
Usage: ./models-broadcast.sh [--state-dir <dir>] [--capture-dir <dir>] [--limit <n>]
```

- `--state-dir` defaults to script-relative `state/`.
- `--capture-dir` writes one JSON record per rendered post and changes no
  ledger or other state. It performs no authentication or network request and
  uses a fixed placeholder DID in the captured `createRecord` body.
- `--limit <n>` posts at most `n` complete deltas, oldest first. It must be a
  positive integer and is mutually exclusive with `--capture-dir`.
- Missing flag values, invalid limits, and unknown flags exit `2`; no eligible
  unledgered deltas exit `3`; missing live-posting credentials exit `4`.

Validate a delta before it is posted: it must contain a timestamp, string
arrays for `added` and `removed`, and changed entries with string `id`,
`old_name`, and `new_name` fields. Every ID must begin with `opencode-go/` or
`opencode/`. A malformed eligible delta fails the run before it is posted and
does not alter the ledger.

Render one post per affected model in deterministic removed, changed, then
added order, sorting alphabetically by provider-prefixed ID:

- `Removed: <model-id> is no longer available.`
- `Updated: <model-id>: "<old-name>" → "<new-name>"`
- `New: <model-id> is now available.`

Enforce a conservative 300-Unicode-code-point text limit. For an oversized
updated post, shorten the old name first, then the new name, and shorten the
model ID only as a last resort. Every shortened non-empty value ends in `…`.
Added and removed posts shorten only their model ID. Log the original and final
value of every shortened field to stderr. The final rendered texts are both
posted and hashed into the ledger.

Build request and capture JSON with `jq -n --arg` and `--argjson`. Strip
trailing slashes from `BLUESKY_PDS`, defaulting to `https://bsky.social`.
Require non-empty session `accessJwt` and `did` values before creating records.
Use `BLUESKY_HANDLE` and `BLUESKY_APP_PASSWORD` only for live posting.

For tests, `BLUESKY_PDS=file://<fixture-root>` enables a fixture transport.
The script reads endpoint-scoped, numbered JSON envelopes such as
`xrpc/com.atproto.server.createSession/1.json` and
`xrpc/com.atproto.repo.createRecord/1.json`. Each envelope is either
`{"status": 200, "body": {...}}` or `{"transport_error": "..."}`. Production
PDS URLs use `curl` normally.

## GitHub Actions

Add two workflows under `.github/workflows/`, both using the same concurrency
group, `models-watch`, with `cancel-in-progress: false`. This serializes
detection, posting, and ledger commits, preventing a detector commit from
invalidating an in-flight broadcaster push.

### Detect Model Updates

- Trigger every six hours and by `workflow_dispatch`.
- Run only for `main`, check out `main` with full history, and set
  `MODELS_WATCH_NO_OSASCRIPT=1`.
- Grant `contents: write`; commit and push only changed `state/latest.json`
  and `state/change-*.json` files with a GitHub Actions bot identity.
- Do not expose Bluesky secrets.

### Broadcast Model Updates

- Trigger after successful `Detect Model Updates` runs on `main` using
  `workflow_run`, and by `workflow_dispatch` on `main`.
- Gate every job to `main`; a dispatch from another ref must not receive
  Bluesky credentials or write state.
- Grant `contents: write`, check out and fast-forward `main`, then expose only
  `BLUESKY_HANDLE`, `BLUESKY_APP_PASSWORD`, and optional `BLUESKY_PDS`.
- Run `./models-broadcast.sh`, treating exit `3` as a successful no-op.
- Commit and push only `state/posted.json` when it changed.

## Tests, documentation, and verification

- Extend the Rust acceptance DSL for broadcaster setup, ledger inspection,
  capture inspection, and `file://` PDS fixture envelopes.
- Cover the bootstrap cutoff/skipped ledger, oldest-first order, hashes,
  `--limit`, capture purity, invalid flags, missing credentials, malformed
  ledger/delta rejection, escaped request bodies, truncation priority, and
  single-attempt transport/5xx failure with no ledger update.
- Validate the workflow files with `actionlint` when available.
- Update the README with credentials, state/ledger format, capture usage,
  initial rollout, conservative text truncation, and the no-retry/possible
  duplicate risk after an ambiguous failed request.

```bash
bash -n models-watch.sh
bash -n models-broadcast.sh
mise run test
# Fallback: cargo test --manifest-path tests-rust/Cargo.toml
```
