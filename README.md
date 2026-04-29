# tiny-cli

**⚠️ Warning:** These are experimental tools I created to use myself but with **no warranty**.

A collection of small, standalone command-line tools and scripts.

- [`cpu-watch`](cpu_watch) – a Bash helper that polls `ps` output, alerts when matching processes exceed a `%CPU` threshold, and prints regular status lines between alerts; configurable pattern, threshold, and interval flags make it easy to watch different workloads without working in `top`.

- [`skill-sync`](skill-sync) – syncs selected local skills into `.agents/skills` based on configured source roots; supports discovery (`--list-all`), machine-readable output (`--json`), and no-op preview (`--dry-run`).

- [`models-watch`](models-watch) – polls `https://models.dev/api.json`, extracts the `opencode-go` model block, and notifies when models are added or removed; writes JSON delta files only on change and supports `osascript` popups or `--notify-file` for scripting.
