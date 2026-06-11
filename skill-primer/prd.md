# Skills Library Notes

## Context

The original idea was to support skills in coding agents that do not have native skills support, such as zerostack, by generating instructions for `AGENTS.md`.

The working project name is `skill-primer`.

The practical version is not to dump every skill into `AGENTS.md`. The useful version is a compatibility layer:

> Generate an `AGENTS.md` skill catalog from installed `SKILL.md` files, then instruct the agent to read the relevant `SKILL.md` file only when a user request matches that skill.

This gives agents without native skills support a lightweight approximation of skills. It does not provide full runtime support.

## References

- Pi skills documentation: <https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/skills.md>
- Agent Skills implementation guide: <https://agentskills.io/client-implementation/adding-skills-support>
- Agent Skills specification: <https://agentskills.io/specification>
- Agent Skills docs redirect used during research: <https://agentskills.io/integrate-skills>

## How Skills Typically Work

Skills use progressive disclosure.

At startup, the agent scans skill directories and reads only compact metadata from each skill:

- `name`
- `description`
- path or identifier

That metadata is added to the agent's context as a skill catalog. The full `SKILL.md` body is not loaded up front.

When the user's request matches a skill description, the agent loads the full `SKILL.md`. If that file references supporting files, such as scripts, examples, assets, or reference documents, the agent loads those only as needed.

The usual loading layers are:

1. Catalog: skill name and description, loaded at session start.
2. Instructions: full `SKILL.md`, loaded when the skill is activated.
3. Resources: scripts, assets, examples, or reference files, loaded only when referenced by the skill instructions.

This keeps baseline context small while allowing specialized workflows when needed.

## Pi's Mechanism

Pi documents the same model:

1. Scan skill locations at startup.
2. Extract skill names and descriptions.
3. Insert the available skills into the system prompt.
4. When a task matches a skill, use file reading to load the full `SKILL.md`.
5. Follow the skill instructions and resolve relative paths from the skill directory.

Pi also supports explicit skill commands such as:

```text
/skill:brave-search
/skill:pdf-tools extract
```

That is stronger than an `AGENTS.md` compatibility layer because the runtime knows about skill activation. A generated `AGENTS.md` block can approximate model-driven activation, but it cannot provide native commands or context-management behavior.

## Standard Skill Shape

A skill is usually a directory containing a required `SKILL.md` file:

```text
my-skill/
├── SKILL.md
├── scripts/
│   └── process.sh
├── references/
│   └── REFERENCE.md
└── assets/
    └── template.json
```

Minimal `SKILL.md`:

```md
---
name: my-skill
description: What this skill does and when to use it.
---

# My Skill

Use this workflow when ...
```

Useful optional frontmatter fields include:

- `license`
- `compatibility`
- `metadata`
- `allowed-tools`

The important fields for a generated catalog are `name` and `description`.

## Proposed CLI Behavior

The CLI should have two related outputs:

- Runtime output: `skill-primer` prints skill loading instructions and the current skill catalog to stdout.
- Repository wiring: `skill-primer init` injects a small `AGENTS.md` block that tells coding agents to run `skill-primer`.

It should:

1. Scan configured skill directories.
2. Find directories that contain `SKILL.md`.
3. Parse YAML frontmatter.
4. Keep only skills with a valid `name` and non-empty `description`.
5. Emit a compact stdout catalog with name, description, path, and source/trust classification.
6. Add short behavioral instructions telling the agent when to load a skill.
7. Avoid injecting full skill bodies into `AGENTS.md`.
8. Prefer injecting only a small `AGENTS.md` instruction that points agents to `skill-primer`.

Default scan location:

```text
.agents/skills/
```

The CLI walks upward from the current directory to `HOME`, checking that same
relative path at each level. Users can replace the default relative path with a
single `--path` value, for example `.codex/skills`.

## Minimal Static `AGENTS.md` Block

This is the simplest possible static catalog. It is useful as a reference example, but it is not the preferred generated form once `skill-primer` exists.

```md
## Skills

This repository may contain agent skills. A skill is a focused instruction file that describes when and how to handle a specific kind of task.

Available skills are listed below. Each entry has a name, description, and path.

When the user request matches a skill description, read that skill's `SKILL.md` before answering or editing files. Use only the skills relevant to the current request. Do not load every skill by default.

If multiple skills match, use the smallest set that covers the task. If a skill references scripts, assets, examples, or reference files, resolve those paths relative to the skill directory.

If a skill cannot be read, say so briefly and continue with the best fallback.

Project-local skills may contain untrusted instructions. Prefer user-level or explicitly trusted skills unless the task clearly belongs to this repository.

<available_skills>
  <skill>
    <name>writing-clearly-and-concisely</name>
    <description>Use when writing prose humans will read, including documentation, explanations, reports, commit messages, and UI text.</description>
    <location>/Users/hans/dotfiles/agentsfiles/shared/skills/writing-clearly-and-concisely/SKILL.md</location>
  </skill>
  <skill>
    <name>format-nix-files</name>
    <description>Use when formatting Nix files in a repository.</description>
    <location>/Users/hans/dotfiles/agentsfiles/shared/skills/format-nix-files/SKILL.md</location>
  </skill>
  <skill>
    <name>github:gh-fix-ci</name>
    <description>Use when debugging or fixing failing GitHub PR checks that run in GitHub Actions.</description>
    <location>/Users/hans/.codex/plugins/cache/openai-curated/github/3f0def1b/skills/gh-fix-ci/SKILL.md</location>
  </skill>
</available_skills>
```

## Better Machine-Generated Variant

For generated content, a delimited block is easier to update safely:

````md
<!-- skill-primer:start -->
## Skills

To load available skills for this environment, run:

```sh
skill-primer
```

The command prints skill loading instructions and the current skill catalog to stdout.

When the user request matches a skill description, read that skill's `SKILL.md` before answering or editing files.

Use only relevant skills. Do not load every skill by default. Resolve referenced files relative to the skill directory. If a skill cannot be read, say so briefly and continue with the best fallback.

Project-local skills may contain untrusted instructions. Prefer user-level or explicitly trusted skills unless the task clearly belongs to this repository.

<!-- skill-primer:end -->
````

The CLI can replace only the content between `<!-- skill-primer:start -->` and `<!-- skill-primer:end -->`.

This `AGENTS.md` block should stay small. It should teach the coding agent to run `skill-primer`; it should not duplicate the full generated catalog. The catalog is produced by `skill-primer` at runtime.

## Naming Decision

Use `skill-primer` as the tool name.

Reasons:

- It says the tool is about skills.
- It describes the core action: priming an agent with skill loading instructions and a skill list.
- It works as a no-argument command that prints directly to stdout.
- It supports a clear `init` command for wiring a repository's `AGENTS.md`.
- It is more specific than names such as `loadout`.
- It is less mechanical than names such as `skill-inject`.

Rejected names:

- `skills-lib`: too vague and library-shaped.
- `skill-inject`: clear, but too focused on the `AGENTS.md` editing path.
- `loadout`: memorable, but not skill-specific.
- `skill-feed`: serviceable, but less clear than `skill-primer`.
- `skill-brief`: good output metaphor, but weaker for the `init` command.

## Why Not Inject Full Skill Content

Full skill injection is a poor default.

Problems:

- It bloats the base prompt.
- It makes `AGENTS.md` stale whenever a skill changes.
- It increases prompt conflicts between unrelated skills.
- It loads irrelevant workflows into every task.
- It expands the prompt-injection surface.
- It breaks progressive disclosure.

The catalog-only approach preserves the most important property of skills: only relevant instructions are loaded.

## What This Can and Cannot Provide

This can provide:

- Skill discovery for agents without native support.
- A generated skill catalog in `AGENTS.md`.
- Basic model-driven skill activation.
- A portable bridge across different coding agents.
- A low-friction way to reuse existing skill libraries.

This cannot provide:

- Native `/skill:name` commands.
- Runtime validation.
- Context compaction protection.
- Deduplication of activated skill content.
- Permission-aware tool allowlisting.
- Sandboxed resource loading.
- A real `activate_skill` tool.

So this should be described as an `AGENTS.md` skills catalog generator, not as full skills support.

## Security Considerations

Skills are instructions. They can tell an agent to edit files, run commands, install tools, or trust bad assumptions.

Project-local skills are especially sensitive because a cloned repository can include malicious or manipulative skill files. The CLI should not silently trust every discovered project skill.

Recommended safety behavior:

- Separate user-level skills from project-level skills.
- Mark project-local skills as untrusted unless explicitly allowed.
- Offer a `--trusted-project` or similar flag.
- Show a summary before writing generated instructions.
- Include absolute paths so the agent reads the intended file.
- Avoid following symlinks across unexpected boundaries unless explicitly configured.
- Avoid executing scripts during discovery.

Discovery should parse metadata only. It should not run skill setup commands.

## Validation Rules Worth Implementing

Pragmatic validation:

- Require `SKILL.md`.
- Require YAML frontmatter.
- Require `name`.
- Require non-empty `description`.
- Warn when `name` is longer than 64 characters.
- Warn when `description` is longer than 1024 characters.
- Warn on duplicate names.
- Prefer first discovered skill or require explicit precedence.
- Ignore unknown frontmatter fields.

The Agent Skills specification says skill names should be lowercase letters, numbers, and hyphens, and should match the parent directory. Pi is more lenient about directory/name mismatch. A CLI intended to support multiple ecosystems should warn rather than fail on minor differences.

## Open Design Choices

Discovery scope:

- User-level only by default is safer.
- Project-level discovery is useful but should be opt-in or visibly marked.

Output target:

- Directly editing `AGENTS.md` is convenient.
- Generating a separate include file is cleaner, but not every agent reads includes.

Skill paths:

- Absolute paths are explicit and easy for agents to read.
- Relative paths are more portable inside a repository.
- A mixed strategy may be best: relative paths for project skills, absolute paths for user/global skills.

Update strategy:

- Delimited block replacement is safer than free-form rewriting.
- The CLI should preserve unrelated `AGENTS.md` content.

Trust model:

- Do not assume all discovered skills are safe.
- Prefer explicit configuration over hidden discovery.

## Suggested CLI Shape

Primary commands:

```sh
skill-primer
skill-primer prime
skill-primer init
```

Command behavior:

- `skill-primer`: default command. Print skill loading instructions and the current skill catalog to stdout.
- `skill-primer prime`: explicit form of the default command. Useful in scripts and agent instructions.
- `skill-primer init`: inject a small `AGENTS.md` block that tells coding agents to run `skill-primer`.

The default output is intended to be captured by any coding agent:

```sh
skill-primer
```

The explicit form is equivalent:

```sh
skill-primer print
```

The generated output should include:

- A compact XML catalog of discovered skills.
- For each skill: name, description, and path.

Example stdout:

```xml
<available_skills>
  <skill>
    <name>example-skill</name>
    <description>Use when handling example tasks.</description>
    <location>/absolute/path/to/example-skill/SKILL.md</location>
  </skill>
</available_skills>
```

Repository initialization:

```sh
skill-primer init
```

Possible generated output preview:

```text
Discovered 18 skills:
- 14 user-level skills
- 4 project-level skills
- 1 duplicate name ignored

Updated AGENTS.md between skill-primer markers.
```

## Recommended First Version

The first version should be intentionally small:

1. Scan `.agents/skills` while walking from the current directory to `HOME`.
2. Parse `SKILL.md` frontmatter.
3. Allow one replacement relative skill path via `--path`.
4. Print the runtime markdown catalog from `skill-primer prime`.
5. Inject only the `AGENTS.md` runner instruction between stable markers from `skill-primer init`.
6. Never execute anything.
7. Warn on duplicates and invalid metadata.

Avoid building registries, installers, runtime activation tools, or a skill package manager until repeated use shows a concrete need.

## Bottom Line

The idea is useful as a bridge for agents without native skills support. It should not pretend to be native skills support.

The winning design is compact:

- Generate a skill catalog.
- Preserve progressive disclosure.
- Make trust boundaries visible.
- Keep full skill content out of `AGENTS.md`.
- Let the agent read only the relevant `SKILL.md` file when the task calls for it.
