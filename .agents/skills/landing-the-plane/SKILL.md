---
name: landing-the-plane
description: A structured session closure workflow to ensure code quality, Beads task completion, and clean handoffs. Use this before closing any task or ending a session.
version: 0.1.0
---

# Landing the Plane

A reusable, tool-agnostic closure workflow. This skill guides the agent through logical "quality gates" before finalizing work. It relies on project-level instructions (e.g., AGENTS.md) or domain-specific skills (e.g., use-jujutsu, verification-before-completion) for actual execution.

## The Quality Gate Ladder

Follow these steps in order. Do not skip gates unless explicitly instructed by the user or project rules.

### Step 1: Issue Sweep
Check for any discovered follow-up work or remaining blockers.
- Run `bd ready` to see available work.
- Create new issues for any technical debt or follow-up tasks discovered during this work.

### Step 2: VCS Scope Review
Verify that the current changes are atomic and intentional.
- Delegate to the relevant VCS (Jujutsu, Git, ...) skill to review open changes.
- Ensure no sensitive files (secrets, local config) are staged.

### Step 3: Verification Gate
Confirm that all quality gates (tests, lints, builds) are passing.
- **Rule:** Do not guess commands. Refer to the project's AGENTS.md, README.md.
- Confirm that all identified gates have passed. **Never close a task with failing gates.**

### Step 4: Final Commit
Ensure all changes are persisted with clear context.
- Commit changes using the project's VCS.
- **Rule:** Include the task ID in the commit message body for traceability.

### Step 5: Remote Sync (Optional)
Only sync with remote if explicitly requested by the user.
- Perform necessary sync operations.
- Use `bd dolt push`/`bd dolt pull` for remote sync
- Verify a clean working state after sync.

### Step 6: Task Closure
Mark the work as finished in the task tracker.
- **Pre-close checkpoint:** Do not close tasks if requirements are still changing, unresolved uncertainty remains, or the user is still deciding direction.
- Require an explicit closure-ready signal from the conversation before running `bd close`.
- Use `bd close <id> --reason "<summary>"` for all completed tasks.

### Step 7: Session Handoff
Provide a clear summary for the next session.
- Document exactly what was completed.
- State which verification gates passed.
- List any newly created follow-up tasks.

## Critical Rules

- **No failing gates:** Never close a task if tests, lints, or builds are failing.
- **Atomic Commits:** Keep changes focused on the task at hand.
- **Evidence First:** Report the exact gate commands that were run and their results during handoff.
- **No premature closure:** Keep tasks in progress while scope/intent is still being negotiated.
