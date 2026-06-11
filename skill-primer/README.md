# skill-primer

Print a compact skill catalog for coding agents that do not have native skill
support.

## Usage

```sh
skills-primer prime
skills-primer ls
skills-primer config
```

`prime` suppresses non-fatal warnings by default because its output is meant
to be embedded into agent instructions. `ls` and `config` show warnings by
default. Use `--warnings` or `--no-warnings` to override the default for any
subcommand:

```sh
skills-primer prime --warnings
skills-primer ls --no-warnings
```

By default, `skills-primer` walks from the current directory up to `HOME` and
looks for one relative skill path at each level:

```text
.agents/skills
```

Use `--path` to choose a different single relative skill path for that same
walk:

```sh
skills-primer ls --path .codex/skills
skills-primer prime --path .codex/skills
```

`--path` accepts one relative path only. It replaces the default
`.agents/skills` path rather than adding another search directory.

Agents should report which skills are available in this format:

```text
*Available skills:* skill-a, skill-b, ...
```

## Skill Format

```md
---
name: my-skill
description: What this skill does and when to use it.
---

# My Skill

Use this workflow when ...
```
