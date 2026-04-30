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

Run every 4 hours:

```cron
0 */4 * * * /path/to/models-watch/models-watch.sh
```

## State

All runtime state lives under `state/` relative to the script:

| File | Purpose |
|------|---------|
| `state/latest.json` | Synthetic merged snapshot of watched models, keyed by provider-prefixed model ID (used for comparison on next run) |
| `state/change-<timestamp>.json` | Delta file, written when models are added, removed, or renamed |

## Testing

```bash
cd tests-rust && cargo test
```

Tests exercise `models-watch.sh` as a black-box command using a Rust fluent acceptance DSL.
