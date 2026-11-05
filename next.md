# Set up TinyCLI development publication and Musonius backup

Implement the accepted decision in
[ADR-001](docs/adr/001-separate-development-from-live-models-watch.md).
Read root `AGENTS.md` first and print its branch-policy notice.

## Goal and limits

Establish `dev` for normal development, publish it to Codeberg and Musonius,
and preserve wanted divergent work. Leave GitHub's live `main`, workflows,
and state untouched. Do not strip tools or migrate state. Keep this a small
setup task, not a general history cleanup.

Use Jujutsu for local repository operations. Load the applicable Jujutsu and
Musonius skills. Use the Musonius skill for volume checks, remote creation,
tracking, first-push ordering, and read-back verification, with one explicit
exception: ADR-001 requires excluding GitHub from ordinary pushes. Do not copy
the skill's all-existing-remotes push policy. Publish `dev` and selected safety
bookmarks, not `main` by default.

## Procedure

1. Inspect `jj st --no-pager`, all bookmarks, remotes, repo push configuration,
   and relevant history. Fetch GitHub and Codeberg to refresh observations;
   fetching is allowed, pushing to GitHub is not. Check for conflicts caused by
   fetch before proceeding. Do not assume the old hand-off's commit IDs are
   current. The earlier hand-off in the dotfiles repo is background only:
   `docs/plans/2026-11-04-tiny-cli-strip-to-models-watch.md`; its strip procedure
   is not the approved direction.
2. Before changing ancestry or bookmark targets, preserve the divergent local
   main tip and current working-copy revision with descriptive safety bookmarks.
   Record their full commit IDs. Do not discard the working-copy state changes.
   Use additional bookmarks if inspection shows other work needs preservation.
3. Inspect candidate development tips. The working-copy parent previously held
   a newer usage-watch fix absent from the older local main line; neither line
   automatically contains all wanted work. Recommend the smallest useful `dev`
   starting point. Ask one focused question if selecting it would exclude work
   whose intended place cannot be inferred. Keep other work on safety bookmarks;
   do not merge or rebase all lines just to complete the backup.
4. Create `dev` at the selected, described revision. Preserve this ADR,
   `AGENTS.md` changes, and `next.md` on the selected development line even if it
   differs from the starting working copy. Keep unrelated state changes separate
   from documentation/setup commits. Resolve local `main` to the freshly fetched
   `main@github` only after its other targets are preserved. This is a local
   bookmark change, not permission to publish `main` anywhere.
5. Dry-run and publish a new `dev` branch to Codeberg (`origin`). Do not replace
   Codeberg's existing `main`, change default branches, or force push. If `dev`
   already exists remotely, inspect direction and stop on unexpected divergence.
6. Add or inspect the Musonius remote. Confirm the external volume is mounted
   before creating a bare repository. Back up `dev` and the explicitly selected
   safety bookmarks. Do not publish unrelated branches. Stop on mount, SSH,
   repository-identity, or divergence problems rather than repairing the host.
7. Set repo-local `git.push` to `["origin", "musonius"]`. Track the intended
   bookmark/remote pairs. Use explicit `--bookmark dev` for ordinary development
   pushes and confirm with a dry run that GitHub is excluded. Safety bookmarks
   need not be published to Codeberg. Do not use a combined unfiltered push that
   could publish the local service `main` or unrelated bookmarks.
8. Review `AGENTS.md` against ADR-001. It must link the ADR, print the notice once
   at startup, distinguish accepted policy from pending setup, and cover code,
   workflow, and state maintenance on GitHub. Keep it short. Mark the ADR setup
   as implemented only after all verification criteria pass; record any pending
   work honestly if setup is blocked.

## Verification and report

- Read back Codeberg `dev` and Musonius bare repository refs. Compare full commit
  IDs with local `dev` and each safety bookmark selected for backup. Push output
  alone is not proof of backup.
- Verify Codeberg's old `main` was not replaced and no command pushed to GitHub.
  GitHub's bot may independently advance `main`; that is expected.
- Verify documentation links and review the scoped diff. Run task-specific checks
  for any scripts or configuration changed; do not run the live broadcaster.
- Report the chosen development tip, preserved bookmark IDs, verified remote
  IDs, push policy, and any blockers. State explicitly that a `dev` backup is
  not a backup of the latest GitHub ledger.

Do not expand this task into an automated live-state backup. Do not delete
`next.md` as part of setup unless asked.
