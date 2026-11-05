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
The decision is accepted, but setup may still be pending. Inspect actual
bookmarks and configuration; do not assume `dev` exists.

At session start, print this notice once:

> Branch policy: use `dev` for normal work and publish it to Codeberg and
> Musonius. Use GitHub `main` only for deliberate live models-watch maintenance.
> Never replace its state with a development copy. See ADR-001.

- Normal work belongs on `dev`; GitHub is not an ordinary push destination.
- Live maintenance includes models-watch code, workflows, and necessary state
  repair. Base it on freshly fetched `main@github`, never on `dev`.
- Preserve live state for code-only fixes. Review state repairs explicitly.
  Never run the live broadcaster as a test.
- If setup is pending, follow `next.md` only when asked to perform the setup.
  Do not resolve existing conflicts or move bookmarks merely to start a session.
