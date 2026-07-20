# models-watch

Detect when [opencode-go](https://opencode.ai/docs/zen) and free OpenCode Zen models change on [models.dev](https://models.dev).

## Usage

```bash
./models-watch.sh [--notify-file <path>] [--report]
```

- Without `--notify-file`: fires a macOS notification popup via `osascript` when models change.
- With `--notify-file <path>`: writes the notification message to `<path>` (for scripting and testing).
- `--report`: print the last 10 recorded changes in human-readable format and exit. Mutually exclusive with fetching — reads local state only, never touches the network.

### `--report` example output

```
2026-04-29 10:34:45 BST
  Added:
    opencode-go/model-a
    opencode/model-b
  Removed:
    (none)
  Changed:
    opencode/model-y "Old Name" → "New Name"

2026-04-28 12:15:00 BST
  Added:
    opencode-go/model-z
  Removed:
    opencode/model-x
  Changed:
    (none)
```

If no changes have been recorded yet:

```
No changes recorded yet.
```

## What it does

1. Fetches `https://models.dev/api.json`.
2. Extracts the `opencode-go` provider block and free models from the `opencode` block (where `cost.input` and `cost.output` are both `0`).
3. Compares provider-prefixed model IDs (`opencode-go/<id>` and `opencode/<id>`) against the last snapshot (`state/latest.json`).
4. On first run or when models are added/removed/changed, writes a JSON delta file (`state/change-<timestamp>.json`).
5. Notifies on change.

## Dependencies

- `curl`
- `jq`
- `osascript` (macOS, for popup notifications)

## Cron

Run every 4 hours. Chain the watcher and feed generator together:

```cron
0 */4 * * * cd /path/to/models-watch && ./models-watch.sh && ./models-feed.sh && …
```

See [Publishing to Codeberg Pages](#publishing-to-codeberg-pages) for the deploy step.

## State

All runtime state lives under `state/` relative to the script:

| File | Purpose |
|------|---------|
| `state/latest.json` | Synthetic merged snapshot of watched models, keyed by provider-prefixed model ID (used for comparison on next run) |
| `state/change-<timestamp>.json` | Delta file, written when models are added, removed, or renamed |
| `state/posted.json` | Ledger of broadcast deltas, written by `models-broadcast.sh` |

## Public RSS Feed

`models-feed.sh` generates an **RSS 2.0 feed** from the change deltas recorded by
`models-watch.sh`.  It is a standalone script with no network access — it reads
`state/change-*.json` and writes a feed file.  The two scripts can be chained
in cron.

### Usage

```bash
./models-feed.sh [--output <file>] [--state-dir <dir>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--output <file>` | `state/feed.rss` | Where to write the RSS feed. Also settable via `MODELS_WATCH_FEED_FILE`. |
| `--state-dir <dir>` | sibling `state/` | Directory containing `change-*.json` deltas. Also settable via `MODELS_WATCH_STATE_DIR`. |
| `--feed-url <url>` | — | Public URL of the feed (adds `<atom:link rel="self">` for reader discovery). Also settable via `MODELS_WATCH_FEED_URL`. |

### Feed format

- RSS 2.0, XML 1.0, UTF-8
- One `<item>` **per affected model**, split by action (added, changed, removed), newest first
- Window: last 100 **items** total (a single delta with many models may be partially included)
- Item `<title>` is `New: <model-id>`, `Updated: <model-id>`, or `Removed: <model-id>`
- Item `<description>` contains the model ID (and for changed, the old → new name) in CDATA
- Item `<guid>` is `models-watch-<ISO timestamp>-<action>-<model-id>` (`isPermaLink="false"`)
- Item `<pubDate>` is the delta timestamp in RFC-822 format

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Feed written (or no deltas, no error) |
| 2 | Unknown flag |
| 3 | No change deltas exist (nothing to publish) |

### Publishing to GitHub Pages

[GitHub Pages](https://pages.github.com/) is configured to serve from the
`docs/` directory on the `main` branch, so the feed is available at:
`https://<user>.github.io/tiny-cli/models-watch.rss`

### Convenience: `mise run publish-feed`

The repo root defines a `publish-feed` mise task that runs the full pipeline
automatically (detect changes, regenerate feed to `docs/`, commit, push `main`).
From the repo root:

```bash
mise run publish-feed
```

This invokes `models-watch/publish-feed.sh`, which chains:
1. `models-watch.sh` — detect model changes
2. `models-feed.sh --output docs/models-watch.rss --feed-url <url>` — regenerate
3. `jj commit` — commit only the feed file
4. `jj bookmark set main -r @-` — move main bookmark
5. `jj git push --bookmark main` — push to GitHub

If no deltas exist or the feed content is unchanged, the script exits 0
without committing or pushing.

## Broadcast to Bluesky

`models-broadcast.sh` posts model change deltas to Bluesky through the
[com.atproto.repo.createRecord](https://docs.bsky.app/docs/api/com-atproto-repo-create-record)
endpoint.

### Usage

```bash
./models-broadcast.sh [--state-dir <dir>] [--capture-dir <dir>] [--limit <n>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--state-dir <dir>` | sibling `state/` | Directory containing `change-*.json` deltas and `posted.json` ledger |
| `--capture-dir <dir>` | — | Preview mode: write rendered posts as numbered JSON files, no auth or network |
| `--limit <n>` | — | Post at most `n` complete deltas (positive integer, mutually exclusive with `--capture-dir`) |

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Capture written or all eligible deltas posted |
| 2 | Unknown flag, missing flag value, or invalid `--limit` |
| 3 | No unledgered deltas to process |
| 4 | Missing Bluesky credentials (`BLUESKY_HANDLE` / `BLUESKY_APP_PASSWORD`) or session creation failed |

### Capture mode (`--capture-dir`)

Writes one JSON record per rendered post into the specified directory. Each
record contains the delta filename, model ID, action type, and rendered post
text. Capture mode performs no authentication, makes no network requests, and
does not modify the posted ledger. It uses a fixed placeholder DID in the
rendered record body.

Use capture mode to preview what would be posted before running live.

### Credentials

Set these environment variables for live posting:

| Variable | Required | Default |
|----------|----------|---------|
| `BLUESKY_HANDLE` | Yes | — |
| `BLUESKY_APP_PASSWORD` | Yes | — |
| `BLUESKY_PDS` | No | `https://bsky.social` |

Use an [App Password](https://bsky.app/settings/app-passwords) rather than
your account password.

### State ledger (`state/posted.json`)

The ledger tracks which deltas have been broadcast:

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

- `deltas`: maps posted delta basenames to the SHA-256 of the compact JSON
  array of their final rendered post texts (UTF-8, no trailing newline).
- `skipped`: auditable terminal entries for deltas that will never be posted.
  A filename may not appear in both maps.

A missing ledger is treated as empty. A malformed ledger (wrong shape, invalid
hashes, overlapping maps) causes the script to exit with an error before making
any requests.

### Text truncation

All posts are limited to 300 Unicode code points. For updated posts, the old
name is shortened first, then the new name, then the model ID as a last resort.
Added and removed posts shorten only the model ID. Every shortened non-empty
value ends in `…`. The original and final values of shortened fields are
logged to stderr.

### Risk: no retry on ambiguous failure

The script makes one attempt per request. Any transport or non-2xx response
stops the run without recording the incomplete delta in the ledger. This
prevents double-posting but means a failure mid-run leaves the ledger
partially updated. The next run will re-attempt unledgered deltas, which may
produce duplicate posts for deltas that were partially posted before the
failure.

### Initial rollout

The existing history contains 21 delta files. The 11 oldest deltas predate
provider-prefixed IDs and have been recorded as skipped in the initial
`state/posted.json`. The 10 newest deltas are eligible for broadcast. See
`docs/plans/2026-07-20-bluesky-broadcast.md` for the bootstrap plan.

## GitHub Actions

The workflows live at the monorepo root under `../.github/workflows/`, which
is the location GitHub Actions discovers. Both run their shell commands inside
`models-watch/`, so script and state paths remain local to this tool.

### Detect Model Updates (`detect.yaml`)

- Runs every 6 hours and on `workflow_dispatch`.
- `main` only, `contents: write`.
- Runs `models-watch.sh`, then commits and pushes changed
  `models-watch/state/latest.json` and `models-watch/state/change-*.json`
  files.

### Broadcast Model Updates (`broadcast.yaml`)

- Runs after a successful `Detect Model Updates` run (`workflow_run`) and
  on `workflow_dispatch`.
- Successful detection runs on `main`, or manual dispatches specifically on
  `main`, with `contents: write`.
- Exposes `BLUESKY_HANDLE`, `BLUESKY_APP_PASSWORD`, and `BLUESKY_PDS`
  secrets.
- Runs `models-broadcast.sh`, treating exit `3` (no eligible deltas) as
  a successful no-op.
- Commits and pushes `models-watch/state/posted.json` when it changes.

Both workflows share the `models-watch` concurrency group with
`cancel-in-progress: false` to serialise detection, posting, and ledger
commits.

## Testing

```bash
bash -n models-watch.sh
bash -n models-broadcast.sh
cd tests-rust && cargo test
```

Tests exercise all three scripts (`models-watch.sh`, `models-feed.sh`, and
`models-broadcast.sh`) as black-box commands using a Rust fluent acceptance
DSL. Broadcaster tests cover capture mode, live posting via `file://` PDS,
text truncation, delta validation, and credential checking.
