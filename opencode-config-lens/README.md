# OpenCode Config Lens

A fullscreen TUI for inspecting OpenCode model configuration and usage.

## What It Does

Scans your `~/.config/opencode/` configuration (both `opencode.jsonc` and `weave-opencode.jsonc`), fetches current model pricing from [models.dev](https://models.dev), and displays a sortable table showing which models are actively configured versus available.

**Problem it solves:** I was losing track of which models were assigned to which agents across multiple config files. This gives me a single view of model usage, cost per 1M tokens, and where each model is referenced.

![Screenshot](docs/screenshot.png)

## Origin

This project was fully vibe-coded — my first experiment with building an actual TUI application using [ratatui](https://github.com/ratatui-org/ratatui). It's seemingly overkill compared to a simple CLI table, but I wanted to explore the TUI paradigm. Pleased with the result; the fullscreen interface with keyboard controls and live status feels surprisingly natural.

## Usage

```bash
# Run with cargo
cargo run

# Or the release binary
cargo run --release

# Use an alternate config directory
ocl --home-dir /path/to/config
```

**Controls:**
- `q` — quit
- `r` — refresh model data
- `s` — cycle sort modes (active-first, cost-asc, cost-desc, model-name)

## Dev

This project uses [mise](https://mise.jdx.dev/) for task running. Tasks are defined in the parent directory's `.mise.toml`:

| Task | Description |
|------|-------------|
| `mise run run` | Run the application in dev mode |
| `mise run release` | Build an optimised release binary |

```bash
# From the repo root
mise run run
mise run release
```
