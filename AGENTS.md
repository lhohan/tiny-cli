# Repository Guidelines

## Project Structure & Module Organization

This repository contains small standalone CLI tools.

Keep each tool self-contained in its own directory with its script, README, and any sidecar data files.

- `README.md`: top-level index of available tools.

## Version control guidelines

Detect if Jujutsu is available. When both Git and Jujutsu are available, use Jujutsu (`jj`). Use Git only if explicitly requested.

## Branch and publication policy

Read [ADR-001](docs/adr/001-separate-development-from-live-models-watch.md) before
bookmark changes, push configuration, publication, or live models-watch work.
The decision is accepted and setup is applied. Inspect actual bookmarks and
configuration rather than assuming the described state.

At session start, print this notice once:

> Branch policy: use `dev` for normal work and publish it to Codeberg and
> Musonius. GitHub `main` is the live models-watch service; change it only for
> deliberate maintenance, never from a development copy. GitHub may hold an
> explicit `dev` copy, but it is excluded from ordinary pushes. See ADR-001.

- Normal work belongs on `dev`; GitHub is not an ordinary push destination.
  Publishing `dev` there is an explicit one-off, never part of `git.push`.
- Live maintenance includes models-watch code, workflows, and necessary state
  repair. Base it on freshly fetched `main@github`, never on `dev`.
- Preserve live state for code-only fixes. Review state repairs explicitly.
  Never run the live broadcaster as a test.
- If setup is pending, follow `next.md` only when asked to perform the setup.
  Do not resolve existing conflicts or move bookmarks merely to start a session.
