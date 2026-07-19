# Plan: mise task to generate feed and publish to pages bookmark

## Goal

Create a `publish-feed` mise task at the **repo root** that runs the full pipeline: detect model changes, regenerate the RSS feed, and publish to the `pages` bookmark on Codeberg. The flow: `models-watch.sh` → `models-feed.sh` → hash check → `jj commit` + `jj bookmark set pages` + `jj git push`.

## Assumptions

- **Repo layout**: `models-watch/` is a subdirectory of the `tiny-cli` monorepo. The feed output file `models-watch.xml` lives at the repo root (tracked). `models-watch/state/` is gitignored.
- **JJ bookmarks**: `main` stays at the code commits. `pages` is on the **same chain** as `main`, ahead by a few commits (feed/docs fixes). The working copy `@` sits on top of `pages`. Verified: `pages` is at `@-`, working copy is on top.
- **No workspace needed**: Since `pages` is on the same chain, no `jj workspace` is required. The script works directly from the main working copy.
- **Feed URL**: Hardcoded as `https://hanlho.codeberg.page/tiny-cli/models-watch.xml` (per user decision).
- **Skip on no change**: If `models-watch.xml` is unchanged after regeneration, the script exits 0 without committing or pushing. Detection uses `shasum -a 256` (available on macOS via `/usr/bin/shasum`, on Linux via `sha256sum`).
- **models-watch.sh state**: `models-watch.sh` writes deltas to `models-watch/state/` (gitignored). The script resolves its own state directory via `SCRIPT_DIR`, so it works regardless of cwd. No new deltas = no feed content change = skip.
- **Network failures**: If `models-watch.sh` fails (network error, exit non-zero), the script aborts before publishing. `set -euo pipefail` ensures this.
- **Remote**: `origin` is `ssh://git@codeberg.org/hanlho/tiny-cli.git` (verified).

## Verified jj command semantics

- `jj commit -m "msg" <filesets>`: Selected paths stay in `@` (described with msg), remaining changes go to new `@` on top. Does NOT move bookmarks. After this, `@-` is the commit with the feed changes.
- `jj bookmark set pages -r @-`: Moves `pages` forward to `@-` (the feed commit). Forward move — no `--allow-backwards` needed.
- `jj git push --remote origin --bookmark pages`: Pushes the pages bookmark to Codeberg.

## Plan

1. **New file `models-watch/publish-feed.sh`** (`#!/usr/bin/env bash`, `set -euo pipefail`):
   - Resolves repo root as `SCRIPT_DIR/../`.
   - Hardcoded `FEED_URL="https://hanlho.codeberg.page/tiny-cli/models-watch.xml"`.
   - `cd` to repo root so all paths are relative.
   - Hash helper: `shasum -a 256 "$1" | cut -d' ' -f1` (macOS-compatible; `sha256sum` as fallback).
   - Flow:
     1. `hash_before=$(hash_file models-watch.xml 2>/dev/null || echo "")`
     2. `./models-watch/models-watch.sh` (detect changes, write deltas; aborts on failure via `set -e`)
     3. `./models-watch/models-feed.sh --output models-watch.xml --feed-url "$FEED_URL"` — catch exit 3 (no deltas) and treat as skip:
        ```bash
        set +e
        ./models-watch/models-feed.sh --output models-watch.xml --feed-url "$FEED_URL"
        feed_exit=$?
        set -e
        if [[ $feed_exit -eq 3 ]]; then
            echo "No deltas found, nothing to publish."
            exit 0
        elif [[ $feed_exit -ne 0 ]]; then
            echo "ERROR: models-feed.sh failed (exit $feed_exit)" >&2
            exit $feed_exit
        fi
        ```
     4. `hash_after=$(hash_file models-watch.xml 2>/dev/null || echo "")`
     5. If `hash_before == hash_after`: print "No feed changes, skipping publish." and `exit 0`
     6. `jj commit -m "chore: update RSS feed" models-watch.xml` — commits only the feed file
     7. `jj bookmark set pages -r @-` — move pages bookmark to the feed commit
     8. `jj git push --remote origin --bookmark pages`
     9. Print "Feed published to pages bookmark."
   - Exit codes:
     - 0: published or skipped (no change / no deltas)
     - Non-zero: failure from models-watch.sh, models-feed.sh, or jj commands

2. **New file `.mise.toml` at repo root** — minimal:
   ```toml
   [tasks.publish-feed]
   description = "Detect model changes, regenerate RSS feed, and publish to Codeberg Pages"
   run = "./models-watch/publish-feed.sh"
   ```

3. **Update `models-watch/README.md`** — add a note in the Publishing section about the `publish-feed` task (advisory).

4. **Verify** with `bash -n models-watch/publish-feed.sh`. No acceptance test needed for this orchestration script — it's a thin glue layer over already-tested scripts.

## Likely files

- `models-watch/publish-feed.sh` — new, self-contained publish orchestrator
- `.mise.toml` (repo root) — new, single `publish-feed` task
- `models-watch/README.md` — advisory update to Publishing section

## Risks

- **jj push auth**: SSH key must be set up for `origin`. The script will fail at the push step if auth is broken.
- **Same-chain coupling**: `pages` and `main` share the same commit chain. Feed commits accumulate on the chain between `main` and `pages`. This is the existing design; this task doesn't change it.
- **Empty commit risk**: Prevented by the hash check — if the feed content is identical, the script exits before committing.
- **Other working-copy changes**: `jj commit -m "msg" models-watch.xml` with a fileset commits only the feed file; other changes stay in the new working copy.

## Validation

- `bash -n models-watch/publish-feed.sh` — syntax check
- Manual: `mise run publish-feed` — should either print "No feed changes, skipping publish." or commit + push + print "Feed published."
- Verify `jj log -r pages` shows the new commit if published
- Verify `curl -s https://hanlho.codeberg.page/tiny-cli/models-watch.xml | head` shows updated feed after push