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

## Testing

```bash
cd tests-rust && cargo test
```

Tests exercise both `models-watch.sh` and `models-feed.sh` as black-box
commands using a Rust fluent acceptance DSL.
