#!/usr/bin/env bash
#
# models-watch — detect opencode-go model changes from models.dev
#
# Usage:
#   ./models-watch.sh [--notify-file <path>]
#
# Fetches https://models.dev/api.json, extracts the opencode-go provider
# block, compares model IDs against the last snapshot, and writes a minimal
# JSON delta file only when models are added or removed.
#
# State lives in ./state/ relative to this script:
#   state/latest.json              — most recent snapshot (kept for next comparison)
#   state/change-<timestamp>.json  — delta, written only on change
#
# Notification:
#   Without --notify-file: fires an osascript popup (macOS only).
#   With --notify-file <path>: writes the notification message to <path>.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
STATE_DIR="${SCRIPT_DIR}/state"
API_URL="${MODELS_WATCH_API_URL:-https://models.dev/api.json}"
LATEST="${STATE_DIR}/latest.json"

# Parse flags
NOTIFY_FILE=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --notify-file)
            NOTIFY_FILE="$2"
            shift 2
            ;;
        *)
            echo "Unknown flag: $1" >&2
            exit 2
            ;;
    esac
done

# Ensure state directory exists
mkdir -p "$STATE_DIR"

# --- Fetch current opencode-go block ---
if [[ "$API_URL" == file://* ]]; then
    raw_json="$(cat "${API_URL#file://}")"
else
    raw_json="$(curl -sS --fail --max-time 30 "$API_URL")"
fi

current="$(echo "$raw_json" | jq '.["opencode-go"]')"

if [[ -z "$current" || "$current" == "null" ]]; then
    echo "ERROR: opencode-go block not found in API response" >&2
    exit 3
fi

# --- Compare with previous snapshot ---
change_detected=false
added=""
removed=""
changed_json="[]"

if [[ -f "$LATEST" ]]; then
    prev_ids="$(jq -r '.models | keys | sort[]' "$LATEST")"
    curr_ids="$(jq -r '.models | keys | sort[]' <<< "$current")"

    added="$(comm -13 <(echo "$prev_ids") <(echo "$curr_ids"))"
    removed="$(comm -23 <(echo "$prev_ids") <(echo "$curr_ids"))"

    # Detect name changes for models present in both snapshots
    changed_json="$(jq -n \
        --argjson prev "$(cat "$LATEST")" \
        --argjson curr "$current" \
        '($prev.models // {}) as $pm |
         ($curr.models // {}) as $cm |
         [($cm | keys[]) as $id |
          select($pm[$id] != null and $pm[$id].name != $cm[$id].name) |
          {id: $id, old_name: $pm[$id].name, new_name: $cm[$id].name}
         ]
        ')"

    changed_count="$(jq 'length' <<< "$changed_json")"

    if [[ -n "$added" || -n "$removed" || "$changed_count" -gt 0 ]]; then
        change_detected=true
    fi
else
    # First run: all models are effectively "added"
    curr_ids="$(jq -r '.models | keys | sort[]' <<< "$current")"
    added="$curr_ids"
    change_detected=true
fi

# --- Write new snapshot ---
echo "$current" > "$LATEST"

# --- On change: write delta and notify ---
if [[ "$change_detected" == "true" ]]; then
    TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    CHANGE_FILE="${STATE_DIR}/change-${TIMESTAMP}.json"

    added_json="[]"
    removed_json="[]"
    if [[ -n "$added" ]]; then
        added_json="$(echo "$added" | jq -R -s 'split("\n") | map(select(. != ""))')"
    fi
    if [[ -n "$removed" ]]; then
        removed_json="$(echo "$removed" | jq -R -s 'split("\n") | map(select(. != ""))')"
    fi

    jq -n --argjson added "$added_json" --argjson removed "$removed_json" \
        --argjson changed "$changed_json" \
        --arg ts "$TIMESTAMP" \
        '{timestamp: $ts, added: $added, removed: $removed, changed: $changed}' \
        > "$CHANGE_FILE"

    # Build human-readable message
    msg_lines=()
    if [[ -n "$added" ]]; then
        msg_lines+=("Added:")
        while IFS= read -r line; do
            [[ -n "$line" ]] && msg_lines+=("  • $line")
        done <<< "$added"
    fi
    if [[ -n "$removed" ]]; then
        [[ ${#msg_lines[@]} -gt 0 ]] && msg_lines+=("")
        msg_lines+=("Removed:")
        while IFS= read -r line; do
            [[ -n "$line" ]] && msg_lines+=("  • $line")
        done <<< "$removed"
    fi

    changed_count="$(jq 'length' <<< "$changed_json")"
    if [[ "$changed_count" -gt 0 ]]; then
        [[ ${#msg_lines[@]} -gt 0 ]] && msg_lines+=("")
        msg_lines+=("Changed:")
        while IFS= read -r line; do
            [[ -n "$line" ]] && msg_lines+=("  • $line")
        done <<< "$(jq -r '.[] | "\(.id): \"\(.old_name)\" → \"\(.new_name)\""' <<< "$changed_json")"
    fi

    msg=$(printf '%s\n' "${msg_lines[@]}")

    if [[ -n "$NOTIFY_FILE" ]]; then
        echo "$msg" > "$NOTIFY_FILE"
    elif [[ -z "${MODELS_WATCH_NO_OSASCRIPT:-}" ]]; then
        # Build AppleScript string expression: "line1" & return & "line2"
        apple_parts=()
        while IFS= read -r line; do
            line="${line//\"/\\\"}"
            apple_parts+=("\"$line\"")
        done <<< "$msg"

        apple_expr="${apple_parts[0]}"
        for ((i=1; i<${#apple_parts[@]}; i++)); do
            apple_expr+=" & return & ${apple_parts[$i]}"
        done

        osascript -e "display alert \"models-watch\" message $apple_expr as informational giving up after 30" >/dev/null 2>&1
    fi
fi

exit 0
