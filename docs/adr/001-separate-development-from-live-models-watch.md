# ADR-001: Separate development from live models-watch

## Status

Accepted. Bookmark, remote, and push-policy setup is applied.

## Context

TinyCLI contains several tools. GitHub Actions runs models-watch from GitHub's
`main` and writes model snapshots, change files, and the posting ledger back to
that branch. Local development and Codeberg have diverged from the bot's line.
The local `main` bookmark is conflicted.

The posting ledger prevents historical deltas from being posted again to
Bluesky. Replacing it with an older development copy can cause duplicate posts.
We need safe development publication and a Musonius backup without changing the
running service or spending time on a state-storage migration.

## Decision

- Keep GitHub's branch named `main`, with its existing workflows and state.
  Treat it as the live models-watch service branch, not the general development
  branch. Do not change GitHub's default branch as part of this setup.
- Use a local `dev` bookmark for normal development. Publish `dev` to Codeberg
  (`origin`) and Musonius (`musonius`). Initially create a new Codeberg branch;
  do not replace its existing `main` or change its default branch.
- Preserve the divergent local work and current working-copy revision with
  explicit safety bookmarks before selecting a development tip or resolving
  the local `main` conflict. Push the selected preservation bookmarks to
  Musonius explicitly; a local bookmark alone is not a backup.
- Configure ordinary development pushes for Codeberg and Musonius only.
  Exclude GitHub from `git.push`; use an explicit `dev` bookmark selection.
  Inspect a dry run before the first publication.
- GitHub may receive `dev` as an explicit, one-off publication
  (`jj git push --remote github --branch dev`). It is not an ordinary push
  destination: `git.push` excludes GitHub, so no default push can reach it, and
  the live `main` branch is never a target.
- Change GitHub `main` only for deliberate live models-watch maintenance:
  script fixes, workflow fixes, or necessary state repair. Fetch its latest tip
  and base the maintenance change on `main@github`, not on `dev` or an old local
  copy. Do not merge the whole development line into the live service branch.
- Preserve live state during code-only maintenance. State repair requires a
  specific reason and review of the proposed state diff. Never run the live
  broadcaster merely to test a change.
- If the bot advances while preparing maintenance, stop, incorporate the new
  tip, and repeat verification. A fetch alone does not update the prepared
  maintenance commit. Do not force push.
- Keep the existing state snapshot on the development line initially. Do not
  merge ongoing bot-state updates into `dev`. This means no ongoing state churn,
  not state-free history. Development state is not authoritative for the feed.
- Do not strip tools, rename the GitHub branch, move state, rewrite history, or
  reconcile all existing lines as part of this setup.

The root `AGENTS.md` must reference this ADR and instruct agents to print a short
branch-policy notice once at session start. This notice is guidance, not a
technical push guard. Agents must inspect actual bookmarks and configuration
rather than assuming the described state.

## Rationale and alternatives

Separating publication destinations avoids disturbing the working feed and
avoids replacing Codeberg's divergent `main`. Naming development `dev` makes
its purpose distinct from GitHub's live `main`.

Stripping TinyCLI to models-watch addresses repository cleanup, not the immediate
publication and backup problem. Moving state to a dedicated branch could reduce
future coupling, but requires workflow changes and a controlled migration. Both
are deferred. Keeping every remote as a mirror would require reconciling the
histories and would retain the bot-state churn in normal development.

## Consequences

- Codeberg and GitHub intentionally carry different lines; they are not mirrors.
  GitHub also carries an explicit `dev` copy for visibility; `main` remains the
  live service branch.
- Models-watch fixes may need selective application to both lines. The GitHub
  version must always be prepared against its latest live state.
- Existing tools and historical state remain in the repository.
- Musonius is one backup copy. Backing up `dev` does not back up the latest
  GitHub-generated ledger. A live-state backup needs a separate, explicit
  refresh and verified publication; automation for that is deferred.
- Existing Codeberg `main` remains available. Changing the default branch is a
  separate decision, not required for safe publication of `dev`.

## Verification

Setup is complete only when:

1. `dev` points at the deliberately selected development revision, and wanted
   divergent work is preserved.
2. Codeberg `dev` matches the selected local `dev`; its existing `main` was not
   replaced.
3. Musonius is on the mounted external volume. Reading back its bare repository
   refs confirms full commit IDs for `dev` and each selected safety bookmark.
4. Ordinary development push configuration excludes GitHub. A dry run for `dev`
   targets only Codeberg and Musonius.
5. No setup operation writes GitHub branches, workflows, or live state. Bot
   advancement on GitHub is expected and is not a setup change.
6. `AGENTS.md` links this ADR and includes the startup notice and maintenance
   exception.
