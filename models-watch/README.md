# models-watch

Detect when [opencode-go](https://opencode.ai/docs/zen) models change on [models.dev](https://models.dev).

## Usage

```bash
./models-watch.sh [--notify-file <path>]
```

- Without `--notify-file`: fires a macOS notification popup via `osascript` when models change.
- With `--notify-file <path>`: writes the notification message to `<path>` (for scripting and testing).

## What it does

1. Fetches `https://models.dev/api.json`.
2. Extracts the `opencode-go` provider block.
3. Compares model IDs against the last snapshot (`state/latest.json`).
4. On first run or when models are added/removed, writes a JSON delta file (`state/change-<timestamp>.json`).
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
| `state/latest.json` | Most recent snapshot (used for comparison on next run) |
| `state/change-<timestamp>.json` | Delta file, written only when models are added or removed |

## Testing

```bash
cd tests-rust && cargo test
```

Tests exercise `models-watch.sh` as a black-box command using a Rust fluent acceptance DSL.
