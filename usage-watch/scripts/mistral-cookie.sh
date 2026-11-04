#!/usr/bin/env bash
# Check a Mistral admin Cookie header against usage-watch, then show where to
# store it. The cookie is read from stdin so it never shows up in `ps`.
set -euo pipefail
IFS=$'\n\t'

SERVICE="mistral-admin-cookie"
SAVE=0
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat <<'EOF'
usage: mistral-cookie.sh [--save] [--service NAME]

Reads a full Cookie header from stdin, checks the session, and runs
usage-watch with MISTRAL_ADMIN_COOKIE set. On success it prints the keychain
and dotfiles steps that make the cookie permanent.

  --save           store the cookie in the macOS keychain after a successful test
  --service NAME   keychain service name (default: mistral-admin-cookie)
  -h, --help       show this help

Paste the header when prompted, then press Enter.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --save) SAVE=1 ;;
    --service)
      [[ $# -ge 2 ]] || { echo "--service needs a value" >&2; exit 2; }
      SERVICE="$2"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

find_bin() {
  if [[ -n "${USAGE_WATCH_BIN:-}" ]]; then
    printf '%s' "$USAGE_WATCH_BIN"
  elif [[ -x "$REPO_ROOT/target/release/usage-watch" ]]; then
    printf '%s' "$REPO_ROOT/target/release/usage-watch"
  elif command -v usage-watch >/dev/null 2>&1; then
    command -v usage-watch
  else
    return 1
  fi
}

# Strip the "Cookie:" prefix, fold newlines, trim surrounding whitespace.
normalize() {
  local raw=$1
  raw=${raw//$'\r'/}
  raw=${raw//$'\n'/ }
  raw=$(printf '%s' "$raw" | sed -E 's/^[[:space:]]*[Cc]ookie:[[:space:]]*//')
  raw="${raw#"${raw%%[![:space:]]*}"}"
  raw="${raw%"${raw##*[![:space:]]}"}"
  printf '%s' "$raw"
}

has_cookie() {
  printf '%s' "$1" | grep -qE "(^|;[[:space:]]*)$2="
}

session_expiry() {
  command -v curl >/dev/null 2>&1 || return 0
  curl -sS --max-time 10 -H "Cookie: $1" \
    https://auth.mistral.ai/sessions/whoami 2>/dev/null |
    sed -n 's/.*"expires_at":"\([^"]*\)".*/\1/p'
}

days_until() {
  # whoami returns fractional seconds; `date` wants whole ones.
  local iso=${1%%.*} exp now
  iso="${iso%Z}Z"
  if ! exp=$(date -j -u -f "%Y-%m-%dT%H:%M:%SZ" "$iso" +%s 2>/dev/null); then
    exp=$(date -u -d "$iso" +%s 2>/dev/null) || return 1
  fi
  now=$(date -u +%s)
  printf '%s' $(((exp - now) / 86400))
}

save_cookie() {
  # `security` prompts twice; feeding the value twice keeps it off the command
  # line, where `ps` would expose it.
  printf '%s\n%s\n' "$1" "$1" |
    security add-generic-password -U -s "$SERVICE" -a "$USER" -w
}

bin=$(find_bin) || {
  echo "usage-watch not found. Build it with 'mise run release' or set USAGE_WATCH_BIN." >&2
  exit 2
}

cookie=""
if [[ -t 0 ]]; then
  echo "Paste the Cookie header from admin.mistral.ai, then press Enter:" >&2
  # Canonical mode caps a line at 1024 bytes and drops the newline past that,
  # which leaves `read` waiting forever on a full Cookie header. Restore from
  # the saved settings: `stty icanon` alone fails after min/time are set.
  saved=$(stty -g)
  stty -icanon min 1 time 0
  trap 'stty "$saved" 2>/dev/null' EXIT
  IFS= read -r -n 4096 cookie || true
  stty "$saved" 2>/dev/null || stty sane 2>/dev/null || true
else
  IFS= read -r cookie || true
fi
cookie=$(normalize "$cookie")

if [[ -z "$cookie" ]]; then
  echo "No cookie read from stdin." >&2
  exit 2
fi

missing=()
has_cookie "$cookie" "ory_session_[A-Za-z0-9]+" || missing+=("ory_session_*")
has_cookie "$cookie" "csrftoken" || missing+=("csrftoken")
if [[ ${#missing[@]} -gt 0 ]]; then
  echo "Cookie header is missing: ${missing[*]}" >&2
  echo "Copy the whole Cookie value from a request to admin.mistral.ai." >&2
  exit 2
fi

if expiry=$(session_expiry "$cookie") && [[ -n "$expiry" ]]; then
  if left=$(days_until "$expiry"); then
    echo "Session expires $expiry (${left} days left)."
  else
    echo "Session expires $expiry."
  fi
fi

echo
status=0
output=$(MISTRAL_ADMIN_COOKIE="$cookie" "$bin" -p mistral -p vibe 2>&1) || status=$?
printf '%s\n' "$output"

if [[ $status -ne 0 ]] || grep -q 'error:' <<<"$output"; then
  echo
  echo "The cookie did not work. If the output mentions redirects or HTTP 401," >&2
  echo "the session has expired: sign in at https://admin.mistral.ai and copy" >&2
  echo "a fresh Cookie header." >&2
  exit 1
fi

if [[ $SAVE -eq 1 ]]; then
  echo
  if save_cookie "$cookie"; then
    echo "Stored in the keychain as \"$SERVICE\"."
  else
    echo "Could not write to the keychain; add it by hand (see below)." >&2
  fi
fi

cat <<EOF

Cookie works. To make it permanent:

1. Store it in the keychain (prompts twice, so it stays out of shell history):
     security add-generic-password -U -s "$SERVICE" -a "\$USER" -w

   Or re-run this script with --save.

2. Load it in fish. Add this to
   packages/home-manager/.config/home-manager/fish.nix
   inside interactiveShellInit:

     if not set -q MISTRAL_ADMIN_COOKIE
       set -gx MISTRAL_ADMIN_COOKIE (security find-generic-password -s "$SERVICE" -w 2>/dev/null)
     end

3. Apply the dotfiles, then open a new shell:
     dr

The session lasts 90 days. When it expires, repeat step 1 with a fresh header.
EOF
