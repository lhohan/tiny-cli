# Plan: Introduce Clap for Argument Parsing

## Objective
Replace manual argument parsing with the `clap` crate for a more robust and maintainable CLI interface.

## Tasks

### Task 1: Add Clap Dependency
**Files:**
- Modify: `Cargo.toml`

**Purpose:** Add clap with derive feature for declarative argument parsing.

**Expected Result:**
- `clap` dependency added with `derive` feature
- Dependencies compile successfully

### Task 2: Refactor CLI Parsing with Clap
**Files:**
- Modify: `src/main.rs:1-160`

**Purpose:** Replace manual `Cli::parse()` implementation with clap's derive macros.

**Expected Result:**
- Define `#[derive(Parser)]` struct for CLI arguments
- Define `#[derive(Subcommand)]` enum for commands
- Remove manual parsing logic
- Maintain same CLI interface (`skill-primer prime --include DIR`)
- All existing tests continue to pass

## Verification
- `cargo test` - all 6 tests pass
- `cargo clippy` - no warnings
- `cargo build` - compiles successfully
