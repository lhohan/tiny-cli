# AGENTS.md

usage-watch - a CLI that shows remaining usage for AI subscriptions (OpenCode Go, ChatGPT Go/Plus, Mistral, Vibe), reading credentials from environment variables.

## Verification Commands

Run these to verify changes:

```bash
# Check formatting, clippy, and warnings
mise run check

# Run all tests
mise run test

# Build release binary
mise run release
```

**Always run `mise run check` before claiming work is done.**

## Project Structure

- `src/` - Source code (CLI args, HTTP client, time helpers, provider modules, rendering)
- `tests/` - CLI behavior and provider response-parser tests
- `README.md` - Usage docs and the per-provider environment variables

## Workflow

1. Make changes
2. Run `mise run check` to catch warnings
3. Run `mise run test` to verify behavior
4. Fix any failures immediately
