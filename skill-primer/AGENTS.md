
## Code

- This is a simple single purpose CLI tool, the code should reflect that. Refactor and create code that is clear and simple.

## Fluent DSL Test Pattern (Type-State Builder)

For integration/CLI tests, use a **type-state builder** that chains setup → action → assertion in a single expression.

### Structure

Three phases, each a distinct struct enforcing valid chains at compile time:

1. **Setup phase** — `Foo::given()` creates a guard (owns resources via `Drop`)
2. **Action phase** — methods consume the guard, build input, then `.when_run()` / `.execute()` transitions to result
3. **Assert phase** — `.should_succeed()`, `.expect_output("...")` consume the result, panic on mismatch

### Rules

- Methods consume `self` (not `&self`) to enforce linear chains
- Action phase returns a new result struct — `when_run()` is the single transition point
- Guard owns temp dirs, daemon processes, etc. — cleanup via `Drop`, never explicit teardown
- `expect_` / `should_` methods return `Self` so assertions compose
- Keep everything synchronous, blocking, deterministic — no async or combinators
- Each test gets a fresh guard instance — no shared state
- If adding a variant code path (e.g. re-import), add a new struct phase rather than branching

### Example (generic)

```rust
Foo::given()
    .arg("x").flag("--verbose")
    .when_run()
    .should_succeed()
    .expect_output("done")
    .expect_exit_code(0);
```

<!-- skills-primer BEGIN-->
## Agent skills integration

Run `cargo run -- --include ~/.agents/skills prime` for to integrate agent skills in this project. Follow those instructions before proceeding with any task. Report available skill in format:

```
*Available skills:* skill-a, skill-b, etc.
```
<!-- skills-primer END-->
