#!/usr/bin/env bash
#
# publish-feed — detect model changes, regenerate RSS feed, publish to GitHub Pages
#
# Runs the full pipeline from the tiny-cli repo root:
#   models-watch.sh → models-feed.sh → hash check → jj commit → push main
#
# GitHub Pages serves from docs/ on the main branch.
#
# Exit codes:
#   0 - feed published, or skipped (no change / no deltas)
#   non-zero - failure from models-watch.sh, models-feed.sh, or jj commands

set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FEED_URL="https://lhohan.github.io/tiny-cli/models-watch.rss"

cd "$REPO_ROOT"

# --------------------------------------------------------------------------
# Hash helper — shasum on macOS, sha256sum on Linux
# --------------------------------------------------------------------------
hash_file() {
    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d' ' -f1
    else
        sha256sum "$1" | cut -d' ' -f1
    fi
}

# --------------------------------------------------------------------------
# 1. Capture hash of existing feed (if any)
# --------------------------------------------------------------------------
hash_before=""
if [[ -f docs/models-watch.rss ]]; then
    hash_before=$(hash_file docs/models-watch.rss)
fi

# --------------------------------------------------------------------------
# 2. Detect model changes (aborts on failure via set -e)
# --------------------------------------------------------------------------
./models-watch/models-watch.sh

# --------------------------------------------------------------------------
# 3. Regenerate feed; exit 3 (no deltas) is a clean skip
# --------------------------------------------------------------------------
set +e
./models-watch/models-feed.sh --output docs/models-watch.rss --feed-url "$FEED_URL"
feed_exit=$?
set -e

if [[ $feed_exit -eq 3 ]]; then
    echo "No deltas found, nothing to publish."
    exit 0
elif [[ $feed_exit -ne 0 ]]; then
    echo "ERROR: models-feed.sh failed (exit $feed_exit)" >&2
    exit "$feed_exit"
fi

# --------------------------------------------------------------------------
# 4. Hash after regeneration
# --------------------------------------------------------------------------
hash_after=$(hash_file docs/models-watch.rss)

# --------------------------------------------------------------------------
# 5. Skip commit if feed content is identical
# --------------------------------------------------------------------------
if [[ "$hash_before" == "$hash_after" ]]; then
    echo "No feed changes, skipping publish."
    exit 0
fi

# --------------------------------------------------------------------------
# 6. Commit only the feed file (other changes stay in working copy)
# --------------------------------------------------------------------------
jj commit -m "chore: update RSS feed" docs/models-watch.rss

# --------------------------------------------------------------------------
# 7. Move main bookmark to the feed commit (@- after commit) and push
# --------------------------------------------------------------------------
jj bookmark set main -r @-
jj git push --bookmark main

echo "Feed published to GitHub Pages."
