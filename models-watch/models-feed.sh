#!/usr/bin/env bash
#
# models-feed — generate an RSS 2.0 feed from models-watch change deltas
#
# Reads state/change-*.json deltas produced by models-watch.sh and publishes
# an RSS 2.0 feed.  Designed to be run standalone — independent from
# models-watch.sh, no network access, no side effects beyond writing the feed.
#
# Usage:
#   ./models-feed.sh [--output <file>] [--state-dir <dir>] [--feed-url <url>]
#
# Flags:
#   --output <file>     Write feed to <file>  (default: state/feed.rss)
#   --state-dir <dir>   Read deltas from <dir> (default: state/ relative
#                       to this script's directory)
#   --feed-url <url>    Public URL of this feed (adds atom:link rel="self")
#
# Environment:
#   MODELS_WATCH_FEED_FILE   Same as --output (default: state/feed.rss)
#   MODELS_WATCH_STATE_DIR   Same as --state-dir
#   MODELS_WATCH_FEED_URL    Same as --feed-url
#
# Exit codes:
#   0  — feed written successfully (or nothing new; no error)
#   2  — unknown flag
#   3  — no change deltas exist (nothing to publish)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# ---------------------------------------------------------------------------
# Defaults & flag parsing
# ---------------------------------------------------------------------------

STATE_DIR="${MODELS_WATCH_STATE_DIR:-${SCRIPT_DIR}/state}"
OUTPUT_FILE="${MODELS_WATCH_FEED_FILE:-}"
FEED_URL="${MODELS_WATCH_FEED_URL:-}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        --state-dir)
            STATE_DIR="$2"
            shift 2
            ;;
        --feed-url)
            FEED_URL="$2"
            shift 2
            ;;
        *)
            echo "Unknown flag: $1" >&2
            exit 2
            ;;
    esac
done

# Default output if not set via flag or env
if [[ -z "$OUTPUT_FILE" ]]; then
    OUTPUT_FILE="${STATE_DIR}/feed.rss"
fi

# ---------------------------------------------------------------------------
# Gather delta files, newest-first
# ---------------------------------------------------------------------------

shopt -s nullglob
delta_files=("${STATE_DIR}"/change-*.json)
shopt -u nullglob

if [[ ${#delta_files[@]} -eq 0 ]]; then
    exit 3
fi

# Sort lexicographically (ISO timestamps sort correctly), take last 100, reverse
IFS=$'\n' sorted=($(sort <<<"${delta_files[*]}")); unset IFS

total=${#sorted[@]}
start=0
if [[ "$total" -gt 100 ]]; then
    start=$(( total - 100 ))
fi

selected=()
for ((i=start; i<total; i++)); do
    selected+=("${sorted[$i]}")
done

# Reverse so newest is first
newest_first=()
for ((i=${#selected[@]}-1; i>=0; i--)); do
    newest_first+=("${selected[$i]}")
done

# ---------------------------------------------------------------------------
# RFC-822 date helper  (dual-path: BSD /bin/date vs GNU date)
# ---------------------------------------------------------------------------
rfc822_now() {
    # Try GNU date's -R first (produces RFC-2822/822)
    if date -R +"%Y" >/dev/null 2>&1; then
        date -R
        return
    fi
    # Fallback for BSD date
    # %a = abbreviated weekday name, %d = day, %b = abbreviated month name,
    # %Y = year, %H:%M:%S = time, %z = timezone offset
    /bin/date "+%a, %d %b %Y %H:%M:%S %z"
}

rfc822_from_iso() {
    local iso_ts="$1"
    local result=""

    # Try BSD /bin/date first
    if /bin/date -j -f "%Y-%m-%dT%H:%M:%SZ" "$iso_ts" "+%s" >/dev/null 2>&1; then
        local unix_ts
        unix_ts=$(TZ=UTC /bin/date -j -f "%Y-%m-%dT%H:%M:%SZ" "$iso_ts" "+%s" 2>/dev/null || true)
        if [[ -n "$unix_ts" ]]; then
            result=$(TZ=UTC /bin/date -j -f "%s" "$unix_ts" "+%a, %d %b %Y %H:%M:%S +0000" 2>/dev/null || true)
        fi
    # Fall back to GNU date
    elif date -d "$iso_ts" "+%s" >/dev/null 2>&1; then
        result=$(TZ=UTC date -d "$iso_ts" "+%a, %d %b %Y %H:%M:%S +0000" 2>/dev/null || true)
    fi

    if [[ -z "$result" ]]; then
        echo "$iso_ts"
    else
        echo "$result"
    fi
}

# ---------------------------------------------------------------------------
# Build RSS feed
# ---------------------------------------------------------------------------

FEED_TITLE="models-watch"
FEED_LINK="https://models.dev"
FEED_DESC="Model change notifications for opencode-go and free OpenCode Zen models"
NOW_RFC822=$(rfc822_now)

# Build one <item> per (action x model), newest-first, capped at 100 items total
MAX_ITEMS=100
items_count=0
xml_escape() {
    local s="$1"
    s="${s//&/&amp;}"
    s="${s//</&lt;}"
    s="${s//>/&gt;}"
    echo "$s"
}

items_xml=""
for delta_file in "${newest_first[@]}"; do
    [[ $items_count -ge $MAX_ITEMS ]] && break

    ts=$(jq -r '.timestamp' "$delta_file")
    pub_date=$(rfc822_from_iso "$ts")

    # ---- Added models, one item each ----
    while IFS= read -r model_id; do
        [[ -z "$model_id" ]] && continue
        [[ $items_count -ge $MAX_ITEMS ]] && break 2

        escaped_id=$(xml_escape "$model_id")
        guid="models-watch-${ts}-new-${escaped_id}"
        description="<![CDATA[${model_id} is now available.]]>"

        items_xml+="
    <item>
      <guid isPermaLink=\"false\">${guid}</guid>
      <pubDate>${pub_date}</pubDate>
      <description>${description}</description>
    </item>"
        items_count=$((items_count + 1))
    done < <(jq -r '.added[]' "$delta_file")

    [[ $items_count -ge $MAX_ITEMS ]] && break

    # ---- Changed models, one item each ----
    while IFS=$'\t' read -r model_id old_name new_name; do
        [[ -z "$model_id" ]] && continue
        [[ $items_count -ge $MAX_ITEMS ]] && break 2

        escaped_id=$(xml_escape "$model_id")
        guid="models-watch-${ts}-updated-${escaped_id}"
        description="<![CDATA[${model_id}: \"${old_name}\" → \"${new_name}\"]]>"

        items_xml+="
    <item>
      <guid isPermaLink=\"false\">${guid}</guid>
      <pubDate>${pub_date}</pubDate>
      <description>${description}</description>
    </item>"
        items_count=$((items_count + 1))
    done < <(jq -r '.changed[] | [.id, .old_name, .new_name] | @tsv' "$delta_file")

    [[ $items_count -ge $MAX_ITEMS ]] && break

    # ---- Removed models, one item each ----
    while IFS= read -r model_id; do
        [[ -z "$model_id" ]] && continue
        [[ $items_count -ge $MAX_ITEMS ]] && break 2

        escaped_id=$(xml_escape "$model_id")
        guid="models-watch-${ts}-removed-${escaped_id}"
        description="<![CDATA[${model_id} is no longer available.]]>"

        items_xml+="
    <item>
      <guid isPermaLink=\"false\">${guid}</guid>
      <pubDate>${pub_date}</pubDate>
      <description>${description}</description>
    </item>"
        items_count=$((items_count + 1))
    done < <(jq -r '.removed[]' "$delta_file")
done

# --------------------------------------------------------------------------
# Assemble full RSS document
# --------------------------------------------------------------------------

# Add Atom namespace if we know our own URL (for atom:link rel="self")
ATOM_NS=""
ATOM_LINK=""
if [[ -n "$FEED_URL" ]]; then
    ATOM_NS=' xmlns:atom="http://www.w3.org/2005/Atom"'
    ATOM_LINK="    <atom:link href=\"${FEED_URL}\" rel=\"self\" type=\"application/rss+xml\"/>"$'\n'
fi

feed="<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<rss version=\"2.0\"${ATOM_NS}>
  <channel>
    ${ATOM_LINK}<title>${FEED_TITLE}</title>
    <link>${FEED_LINK}</link>
    <description>${FEED_DESC}</description>
    <language>en</language>
    <lastBuildDate>${NOW_RFC822}</lastBuildDate>
    ${items_xml}
  </channel>
</rss>"

# ---------------------------------------------------------------------------
# Atomic write
# ---------------------------------------------------------------------------

OUTPUT_DIR="$(dirname "$OUTPUT_FILE")"
mkdir -p "$OUTPUT_DIR"

tmp_file="$(mktemp "${OUTPUT_FILE}.XXXXXX")"
echo "$feed" > "$tmp_file"
mv "$tmp_file" "$OUTPUT_FILE"
