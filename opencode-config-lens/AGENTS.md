# AGENTS.md

OpenCode Config Lens - A CLI for viewing configured agents and their models.

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

- `src/` - Source code (TUI runtime, report rendering, config loading)
- `tests/` - Integration and acceptance tests
- `docs/` - Documentation and screenshots

## Workflow

1. Make changes
2. Run `mise run check` to catch warnings
3. Run `mise run test` to verify behavior
4. Fix any failures immediately

## Task Management

Use Beads for task tracking. Invoke the using-beads-for-task-management skill.
