#!/usr/bin/env bash
#
# models-broadcast — broadcast model changes to Bluesky
#
# Usage:
#   ./models-broadcast.sh [--state-dir <dir>] [--capture-dir <dir>] [--limit <n>]
#
# Reads change deltas from state/ and either captures rendered posts as
# JSON files (--capture-dir) or posts them to Bluesky (live mode).
#
# State lives in ./state/ relative to this script by default:
#   state/change-<timestamp>.json  — delta files
#   state/posted.json              — ledger of posted deltas

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
STATE_DIR="${SCRIPT_DIR}/state"
CAPTURE_DIR=""
LIMIT=""

# ---------------------------------------------------------------------------
# Parse flags
# ---------------------------------------------------------------------------
has_value() {
    local val="$1"
    local flag="$2"
    if [[ -z "$val" || "$val" == --* ]]; then
        echo "ERROR: $flag requires a value" >&2
        exit 2
    fi
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --state-dir)
            has_value "${2:-}" "$1"
            STATE_DIR="$2"
            shift 2
            ;;
        --capture-dir)
            has_value "${2:-}" "$1"
            CAPTURE_DIR="$2"
            shift 2
            ;;
        --limit)
            has_value "${2:-}" "$1"
            LIMIT="$2"
            shift 2
            ;;
        *)
            echo "Unknown flag: $1" >&2
            exit 2
            ;;
    esac
done

# --limit must be a positive integer and is mutually exclusive with --capture-dir
if [[ -n "$LIMIT" ]]; then
    if ! [[ "$LIMIT" =~ ^[1-9][0-9]*$ ]]; then
        echo "ERROR: --limit must be a positive integer" >&2
        exit 2
    fi
    if [[ -n "$CAPTURE_DIR" ]]; then
        echo "ERROR: --limit and --capture-dir are mutually exclusive" >&2
        exit 2
    fi
fi

# ---------------------------------------------------------------------------
# Utilities
# ---------------------------------------------------------------------------

# Validate a delta file: must have a timestamp, added/removed arrays, and
# changed entries with id/old_name/new_name strings.  Every ID must start
# with opencode-go/ or opencode/.
validate_delta() {
    local file="$1"
    local raw
    raw="$(cat "$file")"

    # Must be valid JSON with required fields
    if ! echo "$raw" | jq -e '.timestamp | type == "string"' >/dev/null 2>&1; then
        echo "ERROR: delta $file missing string timestamp" >&2
        return 1
    fi
    if ! echo "$raw" | jq -e '.added | type == "array"' >/dev/null 2>&1; then
        echo "ERROR: delta $file missing added array" >&2
        return 1
    fi
    if ! echo "$raw" | jq -e '.removed | type == "array"' >/dev/null 2>&1; then
        echo "ERROR: delta $file missing removed array" >&2
        return 1
    fi
    if ! echo "$raw" | jq -e '.changed | type == "array"' >/dev/null 2>&1; then
        echo "ERROR: delta $file missing changed array" >&2
        return 1
    fi

    # Each changed entry must have id, old_name, new_name strings
    if ! echo "$raw" | jq -e '
        (.changed | type == "array") and
        ([.changed[] |
            (type == "object") and
            (.id | type == "string") and
            (.old_name | type == "string") and
            (.new_name | type == "string")
        ] | all)
    ' >/dev/null 2>&1; then
        echo "ERROR: delta $file has malformed changed entries" >&2
        return 1
    fi

    # Every ID in added, removed, and changed must be provider-prefixed
    if ! echo "$raw" | jq -e '
        ([.added[], .removed[], (.changed | .[].id)] | all(
            type == "string" and
            (startswith("opencode-go/") or startswith("opencode/"))
        ))' >/dev/null 2>&1; then
        echo "ERROR: delta $file contains non-provider-prefixed IDs" >&2
        return 1
    fi

    return 0
}

# 300 Unicode code point limit for posts
readonly POST_MAX=300

# Truncate a string to at most max_cp code points.
# Appends … if shortened. Logs original/final to stderr when field_name given.
truncate_value() {
    local value="$1"
    local max_cp="$2"
    local field_name="${3:-}"

    local cp_count
    cp_count=$(jq -nr --arg t "$value" '$t | length')

    if (( cp_count <= max_cp )); then
        echo "$value"
        return
    fi

    local new_len=$(( max_cp - 1 ))
    if (( new_len < 0 )); then
        new_len=0
    fi

    local result
    result=$(printf '%s' "$value" | jq -R -r --argjson n "$new_len" '
        if length <= $n then .
        elif $n <= 0 then "…"
        else .[0:$n] + "…"
        end
    ')

    if [[ -n "$field_name" ]]; then
        echo "TRUNC: ${field_name} original=\"${value}\" final=\"${result}\"" >&2
    fi

    echo "$result"
}

# Render the rich "added" announcement from delta metadata. Emits the
# capture/ledger JSON record on stdout; returns 1 when the delta carries no
# .models[id] entry so the caller falls back to the legacy one-line format.
#
# The post includes every field we have data for; if it exceeds POST_MAX code
# points, optional fields are dropped in order of increasing importance
# (Capabilities -> Context -> Pricing -> URL), then the title name and finally
# the ID are truncated. The legacy one-line tail below is unreachable after the
# ID-truncation step; it is retained as a defensive guard.
render_rich_added() {
    local delta_file="$1"
    local model_id="$2"
    local delta_name="$3"
    local overshoot=0

    if ! jq -e --arg id "$model_id" '.models | has($id)' "$delta_file" >/dev/null 2>&1; then
        return 1
    fi

    local tier_name tier_url
    case "$model_id" in
        opencode/*)
            tier_name="OpenCode Zen"
            tier_url="https://models.dev/providers/opencode/"
            ;;
        opencode-go/*)
            tier_name="OpenCode Go"
            tier_url="https://models.dev/providers/opencode-go/"
            ;;
        *)
            return 1
            ;;
    esac

    local meta
    meta="$(jq -c --arg id "$model_id" '.models[$id]' "$delta_file")"

    local name free in_cost out_cost
    name="$(jq -r '.name // empty' <<< "$meta")"
    if [[ -z "$name" ]]; then
        name="${model_id#*/}"
    fi
    free="$(jq -r '(.cost.input == 0 and .cost.output == 0)' <<< "$meta")"
    in_cost="$(jq -r '.cost.input // 0' <<< "$meta")"
    out_cost="$(jq -r '.cost.output // 0' <<< "$meta")"

    # The display name usually already ends with " Free" for free Zen models;
    # only append the word when it is not already present.
    local title_name="$name"
    if [[ "$free" == "true" && "$name" != *" Free" ]]; then
        title_name="${name} Free"
    fi

    local pricing_line
    pricing_line="Pricing: \$$(printf '%.2f' "$in_cost") / \$$(printf '%.2f' "$out_cost") per 1M tokens"
    if [[ "$free" == "true" ]]; then
        pricing_line="${pricing_line} (free)"
    fi

    # Build all sections (title, blank, ID, pricing, context, capabilities,
    # blank, URL) as a JSON array so the inclusion ladder can drop fields.
    local sections
    sections="$(jq -c \
        --arg title "New on ${tier_name}: ${title_name}" \
        --arg id "$model_id" \
        --arg pricing "$pricing_line" \
        --arg url "$tier_url" \
        '
        def fmt_ctx:
          if . >= 1000000 then ((. / 1000000 * 10 | floor) / 10) as $v |
            if $v == ($v | floor) then "\($v | floor)M" else "\($v)M" end
          elif . >= 1000 then ((. / 1000 * 10 | floor) / 10) as $v |
            if $v == ($v | floor) then "\($v | floor)K" else "\($v)K" end
          else tostring end;
        def fmt_limit($v): if $v == null then "–" else ($v | fmt_ctx) end;
        def yn($k): if has($k) then (if .[$k] then "Yes" else "No" end) else "–" end;
        [
          {key: "title", text: $title},
          {key: "blank1", text: ""},
          {key: "id", text: ("ID: " + $id)},
          {key: "pricing", text: $pricing},
          {key: "context", text: ("Context: " + fmt_limit(.limit.context) + ", max output " + fmt_limit(.limit.output))},
          {key: "caps", text: ("Capabilities: Tool calling " + yn("tool_call") + " | Structured output " + yn("structured_output") + " | Reasoning " + yn("reasoning") + " | Attachment support " + yn("attachment"))},
          {key: "blank2", text: ""},
          {key: "url", text: $url}
        ]
        ' <<< "$meta")"

    local text total_cp
    text="$(jq -r 'map(.text) | join("\n")' <<< "$sections")"
    total_cp="$(jq -nr --arg t "$text" '$t | length')"

    # Inclusion ladder: drop least-important optional fields until it fits.
    local drop_order=("caps" "context" "pricing" "url")
    local key
    for key in "${drop_order[@]}"; do
        if (( total_cp <= POST_MAX )); then
            break
        fi
        if [[ "$key" == "url" ]]; then
            sections="$(jq -c 'map(select(.key != "url" and .key != "blank2"))' <<< "$sections")"
        else
            sections="$(jq -c --arg key "$key" 'map(select(.key != $key))' <<< "$sections")"
        fi
        text="$(jq -r 'map(.text) | join("\n")' <<< "$sections")"
        total_cp="$(jq -nr --arg t "$text" '$t | length')"
        echo "DROP: ${key} line" >&2
    done

    # Residual overflow with no optional fields left: truncate the title
    # name, then the ID.
    if (( total_cp > POST_MAX )); then
        overshoot=$(( total_cp - POST_MAX ))
        local title_name_len max_name short_name new_title
        title_name_len="$(jq -nr --arg t "$title_name" '$t | length')"
        max_name=$(( title_name_len - overshoot - 1 ))
        if (( max_name < 1 )); then max_name=1; fi
        short_name="$(truncate_value "$title_name" "$max_name" "name")"
        new_title="New on ${tier_name}: ${short_name}"
        sections="$(jq -c --arg title "$new_title" 'map(if .key == "title" then .text = $title else . end)' <<< "$sections")"
        text="$(jq -r 'map(.text) | join("\n")' <<< "$sections")"
        total_cp="$(jq -nr --arg t "$text" '$t | length')"
    fi

    # The name could not absorb the overflow: truncate the ID itself.
    if (( total_cp > POST_MAX )); then
        local id_cp max_id short_id
        id_cp="$(jq -nr --arg t "$model_id" '$t | length')"
        max_id=$(( id_cp - overshoot - 1 ))
        if (( max_id < 1 )); then max_id=1; fi
        short_id="$(truncate_value "$model_id" "$max_id" "model_id")"
        sections="$(jq -c --arg id "ID: ${short_id}" \
            'map(if .key == "id" then .text = $id else . end)' <<< "$sections")"
        text="$(jq -r 'map(.text) | join("\n")' <<< "$sections")"
        total_cp="$(jq -nr --arg t "$text" '$t | length')"
    fi

    # Defensive tail: unreachable while the ID-truncation step above exists.
    if (( total_cp > POST_MAX )); then
        local mid_len max_mid mid full cp_count
        mid="$model_id"
        full="New: ${model_id} is now available."
        cp_count="$(jq -nr --arg t "$full" '$t | length')"
        if (( cp_count > POST_MAX )); then
            overshoot=$(( cp_count - POST_MAX ))
            mid_len="$(jq -nr --arg t "$mid" '$t | length')"
            max_mid=$(( mid_len - overshoot - 1 ))
            if (( max_mid < 1 )); then max_mid=1; fi
            mid="$(truncate_value "$mid" "$max_mid" "model_id")"
            full="New: ${mid} is now available."
        fi
        jq -n --arg delta "$delta_name" --arg model_id "$mid" \
            --arg action "added" --arg text "$full" \
            '{delta: $delta, model_id: $model_id, action: $action, text: $text}'
        return
    fi

    # Link facet for the URL line. Bluesky facets use UTF-8 byte offsets; the
    # URL is the final line with no trailing whitespace, so byteStart is the
    # total byte length minus the URL's byte length and byteEnd is the total.
    local facets_json=""
    if jq -e '[.[] | select(.key == "url")] | length == 1' <<< "$sections" >/dev/null 2>&1; then
        local total_bytes url_bytes byte_start
        total_bytes="$(printf '%s' "$text" | wc -c | tr -d '[:space:]')"
        url_bytes="$(printf '%s' "$tier_url" | wc -c | tr -d '[:space:]')"
        byte_start=$(( total_bytes - url_bytes ))
        facets_json="$(jq -n --arg uri "$tier_url" --argjson bs "$byte_start" --argjson be "$total_bytes" \
            '[{index: {byteStart: $bs, byteEnd: $be}, features: [{"$type": "app.bsky.richtext.facet#link", uri: $uri}]}]')"
    fi

    if [[ -n "$facets_json" ]]; then
        jq -n --arg delta "$delta_name" --arg model_id "$model_id" \
            --arg action "added" --arg text "$text" --argjson facets "$facets_json" \
            '{delta: $delta, model_id: $model_id, action: $action, text: $text, facets: $facets}'
    else
        jq -n --arg delta "$delta_name" --arg model_id "$model_id" \
            --arg action "added" --arg text "$text" \
            '{delta: $delta, model_id: $model_id, action: $action, text: $text}'
    fi
}

# Render one post text for a model, with truncation to POST_MAX.
render_post() {
    local delta_file="$1"
    local action="$2"       # added|changed|removed
    local model_id="$3"
    local old_name="${4:-}"
    local new_name="${5:-}"

    # `delta_file` is the full path (needed to read .models metadata); the
    # emitted `delta` field keeps the basename so capture output is stable.
    local delta_name
    delta_name="$(basename "$delta_file")"

    local current_mid="$model_id"
    local current_old="$old_name"
    local current_new="$new_name"
    local cp_count=0
    local overshoot=0

    case "$action" in
        changed)
            # Build full text and check against limit
            local full="Updated: ${current_mid}: \"${current_old}\" → \"${current_new}\""
            cp_count=$(printf '%s' "$full" | wc -m)

            if (( cp_count <= POST_MAX )); then
                # No truncation needed
                jq -n --arg delta "$delta_name" --arg model_id "$current_mid" \
                    --arg action "$action" --arg text "$full" \
                    '{delta: $delta, model_id: $model_id, action: $action, text: $text}'
                return
            fi

            # 1. Shorten old_name
            overshoot=$(( cp_count - POST_MAX ))
            local old_len
            old_len=$(printf '%s' "$current_old" | wc -m)
            local max_old=$(( old_len - overshoot - 1 ))
            if (( max_old < 1 )); then max_old=1; fi
            current_old=$(truncate_value "$current_old" "$max_old" "old_name")

            # Rebuild and check
            full="Updated: ${current_mid}: \"${current_old}\" → \"${current_new}\""
            cp_count=$(printf '%s' "$full" | wc -m)
            if (( cp_count <= POST_MAX )); then
                jq -n --arg delta "$delta_name" --arg model_id "$current_mid" \
                    --arg action "$action" --arg text "$full" \
                    '{delta: $delta, model_id: $model_id, action: $action, text: $text}'
                return
            fi

            # 2. Shorten new_name
            overshoot=$(( cp_count - POST_MAX ))
            local new_len
            new_len=$(printf '%s' "$current_new" | wc -m)
            local max_new=$(( new_len - overshoot - 1 ))
            if (( max_new < 1 )); then max_new=1; fi
            current_new=$(truncate_value "$current_new" "$max_new" "new_name")

            # Rebuild and check
            full="Updated: ${current_mid}: \"${current_old}\" → \"${current_new}\""
            cp_count=$(printf '%s' "$full" | wc -m)
            if (( cp_count <= POST_MAX )); then
                jq -n --arg delta "$delta_name" --arg model_id "$current_mid" \
                    --arg action "$action" --arg text "$full" \
                    '{delta: $delta, model_id: $model_id, action: $action, text: $text}'
                return
            fi

            # 3. Shorten model_id (last resort)
            overshoot=$(( cp_count - POST_MAX ))
            local mid_len
            mid_len=$(printf '%s' "$current_mid" | wc -m)
            local max_mid=$(( mid_len - overshoot - 1 ))
            if (( max_mid < 1 )); then max_mid=1; fi
            current_mid=$(truncate_value "$current_mid" "$max_mid" "model_id")

            full="Updated: ${current_mid}: \"${current_old}\" → \"${current_new}\""
            jq -n --arg delta "$delta_name" --arg model_id "$current_mid" \
                --arg action "$action" --arg text "$full" \
                '{delta: $delta, model_id: $model_id, action: $action, text: $text}'
            ;;

        added|removed)
            # Rich announcement when the delta carries metadata for this model;
            # otherwise the legacy one-line format.
            if [[ "$action" == "added" ]]; then
                local rich
                if rich="$(render_rich_added "$delta_file" "$model_id" "$delta_name")"; then
                    echo "$rich"
                    return
                fi
            fi

            local prefix="New: "
            local suffix=" is now available."
            if [[ "$action" == "removed" ]]; then
                prefix="Removed: "
                suffix=" is no longer available."
            fi

            local full="${prefix}${current_mid}${suffix}"
            cp_count=$(printf '%s' "$full" | wc -m)

            if (( cp_count <= POST_MAX )); then
                jq -n --arg delta "$delta_name" --arg model_id "$current_mid" \
                    --arg action "$action" --arg text "$full" \
                    '{delta: $delta, model_id: $model_id, action: $action, text: $text}'
                return
            fi

            # Shorten model_id
            overshoot=$(( cp_count - POST_MAX ))
            mid_len=$(printf '%s' "$current_mid" | wc -m)
            local max_mid=$(( mid_len - overshoot - 1 ))
            if (( max_mid < 1 )); then max_mid=1; fi
            current_mid=$(truncate_value "$current_mid" "$max_mid" "model_id")

            full="${prefix}${current_mid}${suffix}"
            jq -n --arg delta "$delta_name" --arg model_id "$current_mid" \
                --arg action "$action" --arg text "$full" \
                '{delta: $delta, model_id: $model_id, action: $action, text: $text}'
            ;;
    esac
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

# Read the ledger if it exists
LEDGER_FILE="${STATE_DIR}/posted.json"

# Validate ledger shape if present
if [[ -f "$LEDGER_FILE" ]]; then
    if ! jq -e '
        type == "object" and
        (.deltas | type == "object" or .deltas == null) and
        (.skipped | type == "object" or .skipped == null) and
        ([.deltas[]? | type == "string" and test("^[0-9a-fA-F]{64}$")] | all) and
        ([.skipped[]? | type == "string"] | all) and
        (. as $root | [(.deltas // {}) | keys[] | select(($root.skipped // {})[.] != null)] | length == 0)
    ' "$LEDGER_FILE" >/dev/null 2>&1; then
        echo "ERROR: posted.json has invalid shape (must be object with optional deltas/skipped maps, no overlapping keys)" >&2
        exit 1
    fi
fi

# Find all delta files
shopt -s nullglob
delta_files=("${STATE_DIR}"/change-*.json)
shopt -u nullglob

if [[ ${#delta_files[@]} -eq 0 ]]; then
    echo "No delta files found in ${STATE_DIR}" >&2
    exit 3
fi

# Sort by filename (ISO timestamps sort lexicographically)
IFS=$'\n' sorted_deltas=($(sort <<<"${delta_files[*]}")); unset IFS

# Filter out already-ledgered deltas (if ledger exists)
eligible=()
for df in "${sorted_deltas[@]}"; do
    basename_df="$(basename "$df")"

    # Skip if already in ledger deltas or skipped
    if [[ -f "$LEDGER_FILE" ]]; then
        if jq -e --arg name "$basename_df" '(.deltas // {})[$name] != null' "$LEDGER_FILE" >/dev/null 2>&1; then
            continue
        fi
        if jq -e --arg name "$basename_df" '(.skipped // {})[$name] != null' "$LEDGER_FILE" >/dev/null 2>&1; then
            continue
        fi
    fi

    eligible+=("$df")
done

if [[ ${#eligible[@]} -eq 0 ]]; then
    echo "No unledgered deltas to process" >&2
    exit 3
fi

# Apply --limit if set
if [[ -n "$LIMIT" ]]; then
    eligible=("${eligible[@]:0:$LIMIT}")
fi

# Reject malformed input before capture mode or live mode can produce partial
# output.  In particular, live mode must never post an earlier delta when a
# later selected delta is invalid.
for df in "${eligible[@]}"; do
    if ! validate_delta "$df"; then
        echo "ERROR: invalid delta $(basename "$df"), aborting" >&2
        exit 1
    fi
done

# ---------------------------------------------------------------------------
# Capture mode: write render records without auth or state changes
# ---------------------------------------------------------------------------
if [[ -n "$CAPTURE_DIR" ]]; then
    mkdir -p "$CAPTURE_DIR"

    file_counter=1
    for df in "${eligible[@]}"; do
        basename_df="$(basename "$df")"

        # Render posts in removed, changed, added order, each alphabetically
        # Removed
        while IFS=$'\n' read -r model_id; do
            [[ -z "$model_id" ]] && continue
            render_post "$df" "removed" "$model_id" \
                > "${CAPTURE_DIR}/${file_counter}.json"
            file_counter=$((file_counter + 1))
        done < <(jq -r '.removed | sort[]' "$df")

        # Changed
        while IFS=$'\n' read -r line; do
            [[ -z "$line" ]] && continue
            # line is a JSON object with id, old_name, new_name
            mid="$(echo "$line" | jq -r '.id')"
            old="$(echo "$line" | jq -r '.old_name')"
            new="$(echo "$line" | jq -r '.new_name')"
            render_post "$df" "changed" "$mid" "$old" "$new" \
                > "${CAPTURE_DIR}/${file_counter}.json"
            file_counter=$((file_counter + 1))
        done < <(jq -c '.changed | sort_by(.id)[]' "$df")

        # Added
        while IFS=$'\n' read -r model_id; do
            [[ -z "$model_id" ]] && continue
            render_post "$df" "added" "$model_id" \
                > "${CAPTURE_DIR}/${file_counter}.json"
            file_counter=$((file_counter + 1))
        done < <(jq -r '.added | sort[]' "$df")
    done

    exit 0
fi

# ---------------------------------------------------------------------------
# Live posting mode
# ---------------------------------------------------------------------------

if [[ -z "${BLUESKY_HANDLE:-}" || -z "${BLUESKY_APP_PASSWORD:-}" ]]; then
    echo "ERROR: BLUESKY_HANDLE and BLUESKY_APP_PASSWORD are required for live posting" >&2
    exit 4
fi

PDS_URL="${BLUESKY_PDS:-https://bsky.social}"
PDS_URL="${PDS_URL%%/}"  # strip trailing slash

# ---------------------------------------------------------------------------
# Transport abstraction
# ---------------------------------------------------------------------------
pds_session_body() {
    local handle="$1"
    local password="$2"
    jq -n --arg handle "$handle" --arg pass "$password" \
        '{identifier: $handle, password: $pass}'
}

pds_record_body() {
    local did="$1"
    local text="$2"
    local created_at="$3"
    local facets_json="${4:-}"
    if [[ -n "$facets_json" && "$facets_json" != "[]" ]]; then
        jq -n --arg did "$did" --arg text "$text" --arg ts "$created_at" \
            --argjson facets "$facets_json" \
            '{repo: $did, collection: "app.bsky.feed.post", record: {text: $text, createdAt: $ts, facets: $facets}}'
    else
        jq -n --arg did "$did" --arg text "$text" --arg ts "$created_at" \
            '{repo: $did, collection: "app.bsky.feed.post", record: {text: $text, createdAt: $ts}}'
    fi
}

pds_request() {
    local endpoint="$1"
    local number="$2"
    local jwt="${3:-}"
    local body="${4:-}"

    if [[ "$PDS_URL" == file://* ]]; then
        # file:// transport: read fixture files
        local fixture_root="${PDS_URL#file://}"
        local fixture_path="${fixture_root}/xrpc/${endpoint}/${number}.json"
        if [[ ! -f "$fixture_path" ]]; then
            echo '{"transport_error": "fixture not found"}'
            return 1
        fi
        cat "$fixture_path"
    else
        # Real HTTP transport via curl
        local url="${PDS_URL}/xrpc/${endpoint}"
        local -a curl_args=(-sS --max-time 30 -X POST -H "Content-Type: application/json")
        local response_file http_status response curl_status=0
        if [[ -n "$jwt" ]]; then
            curl_args+=(-H "Authorization: Bearer ${jwt}")
        fi
        if [[ -n "$body" ]]; then
            curl_args+=(-d "$body")
        fi
        response_file="$(mktemp "${TMPDIR:-/tmp}/models-broadcast.XXXXXX")"
        if http_status="$(curl "${curl_args[@]}" --output "$response_file" --write-out '%{http_code}' "$url")"; then
            :
        else
            curl_status=$?
        fi
        response="$(<"$response_file")"
        rm -f "$response_file"

        # Use the same envelope as file:// fixtures so HTTP error bodies cannot
        # be mistaken for successful createRecord responses.
        if (( curl_status != 0 )); then
            jq -cn --arg status "$http_status" --arg body "$response" \
                '{transport_error: "curl failed", status: ($status | tonumber?), body: ($body | fromjson? // $body)}'
        else
            jq -cn --arg status "$http_status" --arg body "$response" \
                '{status: ($status | tonumber?), body: ($body | fromjson? // $body)}'
        fi
    fi
}

pds_response_ok() {
    local resp="$1"
    # file:// fixture with transport_error
    if echo "$resp" | jq -e '.transport_error' >/dev/null 2>&1; then
        return 1
    fi
    # Both transports return a status envelope. Only a 2xx response is success.
    if ! echo "$resp" | jq -e '(.status | type == "number") and (.status >= 200 and .status < 300)' >/dev/null 2>&1; then
        return 1
    fi
    return 0
}

pds_response_body() {
    local resp="$1"
    # If the response has a 'body' field (file:// envelope), extract it
    if echo "$resp" | jq -e '.body' >/dev/null 2>&1; then
        echo "$resp" | jq -c '.body'
    else
        # Plain response (curl output for real PDS)
        echo "$resp"
    fi
}

# Create session
session_body="$(pds_session_body "$BLUESKY_HANDLE" "$BLUESKY_APP_PASSWORD")"
if ! session_resp="$(pds_request "com.atproto.server.createSession" 1 "" "$session_body")"; then
    echo "ERROR: createSession request failed" >&2
    exit 4
fi

if ! pds_response_ok "$session_resp"; then
    echo "ERROR: createSession failed: $(echo "$session_resp" | jq -c .)" >&2
    exit 4
fi

session_body_json="$(pds_response_body "$session_resp")"
ACCESS_JWT="$(echo "$session_body_json" | jq -r '.accessJwt // empty')"
DID="$(echo "$session_body_json" | jq -r '.did // empty')"

if [[ -z "$ACCESS_JWT" || -z "$DID" ]]; then
    echo "ERROR: createSession response missing accessJwt or did: $(echo "$session_body_json" | jq -c .)" >&2
    exit 4
fi

# ---------------------------------------------------------------------------
# Process each delta: render, post, compute hash, update ledger
# ---------------------------------------------------------------------------

# Compute SHA-256
sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$@" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$@" | awk '{print $1}'
    else
        echo "ERROR: no sha256sum or shasum available" >&2
        exit 1
    fi
}

# Persist each fully posted delta before moving on. This leaves only the
# unavoidable ambiguity within a delta if a request fails after acceptance.
write_ledger_entry() {
    local name="$1"
    local hash="$2"
    local updated tmp_file

    mkdir -p "$STATE_DIR"
    if [[ -f "$LEDGER_FILE" ]]; then
        updated="$(jq --arg name "$name" --arg hash "$hash" \
            '.deltas = ((.deltas // {}) + {($name): $hash})' "$LEDGER_FILE")"
    else
        updated="$(jq -n --arg name "$name" --arg hash "$hash" '{deltas: {($name): $hash}}')"
    fi

    tmp_file="$(mktemp "${LEDGER_FILE}.tmp.XXXXXX")"
    printf '%s\n' "$updated" > "$tmp_file"
    mv "$tmp_file" "$LEDGER_FILE"
}

request_counter=1

for df in "${eligible[@]}"; do
    basename_df="$(basename "$df")"

    # Collect rendered post texts for this delta
    post_texts=()

    # Removed (alphabetical)
    while IFS=$'\n' read -r model_id; do
        [[ -z "$model_id" ]] && continue
        post_json="$(render_post "$df" "removed" "$model_id")"
        text="$(echo "$post_json" | jq -r '.text')"
        facets="$(echo "$post_json" | jq -c '.facets // empty')"
        post_texts+=("$text")
        created_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        record_body="$(pds_record_body "$DID" "$text" "$created_at" "$facets")"
        if ! resp="$(pds_request "com.atproto.repo.createRecord" "$request_counter" "$ACCESS_JWT" "$record_body")"; then
            echo "ERROR: createRecord request failed for $model_id" >&2
            exit 1
        fi
        if ! pds_response_ok "$resp"; then
            echo "ERROR: createRecord failed for $model_id" >&2
            exit 1
        fi
        request_counter=$((request_counter + 1))
        sleep 1
    done < <(jq -r '.removed | sort[]' "$df")

    # Changed (alphabetical by id)
    while IFS=$'\n' read -r line; do
        [[ -z "$line" ]] && continue
        mid="$(echo "$line" | jq -r '.id')"
        old="$(echo "$line" | jq -r '.old_name')"
        new="$(echo "$line" | jq -r '.new_name')"
        post_json="$(render_post "$df" "changed" "$mid" "$old" "$new")"
        text="$(echo "$post_json" | jq -r '.text')"
        facets="$(echo "$post_json" | jq -c '.facets // empty')"
        post_texts+=("$text")
        created_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        record_body="$(pds_record_body "$DID" "$text" "$created_at" "$facets")"
        if ! resp="$(pds_request "com.atproto.repo.createRecord" "$request_counter" "$ACCESS_JWT" "$record_body")"; then
            echo "ERROR: createRecord request failed for $mid" >&2
            exit 1
        fi
        if ! pds_response_ok "$resp"; then
            echo "ERROR: createRecord failed for $mid" >&2
            exit 1
        fi
        request_counter=$((request_counter + 1))
        sleep 1
    done < <(jq -c '.changed | sort_by(.id)[]' "$df")

    # Added (alphabetical)
    while IFS=$'\n' read -r model_id; do
        [[ -z "$model_id" ]] && continue
        post_json="$(render_post "$df" "added" "$model_id")"
        text="$(echo "$post_json" | jq -r '.text')"
        facets="$(echo "$post_json" | jq -c '.facets // empty')"
        post_texts+=("$text")
        created_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        record_body="$(pds_record_body "$DID" "$text" "$created_at" "$facets")"
        if ! resp="$(pds_request "com.atproto.repo.createRecord" "$request_counter" "$ACCESS_JWT" "$record_body")"; then
            echo "ERROR: createRecord request failed for $model_id" >&2
            exit 1
        fi
        if ! pds_response_ok "$resp"; then
            echo "ERROR: createRecord failed for $model_id" >&2
            exit 1
        fi
        request_counter=$((request_counter + 1))
        sleep 1
    done < <(jq -r '.added | sort[]' "$df")

    # Compute hash: SHA-256 of compact JSON array of post texts (UTF-8, no trailing newline)
    hash_input="$(jq -nc '$ARGS.positional' --args "${post_texts[@]}")"
    hash="$(printf '%s' "$hash_input" | sha256)"
    write_ledger_entry "$basename_df" "$hash"
done

exit 0
