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
| `--output <file>` | `state/feed.xml` | Where to write the RSS feed. Also settable via `MODELS_WATCH_FEED_FILE`. |
| `--state-dir <dir>` | sibling `state/` | Directory containing `change-*.json` deltas. Also settable via `MODELS_WATCH_STATE_DIR`. |

### Feed format

- RSS 2.0, XML 1.0, UTF-8
- One `<item>` per change event (delta), newest first
- Window: last 100 deltas
- Item `<title>` summarises add/remove/change counts
- Item `<description>` lists affected model IDs with `<br/>` line breaks
- Item `<guid>` is `models-watch-<ISO timestamp>` (`isPermaLink="false"`)
- Item `<pubDate>` is the delta timestamp in RFC-822 format
- Model IDs in descriptions are wrapped in `<![CDATA[...]]>` for safe XML

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Feed written (or no deltas, no error) |
| 2 | Unknown flag |
| 3 | No change deltas exist (nothing to publish) |

### Publishing to Codeberg Pages

`models-watch` lives inside the `tiny-cli` monorepo, so the Codeberg Pages
project URL is `https://<your-org>.codeberg.page/tiny-cli/`.

To make the feed publicly available:

1. Create a `pages` branch (or configure Pages to serve from a specific
   directory in your default branch).
2. Run both scripts from the **repo root**, with `--output` pointing to a
   repo-root-relative path so the feed URL is flat:

   ```bash
   cd /path/to/tiny-cli
   ./models-watch/models-watch.sh
   ./models-watch/models-feed.sh --output models-watch.xml
   ```

3. Commit the feed file and push to the `pages` branch:

   ```bash
   # With Jujutsu
   jj new
   jj bookmark create pages
   jj describe -m "chore: update RSS feed"
   jj push --remote origin --bookmark pages

   # With Git
   git checkout pages
   git add models-watch.xml
   git commit -m "chore: update RSS feed"
   git push origin pages
   ```

The feed will be available at `https://<your-org>.codeberg.page/tiny-cli/models-watch.xml`.

## Testing

```bash
cd tests-rust && cargo test
```

Tests exercise both `models-watch.sh` and `models-feed.sh` as black-box
commands using a Rust fluent acceptance DSL.
